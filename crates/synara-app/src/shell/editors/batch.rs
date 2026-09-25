//! Explicit workspace-wide editor commands. No agent or shell execution.
use super::*;
mod refresh;
use std::sync::atomic::{AtomicBool, Ordering};

pub(super) const CLOSED_LIMIT: usize = 8;
pub(super) struct SaveProgress {
    pub(super) completed: usize,
    pub(super) total: usize,
    stop: Arc<AtomicBool>,
}
impl Drop for SaveProgress {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}
struct SaveItem {
    id: u64,
    document: Document,
    text: String,
}
enum SaveHost {
    Local(PathBuf),
    Remote(synara_runtime::RemoteWorkspaceFs),
}
enum Reply {
    Saved(u64, PathBuf, String, synara_runtime::FileVersion),
    Failed(String),
}
impl Shell {
    pub(super) fn toggle_editor_comparison(&mut self, cx: &mut Context<Self>) {
        if self.editors.compare.open {
            self.editors.compare.clear();
            cx.notify();
            return;
        }
        if self.document.is_none() || self.editor_batch_blocked(cx) {
            return;
        }
        let Some(owner) = self.editor_compare_owner() else {
            return;
        };
        if self.editors.compare_ref_root.as_ref() != Some(&owner.root) {
            let reference = self
                .editors
                .compare_refs
                .get(&owner.root)
                .cloned()
                .unwrap_or_else(|| "HEAD".into());
            self.editors.compare_ref_root = Some(owner.root.clone());
            self.editors
                .compare_ref
                .update(cx, |input, cx| input.set_text(reference, cx));
        }
        self.editors.history.clear();
        self.editors.preview = false;
        self.editors.compare.open = true;
        self.editors.compare.owner = Some(owner);
        self.editors.compare.error = None;
        cx.notify();
    }

    fn begin_editor_comparison_request(
        &mut self,
        mode: CompareMode,
        label: &str,
        cx: &mut Context<Self>,
    ) -> Option<(CompareOwner, u64, tokio_util::sync::CancellationToken)> {
        if self.editor_batch_blocked(cx) {
            return None;
        }
        let owner = self.editor_compare_owner()?;
        let compare = &mut self.editors.compare;
        compare.cancel.cancel();
        compare.cancel = Default::default();
        compare.generation = compare.generation.wrapping_add(1);
        compare.open = true;
        compare.pending = true;
        compare.owner = Some(owner.clone());
        compare.mode = Some(mode);
        compare.label = Some(label.into());
        compare.reference = None;
        compare.disk_version = None;
        compare.buffer_at_read = None;
        compare.diff = CompareDiff::default();
        compare.error = None;
        compare.restore_all_confirmed = false;
        compare.selected_change = None;
        let request = (owner, compare.generation, compare.cancel.clone());
        self.editors.history.clear();
        self.editors.preview = false;
        cx.notify();
        Some(request)
    }

    fn set_editor_comparison_snapshot(
        &mut self,
        mode: CompareMode,
        label: String,
        reference: String,
        cx: &mut Context<Self>,
    ) {
        if self.editor_batch_blocked(cx) {
            return;
        }
        let Some(owner) = self.editor_compare_owner() else {
            return;
        };
        let current = self.editor.read(cx).text().to_owned();
        let compare = &mut self.editors.compare;
        compare.cancel.cancel();
        compare.cancel = Default::default();
        compare.generation = compare.generation.wrapping_add(1);
        compare.open = true;
        compare.pending = false;
        compare.owner = Some(owner);
        compare.mode = Some(mode);
        compare.label = Some(label);
        compare.diff = compare_text(&reference, &current);
        compare.reference = Some(reference);
        compare.disk_version = None;
        compare.buffer_at_read = Some(current);
        compare.error = None;
        compare.restore_all_confirmed = false;
        compare.selected_change = None;
        self.editors.history.clear();
        self.editors.preview = false;
        cx.notify();
    }

    pub(super) fn compare_editor_saved_buffer(&mut self, cx: &mut Context<Self>) {
        let Some(document) = self.document.clone() else {
            return;
        };
        let label = format!(
            "Saved snapshot {}",
            document
                .snapshot
                .version
                .0
                .chars()
                .take(8)
                .collect::<String>()
        );
        self.set_editor_comparison_snapshot(
            CompareMode::Saved,
            label,
            document.snapshot.text.clone(),
            cx,
        );
    }

    pub(super) fn compare_editor_disk(&mut self, cx: &mut Context<Self>) {
        let Some(target) = self.workspace_target() else {
            return;
        };
        let Some((owner, generation, cancel)) =
            self.begin_editor_comparison_request(CompareMode::Disk, "Current disk", cx)
        else {
            return;
        };
        let workspace = self.controller.workspace.clone();
        cx.spawn(async move |view, cx| {
            let result = tokio::select! {
                biased;
                _ = cancel.cancelled() => return,
                result = async {
                    match target {
                        WorkspaceTarget::Local { root } => open_document(root, owner.path.clone())
                            .await
                            .map(|document| document.snapshot)
                            .map_err(|error| error.to_string()),
                        WorkspaceTarget::Ssh {
                            workspace: remote,
                            root,
                        } => match remote_filesystem(workspace, remote, root).await {
                            Ok(filesystem) => open_remote_document(filesystem, owner.path.clone())
                                .await
                                .map(|document| document.snapshot)
                                .map_err(|error| error.to_string()),
                            Err(error) => Err(error.to_string()),
                        },
                    }
                } => result,
            };
            let _ = view.update(cx, |this, cx| {
                if this.editors.compare.generation != generation
                    || this.editor_compare_owner().as_ref() != Some(&owner)
                    || this.editors.compare.owner.as_ref() != Some(&owner)
                {
                    return;
                }
                this.editors.compare.pending = false;
                match result {
                    Ok(snapshot) => {
                        let label = format!(
                            "Current disk {}",
                            snapshot.version.0.chars().take(8).collect::<String>()
                        );
                        let current = this.editor.read(cx).text().to_owned();
                        this.editors.compare.label = Some(label);
                        this.editors.compare.diff = compare_text(&snapshot.text, &current);
                        this.editors.compare.reference = Some(snapshot.text);
                        this.editors.compare.disk_version = Some(snapshot.version);
                        this.editors.compare.buffer_at_read = Some(current);
                    }
                    Err(error) => this.editors.compare.error = Some(error),
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn compare_editor_git_ref(&mut self, cx: &mut Context<Self>) {
        let Some(owner) = self.editor_compare_owner() else {
            return;
        };
        let reference = self.editors.compare_ref.read(cx).text().trim().to_owned();
        let reference = if reference.is_empty() {
            "HEAD".to_owned()
        } else {
            reference
        };
        self.editors
            .compare_refs
            .insert(owner.root.clone(), reference.clone());
        self.editors.compare_ref_root = Some(owner.root.clone());
        let Some((owner, generation, cancel)) = self.begin_editor_comparison_request(
            CompareMode::GitRef,
            &format!("Reference {reference}"),
            cx,
        ) else {
            return;
        };
        if !matches!(self.workspace_target(), Some(WorkspaceTarget::Local { .. })) {
            if self.editors.compare.generation == generation {
                self.editors.compare.pending = false;
            }
            self.editors.compare.error =
                Some("Git reference comparison currently requires a local repository.".into());
            cx.notify();
            return;
        }
        let root = owner.root.clone();
        cx.spawn(async move |view, cx| {
            let result = GitService::new(root)
                .file_at_ref(owner.path.clone(), reference.clone(), &cancel)
                .await
                .map(|revision| {
                    let short = revision.commit.chars().take(8).collect::<String>();
                    (format!("{reference} ({short})"), revision.text)
                })
                .map_err(|error| error.to_string());
            let _ = view.update(cx, |this, cx| {
                if this.editors.compare.generation != generation
                    || this.editor_compare_owner().as_ref() != Some(&owner)
                    || this.editors.compare.owner.as_ref() != Some(&owner)
                {
                    return;
                }
                this.editors.compare.pending = false;
                match result {
                    Ok((label, text)) => {
                        let current = this.editor.read(cx).text().to_owned();
                        this.editors.compare.label = Some(format!("Git ref {label}"));
                        this.editors.compare.diff = compare_text(&text, &current);
                        this.editors.compare.reference = Some(text);
                        this.editors.compare.disk_version = None;
                        this.editors.compare.buffer_at_read = Some(current);
                    }
                    Err(error) => this.editors.compare.error = Some(error),
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn merge_conflicted_editor(&mut self, cx: &mut Context<Self>) {
        if !self.conflict_merge_ready(cx) {
            return;
        }
        let (Some(document), Some(disk), Some(disk_version), Some(buffer_at_read)) = (
            self.document.clone(),
            self.editors.compare.reference.clone(),
            self.editors.compare.disk_version.clone(),
            self.editors.compare.buffer_at_read.clone(),
        ) else {
            return;
        };
        let local = self.editor.read(cx).text().to_owned();
        if local != buffer_at_read || self.editor.read(cx).is_composing() {
            self.notice = Some(
                "The editor buffer changed after the disk comparison. Compare disk again before merging; your edits are intact.".into(),
            );
            cx.notify();
            return;
        }

        match three_way_merge(&document.snapshot.text, &local, &disk) {
            Err(MergeLimit::Bytes) => {
                self.notice = Some(
                    "The combined text exceeds the safe merge size. Your buffer was kept; use the disk comparison and resolve this file manually.".into(),
                );
            }
            Err(MergeLimit::Lines) => {
                self.notice = Some(
                    "The file exceeds the safe line limit for automatic merging. Your buffer was kept; use the disk comparison and resolve this file manually.".into(),
                );
            }
            Err(MergeLimit::Cells) => {
                self.notice = Some(
                    "The changes exceed the safe merge limit. Your buffer was kept; use the disk comparison and resolve this file manually.".into(),
                );
            }
            Ok(merge) if merge.text.is_none() => {
                self.notice = Some(format!(
                    "The three-way merge found {} overlapping edit pairs. Your buffer was kept. Review the disk comparison, edit the buffer manually, then choose an explicit recovery action.",
                    merge.conflicts
                ));
            }
            Ok(merge) => {
                let Some(merged) = merge.text else {
                    unreachable!("the overlap case returned above")
                };
                let mut rebased = document;
                rebased.snapshot.text = disk;
                rebased.snapshot.version = disk_version;
                self.document = Some(rebased);
                self.sync_editor_document();
                self.editor
                    .update(cx, |input, cx| input.set_text(merged.clone(), cx));
                self.editors.compare.clear();
                self.clear_conflict_reload_confirmation();
                self.error = None;
                self.notice = Some(format!(
                    "Applied {} local and {} disk change groups without writing the file. Review the merged buffer, then save it explicitly; a newer disk change will still be detected.",
                    merge.local_edits, merge.disk_edits
                ));
            }
        }
        cx.notify();
    }

    pub(super) fn editor_batch_blocked(&self, cx: &App) -> bool {
        self.saving
            || self.close != CloseState::Open
            || self.explorer.modal_open()
            || self
                .editors
                .tabs
                .iter()
                .any(|tab| tab.input.read(cx).is_composing())
    }
    pub(super) fn save_all_editors(&mut self, cx: &mut Context<Self>) {
        if self.editor_batch_blocked(cx) {
            return;
        }
        self.sync_editor_document();
        let Some(target) = self.workspace_target() else {
            return;
        };
        let items: Vec<_> = self
            .editors
            .tabs
            .iter()
            .filter(|tab| tab.dirty(cx))
            .map(|tab| SaveItem {
                id: tab.id,
                document: tab.document.clone(),
                text: tab.input.read(cx).text().to_owned(),
            })
            .collect();
        if items.is_empty() {
            self.notice = Some("All open files are already saved.".into());
            cx.notify();
            return;
        }
        let total = items.len();
        let stop = Arc::new(AtomicBool::new(false));
        self.editors.save_all = Some(SaveProgress {
            completed: 0,
            total,
            stop: stop.clone(),
        });
        self.editors.save_epoch = self.editors.save_epoch.wrapping_add(1);
        let epoch = self.editors.save_epoch;
        let project = self.project;
        let root = target.root().clone();
        self.saving = true;
        self.clear_conflict_reload_confirmation();
        self.error = None;
        self.notice = Some(format!("Saving {total} open files..."));
        let workspace = self.controller.workspace.clone();
        let (sender, receiver) = async_channel::bounded(1);
        self.runtime.spawn(async move {
            let host = match target {
                WorkspaceTarget::Local { root } => Ok(SaveHost::Local(root)),
                WorkspaceTarget::Ssh {
                    workspace: remote,
                    root,
                } => remote_filesystem(workspace, remote, root)
                    .await
                    .map(SaveHost::Remote),
            };
            let host = match host {
                Ok(host) => host,
                Err(error) => {
                    let _ = sender
                        .send(Reply::Failed(format!(
                            "Save all could not access the workspace: {error}"
                        )))
                        .await;
                    return;
                }
            };
            for item in items {
                if stop.load(Ordering::SeqCst) {
                    break;
                }
                let path = item.document.path.clone();
                let result = match &host {
                    SaveHost::Local(root) => {
                        save_document(root.clone(), item.document, item.text.clone()).await
                    }
                    SaveHost::Remote(fs) => {
                        save_remote_document(fs.clone(), item.document, item.text.clone()).await
                    }
                };
                let reply = match result {
                    Ok(version) => Reply::Saved(item.id, path, item.text, version),
                    Err(error) => {
                        let _ = sender
                            .send(Reply::Failed(format!(
                                "Save all stopped at {}: {error}. Later files were not written.",
                                path.display()
                            )))
                            .await;
                        break;
                    }
                };
                if sender.send(reply).await.is_err() {
                    break;
                }
            }
        });
        cx.spawn(async move |view, cx| {
            let mut failed = false;
            while let Ok(reply) = receiver.recv().await {
                if view.update(cx, |this, cx| {
                    if this.editors.save_epoch != epoch { return false; }
                    if this.project != project || this.root().as_ref() != Some(&root) {
                        failed = true;
                        if let Some(progress) = &this.editors.save_all { progress.stop.store(true, Ordering::SeqCst); }
                        this.error = Some("Workspace changed during Save all. The original snapshot was not applied to the new workspace.".into());
                        cx.notify();
                        return true;
                    }
                    match reply {
                        Reply::Saved(id, path, text, version) => {
                            if let Some(tab) = this.editors.tabs.iter_mut().find(|tab| tab.id == id && tab.document.path == path) {
                                // Only the disk baseline advances. Edits typed during
                                // this write remain in the input and stay dirty.
                                tab.document.snapshot.text = text;
                                tab.document.snapshot.version = version;
                                if this.editors.active == Some(id) { this.document = Some(tab.document.clone()); }
                            }
                            if let Some(progress) = &mut this.editors.save_all { progress.completed += 1; }
                        }
                        Reply::Failed(error) => { failed = true; this.error = Some(error); }
                    }
                    cx.notify(); true
                }).ok() != Some(true) { return; }
            }
            let _ = view.update(cx, |this, cx| {
                if this.editors.save_epoch != epoch { return; }
                this.saving = false;
                let Some(progress) = this.editors.save_all.take() else { return };
                let stopped = progress.stop.load(Ordering::SeqCst);
                let dirty = this.editors.tabs.iter().filter(|tab| tab.dirty(cx)).count();
                this.notice = Some(format!("Saved {} of {} files{}. {dirty} open files have unsaved text.",
                    progress.completed, progress.total, if stopped { " (stopped between files)" } else { "" }));
                let clean = !failed && !stopped && progress.completed == progress.total && !this.dirty(cx);
                if this.close.saved(clean) { this.begin_quit(cx); }
                cx.notify();
            });
        }).detach();
        cx.notify();
    }
    pub(super) fn stop_editor_saves(&mut self, cx: &mut Context<Self>) {
        if let Some(progress) = &self.editors.save_all {
            progress.stop.store(true, Ordering::SeqCst);
        }
        self.notice = Some(
            "Stopping after the current atomic file save. Already saved files are not rolled back."
                .into(),
        );
        cx.notify();
    }
    pub(super) fn retain_closed_editor(&mut self, tab: EditorTab, cx: &App) {
        self.editors.autosave.forget(tab.id);
        if tab.dirty(cx) || tab.input.read(cx).is_composing() {
            return;
        }
        self.editors
            .closed
            .retain(|old| old.document.path != tab.document.path);
        self.editors.closed.push(tab);
        if self.editors.closed.len() > CLOSED_LIMIT {
            self.editors.closed.remove(0);
        }
    }
    pub(super) fn close_saved_editors(&mut self, others: bool, cx: &mut Context<Self>) {
        if self.editor_batch_blocked(cx) || self.editors.close.is_some() {
            return;
        }
        self.sync_editor_document();
        let active = self.editors.active;
        let mut count = 0;
        for index in (0..self.editors.tabs.len()).rev() {
            let tab = &self.editors.tabs[index];
            if tab.dirty(cx) || others && Some(tab.id) == active {
                continue;
            }
            let tab = self.editors.tabs.remove(index);
            self.retain_closed_editor(tab, cx);
            count += 1;
        }
        if !self.editors.tabs.iter().any(|tab| Some(tab.id) == active) {
            self.editors.active = None;
            self.document = None;
            if !self.editors.tabs.is_empty() {
                self.activate_editor(0, cx);
            }
        }
        self.notice = Some(format!(
            "Closed {count} saved buffers. Unsaved buffers were kept."
        ));
        cx.notify();
    }
    pub(super) fn reopen_closed_editor(&mut self, cx: &mut Context<Self>) {
        if self.editor_batch_blocked(cx) || self.editors.tabs.len() >= MAX_TABS {
            return;
        }
        while let Some(tab) = self.editors.closed.pop() {
            if self.editors.index(&tab.document.path).is_some() {
                continue;
            }
            self.editors.tabs.push(tab);
            self.activate_editor(self.editors.tabs.len() - 1, cx);
            self.notice = Some("Reopened the retained buffer with its selection and undo history. On-disk changes are checked when saving.".into());
            cx.notify();
            return;
        }
    }
    pub(super) fn move_editor_tab(&mut self, backwards: bool, cx: &mut Context<Self>) {
        if self.editor_batch_blocked(cx) {
            return;
        }
        let Some(index) = self
            .editors
            .tabs
            .iter()
            .position(|tab| Some(tab.id) == self.editors.active)
        else {
            return;
        };
        let next = if backwards {
            index.checked_sub(1)
        } else {
            index
                .checked_add(1)
                .filter(|n| *n < self.editors.tabs.len())
        };
        if let Some(next) = next {
            self.editors.tabs.swap(index, next);
        }
        cx.notify();
    }
    pub(super) fn editor_management(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        div()
            .px_2()
            .py_1()
            .flex()
            .items_center()
            .flex_wrap()
            .gap_1()
            .border_b_1()
            .border_color(rgb(palette().border))
            .child(
                ui::action(
                    "editor-save-all",
                    "Save all",
                    Some(Glyph::Check),
                    false,
                    cx.listener(|this, _: &(), _, cx| this.save_all_editors(cx)),
                )
                .text_size(px(11.)),
            )
            .child(
                ui::action(
                    "editor-refresh-disk",
                    "Refresh saved file",
                    Some(Glyph::Restore),
                    false,
                    cx.listener(|this, _: &(), _, cx| this.refresh_saved_editor(cx)),
                )
                .text_size(px(11.)),
            )
            .child(
                ui::action(
                    "editor-close-saved",
                    "Close saved",
                    None,
                    false,
                    cx.listener(|this, _: &(), _, cx| this.close_saved_editors(false, cx)),
                )
                .text_size(px(11.)),
            )
            .child(
                ui::action(
                    "editor-close-others",
                    "Close other saved",
                    None,
                    false,
                    cx.listener(|this, _: &(), _, cx| this.close_saved_editors(true, cx)),
                )
                .text_size(px(11.)),
            )
            .child(
                ui::action(
                    "editor-reopen",
                    "Reopen buffer",
                    Some(Glyph::Restore),
                    false,
                    cx.listener(|this, _: &(), _, cx| this.reopen_closed_editor(cx)),
                )
                .text_size(px(11.)),
            )
            .child(ui::chrome_button(
                "editor-move-left",
                "Move current tab left",
                Glyph::Back,
                false,
                cx.listener(|this, _: &(), _, cx| this.move_editor_tab(true, cx)),
            ))
            .child(ui::chrome_button(
                "editor-move-right",
                "Move current tab right",
                Glyph::Forward,
                false,
                cx.listener(|this, _: &(), _, cx| this.move_editor_tab(false, cx)),
            ))
            .into_any_element()
    }
}
