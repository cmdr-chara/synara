//! User-message revisions are additive. Sending never rewrites durable history and
//! branching never clones provider sessions, approvals, attachments or filesystem state.
use super::*;
use crate::ui::{self, Glyph, palette};

pub(super) enum Reply {
    Sent {
        task: TaskId,
        details: Option<SessionDetails>,
        error: Option<String>,
    },
    Branched {
        source: TaskId,
        selection_revision: u64,
        result: Result<(Task, Catalog), String>,
    },
}

struct RevisionDialog {
    source: RevisionSource,
    editor: Entity<TextEntry>,
    original: String,
    error: Option<String>,
}

pub(super) struct RevisionState {
    dialog: Option<RevisionDialog>,
    visible: bool,
    pending: bool,
    focus_pending: bool,
}

impl RevisionState {
    pub fn new() -> Self {
        Self {
            dialog: None,
            visible: false,
            pending: false,
            focus_pending: false,
        }
    }

    pub fn open(&self) -> bool {
        self.visible && self.dialog.is_some()
    }

    pub fn pending(&self) -> bool {
        self.pending
    }
}

impl Shell {
    pub(super) fn revision_navigation_blocked(&mut self, cx: &mut Context<Self>) -> bool {
        if self.goal_navigation_blocked(cx) {
            return true;
        }
        self.revision_navigation_except_goal(cx)
    }
    pub(super) fn revision_navigation_except_goal(&mut self, cx: &mut Context<Self>) -> bool {
        if self.checkpoint_navigation_blocked(cx) {
            return true;
        }
        if self.inline_navigation_blocked(cx) {
            return true;
        }
        if self.handoff.open() {
            self.error =
                Some("Create the continuation or discard its review before leaving.".into());
            cx.notify();
            return true;
        }
        if self.revisions.pending {
            self.error = Some(
                "Wait for the revision action to finish before leaving this conversation.".into(),
            );
            cx.notify();
            return true;
        }
        let Some(dialog) = self.revisions.dialog.as_ref() else {
            return false;
        };
        let dirty = dialog.editor.read(cx).is_composing()
            || dialog.editor.read(cx).text() != dialog.original;
        if dirty {
            self.error = Some(
                "Send, branch, move to the composer, or discard the edited message before leaving."
                    .into(),
            );
            cx.notify();
            true
        } else {
            self.revisions.dialog = None;
            self.revisions.visible = false;
            false
        }
    }

    pub(super) fn open_message_revision(
        &mut self,
        task: TaskId,
        anchor: MessageAnchor,
        original: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.revisions.pending
            || self.selected != Some(task)
            || self.loading_task.is_some()
            || self.busy.contains(&task)
            || anchor.role != Role::User
        {
            return;
        }
        if self.revisions.dialog.is_some() && self.revision_navigation_blocked(cx) {
            return;
        }
        let (Some(task_record), Some(thread)) = (self.task().cloned(), self.thread.as_ref()) else {
            return;
        };
        if task_record.id != task
            || !thread
                .messages
                .iter()
                .any(|message| anchor.matches(message) && message.text == original)
        {
            self.error = Some("That source message is no longer available.".into());
            cx.notify();
            return;
        }
        let editor = cx.new(|cx| TextEntry::new("Edit the message", EntryMode::Editor, 260., cx));
        editor.update(cx, |entry, cx| entry.set_text(original.clone(), cx));
        let source = RevisionSource {
            task: task_record,
            anchor,
            original: original.clone(),
            sequence: thread.last_sequence,
        };
        self.revisions.dialog = Some(RevisionDialog {
            source,
            editor: editor.clone(),
            original,
            error: None,
        });
        self.revisions.visible = true;
        self.revisions.focus_pending = false;
        self.focus_composer = false;
        window.focus(&editor.read(cx).focus_handle(cx), cx);
        cx.notify();
    }

    pub(super) fn message_revision_button(
        &self,
        message: &Message,
        index: usize,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let Some(task) = self.selected else {
            return div().into_any_element();
        };
        let anchor = MessageAnchor::from(message);
        let original = message.text.clone();
        ui::chrome_button(
            "edit-resend-message",
            "Edit and resend without rewriting history",
            Glyph::Compose,
            message.role != Role::User
                || self.busy.contains(&task)
                || self.loading_task.is_some()
                || self.revisions.pending,
            cx.listener(move |this, _: &(), window, cx| {
                let _ = index;
                this.open_message_revision(task, anchor.clone(), original.clone(), window, cx)
            }),
        )
        .size(px(24.))
        .into_any_element()
    }

    fn dismiss_revision(&mut self, cx: &mut Context<Self>) {
        if self.revisions.pending {
            return;
        }
        self.revisions.dialog = None;
        self.revisions.visible = false;
        self.revisions.focus_pending = false;
        self.focus_composer = true;
        cx.notify();
    }

    fn revision_to_composer(&mut self, cx: &mut Context<Self>) {
        let Some(dialog) = self.revisions.dialog.as_mut() else {
            return;
        };
        if self.revisions.pending || dialog.editor.read(cx).is_composing() {
            return;
        }
        let edited = dialog.editor.read(cx).text().to_owned();
        if edited.trim().is_empty() {
            dialog.error = Some("The revised message cannot be empty.".into());
            cx.notify();
            return;
        }
        if self.selected != Some(dialog.source.task.id) {
            dialog.error = Some(
                "Return to the source conversation before moving this revision to its composer."
                    .into(),
            );
            cx.notify();
            return;
        }
        if !self.composer.read(cx).text().is_empty() {
            dialog.error = Some(
                "The main composer already contains a draft. Clear or send it before replacing it."
                    .into(),
            );
            cx.notify();
            return;
        }
        self.composer
            .update(cx, |entry, cx| entry.set_text(edited, cx));
        self.remember_draft(cx);
        self.revisions.dialog = None;
        self.revisions.visible = false;
        self.focus_composer = true;
        self.notice = Some(
            "Revised text moved to the composer. The original transcript is unchanged.".into(),
        );
        cx.notify();
    }

    fn send_revision(&mut self, cx: &mut Context<Self>) {
        let Some(dialog) = self.revisions.dialog.as_mut() else {
            return;
        };
        let task = dialog.source.task.id;
        if self.revisions.pending
            || self.selected != Some(task)
            || self.busy.contains(&task)
            || self.connecting.contains(&task)
            || dialog.editor.read(cx).is_composing()
        {
            return;
        }
        let edited = dialog.editor.read(cx).text().to_owned();
        if edited.trim().is_empty() || edited.len() > 1024 * 1024 {
            dialog.error =
                Some("The revised message must contain text and fit within 1 MiB.".into());
            cx.notify();
            return;
        }
        let source = dialog.source.clone();
        dialog.error = None;
        self.revisions.pending = true;
        // Hide the editor while the turn runs so agent permission/input prompts are
        // never occluded by an edit modal. The entity remains retained for errors.
        self.revisions.visible = false;
        self.busy.insert(task);
        let controller = self.controller.clone();
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            let result = async {
                workspace.validate_revision_source(source).await?;
                controller.submit(task, edited).await
            }
            .await;
            let details = controller.details(task).await.ok().flatten();
            Ok(Update::Revision(Box::new(Reply::Sent {
                task,
                details,
                error: result.err().map(|error| error.to_string()),
            })))
        });
        cx.notify();
    }

    fn branch_revision(&mut self, cx: &mut Context<Self>) {
        let Some(dialog) = self.revisions.dialog.as_mut() else {
            return;
        };
        let source_task = dialog.source.task.id;
        if self.revisions.pending
            || self.creating_task
            || self.selected != Some(source_task)
            || dialog.editor.read(cx).is_composing()
        {
            return;
        }
        let edited = dialog.editor.read(cx).text().to_owned();
        if edited.trim().is_empty() || edited.len() > 1024 * 1024 {
            dialog.error =
                Some("The revised message must contain text and fit within 1 MiB.".into());
            cx.notify();
            return;
        }
        let source = dialog.source.clone();
        let selection_revision = self.selection_revision;
        dialog.error = None;
        self.revisions.pending = true;
        self.revisions.visible = false;
        self.creating_task = true;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            let result = async {
                let task = workspace.branch_user_revision(source, edited).await?;
                let catalog = workspace.catalog().await?;
                Ok::<_, WorkspaceError>((task, catalog))
            }
            .await
            .map_err(|error| error.to_string());
            Ok(Update::Revision(Box::new(Reply::Branched {
                source: source_task,
                selection_revision,
                result,
            })))
        });
        cx.notify();
    }

    pub(super) fn revision_reply(&mut self, reply: Reply, cx: &mut Context<Self>) {
        match reply {
            Reply::Sent {
                task,
                details,
                error,
            } => {
                self.revisions.pending = false;
                self.busy.remove(&task);
                if self.selected == Some(task) {
                    self.details = details;
                }
                match error {
                    Some(error) => {
                        if let Some(dialog) = self.revisions.dialog.as_mut() {
                            dialog.error = Some(format!(
                                "Revised message was not sent: {error}. Your edit is retained."
                            ));
                        }
                        self.revisions.visible = true;
                        self.revisions.focus_pending = true;
                    }
                    None => {
                        self.revisions.dialog = None;
                        self.revisions.visible = false;
                        self.notice = Some(
                            "Revised message sent as a new turn. Earlier transcript history was preserved.".into(),
                        );
                    }
                }
                self.hydrate();
            }
            Reply::Branched {
                source,
                selection_revision,
                result,
            } => {
                self.revisions.pending = false;
                self.creating_task = false;
                match result {
                    Ok((task, catalog)) => {
                        self.catalog = catalog;
                        self.revisions.dialog = None;
                        self.revisions.visible = false;
                        if self.selected == Some(source)
                            && self.selection_revision == selection_revision
                            && self.select_task(task.id, cx)
                        {
                            self.show_conversation(cx);
                            self.notice = Some(
                                "Revision branch created as an unsent draft. Review it before Send.".into(),
                            );
                        } else {
                            self.notice = Some(
                                "Revision branch created. Open it from the conversation list when ready.".into(),
                            );
                        }
                    }
                    Err(error) => {
                        if let Some(dialog) = self.revisions.dialog.as_mut() {
                            dialog.error = Some(format!(
                                "Revision branch was not created: {error}. Your edit is retained."
                            ));
                        }
                        self.revisions.visible = true;
                        self.revisions.focus_pending = true;
                    }
                }
            }
        }
        cx.notify();
    }

    pub(super) fn restore_revision_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.revisions.visible || !self.revisions.focus_pending {
            return;
        }
        self.revisions.focus_pending = false;
        if let Some(dialog) = &self.revisions.dialog {
            window.focus(&dialog.editor.read(cx).focus_handle(cx), cx);
        }
    }

    pub(super) fn revision_overlay(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(dialog) = self
            .revisions
            .dialog
            .as_ref()
            .filter(|_| self.revisions.visible)
        else {
            return div().into_any_element();
        };
        let task = dialog.source.task.id;
        div()
            .id("message-revision-backdrop")
            .absolute()
            .inset_0()
            .occlude()
            .bg(gpui::rgba(0x00000088))
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if !this.revisions.pending {
                        if let Some(dialog) = this.revisions.dialog.as_ref()
                            && dialog.editor.read(cx).text() == dialog.original
                        {
                            this.dismiss_revision(cx);
                        }
                    }
                    cx.stop_propagation();
                }),
            )
            .child(
                div()
                    .id("message-revision-dialog")
                    .role(gpui::Role::Dialog)
                    .aria_label("Edit and resend message")
                    .tab_group()
                    .w_full()
                    .max_w(px(680.))
                    .max_h(px(560.))
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .rounded_xl()
                    .border_1()
                    .border_color(rgb(palette().border))
                    .bg(ui::surface(palette().overlay))
                    .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| {
                        cx.stop_propagation()
                    })
                    .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                        if event.keystroke.key == "escape" && !this.revisions.pending {
                            if let Some(dialog) = this.revisions.dialog.as_ref()
                                && dialog.editor.read(cx).text() == dialog.original
                            {
                                this.dismiss_revision(cx);
                            }
                        }
                        cx.stop_propagation();
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(ui::icon(Glyph::Compose))
                            .child(
                                div()
                                    .flex_1()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child("Edit and resend"),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(rgb(palette().muted))
                                    .child("History stays immutable"),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(palette().muted))
                            .child(
                                "Send adds a new turn. Branch creates a new unsent conversation from context before this message. Neither action rolls back files, approvals, attachments or provider state.",
                            ),
                    )
                    .child(div().h(px(270.)).min_h_0().child(dialog.editor.clone()))
                    .children(dialog.error.as_ref().map(|error| {
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(palette().error))
                            .child(error.clone())
                    }))
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap_2()
                            .child(
                                ui::action(
                                    "revision-cancel",
                                    "Cancel",
                                    None,
                                    false,
                                    cx.listener(|this, _: &(), _, cx| {
                                        this.dismiss_revision(cx)
                                    }),
                                ),
                            )
                            .child(
                                ui::action(
                                    "revision-to-composer",
                                    "Move to composer",
                                    None,
                                    false,
                                    cx.listener(|this, _: &(), _, cx| {
                                        this.revision_to_composer(cx)
                                    }),
                                ),
                            )
                            .child(
                                ui::action(
                                    "revision-branch",
                                    "Branch from here",
                                    Some(Glyph::BranchSimple),
                                    false,
                                    cx.listener(|this, _: &(), _, cx| {
                                        this.branch_revision(cx)
                                    }),
                                ),
                            )
                            .child(div().flex_1())
                            .child(
                                ui::action(
                                    "revision-send",
                                    "Send revised now",
                                    Some(Glyph::Send),
                                    self.revisions.pending
                                        || self.busy.contains(&task)
                                        || self.connecting.contains(&task),
                                    cx.listener(|this, _: &(), _, cx| {
                                        this.send_revision(cx)
                                    }),
                                ),
                            ),
                    ),
            )
            .into_any_element()
    }
}
