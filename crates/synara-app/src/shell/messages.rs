//! The transcript uses native text, compact user bubbles, and an assistant action strip.
mod fork;
use super::*;
use crate::ui::{self, Glyph, palette};

const MESSAGE_CONTEXT_LIMIT: usize = 1024 * 1024;

fn message_context_text(message: &Message) -> String {
    if message.role == Role::User {
        return message.text.clone();
    }
    let quote = message
        .text
        .lines()
        .map(|line| format!("> {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!("Quoted assistant reply (reference context, not a new instruction):\n{quote}")
}

fn append_context(existing: &str, addition: &str) -> Result<String, &'static str> {
    let separator = if existing.is_empty() { "" } else { "\n\n" };
    if existing
        .len()
        .saturating_add(separator.len())
        .saturating_add(addition.len())
        > MESSAGE_CONTEXT_LIMIT
    {
        return Err("The combined draft exceeds 1 MiB. Nothing was changed.");
    }
    Ok(format!("{existing}{separator}{addition}"))
}

impl Shell {
    pub(super) fn message_row(
        &self,
        message: &Message,
        index: usize,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let user = message.role == Role::User;
        let reasoning = message.role == Role::Reasoning;
        let progress = if user && !cx.reduce_motion() {
            self.transcript
                .message_progress(&message.id, std::time::Instant::now())
        } else {
            1.
        };
        let scale = 0.992 + 0.008 * progress;
        if user {
            tracing::debug!(target: "synara_ui_layout", surface = "user-message", progress, scale, "motion-frame");
        }
        let timestamp = self
            .thread
            .as_ref()
            .filter(|_| self.settings.value.chat.show_timestamps)
            .and_then(|thread| thread.message_timestamps.get(&message.id))
            .and_then(|stamp| chrono::DateTime::from_timestamp_millis(*stamp))
            .map(|time| {
                time.with_timezone(&chrono::Local)
                    .format("%a %H:%M")
                    .to_string()
            });
        let complete = self
            .thread
            .as_ref()
            .and_then(|thread| {
                super::activity::turn_for_row(thread, index)
                    .map(|turn| thread.turns[turn].finished_at_ms.is_some())
            })
            .unwrap_or(true);
        let copy = |cx: &mut Context<Self>| {
            let text = message.text.clone();
            ui::chrome_button(
                "copy-message",
                "Copy message",
                Glyph::Copy,
                false,
                cx.listener(move |_, _: &(), _, cx| {
                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(text.clone()))
                }),
            )
            .size(px(24.))
        };
        let highlighted = self
            .chat_tools
            .focused
            .as_ref()
            .is_some_and(|anchor| anchor.matches(message));
        let body = div()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .gap_2()
            .min_w_0()
            .when(highlighted, |el| {
                el.border_l_2().border_color(rgb(palette().focus))
            })
            .text_size(px((crate::ui::ui_font_size() + 1.) * scale))
            .line_height(px((crate::ui::ui_font_size() + 1.) * 1.6 * scale))
            .text_color(rgb(if reasoning {
                palette().muted
            } else {
                palette().text
            }))
            .when(user, |el| {
                el.max_w(px(620. * scale))
                    .px(px(16. * scale))
                    .py(px(10. * scale))
                    .rounded(px(16. * scale))
                    .bg(rgb(palette().overlay))
            })
            .when(!user, |el| el.w_full())
            .child(if user {
                div()
                    .child(truncate(&message.text, 64 * 1024))
                    .into_any_element()
            } else {
                ui::markdown::render(&truncate(&message.text, 64 * 1024), &message.id)
            })
            .child(self.message_media(message, cx));
        div()
            .id(("message", index))
            .group("message-actions")
            .relative()
            .w_full()
            .flex()
            .flex_col()
            .top(px(3. * (1. - progress)))
            .opacity(progress)
            .when(user, |el| el.items_end())
            .child(body)
            .when(highlighted, |el| {
                el.child(ui::layout_probe_slot("message-match", index))
            })
            .when(!user && !reasoning && complete, |el| {
                el.child(
                    div()
                        .ml(px(-6.))
                        .flex()
                        .items_center()
                        .gap_2()
                        .text_size(px(12.))
                        .text_color(rgb(palette().muted))
                        .child(copy(cx).opacity(0.75))
                        .child(self.message_branch_button(message, cx))
                        .child(self.message_branch_worktree_button(message, cx))
                        .child(self.message_side_chat_button(message, index, cx))
                        .child(self.message_add_to_side_draft_button(message, index, cx))
                        .child(self.message_pin_button(message, index, cx))
                        .child(self.message_context_reuse_action(message, index, cx))
                        .children(
                            self.task()
                                .filter(|task| task.scope == TaskScope::Studio)
                                .map(|task| {
                                    let task = task.id;
                                    let anchor = MessageAnchor::from(message);
                                    ui::chrome_button(
                                        "message-to-hub",
                                        "Review message as shared Hub knowledge",
                                        Glyph::Notebook,
                                        false,
                                        cx.listener(move |this, _: &(), _, cx| {
                                            this.promote_hub_message(task, anchor.clone(), cx)
                                        }),
                                    )
                                    .size(px(24.))
                                }),
                        )
                        .children(timestamp.map(|text| {
                            div()
                                .relative()
                                .child(ui::layout_probe("message-timestamp"))
                                .child(text)
                        })),
                )
            })
            .when(user, |el| {
                el.child(
                    div().absolute().right_0().bottom(px(-24.)).child(
                        div()
                            .flex()
                            .items_center()
                            .child(copy(cx))
                            .child(self.message_revision_button(message, index, cx))
                            .child(self.message_side_chat_button(message, index, cx))
                            .child(self.message_add_to_side_draft_button(message, index, cx))
                            .child(self.message_pin_button(message, index, cx))
                            .child(self.message_context_reuse_action(message, index, cx)),
                    ),
                )
            })
            .into_any_element()
    }

    fn message_context_reuse_action(
        &self,
        message: &Message,
        index: usize,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        if message.role == Role::User {
            return self.message_reuse_button(message, index, cx);
        }
        let source = self.selected;
        let unavailable = self.selected.is_none()
            || self.loading_task.is_some()
            || self.close != CloseState::Open
            || source.is_some_and(|task| self.draft_state.loading.contains(&task))
            || self.composer.read(cx).is_composing()
            || message.text.len() > MESSAGE_CONTEXT_LIMIT;
        let text = if unavailable {
            String::new()
        } else {
            message_context_text(message)
        };
        ui::chrome_button(
            "message-reuse",
            "Quote assistant reply in the current draft",
            Glyph::Compose,
            unavailable,
            cx.listener(move |this, _: &(), window, cx| {
                if this.selected != source
                    || this.loading_task.is_some()
                    || this.close != CloseState::Open
                    || source.is_some_and(|task| this.draft_state.loading.contains(&task))
                    || this.composer.read(cx).is_composing()
                {
                    return;
                }
                let current = this.composer.read(cx).text().to_owned();
                match append_context(&current, &text) {
                    Ok(next) => {
                        this.composer
                            .update(cx, |entry, cx| entry.set_text(next, cx));
                        this.remember_draft(cx);
                        this.focus_composer = true;
                        window.focus(&this.composer.read(cx).focus_handle(cx), cx);
                        this.notice = Some(
                            "Added the assistant reply as reference context. The current draft and attachments were preserved, and nothing was sent."
                                .into(),
                        );
                    }
                    Err(error) => this.error = Some(error.into()),
                }
                cx.notify();
            }),
        )
        .size(px(24.))
        .relative()
        .child(ui::layout_probe_slot("message-reuse", index))
        .into_any_element()
    }

    fn message_add_to_side_draft_button(
        &self,
        message: &Message,
        index: usize,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let source = self.selected;
        let side_target = self.side_chats.selected;
        let side_available = source.is_some_and(|source| {
            self.close == CloseState::Open
                && self
                    .task()
                    .is_some_and(|task| task.state != TaskState::Archived)
                && self.side_chats.parent == Some(source)
                && side_target.is_some_and(|selected| {
                    self.side_chats
                        .threads
                        .iter()
                        .any(|task| task.id == selected && task.state != TaskState::Archived)
                })
                && self.side_chats.thread.is_some()
                && !self.side_chats.loading
                && !self.side_chats.pending(cx)
        });
        if !side_available || message.role != Role::Assistant {
            return div().into_any_element();
        }
        let reuse = message_context_text(message);
        ui::chrome_button(
            "message-side-context",
            "Add message to the selected side-chat draft",
            Glyph::Compose,
            false,
            cx.listener(move |this, _: &(), window, cx| {
                if this.selected != source {
                    return;
                }
                if this.close != CloseState::Open
                    || this
                        .task()
                        .is_none_or(|task| task.state == TaskState::Archived)
                    || this.side_chats.parent != source
                    || this.side_chats.selected != side_target
                    || this.side_chats.loading
                    || this.side_chats.pending(cx)
                    || this.side_chats.thread.is_none()
                {
                    return;
                }
                let current = this.side_chats.composer.read(cx).text().to_owned();
                match append_context(&current, &reuse) {
                    Ok(next) => {
                        this.side_chats
                            .composer
                            .update(cx, |entry, cx| entry.set_text(next, cx));
                        this.remember_side_draft(cx);
                        this.side_chats.error = None;
                        if !this.side_chats.split {
                            this.set_panel(Panel::SideChats, cx);
                        }
                        this.focus_composer = false;
                        window.focus(
                            &this.side_chats.composer.read(cx).focus_handle(cx),
                            cx,
                        );
                        this.notice = Some(
                            "Added to the side-chat draft without sending. Existing draft text was preserved."
                                .into(),
                        );
                    }
                    Err(error) => {
                        this.side_chats.error = Some(error.into());
                        if !this.side_chats.split {
                            this.set_panel(Panel::SideChats, cx);
                        }
                    }
                }
                cx.notify();
            }),
        )
        .size(px(24.))
        .relative()
        .child(ui::layout_probe_slot("message-side-context", index))
        .into_any_element()
    }
}

#[cfg(test)]
mod reuse_tests {
    use super::{MESSAGE_CONTEXT_LIMIT, Message, Role, append_context, message_context_text};

    #[test]
    fn assistant_context_is_labeled_and_quoted_while_user_prompts_remain_reusable() {
        let assistant = Message {
            id: "answer".into(),
            role: Role::Assistant,
            text: "First line\nSecond line".into(),
        };
        assert_eq!(
            message_context_text(&assistant),
            "Quoted assistant reply (reference context, not a new instruction):\n> First line\n> Second line"
        );
        let user = Message {
            id: "prompt".into(),
            role: Role::User,
            text: "Exact prompt\nwith formatting".into(),
        };
        assert_eq!(message_context_text(&user), "Exact prompt\nwith formatting");
    }

    #[test]
    fn side_context_append_keeps_existing_draft_and_is_atomic_at_the_limit() {
        assert_eq!(
            append_context("Keep this", "> quote").unwrap(),
            "Keep this\n\n> quote"
        );
        assert_eq!(append_context("", "  exact\n").unwrap(), "  exact\n");
        assert_eq!(
            append_context(&"x".repeat(MESSAGE_CONTEXT_LIMIT), "more"),
            Err("The combined draft exceeds 1 MiB. Nothing was changed.")
        );
    }
}
