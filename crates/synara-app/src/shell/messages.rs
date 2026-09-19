//! Conversational hierarchy independent of event delivery and transcript ownership.
use super::*;
use crate::ui::{self, DARK, Glyph};

impl Shell {
    pub(super) fn message_row(
        &self,
        message: &Message,
        index: usize,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let text = message.text.clone();
        let user = message.role == Role::User;
        let reasoning = message.role == Role::Reasoning;
        div()
            .id(("message", index))
            .w_full()
            .flex()
            .when(user, |el| el.justify_end())
            .child(
                div()
                    .min_w_0()
                    .max_w(px(if user { 620. } else { ui::CHAT_WIDTH }))
                    .when(user, |el| el.p_3().rounded_xl().bg(rgb(DARK.overlay)))
                    .when(!user, |el| el.w_full().px_1().py_2())
                    .children(reasoning.then(|| {
                        div()
                            .pb_2()
                            .text_size(px(12.))
                            .text_color(rgb(DARK.muted))
                            .child("Thinking")
                    }))
                    .child(
                        div()
                            .text_size(px(14.))
                            .line_height(px(22.))
                            .text_color(rgb(if reasoning { DARK.muted } else { DARK.text }))
                            .child(truncate(&message.text, 64 * 1024)),
                    )
                    .child(div().flex().justify_end().pt_1().child(ui::icon_button(
                        "copy-message",
                        "Copy message",
                        Glyph::Copy,
                        false,
                        cx.listener(move |_, _: &(), _, cx| {
                            cx.write_to_clipboard(gpui::ClipboardItem::new_string(text.clone()))
                        }),
                    ))),
            )
            .into_any_element()
    }
}
