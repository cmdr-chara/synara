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
    /// Context-menu "Open Path in Terminal": when the drawer is open send a
    /// `cd`, otherwise seed the next terminal.open with the path and open it.
    pub(super) fn open_terminal_at(&mut self, path: String, window: &mut Window, cx: &mut Context<Self>) {
        if self.view_terminal {
            self.term_write(format!("cd '{}'\r", path.replace('\'', "'\\''")), cx);
        } else {
            self.term_pending_cwd = Some(path);
            self.toggle_terminal(window, cx);
        }
    }

    /// Rail button: toggle the bottom terminal drawer. Opening spawns a PTY
    /// session on the workspace root via terminal.open and subscribes to its
    /// event stream; closing kills it.
    pub(super) fn toggle_terminal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.view_terminal = !self.view_terminal;
        self.term_active = self.view_terminal;
        if self.view_terminal {
            if self.term_tabs.is_empty() {
                self.term_tabs.push(TermTab::default());
            }
            self.term_sel = self.term_tabs.len() - 1;
            if self.term_input.is_none() {
                let input = cx.new(|cx| {
                    InputState::new(window, cx).placeholder("type a command")
                });
                cx.subscribe_in(&input, window, |this: &mut Self, _s, ev, w, cx| {
                    if let InputEvent::PressEnter { .. } = ev {
                        this.term_send(w, cx);
                    }
                })
                .detach();
                self.term_input = Some(input);
            }
            self.ensure_terminal_events(cx);
            self.open_terminal(cx);
        } else {
            self.close_all_terminals(cx);
        }
        cx.notify();
    }

    /// "+" chip in the terminal sub-strip: spawn a sibling PTY session.
    pub(super) fn term_new_tab(&mut self, cx: &mut Context<Self>) {
        self.term_tabs.push(TermTab::default());
        self.term_sel = self.term_tabs.len() - 1;
        self.open_terminal(cx);
        cx.notify();
    }

    /// "x" on a tab: kill that PTY; when the last one dies the input row
    /// goes away (the surface itself is closed by the strip's ×).
    pub(super) fn term_close_tab(&mut self, idx: usize, cx: &mut Context<Self>) {
        if idx >= self.term_tabs.len() {
            return;
        }
        let tab = self.term_tabs.remove(idx);
        if self.term_sel > idx {
            self.term_sel -= 1;
        } else if self.term_sel >= self.term_tabs.len() {
            self.term_sel = self.term_tabs.len().saturating_sub(1);
        }
        if self.term_tabs.is_empty() {
            self.term_input = None;
            self.term_search_input = None;
            self.term_search_query.clear();
        }
        let client = self.client.clone();
        let terminal_id = tab.id;
        let thread_id = self.term_thread_id();
        cx.spawn(async move |_this, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            let _ = cx
                .background_executor()
                .spawn(async move {
                    client.call(
                        "terminal.close",
                        json!({
                            "threadId": thread_id,
                            "terminalId": terminal_id,
                            "deleteHistory": true,
                        }),
                    )
                })
                .await;
        })
        .detach();
        cx.notify();
    }

    pub(super) fn term_select_tab(&mut self, idx: usize, cx: &mut Context<Self>) {
        if idx < self.term_tabs.len() {
            self.term_sel = idx;
            cx.notify();
        }
    }

    /// Strip ×: kill every PTY session (the surface itself closes via
    /// toggle_terminal, which calls this).
    pub(super) fn close_all_terminals(&mut self, cx: &mut Context<Self>) {
        for i in (0..self.term_tabs.len()).rev() {
            self.term_close_tab(i, cx);
        }
    }

    /// Magnifier in the terminal sub-strip: toggle the in-buffer find bar
    /// (upstream TerminalSearch). Esc/× closes and clears the query.
    pub(super) fn toggle_term_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.term_search_input.is_some() {
            self.term_search_input = None;
            self.term_search_query.clear();
        } else {
            let input = cx.new(|cx| {
                InputState::new(window, cx).placeholder("find in buffer")
            });
            cx.subscribe_in(&input, window, |this: &mut Self, _s, ev, _w, cx| {
                if let InputEvent::Change = ev {
                    this.term_search_query = _s.read(cx).value().to_string();
                    cx.notify();
                }
            })
            .detach();
            input.update(cx, |s, cx| s.focus_handle(cx).focus(window, cx));
            self.term_search_input = Some(input);
        }
        cx.notify();
    }

    pub(super) fn term_active(&self) -> &TermTab {
        &self.term_tabs[self.term_sel]
    }

    pub(super) fn term_thread_id(&self) -> String {
        self.selected_thread
            .clone()
            .unwrap_or_else(|| "shell".to_string())
    }

    pub(super) fn open_terminal(&mut self, cx: &mut Context<Self>) {
        if self.term_tabs.is_empty() {
            self.term_tabs.push(TermTab::default());
            self.term_sel = 0;
        }
        let client = self.client.clone();
        let terminal_id = self.term_active().id.clone();
        let thread_id = self.term_thread_id();
        let cwd = self
            .term_pending_cwd
            .take()
            .or_else(|| self.files_root())
            .unwrap_or_else(|| "/".to_string());
        cx.spawn(async move |_this, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            let _ = cx
                .background_executor()
                .spawn(async move {
                    client.call(
                        "terminal.open",
                        json!({
                            "threadId": thread_id,
                            "terminalId": terminal_id,
                            "cwd": cwd,
                            "cols": 140,
                            "rows": 30,
                            "streamOutput": true,
                        }),
                    )
                })
                .await;
        })
        .detach();
    }

    pub(super) fn close_terminal(&mut self, cx: &mut Context<Self>) {
        self.term_input = None;
        if let Some(tab) = self.term_tabs.get_mut(self.term_sel) {
            tab.status = None;
            tab.buf.clear();
        }
        let client = self.client.clone();
        let Some(terminal_id) = self.term_tabs.get(self.term_sel).map(|t| t.id.clone()) else {
            return;
        };
        let thread_id = self.term_thread_id();
        cx.spawn(async move |_this, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            let _ = cx
                .background_executor()
                .spawn(async move {
                    client.call(
                        "terminal.close",
                        json!({
                            "threadId": thread_id,
                            "terminalId": terminal_id,
                            "deleteHistory": true,
                        }),
                    )
                })
                .await;
        })
        .detach();
    }

    pub(super) fn term_restart(&mut self, cx: &mut Context<Self>) {
        let client = self.client.clone();
        let terminal_id = self.term_active().id.clone();
        let thread_id = self.term_thread_id();
        let cwd = self.files_root().unwrap_or_else(|| "/".to_string());
        cx.spawn(async move |_this, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            let _ = cx
                .background_executor()
                .spawn(async move {
                    client.call(
                        "terminal.restart",
                        json!({
                            "threadId": thread_id,
                            "terminalId": terminal_id,
                            "cwd": cwd,
                            "cols": 140,
                            "rows": 30,
                        }),
                    )
                })
                .await;
        })
        .detach();
    }

    pub(super) fn term_send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(input) = self.term_input.clone() else {
            return;
        };
        let text = input.read(cx).value().to_string();
        if text.is_empty() {
            return;
        }
        input.update(cx, |s, cx| s.clean(window, cx));
        self.term_write(format!("{text}\r"), cx);
    }

    /// Send raw input to the open PTY session via terminal.write.
    pub(super) fn term_write(&mut self, data: String, cx: &mut Context<Self>) {
        let client = self.client.clone();
        let terminal_id = self.term_active().id.clone();
        let thread_id = self.term_thread_id();
        cx.spawn(async move |_this, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            let _ = cx
                .background_executor()
                .spawn(async move {
                    client.call(
                        "terminal.write",
                        json!({
                            "threadId": thread_id,
                            "terminalId": terminal_id,
                            "data": data,
                        }),
                    )
                })
                .await;
        })
        .detach();
    }

    /// Long-lived subscribeEvents stream feeding the terminal drawer; it
    /// reconnects itself while the drawer is open.
    pub(super) fn ensure_terminal_events(&mut self, cx: &mut Context<Self>) {
        if self.term_events {
            return;
        }
        self.term_events = true;
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            loop {
                let open = this
                    .update(cx, |t: &mut Self, _| t.view_terminal)
                    .unwrap_or(false);
                if !open {
                    break;
                }
                let cell = client.clone();
                let client = cx
                    .background_executor()
                    .spawn(async move { ensure_client(&cell) })
                    .await;
                let Some(client) = client else {
                    cx.background_executor()
                        .timer(std::time::Duration::from_secs(3))
                        .await;
                    continue;
                };
                let (tx, rx) = async_channel::unbounded::<Vec<Value>>();
                let c = client.clone();
                cx.background_executor()
                    .spawn(async move {
                        c.call_stream("terminal.subscribeEvents", json!({}), move |v| {
                            let _ = tx.send_blocking(v);
                        });
                    })
                    .detach();
                while let Ok(values) = rx.recv().await {
                    let _ = this.update(cx, |t: &mut Self, cx| {
                        for ev in values {
                            t.on_terminal_event(ev);
                        }
                        cx.notify();
                    });
                }
                cx.background_executor()
                    .timer(std::time::Duration::from_secs(1))
                    .await;
            }
            let _ = this.update(cx, |t: &mut Self, _| {
                t.term_events = false;
            });
        })
        .detach();
    }

    pub(super) fn on_terminal_event(&mut self, ev: Value) {
        if str_at(&ev, "threadId") != self.term_thread_id() {
            return;
        }
        let id = str_at(&ev, "terminalId");
        let Some(tab) = self.term_tabs.iter_mut().find(|t| t.id == id) else {
            return;
        };
        match str_at(&ev, "type") {
            "started" | "restarted" => {
                let snap = get(&ev, "snapshot");
                tab.buf = strip_ansi(str_at(snap, "history"));
                tab.status = Some(str_at(snap, "status").to_string());
            }
            "output" => {
                tab.buf.push_str(&strip_ansi(str_at(&ev, "data")));
                if tab.buf.len() > 200_000 {
                    let mut cut = tab.buf.len() - 100_000;
                    while !tab.buf.is_char_boundary(cut) {
                        cut += 1;
                    }
                    tab.buf.drain(..cut);
                }
            }
            "cleared" => tab.buf.clear(),
            "exited" => {
                tab.status = Some("exited".to_string());
                let suffix = get(&ev, "exitCode")
                    .as_i64()
                    .map(|c| format!(" code {c}"))
                    .unwrap_or_default();
                tab.buf.push_str(&format!("\n[process exited{suffix}]\n"));
            }
            _ => {}
        }
    }
}
