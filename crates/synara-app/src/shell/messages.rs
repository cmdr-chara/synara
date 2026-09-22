//! The transcript uses native text, compact user bubbles, and an assistant action strip.
mod fork;
use super::*;
use crate::ui::{self, Glyph, palette};

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
            });
        div().id(("message", index)).group("message-actions").relative().w_full().flex().flex_col()
            .top(px(3. * (1. - progress))).opacity(progress)
            .when(user, |el| el.items_end()).child(body)
            .when(highlighted, |el| el.child(ui::layout_probe_slot("message-match", index)))
            .when(!user && !reasoning && complete, |el| el.child(
                div().ml(px(-6.)).flex().items_center().gap_2().text_size(px(12.)).text_color(rgb(palette().muted))
                    .child(copy(cx).opacity(0.75))
                    .child(self.message_branch_button(message, cx))
                    .child(self.message_side_chat_button(message, index, cx))
                    .child(self.message_pin_button(message, index, cx))
                    .child(self.message_reuse_button(message, index, cx))
                    .children(self.task().filter(|task| task.scope == TaskScope::Studio).map(|task| {
                        let task = task.id; let anchor = MessageAnchor::from(message);
                        ui::chrome_button("message-to-hub", "Review message as shared Hub knowledge", Glyph::Notebook, false,
                            cx.listener(move |this, _: &(), _, cx| this.promote_hub_message(task,anchor.clone(),cx))).size(px(24.))
                    }))
                    .children(timestamp.map(|text| div().relative().child(ui::layout_probe("message-timestamp")).child(text)))))
            .when(user, |el| el.child(div().absolute().right_0().bottom(px(-24.)).child(
                div().flex().items_center()
                    .child(copy(cx))
                    .child(self.message_revision_button(message, index, cx))
                    .child(self.message_side_chat_button(message, index, cx))
                    .child(self.message_pin_button(message, index, cx))
                    .child(self.message_reuse_button(message, index, cx)))) )
            .into_any_element()
    }
}
