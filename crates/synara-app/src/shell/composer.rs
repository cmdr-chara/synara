//! Capped conversation input surface around the existing native text/IME entity.
use super::*;
use crate::ui::{self, Glyph, palette};

impl Shell {
    pub(super) fn composer_panel(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let busy = self.selected.is_some_and(|task| self.busy.contains(&task));
        let disabled =
            !busy && (self.controls_blocked() || self.composer.read(cx).text().trim().is_empty());
        let composer_bounds = self.controls.composer_bounds.clone();
        div()
            .relative()
            .w_full()
            .max_w(px(ui::chat_width() + 40.))
            .mx_auto()
            .flex_shrink_0()
            .px_5()
            .pb(px(15.))
            .children((!self.transcript.is_following()).then(|| {
                div()
                    .absolute()
                    .top(px(-44.))
                    .left_0()
                    .w_full()
                    .flex()
                    .justify_center()
                    .child(
                        ui::chrome_button(
                            "jump-latest",
                            "Jump to latest",
                            Glyph::Down,
                            false,
                            cx.listener(|this, _: &(), _, cx| {
                                this.transcript.follow();
                                cx.notify();
                            }),
                        )
                        .size(px(32.))
                        .rounded_full()
                        .bg(rgb(palette().overlay))
                        .border_1()
                        .border_color(rgb(palette().border)),
                    )
            }))
            .children(
                self.thread
                    .as_ref()
                    .is_none_or(|thread| thread.timeline.is_empty())
                    .then(|| {
                        div()
                            .px_2()
                            .pb(px(2.))
                            .flex()
                            .child(self.project_picker(cx))
                    }),
            )
            .child(
                div()
                    .p_2()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .rounded(px(18.))
                    .border_1()
                    .border_color(if self.composer.read(cx).focus_handle(cx).is_focused(window) {
                        rgb(palette().focus)
                    } else { ui::glass_edge() })
                    .bg(ui::surface(palette().overlay))
                    .when(self.settings.value.appearance.personalization.material == SurfaceMaterial::Glass, |el| el.bg(gpui::linear_gradient(
                        145., gpui::linear_color_stop(ui::surface(palette().selected), 0.),
                        gpui::linear_color_stop(ui::surface(palette().overlay), 1.),
                    )))
                    .relative()
                    .child(ui::layout_probe("composer-surface"))
                    .child(
                        gpui::canvas(
                            move |bounds, _, _| composer_bounds.set(bounds),
                            |_, _, _, _| {},
                        )
                        .absolute()
                        .size_full()
                        .top_0()
                        .left_0(),
                    )
                    .child(self.composer.clone())
                    .children(self.composer.read(cx).error.as_ref().map(|error| {
                        div()
                            .px_2()
                            .text_sm()
                            .text_color(rgb(palette().error))
                            .child(error.clone())
                    }))
                    .child(
                        div()
                            .flex()
                            .items_end()
                            .justify_between()
                            .gap_1()
                            .child(div().flex_1().min_w_0().child(self.session_controls(cx)))
                            .child(
                                ui::unavailable_action(
                                    "voice-input",
                                    "",
                                    Glyph::Mic,
                                    "Voice input is not available in this native build yet.",
                                )
                                .aria_label("Voice input, unavailable")
                                .w(px(30.))
                                .px_2(),
                            )
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
            .gap_4()
            .p_6()
            .child(
                gpui::svg()
                    .path("brand/synara.svg")
                    .w(px(37.3))
                    .h(px(40.))
                    .text_color(rgb(palette().text)),
            )
            .child(
                div()
                    .id("welcome-heading")
                    .role(gpui::Role::Heading)
                    .relative()
                    .child(ui::layout_probe("welcome-heading"))
                    .text_size(px(29.))
                    .line_height(px(34.5))
                    .text_color(rgb(palette().text))
                    .child("What should we work on?"),
            )
            .children(self.selected.is_none().then(|| {
                div()
                    .text_sm()
                    .text_color(rgb(palette().muted))
                    .child("Choose a project to get started.")
            }))
            .into_any_element()
    }
}
