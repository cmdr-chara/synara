//! Chat utilities are separate from submission, approval and transcript authority.
use super::*;
use crate::ui::{
    self, Glyph,
    menu::{Choice, ChoiceEvent, ChoiceMenu},
    palette,
};
use gpui::{EntityInputHandler, FocusHandle};
use std::sync::atomic::{AtomicU64, Ordering};

pub(super) enum Reply {
    Pins(TaskId, Result<Vec<MessageAnchor>, String>),
    PinSaved(TaskId, Result<Vec<MessageAnchor>, String>),
    Search {
        task: TaskId,
        epoch: u64,
        jump: bool,
        result: Result<MessageSearch, String>,
    },
    Copied(Result<String, String>),
    Exported(Result<(), String>),
    Ignored,
}
#[derive(Clone)]
enum Action {
    Find,
    Pins,
    Copy,
    Export,
    ReuseLast,
    Commands,
    Jump(MessageAnchor),
    RemovePin(MessageAnchor),
    Insert(String),
    SelectTask(TaskId),
}
struct Popup {
    view: Entity<ChoiceMenu>,
    position: gpui::Point<gpui::Pixels>,
    previous_focus: Option<FocusHandle>,
    _subscription: Subscription,
}
pub(super) struct ChatTools {
    pub query: Entity<TextEntry>,
    pub find_open: bool,
    pub focused: Option<MessageAnchor>,
    pub work_scroll: gpui::ScrollHandle,
    search: MessageSearch,
    search_error: Option<String>,
    epoch: Arc<AtomicU64>,
    searching: bool,
    pins: HashMap<TaskId, Vec<MessageAnchor>>,
    pin_errors: HashMap<TaskId, String>,
    pub pin_writes: HashSet<TaskId>,
    loading_pins: HashSet<TaskId>,
    exporting: bool,
    copying: bool,
    popup: Option<Popup>,
    pending_action: Option<(Option<TaskId>, Option<Action>)>,
    _query_subscription: Subscription,
}
impl ChatTools {
    pub fn new(cx: &mut Context<Shell>) -> Self {
        let query = cx.new(|cx| {
            TextEntry::new(
                "Find in this conversation...",
                EntryMode::SingleLine,
                32.,
                cx,
            )
            .with_leading_icon(Glyph::Search)
        });
        let subscription = cx.subscribe(&query, |this, _, event, cx| match event {
            EntryEvent::Changed => this.queue_message_search(true, cx),
            EntryEvent::Submit => this.step_message_search(false, cx),
            _ => {}
        });
        Self {
            query,
            find_open: false,
            focused: None,
            work_scroll: gpui::ScrollHandle::new(),
            search: MessageSearch::default(),
            search_error: None,
            epoch: Arc::new(AtomicU64::new(0)),
            searching: false,
            pins: HashMap::new(),
            pin_errors: HashMap::new(),
            pin_writes: HashSet::new(),
            loading_pins: HashSet::new(),
            exporting: false,
            copying: false,
            popup: None,
            pending_action: None,
            _query_subscription: subscription,
        }
    }
    pub fn reset_selection(&mut self) {
        self.epoch.fetch_add(1, Ordering::SeqCst);
        self.find_open = false;
        self.searching = false;
        self.search = MessageSearch::default();
        self.search_error = None;
        self.focused = None;
        self.popup = None;
        self.pending_action = None;
    }
    pub fn pending_write(&self) -> bool {
        !self.pin_writes.is_empty() || self.exporting
    }
    pub fn menu_open(&self) -> bool {
        self.popup.is_some()
    }
    pub fn retire(&mut self) {
        self.popup = None;
        self.pending_action = None;
    }
}

fn append_draft(existing: &str, addition: &str) -> Result<String, &'static str> {
    let separator = if existing.is_empty() { "" } else { "\n\n" };
    if existing
        .len()
        .saturating_add(addition.len())
        .saturating_add(separator.len())
        > 1024 * 1024
    {
        return Err("The combined draft exceeds 1 MiB. Nothing was changed.");
    }
    Ok(format!("{existing}{separator}{addition}"))
}
fn command_text(command: &SlashCommand) -> Option<String> {
    let name = command.name.strip_prefix('/').unwrap_or(&command.name);
    (!name.is_empty()
        && name.len() <= 128
        && name
            .chars()
            .all(|ch| ch.is_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.')))
    .then(|| format!("/{name} "))
}
fn role_label(role: Role) -> &'static str {
    match role {
        Role::User => "User",
        Role::Assistant => "Assistant",
        Role::Reasoning => "Reasoning",
    }
}
fn snippet(text: &str) -> String {
    text.split_whitespace()
        .take(24)
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(120)
        .collect()
}
fn next_hit(
    hits: &[MessageAnchor],
    current: Option<&MessageAnchor>,
    backwards: bool,
) -> Option<MessageAnchor> {
    if hits.is_empty() {
        return None;
    }
    let index = current.and_then(|current| hits.iter().position(|hit| hit == current));
    let next = match index {
        Some(index) if backwards => (index + hits.len() - 1) % hits.len(),
        Some(index) => (index + 1) % hits.len(),
        None if backwards => hits.len() - 1,
        None => 0,
    };
    Some(hits[next].clone())
}
impl Shell {
    pub(super) fn load_message_pins(&mut self, task: TaskId) {
        if self.chat_tools.pins.contains_key(&task) || !self.chat_tools.loading_pins.insert(task) {
            return;
        }
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::ChatTools(Box::new(Reply::Pins(
                task,
                workspace
                    .message_pins(task)
                    .await
                    .map_err(|error| error.to_string()),
            ))))
        });
    }
    fn pinned(&self, anchor: &MessageAnchor) -> bool {
        self.selected
            .and_then(|task| self.chat_tools.pins.get(&task))
            .is_some_and(|pins| pins.contains(anchor))
    }
    fn set_pin(&mut self, anchor: MessageAnchor, enabled: bool, cx: &mut Context<Self>) {
        let Some(task) = self.selected else {
            return;
        };
        if self.close != CloseState::Open
            || !self.chat_tools.pins.contains_key(&task)
            || !self.chat_tools.pin_writes.insert(task)
        {
            return;
        }
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::ChatTools(Box::new(Reply::PinSaved(
                task,
                workspace
                    .set_message_pin(task, anchor, enabled)
                    .await
                    .map_err(|error| error.to_string()),
            ))))
        });
        cx.notify();
    }
    pub(super) fn chat_tools_reply(&mut self, reply: Reply, cx: &mut Context<Self>) {
        match reply {
            Reply::Pins(task, result) => {
                self.chat_tools.loading_pins.remove(&task);
                if let std::collections::hash_map::Entry::Vacant(entry) =
                    self.chat_tools.pins.entry(task)
                {
                    match result {
                        Ok(pins) => {
                            entry.insert(pins);
                            self.chat_tools.pin_errors.remove(&task);
                        }
                        Err(error) => {
                            self.chat_tools.pin_errors.insert(task, error);
                        }
                    }
                }
                if self.selected == Some(task) {
                    self.transcript
                        .list
                        .remeasure_items(0..self.transcript.list.item_count());
                }
            }
            Reply::PinSaved(task, result) => {
                self.chat_tools.pin_writes.remove(&task);
                match result {
                    Ok(pins) => {
                        self.chat_tools.pins.insert(task, pins);
                    }
                    Err(error) => self.error = Some(format!("Message pin was not saved: {error}")),
                }
                if self.selected == Some(task) {
                    self.transcript
                        .list
                        .remeasure_items(0..self.transcript.list.item_count());
                }
            }
            Reply::Search {
                task,
                epoch,
                jump,
                result,
            } => {
                if self.selected != Some(task)
                    || self.chat_tools.epoch.load(Ordering::SeqCst) != epoch
                    || !self.chat_tools.find_open
                {
                    return;
                }
                self.chat_tools.searching = false;
                match result {
                    Ok(search) => {
                        let current = self
                            .chat_tools
                            .focused
                            .clone()
                            .filter(|anchor| search.hits.contains(anchor));
                        tracing::debug!(target: "synara_ui_layout", hits = search.hits.len(), limited = search.limited, "message-search-complete");
                        self.chat_tools.search = search;
                        if jump {
                            if let Some(anchor) = self.chat_tools.search.hits.first().cloned() {
                                self.jump_to_message(anchor, cx);
                            }
                        } else {
                            self.chat_tools.focused = current;
                        }
                    }
                    Err(error) => self.chat_tools.search_error = Some(error),
                }
            }
            Reply::Copied(result) => {
                self.chat_tools.copying = false;
                match result {
                    Ok(text) => {
                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
                        self.notice = Some("Text conversation copied. It may contain private conversation content.".into());
                    }
                    Err(error) => {
                        self.error = Some(format!("Conversation was not copied: {error}"))
                    }
                }
            }
            Reply::Exported(result) => {
                self.chat_tools.exporting = false;
                match result {
                    Ok(()) => {
                        self.notice =
                            Some("Text conversation exported to the file you selected.".into())
                    }
                    Err(error) => {
                        self.error = Some(format!(
                            "Conversation export failed: {error}. Existing files are never overwritten. Choose a new filename."
                        ))
                    }
                }
            }
            Reply::Ignored => return,
        }
        cx.notify();
    }
    pub(super) fn refresh_message_search(&mut self, cx: &mut Context<Self>) {
        if self.chat_tools.find_open
            && !self.chat_tools.searching
            && self.chat_tools.search_error.is_none()
            && self
                .thread
                .as_ref()
                .is_some_and(|thread| thread.last_sequence > self.chat_tools.search.sequence)
            && self.selected.is_some_and(|task| !self.busy.contains(&task))
            && !self.chat_tools.query.read(cx).text().trim().is_empty()
        {
            self.queue_message_search(false, cx);
        }
    }
    fn queue_message_search(&mut self, jump: bool, cx: &mut Context<Self>) {
        let epoch = self.chat_tools.epoch.fetch_add(1, Ordering::SeqCst) + 1;
        self.chat_tools.search_error = None;
        if jump {
            self.chat_tools.search = MessageSearch::default();
            self.chat_tools.focused = None;
        }
        let query = self.chat_tools.query.read(cx).text().to_owned();
        self.chat_tools.searching = false;
        let Some(task) = self
            .selected
            .filter(|_| self.chat_tools.find_open && !query.trim().is_empty())
        else {
            cx.notify();
            return;
        };
        self.chat_tools.searching = true;
        let lease = self.chat_tools.epoch.clone();
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            tokio::time::sleep(std::time::Duration::from_millis(180)).await;
            if lease.load(Ordering::SeqCst) != epoch {
                return Ok(Update::ChatTools(Box::new(Reply::Ignored)));
            }
            let result = workspace
                .find_messages(task, query)
                .await
                .map_err(|error| error.to_string());
            Ok(Update::ChatTools(Box::new(Reply::Search {
                task,
                epoch,
                jump,
                result,
            })))
        });
        cx.notify();
    }
    fn jump_to_message(&mut self, anchor: MessageAnchor, cx: &mut Context<Self>) {
        let Some(thread) = &self.thread else {
            return;
        };
        let Some(timeline) = thread.timeline.iter().position(|item| matches!(item, TranscriptItem::Message { index } if thread.messages.get(*index).is_some_and(|message| anchor.matches(message)))) else {
            self.notice = Some("This saved message is not available in the current transcript.".into());
            cx.notify(); return;
        };
        if let Some(turn) = super::activity::turn_for_row(thread, timeline) {
            let summary = &thread.turns[turn];
            let answer = super::activity::answer_index(thread, summary);
            let item = (summary.first_timeline_index..summary.end_timeline_index)
                .filter(|row| match &thread.timeline[*row] {
                    TranscriptItem::Tool { id } => thread.tools.contains_key(id),
                    TranscriptItem::Message { index } => {
                        thread.messages[*index].role != Role::User && Some(*row) != answer
                    }
                    _ => false,
                })
                .position(|row| row == timeline);
            if let Some(item) = item {
                self.chat_tools.work_scroll = gpui::ScrollHandle::new();
                self.chat_tools.work_scroll.scroll_to_top_of_item(item);
            }
            let id = summary.id.clone();
            self.expanded_activity.insert((thread.id, id.clone()));
            self.transcript.invalidate_activity(&id);
        }
        tracing::debug!(target: "synara_ui_layout", timeline, "message-jump");
        self.chat_tools.focused = Some(anchor.clone());
        self.transcript.jump_to_message(thread, &anchor);
        self.transcript
            .list
            .remeasure_items(0..self.transcript.list.item_count());
        cx.notify();
    }
    fn step_message_search(&mut self, backwards: bool, cx: &mut Context<Self>) {
        if let Some(anchor) = next_hit(
            &self.chat_tools.search.hits,
            self.chat_tools.focused.as_ref(),
            backwards,
        ) {
            self.jump_to_message(anchor, cx);
        }
    }
    pub(super) fn open_message_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.thread.is_none() {
            return;
        }
        self.chat_tools.find_open = true;
        self.focus_composer = false;
        self.chat_tools.retire();
        window.focus(&self.chat_tools.query.read(cx).focus_handle(cx), cx);
        self.queue_message_search(true, cx);
    }
    fn close_message_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.chat_tools.find_open = false;
        self.chat_tools.focused = None;
        self.chat_tools.epoch.fetch_add(1, Ordering::SeqCst);
        window.focus(&self.composer.read(cx).focus_handle(cx), cx);
        self.transcript
            .list
            .remeasure_items(0..self.transcript.list.item_count());
        cx.notify();
    }
    fn insert_draft_text(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected.is_none() || self.loading_task.is_some() || self.close != CloseState::Open
        {
            return;
        }
        match append_draft(self.composer.read(cx).text(), text) {
            Ok(text) => {
                self.composer
                    .update(cx, |entry, cx| entry.set_text(text, cx));
                self.remember_draft(cx);
                self.focus_composer = true;
                window.focus(&self.composer.read(cx).focus_handle(cx), cx);
                self.notice = Some(
                    "Added to the draft without sending. Existing draft text was preserved.".into(),
                );
            }
            Err(error) => self.error = Some(error.into()),
        }
        cx.notify();
    }
    fn copy_conversation(&mut self, _cx: &mut Context<Self>) {
        let Some(task) = self.selected else {
            return;
        };
        if self.chat_tools.copying {
            return;
        }
        self.chat_tools.copying = true;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::ChatTools(Box::new(Reply::Copied(
                workspace
                    .text_conversation(task)
                    .await
                    .map_err(|error| error.to_string()),
            ))))
        });
    }
    fn export_conversation(&mut self, cx: &mut Context<Self>) {
        let Some(task) = self.selected else {
            return;
        };
        if self.chat_tools.exporting {
            return;
        }
        self.chat_tools.exporting = true;
        let name = format!("synara-{task}.md");
        // The system dialog makes the destination explicit. The storage worker
        // writes a private, complete new file and refuses an existing destination.
        let picker = cx.prompt_for_new_path(&self.scratch_directory, Some(&name));
        cx.spawn(async move |view, cx| {
            let result = picker.await;
            let _ = view.update(cx, |this, cx| match result {
                Ok(Ok(Some(path))) => {
                    let workspace = this.controller.workspace.clone();
                    this.job(async move { Ok(Update::ChatTools(Box::new(Reply::Exported(workspace.export_text_conversation(task, path).await.map_err(|error| error.to_string()))))) });
                }
                Ok(Ok(None)) => { this.chat_tools.exporting = false; cx.notify(); },
                _ => { this.chat_tools.exporting = false; this.error = Some("The system save dialog is unavailable. Copy the text conversation instead.".into()); cx.notify(); },
            });
        }).detach();
    }
    fn open_chat_menu(
        &mut self,
        title: &str,
        rows: Vec<(Choice, Action)>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.controls.retire();
        self.environment.retire_popup();
        self.settings.popup = None;
        self.navigation.menu_open = false;
        self.focus_composer = false;
        let selected = self.selected;
        let (choices, actions): (Vec<_>, Vec<_>) = rows.into_iter().unzip();
        let view = cx.new(|cx| ChoiceMenu::new(title.into(), choices, cx));
        let subscription = cx.subscribe(&view, move |this, _, event, cx| {
            let action = match event {
                ChoiceEvent::Selected(index) => actions.get(*index).cloned(),
                _ => None,
            };
            // Window-dependent actions are queued once, then consumed in render.
            this.chat_tools.pending_action = Some((selected, action));
            cx.notify();
        });
        let previous_focus = window.focused(cx);
        window.focus(&view.read(cx).focus_handle(cx), cx);
        self.chat_tools.popup = Some(Popup {
            view,
            previous_focus,
            position: gpui::point(
                window.viewport_size().width - px(16.),
                px(ui::CHROME_HEIGHT + 38.),
            ),
            _subscription: subscription,
        });
        cx.notify();
    }
    pub(super) fn consume_chat_action(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((task, action)) = self.chat_tools.pending_action.take() else {
            return;
        };
        if let Some(popup) = self.chat_tools.popup.take()
            && let Some(focus) = popup.previous_focus
        {
            window.focus(&focus, cx);
        }
        let Some(action) = action else {
            return;
        };
        if !matches!(action, Action::SelectTask(_)) && self.selected != task {
            return;
        }
        match action {
            Action::Find => self.open_message_search(window, cx),
            Action::Pins => self.open_pinned_messages(window, cx),
            Action::Copy => self.copy_conversation(cx),
            Action::Export => self.export_conversation(cx),
            Action::ReuseLast => {
                if let Some(text) = self
                    .thread
                    .as_ref()
                    .and_then(|thread| {
                        thread
                            .messages
                            .iter()
                            .rev()
                            .find(|message| message.role == Role::User)
                    })
                    .map(|message| message.text.clone())
                {
                    self.insert_draft_text(&text, window, cx);
                }
            }
            Action::Commands => self.open_agent_commands(window, cx),
            Action::Jump(anchor) => self.jump_to_message(anchor, cx),
            Action::RemovePin(anchor) => self.set_pin(anchor, false, cx),
            Action::Insert(text) => self.insert_draft_text(&text, window, cx),
            Action::SelectTask(task) => {
                if self.select_task(task, cx) {
                    self.show_conversation(cx);
                }
            }
        }
    }
    fn open_pinned_messages(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(task) = self.selected else {
            return;
        };
        let mut rows = vec![];
        if let Some(pins) = self.chat_tools.pins.get(&task) {
            for pin in pins {
                let message = self
                    .thread
                    .as_ref()
                    .and_then(|thread| thread.messages.iter().find(|message| pin.matches(message)));
                rows.push(match message {
                    Some(message) => (
                        Choice {
                            label: snippet(&message.text),
                            detail: role_label(message.role).into(),
                            icon: Some(Glyph::Pin),
                            ..Default::default()
                        },
                        Action::Jump(pin.clone()),
                    ),
                    None => (
                        Choice {
                            label: "Remove unavailable message pin".into(),
                            detail: format!(
                                "{} message is no longer in this transcript",
                                role_label(pin.role)
                            ),
                            icon: Some(Glyph::Close),
                            ..Default::default()
                        },
                        Action::RemovePin(pin.clone()),
                    ),
                });
            }
        } else {
            self.load_message_pins(task);
            let reason = self
                .chat_tools
                .pin_errors
                .get(&task)
                .map_or(
                    "Loading saved pins. Reopen this menu when loading finishes.",
                    |error| error.as_str(),
                )
                .to_owned();
            rows.push((
                Choice {
                    label: "Saved pins unavailable".into(),
                    unavailable: Some(reason),
                    ..Default::default()
                },
                Action::Pins,
            ));
        }
        if rows.is_empty() {
            rows.push((Choice { label: "No pinned messages".into(), unavailable: Some("Pin a message using its pin button. Pins are stored separately from the transcript.".into()), ..Default::default() }, Action::Pins));
        }
        self.open_chat_menu("Pinned messages", rows, window, cx);
    }
    fn open_agent_commands(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut rows = self
            .thread
            .as_ref()
            .map(|thread| {
                thread
                    .commands
                    .iter()
                    .filter_map(|command| {
                        let text = command_text(command)?;
                        Some((
                            Choice {
                                label: text.trim().into(),
                                detail: format!(
                                    "{} {}",
                                    command.description,
                                    command.argument_hint.as_deref().unwrap_or("")
                                ),
                                icon: Some(Glyph::Shortcut),
                                ..Default::default()
                            },
                            Action::Insert(text),
                        ))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if rows.is_empty() {
            rows.push((Choice { label: "No advertised commands".into(), unavailable: Some("Connect an agent that advertises slash commands. Synara does not invent unsupported commands.".into()), ..Default::default() }, Action::Commands));
        }
        self.open_chat_menu("Agent commands - insert into draft", rows, window, cx);
    }
    pub(super) fn open_thread_finder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut tasks = self
            .catalog
            .tasks
            .iter()
            .filter(|task| {
                task.state != TaskState::Archived
                    && (task.scope != TaskScope::Studio || self.settings.value.general.show_studio)
            })
            .collect::<Vec<_>>();
        tasks.sort_by(|a, b| {
            b.updated_at_ms
                .cmp(&a.updated_at_ms)
                .then_with(|| a.id.cmp(&b.id))
        });
        let rows = tasks
            .into_iter()
            .map(|task| {
                let project = self
                    .catalog
                    .projects
                    .iter()
                    .find(|project| project.id == task.project_id)
                    .map_or("Missing project", |project| project.name.as_str());
                let agent = self
                    .profiles
                    .iter()
                    .find(|profile| profile.id == task.agent_id)
                    .map_or(task.agent_id.as_str(), |profile| profile.name.as_str());
                (
                    Choice {
                        label: task.title.clone(),
                        detail: format!(
                            "{project} · {agent} · {:?} · {:?}",
                            task.scope, task.state
                        ),
                        selected: self.selected == Some(task.id),
                        icon: Some(self.agent_glyph(&task.agent_id)),
                        ..Default::default()
                    },
                    Action::SelectTask(task.id),
                )
            })
            .collect();
        self.open_chat_menu("All threads - project, provider or title", rows, window, cx);
    }
    fn open_conversation_actions(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let rows = [
            (
                "Find in conversation",
                "Search saved message text. Ctrl/Command+F",
                Glyph::Search,
                Action::Find,
            ),
            (
                "Pinned messages",
                "Jump to a saved message",
                Glyph::Pin,
                Action::Pins,
            ),
            (
                "Copy text conversation",
                "Copies user, assistant and reasoning text as Markdown",
                Glyph::Copy,
                Action::Copy,
            ),
            (
                "Export text conversation",
                "Save private conversation text to a new local Markdown file",
                Glyph::Files,
                Action::Export,
            ),
            (
                "Reuse last prompt",
                "Append to the current draft without sending",
                Glyph::Compose,
                Action::ReuseLast,
            ),
            (
                "Agent commands",
                "Search advertised commands and insert one into the draft",
                Glyph::Shortcut,
                Action::Commands,
            ),
        ]
        .into_iter()
        .map(|(label, detail, icon, action)| {
            (
                Choice {
                    label: label.into(),
                    detail: detail.into(),
                    icon: Some(icon),
                    ..Default::default()
                },
                action,
            )
        })
        .collect();
        self.open_chat_menu("Conversation actions", rows, window, cx);
    }
    pub(super) fn chat_tools_bar(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let count = self
            .selected
            .and_then(|task| self.chat_tools.pins.get(&task))
            .map_or(0, Vec::len);
        div()
            .w_full()
            .max_w(px(ui::CHAT_WIDTH + 48.))
            .mx_auto()
            .px_5()
            .py_1()
            .flex()
            .items_center()
            .justify_end()
            .gap_1()
            .child(self.saved_context_button(cx))
            .when(
                self.task()
                    .is_some_and(|task| task.scope == TaskScope::Studio),
                |el| el.child(self.studio_outputs_button(cx)),
            )
            .child(
                ui::chrome_button(
                    "chat-find",
                    "Find in conversation",
                    Glyph::Search,
                    false,
                    cx.listener(|this, _: &(), window, cx| this.open_message_search(window, cx)),
                )
                .size(px(26.)),
            )
            .child(
                ui::chrome_button(
                    "chat-pins",
                    "Pinned messages",
                    Glyph::Pin,
                    false,
                    cx.listener(|this, _: &(), window, cx| this.open_pinned_messages(window, cx)),
                )
                .size(px(26.)),
            )
            .children((count > 0).then(|| {
                div()
                    .text_xs()
                    .text_color(rgb(palette().muted))
                    .child(count.to_string())
            }))
            .child(
                ui::chrome_button(
                    "chat-commands",
                    "Agent commands",
                    Glyph::Shortcut,
                    false,
                    cx.listener(|this, _: &(), window, cx| this.open_agent_commands(window, cx)),
                )
                .size(px(26.)),
            )
            .child(
                ui::chrome_button(
                    "chat-actions",
                    "Conversation actions",
                    Glyph::More,
                    false,
                    cx.listener(|this, _: &(), window, cx| {
                        this.open_conversation_actions(window, cx)
                    }),
                )
                .size(px(26.)),
            )
            .into_any_element()
    }
    pub(super) fn message_find_bar(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let position = self
            .chat_tools
            .focused
            .as_ref()
            .and_then(|anchor| {
                self.chat_tools
                    .search
                    .hits
                    .iter()
                    .position(|hit| hit == anchor)
            })
            .map_or(0, |index| index + 1);
        let status = if self.chat_tools.searching {
            "Searching...".into()
        } else if self.chat_tools.search_error.is_some() {
            "Search failed".into()
        } else {
            format!(
                "{position}/{}{} messages",
                self.chat_tools.search.hits.len(),
                if self.chat_tools.search.limited {
                    "+"
                } else {
                    ""
                }
            )
        };
        let disabled = self.chat_tools.search.hits.is_empty();
        div()
            .id("message-find-bar")
            .relative()
            .child(ui::layout_probe("message-find-bar"))
            .w_full()
            .px_3()
            .py_1()
            .flex()
            .items_center()
            .gap_1()
            .border_b_1()
            .border_color(rgb(palette().border))
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                if event.prefer_character_input
                    || this.chat_tools.query.update(cx, |entry, cx| {
                        entry.marked_text_range(window, cx).is_some()
                    })
                {
                    return;
                }
                if event.keystroke.key == "escape" {
                    this.close_message_search(window, cx);
                    cx.stop_propagation();
                }
            }))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .relative()
                    .child(ui::layout_probe("message-find-input"))
                    .child(self.chat_tools.query.clone()),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(palette().muted))
                    .child(status),
            )
            .child(
                ui::chrome_button(
                    "find-previous",
                    "Previous matching message",
                    Glyph::Back,
                    disabled,
                    cx.listener(|this, _: &(), _, cx| this.step_message_search(true, cx)),
                )
                .size(px(24.)),
            )
            .child(
                ui::chrome_button(
                    "find-next",
                    "Next matching message",
                    Glyph::Forward,
                    disabled,
                    cx.listener(|this, _: &(), _, cx| this.step_message_search(false, cx)),
                )
                .size(px(24.)),
            )
            .child(
                ui::chrome_button(
                    "find-close",
                    "Close conversation search",
                    Glyph::Close,
                    false,
                    cx.listener(|this, _: &(), window, cx| this.close_message_search(window, cx)),
                )
                .size(px(24.)),
            )
            .into_any_element()
    }
    pub(super) fn message_pin_button(
        &self,
        message: &Message,
        index: usize,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let anchor = MessageAnchor::from(message);
        let pinned = self.pinned(&anchor);
        let task = self.selected;
        let disabled = task.is_none_or(|task| {
            !self.chat_tools.pins.contains_key(&task) || self.chat_tools.pin_writes.contains(&task)
        });
        ui::chrome_button(
            "message-pin",
            if pinned {
                "Unpin message"
            } else {
                "Pin message"
            },
            Glyph::Pin,
            disabled,
            cx.listener(move |this, _: &(), _, cx| {
                if this.selected == task {
                    this.set_pin(anchor.clone(), !pinned, cx);
                }
            }),
        )
        .size(px(24.))
        .when(pinned, |el| el.text_color(rgb(palette().focus)))
        .relative()
        .child(ui::layout_probe_slot("message-pin", index))
        .into_any_element()
    }
    pub(super) fn message_reuse_button(
        &self,
        message: &Message,
        index: usize,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let too_large = message.text.len() > 1024 * 1024;
        let text = if too_large {
            String::new()
        } else if message.role == Role::User {
            message.text.clone()
        } else {
            message
                .text
                .lines()
                .map(|line| format!("> {line}"))
                .collect::<Vec<_>>()
                .join("\n")
        };
        let task = self.selected;
        ui::chrome_button(
            "message-reuse",
            if message.role == Role::User {
                "Reuse prompt without sending"
            } else {
                "Quote message in draft"
            },
            Glyph::Compose,
            too_large,
            cx.listener(move |this, _: &(), window, cx| {
                if this.selected == task {
                    this.insert_draft_text(&text, window, cx);
                }
            }),
        )
        .size(px(24.))
        .relative()
        .child(ui::layout_probe_slot("message-reuse", index))
        .into_any_element()
    }
    pub(super) fn chat_tools_shortcut(
        &mut self,
        event: &gpui::KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let modifiers = event.keystroke.modifiers;
        if event.prefer_character_input
            || event.is_held
            || modifiers.alt
            || self.close != CloseState::Open
            || self.chat_tools.menu_open()
            || self.navigation.menu_open
            || self.controls.is_open()
            || self.environment.menu_open()
            || self.settings.popup.is_some()
            || self
                .terminal_view
                .read(cx)
                .focus_handle(cx)
                .is_focused(window)
            || self.editor.read(cx).focus_handle(cx).is_focused(window)
        {
            return false;
        }
        let key = event.keystroke.key.as_str();
        if (modifiers.control || modifiers.platform) && !modifiers.shift && key == "k" {
            self.open_thread_finder(window, cx);
            return true;
        }
        if self.panel != Panel::Conversation && !self.dock_open() {
            return false;
        }
        if (modifiers.control || modifiers.platform) && !modifiers.shift && key == "f" {
            self.open_message_search(window, cx);
            return true;
        }
        if key == "f3" && !modifiers.control && !modifiers.platform && self.chat_tools.find_open {
            self.step_message_search(modifiers.shift, cx);
            return true;
        }
        false
    }
    pub(super) fn chat_tools_overlay(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(popup) = &self.chat_tools.popup else {
            return div().into_any_element();
        };
        div()
            .absolute()
            .inset_0()
            .occlude()
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    if let Some(popup) = this.chat_tools.popup.take()
                        && let Some(focus) = popup.previous_focus
                    {
                        window.focus(&focus, cx);
                    }
                    cx.notify();
                    cx.stop_propagation();
                }),
            )
            .child(
                gpui::anchored()
                    .anchor(gpui::Anchor::TopRight)
                    .position(popup.position)
                    .snap_to_window_with_margin(px(8.))
                    .child(popup.view.clone()),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prompt_reuse_preserves_unsent_text_and_unicode_without_auto_send() {
        assert_eq!(
            append_draft("Keep this", "日本語\nnext").unwrap(),
            "Keep this\n\n日本語\nnext"
        );
        assert_eq!(append_draft("", "  exact\n").unwrap(), "  exact\n");
        assert!(append_draft(&"x".repeat(1024 * 1024), "more").is_err());
    }
    #[test]
    fn command_insertion_is_bounded_and_rejects_controls_or_extra_arguments() {
        let command = |name: &str| SlashCommand {
            name: name.into(),
            description: "display only".into(),
            argument_hint: Some("[prompt]".into()),
        };
        assert_eq!(command_text(&command("review")), Some("/review ".into()));
        assert_eq!(
            command_text(&command("/project:check")),
            Some("/project:check ".into())
        );
        for name in [
            "",
            "/",
            "review\nrun",
            "review --all",
            "../escape",
            "review;run",
        ] {
            assert_eq!(command_text(&command(name)), None);
        }
    }
    #[test]
    fn message_navigation_wraps_without_conflating_shared_ids_or_empty_results() {
        let user = MessageAnchor {
            id: "same".into(),
            role: Role::User,
        };
        let assistant = MessageAnchor {
            id: "same".into(),
            role: Role::Assistant,
        };
        let hits = [user.clone(), assistant.clone()];
        assert_eq!(next_hit(&hits, None, false), Some(user.clone()));
        assert_eq!(next_hit(&hits, Some(&user), false), Some(assistant.clone()));
        assert_eq!(next_hit(&hits, Some(&user), true), Some(assistant.clone()));
        assert_eq!(next_hit(&hits, Some(&assistant), false), Some(user));
        assert_eq!(next_hit(&[], None, false), None);
    }
    #[test]
    fn pin_previews_are_bounded_without_splitting_characters() {
        let preview = snippet(&"日本語".repeat(300));
        assert_eq!(preview.chars().count(), 120);
        assert!(!preview.contains('\u{fffd}'));
        assert_eq!(snippet(" hello\n  world\t "), "hello world");
    }
}
