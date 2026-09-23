//! A disk read is never permission to discard an edit made during that read.
use super::*;

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
                if this.editor.read(cx).is_composing()
                    || this.editor.read(cx).text() != baseline
                {
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
}
