//! Hubs are optional context workspaces, not another provider or execution host.
use super::*;
use crate::ui::{self, Glyph, palette};
mod view;

pub(super) enum Reply {
    Loaded(Result<(Vec<HubSummary>, Catalog), String>),
    Created(Result<(HubProfile, Task), String>, u64),
    Saved(Result<HubProfile, String>),
    Thread(Result<Task, String>, u64),
}
pub(super) struct HubState {
    pub rows: Vec<HubSummary>,
    pub selected: Option<ProjectId>,
    loaded: bool,
    loading: bool,
    reload: bool,
    pub saving: bool,
    creating: bool,
    editing: bool,
    original: Option<HubProfile>,
    name: Entity<TextEntry>,
    instructions: Entity<TextEntry>,
    query: Entity<TextEntry>,
    folder: Option<PathBuf>,
    picker: bool,
    error: Option<String>,
    focus_form: bool,
    pending_selection: Option<(TaskId, u64, bool)>,
    _subscriptions: Vec<Subscription>,
}
impl HubState {
    pub fn new(cx: &mut Context<Shell>) -> Self {
        let name = cx.new(|cx| TextEntry::new("Hub name", EntryMode::SingleLine, 36., cx));
        let instructions = cx.new(|cx| {
            TextEntry::new(
                "Project instructions for new threads",
                EntryMode::Editor,
                140.,
                cx,
            )
        });
        let query = cx.new(|cx| TextEntry::new("Find a Hub", EntryMode::SingleLine, 32., cx));
        let subscriptions = [&name, &instructions, &query]
            .into_iter()
            .map(|entry| cx.subscribe(entry, |_, _, _, cx| cx.notify()))
            .collect();
        Self {
            rows: Vec::new(),
            selected: None,
            loaded: false,
            loading: false,
            reload: false,
            saving: false,
            creating: false,
            editing: false,
            original: None,
            name,
            instructions,
            query,
            folder: None,
            picker: false,
            error: None,
            focus_form: false,
            pending_selection: None,
            _subscriptions: subscriptions,
        }
    }
    pub fn error_message(&self) -> Option<&str> {
        self.error.as_deref()
    }
    pub fn pending(&self, cx: &App) -> bool {
        self.saving
            || self.creating
            || self.picker
            || self.dirty(cx)
            || self.editing
                && [&self.name, &self.instructions]
                    .iter()
                    .any(|entry| entry.read(cx).is_composing())
    }
    fn dirty(&self, cx: &App) -> bool {
        if !self.editing {
            return false;
        }
        let fields = [self.name.read(cx).text(), self.instructions.read(cx).text()];
        match &self.original {
            Some(profile) => fields != [profile.name.as_str(), profile.instructions.as_str()],
            None => fields.iter().any(|value| !value.is_empty()) || self.folder.is_some(),
        }
    }
}
impl Shell {
    pub(super) fn load_hubs(&mut self) {
        if self.hubs.loading {
            self.hubs.reload = true;
            return;
        }
        self.hubs.loading = true;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            let result = async { Ok((workspace.hubs().await?, workspace.catalog().await?)) }.await;
            Ok(Update::Hubs(Box::new(Reply::Loaded(
                result.map_err(|error: WorkspaceError| error.to_string()),
            ))))
        });
    }
    pub(super) fn hub_navigation_blocked(&mut self, cx: &mut Context<Self>) -> bool {
        if self.revision_navigation_blocked(cx) {
            return true;
        }
        self.hub_editors_blocked(cx)
    }
    // Sending in place is not navigation. Keep the reviewed goal lease, but retain
    // every unsaved-editor guard used by navigation.
    pub(super) fn hub_send_blocked(&mut self, cx: &mut Context<Self>) -> bool {
        if self.goal_send_blocked(cx) || self.revision_navigation_except_goal(cx) {
            return true;
        }
        self.hub_editors_blocked(cx)
    }
    fn hub_editors_blocked(&mut self, cx: &mut Context<Self>) -> bool {
        if self.followup_navigation_blocked(cx) {
            return true;
        }
        if !self.hubs.pending(cx) {
            return false;
        }
        self.error = Some("Save or explicitly discard the Hub editor before leaving it.".into());
        cx.notify();
        true
    }
    fn hub_profile(&self) -> Option<&HubProfile> {
        self.hubs
            .rows
            .iter()
            .find(|hub| Some(hub.profile.project) == self.hubs.selected)
            .map(|hub| &hub.profile)
    }
    pub(super) fn open_hub(&mut self, project: ProjectId, cx: &mut Context<Self>) {
        if self.hub_navigation_blocked(cx) {
            return;
        }
        if !self
            .hubs
            .rows
            .iter()
            .any(|hub| hub.profile.project == project)
        {
            return;
        }
        let task = self
            .catalog
            .tasks
            .iter()
            .find(|task| task.project_id == project && task.scope == TaskScope::Studio);
        if let Some(task) = task {
            if !self.select_task(task.id, cx) {
                return;
            }
        } else {
            if self.dirty(cx) || self.saving {
                self.error = Some("Save the open file before changing Hub workspaces.".into());
                cx.notify();
                return;
            }
            self.snapshot_draft(cx);
            self.selection_revision = self.selection_revision.wrapping_add(1);
            self.device.retire();
            self.selected = None;
            self.loading_task = None;
            self.thread = None;
            self.refresh_prompt_history(cx);
            self.details = None;
            self.project = Some(project);
            self.reset_editor_tabs();
            self.document = None;
            self.files.clear();
            self.directory.clear();
            self.studio.reset();
            self.composer.update(cx, |entry, cx| entry.clear(cx));
        }
        self.hubs.selected = Some(project);
        self.navigation.studio = true;
        self.hubs.editing = false;
        self.set_panel(Panel::Hubs, cx);
        self.focus_composer = false;
    }
    pub(super) fn show_hubs(&mut self, cx: &mut Context<Self>) {
        if self.hub_navigation_blocked(cx) {
            return;
        }
        if !self.navigation.studio {
            self.navigation.last_synara = self.selected;
        }
        self.hubs.selected = self
            .task()
            .filter(|task| task.scope == TaskScope::Studio)
            .map(|task| task.project_id);
        self.navigation.studio = true;
        self.set_panel(Panel::Hubs, cx);
        self.focus_composer = false;
        if !self.hubs.loaded {
            self.load_hubs();
        }
    }
    pub(super) fn edit_hub(&mut self, create: bool, cx: &mut Context<Self>) {
        if self.hub_navigation_blocked(cx) {
            return;
        }
        let profile = if create {
            None
        } else {
            self.hub_profile().cloned()
        };
        if !create && profile.is_none() {
            return;
        }
        let values = profile
            .as_ref()
            .map(|p| [p.name.clone(), p.instructions.clone()])
            .unwrap_or_default();
        for (entry, text) in [&self.hubs.name, &self.hubs.instructions]
            .into_iter()
            .zip(values)
        {
            entry.update(cx, |entry, cx| entry.set_text(text, cx));
        }
        self.hubs.original = profile;
        self.hubs.folder = None;
        self.hubs.editing = true;
        self.hubs.error = None;
        self.hubs.focus_form = true;
        self.navigation.studio = true;
        self.set_panel(Panel::Hubs, cx);
        self.focus_composer = false;
    }
    fn save_hub_editor(&mut self, cx: &mut Context<Self>) {
        if self.hubs.saving || self.hubs.creating || self.hubs.picker || !self.hubs.editing {
            return;
        }
        let name = self.hubs.name.read(cx).text().trim().to_owned();
        if name.is_empty() || name.len() > 160 || name.chars().any(char::is_control) {
            self.hubs.error = Some("Enter a Hub name of at most 160 bytes.".into());
            cx.notify();
            return;
        }
        let workspace = self.controller.workspace.clone();
        if let Some(mut profile) = self.hubs.original.clone() {
            profile.name = name;
            profile.instructions = self.hubs.instructions.read(cx).text().to_owned();
            if let Err(error) = profile.validate() {
                self.hubs.error = Some(error.to_string());
                cx.notify();
                return;
            }
            self.hubs.saving = true;
            self.job(async move {
                Ok(Update::Hubs(Box::new(Reply::Saved(
                    workspace
                        .save_hub(profile.revision, profile)
                        .await
                        .map_err(|e| e.to_string()),
                ))))
            });
        } else {
            let Some(agent) = self
                .settings
                .value
                .general
                .default_provider
                .clone()
                .or_else(|| self.profiles.first().map(|p| p.id.clone()))
            else {
                return;
            };
            let managed = self.hubs.folder.is_none();
            let root = self.hubs.folder.clone().unwrap_or_else(|| {
                self.scratch_directory
                    .join("hubs")
                    .join(ProjectId::new().to_string())
            });
            let revision = self.selection_revision;
            self.hubs.creating = true;
            self.job(async move {
                let result = async {
                    if managed {
                        tokio::fs::create_dir_all(&root)
                            .await
                            .map_err(synara_runtime::RuntimeError::Io)?;
                    }
                    workspace.create_hub(root, name, agent).await
                }
                .await;
                Ok(Update::Hubs(Box::new(Reply::Created(
                    result.map_err(|e| e.to_string()),
                    revision,
                ))))
            });
        }
        cx.notify();
    }
    fn pick_hub_folder(&mut self, cx: &mut Context<Self>) {
        if self.hubs.picker || self.hubs.creating {
            return;
        }
        self.hubs.picker = true;
        let picker = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Choose Hub working folder".into()),
        });
        cx.spawn(async move |view, cx| {
            let result = picker.await;
            let _ = view.update(cx, |this, cx| {
                this.hubs.picker = false;
                match result {
                    Ok(Ok(Some(paths))) => this.hubs.folder = paths.into_iter().next(),
                    Ok(Ok(None)) => {}
                    _ => this.hubs.error = Some("The folder picker could not open.".into()),
                }
                cx.notify();
            });
        })
        .detach();
    }
    pub(super) fn new_hub_thread(&mut self, cx: &mut Context<Self>) {
        if self.hub_navigation_blocked(cx) || self.creating_task || self.loading_task.is_some() {
            return;
        }
        let Some(profile) = self.hub_profile().cloned() else {
            self.edit_hub(true, cx);
            return;
        };
        let Some(agent) = self
            .task()
            .filter(|t| t.project_id == profile.project)
            .map(|t| t.agent_id.clone())
            .or_else(|| self.settings.value.general.default_provider.clone())
            .or_else(|| self.profiles.first().map(|p| p.id.clone()))
        else {
            return;
        };
        let title = self.task_title.read(cx).text().trim().to_owned();
        let title = if title.is_empty() {
            "New Hub thread".into()
        } else {
            title
        };
        let revision = self.selection_revision;
        let workspace = self.controller.workspace.clone();
        self.creating_task = true;
        self.job(async move {
            Ok(Update::Hubs(Box::new(Reply::Thread(
                workspace
                    .create_hub_thread(profile.project, title, agent)
                    .await
                    .map_err(|e| e.to_string()),
                revision,
            ))))
        });
        cx.notify();
    }
    pub(super) fn hub_reply(&mut self, reply: Reply, cx: &mut Context<Self>) {
        match reply {
            Reply::Loaded(result) => {
                self.hubs.loading = false;
                match result {
                    Ok((rows, catalog)) => {
                        self.hubs.rows = rows;
                        self.hubs.loaded = true;
                        self.catalog = catalog;
                        self.refresh_project_ui(cx);
                        self.hubs.error = None;
                        if self.hubs.reload {
                            self.hubs.reload = false;
                            self.load_hubs();
                            cx.notify();
                            return;
                        }
                        if let Some((task, revision, home)) = self.hubs.pending_selection.take() {
                            let pending_close = self.open_threads.pending_close;
                            let close_after_selection = open_threads::pending_close_matches(
                                pending_close,
                                self.selected,
                                revision,
                            );
                            if revision == self.selection_revision && self.select_task(task, cx) {
                                self.hubs.selected = self.task().map(|t| t.project_id);
                                if home {
                                    self.set_panel(Panel::Hubs, cx);
                                    self.focus_composer = false;
                                } else {
                                    self.show_conversation(cx);
                                }
                                if close_after_selection {
                                    if let Some(pending) = pending_close {
                                        self.close_open_thread_tab_identity(pending.task, cx);
                                    }
                                    self.open_threads.pending_close = None;
                                }
                            } else {
                                self.open_threads.pending_close = None;
                                self.notice = Some(
                                    "Hub work was saved. Open it from Hubs when ready.".into(),
                                );
                            }
                        }
                    }
                    Err(error) => {
                        self.open_threads.pending_close = None;
                        self.hubs.error = Some(error);
                    }
                }
            }
            Reply::Created(result, revision) => {
                self.hubs.creating = false;
                match result {
                    Ok((profile, task)) => {
                        if self.hubs.name.read(cx).text().trim() == profile.name {
                            self.hubs
                                .name
                                .update(cx, |entry, cx| entry.set_text(profile.name.clone(), cx));
                        }
                        self.hubs.selected = Some(profile.project);
                        self.hubs.original = Some(profile.clone());
                        self.hubs.rows.push(HubSummary {
                            profile,
                            threads: 1,
                            imported: false,
                        });
                        if !self.hubs.dirty(cx) {
                            self.hubs.editing = false;
                            self.hubs.pending_selection = Some((task.id, revision, true));
                        } else {
                            self.notice = Some(
                                "Hub created. Your newer edits remain in the context editor."
                                    .into(),
                            );
                        }
                        self.load_hubs();
                    }
                    Err(error) => {
                        self.open_threads.pending_close = None;
                        self.hubs.error = Some(error);
                    }
                }
            }
            Reply::Saved(result) => {
                self.hubs.saving = false;
                match result {
                    Ok(profile) => {
                        if self.hubs.editing {
                            if self.hubs.name.read(cx).text().trim() == profile.name {
                                self.hubs.name.update(cx, |entry, cx| {
                                    entry.set_text(profile.name.clone(), cx)
                                });
                            }
                            self.hubs.original = Some(profile.clone());
                        }
                        if let Some(row) = self
                            .hubs
                            .rows
                            .iter_mut()
                            .find(|row| row.profile.project == profile.project)
                        {
                            row.profile = profile;
                            row.imported = false;
                        }
                        self.hubs.error = None;
                        self.notice = Some(
                            "Hub settings saved. New threads inherit the instructions in their notes."
                                .into(),
                        );
                    }
                    Err(error) => self.hubs.error = Some(error),
                }
            }
            Reply::Thread(result, revision) => {
                self.creating_task = false;
                match result {
                    Ok(task) => {
                        if self.task_title.read(cx).text().trim() == task.title {
                            self.task_title.update(cx, |e, cx| e.clear(cx));
                        }
                        self.hubs.pending_selection = Some((task.id, revision, false));
                        self.load_hubs();
                    }
                    Err(error) => {
                        self.open_threads.pending_close = None;
                        self.hubs.error = Some(error);
                    }
                }
            }
        }
        cx.notify();
    }
    pub(super) fn restore_hub_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.hubs.focus_form && self.panel == Panel::Hubs {
            self.hubs.focus_form = false;
            window.focus(&self.hubs.name.read(cx).focus_handle(cx), cx);
        }
    }
}
