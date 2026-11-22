//! Native gpui-kit workspace: sidebar (spaces/projects/threads) + transcript +
//! composer, fed by rust/app's native WS-RPC (getShellSnapshot /
//! getThreadDetailSnapshot / dispatchCommand relayed upstream).

use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

use gpui_kit::component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputEvent, InputState},
    scroll::ScrollableElement as _,
    v_flex,
    ActiveTheme as _,
};
use gpui_kit::*;

use crate::ws_client::Client;

type SharedClient = Arc<Mutex<Option<Arc<Client>>>>;

pub struct Native {
    focus_handle: FocusHandle,
    client: SharedClient,
    snapshot: Option<Value>,
    detail: Option<Value>,
    selected_thread: Option<String>,
    thread_stream_id: Option<String>,
    composer: Entity<InputState>,
    search: Entity<InputState>,
    sending: bool,
    view_files: bool,
    files_dir: String,
    files: Vec<Value>,
    open_file: Option<(String, String)>,
    files_loading: bool,
    git_status: Option<Value>,
    file_stream_id: Option<String>,
}

fn get<'a>(v: &'a Value, k: &str) -> &'a Value {
    v.get(k).unwrap_or(&Value::Null)
}

fn str_at<'a>(v: &'a Value, k: &str) -> &'a str {
    get(v, k).as_str().unwrap_or("")
}

fn arr<'a>(v: &'a Value, k: &str) -> &'a [Value] {
    get(v, k).as_array().map(Vec::as_slice).unwrap_or(&[])
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Blocking: connect (once) and clone the shared client. Run on a background
/// executor.
fn ensure_client(cell: &SharedClient) -> Option<Arc<Client>> {
    let mut guard = cell.lock().ok()?;
    let dead = guard.as_ref().is_none_or(|c| !c.is_alive());
    if dead {
        *guard = Client::connect();
    }
    guard.clone()
}

impl Native {
    pub fn new(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let composer = cx.new(|cx| {
                InputState::new(window, cx).placeholder("Do anything in Synara")
            });
            cx.subscribe_in(
                &composer,
                window,
                |this: &mut Self, _state, event, window, cx| {
                    if let InputEvent::PressEnter { shift, .. } = event {
                        if !shift {
                            this.send(window, cx);
                        }
                    }
                },
            )
            .detach();
            let search = cx.new(|cx| {
                InputState::new(window, cx).placeholder("Search threads")
            });
            cx.subscribe_in(
                &search,
                window,
                |_this: &mut Self, _state, event, _window, cx| {
                    if let InputEvent::Change = event {
                        cx.notify();
                    }
                },
            )
            .detach();
            let this = Self {
                focus_handle: cx.focus_handle(),
                client: Arc::new(Mutex::new(None)),
                snapshot: None,
                detail: None,
                selected_thread: None,
                thread_stream_id: None,
                composer,
                search,
                sending: false,
                view_files: false,
                files_dir: String::new(),
                files: Vec::new(),
                open_file: None,
                files_loading: false,
                git_status: None,
                file_stream_id: None,
            };
            this.refresh(cx);
            this
        })
    }

    /// (Re)fetch the shell snapshot, then keep it live via subscribeShell.
    fn refresh(&self, cx: &mut Context<Self>) {
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
                let _ = this.update(cx, |this: &mut Self, cx| {
                    if let Some(s) = snapshot {
                        this.snapshot = Some(s);
                    }
                    this.fetch_git_status(cx);
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
                            this.snapshot = Some(s);
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

    fn open_thread(&mut self, thread_id: String, cx: &mut Context<Self>) {
        // Close the previous thread stream if any.
        let old = self.thread_stream_id.take();
        self.selected_thread = Some(thread_id.clone());
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
                        this.detail = d;
                    }
                    cx.notify();
                });
            }
        })
        .detach();
        cx.notify();
    }

    /// Deselect the current thread so the composer starts a new one.
    fn new_thread(&mut self, cx: &mut Context<Self>) {
        if let Some(old) = self.thread_stream_id.take() {
            if let Ok(guard) = self.client.lock() {
                if let Some(client) = guard.as_ref() {
                    client.interrupt(&old);
                }
            }
        }
        self.selected_thread = None;
        self.detail = None;
        cx.notify();
    }

    /// Workspace root of the first project — the file browser starts there.
    fn files_root(&self) -> Option<String> {
        let p = arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects")
            .first()?;
        let root = {
            let r = str_at(p, "workspaceRoot");
            if r.is_empty() {
                str_at(p, "workspace_root")
            } else {
                r
            }
        };
        if root.is_empty() {
            None
        } else {
            Some(root.to_string())
        }
    }

    fn toggle_files(&mut self, cx: &mut Context<Self>) {
        self.view_files = !self.view_files;
        if self.view_files && self.files_dir.is_empty() {
            if let Some(root) = self.files_root() {
                self.load_dir(root, cx);
            }
        }
        cx.notify();
    }

    /// Fetch git.status of the first project's workspace root.
    fn fetch_git_status(&self, cx: &mut Context<Self>) {
        let Some(root) = self.files_root() else {
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

    /// Blocking RPC: list one directory level (dirs + files).
    fn load_dir(&mut self, dir: String, cx: &mut Context<Self>) {
        self.files_loading = true;
        self.open_file = None;
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let d = dir.clone();
            let entries = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell).and_then(|c| {
                        c.call(
                            "projects.listDirectories",
                            json!({"cwd": d, "depth": 1, "includeFiles": true}),
                        )
                    })
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.files_loading = false;
                if let Some(v) = entries {
                    let mut es = arr(&v, "entries").to_vec();
                    es.sort_by_key(|e| {
                        if str_at(e, "kind") == "directory" {
                            0
                        } else {
                            1
                        }
                    });
                    this.files = es;
                    this.files_dir = dir;
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Open a file entry for reading, or descend into a directory entry.
    fn open_entry(&mut self, entry: Value, cx: &mut Context<Self>) {
        let kind = str_at(&entry, "kind").to_string();
        let path = str_at(&entry, "path").to_string();
        if kind == "directory" {
            self.load_dir(path, cx);
            return;
        }
        if kind != "file" {
            return;
        }
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let p = path.clone();
            let v = cx
                .background_executor()
                .spawn(async move {
                    let dir = p
                        .rsplit_once('/')
                        .map(|(d, _)| d.to_string())
                        .unwrap_or_else(|| p.clone());
                    let name = p.rsplit('/').next().unwrap_or(&p).to_string();
                    ensure_client(&cell).and_then(|c| {
                        c.call(
                            "projects.readFile",
                            json!({"cwd": dir, "relativePath": name}),
                        )
                    })
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if let Some(v) = v {
                    let text = str_at(&v, "contents").to_string();
                    this.open_file = Some((path, text));
                    this.watch_open_file(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Subscribe to change events for the currently open file; each event
    /// triggers a re-read so the viewer stays live.
    fn watch_open_file(&mut self, cx: &mut Context<Self>) {
        self.stop_file_watch();
        let Some((path, _)) = self.open_file.clone() else {
            return;
        };
        let cell = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let (tx, rx) = async_channel::unbounded::<()>();
            let cell2 = cell.clone();
            let p = path.clone();
            let got = cx
                .background_executor()
                .spawn(async move {
                    let dir = p
                        .rsplit_once('/')
                        .map(|(d, _)| d.to_string())
                        .unwrap_or_else(|| p.clone());
                    let name = p.rsplit('/').next().unwrap_or(&p).to_string();
                    ensure_client(&cell2).and_then(|c| {
                        c.request(
                            "projects.subscribeFileChange",
                            json!({"cwd": dir, "relativePath": name}),
                        )
                    })
                })
                .await;
            let Some((rid, frames)) = got else {
                return;
            };
            let _ = this.update(cx, |this: &mut Self, _cx| {
                this.file_stream_id = Some(rid);
            });
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
                })
                .detach();
            while rx.recv().await.is_ok() {
                let cell3 = cell.clone();
                let p2 = path.clone();
                let v = cx
                    .background_executor()
                    .spawn(async move {
                        let dir = p2
                            .rsplit_once('/')
                            .map(|(d, _)| d.to_string())
                            .unwrap_or_else(|| p2.clone());
                        let name = p2.rsplit('/').next().unwrap_or(&p2).to_string();
                        ensure_client(&cell3).and_then(|c| {
                            c.call(
                                "projects.readFile",
                                json!({"cwd": dir, "relativePath": name}),
                            )
                        })
                    })
                    .await;
                let still_open = this
                    .update(cx, |this: &mut Self, cx| {
                        if let Some(v) = v {
                            if this
                                .open_file
                                .as_ref()
                                .is_some_and(|(p, _)| *p == path)
                            {
                                let text = str_at(&v, "contents").to_string();
                                this.open_file = Some((path.clone(), text));
                            }
                        }
                        cx.notify();
                    })
                    .is_ok();
                if !still_open {
                    break;
                }
            }
        })
        .detach();
    }

    fn stop_file_watch(&mut self) {
        if let Some(old) = self.file_stream_id.take() {
            if let Ok(g) = self.client.lock() {
                if let Some(c) = g.as_ref() {
                    c.interrupt(&old);
                }
            }
        }
    }

    fn close_file(&mut self, cx: &mut Context<Self>) {
        self.stop_file_watch();
        self.open_file = None;
        cx.notify();
    }

    /// Composer: send the draft as a turn on the selected thread, or create a
    /// thread in the first project then start the turn.
    fn send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.composer.read(cx).value().to_string();
        let text = text.trim().to_string();
        if text.is_empty() || self.sending {
            return;
        }
        self.sending = true;
        self.composer.update(cx, |s, cx| s.clean(window, cx));
        let selected = self.selected_thread.clone();
        let project_id = arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects")
            .first()
            .and_then(|p| p.get("id"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else {
                eprintln!("native: send: no ws client");
                let _ = this.update(cx, |this: &mut Self, cx| {
                    this.sending = false;
                    cx.notify();
                });
                return;
            };
            let model_selection = json!({
                "provider": "codex",
                "instanceId": "codex",
                "model": "gpt-6-astra",
            });
            let tid = selected.clone().unwrap_or_else(uuid);
            let mut cmds = Vec::new();
            if selected.is_none() {
                if let Some(pid) = project_id {
                    cmds.push(json!({
                        "type": "thread.create",
                        "commandId": uuid(),
                        "threadId": tid,
                        "projectId": pid,
                        "title": text.chars().take(60).collect::<String>(),
                        "modelSelection": model_selection,
                        "runtimeMode": "full-access",
                        "interactionMode": "default",
                        "envMode": "local",
                        "branch": null,
                        "worktreePath": null,
                        "createdAt": now_iso(),
                    }));
                }
            }
            cmds.push(json!({
                "type": "thread.turn.start",
                "commandId": uuid(),
                "threadId": tid,
                "message": {
                    "messageId": uuid(),
                    "role": "user",
                    "text": text,
                    "attachments": [],
                },
                "runtimeMode": "full-access",
                "interactionMode": "default",
                "createdAt": now_iso(),
            }));
            for cmd in cmds {
                let c = client.clone();
                let r = cx
                    .background_executor()
                    .spawn(async move { c.call("orchestration.dispatchCommand", cmd) })
                    .await;
                if r.is_none() {
                    eprintln!("native: dispatchCommand failed");
                } else {
                }
            }
            // Refresh snapshot + detail.
            let c = client.clone();
            let s = cx
                .background_executor()
                .spawn(async move { c.call("orchestration.getShellSnapshot", json!({})) })
                .await;
            let p = json!({"threadId": tid});
            let c = client.clone();
            let d = cx
                .background_executor()
                .spawn(async move { c.call("orchestration.getThreadDetailSnapshot", p) })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.sending = false;
                if let Some(s) = s {
                    this.snapshot = Some(s);
                }
                this.selected_thread = Some(tid.clone());
                if let Some(d) = d {
                    this.detail = Some(d);
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
}

impl Focusable for Native {
    fn focus_handle(&self, _cx: &gpui_kit::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

fn row(
    id: impl Into<ElementId>,
    title: String,
    subtitle: String,
    selected: bool,
    cx: &mut Context<Native>,
    on_click: impl Fn(&mut Native, &ClickEvent, &mut Window, &mut Context<Native>) + 'static,
) -> impl IntoElement {
    let theme = cx.theme();
    let mut el = v_flex()
        .id(id)
        .w_full()
        .px_3()
        .py_2()
        .gap_1()
        .cursor_pointer()
        .rounded_md()
        .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(title))
        .child(div().text_xs().text_color(theme.muted_foreground).child(subtitle));
    if selected {
        el = el.bg(theme.list_active);
    }
    el.on_click(cx.listener(on_click))
}

impl Render for Native {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        // ---- sidebar ----
        let mut sidebar = v_flex()
            .w(px(280.))
            .h_full()
            .gap_1()
            .p_2()
            .overflow_y_scrollbar();
        let query = self.search.read(cx).value().to_string().to_lowercase();
        sidebar = sidebar.child(
            h_flex()
                .gap_2()
                .child(div().flex_1().child(Input::new(&self.search)))
                .child(
                    Button::new("new-thread")
                        .ghost()
                        .label("+")
                        .on_click(cx.listener(|this, _ev, _w, cx| this.new_thread(cx))),
                )
                .child(
                    Button::new("toggle-files")
                        .ghost()
                        .label(if self.view_files { "Chat" } else { "Files" })
                        .on_click(cx.listener(|this, _ev, _w, cx| this.toggle_files(cx))),
                ),
        );
        for space in arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "spaces") {
            sidebar = sidebar.child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.muted_foreground)
                    .child(str_at(space, "name").to_uppercase()),
            );
        }
        for project in arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects") {
            let mut proj_head = v_flex().gap_0().child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(str_at(project, "title").to_string()),
            );
            if let Some(status) = &self.git_status {
                let branch = str_at(status, "branch");
                if !branch.is_empty() {
                    let dirty = get(status, "hasWorkingTreeChanges")
                        .as_bool()
                        .unwrap_or(false);
                    proj_head = proj_head.child(
                        div()
                            .text_xs()
                            .text_color(if dirty {
                                theme.warning
                            } else {
                                theme.muted_foreground
                            })
                            .child(if dirty {
                                format!("{branch} (dirty)")
                            } else {
                                branch.to_string()
                            }),
                    );
                }
            }
            sidebar = sidebar.child(proj_head);
            let pid = str_at(project, "id").to_string();
            for thread in arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "threads")
                .iter()
                .filter(|t| str_at(t, "projectId") == pid)
                .filter(|t| {
                    get(t, "deletedAt").is_null() && get(t, "archivedAt").is_null()
                })
                .filter(|t| {
                    query.is_empty()
                        || str_at(t, "title").to_lowercase().contains(&query)
                        || str_at(t, "latestReplyPreview")
                            .to_lowercase()
                            .contains(&query)
                })
                .cloned()
                .collect::<Vec<_>>()
            {
                let tid = str_at(&thread, "id").to_string();
                let title = str_at(&thread, "title");
                let preview = str_at(&thread, "latestReplyPreview");
                let selected = self.selected_thread.as_deref() == Some(tid.as_str());
                sidebar = sidebar.child(row(
                    ElementId::Name(format!("thread-{tid}").into()),
                    if title.is_empty() { "(untitled)".to_string() } else { title.to_string() },
                    preview.chars().take(80).collect(),
                    selected,
                    cx,
                    move |this, _ev, _w, cx| this.open_thread(tid.clone(), cx),
                ));
            }
        }
        sidebar = sidebar.child(
            Button::new("refresh")
                .ghost()
                .label("Refresh")
                .on_click(cx.listener(|this, _ev, _w, cx| this.refresh(cx))),
        );

        // ---- transcript ----
        let mut pane = v_flex().flex_1().h_full().p_4().gap_2();
        let thread_v = get(self.detail.as_ref().unwrap_or(&Value::Null), "thread");
        let session_status = str_at(get(thread_v, "session"), "status").to_string();
        let turn_state = str_at(get(thread_v, "latestTurn"), "state").to_string();
        let title = str_at(thread_v, "title").to_string();
        if !title.is_empty() {
            let mut header = h_flex().gap_2().child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(title),
            );
            if !session_status.is_empty() && session_status != "idle" {
                header = header.child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(session_status.clone()),
                );
            }
            if turn_state == "pending" || turn_state == "running" {
                header = header.child(
                    div()
                        .text_xs()
                        .text_color(theme.warning)
                        .child("working\u{2026}"),
                );
            } else if turn_state == "error" || turn_state == "interrupted" {
                header = header.child(
                    div().text_xs().text_color(theme.danger).child(turn_state),
                );
            }
            if get(thread_v, "hasPendingApprovals").as_bool().unwrap_or(false) {
                header = header.child(
                    div()
                        .text_xs()
                        .text_color(theme.warning)
                        .child("approval required"),
                );
            }
            pane = pane.child(header);
        }
        let messages = arr(thread_v, "messages");
        if messages.is_empty() {
            pane = pane.child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("Select a thread or type below"),
            );
        }
        for m in messages {
            let role = str_at(m, "role");
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
            if text.trim().is_empty() {
                continue;
            }
            pane = pane.child(
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.muted_foreground)
                            .child(role.to_uppercase()),
                    )
                    .child(div().text_sm().child(text.to_string())),
            );
        }

        // Activities (tool calls, errors) for the latest turn.
        for a in arr(thread_v, "activities") {
            let kind = str_at(a, "kind");
            let summary = str_at(a, "summary");
            if summary.is_empty() && kind.is_empty() {
                continue;
            }
            let color = match str_at(a, "tone") {
                "error" => theme.danger,
                _ => theme.muted_foreground,
            };
            pane = pane.child(
                div()
                    .text_xs()
                    .text_color(color)
                    .child(format!("{kind} — {summary}")),
            );
        }

        // Pending interactions (approvals awaiting a decision).
        for p in arr(thread_v, "pendingInteractions") {
            if str_at(p, "status") != "pending" {
                continue;
            }
            let kind = str_at(p, "interaction_kind").replace('_', " ");
            pane = pane.child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.warning)
                    .child(format!("approval pending: {kind}")),
            );
        }

        // ---- files browser / viewer ----
        let mut files_pane = v_flex().flex_1().h_full().p_4().gap_1();
        if self.view_files {
            if let Some((path, text)) = &self.open_file {
                let p = path.clone();
                files_pane = files_pane
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("close-file")
                                    .ghost()
                                    .label("x")
                                    .on_click(
                                        cx.listener(|this, _ev, _w, cx| this.close_file(cx)),
                                    ),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child(p),
                            ),
                    )
                    .child(
                        div()
                            .text_sm()
                            .font_family("monospace")
                            .child(text.clone()),
                    );
            } else {
                let parent = self
                    .files_dir
                    .rsplit_once('/')
                    .map(|(d, _)| d.to_string())
                    .filter(|d| !d.is_empty());
                let mut header = h_flex().gap_2().child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.muted_foreground)
                        .child(self.files_dir.clone()),
                );
                if let Some(parent) = parent {
                    header = header.child(
                        div()
                            .id("files-up")
                            .cursor_pointer()
                            .text_xs()
                            .text_color(theme.accent)
                            .child("..")
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                this.load_dir(parent.clone(), cx)
                            })),
                    );
                }
                files_pane = files_pane.child(header);
                if self.files_loading {
                    files_pane = files_pane.child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("loading\u{2026}"),
                    );
                }
                for entry in self.files.clone() {
                    let name = str_at(&entry, "name").to_string();
                    let is_dir = str_at(&entry, "kind") == "directory";
                    files_pane = files_pane.child(
                        div()
                            .id(ElementId::Name(format!("file-{name}").into()))
                            .cursor_pointer()
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .text_sm()
                            .text_color(if is_dir {
                                theme.accent
                            } else {
                                theme.foreground
                            })
                            .child(if is_dir {
                                format!("{name}/")
                            } else {
                                name
                            })
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                this.open_entry(entry.clone(), cx)
                            })),
                    );
                }
            }
        }

        // ---- composer ----
        let composer_bar = h_flex()
            .w_full()
            .p_3()
            .gap_2()
            .border_t_1()
            .border_color(theme.border)
            .child(div().flex_1().child(Input::new(&self.composer)))
            .child(
                Button::new("send")
                    .primary()
                    .label(if self.sending { "Sending..." } else { "Send" })
                    .on_click(cx.listener(|this, _ev, w, cx| this.send(w, cx))),
            );

        let mut main = v_flex().flex_1().h_full();
        if self.view_files {
            main = main.child(div().flex_1().child(files_pane.overflow_y_scrollbar()));
        } else {
            main = main
                .child(div().flex_1().child(pane.overflow_y_scrollbar()))
                .child(composer_bar);
        }

        h_flex()
            .size_full()
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(sidebar.border_r_1().border_color(theme.border))
            .child(main)
    }
}
