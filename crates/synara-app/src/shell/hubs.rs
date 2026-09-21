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
    description: Entity<TextEntry>,
    instructions: Entity<TextEntry>,
    memory: Entity<TextEntry>,
    query: Entity<TextEntry>,
    include: bool,
    folder: Option<PathBuf>,
    picker: bool,
    show_archived: bool,
    error: Option<String>,
    focus_form: bool,
    pending_selection: Option<(TaskId, u64, bool)>,
    _subscriptions: Vec<Subscription>,
}
impl HubState {
    pub fn new(cx: &mut Context<Shell>) -> Self {
        let name = cx.new(|cx| TextEntry::new("Hub name", EntryMode::SingleLine, 36., cx));
        let description = cx.new(|cx| TextEntry::new("What is this Hub for?", EntryMode::SingleLine, 36., cx));
        let instructions = cx.new(|cx| TextEntry::new("How should agents approach this work?", EntryMode::Editor, 140., cx));
        let memory = cx.new(|cx| TextEntry::new("Shared decisions, facts and reference notes", EntryMode::Editor, 160., cx));
        let query = cx.new(|cx| TextEntry::new("Find a Hub", EntryMode::SingleLine, 32., cx));
        let subscriptions = [&name,&description,&instructions,&memory,&query].into_iter()
            .map(|entry| cx.subscribe(entry, |_,_,_,cx| cx.notify())).collect();
        Self { rows: Vec::new(), selected: None, loaded: false, loading: false, reload: false,
            saving: false, creating: false, editing: false, original: None, name, description,
            instructions, memory, query, include: true, folder: None, picker: false,
            show_archived: false, error: None, focus_form: false, pending_selection: None,
            _subscriptions: subscriptions }
    }
    pub fn error_message(&self) -> Option<&str> { self.error.as_deref() }
    pub fn pending(&self, cx: &App) -> bool {
        self.saving || self.creating || self.picker || self.dirty(cx)
            || self.editing && [&self.name,&self.description,&self.instructions,&self.memory].iter().any(|entry| entry.read(cx).is_composing())
    }
    fn dirty(&self, cx: &App) -> bool {
        if !self.editing { return false; }
        let fields = [self.name.read(cx).text(),self.description.read(cx).text(),self.instructions.read(cx).text(),self.memory.read(cx).text()];
        match &self.original {
            Some(profile) => fields != [profile.name.as_str(),profile.description.as_str(),profile.instructions.as_str(),profile.memory.as_str()]
                || self.include != profile.include_in_new_threads,
            None => fields.iter().any(|value| !value.is_empty()) || self.folder.is_some(),
        }
    }
}
impl Shell {
    pub(super) fn load_hubs(&mut self) {
        if self.hubs.loading { self.hubs.reload = true; return; }
        self.hubs.loading = true;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            let result = async { Ok((workspace.hubs().await?,workspace.catalog().await?)) }.await;
            Ok(Update::Hubs(Box::new(Reply::Loaded(result.map_err(|error: WorkspaceError| error.to_string())))))
        });
    }
    pub(super) fn hub_navigation_blocked(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.hubs.pending(cx) { return false; }
        self.error = Some("Save or explicitly discard the Hub editor before leaving it.".into());
        cx.notify(); true
    }
    fn hub_profile(&self) -> Option<&HubProfile> {
        self.hubs.rows.iter().find(|hub| Some(hub.profile.project) == self.hubs.selected).map(|hub| &hub.profile)
    }
    pub(super) fn open_hub(&mut self, project: ProjectId, cx: &mut Context<Self>) {
        if self.hub_navigation_blocked(cx) { return; }
        let Some(profile) = self.hubs.rows.iter().find(|hub| hub.profile.project == project).map(|hub| hub.profile.clone()) else { return };
        let task = self.catalog.tasks.iter().find(|task| task.id == profile.main_task)
            .or_else(|| self.catalog.tasks.iter().find(|task| task.project_id == project && task.scope == TaskScope::Studio));
        if let Some(task) = task {
            if !self.select_task(task.id, cx) { return; }
        } else {
            if self.dirty(cx) || self.saving {
                self.error = Some("Save the open file before changing Hub workspaces.".into()); cx.notify(); return;
            }
            self.snapshot_draft(cx);
            self.selection_revision = self.selection_revision.wrapping_add(1);
            self.selected = None; self.loading_task = None; self.thread = None; self.details = None;
            self.project = Some(project); self.reset_editor_tabs(); self.document = None;
            self.files.clear(); self.directory.clear(); self.studio.reset();
            self.composer.update(cx, |entry,cx| entry.clear(cx));
        }
        self.hubs.selected = Some(project);
        self.navigation.studio = true;
        self.hubs.editing = false;
        self.set_panel(Panel::Hubs, cx);
        self.focus_composer = false;
    }
    pub(super) fn show_hubs(&mut self, cx: &mut Context<Self>) {
        if self.hub_navigation_blocked(cx) { return; }
        if !self.navigation.studio { self.navigation.last_synara = self.selected; }
        self.hubs.selected = self.task().filter(|task| task.scope == TaskScope::Studio).map(|task| task.project_id);
        self.navigation.studio = true;
        self.set_panel(Panel::Hubs, cx);
        self.focus_composer = false;
        if !self.hubs.loaded { self.load_hubs(); }
    }
    pub(super) fn edit_hub(&mut self, create: bool, cx: &mut Context<Self>) {
        if self.hub_navigation_blocked(cx) { return; }
        let profile = if create { None } else { self.hub_profile().cloned() };
        if !create && profile.is_none() { return; }
        let values = profile.as_ref().map(|p| [p.name.clone(),p.description.clone(),p.instructions.clone(),p.memory.clone()]).unwrap_or_default();
        for (entry,text) in [&self.hubs.name,&self.hubs.description,&self.hubs.instructions,&self.hubs.memory].into_iter().zip(values) {
            entry.update(cx, |entry,cx| entry.set_text(text,cx));
        }
        self.hubs.include = profile.as_ref().is_none_or(|p| p.include_in_new_threads);
        self.hubs.original = profile;
        self.hubs.folder = None; self.hubs.editing = true; self.hubs.error = None;
        self.hubs.focus_form = true;
        self.navigation.studio = true; self.set_panel(Panel::Hubs,cx); self.focus_composer = false;
    }
    fn save_hub_editor(&mut self, cx: &mut Context<Self>) {
        if self.hubs.saving || self.hubs.creating || self.hubs.picker || !self.hubs.editing { return; }
        let name = self.hubs.name.read(cx).text().trim().to_owned();
        if name.is_empty() || name.len() > 160 || name.chars().any(char::is_control) {
            self.hubs.error = Some("Enter a Hub name of at most 160 bytes.".into()); cx.notify(); return;
        }
        let workspace = self.controller.workspace.clone();
        if let Some(mut profile) = self.hubs.original.clone() {
            profile.name = name;
            profile.description = self.hubs.description.read(cx).text().to_owned();
            profile.instructions = self.hubs.instructions.read(cx).text().to_owned();
            profile.memory = self.hubs.memory.read(cx).text().to_owned();
            profile.include_in_new_threads = self.hubs.include;
            if let Err(error) = profile.validate() { self.hubs.error = Some(error.to_string()); cx.notify(); return; }
            self.hubs.saving = true;
            self.job(async move { Ok(Update::Hubs(Box::new(Reply::Saved(workspace.save_hub(profile.revision,profile).await.map_err(|e| e.to_string()))))) });
        } else {
            let Some(agent) = self.settings.value.general.default_provider.clone().or_else(|| self.profiles.first().map(|p| p.id.clone())) else { return };
            let managed = self.hubs.folder.is_none();
            let root = self.hubs.folder.clone().unwrap_or_else(|| self.scratch_directory.join("hubs").join(ProjectId::new().to_string()));
            let revision = self.selection_revision;
            self.hubs.creating = true;
            self.job(async move {
                let result = async {
                    if managed { tokio::fs::create_dir_all(&root).await.map_err(synara_runtime::RuntimeError::Io)?; }
                    workspace.create_hub(root,name,agent).await
                }.await;
                Ok(Update::Hubs(Box::new(Reply::Created(result.map_err(|e| e.to_string()),revision))))
            });
        }
        cx.notify();
    }
    fn pick_hub_folder(&mut self, cx: &mut Context<Self>) {
        if self.hubs.picker || self.hubs.creating { return; }
        self.hubs.picker = true;
        let picker = cx.prompt_for_paths(gpui::PathPromptOptions { files: false, directories: true, multiple: false, prompt: Some("Choose Hub working folder".into()) });
        cx.spawn(async move |view,cx| {
            let result = picker.await;
            let _ = view.update(cx, |this,cx| {
                this.hubs.picker = false;
                match result {
                    Ok(Ok(Some(paths))) => this.hubs.folder = paths.into_iter().next(),
                    Ok(Ok(None)) => {},
                    _ => this.hubs.error = Some("The folder picker could not open.".into()),
                }
                cx.notify();
            });
        }).detach();
    }
    pub(super) fn new_hub_thread(&mut self, cx: &mut Context<Self>) {
        if self.hub_navigation_blocked(cx) || self.creating_task || self.loading_task.is_some() { return; }
        let Some(profile) = self.hub_profile().cloned() else { self.edit_hub(true,cx); return };
        if profile.archived { self.hubs.error = Some("Restore the Hub before creating a thread.".into()); cx.notify(); return; }
        let Some(agent) = self.task().filter(|t|t.project_id == profile.project).map(|t| t.agent_id.clone())
            .or_else(|| self.settings.value.general.default_provider.clone()).or_else(|| self.profiles.first().map(|p|p.id.clone())) else { return };
        let title = self.task_title.read(cx).text().trim().to_owned();
        let title = if title.is_empty() { "New Hub thread".into() } else { title };
        let revision = self.selection_revision;
        let workspace = self.controller.workspace.clone();
        self.creating_task = true;
        self.job(async move { Ok(Update::Hubs(Box::new(Reply::Thread(workspace.create_hub_thread(profile.project,title,agent).await.map_err(|e| e.to_string()),revision)))) });
        cx.notify();
    }
    fn archive_hub(&mut self, cx: &mut Context<Self>) {
        if self.hub_navigation_blocked(cx) { return; }
        let Some(mut profile) = self.hub_profile().cloned() else { return };
        profile.archived = !profile.archived; self.hubs.saving = true;
        let workspace = self.controller.workspace.clone();
        self.job(async move { Ok(Update::Hubs(Box::new(Reply::Saved(workspace.save_hub(profile.revision,profile).await.map_err(|e|e.to_string()))))) });
        cx.notify();
    }
    fn use_hub_context(&mut self, cx: &mut Context<Self>) {
        if self.hub_navigation_blocked(cx) || self.loading_task.is_some() || self.composer.read(cx).is_composing() { return; }
        let Some(profile) = self.hub_profile() else { return };
        if self.task().is_none_or(|task|task.project_id != profile.project || task.scope != TaskScope::Studio) { return; }
        let context = profile.context_draft();
        let text = self.composer.read(cx).text();
        if context.is_empty() { self.notice = Some("This Hub has no shared context yet.".into()); }
        else if context.len().saturating_add(text.len()) > 1024*1024 { self.error = Some("Combined draft exceeds 1 MiB. Nothing was changed.".into()); }
        else {
            let text = format!("{context}{text}");
            self.composer.update(cx,|entry,cx|entry.set_text(text,cx)); self.remember_draft(cx);
            self.show_conversation(cx);
            self.notice = Some("Shared context added to the visible draft. Review it before sending.".into());
        }
        cx.notify();
    }
    pub(super) fn promote_hub_message(&mut self, task: TaskId, anchor: MessageAnchor, cx: &mut Context<Self>) {
        if self.selected != Some(task) || self.hub_navigation_blocked(cx) { return; }
        let Some(profile) = self.hub_profile().cloned().filter(|profile| self.task().is_some_and(|task|
            task.project_id == profile.project && task.scope == TaskScope::Studio)) else { return; };
        let Some(message) = self.thread.as_ref().and_then(|thread|thread.messages.iter().find(|message|anchor.matches(message))) else { return; };
        if message.text.len() > 64 * 1024 {
            self.error = Some("This message is too large for shared knowledge. Copy a smaller selection instead.".into()); cx.notify(); return;
        }
        let addition = format!("\n\nQuoted from thread {task}, message {}:\n{}",anchor.id,
            message.text.lines().map(|line|format!("> {line}")).collect::<Vec<_>>().join("\n"));
        if profile.memory.len().saturating_add(addition.len()) > 64 * 1024 {
            self.error = Some("Shared knowledge would exceed 64 KiB. Copy a smaller selection instead.".into()); cx.notify(); return;
        }
        let text = format!("{}{addition}",profile.memory);
        self.edit_hub(false,cx);
        self.hubs.memory.update(cx,|entry,cx|entry.set_text(text,cx));
        self.notice = Some("Review this quoted message in shared knowledge, then Save context to share it with new threads.".into());
        cx.notify();
    }
    pub(super) fn hub_reply(&mut self, reply: Reply, cx: &mut Context<Self>) {
        match reply {
            Reply::Loaded(result) => {
                self.hubs.loading = false;
                match result {
                    Ok((rows,catalog)) => {
                        self.hubs.rows = rows; self.hubs.loaded = true; self.catalog = catalog; self.hubs.error = None;
                        if self.hubs.reload {
                            self.hubs.reload = false; self.load_hubs(); cx.notify(); return;
                        }
                        if let Some((task,revision,home)) = self.hubs.pending_selection.take() {
                            if revision == self.selection_revision && self.select_task(task,cx) {
                                self.hubs.selected = self.task().map(|t|t.project_id);
                                if home { self.set_panel(Panel::Hubs,cx); self.focus_composer = false; }
                                else { self.show_conversation(cx); }
                            } else { self.notice = Some("Hub work was saved. Open it from Hubs when ready.".into()); }
                        }
                    }
                    Err(error) => self.hubs.error = Some(error),
                }
            }
            Reply::Created(result,revision) => {
                self.hubs.creating = false;
                match result {
                    Ok((profile,task)) => {
                        if self.hubs.name.read(cx).text().trim() == profile.name {
                            self.hubs.name.update(cx,|entry,cx|entry.set_text(profile.name.clone(),cx));
                        }
                        self.hubs.selected = Some(profile.project);
                        self.hubs.original = Some(profile.clone());
                        self.hubs.rows.push(HubSummary { profile, threads: 1, imported: false });
                        if !self.hubs.dirty(cx) {
                            self.hubs.editing = false;
                            self.hubs.pending_selection = Some((task.id,revision,true));
                        } else {
                            self.notice = Some("Hub created. Your newer edits remain in the context editor.".into());
                        }
                        self.load_hubs();
                    }
                    Err(error) => self.hubs.error = Some(error),
                }
            }
            Reply::Saved(result) => {
                self.hubs.saving = false;
                match result {
                    Ok(profile) => {
                        if !self.hubs.editing { self.hubs.show_archived = profile.archived; }
                        if self.hubs.editing {
                            if self.hubs.name.read(cx).text().trim() == profile.name {
                                self.hubs.name.update(cx,|entry,cx|entry.set_text(profile.name.clone(),cx));
                            }
                            self.hubs.original = Some(profile.clone());
                        }
                        if let Some(row) = self.hubs.rows.iter_mut().find(|row|row.profile.project == profile.project) { row.profile = profile; row.imported = false; }
                        self.hubs.error = None; self.notice = Some("Hub context saved. Existing transcripts and drafts were not changed.".into());
                    }
                    Err(error) => self.hubs.error = Some(error),
                }
            }
            Reply::Thread(result,revision) => {
                self.creating_task = false;
                match result {
                    Ok(task) => {
                        if self.task_title.read(cx).text().trim() == task.title { self.task_title.update(cx,|e,cx|e.clear(cx)); }
                        self.hubs.pending_selection = Some((task.id,revision,false)); self.load_hubs();
                    }
                    Err(error) => self.hubs.error = Some(error),
                }
            }
        }
        cx.notify();
    }
    pub(super) fn restore_hub_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.hubs.focus_form && self.panel == Panel::Hubs {
            self.hubs.focus_form = false; window.focus(&self.hubs.name.read(cx).focus_handle(cx),cx);
        }
    }
}
