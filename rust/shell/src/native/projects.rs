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
    /// Upstream toggleProjectPinned: flips isPinned via project.meta.update.
    pub(super) fn toggle_project_pin(&mut self, project_id: String, pinned: bool, cx: &mut Context<Self>) {
        self.dispatch_cmd(
            json!({
                "type": "project.meta.update",
                "commandId": uuid(),
                "projectId": project_id,
                "isPinned": !pinned,
            }),
            cx,
        );
    }

    /// Upstream archiveAllThreadsInProject / deleteProjectThreads: one
    /// command per live thread filed under the project.
    pub(super) fn project_threads_cmd(&mut self, project_id: String, kind: &str, cx: &mut Context<Self>) {
        let Some(snapshot) = self.snapshot.as_ref() else {
            return;
        };
        let ids: Vec<String> = arr(snapshot, "threads")
            .iter()
            .filter(|t| {
                str_at(t, "projectId") == project_id
                    && get(t, "deletedAt").is_null()
                    && get(t, "archivedAt").is_null()
            })
            .map(|t| str_at(t, "id").to_string())
            .collect();
        for tid in ids {
            self.dispatch_cmd(
                json!({
                    "type": kind,
                    "commandId": uuid(),
                    "threadId": tid,
                }),
                cx,
            );
        }
    }

    /// Upstream removeProject: threads are deleted first (project.delete
    /// refuses non-empty folders), then the project row itself.
    pub(super) fn delete_project(&mut self, project_id: String, cx: &mut Context<Self>) {
        self.project_threads_cmd(project_id.clone(), "thread.delete", cx);
        self.dispatch_cmd(
            json!({
                "type": "project.delete",
                "commandId": uuid(),
                "projectId": project_id,
            }),
            cx,
        );
    }

    /// Shared RenameDialog for projects (the "Edit project" menu action).
    pub(super) fn start_project_rename(
        &mut self,
        project_id: String,
        current: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Project name"));
        input.update(cx, |s, cx| {
            s.set_value(current, window, cx);
        });
        cx.subscribe_in(&input, window, |this: &mut Self, _s, event, _w, cx| {
            if let InputEvent::PressEnter { .. } = event {
                this.commit_project_rename(cx);
            }
        })
        .detach();
        let focus = input.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        self.proj_rename_input = Some(input);
        self.proj_rename = Some(project_id);
        cx.notify();
    }

    pub(super) fn commit_project_rename(&mut self, cx: &mut Context<Self>) {
        let Some(input) = self.proj_rename_input.take() else {
            return;
        };
        let title = input.read(cx).value().trim().to_string();
        let Some(pid) = self.proj_rename.take() else {
            return;
        };
        if title.is_empty() {
            cx.notify();
            return;
        }
        self.dispatch_cmd(
            json!({
                "type": "project.meta.update",
                "commandId": uuid(),
                "projectId": pid,
                "title": title,
            }),
            cx,
        );
        cx.notify();
    }

    /// Upstream RelocateProjectDialog: text input prefilled with the
    /// current root; commits `project.meta.update` with the new
    /// `workspaceRoot` (no implicit mkdir, like upstream).
    pub(super) fn start_project_relocate(
        &mut self,
        project_id: String,
        current_root: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("/path/to/project"));
        input.update(cx, |s, cx| {
            s.set_value(current_root, window, cx);
        });
        cx.subscribe_in(&input, window, |this: &mut Self, _s, event, _w, cx| {
            if let InputEvent::PressEnter { .. } = event {
                this.commit_project_relocate(cx);
            }
        })
        .detach();
        let focus = input.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        self.proj_relocate_input = Some(input);
        cx.notify();
    }

    pub(super) fn commit_project_relocate(&mut self, cx: &mut Context<Self>) {
        let Some(input) = self.proj_relocate_input.take() else {
            return;
        };
        let root = input.read(cx).value().trim().to_string();
        let Some((pid, prev_root)) = self.proj_relocate.take() else {
            return;
        };
        if root.is_empty() || root == prev_root {
            cx.notify();
            return;
        }
        self.dispatch_cmd(
            json!({
                "type": "project.meta.update",
                "commandId": uuid(),
                "projectId": pid,
                "workspaceRoot": root,
                "createWorkspaceRootIfMissing": false,
            }),
            cx,
        );
        cx.notify();
    }

    /// Add project (upstream CreateProjectDialog entry): native folder
    /// picker, then `project.create` with the folder name as title.
    pub(super) fn start_add_project(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: None,
        });
        let client = self.client.clone();
        cx.spawn_in(window, async move |this: WeakEntity<Self>, cx: &mut AsyncWindowContext| {
            let Ok(Ok(Some(paths))) = rx.await else { return };
            let Some(dir) = paths.first().cloned() else { return };
            let title = dir
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Project".to_string());
            let root = dir.to_string_lossy().into_owned();
            let cell = client.clone();
            let r = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell).and_then(|c| {
                        c.call(
                            "orchestration.dispatchCommand",
                            json!({
                                "type": "project.create",
                                "commandId": uuid(),
                                "projectId": uuid(),
                                "title": title,
                                "workspaceRoot": root,
                                "kind": "project",
                                "createdAt": now_iso(),
                            }),
                        )
                    })
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if r.is_none() {
                    this.feedback_toast = Some(("Failed to add the project.".to_string(), true));
                }
            });
        })
        .detach();
    }

    /// Open the full Import projects dialog (upstream
    /// useProjectImportDialogStore.openDialog): fresh state, then an
    /// immediate scan for the chosen providers.
    pub(super) fn open_import_dialog(
        &mut self,
        providers: Vec<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let search = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Search projects or folders")
        });
        cx.subscribe_in(&search, window, |_this: &mut Self, _s, ev, _w, cx| {
            if let InputEvent::Change = ev {
                cx.notify();
            }
        })
        .detach();
        self.import_dlg = Some(ImportDlg {
            providers,
            catalog: None,
            scanning: false,
            running: false,
            error: None,
            selected: std::collections::HashSet::new(),
            roots: std::collections::HashMap::new(),
            include_archived: false,
            search,
            outcomes: std::collections::HashMap::new(),
            progress: None,
            stop: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            expanded: std::collections::HashSet::new(),
            root_inputs: std::collections::HashMap::new(),
        });
        self.import_scan(cx);
        cx.notify();
    }

    /// "Find projects" / "Scan again": orchestration.listProjectImports for
    /// the enabled providers, then preselect every selectable item.
    pub(super) fn import_scan(&mut self, cx: &mut Context<Self>) {
        let Some(d) = self.import_dlg.as_mut() else { return };
        if d.providers.is_empty() {
            return;
        }
        d.scanning = true;
        d.error = None;
        d.catalog = None;
        d.outcomes.clear();
        d.progress = None;
        d.expanded.clear();
        d.root_inputs.clear();
        let providers = d.providers.clone();
        let cell = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
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
                let Some(d) = this.import_dlg.as_mut() else { return };
                d.scanning = false;
                match v {
                    Some(cat) => {
                        let mut sel = std::collections::HashSet::new();
                        for p in arr(&cat, "projects") {
                            for k in import_selectable_keys(p, d.include_archived) {
                                sel.insert(k);
                            }
                        }
                        d.selected = sel;
                        d.catalog = Some(cat);
                    }
                    None => {
                        d.error = Some("Could not find local projects.".to_string());
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// buildProjectImportQueue: selected, selectable items not yet imported
    /// — (item key, display title, importProject input).
    pub(super) fn import_queue(d: &ImportDlg) -> Vec<(String, String, Value)> {
        let mut queue = Vec::new();
        let Some(cat) = d.catalog.as_ref() else { return queue };
        for p in arr(cat, "projects") {
            let pk = str_at(p, "key");
            let ptitle = str_at(p, "title");
            let root_override = d
                .roots
                .get(pk)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            let selectable: std::collections::HashSet<String> =
                import_selectable_keys(p, d.include_archived)
                    .into_iter()
                    .collect();
            let threads = arr(p, "threads");
            if threads.is_empty() {
                let key = import_item_key(pk, None);
                if selectable.contains(&key)
                    && d.selected.contains(&key)
                    && !matches!(d.outcomes.get(&key), Some(None))
                {
                    let mut input =
                        json!({"projectKey": pk, "threadKey": Value::Null, "spaceId": Value::Null});
                    if let Some(r) = &root_override {
                        input["workspaceRoot"] = json!(r);
                    }
                    queue.push((key, ptitle.to_string(), input));
                }
            } else {
                for t in threads {
                    let key = import_item_key(pk, Some(str_at(t, "key")));
                    if !selectable.contains(&key)
                        || !d.selected.contains(&key)
                        || matches!(d.outcomes.get(&key), Some(None))
                    {
                        continue;
                    }
                    let mut input = json!({"projectKey": pk,
                        "threadKey": str_at(t, "key"), "spaceId": Value::Null});
                    if let Some(r) = &root_override {
                        input["workspaceRoot"] = json!(r);
                    }
                    let ttitle = str_at(t, "title");
                    let label = if ttitle.is_empty() {
                        format!("{ptitle} · Untitled conversation")
                    } else {
                        format!("{ptitle} · {ttitle}")
                    };
                    queue.push((key, label, input));
                }
            }
        }
        queue
    }

    /// "Import selected" / "Retry failed": sequential
    /// orchestration.importProject per queued item, then a snapshot sync.
    pub(super) fn import_run(&mut self, retry_only: bool, cx: &mut Context<Self>) {
        let Some(d) = self.import_dlg.as_mut() else { return };
        if d.running {
            return;
        }
        let queue: Vec<(String, String, Value)> = Self::import_queue(d)
            .into_iter()
            .filter(|(k, _, _)| {
                !retry_only || matches!(d.outcomes.get(k), Some(Some(_)))
            })
            .collect();
        if queue.is_empty() {
            return;
        }
        d.stop.store(false, std::sync::atomic::Ordering::Relaxed);
        d.running = true;
        d.error = None;
        let stop = d.stop.clone();
        let total = queue.len();
        let cell = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            for (i, (key, title, input)) in queue.into_iter().enumerate() {
                if stop.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }
                let _ = this.update(cx, |this: &mut Self, cx| {
                    if let Some(d) = this.import_dlg.as_mut() {
                        d.progress = Some((i + 1, total, title.clone()));
                        cx.notify();
                    }
                });
                let cell2 = cell.clone();
                let res = cx
                    .background_executor()
                    .spawn(async move {
                        ensure_client(&cell2)
                            .and_then(|c| c.call("orchestration.importProject", input))
                    })
                    .await;
                let _ = this.update(cx, |this: &mut Self, cx| {
                    if let Some(d) = this.import_dlg.as_mut() {
                        d.outcomes.insert(
                            key,
                            match res {
                                Some(_) => None,
                                None => Some("Import failed. Try again.".to_string()),
                            },
                        );
                        cx.notify();
                    }
                });
            }
            let cell3 = cell.clone();
            let snap = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell3).and_then(|c| {
                        c.call("orchestration.getShellSnapshot", json!({}))
                    })
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if let Some(d) = this.import_dlg.as_mut() {
                    d.running = false;
                }
                if let Some(s) = snap {
                    this.snapshot = Some(s);
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Import thread (upstream onImportThread): create a thread in the
    /// current project, then orchestration.importThread with the pasted
    /// session id; the thread is deleted when the import fails.
    pub(super) fn start_import_thread(&mut self, external_id: String, cx: &mut Context<Self>) {
        let Some(project) = arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects")
            .first()
            .cloned()
        else {
            self.feedback_toast =
                Some(("Add a project before importing a thread.".to_string(), true));
            return;
        };
        let thread_id = uuid();
        let title = "Imported thread";
        let cmd = json!({
            "type": "thread.create",
            "commandId": uuid(),
            "threadId": thread_id,
            "projectId": str_at(&project, "id"),
            "title": title,
            "runtimeMode": "full-access",
            "interactionMode": "default",
            "envMode": "local",
            "branch": Value::Null,
            "worktreePath": Value::Null,
            "createdAt": now_iso(),
        });
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let created = cx
                .background_executor()
                .spawn({
                    let cell = cell.clone();
                    let cmd = cmd.clone();
                    async move { ensure_client(&cell).and_then(|c| c.call("orchestration.dispatchCommand", cmd)) }
                })
                .await;
            let imported = if created.is_some() {
                let cell = cell.clone();
                let tid = thread_id.clone();
                let ext = external_id.clone();
                cx.background_executor()
                    .spawn(async move {
                        ensure_client(&cell).and_then(|c| {
                            c.call(
                                "orchestration.importThread",
                                json!({"threadId": tid, "externalId": ext}),
                            )
                        })
                    })
                    .await
            } else {
                None
            };
            if created.is_some() && imported.is_none() {
                let cell = cell.clone();
                let tid = thread_id.clone();
                let _ = cx
                    .background_executor()
                    .spawn(async move {
                        ensure_client(&cell).map(|c| {
                            c.call(
                                "orchestration.dispatchCommand",
                                json!({"type": "thread.delete", "commandId": uuid(), "threadId": tid}),
                            )
                        })
                    })
                    .await;
            }
            let _ = this.update(cx, |this: &mut Self, cx| {
                if imported.is_some() {
                    this.open_thread(thread_id.clone(), cx);
                } else {
                    this.feedback_toast =
                        Some(("Failed to import the thread.".to_string(), true));
                }
            });
        })
        .detach();
    }
}
