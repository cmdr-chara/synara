//! Extracted from native/mod.rs during the surface split — see mod.rs.
#![allow(unused_imports)]

use super::*;
use gpui_kit::component::{
    button::{Button, ButtonCustomVariant, ButtonRounded, ButtonVariant, ButtonVariants as _},
    h_flex,
    input::{Editor, EditorState, Input, InputEvent, InputState},
    menu::{DropdownMenu as _, PopupMenuItem},
    slider::{Slider, SliderEvent, SliderState},
    scroll::ScrollableElement as _,
    spinner::Spinner,
    text::TextView,
    theme::Theme,
    v_flex,
    Icon,
    ActiveTheme as _, IconName, Sizable as _,
};
use gpui_kit::*;
use gpui_kit::component::WindowExt as _;
use gpui_kit::base::Disableable as _;
use gpui_kit::assets::IconName as AssetIcon;
use gpui::prelude::FluentBuilder as _;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

impl Native {
    /// Toggle the embedded browser panel. The webview entity is created
    /// lazily because it needs the window handle; closing hides the native
    /// subview rather than dropping it, since wry teardown is heavyweight
    /// and reopening should be instant.
    pub(super) fn toggle_browser(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.view_browser = !self.view_browser;
        if self.view_browser {
            if self.browser_webview.is_none() {
                let webview = cx.new(|cx| {
                    let builder = wry::WebViewBuilder::new();
                    #[cfg(debug_assertions)]
                    let builder = builder.with_devtools(true);
                    gpui_webview::WebView::build(builder, window, cx)
                        .expect("Failed to create browser panel webview")
                });
                webview.update(cx, |view, _| {
                    view.load_url("http://127.0.0.1:3000");
                });
                self.browser_webview = Some(webview);
            } else if let Some(webview) = &self.browser_webview {
                webview.update(cx, |view, _| {
                    view.show();
                });
            }
            if self.browser_input.is_none() {
                let input = cx.new(|cx| {
                    InputState::new(window, cx).placeholder("http://127.0.0.1:3000")
                });
                cx.subscribe_in(&input, window, |this: &mut Self, state, ev, _w, cx| {
                    if let InputEvent::PressEnter { .. } = ev {
                        let mut url = state.read(cx).value().to_string();
                        if !url.is_empty() && !url.contains("://") {
                            url = format!("http://{url}");
                        }
                        if let Some(webview) = &this.browser_webview {
                            webview.update(cx, |view, _| {
                                view.load_url(&url);
                            });
                        }
                    }
                })
                .detach();
                self.browser_input = Some(input);
            }
        } else {
            // Dropping the entity hides the native subview (wry teardown is
            // lazy but invisible at once); recreating on reopen reloads the
            // document, acceptable against a stale overlay trapping input.
            self.browser_webview = None;
            self.browser_input = None;
        }
        cx.notify();
    }

    /// Toggle the computer-use panel: opening connects the frame stream
    /// and starts the refresh timer; closing retires the receiver thread.
    pub(super) fn toggle_cua(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.view_cua = !self.view_cua;
        if self.view_cua {
            if self.cua.is_none() {
                self.cua = Some(crate::cua::CuaHandle::new());
            }
            if let Some(handle) = &self.cua {
                handle.start();
            }
            self.ensure_cua_refresh(cx);
            if self.cua_input.is_none() {
                let input = cx.new(|cx| {
                    InputState::new(window, cx).placeholder("type into the desktop, Enter sends")
                });
                cx.subscribe_in(&input, window, |this: &mut Self, state, ev, w, cx| {
                    if let InputEvent::PressEnter { .. } = ev {
                        let text = state.read(cx).value().to_string();
                        if text.is_empty() {
                            return;
                        }
                        state.update(cx, |s, cx| {
                            s.set_value("", w, cx);
                        });
                        let thread = this
                            .selected_thread
                            .clone()
                            .unwrap_or_else(|| "shell".to_string());
                        let client_cell = this.client.clone();
                        cx.spawn(async move |_this, cx: &mut AsyncApp| {
                            let cell = client_cell.clone();
                            if let Some(client) = cx
                                .background_executor()
                                .spawn(async move { ensure_client(&cell) })
                                .await
                            {
                                let thread2 = thread.clone();
                                let text2 = text.clone();
                                let client2 = client.clone();
                                let _ = cx
                                    .background_executor()
                                    .spawn(async move {
                                        crate::cua::type_text(&client2, &thread2, &text2)
                                    })
                                    .await;
                                let _ = cx
                                    .background_executor()
                                    .spawn(async move {
                                        crate::cua::key(&client, &thread, "return")
                                    })
                                    .await;
                            }
                        })
                        .detach();
                    }
                })
                .detach();
                self.cua_input = Some(input);
            }
            if self.cua_screen_points.is_none() {
                let client_cell = self.client.clone();
                cx.spawn(async move |this, cx: &mut AsyncApp| {
                    let cell = client_cell.clone();
                    if let Some(client) = cx
                        .background_executor()
                        .spawn(async move { ensure_client(&cell) })
                        .await
                    {
                        let result = cx
                            .background_executor()
                            .spawn(async move {
                                client.call("computer.getScreenSize", json!({}))
                            })
                            .await;
                        if let Some(v) = result {
                            let w = v["screenSize"]["width"].as_f64().or_else(|| v["screenSize"]["width"].as_u64().map(|u| u as f64));
                            let h = v["screenSize"]["height"].as_f64().or_else(|| v["screenSize"]["height"].as_u64().map(|u| u as f64));
                            if let (Some(w), Some(h)) = (w, h) {
                                let _ = this.update(cx, |view, cx| {
                                    view.cua_screen_points = Some((w, h));
                                    cx.notify();
                                });
                            }
                        }
                    }
                })
                .detach();
            }
        } else if let Some(handle) = &self.cua {
            handle.stop();
        }
        cx.notify();
    }

    /// Frame cache path for the newest still: gpui images load from disk,
    /// so the receiver's PNG lands in a rotating pair of files (gpui caches
    /// by path — alternating names defeats the cache exactly on change).
    pub(super) fn cua_frame_path(&self) -> Option<std::path::PathBuf> {
        let handle = self.cua.as_ref()?;
        let s = handle.shared.lock().ok()?;
        let png = s.frame_png.as_ref()?;
        let dir = std::env::temp_dir().join("synara-cua");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(format!("frame-{}.png", s.sequence % 2));
        if std::fs::write(&path, png).is_ok() {
            Some(path)
        } else {
            None
        }
    }

    /// PNG pixel dimensions from the IHDR header (big-endian u32s at 16/20).
    pub(super) fn cua_png_size(&self) -> Option<(f32, f32)> {
        let handle = self.cua.as_ref()?;
        let s = handle.shared.lock().ok()?;
        let png = s.frame_png.as_ref()?;
        if png.len() < 24 {
            return None;
        }
        let w = u32::from_be_bytes([png[16], png[17], png[18], png[19]]) as f32;
        let h = u32::from_be_bytes([png[20], png[21], png[22], png[23]]) as f32;
        Some((w, h))
    }
}
