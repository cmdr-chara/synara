//! Capped conversation input surface around the existing native text/IME entity.
use super::*;
use crate::ui::{self, DARK, Glyph};

impl Shell {
    pub(super) fn composer_panel(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let busy = self.selected.is_some_and(|task| self.busy.contains(&task));
        let disabled =
            !busy && (self.controls_blocked() || self.composer.read(cx).text().trim().is_empty());
        let project = self
            .catalog
            .projects
            .iter()
            .find(|project| Some(project.id) == self.project);
        div()
            .w_full()
            .max_w(px(ui::CHAT_WIDTH))
            .mx_auto()
            .flex_shrink_0()
            .px_3()
            .pb_3()
            .children((!self.transcript.is_following()).then(|| {
                div().flex().justify_center().pb_2().child(
                    ui::button("jump-latest", "Jump to latest", false).on_click(cx.listener(
                        |this, _, _, cx| {
                            this.transcript.follow();
                            cx.notify();
                        },
                    )),
                )
            }))
            .children(project.map(|project| {
                div()
                    .px_3()
                    .pb_2()
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_size(px(12.))
                    .text_color(rgb(DARK.muted))
                    .child(ui::icon(Glyph::Folder))
                    .child(project.name.clone())
            }))
            .child(
                div()
                    .p_2()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .rounded_xl()
                    .border_1()
                    .border_color(rgb(
                        if self.composer.read(cx).focus_handle(cx).is_focused(window) {
                            DARK.focus
                        } else {
                            DARK.border
                        },
                    ))
                    .bg(rgb(DARK.overlay))
                    .child(self.composer.clone())
                    .children(self.composer.read(cx).error.as_ref().map(|error| {
                        div()
                            .px_2()
                            .text_sm()
                            .text_color(rgb(DARK.error))
                            .child(error.clone())
                    }))
                    .child(
                        div()
                            .flex()
                            .items_end()
                            .justify_between()
                            .gap_2()
                            .child(div().flex_1().min_w_0().child(self.session_controls(cx)))
                            .child(
                                ui::icon_button(
                                    "composer-submit",
                                    if busy {
                                        "Stop response"
                                    } else {
                                        "Send message"
                                    },
                                    if busy { Glyph::Stop } else { Glyph::Send },
                                    disabled,
                                    cx.listener(|this, _: &(), _, cx| {
                                        if this
                                            .selected
                                            .is_some_and(|task| this.busy.contains(&task))
                                        {
                                            this.cancel(cx);
                                        } else {
                                            this.send_prompt(cx);
                                        }
                                    }),
                                )
                                .child(ui::layout_probe("composer-submit")),
                            ),
                    ),
            )
            .into_any_element()
    }
    pub(super) fn welcome(&self) -> gpui::AnyElement {
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_3()
            .p_6()
            .child(
                div()
                    .text_size(px(28.))
                    .text_color(rgb(DARK.text))
                    .child("What should we work on?"),
            )
            .children(self.selected.is_none().then(|| {
                div()
                    .text_sm()
                    .text_color(rgb(DARK.muted))
                    .child("Open a project and create a thread to get started.")
            }))
            .into_any_element()
    }
}
