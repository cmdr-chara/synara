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
    /// Workspace root of the first project that still exists on disk — the
    /// file browser starts there. Seeded/stale projects are skipped.
    pub(super) fn files_root(&self) -> Option<String> {
        for p in arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects") {
            let r = str_at(p, "workspaceRoot");
            let root = if r.is_empty() {
                str_at(p, "workspace_root")
            } else {
                r
            };
            if !root.is_empty() && std::path::Path::new(root).is_dir() {
                return Some(root.to_string());
            }
        }
        None
    }

    pub(super) fn toggle_files(&mut self, cx: &mut Context<Self>) {
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

    pub(super) fn toggle_dev(&mut self, cx: &mut Context<Self>) {
        self.view_dev = !self.view_dev;
        if self.view_dev {
            self.view_files = false;
            self.view_inbox = false;
            self.fetch_dev_servers(cx);
        }
        cx.notify();
    }

    /// Start a dev server for the first project with the typed command.
    pub(super) fn dev_run(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
    pub(super) fn dev_stop(&mut self, project_id: String, cx: &mut Context<Self>) {
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

    /// `projects.listDirectories` into the mention browser state (kept
    /// separate from the file-browser pane's `files`).
    pub(super) fn load_mention_dir(&mut self, dir: String, cx: &mut Context<Self>) {
        self.mention_seq += 1;
        let seq = self.mention_seq;
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
                if seq != this.mention_seq {
                    return;
                }
                if let Some(v) = entries {
                    let mut es = arr(&v, "entries").to_vec();
                    es.sort_by_key(|e| {
                        if str_at(e, "kind") == "directory" {
                            0
                        } else {
                            1
                        }
                    });
                    this.mention_entries = es;
                    this.mention_dir = dir;
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Files-pane fuzzy search: 120ms debounce into `projects.searchEntries`
    /// (upstream useWorkspaceFileSearch), limit 80, files only.
    pub(super) fn on_file_search_change(&mut self, cx: &mut Context<Self>) {
        self.file_search_seq += 1;
        let seq = self.file_search_seq;
        let query = self.file_search.read(cx).value().trim().to_string();
        if query.is_empty() {
            self.file_search_results.clear();
            cx.notify();
            return;
        }
        let dir = self.files_dir.clone();
        let cell = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(120))
                .await;
            if this
                .update(cx, |this: &mut Self, _cx| seq != this.file_search_seq)
                .unwrap_or(true)
            {
                return;
            }
            let q = query.clone();
            let d = dir.clone();
            let res = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell).and_then(|c| {
                        c.call(
                            "projects.searchEntries",
                            json!({"cwd": d, "query": q, "limit": 80, "kind": "file"}),
                        )
                    })
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if seq != this.file_search_seq {
                    return;
                }
                this.file_search_results = res
                    .map(|v| arr(&v, "entries").to_vec())
                    .unwrap_or_default();
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn load_dir(&mut self, dir: String, cx: &mut Context<Self>) {
        self.files_loading = true;
        self.open_file = None;
        self.tree_entries.clear();
        self.expanded_dirs.clear();
        self.tree_loading.clear();
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
    pub(super) fn open_entry(&mut self, entry: Value, cx: &mut Context<Self>) {
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

    /// Expand/collapse a directory row in the explorer tree (upstream
    /// onToggleDirectory). The first expand lazily fetches the listing;
    /// later toggles reuse the cached `tree_entries`.
    pub(super) fn toggle_tree_dir(&mut self, path: String, cx: &mut Context<Self>) {
        if !self.expanded_dirs.remove(&path) {
            self.expanded_dirs.insert(path.clone());
            if !self.tree_entries.contains_key(&path) {
                self.load_tree_dir(path, cx);
            }
        }
        cx.notify();
    }

    /// Lazy-load one directory level for the explorer tree (upstream
    /// WorkspaceDirectory → projects.listDirectories).
    pub(super) fn load_tree_dir(&mut self, dir: String, cx: &mut Context<Self>) {
        if !self.tree_loading.insert(dir.clone()) {
            return;
        }
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
                this.tree_loading.remove(&dir);
                if let Some(v) = entries {
                    let mut es = arr(&v, "entries").to_vec();
                    es.sort_by_key(|e| {
                        if str_at(e, "kind") == "directory" {
                            0
                        } else {
                            1
                        }
                    });
                    this.tree_entries.insert(dir, es);
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Rows of one level of the explorer tree (upstream WorkspaceDirectory +
    /// ExplorerRow): h-7 rows indented 8+12·depth px, disclosure chevron on
    /// directories, selected block on the open file, lazy expanded children.
    pub(super) fn file_tree_rows(
        &self,
        entries: &[Value],
        depth: usize,
        theme: &Theme,
        subtle: Hsla,
        out: &mut Vec<gpui::AnyElement>,
        cx: &mut Context<Self>,
    ) {
        for entry in entries {
            if !explorer_entry_visible(entry) {
                continue;
            }
            let name = str_at(entry, "name").to_string();
            let path = str_at(entry, "path").to_string();
            let is_dir = str_at(entry, "kind") == "directory";
            let expanded = is_dir && self.expanded_dirs.contains(&path);
            let selected = !is_dir
                && self
                    .open_file
                    .as_ref()
                    .map(|(p, _)| p == &path)
                    .unwrap_or(false);
            let indent = px(8. + depth as f32 * 12.);
            let entry_c = entry.clone();
            let path_c = path.clone();
            let mut row = h_flex()
                .id(ElementId::Name(format!("file-{path}").into()))
                .cursor_pointer()
                .h(px(28.))
                .items_center()
                .gap_1p5()
                .pl(indent)
                .pr_2()
                .rounded_md()
                .child(
                    svg_icon(if is_dir {
                        if expanded {
                            icons::CHEVRON_DOWN
                        } else {
                            icons::CHEVRON_RIGHT
                        }
                    } else {
                        icons::FILE_TEXT
                    })
                    .size(px(14.))
                    .text_color(theme.muted_foreground),
                )
                .child(
                    div()
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(theme.foreground)
                        .child(name),
                );
            if selected {
                row = row.bg(theme.secondary);
            } else {
                row = row.hover(move |d| d.bg(subtle));
            }
            row = row.on_click(cx.listener(move |this, _ev, _w, cx| {
                if is_dir {
                    this.toggle_tree_dir(path_c.clone(), cx);
                } else {
                    this.open_entry(entry_c.clone(), cx);
                }
            }));
            let path_rc = path.clone();
            row = row.on_mouse_down(
                gpui::MouseButton::Right,
                cx.listener(move |this, ev: &MouseDownEvent, _w, cx| {
                    this.ctx_menu = None;
                    this.file_ctx_menu =
                        Some((path_rc.clone(), ev.position.x.into(), ev.position.y.into()));
                    cx.stop_propagation();
                    cx.notify();
                }),
            );
            // Upstream drag-to-chat: dragging a file onto the composer
            // references it.
            let ghost_name = str_at(entry, "name").to_string();
            let path_drag = path.clone();
            row = row.on_drag(FileDrag(path_drag), move |_d, _pos, _w, cx| {
                cx.new(|_| DragGhost(ghost_name.clone()))
            });
            out.push(row.into_any_element());
            if expanded {
                if let Some(children) = self.tree_entries.get(&path) {
                    let kids = children.clone();
                    self.file_tree_rows(&kids, depth + 1, theme, subtle, out, cx);
                } else if self.tree_loading.contains(&path) {
                    out.push(
                        div()
                            .pl(px(8. + (depth + 1) as f32 * 12.))
                            .py(px(4.))
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child("loading\u{2026}")
                            .into_any_element(),
                    );
                }
            }
        }
    }

    /// Subscribe to change events for the currently open file; each event
    /// triggers a re-read so the viewer stays live.
    pub(super) fn watch_open_file(&mut self, cx: &mut Context<Self>) {
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

    pub(super) fn stop_file_watch(&mut self) {
        if let Some(old) = self.file_stream_id.take() {
            if let Ok(g) = self.client.lock() {
                if let Some(c) = g.as_ref() {
                    c.interrupt(&old);
                }
            }
        }
    }

    pub(super) fn close_file(&mut self, cx: &mut Context<Self>) {
        self.file_dirty = false;
        self.open_in_menu = false;
        self.stop_file_watch();
        self.open_file = None;
        self.open_file_meta = None;
        self.file_editor = None;
        self.editor_synced = None;
        self.editing_file = false;
        cx.notify();
    }

    /// Map a file path to an editor language id for highlighting.
    pub(super) fn editor_language(path: &str) -> &'static str {
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
    pub(super) fn save_file(&mut self, cx: &mut Context<Self>) {
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
                    this.file_dirty = false;
                }
                cx.notify();
            });
        })
        .detach();
    }
}
