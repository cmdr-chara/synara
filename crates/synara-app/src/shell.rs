use gpui::Focusable;
mod activity;
mod hubs;
mod chat_tools;
mod chrome;
mod composer;
mod controls;
mod conversation;
mod dock;
mod drafts;
mod environment;
mod explorer;
mod editors;
mod command_palette;
mod kanban;
mod messages;
mod navigation;
mod organization;
mod overview;
mod panels;
mod registry;
mod review;
mod saved_context;
mod settings;
mod studio;
mod terminal;
mod terminals;
mod transcript;
mod zen;
use crate::close::CloseState;
use crate::input::{EntryEvent, EntryMode, TextEntry};
use gpui::{App, Context, Entity, SharedString, Subscription, Window, div, prelude::*, px, rgb};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
};
use synara_agent::{InteractionScope, TraceEntry, UiInteraction};
use synara_core::*;
use synara_runtime::{
    FileEntry, NativeTerminal, TerminalKey, TerminalModifiers, TerminalRenderSnapshot,
};
use synara_workspace::*;
use terminal::{TerminalSession, TerminalView};
use tokio::{runtime::Handle, sync::mpsc};

pub struct Bootstrap {
    pub environment: LoadedEnvironmentLayout,
    pub settings: AppSettings,
    pub scratch_directory: PathBuf,
    pub agent_directory: PathBuf,
    pub catalog: Catalog,
    pub profiles: Vec<AgentProfile>,
    pub selection: Selection,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Panel {
    Conversation,
    Dock,
    Kanban,
    Hubs,
    Help,
    Files,
    Changes,
    Terminal,
    Inspector,
    Settings,
    Registry,
    Remote,
}
#[derive(Clone)]
enum WorkspaceTarget {
    Local { root: PathBuf },
    Ssh { workspace: Workspace, root: PathBuf },
}
impl WorkspaceTarget {
    fn root(&self) -> &PathBuf {
        match self {
            Self::Local { root } | Self::Ssh { root, .. } => root,
        }
    }
}
type InteractionKey = (ThreadId, String);
struct FormState {
    request: UserInputRequest,
    inputs: BTreeMap<String, Entity<TextEntry>>,
    values: BTreeMap<String, InputValue>,
    error: Option<String>,
}
enum Update {
    Hubs(Box<hubs::Reply>),
    Terminals(Box<terminals::Reply>),
    Review(Box<review::Reply>),
    Explorer(Box<explorer::ExplorerReply>),
    Studio(Box<studio::StudioReply>),
    SavedContext(Box<saved_context::ContextReply>),
    Organization(Box<organization::OrganizationReply>),
    ChatTools(Box<chat_tools::Reply>),
    EnvironmentSaved(Option<String>),
    Kanban(Box<kanban::KanbanReply>),
    DraftLoaded(TaskId, Result<String, String>),
    DraftSaved(TaskId, Option<String>),
    Registry(Box<registry::RegistryReply>),
    Catalog(Catalog),
    WorkspaceAdded(Project, Catalog),
    TaskCreated(Task, Catalog, u64),
    TaskCreationFailed(String),
    ThreadLoaded(Task, Box<Thread>),
    ThreadLoadFailed(TaskId, String),
    Event(EventEnvelope),
    Hydrate,
    Interaction(UiInteraction),
    Connected {
        task: TaskId,
        details: Option<SessionDetails>,
        error: Option<String>,
    },
    PromptDone {
        task: TaskId,
        details: Option<SessionDetails>,
        error: Option<String>,
    },
    Details {
        task: TaskId,
        details: Option<SessionDetails>,
        trace: Vec<TraceEntry>,
    },
    SettingsSaved(Box<AppSettings>, Option<String>),
    ProfileActivity(settings::ProfileActivity),
    ControlFinished {
        task: TaskId,
        result: WorkspaceResult<Option<Task>>,
        details: Option<SessionDetails>,
    },
    Files {
        root: PathBuf,
        directory: PathBuf,
        entries: Vec<FileEntry>,
    },
    Document {
        root: PathBuf,
        generation: u64,
        document: Document,
    },
    DocumentFailed { generation: u64, error: String },
    SaveFailed(String),
    Saved {
        root: PathBuf,
        path: PathBuf,
        text: String,
        version: synara_runtime::FileVersion,
    },
    Tick,
    Done(String),
    Error(String),
}
pub struct Shell {
    hubs: hubs::HubState,
    terminals: terminals::TerminalWorkspace,
    editors: editors::EditorState,
    command_palette: command_palette::PaletteState,
    review: review::ReviewState,
    explorer: explorer::ExplorerState,
    studio: studio::StudioState,
    saved_context: saved_context::SavedContextState,
    organization: organization::OrganizationState,
    chat_tools: chat_tools::ChatTools,
    environment: environment::EnvironmentState,
    kanban: kanban::KanbanState,
    controls: controls::ControlState,
    navigation: navigation::NavigationState,
    settings: settings::SettingsState,
    close: CloseState,
    close_focus: gpui::FocusHandle,
    registry: registry::RegistryState,
    controller: Arc<Controller>,
    runtime: Handle,
    sender: async_channel::Sender<Update>,
    catalog: Catalog,
    profiles: Vec<AgentProfile>,
    scratch_directory: PathBuf,
    creating_task: bool,
    loading_task: Option<TaskId>,
    selection_revision: u64,
    project: Option<ProjectId>,
    selected: Option<TaskId>,
    thread: Option<Thread>,
    details: Option<SessionDetails>,
    trace: Vec<TraceEntry>,
    composer: Entity<TextEntry>,
    workspace_path: Entity<TextEntry>,
    remote_host: Entity<TextEntry>,
    remote_port: Entity<TextEntry>,
    remote_user: Entity<TextEntry>,
    remote_root: Entity<TextEntry>,
    remote_known_hosts: Entity<TextEntry>,
    remote_identity: Entity<TextEntry>,
    remote_helper: Entity<TextEntry>,
    task_title: Entity<TextEntry>,
    editor: Entity<TextEntry>,
    file_search: Entity<TextEntry>,
    // Focus-only alias for existing shell shortcut guards. Ownership is in terminals.
    terminal_view: Entity<TerminalView>,
    drafts: HashMap<TaskId, String>,
    draft_state: drafts::DraftState,
    busy: HashSet<TaskId>,
    connecting: HashSet<TaskId>,
    panel: Panel,
    dock_panel: Panel,
    dock_motion: crate::ui::motion::Drawer,
    error: Option<String>,
    notice: Option<String>,
    focus_composer: bool,
    transcript: transcript::TranscriptState,
    expanded_activity: HashSet<(ThreadId, String)>,
    pending: HashMap<InteractionKey, UiInteraction>,
    forms: HashMap<InteractionKey, FormState>,
    files: Vec<FileEntry>,
    directory: PathBuf,
    file_page: usize,
    document: Option<Document>,
    saving: bool,
    terminal_closing: bool,
    polling: bool,
    _updates: gpui::Task<()>,
    _subscriptions: Vec<Subscription>,
}
impl Shell {
    pub fn new(
        controller: Arc<Controller>,
        runtime: Handle,
        bootstrap: Bootstrap,
        mut interactions: mpsc::Receiver<UiInteraction>,
        cx: &mut Context<Self>,
    ) -> Self {
        let (sender, receiver) = async_channel::bounded(128);
        let mut events = controller.workspace.subscribe();
        let forward = sender.clone();
        runtime.spawn(async move {
            loop {
                let update = match events.recv().await {
                    Ok(event) => Update::Event(event),
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => Update::Hydrate,
                    Err(_) => break,
                };
                if forward.send(update).await.is_err() {
                    break;
                }
            }
        });
        let forward = sender.clone();
        runtime.spawn(async move {
            while let Some(interaction) = interactions.recv().await {
                if forward
                    .send(Update::Interaction(interaction))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        });
        let forward = sender.clone();
        runtime.spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                if forward.send(Update::Tick).await.is_err() {
                    break;
                }
            }
        });
        let updates = cx.spawn(async move |view, cx| {
            while let Ok(update) = receiver.recv().await {
                if view
                    .update(cx, |this, cx| this.receive(update, cx))
                    .is_err()
                {
                    break;
                }
            }
        });
        let composer = cx.new(|cx| {
            TextEntry::new(
                "Ask for follow-up changes",
                EntryMode::Composer,
                crate::ui::COMPOSER_INPUT_HEIGHT,
                cx,
            )
        });
        let workspace_path =
            cx.new(|cx| TextEntry::new("Workspace directory", EntryMode::SingleLine, 38., cx));
        let remote_host = cx.new(|cx| TextEntry::new("SSH host", EntryMode::SingleLine, 36., cx));
        let remote_port = cx.new(|cx| TextEntry::new("SSH port", EntryMode::SingleLine, 36., cx));
        remote_port.update(cx, |entry, cx| entry.set_text("22".into(), cx));
        let remote_user =
            cx.new(|cx| TextEntry::new("SSH user (optional)", EntryMode::SingleLine, 36., cx));
        let remote_root = cx.new(|cx| {
            TextEntry::new(
                "Remote absolute workspace root",
                EntryMode::SingleLine,
                36.,
                cx,
            )
        });
        let remote_known_hosts = cx.new(|cx| {
            TextEntry::new(
                "Local pinned known_hosts path",
                EntryMode::SingleLine,
                36.,
                cx,
            )
        });
        let remote_identity = cx.new(|cx| {
            TextEntry::new(
                "Local private identity path",
                EntryMode::SingleLine,
                36.,
                cx,
            )
        });
        let remote_helper = cx.new(|cx| {
            TextEntry::new(
                "Remote synara-remote-fs path",
                EntryMode::SingleLine,
                36.,
                cx,
            )
        });
        remote_helper.update(cx, |entry, cx| {
            entry.set_text("synara-remote-fs".into(), cx)
        });
        let task_title =
            cx.new(|cx| TextEntry::new("New task title", EntryMode::SingleLine, 36., cx));
        let editor =
            cx.new(|cx| TextEntry::new("Select a UTF-8 text file", EntryMode::Editor, 480., cx));
        let file_search = cx.new(|cx| {
            TextEntry::new("Search files...", EntryMode::SingleLine, 30., cx)
                .with_leading_icon(crate::ui::Glyph::Search)
        });
        let terminal_view = cx.new(TerminalView::new);
        let registry = registry::RegistryState::new(bootstrap.agent_directory, cx);
        let subscriptions = vec![
            cx.subscribe(&file_search, |this, _, _, cx| {
                this.file_page = 0;
                cx.notify();
            }),
            cx.subscribe(&registry.query, |_, _, _, cx| cx.notify()),
            cx.subscribe(&composer, |this, _, event, cx| match event {
                EntryEvent::Submit => this.send_prompt(cx),
                EntryEvent::Changed => this.remember_draft(cx),
                _ => cx.notify(),
            }),
            cx.subscribe(&workspace_path, |this, _, event, cx| {
                if matches!(event, EntryEvent::Submit) {
                    this.open_workspace(cx)
                }
            }),
            cx.subscribe(&task_title, |this, _, event, cx| {
                if matches!(event, EntryEvent::Submit) {
                    this.create_chat(
                        if this.navigation.studio {
                            TaskScope::Studio
                        } else {
                            TaskScope::Chat
                        },
                        cx,
                    )
                }
            }),
            cx.subscribe(&editor, |this, _, event, cx| match event {
                EntryEvent::Save => this.save_file(cx),
                _ => cx.notify(),
            }),
        ];
        let project = bootstrap
            .selection
            .project
            .filter(|id| bootstrap.catalog.projects.iter().any(|p| p.id == *id))
            .or_else(|| bootstrap.catalog.projects.first().map(|p| p.id));
        let selected = bootstrap
            .selection
            .task
            .filter(|id| bootstrap.catalog.tasks.iter().any(|t| t.id == *id))
            .or_else(|| {
                bootstrap
                    .catalog
                    .tasks
                    .iter()
                    .find(|t| Some(t.project_id) == project)
                    .map(|t| t.id)
            });
        let mut this = Self {
            hubs: hubs::HubState::new(cx),
            terminals: terminals::TerminalWorkspace::default(),
            editors: editors::EditorState::new(cx),
            command_palette: command_palette::PaletteState::new(cx),
            review: review::ReviewState::default(),
            explorer: explorer::ExplorerState::new(cx),
            studio: studio::StudioState::new(cx),
            saved_context: saved_context::SavedContextState::default(),
            organization: organization::OrganizationState::new(cx),
            chat_tools: chat_tools::ChatTools::new(cx),
            environment: environment::EnvironmentState::new(bootstrap.environment, cx),
            kanban: kanban::KanbanState::default(),
            controls: controls::ControlState::new(cx),
            navigation: navigation::NavigationState::new(cx),
            settings: settings::SettingsState::new(bootstrap.settings, cx),
            close: CloseState::Open,
            close_focus: cx.focus_handle(),
            registry,
            controller,
            runtime,
            sender,
            catalog: bootstrap.catalog,
            profiles: bootstrap.profiles,
            scratch_directory: bootstrap.scratch_directory,
            creating_task: false,
            loading_task: None,
            selection_revision: 0,
            project,
            selected: None,
            thread: None,
            details: None,
            trace: vec![],
            composer,
            workspace_path,
            remote_host,
            remote_port,
            remote_user,
            remote_root,
            remote_known_hosts,
            remote_identity,
            remote_helper,
            task_title,
            editor,
            file_search,
            terminal_view,
            drafts: HashMap::new(),
            draft_state: drafts::DraftState::default(),
            busy: HashSet::new(),
            connecting: HashSet::new(),
            panel: Panel::Conversation,
            dock_panel: Panel::Dock,
            dock_motion: crate::ui::motion::Drawer::new(false),
            error: None,
            notice: None,
            focus_composer: false,
            transcript: transcript::TranscriptState::new(),
            expanded_activity: HashSet::new(),
            pending: HashMap::new(),
            forms: HashMap::new(),
            files: vec![],
            directory: PathBuf::new(),
            file_page: 0,
            document: None,
            saving: false,
            terminal_closing: false,
            polling: false,
            _updates: updates,
            _subscriptions: subscriptions,
        };
        this.load_organization();
        this.load_hubs();
        this.composer.update(cx, |entry, _| {
            entry.set_send_on_enter(this.settings.value.chat.send_on_enter)
        });
        if let Some(selected) = selected {
            this.select_task(selected, cx);
            this.show_conversation(cx);
        }
        if this.settings.value.appearance.personalization.zen_mode {
            this.settings.personalization.tools_shown = false;
        }
        if let Some(error) = &this.environment.recovery {
            this.notice = Some(error.clone());
        }
        this
    }
    pub fn request_close(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.hubs.pending(cx) {
            self.notice = Some("Finish Hub creation or save/discard the Hub context editor before closing.".into());
            cx.notify();
            return false;
        }
        if self.appearance_pending(cx) {
            self.notice = Some("Finish the appearance save or image picker, and clear any pasted profile before closing.".into());
            cx.notify();
            return false;
        }
        if self.editor.read(cx).is_composing() {
            self.notice = Some("Finish composing text in the editor before closing.".into());
            cx.notify();
            return false;
        }
        if self.organization.saving
            || self.organization.dialog.is_some()
            || self.saved_context.dialog.is_some()
            || self.explorer.modal_open()
        {
            self.notice = Some(
                "Finish pending saves and close the open editor dialog before closing Synara."
                    .into(),
            );
            cx.notify();
            return false;
        }
        if self.chat_tools.pending_write() {
            self.notice = Some(
                "Finish or cancel the conversation export and pending pin saves before closing."
                    .into(),
            );
            cx.notify();
            return false;
        }
        if self.kanban.creating {
            self.notice =
                Some("Finishing task creation before closing. Your prompt is being saved.".into());
            cx.notify();
            return false;
        }
        if let Some(dialog) = &self.kanban.dialog {
            if dialog.read(cx).has_text(cx) {
                dialog.update(cx, |dialog, cx| dialog.failed("Create the task or explicitly discard this unfinished prompt before closing.".into(), cx));
                window.focus(&dialog.read(cx).focus_handle(cx), cx);
                return false;
            }
            self.kanban.dialog = None;
        }
        if self.terminal_closing || self.draft_state.quitting || self.environment.quitting {
            return false;
        }
        self.reveal_dirty_editor(cx);
        let dirty = self.dirty(cx);
        tracing::debug!(target: "synara_ui_layout", dirty, saving = self.saving, document = self.document.is_some(), "close-request");
        if self.close.request(dirty, self.saving) {
            self.begin_quit(cx);
            false
        } else {
            window.focus(&self.close_focus, cx);
            cx.notify();
            false
        }
    }

    fn begin_quit(&mut self, cx: &mut Context<Self>) {
        if self.dirty(cx) {
            self.reveal_dirty_editor(cx);
            self.close = CloseState::Review;
            cx.notify();
            return;
        }
        if self.terminal_layout_before_quit(cx) {
            return;
        }
        if self.review_before_quit(cx) {
            return;
        }
        self.kanban.cancel_pending_launches();
        if self.terminal_closing {
            return;
        }
        if self.save_environment_before_quit() {
            cx.notify();
            return;
        }
        if self.save_drafts_before_quit(cx) {
            return;
        }
        self.begin_terminal_shutdown(cx);
    }

    fn retire_terminal(&self, terminal: TerminalSession) {
        self.runtime.spawn(async move {
            let _ = terminal.kill();
            let _ = tokio::time::timeout(std::time::Duration::from_secs(5), terminal.wait()).await;
        });
    }
    fn close_panel(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let waiting = self.close == CloseState::WaitingForSave;
        let terminal_closing = self.terminal_closing;
        div().size_full().flex().flex_col().items_center().justify_center()
            .bg(rgb(0x10151d)).text_color(rgb(0xe3e8f0)).font_family("DejaVu Sans")
            .child(div().w(px(620.)).p_6().rounded_lg().border_1().border_color(rgb(0x35465b))
                .flex().flex_col().gap_4().bg(rgb(0x1b2532))
                .id("close-review").track_focus(&self.close_focus)
                .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                    if event.keystroke.key == "escape" {
                        this.close.cancel();
                        window.focus(&this.editor.read(cx).focus_handle(cx), cx);
                        cx.notify();
                    }
                }))
                .child(div().text_xl().child(if terminal_closing && self.terminals.starting() {
                    "Waiting for terminal startup before closing Synara"
                } else if terminal_closing {
                    "Stopping terminal before closing Synara"
                } else if waiting {
                    "Waiting for the file to finish saving"
                } else {
                    "Save changes before closing Synara?"
                }))
                .child(self.document.as_ref().map_or_else(String::new, |d| d.path.display().to_string()))
                .child("The file remains open if saving fails or the on-disk version has changed. Closing stops active agent and terminal processes.")
                .children(self.error.as_ref().map(|e| div().text_color(rgb(0xffb1b5)).child(e.clone())))
                .child(div().flex().gap_3()
                    .children((!terminal_closing).then(|| button("cancel-close", "Keep working", false).on_click(cx.listener(|this, _, window, cx| {
                        this.close.cancel();
                        window.focus(&this.editor.read(cx).focus_handle(cx), cx);
                        cx.notify();
                    }))))
                    .children((!waiting && !terminal_closing).then(|| button("discard-and-close", "Discard and close", false)
                        .relative().child(crate::ui::layout_probe("discard-and-close"))
                        .on_click(cx.listener(|this, _, _, cx| { if !this.saving { this.discard_active_document(cx); this.begin_quit(cx); } }))))
                    .children((!waiting && !terminal_closing).then(|| button("save-and-close", "Save and close", true)
                        .relative().child(crate::ui::layout_probe("save-and-close"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.close = CloseState::WaitingForSave;
                            this.save_file(cx);
                        }))))))
            .into_any_element()
    }
    fn job(
        &self,
        task: impl std::future::Future<Output = WorkspaceResult<Update>> + Send + 'static,
    ) {
        let sender = self.sender.clone();
        self.runtime.spawn(async move {
            let update = task
                .await
                .unwrap_or_else(|error| Update::Error(error.to_string()));
            let _ = sender.send(update).await;
        });
    }
    fn task(&self) -> Option<&Task> {
        let id = self.selected?;
        self.catalog.tasks.iter().find(|t| t.id == id)
    }
    fn agent_glyph(&self, agent_id: &str) -> crate::ui::Glyph {
        let executable = self
            .profiles
            .iter()
            .find(|profile| profile.id == agent_id)
            .and_then(|profile| profile.command.file_stem())
            .and_then(|name| name.to_str());
        crate::ui::provider_glyph(agent_id, executable)
    }

    fn selected_agent_glyph(&self) -> crate::ui::Glyph {
        self.task().map_or(crate::ui::Glyph::Agent, |task| {
            self.agent_glyph(&task.agent_id)
        })
    }
    fn workspace_target(&self) -> Option<WorkspaceTarget> {
        let project = self
            .catalog
            .projects
            .iter()
            .find(|project| Some(project.id) == self.project)?;
        let workspace = self
            .catalog
            .workspaces
            .iter()
            .find(|workspace| workspace.id == project.workspace_id)?;
        match &workspace.location {
            WorkspaceLocation::Local { root } => Some(WorkspaceTarget::Local {
                root: root.join(&project.relative_directory),
            }),
            WorkspaceLocation::Ssh { root, .. } => Some(WorkspaceTarget::Ssh {
                workspace: workspace.clone(),
                root: PathBuf::from(root).join(&project.relative_directory),
            }),
        }
    }
    fn root(&self) -> Option<PathBuf> {
        self.workspace_target().map(|target| target.root().clone())
    }
    fn dirty(&self, cx: &App) -> bool {
        self.active_document_dirty(cx) || self.editors.dirty(cx)
    }
    fn replace_task(&mut self, task: Task) {
        if let Some(existing) = self.catalog.tasks.iter_mut().find(|t| t.id == task.id) {
            *existing = task;
        } else {
            self.catalog.tasks.insert(0, task);
        }
    }
    fn select_task(&mut self, id: TaskId, cx: &mut Context<Self>) -> bool {
        if self.hub_navigation_blocked(cx) { return false; }
        if self.explorer.modal_open() {
            return false;
        }
        let Some(task) = self
            .catalog
            .tasks
            .iter()
            .find(|task| task.id == id)
            .cloned()
        else {
            return false;
        };
        if Some(task.project_id) != self.project && (self.dirty(cx) || self.saving) {
            self.error =
                Some("Save or discard the open document before switching projects.".into());
            cx.notify();
            return false;
        }
        self.snapshot_draft(cx);
        self.selection_revision = self.selection_revision.wrapping_add(1);
        self.navigation.studio = task.scope == TaskScope::Studio;
        if self.navigation.studio {
            self.hubs.selected = Some(task.project_id);
            self.navigation.last_studio = Some(id);
        } else {
            self.navigation.last_synara = Some(id);
        }
        self.navigation.collapsed_projects.remove(&task.project_id);
        if Some(task.project_id) != self.project {
            self.reset_editor_tabs();
            self.document = None;
            self.files.clear();
            self.directory.clear();
        }
        self.controls.retire();
        self.navigation.record_task(id);
        self.selected = Some(id);
        self.loading_task = Some(id);
        self.chat_tools.reset_selection();
        self.studio.reset();
        self.explorer.reset_search();
        self.load_message_pins(id);
        self.project = Some(task.project_id);
        self.details = None;
        self.trace.clear();
        self.thread = Some(Thread::new(task.thread_id));
        self.error = None;
        self.composer.update(cx, |entry, cx| {
            entry.set_text(self.drafts.get(&id).cloned().unwrap_or_default(), cx)
        });
        self.load_draft(id);
        self.transcript = transcript::TranscriptState::new();
        self.focus_composer = true;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            let result = async {
                workspace
                    .save_selection(Selection {
                        project: Some(task.project_id),
                        task: Some(id),
                    })
                    .await?;
                workspace.thread(task.thread_id).await
            }
            .await;
            Ok(match result {
                Ok(thread) => Update::ThreadLoaded(task, Box::new(thread)),
                Err(error) => Update::ThreadLoadFailed(id, error.to_string()),
            })
        });
        if self.panel == Panel::Files {
            self.refresh_files();
        }
        if self.panel == Panel::Changes {
            self.refresh_git(cx);
        }
        cx.notify();
        true
    }
    fn hydrate(&self) {
        if let Some(task) = self.task().cloned() {
            let workspace = self.controller.workspace.clone();
            self.job(async move {
                let thread = workspace.thread(task.thread_id).await?;
                Ok(Update::ThreadLoaded(task, Box::new(thread)))
            });
        }
    }
    fn open_workspace(&mut self, cx: &mut Context<Self>) {
        if self.hub_navigation_blocked(cx) { return; }
        let text = self.workspace_path.read(cx).text().trim().to_owned();
        if text.is_empty() {
            self.error = Some("Enter an existing absolute directory or use Browse.".into());
            cx.notify();
            return;
        }
        let path = PathBuf::from(text);
        if !path.is_absolute() {
            self.error = Some("The workspace directory must be an absolute path.".into());
            cx.notify();
            return;
        }
        if self.dirty(cx) || self.saving {
            self.error =
                Some("Save or discard the open document before switching workspaces.".into());
            cx.notify();
            return;
        }
        let workspace = self.controller.workspace.clone();
        self.error = None;
        self.job(async move {
            let project = workspace.add_local_workspace(path).await?;
            Ok(Update::WorkspaceAdded(project, workspace.catalog().await?))
        });
    }
    fn browse_workspace(&mut self, cx: &mut Context<Self>) {
        let picker = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Open workspace".into()),
        });
        cx.spawn(async move |view, cx| {
            let result = picker.await;
            let _ = view.update(cx, |this, cx| match result {
                Ok(Ok(Some(paths))) => {
                    if let Some(path) = paths.into_iter().next() {
                        this.workspace_path.update(cx, |entry, cx| {
                            entry.set_text(path.to_string_lossy().into_owned(), cx)
                        });
                        this.open_workspace(cx);
                    }
                }
                Ok(Ok(None)) => {}
                _ => {
                    this.error = Some(
                        "The system file picker could not open. Enter the workspace path instead."
                            .into(),
                    );
                    cx.notify();
                }
            });
        })
        .detach();
    }
    fn open_remote_workspace(&mut self, cx: &mut Context<Self>) {
        if self.hub_navigation_blocked(cx) { return; }
        if self.dirty(cx) || self.saving {
            self.error =
                Some("Save or discard the open document before switching workspaces.".into());
            cx.notify();
            return;
        }
        let host = self.remote_host.read(cx).text().trim().to_owned();
        let root = self.remote_root.read(cx).text().trim().to_owned();
        let known_hosts = PathBuf::from(self.remote_known_hosts.read(cx).text().trim());
        let identity_file = PathBuf::from(self.remote_identity.read(cx).text().trim());
        let helper = PathBuf::from(self.remote_helper.read(cx).text().trim());
        let user = self.remote_user.read(cx).text().trim().to_owned();
        let port = match self.remote_port.read(cx).text().trim().parse::<u16>() {
            Ok(port) if port != 0 => port,
            _ => {
                self.error = Some("SSH port must be an integer from 1 through 65535.".into());
                cx.notify();
                return;
            }
        };
        if host.is_empty()
            || root.is_empty()
            || known_hosts.as_os_str().is_empty()
            || identity_file.as_os_str().is_empty()
            || helper.as_os_str().is_empty()
        {
            self.error = Some(
                "Host, remote root, pinned known_hosts, identity and remote helper are required."
                    .into(),
            );
            cx.notify();
            return;
        }
        let request = NewSshWorkspace {
            name: host.clone(),
            target: synara_runtime::SshTarget {
                host,
                port,
                user: (!user.is_empty()).then_some(user),
            },
            root,
            known_hosts,
            identity_file,
            helper,
        };
        let workspace = self.controller.workspace.clone();
        self.error = None;
        self.notice = Some("Verifying pinned SSH trust and remote workspace root...".into());
        self.job(async move {
            let project = workspace.add_ssh_workspace(request).await?;
            Ok(Update::WorkspaceAdded(project, workspace.catalog().await?))
        });
        cx.notify();
    }

    fn create_task(&mut self, cx: &mut Context<Self>) {
        self.create_chat(TaskScope::Project, cx);
    }

    fn create_chat(&mut self, scope: TaskScope, cx: &mut Context<Self>) {
        if self.hub_navigation_blocked(cx) { return; }
        if scope == TaskScope::Studio { self.new_hub_thread(cx); return; }
        if self.creating_task
            || self.loading_task.is_some()
            || self
                .selected
                .is_some_and(|id| self.draft_state.loading.contains(&id))
        {
            return;
        }
        if self.task().is_some_and(|task| {
            task.scope == scope
                && task.state == TaskState::Ready
                && self.drafts.contains_key(&task.id)
        }) && self
            .thread
            .as_ref()
            .is_some_and(|thread| thread.turns.is_empty() && thread.messages.is_empty())
            && self.composer.read(cx).text().is_empty()
            && self.task_title.read(cx).text().trim().is_empty()
        {
            self.set_panel(Panel::Conversation, cx);
            self.focus_composer = true;
            cx.notify();
            return;
        }
        if scope != TaskScope::Project && (self.dirty(cx) || self.saving) {
            self.error =
                Some("Save or discard the open document before starting a standalone chat.".into());
            cx.notify();
            return;
        }
        let Some(agent) = self
            .task()
            .map(|task| task.agent_id.clone())
            .or_else(|| self.settings.value.general.default_provider.clone())
            .or_else(|| self.profiles.first().map(|p| p.id.clone()))
        else {
            return;
        };
        let project = (scope == TaskScope::Project)
            .then_some(self.project)
            .flatten();
        if scope == TaskScope::Project && project.is_none() {
            self.browse_workspace(cx);
            return;
        }
        let title = self.task_title.read(cx).text().trim().to_owned();
        let title = if title.is_empty() {
            if scope == TaskScope::Studio {
                "New studio chat".into()
            } else {
                "New thread".into()
            }
        } else {
            title
        };
        let workspace = self.controller.workspace.clone();
        let scratch = self.scratch_directory.join(ThreadId::new().to_string());
        let revision = self.selection_revision;
        self.creating_task = true;
        self.job(async move {
            let result = async {
                let project = if let Some(project) = project {
                    project
                } else {
                    std::fs::create_dir_all(&scratch).map_err(synara_runtime::RuntimeError::Io)?;
                    workspace.add_local_workspace(scratch).await?.id
                };
                let task = workspace
                    .create_scoped_task(project, title, agent, scope)
                    .await?;
                Ok::<_, WorkspaceError>((task, workspace.catalog().await?))
            }
            .await;
            Ok(match result {
                Ok((task, catalog)) => Update::TaskCreated(task, catalog, revision),
                Err(error) => Update::TaskCreationFailed(error.to_string()),
            })
        });
        cx.notify();
    }
    fn connect(&mut self, operation: &str, cx: &mut Context<Self>) {
        let Some(id) = self.selected else { return };
        if self.connecting.contains(&id) || self.controls.is_pending(id) {
            return;
        }
        self.connecting.insert(id);
        self.error = None;
        let controller = self.controller.clone();
        let operation = operation.to_owned();
        self.job(async move {
            let result = match operation.as_str() {
                "restart" => controller.restart(id).await,
                "fresh" => controller.fresh_session(id).await,
                _ => controller.connect(id).await,
            };
            let details = controller.details(id).await.ok().flatten();
            Ok(Update::Connected {
                task: id,
                details,
                error: result.err().map(|e| e.to_string()),
            })
        });
        cx.notify();
    }
    fn authenticate(&mut self, method: String, cx: &mut Context<Self>) {
        let Some(id) = self.selected else { return };
        if self.connecting.contains(&id) || self.controls.is_pending(id) {
            return;
        }
        self.connecting.insert(id);
        let controller = self.controller.clone();
        self.error = None;
        self.job(async move {
            let result = controller.authenticate(id, method).await;
            let details = controller.details(id).await.ok().flatten();
            Ok(Update::Connected {
                task: id,
                details,
                error: result.err().map(|e| e.to_string()),
            })
        });
        cx.notify();
    }
    fn send_prompt(&mut self, cx: &mut Context<Self>) {
        if self.hub_navigation_blocked(cx) { return; }
        let Some(id) = self.selected else {
            self.error = Some("Create or select a task first.".into());
            cx.notify();
            return;
        };
        if self.busy.contains(&id) || self.connecting.contains(&id) || self.controls.is_pending(id)
        {
            return;
        }
        let text = self.composer.read(cx).text().to_owned();
        if text.trim().is_empty() {
            return;
        }
        self.snapshot_draft(cx);
        self.draft_state.submitted(id, text.clone());
        self.busy.insert(id);
        self.error = None;
        self.notice = None;
        self.transcript.follow();
        let controller = self.controller.clone();
        let untitled = self.task().is_some_and(|task| {
            matches!(
                task.title.as_str(),
                "New task" | "New thread" | "New studio chat" | "New Hub thread"
            )
        });
        let hub_thread = self.task().is_some_and(|task| task.scope == TaskScope::Studio);
        self.job(async move {
            let result = async {
                if untitled {
                    let title_text = if hub_thread { text.rsplit_once("\nTask:\n").map_or(text.as_str(),|(_,prompt)|prompt) } else { text.as_str() };
                    let title: String = title_text
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ")
                        .chars()
                        .take(64)
                        .collect();
                    controller.workspace.rename_task(id, title).await?;
                }
                controller.submit(id, text).await
            }
            .await;
            let details = controller.details(id).await.ok().flatten();
            Ok(Update::PromptDone {
                task: id,
                details,
                error: result.err().map(|e| e.to_string()),
            })
        });
        cx.notify();
    }
    fn cancel(&mut self, cx: &mut Context<Self>) {
        if let Some(id) = self.selected {
            let controller = self.controller.clone();
            self.job(async move {
                controller.cancel(id).await?;
                Ok(Update::Done("Cancellation requested".into()))
            });
        }
        cx.notify();
    }
    fn refresh_files(&self) {
        let Some(target) = self.workspace_target() else {
            return;
        };
        let root = target.root().clone();
        let directory = self.directory.clone();
        let workspace_service = self.controller.workspace.clone();
        self.job(async move {
            let entries = match target {
                WorkspaceTarget::Local { root } => list_files(root, directory.clone()).await?,
                WorkspaceTarget::Ssh { workspace, root } => {
                    let filesystem = remote_filesystem(workspace_service, workspace, root).await?;
                    list_remote_files(filesystem, directory.clone()).await?
                }
            };
            Ok(Update::Files {
                root,
                directory,
                entries,
            })
        });
    }

    fn open_file(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if self.explorer.modal_open() || self.saving || self.close != CloseState::Open {
            return;
        }
        if let Some(index) = self.editors.index(&path) {
            self.activate_editor(index, cx);
            return;
        }
        if self.editors.tabs.len() >= editors::MAX_TABS {
            self.error = Some("Close an editor tab before opening another file (24 tabs maximum).".into());
            cx.notify();
            return;
        }
        let Some(target) = self.workspace_target() else { return };
        let root = target.root().clone();
        let generation = self.editors.request_open(path.clone());
        let workspace_service = self.controller.workspace.clone();
        self.job(async move {
            let result = async {
                match target {
                    WorkspaceTarget::Local { root } => open_document(root, path).await,
                    WorkspaceTarget::Ssh { workspace, root } => {
                        let filesystem = remote_filesystem(workspace_service, workspace, root).await?;
                        open_remote_document(filesystem, path).await
                    }
                }
            }.await;
            Ok(match result {
                Ok(document) => Update::Document { root, generation, document },
                Err(error) => Update::DocumentFailed { generation, error: error.to_string() },
            })
        });
        cx.notify();
    }

    fn save_file(&mut self, cx: &mut Context<Self>) {
        if self.explorer.modal_open() {
            return;
        }
        if self.saving {
            return;
        }
        let (Some(target), Some(document)) = (self.workspace_target(), self.document.clone())
        else {
            return;
        };
        let root = target.root().clone();
        let text = self.editor.read(cx).text().to_owned();
        let workspace_service = self.controller.workspace.clone();
        self.saving = true;
        self.error = None;
        self.job(async move {
            let path = document.path.clone();
            let result = match target {
                WorkspaceTarget::Local { root } => {
                    save_document(root, document, text.clone()).await
                }
                WorkspaceTarget::Ssh { workspace, root } => {
                    let filesystem = remote_filesystem(workspace_service, workspace, root).await?;
                    save_remote_document(filesystem, document, text.clone()).await
                }
            };
            match result {
                Ok(version) => Ok(Update::Saved {
                    root,
                    path,
                    text,
                    version,
                }),
                Err(error) => Ok(Update::SaveFailed(format!("Save failed: {error}"))),
            }
        });
        cx.notify();
    }
    fn poll(&mut self) {
        if self.polling {
            return;
        }
        if let Some(id) = self.selected
            && (self.panel == Panel::Inspector
                || self.busy.contains(&id)
                || self.connecting.contains(&id))
        {
            self.polling = true;
            let controller = self.controller.clone();
            let inspect = self.panel == Panel::Inspector;
            self.job(async move {
                let details = controller.details(id).await?;
                let trace = if inspect {
                    controller.trace(id, false).await?
                } else {
                    vec![]
                };
                Ok(Update::Details {
                    task: id,
                    details,
                    trace,
                })
            });
        }
    }
    fn receive(&mut self, update: Update, cx: &mut Context<Self>) {
        match update {
            Update::Hubs(reply) => self.hub_reply(*reply, cx),
            Update::Terminals(reply) => self.terminal_reply(*reply, cx),
            Update::Review(reply) => self.review_reply(*reply, cx),
            Update::Explorer(reply) => self.explorer_reply(*reply, cx),
            Update::Studio(reply) => self.studio_reply(*reply, cx),
            Update::SavedContext(reply) => self.saved_context_reply(*reply, cx),
            Update::Organization(reply) => self.organization_reply(*reply, cx),
            Update::ChatTools(reply) => self.chat_tools_reply(*reply, cx),
            Update::EnvironmentSaved(error) => self.environment_saved(error, cx),
            Update::Kanban(reply) => self.kanban_reply(*reply, cx),
            Update::DraftLoaded(task, result) => self.restore_draft(task, result, cx),
            Update::DraftSaved(task, error) => self.draft_saved(task, error, cx),
            Update::Registry(reply) => self.registry_reply(*reply, cx),
            Update::Tick => {
                self.tick_terminals(cx);
                self.tick_review(cx);
                self.refresh_message_search(cx);
                self.poll_kanban();
                self.flush_drafts(false);
                self.flush_environment(false);
                let previous = self.pending.len();
                self.pending.retain(|key, interaction| {
                    if !interaction.is_active() {
                        self.transcript.interaction_changed(key);
                    }
                    interaction.is_active()
                });
                self.forms.retain(|key, _| self.pending.contains_key(key));
                if self.pending.len() != previous {
                    cx.notify();
                }
                self.poll();
                return;
            }
            Update::Catalog(catalog) => self.catalog = catalog,
            Update::WorkspaceAdded(project, catalog) => {
                self.catalog = catalog;
                // The user may have edited while the workspace was opening.
                if self.dirty(cx) || self.saving || self.close != CloseState::Open {
                    self.notice = Some(
                        "Workspace added. Save or discard the document before selecting it.".into(),
                    );
                    cx.notify();
                    return;
                }
                self.project = Some(project.id);
                self.snapshot_draft(cx);
                self.selected = None;
                self.thread = None;
                self.reset_editor_tabs();
                self.document = None;
                self.files.clear();
                self.directory.clear();
                self.create_task(cx);
            }
            Update::TaskCreated(task, catalog, revision) => {
                self.creating_task = false;
                self.catalog = catalog;
                if self.task_title.read(cx).text().trim() == task.title.trim() {
                    self.task_title.update(cx, |entry, cx| entry.clear(cx));
                }
                if revision == self.selection_revision && self.select_task(task.id, cx) {
                    self.set_panel(Panel::Conversation, cx);
                } else {
                    self.notice = Some("The new thread and its draft were saved. Open it from thread search when ready.".into());
                }
            }
            Update::TaskCreationFailed(error) => {
                self.creating_task = false;
                self.error = Some(error);
            }
            Update::ThreadLoadFailed(task, error) => {
                if self.loading_task == Some(task) {
                    self.loading_task = None;
                }
                if self.selected == Some(task) {
                    self.error = Some(error);
                }
            }
            Update::ThreadLoaded(task, thread) => {
                if self.loading_task == Some(task.id) {
                    self.loading_task = None;
                }
                if self.selected == Some(task.id)
                    && self.thread.as_ref().is_none_or(|old| {
                        old.id != thread.id || old.last_sequence <= thread.last_sequence
                    })
                {
                    self.transcript.sync(&thread, None);
                    self.thread = Some(*thread);
                    self.replace_task(task);
                }
            }
            Update::Event(envelope) => {
                self.acknowledge_draft(&envelope, cx);
                if let Some(task) = self
                    .catalog
                    .tasks
                    .iter_mut()
                    .find(|t| t.thread_id == envelope.thread_id)
                {
                    task.updated_at_ms = envelope.timestamp_ms;
                    if let ThreadEvent::TitleChanged { title } = &envelope.event {
                        task.title = title.clone();
                    }
                }
                if self
                    .thread
                    .as_ref()
                    .is_some_and(|thread| thread.id == envelope.thread_id)
                {
                    let result = self.thread.as_mut().unwrap().apply(&envelope);
                    if let Err(error) = result {
                        if matches!(error, ReplayError::Sequence { .. }) {
                            self.hydrate();
                        } else {
                            self.error = Some(format!("Conversation update failed: {error}"));
                        }
                    }
                    if let Some(thread) = &self.thread {
                        self.transcript.sync(thread, Some(&envelope.event));
                    }
                    if let Some(thread) = &self.thread
                        && let Some(task) = self
                            .catalog
                            .tasks
                            .iter_mut()
                            .find(|t| Some(t.id) == self.selected)
                    {
                        task.state = thread.state;
                    }
                }
            }
            Update::Hydrate => self.hydrate(),
            Update::Interaction(interaction) => {
                if !interaction.is_active() {
                    return;
                }
                let key = match &interaction {
                    UiInteraction::Permission {
                        context, request, ..
                    } => (context.thread_id, request.id.clone()),
                    UiInteraction::Input {
                        context, request, ..
                    } => {
                        let key = (context.thread_id, request.id.clone());
                        let mut inputs = BTreeMap::new();
                        let mut values = BTreeMap::new();
                        for field in &request.fields {
                            match field.kind {
                                InputFieldKind::Text { .. } | InputFieldKind::Number { .. } => {
                                    inputs.insert(
                                        field.id.clone(),
                                        cx.new(|cx| {
                                            TextEntry::new(
                                                &field.label,
                                                EntryMode::SingleLine,
                                                36.,
                                                cx,
                                            )
                                        }),
                                    );
                                }
                                InputFieldKind::Boolean => {
                                    values.insert(field.id.clone(), InputValue::Boolean(false));
                                }
                                _ => {}
                            }
                        }
                        self.forms.insert(
                            key.clone(),
                            FormState {
                                request: request.clone(),
                                inputs,
                                values,
                                error: None,
                            },
                        );
                        key
                    }
                };
                self.transcript.interaction_changed(&key);
                self.pending.insert(key, interaction);
            }
            Update::Connected {
                task,
                details,
                error,
            } => {
                self.connecting.remove(&task);
                if self.selected == Some(task) {
                    self.details = details;
                    self.error = error;
                }
            }
            Update::PromptDone {
                task,
                details,
                error,
            } => {
                self.busy.remove(&task);
                if self.selected == Some(task) {
                    self.details = details;
                    self.error = error;
                }
                self.hydrate();
            }
            Update::Details {
                task,
                details,
                trace,
            } => {
                self.polling = false;
                if self.selected == Some(task) {
                    self.details = details;
                    if self.panel == Panel::Inspector {
                        self.trace = trace;
                    }
                }
            }
            Update::SettingsSaved(settings, error) => {
                self.settings.saving = false;
                if let Some(error) = error {
                    self.error = Some(error);
                } else {
                    self.settings.value = *settings;
                    self.composer.update(cx, |entry, _| {
                        entry.set_send_on_enter(self.settings.value.chat.send_on_enter)
                    });
                    cx.set_reduce_motion(self.settings.value.appearance.reduced_motion);
                    cx.refresh_windows();
                    if !self.settings.value.general.show_studio && self.navigation.studio {
                        let panel = self.panel;
                        self.switch_mode(false, cx);
                        self.panel = panel;
                        self.focus_composer = false;
                    }
                }
            }
            Update::ProfileActivity(activity) => {
                self.settings.activity_loading = false;
                self.settings.activity = Some(activity);
            }
            Update::ControlFinished {
                task,
                result,
                details,
            } => {
                self.controls.completed(task);
                tracing::debug!(target: "synara_ui_layout", "session-control-completed");
                match result {
                    Ok(changed) => {
                        if let Some(changed) = changed {
                            self.replace_task(changed);
                        }
                        if self.selected == Some(task) {
                            self.details = details;
                            self.error = None;
                        }
                    }
                    Err(error) => {
                        if self.selected == Some(task) {
                            self.error = Some(error.to_string());
                        }
                    }
                }
            }
            Update::Files {
                root,
                directory,
                entries,
            } => {
                if self.root() == Some(root) && self.directory == directory {
                    self.files = entries;
                    self.file_page = 0;
                }
            }
            Update::Document { root, generation, document } => {
                if self.editors.finish_open(generation)
                    && self.root() == Some(root)
                    && !self.saving && !self.explorer.modal_open()
                    && self.close == CloseState::Open
                {
                    self.install_editor_document(document, cx);
                    self.error = None;
                }
            }
            Update::DocumentFailed { generation, error } => {
                if self.editors.finish_open(generation) { self.error = Some(error); }
            }
            Update::SaveFailed(error) => {
                tracing::warn!("File save failed. The document remains open.");
                self.saving = false;
                self.close.saved(false);
                self.error = Some(error);
            }
            Update::Saved {
                root,
                path,
                text,
                version,
            } => {
                self.saving = false;
                if self.root() == Some(root)
                    && let Some(document) = self.document.as_mut().filter(|d| d.path == path)
                {
                    document.snapshot.text = text;
                    document.snapshot.version = version;
                    self.notice = Some("File saved".into());
                }
                self.sync_editor_document();
                self.finish_editor_close(cx);
                if self.close.saved(!self.active_document_dirty(cx)) {
                    self.begin_quit(cx);
                }
            }
            Update::Done(message) => {
                if !message.is_empty() {
                    self.notice = Some(message);
                }
                self.refresh_git_if_visible(cx);
            }
            Update::Error(error) => {
                // Only a save completion can release the outstanding save guard.
                self.polling = false;
                self.error = Some(error);
            }
        }
        cx.notify();
    }
    fn refresh_git_if_visible(&mut self, cx: &mut Context<Self>) {
        if self.panel == Panel::Changes {
            self.refresh_git(cx);
        }
    }
    fn set_panel(&mut self, panel: Panel, cx: &mut Context<Self>) {
        if panel != Panel::Hubs && self.hub_navigation_blocked(cx) { return; }
        if self.explorer.modal_open() {
            return;
        }
        self.studio.open = false;
        self.chat_tools.retire();
        self.environment.retire_popup();
        let panel = self.track_environment_panel(panel);
        self.controls.retire();
        self.settings.popup = None;
        if panel != Panel::Conversation {
            self.selection_revision = self.selection_revision.wrapping_add(1);
        }
        if self.settings.value.appearance.personalization.zen_mode
            && matches!(panel, Panel::Files | Panel::Changes | Panel::Terminal | Panel::Dock)
        { self.settings.personalization.tools_shown = true; }
        self.panel = panel;
        self.error = None;
        self.notice = None;
        match panel {
            Panel::Hubs => self.focus_composer = false,
            Panel::Registry => self.load_registry_if_needed(cx),
            Panel::Settings => {
                self.focus_composer = false;
                self.load_profile_activity();
            }
            Panel::Files => self.refresh_files(),
            Panel::Changes => self.refresh_git(cx),
            Panel::Terminal => self.ensure_terminals(cx),
            Panel::Inspector => self.poll(),
            _ => {}
        }
        cx.notify();
    }
}
fn button(
    id: impl Into<gpui::ElementId>,
    text: impl Into<SharedString>,
    active: bool,
) -> gpui::Stateful<gpui::Div> {
    crate::ui::button(id, text, active)
}

async fn remote_filesystem(
    workspace_service: WorkspaceService,
    workspace: Workspace,
    root: PathBuf,
) -> WorkspaceResult<synara_runtime::RemoteWorkspaceFs> {
    let profile = workspace_service
        .ssh_profile(workspace.id)
        .await?
        .ok_or_else(|| {
            WorkspaceError::Invalid("remote workspace is missing its pinned SSH profile".into())
        })?;
    profile.filesystem(&workspace, &root).await
}

async fn git_service(
    workspace_service: WorkspaceService,
    target: WorkspaceTarget,
) -> WorkspaceResult<GitService> {
    match target {
        WorkspaceTarget::Local { root } => Ok(GitService::new(root)),
        WorkspaceTarget::Ssh { workspace, root } => {
            let profile = workspace_service
                .ssh_profile(workspace.id)
                .await?
                .ok_or_else(|| {
                    WorkspaceError::Invalid(
                        "remote workspace is missing its pinned SSH profile".into(),
                    )
                })?;
            Ok(GitService::with_host(
                root,
                Arc::new(profile.host(&workspace)?),
            ))
        }
    }
}

fn truncate(text: &str, limit: usize) -> String {
    if text.len() <= limit {
        return text.into();
    }
    let mut end = limit;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!(
        "{}\n[Display shortened. Copy the full content to inspect it.]",
        &text[..end]
    )
}
