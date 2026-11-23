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
    theme::Theme,
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
    pub const P_CLAUDE: &[u8] = include_bytes!("../assets/icons/claude.svg");
    pub const P_CURSOR: &[u8] = include_bytes!("../assets/icons/cursor.svg");
    pub const P_GROK: &[u8] = include_bytes!("../assets/icons/grok.svg");
    pub const P_PI: &[u8] = include_bytes!("../assets/icons/pi.svg");
    pub const P_OPENCODE: &[u8] = include_bytes!("../assets/icons/opencode.svg");
    pub const P_DROID: &[u8] = include_bytes!("../assets/icons/droid.svg");
    pub const P_DEVIN: &[u8] = include_bytes!("../assets/icons/devin.svg");
    pub const P_OMP: &[u8] = include_bytes!("../assets/icons/omp.svg");
    pub const P_ANTIGRAVITY: &[u8] = include_bytes!("../assets/icons/antigravity.svg");
    pub const LOGO: &[u8] = include_bytes!("../assets/icons/synara-logo.svg");
    pub const SHIELD_ACCESS: &[u8] =
        include_bytes!("../assets/icons/shield-access.svg");
    pub const SHIELD_CODE: &[u8] = include_bytes!("../assets/icons/shield-code.svg");
    pub const CHEVRON_DOWN: &[u8] =
        include_bytes!("../assets/icons/chevron-down-small.svg");
    pub const CHEVRON_RIGHT: &[u8] =
        include_bytes!("../assets/icons/chevron-right.svg");
    pub const WORKTREE: &[u8] =
        include_bytes!("../assets/icons/worktree-split.svg");
    pub const REFRESH: &[u8] =
        include_bytes!("../assets/icons/arrow-rotate-clockwise.svg");
    pub const SORT: &[u8] =
        include_bytes!("../assets/icons/sort-arrow-up-down.svg");
    pub const FILTER: &[u8] = include_bytes!("../assets/icons/filter-1.svg");
    pub const PLUS_MED: &[u8] = include_bytes!("../assets/icons/plus-medium.svg");
    pub const PENCIL: &[u8] = include_bytes!("../assets/icons/pencil.svg");
    pub const TRASH: &[u8] = include_bytes!("../assets/icons/trash-can.svg");
    pub const S_PEOPLE_ID_CARD: &[u8] =
        include_bytes!("../assets/icons/people-id-card.svg");
    pub const S_COLOR_PALETTE: &[u8] =
        include_bytes!("../assets/icons/color-palette.svg");
    pub const S_LAYOUT_SIDEBAR: &[u8] =
        include_bytes!("../assets/icons/layout-sidebar.svg");
    pub const S_CHAT_BUBBLES: &[u8] =
        include_bytes!("../assets/icons/chat-bubbles.svg");
    pub const S_BELL: &[u8] = include_bytes!("../assets/icons/bell.svg");
    pub const S_SHORTCUT: &[u8] = include_bytes!("../assets/icons/shortcut.svg");
    pub const S_PUZZLE: &[u8] = include_bytes!("../assets/icons/puzzle.svg");
    pub const S_BRAIN: &[u8] = include_bytes!("../assets/icons/brain.svg");
    pub const S_BUILDING_BLOCKS: &[u8] =
        include_bytes!("../assets/icons/building-blocks.svg");
    pub const S_GAUGE: &[u8] = include_bytes!("../assets/icons/gauge.svg");
    pub const S_CONNECTORS: &[u8] =
        include_bytes!("../assets/icons/connectors-1.svg");
    pub const S_PLUGIN: &[u8] = include_bytes!("../assets/icons/plugin-1.svg");
    pub const S_GLOBE: &[u8] = include_bytes!("../assets/icons/globe.svg");
    pub const S_COMPUTER_USE: &[u8] =
        include_bytes!("../assets/icons/computer-use.svg");
    pub const S_SCREEN_CAPTURE: &[u8] =
        include_bytes!("../assets/icons/screen-capture.svg");
    pub const S_BRANCH_SIMPLE: &[u8] =
        include_bytes!("../assets/icons/branch-simple.svg");
    pub const S_ARCHIVE: &[u8] = include_bytes!("../assets/icons/archive.svg");
    pub const S_SHIELD: &[u8] = include_bytes!("../assets/icons/shield.svg");
    pub const S_TOOLBOX: &[u8] = include_bytes!("../assets/icons/toolbox.svg");
    pub const S_CIRCLE_INFO: &[u8] =
        include_bytes!("../assets/icons/circle-info.svg");
    pub const S_GEAR: &[u8] =
        include_bytes!("../assets/icons/settings-gear-4.svg");
    pub const B_SUPABASE: &[u8] = include_bytes!("../assets/icons/brands/supabase.svg");
    pub const B_VERCEL: &[u8] = include_bytes!("../assets/icons/brands/vercel.svg");
    pub const B_SLACK: &[u8] = include_bytes!("../assets/icons/brands/slack.svg");
    pub const B_LINEAR: &[u8] = include_bytes!("../assets/icons/brands/linear.svg");
    pub const B_NOTION: &[u8] = include_bytes!("../assets/icons/brands/notion.svg");
    pub const B_STRIPE: &[u8] = include_bytes!("../assets/icons/brands/stripe.svg");
    pub const B_WORKOS: &[u8] = include_bytes!("../assets/icons/brands/workos.svg");
    pub const B_CONVEX: &[u8] = include_bytes!("../assets/icons/brands/convex.svg");
    pub const B_CLOUDFLARE: &[u8] = include_bytes!("../assets/icons/brands/cloudflare.svg");
    pub const B_SENTRY: &[u8] = include_bytes!("../assets/icons/brands/sentry.svg");
    pub const B_NEON: &[u8] = include_bytes!("../assets/icons/brands/neon.svg");
    pub const B_POSTHOG: &[u8] = include_bytes!("../assets/icons/brands/posthog.svg");
    pub const B_NETLIFY: &[u8] = include_bytes!("../assets/icons/brands/netlify.svg");
    pub const B_RAILWAY: &[u8] = include_bytes!("../assets/icons/brands/railway.svg");
    pub const B_PRISMA: &[u8] = include_bytes!("../assets/icons/brands/prisma.svg");
    pub const B_HUGGINGFACE: &[u8] = include_bytes!("../assets/icons/brands/huggingface.svg");
    pub const B_ATLASSIAN: &[u8] = include_bytes!("../assets/icons/brands/atlassian.svg");
    pub const B_TODOIST: &[u8] = include_bytes!("../assets/icons/brands/todoist.svg");
    pub const B_MCP: &[u8] = include_bytes!("../assets/icons/brands/mcp.svg");
}

fn svg_icon(data: &'static [u8]) -> Icon {
    Icon::default().data(data)
}

// Small ghost icon button matching the web sidebar-icon-button affordance.
fn icon_btn(id: &'static str, data: &'static [u8], size: f32, hover: Hsla, ink: Hsla) -> Stateful<Div> {
    icon_btn_pad(id, data, size, 16., hover, ink)
}

fn icon_btn_pad(
    id: &'static str,
    data: &'static [u8],
    size: f32,
    pad: f32,
    hover: Hsla,
    ink: Hsla,
) -> Stateful<Div> {
    div()
        .id(ElementId::Name(id.into()))
        .size(px(size + pad))
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
    settings_section: &'static str,
    view_inbox: bool,
    view_tasks: bool,
    view_kanban: bool,
    view_pullreq: bool,
    view_autom: bool,
    view_studio: bool,
    view_spaces: bool,
    inbox_items: Vec<Value>,
    inbox_errors: Vec<Value>,
    inbox_loading: bool,
    inbox_tasks: Vec<Value>,
    inbox_recap: Option<Value>,
    inbox_usage: Vec<Value>,
    inbox_signed_in: Option<bool>,
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
    settings_scroll: ScrollHandle,
    settings_nav_scroll: ScrollHandle,
    settings_data: Value,
    settings_loading: bool,
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
                InputState::new(window, cx).placeholder("")
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
                settings_section: "general",
                view_inbox: false,
                view_tasks: false,
                view_kanban: false,
                view_pullreq: false,
                view_autom: false,
                view_studio: false,
                view_spaces: false,
                inbox_items: Vec::new(),
                inbox_errors: Vec::new(),
                inbox_loading: false,
                inbox_tasks: Vec::new(),
                inbox_recap: None,
                inbox_usage: Vec::new(),
                inbox_signed_in: None,
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
                settings_scroll: ScrollHandle::new(),
                settings_nav_scroll: ScrollHandle::new(),
                settings_data: Value::Null,
                settings_loading: false,
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
            self.view_tasks = false;
            self.view_kanban = false;
            self.view_pullreq = false;
            self.view_autom = false;
            self.view_studio = false;
            self.view_spaces = false;
            self.view_settings = false;
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
            self.view_tasks = false;
            self.view_kanban = false;
            self.view_pullreq = false;
            self.view_autom = false;
            self.view_studio = false;
            self.view_spaces = false;
            self.fetch_inbox(cx);
        }
        cx.notify();
    }

    /// Clear every rail-route surface (the transcript shows when all are
    /// off). Docks (changes/terminal) are independent surfaces, untouched.
    fn clear_views(&mut self) {
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
    }

    /// Rail route toggle: open that surface (closing the others); clicking
    /// the active one again returns to the transcript.
    fn open_route(&mut self, which: u8, cx: &mut Context<Self>) {
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
            let bundle = cx
                .background_executor()
                .spawn(async move {
                    let Some(c) = ensure_client(&cell) else {
                        return Value::Null;
                    };
                    let tasks = c.call("todo.list", json!({})).unwrap_or(Value::Null);
                    let recap = c.call("stats.getRecap", json!({})).unwrap_or(Value::Null);
                    let usage = c
                        .call("server.listProviderUsage", json!({}))
                        .unwrap_or(Value::Null);
                    let acct = c.call("account.status", json!({})).unwrap_or(Value::Null);
                    json!({"tasks": tasks, "recap": recap, "usage": usage, "account": acct})
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
                cx.notify();
            });
        })
        .detach();
    }

    /// Blocking RPC bundle for the settings panes: provider statuses,
    /// usage quotas, skills, connectors, integrations, worktrees,
    /// profile stats, computer backend status, archived thread count.
    fn fetch_settings(&self, cx: &mut Context<Self>) {
        let cell = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.settings_loading = true;
                cx.notify();
            });
            let bundle = cx
                .background_executor()
                .spawn(async move {
                    let Some(c) = ensure_client(&cell) else {
                        return Value::Null;
                    };
                    let cfg = c.call("server.getConfig", json!({})).unwrap_or(Value::Null);
                    let app = c.call("server.getSettings", json!({})).unwrap_or(Value::Null);
                    let usage = c.call("server.listProviderUsage", json!({})).unwrap_or(Value::Null);
                    let skills = c.call("provider.listSkills", json!({})).unwrap_or(Value::Null);
                    let conns = c.call("server.listMcpConnectors", json!({})).unwrap_or(Value::Null);
                    let props = c.call("server.listMcpConnectorProposals", json!({})).unwrap_or(Value::Null);
                    let exts = c.call("server.listExternalMcpIntegrations", json!({})).unwrap_or(Value::Null);
                    let wts = c.call("server.listWorktrees", json!({})).unwrap_or(Value::Null);
                    let prof = c.call("stats.getProfileStats", json!({})).unwrap_or(Value::Null);
                    let toks = c.call("stats.getProfileTokenStats", json!({})).unwrap_or(Value::Null);
                    let ana = c.call("stats.getUsageAnalytics", json!({})).unwrap_or(Value::Null);
                    let comp = c.call("computer.getStatus", json!({})).unwrap_or(Value::Null);
                    let snap = c.call("orchestration.getShellSnapshot", json!({})).unwrap_or(Value::Null);
                    let acct = c.call("account.status", json!({})).unwrap_or(Value::Null);
                    json!({"config":cfg,"settings":app,"usage":usage,"skills":skills,
                           "connectors":conns,"proposals":props,"external":exts,
                           "worktrees":wts,"profile":prof,"tokens":toks,"analytics":ana,
                           "computer":comp,"snapshot":snap,"account":acct})
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.settings_loading = false;
                this.settings_data = bundle;
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

    /// Workspace root of the selected thread's project, if resolvable.
    fn thread_project_root(&self) -> Option<String> {
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
                if r.is_empty() { str_at(p, "workspace_root") } else { r }
            })
            .filter(|r| !r.is_empty());
        root.map(|r| r.to_string())
    }

    /// Fetch git.status of the selected thread's project root (fallback: first project).
    fn fetch_git_status(&self, cx: &mut Context<Self>) {
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
                    icon_btn_pad("sidebar-search", icons::SEARCH, 18., 8., theme.secondary, theme.muted_foreground)
                        .on_click(cx.listener(|this, _ev, _w, cx| {
                            this.search_open = !this.search_open;
                            cx.notify();
                        })),
                )
                .child(
                    icon_btn_pad("sidebar-new", icons::NEW_THREAD, 18., 8., theme.secondary, theme.muted_foreground)
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
            let proj_head = v_flex()
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
                    // Upstream: clicking a project row selects it and shows
                    // the project surface; it never opens the files browser.
                    this.view_files = false;
                    cx.notify();
                }));
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
        // Upstream shows the new-chat landing inside the thread pane when the
        // thread has no messages yet (same logo + heading + watermark + banner).
        let thread_msgs_empty = self.selected_thread.is_some()
            && arr(
                get(self.detail.as_ref().unwrap_or(&Value::Null), "thread"),
                "messages",
            )
            .is_empty();
        let landing_title: Option<String> = if self.selected_thread.is_none() {
            Some("What should we do in this folder?".to_string())
        } else if thread_msgs_empty {
            let tid = self.selected_thread.as_deref().unwrap_or("");
            let snap = self.snapshot.as_ref().unwrap_or(&Value::Null);
            let pid = arr(snap, "threads")
                .iter()
                .find(|t| str_at(t, "id") == tid)
                .map(|t| str_at(t, "projectId").to_string())
                .unwrap_or_default();
            let name = arr(snap, "projects")
                .iter()
                .find(|p| str_at(p, "id") == pid)
                .map(|p| str_at(p, "title").to_string())
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| "this folder".to_string());
            Some(format!("What should we do in {name}?"))
        } else {
            None
        };
        let show_landing = landing_title.is_some();
        if self.selected_thread.is_some() && !show_landing {
            pane = pane.p_4();
        }
        if show_landing {
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
                    div().absolute().top(px(15.)).left_0().w_full().child(
                        h_flex().w_full().justify_center().px_6().child(
                            h_flex()
                                .relative()
                                .id("sponsor-banner")
                                .group("sponsor-banner")
                                .cursor_pointer()
                                .w(px(520.))
                                .rounded(px(16.))
                                .px_4()
                                .py(px(12.))
                                .gap_4()
                                .items_center()
                                .hover(|d| d.bg(rgba(0x0000000a)))
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
                            .child(landing_title.clone().unwrap_or_default()),
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
            pane = pane.child(
                h_flex().w_full().justify_center().child(
                    div().w(px(755.)).child(header),
                ),
            );
        }
        let messages = arr(thread_v, "messages");
        let find_q = self.find_query.to_lowercase();
        if messages.is_empty() && self.selected_thread.is_some() && !show_landing {
            pane = pane.child(
                h_flex().w_full().justify_center().child(
                    div().w(px(755.)).child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("Select a thread or type below"),
                    ),
                ),
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
                    .rounded(px(16.))
                    .px_4()
                    .py(px(10.))
                    .max_w(px(560.));
                if is_match {
                    pill = pill.border_1().border_color(theme.warning);
                }
                pane = pane.child(
                    h_flex().w_full().justify_center().child(
                        h_flex()
                            .w(px(755.))
                            .justify_end()
                            .child(pill.child(div().text_sm().child(text))),
                    ),
                );
            } else {
                let mut wrap = div();
                if is_match {
                    wrap = wrap.border_l_2().border_color(theme.warning).pl_2();
                }
                pane = pane.child(
                    h_flex().w_full().justify_center().child(
                        div().w(px(755.)).child(wrap.child(
                            TextView::markdown(
                                ElementId::Name(format!("md-{i}-{}", str_at(m, "id")).into()),
                                text,
                            )
                            .selectable(true),
                        )),
                    ),
                );
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
            pane = pane.child(
                h_flex().w_full().justify_center().child(
                    div().w(px(755.)).child(activity_row(&activities[0])),
                ),
            );
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
            pane = pane.child(
                h_flex().w_full().justify_center().child(
                    div().w(px(755.)).child(card),
                ),
            );
        }

        // Active tools currently running.
        for t in arr(thread_v, "activeTools") {
            let item = str_at(t, "item_id");
            if item.is_empty() {
                continue;
            }
            pane = pane.child(
                h_flex().w_full().justify_center().child(
                    div().w(px(755.)).child(
                        div()
                            .text_xs()
                            .text_color(theme.primary)
                            .child(format!("\u{25cf} tool running: {item}")),
                    ),
                ),
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
            pane = pane.child(
                h_flex().w_full().justify_center().child(
                    div().w(px(755.)).child(card),
                ),
            );
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
            pane = pane.child(
                h_flex().w_full().justify_center().child(
                    div().w(px(755.)).child(row_el),
                ),
            );
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

        // ---- inbox pane (upstream /inbox recap layout) ----
        let mut inbox_pane = v_flex().flex_1().h_full().p_4().gap_2();
        if self.view_inbox {
            use chrono::Timelike as _;
            let now = chrono::Local::now();
            let slot = match now.hour() {
                5..=11 => "Good morning",
                12..=17 => "Good afternoon",
                _ => "Good evening",
            };
            let greeting = format!("{}. {}.", slot, now.format("%A, %B %-d"));
            let sub = if self.inbox_tasks.is_empty() {
                "You're all caught up.".to_string()
            } else {
                let n = self.inbox_tasks.len();
                format!("{n} thing{} need{} you.", if n == 1 { "" } else { "s" }, if n == 1 { "s" } else { "" })
            };
            let tile = |children: gpui::AnyElement| {
                div().rounded(px(16.)).bg(rgb(0xf6f6f6)).child(children)
            };

            let header = h_flex()
                .items_baseline()
                .gap_2()
                .child(
                    div()
                        .text_size(px(24.))
                        .font_weight(FontWeight::MEDIUM)
                        .child("Inbox"),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(format!("{greeting} {sub}")),
                );

            // Tasks card (left column, ~1/3 of the row)
            let tasks_card = v_flex()
                .w(px(300.))
                .gap_2()
                .p_4()
                .child(
                    h_flex()
                        .justify_between()
                        .items_center()
                        .child(div().text_sm().child(format!("Today's tasks {}", self.inbox_tasks.len())))
                        .child(div().text_sm().text_color(theme.muted_foreground).child("All tasks")),
                )
                .child(
                    h_flex()
                        .items_center()
                        .gap_2()
                        .child(
                            Icon::new(AssetIcon::CircleDot)
                                .size(px(14.))
                                .text_color(theme.muted_foreground),
                        )
                        .child(
                            div()
                                .text_sm()
                                .whitespace_nowrap()
                                .text_color(theme.muted_foreground)
                                .child("Add a task for today, or press Tab to hand it to an agent"),
                        ),
                );

            // Right column: recap + usage tiles
            let mut right_col = v_flex().flex_1().gap_3();
            let recap_text = self
                .inbox_recap
                .as_ref()
                .map(|r| str_at(r, "text").to_string())
                .filter(|s: &String| !s.is_empty())
                .unwrap_or_else(|| "Nothing ran yet today. This fills in as your agents work.".to_string());
            right_col = right_col.child(tile(
                div().p_4().text_sm().child(recap_text).into_any_element(),
            ));
            let provider = self
                .inbox_usage
                .first()
                .map(|u| str_at(u, "provider").to_string())
                .filter(|s: &String| !s.is_empty())
                .unwrap_or_else(|| "Devin".to_string());
            let pct = self
                .inbox_usage
                .first()
                .and_then(|u| get(u, "usedPct").as_f64().or_else(|| get(u, "remainingPct").as_f64()))
                .map(|p| if get(self.inbox_usage.first().unwrap_or(&Value::Null), "usedPct").is_f64() { 100.0 - p } else { p })
                .unwrap_or(93.0);
            let resets = self
                .inbox_usage
                .first()
                .map(|u| str_at(u, "resetsIn").to_string())
                .filter(|s: &String| !s.is_empty())
                .unwrap_or_else(|| "5d 18h".to_string());
            right_col = right_col.child(tile(
                v_flex()
                    .p_4()
                    .gap_2()
                    .child(div().text_sm().text_color(theme.muted_foreground).child(format!("Left on {provider}, weekly")))
                    .child(
                        h_flex()
                            .items_center()
                            .gap_1p5()
                            .child(svg_icon(icons::CODEX).size(px(18.)).text_color(theme.foreground))
                            .child(
                                div()
                                    .text_size(px(20.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(format!("{pct:.0}%")),
                            ),
                    )
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .w(px(48.))
                                    .h(px(6.))
                                    .rounded(px(3.))
                                    .bg(theme.border)
                                    .child(div().w(gpui::relative((pct / 100.0) as f32)).h_full().rounded(px(3.)).bg(rgb(0x22a865))),
                            )
                            .child(div().text_sm().text_color(theme.muted_foreground).child(format!("Resets in {resets}"))),
                    )
                    .into_any_element(),
            ));

            let grid = h_flex()
                .gap_3()
                .items_start()
                .child(tile(tasks_card.into_any_element()))
                .child(right_col);

            let mut page = v_flex().flex_1().p(px(32.)).gap_4().child(header).child(grid);
            if self.inbox_signed_in != Some(true) {
                page = page.child(
                    h_flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child("Sign in to automatically save private recaps across your devices."),
                        )
                        .child(Button::new("inbox-signin").outline().label("Sign in")),
                );
            }
            inbox_pane = inbox_pane.child(page);
        }

        // ---- composer ----
        // chat-composer-shell squircle: 19px radius card, hairline border,
        // soft surface shadow (approximated by the border + card tone).
        let mode_accent = rgb(0xc24904);
        let draft_empty = self.composer.read(cx).value().trim().is_empty();
        // Upstream ComposerPromptEditor: font-system-ui text-chat (13px)
        // leading-relaxed (1.625 -> ~21.1px/line), min-height 2lh (~42px),
        // padding x12/top12/bottom8; placeholder sits at the text origin.
        let mut input_zone = div().relative().child(
            div()
                .min_h(px(42.25))
                .child(Input::new(&self.composer).appearance(false)),
        );
        if draft_empty {
            // Placeholder rendered on top: the web uses muted-foreground/40,
            // much lighter than the editor's own muted foreground tone.
            input_zone = input_zone.child(
                div()
                    .absolute()
                    .top(px(12.))
                    .left(px(12.))
                    .text_size(px(13.))
                    .line_height(px(21.125))
                    .text_color(rgba(0x67676766))
                    .child("Do anything in Synara"),
            );
        }
        let composer_card = v_flex()
            .w_full()
            .max_w(px(755.))
            .border_1()
            .border_color(theme.border)
            .rounded(px(19.))
            .bg(theme.popover)
            .shadow_md()
            .px_2()
            .pt(px(4.))
            .pb(px(8.))
            .gap(px(8.))
            .child(input_zone)
            .child(
                h_flex()
                    .gap_3()
                    .items_center()
                    .child(
                        // "+" extras trigger: 28px filled circle, black/3%.
                        div()
                            .id("composer-attach")
                            .size(px(28.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_full()
                            .bg(rgba(0x00000008))
                            .cursor_pointer()
                            .hover(|d| d.bg(rgba(0x00000010)))
                            .child(
                                svg_icon(icons::PLUS)
                                    .size(px(14.))
                                    .text_color(theme.foreground),
                            )
                            .on_click(
                                cx.listener(|this, _ev, _w, cx| this.attach(cx)),
                            ),
                    )
                    .child(
                        h_flex()
                            .id("composer-mode")
                            .cursor_pointer()
                            .gap(px(6.))
                            .items_center()
                            .px(px(10.))
                            .py(px(3.))
                            .rounded(px(8.))
                            .child(
                                svg_icon(match self.runtime_mode {
                                    "auto" => icons::SHIELD_CODE,
                                    _ => icons::SHIELD_ACCESS,
                                })
                                .size(px(14.))
                                .text_color(mode_accent),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(mode_accent)
                                    .child(match self.runtime_mode {
                                        "approval-required" => "Approval required",
                                        "auto" => "Approve for me",
                                        _ => "Full access",
                                    }),
                            )
                            .child(
                                svg_icon(icons::CHEVRON_DOWN)
                                    .size(px(12.))
                                    .text_color(mode_accent.opacity(0.7)),
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
                            .icon(svg_icon(icons::CODEX))
                            .label("GPT-6 Astra")
                            .dropdown_caret(true)
                            .dropdown_menu(|menu, _, _| {
                                menu.item(PopupMenuItem::new("\u{2713} GPT-6 Astra (codex)"))
                            }),
                    )
                    .child({
                        // Empty draft renders the muted (disabled) circle like
                        // the web composer; filled primary once there's text.
                        div()
                            .id("send")
                            .size(px(28.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_full()
                            .bg(if draft_empty {
                                theme.foreground.opacity(0.2)
                            } else {
                                theme.foreground
                            })
                            .cursor_pointer()
                            .child(
                                svg_icon(icons::SEND)
                                    .size(px(20.))
                                    .text_color(theme.background),
                            )
                            .on_click(cx.listener(|this, _ev, w, cx| this.send(w, cx)))
                    }),
            );
        let mut composer_col = v_flex().w(px(755.)).gap_0();
        // Worktree tray above the card (thread view): folder+project,
        // split-icon+branch+chevron on the left; Temporary + Worktree
        // checkbox on the right (upstream BranchToolbar tray).
        if self.selected_thread.is_some() {
            let projects = arr(
                self.snapshot.as_ref().unwrap_or(&Value::Null),
                "projects",
            );
            let thread = arr(
                self.snapshot.as_ref().unwrap_or(&Value::Null),
                "threads",
            )
            .iter()
            .find(|t| str_at(t, "id") == self.selected_thread.as_deref().unwrap_or(""));
            let proj_name = thread
                .and_then(|t| {
                    let pid = str_at(t, "projectId");
                    projects
                        .iter()
                        .find(|p| str_at(p, "id") == pid)
                        .map(|p| str_at(p, "title").to_string())
                })
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| {
                    self.files_root()
                        .and_then(|r| {
                            std::path::Path::new(&r)
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                        })
                        .unwrap_or_else(|| "project".to_string())
                });
            let branch = self
                .git_status
                .as_ref()
                .map(|s| str_at(s, "branch").to_string())
                .unwrap_or_default();
            let tray = h_flex()
                .w_full()
                .items_center()
                .justify_between()
                .h(px(30.))
                .pb(px(6.))
                .child(
                    h_flex()
                        .gap(px(6.))
                        .items_center()
                        .child(
                            svg_icon(icons::FOLDER)
                                .size(px(14.))
                                .text_color(theme.muted_foreground),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(theme.foreground)
                                .child(proj_name),
                        )
                        .child(
                            svg_icon(icons::WORKTREE)
                                .size(px(14.))
                                .text_color(theme.muted_foreground),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(theme.foreground)
                                .child(if branch.is_empty() {
                                    "local".to_string()
                                } else {
                                    branch
                                }),
                        )
                        .child(
                            svg_icon(icons::CHEVRON_DOWN)
                                .size(px(10.))
                                .text_color(theme.muted_foreground),
                        ),
                )
                .child(
                    h_flex()
                        .gap(px(10.))
                        .items_center()
                        .child(
                            svg_icon(icons::AUTOMATIONS)
                                .size(px(13.))
                                .text_color(theme.muted_foreground),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(theme.muted_foreground)
                                .child("Temporary"),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(theme.muted_foreground)
                                .child("Worktree"),
                        )
                        .child(
                            div()
                                .size(px(14.))
                                .rounded(px(4.))
                                .border_1()
                                .border_color(theme.border),
                        ),
                );
            composer_col = composer_col
                .child(div().w_full().overflow_hidden().child(tray));
        }
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
            .pb(px(18.))
            .child(
                h_flex()
                    .w_full()
                    .justify_center()
                    .pr(px(4.))
                    .child(composer_col),
            );

        // ---- settings pane ----
        // ---- settings pane (upstream /settings layout) ----
        // Nav: (group label, [(section id, label, icon, badge)])
        let settings_nav: &[(&str, &[(&str, &str, &[u8], Option<&str>)])] = &[
            ("", &[
                ("profile", "Profile", icons::USER, None),
                ("accounts", "Accounts", icons::S_PEOPLE_ID_CARD, None),
            ]),
            ("Synara", &[
                ("general", "General", icons::S_GEAR, None),
                ("appearance", "Appearance", icons::S_COLOR_PALETTE, None),
                ("layout", "Layout", icons::S_LAYOUT_SIDEBAR, None),
                ("chats", "Chats", icons::S_CHAT_BUBBLES, None),
                ("notifications", "Notifications", icons::S_BELL, None),
                ("shortcuts", "Keyboard shortcuts", icons::S_SHORTCUT, None),
            ]),
            ("Agents", &[
                ("providers", "Providers", icons::S_PUZZLE, None),
                ("models", "Models", icons::S_BRAIN, None),
                ("skills", "Skills", icons::S_BUILDING_BLOCKS, None),
                ("usage", "Usage", icons::S_GAUGE, None),
            ]),
            ("Connections", &[
                ("connectors", "Connectors", icons::S_CONNECTORS, None),
                ("integrations", "External agents", icons::S_PLUGIN, None),
                ("devices", "Devices", icons::S_GLOBE, None),
                ("computer", "Computer use", icons::S_COMPUTER_USE, Some("Beta")),
                ("appsnap", "AppSnap", icons::S_SCREEN_CAPTURE, None),
            ]),
            ("Workspace", &[
                ("worktrees", "Worktrees", icons::S_BRANCH_SIMPLE, None),
                ("archive", "Archive", icons::S_ARCHIVE, None),
            ]),
            ("System", &[
                ("privacy", "Privacy", icons::S_SHIELD, None),
                ("advanced", "Advanced", icons::S_TOOLBOX, None),
                ("about", "About", icons::S_CIRCLE_INFO, None),
            ]),
        ];
        let section_title: &str = settings_nav
            .iter()
            .flat_map(|(_, items)| items.iter())
            .find(|(id, ..)| *id == self.settings_section)
            .map(|(_, l, ..)| *l)
            .unwrap_or("General");

        // Settings card: rounded-xl, faint fill (foreground/2%), hairline border.
        let card = |rows: gpui::AnyElement| {
            div()
                .rounded(px(12.))
                .border_1()
                .border_color(theme.border)
                .bg(rgb(0xfafafa))
                .child(rows)
        };
        // One row: title + description left, control right.
        let srow = |title: &'static str, desc: &'static str, ctrl: gpui::AnyElement| {
            h_flex()
                .w_full()
                .items_center()
                .justify_between()
                .gap_3()
                .px_3()
                .py(px(10.))
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .gap(px(2.))
                        .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(title))
                        .child(div().w_full().text_sm().text_color(theme.muted_foreground).child(desc)),
                )
                .child(ctrl)
        };
        let divider = || div().h(px(1.)).w_full().bg(theme.border);
        // Select-style control pill (static parity render).
        let select_pill = |label: String, icon: Option<&'static [u8]>| {
            let mut b = h_flex()
                .h(px(28.))
                .px(px(10.))
                .gap_1p5()
                .items_center()
                .rounded(px(8.))
                .border_1()
                .border_color(theme.border);
            if let Some(ic) = icon {
                b = b.child(svg_icon(ic).size(px(14.)).text_color(theme.muted_foreground));
            }
            b.child(div().text_sm().whitespace_nowrap().child(label))
                .child(svg_icon(icons::CHEVRON_DOWN).size(px(12.)).text_color(theme.muted_foreground))
                .into_any_element()
        };
        let outline_btn = |label: &'static str| {
            div()
                .flex_shrink_0()
                .h(px(28.))
                .px_3()
                .flex()
                .items_center()
                .rounded(px(8.))
                .border_1()
                .border_color(theme.border)
                .child(div().text_sm().whitespace_nowrap().child(label))
                .into_any_element()
        };
        let section_label = |t: &'static str| {
            div()
                .px_2()
                .py_1()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(t)
        };

        // ---- shared controls for ported settings sections ----
        let accent_c = rgb(0xc74614);
        let toggle = move |on: bool| {
            let mut t = div()
                .w(px(28.))
                .h(px(16.))
                .rounded_full()
                .flex()
                .items_center();
            if on {
                t = t.bg(accent_c).justify_end();
            } else {
                t = t.bg(rgb(0xd9d9d9)).justify_start();
            }
            t.child(
                div()
                    .w(px(12.))
                    .h(px(12.))
                    .rounded_full()
                    .bg(gpui::white())
                    .mx(px(2.)),
            )
            .into_any_element()
        };
        // Segmented control (active chip on light-gray).
        let seg = |opts: &[&'static str], active: usize| {
            let mut s = h_flex().items_center().rounded(px(8.)).p(px(2.)).gap(px(2.));
            for (i, o) in opts.iter().enumerate() {
                let mut c = div()
                    .h(px(24.))
                    .px(px(10.))
                    .flex()
                    .items_center()
                    .rounded(px(6.));
                if i == active {
                    c = c.bg(rgb(0xf0f0f0));
                }
                s = s.child(c.child(div().text_xs().child(*o)));
            }
            s.into_any_element()
        };
        // Color swatch pill: bordered pill with color box + hex label.
        // Color-filled pill with the hex label inside, like upstream's theme swatch.
        let swatch_pill = |hex_label: &'static str, color: u32| {
            let r = ((color >> 16) & 0xff) as f32;
            let g = ((color >> 8) & 0xff) as f32;
            let b = (color & 0xff) as f32;
            let lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
            let ink = if lum > 140. { rgba(0x000000aa) } else { rgba(0xffffffcc) };
            div()
                .w(px(160.))
                .h(px(26.))
                .px_3()
                .flex()
                .items_center()
                .rounded(px(8.))
                .border_1()
                .border_color(theme.border)
                .bg(rgb(color))
                .child(div().text_sm().text_color(ink).child(hex_label))
                .into_any_element()
        };
        // Thin track slider + value label.
        let slider = |frac: f32, val: &'static str| {
            h_flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .relative()
                        .w(px(110.))
                        .h(px(14.))
                        .child(
                            div()
                                .absolute()
                                .top(px(6.))
                                .left_0()
                                .w_full()
                                .h(px(2.))
                                .rounded_full()
                                .bg(theme.foreground),
                        )
                        .child(
                            div()
                                .absolute()
                                .top(px(1.))
                                .left(px(110f32 * frac - 6.))
                                .w(px(12.))
                                .h(px(12.))
                                .rounded_full()
                                .bg(gpui::white())
                                .border_1()
                                .border_color(theme.foreground),
                        ),
                )
                .child(div().text_xs().text_color(theme.muted_foreground).child(val))
                .into_any_element()
        };
        // Bordered number field + "px" suffix.
        let num_field = |v: &'static str| {
            h_flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .h(px(28.))
                        .w(px(44.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(8.))
                        .border_1()
                        .border_color(theme.border)
                        .child(div().text_sm().child(v)),
                )
                .child(div().text_sm().text_color(theme.muted_foreground).child("px"))
                .into_any_element()
        };
        // Miniature app-window mockup for the Appearance theme picker, matching
        // upstream ThemeModePicker (MOCKUP_COLORS + MOCKUP_LAYOUT) pixel for pixel.
        // Thumb area is 190x133 (aspect 10/7 inside border-2 + p-3px of a 200px card).
        struct MockColors { backdrop: u32, panel: u32, hbar: u32, hbar_soft: u32, card: u32, rowbar: u32, hairline: u32 }
        const MOCK_LIGHT: MockColors = MockColors { backdrop: 0xe9e9e9, panel: 0xf6f6f6, hbar: 0xcfcfcf, hbar_soft: 0xe0e0e0, card: 0xffffff, rowbar: 0xe3e3e3, hairline: 0xefefef };
        const MOCK_DARK: MockColors = MockColors { backdrop: 0x5f5f5f, panel: 0x2c2c2c, hbar: 0xa6a6a6, hbar_soft: 0x7d7d7d, card: 0x3a3a3a, rowbar: 0x707070, hairline: 0x4d4d4d };
        let mock_surface = |c: &MockColors| {
            div()
                .size_full()
                .bg(rgb(c.backdrop))
                .child(
                    div().absolute().left(px(15.)).right(px(15.)).top(px(19.)).bottom_0()
                        .bg(rgb(c.panel)).rounded_t(px(8.))
                        .child(div().absolute().left(px(52.)).top(px(15.)).w(px(64.)).h(px(4.)).rounded_full().bg(rgb(c.hbar)))
                        .child(div().absolute().left(px(38.)).top(px(23.)).w(px(92.)).h(px(3.)).rounded_full().bg(rgb(c.hbar_soft)))
                        .child(div().absolute().left(px(15.)).right(px(15.)).top(px(38.)).bottom_0()
                            .bg(rgb(c.card)).rounded_t(px(6.))
                            .child(rows_card(c))),
                )
        };
        fn rows_card(c: &MockColors) -> gpui::Div {
            let mut card = div().size_full();
            for i in 0..3 {
                let y = 9. + (i as f32) * 21.;
                card = card
                    .child(div().absolute().left(px(13.)).top(px(y)).w(px(37.)).h(px(4.)).rounded_full().bg(rgb(c.rowbar)))
                    .child(div().absolute().left(px(13.)).right(px(13.)).top(px(y + 11.)).h(px(1.)).bg(rgb(c.hairline)));
            }
            card
        }
        let theme_fg = theme.foreground;
        let theme_card = move |kind: u8, label: &'static str, selected: bool| {
            let thumb: gpui::AnyElement = match kind {
                0 => div()
                    .size_full()
                    .child(mock_surface(&MOCK_LIGHT))
                    .child(
                        div().absolute().right_0().top_0().bottom_0().w(px(95.)).overflow_hidden()
                            .child(div().absolute().right_0().top_0().bottom_0().w(px(190.))
                                .child(mock_surface(&MOCK_DARK))),
                    )
                    .into_any_element(),
                1 => div().size_full().child(mock_surface(&MOCK_LIGHT)).into_any_element(),
                _ => div().size_full().child(mock_surface(&MOCK_DARK)).into_any_element(),
            };
            let frame = div()
                .w_full()
                .p(px(3.))
                .rounded(px(14.))
                .border_2()
                .border_color(if selected { theme_fg } else { gpui::transparent_black() })
                .child(
                    div().w(px(190.)).h(px(133.)).rounded(px(8.)).overflow_hidden().relative()
                        .child(thumb),
                );
            v_flex()
                .flex_1()
                .min_w_0()
                .gap(px(6.))
                .items_center()
                .child(frame)
                .child(div().text_sm().when(selected, |d| d.font_weight(FontWeight::MEDIUM)).child(label))
                .into_any_element()
        };

        let settings_h = window.viewport_size().height - px(47.);
        let mut settings_pane = v_flex()
            .w(px(672.))
            .h(settings_h)
            .id("settings-scroll")
            .track_scroll(&self.settings_scroll)
            .overflow_y_scroll()
            .vertical_scrollbar(&self.settings_scroll)
            .px_6()
            .py_8()
            .gap_6();
        {
            let mut head = h_flex().w_full().items_start().justify_between().gap_4();
            if self.settings_section == "profile" {
                head = h_flex().items_start().w_full().justify_end().gap_4();
            } else {
            head = head.child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap(px(6.))
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_xl()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(section_title.to_string()),
                            )
                            .when(self.settings_section == "computer", |row| {
                                row.child(
                                    div()
                                        .px_1p5()
                                        .rounded(px(5.))
                                        .border_1()
                                        .border_color(theme.border)
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .child("Beta"),
                                )
                            }),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .whitespace_nowrap()
                            .overflow_hidden()
                            .child(match &*self.settings_section {
                                "general" => "Defaults for new chats, power, time, and getting started.".to_string(),
                                "appearance" => "Customize the theme, typography, density, and time format.".to_string(),
                                "notifications" => "Choose how Synara tells you when work finishes or needs attention.".to_string(),
                                "layout" => "Choose what the sidebar, the chat top bar, and the Environment panel show.".to_string(),
                                "chats" => "Control live responses, follow-ups, review defaults, and safety confirmations.".to_string(),
                                "shortcuts" => "Change, add, or remove the shortcut for every Synara command.".to_string(),
                                "profile" => "Your local activity, streaks, and a shareable stats card.".to_string(),
                                "accounts" => "Sign in to each agent once per account. Every account gets its own tab in the model picker, its own usage limits, and its own sign-in folder.".to_string(),
                                "providers" => "Choose visible coding agents and manage their installed CLI tools.".to_string(),
                                "models" => "Choose the source control writing style, Git writing model, and custom model slugs.".to_string(),
                                "skills" => "Review reusable workflows discovered across all configured providers.".to_string(),
                                "usage" => "See remaining quota and credits for every signed-in provider.".to_string(),
                                "connectors" => "Sign in to Supabase, Linear, Notion, and other services your agents can use.".to_string(),
                                "integrations" => "Give Codex, Claude, and other local agents scoped access to Synara tasks.".to_string(),
                                "devices" => "Control this computer from your other devices, or control other computers from here.".to_string(),
                                "computer" => "Let agents see and control this computer's desktop, and check backend status.".to_string(),
                                "appsnap" => "Capture another app's frontmost window directly into a task.".to_string(),
                                "worktrees" => "Review and clean up isolated workspaces created by Synara.".to_string(),
                                "archive" => "Find and restore threads you previously archived.".to_string(),
                                "privacy" => "What Synara shares, and what it can read on this computer.".to_string(),
                                "advanced" => "Recovery tools and this browser's session.".to_string(),
                                "about" => "Version, release channel, and what changed.".to_string(),
                                _ => format!("{section_title} settings are not yet ported to the native shell."),
                            }),
                    ),
            );
            }
            if self.settings_section == "profile" {
                head = head.child(h_flex().gap_2().child(outline_btn("Share")).child(outline_btn("Edit")));
            } else {
                let reset_label = if self.settings_section == "shortcuts" { "\u{21ba} Reset all to defaults" } else { "\u{21ba} Restore defaults" };
                head = head.child(outline_btn(reset_label));
            }
            settings_pane = settings_pane.child(div().mb_2().child(head));

            if self.settings_section == "general" {
                settings_pane = settings_pane
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(section_label("New chats"))
                            .child(card(
                        v_flex()
                            .child(srow(
                                "Agent and account",
                                "New chats start on this account until you pick another in the model picker. After that they reuse your most recent choice.",
                                select_pill("Codex".to_string(), Some(icons::CODEX)),
                            ))
                            .child(divider())
                            .child(srow(
                                "New threads",
                                "Pick the default workspace mode for newly created draft threads.",
                                select_pill("Local".to_string(), None),
                            ))
                            .into_any_element(),
                            )
                            .into_any_element(),
                        ),
                    )
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(section_label("Power"))
                            .child(card(
                        v_flex()
                            .child(srow(
                                "Keep computer awake",
                                "Uses macOS caffeinate. On: Keep this computer awake at all times. Agent: Keep this computer awake while an agent is working. Off: Let the system sleep normally.",
                                h_flex()
                                    .items_center()
                                    .rounded(px(8.))
                                    .border_1()
                                    .border_color(theme.border)
                                    .p(px(2.))
                                    .gap(px(2.))
                                    .child(div().h(px(24.)).px(px(10.)).flex().items_center().rounded(px(6.)).child(div().text_xs().child("On")))
                                    .child(div().h(px(24.)).px(px(10.)).flex().items_center().rounded(px(6.)).child(div().text_xs().child("Agent")))
                                    .child(div().h(px(24.)).px(px(10.)).flex().items_center().rounded(px(6.)).bg(theme.foreground).child(div().text_xs().text_color(theme.background).child("Off")))
                                    .into_any_element(),
                            ))
                            .into_any_element(),
                            )
                            .into_any_element(),
                        ),
                    )
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(section_label("Time"))
                            .child(card(
                        v_flex()
                            .child(srow(
                                "Time format",
                                "System default follows your browser or OS clock preference.",
                                select_pill("System default".to_string(), None),
                            ))
                            .into_any_element(),
                            )
                            .into_any_element(),
                        ),
                    )
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(section_label("Getting started"))
                            .child(card(
                        v_flex()
                            .child(srow(
                                "Welcome tour",
                                "Replay the first-run setup: agents, account, iPhone pairing, and first project.",
                                outline_btn("Open welcome tour"),
                            ))
                            .child(divider())
                            .child(srow(
                                "App tour",
                                "See where Activity, Tasks, Inbox, and Code review live.",
                                outline_btn("Replay tour"),
                            ))
                            .into_any_element(),
                            )
                            .into_any_element(),
                        ),
                    );
            }

            if self.settings_section == "notifications" {
                settings_pane = settings_pane.child(
                    v_flex()
                        .gap_1p5()
                        .child(section_label("Activity alerts"))
                        .child(card(
                            v_flex()
                                .child(srow(
                                    "Activity toasts",
                                    "Show an in-app toast when a chat or managed terminal agent finishes or needs input.",
                                    toggle(true),
                                ))
                                .child(divider())
                                .child(srow(
                                    "Desktop notifications",
                                    "Show an OS notification when a chat or managed terminal agent finishes or needs input while the app is in the background. Allow browser notifications to get alerts when chats or terminal agents finish or need input in the background.",
                                    h_flex()
                                        .items_center()
                                        .gap_3()
                                        .child(div().text_sm().child("Test"))
                                        .child(toggle(true))
                                        .into_any_element(),
                                ))
                                .child(divider())
                                .child(srow(
                                    "Wait for subagents",
                                    "Alert once the agent and all of its background subagents have finished. Turn this off to be alerted each time the agent or one of its subagents stops.",
                                    toggle(true),
                                ))
                                .into_any_element(),
                        )),
                );
            }

            if self.settings_section == "layout" {
                let mtog = |label: &'static str, on: bool| {
                    h_flex()
                        .w(px(170.))
                        .items_center()
                        .justify_between()
                        .child(div().text_sm().child(label))
                        .child(toggle(on))
                        .into_any_element()
                };
                let env_group = |label: &'static str,
                                 left: Vec<gpui::AnyElement>,
                                 right: Vec<gpui::AnyElement>| {
                    v_flex()
                        .gap_2()
                        .child(div().text_xs().text_color(theme.muted_foreground).child(label))
                        .child(
                            h_flex()
                                .gap_8()
                                .child(v_flex().gap_2().children(left))
                                .child(v_flex().gap_2().children(right)),
                        )
                        .into_any_element()
                };
                settings_pane = settings_pane
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(section_label("Sidebar organization"))
                            .child(card(
                                v_flex()
                                    .child(srow(
                                        "Project order",
                                        "Controls how projects are arranged in the main sidebar.",
                                        select_pill("Manual order".to_string(), None),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Thread order",
                                        "Controls how threads are arranged inside each project in the main sidebar.",
                                        select_pill("Recently active".to_string(), None),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Working and unread on top",
                                        "With Recently active order, move working and unread threads above the rest. The list holds still while your pointer is over it.",
                                        toggle(true).into_any_element(),
                                    ))
                                    .into_any_element(),
                            )),
                    )
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(section_label("Sidebar sections"))
                            .child(card(
                                v_flex()
                                    .child(srow(
                                        "Chats",
                                        "Show the standalone Chats list in the sidebar footer (chats not tied to a project).",
                                        toggle(true).into_any_element(),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Hubs",
                                        "Show the Hubs tab in the sidebar switcher.",
                                        toggle(true).into_any_element(),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Automation runs",
                                        "Show the thread each standalone automation run creates. Runs stay listed on the automation's page either way; threads owned by dedicated or heartbeat automations always stay visible.",
                                        toggle(true).into_any_element(),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Child threads",
                                        "Show threads an agent created from another thread nested under that thread.",
                                        toggle(false).into_any_element(),
                                    ))
                                    .into_any_element(),
                            )),
                    )
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(section_label("Environment panel"))
                            .child(card(
                                v_flex()
                                    .child(srow(
                                        "Open by default",
                                        "Open the chat Environment panel automatically on normal threads. When off, the panel stays closed until you open it. Your last open/close also updates this preference.",
                                        toggle(false).into_any_element(),
                                    ))
                                    .child(divider())
                                    .child(
                                        div().px_3().py(px(10.)).child(env_group(
                                            "Code and status",
                                            vec![
                                                mtog("Subagents", true),
                                                mtog("Repository", true),
                                                mtog("Editor", true),
                                            ],
                                            vec![
                                                mtog("Usage", true),
                                                mtog("Pull request", true),
                                                mtog("Actions", true),
                                            ],
                                        )),
                                    )
                                    .child(divider())
                                    .child(
                                        div().px_3().py(px(10.)).child(env_group(
                                            "Context and notes",
                                            vec![
                                                mtog("Recap", true),
                                                mtog("Project instructions", false),
                                            ],
                                            vec![
                                                mtog("Pinned messages", true),
                                                mtog("Notepad", false),
                                            ],
                                        )),
                                    )
                                    .into_any_element(),
                            )),
                    )
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(section_label("Chat top bar"))
                            .child(card(
                                v_flex()
                                    .child(srow(
                                        "Thread tabs",
                                        "Show open threads as tabs in the chat header. Turn off to show only the current thread's title.",
                                        toggle(true).into_any_element(),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Hand off",
                                        "Show the Hand off button in the chat top bar. Handing off stays available from a thread's sidebar menu and the composer's model picker.",
                                        toggle(true).into_any_element(),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Project actions",
                                        "Show the selected project's actions in the chat top bar. When off, actions still run from their keybindings and the Environment panel's Actions section.",
                                        toggle(true).into_any_element(),
                                    ))
                                    .into_any_element(),
                            )),
                    )
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(section_label("Opens on its own"))
                            .child(card(
                                v_flex()
                                    .child(srow(
                                        "Automatically open simulator",
                                        "Open the iOS Simulator pane when an agent uses a device. Turn this off to use Simulator.app without the mirrored pane reopening. You can still open the pane manually.",
                                        toggle(true).into_any_element(),
                                    ))
                                    .into_any_element(),
                            )),
                    );
            }

            if self.settings_section == "chats" {
                settings_pane = settings_pane
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(section_label("Conversation"))
                            .child(card(
                                v_flex()
                                    .child(srow(
                                        "Follow-up behavior",
                                        "Choose whether messages sent during an active turn wait in the queue or steer the current run. Ctrl/Cmd+Enter uses the opposite behavior for one message.",
                                        seg(&["Queue", "Steer"], 0),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Side chat expiry",
                                        "Expire a side chat after it sits idle, unviewed and not running, for this long. Expired side chats are read-only and unload their provider session.",
                                        seg(&["1 hour", "24 hours", "Never"], 0),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Enter while dictating",
                                        "Choose what Enter does while a voice note is recording: stop and transcribe into the composer, or stop and send the message once it is transcribed.",
                                        seg(&["Stop", "Stop and send"], 0),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Assistant output",
                                        "Show token-by-token output while a response is in progress.",
                                        toggle(true).into_any_element(),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Fold finished turns",
                                        "Hide a finished turn's tool calls and intermediate messages behind a single \"Worked for\u{2026}\" line. A turn stays open while it runs or while its background subagents are still working. Turn this off to keep every step visible.",
                                        toggle(true).into_any_element(),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Move sent messages to top",
                                        "Move each sent message to the top of the conversation. Turn off to keep it at the bottom and follow replies as they stream.",
                                        toggle(true).into_any_element(),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Effort slider",
                                        "Show effort as a slider at the bottom of the composer's model picker, with fast mode and reset alongside it, instead of separate Effort and Speed rows.",
                                        toggle(true).into_any_element(),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Show suggestions",
                                        "After a reply finishes, show a predicted next message in the empty composer. Press Tab to insert it as a draft you can edit before sending.",
                                        toggle(true).into_any_element(),
                                    ))
                                    .into_any_element(),
                            )),
                    )
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(section_label("Review"))
                            .child(card(
                                v_flex()
                                    .child(srow(
                                        "Diff layout",
                                        "Default stacked or split layout for threads without a saved choice. Each thread remembers its review-panel changes independently.",
                                        seg(&["Stacked", "Split"], 1),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Open pull requests and issues",
                                        "Choose where a pull request or issue link in a chat opens: the built-in review view, the in-app browser, or your external browser. Ctrl/Cmd+click always opens the external browser.",
                                        select_pill("In Synara".to_string(), None),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Pull request diff colors",
                                        "Show additions in green and deletions in red in pull request summaries.",
                                        toggle(true).into_any_element(),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Include fork upstreams",
                                        "Also list pull requests and issues from each project's other GitHub remotes, such as the repository a fork was made from. Off reads only the project's own repository.",
                                        toggle(false).into_any_element(),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Diff line wrapping",
                                        "Set the default wrap state when the diff panel opens. The in-panel wrap toggle only affects the current diff session.",
                                        toggle(false).into_any_element(),
                                    ))
                                    .into_any_element(),
                            )),
                    )
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(section_label("Safety confirmations"))
                            .child(card(
                                v_flex()
                                    .child(srow(
                                        "Delete confirmation",
                                        "Ask before deleting a thread and its chat history.",
                                        toggle(true).into_any_element(),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Archive confirmation",
                                        "Ask before archiving a thread.",
                                        toggle(false).into_any_element(),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Terminal close confirmation",
                                        "Ask before closing a terminal tab and clearing its history.",
                                        toggle(true).into_any_element(),
                                    ))
                                    .into_any_element(),
                            )),
                    );
            }


            if self.settings_section == "shortcuts" {
                let shortcut_rows: [(&'static str, &'static str, &'static str); 69] = [
    ("Toggle sidebar", "Collapse or reveal the sidebar shell.", "\u{2318}B"),
    ("Add project", "Open the Create project dialog to import a local folder.", "\u{21e7}\u{2318}O"),
    ("Search projects and threads", "Open the sidebar search palette from anywhere in the app.", "\u{2318}K"),
    ("Toggle Activity", "Show or hide running tasks, completed work, and items that need attention.", "\u{2325}\u{2318}U"),
    ("Import thread", "Bring an existing conversation into the current workspace.", "\u{2318}I"),
    ("Previous space", "Switch to the previous project space and restore its last working context.", "\u{2325}\u{2318}Left"),
    ("Next space", "Switch to the next project space and restore its last working context.", "\u{2325}\u{2318}Right"),
    ("New thread", "Start a fresh thread in the current project, or the most recent one.", "\u{2318}N"),
    ("New thread in latest project", "Jump back into the most recently used project with a new thread.", "\u{21e7}\u{2318}N"),
    ("New chat", "Open the empty chat landing view.", "\u{2325}\u{2318}N"),
    ("New terminal thread", "Create a thread that opens directly into terminal mode.", "\u{21e7}\u{2318}T"),
    ("New Claude thread", "Start a fresh thread with Claude selected.", "\u{2325}\u{2318}C"),
    ("New Codex thread", "Start a fresh thread with Codex selected.", "\u{2325}\u{2318}X"),
    ("New Cursor thread", "Start a fresh thread with Cursor selected.", "\u{2325}\u{2318}R"),
    ("Split chat", "Open the current conversation in a second pane.", "\u{2318}\\"),
    ("Previous recent view", "Cycle backward through recently opened primary views.", "\u{2303}\u{21e7}Tab"),
    ("Next recent view", "Cycle forward through recently opened primary views.", "\u{2303}Tab"),
    ("Model picker", "Open the composer provider and model picker.", "\u{21e7}\u{2318}M"),
    ("Next model", "Cycle to the next model for the active provider (favorites first, then remaining models).", "\u{2325}]"),
    ("Previous model", "Cycle to the previous model for the active provider (favorites first, then remaining models).", "\u{2325}["),
    ("Next model effort", "Cycle the model's available effort levels and briefly show the selected level.", "\u{21e7}Tab"),
    ("Reasoning picker", "Open the composer reasoning and trait controls.", "\u{21e7}\u{2318}E"),
    ("Open usage settings", "Open Settings \u{2192} Usage for provider quota and token totals.", "\u{21e7}\u{2318}U"),
    ("Focus composer", "Focus or blur the chat prompt composer.", "\u{2318}L"),
    ("Find in thread", "Search the current transcript and jump to each matching message.", "\u{2318}F"),
    ("Search files", "Open the workspace file-name search palette.", "\u{2318}P"),
    ("Search snippets", "Open the workspace content search palette across file snippets.", "\u{21e7}\u{2318}F"),
    ("Toggle terminal", "Show or hide the terminal surface for the active thread.", "\u{2318}J"),
    ("Focus terminal", "Open or focus the terminal panel for this thread.", "\u{2318}T"),
    ("Close terminal", "Close the terminal session for this thread.", "\u{2318}W"),
    ("Toggle diff", "Open or close the working tree diff panel.", "\u{2318}D"),
    ("Next change", "Jump the diff viewport to the next changed file.", "\u{2325}Down"),
    ("Previous change", "Jump the diff viewport to the previous changed file.", "\u{2325}Up"),
    ("Toggle side chat", "Open or hide a side chat beside the main conversation.", "\u{2325}\u{2318}S"),
    ("Toggle browser", "Reveal the built-in browser panel for the active thread.", "\u{21e7}\u{2318}B"),
    ("Toggle iOS Simulator", "Reveal the iOS Simulator panel for the active thread. macOS servers only.", ""),
    ("Copy thread ID", "Copy the active thread's ID to the clipboard.", "\u{21e7}\u{2318}C"),
    ("Archive thread", "Archive the active chat, with the same confirmation and undo as the thread menu.", "\u{2325}\u{21e7}\u{2318}A"),
    ("Snooze thread", "Open the snooze date and time picker for the active chat.", "\u{2325}\u{21e7}\u{2318}S"),
    ("Mark thread unread", "Mark the active chat unread and restore its dismissed notification.", "\u{2325}\u{21e7}\u{2318}U"),
    ("Previous visible thread", "Cycle to the previous thread that is currently visible in the sidebar.", "\u{21e7}\u{2318}["),
    ("Next visible thread", "Cycle to the next thread that is currently visible in the sidebar.", "\u{21e7}\u{2318}]"),
    ("Next chat needing attention", "Open the most urgent other chat: one waiting on you first, then unseen work, as Activity's Priority ranks them.", "\u{2325}\u{2318}A"),
    ("Previous tab", "Switch to the open thread tab on the left of the active one.", "\u{2303}\u{2318}Left"),
    ("Next tab", "Switch to the open thread tab on the right of the active one.", "\u{2303}\u{2318}Right"),
    ("Close tab", "Close the active open-thread tab, like a browser tab.", "\u{2318}W"),
    ("Open in favorite editor", "Send the current thread or workspace target to your preferred editor.", "\u{2318}O"),
    ("Save file", "Write the focused editor's unsaved changes back to disk.", "\u{2318}S"),
    ("Commit and push", "Commit pending changes and push the active thread's repo.", "\u{2303}\u{2318}P"),
    ("Editor commands", "Open the editor command picker for the focused code editor.", "F1"),
    ("Rename symbol", "Rename the symbol under the cursor across the workspace.", "F2"),
    ("Go to definition", "Jump to the definition of the symbol under the cursor.", "F12"),
    ("Find references", "List references to the symbol under the cursor.", "\u{21e7}F12"),
    ("Go to symbol in file", "Jump to a symbol in the open file.", ""),
    ("Go to line", "Jump to a line, or line:column, in the open file.", ""),
    ("Show problems", "List diagnostics for the open file.", ""),
    ("Quick fix", "Show quick fixes and refactorings for the code at the cursor.", "\u{2318}."),
    ("Start or continue debugging", "Launch the debugger for the open file, or continue a paused session.", "F5"),
    ("Stop debugging", "Stop the current debug session.", "\u{21e7}F5"),
    ("Toggle breakpoint", "Add or remove a breakpoint on the cursor line.", "F9"),
    ("Step over", "Run the next statement without entering calls.", "F10"),
    ("Step into", "Enter the next call.", "F11"),
    ("Step out", "Run until the current call returns.", "\u{21e7}F11"),
    ("Open full-width terminal workspace", "Expand the active thread into the workspace terminal layout.", "\u{21e7}\u{2318}J"),
    ("Focus terminal tab", "Switch the workspace to the terminal tab.", "\u{2303}1"),
    ("Focus chat tab", "Switch the workspace back to the chat tab.", "\u{2303}2"),
    ("Close active workspace panel", "Close the currently focused workspace panel or tab.", "\u{2318}W"),
    ("Jump to visible thread 1\u{2013}9", "Focus a visible thread directly from the sidebar number row.", "\u{2318}1\u{2013}9"),
    ("Jump to space 1\u{2013}9", "Switch straight to a tab of the space switcher. 1 is always Void.", "\u{2325}\u{2318}1\u{2013}9"),
];
                let search_field = h_flex()
                    .w_full()
                    .h(px(32.))
                    .items_center()
                    .px_3()
                    .rounded(px(8.))
                    .border_1()
                    .border_color(theme.border)
                    .justify_between()
                    .child(div().text_sm().text_color(theme.muted_foreground).child("Search shortcuts"))
                    .child(div().text_xs().text_color(theme.muted_foreground).child("\u{2318}K"))
                    .into_any_element();
                let mut list = v_flex();
                let mut first = true;
                for (title, desc, kbd) in shortcut_rows {
                    if !first {
                        list = list.child(divider());
                    }
                    first = false;
                    let mut right = h_flex().items_center().gap_2();
                    if !kbd.is_empty() {
                        right = right.child(div().text_sm().text_color(theme.muted_foreground).child(kbd));
                    }
                    right = right.child(
                        h_flex()
                            .items_center()
                            .gap_1()
                            .child(svg_icon(icons::PENCIL).size(px(14.)).text_color(theme.muted_foreground))
                            .child(svg_icon(icons::TRASH).size(px(14.)).text_color(theme.muted_foreground)),
                    );
                    list = list.child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .px_3()
                            .py(px(8.))
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .gap(px(1.))
                                    .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(title))
                                    .child(div().w_full().text_sm().text_color(theme.muted_foreground).child(desc)),
                            )
                            .child(right),
                    );
                }
                settings_pane = settings_pane
                    .child(search_field)
                    .child(card(list.into_any_element()));
            }


            // ==================== remaining sections ====================
            let sdata = &self.settings_data;
            let cfg_providers: Vec<Value> = arr(get(sdata, "config"), "providers").to_vec();
            let usage_rows: Vec<Value> = arr(get(sdata, "usage"), "snapshots")
                .to_vec()
                .into_iter()
                .chain(arr(get(sdata, "usage"), "rows").to_vec())
                .chain(arr(get(sdata, "usage"), "providers").to_vec())
                .chain(if get(sdata, "usage").is_array() {
                    arr(get(sdata, "usage"), "").to_vec()
                } else {
                    Vec::new()
                })
                .collect();
            let dash = |v: &Value| div().text_sm().text_color(theme.muted_foreground).child("\u{2014}").into_any_element();
            fn provider_display(id: &str) -> &str {
                match id {
                    "codex" => "Codex", "claudeAgent" | "claude" => "Claude", "cursor" => "Cursor",
                    "antigravity" => "Antigravity", "grok" => "Grok", "droid" => "Droid",
                    "devin" => "Devin", "opencode" => "OpenCode", "pi" => "Pi", "omp" => "Oh My Pi",
                    other => other,
                }
            }

            // ---------- Profile ----------
            if self.settings_section == "profile" {
                let prof = get(sdata, "profile");
                let num = |k: &str| get(prof, k).as_i64().map(|n| n.to_string());
                let threads_total = arr(get(sdata, "snapshot"), "threads").len()
                    .max(arr(get(sdata, "snapshot"), "items").len());
                let stat = |val: String, label: &'static str| {
                    v_flex()
                        .flex_1()
                        .items_center()
                        .gap(px(2.))
                        .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(val))
                        .child(div().text_xs().text_color(theme.muted_foreground).child(label))
                        .into_any_element()
                };
                let ins = |label: &'static str, val: String| {
                    h_flex()
                        .w_full()
                        .items_center()
                        .justify_between()
                        .py(px(4.))
                        .child(div().text_sm().text_color(theme.muted_foreground).child(label))
                        .child(div().text_sm().child(val))
                        .into_any_element()
                };
                let heat_col = || {
                    v_flex()
                        .gap(px(3.))
                        .children((0..7).map(|_| {
                            div().w(px(9.)).h(px(9.)).rounded(px(2.)).bg(rgb(0xededed)).into_any_element()
                        }))
                        .into_any_element()
                };
                let months = ["Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov"];
                settings_pane = settings_pane
                    .child(
                        v_flex()
                            .w_full()
                            .items_center()
                            .gap(px(6.))
                            .py_4()
                            .child(
                                div()
                                    .w(px(44.))
                                    .h(px(44.))
                                    .rounded_full()
                                    .bg(rgb(0x34a853))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).text_color(gpui::white()).child("DE")),
                            )
                            .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child("Devin"))
                            .child(div().text_xs().text_color(theme.muted_foreground).child("@devin \u{b7} Synara")),
                    )
                    .child(
                        h_flex()
                            .w_full()
                            .py_2()
                            .child(stat(num("lifetimeTokens").unwrap_or_else(|| "\u{2014}".into()), "Lifetime tokens"))
                            .child(stat(num("peakDay").unwrap_or_else(|| "\u{2014}".into()), "Peak day"))
                            .child(stat(num("totalPrompts").unwrap_or_else(|| "0".into()), "Total prompts"))
                            .child(stat(num("currentStreak").map(|s| format!("{s} days")).unwrap_or_else(|| "0 days".into()), "Current streak"))
                            .child(stat(num("longestStreak").map(|s| format!("{s} days")).unwrap_or_else(|| "0 days".into()), "Longest streak")),
                    )
                    .child(section_label("Activity"))
                    .child(
                        v_flex()
                            .w_full()
                            .gap_2()
                            .child(h_flex().gap(px(3.)).children((0..44).map(|_| heat_col())))
                            .child(
                                h_flex()
                                    .w_full()
                                    .justify_between()
                                    .children(months.map(|m| {
                                        div().text_xs().text_color(theme.muted_foreground).child(m).into_any_element()
                                    })),
                            ),
                    )
                    .child(
                        h_flex()
                            .w_full()
                            .items_start()
                            .gap_6()
                            .child(
                                v_flex()
                                    .flex_1()
                                    .gap_1()
                                    .child(section_label("Activity insights"))
                                    .child(ins("Most used provider", "\u{2014}".into()))
                                    .child(ins("Most used reasoning", "\u{2014}".into()))
                                    .child(ins("Most active hour", "\u{2014}".into()))
                                    .child(ins("Most worked project", "\u{2014}".into()))
                                    .child(ins("Skills explored", "0".into()))
                                    .child(ins("Total skills used", "0".into()))
                                    .child(ins("Total threads", format!("{threads_total}"))),
                            )
                            .child(
                                v_flex()
                                    .flex_1()
                                    .gap_1()
                                    .child(section_label("Most used plugins"))
                                    .child(div().py(px(4.)).text_sm().text_color(theme.muted_foreground).child("No skills or agents used yet.")),
                            ),
                    )
                    .child(section_label("Model usage"))
                    .child(div().text_sm().text_color(theme.muted_foreground).child("Share of turns."))
                    .child(div().pt_1().text_sm().text_color(theme.muted_foreground).child("No model activity yet."));
            }

            // ---------- Accounts ----------
            if self.settings_section == "accounts" {
                let acct_icon = |pid: &str| match pid {
                    "devin" => icons::P_DEVIN, "pi" => icons::P_PI, "codex" => icons::CODEX,
                    "claudeAgent" | "claude" => icons::P_CLAUDE, "cursor" => icons::P_CURSOR,
                    "grok" => icons::P_GROK, "droid" => icons::P_DROID, "opencode" => icons::P_OPENCODE,
                    "antigravity" => icons::P_ANTIGRAVITY, "omp" => icons::P_OMP,
                    _ => icons::S_PUZZLE,
                };
                let shown: Vec<(&'static str, &'static str, &'static str, &'static str, &'static str)> = if cfg_providers.is_empty() {
                    vec![
                        ("devin", "Devin", "Authenticated \u{b7} Devin API Key", "Pro", "Daily 100% left \u{b7} Weekly 93% left"),
                        ("pi", "Pi", "Available", "", ""),
                    ]
                } else {
                    cfg_providers.iter().filter_map(|p| {
                        let pid = str_at(p, "provider");
                        let pid = if pid.is_empty() { str_at(p, "driver") } else { pid };
                        match &*pid {
                            "devin" => Some(("devin","Devin","Authenticated \u{b7} Devin API Key","Pro","Daily 100% left \u{b7} Weekly 93% left")),
                            "pi" => Some(("pi","Pi","Available","","")),
                            _ => None,
                        }
                    }).collect()
                };
                for (pid, name, status, plan, quota) in shown {
                    let _ = pid;
                    settings_pane = settings_pane
                        .child(
                            h_flex()
                                .w_full()
                                .items_center()
                                .justify_between()
                                .child(section_label(name))
                                .child(
                                    h_flex()
                                        .gap_1()
                                        .child(div().px_2().py_1().text_xs().text_color(theme.muted_foreground).child("\u{2b06} Import directory"))
                                        .child(div().px_2().py_1().text_xs().text_color(theme.muted_foreground).child("+ Add account")),
                                ),
                        )
                        .child(card(
                            h_flex()
                                .w_full()
                                .items_center()
                                .gap_3()
                                .px_3()
                                .py(px(10.))
                                .child(svg_icon(acct_icon(pid)).size(px(18.)).text_color(theme.foreground))
                                .child(
                                    v_flex()
                                        .flex_1()
                                        .min_w_0()
                                        .gap(px(1.))
                                        .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(name))
                                        .child(
                                            h_flex()
                                                .items_center()
                                                .gap_1p5()
                                                .child(div().w(px(6.)).h(px(6.)).rounded_full().bg(rgb(0x22a865)))
                                                .child(div().text_xs().text_color(theme.muted_foreground).child(status)),
                                        )
                                        .children(if plan.is_empty() { vec![] } else {
                                            vec![div().text_xs().text_color(theme.muted_foreground).child(plan)]
                                        })
                                        .children(if quota.is_empty() { vec![] } else {
                                            vec![div().text_xs().text_color(theme.muted_foreground).child(quota)]
                                        }),
                                )
                                .child(toggle(true))
                                .child(svg_icon(icons::CHEVRON_RIGHT).size(px(14.)).text_color(theme.muted_foreground))
                                .into_any_element(),
                        ));
                }
                settings_pane = settings_pane.child(
                    v_flex()
                        .gap_2()
                        .px_2()
                        .py_2()
                        .child(div().w_full().text_xs().text_color(theme.muted_foreground).child(
                            "Not installed: Codex, Claude, Cursor, Antigravity, Grok, Droid, OpenCode, Oh My Pi. Install them under Providers to add their accounts."))
                        .child(div().w_full().text_xs().text_color(theme.muted_foreground).child(
                            "A chat stays on the account it started with. Pick the account for a new chat in the model picker. CLI versions, updates, and binary paths live in Providers.")),
                );
            }

            // ---------- Providers ----------
            if self.settings_section == "providers" {
                let p_icon = |pid: &str| match pid {
                    "codex" => icons::CODEX, "claudeAgent" | "claude" => icons::P_CLAUDE,
                    "cursor" => icons::P_CURSOR, "antigravity" => icons::P_ANTIGRAVITY,
                    "grok" => icons::P_GROK, "droid" => icons::P_DROID, "devin" => icons::P_DEVIN,
                    "opencode" => icons::P_OPENCODE, "pi" => icons::P_PI, "omp" => icons::P_OMP,
                    _ => icons::S_PUZZLE,
                };
                let mut rows: Vec<(String, String, String, bool)> = Vec::new();
                if cfg_providers.is_empty() {
                    for (pid, name, status, upd) in [
                        ("codex","Codex","Unavailable",false),
                        ("claudeAgent","Claude","Unavailable",false),
                        ("cursor","Cursor","Unavailable",false),
                        ("antigravity","Antigravity","Unavailable",false),
                        ("grok","Grok","Unavailable",false),
                        ("droid","Droid","Unavailable",false),
                        ("opencode","OpenCode","Unavailable",false),
                        ("pi","Pi","Installed \u{b7} sign-in not verified \u{b7} v1.4.0 -> v1.6.0",true),
                        ("devin","Devin","Connected \u{b7} Current v3000.14.30821",false),
                        ("omp","Oh My Pi","Unavailable",false),
                    ] {
                        rows.push((pid.to_string(), name.to_string(), status.to_string(), upd));
                    }
                } else {
                    for p in &cfg_providers {
                        let pid = str_at(p, "provider");
                        let pid = if pid.is_empty() { str_at(p, "driver") } else { pid };
                        if pid.is_empty() { continue; }
                        let avail = get(p, "available").as_bool().unwrap_or(false);
                        let ver = str_at(p, "version");
                        let status = if avail {
                            if ver.is_empty() { "Installed".to_string() } else { format!("Installed \u{b7} {ver}") }
                        } else {
                            "Unavailable".to_string()
                        };
                        let upd = get(p, "updateAvailable").as_bool().unwrap_or(false);
                        rows.push((pid.to_string(), provider_display(pid).to_string(), status, upd));
                    }
                }
                settings_pane = settings_pane
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .child(section_label("Agents"))
                            .child(div().px_2().py_1().text_xs().text_color(theme.muted_foreground).child("Refresh status")),
                    )
                    .child(card(
                        v_flex()
                            .children(rows.iter().enumerate().flat_map(|(i, (pid, name, status, upd))| {
                                let row = h_flex()
                                    .w_full()
                                    .items_center()
                                    .gap_2p5()
                                    .px_3()
                                    .py(px(10.))
                                    .child(div().w(px(16.)).text_sm().text_color(theme.muted_foreground).child("\u{283f}"))
                                    .child(svg_icon(p_icon(pid)).size(px(16.)).text_color(theme.foreground))
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap(px(1.))
                                            .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(name.clone()))
                                            .child(div().text_xs().text_color(theme.muted_foreground).child(status.clone())),
                                    )
                                    .children(if *upd {
                                        vec![div()
                                            .px_2()
                                            .py_1()
                                            .rounded(px(6.))
                                            .border_1()
                                            .border_color(theme.border)
                                            .text_xs()
                                            .child("\u{2b07} Update")
                                            .into_any_element()]
                                    } else { vec![] })
                                    .child(toggle(true))
                                    .child(svg_icon(icons::CHEVRON_RIGHT).size(px(14.)).text_color(theme.muted_foreground))
                                    .into_any_element();
                                if i > 0 {
                                    vec![divider().into_any_element(), row]
                                } else {
                                    vec![row]
                                }
                            }))
                            .into_any_element(),
                    ))
                    .child(section_label("Performance"))
                    .child(card(
                        v_flex()
                            .child(
                                v_flex()
                                    .px_3()
                                    .py(px(10.))
                                    .gap(px(2.))
                                    .child(
                                        h_flex()
                                            .w_full()
                                            .items_center()
                                            .justify_between()
                                            .child(div().text_sm().font_weight(FontWeight::MEDIUM).child("Keep Synara responsive"))
                                            .child(toggle(true)),
                                    )
                                    .child(div().w_full().text_sm().text_color(theme.muted_foreground).child(
                                        "Give agent processes and their commands a moderately lower CPU priority when the machine is busy."))
                                    .child(div().w_full().text_xs().text_color(theme.muted_foreground).child(
                                        "Applies to newly launched agent processes. Restart existing sessions to apply consistently.")),
                            )
                            .into_any_element(),
                    ))
                    .child(section_label("Updates"))
                    .child(card(
                        v_flex()
                            .child(srow("Automatic CLI update checks",
                                "Check Codex, Claude, and other provider CLIs for newer versions in the background.",
                                toggle(true)))
                            .child(divider())
                            .child(
                                v_flex()
                                    .px_3()
                                    .py(px(10.))
                                    .gap(px(2.))
                                    .child(div().text_sm().font_weight(FontWeight::MEDIUM).child("Provider updates"))
                                    .child(div().w_full().text_sm().text_color(theme.muted_foreground).child(
                                        "Review installed provider tools that Synara can safely update."))
                                    .child(div().w_full().text_xs().text_color(theme.muted_foreground).child(
                                        "1 update available")),
                            )
                            .into_any_element(),
                    ));
            }

            // ---------- Models ----------
            if self.settings_section == "models" {
                settings_pane = settings_pane
                    .child(section_label("Generation defaults"))
                    .child(card(
                        v_flex()
                            .child(srow("Source control writing style", "In each project, matches recent change descriptions and change request titles.", select_pill("Repository conventions".into(), None)))
                            .child(divider())
                            .child(srow("Add Synara attribution", "Adds a \"Created with Synara\" link to the end of pull request descriptions that Synara creates. Commits and branch names are never changed.", toggle(true)))
                            .child(divider())
                            .child(srow("Git writing model", "Used for generated commit messages, PR titles, and branch names.", select_pill("GPT-6 Luna".into(), None)))
                            .into_any_element(),
                    ))
                    .child(section_label("Custom models"))
                    .child(card(
                        v_flex()
                            .px_3()
                            .py(px(10.))
                            .gap_2()
                            .child(div().text_sm().font_weight(FontWeight::MEDIUM).child("Saved model slugs"))
                            .child(div().w_full().text_sm().text_color(theme.muted_foreground).child("Add custom model slugs for supported providers."))
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .gap_2()
                                    .child(select_pill("Codex".into(), None))
                                    .child(
                                        div()
                                            .flex_1()
                                            .h(px(30.))
                                            .px_2()
                                            .rounded(px(6.))
                                            .border_1()
                                            .border_color(theme.border)
                                            .flex()
                                            .items_center()
                                            .child(div().text_xs().text_color(theme.muted_foreground).child("gpt-6.7-codex-ultra-preview")),
                                    )
                                    .child(
                                        div()
                                            .px_3()
                                            .py_1p5()
                                            .rounded_full()
                                            .bg(rgb(0x26251f))
                                            .text_xs()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(gpui::white())
                                            .child("+ Add"),
                                    ),
                            )
                            .into_any_element(),
                    ));
            }

            // ---------- Skills ----------
            if self.settings_section == "skills" {
                let skills = arr(get(sdata, "skills"), "skills").to_vec();
                let skills = if skills.is_empty() { arr(get(sdata, "skills"), "items").to_vec() } else { skills };
                settings_pane = settings_pane
                    .child(section_label("Portable skills"))
                    .child(card(
                        v_flex()
                            .px_3()
                            .py(px(10.))
                            .gap(px(2.))
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .child(div().text_sm().font_weight(FontWeight::MEDIUM).child("Synara skills folder"))
                                    .child(div().text_xs().text_color(theme.muted_foreground).child(format!("{} of {} skills enabled", skills.len(), skills.len()))),
                            )
                            .child(div().w_full().text_sm().text_color(theme.muted_foreground).child(
                                "Skills placed here are available on every provider. When a provider already ships its own copy of a skill, that copy is used; otherwise Synara's copy is the fallback."))
                            .child(div().w_full().text_xs().text_color(theme.muted_foreground).child("~/.synara/skills"))
                            .into_any_element(),
                    ))
                    .child(section_label("Skills"));
                if skills.is_empty() {
                    settings_pane = settings_pane.child(card(
                        v_flex()
                            .px_3()
                            .py(px(10.))
                            .gap(px(2.))
                            .child(div().text_sm().font_weight(FontWeight::MEDIUM).child("No skills found"))
                            .child(div().w_full().text_sm().text_color(theme.muted_foreground).child(
                                "Add a skill folder containing a SKILL.md to the Synara skills folder above, or install skills for any supported provider."))
                            .into_any_element(),
                    ));
                }
                for s in &skills {
                    let nm = str_at(s, "name").to_string();
                    let desc = str_at(s, "description").to_string();
                    let pv = str_at(s, "provider").to_string();
                    let desc = if desc.is_empty() { provider_display(&pv).to_string() } else { desc };
                    settings_pane = settings_pane.child(card(
                        v_flex()
                            .px_3()
                            .py(px(8.))
                            .gap(px(1.))
                            .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(nm))
                            .child(div().text_xs().text_color(theme.muted_foreground).child(desc))
                            .into_any_element(),
                    ));
                }
            }

            // ---------- Usage ----------
            if self.settings_section == "usage" {
                let p_icon2 = |pid: &str| match pid {
                    "codex" => icons::CODEX, "claudeAgent" | "claude" => icons::P_CLAUDE,
                    "cursor" => icons::P_CURSOR, "antigravity" => icons::P_ANTIGRAVITY,
                    "grok" => icons::P_GROK, "droid" => icons::P_DROID, "devin" => icons::P_DEVIN,
                    "opencode" => icons::P_OPENCODE, "pi" => icons::P_PI, "omp" => icons::P_OMP,
                    _ => icons::S_PUZZLE,
                };
                let sidebar_rows: [(&'static str, &'static str, bool); 9] = [
                    ("codex","Codex",true), ("claudeAgent","Claude",true), ("cursor","Cursor",false),
                    ("antigravity","Antigravity",false), ("grok","Grok",false), ("droid","Droid",false),
                    ("opencode","OpenCode",false), ("pi","Pi",false), ("devin","Devin",false),
                ];
                settings_pane = settings_pane
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                div()
                                    .px_2()
                                    .py_1()
                                    .rounded(px(6.))
                                    .bg(rgb(0xeeeeee))
                                    .text_xs()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child("Overview"),
                            )
                            .child(div().px_2().py_1().text_xs().text_color(theme.muted_foreground).child("Analytics")),
                    )
                    .child(section_label("Sidebar \u{b7} up to 2 accounts"))
                    .child(card(
                        v_flex()
                            .children(sidebar_rows.iter().enumerate().flat_map(|(i, (pid, name, on))| {
                                let row = h_flex()
                                    .w_full()
                                    .items_center()
                                    .gap_2p5()
                                    .px_3()
                                    .py(px(9.))
                                    .child(svg_icon(p_icon2(pid)).size(px(16.)).text_color(theme.foreground))
                                    .child(div().flex_1().text_sm().font_weight(FontWeight::MEDIUM).child(name.to_string()))
                                    .child(toggle(*on))
                                    .into_any_element();
                                if i > 0 { vec![divider().into_any_element(), row] } else { vec![row] }
                            }))
                            .into_any_element(),
                    ))
                    .child(section_label("Ring"))
                    .child(card(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .px_3()
                            .py(px(10.))
                            .child(div().flex_1().min_w_0().w_full().text_sm().text_color(theme.muted_foreground).child(
                                "Show both limits as two rings, or a single ring for one of them."))
                            .child(
                                h_flex()
                                    .gap_3()
                                    .child(div().text_sm().font_weight(FontWeight::MEDIUM).child("Both"))
                                    .child(div().text_sm().text_color(theme.muted_foreground).child("5h"))
                                    .child(div().text_sm().text_color(theme.muted_foreground).child("Weekly")),
                            )
                            .into_any_element(),
                    ))
                    .child(section_label("Usage popovers"))
                    .child(card(
                        v_flex()
                            .child(srow("Show details by default",
                                "Open the details below the limits. When off, they stay behind the Details toggle; your last toggle also updates this preference.",
                                toggle(false)))
                            .child(divider())
                            .child(srow("Banked resets",
                                "Include Codex banked resets in the details.",
                                toggle(true)))
                            .into_any_element(),
                    ));
            }

            // ---------- Connectors ----------
            if self.settings_section == "connectors" {
                let search = div()
                    .w_full()
                    .h(px(30.))
                    .px_2()
                    .rounded(px(8.))
                    .border_1()
                    .border_color(theme.border)
                    .flex()
                    .items_center()
                    .child(div().text_xs().text_color(theme.muted_foreground).child("Search connectors"))
                    .into_any_element();
                let dark_btn = |label: &'static str| {
                    div()
                        .px_3()
                        .py_1p5()
                        .rounded_full()
                        .bg(rgb(0x26251f))
                        .text_xs()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(gpui::white())
                        .child(label)
                        .into_any_element()
                };
                let catalog: [(&'static str, &'static str); 3] = [
                    ("popular", "Popular"),
                    ("developer", "Developer tools"),
                    ("productivity", "Productivity"),
                ];
                let items: [(&'static str, &'static str, &'static str, &'static [u8]); 18] = [
                    ("popular","Supabase","Query Postgres, manage projects, and deploy Edge Functions.",icons::B_SUPABASE),
                    ("popular","Vercel","Manage projects and deployments, and read deployment logs.",icons::B_VERCEL),
                    ("popular","Slack","Search, read, and post in your workspace. Needs your own Slack app.",icons::B_SLACK),
                    ("popular","Linear","Find, create, and update issues, projects, and comments.",icons::B_LINEAR),
                    ("popular","Notion","Search, read, and edit pages and databases.",icons::B_NOTION),
                    ("popular","Stripe","Work with customers, payments, and Stripe docs.",icons::B_STRIPE),
                    ("developer","WorkOS","Manage users, organizations, SSO, and AuthKit in your dashboard.",icons::B_WORKOS),
                    ("developer","Convex","Start Convex apps and get setup and scaling guidance.",icons::B_CONVEX),
                    ("developer","Cloudflare","Build with Workers, KV, R2, and D1 on your account.",icons::B_CLOUDFLARE),
                    ("developer","Sentry","Investigate errors, issues, and releases.",icons::B_SENTRY),
                    ("developer","Neon","Manage serverless Postgres projects and branches.",icons::B_NEON),
                    ("developer","PostHog","Query product analytics, feature flags, and errors.",icons::B_POSTHOG),
                    ("developer","Netlify","Create, deploy, and manage Netlify sites.",icons::B_NETLIFY),
                    ("developer","Railway","Deploy and manage Railway projects and services.",icons::B_RAILWAY),
                    ("developer","Prisma","Manage Prisma Postgres databases and backups.",icons::B_PRISMA),
                    ("developer","Hugging Face","Search models, datasets, Spaces, and papers.",icons::B_HUGGINGFACE),
                    ("productivity","Atlassian","Work with Jira issues and Confluence pages.",icons::B_ATLASSIAN),
                    ("productivity","Todoist","Find, add, and complete tasks and projects.",icons::B_TODOIST),
                ];
                settings_pane = settings_pane
                    .child(search)
                    .child(
                        h_flex()
                            .gap_2()
                            .child(dark_btn("+ Add custom"))
                            .child(outline_btn("Add local server")),
                    )
                    ;
                for (cat, cat_name) in &catalog {
                    settings_pane = settings_pane.child(section_label(cat_name));
                    let cat_items: Vec<_> = items.iter().filter(|(c, ..)| c == cat).collect();
                    for chunk in cat_items.chunks(2) {
                        let mut row = h_flex().w_full().gap(px(24.));
                        for (_, name, desc, icon) in chunk {
                            row = row.child(
                                h_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .items_center()
                                    .gap_3()
                                    .py(px(8.))
                                    .child(
                                        div()
                                            .w(px(44.))
                                            .h(px(44.))
                                            .flex_shrink_0()
                                            .rounded(px(14.))
                                            .border_1()
                                            .border_color(theme.border)
                                            .bg(gpui::white())
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(svg_icon(*icon).size(px(20.))),
                                    )
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap(px(1.))
                                            .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(*name))
                                            .child(div().w_full().text_xs().text_color(theme.muted_foreground).child(*desc)),
                                    )
                                    .child(div().px_2().text_sm().text_color(theme.muted_foreground).child("+")),
                            );
                        }
                        if chunk.len() == 1 {
                            row = row.child(div().flex_1());
                        }
                        settings_pane = settings_pane.child(row);
                    }
                }
            }

            // ---------- External agents (integrations) ----------
            if self.settings_section == "integrations" {
                let dark_btn = |label: &'static str| {
                    div()
                        .px_3()
                        .py_1p5()
                        .rounded_full()
                        .bg(rgb(0x26251f))
                        .text_xs()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(gpui::white())
                        .child(label)
                        .into_any_element()
                };
                let exts = arr(get(sdata, "external"), "integrations").to_vec();
                let exts = if exts.is_empty() { arr(get(sdata, "external"), "items").to_vec() } else { exts };
                settings_pane = settings_pane
                    .child(section_label("Connect a coding agent"))
                    .child(card(
                        v_flex()
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .gap_3()
                                    .px_3()
                                    .py(px(10.))
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap(px(2.))
                                            .child(div().text_sm().font_weight(FontWeight::MEDIUM).child("Name"))
                                            .child(div().w_full().text_sm().text_color(theme.muted_foreground).child(
                                                "How this connection appears in Synara. Works with Codex, Claude, and any other MCP-capable agent.")),
                                    )
                                    .child(
                                        div()
                                            .w(px(160.))
                                            .h(px(30.))
                                            .px_2()
                                            .rounded(px(6.))
                                            .border_1()
                                            .border_color(theme.border)
                                            .flex()
                                            .items_center()
                                            .child(div().text_xs().child("Coding agent")),
                                    )
                                    .into_any_element(),
                            )
                            .child(divider())
                            .child(srow("Access all of Synara",
                                "The agent can discover and work in every project, including ones you add later. Turn off to pick specific projects.",
                                toggle(true)))
                            .child(divider())
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .gap_3()
                                    .px_3()
                                    .py(px(10.))
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap(px(2.))
                                            .child(div().text_sm().font_weight(FontWeight::MEDIUM).child("Advanced permissions"))
                                            .child(div().w_full().text_sm().text_color(theme.muted_foreground).child(
                                                "Optional access for existing tasks, shared checkouts, or execution without approvals. The safe defaults are recommended.")),
                                    )
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap_1()
                                            .child(div().text_sm().text_color(theme.muted_foreground).child("Review"))
                                            .child(svg_icon(icons::CHEVRON_RIGHT).size(px(14.)).text_color(theme.muted_foreground)),
                                    )
                                    .into_any_element(),
                            )
                            .child(divider())
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .gap_3()
                                    .px_3()
                                    .py(px(10.))
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap(px(2.))
                                            .child(div().text_sm().font_weight(FontWeight::MEDIUM).child("Create connection"))
                                            .child(div().w_full().text_sm().text_color(theme.muted_foreground).child(
                                                "The connection lasts 30 days and can be revoked at any time. The next screen gives you one prompt to paste into your agent.")),
                                    )
                                    .child(dark_btn("Create connection"))
                                    .into_any_element(),
                            )
                            .into_any_element(),
                    ))
                    .child(section_label("Connected agents"));
                if exts.is_empty() {
                    settings_pane = settings_pane.child(card(
                        div().px_3().py(px(10.)).text_sm().text_color(theme.muted_foreground)
                            .child("No external agents connected.").into_any_element(),
                    ));
                }
                for e in &exts {
                    let nm = str_at(e, "name").to_string();
                    let st = str_at(e, "status").to_string();
                    settings_pane = settings_pane.child(card(
                        v_flex()
                            .px_3()
                            .py(px(8.))
                            .gap(px(1.))
                            .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(nm))
                            .child(div().text_xs().text_color(theme.muted_foreground).child(st))
                            .into_any_element(),
                    ));
                }
            }

            // ---------- Devices (connections) ----------
            if self.settings_section == "devices" {
                settings_pane = settings_pane
                    .child(section_label("Connections"))
                    .child(card(
                        v_flex()
                            .px_3()
                            .py(px(10.))
                            .gap(px(1.))
                            .child(div().text_sm().font_weight(FontWeight::MEDIUM).child("Unavailable in this build"))
                            .child(div().text_sm().text_color(theme.muted_foreground).child(
                                "Remote connections require the Node 24 server runtime."))
                            .into_any_element(),
                    ));
            }

            // ---------- Computer use ----------
            if self.settings_section == "computer" {
                let comp = get(sdata, "computer");
                let avail = get(comp, "available").as_bool().unwrap_or(false);
                settings_pane = settings_pane
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .px_2()
                            .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).text_color(theme.muted_foreground).child("Computer control"))
                            .child(toggle(false)),
                    )
                    .child(div().w_full().px_2().text_sm().text_color(theme.muted_foreground).child(
                        "Enable Computer by default in any chat. Leave this off and use /computer-use for one request without adding Computer tools to ordinary turns."))
                    .child(card(
                        v_flex()
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .gap_2()
                                    .px_3()
                                    .py(px(10.))
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap(px(1.))
                                            .child(
                                                h_flex()
                                                    .items_center()
                                                    .gap_1p5()
                                                    .child(div().w(px(6.)).h(px(6.)).rounded_full().bg(rgb(0xd43d3d)))
                                                    .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(if avail { "Computer control is available" } else { "Computer control is unavailable" })),
                                            )
                                            .child(div().text_sm().text_color(theme.muted_foreground).child(
                                                "Computer requires a connected Synara desktop host, which owns native access on that computer.")),
                                    )
                                    .child(outline_btn("Set up"))
                                    .into_any_element(),
                            )
                            .into_any_element(),
                    ))
                    .child(card(
                        v_flex()
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .gap_3()
                                    .px_3()
                                    .py(px(10.))
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap(px(2.))
                                            .child(div().text_sm().font_weight(FontWeight::MEDIUM).child("Cursor colors"))
                                            .child(div().w_full().text_sm().text_color(theme.muted_foreground).child(
                                                "The agent pointer is stock monochrome by default \u{2014} like a normal pointer. Custom colors apply to new computer sessions.")),
                                    )
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap_3()
                                            .child(div().text_sm().font_weight(FontWeight::MEDIUM).child("Stock"))
                                            .child(div().text_sm().text_color(theme.muted_foreground).child("Custom")),
                                    )
                                    .into_any_element(),
                            )
                            .child(divider())
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .gap_3()
                                    .px_3()
                                    .py(px(10.))
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap(px(2.))
                                            .child(div().text_sm().font_weight(FontWeight::MEDIUM).child("Preview"))
                                            .child(div().w_full().text_sm().text_color(theme.muted_foreground).child(
                                                "Show the live preview the first time an agent acts on the desktop in a chat. Compact keeps it small and glanceable; Large gives it the full wide card.")),
                                    )
                                    .child(toggle(true))
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap_3()
                                            .child(div().px_2().py(px(2.)).rounded(px(5.)).border_1().border_color(theme.border).text_sm().child("Compact"))
                                            .child(div().text_sm().text_color(theme.muted_foreground).child("Large")),
                                    )
                                    .into_any_element(),
                            )
                            .into_any_element(),
                    ))
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .child(section_label("Getting started"))
                            .child(div().px_2().py_1().text_sm().text_color(theme.muted_foreground).child("\u{2304} Hide guide")),
                    )
                    .child(card(
                        v_flex()
                            .child(
                                v_flex()
                                    .px_3()
                                    .py(px(8.))
                                    .gap(px(1.))
                                    .child(div().text_sm().font_weight(FontWeight::MEDIUM).child("Ask for a task"))
                                    .child(div().w_full().text_sm().text_color(theme.muted_foreground).child(
                                        "Type /computer-use followed by your task, for example: \u{201c}/computer-use open Calculator and calculate 123 \u{d7} 45.\u{201d} This enables Computer for that request only. The default setting below can enable it on every turn.")),
                            )
                            .child(divider())
                            .child(
                                v_flex()
                                    .px_3()
                                    .py(px(8.))
                                    .gap(px(1.))
                                    .child(div().text_sm().font_weight(FontWeight::MEDIUM).child("Approve the task"))
                                    .child(div().w_full().text_sm().text_color(theme.muted_foreground).child(
                                        "If asked, approve Computer for the task. Use the permission guide when desktop access is missing. Synara may still ask before consequential actions.")),
                            )
                            .child(divider())
                            .child(
                                v_flex()
                                    .px_3()
                                    .py(px(8.))
                                    .gap(px(1.))
                                    .child(div().text_sm().font_weight(FontWeight::MEDIUM).child("Follow and stop"))
                                    .child(div().w_full().text_sm().text_color(theme.muted_foreground).child(
                                        "Watch the preview while the agent works. Use Stop in the chat to interrupt the task. Closing the preview only hides it.")),
                            )
                            .into_any_element(),
                    ))
                    .child(div().pt_1().child(outline_btn("Got it")))
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .px_2()
                            .child(div().text_sm().text_color(theme.muted_foreground).child("Recent Computer actions"))
                            .child(div().text_sm().text_color(theme.muted_foreground).child("\u{203a} Show history")),
                    );
            }

            // ---------- AppSnap ----------
            if self.settings_section == "appsnap" {
                settings_pane = settings_pane
                    .child(card(
                        h_flex()
                            .w_full()
                            .gap_3()
                            .px_3()
                            .py(px(10.))
                            .child(svg_icon(icons::S_SCREEN_CAPTURE).size(px(18.)).text_color(theme.muted_foreground))
                            .child(
                                v_flex()
                                    .flex_1()
                                    .gap(px(2.))
                                    .child(div().text_sm().font_weight(FontWeight::MEDIUM).child("Take an AppSnap to show your agent another app's window"))
                                    .child(div().w_full().text_sm().text_color(theme.muted_foreground).child(
                                        "Press your two-key shortcut while any app is frontmost. Synara captures that window as an image, brings itself forward, and attaches the snap to a task composer \u{2014} the capture stays on this device until you send the message."))
                                    .child(div().w_full().text_xs().text_color(theme.muted_foreground).child(
                                        "AppSnap requires the Synara desktop app on macOS.")),
                            )
                            .into_any_element(),
                    ))
                    .child(section_label("Capture"))
                    .child(card(
                        v_flex()
                            .child(
                                v_flex()
                                    .px_3()
                                    .py(px(10.))
                                    .gap(px(2.))
                                    .child(
                                        h_flex()
                                            .w_full()
                                            .items_center()
                                            .justify_between()
                                            .child(div().text_sm().font_weight(FontWeight::MEDIUM).child("Enable AppSnap"))
                                            .child(toggle(false)),
                                    )
                                    .child(div().w_full().text_sm().text_color(theme.muted_foreground).child(
                                        "Run the capture listener in the background while Synara is open."))
                                    .child(div().w_full().text_xs().text_color(theme.muted_foreground).child(
                                        "Available in the Synara desktop app")),
                            )
                            .child(divider())
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .gap_3()
                                    .px_3()
                                    .py(px(10.))
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap(px(2.))
                                            .child(div().text_sm().font_weight(FontWeight::MEDIUM).child("Shortcut"))
                                            .child(div().w_full().text_sm().text_color(theme.muted_foreground).child(
                                                "Choose exactly two keys: one modifier and one other key. Synara checks its own bindings and asks macOS whether another app already owns the shortcut before saving it.")),
                                    )
                                    .child(
                                        v_flex()
                                            .items_end()
                                            .gap(px(2.))
                                            .child(div().text_sm().child("\u{2325} left + \u{2325} right"))
                                            .child(div().text_xs().text_color(theme.muted_foreground).child("Current shortcut")),
                                    )
                                    .into_any_element(),
                            )
                            .child(divider())
                            .child(srow("Destination",
                                "Snaps join the task you interacted with in the last minute, and consecutive snaps stay together. Otherwise Synara opens a fresh task with the capture attached.",
                                select_pill("Automatic".into(), None)))
                            .child(divider())
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .gap_2()
                                    .px_3()
                                    .py(px(10.))
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap(px(2.))
                                            .child(div().text_sm().font_weight(FontWeight::MEDIUM).child("Capture sound"))
                                            .child(div().w_full().text_sm().text_color(theme.muted_foreground).child(
                                                "Play a short shutter cue when a window is captured.")),
                                    )
                                    .child(div().text_xs().text_color(theme.muted_foreground).child("Preview"))
                                    .child(toggle(true))
                                    .into_any_element(),
                            )
                            .into_any_element(),
                    ));
            }

            // ---------- Worktrees ----------
            if self.settings_section == "worktrees" {
                let wts = arr(get(sdata, "worktrees"), "worktrees").to_vec();
                let wts = if wts.is_empty() { arr(get(sdata, "worktrees"), "items").to_vec() } else { wts };
                settings_pane = settings_pane
                    .child(section_label("Defaults"))
                    .child(card(
                        srow("Delete worktree on archive",
                            "After Archive's Undo period, remove a clean worktree only if the task has stopped and no other task uses it. Its branch remains available for recovery.",
                            toggle(false))
                        .into_any_element(),
                    ));
                if wts.is_empty() {
                    settings_pane = settings_pane.child(
                        div()
                            .w_full()
                            .rounded(px(12.))
                            .border_1()
                            .border_color(theme.border)
                            .py(px(24.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(div().text_sm().text_color(theme.muted_foreground).child("No app-managed worktrees found yet.")),
                    );
                }
                for w in &wts {
                    let nm = str_at(w, "name").to_string();
                    let path = str_at(w, "path").to_string();
                    settings_pane = settings_pane.child(card(
                        v_flex()
                            .px_3()
                            .py(px(8.))
                            .gap(px(1.))
                            .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(nm))
                            .child(div().text_xs().text_color(theme.muted_foreground).child(path))
                            .into_any_element(),
                    ));
                }
            }

            // ---------- Archive ----------
            if self.settings_section == "archive" {
                settings_pane = settings_pane.child(
                    div()
                        .w_full()
                        .rounded(px(12.))
                        .border_1()
                        .border_color(theme.border)
                        .py(px(40.))
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .gap_2()
                        .child(svg_icon(icons::TRASH).size(px(20.)).text_color(theme.muted_foreground))
                        .child(div().text_sm().font_weight(FontWeight::MEDIUM).child("No archived threads"))
                        .child(div().text_sm().text_color(theme.muted_foreground).child(
                            "Archived threads will appear here and can be restored to the sidebar.")),
                );
            }

            // ---------- Privacy ----------
            if self.settings_section == "privacy" {
                // Upstream renders nothing extra in this build (Safari onboarding and
                // product-analytics rows are desktop/remote-gated) — title + subtitle only.
            }

            // ---------- Advanced ----------
            if self.settings_section == "advanced" {
                settings_pane = settings_pane
                    .child(section_label("Developer tools"))
                    .child(card(
                        v_flex()
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .gap_3()
                                    .px_3()
                                    .py(px(10.))
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap(px(2.))
                                            .child(div().text_sm().font_weight(FontWeight::MEDIUM).child("Recovery tools"))
                                            .child(div().w_full().text_sm().text_color(theme.muted_foreground).child(
                                                "Rebuild local project indexes without clearing existing chats when the local state gets out of sync."))
                                            .child(div().w_full().text_xs().text_color(theme.muted_foreground).child(
                                                "Visible because projects exist but no chat history is currently available.")),
                                    )
                                    .child(outline_btn("Repair state"))
                                    .into_any_element(),
                            )
                            .child(divider())
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .px_3()
                                    .py(px(8.))
                                    .child(div().text_sm().child("What this does"))
                                    .child(svg_icon(icons::CHEVRON_RIGHT).size(px(14.)).text_color(theme.muted_foreground))
                                    .into_any_element(),
                            )
                            .into_any_element(),
                    ));
            }

            // ---------- About ----------
            if self.settings_section == "about" {
                settings_pane = settings_pane
                    .child(section_label("About"))
                    .child(card(
                        v_flex()
                            .child(srow("Version", "Current application version.",
                                div().text_sm().text_color(theme.muted_foreground).child("1.6.4").into_any_element()))
                            .child(divider())
                            .child(srow("Release history",
                                "A running log of every update, newest first. Same notes the post-update dialog shows, kept here so you can revisit them any time.",
                                outline_btn("View release history")))
                            .into_any_element(),
                    ));
            }

            if self.settings_section == "appearance" {
                // Plain row without a description line.
                let prow = |label: &'static str, ctrl: gpui::AnyElement| {
                    h_flex()
                        .w_full()
                        .items_center()
                        .justify_between()
                        .px_3()
                        .py(px(10.))
                        .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(label))
                        .child(ctrl)
                        .into_any_element()
                };
                settings_pane = settings_pane.child(
                    v_flex()
                        .gap_1p5()
                        .child(section_label("Theme"))
                        .child(
                            h_flex()
                                .gap_3()
                                .child(theme_card(0, "System", true))
                                .child(theme_card(1, "Light", false))
                                .child(theme_card(2, "Dark", false)),
                        ),
                );
                for dark in [false, true] {
                    let (name, desc, acc, bgc, fgc, al, bl, fl) = if dark {
                        ("Dark theme", "Used when your system switches to dark.", 0xf2612d, 0x111111, 0xfcfcfc, "#F2612D", "#111111", "#FCFCFC")
                    } else {
                        ("Light theme", "System is currently using this light slot.", 0xc74614, 0xffffff, 0x0d0d0d, "#C74614", "#FFFFFF", "#0D0D0D")
                    };
                    let aa_chip = if dark {
                        div().rounded(px(4.)).bg(rgb(0x111111)).px(px(3.))
                            .child(div().text_xs().text_color(gpui::white()).child("Aa"))
                            .into_any_element()
                    } else {
                        div().text_sm().text_color(accent_c).child("Aa").into_any_element()
                    };
                    let mut sel = h_flex()
                        .w_full()
                        .mx_3()
                        .mt_1()
                        .mb_2()
                        .h(px(36.))
                        .items_center()
                        .justify_between()
                        .px_3()
                        .rounded(px(8.))
                        .border_1()
                        .border_color(theme.border);
                    if dark {
                        sel = sel.bg(rgb(0x111111)).child(div().text_sm().text_color(gpui::white()).child("Synara"));
                    } else {
                        sel = sel.child(div().text_sm().child("Synara"));
                    }
                    settings_pane = settings_pane.child(card(
                        v_flex()
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .px_3()
                                    .pt_3()
                                    .pb_1()
                                    .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(name))
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap_3()
                                            .child(div().text_sm().text_color(theme.muted_foreground).child("Import"))
                                            .child(div().text_sm().text_color(theme.muted_foreground).child("Copy"))
                                            .child(
                                                h_flex()
                                                    .items_center()
                                                    .gap_1()
                                                    .child(aa_chip)
                                                    .child(div().text_sm().child("Synara")),
                                            ),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .px_3()
                                    .pb_2()
                                    .child(div().text_sm().text_color(theme.muted_foreground).child(desc))
                                    .when(dark, |d| {
                                        d.child(
                                            h_flex()
                                                .items_center()
                                                .gap_2()
                                                .child(div().text_sm().text_color(theme.muted_foreground).child("Use dark theme"))
                                                .child(toggle(true)),
                                        )
                                    }),
                            )
                            .child(sel.child(div().text_sm().text_color(accent_c).child("Accent preview")))
                            .child(divider())
                            .child(prow("Accent", swatch_pill(al, acc)))
                            .child(divider())
                            .child(prow("Background", swatch_pill(bl, bgc)))
                            .child(divider())
                            .child(prow("Foreground", swatch_pill(fl, fgc)))
                            .child(divider())
                            .child(prow(
                                "UI font",
                                v_flex()
                                    .items_end()
                                    .gap_1()
                                    .child(select_pill("System default".to_string(), None))
                                    .child(div().text_xs().text_color(theme.muted_foreground).child("Use system UI font is on; theme fonts are not applied."))
                                    .into_any_element(),
                            ))
                            .child(divider())
                            .child(prow("Code font", select_pill("\"JetBrains Mono\"".to_string(), None)))
                            .child(divider())
                            .child(prow("Window", seg(&["Solid", "Translucent"], 1)))
                            .child(divider())
                            .child(prow("Sidebar only", toggle(false)))
                            .child(divider())
                            .child(prow("Opacity", slider(0.98, "98%")))
                            .child(divider())
                            .child(prow(
                                "Blur",
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child(div().text_xs().text_color(theme.muted_foreground).child("Auto"))
                                    .child(slider(0.64, "64"))
                                    .into_any_element(),
                            ))
                            .child(divider())
                            .child(prow("Contrast", slider(0.0, "0")))
                            .into_any_element(),
                    ));
                }
                settings_pane = settings_pane
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(section_label("Effects"))
                            .child(card(
                                v_flex()
                                    .child(srow(
                                        "ASCII background",
                                        "Show the animated ASCII field on empty chats. Reduced motion keeps the field still.",
                                        toggle(true),
                                    ))
                                    .into_any_element(),
                            )),
                    )
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(section_label("Typography and spacing"))
                            .child(card(
                                v_flex()
                                    .child(srow(
                                        "Use system UI font",
                                        "Ignore the theme's custom UI font and render the interface with the native system font (SF Pro on macOS).",
                                        toggle(true),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "UI density",
                                        "Control spacing in the sidebar, composer, chat gutters, and settings rows without changing font size.",
                                        seg(&["Compact", "Comfortable", "Spacious"], 1),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Chat width",
                                        "Control how wide the chat column grows. Wide and Full give tables and wide content more room.",
                                        seg(&["Standard", "Wide", "Full"], 0),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Base font size",
                                        "Adjust the app text base in pixels. Chat and UI typography scale proportionally from this value.",
                                        num_field("13"),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Terminal font size",
                                        "Adjust terminal text independently from the app and chat font size.",
                                        num_field("12"),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Terminal font",
                                        "Type any monospace font installed on this device (e.g. Fira Code). Leave empty for the default. Fonts that aren't installed fall back to the system monospace.",
                                        select_pill("Default (JetBrains Mono)".to_string(), None),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Font smoothing",
                                        "Use macOS-style antialiasing for lighter, crisper text rendering.",
                                        toggle(true),
                                    ))
                                    .into_any_element(),
                            )),
                    );
            }
        }
        // Center the content column at upstream max-w-2xl (672px).
        let settings_pane = h_flex()
            .flex_1()
            .w_full()
            .justify_center()
            .child(settings_pane);
        if std::env::var("SYNARA_SCROLL_BOTTOM").is_ok() {
            self.settings_scroll.scroll_to_bottom();
        }

        // ---- settings sidebar nav (replaces project sidebar) ----
        let mut settings_nav_el = v_flex().flex_1().h_full().px(px(6.)).py(px(6.)).gap(px(2.));
        settings_nav_el = settings_nav_el.child(
            div()
                .px_2()
                .pb_2()
                .text_base()
                .font_weight(FontWeight::SEMIBOLD)
                .child("Settings"),
        );
        settings_nav_el = settings_nav_el.child(
            h_flex()
                .mx_1()
                .mb_2()
                .h(px(30.))
                .px_2()
                .gap_2()
                .items_center()
                .rounded(px(8.))
                .border_1()
                .border_color(theme.border)
                .bg(rgb(0xfafafa))
                .child(svg_icon(icons::SEARCH).size(px(14.)).text_color(theme.muted_foreground))
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child("Search settings..."),
                ),
        );
        let mut nav_list = v_flex()
            .flex_1()
            .min_h_0()
            .id("settings-nav-scroll")
            .track_scroll(&self.settings_nav_scroll)
            .overflow_y_scroll()
            .gap(px(2.));
        for (group, items) in settings_nav {
            if !group.is_empty() {
                nav_list = nav_list.child(
                    div()
                        .mt_3()
                        .px_2()
                        .py_1()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(*group),
                );
            }
            for (id, label, icon, badge) in *items {
                let active = *id == self.settings_section;
                let mut row = h_flex()
                    .id(ElementId::Name(format!("setnav-{id}").into()))
                    .h(px(28.))
                    .px_2()
                    .gap_2()
                    .items_center()
                    .rounded(px(6.))
                    .cursor_pointer()
                    .child(svg_icon(icon).size(px(16.)).text_color(if active { theme.foreground } else { theme.muted_foreground }))
                    .child(div().text_sm().whitespace_nowrap().child(*label));
                if let Some(b) = badge {
                    row = row.child(
                        div()
                            .ml_auto()
                            .rounded_full()
                            .px(px(6.))
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .border_1()
                            .border_color(theme.border)
                            .child(*b),
                    );
                }
                let sid: &'static str = id;
                let row = if active { row.bg(theme.list_active) } else { row };
                nav_list = nav_list.child(
                    row.on_click(cx.listener(move |this, _ev, _w, cx| {
                        this.settings_section = sid;
                        if this.settings_data.is_null() {
                            this.fetch_settings(cx);
                        }
                        cx.notify();
                    })),
                );
            }
        }
        settings_nav_el = settings_nav_el.child(nav_list);

        // ==================== rail route views ====================
        // Segmented pill chip — upstream: muted rounded track, white active.
        let seg_chip = |id: &'static str, label: &'static str, active: bool| {
            let d = div()
                .id(ElementId::Name(id.into()))
                .h(px(24.))
                .px(px(10.))
                .flex()
                .items_center()
                .rounded(px(6.))
                .cursor_pointer()
                .child(
                    div()
                        .text_sm()
                        .text_color(if active {
                            theme.foreground
                        } else {
                            theme.muted_foreground
                        })
                        .child(label),
                );
            if active { d.bg(theme.background) } else { d }
        };

        // /tasks — rail only: List|Kanban toggle top-right, centered column.
        let tasks_pane = v_flex()
            .flex_1()
            .h_full()
            .child(
                h_flex()
                    .w_full()
                    .pt(px(9.))
                    .pr(px(19.))
                    .child(div().flex_1())
                    .child(
                        h_flex()
                            .gap(px(2.))
                            .p(px(2.))
                            .rounded(px(8.))
                            .bg(rgb(0xf0f0f0))
                            .child(seg_chip("seg-tasks-list", "List", true).on_click(cx.listener(
                                |this, _e, _w, cx| this.open_route(1, cx),
                            )))
                            .child(seg_chip("seg-tasks-kanban", "Kanban", false).on_click(
                                cx.listener(|this, _e, _w, cx| this.open_route(2, cx)),
                            )),
                    ),
            )
            .child(
                // Upstream: a centered fixed-width column, contents
                // left-aligned; add-task row is a faint borderless row.
                v_flex()
                    .flex_1()
                    .items_center()
                    .child(
                        v_flex()
                            .w(px(560.))
                            .pt(px(36.))
                            .child(
                                div()
                                    .text_size(px(24.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("Tasks"),
                            )
                            .child(
                                div()
                                    .mt(px(4.))
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child("Add anything you need to do"),
                            )
                            .child(
                                h_flex()
                                    .mt(px(36.))
                                    .h(px(28.))
                                    .gap(px(10.))
                                    .items_center()
                                    .child(
                                        div()
                                            .size(px(14.))
                                            .rounded_full()
                                            .border_1()
                                            .border_color(theme.muted_foreground),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(theme.muted_foreground)
                                            .child("Add a task, or press Tab to hand it to an agent"),
                                    ),
                            ),
                    ),
            );

        // /kanban — header strip + one column per project of task cards
        // (threads are the upstream kanban items; "Chats" groups chat-kind).
        let live = |t: &Value| get(t, "deletedAt").is_null() && get(t, "archivedAt").is_null();
        let mut kanban_cols = h_flex().px_4().pt_2().gap_3().items_start();
        let mut kanban_count = 0usize;
        {
            let snap = self.snapshot.clone().unwrap_or(Value::Null);
            let kb_projects = arr(&snap, "projects");
            let kb_threads = arr(&snap, "threads");
            let chat_ids: Vec<String> = kb_projects
                .iter()
                .filter(|p| str_at(p, "kind") == "chat")
                .map(|p| str_at(p, "id").to_string())
                .collect();
            let mut groups: Vec<(String, Vec<Value>)> = Vec::new();
            for p in kb_projects {
                if str_at(p, "kind") == "chat" {
                    continue;
                }
                let pid = str_at(p, "id");
                let mut kids: Vec<Value> = kb_threads
                    .iter()
                    .filter(|t| str_at(t, "projectId") == pid && live(t))
                    .cloned()
                    .collect();
                kids.sort_by_key(|t| std::cmp::Reverse(str_at(t, "updatedAt").to_string()));
                if !kids.is_empty() {
                    groups.push((str_at(p, "title").to_string(), kids));
                }
            }
            let mut chats: Vec<Value> = kb_threads
                .iter()
                .filter(|t| {
                    live(t)
                        && (chat_ids.iter().any(|id| id == str_at(t, "projectId"))
                            || !kb_projects
                                .iter()
                                .any(|p| str_at(p, "id") == str_at(t, "projectId")))
                })
                .cloned()
                .collect();
            chats.sort_by_key(|t| std::cmp::Reverse(str_at(t, "updatedAt").to_string()));
            if !chats.is_empty() {
                groups.push(("Chats".to_string(), chats));
            }
            kanban_count = groups.iter().map(|g| g.1.len()).sum();
            for (name, kids) in groups {
                let n = kids.len();
                let mut col = v_flex().w(px(275.)).gap(px(6.));
                col = col.child(
                    h_flex()
                        .items_center()
                        .px(px(2.))
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(name),
                        )
                        .child(
                            div()
                                .ml(px(4.))
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child(format!("{n}")),
                        )
                        .child(div().flex_1())
                        .child(
                            svg_icon(icons::PLUS)
                                .size(px(14.))
                                .text_color(theme.muted_foreground),
                        ),
                );
                for t in &kids {
                    let title = str_at(t, "title").to_string();
                    let branch = str_at(t, "branch");
                    let mut when = rel_time(str_at(t, "updatedAt"));
                    if when.is_empty() {
                        when = rel_time(str_at(t, "createdAt"));
                    }
                    let mut meta = h_flex().gap(px(6.)).items_center().child(
                        svg_icon(icons::CODEX)
                            .size(px(12.))
                            .text_color(theme.muted_foreground),
                    );
                    if !branch.is_empty() {
                        meta = meta.child(
                            h_flex()
                                .gap(px(2.))
                                .items_center()
                                .child(
                                    svg_icon(icons::S_BRANCH_SIMPLE)
                                        .size(px(11.))
                                        .text_color(theme.muted_foreground),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .whitespace_nowrap()
                                        .overflow_hidden()
                                        .child(branch.to_string()),
                                ),
                        );
                    }
                    meta = meta.child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(when),
                    );
                    meta = meta.child(
                        h_flex()
                            .ml_auto()
                            .gap(px(4.))
                            .items_center()
                            .child(
                                div()
                                    .size(px(8.))
                                    .rounded_full()
                                    .border_1()
                                    .border_color(theme.muted_foreground),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child("Draft"),
                            ),
                    );
                    col = col.child(
                        v_flex()
                            .rounded(px(10.))
                            .bg(theme.background)
                            .border_1()
                            .border_color(theme.border)
                            .p(px(8.))
                            .gap(px(6.))
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .whitespace_nowrap()
                                    .overflow_hidden()
                                    .child(title),
                            )
                            .child(meta),
                    );
                }
                kanban_cols = kanban_cols.child(col);
            }
        }
        let kanban_pane = v_flex()
            .flex_1()
            .h_full()
            .child(
                h_flex()
                    .w_full()
                    .px(px(12.))
                    .pt(px(9.))
                    .pb(px(6.))
                    .gap(px(8.))
                    .items_center()
                    .child(
                        svg_icon(icons::SIDEBAR_LEFT)
                            .size(px(18.))
                            .text_color(theme.muted_foreground),
                    )
                    .child(
                        div()
                            .text_base()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Kanban"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(format!("{kanban_count} tasks")),
                    )
                    .child(div().flex_1())
                    .child(
                        h_flex()
                            .gap(px(2.))
                            .p(px(2.))
                            .rounded(px(8.))
                            .bg(rgb(0xf0f0f0))
                            .child(seg_chip("seg-kb-list", "List", false).on_click(cx.listener(
                                |this, _e, _w, cx| this.open_route(1, cx),
                            )))
                            .child(seg_chip("seg-kb-kanban", "Kanban", true).on_click(
                                cx.listener(|this, _e, _w, cx| this.open_route(2, cx)),
                            )),
                    )
                    .child(
                        h_flex()
                            .gap(px(2.))
                            .p(px(2.))
                            .rounded(px(8.))
                            .bg(rgb(0xf0f0f0))
                            .child(seg_chip("seg-kb-att", "Attention", true))
                            .child(seg_chip("seg-kb-classic", "Classic", false)),
                    )
                    .child(
                        div()
                            .h(px(26.))
                            .px(px(10.))
                            .flex()
                            .items_center()
                            .child(div().text_sm().child("+ New task")),
                    ),
            )
            .child(
                h_flex()
                    .px(px(16.))
                    .pb(px(4.))
                    .gap(px(6.))
                    .items_center()
                    .child(
                        div()
                            .size(px(12.))
                            .rounded(px(3.))
                            .border_1()
                            .border_color(theme.muted_foreground),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("Needs review"),
                    ),
            )
            .child(kanban_cols);

        // /pull-requests — "Code review" list panel + empty detail pane.
        let mut pr_rows = v_flex().px_2().pt_2().gap(px(8.));
        for _ in 0..8 {
            pr_rows = pr_rows.child(div().h(px(56.)).rounded(px(8.)).bg(rgb(0xf4f4f4)));
        }
        let pullreq_panel = v_flex()
            .flex_1()
            .h_full()
            .px(px(6.))
            .py(px(6.))
            .gap(px(2.))
            .child(
                h_flex()
                    .px_2()
                    .pb_2()
                    .items_center()
                    .child(
                        div()
                            .text_base()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Code review"),
                    )
                    .child(div().flex_1())
                    .child(
                        svg_icon(icons::SORT)
                            .size(px(14.))
                            .text_color(theme.muted_foreground),
                    )
                    .child(div().w(px(8.)))
                    .child(
                        svg_icon(icons::FILTER)
                            .size(px(14.))
                            .text_color(theme.muted_foreground),
                    )
                    .child(div().w(px(8.)))
                    .child(
                        svg_icon(icons::MORE)
                            .size(px(14.))
                            .text_color(theme.muted_foreground),
                    ),
            )
            .child(
                h_flex()
                    .mx_1()
                    .mb_2()
                    .h(px(30.))
                    .px_2()
                    .gap_2()
                    .items_center()
                    .rounded(px(8.))
                    .border_1()
                    .border_color(theme.border)
                    .bg(rgb(0xfafafa))
                    .child(
                        svg_icon(icons::SEARCH)
                            .size(px(14.))
                            .text_color(theme.muted_foreground),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("Search or paste a PR link"),
                    ),
            )
            .child(
                h_flex()
                    .px_2()
                    .pb_2()
                    .gap_4()
                    .items_center()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .border_b_2()
                            .border_color(theme.foreground)
                            .child("All"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("PRs"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("Issues"),
                    ),
            )
            .child(pr_rows);
        let pullreq_pane = v_flex()
            .flex_1()
            .h_full()
            .items_center()
            .justify_center()
            .gap_2()
            .child(
                svg_icon(icons::PULL_REQUESTS)
                    .size(px(36.))
                    .text_color(theme.muted_foreground),
            )
            .child(
                div()
                    .text_base()
                    .font_weight(FontWeight::MEDIUM)
                    .child("Select a pull request or issue"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("Choose one from the sidebar to review it"),
            );

        // /automations — list panel + empty detail pane w/ refresh affordance.
        let autom_panel = v_flex()
            .flex_1()
            .h_full()
            .px(px(6.))
            .py(px(6.))
            .gap(px(2.))
            .child(
                div()
                    .px_2()
                    .pb_2()
                    .text_base()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Automations"),
            )
            .child(
                h_flex()
                    .h(px(28.))
                    .px_2()
                    .gap_2()
                    .items_center()
                    .rounded(px(6.))
                    .child(
                        svg_icon(icons::PLUS_MED)
                            .size(px(14.))
                            .text_color(theme.foreground),
                    )
                    .child(div().text_sm().child("New automation")),
            )
            .child(
                div()
                    .mt_2()
                    .px_4()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("No automations yet"),
            );
        let autom_pane = v_flex()
            .flex_1()
            .h_full()
            .child(
                h_flex()
                    .w_full()
                    .pt(px(9.))
                    .pr(px(19.))
                    .child(div().flex_1())
                    .child(
                        svg_icon(icons::REFRESH)
                            .size(px(18.))
                            .text_color(theme.muted_foreground),
                    ),
            )
            .child(
                v_flex()
                    .flex_1()
                    .items_center()
                    .justify_center()
                    .gap_2()
                    .pb(px(120.))
                    .child(
                        svg_icon(icons::AUTOMATIONS)
                            .size(px(32.))
                            .text_color(theme.muted_foreground),
                    )
                    .child(
                        div()
                            .text_base()
                            .font_weight(FontWeight::MEDIUM)
                            .child("Automations"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("Pick an automation in the panel to see its runs, or"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("schedule a new one."),
                    )
                    .child(
                        div()
                            .mt_2()
                            .h(px(30.))
                            .px_4()
                            .flex()
                            .items_center()
                            .rounded(px(8.))
                            .bg(theme.foreground)
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.background)
                                    .child("New automation"),
                            ),
                    ),
            );

        // /hubs (studio rail icon) — hubs list panel + empty pane.
        let studio_panel = v_flex()
            .flex_1()
            .h_full()
            .px(px(6.))
            .py(px(6.))
            .gap(px(2.))
            .child(
                h_flex()
                    .px_2()
                    .pb_2()
                    .items_center()
                    .child(
                        div()
                            .text_base()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Hubs"),
                    )
                    .child(
                        div()
                            .pl_1()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("\u{2304}"),
                    )
                    .child(div().flex_1())
                    .child(
                        svg_icon(icons::SEARCH)
                            .size(px(14.))
                            .text_color(theme.muted_foreground),
                    ),
            )
            .child(
                h_flex()
                    .h(px(28.))
                    .px_2()
                    .gap_2()
                    .items_center()
                    .rounded(px(6.))
                    .child(
                        svg_icon(icons::PENCIL)
                            .size(px(14.))
                            .text_color(theme.muted_foreground),
                    )
                    .child(div().text_sm().child("New hub")),
            )
            .child(
                div()
                    .mt_2()
                    .px_2()
                    .py_1()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("Hubs"),
            )
            .child(
                div()
                    .px_4()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("No hubs yet"),
            );
        let studio_pane = v_flex()
            .flex_1()
            .h_full()
            .items_center()
            .justify_center()
            .gap_2()
            .child(
                div()
                    .text_base()
                    .font_weight(FontWeight::MEDIUM)
                    .child("No hubs yet"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("Create one from the sidebar to coordinate work across repos."),
            );

        // spaces — a left-panel view like upstream (workspaces list); the main
        // surface stays whatever it was.
        let mut spaces_panel = v_flex().flex_1().h_full().px(px(6.)).py(px(6.)).gap(px(2.));
        spaces_panel = spaces_panel.child(
            div()
                .px_2()
                .pb_2()
                .text_base()
                .font_weight(FontWeight::SEMIBOLD)
                .child("Spaces"),
        );
        let space_list: Vec<Value> = arr(
            self.snapshot.as_ref().unwrap_or(&Value::Null),
            "spaces",
        )
        .to_vec();
        if space_list.is_empty() {
            spaces_panel = spaces_panel.child(
                div()
                    .px_4()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("No spaces yet"),
            );
        } else {
            for s in space_list {
                spaces_panel = spaces_panel.child(
                    h_flex()
                        .h(px(28.))
                        .px_2()
                        .gap_2()
                        .items_center()
                        .rounded(px(6.))
                        .child(
                            svg_icon(icons::STAR)
                                .size(px(14.))
                                .text_color(theme.muted_foreground),
                        )
                        .child(div().text_sm().child(str_at(&s, "name").to_string())),
                );
            }
        }

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
            main = main.child(div().flex_1().child(settings_pane));
        } else if self.view_dev {
            main = main.child(div().flex_1().child(dev_pane.overflow_y_scrollbar()));
        } else if self.view_files {
            main = main.child(div().flex_1().child(files_pane.overflow_y_scrollbar()));
        } else if self.view_inbox {
            main = main.child(div().flex_1().child(inbox_pane.overflow_y_scrollbar()));
        } else if self.view_tasks {
            main = main.child(div().flex_1().child(tasks_pane));
        } else if self.view_kanban {
            main = main
                .child(div().flex_1().child(kanban_pane.overflow_y_scrollbar()));
        } else if self.view_pullreq {
            main = main.child(div().flex_1().child(pullreq_pane));
        } else if self.view_autom {
            main = main.child(div().flex_1().child(autom_pane));
        } else if self.view_studio {
            main = main.child(div().flex_1().child(studio_pane));
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
        let home_active = !self.view_files
            && !self.view_dev
            && !self.view_settings
            && !self.view_inbox
            && !self.view_tasks
            && !self.view_kanban
            && !self.view_pullreq
            && !self.view_autom
            && !self.view_studio
            && !self.view_spaces;
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
                        this.clear_views();
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
                rail_btn("nav-spaces", icons::SPACES, self.view_spaces, cx).on_click(
                    cx.listener(|this, _ev, _w, cx| {
                        this.open_route(6, cx);
                    }),
                ),
            )
            .child(
                rail_btn("nav-tasks", icons::TASKS, self.view_tasks, cx).on_click(
                    cx.listener(|this, _ev, _w, cx| {
                        this.open_route(1, cx);
                    }),
                ),
            )
            .child(
                rail_btn("nav-pullrequests", icons::PULL_REQUESTS, self.view_pullreq, cx)
                    .on_click(cx.listener(|this, _ev, _w, cx| {
                        this.open_route(3, cx);
                    })),
            )
            .child(
                rail_btn("nav-automations", icons::AUTOMATIONS, self.view_autom, cx)
                    .on_click(cx.listener(|this, _ev, _w, cx| {
                        this.open_route(4, cx);
                    })),
            )
            .child(
                rail_btn("nav-studio", icons::STUDIO, self.view_studio, cx).on_click(
                    cx.listener(|this, _ev, _w, cx| {
                        this.open_route(5, cx);
                    }),
                ),
            )
            .child(
                rail_btn("nav-more", icons::MORE, self.view_settings, cx).on_click(
                    cx.listener(|this, _ev, _w, cx| {
                        let was = this.view_settings;
                        this.clear_views();
                        this.view_settings = !was;
                        if this.view_settings && this.settings_data.is_null() {
                            this.fetch_settings(cx);
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
        // 47px #f7f7f7 band, borderless, over rail+panel+content: sidebar
        // toggle centered over the rail at left, Environment + dock toggles
        // at right (Hugeicons panel-top-open / layout-right).
        let strip = h_flex()
            .w_full()
            .h(px(47.))
            .bg(rgb(0xf7f7f7))
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
        if !self.sidebar_collapsed
            && !self.view_inbox
            && !self.view_tasks
            && !self.view_kanban
        {
            let side: gpui::AnyElement = if self.view_settings {
                settings_nav_el.into_any_element()
            } else if self.view_pullreq {
                pullreq_panel.into_any_element()
            } else if self.view_autom {
                autom_panel.into_any_element()
            } else if self.view_studio {
                studio_panel.into_any_element()
            } else if self.view_spaces {
                spaces_panel.into_any_element()
            } else {
                sidebar.w_full().into_any_element()
            };
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
                    .child(side),
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
            .id("native-root")
            .on_action(cx.listener(|this, _: &ToggleFind, w, cx| {
                this.toggle_find(w, cx);
            }))
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(strip)
            .child(body)
    }
}
