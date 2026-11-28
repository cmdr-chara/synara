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
    /// The new-chat landing is the transcript pane when no rail surface is
    /// open and the selected thread is empty (mirrors the render branch).
    pub(super) fn landing_active(&self) -> bool {
        if self.view_settings
            || self.view_dev
            || self.view_files
            || self.view_inbox
            || self.view_tasks
            || self.view_kanban
            || self.view_pullreq
            || self.view_autom
            || self.view_studio
            || self.view_changes
        {
            return false;
        }
        if self.view_terminal && self.term_active {
            return false;
        }
        if self.selected_thread.is_none() {
            return true;
        }
        arr(
            get(self.detail.as_ref().unwrap_or(&Value::Null), "thread"),
            "messages",
        )
        .is_empty()
    }

    pub(super) fn refresh_desktop_update_state(&mut self, cx: &mut Context<Self>) {
        let next = crate::updater::desktop_update_service().snapshot_state();
        if next != self.desktop_update_state {
            self.desktop_update_state = next;
            cx.notify();
            cx.refresh_windows();
        }
    }

    /// Sidebar updater action, backed by `server.desktopUpdate` rather than by
    /// poking the shell-local service directly: the backend remains the owner
    /// of busy-chat restart admission, while the shell bridge executes the update.
    pub(super) fn perform_desktop_update_action(&mut self, cx: &mut Context<Self>) {
        let Some(state) = self
            .desktop_update_state
            .clone()
            .or_else(|| crate::updater::desktop_update_service().snapshot_state())
        else {
            return;
        };
        if crate::updater::desktop_update_button_disabled(Some(&state))
            || self.desktop_update_installing
        {
            return;
        }
        let action = match crate::updater::resolve_desktop_update_button_action(&state) {
            crate::updater::DesktopUpdateButtonAction::Check => "check",
            crate::updater::DesktopUpdateButtonAction::Download => "download",
            crate::updater::DesktopUpdateButtonAction::Install => "install",
            crate::updater::DesktopUpdateButtonAction::None => return,
        };
        let expected_version = state
            .downloaded_version
            .clone()
            .or_else(|| state.available_version.clone());
        if action == "install" {
            self.desktop_update_installing = true;
            cx.notify();
        }
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else {
                let _ = this.update(cx, |this: &mut Self, cx| {
                    this.desktop_update_installing = false;
                    cx.notify();
                });
                return;
            };
            let expected = expected_version.clone();
            let result = cx
                .background_executor()
                .spawn(async move {
                    if action == "install" {
                        let Some(version) = expected else {
                            return Value::Null;
                        };
                        let payload = json!({
                            "action": "install",
                            "confirmRestart": true,
                            "expectedVersion": version,
                        });
                        let first = client
                            .call("server.desktopUpdate", payload)
                            .unwrap_or(Value::Null);
                        let Some(token) = first
                            .get("restartToken")
                            .and_then(Value::as_str)
                            .map(str::to_string)
                        else {
                            return first;
                        };
                        if !first
                            .get("accepted")
                            .and_then(Value::as_bool)
                            .unwrap_or(false)
                        {
                            return first;
                        }
                        client
                            .call(
                                "server.desktopUpdate",
                                json!({
                                    "action": "install",
                                    "confirmRestart": true,
                                    "expectedVersion": version,
                                    "restartToken": token,
                                }),
                            )
                            .unwrap_or(Value::Null)
                    } else {
                        client
                            .call("server.desktopUpdate", json!({"action": action}))
                            .unwrap_or(Value::Null)
                    }
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if let Some(next) = desktop_update_state_from_result(&result) {
                    this.desktop_update_state = Some(next);
                }
                this.desktop_update_installing = false;
                cx.notify();
            });
        })
        .detach();
    }

    /// Run one Stable↔Beta verb off the UI thread. Install can download a
    /// large payload, while import/launch are local process handoffs.
    pub(super) fn perform_beta_action(&mut self, action: &'static str, cx: &mut Context<Self>) {
        if self.beta_action_running.is_some() {
            return;
        }
        self.beta_action_running = Some(action);
        self.beta_action_message = None;
        cx.notify();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let channel = crate::beta_channel::DesktopBetaChannel::current();
                    match action {
                        "install" => channel.install(),
                        "launch" => channel.launch(),
                        "import" => channel.import_and_launch(),
                        "leave" => channel.leave(),
                        _ => crate::beta_channel::BetaActionResult {
                            ok: false,
                            error: Some("internal"),
                            message: Some("Unknown beta action.".into()),
                        },
                    }
                })
                .await;
            let ok = result.ok;
            let message = result.message.or(if ok {
                Some(match action {
                    "install" => "Synara Beta is installed and opening.".into(),
                    "launch" => "Synara Beta is opening.".into(),
                    "import" => "Your data will be copied when Synara Beta starts.".into(),
                    "leave" => "Synara is opening.".into(),
                    _ => "Done.".into(),
                })
            } else {
                None
            });
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.beta_action_running = None;
                this.beta_action_message = message;
                cx.notify();
            });
            // Upstream hands the user back to Stable and then exits the Beta
            // app. Stable is already launched by the background action; quitting
            // here is part of the successful leave flow rather than UI cleanup.
            if action == "leave" && ok {
                cx.update(|app| app.quit());
            }
        })
        .detach();
    }

    pub(super) fn eval_whats_new(&mut self, cx: &mut Context<Self>) {
        match self.wn_seen.clone() {
            None => self.mark_whats_new_seen(cx),
            Some(seen) if wn_cmp_versions(APP_VERSION, &seen) > 0 => {
                if whats_new_entries()
                    .iter()
                    .any(|e| str_at(e, "version") == APP_VERSION)
                {
                    self.wn_popout = true;
                } else {
                    self.mark_whats_new_seen(cx);
                }
            }
            _ => {}
        }
    }

    /// Acknowledge the current release (upstream markSeen): record the version
    /// under settings.sidebarUi and close every What's-new surface.
    pub(super) fn mark_whats_new_seen(&mut self, cx: &mut Context<Self>) {
        self.wn_seen = Some(APP_VERSION.to_string());
        self.wn_popout = false;
        self.wn_view = 0;
        self.persist_sidebar_ui(cx);
    }
}
