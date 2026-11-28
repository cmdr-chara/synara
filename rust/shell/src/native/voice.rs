//! Extracted from native/mod.rs during the surface split — see mod.rs.
#![allow(unused_imports)]

use super::*;
use gpui::prelude::FluentBuilder as _;
use gpui_kit::assets::IconName as AssetIcon;
use gpui_kit::base::Disableable as _;
use gpui_kit::component::WindowExt as _;
use gpui_kit::component::{
    button::{Button, ButtonCustomVariant, ButtonRounded, ButtonVariant, ButtonVariants as _},
    h_flex,
    input::{Editor, EditorState, Input, InputEvent, InputState},
    menu::{DropdownMenu as _, PopupMenuItem},
    scroll::ScrollableElement as _,
    slider::{Slider, SliderEvent, SliderState},
    spinner::Spinner,
    text::TextView,
    theme::Theme,
    v_flex, ActiveTheme as _, Icon, IconName, Sizable as _,
};
use gpui_kit::*;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

impl Native {
    /// Composer mic: start capturing a voice note. Failure surfaces as a
    /// toast; the button stays inert when a transcription is in flight.
    pub(super) fn start_voice(&mut self, cx: &mut Context<Self>) {
        if self.voice_transcribing || self.voice_capture.is_some() {
            return;
        }
        match crate::voice::VoiceCapture::start() {
            Ok(capture) => {
                self.voice_capture = Some(capture);
            }
            Err(e) => {
                self.feedback_toast = Some((format!("Voice recording unavailable: {e}"), true));
            }
        }
        cx.notify();
    }

    /// Stop the capture, hand the WAV clip to the transcription backend and
    /// insert the returned text into the composer draft. The blocking WS call
    /// runs on the background executor exactly like the snapshot fetches.
    pub(super) fn stop_voice(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(capture) = self.voice_capture.take() else {
            return;
        };
        let wav = capture.finish();
        if wav.is_empty() {
            cx.notify();
            return;
        }
        self.voice_transcribing = true;
        cx.notify();
        let client = self
            .client
            .lock()
            .ok()
            .and_then(|guard| guard.as_ref().cloned());
        let Some(client) = client else {
            self.voice_transcribing = false;
            cx.notify();
            return;
        };
        use base64::Engine as _;
        let audio_base64 = base64::engine::general_purpose::STANDARD.encode(&wav);
        let payload = json!({
            "mimeType": "audio/wav",
            "sampleRateHz": 24000,
            "durationMs": (wav.len().saturating_sub(44) / 2 / 24) as u64,
            "audioBase64": audio_base64,
        });
        let composer = self.composer.clone();
        let window_handle = window.window_handle();
        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let c = client.clone();
            let result = cx
                .background_executor()
                .spawn(async move { c.call("server.transcribeVoice", payload) })
                .await;
            let text = result
                .as_ref()
                .and_then(|v| {
                    v.get("text")
                        .or_else(|| v.get("transcript"))
                        .and_then(Value::as_str)
                })
                .unwrap_or("")
                .trim()
                .to_string();
            if !text.is_empty() {
                let text2 = text.clone();
                let composer2 = composer.clone();
                let _ = cx.update_window(window_handle, move |_, window, app| {
                    composer.update(app, |state, cx| {
                        let current = state.value().to_string();
                        let merged = if current.trim().is_empty() {
                            text2.clone()
                        } else if current.ends_with(' ') || current.ends_with('\n') {
                            format!("{current}{text2}")
                        } else {
                            format!("{current} {text2}")
                        };
                        state.set_value(merged, window, cx);
                        let focus = composer2.read(cx).focus_handle(cx);
                        window.focus(&focus, cx);
                    });
                });
            }
            let empty = text.is_empty();
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.voice_transcribing = false;
                if empty {
                    this.feedback_toast =
                        Some(("Voice transcription returned no text".to_string(), true));
                }
                cx.notify();
            });
            // voiceEnterBehavior "send": the transcription just landed in the
            // draft; fire the normal send path with the composer's window.
            if !empty {
                let _ = cx.update_window(window_handle, move |_, window, app| {
                    let _ = this.update(app, |this: &mut Self, cx| {
                        if std::mem::take(&mut this.voice_send_after) {
                            this.send(window, cx);
                        }
                    });
                });
            }
        })
        .detach();
    }
}
