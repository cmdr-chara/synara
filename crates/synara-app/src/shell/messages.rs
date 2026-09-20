//! The transcript uses native text, compact user bubbles, and an assistant action strip.
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
        let body = div()
            .min_w_0()
            .text_size(px(15. * scale))
            .line_height(px(24. * scale))
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
            .when(!user && !reasoning && complete, |el| el.child(
                div().ml(px(-6.)).flex().items_center().gap_2().text_size(px(12.)).text_color(rgb(palette().muted))
                    .child(copy(cx).opacity(0.75))
                    .child(ui::unavailable_action("fork-message", "", Glyph::Fork, "Branching from a message is not available in this native build yet.")
                        .aria_label("Branch from message, unavailable").size(px(24.)).p_0().gap_0().justify_center())
                    .child(ui::unavailable_action("pin-message", "", Glyph::Pin, "Pinned messages are not available in this native build yet.")
                        .aria_label("Pin message, unavailable").size(px(24.)).p_0().gap_0().justify_center())
                    .children(timestamp.map(|text| div().relative().child(ui::layout_probe("message-timestamp")).child(text)))))
            .when(user, |el| el.child(div().absolute().right_0().bottom(px(-24.)).child(
                copy(cx).opacity(0.).group_hover("message-actions", |style| style.opacity(1.))
                    .focus_visible(|style| style.opacity(1.).border_color(rgb(palette().focus))))))
            .into_any_element()
    }
}
