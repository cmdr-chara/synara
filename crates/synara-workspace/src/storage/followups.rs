//! A durable text draft queue, not a scheduler. Reading/reordering never runs an agent.
use super::*;
use crate::{WorkspaceError,WorkspaceResult,WorkspaceService};
use serde::{Deserialize,Serialize};
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FollowupDraft { pub id:String, pub text:String }
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FollowupQueue { pub version:u32, pub revision:u64, pub items:Vec<FollowupDraft> }
impl Default for FollowupQueue {
    fn default()->Self {Self {version:1,revision:0,items:vec![]}}
}
impl FollowupQueue {
    fn validate(&self)->WorkspaceResult<()> {
        if self.version!=1 || self.items.len()>16 || self.items.iter().enumerate().any(|(i,item)|
            item.id.len()!=36 || uuid::Uuid::parse_str(&item.id).is_err() || !valid_text(&item.text)
                || self.items[..i].iter().any(|old|old.id==item.id)) {
            return Err(WorkspaceError::Invalid("Stored follow-up queue is invalid or from an unsupported version. It was not replaced.".into()));
        }
        Ok(())
    }
}
fn valid_text(text:&str)->bool {!text.trim().is_empty() && text.len()<=64*1024 && !text.contains('\0')}
#[derive(Clone)]
pub enum FollowupEdit { Add(String), Edit {id:String,text:String}, Remove(String), Move {id:String,up:bool} }
fn key(task:TaskId)->String {format!("task-followups:{task}")}
fn read(connection:&Connection,task:TaskId)->WorkspaceResult<FollowupQueue> {
    let exists:bool=connection.query_row("SELECT EXISTS(SELECT 1 FROM tasks WHERE id=?1)",[task.to_string()],|row|row.get(0)).map_err(StorageError::from)?;
    if !exists {return Err(WorkspaceError::NotFound);}
    let raw:Option<String>=connection.query_row("SELECT data FROM preferences WHERE key=?1",[key(task)],|r|r.get(0)).optional().map_err(StorageError::from)?;
    let queue:FollowupQueue=raw.as_deref().map(decode).transpose()?.unwrap_or_default();
    queue.validate()?;Ok(queue)
}
impl WorkspaceService {
    pub async fn followup_queue(&self,task:TaskId)->WorkspaceResult<FollowupQueue> {
        self.access(move|store|read(&store.connection,task)).await
    }
    pub async fn edit_followups(&self,task:TaskId,revision:u64,edit:FollowupEdit)->WorkspaceResult<FollowupQueue> {
        self.access(move|store|{
            let tx=store.connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(StorageError::from)?;
            let mut queue=read(&tx,task)?;
            if queue.revision!=revision {return Err(WorkspaceError::Invalid("Follow-ups changed elsewhere. Reload before saving. Your current text is retained.".into()));}
            match edit {
                FollowupEdit::Add(text)=>{
                    if !valid_text(&text) {return Err(WorkspaceError::Invalid("A follow-up must contain text and fit within 64 KiB.".into()));}
                    queue.items.push(FollowupDraft {id:uuid::Uuid::new_v4().to_string(),text});
                }
                FollowupEdit::Edit {id,text}=>{
                    if !valid_text(&text) {return Err(WorkspaceError::Invalid("A follow-up must contain text and fit within 64 KiB.".into()));}
                    queue.items.iter_mut().find(|item|item.id==id).ok_or(WorkspaceError::NotFound)?.text=text;
                }
                FollowupEdit::Remove(id)=>queue.items.retain(|item|item.id!=id),
                FollowupEdit::Move {id,up}=>{
                    let i=queue.items.iter().position(|item|item.id==id).ok_or(WorkspaceError::NotFound)?;
                    let j=if up{i.checked_sub(1)}else{i.checked_add(1).filter(|j|*j<queue.items.len())};
                    if let Some(j)=j {queue.items.swap(i,j);}
                }
            }
            queue.validate()?;
            queue.revision=queue.revision.checked_add(1).ok_or(StorageError::Limit)?;
            tx.execute("INSERT INTO preferences(key,data) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET data=excluded.data",params![key(task),encode(&queue)?]).map_err(StorageError::from)?;
            tx.commit().map_err(StorageError::from)?;Ok(queue)
        }).await
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn followups_preserve_unicode_order_reject_stale_edits_and_do_not_start_work() {
        let dir=tempfile::tempdir().unwrap();let service=WorkspaceService::memory().unwrap();
        let project=service.add_local_workspace(dir.path().into()).await.unwrap();
        let agent=service.profiles().await.unwrap()[0].id.clone();
        let task=service.create_task(project.id,"Queue".into(),agent).await.unwrap();
        let first=service.edit_followups(task.id,0,FollowupEdit::Add("日本語\nKeep formatting  ".into())).await.unwrap();
        assert!(service.edit_followups(task.id,0,FollowupEdit::Add("Stale".into())).await.is_err());
        let second=service.edit_followups(task.id,first.revision,FollowupEdit::Add("Next".into())).await.unwrap();
        let id=second.items[1].id.clone();
        let ordered=service.edit_followups(task.id,second.revision,FollowupEdit::Move {id,up:true}).await.unwrap();
        assert_eq!(ordered.items[0].text,"Next");assert_eq!(ordered.items[1].text,"日本語\nKeep formatting  ");
        assert!(service.session(task.thread_id).await.unwrap().is_none());
        assert!(service.thread(task.thread_id).await.unwrap().messages.is_empty());
        assert!(service.edit_followups(task.id,ordered.revision,FollowupEdit::Add("x".repeat(65537))).await.is_err());
    }
}
