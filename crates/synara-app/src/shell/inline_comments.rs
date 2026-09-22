//! File reviews share editor, filesystem and draft owners. No prompt is sent here.
use super::*;
use crate::ui::{self, Glyph, palette};
use std::ops::Range;

pub(super) struct InlineState {
    task: Option<TaskId>,
    epoch: u64,
    busy: bool,
    loading: bool,
    open: bool,
    saved: InlineComments,
    capture: Option<(Document, Range<usize>)>,
    input: Entity<TextEntry>,
    error: Option<String>,
    _subscription: Subscription,
}
pub(super) enum Outcome {
    Loaded(InlineComments),
    Saved(InlineComments, Option<String>),
    Validated {
        revision: u64,
        selection: u64,
        draft_version: u64,
        expected: String,
        draft: String,
        documents: Vec<Document>,
    },
}
pub(super) struct Reply {
    task: TaskId,
    epoch: u64,
    result: Result<Outcome, String>,
}
impl InlineState {
    pub fn new(cx: &mut Context<Shell>) -> Self {
        let input = cx.new(|cx| {
            TextEntry::new(
                "Comment on the reviewed file selection",
                EntryMode::Editor,
                72.,
                cx,
            )
        });
        let subscription = cx.subscribe(&input, |_, _, _, cx| cx.notify());
        Self {
            task: None,
            epoch: 0,
            busy: false,
            loading: false,
            open: false,
            saved: InlineComments::default(),
            capture: None,
            input,
            error: None,
            _subscription: subscription,
        }
    }
}
impl Shell {
    pub(super) fn inline_navigation_blocked(&mut self, cx: &mut Context<Self>) -> bool {
        let state = &self.inline_comments;
        // The task/epoch-fenced load does not own edits or a write. Allow the
        // selecting navigation to finish while it runs. Validation still blocks.
        if state.busy && !state.loading
            || state.capture.is_some()
                && (!state.input.read(cx).text().is_empty() || state.input.read(cx).is_composing())
        {
            self.notice=Some("Save or discard the inline comment and finish validation before leaving this task.".into());
            cx.notify();
            return true;
        }
        false
    }
    pub(super) fn load_inline_comments(&mut self, task: TaskId, cx: &mut Context<Self>) {
        let state = &mut self.inline_comments;
        state.task = Some(task);
        state.epoch = state.epoch.wrapping_add(1);
        state.busy = true;
        state.loading = true;
        state.saved = InlineComments::default();
        state.capture = None;
        state.open = false;
        state.error = None;
        state.input.update(cx, |input, cx| input.clear(cx));
        let epoch = state.epoch;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::InlineComments(Box::new(Reply {
                task,
                epoch,
                result: workspace
                    .inline_comments(task)
                    .await
                    .map(Outcome::Loaded)
                    .map_err(|e| e.to_string()),
            })))
        });
    }
    pub(super) fn open_inline_comment(&mut self, cx: &mut Context<Self>) {
        if self.inline_comments.busy
            || self.close != CloseState::Open
            || self.loading_task.is_some()
        {
            return;
        }
        self.inline_comments.open = true;
        if self.inline_comments.capture.is_some()
            && !self.inline_comments.input.read(cx).text().is_empty()
        {
            cx.notify();
            return;
        }
        if self.editor.read(cx).is_composing() || self.active_document_dirty(cx) {
            self.inline_comments.error =
                Some("Save the file or discard unsaved edits before reviewing its version.".into());
            cx.notify();
            return;
        }
        if self
            .task()
            .is_none_or(|t| Some(t.project_id) != self.project || t.state == TaskState::Archived)
        {
            return;
        }
        if let Some(document) = self.document.clone() {
            self.inline_comments.capture = Some((document, self.editor.read(cx).selection_range()));
            self.inline_comments.error = None;
        }
        cx.notify();
    }
    fn edit_inline_comment(
        &mut self,
        edit: InlineCommentEdit,
        original: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let Some(task) = self
            .selected
            .filter(|t| Some(*t) == self.inline_comments.task)
        else {
            return;
        };
        if self.inline_comments.busy
            || self.loading_task.is_some()
            || self.close != CloseState::Open
        {
            return;
        }
        let state = &mut self.inline_comments;
        state.busy = true;
        state.error = None;
        let epoch = state.epoch;
        let revision = state.saved.revision;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::InlineComments(Box::new(Reply {
                task,
                epoch,
                result: workspace
                    .edit_inline_comments(task, revision, edit)
                    .await
                    .map(|v| Outcome::Saved(v, original))
                    .map_err(|e| e.to_string()),
            })))
        });
        cx.notify();
    }
    fn save_inline_comment(&mut self, cx: &mut Context<Self>) {
        if self.inline_comments.input.read(cx).is_composing() {
            return;
        }
        let Some((document, range)) = &self.inline_comments.capture else {
            return;
        };
        let original = self.inline_comments.input.read(cx).text().to_owned();
        match InlineComment::from_selection(document, range.clone(), original.clone()) {
            Ok(comment) => {
                self.edit_inline_comment(InlineCommentEdit::Add(comment), Some(original), cx)
            }
            Err(e) => {
                self.inline_comments.error = Some(e.to_string());
                cx.notify();
            }
        }
    }
    fn append_inline_comments(&mut self, cx: &mut Context<Self>) {
        let Some(task) = self
            .selected
            .filter(|t| Some(*t) == self.inline_comments.task)
        else {
            return;
        };
        if self.inline_comments.busy
            || self.loading_task.is_some()
            || self.close != CloseState::Open
            || self.composer.read(cx).is_composing()
            || self.draft_state.loading.contains(&task)
            || self.task().is_none_or(|t| {
                Some(t.project_id) != self.project || t.state == TaskState::Archived
            })
        {
            return;
        }
        let Some(target) = self.workspace_target() else {
            return;
        };
        let expected = self.composer.read(cx).text().to_owned();
        let queue = self.inline_comments.saved.clone();
        let draft = match queue.draft(&expected) {
            Ok(v) => v,
            Err(e) => {
                self.inline_comments.error = Some(e.to_string());
                cx.notify();
                return;
            }
        };
        let draft_version = self.draft_state.version(task);
        let selection = self.selection_revision;
        self.inline_comments.busy = true;
        self.inline_comments.error = None;
        let epoch = self.inline_comments.epoch;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            let result=async {
                if workspace.inline_comments(task).await?.revision!=queue.revision {return Err(WorkspaceError::Invalid("Comments changed. Reload and review again.".into()));}
                let mut documents=Vec::new();
                for comment in &queue.items {
                    let document=match &target {
                        WorkspaceTarget::Local {root}=>open_document(root.clone(),comment.path.clone()).await?,
                        WorkspaceTarget::Ssh {workspace:remote,root}=>{
                            let filesystem=remote_filesystem(workspace.clone(),remote.clone(),root.clone()).await?;
                            open_remote_document(filesystem,comment.path.clone()).await?
                        }
                    };
                    comment.check_document(&document)?;
                    documents.push(document);
                }
                Ok(Outcome::Validated {revision:queue.revision,selection,draft_version,expected,draft,documents})
            }.await;
            Ok(Update::InlineComments(Box::new(Reply {task,epoch,result:result.map_err(|e:WorkspaceError|format!("{e} No comments were added. Deleted or renamed files must be reviewed again."))})))
        });
        cx.notify();
    }
    pub(super) fn inline_comments_reply(&mut self, reply: Reply, cx: &mut Context<Self>) {
        if Some(reply.task) != self.selected
            || Some(reply.task) != self.inline_comments.task
            || reply.epoch != self.inline_comments.epoch
        {
            return;
        }
        self.inline_comments.busy = false;
        self.inline_comments.loading = false;
        match reply.result {
            Err(e) => self.inline_comments.error = Some(e),
            Ok(Outcome::Loaded(value)) => self.inline_comments.saved = value,
            Ok(Outcome::Saved(value, original)) => {
                self.inline_comments.saved = value;
                if original.as_deref() == Some(self.inline_comments.input.read(cx).text()) {
                    self.inline_comments.capture = None;
                    self.inline_comments
                        .input
                        .update(cx, |input, cx| input.clear(cx));
                }
            }
            Ok(Outcome::Validated {
                revision,
                selection,
                draft_version,
                expected,
                draft,
                documents,
            }) => {
                let buffers_match = documents.iter().all(|doc| {
                    self.editors.tabs.iter().all(|tab| {
                        tab.document.path != doc.path
                            || tab.input.read(cx).text() == doc.snapshot.text
                    })
                });
                if revision != self.inline_comments.saved.revision
                    || selection != self.selection_revision
                    || self.draft_state.version(reply.task) != draft_version
                    || self.composer.read(cx).text() != expected
                    || self.composer.read(cx).is_composing()
                    || self.loading_task.is_some()
                    || !buffers_match
                    || self.close != CloseState::Open
                {
                    self.inline_comments.error = Some(
                        "Task, draft or editor changed during validation. Nothing was added."
                            .into(),
                    );
                } else {
                    self.composer
                        .update(cx, |input, cx| input.set_text(draft, cx));
                    self.remember_draft(cx);
                    self.inline_comments.open = false;
                    self.show_conversation(cx);
                    self.focus_composer = true;
                    self.notice=Some("Reviewed comments added without sending. Saved comments remain until removed. Files can still change after review.".into());
                }
            }
        }
        cx.notify();
    }
    pub(super) fn inline_comments_panel(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let state = &self.inline_comments;
        let mut panel = div().flex().flex_col().gap_1().flex_shrink_0().child(
            ui::action(
                "inline-review-open",
                "Comment on selected lines",
                Some(Glyph::Notebook),
                false,
                cx.listener(|this, _: &(), _, cx| this.open_inline_comment(cx)),
            )
            .relative()
            .child(ui::layout_probe("inline-review-open"))
            .text_size(px(11.)),
        );
        if !state.open {
            return panel.into_any_element();
        }
        panel=panel.child(div().text_size(px(11.)).text_color(rgb(palette().muted))
            .child(format!("{} / 12 saved comments for this task. Saved files only. Append rechecks each version.",state.saved.items.len())));
        if let Some((document, range)) = &state.capture {
            let lines = InlineComment::from_selection(document, range.clone(), "preview".into())
                .map(|c| format!("{}:{}-{}", c.path.display(), c.first_line, c.last_line))
                .unwrap_or_else(|e| e.to_string());
            panel = panel
                .child(div().text_size(px(11.)).child(lines))
                .child(
                    div()
                        .h(px(72.))
                        .flex_shrink_0()
                        .relative()
                        .child(ui::layout_probe("inline-review-input"))
                        .child(state.input.clone()),
                )
                .child(
                    div()
                        .flex()
                        .gap_1()
                        .child(
                            ui::action(
                                "inline-review-save",
                                "Save comment",
                                None,
                                false,
                                cx.listener(|this, _: &(), _, cx| this.save_inline_comment(cx)),
                            )
                            .relative()
                            .child(ui::layout_probe("inline-review-save")),
                        )
                        .child(ui::action(
                            "inline-review-discard",
                            "Discard edit",
                            None,
                            false,
                            cx.listener(|this, _: &(), _, cx| {
                                if !this.inline_comments.busy
                                    && !this.inline_comments.input.read(cx).is_composing()
                                {
                                    this.inline_comments.capture = None;
                                    this.inline_comments.input.update(cx, |i, cx| i.clear(cx));
                                    cx.notify();
                                }
                            }),
                        )),
                );
        }
        panel = panel
            .child(
                div()
                    .id("inline-review-list")
                    .max_h(px(115.))
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .children(state.saved.items.iter().enumerate().map(|(index, item)| {
                        let id = item.id.clone();
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(div().flex_1().min_w_0().text_size(px(11.)).child(format!(
                                "{}:{}-{} | {}",
                                item.path.display(),
                                item.first_line,
                                item.last_line,
                                item.text
                            )))
                            .child(ui::action(
                                ("inline-review-remove", index),
                                "Remove",
                                None,
                                false,
                                cx.listener(move |this, _: &(), _, cx| {
                                    this.edit_inline_comment(
                                        InlineCommentEdit::Remove(id.clone()),
                                        None,
                                        cx,
                                    )
                                }),
                            ))
                    })),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_1()
                    .child(
                        ui::action(
                            "inline-review-append",
                            if state.busy {
                                "Checking..."
                            } else {
                                "Add comments to draft"
                            },
                            None,
                            false,
                            cx.listener(|this, _: &(), _, cx| this.append_inline_comments(cx)),
                        )
                        .relative()
                        .child(ui::layout_probe("inline-review-append")),
                    )
                    .child(
                        ui::action(
                            "inline-review-clear",
                            "Clear saved",
                            None,
                            false,
                            cx.listener(|this, _: &(), _, cx| {
                                this.edit_inline_comment(InlineCommentEdit::Clear, None, cx)
                            }),
                        )
                        .relative()
                        .child(ui::layout_probe("inline-review-clear")),
                    )
                    .child(ui::action(
                        "inline-review-close",
                        "Close",
                        None,
                        false,
                        cx.listener(|this, _: &(), _, cx| {
                            if !this.inline_navigation_blocked(cx) {
                                this.inline_comments.open = false;
                                cx.notify();
                            }
                        }),
                    )),
            )
            .children(state.error.as_ref().map(|error| {
                div()
                    .relative()
                    .child(ui::layout_probe("inline-review-error"))
                    .text_size(px(11.))
                    .text_color(rgb(palette().error))
                    .child(error.clone())
            }));
        panel.into_any_element()
    }
}
