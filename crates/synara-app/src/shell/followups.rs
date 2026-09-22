//! Persistent manual follow-up drafts. This surface never sends or steers work.
use super::*;
use crate::ui::{self,Glyph,palette};
pub(super) enum Reply {
    Loaded(TaskId,Result<FollowupQueue,String>),
    Saved(TaskId,Option<(u64,String)>,Option<(String,String)>,Result<FollowupQueue,String>),
}
pub(super) struct FollowupState {
    task:Option<TaskId>,
    value:Option<FollowupQueue>,
    loading:bool,
    saving:bool,
    open:bool,
    input:Entity<TextEntry>,
    editing:Option<(String,String)>,
    error:Option<String>,
    _subscription:Subscription,
}
impl FollowupState {
    pub fn new(cx:&mut Context<Shell>)->Self {
        let input=cx.new(|cx|TextEntry::new("Edit follow-up text",EntryMode::Editor,120.,cx));
        let subscription=cx.subscribe(&input,|this,_,event,cx|{
            if matches!(event,EntryEvent::Save) {this.save_followup_edit(cx);}
            cx.notify();
        });
        Self {task:None,value:None,loading:false,saving:false,open:false,input,editing:None,error:None,_subscription:subscription}
    }
    pub fn pending(&self,cx:&App)->bool {
        self.saving || self.editing.as_ref().is_some_and(|(_,original)|self.input.read(cx).text()!=original || self.input.read(cx).is_composing())
    }
}
impl Shell {
    pub(super) fn followup_navigation_blocked(&mut self,cx:&mut Context<Self>)->bool {
        if self.recap.pending() {
            self.error=Some("Create or cancel the reviewed recap request before leaving this conversation.".into());cx.notify();return true;
        }
        if self.debug_workflow.pending(cx) {
            self.error=Some("Save or discard the Debug evidence edit before leaving this conversation.".into());cx.notify();return true;
        }
        if self.followups.pending(cx) {
            self.error=Some("Save or discard the follow-up editor before leaving this conversation.".into());cx.notify();true
        } else {false}
    }
    pub(super) fn load_followups(&mut self,task:TaskId) {
        self.followups.task=Some(task);self.followups.value=None;self.followups.loading=true;
        self.followups.editing=None;self.followups.open=false;self.followups.error=None;
        let workspace=self.controller.workspace.clone();
        self.job(async move{Ok(Update::Followups(Box::new(Reply::Loaded(task,workspace.followup_queue(task).await.map_err(|e|e.to_string())))))});
    }
    fn change_followup(&mut self,edit:FollowupEdit,clear:Option<(u64,String)>,editing:Option<(String,String)>,cx:&mut Context<Self>) {
        let Some(task)=self.selected.filter(|t|Some(*t)==self.followups.task) else{return};
        if self.followups.saving || self.followups.loading || self.close!=CloseState::Open {return;}
        let Some(queue)=&self.followups.value else{return};let revision=queue.revision;
        self.followups.saving=true;self.followups.error=None;
        let workspace=self.controller.workspace.clone();
        self.job(async move{Ok(Update::Followups(Box::new(Reply::Saved(task,clear,editing,
            workspace.edit_followups(task,revision,edit).await.map_err(|e|e.to_string())))))});
        cx.notify();
    }
    fn queue_current_draft(&mut self,cx:&mut Context<Self>) {
        let Some(task)=self.selected else {return};
        if self.followups.editing.is_some() || self.composer.read(cx).is_composing() || self.loading_task.is_some() {return;}
        if self.attachment_send_blocked() || self.attachments_have_pending() {
            self.followups.error=Some("The follow-up queue stores text only. Send or remove attachments first.".into());self.followups.open=true;cx.notify();return;
        }
        let text=self.composer.read(cx).text().to_owned();
        if text.trim().is_empty() {return;}
        self.snapshot_draft(cx);
        self.change_followup(FollowupEdit::Add(text.clone()),Some((self.draft_state.version(task),text)),None,cx);
        self.followups.open=true;
    }
    fn save_followup_edit(&mut self,cx:&mut Context<Self>) {
        let Some((id,_))=self.followups.editing.clone() else{return};
        if self.followups.input.read(cx).is_composing() {return;}
        let text=self.followups.input.read(cx).text().to_owned();
        self.change_followup(FollowupEdit::Edit {id:id.clone(),text:text.clone()},None,Some((id,text)),cx);
    }
    fn append_followup(&mut self,origin:Option<TaskId>,text:&str,cx:&mut Context<Self>) {
        if self.selected != origin || self.loading_task.is_some() || self.composer.read(cx).is_composing() || self.followups.pending(cx) {return;}
        let current=self.composer.read(cx).text();let separator=if current.is_empty(){""}else{"\n\n"};
        if current.len().saturating_add(separator.len()).saturating_add(text.len())>1024*1024 {
            self.followups.error=Some("Combined draft exceeds 1 MiB. Nothing was changed.".into());
        } else {
            let value=format!("{current}{separator}{text}");
            self.composer.update(cx,|entry,cx|entry.set_text(value,cx));self.remember_draft(cx);self.focus_composer=true;
            self.notice=Some("Follow-up appended without sending. The queued copy remains until you remove it.".into());
        }
        cx.notify();
    }
    pub(super) fn followup_reply(&mut self,reply:Reply,cx:&mut Context<Self>) {
        let (task,result,clear,edited)=match reply {
            Reply::Loaded(task,result)=>{if self.followups.task==Some(task){self.followups.loading=false;} (task,result,None,None)},
            Reply::Saved(task,clear,edited,result)=>{self.followups.saving=false;(task,result,clear,edited)},
        };
        if self.followups.task!=Some(task) {return;}
        match result {
            Ok(value)=>{
                if self.followups.value.as_ref().is_none_or(|old|old.revision<=value.revision) {self.followups.value=Some(value);}
                if let Some((version,text))=clear && self.selected==Some(task) && self.draft_state.version(task)==version && self.composer.read(cx).text()==text {
                    self.composer.update(cx,|entry,cx|entry.clear(cx));self.remember_draft(cx);
                }
                if let Some((id,text))=edited && self.followups.editing.as_ref().is_some_and(|(current,_)|*current==id) {
                    if self.followups.input.read(cx).text()==text {self.followups.editing=None;}
                    else {self.followups.editing=Some((id,text));}
                }
                self.followups.error=None;
            }
            Err(error)=>self.followups.error=Some(error),
        }
        cx.notify();
    }
    pub(super) fn followup_toggle(&self,cx:&mut Context<Self>)->gpui::AnyElement {
        let count=self.followups.value.as_ref().map_or(0,|queue|queue.items.len());
        ui::action("open-followups",if count==0 {String::new()} else {count.to_string()},Some(Glyph::Clock),self.followups.open,
            cx.listener(|this,_:&(),_,cx|{this.followups.open=!this.followups.open;cx.notify();}))
            .aria_label(format!("Saved follow-up drafts: {count}")).text_size(px(11.)).into_any_element()
    }
    pub(super) fn followups_view(&self,cx:&mut Context<Self>)->gpui::AnyElement {
        let state=&self.followups;
        if !state.open && state.error.is_none() {return div().into_any_element();}
        let items=state.value.as_ref().map_or(&[][..],|q|q.items.as_slice());
        let mut root=div().id("composer-followups").min_w_0().flex().flex_col().gap_1()
            .child(div().flex().flex_wrap().items_center().gap_2()
                .child(ui::action("toggle-followups",format!("Follow-ups ({})",items.len()),Some(Glyph::Clock),state.open,
                    cx.listener(|this,_:&(),_,cx|{this.followups.open=!this.followups.open;cx.notify();})).text_size(px(12.)))
                .child(ui::action("queue-current-draft",if state.saving{"Saving..."}else{"Queue text draft"},Some(Glyph::Plus),false,
                    cx.listener(|this,_:&(),_,cx|this.queue_current_draft(cx))).text_size(px(12.))))
            .children(state.error.as_ref().map(|error|div().px_2().text_size(px(12.)).text_color(rgb(palette().error)).child(error.clone())));
        if state.open {
            root=root.child(div().px_2().text_size(px(11.)).text_color(rgb(palette().muted)).child("Saved text drafts, not automatic sends. Append one when ready, then use Send."))
                .children(state.loading.then(||div().px_2().text_size(px(12.)).child("Loading follow-ups...")));
            if let Some((_,original))=&state.editing {
                root=root.child(div().h(px(120.)).flex().flex_col().child(state.input.clone()))
                    .child(div().flex().gap_2()
                        .child(ui::action("save-followup-edit","Save edit",Some(Glyph::Check),false,cx.listener(|this,_:&(),_,cx|this.save_followup_edit(cx))))
                        .child(ui::action("discard-followup-edit","Discard edit",None,false,cx.listener(|this,_:&(),_,cx|{
                            if !this.followups.saving {this.followups.editing=None;cx.notify();}
                        })))
                        .child(div().text_size(px(11.)).text_color(rgb(palette().muted)).child(if state.input.read(cx).text()==original{"Saved text"}else{"Unsaved edit"})));
            } else {
                root=root.child(div().id("followup-list").max_h(px(180.)).overflow_y_scroll().flex().flex_col()
                    .children(items.iter().enumerate().map(|(index,item)|{
                        let origin=state.task;let edit=item.clone();let text=item.text.clone();let up=item.id.clone();let down=item.id.clone();let remove=item.id.clone();
                        div().id(("followup-row",index)).min_w_0().flex().flex_wrap().items_center().gap_1().border_b_1().border_color(rgb(palette().border))
                            .child(ui::action(("edit-followup",index),format!("{}. {}",index+1,item.text.split_whitespace().take(16).collect::<Vec<_>>().join(" ").chars().take(120).collect::<String>()),Some(Glyph::Compose),false,
                                cx.listener(move |this,_:&(),window,cx|{
                                    if this.selected == origin && !this.followups.saving {
                                        this.followups.input.update(cx,|entry,cx|entry.set_text(edit.text.clone(),cx));
                                        this.followups.editing=Some((edit.id.clone(),edit.text.clone()));
                                        this.focus_composer=false;window.focus(&this.followups.input.read(cx).focus_handle(cx),cx);cx.notify();
                                    }
                                })).flex_1().min_w(px(130.)).text_size(px(12.)))
                            .child(ui::action(("append-followup",index),"Append",None,false,cx.listener(move |this,_:&(),_,cx|this.append_followup(origin,&text,cx))).text_size(px(12.)))
                            .child(ui::chrome_button("move-followup-up","Move earlier",Glyph::Back,index==0||state.saving,cx.listener(move|this,_:&(),_,cx|this.change_followup(FollowupEdit::Move{id:up.clone(),up:true},None,None,cx))).id(("followup-up",index)).size(px(23.)))
                            .child(ui::chrome_button("move-followup-down","Move later",Glyph::Forward,index+1==items.len()||state.saving,cx.listener(move|this,_:&(),_,cx|this.change_followup(FollowupEdit::Move{id:down.clone(),up:false},None,None,cx))).id(("followup-down",index)).size(px(23.)))
                            .child(ui::chrome_button("remove-followup","Remove saved follow-up",Glyph::Close,state.saving,cx.listener(move|this,_:&(),_,cx|this.change_followup(FollowupEdit::Remove(remove.clone()),None,None,cx))).id(("followup-remove",index)).size(px(23.)))
                    }))
                    .children((items.is_empty()&&!state.loading).then(||div().p_2().text_size(px(12.)).text_color(rgb(palette().muted)).child("No follow-ups. Queue a draft while the current response is running."))));
            }
        }
        if state.error.is_some() || state.value.is_none() && !state.loading {
            root=root.child(ui::action("reload-followups","Reload follow-ups",Some(Glyph::Restore),false,cx.listener(|this,_:&(),_,cx|{
                if let Some(task)=this.selected && !this.followups.pending(cx) {this.load_followups(task);this.followups.open=true;cx.notify();}
            })).text_size(px(12.)));
        }
        root.into_any_element()
    }
}
