//! Native gpui-kit workspace: sidebar (spaces/projects/threads) + transcript +
//! composer, fed by rust/app's native WS-RPC (getShellSnapshot /
//! getThreadDetailSnapshot / dispatchCommand relayed upstream).

use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

use gpui_kit::component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Editor, EditorState, Input, InputEvent, InputState},
    menu::{DropdownMenu as _, PopupMenuItem},
    scroll::ScrollableElement as _,
    spinner::Spinner,
    text::TextView,
    theme::{Theme, ThemeMode},
    v_flex,
    Icon,
    ActiveTheme as _, IconName, Sizable as _,
};
use gpui_kit::*;
use gpui_kit::assets::IconName as AssetIcon;
use gpui::prelude::FluentBuilder as _;

use crate::ws_client::Client;

actions!(native, [ToggleFind]);

// Real upstream icon assets: Hugeicons glyphs regenerated as SVG, Central
// icons copied verbatim from apps/web/public/central-icons-reversed.
mod icons {
    pub const HOME: &[u8] = include_bytes!("../assets/icons/home-07.svg");
    pub const INBOX: &[u8] = include_bytes!("../assets/icons/inbox.svg");
    pub const SPACES: &[u8] = include_bytes!("../assets/icons/folder-library.svg");
    pub const KANBAN: &[u8] = include_bytes!("../assets/icons/columns-3-wide.svg");
    pub const TASKS: &[u8] = include_bytes!("../assets/icons/apple-reminder.svg");
    pub const PULL_REQUESTS: &[u8] =
        include_bytes!("../assets/icons/git-compare-arrows.svg");
    pub const AUTOMATIONS: &[u8] = include_bytes!("../assets/icons/clock-hour-7.svg");
    pub const STUDIO: &[u8] = include_bytes!("../assets/icons/dashboard-circle.svg");
    pub const SIDEBAR_LEFT: &[u8] =
        include_bytes!("../assets/icons/sidebar-simple-left-wide.svg");
    pub const SIDEBAR_RIGHT: &[u8] =
        include_bytes!("../assets/icons/sidebar-simple-right-wide.svg");
    pub const SEARCH: &[u8] = include_bytes!("../assets/icons/magnifying-glass.svg");
    pub const NEW_THREAD: &[u8] = include_bytes!("../assets/icons/compose-pencil.svg");
    pub const BELL: &[u8] = include_bytes!("../assets/icons/notes.svg");
    pub const USER: &[u8] = include_bytes!("../assets/icons/user.svg");
    pub const HELP: &[u8] = include_bytes!("../assets/icons/circle-questionmark.svg");
    pub const MORE: &[u8] = include_bytes!("../assets/icons/dot-grid-1x3-horizontal.svg");
    pub const PLUS: &[u8] = include_bytes!("../assets/icons/plus-sign.svg");
    pub const SEND: &[u8] = include_bytes!("../assets/icons/arrow-up.svg");
    pub const FOLDER: &[u8] = include_bytes!("../assets/icons/folder-closed.svg");
    pub const TERMINAL: &[u8] = include_bytes!("../assets/icons/square-terminal.svg");
    pub const PIN: &[u8] = include_bytes!("../assets/icons/pin.svg");
    pub const PANEL_TOP_OPEN: &[u8] =
        include_bytes!("../assets/icons/panel-top-open.svg");
    pub const LAYOUT_RIGHT: &[u8] = include_bytes!("../assets/icons/layout-right.svg");
    pub const PIN_FILL: &[u8] = include_bytes!("../assets/icons/pin-fill.svg");
    pub const STAR: &[u8] = include_bytes!("../assets/icons/star.svg");
    pub const FOLDER_OPEN: &[u8] = include_bytes!("../assets/icons/folder-open.svg");
    pub const CODEX: &[u8] = include_bytes!("../assets/icons/codex.svg");
    pub const LOGO: &[u8] = include_bytes!("../assets/icons/synara-logo.svg");
}

fn svg_icon(data: &'static [u8]) -> Icon {
    Icon::default().data(data)
}

// Small ghost icon button matching the web sidebar-icon-button affordance.
fn icon_btn(id: &'static str, data: &'static [u8], size: f32, hover: Hsla, ink: Hsla) -> Stateful<Div> {
    div()
        .id(ElementId::Name(id.into()))
        .size(px(size + 16.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.))
        .cursor_pointer()
        .hover(move |d| d.bg(hover))
        .child(svg_icon(data).size(px(size)).text_color(ink))
}

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
    open_file_meta: Option<(String, String, String)>,
    file_editor: Option<Entity<EditorState>>,
    editor_synced: Option<String>,
    editing_file: bool,
    files_loading: bool,
    git_status: Option<Value>,
    file_stream_id: Option<String>,
    search_query: String,
    search_hits: Vec<Value>,
    view_dev: bool,
    dev_servers: Vec<Value>,
    dev_cmd: Entity<InputState>,
    runtime_mode: &'static str,
    chats_collapsed: bool,
    view_settings: bool,
    view_inbox: bool,
    inbox_items: Vec<Value>,
    inbox_errors: Vec<Value>,
    inbox_loading: bool,
    tools_expanded: bool,
    pending_attachments: Vec<Value>,
    composer_thread_id: Option<String>,
    uploading: bool,
    open_threads: Vec<String>,
    pinned_threads: Vec<String>,
    renaming: bool,
    rename_input: Option<Entity<InputState>>,
    view_changes: bool,
    changes_files: Vec<Value>,
    changes_diff: Option<(String, String)>,
    changes_loading: bool,
    changes_unavailable: bool,
    view_terminal: bool,
    term_id: String,
    term_buf: String,
    term_status: Option<String>,
    term_input: Option<Entity<InputState>>,
    term_events: bool,
    find_open: bool,
    find_input: Option<Entity<InputState>>,
    find_query: String,
    sponsor_dismissed: bool,
    search_open: bool,
    sidebar_collapsed: bool,
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

fn rel_time(iso: &str) -> String {
    let Ok(t) = chrono::DateTime::parse_from_rfc3339(iso) else {
        return String::new();
    };
    let d = chrono::Utc::now().signed_duration_since(t).num_seconds().max(0);
    if d < 3600 {
        format!("{}m", d / 60)
    } else if d < 86400 {
        format!("{}h", d / 3600)
    } else {
        format!("{}d", d / 86400)
    }
}

fn uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Drop ANSI escape sequences (CSI/OSC/single-char) and CR/BS/BEL from PTY
/// output — the drawer renders plain text.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c == '\u{1b}' {
            match it.peek().copied() {
                Some('[') => {
                    it.next();
                    while let Some(n) = it.next() {
                        if n.is_ascii_alphabetic() || n == '@' || n == '`' {
                            break;
                        }
                    }
                }
                Some(']') => {
                    it.next();
                    loop {
                        match it.next() {
                            None | Some('\u{7}') => break,
                            Some('\u{1b}') => {
                                if it.peek() == Some(&'\\') {
                                    it.next();
                                }
                                break;
                            }
                            Some(_) => {}
                        }
                    }
                }
                _ => {
                    it.next();
                }
            }
        } else if c != '\r' && c != '\u{8}' && c != '\u{7}' {
            out.push(c);
        }
    }
    out
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
                |this: &mut Self, _state, event, _window, cx| {
                    if let InputEvent::Change = event {
                        this.on_search_change(cx);
                    }
                },
            )
            .detach();
            cx.bind_keys([KeyBinding::new("cmd-f", ToggleFind, Some("native"))]);
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
                open_file_meta: None,
                file_editor: None,
                editor_synced: None,
                editing_file: false,
                files_loading: false,
                git_status: None,
                file_stream_id: None,
                search_query: String::new(),
                search_hits: Vec::new(),
                view_dev: false,
                dev_servers: Vec::new(),
                dev_cmd: cx.new(|cx| {
                    InputState::new(window, cx).placeholder("Dev server command")
                }),
                runtime_mode: "full-access",
                chats_collapsed: true,
                view_settings: false,
                view_inbox: false,
                inbox_items: Vec::new(),
                inbox_errors: Vec::new(),
                inbox_loading: false,
                tools_expanded: false,
                pending_attachments: Vec::new(),
                composer_thread_id: None,
                uploading: false,
                open_threads: Vec::new(),
                pinned_threads: Vec::new(),
                renaming: false,
                rename_input: None,
                view_changes: false,
                changes_files: Vec::new(),
                changes_diff: None,
                changes_loading: false,
                changes_unavailable: false,
                view_terminal: false,
                term_id: String::new(),
                term_buf: String::new(),
                term_status: None,
                term_input: None,
                term_events: false,
                sponsor_dismissed: false,
            search_open: false,
            sidebar_collapsed: false,
            find_open: false,
                find_input: None,
                find_query: String::new(),
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
        self.renaming = false;
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
        self.renaming = false;
        cx.notify();
    }

    /// Close one open tab; if it was selected, open the last remaining tab.
    fn close_tab(&mut self, thread_id: String, cx: &mut Context<Self>) {
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

    fn toggle_pin(&mut self, thread_id: String, cx: &mut Context<Self>) {
        if self.pinned_threads.contains(&thread_id) {
            self.pinned_threads.retain(|t| t != &thread_id);
        } else {
            self.pinned_threads.push(thread_id);
        }
        cx.notify();
    }

    fn start_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let title = str_at(
            get(self.detail.as_ref().unwrap_or(&Value::Null), "thread"),
            "title",
        )
        .to_string();
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Thread title"));
        input.update(cx, |s, cx| {
            s.set_value(title, window, cx);
        });
        self.rename_input = Some(input);
        self.renaming = true;
        cx.notify();
    }

    fn commit_rename(&mut self, cx: &mut Context<Self>) {
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
        if self.view_files {
            self.view_dev = false;
            self.view_inbox = false;
            if self.files_dir.is_empty() {
                if let Some(root) = self.files_root() {
                    self.load_dir(root, cx);
                }
            }
        }
        cx.notify();
    }

    fn toggle_dev(&mut self, cx: &mut Context<Self>) {
        self.view_dev = !self.view_dev;
        if self.view_dev {
            self.view_files = false;
            self.view_inbox = false;
            self.fetch_dev_servers(cx);
        }
        cx.notify();
    }

    fn toggle_inbox(&mut self, cx: &mut Context<Self>) {
        self.view_inbox = !self.view_inbox;
        if self.view_inbox {
            self.view_files = false;
            self.view_dev = false;
            self.view_settings = false;
            self.fetch_inbox(cx);
        }
        cx.notify();
    }

    fn toggle_changes(&mut self, cx: &mut Context<Self>) {
        self.view_changes = !self.view_changes;
        if self.view_changes {
            self.fetch_changes(cx);
        }
        cx.notify();
    }

    fn fetch_changes(&mut self, cx: &mut Context<Self>) {
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
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.changes_loading = false;
                match res {
                    Some(v) => {
                        this.changes_unavailable = false;
                        this.changes_files = arr(get(&v, "workingTree"), "files").to_vec();
                    }
                    None => {
                        this.changes_unavailable = true;
                        this.changes_files = Vec::new();
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn open_diff(&mut self, path: String, cx: &mut Context<Self>) {
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
            let Some(client) = client else { return };
            let p = path.clone();
            let res = cx
                .background_executor()
                .spawn(async move {
                    client.call(
                        "git.readWorkingTreeDiff",
                        json!({"cwd": cwd, "filePath": p}),
                    )
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if let Some(v) = res {
                    this.changes_diff = Some((path.clone(), str_at(&v, "patch").to_string()));
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Cmd+F: transcript find-bar. Highlights matching messages with an accent
    /// bar and shows the match count, like the web find-in-thread bar.
    fn toggle_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.find_open = !self.find_open;
        if self.find_open {
            let input = cx.new(|cx| {
                InputState::new(window, cx).placeholder("Find in thread")
            });
            cx.subscribe_in(&input, window, |this: &mut Self, state, ev, _w, cx| {
                if let InputEvent::Change = ev {
                    this.find_query = state.read(cx).value().to_string();
                    cx.notify();
                }
            })
            .detach();
            input.update(cx, |s, cx| s.focus_handle(cx).focus(window, cx));
            self.find_input = Some(input);
        } else {
            self.find_input = None;
            self.find_query.clear();
        }
        cx.notify();
    }

    /// Rail button: toggle the bottom terminal drawer. Opening spawns a PTY
    /// session on the workspace root via terminal.open and subscribes to its
    /// event stream; closing kills it.
    fn toggle_terminal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.view_terminal = !self.view_terminal;
        if self.view_terminal {
            self.term_id = uuid();
            self.term_buf.clear();
            self.term_status = None;
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
            self.ensure_terminal_events(cx);
            self.open_terminal(cx);
        } else {
            self.close_terminal(cx);
        }
        cx.notify();
    }

    fn open_terminal(&mut self, cx: &mut Context<Self>) {
        let client = self.client.clone();
        let terminal_id = self.term_id.clone();
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
                        "terminal.open",
                        json!({
                            "threadId": "shell",
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

    fn close_terminal(&mut self, cx: &mut Context<Self>) {
        self.term_input = None;
        self.term_status = None;
        self.term_buf.clear();
        let client = self.client.clone();
        let terminal_id = self.term_id.clone();
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
                            "threadId": "shell",
                            "terminalId": terminal_id,
                            "deleteHistory": true,
                        }),
                    )
                })
                .await;
        })
        .detach();
    }

    fn term_restart(&mut self, cx: &mut Context<Self>) {
        let client = self.client.clone();
        let terminal_id = self.term_id.clone();
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
                            "threadId": "shell",
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

    fn term_send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(input) = self.term_input.clone() else {
            return;
        };
        let text = input.read(cx).value().to_string();
        if text.is_empty() {
            return;
        }
        input.update(cx, |s, cx| s.clean(window, cx));
        let client = self.client.clone();
        let terminal_id = self.term_id.clone();
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
                            "threadId": "shell",
                            "terminalId": terminal_id,
                            "data": format!("{text}\r"),
                        }),
                    )
                })
                .await;
        })
        .detach();
    }

    /// Long-lived subscribeEvents stream feeding the terminal drawer; it
    /// reconnects itself while the drawer is open.
    fn ensure_terminal_events(&mut self, cx: &mut Context<Self>) {
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

    fn on_terminal_event(&mut self, ev: Value) {
        if str_at(&ev, "terminalId") != self.term_id || str_at(&ev, "threadId") != "shell" {
            return;
        }
        match str_at(&ev, "type") {
            "started" | "restarted" => {
                let snap = get(&ev, "snapshot");
                self.term_buf = strip_ansi(str_at(snap, "history"));
                self.term_status = Some(str_at(snap, "status").to_string());
            }
            "output" => {
                self.term_buf.push_str(&strip_ansi(str_at(&ev, "data")));
                if self.term_buf.len() > 200_000 {
                    let mut cut = self.term_buf.len() - 100_000;
                    while !self.term_buf.is_char_boundary(cut) {
                        cut += 1;
                    }
                    self.term_buf.drain(..cut);
                }
            }
            "cleared" => self.term_buf.clear(),
            "exited" => {
                self.term_status = Some("exited".to_string());
                let suffix = get(&ev, "exitCode")
                    .as_i64()
                    .map(|c| format!(" code {c}"))
                    .unwrap_or_default();
                self.term_buf.push_str(&format!("\n[process exited{suffix}]\n"));
            }
            _ => {}
        }
    }

    /// "+" button: native file picker -> upload each file -> chip in composer.
    fn attach(&mut self, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some("Attach files".into()),
        });
        let client = self.client.clone();
        if self.composer_thread_id.is_none() && self.selected_thread.is_none() {
            self.composer_thread_id = Some(uuid());
        }
        let thread_id = self
            .selected_thread
            .clone()
            .or_else(|| self.composer_thread_id.clone());
        self.uploading = true;
        cx.notify();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let paths = rx.await.ok().and_then(|r| r.ok()).flatten().unwrap_or_default();
            let cell = client.clone();
            let thread_id = thread_id.unwrap_or_default();
            let uploaded: Vec<Value> = cx
                .background_executor()
                .spawn(async move {
                    let Some(c) = ensure_client(&cell) else {
                        return Vec::new();
                    };
                    let mut out = Vec::new();
                    for p in paths {
                        let Ok(bytes) = std::fs::read(&p) else {
                            continue;
                        };
                        let name = p
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("file")
                            .to_string();
                        let ext = p
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("")
                            .to_lowercase();
                        let (kind, mime) = match ext.as_str() {
                            "png" => ("image", "image/png"),
                            "jpg" | "jpeg" => ("image", "image/jpeg"),
                            "gif" => ("image", "image/gif"),
                            "webp" => ("image", "image/webp"),
                            "pdf" => ("file", "application/pdf"),
                            "txt" | "md" => ("file", "text/plain"),
                            "json" => ("file", "application/json"),
                            "zip" => ("file", "application/zip"),
                            _ => ("file", "application/octet-stream"),
                        };
                        if let Some(att) =
                            c.upload_attachment(&thread_id, kind, &name, mime, &bytes)
                        {
                            out.push(att);
                        }
                    }
                    out
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.uploading = false;
                this.pending_attachments.extend(uploaded);
                cx.notify();
            });
        })
        .detach();
    }

    /// Blocking RPC: GitHub inbox (PRs + issues of the project's repos).
    fn fetch_inbox(&self, cx: &mut Context<Self>) {
        let cell = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.inbox_loading = true;
                cx.notify();
            });
            let v = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell)
                        .and_then(|c| {
                            c.call("githubInbox.list", json!({"state": "open", "sort": "updated"}))
                        })
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.inbox_loading = false;
                this.inbox_items = v.as_ref().map(|v| arr(v, "items").to_vec()).unwrap_or_default();
                this.inbox_errors = v.map(|v| arr(&v, "errors").to_vec()).unwrap_or_default();
                cx.notify();
            });
        })
        .detach();
    }

    /// Blocking RPC: list running dev servers.
    fn fetch_dev_servers(&self, cx: &mut Context<Self>) {
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

    /// Start a dev server for the first project with the typed command.
    fn dev_run(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let command = self.dev_cmd.read(cx).value().to_string();
        let command = command.trim().to_string();
        if command.is_empty() {
            return;
        }
        self.dev_cmd.update(cx, |s, cx| s.clean(window, cx));
        let project_id = arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects")
            .first()
            .and_then(|p| p.get("id"))
            .and_then(Value::as_str)
            .unwrap_or("default")
            .to_string();
        let Some(cwd) = self.files_root() else {
            return;
        };
        let cell = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let v = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell).and_then(|c| {
                        c.call(
                            "projects.runDevServer",
                            json!({"projectId": project_id, "command": command, "cwd": cwd}),
                        )
                    })
                })
                .await;
            if v.is_none() {
                eprintln!("native: runDevServer failed");
            }
            let _ = this.update(cx, |this: &mut Self, cx| this.fetch_dev_servers(cx));
        })
        .detach();
    }

    /// Stop a dev server by project id.
    fn dev_stop(&mut self, project_id: String, cx: &mut Context<Self>) {
        let cell = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let v = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell).and_then(|c| {
                        c.call("projects.stopDevServer", json!({"projectId": project_id}))
                    })
                })
                .await;
            if v.is_none() {
                eprintln!("native: stopDevServer failed");
            }
            let _ = this.update(cx, |this: &mut Self, cx| this.fetch_dev_servers(cx));
        })
        .detach();
    }

    /// Debounced content search via orchestration.searchThreads.
    fn on_search_change(&mut self, cx: &mut Context<Self>) {
        let query = self.search.read(cx).value().to_string();
        self.search_query = query.clone();
        self.search_hits = Vec::new();
        cx.notify();
        let trimmed = query.trim().to_string();
        if trimmed.chars().count() < 2 {
            return;
        }
        let cell = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(300))
                .await;
            let still = this
                .update(cx, |this: &mut Self, _| this.search_query.trim() == trimmed)
                .unwrap_or(false);
            if !still {
                return;
            }
            let v = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell).and_then(|c| {
                        c.call(
                            "orchestration.searchThreads",
                            json!({"query": trimmed, "limit": 20}),
                        )
                    })
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.search_hits = v.map(|v| arr(&v, "matches").to_vec()).unwrap_or_default();
                cx.notify();
            });
        })
        .detach();
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
                    this.open_file_meta = Some((
                        str_at(&v, "version").to_string(),
                        str_at(&v, "encoding").to_string(),
                        str_at(&v, "lineEnding").to_string(),
                    ));
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
                                this.open_file_meta = Some((
                                    str_at(&v, "version").to_string(),
                                    str_at(&v, "encoding").to_string(),
                                    str_at(&v, "lineEnding").to_string(),
                                ));
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
        self.open_file_meta = None;
        self.file_editor = None;
        self.editor_synced = None;
        self.editing_file = false;
        cx.notify();
    }

    /// Map a file path to an editor language id for highlighting.
    fn editor_language(path: &str) -> &'static str {
        let ext = path.rsplit('.').next().unwrap_or("");
        match ext {
            "rs" => "rust",
            "ts" | "tsx" => "typescript",
            "js" | "jsx" | "mjs" | "cjs" => "javascript",
            "json" => "json",
            "md" => "markdown",
            "py" => "python",
            "toml" => "toml",
            "yaml" | "yml" => "yaml",
            "html" => "html",
            "css" => "css",
            "sh" | "bash" | "zsh" => "shell",
            _ => "",
        }
    }

    /// Save the edited file back via projects.writeFile, guarded by the
    /// version reported by the last read.
    fn save_file(&mut self, cx: &mut Context<Self>) {
        let Some((path, _)) = self.open_file.clone() else {
            return;
        };
        let Some(editor) = self.file_editor.clone() else {
            return;
        };
        let contents = editor.read(cx).value().to_string();
        let (version, encoding, line_ending) =
            self.open_file_meta.clone().unwrap_or_default();
        let dir = path
            .rsplit_once('/')
            .map(|(d, _)| d.to_string())
            .unwrap_or_else(|| path.clone());
        let name = path.rsplit('/').next().unwrap_or(&path).to_string();
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let p = path.clone();
            let v = cx
                .background_executor()
                .spawn(async move {
                    let mut payload = json!({
                        "cwd": dir,
                        "relativePath": name,
                        "contents": contents,
                    });
                    if !version.is_empty() {
                        payload["expectedVersion"] = json!(version);
                        if !encoding.is_empty() {
                            payload["encoding"] = json!(encoding);
                        }
                        if !line_ending.is_empty() {
                            payload["lineEnding"] = json!(line_ending);
                        }
                    }
                    ensure_client(&cell)
                        .and_then(|c| c.call("projects.writeFile", payload))
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if let Some(v) = v {
                    if let Some(ed) = &this.file_editor {
                        let text = ed.read(cx).value().to_string();
                        this.open_file = Some((p, text));
                    }
                    let meta = this.open_file_meta.get_or_insert_default();
                    if let Some(ver) = v.get("version").and_then(|x| x.as_str()) {
                        meta.0 = ver.to_string();
                        this.editor_synced = Some(ver.to_string());
                    }
                    this.editing_file = false;
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Dispatch a simple thread command (interrupt, archive, delete, ...).
    fn dispatch_thread_cmd(&mut self, cmd_type: &'static str, cx: &mut Context<Self>) {
        let Some(tid) = self.selected_thread.clone() else {
            return;
        };
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let cmd = json!({
                "type": cmd_type,
                "commandId": uuid(),
                "threadId": tid,
                "createdAt": now_iso(),
            });
            let r = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell)
                        .and_then(|c| c.call("orchestration.dispatchCommand", cmd))
                })
                .await;
            if r.is_none() {
                eprintln!("native: {cmd_type} dispatch failed");
            }
            let cell = client.clone();
            let s = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell)
                        .and_then(|c| c.call("orchestration.getShellSnapshot", json!({})))
                })
                .await;
            let cell = client.clone();
            let p = json!({"threadId": tid});
            let d = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell)
                        .and_then(|c| c.call("orchestration.getThreadDetailSnapshot", p))
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if let Some(s) = s {
                    this.snapshot = Some(s);
                }
                if let Some(d) = d {
                    this.detail = Some(d);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Dispatch thread.approval.respond for a pending interaction.
    fn respond_approval(&mut self, request_id: String, decision: &str, cx: &mut Context<Self>) {
        let Some(tid) = self.selected_thread.clone() else {
            return;
        };
        let client = self.client.clone();
        let decision = decision.to_string();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let cmd = json!({
                "type": "thread.approval.respond",
                "commandId": uuid(),
                "threadId": tid,
                "requestId": request_id,
                "decision": decision,
                "createdAt": now_iso(),
            });
            let r = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell)
                        .and_then(|c| c.call("orchestration.dispatchCommand", cmd))
                })
                .await;
            if r.is_none() {
                eprintln!("native: approval.respond dispatch failed");
            }
            let cell = client.clone();
            let p = json!({"threadId": tid});
            let d = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell)
                        .and_then(|c| c.call("orchestration.getThreadDetailSnapshot", p))
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if let Some(d) = d {
                    this.detail = Some(d);
                }
                cx.notify();
            });
        })
        .detach();
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
        let mode = self.runtime_mode;
        // Attachments uploaded earlier keep the thread id they were staged
        // under (a fresh thread adopts it as its id).
        if self.composer_thread_id.is_none() && self.selected_thread.is_none() {
            self.composer_thread_id = Some(uuid());
        }
        let attachments = std::mem::take(&mut self.pending_attachments);
        let att_thread = if self.selected_thread.is_some() {
            self.selected_thread.clone()
        } else {
            self.composer_thread_id.clone()
        };
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
            let tid = selected.clone().unwrap_or_else(|| {
                att_thread.clone().unwrap_or_else(uuid)
            });
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
                        "runtimeMode": mode,
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
                    "attachments": attachments,
                },
                "runtimeMode": mode,
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        // ---- sidebar ----
        let mut sidebar = v_flex()
            .w_full()
            .h_full()
            .gap_0p5()
            .px_2()
            .pb_2()
            // Card interior below the top strip — the first row lands ~4px
            // under the card's top edge like the upstream panel.
            .pt(px(4.))
            .overflow_y_scrollbar();
        let query = self.search.read(cx).value().to_string().to_lowercase();
        // Header: bold wordmark + chevron (workspace switcher), search +
        // new-thread icon buttons on the right — mirrors the web sidebar.
        sidebar = sidebar.child(
            h_flex()
                .items_center()
                .gap_1()
                .mb_1()
                .child(
                    div()
                        .px_1()
                        .text_size(px(15.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Synara"),
                )
                .child(
                    Button::new("app-menu")
                        .ghost()
                        .compact()
                        .icon(AssetIcon::ChevronDown)
                        .dropdown_menu(|menu, _, _| {
                            menu.item(
                                PopupMenuItem::new("Synara on GitHub").on_click(|_, _, cx| {
                                    cx.open_url("https://github.com/Emanuele-web04/synara");
                                }),
                            )
                            .item(
                                PopupMenuItem::new("Synara on GitHub (Rust port)").on_click(
                                    |_, _, cx| {
                                        cx.open_url("https://github.com/cmdr-chara/synara/tree/feat/rust-port");
                                    },
                                ),
                            )
                        }),
                )
                .child(div().flex_1())
                .child(
                    icon_btn("sidebar-search", icons::SEARCH, 18., theme.secondary, theme.muted_foreground)
                        .on_click(cx.listener(|this, _ev, _w, cx| {
                            this.search_open = !this.search_open;
                            cx.notify();
                        })),
                )
                .child(
                    icon_btn("sidebar-new", icons::NEW_THREAD, 18., theme.secondary, theme.muted_foreground)
                        .on_click(cx.listener(|this, _ev, _w, cx| this.new_thread(cx))),
                ),
        );
        if self.search_open {
            sidebar = sidebar.child(
                h_flex()
                    .gap_2()
                    .child(div().flex_1().child(Input::new(&self.search))),
            );
        }
        sidebar = sidebar.child(
            h_flex()
                .id("new-thread")
                .w_full()
                .h(px(32.))
                .px_2()
                .gap_2()
                .items_center()
                .rounded(px(8.))
                .cursor_pointer()
                .hover(|d| d.bg(theme.secondary))
                .child(
                    svg_icon(icons::NEW_THREAD)
                        .size(px(16.))
                        .text_color(theme.foreground),
                )
                .child(div().text_size(px(13.)).child("New thread"))
                .on_click(cx.listener(|this, _ev, _w, cx| this.new_thread(cx))),
        );
        sidebar = sidebar.child(
            div()
                .mt(px(15.))
                .text_size(px(13.))
                .text_color(theme.muted_foreground)
                .child("Projects"),
        );
        if arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects").is_empty() {
            sidebar = sidebar.child(
                div()
                    .w_full()
                    .py_2()
                    .text_size(px(13.))
                    .text_color(theme.muted_foreground)
                    .child(div().text_center().child("No projects yet")),
            );
        }
        let all_threads: Vec<Value> =
            arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "threads").to_vec();
        let live_thread = |t: &Value| {
            get(t, "deletedAt").is_null() && get(t, "archivedAt").is_null()
        };
        let matches_query = |t: &Value| {
            query.is_empty()
                || str_at(t, "title").to_lowercase().contains(&query)
                || str_at(t, "latestReplyPreview")
                    .to_lowercase()
                    .contains(&query)
        };
        let pinned_ids = self.pinned_threads.clone();
        let selected_tid = self.selected_thread.clone();
        let thread_row = |thread: &Value,
                          theme: &Theme,
                          pinned: &[String],
                          selected: Option<&str>,
                          indent: f32,
                          cx: &mut Context<Self>|
         -> Stateful<Div> {
            let tid = str_at(thread, "id").to_string();
            let title = str_at(thread, "title");
            let is_pinned = pinned.contains(&tid);
            let is_sel = selected == Some(tid.as_str());
            let mut el = h_flex()
                .id(ElementId::Name(format!("thread-{tid}").into()))
                .w_full()
                .h(px(28.))
                .pl(px(indent))
                .pr_2()
                .gap(px(6.))
                .items_center()
                .cursor_pointer()
                .rounded_md()
                .child(
                    svg_icon(icons::CODEX)
                        .size(px(12.))
                        .text_color(theme.muted_foreground),
                )
                .child(
                    div()
                        .flex_1()
                        .overflow_hidden()
                        .text_size(px(13.))
                        .whitespace_nowrap()
                        .text_color(if is_sel {
                            theme.foreground
                        } else {
                            theme.muted_foreground
                        })
                        .child(if title.is_empty() {
                            "(untitled)".to_string()
                        } else {
                            title.to_string()
                        }),
                );
            if is_pinned {
                el = el.child(
                    svg_icon(icons::PIN_FILL)
                        .size(px(11.))
                        .text_color(theme.muted_foreground),
                );
            }
            if is_sel {
                el = el.bg(theme.list_active);
            } else {
                el = el.hover(|d| d.bg(theme.secondary));
            }
            el.on_click(cx.listener(move |this, _ev, _w, cx| {
                this.open_thread(tid.clone(), cx)
            }))
        };
        // Projects: only real (non-chat) projects appear here — chat-kind
        // projects (Home) surface only via the "Chats" section. Pinned first,
        // then creation order, matching the web sidebar.
        let mut projects_list: Vec<Value> = arr(
            self.snapshot.as_ref().unwrap_or(&Value::Null),
            "projects",
        )
        .iter()
        .filter(|p| str_at(p, "kind") != "chat")
        .cloned()
        .collect();
        projects_list.sort_by_key(|p| {
            (
                if get(p, "isPinned").as_bool().unwrap_or(false) {
                    0
                } else {
                    1
                },
                str_at(p, "createdAt").to_string(),
            )
        });
        for (project_index, project) in projects_list.iter().enumerate() {
            let pid = str_at(project, "id").to_string();
            // Expanded when it has visible threads nested below.
            let mut kids: Vec<Value> = all_threads
                .iter()
                .filter(|t| str_at(t, "projectId") == pid && live_thread(t) && matches_query(t))
                .cloned()
                .collect();
            // Newest threads first — the web sidebar lists latest activity on top.
            kids.sort_by_key(|t| {
                (
                    if pinned_ids.contains(&str_at(t, "id").to_string()) {
                        0
                    } else {
                        1
                    },
                    std::cmp::Reverse(str_at(t, "createdAt").to_string()),
                )
            });
            let expanded = !kids.is_empty();
            let pinned_proj = get(project, "isPinned").as_bool().unwrap_or(false);
            // A project whose workspace carries a favicon (here: the synara
            // checkout) shows its logo in a hairline frame; the pin glyph wins
            // for pinned projects; otherwise the folder opens/closes.
            let has_logo = str_at(project, "workspaceRoot").ends_with("synara");
            let glyph = if pinned_proj {
                icons::PIN_FILL
            } else if has_logo {
                icons::LOGO
            } else if expanded {
                icons::FOLDER_OPEN
            } else {
                icons::FOLDER
            };
            let glyph_el = if has_logo && !pinned_proj {
                div()
                    .size(px(19.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(5.))
                    .border_1()
                    .border_color(theme.border)
                    .child(svg_icon(glyph).size(px(12.)).text_color(theme.foreground))
                    .into_any_element()
            } else {
                svg_icon(glyph)
                    .size(px(16.))
                    .text_color(theme.muted_foreground)
                    .into_any_element()
            };
            // The disclosure chevron only appears beside the title when the
            // leading glyph cannot convey open/closed (pin or logo).
            let needs_chevron = pinned_proj || has_logo;
            let is_active_project = has_logo;
            let mut row_head = h_flex()
                .gap_1p5()
                .items_center()
                .h(px(26.))
                .child(glyph_el)
                .child(
                    div()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_size(px(13.))
                        .child(str_at(project, "title").to_string()),
                );
            if needs_chevron {
                row_head = row_head.child(
                    div()
                        .text_size(px(10.))
                        .text_color(theme.muted_foreground)
                        .child(if expanded { "\u{2228}" } else { "\u{2228}" }),
                );
            }
            row_head = row_head.child(div().flex_1());
            if is_active_project {
                row_head = row_head.child(
                    div()
                        .size(px(6.))
                        .rounded_full()
                        .bg(gpui_kit::rgb(0x22c55e)),
                );
            }
            let mut proj_head = v_flex()
                .gap_0()
                .mt(if project_index == 0 { px(8.) } else { px(14.) })
                .id(ElementId::Name(
                    format!("project-{}", str_at(project, "id")).into(),
                ))
                .cursor_pointer()
                .rounded_md()
                .px_2()
                .child(row_head)
                .on_click(cx.listener(|this, _ev, _w, cx| {
                    if !this.view_files {
                        this.toggle_files(cx);
                    } else {
                        cx.notify();
                    }
                }));
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
            for thread in &kids {
                sidebar = sidebar.child(thread_row(
                    thread,
                    &theme,
                    &pinned_ids,
                    selected_tid.as_deref(),
                    34.,
                    cx,
                ));
            }
        }
        sidebar = sidebar.child(
            div()
                .id("chats-toggle")
                .mt_2()
                .cursor_pointer()
                .text_size(px(13.))
                .text_color(theme.muted_foreground)
                .child(if self.chats_collapsed {
                    "Chats \u{203a}"
                } else {
                    "Chats \u{2228}"
                })
                .on_click(cx.listener(|this, _ev, _w, cx| {
                    this.chats_collapsed = !this.chats_collapsed;
                    cx.notify();
                })),
        );
        // "Chats" = threads filed under chat-kind projects (Home), matching
        // the web sidebar's home chats section.
        let chat_project_ids: Vec<String> =
            arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects")
                .iter()
                .filter(|p| str_at(p, "kind") == "chat")
                .map(|p| str_at(p, "id").to_string())
                .collect();
        let mut chat_threads: Vec<Value> = if self.chats_collapsed {
            Vec::new()
        } else {
            all_threads
                .iter()
                .filter(|t| {
                    chat_project_ids.iter().any(|id| id == str_at(t, "projectId"))
                        && live_thread(t)
                        && matches_query(t)
                })
                .cloned()
                .collect()
        };
        chat_threads.sort_by_key(|t| {
            if pinned_ids.contains(&str_at(t, "id").to_string()) { 0 } else { 1 }
        });
        for thread in &chat_threads {
            sidebar = sidebar.child(thread_row(
                thread,
                &theme,
                &pinned_ids,
                selected_tid.as_deref(),
                8.,
                cx,
            ));
        }
        // Server-side content matches (searchThreads).
        if !self.search_hits.is_empty() {
            sidebar = sidebar.child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.muted_foreground)
                    .child("CONTENT MATCHES"),
            );
            for hit in self.search_hits.clone() {
                let tid = str_at(&hit, "threadId").to_string();
                let count = get(&hit, "matchCount").as_i64().unwrap_or(0);
                let excerpt = str_at(&hit, "excerpt");
                // Show the thread's title when we know it.
                let known = arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "threads")
                    .iter()
                    .find(|t| str_at(t, "id") == tid)
                    .map(|t| str_at(t, "title").to_string())
                    .filter(|t| !t.is_empty())
                    .unwrap_or_else(|| tid.chars().take(8).collect());
                sidebar = sidebar.child(row(
                    ElementId::Name(format!("hit-{tid}-{count}").into()),
                    format!("{known} (x{count})"),
                    excerpt.chars().take(80).collect(),
                    false,
                    cx,
                    move |this, _ev, _w, cx| this.open_thread(tid.clone(), cx),
                ));
            }
        }
        // ---- transcript ----
        let mut pane = v_flex().flex_1().h_full().gap_2();
        if self.selected_thread.is_some() {
            pane = pane.p_4();
        }
        if self.selected_thread.is_none() {
            const WM_CHARS: &[u8] =
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
            // AsciiWaveBackground parity: 10px glyphs on a 13px grid over the
            // whole pane, inked only inside drifting noise blotches
            // (PATCH_THRESHOLD 0.4..0.78); a static deterministic field.
            let mut field = v_flex().gap_0();
            for r in 0..72usize {
                let mut line = String::with_capacity(160);
                for c in 0..160usize {
                    let nx = c as f32;
                    let ny = r as f32;
                    let noise = ((nx * 0.045).sin() * (ny * 0.06).cos()
                        + (nx * 0.012 + ny * 0.017).sin() * 0.5
                        + (nx * 0.003 * ny).cos() * 0.3)
                        / 1.8
                        + 0.5;
                    if noise > 0.55 && noise < 0.85 && (r * 7 + c * 13) % 4 < 1 {
                        let g = (r * 131 + c * 17 + r * c * 3 + c) % WM_CHARS.len();
                        line.push(WM_CHARS[g] as char);
                    } else {
                        line.push(' ');
                    }
                }
                field = field.child(
                    div()
                        .text_size(px(10.))
                        .line_height(px(13.))
                        .whitespace_nowrap()
                        .text_color(rgba(0x00000014))
                        .child(line),
                );
            }
            let wm = div()
                .size_full()
                .overflow_hidden()
                .child(field);
            let mut home = v_flex()
                .relative()
                .flex_1()
                .h_full()
                .items_center()
                .justify_center()
                .gap_4()
                .child(div().absolute().inset_0().overflow_hidden().child(wm));
            if !self.sponsor_dismissed {
                // Upstream SponsorLandingBanner: centered, max-w-520, borderless
                // row — star + title + subtitle; dismiss X appears only on hover.
                home = home.child(
                    div().absolute().top(px(22.)).left_0().w_full().child(
                        h_flex().w_full().justify_center().px_6().pl(px(66.)).child(
                            h_flex()
                                .relative()
                                .id("sponsor-banner")
                                .group("sponsor-banner")
                                .cursor_pointer()
                                .w(px(588.))
                                .rounded(px(16.))
                                .px_4()
                                .py(px(9.))
                                .gap_4()
                                .items_center()
                                .bg(rgb(0xf6f6f6))
                                .hover(|d| d.bg(theme.secondary))
                                .on_click(cx.listener(|_this, _ev, _w, cx| {
                                    cx.open_url("https://www.trysynara.com/sponsor");
                                }))
                                .child(
                                    div()
                                        .size(px(40.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(
                                            svg_icon(icons::STAR)
                                                .size(px(20.))
                                                .text_color(theme.primary),
                                        ),
                                )
                                .child(
                                    v_flex()
                                        .flex_1()
                                        .min_w_0()
                                        .gap_0p5()
                                        .child(
                                            div()
                                                .text_size(px(14.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .whitespace_nowrap()
                                                .child("Support Synara"),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(14.))
                                                .text_color(theme.muted_foreground)
                                                .whitespace_nowrap()
                                                .child("Explore sponsorships and help fund development"),
                                        ),
                                )
                                .child(
                                    div()
                                        .id("sponsor-dismiss")
                                        .absolute()
                                        .top(px(-6.))
                                        .right(px(-6.))
                                        .size(px(22.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded_full()
                                        .border_1()
                                        .border_color(theme.border)
                                        .bg(theme.background)
                                        .cursor_pointer()
                                        .invisible()
                                        .group_hover("sponsor-banner", |d| d.visible())
                                        .child(
                                            Icon::new(AssetIcon::X)
                                                .size(px(12.))
                                                .text_color(theme.muted_foreground),
                                        )
                                        .on_click(cx.listener(|this, _ev, _w, cx| {
                                            this.sponsor_dismissed = true;
                                            cx.notify();
                                        })),
                                ),
                        ),
                    ),
                );
            }
            pane = pane.child(
                home.child(svg_icon(icons::LOGO).size(px(40.)))
                    .child(
                        div()
                            .mt(px(-17.))
                            .text_size(px(30.))
                            .font_weight(FontWeight::NORMAL)
                            .text_color({
                                let mut c = theme.foreground;
                                c.a *= 0.95;
                                c
                            })
                            .child("What should we do in this folder?"),
                    ),
            );
        }
        // ---- open tabs strip ----
        if !self.open_threads.is_empty() {
            let all_threads = arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "threads");
            let mut strip = h_flex().gap_1().items_center().flex_wrap();
            for ot in self.open_threads.clone() {
                let active = self.selected_thread.as_deref() == Some(ot.as_str());
                let ot_title = all_threads
                    .iter()
                    .find(|t| str_at(t, "id") == ot)
                    .map(|t| str_at(t, "title").to_string())
                    .filter(|t| !t.is_empty())
                    .unwrap_or_else(|| ot.chars().take(8).collect());
                let sel = ot.clone();
                let close = ot.clone();
                let mut tab = Button::new(ElementId::Name(format!("tab-{ot}").into()))
                    .ghost()
                    .compact()
                    .icon(IconName::Bot)
                    .label(ot_title);
                if active {
                    tab = tab.outline();
                }
                strip = strip
                    .child(tab.on_click(cx.listener(move |this, _ev, _w, cx| {
                        if this.selected_thread.as_deref() != Some(sel.as_str()) {
                            this.open_thread(sel.clone(), cx);
                        }
                    })))
                    .child(
                        Button::new(ElementId::Name(format!("tab-close-{close}").into()))
                            .ghost()
                            .compact()
                            .label("\u{00d7}")
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                this.close_tab(close.clone(), cx);
                            })),
                    );
            }
            pane = pane.child(strip);
        }
        let thread_v = get(self.detail.as_ref().unwrap_or(&Value::Null), "thread");
        let session_status = str_at(get(thread_v, "session"), "status").to_string();
        let turn_state = str_at(get(thread_v, "latestTurn"), "state").to_string();
        let title = str_at(thread_v, "title").to_string();
        if !title.is_empty() {
            let tid_now = self.selected_thread.clone().unwrap_or_default();
            let pinned = self.pinned_threads.contains(&tid_now);
            let mut header = h_flex().gap_2().items_center();
            if self.renaming {
                if let Some(input) = &self.rename_input {
                    header = header
                        .child(div().w(px(240.)).child(Input::new(input)))
                        .child(
                            Button::new("rename-ok")
                                .ghost()
                                .compact()
                                .label("\u{2713}")
                                .on_click(cx.listener(|this, _ev, _w, cx| {
                                    this.commit_rename(cx);
                                })),
                        )
                        .child(
                            Button::new("rename-cancel")
                                .ghost()
                                .compact()
                                .label("\u{00d7}")
                                .on_click(cx.listener(|this, _ev, _w, cx| {
                                    this.renaming = false;
                                    this.rename_input = None;
                                    cx.notify();
                                })),
                        );
                }
            } else {
                header = header
                    .child(
                        Button::new("thread-tab")
                            .ghost()
                            .icon(IconName::Bot)
                            .label(title.clone()),
                    )
                    .child(
                        Button::new("thread-menu")
                            .ghost()
                            .compact()
                            .label("\u{22ef}")
                            .dropdown_menu({
                                let this = cx.entity();
                                let tid = tid_now.clone();
                                let pin_label = if pinned { "Unpin" } else { "Pin" };
                                move |mut menu, _, _| {
                                    let t = this.clone();
                                    let t_pin = this.clone();
                                    let t2 = tid.clone();
                                    menu = menu
                                        .item(PopupMenuItem::new("Rename").on_click(
                                            move |_, window, cx| {
                                                t.update(cx, |this, cx| {
                                                    this.start_rename(window, cx);
                                                });
                                            },
                                        ))
                                        .item(PopupMenuItem::new(pin_label).on_click(
                                            move |_, _, cx| {
                                                t_pin.update(cx, |this, cx| {
                                                    this.toggle_pin(t2.clone(), cx);
                                                });
                                            },
                                        ));
                                    menu
                                }
                            }),
                    );
            }
            if !session_status.is_empty() && session_status != "idle" {
                header = header.child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(session_status.clone()),
                );
            }
            if turn_state == "pending" || turn_state == "running" {
                header = header
                    .child(Spinner::new().small())
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.warning)
                            .child("working\u{2026}"),
                    );
            } else if turn_state == "error" || turn_state == "interrupted" {
                header = header.child(
                    div()
                        .text_xs()
                        .text_color(theme.danger)
                        .child(turn_state.clone()),
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
            if turn_state == "pending" || turn_state == "running" {
                header = header.child(
                    Button::new("stop-turn")
                        .ghost()
                        .label("Stop")
                        .on_click(cx.listener(|this, _ev, _w, cx| {
                            this.dispatch_thread_cmd("thread.turn.interrupt", cx)
                        })),
                );
            }
            header = header.child(
                Button::new("archive-thread")
                    .ghost()
                    .label("Archive")
                    .on_click(cx.listener(|this, _ev, _w, cx| {
                        this.dispatch_thread_cmd("thread.archive", cx)
                    })),
            );
            pane = pane.child(header);
        }
        let messages = arr(thread_v, "messages");
        let find_q = self.find_query.to_lowercase();
        if messages.is_empty() && self.selected_thread.is_some() {
            pane = pane.child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("Select a thread or type below"),
            );
        }
        for (i, m) in messages.iter().enumerate() {
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
            let is_match = !find_q.is_empty() && text.to_lowercase().contains(&find_q);
            if role == "user" {
                let mut pill = div()
                    .bg(theme.list_active)
                    .rounded_lg()
                    .px_3()
                    .py_2();
                if is_match {
                    pill = pill.border_1().border_color(theme.warning);
                }
                pane = pane.child(
                    h_flex()
                        .w_full()
                        .justify_end()
                        .child(pill.child(div().text_sm().child(text))),
                );
            } else {
                let mut wrap = div();
                if is_match {
                    wrap = wrap.border_l_2().border_color(theme.warning).pl_2();
                }
                pane = pane.child(wrap.child(
                    TextView::markdown(
                        ElementId::Name(format!("md-{i}-{}", str_at(m, "id")).into()),
                        text,
                    )
                    .selectable(true),
                ));
            }
        }

        // Activities grouped in a collapsible tool-call card (web parity):
        // a single activity stays a plain row; several collapse into a card.
        let activities: Vec<Value> = arr(thread_v, "activities")
            .iter()
            .filter(|a| !str_at(a, "kind").is_empty() || !str_at(a, "summary").is_empty())
            .cloned()
            .collect();
        let activity_row = |a: &Value| {
            let kind = str_at(a, "kind");
            let summary = str_at(a, "summary");
            let color = match str_at(a, "tone") {
                "error" => theme.danger,
                _ => theme.muted_foreground,
            };
            let label = if summary.is_empty() {
                kind.to_string()
            } else if kind.is_empty() {
                summary.to_string()
            } else {
                format!("{kind} — {summary}")
            };
            h_flex()
                .gap_2()
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child("\u{25e6}"),
                )
                .child(div().text_xs().text_color(color).child(label))
        };
        if activities.len() == 1 {
            pane = pane.child(activity_row(&activities[0]));
        } else if activities.len() > 1 {
            let mut card = v_flex()
                .border_1()
                .border_color(theme.border)
                .rounded_lg()
                .p_2()
                .gap_1()
                .child(
                    h_flex()
                        .id("tools-card")
                        .cursor_pointer()
                        .gap_2()
                        .items_center()
                        .on_click(cx.listener(|this, _ev, _w, cx| {
                            this.tools_expanded = !this.tools_expanded;
                            cx.notify();
                        }))
                        .child(
                            Icon::new(AssetIcon::Wrench)
                                .size(px(12.))
                                .text_color(theme.muted_foreground),
                        )
                        .child(
                            div().text_xs().font_weight(FontWeight::SEMIBOLD).child(format!(
                                "{} actions {}",
                                activities.len(),
                                if self.tools_expanded { "\u{25be}" } else { "\u{25b8}" }
                            )),
                        ),
                );
            if self.tools_expanded {
                for a in &activities {
                    card = card.child(activity_row(a));
                }
            }
            pane = pane.child(card);
        }

        // Active tools currently running.
        for t in arr(thread_v, "activeTools") {
            let item = str_at(t, "item_id");
            if item.is_empty() {
                continue;
            }
            pane = pane.child(
                div()
                    .text_xs()
                    .text_color(theme.primary)
                    .child(format!("\u{25cf} tool running: {item}")),
            );
        }

        // Proposed plans (rendered as markdown cards).
        for p in arr(thread_v, "proposedPlans") {
            let md = str_at(p, "plan_markdown");
            if md.trim().is_empty() {
                continue;
            }
            let implemented = !get(p, "implemented_at").is_null();
            let mut card = v_flex()
                .gap_1()
                .w_full()
                .bg(theme.list_active)
                .rounded_md()
                .p_2()
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.primary)
                        .child(if implemented {
                            "PROPOSED PLAN (implemented)".to_string()
                        } else {
                            "PROPOSED PLAN".to_string()
                        }),
                );
            card = card.child(
                TextView::markdown(
                    ElementId::Name(format!("plan-{}", str_at(p, "id")).into()),
                    md.to_string(),
                )
                .selectable(true),
            );
            pane = pane.child(card);
        }

        // Pending interactions (approvals awaiting a decision).
        for p in arr(thread_v, "pendingInteractions") {
            if str_at(p, "status") != "pending" {
                continue;
            }
            let kind = str_at(p, "interaction_kind").replace('_', " ");
            let rid = str_at(p, "request_id").to_string();
            let mut row_el = h_flex().gap_2().child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.warning)
                    .child(format!("approval pending: {kind}")),
            );
            if !rid.is_empty() {
                for (label, decision) in [("Approve", "accept"), ("Decline", "decline")] {
                    let r = rid.clone();
                    row_el = row_el.child(
                        Button::new(ElementId::Name(
                            format!("approve-{decision}-{rid}").into(),
                        ))
                        .ghost()
                        .label(label)
                        .on_click(cx.listener(move |this, _ev, _w, cx| {
                            this.respond_approval(r.clone(), decision, cx)
                        })),
                    );
                }
            }
            pane = pane.child(row_el);
        }

        // ---- files browser / viewer ----
        let mut files_pane = v_flex().flex_1().h_full().p_4().gap_1();
        if self.view_files {
            if let Some((path, text)) = self.open_file.clone() {
                if self.file_editor.is_none() {
                    self.file_editor = Some(cx.new(|cx| EditorState::new(window, cx)));
                }
                let editor = self.file_editor.clone().unwrap();
                let sync_key = format!(
                    "{}:{}",
                    path,
                    self.open_file_meta
                        .as_ref()
                        .map(|m| m.0.as_str())
                        .unwrap_or("")
                );
                if !self.editing_file && self.editor_synced.as_deref() != Some(&sync_key) {
                    let lang = Self::editor_language(&path);
                    let t = text.clone();
                    editor.update(cx, |s, cx| {
                        s.set_highlighter(lang, cx);
                        s.set_value(t, window, cx);
                    });
                    self.editor_synced = Some(sync_key);
                }
                let p = path.clone();
                let editing = self.editing_file;
                let mut header = h_flex()
                    .gap_2()
                    .child(
                        Button::new("close-file")
                            .ghost()
                            .label("x")
                            .on_click(cx.listener(|this, _ev, _w, cx| this.close_file(cx))),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(p),
                    );
                if editing {
                    header = header
                        .child(
                            Button::new("save-file")
                                .primary()
                                .label("Save")
                                .on_click(cx.listener(|this, _ev, _w, cx| this.save_file(cx))),
                        )
                        .child(
                            Button::new("cancel-edit")
                                .ghost()
                                .label("Cancel")
                                .on_click(cx.listener(|this, _ev, _w, cx| {
                                    this.editing_file = false;
                                    this.editor_synced = None;
                                    cx.notify();
                                })),
                        );
                } else {
                    header = header.child(
                        Button::new("edit-file")
                            .ghost()
                            .label("Edit")
                            .on_click(cx.listener(|this, _ev, _w, cx| {
                                this.editing_file = true;
                                cx.notify();
                            })),
                    );
                }
                files_pane = files_pane
                    .child(header)
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .child(Editor::new(&editor).readonly(!editing)),
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
                            .text_color(theme.primary)
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
                                theme.primary
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

        // ---- dev servers pane ----
        let mut dev_pane = v_flex().flex_1().h_full().p_4().gap_2();
        if self.view_dev {
            dev_pane = dev_pane.child(
                h_flex()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.muted_foreground)
                            .child("DEV SERVERS"),
                    )
                    .child(
                        Button::new("dev-refresh")
                            .ghost()
                            .label("Refresh")
                            .on_click(cx.listener(|this, _ev, _w, cx| {
                                this.fetch_dev_servers(cx)
                            })),
                    ),
            );
            dev_pane = dev_pane.child(
                h_flex()
                    .gap_2()
                    .child(div().flex_1().child(Input::new(&self.dev_cmd)))
                    .child(
                        Button::new("dev-run")
                            .primary()
                            .label("Run")
                            .on_click(cx.listener(|this, _ev, w, cx| this.dev_run(w, cx))),
                    ),
            );
            if self.dev_servers.is_empty() {
                dev_pane = dev_pane.child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child("No dev servers running"),
                );
            }
            for s in self.dev_servers.clone() {
                let pid_key = str_at(&s, "projectId").to_string();
                let status = str_at(&s, "status");
                let color = match status {
                    "running" => theme.primary,
                    "failed" | "exited" | "stopped" => theme.danger,
                    _ => theme.muted_foreground,
                };
                dev_pane = dev_pane.child(
                    h_flex()
                        .gap_2()
                        .child(
                            div()
                                .flex_1()
                                .text_xs()
                                .font_family("monospace")
                                .child(str_at(&s, "command").to_string()),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(color)
                                .child(format!("{status} (pid {})", get(&s, "pid"))),
                        )
                        .child(
                            Button::new(ElementId::Name(
                                format!("dev-stop-{pid_key}").into(),
                            ))
                            .ghost()
                            .label("Stop")
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                this.dev_stop(pid_key.clone(), cx)
                            })),
                        ),
                );
            }
        }

        // ---- inbox pane (GitHub PRs + issues) ----
        let mut inbox_pane = v_flex().flex_1().h_full().p_4().gap_2();
        if self.view_inbox {
            inbox_pane = inbox_pane.child(
                h_flex()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.muted_foreground)
                            .child("INBOX"),
                    )
                    .child(
                        Button::new("inbox-refresh")
                            .ghost()
                            .label("Refresh")
                            .on_click(cx.listener(|this, _ev, _w, cx| {
                                this.fetch_inbox(cx)
                            })),
                    ),
            );
            if self.inbox_loading {
                inbox_pane = inbox_pane.child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child("Loading…"),
                );
            } else if self.inbox_items.is_empty() {
                inbox_pane = inbox_pane.child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(if self.inbox_errors.is_empty() {
                            "No open pull requests or issues".to_string()
                        } else {
                            self.inbox_errors
                                .iter()
                                .map(|e| str_at(e, "message").to_string())
                                .collect::<Vec<_>>()
                                .join("\n")
                        }),
                );
            }
            for it in self.inbox_items.clone() {
                let is_pr = str_at(&it, "kind") == "pullRequest";
                let url = str_at(&it, "url").to_string();
                let state = str_at(&it, "state").to_lowercase();
                let state_color = match state.as_str() {
                    "merged" | "closed" => theme.muted_foreground,
                    "open" => theme.primary,
                    _ => theme.muted_foreground,
                };
                inbox_pane = inbox_pane.child(
                    h_flex()
                        .id(ElementId::Name(format!("inbox-{}-{}", str_at(&it, "repository"), get(&it, "number")).into()))
                        .gap_2()
                        .items_center()
                        .cursor_pointer()
                        .on_mouse_down(gpui::MouseButton::Left, move |_, _, cx| {
                            if !url.is_empty() {
                                cx.open_url(&url);
                            }
                        })
                        .child(
                            Icon::new(if is_pr {
                                AssetIcon::GitPullRequest
                            } else {
                                AssetIcon::CircleDot
                            })
                            .size(px(14.))
                            .text_color(state_color),
                        )
                        .child(
                            v_flex()
                                .flex_1()
                                .gap_0p5()
                                .child(
                                    div().text_sm().child(format!(
                                        "#{} {}",
                                        get(&it, "number"),
                                        str_at(&it, "title")
                                    )),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .child(format!(
                                            "{} · {} · {}",
                                            str_at(&it, "repository"),
                                            str_at(get(&it, "author"), "login"),
                                            rel_time(str_at(&it, "updatedAt"))
                                        )),
                                ),
                        ),
                );
            }
        }

        // ---- composer ----
        // chat-composer-shell squircle: 19px radius card, hairline border,
        // soft surface shadow (approximated by the border + card tone).
        let circle_btn = |id: &'static str, icon: &'static [u8]| {
            div()
                .id(id)
                .size(px(28.))
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .border_1()
                .border_color(theme.border)
                .cursor_pointer()
                .child(
                    svg_icon(icon)
                        .size(px(16.))
                        .text_color(theme.muted_foreground),
                )
        };
        let composer_card = v_flex()
            .w_full()
            .max_w(px(759.))
            .border_1()
            .border_color(theme.border)
            .rounded(px(19.))
            .bg(theme.popover)
            .shadow_md()
            .px_3()
            .pt(px(10.))
            .pb(px(9.))
            .gap_2()
            .child(Input::new(&self.composer).appearance(false))
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        circle_btn("composer-attach", icons::PLUS).on_click(
                            cx.listener(|this, _ev, _w, cx| this.attach(cx)),
                        ),
                    )
                    .child(
                        h_flex()
                            .id("composer-mode")
                            .cursor_pointer()
                            .gap_1()
                            .items_center()
                            .child(
                                Icon::new(AssetIcon::TriangleAlert)
                                    .size(px(12.))
                                    .text_color(rgb(0xf59e0b)),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(rgb(0xf59e0b))
                                    .child(format!(
                                        "{} \u{25be}",
                                        match self.runtime_mode {
                                            "approval-required" => "Approval required",
                                            "auto" => "Approve for me",
                                            _ => "Full access",
                                        }
                                    )),
                            )
                            .on_click(cx.listener(|this, _ev, _w, cx| {
                                this.runtime_mode = match this.runtime_mode {
                                    "approval-required" => "auto",
                                    "auto" => "full-access",
                                    _ => "approval-required",
                                };
                                cx.notify();
                            })),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("composer-model")
                            .ghost()
                            .compact()
                            .label("\u{25b8} GPT-6 Astra \u{25be}")
                            .dropdown_menu(|menu, _, _| {
                                menu.item(PopupMenuItem::new("\u{2713} GPT-6 Astra (codex)"))
                            }),
                    )
                    .child({
                        // Empty draft renders the muted (disabled) circle like
                        // the web composer; filled primary once there's text.
                        let draft_empty = self.composer.read(cx).value().trim().is_empty();
                        div()
                            .id("send")
                            .size(px(28.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_full()
                            .bg(if draft_empty {
                                rgb(0xcfcfcf).into()
                            } else {
                                theme.primary
                            })
                            .cursor_pointer()
                            .child(
                                svg_icon(icons::SEND)
                                    .size(px(14.))
                                    .text_color(theme.background),
                            )
                            .on_click(cx.listener(|this, _ev, w, cx| this.send(w, cx)))
                    }),
            );
        let mut composer_col = v_flex().w_full().max_w(px(759.)).gap_0();
        if !self.pending_attachments.is_empty() || self.uploading {
            let mut chips = h_flex().gap_1().flex_wrap().pb_1();
            if self.uploading {
                chips = chips.child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child("uploading\u{2026}"),
                );
            }
            for (i, a) in self.pending_attachments.iter().enumerate() {
                let label = str_at(a, "name");
                let label = if label.is_empty() {
                    str_at(a, "fileName").to_string()
                } else {
                    label.to_string()
                };
                chips = chips.child(
                    h_flex()
                        .id(ElementId::Name(format!("att-{i}").into()))
                        .cursor_pointer()
                        .gap_1()
                        .items_center()
                        .border_1()
                        .border_color(theme.border)
                        .rounded_md()
                        .px_2()
                        .py_0p5()
                        .on_click(cx.listener(move |this, _ev, _w, cx| {
                            if i < this.pending_attachments.len() {
                                this.pending_attachments.remove(i);
                            }
                            cx.notify();
                        }))
                        .child(div().text_xs().child(format!("{label} \u{00d7}"))),
                );
            }
            composer_col = composer_col.child(chips);
        }
        composer_col = composer_col.child(composer_card);
        let composer_bar = div()
            .w_full()
            .pb(px(16.))
            .child(
                h_flex()
                    .justify_center()
                    .pr(px(40.))
                    .child(composer_col),
            );

        // ---- settings pane ----
        let settings_pane = v_flex()
            .size_full()
            .p_4()
            .gap_4()
            .child(
                div()
                    .text_xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Settings"),
            )
            .child(
                v_flex()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.muted_foreground)
                            .child("APPEARANCE"),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("theme-light")
                                    .icon(IconName::Sun)
                                    .label("Light")
                                    .on_click(cx.listener(|_this, _ev, _w, cx| {
                                        Theme::change(ThemeMode::Light, None, cx);
                                    })),
                            )
                            .child(
                                Button::new("theme-dark")
                                    .icon(IconName::Moon)
                                    .label("Dark")
                                    .on_click(cx.listener(|_this, _ev, _w, cx| {
                                        Theme::change(ThemeMode::Dark, None, cx);
                                    })),
                            ),
                    ),
            )
            .child(
                v_flex()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.muted_foreground)
                            .child("ABOUT"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .child("Synara (Rust port)"),
                    )
                    .child(
                        Button::new("about-repo")
                            .ghost()
                            .compact()
                            .icon(IconName::ExternalLink)
                            .label("github.com/cmdr-chara/synara")
                            .on_click(cx.listener(|_this, _ev, _w, cx| {
                                cx.open_url(
                                    "https://github.com/cmdr-chara/synara/tree/feat/rust-port",
                                );
                            })),
                    ),
            );

        // ---- terminal drawer (bottom pane over the transcript column) ----
        let mut term_drawer = v_flex()
            .h(px(220.))
            .border_t_1()
            .border_color(theme.border);
        let mut term_head = h_flex()
            .px_2()
            .py_1()
            .gap_2()
            .items_center()
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.muted_foreground)
                    .child("TERMINAL"),
            );
        if let Some(st) = &self.term_status {
            term_head = term_head.child(
                div()
                    .text_xs()
                    .text_color(if st == "exited" { theme.warning } else { theme.success })
                    .child(st.clone()),
            );
        }
        term_head = term_head
            .child(div().flex_1())
            .child(
                Button::new("term-restart")
                    .ghost()
                    .compact()
                    .icon(IconName::RefreshCw)
                    .on_click(cx.listener(|this, _ev, _w, cx| this.term_restart(cx))),
            )
            .child(
                Button::new("term-close")
                    .ghost()
                    .compact()
                    .label("\u{00d7}")
                    .on_click(cx.listener(|this, _ev, w, cx| this.toggle_terminal(w, cx))),
            );
        term_drawer = term_drawer
            .child(term_head)
            .child(
                div()
                    .flex_1()
                    .overflow_y_scrollbar()
                    .px_2()
                    .py_1()
                    .font_family(theme.mono_font_family.clone())
                    .text_xs()
                    .child(if self.term_buf.is_empty() {
                        "Starting terminal\u{2026}".to_string()
                    } else {
                        self.term_buf.clone()
                    }),
            );
        if let Some(inp) = &self.term_input {
            term_drawer = term_drawer.child(
                h_flex()
                    .px_2()
                    .pb_1()
                    .gap_1()
                    .items_center()
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.success)
                            .font_family(theme.mono_font_family.clone())
                            .child("$"),
                    )
                    .child(div().flex_1().child(Input::new(inp))),
            );
        }

        // ---- transcript find bar (Cmd+F) ----
        let mut find_bar = h_flex()
            .px_2()
            .py_1()
            .gap_2()
            .items_center()
            .border_b_1()
            .border_color(theme.border);
        if let Some(fi) = &self.find_input {
            let detail = self.detail.clone().unwrap_or(Value::Null);
            let tv = get(&detail, "thread");
            let q = self.find_query.to_lowercase();
            let count = if q.is_empty() {
                0
            } else {
                arr(tv, "messages")
                    .iter()
                    .filter(|m| {
                        let t = str_at(m, "text").to_string();
                        let t = if t.trim().is_empty() {
                            get(m, "textSegments")
                                .as_array()
                                .map(|segs| {
                                    segs.iter()
                                        .filter_map(|s| {
                                            s.get("text").and_then(Value::as_str)
                                        })
                                        .collect::<Vec<_>>()
                                        .join("")
                                })
                                .unwrap_or_default()
                        } else {
                            t
                        };
                        t.to_lowercase().contains(&q)
                    })
                    .count()
            };
            find_bar = find_bar
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child("Find"),
                )
                .child(div().w(px(220.)).child(Input::new(fi)))
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(format!("{count} matches")),
                )
                .child(div().flex_1())
                .child(
                    Button::new("find-close")
                        .ghost()
                        .compact()
                        .label("\u{00d7}")
                        .on_click(cx.listener(|this, _ev, w, cx| {
                            this.toggle_find(w, cx);
                        })),
                );
        }

        // ---- main column under the strip: panes + composer ----
        let mut main = v_flex().flex_1().h_full();
        if self.view_settings {
            main = main.child(div().flex_1().child(settings_pane.overflow_y_scrollbar()));
        } else if self.view_dev {
            main = main.child(div().flex_1().child(dev_pane.overflow_y_scrollbar()));
        } else if self.view_files {
            main = main.child(div().flex_1().child(files_pane.overflow_y_scrollbar()));
        } else if self.view_inbox {
            main = main.child(div().flex_1().child(inbox_pane.overflow_y_scrollbar()));
        } else {
            if self.find_open {
                main = main.child(find_bar);
            }
            main = main.child(div().flex_1().child(pane.overflow_y_scrollbar()));
            if self.view_terminal {
                main = main.child(term_drawer);
            }
            main = main.child(composer_bar);
        }

        // ---- changes dock (right pane) ----
        let mut changes_pane = v_flex()
            .w(px(300.))
            .h_full()
            .p_3()
            .gap_2()
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.muted_foreground)
                            .child("CHANGES"),
                    )
                    .child(
                        Button::new("changes-refresh")
                            .ghost()
                            .compact()
                            .icon(IconName::RefreshCw)
                            .on_click(cx.listener(|this, _ev, _w, cx| {
                                this.changes_diff = None;
                                this.fetch_changes(cx);
                            })),
                    ),
            );
        if self.changes_loading {
            changes_pane = changes_pane.child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child("Loading\u{2026}"),
            );
        } else if self.changes_unavailable {
            changes_pane = changes_pane.child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child("Not a git repository."),
            );
        } else if self.changes_files.is_empty() {
            changes_pane = changes_pane.child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child("No working tree changes."),
            );
        } else {
            for f in self.changes_files.clone() {
                let path = str_at(&f, "path").to_string();
                let ins = get(&f, "insertions").as_i64().unwrap_or(0);
                let del = get(&f, "deletions").as_i64().unwrap_or(0);
                let p = path.clone();
                changes_pane = changes_pane.child(
                    h_flex()
                        .id(ElementId::Name(format!("chg-{path}").into()))
                        .gap_2()
                        .items_center()
                        .cursor_pointer()
                        .child(div().flex_1().text_xs().child(path))
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.success)
                                .child(format!("+{ins}")),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.danger)
                                .child(format!("-{del}")),
                        )
                        .on_click(cx.listener(move |this, _ev, _w, cx| {
                            this.open_diff(p.clone(), cx);
                        })),
                );
            }
        }
        if let Some((path, patch)) = &self.changes_diff {
            changes_pane = changes_pane.child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_xs().child(path.clone()))
                    .child(
                        Button::new("diff-close")
                            .ghost()
                            .compact()
                            .label("\u{00d7}")
                            .on_click(cx.listener(|this, _ev, _w, cx| {
                                this.changes_diff = None;
                                cx.notify();
                            })),
                    ),
            );
            let mut diff_view = v_flex().gap_0();
            for line in patch.lines().take(400) {
                let color = if line.starts_with('+') && !line.starts_with("+++") {
                    theme.success
                } else if line.starts_with('-') && !line.starts_with("---") {
                    theme.danger
                } else if line.starts_with("@@") {
                    theme.accent
                } else {
                    theme.muted_foreground
                };
                diff_view = diff_view.child(
                    div()
                        .text_xs()
                        .font_family(theme.mono_font_family.clone())
                        .text_color(color)
                        .child(line.to_string()),
                );
            }
            changes_pane = changes_pane.child(
                div()
                    .flex_1()
                    .id("diff-scroll")
                    .overflow_y_scrollbar()
                    .child(diff_view),
            );
        }

        // ---- icon rail ----
        // --app-rail-width: 51px; 36px buttons on 8px radius, 6px gap,
        // 10px top/bottom padding. Idle glyphs sit at muted-foreground/58;
        // the active button gets the sidebar-selected (black/3%) tint and
        // an ink glyph. h-px w-5 hairlines separate the icon groups.
        let mut glyph_idle = theme.muted_foreground;
        glyph_idle.a *= 0.58;
        let rail_btn = |id: &'static str, icon: &'static [u8], active: bool, _cx: &mut Context<Self>| {
            div()
                .id(ElementId::Name(id.into()))
                .size(px(36.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(8.))
                .cursor_pointer()
                .when(active, |d| d.bg(theme.secondary))
                .when(!active, |d| {
                    d.hover(|h| h.bg(theme.secondary))
                })
                .child(
                    svg_icon(icon)
                        .size(px(20.))
                        .text_color(if active { theme.foreground } else { glyph_idle }),
                )
        };
        let _rail_divider = || {
            div()
                .w(px(20.))
                .h(px(1.))
                .bg(theme.border)
                .my(px(3.))
        };
        let home_active =
            !self.view_files && !self.view_dev && !self.view_settings && !self.view_inbox;
        let rail = v_flex()
            .w(px(51.))
            .h_full()
            .pt(px(7.))
            .pb(px(4.))
            .items_center()
            .gap(px(6.))
            .child(
                rail_btn("nav-home", icons::HOME, home_active, cx).on_click(cx.listener(
                    |this, _ev, _w, cx| {
                        this.view_files = false;
                        this.view_dev = false;
                        this.view_settings = false;
                        this.view_inbox = false;
                        this.view_changes = false;
                        cx.notify();
                    },
                )),
            )
            .child(
                rail_btn("nav-inbox", icons::INBOX, self.view_inbox, cx).on_click(
                    cx.listener(|this, _ev, _w, cx| {
                        this.toggle_inbox(cx);
                    }),
                ),
            )
            .child(
                rail_btn("nav-spaces", icons::SPACES, self.view_files, cx).on_click(
                    cx.listener(|this, _ev, _w, cx| {
                        this.view_settings = false;
                        this.view_inbox = false;
                        this.toggle_files(cx);
                    }),
                ),
            )
            .child(
                rail_btn("nav-tasks", icons::TASKS, self.view_dev, cx).on_click(
                    cx.listener(|this, _ev, _w, cx| {
                        this.view_settings = false;
                        this.view_inbox = false;
                        this.toggle_dev(cx);
                    }),
                ),
            )
            .child(
                rail_btn("nav-pullrequests", icons::PULL_REQUESTS, false, cx).on_click(
                    cx.listener(|this, _ev, _w, cx| {
                        this.toggle_inbox(cx);
                    }),
                ),
            )
            .child(
                rail_btn("nav-automations", icons::AUTOMATIONS, false, cx).on_click(
                    cx.listener(|this, _ev, _w, cx| {
                        this.view_settings = false;
                        this.view_inbox = false;
                        this.toggle_dev(cx);
                    }),
                ),
            )
            .child(
                rail_btn("nav-studio", icons::STUDIO, false, cx).on_click(
                    cx.listener(|this, _ev, _w, cx| {
                        this.view_settings = false;
                        this.view_inbox = false;
                        this.view_files = false;
                        this.toggle_dev(cx);
                    }),
                ),
            )
            .child(
                rail_btn("nav-more", icons::MORE, self.view_settings, cx).on_click(
                    cx.listener(|this, _ev, _w, cx| {
                        this.view_settings = !this.view_settings;
                        if this.view_settings {
                            this.view_files = false;
                            this.view_dev = false;
                            this.view_inbox = false;
                        }
                        cx.notify();
                    }),
                ),
            )
            .child(div().flex_1())
            .child(
                rail_btn("nav-help", icons::HELP, false, cx)
                    .on_click(cx.listener(|_this, _ev, _w, cx| {
                        cx.open_url("https://github.com/Emanuele-web04/synara");
                    })),
            )
            .child(
                rail_btn("nav-user", icons::USER, false, cx)
                    .on_click(cx.listener(|_this, _ev, _w, cx| {
                        cx.open_url("https://github.com/Emanuele-web04/synara");
                    })),
            );

        // ---- top strip (AppShellTopStrip) ----
        // 47px #f7f7f7 band + 1px border over rail+panel+content: sidebar
        // toggle centered over the rail at left, Environment + dock toggles
        // at right (Hugeicons panel-top-open / layout-right).
        let strip = h_flex()
            .w_full()
            .h(px(47.))
            .bg(rgb(0xf7f7f7))
            .border_b_1()
            .border_color(rgb(0xe5e5e5))
            .items_center()
            .pr(px(19.))
            .child(
                div()
                    .w(px(51.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        rail_btn("nav-sidebartoggle", icons::SIDEBAR_LEFT, false, cx).on_click(
                            cx.listener(|this, _ev, _w, cx| {
                                this.sidebar_collapsed = !this.sidebar_collapsed;
                                cx.notify();
                            }),
                        ),
                    ),
            )
            .child(div().flex_1())
            .child(
                rail_btn("nav-env", icons::PANEL_TOP_OPEN, self.view_files, cx).on_click(
                    cx.listener(|this, _ev, _w, cx| {
                        this.toggle_files(cx);
                    }),
                ),
            )
            .child(
                rail_btn("nav-dock", icons::LAYOUT_RIGHT, self.view_changes, cx).on_click(
                    cx.listener(|this, _ev, _w, cx| {
                        this.toggle_changes(cx);
                    }),
                ),
            );

        // ---- left column: #f7f7f7 well holding the rail and the floating
        // white sidebar card (bordered right+bottom, rounded corners, ~4px
        // bottom inset) exactly like upstream's app-rail-panel.
        let mut left_well = h_flex()
            .h_full()
            .bg(rgb(0xf7f7f7))
            .child(rail.border_r_1().border_color(rgb(0xe5e5e5)));
        if !self.sidebar_collapsed {
            left_well = left_well.child(
                v_flex()
                    .w(px(257.))
                    .h_full()
                    .mb(px(4.))
                    .bg(theme.background)
                    .border_r_1()
                    .border_b_1()
                    .border_color(rgb(0xe5e5e5))
                    .rounded(px(10.))
                    .overflow_hidden()
                    .child(sidebar.w_full()),
            );
        }

        let mut body = h_flex()
            .flex_1()
            .h_full()
            .child(left_well)
            .child(main);
        if self.view_changes {
            body = body.child(changes_pane.border_l_1().border_color(theme.border));
        }

        v_flex()
            .size_full()
            .key_context("native")
            .on_action(cx.listener(|this, _: &ToggleFind, w, cx| {
                this.toggle_find(w, cx);
            }))
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(strip)
            .child(body)
    }
}
