//! Capped conversation input surface around the existing native text/IME entity.
use super::*;
mod commands;
mod context;
use crate::ui::{self, Glyph, palette};

impl Shell {
    pub(super) fn composer_panel(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let busy = self.selected.is_some_and(|task| self.busy.contains(&task));
        let native_command = self.native_command_draft(cx);
        let disabled = !busy
            && (self.direct_route_loading()
                || self.loading_task.is_some()
                || self
                    .selected
                    .is_some_and(|t| self.draft_state.loading.contains(&t))
                || self.goal_send_pending(cx)
                || (!native_command && self.controls_blocked())
                || self.attachment_send_blocked()
                || (!native_command && self.attachment_capability_error().is_some())
                || self.composer.read(cx).text().trim().is_empty());
        let composer_bounds = self.controls.composer_bounds.clone();
        div()
            .relative()
            .on_drop(cx.listener(|this, paths: &gpui::ExternalPaths, _, cx| {
                this.attachment_paths(paths.paths().to_vec(), cx);
            }))
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
                    .border_color(
                        if self.composer.read(cx).focus_handle(cx).is_focused(window) {
                            rgb(palette().focus)
                        } else {
                            ui::glass_edge()
                        },
                    )
                    .bg(ui::surface(palette().overlay))
                    .when(
                        self.settings.value.appearance.personalization.material
                            == SurfaceMaterial::Glass,
                        |el| {
                            el.bg(gpui::linear_gradient(
                                145.,
                                gpui::linear_color_stop(ui::surface(palette().selected), 0.),
                                gpui::linear_color_stop(ui::surface(palette().overlay), 1.),
                            ))
                        },
                    )
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
                    .child(
                        div()
                            .id("composer-context-tray")
                            .max_h(px(210.))
                            .overflow_y_scroll()
                            .child(self.attachments_view(cx))
                            .child(self.appsnap_view(cx))
                            .child(self.followups_view(cx)),
                    )
                    .child(
                        div()
                            .relative()
                            .child(self.composer.clone())
                            .child(ui::layout_probe("primary-composer-input")),
                    )
                    .children(self.composer.read(cx).error.as_ref().map(|error| {
                        div()
                            .px_2()
                            .text_sm()
                            .text_color(rgb(palette().error))
                            .child(error.clone())
                    }))
                    .children(self.voice.message.as_ref().map(|message| {
                        div()
                            .px_2()
                            .text_xs()
                            .text_color(rgb(if self.voice.failed {
                                palette().error
                            } else {
                                palette().muted
                            }))
                            .child(message.clone())
                    }))
                    .child(
                        div()
                            .px_2()
                            .text_xs()
                            .text_color(rgb(palette().muted))
                            .child("Voice clips upload to ChatGPT for transcription. The transcript stays an unsent draft until you choose Send."),
                    )
                    .child(self.native_commands_view(cx))
                    .child(self.context_usage_view(cx))
                    .child(
                        div()
                            .flex()
                            .items_end()
                            .justify_between()
                            .gap_1()
                            .child(div().flex_1().min_w_0().child(
                                if self.uses_direct_model() || self.direct_route_loading() {
                                    self.direct_model_controls(cx)
                                } else {
                                    self.session_controls(cx)
                                },
                            ))
                            .child(
                                ui::chrome_button(
                                    "attach-files",
                                    "Attach images or UTF-8 files",
                                    Glyph::Attach,
                                    self.attachment_send_blocked(),
                                    cx.listener(|this, _: &(), _, cx| this.choose_attachments(cx)),
                                )
                                .size(px(28.)),
                            )
                            .child(
                                ui::chrome_button(
                                    "appsnap-toggle",
                                    "AppSnap: capture one application window",
                                    Glyph::Capture,
                                    self.selected.is_none(),
                                    cx.listener(|this, _: &(), _, cx| this.toggle_appsnap(cx)),
                                )
                                .size(px(28.)),
                            )
                            .child(self.followup_toggle(cx))
                            .child(if cfg!(any(
                                target_os = "linux",
                                target_os = "macos",
                                target_os = "windows"
                            )) {
                                ui::chrome_button(
                                    "voice-input",
                                    if self.voice.recording() {
                                        "Stop recording and transcribe"
                                    } else if self.voice.transcribing() {
                                        "Transcribing voice"
                                    } else {
                                        "Record voice draft; upload to ChatGPT for transcription"
                                    },
                                    if self.voice.recording() { Glyph::Stop } else { Glyph::Mic },
                                    self.voice.transcribing()
                                        || (!self.voice.recording()
                                            && (self.selected.is_none()
                                                || self.loading_task.is_some()
                                                || self.close != CloseState::Open)),
                                    cx.listener(|this, _: &(), _, cx| this.voice_primary(cx)),
                                )
                            } else {
                                ui::unavailable_action(
                                    "voice-input",
                                    "Voice input",
                                    Glyph::Mic,
                                    "This native build does not support microphone recording.",
                                )
                                .w(px(30.))
                                .px_2()
                            })
                            .children(self.voice.active().then(|| {
                                ui::chrome_button(
                                    "voice-cancel",
                                    "Cancel voice recording or transcription",
                                    Glyph::Close,
                                    false,
                                    cx.listener(|this, _: &(), _, cx| this.voice_cancel(cx)),
                                )
                            }))
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
                                .child(ui::layout_probe_enabled("composer-submit", !disabled)),
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
