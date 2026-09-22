//! Project-scoped terminal ownership. Layout restore never executes a command.
//! Every asynchronous reply carries a scope, tab identity and process generation.
mod view;
use super::*;
use std::time::{Duration, Instant};

const MAX_LIVE_TERMINALS: usize = 24;
const MAX_OPEN_SCOPES: usize = 32;

#[derive(Clone, Debug)]
pub(super) struct Key {
    scope: TerminalScope,
    id: u64,
    generation: u64,
}
#[derive(Clone, Copy, PartialEq)]
pub(super) enum StopAction {
    Keep,
    Close,
    Restart,
}

#[derive(Default)]
pub(super) struct TerminalWorkspace {
    groups: HashMap<TerminalScope, Group>,
    tick: u64,
}
struct Group {
    target: WorkspaceTarget,
    layout: TerminalLayout,
    entries: HashMap<u64, Entry>,
    loaded: bool,
    loading: bool,
    load_error: Option<String>,
    edits: u64,
    saved_edits: u64,
    changed_at: Instant,
    saving: bool,
    save_error: Option<String>,
    confirmation: Option<(u64, StopAction)>,
    renaming: Option<u64>,
    reload_confirm: bool,
}
struct Entry {
    view: Entity<TerminalView>,
    name: Entity<TextEntry>,
    query: Entity<TextEntry>,
    _subscriptions: Vec<Subscription>,
    search: bool,
    matches: (usize, usize),
    process: Option<TerminalSession>,
    history: Option<TerminalSession>,
    generation: u64,
    starting: bool,
    stopping: bool,
    polling: bool,
    exited: bool,
    error: Option<String>,
}
impl Group {
    fn changed(&mut self) {
        self.edits = self.edits.wrapping_add(1);
        self.changed_at = Instant::now();
    }
    fn pending_save(&self) -> bool {
        self.edits != self.saved_edits || self.saving
    }
    fn running(&self) -> bool {
        self.entries.values().any(|entry| {
            entry.starting || entry.stopping || entry.process.is_some() && !entry.exited
        })
    }
}
impl TerminalWorkspace {
    pub(super) fn starting(&self) -> bool {
        self.groups
            .values()
            .any(|group| group.entries.values().any(|entry| entry.starting))
    }
    pub(super) fn running(&self) -> bool {
        self.groups.values().any(Group::running)
    }
}

pub(super) enum Reply {
    Loaded(TerminalScope, Result<TerminalLayout, String>),
    Saved(TerminalScope, u64, Result<TerminalLayout, String>),
    Started(Key, Result<TerminalSession, String>),
    Snapshot(Key, Result<TerminalRenderSnapshot, String>),
    Stopped(Key, StopAction, Result<TerminalRenderSnapshot, String>),
}

impl Shell {
    fn terminal_scope(&self) -> Option<TerminalScope> {
        Some(TerminalScope {
            project: self.project?,
            root: self.root()?,
        })
    }
    fn terminal_entry(&self, key: &Key) -> Option<&Entry> {
        self.terminals
            .groups
            .get(&key.scope)?
            .entries
            .get(&key.id)
            .filter(|entry| entry.generation == key.generation)
    }
    fn terminal_entry_mut(&mut self, key: &Key) -> Option<&mut Entry> {
        self.terminals
            .groups
            .get_mut(&key.scope)?
            .entries
            .get_mut(&key.id)
            .filter(|entry| entry.generation == key.generation)
    }
    fn terminal_key(&self, scope: &TerminalScope, id: u64) -> Option<Key> {
        Some(Key {
            scope: scope.clone(),
            id,
            generation: self
                .terminals
                .groups
                .get(scope)?
                .entries
                .get(&id)?
                .generation,
        })
    }
    pub(super) fn ensure_terminals(&mut self, cx: &mut Context<Self>) {
        let (Some(scope), Some(target)) = (self.terminal_scope(), self.workspace_target()) else {
            return;
        };
        if self.terminals.groups.contains_key(&scope) {
            return;
        }
        if self.terminals.groups.len() >= MAX_OPEN_SCOPES {
            self.notice = Some("Terminal workspaces are limited to 32 open projects per app session. Existing shells are retained.".into());
            return;
        }
        self.terminals.groups.insert(
            scope.clone(),
            Group {
                target,
                layout: TerminalLayout::default(),
                entries: HashMap::new(),
                loaded: false,
                loading: false,
                load_error: None,
                edits: 0,
                saved_edits: 0,
                changed_at: Instant::now(),
                saving: false,
                save_error: None,
                confirmation: None,
                renaming: None,
                reload_confirm: false,
            },
        );
        self.load_terminals(scope, cx);
    }
    fn load_terminals(&mut self, scope: TerminalScope, cx: &mut Context<Self>) {
        let Some(group) = self.terminals.groups.get_mut(&scope) else {
            return;
        };
        if group.loading || group.saving || group.running() {
            return;
        }
        group.loading = true;
        group.load_error = None;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            let result = workspace
                .terminal_layout(scope.clone())
                .await
                .map_err(|error| error.to_string());
            Ok(Update::Terminals(Box::new(Reply::Loaded(scope, result))))
        });
        cx.notify();
    }
    fn make_terminal_entry(
        &self,
        scope: TerminalScope,
        tab: &TerminalTabLayout,
        cx: &mut Context<Self>,
    ) -> Entry {
        let view = cx.new(TerminalView::new);
        let name = cx.new(|cx| TextEntry::new("Terminal name", EntryMode::SingleLine, 32., cx));
        name.update(cx, |input, cx| input.set_text(tab.name.clone(), cx));
        let query =
            cx.new(|cx| TextEntry::new("Find in visible rows", EntryMode::SingleLine, 32., cx));
        let id = tab.id;
        let focus_scope = scope.clone();
        let subscription = cx.subscribe(&query, move |this, _, event, cx| {
            if matches!(event, EntryEvent::Changed | EntryEvent::Submit) {
                this.search_terminal(
                    &scope,
                    id,
                    matches!(event, EntryEvent::Submit).then_some(false),
                    cx,
                );
            }
        });
        let focus_subscription = cx.subscribe(
            &view,
            move |this, source, event: &terminal::WorkspaceEvent, cx| {
                if this.terminal_scope().as_ref() != Some(&focus_scope)
                    || !this
                        .terminals
                        .groups
                        .get(&focus_scope)
                        .is_some_and(|group| {
                            !group.loading
                                && group.entries.get(&id).is_some_and(|entry| {
                                    entry.view.entity_id() == source.entity_id()
                                })
                        })
                {
                    return;
                }
                this.terminal_view = source.clone();
                match event {
                    terminal::WorkspaceEvent::Focused => {}
                    terminal::WorkspaceEvent::New => this.new_terminal(true, cx),
                    terminal::WorkspaceEvent::Close => {
                        this.request_terminal_action(focus_scope.clone(), id, StopAction::Close, cx)
                    }
                    terminal::WorkspaceEvent::Find => {
                        if let Some(entry) = this
                            .terminals
                            .groups
                            .get_mut(&focus_scope)
                            .and_then(|group| group.entries.get_mut(&id))
                        {
                            entry.search = !entry.search;
                            let focus = entry.search.then(|| entry.query.read(cx).focus_handle(cx));
                            entry
                                .view
                                .update(cx, |view, cx| view.request_focus(focus, cx));
                        }
                    }
                    terminal::WorkspaceEvent::Next(backwards) => {
                        if let Some(group) = this.terminals.groups.get_mut(&focus_scope) {
                            let ids: Vec<_> = group.layout.tabs.iter().map(|tab| tab.id).collect();
                            if !ids.is_empty() {
                                let index = ids
                                    .iter()
                                    .position(|candidate| *candidate == id)
                                    .unwrap_or(0);
                                let next = if *backwards {
                                    (index + ids.len() - 1) % ids.len()
                                } else {
                                    (index + 1) % ids.len()
                                };
                                if group.layout.secondary == Some(ids[next]) {
                                    group.layout.secondary = group.layout.active;
                                }
                                group.layout.active = Some(ids[next]);
                                if let Some(entry) = group.entries.get(&ids[next]) {
                                    entry
                                        .view
                                        .update(cx, |view, cx| view.request_focus(None, cx));
                                }
                                group.changed();
                            }
                        }
                    }
                }
                this.focus_composer = false;
                cx.notify();
            },
        );
        Entry {
            view,
            name,
            query,
            _subscriptions: vec![subscription, focus_subscription],
            search: false,
            matches: (0, 0),
            process: None,
            history: None,
            generation: 0,
            starting: false,
            stopping: false,
            polling: false,
            exited: false,
            error: None,
        }
    }
    pub(super) fn new_terminal(&mut self, start: bool, cx: &mut Context<Self>) {
        self.ensure_terminals(cx);
        let Some(scope) = self.terminal_scope() else {
            return;
        };
        let Some(group) = self
            .terminals
            .groups
            .get(&scope)
            .filter(|group| group.loaded && !group.loading)
        else {
            return;
        };
        if group.layout.tabs.len() >= MAX_TERMINAL_TABS || self.terminal_closing {
            self.notice = Some(
                "Each workspace supports up to 12 terminal tabs. Close an unused tab first.".into(),
            );
            cx.notify();
            return;
        }
        let Some(next_id) = group.layout.next_id.checked_add(1) else {
            return;
        };
        let tab = TerminalTabLayout {
            id: group.layout.next_id,
            name: format!("Terminal {}", group.layout.next_id),
        };
        let entry = self.make_terminal_entry(scope.clone(), &tab, cx);
        let group = self.terminals.groups.get_mut(&scope).unwrap();
        group.layout.next_id = next_id;
        group.layout.active = Some(tab.id);
        group.layout.tabs.push(tab.clone());
        group.entries.insert(tab.id, entry);
        group.changed();
        self.focus_composer = false;
        self.set_panel(Panel::Terminal, cx);
        if start {
            self.launch_terminal(scope, tab.id, cx);
        }
        cx.notify();
    }
    fn launch_terminal(&mut self, scope: TerminalScope, id: u64, cx: &mut Context<Self>) {
        if self.terminal_closing {
            return;
        }
        let live = self
            .terminals
            .groups
            .values()
            .flat_map(|group| group.entries.values())
            .filter(|entry| entry.starting || entry.process.is_some() && !entry.exited)
            .count();
        if live >= MAX_LIVE_TERMINALS {
            self.notice = Some("The app supports 24 simultaneous terminal processes. Stop a shell before starting another.".into());
            cx.notify();
            return;
        }
        let Some(group) = self
            .terminals
            .groups
            .get_mut(&scope)
            .filter(|group| group.loaded && !group.loading)
        else {
            return;
        };
        let Some(entry) = group.entries.get_mut(&id) else {
            return;
        };
        if entry.starting || entry.stopping || entry.process.is_some() {
            return;
        }
        let Some(generation) = entry.generation.checked_add(1) else {
            return;
        };
        entry.generation = generation;
        entry.polling = false;
        entry.starting = true;
        entry.exited = false;
        entry.error = None;
        let target = group.target.clone();
        let key = Key {
            scope,
            id,
            generation,
        };
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            let result = spawn_session(workspace, target)
                .await
                .map_err(|error| error.to_string());
            Ok(Update::Terminals(Box::new(Reply::Started(key, result))))
        });
        cx.notify();
    }
    fn request_terminal_action(
        &mut self,
        scope: TerminalScope,
        id: u64,
        action: StopAction,
        cx: &mut Context<Self>,
    ) {
        let Some(group) = self.terminals.groups.get_mut(&scope) else {
            return;
        };
        let Some(entry) = group.entries.get(&id) else {
            return;
        };
        if group.loading || entry.starting || entry.stopping || self.terminal_closing {
            return;
        }
        if entry.process.is_some() && !entry.exited {
            group.confirmation = Some((id, action));
        } else if action == StopAction::Close {
            self.remove_terminal(&scope, id, cx);
        } else if entry.process.is_some() {
            self.stop_terminal_entry(scope, id, action, cx);
        } else if action == StopAction::Restart {
            self.launch_terminal(scope, id, cx);
        }
        cx.notify();
    }
    fn stop_terminal_entry(
        &mut self,
        scope: TerminalScope,
        id: u64,
        action: StopAction,
        cx: &mut Context<Self>,
    ) {
        let Some(mut key) = self.terminal_key(&scope, id) else {
            return;
        };
        let Some(entry) = self.terminal_entry_mut(&key) else {
            return;
        };
        if entry.starting || entry.stopping {
            return;
        }
        let Some(process) = entry.process.clone() else {
            return;
        };
        let Some(generation) = entry.generation.checked_add(1) else {
            return;
        };
        entry.generation = generation;
        entry.polling = false;
        key.generation = generation;
        entry.stopping = true;
        entry.error = None;
        self.job(async move {
            let result = async {
                process.kill()?;
                tokio::time::timeout(Duration::from_secs(5), process.wait())
                    .await
                    .map_err(|_| synara_runtime::RuntimeError::Timeout)??;
                tokio::task::spawn_blocking(move || process.render_snapshot())
                    .await
                    .map_err(|_| WorkspaceError::Worker)?
                    .map_err(WorkspaceError::from)
            }
            .await
            .map_err(|error: WorkspaceError| error.to_string());
            Ok(Update::Terminals(Box::new(Reply::Stopped(
                key, action, result,
            ))))
        });
        cx.notify();
    }
    fn remove_terminal(&mut self, scope: &TerminalScope, id: u64, cx: &mut Context<Self>) {
        let Some(group) = self.terminals.groups.get_mut(scope) else {
            return;
        };
        if group.entries.get(&id).is_some_and(|entry| {
            entry.starting || entry.stopping || entry.process.is_some() && !entry.exited
        }) {
            return;
        }
        group.entries.remove(&id);
        let previous = group
            .layout
            .tabs
            .iter()
            .position(|tab| tab.id == id)
            .unwrap_or(0);
        group.layout.tabs.retain(|tab| tab.id != id);
        if group.layout.active == Some(id) {
            group.layout.active = group
                .layout
                .tabs
                .get(previous.min(group.layout.tabs.len().saturating_sub(1)))
                .map(|tab| tab.id);
        }
        if group.layout.secondary == Some(id) || group.layout.secondary == group.layout.active {
            group.layout.secondary = None;
        }
        group.confirmation = None;
        if group.renaming == Some(id) {
            group.renaming = None;
        }
        group.changed();
        cx.notify();
    }
    fn select_terminal(
        &mut self,
        scope: &TerminalScope,
        id: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(group) = self.terminals.groups.get_mut(scope) else {
            return;
        };
        let Some(entry) = group.entries.get(&id) else {
            return;
        };
        let focus = entry.view.read(cx).focus_handle(cx);
        self.terminal_view = entry.view.clone();
        if group.layout.secondary == Some(id) {
            group.layout.secondary = group.layout.active;
        }
        if group.layout.active != Some(id) {
            group.layout.active = Some(id);
            group.changed();
        }
        self.focus_composer = false;
        window.focus(&focus, cx);
        cx.notify();
    }
    fn split_terminals(&mut self, cx: &mut Context<Self>) {
        let Some(scope) = self.terminal_scope() else {
            return;
        };
        let Some(group) = self.terminals.groups.get_mut(&scope) else {
            return;
        };
        if !group.loaded || group.loading {
            return;
        }
        if group.layout.secondary.is_some() {
            group.layout.secondary = None;
            group.changed();
        } else if let Some(tab) = group
            .layout
            .tabs
            .iter()
            .find(|tab| Some(tab.id) != group.layout.active)
        {
            group.layout.secondary = Some(tab.id);
            group.changed();
        } else {
            let previous = group.layout.active;
            self.new_terminal(false, cx);
            if let Some(group) = self.terminals.groups.get_mut(&scope) {
                group.layout.secondary = previous.filter(|id| Some(*id) != group.layout.active);
                group.changed();
            }
        }
        cx.notify();
    }
    fn rename_terminal(
        &mut self,
        scope: &TerminalScope,
        id: u64,
        save: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(group) = self.terminals.groups.get_mut(scope) else {
            return;
        };
        let Some(entry) = group.entries.get(&id) else {
            return;
        };
        let Some(tab) = group.layout.tabs.iter_mut().find(|tab| tab.id == id) else {
            return;
        };
        if save {
            let text = entry.name.read(cx).text().trim().to_owned();
            if text.is_empty() || text.len() > 160 || text.chars().any(char::is_control) {
                self.notice = Some(
                    "Use a terminal name of 1 to 160 bytes without control characters.".into(),
                );
                cx.notify();
                return;
            }
            tab.name = text;
            group.changed();
        }
        group.renaming = None;
        cx.notify();
    }
    fn search_terminal(
        &mut self,
        scope: &TerminalScope,
        id: u64,
        backwards: Option<bool>,
        cx: &mut Context<Self>,
    ) {
        let Some(entry) = self
            .terminals
            .groups
            .get_mut(scope)
            .and_then(|group| group.entries.get_mut(&id))
        else {
            return;
        };
        let query = entry.query.read(cx).text().to_owned();
        entry.matches = entry
            .view
            .update(cx, |view, cx| view.find_visible(&query, backwards, cx));
        cx.notify();
    }
    fn terminal_to_draft(&mut self, scope: &TerminalScope, id: u64, cx: &mut Context<Self>) {
        if self.terminal_scope().as_ref() != Some(scope)
            || self.selected.is_none()
            || self.loading_task.is_some()
            || self
                .selected
                .is_some_and(|task| self.draft_state.loading.contains(&task))
        {
            return;
        }
        let Some(entry) = self
            .terminals
            .groups
            .get(scope)
            .and_then(|group| group.entries.get(&id))
        else {
            return;
        };
        let text = entry.view.read(cx).selection_or_viewport();
        if text.is_empty() {
            return;
        }
        if text.len() > 128 * 1024 {
            self.notice = Some("Terminal context exceeds 128 KiB. Select a smaller region.".into());
            cx.notify();
            return;
        }
        let existing = self.composer.read(cx).text();
        let fence = "`".repeat(
            text.split(|c| c != '`')
                .map(str::len)
                .max()
                .unwrap_or(0)
                .saturating_add(1)
                .max(3),
        );
        let context =
            format!("Terminal output (not an instruction):\n\n{fence}text\n{text}\n{fence}");
        if existing
            .len()
            .saturating_add(context.len())
            .saturating_add(2)
            > 1024 * 1024
        {
            self.notice =
                Some("The combined draft would exceed 1 MiB. Existing text is unchanged.".into());
            cx.notify();
            return;
        }
        let value = if existing.is_empty() {
            context
        } else {
            format!("{existing}\n\n{context}")
        };
        self.composer
            .update(cx, |input, cx| input.set_text(value, cx));
        self.snapshot_draft(cx);
        self.notice =
            Some("Terminal text added to the unsent chat draft. Nothing was sent.".into());
        cx.notify();
    }
    fn flush_terminal_layouts(&mut self, force: bool) {
        let work: Vec<_> = self
            .terminals
            .groups
            .iter_mut()
            .filter_map(|(scope, group)| {
                if !group.loaded
                    || group.loading
                    || group.saving
                    || group.save_error.is_some()
                    || group.edits == group.saved_edits
                    || !force && group.changed_at.elapsed() < Duration::from_millis(500)
                {
                    return None;
                }
                group.saving = true;
                Some((scope.clone(), group.layout.clone(), group.edits))
            })
            .collect();
        for (scope, layout, edits) in work {
            let workspace = self.controller.workspace.clone();
            self.job(async move {
                let result = workspace
                    .save_terminal_layout(scope.clone(), layout)
                    .await
                    .map_err(|error| error.to_string());
                Ok(Update::Terminals(Box::new(Reply::Saved(
                    scope, edits, result,
                ))))
            });
        }
    }
    pub(super) fn tick_terminals(&mut self, cx: &mut Context<Self>) {
        if self.panel == Panel::Terminal {
            self.ensure_terminals(cx);
        }
        self.flush_terminal_layouts(false);
        self.terminals.tick = self.terminals.tick.wrapping_add(1);
        let visible_scope = self.terminal_scope();
        let poll_background = self.terminals.tick.is_multiple_of(4);
        let mut jobs = Vec::new();
        for (scope, group) in &mut self.terminals.groups {
            let visible = self.panel == Panel::Terminal && visible_scope.as_ref() == Some(scope);
            for (id, entry) in &mut group.entries {
                if entry.starting
                    || entry.stopping
                    || entry.polling
                    || entry.exited && !visible
                    || !(poll_background
                        || visible
                            && (group.layout.active == Some(*id)
                                || group.layout.secondary == Some(*id)))
                {
                    continue;
                }
                if let Some(process) = entry.process.as_ref().or(entry.history.as_ref()).cloned() {
                    entry.polling = true;
                    jobs.push((
                        Key {
                            scope: scope.clone(),
                            id: *id,
                            generation: entry.generation,
                        },
                        process,
                    ));
                }
            }
        }
        for (key, process) in jobs {
            self.job(async move {
                let result = tokio::task::spawn_blocking(move || process.render_snapshot())
                    .await
                    .map_err(|error| error.to_string())
                    .and_then(|result| result.map_err(|error| error.to_string()));
                Ok(Update::Terminals(Box::new(Reply::Snapshot(key, result))))
            });
        }
    }
    pub(super) fn terminal_layout_before_quit(&mut self, cx: &mut Context<Self>) -> bool {
        self.flush_terminal_layouts(true);
        let pending = self.terminals.groups.values().any(|group| {
            group.pending_save() || group.renaming.is_some() || group.confirmation.is_some()
        });
        if pending {
            self.notice = Some("Finish terminal confirmations or name edits and let tab layouts save before closing. Shells remain running.".into());
            self.close.cancel();
            cx.notify();
        }
        pending
    }
    pub(super) fn begin_terminal_shutdown(&mut self, cx: &mut Context<Self>) {
        self.terminal_closing = true;
        self.continue_terminal_shutdown(cx);
    }
    fn continue_terminal_shutdown(&mut self, cx: &mut Context<Self>) {
        if !self.terminal_closing {
            return;
        }
        let work: Vec<_> = self
            .terminals
            .groups
            .iter()
            .flat_map(|(scope, group)| {
                group
                    .entries
                    .iter()
                    .filter(|(_, entry)| {
                        entry.process.is_some() && !entry.starting && !entry.stopping
                    })
                    .map(|(id, _)| (scope.clone(), *id))
            })
            .collect();
        for (scope, id) in work {
            self.stop_terminal_entry(scope, id, StopAction::Keep, cx);
        }
        let pending = self.terminals.groups.values().any(|group| {
            group
                .entries
                .values()
                .any(|entry| entry.process.is_some() || entry.starting || entry.stopping)
        });
        if !pending {
            self.terminal_closing = false;
            cx.quit();
        } else {
            self.notice = Some("Stopping all terminal processes before closing Synara...".into());
            cx.notify();
        }
    }
    pub(super) fn terminal_reply(&mut self, reply: Reply, cx: &mut Context<Self>) {
        match reply {
            Reply::Loaded(scope, result) => {
                if !self.terminals.groups.contains_key(&scope) {
                    return;
                }
                match result {
                    Ok(layout) => {
                        let entries = layout
                            .tabs
                            .iter()
                            .map(|tab| (tab.id, self.make_terminal_entry(scope.clone(), tab, cx)))
                            .collect();
                        let group = self.terminals.groups.get_mut(&scope).unwrap();
                        group.layout = layout;
                        group.entries = entries;
                        group.loaded = true;
                        group.loading = false;
                        group.saved_edits = group.edits;
                        group.save_error = None;
                        group.confirmation = None;
                        group.renaming = None;
                        group.reload_confirm = false;
                    }
                    Err(error) => {
                        let group = self.terminals.groups.get_mut(&scope).unwrap();
                        group.loading = false;
                        group.load_error = Some(error);
                    }
                }
            }
            Reply::Saved(scope, edits, result) => {
                let Some(group) = self.terminals.groups.get_mut(&scope) else {
                    return;
                };
                group.saving = false;
                match result {
                    Ok(value) => {
                        group.layout.revision = value.revision;
                        group.saved_edits = edits;
                    }
                    Err(error) => {
                        group.save_error = Some(error);
                        self.notice = Some(
                            "Terminal layout was not saved. Local tabs and processes are retained."
                                .into(),
                        );
                    }
                }
            }
            Reply::Started(key, result) => {
                let Some(entry) = self.terminal_entry_mut(&key) else {
                    if let Ok(process) = result {
                        self.retire_terminal(process);
                    }
                    return;
                };
                entry.starting = false;
                match result {
                    Ok(process) => {
                        entry.history = None;
                        entry.process = Some(process.clone());
                        entry
                            .view
                            .update(cx, |view, cx| view.set_session(process, cx));
                    }
                    Err(error) => {
                        entry.error = Some(error);
                        entry.exited = entry.history.is_some();
                    }
                }
            }
            Reply::Snapshot(key, result) => {
                let Some(entry) = self.terminal_entry_mut(&key) else {
                    return;
                };
                entry.polling = false;
                if entry.stopping || entry.process.is_none() && entry.history.is_none() {
                    return;
                }
                match result {
                    Ok(snapshot) => {
                        entry.exited = snapshot.exit_code.is_some();
                        entry
                            .view
                            .update(cx, |view, cx| view.set_snapshot(snapshot, cx));
                        if entry.search {
                            let query = entry.query.read(cx).text().to_owned();
                            entry.matches = entry
                                .view
                                .update(cx, |view, cx| view.find_visible(&query, None, cx));
                        }
                        if entry.exited && entry.process.is_some() {
                            entry.history = entry.process.take();
                            entry.view.update(cx, |view, cx| view.clear_session(cx));
                        }
                    }
                    Err(error) => entry.error = Some(error),
                }
            }
            Reply::Stopped(key, action, result) => {
                let Some(entry) = self.terminal_entry_mut(&key) else {
                    return;
                };
                entry.stopping = false;
                match result {
                    Ok(snapshot) => {
                        entry.history = entry.process.take();
                        entry.exited = true;
                        entry.view.update(cx, |view, cx| {
                            view.set_snapshot(snapshot, cx);
                            view.clear_session(cx);
                        });
                        if action == StopAction::Close {
                            self.remove_terminal(&key.scope, key.id, cx);
                        } else if action == StopAction::Restart && !self.terminal_closing {
                            self.launch_terminal(key.scope, key.id, cx);
                        }
                    }
                    Err(error) => {
                        entry.error = Some(error);
                        if self.terminal_closing {
                            self.terminal_closing = false;
                            self.close.cancel();
                            self.error = Some("A terminal did not finish shutting down. Synara stayed open and retained process ownership. Retry Stop in its workspace.".into());
                        }
                    }
                }
            }
        }
        self.continue_terminal_shutdown(cx);
        cx.notify();
    }
}

async fn spawn_session(
    workspace: WorkspaceService,
    target: WorkspaceTarget,
) -> WorkspaceResult<TerminalSession> {
    match target {
        WorkspaceTarget::Local { root } => tokio::task::spawn_blocking(move || {
            #[cfg(windows)]
            let command = std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".into());
            #[cfg(not(windows))]
            let command = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
            NativeTerminal::spawn(&synara_runtime::LaunchSpec::new(command), &root, 24, 88)
                .map(|process| TerminalSession::Local(Arc::new(process)))
        })
        .await
        .map_err(|_| WorkspaceError::Worker)?
        .map_err(WorkspaceError::from),
        WorkspaceTarget::Ssh {
            workspace: remote,
            root,
        } => {
            #[cfg(unix)]
            {
                let profile = workspace.ssh_profile(remote.id).await?.ok_or_else(|| {
                    WorkspaceError::Invalid(
                        "Remote workspace is missing its pinned SSH profile".into(),
                    )
                })?;
                let host = profile.host(&remote)?;
                tokio::task::spawn_blocking(move || {
                    synara_runtime::RemoteTerminal::spawn(
                        &host,
                        &synara_runtime::LaunchSpec::new("/bin/sh"),
                        &root,
                        24,
                        88,
                    )
                    .map(|process| TerminalSession::Remote(Arc::new(process)))
                })
                .await
                .map_err(|_| WorkspaceError::Worker)?
                .map_err(WorkspaceError::from)
            }
            #[cfg(not(unix))]
            {
                let _ = (workspace, remote, root);
                Err(WorkspaceError::Runtime(
                    synara_runtime::RuntimeError::Unsupported(
                        "Remote interactive terminals require the validated Unix PTY backend"
                            .into(),
                    ),
                ))
            }
        }
    }
}
