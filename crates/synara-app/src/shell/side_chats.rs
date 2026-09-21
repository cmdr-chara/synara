//! Side chats keep an independent task/session while sharing only the source task's
//! project and working-directory authority. They never inherit approvals or provider sessions.
use super::*;
use crate::ui::{self, Glyph, palette};

pub(super) enum Reply {
    Index {
        parent: TaskId,
        generation: u64,
        result: Result<SideThreadIndex, String>,
    },
    Thread {
        parent: TaskId,
        child: TaskId,
        generation: u64,
        result: Result<(Task, Thread, String), String>,
    },
    Created {
        parent: TaskId,
        generation: u64,
        result: Result<(Task, Thread, String, Catalog), String>,
    },
    AgentChanged {
        child: TaskId,
        result: Result<Task, String>,
    },
}

pub(super) struct SideChatState {
    pub parent: Option<TaskId>,
    pub threads: Vec<Task>,
    pub selected: Option<TaskId>,
    pub thread: Option<Thread>,
    pub composer: Entity<TextEntry>,
    pub error: Option<String>,
    generation: u64,
    loading: bool,
    creating: bool,
    selecting: bool,
    _subscription: Subscription,
}

impl SideChatState {
    pub fn new(cx: &mut Context<Shell>) -> Self {
        let composer = cx.new(|cx| {
            TextEntry::new(
                "Ask in this side chat",
                EntryMode::Composer,
                crate::ui::COMPOSER_INPUT_HEIGHT,
                cx,
            )
        });
        let subscription = cx.subscribe(&composer, |this, _, event, cx| match event {
            EntryEvent::Submit => this.send_side_prompt(cx),
            EntryEvent::Changed => this.remember_side_draft(cx),
            EntryEvent::AttachmentPaste(_) | EntryEvent::AttachmentFiles(_) => {
                this.side_chats.error = Some(
                    "Side-chat attachments are not wired yet. Open this side chat as the main conversation to attach files.".into(),
                );
                cx.notify();
            }
            _ => {}
        });
        Self {
            parent: None,
            threads: Vec::new(),
            selected: None,
            thread: None,
            composer,
            error: None,
            generation: 0,
            loading: false,
            creating: false,
            selecting: false,
            _subscription: subscription,
        }
    }

    pub fn pending(&self, cx: &App) -> bool {
        self.creating || self.selecting || self.composer.read(cx).is_composing()
    }

    fn reset_for(&mut self, parent: TaskId) {
        self.parent = Some(parent);
        self.threads.clear();
        self.selected = None;
        self.thread = None;
        self.error = None;
        self.loading = true;
        self.selecting = false;
        self.generation = self.generation.wrapping_add(1);
    }
}

impl Shell {
    pub(super) fn load_side_chats(&mut self, parent: TaskId, cx: &mut Context<Self>) {
        if self.side_chats.parent != Some(parent) {
            self.side_chats.reset_for(parent);
        } else {
            self.side_chats.loading = true;
            self.side_chats.generation = self.side_chats.generation.wrapping_add(1);
        }
        let generation = self.side_chats.generation;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            let result = workspace
                .side_threads(parent)
                .await
                .map_err(|error| error.to_string());
            Ok(Update::SideChats(Box::new(Reply::Index {
                parent,
                generation,
                result,
            })))
        });
        cx.notify();
    }

    fn load_side_thread(
        &mut self,
        parent: TaskId,
        child: TaskId,
        generation: u64,
        persist_selection: bool,
        cx: &mut Context<Self>,
    ) {
        self.side_chats.loading = true;
        self.side_chats.selecting = persist_selection;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            let result = async {
                if persist_selection {
                    workspace.select_side_thread(parent, child).await?;
                }
                let task = workspace.task(child).await?;
                let thread = workspace.thread(task.thread_id).await?;
                let draft = workspace.task_draft(child).await?;
                Ok::<_, WorkspaceError>((task, thread, draft))
            }
            .await
            .map_err(|error| error.to_string());
            Ok(Update::SideChats(Box::new(Reply::Thread {
                parent,
                child,
                generation,
                result,
            })))
        });
        cx.notify();
    }

    fn select_side_chat(&mut self, child: TaskId, cx: &mut Context<Self>) {
        let Some(parent) = self.side_chats.parent else { return };
        if self.side_chats.selected == Some(child) || self.side_chats.selecting {
            return;
        }
        if !self.side_chats.threads.iter().any(|task| task.id == child) {
            self.side_chats.error = Some("That side chat is no longer available.".into());
            cx.notify();
            return;
        }
        self.side_chats.generation = self.side_chats.generation.wrapping_add(1);
        let generation = self.side_chats.generation;
        self.load_side_thread(parent, child, generation, true, cx);
    }

    fn switch_side_agent(&mut self, child: TaskId, agent: String, cx: &mut Context<Self>) {
        if self.side_chats.selected != Some(child)
            || self.side_chats.loading
            || self.side_chats.selecting
            || self.busy.contains(&child)
            || self.side_chats.thread.as_ref().is_none_or(|thread| {
                thread.last_sequence != 0 || !thread.timeline.is_empty()
            })
        {
            return;
        }
        if self
            .side_chats
            .threads
            .iter()
            .find(|task| task.id == child)
            .is_some_and(|task| task.agent_id == agent)
        {
            return;
        }
        self.side_chats.selecting = true;
        self.side_chats.error = None;
        let controller = self.controller.clone();
        self.job(async move {
            let result = controller
                .switch_agent(child, agent)
                .await
                .map_err(|error| error.to_string());
            Ok(Update::SideChats(Box::new(Reply::AgentChanged {
                child,
                result,
            })))
        });
        cx.notify();
    }

    pub(super) fn create_side_chat(
        &mut self,
        message: Option<MessageAnchor>,
        cx: &mut Context<Self>,
    ) {
        if self.close != CloseState::Open
            || self.side_chats.creating
            || self.hub_navigation_blocked(cx)
        {
            return;
        }
        let Some(parent_task) = self.task().cloned() else {
            self.error = Some("Select a conversation before creating a side chat.".into());
            cx.notify();
            return;
        };
        if parent_task.state == TaskState::Archived {
            self.error = Some("Restore the conversation before creating a side chat.".into());
            cx.notify();
            return;
        }
        if self.side_chats.parent != Some(parent_task.id) {
            self.side_chats.reset_for(parent_task.id);
        }
        self.side_chats.creating = true;
        self.side_chats.error = None;
        let parent = parent_task.id;
        let agent = parent_task.agent_id;
        let generation = self.side_chats.generation;
        let workspace = self.controller.workspace.clone();
        self.set_panel(Panel::SideChats, cx);
        self.job(async move {
            let result = async {
                let task = workspace.create_side_thread(parent, agent, message).await?;
                let thread = workspace.thread(task.thread_id).await?;
                let draft = workspace.task_draft(task.id).await?;
                let catalog = workspace.catalog().await?;
                Ok::<_, WorkspaceError>((task, thread, draft, catalog))
            }
            .await
            .map_err(|error| error.to_string());
            Ok(Update::SideChats(Box::new(Reply::Created {
                parent,
                generation,
                result,
            })))
        });
        cx.notify();
    }

    pub(super) fn message_side_chat_button(
        &self,
        message: &Message,
        index: usize,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let anchor = MessageAnchor::from(message);
        ui::chrome_button(
            "message-side-chat",
            "Start a side chat from this message",
            Glyph::Chat,
            message.role == Role::Reasoning
                || self.side_chats.creating
                || self.loading_task.is_some(),
            cx.listener(move |this, _: &(), _, cx| {
                let _ = index;
                this.create_side_chat(Some(anchor.clone()), cx)
            }),
        )
        .size(px(24.))
        .into_any_element()
    }

    fn remember_side_draft(&mut self, cx: &mut Context<Self>) {
        let Some(task) = self.side_chats.selected else { return };
        if self.side_chats.loading {
            return;
        }
        let text = self.side_chats.composer.read(cx).text().to_owned();
        self.remember_task_draft(task, text, cx);
    }

    fn send_side_prompt(&mut self, cx: &mut Context<Self>) {
        let Some(task) = self.side_chats.selected else { return };
        if self.close != CloseState::Open
            || self.side_chats.loading
            || self.side_chats.selecting
            || self.side_chats.composer.read(cx).is_composing()
            || self.busy.contains(&task)
            || self.connecting.contains(&task)
        {
            return;
        }
        let text = self.side_chats.composer.read(cx).text().to_owned();
        if text.trim().is_empty() {
            return;
        }
        self.remember_task_draft(task, text.clone(), cx);
        self.draft_state.submitted(task, text.clone());
        self.busy.insert(task);
        self.side_chats.error = None;
        let controller = self.controller.clone();
        self.job(async move {
            let result = controller.submit(task, text).await;
            let details = controller.details(task).await.ok().flatten();
            Ok(Update::PromptDone {
                task,
                details,
                error: result.err().map(|error| error.to_string()),
            })
        });
        cx.notify();
    }

    fn cancel_side_prompt(&mut self, cx: &mut Context<Self>) {
        let Some(task) = self.side_chats.selected.filter(|task| self.busy.contains(task)) else {
            return;
        };
        let controller = self.controller.clone();
        self.job(async move {
            controller.cancel(task).await?;
            Ok(Update::Done("Side-chat cancellation requested".into()))
        });
        cx.notify();
    }

    pub(super) fn reload_visible_side_chat(&mut self, task: TaskId, cx: &mut Context<Self>) {
        if self.side_chats.selected != Some(task) {
            return;
        }
        let Some(parent) = self.side_chats.parent else { return };
        let generation = self.side_chats.generation;
        self.load_side_thread(parent, task, generation, false, cx);
    }

    pub(super) fn side_chat_event(&mut self, envelope: &EventEnvelope, cx: &mut Context<Self>) {
        let Some(thread) = self.side_chats.thread.as_mut() else { return };
        if thread.id != envelope.thread_id {
            return;
        }
        match thread.apply(envelope) {
            Ok(_) => {}
            Err(ReplayError::Sequence { .. }) => {
                if let Some(task) = self.side_chats.selected {
                    self.reload_visible_side_chat(task, cx);
                }
                return;
            }
            Err(error) => {
                self.side_chats.error = Some(format!("Side-chat update failed: {error}"));
                cx.notify();
                return;
            }
        }
        let state = thread.state;
        let title = thread.title.clone();
        if let Some(task) = self
            .side_chats
            .threads
            .iter_mut()
            .find(|task| Some(task.id) == self.side_chats.selected)
        {
            task.state = state;
            if !title.is_empty() && title != "New task" {
                task.title = title;
            }
            task.updated_at_ms = envelope.timestamp_ms;
        }
        cx.notify();
    }

    pub(super) fn side_chat_reply(&mut self, reply: Reply, cx: &mut Context<Self>) {
        match reply {
            Reply::Index {
                parent,
                generation,
                result,
            } => {
                if self.side_chats.parent != Some(parent)
                    || self.side_chats.generation != generation
                {
                    return;
                }
                self.side_chats.loading = false;
                match result {
                    Ok(index) => {
                        self.side_chats.threads = index.threads;
                        self.side_chats.error = None;
                        let selected = index
                            .selected
                            .or_else(|| self.side_chats.threads.first().map(|task| task.id));
                        self.side_chats.selected = selected;
                        if let Some(child) = selected {
                            self.load_side_thread(parent, child, generation, false, cx);
                        } else {
                            self.side_chats.thread = None;
                            self.side_chats.composer.update(cx, |entry, cx| entry.clear(cx));
                        }
                    }
                    Err(error) => {
                        self.side_chats.error = Some(error);
                        self.side_chats.threads.clear();
                        self.side_chats.selected = None;
                        self.side_chats.thread = None;
                    }
                }
            }
            Reply::Thread {
                parent,
                child,
                generation,
                result,
            } => {
                if self.side_chats.parent != Some(parent)
                    || self.side_chats.generation != generation
                {
                    return;
                }
                self.side_chats.loading = false;
                self.side_chats.selecting = false;
                match result {
                    Ok((task, thread, stored_draft)) => {
                        if let Some(row) = self
                            .side_chats
                            .threads
                            .iter_mut()
                            .find(|candidate| candidate.id == child)
                        {
                            *row = task.clone();
                        } else {
                            self.side_chats.threads.insert(0, task.clone());
                        }
                        self.replace_task(task);
                        self.side_chats.selected = Some(child);
                        self.side_chats.thread = Some(thread);
                        let draft = self
                            .drafts
                            .get(&child)
                            .cloned()
                            .unwrap_or_else(|| stored_draft.clone());
                        self.drafts.entry(child).or_insert(stored_draft);
                        self.side_chats
                            .composer
                            .update(cx, |entry, cx| entry.set_text(draft, cx));
                        self.side_chats.error = None;
                    }
                    Err(error) => {
                        self.side_chats.error = Some(error);
                    }
                }
            }
            Reply::Created {
                parent,
                generation,
                result,
            } => {
                self.side_chats.creating = false;
                match result {
                    Ok((task, thread, stored_draft, catalog)) => {
                        self.catalog = catalog;
                        if self.side_chats.parent != Some(parent)
                            || self.side_chats.generation != generation
                        {
                            self.notice = Some(
                                "Side chat created. Open Side chats from its source conversation when ready.".into(),
                            );
                            cx.notify();
                            return;
                        }
                        let id = task.id;
                        self.side_chats
                            .threads
                            .retain(|candidate| candidate.id != id);
                        self.side_chats.threads.insert(0, task);
                        self.side_chats.selected = Some(id);
                        self.side_chats.thread = Some(thread);
                        self.drafts.insert(id, stored_draft.clone());
                        self.side_chats
                            .composer
                            .update(cx, |entry, cx| entry.set_text(stored_draft, cx));
                        self.side_chats.loading = false;
                        self.side_chats.error = None;
                        self.notice = Some(
                            "Side chat created without starting an agent. Review its draft, then Send.".into(),
                        );
                    }
                    Err(error) => self.side_chats.error = Some(error),
                }
            }
            Reply::AgentChanged { child, result } => {
                self.side_chats.selecting = false;
                match result {
                    Ok(task) => {
                        if let Some(row) = self
                            .side_chats
                            .threads
                            .iter_mut()
                            .find(|candidate| candidate.id == child)
                        {
                            *row = task.clone();
                        }
                        self.replace_task(task);
                        self.side_chats.error = None;
                        self.notice = Some(
                            "Side-chat agent changed before its first turn. No session was started.".into(),
                        );
                    }
                    Err(error) => self.side_chats.error = Some(error),
                }
            }
        }
        cx.notify();
    }

    fn side_chat_item(
        &self,
        thread: &Thread,
        index: usize,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        match &thread.timeline[index] {
            TranscriptItem::Message { index: message_index } => {
                let message = &thread.messages[*message_index];
                let user = message.role == Role::User;
                let reasoning = message.role == Role::Reasoning;
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .when(user, |row| row.items_end())
                    .child(
                        div()
                            .max_w(px(620.))
                            .when(user, |body| {
                                body.px_3()
                                    .py_2()
                                    .rounded_lg()
                                    .bg(ui::surface(palette().overlay))
                            })
                            .text_size(px(13.))
                            .line_height(px(20.))
                            .text_color(rgb(if reasoning {
                                palette().muted
                            } else {
                                palette().text
                            }))
                            .child(if user || reasoning {
                                div()
                                    .child(truncate(&message.text, 64 * 1024))
                                    .into_any_element()
                            } else {
                                ui::markdown::render(
                                    &truncate(&message.text, 64 * 1024),
                                    &format!("side-{}-{}", thread.id, message.id),
                                )
                            }),
                    )
                    .into_any_element()
            }
            TranscriptItem::Tool { id } => {
                let Some(tool) = thread.tools.get(id) else {
                    return div().into_any_element();
                };
                div()
                    .w_full()
                    .py_2()
                    .border_b_1()
                    .border_color(rgb(palette().border))
                    .text_size(px(12.))
                    .child(format!("{} · {:?}", tool.title, tool.status))
                    .children(tool.output.iter().take(12).map(|output| {
                        let text = match output {
                            ToolOutput::Text { text } => truncate(text, 8000),
                            ToolOutput::Diff { path, .. } => format!("Diff · {path}"),
                            ToolOutput::Terminal { id } => format!("Terminal · {id}"),
                            ToolOutput::Resource { uri, name } => format!("{name} · {uri}"),
                        };
                        div()
                            .mt_1()
                            .font_family(ui::code_font())
                            .text_color(rgb(palette().muted))
                            .child(text)
                    }))
                    .into_any_element()
            }
            TranscriptItem::Permission { id } => {
                let key = (thread.id, id.clone());
                let Some(UiInteraction::Permission { request, .. }) = self
                    .pending
                    .get(&key)
                    .filter(|interaction| interaction.is_active())
                else {
                    return div()
                        .text_size(px(12.))
                        .text_color(rgb(palette().muted))
                        .child("Permission request resolved or expired")
                        .into_any_element();
                };
                div()
                    .w_full()
                    .py_3()
                    .border_y_1()
                    .border_color(rgb(palette().focus))
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(request.title.clone()),
                    )
                    .child(
                        div()
                            .mt_2()
                            .flex()
                            .flex_wrap()
                            .gap_2()
                            .children(request.choices.iter().enumerate().map(|(choice_index, choice)| {
                                let key = key.clone();
                                let selected = choice.id.clone();
                                button(
                                    ("side-permission", choice_index),
                                    choice.label.clone(),
                                    false,
                                )
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.answer_permission(
                                        key.clone(),
                                        Some(selected.clone()),
                                        cx,
                                    )
                                }))
                            }))
                            .child(
                                button("side-permission-cancel", "Cancel", false).on_click(
                                    cx.listener(move |this, _, _, cx| {
                                        this.answer_permission(key.clone(), None, cx)
                                    }),
                                ),
                            ),
                    )
                    .into_any_element()
            }
            TranscriptItem::Input { id } => {
                self.input_request((thread.id, id.clone()), cx)
            }
            TranscriptItem::Notice { text, is_error } => div()
                .w_full()
                .py_2()
                .text_size(px(12.))
                .text_color(rgb(if *is_error {
                    palette().error
                } else {
                    palette().muted
                }))
                .child(truncate(text, 16 * 1024))
                .into_any_element(),
        }
    }

    pub(super) fn side_chat_panel(
        &self,
        _width: f32,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let state = &self.side_chats;
        let busy = state.selected.is_some_and(|task| self.busy.contains(&task));
        let can_send = state.selected.is_some()
            && !state.loading
            && !state.selecting
            && !state.composer.read(cx).text().trim().is_empty();
        let mut panel = div()
            .id("side-chats-panel")
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .child(
                div()
                    .h(px(38.))
                    .flex_shrink_0()
                    .px_3()
                    .flex()
                    .items_center()
                    .gap_2()
                    .border_b_1()
                    .border_color(rgb(palette().border))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("Side chats"),
                    )
                    .child(
                        ui::chrome_button(
                            "new-side-chat",
                            "New side chat",
                            Glyph::Plus,
                            state.creating || state.parent.is_none(),
                            cx.listener(|this, _: &(), _, cx| {
                                this.create_side_chat(None, cx)
                            }),
                        )
                        .size(px(26.)),
                    ),
            );

        if let Some(error) = &state.error {
            panel = panel.child(
                div()
                    .px_3()
                    .py_2()
                    .text_size(px(12.))
                    .text_color(rgb(palette().error))
                    .child(error.clone()),
            );
        }

        panel = panel.child(
            div()
                .id("side-chat-tabs")
                .flex_shrink_0()
                .max_h(px(112.))
                .overflow_y_scroll()
                .border_b_1()
                .border_color(rgb(palette().border))
                .children(state.threads.iter().enumerate().map(|(index, task)| {
                    let id = task.id;
                    ui::action(
                        ("side-chat-row", index),
                        task.title.clone(),
                        Some(self.agent_glyph(&task.agent_id)),
                        state.selected == Some(id),
                        cx.listener(move |this, _: &(), _, cx| {
                            this.select_side_chat(id, cx)
                        }),
                    )
                    .h(px(30.))
                    .text_size(px(12.))
                }))
                .children(
                    (state.threads.is_empty() && !state.loading).then(|| {
                        div()
                            .px_3()
                            .py_4()
                            .text_size(px(12.))
                            .text_color(rgb(palette().muted))
                            .child(
                                "No side chats yet. Create one here or branch from a message.",
                            )
                    }),
                ),
        );

        if let (Some(task), Some(thread)) = (state.selected, state.thread.as_ref()) {
            let start = thread.timeline.len().saturating_sub(200);
            panel = panel
                .child(
                    div()
                        .h(px(34.))
                        .flex_shrink_0()
                        .px_3()
                        .flex()
                        .items_center()
                        .gap_2()
                        .border_b_1()
                        .border_color(rgb(palette().border))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_size(px(12.))
                                .text_color(rgb(palette().muted))
                                .child(format!("{:?}", thread.state)),
                        )
                        .child(ui::action(
                            "open-side-chat-main",
                            "Open full conversation",
                            None,
                            false,
                            cx.listener(move |this, _: &(), _, cx| {
                                if this.select_task(task, cx) {
                                    this.show_conversation(cx);
                                }
                            }),
                        )),
                )

                .children(
                    (thread.last_sequence == 0 && thread.timeline.is_empty()).then(|| {
                        let current = state
                            .threads
                            .iter()
                            .find(|candidate| candidate.id == task)
                            .map(|candidate| candidate.agent_id.as_str());
                        div()
                            .id("side-chat-agent-picker")
                            .flex_shrink_0()
                            .px_3()
                            .py_2()
                            .flex()
                            .items_center()
                            .gap_1()
                            .overflow_x_scroll()
                            .child(
                                div()
                                    .mr_1()
                                    .text_size(px(11.))
                                    .text_color(rgb(palette().muted))
                                    .child("Agent before first turn"),
                            )
                            .children(self.profiles.iter().take(16).enumerate().map(|(index, profile)| {
                                let child = task;
                                let agent = profile.id.clone();
                                ui::action(
                                    ("side-agent", index),
                                    profile.name.clone(),
                                    Some(self.agent_glyph(&profile.id)),
                                    current == Some(profile.id.as_str()),
                                    cx.listener(move |this, _: &(), _, cx| {
                                        this.switch_side_agent(child, agent.clone(), cx)
                                    }),
                                )
                                .h(px(26.))
                                .text_size(px(11.))
                            }))
                            .children((self.profiles.len() > 16).then(|| {
                                div()
                                    .text_size(px(11.))
                                    .text_color(rgb(palette().muted))
                                    .child("Open full conversation for more agents")
                            }))
                    }),
                )
                .children((start > 0).then(|| {
                    div()
                        .px_3()
                        .py_1()
                        .text_size(px(11.))
                        .text_color(rgb(palette().muted))
                        .child("Showing the latest 200 timeline items. Open the full conversation for complete history.")
                }))
                .child(
                    div()
                        .id("side-chat-transcript")
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scroll()
                        .px_3()
                        .py_3()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .children(
                            (start..thread.timeline.len())
                                .map(|index| self.side_chat_item(thread, index, cx)),
                        ),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .border_t_1()
                        .border_color(rgb(palette().border))
                        .p_2()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(state.composer.clone())
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap_2()
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(rgb(palette().muted))
                                        .child("Independent session · same workspace authority"),
                                )
                                .child(
                                    ui::icon_button(
                                        "side-chat-send",
                                        if busy { "Stop side chat" } else { "Send side-chat message" },
                                        if busy { Glyph::Stop } else { Glyph::Send },
                                        !busy && !can_send,
                                        cx.listener(move |this, _: &(), _, cx| {
                                            if busy {
                                                this.cancel_side_prompt(cx);
                                            } else {
                                                this.send_side_prompt(cx);
                                            }
                                        }),
                                    ),
                                ),
                        ),
                );
        } else if state.loading {
            panel = panel.child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(12.))
                    .text_color(rgb(palette().muted))
                    .child("Loading side chats..."),
            );
        }

        panel.into_any_element()
    }
}
