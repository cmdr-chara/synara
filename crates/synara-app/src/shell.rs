use gpui::Focusable;
mod chrome;
mod composer;
mod controls;
mod conversation;
mod messages;
mod navigation;
mod panels;
mod registry;
mod terminal;
mod transcript;
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
    pub agent_directory: PathBuf,
    pub catalog: Catalog,
    pub profiles: Vec<AgentProfile>,
    pub selection: Selection,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Panel {
    Conversation,
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
    Registry(Box<registry::RegistryReply>),
    Catalog(Catalog),
    WorkspaceAdded(Project, Catalog),
    TaskCreated(Task, Catalog),
    ThreadLoaded(Task, Box<Thread>),
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
    Profiles(Vec<AgentProfile>),
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
        document: Document,
    },
    SaveFailed(String),
    Saved {
        root: PathBuf,
        path: PathBuf,
        text: String,
        version: synara_runtime::FileVersion,
    },
    Git {
        root: PathBuf,
        status: GitStatus,
        diff: String,
        staged: bool,
    },
    TerminalStarted {
        root: PathBuf,
        generation: u64,
        terminal: TerminalSession,
    },
    TerminalOutput {
        root: PathBuf,
        generation: u64,
        snapshot: TerminalRenderSnapshot,
    },
    TerminalFailed {
        generation: u64,
        error: String,
    },
    TerminalShutdown {
        generation: u64,
        error: Option<String>,
    },
    Tick,
    Done(String),
    Error(String),
}
pub struct Shell {
    controls: controls::ControlState,
    navigation: navigation::NavigationState,
    close: CloseState,
    close_focus: gpui::FocusHandle,
    registry: registry::RegistryState,
    controller: Arc<Controller>,
    runtime: Handle,
    sender: async_channel::Sender<Update>,
    catalog: Catalog,
    profiles: Vec<AgentProfile>,
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
    profile_editor: Entity<TextEntry>,
    editor: Entity<TextEntry>,
    commit_message: Entity<TextEntry>,
    terminal_view: Entity<TerminalView>,
    drafts: HashMap<TaskId, String>,
    busy: HashSet<TaskId>,
    connecting: HashSet<TaskId>,
    panel: Panel,
    error: Option<String>,
    notice: Option<String>,
    focus_composer: bool,
    transcript: transcript::TranscriptState,
    pending: HashMap<InteractionKey, UiInteraction>,
    forms: HashMap<InteractionKey, FormState>,
    files: Vec<FileEntry>,
    directory: PathBuf,
    file_page: usize,
    document: Option<Document>,
    saving: bool,
    git: GitStatus,
    diff: String,
    staged: bool,
    terminal: Option<TerminalSession>,
    terminal_root: Option<PathBuf>,
    terminal_generation: u64,
    terminal_starting: bool,
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
        let profile_editor = cx
            .new(|cx| TextEntry::new("Agent launch profiles (JSON)", EntryMode::Editor, 390., cx));
        let editor =
            cx.new(|cx| TextEntry::new("Select a UTF-8 text file", EntryMode::Editor, 480., cx));
        let commit_message =
            cx.new(|cx| TextEntry::new("Commit message", EntryMode::SingleLine, 38., cx));
        let terminal_view = cx.new(TerminalView::new);
        profile_editor.update(cx, |entry, cx| {
            entry.set_text(
                serde_json::to_string_pretty(&bootstrap.profiles).unwrap_or_default(),
                cx,
            )
        });
        let registry = registry::RegistryState::new(bootstrap.agent_directory, cx);
        let subscriptions = vec![
            cx.subscribe(&registry.query, |_, _, _, cx| cx.notify()),
            cx.subscribe(&composer, |this, _, event, cx| match event {
                EntryEvent::Submit => this.send_prompt(cx),
                _ => cx.notify(),
            }),
            cx.subscribe(&workspace_path, |this, _, event, cx| {
                if matches!(event, EntryEvent::Submit) {
                    this.open_workspace(cx)
                }
            }),
            cx.subscribe(&task_title, |this, _, event, cx| {
                if matches!(event, EntryEvent::Submit) {
                    this.create_task(cx)
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
            controls: controls::ControlState::new(cx),
            navigation: navigation::NavigationState::new(cx),
            close: CloseState::Open,
            close_focus: cx.focus_handle(),
            registry,
            controller,
            runtime,
            sender,
            catalog: bootstrap.catalog,
            profiles: bootstrap.profiles,
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
            profile_editor,
            editor,
            commit_message,
            terminal_view,
            drafts: HashMap::new(),
            busy: HashSet::new(),
            connecting: HashSet::new(),
            panel: Panel::Conversation,
            error: None,
            notice: None,
            focus_composer: false,
            transcript: transcript::TranscriptState::new(),
            pending: HashMap::new(),
            forms: HashMap::new(),
            files: vec![],
            directory: PathBuf::new(),
            file_page: 0,
            document: None,
            saving: false,
            git: GitStatus::default(),
            diff: String::new(),
            staged: false,
            terminal: None,
            terminal_root: None,
            terminal_generation: 0,
            terminal_starting: false,
            terminal_closing: false,
            polling: false,
            _updates: updates,
            _subscriptions: subscriptions,
        };
        if let Some(selected) = selected {
            this.select_task(selected, cx);
        }
        this
    }
    pub fn request_close(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.terminal_closing {
            return false;
        }
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
        if self.terminal_closing {
            return;
        }
        self.terminal_closing = true;
        if self.terminal_starting {
            self.notice = Some("Waiting for terminal startup before closing Synara...".into());
            cx.notify();
            return;
        }
        let Some(terminal) = self.terminal.clone() else {
            self.terminal_closing = false;
            cx.quit();
            return;
        };
        let generation = self.terminal_generation;
        self.notice = Some("Stopping the terminal before closing Synara...".into());
        self.queue_terminal_shutdown(terminal, generation);
        cx.notify();
    }

    fn queue_terminal_shutdown(&self, terminal: TerminalSession, generation: u64) {
        self.job(async move {
            let result = async {
                terminal.kill()?;
                tokio::time::timeout(std::time::Duration::from_secs(5), terminal.wait())
                    .await
                    .map_err(|_| synara_runtime::RuntimeError::Timeout)??;
                Ok::<(), synara_runtime::RuntimeError>(())
            }
            .await;
            Ok(Update::TerminalShutdown {
                generation,
                error: result.err().map(|error| error.to_string()),
            })
        });
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
                .child(div().text_xl().child(if terminal_closing && self.terminal_starting {
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
                        .on_click(cx.listener(|this, _, _, cx| { if !this.saving { this.begin_quit(cx); } }))))
                    .children((!waiting && !terminal_closing).then(|| button("save-and-close", "Save and close", true)
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
        self.document
            .as_ref()
            .is_some_and(|d| self.editor.read(cx).text() != d.snapshot.text)
    }
    fn replace_task(&mut self, task: Task) {
        if let Some(existing) = self.catalog.tasks.iter_mut().find(|t| t.id == task.id) {
            *existing = task;
        } else {
            self.catalog.tasks.insert(0, task);
        }
    }
    fn select_task(&mut self, id: TaskId, cx: &mut Context<Self>) {
        let Some(task) = self
            .catalog
            .tasks
            .iter()
            .find(|task| task.id == id)
            .cloned()
        else {
            return;
        };
        if Some(task.project_id) != self.project && (self.dirty(cx) || self.saving) {
            self.error =
                Some("Save or discard the open document before switching projects.".into());
            cx.notify();
            return;
        }
        if let Some(previous) = self.selected {
            self.drafts
                .insert(previous, self.composer.read(cx).text().to_owned());
        }
        if Some(task.project_id) != self.project {
            self.document = None;
            self.files.clear();
            self.directory.clear();
            self.git = GitStatus::default();
            self.diff.clear();
        }
        self.controls.retire();
        self.selected = Some(id);
        self.project = Some(task.project_id);
        self.details = None;
        self.trace.clear();
        self.thread = Some(Thread::new(task.thread_id));
        self.error = None;
        self.composer.update(cx, |entry, cx| {
            entry.set_text(self.drafts.get(&id).cloned().unwrap_or_default(), cx)
        });
        self.transcript = transcript::TranscriptState::new();
        self.focus_composer = true;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            workspace
                .save_selection(Selection {
                    project: Some(task.project_id),
                    task: Some(id),
                })
                .await?;
            let thread = workspace.thread(task.thread_id).await?;
            Ok(Update::ThreadLoaded(task, Box::new(thread)))
        });
        if self.panel == Panel::Files {
            self.refresh_files();
        }
        if self.panel == Panel::Changes {
            self.refresh_git();
        }
        cx.notify();
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
        let Some(project) = self.project else {
            self.error = Some("Open a workspace first.".into());
            cx.notify();
            return;
        };
        let Some(agent) = self.profiles.first().map(|p| p.id.clone()) else {
            return;
        };
        let title = self.task_title.read(cx).text().trim().to_owned();
        let title = if title.is_empty() {
            "New task".into()
        } else {
            title
        };
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            let task = workspace.create_task(project, title, agent).await?;
            Ok(Update::TaskCreated(task, workspace.catalog().await?))
        });
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
        self.busy.insert(id);
        self.error = None;
        self.notice = None;
        self.transcript.follow();
        let controller = self.controller.clone();
        self.job(async move {
            let result = controller.submit(id, text).await;
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
    fn save_profiles(&mut self, cx: &mut Context<Self>) {
        let profiles = match parse_profiles(self.profile_editor.read(cx).text()) {
            Ok(p) => p,
            Err(e) => {
                self.error = Some(e.to_string());
                cx.notify();
                return;
            }
        };
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            workspace.save_profiles(profiles.clone()).await?;
            Ok(Update::Profiles(profiles))
        });
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

    fn refresh_git(&self) {
        let Some(target) = self.workspace_target() else {
            return;
        };
        let root = target.root().clone();
        let staged = self.staged;
        let workspace_service = self.controller.workspace.clone();
        self.job(async move {
            let git = git_service(workspace_service, target).await?;
            let status = git.status().await?;
            let diff = git.diff(staged, None).await?;
            Ok(Update::Git {
                root,
                status,
                diff,
                staged,
            })
        });
    }

    fn git_index_path(&self, path: PathBuf, unstage: bool) {
        let Some(target) = self.workspace_target() else {
            return;
        };
        let workspace_service = self.controller.workspace.clone();
        self.job(async move {
            let git = git_service(workspace_service, target).await?;
            if unstage {
                git.unstage(path).await?;
            } else {
                git.stage(path).await?;
            }
            Ok(Update::Done("Index updated".into()))
        });
    }

    fn commit_git(&mut self, message: String, cx: &mut Context<Self>) {
        if message.trim().is_empty() {
            self.error = Some("Enter a commit message first.".into());
            cx.notify();
            return;
        }
        let Some(target) = self.workspace_target() else {
            return;
        };
        let workspace_service = self.controller.workspace.clone();
        self.job(async move {
            git_service(workspace_service, target)
                .await?
                .commit(message)
                .await?;
            Ok(Update::Done(
                "Commit created in the selected workspace".into(),
            ))
        });
    }

    fn open_file(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if self.dirty(cx) || self.saving {
            self.error =
                Some("Save or discard the current document before opening another file.".into());
            cx.notify();
            return;
        }
        let Some(target) = self.workspace_target() else {
            return;
        };
        let root = target.root().clone();
        let workspace_service = self.controller.workspace.clone();
        self.job(async move {
            let document = match target {
                WorkspaceTarget::Local { root } => open_document(root, path).await?,
                WorkspaceTarget::Ssh { workspace, root } => {
                    let filesystem = remote_filesystem(workspace_service, workspace, root).await?;
                    open_remote_document(filesystem, path).await?
                }
            };
            Ok(Update::Document { root, document })
        });
    }

    fn save_file(&mut self, cx: &mut Context<Self>) {
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
    fn start_terminal(&mut self, cx: &mut Context<Self>) {
        let Some(target) = self.workspace_target() else {
            return;
        };
        let root = target.root().clone();
        if self.terminal_starting || self.terminal_closing {
            return;
        }
        let previous = self.terminal.take();
        self.terminal_generation = self.terminal_generation.wrapping_add(1);
        let generation = self.terminal_generation;
        self.terminal_starting = true;
        self.terminal_root = None;
        self.terminal_view
            .update(cx, |terminal, cx| terminal.clear_session(cx));
        let workspace_service = self.controller.workspace.clone();
        self.job(async move {
            if let Some(previous) = previous {
                let cleanup = async {
                    previous.kill()?;
                    tokio::time::timeout(std::time::Duration::from_secs(5), previous.wait())
                        .await
                        .map_err(|_| synara_runtime::RuntimeError::Timeout)??;
                    Ok::<(), synara_runtime::RuntimeError>(())
                }
                .await;
                if let Err(error) = cleanup {
                    return Ok(Update::TerminalFailed {
                        generation,
                        error: format!("previous terminal cleanup failed: {error}"),
                    });
                }
            }
            let result = match target {
                WorkspaceTarget::Local { root: cwd } => {
                    tokio::task::spawn_blocking(move || {
                        #[cfg(windows)]
                        let command =
                            std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".into());
                        #[cfg(not(windows))]
                        let command =
                            std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
                        NativeTerminal::spawn(
                            &synara_runtime::LaunchSpec::new(command),
                            &cwd,
                            24,
                            88,
                        )
                        .map(|terminal| TerminalSession::Local(Arc::new(terminal)))
                    })
                    .await
                    .map_err(|_| WorkspaceError::Worker)?
                    .map_err(WorkspaceError::from)
                }
                WorkspaceTarget::Ssh { workspace, root } => {
                    #[cfg(unix)]
                    {
                        let profile = workspace_service
                            .ssh_profile(workspace.id)
                            .await?
                            .ok_or_else(|| {
                                WorkspaceError::Invalid(
                                    "remote workspace is missing its pinned SSH profile".into(),
                                )
                            })?;
                        let host = profile.host(&workspace)?;
                        tokio::task::spawn_blocking(move || {
                            synara_runtime::RemoteTerminal::spawn(
                                &host,
                                &synara_runtime::LaunchSpec::new("/bin/sh"),
                                &root,
                                24,
                                88,
                            )
                            .map(|terminal| TerminalSession::Remote(Arc::new(terminal)))
                        })
                        .await
                        .map_err(|_| WorkspaceError::Worker)?
                        .map_err(WorkspaceError::from)
                    }
                    #[cfg(not(unix))]
                    {
                        let _ = (workspace_service, workspace, root);
                        Err(WorkspaceError::Runtime(
                            synara_runtime::RuntimeError::Unsupported(
                                "remote interactive terminals currently require the validated Unix PTY backend".into(),
                            ),
                        ))
                    }
                }
            };
            match result {
                Ok(terminal) => Ok(Update::TerminalStarted {
                    root,
                    generation,
                    terminal,
                }),
                Err(error) => Ok(Update::TerminalFailed {
                    generation,
                    error: error.to_string(),
                }),
            }
        });
        self.notice = Some("Starting an interactive shell in the selected workspace".into());
        cx.notify();
    }

    fn interrupt_terminal(&mut self, cx: &mut Context<Self>) {
        if let Some(terminal) = &self.terminal {
            let result = terminal.key(
                TerminalKey::Character('c'),
                TerminalModifiers {
                    control: true,
                    ..TerminalModifiers::default()
                },
            );
            if let Err(error) = result {
                self.error = Some(error.to_string());
            }
        }
        cx.notify();
    }

    fn stop_terminal(&mut self, cx: &mut Context<Self>) {
        if let Some(terminal) = &self.terminal {
            match terminal.kill() {
                Ok(()) => self.notice = Some("Shell stop requested".into()),
                Err(error) => self.error = Some(error.to_string()),
            }
        }
        cx.notify();
    }
    fn poll(&mut self) {
        if let Some(terminal) = self.terminal.clone()
            && self.panel == Panel::Terminal
            && let Some(root) = self.terminal_root.clone()
        {
            let generation = self.terminal_generation;
            self.job(async move {
                let snapshot = tokio::task::spawn_blocking(move || terminal.render_snapshot())
                    .await
                    .map_err(|_| WorkspaceError::Worker)??;
                Ok(Update::TerminalOutput {
                    root,
                    generation,
                    snapshot,
                })
            });
        }
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
            Update::Registry(reply) => self.registry_reply(*reply, cx),
            Update::Tick => {
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
                self.selected = None;
                self.thread = None;
                self.document = None;
                self.files.clear();
                self.directory.clear();
                self.create_task(cx);
            }
            Update::TaskCreated(task, catalog) => {
                self.catalog = catalog;
                self.task_title.update(cx, |entry, cx| entry.clear(cx));
                self.select_task(task.id, cx);
            }
            Update::ThreadLoaded(task, thread) => {
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
                    if let ThreadEvent::TextDelta {
                        role: Role::User,
                        text,
                        ..
                    } = &envelope.event
                        && self.composer.read(cx).text() == text
                    {
                        self.composer.update(cx, |entry, cx| entry.clear(cx));
                        if let Some(id) = self.selected {
                            self.drafts.remove(&id);
                        }
                    }
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
            Update::Profiles(profiles) => {
                self.profiles = profiles;
                self.notice = Some(
                    "Agent profiles saved. Launch changes apply when a task reconnects.".into(),
                );
                self.error = None;
            }
            Update::ControlFinished {
                task,
                result,
                details,
            } => {
                self.controls.completed(task);
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
            Update::Document { root, document } => {
                if self.root() == Some(root) && !self.dirty(cx) && !self.saving {
                    self.editor.update(cx, |entry, cx| {
                        entry.set_text(document.snapshot.text.clone(), cx)
                    });
                    self.document = Some(document);
                    self.error = None;
                }
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
                if self.close.saved(!self.dirty(cx)) {
                    self.begin_quit(cx);
                }
            }
            Update::Git {
                root,
                status,
                diff,
                staged,
            } => {
                if self.root() == Some(root) && self.staged == staged {
                    self.git = status;
                    self.diff = diff;
                }
            }
            Update::TerminalStarted {
                root,
                generation,
                terminal,
            } => {
                if generation != self.terminal_generation || self.root() != Some(root.clone()) {
                    self.retire_terminal(terminal);
                    return;
                }
                self.terminal_starting = false;
                self.terminal = Some(terminal.clone());
                self.terminal_root = Some(root);
                self.terminal_view
                    .update(cx, |view, cx| view.set_session(terminal.clone(), cx));
                if self.terminal_closing {
                    self.notice = Some("Stopping the terminal before closing Synara...".into());
                    self.queue_terminal_shutdown(terminal, generation);
                } else {
                    self.notice = None;
                    self.poll();
                }
            }
            Update::TerminalOutput {
                root,
                generation,
                snapshot,
            } => {
                if generation == self.terminal_generation && self.terminal_root == Some(root) {
                    self.terminal_view
                        .update(cx, |view, cx| view.set_snapshot(snapshot, cx));
                }
            }
            Update::TerminalFailed { generation, error } => {
                if generation == self.terminal_generation {
                    self.terminal_starting = false;
                    self.terminal = None;
                    self.terminal_root = None;
                    if self.terminal_closing {
                        self.terminal_closing = false;
                        self.close.cancel();
                        self.error = Some(format!(
                            "Terminal startup or retirement failed while closing; Synara stayed open to preserve process ownership: {error}"
                        ));
                    } else {
                        self.error = Some(error);
                    }
                }
            }
            Update::TerminalShutdown { generation, error } => {
                if generation != self.terminal_generation {
                    return;
                }
                self.terminal_closing = false;
                if let Some(error) = error {
                    self.close.cancel();
                    self.error = Some(format!(
                        "Terminal shutdown failed; Synara stayed open to preserve process ownership: {error}"
                    ));
                } else {
                    self.terminal = None;
                    self.terminal_root = None;
                    cx.quit();
                    return;
                }
            }
            Update::Done(message) => {
                if !message.is_empty() {
                    self.notice = Some(message);
                }
                self.refresh_git_if_visible();
            }
            Update::Error(error) => {
                // Only a save completion can release the outstanding save guard.
                self.polling = false;
                self.error = Some(error);
            }
        }
        cx.notify();
    }
    fn refresh_git_if_visible(&self) {
        if self.panel == Panel::Changes {
            self.refresh_git();
        }
    }
    fn set_panel(&mut self, panel: Panel, cx: &mut Context<Self>) {
        self.controls.retire();
        self.panel = panel;
        self.error = None;
        self.notice = None;
        match panel {
            Panel::Registry => self.load_registry_if_needed(cx),
            Panel::Files => self.refresh_files(),
            Panel::Changes => self.refresh_git(),
            Panel::Inspector | Panel::Terminal => self.poll(),
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
