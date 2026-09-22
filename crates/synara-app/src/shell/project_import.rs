//! Explicit, read-only history review. No restored prompt or provider process is run.
use super::*;
use crate::ui::{self, palette};
use tokio_util::sync::CancellationToken;
mod view;

pub(super) enum Reply {
    Scanned(HistoryDiscovery),
    Previewed(HistoryPreview),
    Reviewed(HistoryImportReview),
    Imported(HistoryImportOutcome),
    Failed(String),
}
pub(super) struct ImportState {
    root: Entity<TextEntry>,
    provider: HistoryProvider,
    scan: Option<HistoryDiscovery>,
    preview: Option<HistoryPreview>,
    review: Option<HistoryImportReview>,
    destination: Option<ProjectId>,
    agent: Option<String>,
    result: Option<TaskId>,
    error: Option<String>,
    notice: Option<String>,
    busy: bool,
    committing: bool,
    picker: bool,
    token: CancellationToken,
    message: usize,
    chunk: usize,
    file_page: usize,
    _subscription: Subscription,
}
impl ImportState {
    pub fn new(cx: &mut Context<Shell>) -> Self {
        let root = cx.new(|cx| {
            TextEntry::new(
                "Absolute local history folder",
                EntryMode::SingleLine,
                34.,
                cx,
            )
        });
        let subscription = cx.subscribe(&root, |this, _, event, cx| {
            if matches!(event, EntryEvent::Changed) && !this.project_import.busy {
                this.project_import.clear_source();
                cx.notify();
            }
        });
        Self {
            root,
            provider: HistoryProvider::Codex,
            scan: None,
            preview: None,
            review: None,
            destination: None,
            agent: None,
            result: None,
            error: None,
            notice: None,
            busy: false,
            committing: false,
            picker: false,
            token: Default::default(),
            message: 0,
            chunk: 0,
            file_page: 0,
            _subscription: subscription,
        }
    }
    pub fn pending(&self) -> bool {
        self.busy || self.picker || self.review.is_some()
    }
    fn clear_source(&mut self) {
        self.scan = None;
        self.preview = None;
        self.review = None;
        self.result = None;
        self.error = None;
        self.notice = None;
        self.message = 0;
        self.chunk = 0;
        self.file_page = 0;
    }
}
impl Shell {
    fn import_job(
        &mut self,
        committing: bool,
        work: impl std::future::Future<Output = Result<Reply, String>> + Send + 'static,
        cx: &mut Context<Self>,
    ) {
        if self.project_import.busy || self.close != CloseState::Open {
            return;
        }
        self.project_import.busy = true;
        self.project_import.committing = committing;
        self.project_import.error = None;
        self.project_import.notice = None;
        let sender = self.sender.clone();
        self.runtime.spawn(async move {
            let reply = work.await.unwrap_or_else(Reply::Failed);
            let _ = sender.send(Update::ProjectImport(Box::new(reply))).await;
        });
        cx.notify();
    }
    fn import_pick_folder(&mut self, cx: &mut Context<Self>) {
        if self.project_import.pending() || self.close != CloseState::Open {
            return;
        }
        self.project_import.picker = true;
        let picker = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Choose local provider history folder".into()),
        });
        cx.spawn(async move |view,cx| {
            let result=picker.await;
            let _=view.update(cx,|this,cx| {
                this.project_import.picker=false;
                match result {
                    Ok(Ok(Some(paths))) if paths.len()==1 => {
                        this.project_import.clear_source();
                        this.project_import.root.update(cx,|entry,cx|entry.set_text(paths[0].display().to_string(),cx));
                    }
                    Ok(Ok(None)) => {},
                    _ => this.project_import.error=Some("The folder picker could not select a folder. You may enter an absolute path instead.".into()),
                }
                cx.notify();
            });
        }).detach();
    }
    fn scan_history(&mut self, cx: &mut Context<Self>) {
        if self.project_import.pending() {
            return;
        }
        let root = PathBuf::from(self.project_import.root.read(cx).text());
        let provider = self.project_import.provider;
        self.project_import.clear_source();
        self.project_import.token = CancellationToken::new();
        let token = self.project_import.token.clone();
        self.import_job(
            false,
            async move {
                HistorySource::discover(root, provider, token)
                    .await
                    .map(Reply::Scanned)
                    .map_err(|e| e.to_string())
            },
            cx,
        );
    }
    fn preview_history(&mut self, file: HistoryFile, leaf: Option<String>, cx: &mut Context<Self>) {
        if self.project_import.pending() {
            return;
        }
        let Some(source) = self.project_import.scan.as_ref().map(|s| s.source.clone()) else {
            return;
        };
        self.project_import.review = None;
        self.project_import.result = None;
        self.project_import.token = CancellationToken::new();
        let token = self.project_import.token.clone();
        self.import_job(
            false,
            async move {
                source
                    .preview(file, leaf, token)
                    .await
                    .map(Reply::Previewed)
                    .map_err(|e| e.to_string())
            },
            cx,
        );
    }
    fn review_history(&mut self, cx: &mut Context<Self>) {
        if self.project_import.pending() {
            return;
        }
        let (Some(preview), Some(project), Some(agent)) = (
            self.project_import.preview.clone(),
            self.project_import.destination,
            self.project_import.agent.clone(),
        ) else {
            self.project_import.error = Some(
                "Choose a source preview, a local destination, and a future coding agent first."
                    .into(),
            );
            cx.notify();
            return;
        };
        let workspace = self.controller.workspace.clone();
        self.import_job(
            false,
            async move {
                workspace
                    .review_history_import(preview, project, agent)
                    .await
                    .map(Reply::Reviewed)
                    .map_err(|e| e.to_string())
            },
            cx,
        );
    }
    fn confirm_history(&mut self, cx: &mut Context<Self>) {
        if self.project_import.busy || self.project_import.picker {
            return;
        }
        let Some(review) = self.project_import.review.clone() else {
            return;
        };
        let workspace = self.controller.workspace.clone();
        self.project_import.token = CancellationToken::new();
        let token = self.project_import.token.clone();
        self.import_job(
            true,
            async move {
                workspace
                    .import_history(review, token)
                    .await
                    .map(Reply::Imported)
                    .map_err(|e| e.to_string())
            },
            cx,
        );
    }
    pub(super) fn import_reply(&mut self, reply: Reply, cx: &mut Context<Self>) {
        self.project_import.busy = false;
        self.project_import.committing = false;
        match reply {
            Reply::Scanned(scan) => {
                self.project_import.scan = Some(scan);
            }
            Reply::Previewed(preview) => {
                self.project_import.preview = Some(preview);
                self.project_import.message = 0;
                self.project_import.chunk = 0;
            }
            Reply::Reviewed(review) => self.project_import.review = Some(review),
            Reply::Imported(outcome) => {
                self.project_import.review = None;
                let task = match outcome {
                    HistoryImportOutcome::Imported(task) => {
                        self.project_import.notice=Some("Imported as an unsent standalone chat. Source files were not changed. No provider session was resumed.".into());
                        Some(task)
                    }
                    HistoryImportOutcome::AlreadyImported {
                        task,
                        source_changed,
                    } => {
                        self.project_import.notice=Some(if task.is_none(){"This session was imported previously and its native chat was deleted. The receipt is retained to prevent accidental re-import."}
                            else if source_changed{"This session already exists in Synara. Its source or branch has changed. The existing import was preserved, not silently replaced or duplicated."}
                            else{"This session is already imported. Its existing native chat was preserved."}.into());
                        task
                    }
                };
                self.project_import.result = task.as_ref().map(|t| t.id);
                if let Some(task) = task {
                    if !self.catalog.tasks.iter().any(|t| t.id == task.id) {
                        self.catalog.tasks.push(task);
                    }
                }
            }
            Reply::Failed(error) => self.project_import.error = Some(error),
        }
        cx.notify();
    }
    fn open_imported_chat(&mut self, cx: &mut Context<Self>) {
        if self.project_import.pending() {
            return;
        }
        if let Some(task) = self.project_import.result {
            if self.select_task(task, cx) {
                self.show_conversation(cx);
            }
        }
    }
}
