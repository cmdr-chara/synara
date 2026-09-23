//! Explicit file actions over the existing contained, version-checked services.
//! Modal ownership prevents navigation or editing during a filesystem mutation.
use super::*;
use crate::ui::{self, Glyph, palette};
use gpui::FocusHandle;
use synara_runtime::{FileEntry, SearchMatch, validate_relative};

#[derive(Clone, Copy, PartialEq)]
pub(super) enum FileAction {
    CreateFile,
    CreateFolder,
    Rename,
    Delete,
}
impl FileAction {
    fn title(self) -> &'static str {
        match self {
            Self::CreateFile => "New file",
            Self::CreateFolder => "New folder",
            Self::Rename => "Rename file",
            Self::Delete => "Delete file",
        }
    }
}
struct FileDialog {
    action: FileAction,
    directory: PathBuf,
    source: Option<Document>,
    project: ProjectId,
    target: WorkspaceTarget,
    input: Entity<TextEntry>,
    focus: FocusHandle,
    previous: Option<FocusHandle>,
    busy: bool,
    error: Option<String>,
    needs_focus: bool,
}
pub(super) enum ExplorerReply {
    Mutated {
        generation: u64,
        result: Result<Option<Document>, String>,
    },
    Search {
        generation: u64,
        project: Option<ProjectId>,
        directory: PathBuf,
        result: Result<Vec<SearchMatch>, String>,
    },
    NameSearch {
        generation: u64,
        project: Option<ProjectId>,
        scope: NameSearchScope,
        result: Result<Vec<FileEntry>, String>,
    },
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum SearchKind {
    Names,
    Contents,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct NameSearchScope {
    project: ProjectId,
    workspace: WorkspaceId,
    location: WorkspaceLocation,
    root: PathBuf,
}
pub(super) struct ExplorerState {
    dialog: Option<FileDialog>,
    generation: u64,
    query: Entity<TextEntry>,
    pub search_open: bool,
    searching: bool,
    results: Vec<SearchMatch>,
    paths: Vec<FileEntry>,
    paths_scope: Option<NameSearchScope>,
    selected_result: usize,
    error: Option<String>,
    searched: bool,
    project_wide: bool,
    search_kind: SearchKind,
    previous_focus: Option<FocusHandle>,
    _subscription: Subscription,
}
impl ExplorerState {
    pub fn new(cx: &mut Context<Shell>) -> Self {
        let query = cx.new(|cx| TextEntry::new("Search files...", EntryMode::SingleLine, 32., cx));
        let subscription = cx.subscribe(&query, |this, _, event, cx| {
            match event {
                EntryEvent::Submit => this.search_explorer(cx),
                EntryEvent::Changed => {
                    this.explorer.reset_search();
                }
                _ => {}
            }
            cx.notify();
        });
        Self {
            dialog: None,
            generation: 0,
            query,
            search_open: false,
            searching: false,
            results: vec![],
            paths: vec![],
            paths_scope: None,
            selected_result: 0,
            error: None,
            searched: false,
            project_wide: true,
            search_kind: SearchKind::Contents,
            previous_focus: None,
            _subscription: subscription,
        }
    }
    pub fn modal_open(&self) -> bool {
        self.dialog.is_some()
    }
    pub fn reset_search(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.results.clear();
        self.paths.clear();
        self.paths_scope = None;
        self.selected_result = 0;
        self.searching = false;
        self.error = None;
        self.searched = false;
    }
}
fn file_name(name: &str) -> Result<&str, &'static str> {
    if name.is_empty()
        || name != name.trim()
        || name.len() > 240
        || matches!(name, "." | "..")
        || name.chars().any(|ch| {
            ch.is_control() || matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
        })
        || name.ends_with('.')
    {
        return Err("Enter one filename, without path separators or control characters.");
    }
    // Avoid names which Windows interprets as devices even with an extension.
    let stem = name
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && matches!(stem.as_bytes()[3], b'1'..=b'9'))
    {
        return Err("That name is reserved by the operating system.");
    }
    Ok(name)
}
impl Shell {
    pub(super) fn open_file_name_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open_explorer_search(SearchKind::Names, window, cx);
    }

    pub(super) fn open_content_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open_explorer_search(SearchKind::Contents, window, cx);
    }

    fn open_explorer_search(
        &mut self,
        kind: SearchKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_panel(Panel::Files, cx);
        if self.panel != Panel::Files {
            return;
        }
        self.editors.tree_visible = true;
        self.explorer.search_kind = kind;
        self.explorer.search_open = true;
        self.explorer.reset_search();
        window.focus(&self.explorer.query.read(cx).focus_handle(cx), cx);
        cx.notify();
    }

    pub(super) fn explorer_shortcut(
        &mut self,
        event: &gpui::KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let modifiers = event.keystroke.modifiers;
        if event.prefer_character_input
            || event.is_held
            || modifiers.alt
            || !(modifiers.control || modifiers.platform)
            || self
                .terminal_view
                .read(cx)
                .focus_handle(cx)
                .is_focused(window)
            || self.composer.read(cx).is_composing()
            || self.editor.read(cx).is_composing()
        {
            return false;
        }
        let kind = match (event.keystroke.key.as_str(), modifiers.shift) {
            ("p", false) => SearchKind::Names,
            ("f", true) => SearchKind::Contents,
            _ => return false,
        };
        self.open_explorer_search(kind, window, cx);
        true
    }

    pub(super) fn open_file_action(
        &mut self,
        action: FileAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.explorer.modal_open() || self.saving || self.close != CloseState::Open {
            return;
        }
        if self.dirty(cx) {
            self.error = Some(
                "Save or discard the edited file before creating, renaming or deleting files."
                    .into(),
            );
            cx.notify();
            return;
        }
        let (Some(target), Some(project)) = (self.workspace_target(), self.project) else {
            return;
        };
        let source = if matches!(action, FileAction::Rename | FileAction::Delete) {
            let Some(source) = self.document.clone() else {
                return;
            };
            Some(source)
        } else {
            None
        };
        let initial = if action == FileAction::Rename {
            source
                .as_ref()
                .and_then(|d| d.path.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or_default()
        } else {
            ""
        }
        .to_owned();
        let input = cx.new(|cx| {
            let mut entry = TextEntry::new("Filename", EntryMode::SingleLine, 36., cx);
            entry.set_text(initial, cx);
            entry
        });
        let directory = source
            .as_ref()
            .and_then(|d| d.path.parent())
            .map(PathBuf::from)
            .unwrap_or_else(|| self.directory.clone());
        self.controls.retire();
        self.chat_tools.retire();
        self.environment.retire_popup();
        self.navigation.menu_open = false;
        self.settings.popup = None;
        self.focus_composer = false;
        self.explorer.generation = self.explorer.generation.wrapping_add(1);
        self.explorer.dialog = Some(FileDialog {
            action,
            directory,
            source,
            project,
            target,
            input,
            focus: cx.focus_handle(),
            previous: window.focused(cx),
            busy: false,
            error: None,
            needs_focus: true,
        });
        cx.notify();
    }
    fn dismiss_file_action(&mut self, cx: &mut Context<Self>) {
        if self.explorer.dialog.as_ref().is_some_and(|d| d.busy) {
            return;
        }
        if let Some(dialog) = self.explorer.dialog.take() {
            self.explorer.previous_focus = dialog.previous;
        }
        cx.notify();
    }
    fn execute_file_action(&mut self, cx: &mut Context<Self>) {
        let Some(dialog) = &self.explorer.dialog else {
            return;
        };
        if dialog.busy || self.saving || self.dirty(cx) || self.project != Some(dialog.project) {
            return;
        }
        let name = dialog.input.read(cx).text().to_owned();
        if dialog.action != FileAction::Delete
            && let Err(error) = file_name(&name)
        {
            self.explorer.dialog.as_mut().unwrap().error = Some(error.into());
            cx.notify();
            return;
        }
        let action = dialog.action;
        let destination = dialog.directory.join(name);
        let source = dialog.source.clone();
        let target = dialog.target.clone();
        let generation = self.explorer.generation;
        let service = self.controller.workspace.clone();
        self.explorer.dialog.as_mut().unwrap().busy = true;
        self.job(async move {
            let result = async move {
                match target {
                    WorkspaceTarget::Local { root } => match action {
                        FileAction::CreateFile => {
                            create_document(root, destination, String::new(), false)
                                .await
                                .map(Some)
                        }
                        FileAction::CreateFolder => {
                            create_directory(root, destination).await.map(|_| None)
                        }
                        FileAction::Rename => rename_document(
                            root,
                            source.ok_or(WorkspaceError::NotFound)?,
                            destination,
                        )
                        .await
                        .map(Some),
                        FileAction::Delete => {
                            delete_document(root, source.ok_or(WorkspaceError::NotFound)?)
                                .await
                                .map(|_| None)
                        }
                    },
                    WorkspaceTarget::Ssh { workspace, root } => {
                        let filesystem = remote_filesystem(service, workspace, root).await?;
                        match action {
                            FileAction::CreateFile => create_remote_document(
                                filesystem,
                                destination,
                                String::new(),
                                false,
                            )
                            .await
                            .map(Some),
                            FileAction::CreateFolder => {
                                create_remote_directory(filesystem, destination)
                                    .await
                                    .map(|_| None)
                            }
                            FileAction::Rename => rename_remote_document(
                                filesystem,
                                source.ok_or(WorkspaceError::NotFound)?,
                                destination,
                            )
                            .await
                            .map(Some),
                            FileAction::Delete => delete_remote_document(
                                filesystem,
                                source.ok_or(WorkspaceError::NotFound)?,
                            )
                            .await
                            .map(|_| None),
                        }
                    }
                }
            }
            .await;
            Ok(Update::Explorer(Box::new(ExplorerReply::Mutated {
                generation,
                result: result.map_err(|error| error.to_string()),
            })))
        });
        cx.notify();
    }
    pub(super) fn explorer_reply(&mut self, reply: ExplorerReply, cx: &mut Context<Self>) {
        match reply {
            ExplorerReply::Mutated { generation, result } => {
                if generation != self.explorer.generation {
                    return;
                }
                let Some(dialog) = self.explorer.dialog.as_mut() else {
                    return;
                };
                dialog.busy = false;
                match result {
                    Err(error) => {
                        dialog.error = Some(format!(
                            "File operation failed: {error}. Reload an externally changed file before retrying."
                        ))
                    }
                    Ok(document) => {
                        let action = dialog.action;
                        let project = dialog.project;
                        self.dismiss_file_action(cx);
                        if self.project == Some(project) {
                            if let Some(document) = document {
                                if action == FileAction::Rename {
                                    self.replace_editor_document(document, cx);
                                } else {
                                    self.install_editor_document(document, cx);
                                }
                            } else if action == FileAction::Delete {
                                self.remove_active_editor(cx);
                            }
                            self.refresh_files();
                            self.explorer.reset_search();
                        }
                        self.notice = Some(format!(
                            "{} completed in the selected workspace.",
                            action.title()
                        ));
                    }
                }
            }
            ExplorerReply::Search {
                generation,
                project,
                directory,
                result,
            } => {
                if generation != self.explorer.generation
                    || self.project != project
                    || self.directory != directory
                {
                    return;
                }
                self.explorer.searching = false;
                self.explorer.searched = true;
                match result {
                    Ok(results) => self.explorer.results = results,
                    Err(error) => self.explorer.error = Some(error),
                }
            }
            ExplorerReply::NameSearch {
                generation,
                project,
                scope,
                result,
            } => {
                if generation != self.explorer.generation
                    || self.project != project
                    || self.current_name_search_scope().as_ref() != Some(&scope)
                {
                    return;
                }
                self.explorer.searching = false;
                self.explorer.searched = true;
                match result {
                    Ok(paths) => {
                        self.explorer.paths = paths;
                        self.explorer.paths_scope = Some(scope);
                    }
                    Err(error) => self.explorer.error = Some(error),
                }
            }
        }
        cx.notify();
    }
    pub(super) fn restore_explorer_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(dialog) = &mut self.explorer.dialog
            && dialog.needs_focus
        {
            let focus = if dialog.action == FileAction::Delete {
                dialog.focus.clone()
            } else {
                dialog.input.read(cx).focus_handle(cx)
            };
            window.focus(&focus, cx);
            dialog.needs_focus = false;
        } else if let Some(focus) = self.explorer.previous_focus.take() {
            window.focus(&focus, cx);
        }
    }
    fn search_explorer(&mut self, cx: &mut Context<Self>) {
        let query = self.explorer.query.read(cx).text().trim().to_owned();
        if query.is_empty() || self.explorer.searching || self.explorer.modal_open() {
            return;
        }
        let Some(target) = self.workspace_target() else {
            self.explorer.error = Some("Choose a project workspace before searching files.".into());
            cx.notify();
            return;
        };
        let name_scope = (self.explorer.search_kind == SearchKind::Names)
            .then(|| self.current_name_search_scope())
            .flatten();
        if self.explorer.search_kind == SearchKind::Names && name_scope.is_none() {
            self.explorer.error = Some("Choose a project workspace before searching files.".into());
            cx.notify();
            return;
        }
        self.explorer.reset_search();
        self.explorer.searching = true;
        let generation = self.explorer.generation;
        let project = self.project;
        let kind = self.explorer.search_kind;
        let directory = self.directory.clone();
        let search_directory = if self.explorer.project_wide {
            PathBuf::new()
        } else {
            directory.clone()
        };
        let service = self.controller.workspace.clone();
        if kind == SearchKind::Names {
            let Some(scope) = name_scope else {
                return;
            };
            self.job(async move {
                let result = match target {
                    WorkspaceTarget::Local { root } => search_file_entries(root, query, 200).await,
                    WorkspaceTarget::Ssh { workspace, root } => {
                        let filesystem = remote_filesystem(service, workspace, root).await?;
                        search_remote_file_entries(filesystem, query, 200).await
                    }
                };
                Ok(Update::Explorer(Box::new(ExplorerReply::NameSearch {
                    generation,
                    project,
                    scope,
                    result: result.map_err(|error| error.to_string()),
                })))
            });
            cx.notify();
            return;
        }
        self.job(async move {
            let result = async {
                match target {
                    WorkspaceTarget::Local { root } => {
                        search_files(root, search_directory, query, 200).await
                    }
                    WorkspaceTarget::Ssh { workspace, root } => {
                        search_remote_files(
                            remote_filesystem(service, workspace, root).await?,
                            search_directory,
                            query,
                            200,
                        )
                        .await
                    }
                }
            }
            .await;
            Ok(Update::Explorer(Box::new(ExplorerReply::Search {
                generation,
                project,
                directory,
                result: result.map_err(|error| error.to_string()),
            })))
        });
        cx.notify();
    }

    fn search_result_key(
        &mut self,
        event: &gpui::KeyDownEvent,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.explorer.search_open
            || self.explorer.query.read(cx).is_composing()
            || !self
                .explorer
                .query
                .read(cx)
                .focus_handle(cx)
                .is_focused(window)
        {
            return false;
        }
        let modifiers = event.keystroke.modifiers;
        if modifiers.control || modifiers.platform || modifiers.alt || modifiers.shift {
            return false;
        }
        if event.keystroke.key == "escape" {
            self.explorer.search_open = false;
            self.explorer.reset_search();
            cx.notify();
            return true;
        }
        let count = if self.explorer.search_kind == SearchKind::Names {
            self.explorer.paths.len()
        } else {
            self.explorer.results.len()
        };
        if count == 0 {
            return false;
        }
        match event.keystroke.key.as_str() {
            "down" => self.explorer.selected_result = (self.explorer.selected_result + 1) % count,
            "up" => {
                self.explorer.selected_result = (self.explorer.selected_result + count - 1) % count
            }
            "enter" => {
                if self.explorer.search_kind == SearchKind::Names {
                    if let Some(entry) = self
                        .explorer
                        .paths
                        .get(self.explorer.selected_result)
                        .cloned()
                    {
                        self.open_name_search_result(entry, cx);
                    }
                } else if let Some(result) =
                    self.explorer.results.get(self.explorer.selected_result)
                {
                    let path = result.relative_path.clone();
                    let Ok(line) = usize::try_from(result.line) else {
                        return true;
                    };
                    self.editors.jump_after_open = Some((path.clone(), line));
                    self.open_file(path, cx);
                }
            }
            _ => return false,
        }
        cx.notify();
        true
    }

    fn open_name_search_result(&mut self, entry: FileEntry, cx: &mut Context<Self>) {
        let current_scope = self.current_name_search_scope();
        if self.explorer.paths_scope.as_ref() != current_scope.as_ref() {
            self.explorer.reset_search();
            self.explorer.error =
                Some("The project workspace changed. Run the search again.".into());
            cx.notify();
            return;
        }
        if validate_relative(&entry.relative_path).is_err() {
            self.error = Some("The search result is not a valid workspace path.".into());
            cx.notify();
            return;
        }
        if entry.symlink {
            self.error = Some("Symlink navigation is disabled at the workspace boundary.".into());
            cx.notify();
            return;
        }
        if !entry.directory {
            self.open_file(entry.relative_path, cx);
            return;
        }
        if self.explorer.modal_open() || self.saving || self.close != CloseState::Open {
            return;
        }
        self.directory = entry.relative_path;
        self.file_page = 0;
        self.editors.tree_visible = true;
        self.explorer.search_open = false;
        self.explorer.reset_search();
        self.file_search.update(cx, |input, cx| input.clear(cx));
        self.refresh_files();
        cx.notify();
    }

    fn current_name_search_scope(&self) -> Option<NameSearchScope> {
        let project_id = self.project?;
        let project = self
            .catalog
            .projects
            .iter()
            .find(|project| project.id == project_id)?;
        let workspace = self
            .catalog
            .workspaces
            .iter()
            .find(|workspace| workspace.id == project.workspace_id)?;
        let target = self.workspace_target()?;
        Some(NameSearchScope {
            project: project_id,
            workspace: workspace.id,
            location: workspace.location.clone(),
            root: target.root().clone(),
        })
    }
    fn editor_to_draft(&mut self, selection: bool, cx: &mut Context<Self>) {
        let Some(document) = &self.document else {
            return;
        };
        if self.selected.is_none() || self.loading_task.is_some() || self.close != CloseState::Open
        {
            return;
        }
        let path = serde_json::to_string(&document.path.to_string_lossy()).unwrap_or_default();
        let addition = if selection {
            let text = self.editor.read(cx).selected_text();
            if text.is_empty() {
                self.notice = Some("Select text in the editor first.".into());
                cx.notify();
                return;
            }
            if text.len() > 128 * 1024 {
                self.notice = Some("Select at most 128 KiB to add to the draft.".into());
                cx.notify();
                return;
            }
            format!(
                "From workspace file {path}:\n{}",
                text.split('\n')
                    .map(|line| format!("> {line}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        } else {
            format!("Workspace file: {path}")
        };
        let draft = self.composer.read(cx).text();
        if draft.len().saturating_add(addition.len()).saturating_add(2) > 1024 * 1024 {
            self.error = Some("The combined draft exceeds 1 MiB. Nothing was changed.".into());
            cx.notify();
            return;
        }
        let combined = if draft.is_empty() {
            addition
        } else {
            format!("{draft}\n\n{addition}")
        };
        self.composer
            .update(cx, |entry, cx| entry.set_text(combined, cx));
        self.remember_draft(cx);
        self.notice =
            Some("Added to the chat draft without sending. The open file was not changed.".into());
        cx.notify();
    }
    pub(super) fn explorer_toolbar(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        div()
            .px_2()
            .flex()
            .items_center()
            .gap_1()
            .child(ui::chrome_button(
                "file-new",
                "New file",
                Glyph::Plus,
                false,
                cx.listener(|this, _: &(), window, cx| {
                    this.open_file_action(FileAction::CreateFile, window, cx)
                }),
            ))
            .child(ui::chrome_button(
                "folder-new",
                "New folder",
                Glyph::Folder,
                false,
                cx.listener(|this, _: &(), window, cx| {
                    this.open_file_action(FileAction::CreateFolder, window, cx)
                }),
            ))
            .child(ui::chrome_button(
                "files-content-search",
                "Search file contents",
                Glyph::Search,
                false,
                cx.listener(|this, _: &(), window, cx| {
                    this.open_explorer_search(SearchKind::Contents, window, cx);
                }),
            ))
            .into_any_element()
    }
    pub(super) fn editor_actions(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let path = self
            .document
            .as_ref()
            .map(|d| d.path.display().to_string())
            .unwrap_or_default();
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_1()
            .child(ui::chrome_button(
                "file-copy-path",
                "Copy relative path",
                Glyph::Copy,
                false,
                move |_: &(), _, cx| {
                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(path.clone()))
                },
            ))
            .child(ui::chrome_button(
                "file-to-chat",
                "Add file reference to draft",
                Glyph::Chat,
                false,
                cx.listener(|this, _: &(), _, cx| this.editor_to_draft(false, cx)),
            ))
            .child(ui::chrome_button(
                "selection-to-chat",
                "Quote selected text in draft",
                Glyph::Compose,
                false,
                cx.listener(|this, _: &(), _, cx| this.editor_to_draft(true, cx)),
            ))
            .child(
                ui::button("file-rename", "Rename", false)
                    .relative()
                    .child(ui::layout_probe("file-rename"))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.open_file_action(FileAction::Rename, window, cx)
                    })),
            )
            .child(
                ui::button("file-delete", "Delete...", false)
                    .relative()
                    .child(ui::layout_probe("file-delete"))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.open_file_action(FileAction::Delete, window, cx)
                    })),
            )
            .into_any_element()
    }
    pub(super) fn explorer_search_panel(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let state = &self.explorer;
        let names = state.search_kind == SearchKind::Names;
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .gap_1()
            .capture_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                if this.search_result_key(event, window, cx) {
                    cx.stop_propagation();
                }
            }))
            .child(
                div()
                    .px_2()
                    .relative()
                    .child(ui::layout_probe("file-content-query"))
                    .child(state.query.clone()),
            )
            .child(
                div()
                    .flex()
                    .gap_1()
                    .child(
                        ui::button("file-search-names", "Files and folders", names).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.explorer.search_kind = SearchKind::Names;
                                this.explorer.reset_search();
                                cx.notify();
                            }),
                        ),
                    )
                    .child(
                        ui::button("file-search-contents", "Contents", !names).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.explorer.search_kind = SearchKind::Contents;
                                this.explorer.reset_search();
                                cx.notify();
                            }),
                        ),
                    ),
            )
            .children((!names).then(|| {
                ui::button(
                    "file-search-scope",
                    if state.project_wide {
                        "Scope: project"
                    } else {
                        "Scope: folder"
                    },
                    state.project_wide,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.explorer.project_wide = !this.explorer.project_wide;
                    this.explorer.reset_search();
                    cx.notify();
                }))
            }))
            .child(
                ui::button(
                    "file-search-run",
                    if state.searching {
                        "Searching..."
                    } else {
                        if names {
                            "Search names"
                        } else {
                            "Search contents"
                        }
                    },
                    false,
                )
                .relative()
                .child(ui::layout_probe("file-search-run"))
                .on_click(cx.listener(|this, _, _, cx| this.search_explorer(cx))),
            )
            .child(
                div()
                    .px_2()
                    .text_xs()
                    .text_color(rgb(palette().muted))
                    .child(if state.searched {
                        format!(
                            "{} matches{}",
                            if names {
                                state.paths.len()
                            } else {
                                state.results.len()
                            },
                            if (if names {
                                state.paths.len()
                            } else {
                                state.results.len()
                            }) >= 200
                            {
                                " (limit reached)"
                            } else {
                                ""
                            }
                        )
                    } else {
                        if names {
                            "Search file and folder names across the project. Press Enter.".into()
                        } else {
                            format!(
                                "Literal search in the {}. Press Enter.",
                                if state.project_wide {
                                    "project"
                                } else {
                                    "current folder"
                                }
                            )
                        }
                    }),
            )
            .children(
                state
                    .error
                    .as_ref()
                    .map(|error| div().px_2().text_xs().child(error.clone())),
            )
            .child(
                div()
                    .id("file-search-results")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .children(state.paths.iter().enumerate().map(|(i, entry)| {
                        let entry = entry.clone();
                        let path_label = entry.relative_path.display().to_string();
                        let kind_label = if entry.directory { "Directory" } else { "File" };
                        let label = format!("{kind_label}: {path_label}");
                        ui::button_shell(
                            SharedString::from(format!("file-name-match-{i}")),
                            label.clone(),
                            i == state.selected_result,
                        )
                        .w_full()
                        .min_w_0()
                        .relative()
                        .child(ui::layout_probe_slot("file-name-match", i))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(ui::icon(if entry.directory {
                                    Glyph::Folder
                                } else {
                                    Glyph::Files
                                }))
                                .child(div().text_ellipsis().text_xs().child(label)),
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.open_name_search_result(entry.clone(), cx);
                        }))
                    }))
                    .children(state.results.iter().enumerate().map(|(i, result)| {
                        let path = result.relative_path.clone();
                        let line = result.line;
                        ui::button_shell(
                            SharedString::from(format!("file-match-{i}")),
                            result.relative_path.display().to_string(),
                            i == state.selected_result,
                        )
                        .flex()
                        .flex_col()
                        .items_start()
                        .min_w_0()
                        .relative()
                        .child(ui::layout_probe_slot("file-content-match", i))
                        .child(div().w_full().text_ellipsis().text_xs().child(format!(
                            "{}:{}",
                            result.relative_path.display(),
                            line
                        )))
                        .child(
                            div()
                                .w_full()
                                .text_ellipsis()
                                .text_xs()
                                .text_color(rgb(palette().muted))
                                .child(result.preview.clone()),
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.editors.jump_after_open = Some((path.clone(), line as usize));
                            this.open_file(path.clone(), cx);
                            this.notice =
                                Some(format!("Match in {} at line {line}", path.display()));
                        }))
                    })),
            )
            .into_any_element()
    }
    pub(super) fn file_action_overlay(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(dialog) = &self.explorer.dialog else {
            return div().into_any_element();
        };
        let action = dialog.action;
        let busy = dialog.busy;
        let deleting = action == FileAction::Delete;
        let source = dialog
            .source
            .as_ref()
            .map(|d| d.path.display().to_string())
            .unwrap_or_else(|| dialog.directory.display().to_string());
        div().absolute().inset_0().bg(gpui::rgba(0x00000088)).occlude().flex().items_center().justify_center().p_4()
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(div().id("file-action-dialog").role(gpui::Role::Dialog).aria_label(action.title()).track_focus(&dialog.focus).tab_group()
                .w(px(460.)).max_w_full().p_4().rounded_xl().bg(rgb(palette().overlay)).border_1().border_color(rgb(palette().border)).flex().flex_col().gap_3()
                .capture_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, _, cx| {
                    if busy { cx.stop_propagation(); return; }
                    if this.explorer.dialog.as_ref().is_some_and(|d| d.input.read(cx).is_composing()) || event.prefer_character_input { return; }
                    if event.keystroke.key == "escape" { this.dismiss_file_action(cx); cx.stop_propagation(); }
                    // Delete requires its deliberately focused confirmation button,
                    // never an Enter which opened the dialog or is being held.
                }))
                .child(ui::layout_probe("file-action-dialog"))
                .child(div().text_lg().child(action.title()))
                .child(div().text_sm().child(if deleting { format!("Permanently delete {source}? This does not use the Trash. Externally changed files are refused.") } else { format!("In workspace folder: {}", dialog.directory.display()) }))
                .when(!deleting, |el| el.child(if busy { div().child(dialog.input.read(cx).text().to_owned()).into_any_element() } else { div().relative().child(ui::layout_probe("file-action-name")).child(dialog.input.clone()).into_any_element() }))
                .children(dialog.error.as_ref().map(|e| div().text_color(rgb(palette().error)).text_sm().child(e.clone())))
                .child(div().flex().justify_end().gap_2()
                    .child(ui::button("file-action-cancel", "Cancel", false).relative().child(ui::layout_probe("file-action-cancel")).on_click(cx.listener(|this, _, _, cx| this.dismiss_file_action(cx))))
                    .child(ui::button("file-action-confirm", if busy { "Working..." } else if deleting { "Delete file" } else { "Confirm" }, false).relative().child(ui::layout_probe("file-action-confirm"))
                        .on_click(cx.listener(|this, _, _, cx| this.execute_file_action(cx)))))).into_any_element()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names_cannot_change_destination_scope_or_target_platform_devices() {
        for name in [
            "", ".", "..", "../x", "/tmp/x", "a/b", "a\\b", "x\n", " x", "x ", "x.", "NUL",
            "COM1.txt", "c:foo",
        ] {
            assert!(file_name(name).is_err(), "{name}");
        }
        for name in [
            "notes.md",
            "Caffè.txt",
            "日本語",
            ".gitignore",
            "two words.txt",
        ] {
            assert_eq!(file_name(name).unwrap(), name);
        }
    }
}
