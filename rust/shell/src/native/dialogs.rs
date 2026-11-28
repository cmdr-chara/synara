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
    /// Clear every rail-route surface (the transcript shows when all are
    /// off). Docks (changes/terminal) are independent surfaces, untouched.
    pub(super) fn clear_views(&mut self) {
        self.view_files = false;
        self.view_dev = false;
        self.view_inbox = false;
        self.view_settings = false;
        self.view_tasks = false;
        self.view_kanban = false;
        self.view_pullreq = false;
        self.view_autom = false;
        self.view_studio = false;
        self.view_spaces = false;
        self.hover_thread = None;
        self.hover_card = None;
        self.hover_card_seq += 1;
        self.hover_project = None;
        self.project_card = None;
        self.project_card_seq += 1;
    }

    /// Rail route toggle: open that surface (closing the others); clicking
    /// the active one again returns to the transcript.
    pub(super) fn open_route(&mut self, which: u8, cx: &mut Context<Self>) {
        let was = match which {
            1 => self.view_tasks,
            2 => self.view_kanban,
            3 => self.view_pullreq,
            4 => self.view_autom,
            5 => self.view_studio,
            6 => self.view_spaces,
            _ => false,
        };
        self.clear_views();
        if !was {
            match which {
                1 => self.view_tasks = true,
                2 => self.view_kanban = true,
                3 => self.view_pullreq = true,
                4 => self.view_autom = true,
                5 => self.view_studio = true,
                6 => self.view_spaces = true,
                _ => {}
            }
        }
        if self.view_autom {
            self.fetch_automations(cx);
        }
        if self.view_pullreq {
            self.fetch_github_inbox(cx);
        }
        if self.view_tasks || self.view_kanban {
            self.fetch_inbox(cx);
        }
        cx.notify();
    }

    /// Upstream SnoozeUntilDialog: free-form "YYYY-MM-DD HH:MM" entry,
    /// prefilled with the current snooze deadline or now+1h.
    pub(super) fn open_snooze_dialog(
        &mut self,
        thread_id: String,
        current: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let initial = current
            .and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .ok()
                    .map(|d| d.with_timezone(&chrono::Local))
            })
            .unwrap_or_else(|| chrono::Local::now() + chrono::Duration::hours(1))
            .format("%Y-%m-%d %H:%M")
            .to_string();
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("YYYY-MM-DD HH:MM"));
        input.update(cx, |s, cx| s.set_value(initial, window, cx));
        cx.subscribe_in(&input, window, |_this, _s, event, _w, cx| {
            if let InputEvent::Change = event {
                cx.notify();
            }
        })
        .detach();
        self.snooze_dlg_tid = Some(thread_id);
        self.snooze_dlg_input = Some(input.clone());
        let focus = input.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        cx.notify();
    }

    pub(super) fn close_snooze_dialog(&mut self, cx: &mut Context<Self>) {
        self.snooze_dlg_tid = None;
        self.snooze_dlg_input = None;
        cx.notify();
    }

    /// Accepts "YYYY-MM-DD HH:MM" (or with 'T'); must be a future time
    /// (upstream parseFutureDateTimeLocalValue).
    pub(super) fn commit_snooze_dialog(&mut self, cx: &mut Context<Self>) {
        let Some(tid) = self.snooze_dlg_tid.clone() else {
            return;
        };
        let raw = self
            .snooze_dlg_input
            .as_ref()
            .map(|i| i.read(cx).value().trim().replace('T', " "))
            .unwrap_or_default();
        let Some(naive) = chrono::NaiveDateTime::parse_from_str(&raw, "%Y-%m-%d %H:%M").ok() else {
            return;
        };
        let Some(local) = naive.and_local_timezone(chrono::Local).earliest() else {
            return;
        };
        if local <= chrono::Local::now() {
            return;
        }
        let until = local.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        self.dispatch_cmd(
            json!({
                "type": "thread.meta.update",
                "commandId": uuid(),
                "threadId": tid,
                "snoozedUntil": until,
            }),
            cx,
        );
        self.close_snooze_dialog(cx);
    }

    pub(super) fn open_feedback(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Share details (required)"));
        self.feedback_input = Some(input.clone());
        self.feedback_category = None;
        self.feedback_open = true;
        self.feedback_sending = false;
        let focus = input.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        cx.notify();
    }

    /// POST the dialog to upstream's public feedback endpoint
    /// (feedback.ts submitFeedback → trysynara.com/api/feedback).
    pub(super) fn submit_feedback(&mut self, cx: &mut Context<Self>) {
        let details = self
            .feedback_input
            .as_ref()
            .map(|i| i.read(cx).value().to_string())
            .unwrap_or_default();
        if details.trim().is_empty() || self.feedback_sending {
            return;
        }
        self.feedback_sending = true;
        let cat = self.feedback_category.map(|i| Self::FEEDBACK_CATEGORIES[i]);
        let detail = self.detail.clone().unwrap_or(Value::Null);
        let thread_v = get(&detail, "thread");
        let provider = str_at(thread_v, "providerName").to_string();
        let messages_n = arr(thread_v, "messages").len();
        let activities_n = arr(thread_v, "activities").len();
        let session_status = session_phase(Some(&detail)).to_string();
        let runtime_mode = self.runtime_mode.to_string();
        let interaction_mode = self.interaction_mode.to_string();
        let lead = cat.map(|c| c.2).unwrap_or("I have some feedback");
        let body = json!({
            "category": cat.map(|c| c.0),
            "details": details.trim(),
            "summary": format!("{lead} in Synara 1.0.0."),
            "diagnostics": {
                "appVersion": "1.0.0",
                "submittedAt": now_iso(),
                "userAgent": "synara-shell/1.0.0",
                "platform": "macOS",
                "language": "en-US",
                "viewport": "0x0",
                "provider": if provider.is_empty() { Value::Null } else { Value::String(provider) },
                "model": Value::Null,
                "projectKind": Value::Null,
                "environmentMode": Value::Null,
                "runtimeMode": runtime_mode,
                "interactionMode": interaction_mode,
                "sessionStatus": session_status,
                "latestTurnState": Value::Null,
                "messageCount": messages_n,
                "activityCount": activities_n,
                "hasPendingApproval": false,
                "hasPendingUserInput": false,
                "hasThreadError": false,
            },
        });
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let res = cx
                .background_executor()
                .spawn(async move {
                    ureq::post("https://www.trysynara.com/api/feedback")
                        .timeout(std::time::Duration::from_secs(20))
                        .set("x-synara-feedback", "1")
                        .send_json(body)
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.feedback_sending = false;
                match res {
                    Ok(_) => {
                        this.feedback_open = false;
                        this.feedback_toast = Some(("Feedback sent".to_string(), false));
                    }
                    Err(_) => {
                        this.feedback_toast = Some(("Could not send feedback".to_string(), true));
                    }
                }
                cx.spawn(async move |this2: WeakEntity<Self>, cx2: &mut AsyncApp| {
                    cx2.background_executor()
                        .timer(std::time::Duration::from_secs(4))
                        .await;
                    let _ = this2.update(cx2, |this2: &mut Self, cx2| {
                        this2.feedback_toast = None;
                        cx2.notify();
                    });
                })
                .detach();
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Upstream OneTimeCoachmark: a single accent hint at a time, seen-keyed,
    /// marked seen the moment it opens, auto-hidden after 8s.
    pub(super) fn show_next_coachmark(&mut self, cx: &mut Context<Self>) {
        if self.coachmark.is_some() {
            return;
        }
        let next = if !self.coachmarks_seen.contains("activity") {
            "activity"
        } else if !self.coachmarks_seen.contains("tasks") {
            "tasks"
        } else {
            return;
        };
        self.coachmark = Some(next);
        self.coachmarks_seen.insert(next.to_string());
        self.persist_sidebar_ui(cx);
        cx.notify();
        let weak = cx.weak_entity();
        cx.spawn(async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
            cx.background_executor()
                .timer(std::time::Duration::from_secs(8))
                .await;
            let _ = weak.update(cx, |this: &mut Self, cx| {
                if this.coachmark == Some(next) {
                    this.coachmark = None;
                    this.show_next_coachmark(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Clicking the wrapped control dismisses its hint (upstream marks it
    /// seen at open, so no extra write is needed here).
    pub(super) fn dismiss_coachmark(&mut self, key: &'static str, cx: &mut Context<Self>) {
        if self.coachmark == Some(key) {
            self.coachmark = None;
            self.show_next_coachmark(cx);
            cx.notify();
        }
    }

    /// Upstream "Trigger action failed toasts" — fires stacked Git-failure
    /// error toasts for local UI testing.
    pub(super) fn trigger_debug_toasts(&mut self, cx: &mut Context<Self>) {
        self.feedback_toast = Some(("Git action failed.".to_string(), true));
        cx.notify();
    }
}
