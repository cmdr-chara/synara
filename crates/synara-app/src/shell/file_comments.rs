//! Explicit, task-scoped file review. Drafts are transient until attached to the
//! normal saved composer. Verification reads through the existing filesystem.
use super::*;
use crate::ui::{self, Glyph};

pub(super) struct Reply {
    epoch: u64,
    input: String,
    result: Result<String, String>,
}
struct CommentDraft {
    task: Task,
    selection: u64,
    anchor: FileReviewAnchor,
    input: Entity<TextEntry>,
    _subscription: Subscription,
    busy: bool,
    discard: bool,
    error: Option<String>,
}
#[derive(Default)]
pub(super) struct CommentState {
    draft: Option<CommentDraft>,
    epoch: u64,
}
impl CommentState {
    pub fn open(&self) -> bool {
        self.draft.is_some()
    }
}
fn same_owner(a: &Task, b: &Task) -> bool {
    a.id == b.id
        && a.thread_id == b.thread_id
        && a.project_id == b.project_id
        && a.working_directory == b.working_directory
        && b.state != TaskState::Archived
}
async fn check_owner(
    service: &WorkspaceService,
    expected: &Task,
    target: &WorkspaceTarget,
) -> Result<(), String> {
    let task = service.task(expected.id).await.map_err(|e| e.to_string())?;
    if !same_owner(expected, &task) || task.working_directory != *target.root() {
        return Err("The target task or directory changed. Your comment was kept.".into());
    }
    let workspace = service
        .workspace_for_task(&task)
        .await
        .map_err(|e| e.to_string())?;
    let matches = match target {
        WorkspaceTarget::Local { .. } => {
            matches!(workspace.location, WorkspaceLocation::Local { .. })
        }
        WorkspaceTarget::Ssh {
            workspace: expected,
            ..
        } => expected.id == workspace.id && expected.location == workspace.location,
    };
    if !matches {
        return Err("The target workspace changed. Your comment was kept.".into());
    }
    Ok(())
}
impl Shell {
    fn file_comment_current(&self, draft: &CommentDraft, cx: &App) -> bool {
        self.selected == Some(draft.task.id)
            && self.project == Some(draft.task.project_id)
            && self.selection_revision == draft.selection
            && self
                .task()
                .is_some_and(|task| same_owner(&draft.task, task))
            && self.loading_task.is_none()
            && !self.draft_state.loading.contains(&draft.task.id)
            && self.close == CloseState::Open
            && !self.composer.read(cx).is_composing()
            && !self.editor.read(cx).is_composing()
            && !self.active_document_dirty(cx)
            && self.document.as_ref().is_some_and(|doc| {
                doc.path == draft.anchor.path && doc.snapshot.version == draft.anchor.version
            })
    }
    pub(super) fn open_file_comment(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.file_comments.open() {
            return;
        }
        let (Some(task), Some(document)) = (self.task().cloned(), self.document.clone()) else {
            return;
        };
        if self.active_document_dirty(cx)
            || self.editor.read(cx).is_composing()
            || self.saving
            || self.loading_task.is_some()
            || self.draft_state.loading.contains(&task.id)
            || task.state == TaskState::Archived
            || self.close != CloseState::Open
            || self.workflows.open()
            || self.explorer.modal_open()
        {
            self.notice =
                Some("Select a writable chat and save the file before reviewing its lines.".into());
            cx.notify();
            return;
        }
        let (start, end) = self.editor.read(cx).selected_line_range();
        let anchor = match FileReviewAnchor::capture(&document, start, end) {
            Ok(anchor) => anchor,
            Err(error) => {
                self.error = Some(error);
                cx.notify();
                return;
            }
        };
        let input = cx.new(|cx| {
            TextEntry::new(
                "Comment on this exact file range...",
                EntryMode::Editor,
                110.,
                cx,
            )
        });
        let subscription = cx.subscribe(&input, |_, _, _, cx| cx.notify());
        self.file_comments.epoch = self.file_comments.epoch.wrapping_add(1);
        self.file_comments.draft = Some(CommentDraft {
            task,
            selection: self.selection_revision,
            anchor,
            input: input.clone(),
            _subscription: subscription,
            busy: false,
            discard: false,
            error: None,
        });
        self.editors.focus_editor = false;
        self.focus_composer = false;
        window.focus(&input.read(cx).focus_handle(cx), cx);
        cx.notify();
    }
    fn attach_file_comment(&mut self, cx: &mut Context<Self>) {
        let Some(draft) = &self.file_comments.draft else {
            return;
        };
        if draft.busy || draft.input.read(cx).is_composing() {
            return;
        }
        if !self.file_comment_current(draft, cx) {
            self.file_comments.draft.as_mut().unwrap().error =
                Some("Return to the original task and saved file. Nothing was attached.".into());
            cx.notify();
            return;
        }
        let input = draft.input.read(cx).text().to_owned();
        let instruction = match draft.anchor.instruction(&input) {
            Ok(value) => value,
            Err(error) => {
                self.file_comments.draft.as_mut().unwrap().error = Some(error);
                cx.notify();
                return;
            }
        };
        let Some(target) = self.workspace_target() else {
            return;
        };
        let task = draft.task.clone();
        let anchor = draft.anchor.clone();
        let service = self.controller.workspace.clone();
        let epoch = self.file_comments.epoch;
        let draft = self.file_comments.draft.as_mut().unwrap();
        draft.busy = true;
        draft.error = None;
        self.job(async move {
            let result = async {
                check_owner(&service, &task, &target).await?;
                let current = match target.clone() {
                    WorkspaceTarget::Local { root } => {
                        open_document(root, anchor.path.clone()).await
                    }
                    WorkspaceTarget::Ssh { workspace, root } => {
                        let fs = remote_filesystem(service.clone(), workspace, root)
                            .await
                            .map_err(|e| e.to_string())?;
                        open_remote_document(fs, anchor.path.clone()).await
                    }
                }
                .map_err(|e| {
                    format!("Cannot re-read the reviewed file: {e}. Nothing was attached.")
                })?;
                anchor.verify(&current)?;
                check_owner(&service, &task, &target).await?;
                Ok(instruction)
            }
            .await;
            Ok(Update::FileComment(Box::new(Reply {
                epoch,
                input,
                result,
            })))
        });
        cx.notify();
    }
    pub(super) fn file_comment_reply(&mut self, reply: Reply, cx: &mut Context<Self>) {
        if reply.epoch != self.file_comments.epoch {
            return;
        }
        let Some(draft) = self.file_comments.draft.as_ref() else {
            return;
        };
        if !draft.busy {
            return;
        }
        let result = (|| -> Result<String, String> {
            if !self.file_comment_current(draft, cx)
                || draft.input.read(cx).is_composing()
                || draft.input.read(cx).text() != reply.input
            {
                return Err(
                    "The task, file or comment changed while checking. Your text was kept.".into(),
                );
            }
            append_file_review(self.composer.read(cx).text(), &reply.result?)
        })();
        match result {
            Ok(combined) => {
                self.composer
                    .update(cx, |entry, cx| entry.set_text(combined, cx));
                self.remember_draft(cx);
                self.file_comments.epoch = self.file_comments.epoch.wrapping_add(1);
                self.file_comments.draft = None;
                self.set_panel(Panel::Conversation, cx);
                self.focus_composer = true;
                self.notice = Some("File comment attached to the saved, unsent draft. Its path, range and hash remain a frozen snapshot. Edit or remove it before Send as needed.".into());
            }
            Err(error) => {
                let draft = self.file_comments.draft.as_mut().unwrap();
                draft.busy = false;
                draft.error = Some(error);
            }
        }
        cx.notify();
    }
    pub(super) fn file_comment_panel(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(draft) = &self.file_comments.draft else {
            return div().into_any_element();
        };
        let a = &draft.anchor;
        div().id("file-comment-review").flex().flex_col().gap_1().p_2().border_1().border_color(rgb(ui::palette().border))
            .child(format!("{} · lines {}-{} · task {}", a.path.display(), a.start_line, a.end_line, draft.task.title))
            .child(div().text_xs().child(format!("SHA-256 {} · temporary until attached; never sends automatically", a.version.0)))
            .child(div().id("file-comment-excerpt").max_h(px(70.)).overflow_y_scroll().text_xs().child(a.excerpt.clone()))
            .child(div().h(px(110.)).relative().child(draft.input.clone()).child(ui::layout_probe("file-comment-input")))
            .children(draft.error.as_ref().map(|error| div().relative().child(error.clone()).child(ui::layout_probe("file-comment-error"))))
            .child(div().flex().flex_wrap().gap_2()
                .child(ui::action("file-comment-attach", if draft.busy { "Checking current file..." } else { "Recheck and attach to draft" }, Some(Glyph::Plus), false,
                    cx.listener(|this, _: &(), _, cx| this.attach_file_comment(cx))).relative().child(ui::layout_probe("file-comment-attach")))
                .child(ui::action("file-comment-discard", "Discard comment...", None, false,
                    cx.listener(|this, _: &(), _, cx| {
                        if let Some(draft) = &mut this.file_comments.draft && !draft.input.read(cx).is_composing() { draft.discard = true; cx.notify(); }
                    })).relative().child(ui::layout_probe("file-comment-discard"))))
            .children(draft.discard.then(|| div().flex().gap_2().child("Discard this comment and its edits? The normal chat draft stays unchanged.")
                .child(ui::action("file-comment-discard-confirm", "Discard", None, false,
                    cx.listener(|this, _: &(), _, cx| {
                        if this.file_comments.draft.as_ref().is_some_and(|d| !d.input.read(cx).is_composing()) {
                            this.file_comments.epoch = this.file_comments.epoch.wrapping_add(1); this.file_comments.draft = None;
                            this.editors.focus_editor = true; cx.notify();
                        }
                    })).relative().child(ui::layout_probe("file-comment-discard-confirm")))
                .child(ui::action("file-comment-keep", "Keep editing", None, false,
                    cx.listener(|this, _: &(), _, cx| { if let Some(d) = &mut this.file_comments.draft { d.discard = false; cx.notify(); } })))))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn file_comment_owner_does_not_follow_another_task_or_archive() {
        let a = Task {
            id: TaskId::new(),
            project_id: ProjectId::new(),
            title: "Review".into(),
            state: TaskState::Ready,
            thread_id: ThreadId::new(),
            agent_id: "generic-agent".into(),
            working_directory: PathBuf::from("/project"),
            updated_at_ms: 0,
            scope: Default::default(),
        };
        let mut b = a.clone();
        assert!(same_owner(&a, &b));
        b.state = TaskState::Archived;
        assert!(!same_owner(&a, &b));
        b = a.clone();
        b.working_directory = "/other".into();
        assert!(!same_owner(&a, &b));
        b = a.clone();
        b.thread_id = ThreadId::new();
        assert!(!same_owner(&a, &b));
        b = a.clone();
        b.id = TaskId::new();
        assert!(!same_owner(&a, &b));
        b = a.clone();
        b.project_id = ProjectId::new();
        assert!(!same_owner(&a, &b));
    }
}
