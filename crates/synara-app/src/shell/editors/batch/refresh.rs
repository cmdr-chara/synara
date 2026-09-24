//! A disk read is never permission to discard an edit made during that read.
use super::*;

fn buffer_matches_snapshot(expected: &str, current: &str, composing: bool) -> bool {
    !composing && current == expected
}

impl Shell {
    pub(in crate::shell) fn refresh_saved_editor(&mut self, cx: &mut Context<Self>) {
        if self.editor_batch_blocked(cx) || self.editors.loading.is_some() {
            return;
        }
        self.sync_editor_document();
        if self.active_document_dirty(cx) {
            self.error = Some(
                "Refresh kept your unsaved text. Save it elsewhere, or explicitly discard the buffer and reopen the file. A save conflict is never overwritten by refresh.".into(),
            );
            cx.notify();
            return;
        }
        let (Some(target), Some(document), Some(active)) = (
            self.workspace_target(),
            self.document.clone(),
            self.editors.active,
        ) else {
            return;
        };
        let root = target.root().clone();
        let project = self.project;
        let generation = self.editors.generation;
        let input = self.editor.entity_id();
        let path = document.path.clone();
        let baseline = document.snapshot.text;
        let workspace = self.controller.workspace.clone();
        self.clear_conflict_reload_confirmation();
        self.saving = true;
        self.error = None;
        let read = self.runtime.spawn(async move {
            match target {
                WorkspaceTarget::Local { root } => open_document(root, path).await,
                WorkspaceTarget::Ssh {
                    workspace: remote,
                    root,
                } => {
                    let filesystem = remote_filesystem(workspace, remote, root).await?;
                    open_remote_document(filesystem, path).await
                }
            }
        });
        cx.spawn(async move |view, cx| {
            let result = read.await;
            let _ = view.update(cx, |this, cx| {
                this.saving = false;
                if this.close != CloseState::Open {
                    let clean = !this.dirty(cx);
                    if this.close.saved(clean) {
                        this.begin_quit(cx);
                    }
                    cx.notify();
                    return;
                }
                if this.project != project
                    || this.root().as_ref() != Some(&root)
                    || this.editors.active != Some(active)
                    || this.editors.generation != generation
                    || this.editor.entity_id() != input
                {
                    cx.notify();
                    return;
                }
                // Input remains live during I/O. Even an unsaved buffer that
                // was clean when refresh began must never be replaced late.
                if !buffer_matches_snapshot(
                    &baseline,
                    this.editor.read(cx).text(),
                    this.editor.read(cx).is_composing(),
                ) {
                    this.notice = Some(
                        "Disk refresh discarded because the buffer changed while reading. Your edits are intact.".into(),
                    );
                    cx.notify();
                    return;
                }
                match result {
                    Ok(Ok(document)) => {
                        if document.snapshot.text == baseline {
                            // Advance the disk version without disturbing undo,
                            // selection or scroll when the text is unchanged.
                            this.document = Some(document);
                            this.sync_editor_document();
                        } else {
                            this.replace_editor_document(document, cx);
                        }
                        this.notice = Some("Reloaded the saved file from disk. No file was written.".into());
                    }
                    Ok(Err(error)) => {
                        this.error = Some(format!("Disk refresh failed: {error}. The buffer was kept."));
                    }
                    Err(error) => {
                        this.error = Some(format!("Disk refresh worker failed: {error}. The buffer was kept."));
                    }
                }
                cx.notify();
            });
        }).detach();
        cx.notify();
    }

    pub(in crate::shell::editors) fn resolve_conflict_reload(&mut self, cx: &mut Context<Self>) {
        let active_path = self
            .document
            .as_ref()
            .map(|document| document.path.as_path());
        if !is_save_conflict(self.error.as_deref(), active_path) || self.editor_batch_blocked(cx) {
            return;
        }
        if !self.conflict_reload_confirmed(cx) {
            let (Some(tab_id), Some(document)) = (self.editors.active, self.document.as_ref())
            else {
                return;
            };
            self.editors.reload_confirmation = Some(super::super::ReloadConfirmation {
                tab_id,
                disk_version: document.snapshot.version.clone(),
                buffer: self.editor.read(cx).text().to_owned(),
            });
            cx.notify();
            return;
        }
        self.clear_conflict_reload_confirmation();
        self.sync_editor_document();
        let (Some(target), Some(document), Some(active)) = (
            self.workspace_target(),
            self.document.clone(),
            self.editors.active,
        ) else {
            return;
        };
        let root = target.root().clone();
        let project = self.project;
        let generation = self.editors.generation;
        let input = self.editor.entity_id();
        let baseline = self.editor.read(cx).text().to_owned();
        let workspace = self.controller.workspace.clone();
        self.saving = true;
        self.error = None;
        self.notice = Some("Reloading the file from disk. No file will be written.".into());
        let read = self.runtime.spawn(async move {
            match target {
                WorkspaceTarget::Local { root } => open_document(root, document.path).await,
                WorkspaceTarget::Ssh {
                    workspace: remote,
                    root,
                } => {
                    let filesystem = remote_filesystem(workspace, remote, root).await?;
                    open_remote_document(filesystem, document.path).await
                }
            }
        });
        cx.spawn(async move |view, cx| {
            let result = read.await;
            let _ = view.update(cx, |this, cx| {
                let current_editor = this.project == project
                    && this.root().as_ref() == Some(&root)
                    && this.editors.active == Some(active)
                    && this.editors.generation == generation
                    && this.editor.entity_id() == input;
                if !current_editor {
                    // Editor I/O is serialized by `saving`; project navigation and other
                    // editor saves are refused while this operation owns that guard.
                    this.saving = false;
                    cx.notify();
                    return;
                }
                this.saving = false;
                if !buffer_matches_snapshot(
                    &baseline,
                    this.editor.read(cx).text(),
                    this.editor.read(cx).is_composing(),
                ) {
                    this.notice = Some(
                        "Reload canceled because the buffer changed while reading. Your edits are intact.".into(),
                    );
                    cx.notify();
                    return;
                }
                match result {
                    Ok(Ok(document)) => {
                        this.replace_editor_document(document, cx);
                        this.error = None;
                        this.notice = Some(
                            "Reloaded the current disk version and discarded the confirmed buffer. No file was written.".into(),
                        );
                    }
                    Ok(Err(error)) => {
                        this.error = Some(format!(
                            "Reload failed: {error}. Your buffer was kept."
                        ));
                    }
                    Err(error) => {
                        this.error = Some(format!(
                            "Reload worker failed: {error}. Your buffer was kept."
                        ));
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(in crate::shell::editors) fn overwrite_conflicted_editor(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let active_path = self
            .document
            .as_ref()
            .map(|document| document.path.as_path());
        if !is_save_conflict(self.error.as_deref(), active_path) || self.editor_batch_blocked(cx) {
            return;
        }
        self.clear_conflict_reload_confirmation();
        self.sync_editor_document();
        let (Some(target), Some(document), Some(active)) = (
            self.workspace_target(),
            self.document.clone(),
            self.editors.active,
        ) else {
            return;
        };
        let root = target.root().clone();
        let project = self.project;
        let generation = self.editors.generation;
        let input = self.editor.entity_id();
        let path = document.path.clone();
        let text = self.editor.read(cx).text().to_owned();
        let workspace = self.controller.workspace.clone();
        self.saving = true;
        self.error = None;
        self.notice =
            Some("Reading the latest disk version before the explicit overwrite...".into());
        let write: tokio::task::JoinHandle<WorkspaceResult<Document>> =
            self.runtime.spawn(async move {
                match target {
                    WorkspaceTarget::Local { root } => {
                        let mut current = open_document(root.clone(), path).await?;
                        let version = save_document(root, current.clone(), text.clone()).await?;
                        current.snapshot.text = text;
                        current.snapshot.version = version;
                        Ok(current)
                    }
                    WorkspaceTarget::Ssh {
                        workspace: remote,
                        root,
                    } => {
                        let filesystem = remote_filesystem(workspace, remote, root).await?;
                        let mut current = open_remote_document(filesystem.clone(), path).await?;
                        let version =
                            save_remote_document(filesystem, current.clone(), text.clone()).await?;
                        current.snapshot.text = text;
                        current.snapshot.version = version;
                        Ok(current)
                    }
                }
            });
        cx.spawn(async move |view, cx| {
            let result = write.await;
            let _ = view.update(cx, |this, cx| {
                let current_editor = this.project == project
                    && this.root().as_ref() == Some(&root)
                    && this.editors.active == Some(active)
                    && this.editors.generation == generation
                    && this.editor.entity_id() == input;
                if !current_editor {
                    // The same global editor-I/O guard prevents a newer save from starting
                    // before this completion releases it.
                    this.saving = false;
                    cx.notify();
                    return;
                }
                this.saving = false;
                match result {
                    Ok(Ok(document)) => {
                        this.document = Some(document);
                        this.sync_editor_document();
                        this.error = None;
                        this.notice = Some(
                            "Disk now contains the buffer captured when Overwrite was chosen. Any later typing remains unsaved.".into(),
                        );
                    }
                    Ok(Err(error)) => {
                        this.error = Some(format!(
                            "Overwrite failed: {error}. Your buffer was kept."
                        ));
                    }
                    Err(error) => {
                        this.error = Some(format!(
                            "Overwrite worker failed: {error}. Your buffer was kept."
                        ));
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::buffer_matches_snapshot;
    use super::is_save_conflict;
    use std::path::Path;

    #[test]
    fn conflict_actions_only_appear_for_compare_and_swap_conflicts() {
        let path = Path::new("src/main.rs");
        assert!(is_save_conflict(
            Some("Save failed: file changed outside this editor"),
            Some(path),
        ));
        assert!(is_save_conflict(
            Some("Overwrite failed: file changed outside this editor"),
            Some(path),
        ));
        assert!(is_save_conflict(
            Some("Save all stopped at src/main.rs: file changed outside this editor"),
            Some(path),
        ));
        assert!(!is_save_conflict(
            Some("Save all stopped at src/other.rs: file changed outside this editor"),
            Some(path),
        ));
        assert!(!is_save_conflict(
            Some("Save all stopped at src/main.rs:copy: file changed outside this editor"),
            Some(path),
        ));
        assert!(!is_save_conflict(
            Some("Save failed: permission denied"),
            Some(path),
        ));
        assert!(!is_save_conflict(
            Some("Git operation failed: file changed outside this editor"),
            Some(path),
        ));
        assert!(!is_save_conflict(
            Some("Save failed: file changed outside this editor"),
            None
        ));
        assert!(is_save_conflict(
            Some(
                "Auto-save stopped at src/main.rs: file changed outside this editor. Auto-save is off."
            ),
            Some(path)
        ));
        assert!(!is_save_conflict(
            Some("Auto-save stopped at src/main.rs:copy: file changed outside this editor"),
            Some(path)
        ));
        assert!(!is_save_conflict(
            Some("Auto-save stopped at src/other.rs: file changed outside this editor"),
            Some(path)
        ));
        assert!(!is_save_conflict(None, Some(path)));
    }

    #[test]
    fn reload_preserves_composing_or_changed_buffers() {
        assert!(buffer_matches_snapshot("same", "same", false));
        assert!(!buffer_matches_snapshot("before", "after", false));
        assert!(!buffer_matches_snapshot("same", "same", true));
    }
}
