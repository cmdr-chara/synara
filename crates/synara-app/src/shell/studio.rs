//! Studio output browser. All file data comes from the bounded workspace service.
use super::*;
use crate::ui::{self, Glyph, palette};
use std::path::Path;
mod export;
mod pdf;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum StudioKindFilter {
    #[default]
    All,
    Images,
    Documents,
    Other,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum StudioSortOrder {
    #[default]
    OutputsFirst,
    Name,
    LatestReport,
    Largest,
}

impl StudioSortOrder {
    fn next(self) -> Self {
        match self {
            Self::OutputsFirst => Self::Name,
            Self::Name => Self::LatestReport,
            Self::LatestReport => Self::Largest,
            Self::Largest => Self::OutputsFirst,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::OutputsFirst => "Reported first",
            Self::Name => "Name",
            Self::LatestReport => "Latest report",
            Self::Largest => "Largest",
        }
    }
}

pub(super) enum StudioReply {
    PdfLoaded {
        task: TaskId,
        generation: u64,
        path: PathBuf,
        result: Result<(StudioPdf, StudioPdfPage), String>,
    },
    ExportReviewed {
        task: TaskId,
        generation: u64,
        result: Result<StudioExportReview, String>,
    },
    Exported(Result<(), String>),
    VersionExported(Result<(), String>),
    Listed {
        task: TaskId,
        generation: u64,
        result: Result<StudioFiles, String>,
    },
    Previewed {
        task: TaskId,
        generation: u64,
        path: PathBuf,
        result: Result<StudioPreview, String>,
    },
    VersionsCaptured {
        task: TaskId,
        generation: u64,
        path: PathBuf,
        result: Result<Vec<StudioTextVersion>, String>,
    },
    VersionsCleared {
        task: TaskId,
        generation: u64,
        path: PathBuf,
        result: Result<usize, String>,
    },
    HistoryLoaded {
        task: TaskId,
        generation: u64,
        path: PathBuf,
        result: Result<GitFileHistory, String>,
    },
    RevisionLoaded {
        task: TaskId,
        generation: u64,
        path: PathBuf,
        result: Result<GitFileRevision, String>,
    },
}
enum Preview {
    Pdf(pdf::PdfView),
    Text {
        text: String,
        markdown: bool,
    },
    DocumentText(String),
    Image {
        image: Arc<gpui::Image>,
        width: u32,
        height: u32,
    },
    Unsupported(u64),
}
pub(super) struct StudioState {
    pub open: bool,
    task: Option<TaskId>,
    query: Entity<TextEntry>,
    listing: StudioFiles,
    generation: u64,
    preview_generation: u64,
    preview_cancel: tokio_util::sync::CancellationToken,
    pub exporting: bool,
    export_review: Option<StudioExportReview>,
    loading: bool,
    refresh_pending: bool,
    preview_loading: bool,
    selected: Option<PathBuf>,
    reopen_after_navigation: Option<(TaskId, PathBuf)>,
    preview: Option<Preview>,
    history: Option<GitFileHistory>,
    revision: Option<GitFileRevision>,
    snapshots: Vec<StudioTextVersion>,
    selected_snapshot: Option<usize>,
    clear_versions_confirm: bool,
    history_loading: bool,
    error: Option<String>,
    only_outputs: bool,
    kind_filter: StudioKindFilter,
    sort_order: StudioSortOrder,
    turn_filter: Option<(TaskId, usize)>,
    raw_text: bool,
    image_zoom: Option<f32>,
    _subscription: Subscription,
}
impl StudioState {
    pub fn new(cx: &mut Context<Shell>) -> Self {
        let query = cx.new(|cx| {
            TextEntry::new("Find Library files...", EntryMode::SingleLine, 32., cx)
                .with_leading_icon(Glyph::Search)
        });
        let subscription = cx.subscribe(&query, |_, _, _, cx| cx.notify());
        Self {
            open: false,
            task: None,
            query,
            listing: StudioFiles::default(),
            generation: 0,
            preview_generation: 0,
            preview_cancel: Default::default(),
            exporting: false,
            export_review: None,
            loading: false,
            refresh_pending: false,
            preview_loading: false,
            selected: None,
            reopen_after_navigation: None,
            preview: None,
            history: None,
            revision: None,
            snapshots: Vec::new(),
            selected_snapshot: None,
            clear_versions_confirm: false,
            history_loading: false,
            error: None,
            only_outputs: false,
            kind_filter: StudioKindFilter::All,
            sort_order: StudioSortOrder::OutputsFirst,
            turn_filter: None,
            raw_text: false,
            image_zoom: None,
            _subscription: subscription,
        }
    }
    pub fn cancel_preview(&mut self) {
        self.preview_cancel.cancel();
        self.preview_loading = false;
    }
    pub fn reset(&mut self) {
        self.preview_cancel.cancel();
        self.export_review = None;
        self.open = false;
        self.task = None;
        self.generation = self.generation.wrapping_add(1);
        self.preview_generation = self.preview_generation.wrapping_add(1);
        self.preview = None;
        self.history = None;
        self.revision = None;
        self.snapshots.clear();
        self.selected_snapshot = None;
        self.clear_versions_confirm = false;
        self.history_loading = false;
        self.selected = None;
        self.reopen_after_navigation = None;
        self.listing = StudioFiles::default();
        self.loading = false;
        self.refresh_pending = false;
        self.preview_loading = false;
        self.error = None;
        self.raw_text = false;
        self.turn_filter = None;
        self.image_zoom = None;
    }
}
impl Drop for StudioState {
    fn drop(&mut self) {
        self.preview_cancel.cancel();
    }
}
fn image_path(path: &std::path::Path) -> bool {
    path.extension().and_then(|x| x.to_str()).is_some_and(|x| {
        matches!(
            x.to_ascii_lowercase().as_str(),
            "png" | "jpg" | "jpeg" | "webp"
        )
    })
}
fn document_path(path: &Path) -> bool {
    path.extension().and_then(|x| x.to_str()).is_some_and(|x| {
        matches!(
            x.to_ascii_lowercase().as_str(),
            "pdf"
                | "txt"
                | "md"
                | "markdown"
                | "rst"
                | "csv"
                | "tsv"
                | "rtf"
                | "doc"
                | "docx"
                | "odt"
                | "json"
                | "xml"
                | "yaml"
                | "yml"
                | "toml"
        )
    })
}
fn studio_kind_matches(path: &Path, filter: StudioKindFilter) -> bool {
    match filter {
        StudioKindFilter::All => true,
        StudioKindFilter::Images => image_path(path),
        StudioKindFilter::Documents => document_path(path),
        StudioKindFilter::Other => !image_path(path) && !document_path(path),
    }
}
fn studio_files_matching<'a>(
    entries: &'a [StudioFile],
    query: &str,
    only_outputs: bool,
    kind_filter: StudioKindFilter,
    turn_filter: Option<(TaskId, usize)>,
) -> Vec<&'a StudioFile> {
    entries
        .iter()
        .filter(|file| {
            (!only_outputs || file.reported_output)
                && studio_kind_matches(&file.path, kind_filter)
                && turn_filter.is_none_or(|(task, turn)| {
                    file.source_task == Some(task)
                        && file
                            .source_turn
                            .as_ref()
                            .is_some_and(|source| source.number == turn)
                })
                && (query.is_empty() || file.path.to_string_lossy().to_lowercase().contains(query))
        })
        .collect()
}
fn studio_file_path_key(file: &StudioFile) -> String {
    file.path.to_string_lossy().to_lowercase()
}
fn sort_studio_files(files: &mut [&StudioFile], order: StudioSortOrder) {
    files.sort_by(|left, right| {
        let primary = match order {
            StudioSortOrder::OutputsFirst => right.reported_output.cmp(&left.reported_output),
            StudioSortOrder::Name => std::cmp::Ordering::Equal,
            StudioSortOrder::LatestReport => right.reported_at_ms.cmp(&left.reported_at_ms),
            StudioSortOrder::Largest => right.bytes.cmp(&left.bytes),
        };
        primary.then_with(|| studio_file_path_key(left).cmp(&studio_file_path_key(right)))
    });
}
fn take_reopen_path(task: TaskId, pending: Option<(TaskId, PathBuf)>) -> Option<PathBuf> {
    pending.and_then(|(expected_task, path)| (expected_task == task).then_some(path))
}
fn restored_output_is_attributed(task: TaskId, path: &Path, listing: &[StudioFile]) -> bool {
    listing
        .iter()
        .any(|entry| entry.path == path && entry.source_task == Some(task))
}
impl Shell {
    pub(super) fn studio_tool_finished(&mut self, thread: ThreadId, cx: &mut Context<Self>) {
        if !self.studio.open
            || self.studio.task != self.selected
            || !self
                .task()
                .is_some_and(|task| task.thread_id == thread && task.scope == TaskScope::Studio)
        {
            return;
        }
        if self.studio.loading {
            self.studio.refresh_pending = true;
        } else {
            self.refresh_studio_outputs(cx);
        }
    }
    pub(super) fn open_studio_outputs(&mut self, cx: &mut Context<Self>) {
        if self
            .task()
            .is_none_or(|task| task.scope != TaskScope::Studio)
        {
            return;
        }
        self.set_panel(Panel::Files, cx);
        self.studio.open = true;
        self.refresh_studio_outputs(cx);
    }
    fn refresh_studio_outputs(&mut self, cx: &mut Context<Self>) {
        let Some(task) = self
            .task()
            .filter(|task| task.scope == TaskScope::Studio)
            .map(|task| task.id)
        else {
            return;
        };
        self.studio.generation = self.studio.generation.wrapping_add(1);
        let generation = self.studio.generation;
        self.studio.task = Some(task);
        self.studio.loading = true;
        self.studio.error = None;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::Studio(Box::new(StudioReply::Listed {
                task,
                generation,
                result: workspace
                    .studio_files(task)
                    .await
                    .map_err(|error| error.to_string()),
            })))
        });
        cx.notify();
    }
    fn preview_studio_file(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let Some(task) = self.selected.filter(|task| Some(*task) == self.studio.task) else {
            return;
        };
        self.studio.preview_cancel.cancel();
        self.studio.preview_cancel = Default::default();
        self.studio.export_review = None;
        self.studio.preview_generation = self.studio.preview_generation.wrapping_add(1);
        let generation = self.studio.preview_generation;
        self.studio.selected = Some(path.clone());
        self.studio.image_zoom = None;
        self.studio.preview = None;
        self.studio.history = None;
        self.studio.revision = None;
        self.studio.snapshots.clear();
        self.studio.selected_snapshot = None;
        self.studio.clear_versions_confirm = false;
        self.studio.history_loading = false;
        self.studio.preview_loading = true;
        self.studio.error = None;
        if path
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|s| s.eq_ignore_ascii_case("pdf"))
        {
            self.start_studio_pdf(task, path, generation, cx);
            return;
        }
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::Studio(Box::new(StudioReply::Previewed {
                task,
                generation,
                result: workspace
                    .studio_preview(task, path.clone())
                    .await
                    .map_err(|error| error.to_string()),
                path,
            })))
        });
        cx.notify();
    }
    fn clear_studio_versions(&mut self, cx: &mut Context<Self>) {
        let Some(task) = self.selected.filter(|task| Some(*task) == self.studio.task) else {
            return;
        };
        let Some(path) = self.studio.selected.clone() else {
            return;
        };
        if self.studio.snapshots.is_empty() {
            return;
        }
        if !self.studio.clear_versions_confirm {
            self.studio.clear_versions_confirm = true;
            cx.notify();
            return;
        }
        self.studio.clear_versions_confirm = false;
        let generation = self.studio.preview_generation;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::Studio(Box::new(StudioReply::VersionsCleared {
                task,
                generation,
                path: path.clone(),
                result: workspace
                    .clear_studio_text_versions(task, path, true)
                    .await
                    .map_err(|error| error.to_string()),
            })))
        });
        cx.notify();
    }

    fn open_studio_history(&mut self, cx: &mut Context<Self>) {
        let Some(task) = self.task().filter(|task| {
            self.studio.open && self.studio.task == Some(task.id) && task.scope == TaskScope::Studio
        }) else {
            return;
        };
        if !matches!(self.studio.preview, Some(Preview::Text { .. })) || self.studio.history_loading
        {
            return;
        }
        let Some(path) = self.studio.selected.clone() else {
            return;
        };
        let root = task.working_directory.clone();
        let task = task.id;
        let generation = self.studio.preview_generation;
        let cancel = self.studio.preview_cancel.clone();
        self.studio.history = None;
        self.studio.revision = None;
        self.studio.history_loading = true;
        self.studio.error = None;
        self.job(async move {
            let result = GitService::new(root)
                .file_history(path.clone(), &cancel)
                .await
                .map_err(|error| error.to_string());
            Ok(Update::Studio(Box::new(StudioReply::HistoryLoaded {
                task,
                generation,
                path,
                result,
            })))
        });
        cx.notify();
    }
    fn open_studio_revision(&mut self, commit: String, cx: &mut Context<Self>) {
        let Some(task) = self.task().filter(|task| {
            self.studio.open && self.studio.task == Some(task.id) && task.scope == TaskScope::Studio
        }) else {
            return;
        };
        let (Some(path), Some(history)) =
            (self.studio.selected.clone(), self.studio.history.clone())
        else {
            return;
        };
        if self.studio.history_loading || !history.commits.iter().any(|row| row.id == commit) {
            return;
        }
        let root = task.working_directory.clone();
        let task = task.id;
        let generation = self.studio.preview_generation;
        let cancel = self.studio.preview_cancel.clone();
        self.studio.history_loading = true;
        self.studio.revision = None;
        self.studio.error = None;
        self.job(async move {
            let result = GitService::new(root)
                .file_revision(&history, &commit, &cancel)
                .await
                .map_err(|error| error.to_string());
            Ok(Update::Studio(Box::new(StudioReply::RevisionLoaded {
                task,
                generation,
                path,
                result,
            })))
        });
        cx.notify();
    }
    fn open_studio_output_with_source(
        &mut self,
        source: TaskId,
        path: PathBuf,
        cx: &mut Context<Self>,
    ) {
        let Some(current) = self.task() else { return };
        let Some(source_task) = self.catalog.tasks.iter().find(|task| task.id == source) else {
            self.studio.error = Some("This output's source chat is no longer available.".into());
            cx.notify();
            return;
        };
        let output_is_current = self.studio.open
            && self.studio.task == Some(current.id)
            && current.scope == TaskScope::Studio
            && source_task.scope == TaskScope::Studio
            && current.project_id == source_task.project_id
            && current.working_directory == source_task.working_directory
            && self
                .studio
                .listing
                .entries
                .iter()
                .any(|entry| entry.path == path && entry.source_task == Some(source));
        if !output_is_current {
            self.studio.error = Some(
                "This output's source link is stale. Refresh the Library and choose the output again."
                    .into(),
            );
            cx.notify();
            return;
        }
        if self.selected != Some(source) {
            if !self.select_task(source, cx) {
                return;
            }
            self.studio.reopen_after_navigation = Some((source, path));
            self.open_studio_outputs(cx);
        }
        // When already in the reporting thread, the selected output is already
        // open in the Library alongside this conversation.
    }
    pub(super) fn studio_reply(&mut self, reply: StudioReply, cx: &mut Context<Self>) {
        match reply {
            StudioReply::PdfLoaded {
                task,
                generation,
                path,
                result,
            } => self.studio_pdf_loaded(task, generation, path, result),
            StudioReply::ExportReviewed {
                task,
                generation,
                result,
            } => self.studio_export_reviewed(task, generation, result),
            StudioReply::Exported(result) => {
                self.studio.exporting = false;
                match result {
                    Ok(()) => self.notice = Some("The reviewed Library file was saved to a new destination without replacing existing files.".into()),
                    Err(e) => self.error = Some(format!("Library export failed: {e}")),
                }
            }
            StudioReply::VersionExported(result) => {
                self.studio.exporting = false;
                match result {
                    Ok(()) => self.notice = Some(
                        "The selected durable Studio version was saved to a new destination without changing the workspace file."
                            .into(),
                    ),
                    Err(error) => {
                        self.error = Some(format!("Studio version export failed: {error}"))
                    }
                }
            }
            StudioReply::Listed {
                task,
                generation,
                result,
            } => {
                if self.selected != Some(task)
                    || self.studio.task != Some(task)
                    || self.studio.generation != generation
                {
                    return;
                }
                self.studio.loading = false;
                match result {
                    Ok(listing) => {
                        self.studio.listing = listing;
                        let restored =
                            take_reopen_path(task, self.studio.reopen_after_navigation.take());
                        if let Some(path) =
                            restored.clone().or_else(|| self.studio.selected.clone())
                        {
                            let output_exists = if restored.is_some() {
                                restored_output_is_attributed(
                                    task,
                                    &path,
                                    &self.studio.listing.entries,
                                )
                            } else {
                                self.studio
                                    .listing
                                    .entries
                                    .iter()
                                    .any(|entry| entry.path == path)
                            };
                            if output_exists {
                                self.preview_studio_file(path, cx);
                            } else {
                                self.studio.preview = None;
                                self.studio.selected = None;
                                if restored.is_some() {
                                    self.studio.error = Some(
                                        "This output is no longer attributed to its reporting Hub."
                                            .into(),
                                    );
                                }
                            }
                        }
                    }
                    Err(error) => self.studio.error = Some(error),
                }
                if self.studio.refresh_pending {
                    self.studio.refresh_pending = false;
                    self.refresh_studio_outputs(cx);
                }
            }
            StudioReply::Previewed {
                task,
                generation,
                path,
                result,
            } => {
                if self.selected != Some(task)
                    || self.studio.task != Some(task)
                    || self.studio.preview_generation != generation
                    || self.studio.selected.as_ref() != Some(&path)
                {
                    return;
                }
                self.studio.preview_loading = false;
                match result {
                    Ok(StudioPreview::Text { text, markdown }) => {
                        if text.len() <= 128 * 1024 {
                            let workspace = self.controller.workspace.clone();
                            let version_path = path.clone();
                            let version_text = text.clone();
                            self.job(async move {
                                Ok(Update::Studio(Box::new(StudioReply::VersionsCaptured {
                                    task,
                                    generation,
                                    path: version_path.clone(),
                                    result: workspace
                                        .capture_studio_text_version(
                                            task,
                                            version_path,
                                            version_text,
                                        )
                                        .await
                                        .map_err(|error| error.to_string()),
                                })))
                            });
                        }
                        self.studio.preview = Some(Preview::Text { text, markdown })
                    }
                    Ok(StudioPreview::DocumentText { text }) => {
                        self.studio.preview = Some(Preview::DocumentText(text));
                    }
                    Ok(StudioPreview::Image {
                        bytes,
                        format,
                        width,
                        height,
                    }) => {
                        let format = match format {
                            PreviewImageFormat::Png => gpui::ImageFormat::Png,
                            PreviewImageFormat::Jpeg => gpui::ImageFormat::Jpeg,
                        };
                        self.studio.preview = Some(Preview::Image {
                            image: Arc::new(gpui::Image::from_bytes(format, bytes)),
                            width,
                            height,
                        });
                    }
                    Ok(StudioPreview::Unsupported { bytes }) => {
                        self.studio.preview = Some(Preview::Unsupported(bytes))
                    }
                    Err(error) => self.studio.error = Some(error),
                }
            }
            StudioReply::VersionsCaptured {
                task,
                generation,
                path,
                result,
            } => {
                if self.selected != Some(task)
                    || self.studio.task != Some(task)
                    || self.studio.preview_generation != generation
                    || self.studio.selected.as_ref() != Some(&path)
                {
                    return;
                }
                match result {
                    Ok(versions) => self.studio.snapshots = versions,
                    Err(error) => {
                        self.studio.error = Some(format!(
                            "Studio version history could not be saved: {error}"
                        ))
                    }
                }
            }
            StudioReply::VersionsCleared {
                task,
                generation,
                path,
                result,
            } => {
                if self.selected != Some(task)
                    || self.studio.task != Some(task)
                    || self.studio.preview_generation != generation
                    || self.studio.selected.as_ref() != Some(&path)
                {
                    return;
                }
                self.studio.clear_versions_confirm = false;
                match result {
                    Ok(_) => {
                        self.studio.snapshots.clear();
                        self.studio.selected_snapshot = None;
                        self.notice = Some(
                            "Durable preview history cleared. The workspace file was not changed."
                                .into(),
                        );
                    }
                    Err(error) => {
                        self.studio.error = Some(format!(
                            "Studio version history could not be cleared: {error}"
                        ))
                    }
                }
            }
            StudioReply::HistoryLoaded {
                task,
                generation,
                path,
                result,
            } => {
                if self.selected != Some(task)
                    || self.studio.task != Some(task)
                    || self.studio.preview_generation != generation
                    || self.studio.selected.as_ref() != Some(&path)
                {
                    return;
                }
                self.studio.history_loading = false;
                match result {
                    Ok(history) => self.studio.history = Some(history),
                    Err(error) => self.studio.error = Some(error),
                }
            }
            StudioReply::RevisionLoaded {
                task,
                generation,
                path,
                result,
            } => {
                if self.selected != Some(task)
                    || self.studio.task != Some(task)
                    || self.studio.preview_generation != generation
                    || self.studio.selected.as_ref() != Some(&path)
                {
                    return;
                }
                self.studio.history_loading = false;
                match result {
                    Ok(revision) => self.studio.revision = Some(revision),
                    Err(error) => self.studio.error = Some(error),
                }
            }
        }
        cx.notify();
    }
    pub(super) fn studio_outputs_button(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        ui::chrome_button(
            "studio-outputs",
            "Hub Library and workspace files",
            Glyph::Files,
            false,
            cx.listener(|this, _: &(), _, cx| this.open_studio_outputs(cx)),
        )
        .size(px(26.))
        .into_any_element()
    }
    fn studio_reference_to_draft(&mut self, cx: &mut Context<Self>) {
        if !self.studio.open
            || self.studio.task != self.selected
            || self.close != CloseState::Open
            || self.loading_task.is_some()
        {
            return;
        }
        let Some(path) = self.studio.selected.as_ref() else {
            return;
        };
        let reference = format!(
            "Hub Library file: {}",
            serde_json::to_string(&path.to_string_lossy()).unwrap_or_default()
        );
        let text = self.composer.read(cx).text();
        let separator = if text.is_empty() { "" } else { "\n\n" };
        if text
            .len()
            .saturating_add(reference.len())
            .saturating_add(separator.len())
            > 1024 * 1024
        {
            self.studio.error = Some("The draft is too large. Nothing was inserted.".into());
        } else {
            let text = format!("{text}{separator}{reference}");
            self.composer
                .update(cx, |entry, cx| entry.set_text(text, cx));
            self.remember_draft(cx);
            self.notice = Some("File path added to the draft without sending. This is a path reference, not an image attachment.".into());
        }
        cx.notify();
    }
    pub(super) fn studio_files_panel(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let query = self.studio.query.read(cx).text().trim().to_lowercase();
        let mut matches = studio_files_matching(
            &self.studio.listing.entries,
            &query,
            self.studio.only_outputs,
            self.studio.kind_filter,
            self.studio.turn_filter,
        );
        sort_studio_files(&mut matches, self.studio.sort_order);
        let selected = self.studio.selected.clone();
        let reporting = selected.as_ref().and_then(|path| {
            self.studio
                .listing
                .entries
                .iter()
                .find(|entry| &entry.path == path)
        });
        let source_task = selected
            .as_ref()
            .and_then(|path| {
                self.studio
                    .listing
                    .entries
                    .iter()
                    .find(|entry| &entry.path == path)
            })
            .and_then(|entry| entry.source_task);
        let preview = match &self.studio.preview {
            Some(Preview::Pdf(view)) => self.studio_pdf_panel(view, cx),
            Some(Preview::DocumentText(text)) => div().id("studio-docx-text-preview")
                .flex_1().min_h_0().overflow_y_scroll().p_3().font_family(ui::code_font())
                .text_size(px(13.)).child(truncate(text, 128 * 1024)).into_any_element(),
            Some(Preview::Text { text, markdown }) => {
                div().id("studio-text-preview").relative().child(ui::layout_probe("studio-text-preview"))
                    .flex_1().min_h_0().overflow_y_scroll().p_3().text_size(px(13.))
                    .when(text.len() > 128*1024, |el| el.child(
                        div().text_size(px(11.)).text_color(rgb(palette().muted)).pb_2()
                            .child("Preview limited to 128 KiB. Copy text includes the full loaded file.")))
                    .child(if *markdown && !self.studio.raw_text { ui::markdown::render(&truncate(text, 128*1024), "studio-file-preview") }
                        else { div().relative().child(ui::layout_probe("studio-raw-preview"))
                            .font_family(ui::code_font()).child(truncate(text,128*1024)).into_any_element() })
                    .into_any_element()
            }
            Some(Preview::Image { image, width, height }) => div().id("studio-image-preview").relative().child(ui::layout_probe("studio-image-preview"))
                .flex_1().min_h_0().min_w_0().flex().flex_col().gap_2()
                .child(div().flex().items_center().flex_wrap().gap_1()
                    .child(ui::button("studio-image-fit", "Fit", self.studio.image_zoom.is_none()).text_size(px(11.))
                        .relative().child(ui::layout_probe("studio-image-fit"))
                        .on_click(cx.listener(|this, _, _, cx| { this.studio.image_zoom = None; cx.notify(); })))
                    .child(ui::button("studio-image-actual", "100%", self.studio.image_zoom == Some(1.)).text_size(px(11.))
                        .on_click(cx.listener(|this, _, _, cx| { this.studio.image_zoom = Some(1.); cx.notify(); })))
                    .child(ui::button("studio-image-out", "-", false).aria_label("Zoom image out").text_size(px(11.))
                        .on_click(cx.listener(|this, _, _, cx| { this.studio.image_zoom = Some((this.studio.image_zoom.unwrap_or(1.) / 1.25).clamp(0.125, 4.)); cx.notify(); })))
                    .child(ui::button("studio-image-in", "+", false).aria_label("Zoom image in").text_size(px(11.))
                        .relative().child(ui::layout_probe("studio-image-in"))
                        .on_click(cx.listener(|this, _, _, cx| { this.studio.image_zoom = Some((this.studio.image_zoom.unwrap_or(1.) * 1.25).clamp(0.125, 4.)); cx.notify(); })))
                    .child(div().text_size(px(11.)).text_color(rgb(palette().muted)).child(format!("{width} × {height}{}", self.studio.image_zoom.map_or(String::new(), |zoom| format!(" · {:.0}%", zoom * 100.))))))
                .child(div().id("studio-image-viewport").flex_1().min_h_0().min_w_0().overflow_scroll().p_2()
                    .child(if let Some(zoom) = self.studio.image_zoom {
                        div().relative().child(ui::layout_probe("studio-zoomed-image"))
                            .w(px(*width as f32 * zoom)).h(px(*height as f32 * zoom)).flex_shrink_0()
                            .child(gpui::img(image.clone()).size_full().object_fit(gpui::ObjectFit::Contain)).into_any_element()
                    } else { gpui::img(image.clone()).w_full().h(px(240.)).object_fit(gpui::ObjectFit::Contain).into_any_element() }))
                .into_any_element(),
            Some(Preview::Unsupported(bytes)) => div().flex_1().min_h_0().p_4().text_size(px(12.)).text_color(rgb(palette().muted))
                .child(format!("Preview unavailable for this file ({bytes} bytes). PNG/JPEG, still WebP, DOCX/ODT/ODP/ODS/PPTX/XLSX text and UTF-8 text are supported within the preview limits. The file has not been executed or opened externally.")).into_any_element(),
            None => div().flex_1().min_h_0().p_4().text_size(px(13.)).text_color(rgb(palette().muted))
                .child(if self.studio.preview_loading { "Loading preview..." } else { "Select a file to preview it." }).into_any_element(),
        };
        div().id("studio-files-panel").relative().child(ui::layout_probe("studio-files-panel"))
            .flex().flex_col().flex_1().min_h_0().min_w_0().gap_2().p_3()
            .child(div().flex().items_center().gap_2().child(ui::icon(Glyph::Files))
                .child(div().flex_1().text_size(px(14.)).child("Hub Library"))
                .child(ui::button("studio-refresh", if self.studio.loading { "Refreshing..." } else { "Refresh" }, false)
                    .relative().child(ui::layout_probe("studio-refresh"))
                    .on_click(cx.listener(|this,_,_,cx| { if !this.studio.loading { this.refresh_studio_outputs(cx); } }))))
            .child(div().relative().child(ui::layout_probe("studio-search")).child(self.studio.query.clone()))
            .child(div().flex().items_center().gap_1().flex_wrap()
                .child(ui::button("studio-all-files","All files",!self.studio.only_outputs).text_size(px(11.)).on_click(cx.listener(|this,_,_,cx| { this.studio.only_outputs=false;cx.notify(); })))
                .child(ui::button("studio-reported","Reported outputs",self.studio.only_outputs).text_size(px(11.)).on_click(cx.listener(|this,_,_,cx| { this.studio.only_outputs=true;cx.notify(); })))
            )
            .child(div().flex().items_center().gap_1().flex_wrap()
                .child(ui::button("studio-type-all", "All types", self.studio.kind_filter == StudioKindFilter::All).text_size(px(11.)).on_click(cx.listener(|this,_,_,cx| { this.studio.kind_filter=StudioKindFilter::All;cx.notify(); })))
                .child(ui::button("studio-type-images", "Images", self.studio.kind_filter == StudioKindFilter::Images).text_size(px(11.)).relative().child(ui::layout_probe("studio-images")).on_click(cx.listener(|this,_,_,cx| { this.studio.kind_filter=StudioKindFilter::Images;cx.notify(); })))
                .child(ui::button("studio-type-documents", "Documents", self.studio.kind_filter == StudioKindFilter::Documents).text_size(px(11.)).on_click(cx.listener(|this,_,_,cx| { this.studio.kind_filter=StudioKindFilter::Documents;cx.notify(); })))
                .child(ui::button("studio-type-other", "Other", self.studio.kind_filter == StudioKindFilter::Other).text_size(px(11.)).on_click(cx.listener(|this,_,_,cx| { this.studio.kind_filter=StudioKindFilter::Other;cx.notify(); }))))
            .child(div().flex().items_center().justify_between().flex_wrap().gap_1()
                .child(div().text_size(px(11.)).text_color(rgb(palette().muted)).child(format!("Showing {} of {} files · Output attribution comes from completed tools, not proof of authorship.",matches.len(),self.studio.listing.entries.len())))
                .child(ui::button("studio-sort", format!("Sort: {}",self.studio.sort_order.label()), self.studio.sort_order != StudioSortOrder::OutputsFirst).text_size(px(11.)).relative().child(ui::layout_probe("studio-sort"))
                    .on_click(cx.listener(|this,_,_,cx| { this.studio.sort_order=this.studio.sort_order.next();cx.notify(); })))
                .when(self.studio.only_outputs || self.studio.kind_filter != StudioKindFilter::All || self.studio.turn_filter.is_some() || !query.is_empty(), |el| el.child(
                    ui::button("studio-clear-filters", "Clear filters", false).text_size(px(11.)).on_click(cx.listener(|this,_,_,cx| {
                        this.studio.only_outputs=false;
                        this.studio.kind_filter=StudioKindFilter::All;
                        this.studio.turn_filter=None;
                        this.studio.query.update(cx, |entry,cx| entry.set_text(String::new(),cx));
                        cx.notify();
                    }))
                )))
            .children(self.studio.turn_filter.map(|(_, turn)| ui::button("studio-all-turns", format!("Turn {turn} filter · Show all turns"), true).relative().child(ui::layout_probe("studio-all-turns"))
                .text_size(px(11.)).on_click(cx.listener(|this, _, _, cx| { this.studio.turn_filter = None; cx.notify(); }))))
            .children(reporting.and_then(|entry| entry.source_task.zip(entry.source_turn.as_ref())).map(|(task, turn)| {
                let number = turn.number;
                ui::button("studio-reporting-turn", format!("Reported in turn {number} · Show this turn's outputs"), self.studio.turn_filter == Some((task, number))).relative().child(ui::layout_probe("studio-reporting-turn"))
                    .text_size(px(11.)).on_click(cx.listener(move |this, _, _, cx| {
                        this.studio.turn_filter = Some((task, number)); this.studio.only_outputs = true; cx.notify();
                    }))
            }))
            .children(reporting.and_then(|entry| entry.reported_at_ms).map(|timestamp| {
                let time = chrono::DateTime::from_timestamp_millis(timestamp)
                    .map(|date| date.format("%Y-%m-%d %H:%M:%S UTC").to_string()).unwrap_or_else(|| "unknown time".into());
                div().text_size(px(11.)).text_color(rgb(palette().muted))
                    .child(format!("Latest tool report: {time}. Preview shows the current file, not a historical snapshot."))
            }))
        .children(source_task.zip(selected.clone()).map(|(source, path)| {
            ui::action(
                "library-source-thread",
                "Open reporting chat with output",
                Some(Glyph::Chat),
                false,
                cx.listener(move |this, _: &(), _, cx| {
                    this.open_studio_output_with_source(source, path.clone(), cx)
                }),
            )
        }))
            .children(self.studio.error.clone().map(|error| div().relative().child(ui::layout_probe("studio-error")).text_size(px(12.)).text_color(rgb(palette().error)).child(error)))
            .when(self.studio.listing.limited || self.studio.listing.unreadable>0, |el| el.child(div().text_size(px(11.)).text_color(rgb(palette().muted))
                .child(format!("Bounded listing{} · {} unreadable directories/files",if self.studio.listing.limited { " reached its limit" } else { "" },self.studio.listing.unreadable))))
            .child(div().id("studio-file-list").h(px(190.)).flex_shrink_0().overflow_y_scroll().flex().flex_col()
                .children(matches.iter().enumerate().map(|(index,file)| {
                    let path=file.path.clone(); let active=selected.as_ref()==Some(&path);
                    ui::action(("studio-file",index),file.path.to_string_lossy().into_owned(),Some(if image_path(&path) { Glyph::Capture } else { Glyph::Files }),active,
                        cx.listener(move |this, _: &(),_,cx| this.preview_studio_file(path.clone(),cx)))
                        .h(px(28.)).text_size(px(12.)).relative().child(ui::layout_probe_slot("studio-file",index))
                }))
                .when(matches.is_empty(), |el| el.child(div().p_3().text_size(px(12.)).text_color(rgb(palette().muted))
                    .child(if self.studio.loading { "Looking for files..." } else if self.studio.only_outputs { "No completed tool changes reference a visible file yet. All files shows other workspace content." } else if self.studio.listing.entries.is_empty() { "Files appear here after they are created in this Hub working folder." } else { "No files match the current filters. Clear filters or change the search." }))))
            .child(div().text_size(px(12.)).text_ellipsis().child(selected.as_ref().map(|p|p.to_string_lossy().into_owned()).unwrap_or_default()))
            .children(matches!(self.studio.preview, Some(Preview::DocumentText(_))).then(||
                div().text_size(px(11.)).text_color(rgb(palette().muted))
                    .child("Extracted document text · read only. Copy text copies the extraction; export saves the original document.")))
            .when(matches!(self.studio.preview, Some(Preview::Text { markdown: true, .. })), |el| el.child(
                ui::button("studio-raw-toggle", if self.studio.raw_text { "Show rendered Markdown" } else { "Show raw text" }, self.studio.raw_text)
                    .text_size(px(11.)).aria_label("Toggle raw Markdown source")
                    .relative().child(ui::layout_probe("studio-raw-toggle"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.studio.raw_text = !this.studio.raw_text;
                        cx.notify();
                    }))))
            .child(preview)
            .children(matches!(self.studio.preview, Some(Preview::Text { .. })).then(||
                ui::button("studio-history", if self.studio.history_loading { "Loading committed versions..." } else { "Committed versions" }, self.studio.history_loading)
                    .text_size(px(11.))
                    .on_click(cx.listener(|this, _, _, cx| this.open_studio_history(cx)))
            ))
            .children(self.studio.history.as_ref().map(|history| {
                div().id("studio-history-list").max_h(px(150.)).overflow_y_scroll().flex().flex_col().gap_1()
                    .child(div().text_size(px(11.)).text_color(rgb(palette().muted))
                        .child(format!("Committed snapshots · {}{}", history.commits.len(), if history.limited { " (first 50)" } else { "" })))
                    .children(history.commits.iter().enumerate().map(|(index, commit)| {
                        let id = commit.id.clone();
                        let label = format!("{} · {}", &id[..8], commit.subject.chars().take(80).collect::<String>());
                        ui::button(("studio-history-commit", index), label, self.studio.revision.as_ref().is_some_and(|revision| revision.commit == id))
                            .text_size(px(11.))
                            .on_click(cx.listener(move |this, _, _, cx| this.open_studio_revision(id.clone(), cx)))
                    }))
            }))
            .children(self.studio.revision.as_ref().map(|revision| {
                let commit = revision.commit.clone();
                div().id("studio-history-preview").flex().flex_col().gap_1()
                    .child(div().flex().items_center().gap_2()
                        .child(div().flex_1().text_size(px(11.)).child(format!("Historical snapshot {} · read only", &commit[..8])))
                        .child(ui::button("studio-history-copy", "Copy snapshot", false).text_size(px(11.))
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(revision) = &this.studio.revision {
                                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(revision.text.clone()));
                                }
                            }))))
                    .child(div().id("studio-history-text").max_h(px(180.)).overflow_y_scroll().p_2().font_family(ui::code_font())
                        .text_size(px(11.)).child(truncate(&revision.text, 128 * 1024)))
            }))
            .children(selected.as_ref().and_then(|path| {
                let versions: Vec<_> = self.studio.snapshots.iter().enumerate()
                    .filter(|(_, snapshot)| Some(snapshot.task) == self.studio.task && snapshot.path == *path)
                    .collect();
                (versions.len() > 1).then(|| div().id("studio-session-versions").flex().flex_col().gap_1()
                    .child(div().text_size(px(11.)).text_color(rgb(palette().muted))
                        .child("Durable previews · snapshots captured when this file was refreshed; retained across restart for this Hub"))
                    .child(ui::button(
                        "studio-clear-versions",
                        if self.studio.clear_versions_confirm { "Confirm clear durable previews" } else { "Clear durable previews..." },
                        self.studio.clear_versions_confirm,
                    ).text_size(px(11.)).on_click(cx.listener(|this, _, _, cx| this.clear_studio_versions(cx))))
                    .child(div().flex().flex_wrap().gap_1().children(versions.into_iter().map(|(index, snapshot)| {
                        let timestamp = chrono::DateTime::from_timestamp_millis(snapshot.captured_at_ms)
                            .map(|date| date.format("%H:%M:%S UTC").to_string()).unwrap_or_else(|| "unknown time".into());
                        ui::button(("studio-session-version", index), timestamp, self.studio.selected_snapshot == Some(index))
                            .text_size(px(11.))
                            .on_click(cx.listener(move |this, _, _, cx| { this.studio.selected_snapshot = Some(index); cx.notify(); }))
                    })))
                    .children(self.studio.selected_snapshot.and_then(|index| self.studio.snapshots.get(index))
                        .filter(|snapshot| Some(snapshot.task) == self.studio.task && snapshot.path == *path)
                        .map(|snapshot| div().flex().flex_col().gap_1()
                            .child(div().flex().flex_wrap().gap_1()
                                .child(ui::button("studio-session-copy", "Copy selected durable preview", false).text_size(px(11.))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        if let Some(text) = this.studio.selected_snapshot.and_then(|index| this.studio.snapshots.get(index)).map(|snapshot| snapshot.text.clone()) {
                                            cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
                                        }
                                    })))
                                .child(ui::button("studio-session-export", "Save selected version as...", false).text_size(px(11.))
                                    .on_click(cx.listener(|this, _, _, cx| this.save_selected_studio_version(cx)))))
                            .child(div().id("studio-session-text").max_h(px(180.)).overflow_y_scroll().p_2().font_family(ui::code_font())
                                .text_size(px(11.)).child(truncate(&snapshot.text, 128 * 1024)))))
                )
            }))
            .child(self.studio_export_controls(cx))
            .child(div().flex().items_center().gap_1().flex_wrap().border_t_1().border_color(rgb(palette().border)).pt_2()
                .child(ui::button("studio-copy-path","Copy path",false).text_size(px(11.)).on_click(cx.listener(|this,_,_,cx| {
                    if let Some(path)=&this.studio.selected { cx.write_to_clipboard(gpui::ClipboardItem::new_string(path.to_string_lossy().into_owned())); }
                })))
                .child(ui::button("studio-reference","Add path to draft",false).text_size(px(11.)).relative().child(ui::layout_probe("studio-reference"))
                    .on_click(cx.listener(|this,_,_,cx| this.studio_reference_to_draft(cx))))
                .when(matches!(self.studio.preview,Some(Preview::Text {..} | Preview::DocumentText(_))), |el| el
                    .child(ui::button("studio-copy-text","Copy text",false).text_size(px(11.)).on_click(cx.listener(|this,_,_,cx| {
                        let text = match &this.studio.preview {
                            Some(Preview::Text { text, .. } | Preview::DocumentText(text)) => Some(text.clone()),
                            _ => None,
                        };
                        if let Some(text) = text {cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));}
                    }))))
                .when(matches!(self.studio.preview,Some(Preview::Text {..})), |el| el
                    .child(ui::button("studio-edit","Open in editor",false).text_size(px(11.)).relative().child(ui::layout_probe("studio-edit"))
                        .on_click(cx.listener(|this,_,_,cx| {
                            if let Some(path)=this.studio.selected.clone() { this.set_panel(Panel::Files,cx); this.open_file(path,cx); }
                        })))))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        StudioKindFilter, StudioSortOrder, restored_output_is_attributed, sort_studio_files,
        studio_files_matching, take_reopen_path,
    };
    use std::path::PathBuf;
    use synara_core::TaskId;
    use synara_workspace::{StudioFile, StudioOutputTurn};

    fn file(
        path: &str,
        bytes: u64,
        reported_at_ms: Option<i64>,
        source_task: Option<TaskId>,
        source_turn: Option<usize>,
    ) -> StudioFile {
        StudioFile {
            path: PathBuf::from(path),
            bytes,
            reported_output: reported_at_ms.is_some(),
            source_task,
            source_turn: source_turn.map(|number| StudioOutputTurn {
                number,
                started_at_ms: number as i64,
            }),
            reported_at_ms,
        }
    }

    #[test]
    fn reopening_output_is_scoped_to_the_requested_source_task() {
        let task = TaskId::new();
        let other = TaskId::new();
        let path = PathBuf::from("reports/result.md");

        assert_eq!(
            take_reopen_path(task, Some((task, path.clone()))),
            Some(path)
        );
        assert_eq!(
            take_reopen_path(task, Some((other, PathBuf::from("reports/other.md")))),
            None
        );
    }

    #[test]
    fn restored_output_must_still_be_attributed_to_the_reporting_task() {
        let task = TaskId::new();
        let other = TaskId::new();
        let path = PathBuf::from("reports/result.md");
        let entry = |source_task| StudioFile {
            path: path.clone(),
            bytes: 32,
            reported_output: true,
            source_task,
            source_turn: None,
            reported_at_ms: None,
        };

        assert!(restored_output_is_attributed(
            task,
            &path,
            &[entry(Some(task))]
        ));
        assert!(!restored_output_is_attributed(
            task,
            &path,
            &[entry(Some(other))]
        ));
        assert!(!restored_output_is_attributed(task, &path, &[entry(None)]));
    }

    #[test]
    fn library_facets_combine_search_type_output_and_reporting_turn() {
        let task = TaskId::new();
        let other_task = TaskId::new();
        let entries = vec![
            file("images/hero.WEBP", 20, Some(30), Some(task), Some(4)),
            file("notes/plan.md", 40, Some(20), Some(task), Some(4)),
            file("images/draft.png", 50, None, None, None),
            file("images/other.png", 60, Some(10), Some(other_task), Some(2)),
        ];

        let matches = studio_files_matching(
            &entries,
            "hero",
            true,
            StudioKindFilter::Images,
            Some((task, 4)),
        );

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].path, PathBuf::from("images/hero.WEBP"));
        assert!(
            studio_files_matching(&entries, "", false, StudioKindFilter::Documents, None)
                .iter()
                .any(|entry| entry.path == std::path::Path::new("notes/plan.md"))
        );
    }

    #[test]
    fn library_sort_orders_are_deterministic_and_keep_unknown_reports_last() {
        let entries = [
            file("zeta.md", 12, Some(100), None, None),
            file("Alpha.md", 12, Some(300), None, None),
            file("large.bin", 80, None, None, None),
        ];
        let paths = |order| {
            let mut matches = entries.iter().collect::<Vec<_>>();
            sort_studio_files(&mut matches, order);
            matches
                .into_iter()
                .map(|entry| entry.path.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
        };

        assert_eq!(
            paths(StudioSortOrder::Name),
            ["Alpha.md", "large.bin", "zeta.md"]
        );
        assert_eq!(
            paths(StudioSortOrder::LatestReport),
            ["Alpha.md", "zeta.md", "large.bin"]
        );
        assert_eq!(
            paths(StudioSortOrder::Largest),
            ["large.bin", "Alpha.md", "zeta.md"]
        );
        assert_eq!(
            paths(StudioSortOrder::OutputsFirst),
            ["Alpha.md", "zeta.md", "large.bin"]
        );
    }
}
