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
    pub(super) fn toggle_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.find_open = !self.find_open;
        if self.find_open {
            let input = cx.new(|cx| InputState::new(window, cx).placeholder("Find in thread"));
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

    /// Open the workspace search palette: "search" = ⌘K chats+projects
    /// (upstream SidebarSearchPalette), "files" = ⌘P projects.searchEntries,
    /// "snippets" = ⇧⌘F projects.searchContent (WorkspaceSearchPalette).
    pub(super) fn open_palette(
        &mut self,
        mode: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.palette_mode == Some(mode) && mode != "search" {
            return;
        }
        self.palette_mode = Some(mode);
        self.palette_rows = Vec::new();
        self.palette_sel = 0;
        self.palette_seq += 1;
        let placeholder = match mode {
            "files" => "Search files",
            "snippets" => "Search code",
            _ => "Search chats or run a command",
        };
        let input = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
        cx.subscribe_in(&input, window, |this: &mut Self, _s, ev, _w, cx| match ev {
            InputEvent::Change => this.palette_query_changed(cx),
            InputEvent::PressEnter { .. } => this.palette_activate(_w, cx),
            _ => {}
        })
        .detach();
        // Focus on the next frame: during key dispatch gpui can restore focus
        // to the dispatch target, swallowing an immediate focus() call.
        window.defer(cx, {
            let input = input.clone();
            move |window, cx| {
                input.update(cx, |s, cx| s.focus_handle(cx).focus(window, cx));
            }
        });
        self.palette_input = Some(input);
        if mode != "search" {
            // Prewarm like upstream so the first keystroke hits a warm index.
            if let Some(cwd) = self.files_root() {
                let client = self.client.clone();
                cx.background_executor()
                    .spawn(async move {
                        if let Some(c) = ensure_client(&client) {
                            c.call("projects.prewarmSearchIndex", json!({"cwd": cwd}));
                        }
                    })
                    .detach();
            }
        }
        cx.notify();
    }

    pub(super) fn close_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.palette_mode = None;
        self.palette_input = None;
        self.palette_rows = Vec::new();
        self.palette_sel = 0;
        // The input's focus handle unmounts with the popup; give focus back
        // to the root or subsequent key events dispatch to a dead handle.
        let f = self.focus_handle.clone();
        window.focus(&f, cx);
        cx.notify();
    }

    /// Re-query on input change: RPC rows for files/snippets, a notify for
    /// chats mode (its rows are computed from the snapshot at render).
    pub(super) fn palette_query_changed(&mut self, cx: &mut Context<Self>) {
        let Some(mode) = self.palette_mode else {
            return;
        };
        self.palette_sel = 0;
        if mode == "search" {
            cx.notify();
            return;
        }
        let Some(input) = self.palette_input.clone() else {
            return;
        };
        let query = input.read(cx).value().to_string();
        let min = if mode == "files" { 1 } else { 2 };
        if query.trim().len() < min {
            self.palette_rows = Vec::new();
            self.palette_seq += 1;
            cx.notify();
            return;
        }
        let Some(cwd) = self.files_root() else {
            return;
        };
        self.palette_seq += 1;
        let seq = self.palette_seq;
        let tag = if mode == "files" {
            "projects.searchEntries"
        } else {
            "projects.searchContent"
        };
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let v = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell)
                        .and_then(|c| c.call(tag, json!({"cwd": cwd, "query": query, "limit": 30})))
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if this.palette_seq != seq {
                    return;
                }
                let key = if this.palette_mode == Some("files") {
                    "entries"
                } else {
                    "matches"
                };
                this.palette_rows = v.map(|v| arr(&v, key).to_vec()).unwrap_or_default();
                if this.palette_sel >= this.palette_rows.len() {
                    this.palette_sel = 0;
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Current palette rows as (kind, label, sub, payload): kind picks the
    /// row icon and the activate action; file/directory/snippet payloads are
    /// cwd-relative paths, project/thread payloads are ids.
    pub(super) fn palette_items(&self, query: &str) -> Vec<(String, String, String, String)> {
        let q = query.trim().to_lowercase();
        match self.palette_mode {
            Some("files") => self
                .palette_rows
                .iter()
                .map(|e| {
                    let rel = str_at(e, "path").to_string();
                    let base = rel.rsplit('/').next().unwrap_or(&rel).to_string();
                    (
                        str_at(e, "kind").to_string(),
                        base,
                        str_at(e, "parentPath").to_string(),
                        rel,
                    )
                })
                .collect(),
            Some("snippets") => self
                .palette_rows
                .iter()
                .map(|m| {
                    let rel = str_at(m, "path").to_string();
                    let line = get(m, "lineNumber").as_i64().unwrap_or(0);
                    (
                        "snippet".to_string(),
                        format!("{rel}:{line}"),
                        str_at(m, "lineText").trim().to_string(),
                        rel,
                    )
                })
                .collect(),
            // Second palette page (upstream mode "import-projects"): pick a
            // provider source, then a fetched importable project.
            Some("import-sources") => {
                let sources: [(&str, &str); 3] = [
                    ("claudeAgent", "From Claude Code"),
                    ("codex", "From Codex"),
                    ("all", "From Claude Code & Codex"),
                ];
                sources
                    .iter()
                    .filter(|(_, l)| q.is_empty() || l.to_lowercase().contains(&q))
                    .map(|(id, l)| {
                        (
                            "import-src".to_string(),
                            (*l).to_string(),
                            String::new(),
                            (*id).to_string(),
                        )
                    })
                    .collect()
            }
            Some("import-projs") => self
                .palette_rows
                .iter()
                .filter(|p| {
                    q.is_empty()
                        || str_at(p, "title").to_lowercase().contains(&q)
                        || str_at(p, "workspaceRoot").to_lowercase().contains(&q)
                })
                .map(|p| {
                    (
                        "import-proj".to_string(),
                        str_at(p, "title").to_string(),
                        str_at(p, "workspaceRoot").to_string(),
                        str_at(p, "key").to_string(),
                    )
                })
                .collect(),
            Some("import-thread") => Vec::new(),
            _ => {
                let snap = self.snapshot.as_ref().unwrap_or(&Value::Null);
                let mut items: Vec<(String, String, String, String)> = Vec::new();
                let project_title = |pid: &str| -> String {
                    arr(snap, "projects")
                        .iter()
                        .find(|p| str_at(p, "id") == pid)
                        .map(|p| str_at(p, "title").to_string())
                        .filter(|t| !t.is_empty())
                        .unwrap_or_else(|| "Unknown project".to_string())
                };
                for t in arr(snap, "threads") {
                    if !(get(t, "deletedAt").is_null() && get(t, "archivedAt").is_null()) {
                        continue;
                    }
                    let title = str_at(t, "title").to_string();
                    if !q.is_empty()
                        && !title.to_lowercase().contains(&q)
                        && !str_at(t, "latestReplyPreview").to_lowercase().contains(&q)
                    {
                        continue;
                    }
                    items.push((
                        "thread".to_string(),
                        title,
                        project_title(str_at(t, "projectId")),
                        str_at(t, "id").to_string(),
                    ));
                }
                // Upstream Sidebar searchPaletteActions + navigation actions,
                // in order (Sidebar.tsx:7590-7710, 8036-8060).
                let actions: [(&str, &str, &str); 9] = [
                    ("new-chat", "New chat", "\u{2325}\u{2318}N"),
                    ("new-thread", "New thread", "\u{2318}N"),
                    ("add-project", "Add project", "\u{21e7}\u{2318}O"),
                    ("import-projects", "Import projects from\u{2026}", ""),
                    ("import-thread", "Import thread from\u{2026}", "\u{2318}I"),
                    ("feedback", "Feedback", ""),
                    ("settings", "Settings", ""),
                    ("usage-settings", "Usage", "\u{21e7}\u{2318}U"),
                    ("new-space", "New space", ""),
                ];
                for (id, label, kbd) in actions {
                    if q.is_empty() || label.to_lowercase().contains(&q) {
                        items.push((
                            "action".to_string(),
                            label.to_string(),
                            kbd.to_string(),
                            id.to_string(),
                        ));
                    }
                }
                let navs: [(&str, &str, &str); 6] = [
                    ("go-inbox", "Go to Inbox", ""),
                    ("go-kanban", "Go to Kanban", ""),
                    ("go-studio", "Go to Studio", ""),
                    ("go-automations", "Go to Automations", ""),
                    ("toggle-computer", "Toggle computer", ""),
                    ("toggle-browser", "Toggle browser", "\u{21e7}\u{2318}B"),
                ];
                for (id, label, kbd) in navs {
                    if q.is_empty() || label.to_lowercase().contains(&q) {
                        items.push((
                            "action".to_string(),
                            label.to_string(),
                            kbd.to_string(),
                            id.to_string(),
                        ));
                    }
                }
                if !arr(snap, "projects").is_empty()
                    && (q.is_empty() || "new automation".contains(&q))
                {
                    items.push((
                        "action".to_string(),
                        "New automation".to_string(),
                        String::new(),
                        "new-automation".to_string(),
                    ));
                }
                for p in arr(snap, "projects") {
                    let title = str_at(p, "title").to_string();
                    if !q.is_empty() && !title.to_lowercase().contains(&q) {
                        continue;
                    }
                    items.push((
                        "project".to_string(),
                        title,
                        str_at(p, "kind").to_string(),
                        str_at(p, "id").to_string(),
                    ));
                }
                items.truncate(30);
                items
            }
        }
    }

    /// Activate the highlighted row: file/snippet rows open the file viewer,
    /// directories open the explorer there, projects open their workspace
    /// explorer, threads open their transcript.
    pub(super) fn palette_activate(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let query = self
            .palette_input
            .as_ref()
            .map(|i| i.read(cx).value().to_string())
            .unwrap_or_default();
        let items = self.palette_items(&query);
        let Some((kind, _label, _sub, payload)) = items.get(self.palette_sel).cloned() else {
            // import-thread has no rows: Enter submits the typed session id.
            if self.palette_mode == Some("import-thread") && !query.trim().is_empty() {
                let ext = query.trim().to_string();
                self.close_palette(window, cx);
                self.start_import_thread(ext, cx);
            } else {
                self.close_palette(window, cx);
            }
            return;
        };
        // Sub-page transitions keep the palette open (upstream mode switch).
        match (kind.as_str(), payload.as_str()) {
            ("action", "import-projects") => {
                self.set_palette_mode("import-sources", window, cx);
                return;
            }
            ("action", "import-thread") => {
                self.set_palette_mode("import-thread", window, cx);
                return;
            }
            ("import-src", _) => {
                // Upstream openProjectImportDialog: pick the provider set
                // ("all" = Codex + Claude Code), close the palette and open
                // the full dialog with an immediate scan.
                let providers = if payload == "all" {
                    vec!["codex".to_string(), "claudeAgent".to_string()]
                } else {
                    vec![payload.clone()]
                };
                self.close_palette(window, cx);
                self.open_import_dialog(providers, window, cx);
                return;
            }
            ("import-proj", _) => {
                self.palette_run_import_project(payload.clone(), cx);
                return;
            }
            _ => {}
        }
        let cwd = self.files_root().unwrap_or_default();
        self.close_palette(window, cx);
        match kind.as_str() {
            "directory" => {
                self.clear_views();
                self.view_files = true;
                self.load_dir(format!("{cwd}/{payload}"), cx);
            }
            "file" | "snippet" => {
                self.clear_views();
                self.view_files = true;
                self.open_entry(
                    json!({"kind": "file", "path": format!("{cwd}/{payload}")}),
                    cx,
                );
            }
            "project" => {
                self.clear_views();
                self.view_files = true;
                let root = arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects")
                    .iter()
                    .find(|p| str_at(p, "id") == payload)
                    .map(|p| {
                        let r = str_at(p, "workspaceRoot");
                        if r.is_empty() {
                            str_at(p, "workspace_root").to_string()
                        } else {
                            r.to_string()
                        }
                    })
                    .filter(|r: &String| !r.is_empty());
                if let Some(root) = root {
                    self.load_dir(root, cx);
                }
            }
            "thread" => self.open_thread(payload, cx),
            "action" => match payload.as_str() {
                "new-chat" | "new-thread" => self.new_thread(cx),
                "add-project" => self.start_add_project(window, cx),
                "feedback" => self.open_feedback(window, cx),
                "settings" => {
                    self.clear_views();
                    self.view_settings = true;
                    if self.settings_data.is_null() {
                        self.fetch_settings(cx);
                    }
                }
                "usage-settings" => {
                    self.clear_views();
                    self.view_settings = true;
                    self.settings_section = "usage";
                    if self.settings_data.is_null() {
                        self.fetch_settings(cx);
                    }
                }
                "new-space" => {
                    self.clear_views();
                    self.view_spaces = true;
                }
                "go-inbox" => {
                    self.clear_views();
                    self.view_inbox = true;
                    self.fetch_inbox(cx);
                }
                "toggle-computer" => self.toggle_cua(window, cx),
                "toggle-browser" => self.toggle_browser(window, cx),
                "go-kanban" => self.open_route(2, cx),
                "go-studio" => self.open_route(5, cx),
                "go-automations" => self.open_route(4, cx),
                "new-automation" => {
                    self.open_route(4, cx);
                    self.open_autom_dialog(None, window, cx);
                }
                _ => {}
            },
            _ => {}
        }
        cx.notify();
    }

    /// Switch palette page while open (upstream mode switch): new input with
    /// the page's placeholder, rows reset.
    pub(super) fn set_palette_mode(
        &mut self,
        mode: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.palette_mode = Some(mode);
        self.palette_rows = Vec::new();
        self.palette_sel = 0;
        self.palette_seq += 1;
        let placeholder = match mode {
            "files" => "Search files",
            "snippets" => "Search code",
            "import-sources" | "import-projs" => "Import projects from\u{2026}",
            "import-thread" => "Import thread from\u{2026}",
            _ => "Search chats or run a command",
        };
        let input = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
        cx.subscribe_in(&input, window, |this: &mut Self, _s, ev, _w, cx| match ev {
            InputEvent::Change => this.palette_query_changed(cx),
            InputEvent::PressEnter { .. } => this.palette_activate(_w, cx),
            _ => {}
        })
        .detach();
        window.defer(cx, {
            let input = input.clone();
            move |window, cx| {
                input.update(cx, |s, cx| s.focus_handle(cx).focus(window, cx));
            }
        });
        self.palette_input = Some(input);
        cx.notify();
    }

    /// Palette second page → orchestration.listProjectImports for the picked
    /// provider source; results list in the palette (kind "import-proj").
    pub(super) fn palette_fetch_imports(&mut self, source: String, cx: &mut Context<Self>) {
        let providers: Vec<&str> = match source.as_str() {
            "all" => vec!["claudeAgent", "codex"],
            s => vec![match s {
                "claudeAgent" => "claudeAgent",
                _ => "codex",
            }],
        };
        self.palette_mode = Some("import-projs");
        self.palette_rows = Vec::new();
        self.palette_sel = 0;
        self.palette_seq += 1;
        let seq = self.palette_seq;
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let v = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell).and_then(|c| {
                        c.call(
                            "orchestration.listProjectImports",
                            json!({"providers": providers}),
                        )
                    })
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if this.palette_seq != seq || this.palette_mode != Some("import-projs") {
                    return;
                }
                this.palette_rows = v.map(|v| arr(&v, "projects").to_vec()).unwrap_or_default();
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Import the picked catalog project (orchestration.importProject with
    /// its projectKey), then refresh the shell snapshot.
    pub(super) fn palette_run_import_project(&mut self, key: String, cx: &mut Context<Self>) {
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let r = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell).and_then(|c| {
                        c.call("orchestration.importProject", json!({"projectKey": key}))
                    })
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                match r {
                    Some(_) => {
                        this.palette_mode = None;
                        this.palette_input = None;
                        this.feedback_toast = Some(("Project imported.".to_string(), false));
                    }
                    None => {
                        this.feedback_toast =
                            Some(("Failed to import the project.".to_string(), true));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Debounced content search via orchestration.searchThreads.
    pub(super) fn on_search_change(&mut self, cx: &mut Context<Self>) {
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
}
