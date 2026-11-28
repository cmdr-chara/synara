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
    pub(super) fn open_thread(&mut self, thread_id: String, cx: &mut Context<Self>) {
        // Close the previous thread stream if any.
        let old = self.thread_stream_id.take();
        self.selected_thread = Some(thread_id.clone());
        // Upstream markThreadVisited: lastVisitedAt is monotonic.
        {
            let prev = self
                .visited_threads
                .get(&thread_id)
                .cloned()
                .unwrap_or_default();
            let now = now_iso();
            if now > prev {
                self.visited_threads.insert(thread_id.clone(), now);
                self.persist_sidebar_ui(cx);
            }
        }
        self.renaming = false;
        // Selecting a thread always returns to the transcript surface.
        self.clear_views();
        self.fetch_git_status(cx);
        if !self.open_threads.contains(&thread_id) {
            self.open_threads.push(thread_id.clone());
        }
        self.detail = None;
        let client = self.client.clone();
        let tid = thread_id;
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            if let Some(old) = old {
                client.interrupt(&old);
            }
            let payload = json!({"threadId": tid});
            let c = client.clone();
            let p = payload.clone();
            let detail = cx
                .background_executor()
                .spawn(async move { c.call("orchestration.getThreadDetailSnapshot", p) })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if this.selected_thread.as_deref() == Some(tid.as_str()) {
                    this.detail = detail;
                    this.transcript_scroll.scroll_to_bottom();
                    this.prefetch_md_images(cx);
                }
                cx.notify();
            });
            // Live transcript via subscribeThread: each chunk triggers a detail
            // refetch (chunk shapes vary; refetch is schema-free).
            let c = client.clone();
            let (rid, frames) = match c.request("orchestration.subscribeThread", payload.clone()) {
                Some(v) => v,
                None => return,
            };
            let rid_for_ui = rid.clone();
            let _ = this.update(cx, |this: &mut Self, _| {
                this.thread_stream_id = Some(rid_for_ui);
            });
            let (tx, rx) = async_channel::unbounded::<()>();
            let c = client.clone();
            cx.background_executor()
                .spawn(async move {
                    while let Ok(msg) = frames.recv() {
                        match msg.get("_tag").and_then(Value::as_str) {
                            Some("Chunk") => {
                                let _ = tx.send_blocking(());
                            }
                            Some("Exit") => break,
                            _ => {}
                        }
                    }
                    c.finish_request(&rid);
                })
                .detach();
            while rx.recv().await.is_ok() {
                let c = client.clone();
                let p = payload.clone();
                let d = cx
                    .background_executor()
                    .spawn(async move { c.call("orchestration.getThreadDetailSnapshot", p) })
                    .await;
                let _ = this.update(cx, |this: &mut Self, cx| {
                    if this.selected_thread.as_deref() == Some(tid.as_str()) {
                        // Keep end-follow while the reader is at the bottom
                        // (upstream tail anchor); releasing upward keeps the
                        // position and shows the scroll-to-bottom button.
                        let off = this.transcript_scroll.offset();
                        let max = this.transcript_scroll.max_offset();
                        let at_bottom = max.y + off.y <= px(120.);
                        this.detail = d;
                        if at_bottom {
                            this.transcript_scroll.scroll_to_bottom();
                        }
                        this.prefetch_md_images(cx);
                    }
                    cx.notify();
                });
                // Queued follow-ups drain once the turn settles (upstream's
                // queuedComposerDrain; a user Stop holds them instead).
                let _ = this.update(cx, |this: &mut Self, cx| this.drain_queued(cx));
            }
            // Stream ended (socket dropped): clear the id so the next send or
            // reopen re-subscribes instead of leaving a dead stream marker.
            let _ = this.update(cx, |this: &mut Self, cx| {
                if this.selected_thread.as_deref() == Some(tid.as_str()) {
                    this.thread_stream_id = None;
                    cx.notify();
                }
            });
        })
        .detach();
        cx.notify();
    }

    pub(super) fn new_thread(&mut self, cx: &mut Context<Self>) {
        if let Some(old) = self.thread_stream_id.take() {
            if let Ok(guard) = self.client.lock() {
                if let Some(client) = guard.as_ref() {
                    client.interrupt(&old);
                }
            }
        }
        self.selected_thread = None;
        self.detail = None;
        self.renaming = false;
        cx.notify();
    }

    /// Close one open tab; if it was selected, open the last remaining tab.
    pub(super) fn close_tab(&mut self, thread_id: String, cx: &mut Context<Self>) {
        self.open_threads.retain(|t| t != &thread_id);
        if self.selected_thread.as_deref() == Some(thread_id.as_str()) {
            match self.open_threads.last().cloned() {
                Some(next) => self.open_thread(next, cx),
                None => self.new_thread(cx),
            }
        } else {
            cx.notify();
        }
    }

    /// Upstream closeTabsInScope: close every open tab in one scope of the
    /// tab whose menu was used ("left" | "right" | "others").
    pub(super) fn close_tabs_scope(
        &mut self,
        thread_id: &str,
        scope: &str,
        cx: &mut Context<Self>,
    ) {
        let Some(idx) = self.open_threads.iter().position(|t| t == thread_id) else {
            return;
        };
        let keep = |i: usize, t: &String| match scope {
            "left" => i >= idx,
            "right" => i <= idx,
            _ => t == thread_id,
        };
        self.open_threads = self
            .open_threads
            .iter()
            .enumerate()
            .filter(|(i, t)| keep(*i, *t))
            .map(|(_, t)| t.clone())
            .collect();
        match self.selected_thread.as_deref() {
            Some(sel) if !self.open_threads.iter().any(|t| t == sel) => {
                self.open_thread(thread_id.to_string(), cx);
            }
            None => {
                self.open_thread(thread_id.to_string(), cx);
            }
            _ => cx.notify(),
        }
    }

    pub(super) fn toggle_pin(&mut self, thread_id: String, cx: &mut Context<Self>) {
        if self.pinned_threads.contains(&thread_id) {
            self.pinned_threads.retain(|t| t != &thread_id);
        } else {
            self.pinned_threads.push(thread_id);
        }
        cx.notify();
    }

    pub(super) fn start_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let title = str_at(
            get(self.detail.as_ref().unwrap_or(&Value::Null), "thread"),
            "title",
        )
        .to_string();
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Thread title"));
        input.update(cx, |s, cx| {
            s.set_value(title, window, cx);
        });
        cx.subscribe_in(&input, window, |this: &mut Self, _s, event, _w, cx| {
            if let InputEvent::PressEnter { .. } = event {
                this.commit_rename(cx);
            }
        })
        .detach();
        let focus = input.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        self.rename_input = Some(input);
        self.renaming = true;
        cx.notify();
    }

    pub(super) fn commit_rename(&mut self, cx: &mut Context<Self>) {
        let Some(input) = self.rename_input.take() else {
            return;
        };
        let title = input.read(cx).value().trim().to_string();
        self.renaming = false;
        if title.is_empty() {
            cx.notify();
            return;
        }
        let Some(tid) = self.selected_thread.clone() else {
            cx.notify();
            return;
        };
        if let Some(detail) = self.detail.as_mut() {
            if let Some(t) = detail.get_mut("thread") {
                t["title"] = json!(title);
            }
        }
        let client = self.client.clone();
        cx.spawn(async move |_this, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            let payload = json!({
                "type": "thread.meta.update",
                "commandId": uuid(),
                "threadId": tid,
                "title": title,
            });
            cx.background_executor()
                .spawn(async move { client.call("orchestration.dispatchCommand", payload) })
                .detach();
        })
        .detach();
        cx.notify();
    }

    /// Cycle open thread tabs by offset (wraps).
    pub(super) fn cycle_tab(&mut self, offset: i64, cx: &mut Context<Self>) {
        let n = self.open_threads.len();
        if n == 0 {
            return;
        }
        let cur = self
            .selected_thread
            .as_ref()
            .and_then(|s| self.open_threads.iter().position(|t| t == s))
            .unwrap_or(0) as i64;
        let next = ((cur + offset) % n as i64 + n as i64) as usize % n;
        let t = self.open_threads[next].clone();
        self.open_thread(t, cx);
    }

    /// ⌥⇧⌘A — archive the selected thread via dispatchCommand, then close
    /// its tab (the shell snapshot refresh removes it from the sidebar).
    pub(super) fn archive_selected(&mut self, cx: &mut Context<Self>) {
        let Some(tid) = self.selected_thread.clone() else {
            return;
        };
        let client = self.client.clone();
        let tid_cmd = tid.clone();
        cx.spawn(async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            let payload = json!({
                "type": "thread.archive",
                "commandId": uuid(),
                "threadId": tid_cmd,
            });
            cx.background_executor()
                .spawn(async move { client.call("orchestration.dispatchCommand", payload) })
                .detach();
        })
        .detach();
        self.close_tab(tid, cx);
    }

    /// Context-menu actions: fork or hand off a thread. Both dispatch the
    /// lineage command upstream's threadContextMenu sends, then refetch the
    /// snapshot so the new row appears in the sidebar.
    pub(super) fn lineage(
        &mut self,
        tid: String,
        is_handoff: bool,
        env_mode: Option<&'static str>,
        cx: &mut Context<Self>,
    ) {
        let project_id = self
            .snapshot
            .as_ref()
            .and_then(|s| {
                arr(s, "threads").iter().find_map(|t| {
                    if str_at(t, "id") == tid {
                        Some(str_at(t, "projectId").to_string())
                    } else {
                        None
                    }
                })
            })
            .unwrap_or_else(|| "default".to_string());
        let client = self.client.clone();
        let ctype = if is_handoff {
            "thread.handoff.create"
        } else {
            "thread.fork.create"
        };
        cx.background_executor()
            .spawn(async move {
                let Some(client) = ensure_client(&client) else {
                    return;
                };
                let mut payload = json!({
                    "type": ctype,
                    "commandId": uuid(),
                    "threadId": uuid(),
                    "projectId": project_id,
                    "sourceThreadId": tid,
                    "createdAt": now_iso(),
                });
                if let Some(mode) = env_mode {
                    payload["envMode"] = json!(mode);
                }
                client.call("orchestration.dispatchCommand", payload);
            })
            .detach();
    }

    /// Upstream resolveThreadWorkspaceCwd: worktree threads resolve to their
    /// worktreePath (or a workingDirectory inside it); local threads resolve
    /// to workingDirectory or the project's workspaceRoot.
    pub(super) fn thread_workspace_cwd(&self, tid: &str) -> Option<String> {
        let s = self.snapshot.as_ref()?;
        let t = arr(s, "threads").iter().find(|t| str_at(t, "id") == tid)?;
        let env_mode = str_at(t, "envMode");
        let wt = str_at(t, "worktreePath");
        let wd = str_at(t, "workingDirectory");
        if env_mode == "worktree" {
            if !wt.is_empty() && !wd.is_empty() && wd.starts_with(wt) {
                return Some(wd.to_string());
            }
            return (!wt.is_empty()).then(|| wt.to_string());
        }
        if !wd.is_empty() {
            return Some(wd.to_string());
        }
        let pid = str_at(t, "projectId");
        arr(s, "projects").iter().find_map(|p| {
            if str_at(p, "id") != pid {
                return None;
            }
            let r = str_at(p, "workspaceRoot");
            let r = if r.is_empty() {
                str_at(p, "workspace_root")
            } else {
                r
            };
            (!r.is_empty()).then(|| r.to_string())
        })
    }

    /// Context-menu Delete: dispatch thread.delete, unpin the row and close
    /// its tab, like upstream deleteThread.
    pub(super) fn delete_thread(&mut self, tid: String, cx: &mut Context<Self>) {
        self.pinned_threads.retain(|p| p != &tid);
        let client = self.client.clone();
        let tid_cmd = tid.clone();
        cx.spawn(async move |_this, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            let payload = json!({
                "type": "thread.delete",
                "commandId": uuid(),
                "threadId": tid_cmd,
            });
            cx.background_executor()
                .spawn(async move { client.call("orchestration.dispatchCommand", payload) })
                .detach();
        })
        .detach();
        self.close_tab(tid, cx);
    }

    pub(super) fn insert_goal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let composer = self.composer.clone();
        self.composer.update(cx, |state, cx| {
            let current = state.value().to_string();
            if !current.trim_start().to_lowercase().starts_with("/goal") {
                let next = if current.trim().is_empty() {
                    "/goal ".to_string()
                } else {
                    format!("/goal {}", current)
                };
                state.set_value(next, window, cx);
            }
        });
        let focus = composer.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        cx.notify();
    }

    /// Toggle a task's completed state from the Inbox tile (upstream
    /// TaskListItem checkbox → todo.update {completed}).
    pub(super) fn toggle_todo(&mut self, id: String, done: bool, cx: &mut Context<Self>) {
        // Optimistic flip so the row repaints before the round-trip lands.
        for task in self.inbox_tasks.iter_mut() {
            if str_at(task, "id") == id {
                if let Some(m) = task.as_object_mut() {
                    m.insert(
                        "completedAt".into(),
                        if done {
                            json!(chrono::Utc::now().to_rfc3339())
                        } else {
                            Value::Null
                        },
                    );
                }
            }
        }
        let cell = self.client.clone();
        cx.spawn(async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let id = id.clone();
            let _ = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell)
                        .and_then(|c| c.call("todo.update", json!({"id": id, "completed": done})))
                })
                .await;
        })
        .detach();
        cx.notify();
    }

    /// Workspace root of the selected thread's project, if resolvable.
    pub(super) fn thread_project_root(&self) -> Option<String> {
        let tid = self.selected_thread.as_deref()?;
        let snap = self.snapshot.as_ref()?;
        let pid = arr(snap, "threads")
            .iter()
            .find(|t| str_at(t, "id") == tid)
            .map(|t| str_at(t, "projectId").to_string())?;
        let root = arr(snap, "projects")
            .iter()
            .find(|p| str_at(p, "id") == pid)
            .map(|p| {
                let r = str_at(p, "workspaceRoot");
                if r.is_empty() {
                    str_at(p, "workspace_root")
                } else {
                    r
                }
            })
            .filter(|r| !r.is_empty());
        root.map(|r| r.to_string())
    }

    pub(super) fn open_split_thread(&mut self, thread_id: String, cx: &mut Context<Self>) {
        let old = self.split_stream_id.take();
        self.split_thread = Some(thread_id.clone());
        self.focused_pane = 1;
        self.split_detail = None;
        let client = self.client.clone();
        let tid = thread_id;
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            if let Some(old) = old {
                client.interrupt(&old);
            }
            let payload = json!({"threadId": tid});
            let c = client.clone();
            let p = payload.clone();
            let detail = cx
                .background_executor()
                .spawn(async move { c.call("orchestration.getThreadDetailSnapshot", p) })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if this.split_thread.as_deref() == Some(tid.as_str()) {
                    this.split_detail = detail;
                }
                cx.notify();
            });
            // Live transcript via subscribeThread (same pattern as the
            // primary pane): each chunk triggers a detail refetch.
            let c = client.clone();
            let (rid, frames) = match c.request("orchestration.subscribeThread", payload.clone()) {
                Some(v) => v,
                None => return,
            };
            let rid_for_ui = rid.clone();
            let _ = this.update(cx, |this: &mut Self, _| {
                this.split_stream_id = Some(rid_for_ui);
            });
            let (tx, rx) = async_channel::unbounded::<()>();
            let c = client.clone();
            cx.background_executor()
                .spawn(async move {
                    while let Ok(msg) = frames.recv() {
                        match msg.get("_tag").and_then(Value::as_str) {
                            Some("Chunk") => {
                                let _ = tx.send_blocking(());
                            }
                            Some("Exit") => break,
                            _ => {}
                        }
                    }
                    c.finish_request(&rid);
                })
                .detach();
            while rx.recv().await.is_ok() {
                let c = client.clone();
                let p = payload.clone();
                let d = cx
                    .background_executor()
                    .spawn(async move { c.call("orchestration.getThreadDetailSnapshot", p) })
                    .await;
                let _ = this.update(cx, |this: &mut Self, cx| {
                    if this.split_thread.as_deref() == Some(tid.as_str()) {
                        this.split_detail = d;
                    }
                    cx.notify();
                });
            }
        })
        .detach();
        cx.notify();
    }

    pub(super) fn close_split(&mut self, cx: &mut Context<Self>) {
        if let Some(rid) = self.split_stream_id.take() {
            let cell = self.client.clone();
            cx.background_executor()
                .spawn(async move {
                    if let Some(c) = ensure_client(&cell) {
                        c.interrupt(&rid);
                    }
                })
                .detach();
        }
        self.split_open = false;
        self.split_thread = None;
        self.split_detail = None;
        self.focused_pane = 0;
        cx.notify();
    }

    pub(super) fn send_split(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(input) = self.split_composer.clone() else {
            return;
        };
        let text = input.read(cx).value().trim().to_string();
        let Some(tid) = self.split_thread.clone() else {
            return;
        };
        if text.is_empty() || self.sending {
            return;
        }
        self.sending = true;
        input.update(cx, |s, cx| s.clean(window, cx));
        let client = self.client.clone();
        let mode = self.runtime_mode;
        let imode = self.interaction_mode;
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else {
                let _ = this.update(cx, |this: &mut Self, cx| {
                    this.sending = false;
                    cx.notify();
                });
                return;
            };
            let cmd = json!({
                "type": "thread.turn.start",
                "commandId": uuid(),
                "threadId": tid,
                "message": {
                    "messageId": uuid(),
                    "role": "user",
                    "text": text,
                    "attachments": [],
                },
                "runtimeMode": mode,
                "interactionMode": imode,
                "createdAt": now_iso(),
            });
            let c = client.clone();
            let _ = cx
                .background_executor()
                .spawn(async move { c.call("orchestration.dispatchCommand", cmd) })
                .await;
            let p = json!({"threadId": tid});
            let c = client.clone();
            let d = cx
                .background_executor()
                .spawn(async move { c.call("orchestration.getThreadDetailSnapshot", p) })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.sending = false;
                if let Some(d) = d {
                    this.split_detail = Some(d);
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
}
