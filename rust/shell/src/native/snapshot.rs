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
    pub(super) fn deliver_activity_notification(
        &mut self,
        thread_id: &str,
        title: String,
        body: String,
        warning: bool,
        cx: &mut Context<Self>,
    ) {
        let toast = self.setting_bool("enableTaskCompletionToasts", true);
        let system = self.setting_bool("enableSystemTaskCompletionNotifications", true)
            && !self.notification_window_active;
        if !toast && !system {
            return;
        }
        let Some(handle) = self.notification_window else {
            return;
        };
        let thread_id = thread_id.to_string();
        let weak = cx.weak_entity();
        let _ = cx.update_window(handle, move |_, window, cx| {
            let click_thread = thread_id.clone();
            let click_weak = weak.clone();
            let mut notification = gpui_kit::component::notification::Notification::new()
                .title(title)
                .message(body)
                .autohide(true)
                .on_click(move |_, _window, app| {
                    let thread = click_thread.clone();
                    let _ = click_weak.update(app, |this, cx| this.open_thread(thread, cx));
                });
            notification = if warning {
                notification.with_type(gpui_kit::component::notification::NotificationType::Warning)
            } else {
                notification.with_type(gpui_kit::component::notification::NotificationType::Success)
            };
            notification = match (toast, system) {
                (true, true) => notification.in_app_and_system(),
                (false, true) => notification.system(),
                _ => notification,
            };
            window.push_notification(notification, cx);
        });
    }

    /// Port of the web task-completion watcher for thread sessions: initial
    /// hydration only primes state, then running→settled transitions and newly
    /// pending input produce toast/OS notifications according to settings.
    pub(super) fn observe_snapshot_notifications(&mut self, incoming: &Value, cx: &mut Context<Self>) {
        let threads = arr(incoming, "threads");
        // The server materializes snoozeReminderAt only when a snooze expires,
        // so its presence is itself the due-reminder event. Dedupe by the exact
        // reminder timestamp across reconnects and snapshot refetches.
        for thread in threads {
            let reminder_at = str_at(thread, "snoozeReminderAt");
            if reminder_at.is_empty()
                || !get(thread, "archivedAt").is_null()
                || !get(thread, "snoozedUntil").is_null()
            {
                continue;
            }
            let id = str_at(thread, "id");
            let key = format!("snooze:{id}:{reminder_at}");
            if !self.notification_keys.insert(key) {
                continue;
            }
            let title = str_at(thread, "title");
            let title = if title.trim().is_empty() { "Untitled thread".to_string() } else { title.to_string() };
            self.deliver_activity_notification(
                id,
                title,
                "Ready to pick this thread back up?".to_string(),
                false,
                cx,
            );
        }
        if !self.notification_primed {
            self.notification_states = threads
                .iter()
                .map(|thread| {
                    (
                        str_at(thread, "id").to_string(),
                        (
                            thread_running_for_activity(thread),
                            get(thread, "hasPendingApprovals").as_bool() == Some(true)
                                || get(thread, "hasPendingUserInput").as_bool() == Some(true),
                        ),
                    )
                })
                .filter(|(id, _)| !id.is_empty())
                .collect();
            self.notification_primed = true;
            return;
        }

        let wait_subagents = self.setting_bool("notifyAfterSubagentsFinish", true);
        let mut next = std::collections::HashMap::new();
        let mut events: Vec<(String, String, String, bool, String)> = Vec::new();
        for thread in threads {
            let id = str_at(thread, "id").to_string();
            if id.is_empty() {
                continue;
            }
            let running = thread_running_for_activity(thread);
            let attention = get(thread, "hasPendingApprovals").as_bool() == Some(true)
                || get(thread, "hasPendingUserInput").as_bool() == Some(true);
            let Some((previous_running, previous_attention)) = self.notification_states.get(&id).copied() else {
                next.insert(id, (running, attention));
                continue;
            };
            let latest = get(thread, "latestTurn");
            let completed_at = str_at(latest, "completedAt");
            let completion_key = format!(
                "{}:{}:{}",
                id,
                str_at(latest, "turnId"),
                completed_at
            );
            let completion = previous_running
                && !running
                && str_at(latest, "state") == "completed"
                && !completed_at.is_empty()
                && get(thread, "snoozedUntil").is_null()
                && get(thread, "archivedAt").is_null()
                && (!wait_subagents || get(thread, "parentThreadId").is_null())
                && get(thread, "pendingBackgroundTaskCount").as_u64().unwrap_or(0) == 0
                && !self.notification_keys.contains(&completion_key);
            if completion {
                self.notification_keys.insert(completion_key.clone());
                let title = str_at(thread, "title");
                let title = if title.trim().is_empty() { "Untitled thread".to_string() } else { title.to_string() };
                let preview = str_at(thread, "latestReplyPreview");
                let body = if preview.trim().is_empty() {
                    "Finished working.".to_string()
                } else {
                    preview.to_string()
                };
                events.push((id.clone(), title, body, false, completion_key));
            }
            if !previous_attention && attention && get(thread, "snoozedUntil").is_null() {
                let title = str_at(thread, "title");
                let title = if title.trim().is_empty() { "Untitled thread".to_string() } else { title.to_string() };
                let key = format!("attention:{}:{}", id, str_at(get(thread, "session"), "updatedAt"));
                if self.notification_keys.insert(key.clone()) {
                    events.push((id.clone(), "Input needed".to_string(), format!("{title}: input requested"), true, key));
                }
            }
            next.insert(id, (running, attention));
        }
        self.notification_states = next;
        if self.notification_keys.len() > 256 {
            self.notification_keys = self.notification_keys.iter().skip(128).cloned().collect();
        }
        for (thread_id, title, body, warning, _) in events {
            self.deliver_activity_notification(&thread_id, title, body, warning, cx);
        }
    }

    pub(super) fn adopt_snapshot(&mut self, snapshot: Value, cx: &mut Context<Self>) {
        self.observe_snapshot_notifications(&snapshot, cx);
        self.snapshot = Some(snapshot);
    }

    /// (Re)fetch the shell snapshot, then keep it live via subscribeShell.
    pub(super) fn refresh(&self, cx: &mut Context<Self>) {
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            loop {
                let cell = client.clone();
                let client = cx
                    .background_executor()
                    .spawn(async move { ensure_client(&cell) })
                    .await;
                let Some(client) = client else {
                    eprintln!("native: ws connect failed");
                    cx.background_executor()
                        .timer(std::time::Duration::from_secs(3))
                        .await;
                    continue;
                };
                let c = client.clone();
                let snapshot = cx
                    .background_executor()
                    .spawn(async move { c.call("orchestration.getShellSnapshot", json!({})) })
                    .await;
                let c2 = client.clone();
                let cfg = cx
                    .background_executor()
                    .spawn(async move { c2.call("server.getConfig", json!({})) })
                    .await;
                let _ = this.update(cx, |this: &mut Self, cx| {
                    if let Some(providers) = cfg
                        .as_ref()
                        .and_then(|v| v.get("providers"))
                        .and_then(|p| p.as_array())
                        .cloned()
                    {
                        this.provider_statuses = providers;
                    }
                    if let Some(rules) = cfg.and_then(|v| {
                        v.get("keybindings")
                            .and_then(|k| k.as_array())
                            .cloned()
                    }) {
                        this.kb_rules = rules;
                    }
                    if let Some(s) = snapshot {
                        this.adopt_snapshot(s, cx);
                    }
                    this.fetch_git_status(cx);
                    if this.settings_data.is_null() {
                        this.fetch_settings(cx);
                    }
                    cx.notify();
                });
                // Live updates: subscribeShell streams shell-domain events; on
                // each chunk just refetch the snapshot.
                let (tx, rx) = async_channel::unbounded::<()>();
                let c = client.clone();
                cx.background_executor()
                    .spawn(async move {
                        c.call_stream("orchestration.subscribeShell", json!({}), move |_| {
                            let _ = tx.send_blocking(());
                        });
                    })
                    .detach();
                while rx.recv().await.is_ok() {
                    let c = client.clone();
                    let s = cx
                        .background_executor()
                        .spawn(async move { c.call("orchestration.getShellSnapshot", json!({})) })
                        .await;
                    let _ = this.update(cx, |this: &mut Self, cx| {
                        if let Some(s) = s {
                            this.adopt_snapshot(s, cx);
                        }
                        cx.notify();
                    });
                }
                // The stream ended (socket dropped): reconnect and resubscribe.
                cx.background_executor()
                    .timer(std::time::Duration::from_secs(1))
                    .await;
            }
        })
        .detach();
        cx.notify();
    }

    /// Send the first queued follow-up when the session is ready —
    /// upstream's queuedComposerDrain. Callers: the thread stream loop
    /// (after each event batch) and send() when a pill was just queued
    /// against a possibly stale "running" detail.
    pub(super) fn drain_queued(&mut self, cx: &mut Context<Self>) {
        if self.queue_paused
            || self.sending
            || self.queued_turns.is_empty()
            || session_phase(self.detail.as_ref()) != "ready"
        {
            return;
        }
        let Some(tid) = self.selected_thread.clone() else {
            return;
        };
        let qtext = self.queued_turns.first().cloned().unwrap_or_default();
        let qmode = self.runtime_mode;
        let qimode = self.interaction_mode;
        let Some(client) = self
            .client
            .lock()
            .ok()
            .and_then(|guard| guard.as_ref().cloned())
        else {
            return;
        };
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cmd = json!({
                "type": "thread.turn.start",
                "commandId": uuid(),
                "threadId": tid,
                "message": {
                    "messageId": uuid(),
                    "role": "user",
                    "text": qtext,
                    "attachments": [],
                },
                "runtimeMode": qmode,
                "interactionMode": qimode,
                "createdAt": now_iso(),
            });
            let r = cx
                .background_executor()
                .spawn(async move { client.call("orchestration.dispatchCommand", cmd) })
                .await;
            if r.is_some() {
                let _ = this.update(cx, |this: &mut Self, cx| {
                    if !this.queued_turns.is_empty() {
                        this.queued_turns.remove(0);
                    }
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// Deselect the current thread so the composer starts a new one.
    /// Prefetch markdown images referenced by the loaded thread detail into
    /// the shell cache the TextView image resolver reads. Idempotent: URLs
    /// already cached or in flight are skipped. Failed fetches stay pending,
    /// so a later detail refresh retries them.
    pub(super) fn prefetch_md_images(&mut self, cx: &mut Context<Self>) {
        let Some(detail) = self.detail.clone() else {
            return;
        };
        let thread_v = get(&detail, "thread");
        let mut urls = Vec::new();
        for m in arr(thread_v, "messages") {
            let mut text = str_at(m, "text").to_string();
            if text.trim().is_empty() {
                text = get(m, "textSegments")
                    .as_array()
                    .map(|segs| {
                        segs.iter()
                            .filter_map(|s| s.get("text").and_then(Value::as_str))
                            .collect::<Vec<_>>()
                            .join("")
                    })
                    .unwrap_or_default();
            }
            extract_md_image_urls(&text, &mut urls);
            if let Some(base) = crate::ws_client::BACKEND_BASE.get() {
                for a in arr(m, "attachments") {
                    if str_at(a, "type") == "image" {
                        let id = str_at(a, "id");
                        if !id.is_empty() {
                            urls.push(format!("{base}/attachments/{id}"));
                        }
                    }
                }
            }
        }
        for url in urls {
            if self.md_images.lock().unwrap().contains_key(&url)
                || self.md_images_pending.lock().unwrap().contains(&url)
            {
                continue;
            }
            self.md_images_pending.lock().unwrap().insert(url.clone());
            let cache = self.md_images.clone();
            let pending = self.md_images_pending.clone();
            cx.spawn(async move |this, cx| {
                let fetch_url = url.clone();
                let img = cx
                    .background_executor()
                    .spawn(async move { fetch_decode_image(&fetch_url) })
                    .await;
                if let Some(img) = img {
                    cache.lock().unwrap().insert(url.clone(), img);
                    pending.lock().unwrap().remove(&url);
                    let _ = this.update(cx, |_this, cx| cx.notify());
                }
            })
            .detach();
        }
    }

    pub(super) fn fetch_changes(&mut self, cx: &mut Context<Self>) {
        let Some(cwd) = self.files_root() else {
            self.changes_unavailable = true;
            cx.notify();
            return;
        };
        self.changes_loading = true;
        self.changes_unavailable = false;
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            let c = client.clone();
            let c2 = cwd.clone();
            let res = cx
                .background_executor()
                .spawn(async move { c.call("git.status", json!({"cwd": c2})) })
                .await;
            let c = client.clone();
            let c2 = cwd.clone();
            let staged_res = cx
                .background_executor()
                .spawn(async move {
                    c.call(
                        "git.readWorkingTreeDiff",
                        json!({"cwd": c2, "scope": "staged"}),
                    )
                })
                .await;
            let c2 = cwd.clone();
            let unstaged_res = cx
                .background_executor()
                .spawn(async move {
                    client.call(
                        "git.readWorkingTreeDiff",
                        json!({"cwd": c2, "scope": "unstaged"}),
                    )
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.changes_loading = false;
                match res {
                    Some(_v) => {
                        this.changes_unavailable = false;
                        let parse = |r: Option<Value>| -> Vec<Value> {
                            r.map(|v| {
                                parse_patch_files(str_at(&v, "patch"))
                            })
                            .unwrap_or_default()
                        };
                        this.staged_files = parse(staged_res);
                        this.unstaged_files = parse(unstaged_res);
                    }
                    None => {
                        this.changes_unavailable = true;
                        this.staged_files = Vec::new();
                        this.unstaged_files = Vec::new();
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Code review list: `githubInbox.list` superset for the current state
    /// and sort (upstream githubInboxListQueryOptions). Kind and text stay
    /// client-side filters over that superset.
    pub(super) fn fetch_github_inbox(&mut self, cx: &mut Context<Self>) {
        self.gh_loading = true;
        self.gh_epoch += 1;
        let epoch = self.gh_epoch;
        let state = if self.gh_state_filter == "open" { "open" } else { "closed" };
        let sort = self.gh_sort;
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            let res = cx
                .background_executor()
                .spawn(async move {
                    client.call("githubInbox.list", json!({"state": state, "sort": sort}))
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if this.gh_epoch != epoch {
                    return;
                }
                this.gh_loading = false;
                if let Some(v) = res {
                    this.gh_items = arr(&v, "items").to_vec();
                    this.gh_errors = arr(&v, "errors").to_vec();
                    this.gh_viewer = str_at(&v, "viewer").to_string();
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Cmd+F: transcript find-bar. Highlights matching messages with an accent
    /// bar and shows the match count, like the web find-in-thread bar.
    /// Click on a diff line -> `git.blameLine` then the blame popover
    /// (upstream DiffLineBlamePopover). Deleted lines blame the base
    /// tree (base:"branch"); everything else blames the working tree.
    pub(super) fn fetch_blame(
        &mut self,
        path: String,
        line: u32,
        base: bool,
        x: f32,
        y: f32,
        cx: &mut Context<Self>,
    ) {
        let Some(cwd) = self.files_root() else {
            return;
        };
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else {
                return;
            };
            let mut p = json!({"cwd": cwd, "filePath": path, "line": line});
            if base {
                p["base"] = json!("branch");
            }
            let res = cx
                .background_executor()
                .spawn(async move { client.call("git.blameLine", p) })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if let Some(v) = res {
                    this.blame_pop = Some((v, x, y));
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// Sample the shared frame slot on a fixed cadence: the receiver thread
    /// never touches gpui state, the UI thread copies the newest PNG into
    /// the panel cache and redraws only when the sequence advanced.
    pub(super) fn ensure_cua_refresh(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx: &mut AsyncApp| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(350))
                    .await;
                let alive = this.update(cx, |view, cx| {
                    if !view.view_cua {
                        return false;
                    }
                    let new_frame = view
                        .cua
                        .as_ref()
                        .and_then(|h| h.shared.lock().ok().map(|s| (s.sequence, s.frame_png.clone(), s.connected, s.error.clone())));
                    if let Some((seq, png, _conn, _err)) = new_frame {
                        if seq != view.cua_seen_seq && png.is_some() {
                            view.cua_seen_seq = seq;
                            cx.notify();
                        }
                    }
                    true
                });
                if alive.is_err() || !alive.unwrap_or(false) {
                    break;
                }
            }
        })
        .detach();
    }

    /// Blocking RPC: GitHub inbox (PRs + issues of the project's repos).
    pub(super) fn fetch_inbox(&self, cx: &mut Context<Self>) {
        let cell = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.inbox_loading = true;
                cx.notify();
            });
            let bundle = cx
                .background_executor()
                .spawn(async move {
                    let Some(c) = ensure_client(&cell) else {
                        return Value::Null;
                    };
                    let tasks = c.call("todo.list", json!({})).unwrap_or(Value::Null);
                    // Upstream recapInputForRange: the day's working window
                    // (4am→4am) with HOURLY boundaries — the recap's slots are
                    // per-hour buckets the slot columns and sparkline group.
                    let (from_ms, to_ms, _b) = inbox_day_range();
                    let hourly = |from: f64, to: f64| -> Vec<String> {
                        let mut v = Vec::new();
                        let mut h = from + 3_600_000.0;
                        while h < to {
                            v.push(ms_to_iso(h));
                            h += 3_600_000.0;
                        }
                        v
                    };
                    let today_in = json!({
                        "from": ms_to_iso(from_ms),
                        "to": ms_to_iso(to_ms),
                        "slotBoundaries": hourly(from_ms, to_ms),
                    });
                    let (pfrom, pto, _pb) = inbox_day_range_at(from_ms - 1.0);
                    let prev_in = json!({
                        "from": ms_to_iso(pfrom),
                        "to": ms_to_iso(pto),
                        "slotBoundaries": hourly(pfrom, pto),
                    });
                    let recap = c.call("stats.getRecap", today_in).unwrap_or(Value::Null);
                    let prev_recap = c.call("stats.getRecap", prev_in).unwrap_or(Value::Null);
                    let usage = c
                        .call("server.listProviderUsage", json!({}))
                        .unwrap_or(Value::Null);
                    let acct = c.call("account.status", json!({})).unwrap_or(Value::Null);
                    let provs = c
                        .call_first_chunk("server.subscribeProviderStatuses", json!({}))
                        .unwrap_or(Value::Null);
                    json!({"tasks": tasks, "recap": recap, "prevRecap": prev_recap, "usage": usage, "account": acct, "providers": provs})
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.inbox_loading = false;
                this.inbox_tasks = arr(get(&bundle, "tasks"), "todos")
                    .to_vec()
                    .into_iter()
                    .chain(arr(get(&bundle, "tasks"), "items").to_vec())
                    .chain(if get(&bundle, "tasks").is_array() {
                        arr(&bundle, "tasks").to_vec()
                    } else {
                        Vec::new()
                    })
                    .collect();
                this.inbox_recap = Some(get(&bundle, "recap").clone());
                this.inbox_recap_prev = Some(get(&bundle, "prevRecap").clone());
                this.inbox_usage = arr(get(&bundle, "usage"), "snapshots")
                    .to_vec()
                    .into_iter()
                    .chain(arr(get(&bundle, "usage"), "rows").to_vec())
                    .chain(arr(get(&bundle, "usage"), "providers").to_vec())
                    .collect();
                let acct = get(&bundle, "account");
                let signed = get(acct, "signedIn")
                    .as_bool()
                    .or_else(|| get(acct, "authenticated").as_bool())
                    .or_else(|| Some(!str_at(acct, "accountId").is_empty()));
                this.inbox_signed_in = signed;
                // Upstream getNotifiableProviderUpdateStatuses: behind_latest,
                // latestVersion known, one-click update command present.
                this.inbox_updates = arr(get(&bundle, "providers"), "providers")
                    .to_vec()
                    .into_iter()
                    .filter(|p| {
                        let adv = get(p, "versionAdvisory");
                        str_at(adv, "status") == "behind_latest"
                            && !get(adv, "latestVersion").is_null()
                            && get(adv, "canUpdate").as_bool() == Some(true)
                            && !get(adv, "updateCommand").is_null()
                    })
                    .collect();
                cx.notify();
            });
        })
        .detach();
    }

    /// automation.list → definitions + runs (upstream RailAutomationsPanel).
    pub(super) fn fetch_automations(&self, cx: &mut Context<Self>) {
        let cell = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let bundle = cx
                .background_executor()
                .spawn(async move {
                    let Some(c) = ensure_client(&cell) else {
                        return Value::Null;
                    };
                    c.call("automation.list", json!({})).unwrap_or(Value::Null)
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.autom_defs = arr(&bundle, "definitions").to_vec();
                this.autom_runs = arr(&bundle, "runs").to_vec();
                if let Some(sel) = &this.autom_sel {
                    if !this
                        .autom_defs
                        .iter()
                        .any(|d| str_at(d, "id") == sel.as_str())
                    {
                        this.autom_sel = None;
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Blocking RPC bundle for the settings panes: provider statuses,
    /// usage quotas, skills, connectors, integrations, worktrees,
    /// profile stats, computer backend status, archived thread count.
    pub(super) fn fetch_settings(&self, cx: &mut Context<Self>) {
        let cell = self.client.clone();
        let cell_tail = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.settings_loading = true;
                cx.notify();
            });
            // Stage 1: the critical server-owned settings hydrate first. The
            // heavy tail below is a chain of blocking serial RPCs (up to 60s
            // each) and any slow or hung call down there must not starve
            // appearance/sidebar hydration — it used to never land at all.
            let critical = cx
                .background_executor()
                .spawn(async move {
                    let Some(c) = ensure_client(&cell) else {
                        return Value::Null;
                    };
                    let cfg = c.call("server.getConfig", json!({})).unwrap_or(Value::Null);
                    let app = c.call("server.getSettings", json!({})).unwrap_or(Value::Null);
                    let usage = c.call("server.listProviderUsage", json!({})).unwrap_or(Value::Null);
                    json!({"config":cfg,"settings":app,"usage":usage})
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if let Some(rules) = critical
                    .get("config")
                    .and_then(|c| c.get("keybindings"))
                    .and_then(|k| k.as_array())
                    .cloned()
                {
                    this.kb_rules = rules;
                }
                this.settings_loading = false;
                this.settings_data = critical;
                native_hydrate_settings(this, cx);
                cx.notify();
            });
            // Stage 2: the heavy tail merges into settings_data whenever it
            // finishes; nothing user-visible in the chrome depends on it.
            let tail = cx
                .background_executor()
                .spawn(async move {
                    let Some(c) = ensure_client(&cell_tail) else {
                        return Value::Null;
                    };
                    let skills = c.call("provider.listSkills", json!({})).unwrap_or(Value::Null);
                    let conns = c.call("server.listMcpConnectors", json!({})).unwrap_or(Value::Null);
                    let props = c.call("server.listMcpConnectorProposals", json!({})).unwrap_or(Value::Null);
                    let exts = c.call("server.listExternalMcpIntegrations", json!({})).unwrap_or(Value::Null);
                    let wts = c.call("server.listWorktrees", json!({})).unwrap_or(Value::Null);
                    let prof = c.call("stats.getProfileStats", json!({})).unwrap_or(Value::Null);
                    let toks = c.call("stats.getProfileTokenStats", json!({})).unwrap_or(Value::Null);
                    // Upstream default "30d" range, previous period included
                    // (usageAnalyticsWindow): from = local midnight today-59,
                    // to = local midnight tomorrow, scope "synara".
                    let today = chrono::Local::now().date_naive();
                    // Upstream serializes with Date.toISOString(): UTC "Z"
                    // (usageAnalyticsWindow), not a local-offset RFC3339.
                    let ana_from = today
                        .checked_sub_days(chrono::Days::new(59))
                        .map(|d| d.and_hms_opt(0, 0, 0).unwrap())
                        .map(|d| {
                            d.and_local_timezone(chrono::Local)
                                .unwrap()
                                .with_timezone(&chrono::Utc)
                                .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                                .to_string()
                        });
                    let ana_to = today
                        .checked_add_days(chrono::Days::new(1))
                        .map(|d| d.and_hms_opt(0, 0, 0).unwrap())
                        .map(|d| {
                            d.and_local_timezone(chrono::Local)
                                .unwrap()
                                .with_timezone(&chrono::Utc)
                                .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                                .to_string()
                        });
                    let ana = match (ana_from, ana_to) {
                        (Some(f), Some(t)) => c
                            .call(
                                "stats.getUsageAnalytics",
                                json!({"from": f, "to": t, "scope": "synara"}),
                            )
                            .unwrap_or(Value::Null),
                        _ => Value::Null,
                    };
                    let comp = c.call("computer.getStatus", json!({})).unwrap_or(Value::Null);
                    let snap = c.call("orchestration.getShellSnapshot", json!({})).unwrap_or(Value::Null);
                    let acct = c.call("account.status", json!({})).unwrap_or(Value::Null);
                    json!({"skills":skills,
                           "connectors":conns,"proposals":props,"external":exts,
                           "worktrees":wts,"profile":prof,"tokens":toks,"analytics":ana,
                           "computer":comp,"snapshot":snap,"account":acct})
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if let (Some(dst), Some(src)) = (
                    this.settings_data.as_object_mut(),
                    tail.as_object(),
                ) {
                    for (k, v) in src {
                        dst.insert(k.clone(), v.clone());
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn refresh_vault(&mut self, cx: &mut Context<Self>) {
        match browserhost_rs::cdp::shared_browser_vault().and_then(|vault| vault.snapshot()) {
            Ok(snapshot) => {
                self.vault_snapshot = Some(snapshot);
                self.vault_error = None;
            }
            Err(error) => {
                self.vault_error = Some(error);
            }
        }
        cx.notify();
    }

    pub(super) fn refresh_cookie_sources(&mut self, cx: &mut Context<Self>) {
        let mut rows = Vec::new();
        for source in browserhost_rs::cookies::sources() {
            let mut source = source;
            if let Some(id) = source.get("id").and_then(Value::as_str) {
                if let Ok(profiles) = browserhost_rs::cookies::profiles(id) {
                    if let Some(object) = source.as_object_mut() {
                        object.insert("profiles".into(), Value::Array(profiles));
                    }
                }
            }
            rows.push(source);
        }
        self.cookie_sources = rows;
        cx.notify();
    }

    /// Blocking RPC: list running dev servers.
    pub(super) fn fetch_dev_servers(&self, cx: &mut Context<Self>) {
        let cell = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let v = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell)
                        .and_then(|c| c.call("projects.listDevServers", json!({})))
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.dev_servers = v.map(|v| arr(&v, "servers").to_vec()).unwrap_or_default();
                cx.notify();
            });
        })
        .detach();
    }

    /// Fetch git.status of the selected thread's project root (fallback: first project).
    pub(super) fn fetch_git_status(&self, cx: &mut Context<Self>) {
        let Some(root) = self
            .thread_project_root()
            .or_else(|| self.files_root())
        else {
            return;
        };
        let cell = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let v = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell)
                        .and_then(|c| c.call("git.status", json!({"cwd": root})))
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.git_status = v;
                cx.notify();
            });
        })
        .detach();
    }
}
