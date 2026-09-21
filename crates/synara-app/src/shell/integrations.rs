//! Native integration controls, not a provider-specific extension runtime.
use super::*;
use crate::ui::{self, palette};
use synara_runtime::SecretReference;
mod view;

pub(super) enum Reply {
    Loaded(IntegrationSettings),
    Changed(IntegrationSettings, String),
    Reviewed(SkillReview, Option<String>),
    Probed(String, u64, Result<McpProbeReport,String>),
    Insert(TaskId, u64, String),
    Disconnected,
    Failed(String),
}
#[derive(Clone)]
enum Confirm { Skill(String), Mcp(String), Disconnect(TaskId) }
struct McpForm {
    config: ManagedMcp,
    name: Entity<TextEntry>,
    endpoint: Entity<TextEntry>,
    service: Entity<TextEntry>,
    account: Entity<TextEntry>,
}
pub(super) struct IntegrationState {
    value: Option<IntegrationSettings>,
    pub busy: bool,
    query: Entity<TextEntry>,
    skill_path: Entity<TextEntry>,
    filter: Option<bool>,
    error: Option<String>,
    notice: Option<String>,
    review: Option<(SkillReview,Option<String>)>,
    preview: Option<InstalledSkill>,
    form: Option<McpForm>,
    confirm: Option<Confirm>,
    reports: HashMap<String,(u64,Result<McpProbeReport,String>)>,
    expanded: Option<String>,
    _subscription: Subscription,
}
impl IntegrationState {
    pub fn new(cx: &mut Context<Shell>) -> Self {
        let query = cx.new(|cx|TextEntry::new("Search name, description, origin or agent",EntryMode::SingleLine,32.,cx));
        let skill_path = cx.new(|cx|TextEntry::new("Absolute path to a local .md file",EntryMode::SingleLine,32.,cx));
        let subscription = cx.subscribe(&query,|_,_,_,cx|cx.notify());
        Self {value:None,busy:false,query,skill_path,filter:None,error:None,notice:None,review:None,preview:None,form:None,confirm:None,reports:HashMap::new(),expanded:None,_subscription:subscription}
    }
    pub fn loaded(&self) -> bool {self.value.is_some()}
    pub fn pending(&self) -> bool {self.busy || self.form.is_some() || self.review.is_some() || self.confirm.is_some()}
    fn matches(&self, enabled: bool, fields: &[&str], cx: &Context<Shell>) -> bool {
        let query = self.query.read(cx).text().trim().to_lowercase();
        self.filter.is_none_or(|filter|filter==enabled) && fields.iter().any(|field|field.to_lowercase().contains(&query))
    }
}
impl Shell {
    fn integration_job(&mut self, work: impl std::future::Future<Output=Result<Reply,String>> + Send + 'static, cx: &mut Context<Self>) {
        if self.integrations.busy || self.close!=CloseState::Open {return;}
        self.integrations.busy = true;
        self.integrations.error = None;
        self.integrations.notice = None;
        let sender = self.sender.clone();
        self.runtime.spawn(async move {
            let reply = work.await.unwrap_or_else(Reply::Failed);
            let _ = sender.send(Update::Integrations(Box::new(reply))).await;
        });
        cx.notify();
    }
    pub(super) fn load_integrations(&mut self, cx: &mut Context<Self>) {
        if self.integrations.busy {return;}
        self.integrations.reports.clear();
        let workspace = self.controller.workspace.clone();
        self.integration_job(async move {workspace.integrations().await.map(Reply::Loaded).map_err(|e|e.to_string())},cx);
    }
    pub(super) fn integration_reply(&mut self, reply: Reply, cx: &mut Context<Self>) {
        self.integrations.busy = false;
        match reply {
            Reply::Loaded(value) => {self.integrations.value=Some(value);self.integrations.error=None;},
            Reply::Changed(value,message) => {
                self.integrations.value=Some(value);
                self.integrations.notice=Some(message);
                self.integrations.review=None;self.integrations.preview=None;self.integrations.form=None;self.integrations.confirm=None;
                self.integrations.reports.clear();
                self.poll();
            },
            Reply::Reviewed(review,replace) => {self.integrations.preview=None;self.integrations.review=Some((review,replace));},
            Reply::Probed(id,revision,result) => {
                if self.integrations.value.as_ref().is_some_and(|value|value.revision==revision) {
                    self.integrations.reports.insert(id.clone(),(revision,result));
                    self.integrations.expanded=Some(id);
                }
            },
            Reply::Insert(task,generation,text) => {
                if self.selected!=Some(task) || self.selection_revision!=generation || self.loading_task.is_some() || self.composer.read(cx).is_composing() {
                    self.integrations.error=Some("The conversation changed. Nothing was inserted. Select the intended conversation and try again.".into());
                } else {
                    let draft=self.composer.read(cx).text();
                    let combined=format!("{draft}{}{text}",if draft.is_empty(){""}else{"\n\n"});
                    if combined.len()>1024*1024 {
                        self.integrations.error=Some("The combined draft exceeds 1 MiB. Nothing was inserted.".into());
                    } else {
                        self.composer.update(cx,|entry,cx|entry.set_text(combined,cx));
                        self.remember_draft(cx);
                        self.set_panel(Panel::Conversation,cx);
                        self.notice=Some("Skill document inserted into the draft. It has not been sent to an agent.".into());
                    }
                }
            },
            Reply::Disconnected => {self.integrations.confirm=None;self.integrations.notice=Some("The shared agent process was disconnected. No task was restarted. Retry the configuration change.".into());self.poll();},
            Reply::Failed(error) => self.integrations.error=Some(error),
        }
        cx.notify();
    }
    fn choose_skill(&mut self, replace: Option<String>, cx: &mut Context<Self>) {
        if self.integrations.busy || self.close!=CloseState::Open {return;}
        self.integrations.busy=true;
        let picker=cx.prompt_for_paths(gpui::PathPromptOptions {files:true,directories:false,multiple:false,prompt:Some("Review a Markdown skill document".into())});
        cx.spawn(async move |view,cx| {
            let result=picker.await;
            let _=view.update(cx,|this,cx| {
                this.integrations.busy=false;
                match result {
                    Ok(Ok(Some(paths))) if paths.len()==1 => {
                        let path=paths[0].clone();let workspace=this.controller.workspace.clone();
                        this.integration_job(async move {workspace.review_skill(path).await.map(|review|Reply::Reviewed(review,replace)).map_err(|e|e.to_string())},cx);
                    },
                    Ok(Ok(None)) => {},
                    _ => this.integrations.error=Some("The native skill file picker was unavailable or did not return one file.".into()),
                }
                cx.notify();
            });
        }).detach();
        cx.notify();
    }
    fn review_skill_path(&mut self, cx: &mut Context<Self>) {
        if self.integrations.skill_path.read(cx).is_composing() {return;}
        let path=std::path::PathBuf::from(self.integrations.skill_path.read(cx).text().trim());
        let workspace=self.controller.workspace.clone();
        self.integration_job(async move {workspace.review_skill(path).await.map(|review|Reply::Reviewed(review,None)).map_err(|e|e.to_string())},cx);
    }
    fn approve_skill(&mut self, cx: &mut Context<Self>) {
        let Some(value)=&self.integrations.value else {return};
        let Some((review,replace))=self.integrations.review.clone() else {return};
        let revision=value.revision;let workspace=self.controller.workspace.clone();
        self.integration_job(async move {workspace.install_skill(revision,review,replace).await.map(|value|Reply::Changed(value,"Skill document installed disabled. Enable it separately for explicit draft insertion. No agent files or packages were changed.".into())).map_err(|e|e.to_string())},cx);
    }
    fn change_skill(&mut self, edit: SkillEdit, cx: &mut Context<Self>) {
        let Some(value)=&self.integrations.value else {return};
        let revision=value.revision;let workspace=self.controller.workspace.clone();
        self.integration_job(async move {workspace.edit_skill(revision,edit).await.map(|value|Reply::Changed(value,"Synara's skill library updated. Previously inserted drafts and provider-owned skills are unchanged.".into())).map_err(|e|e.to_string())},cx);
    }
    fn insert_skill(&mut self, id: String, cx: &mut Context<Self>) {
        let Some(task)=self.selected else {return};
        let Some(value)=&self.integrations.value else {return};
        let revision=value.revision;let generation=self.selection_revision;
        let workspace=self.controller.workspace.clone();
        self.integration_job(async move {
            let value=workspace.integrations().await.map_err(|e|e.to_string())?;
            if value.revision!=revision {return Err("The skill library changed. Reload before inserting.".into());}
            let skill=value.skills.into_iter().find(|skill|skill.id==id && skill.enabled).ok_or("This skill is disabled or no longer installed.")?;
            Ok(Reply::Insert(task,generation,format!("## Skill: {}\n\n{}",skill.title,skill.markdown)))
        },cx);
    }
    fn edit_mcp_form(&mut self, config: Option<ManagedMcp>, cx: &mut Context<Self>) {
        if self.integrations.busy || self.close!=CloseState::Open {return;}
        let config=match config {
            Some(config)=>config,
            None=>{let Some(task)=self.task() else {self.integrations.error=Some("Select a task and agent before adding an MCP connection.".into());cx.notify();return;};
                ManagedMcp::new(task.id,task.agent_id.clone())},
        };
        let entry=|placeholder:&str,value:String,cx:&mut Context<Self>|{
            let entry=cx.new(|cx|TextEntry::new(placeholder,EntryMode::SingleLine,32.,cx));
            entry.update(cx,|entry,cx|entry.set_text(value,cx));entry
        };
        let name=entry("Connection name",config.name.clone(),cx);
        let endpoint=entry("https://server.example/mcp",config.endpoint.clone(),cx);
        let service=entry("Credential service (optional)",config.bearer.as_ref().map(|r|r.service.clone()).unwrap_or_default(),cx);
        let account=entry("Credential account (optional)",config.bearer.as_ref().map(|r|r.account.clone()).unwrap_or_default(),cx);
        self.integrations.form=Some(McpForm {config,name,endpoint,service,account});
        self.integrations.error=None;
        cx.notify();
    }
    fn save_mcp_form(&mut self,cx:&mut Context<Self>) {
        let Some(form)=&self.integrations.form else {return};
        if [ &form.name,&form.endpoint,&form.service,&form.account ].iter().any(|entry|entry.read(cx).is_composing()) {return;}
        let mut config=form.config.clone();
        config.name=form.name.read(cx).text().trim().into();
        config.endpoint=form.endpoint.read(cx).text().trim().into();
        let service=form.service.read(cx).text().trim().to_owned();
        let account=form.account.read(cx).text().trim().to_owned();
        config.bearer=if service.is_empty() && account.is_empty(){None}else{Some(SecretReference {service,account})};
        if let Err(error)=config.validate() {self.integrations.error=Some(error.to_string());cx.notify();return;}
        self.change_mcp(McpEdit::Save(config),cx);
    }
    fn change_mcp(&mut self,edit:McpEdit,cx:&mut Context<Self>) {
        let Some(value)=&self.integrations.value else {return};
        let revision=value.revision;let controller=self.controller.clone();
        let message=match &edit {
            McpEdit::Save(_)=>"Configuration saved disabled. Enable it separately for this task and agent. No test or agent connection was started.",
            McpEdit::SetEnabled{enabled:true,..}=>"Enabled for this task and agent on the next session. This is not connection evidence. No agent was started.",
            McpEdit::SetEnabled{enabled:false,..}=>"Disabled for future sessions. The previous session was retired where necessary. No agent was started.",
            McpEdit::Remove(_)=>"Connection removed from Synara. Revoke external credentials at their provider. The credential-store entry was not deleted.",
        }.to_owned();
        self.integration_job(async move {controller.configure_mcp(revision,edit).await.map(|value|Reply::Changed(value,message)).map_err(|e|e.to_string())},cx);
    }
    fn test_mcp_row(&mut self,id:String,cx:&mut Context<Self>) {
        let Some(value)=&self.integrations.value else {return};
        let revision=value.revision;let controller=self.controller.clone();
        self.integration_job(async move {let result=controller.test_mcp(revision,id.clone()).await.map_err(|e|e.to_string());Ok(Reply::Probed(id,revision,result))},cx);
    }
    fn confirm_integration(&mut self,cx:&mut Context<Self>) {
        let Some(confirm)=self.integrations.confirm.clone() else {return};
        match confirm {
            Confirm::Skill(id)=>self.change_skill(SkillEdit::Remove(id),cx),
            Confirm::Mcp(id)=>self.change_mcp(McpEdit::Remove(id),cx),
            Confirm::Disconnect(task)=>{
                let controller=self.controller.clone();
                self.integration_job(async move {controller.disconnect_mcp_agent(task).await.map(|_|Reply::Disconnected).map_err(|e|e.to_string())},cx);
            },
        }
    }
}
