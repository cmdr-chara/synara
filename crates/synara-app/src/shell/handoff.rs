//! Explicit reviewed continuation UI. This owns only transient review state;
//! task creation and durable relationships remain with Controller/WorkspaceService.
use super::*;
use crate::ui::{self, Glyph, palette};

#[derive(Clone)]
struct Target {
    label: String,
    choice: HandoffTarget,
}
pub(super) enum Reply {
    Targets(u64, Result<(Vec<AgentProfile>, ProviderSettings), String>),
    Reviewed(u64, Result<Box<HandoffReview>, String>),
    Created(u64, Result<Task, String>),
    Origin(TaskId, u64, Option<ThreadOrigin>),
}
struct Dialog {
    source: TaskId,
    selection_revision: u64,
    targets: Vec<Target>,
    query: Entity<TextEntry>,
    _query_subscription: Subscription,
    editor: Entity<TextEntry>,
    review: Option<HandoffReview>,
    error: Option<String>,
    /// Electron-style quick menu: picking a target creates the unsent
    /// continuation immediately instead of opening the review editor.
    quick: bool,
}
#[derive(Default)]
pub(super) struct HandoffState {
    dialog: Option<Dialog>,
    generation: u64,
    busy: bool,
    creating: bool,
    focus_editor: bool,
    origin: Option<(TaskId, ThreadOrigin)>,
}
impl HandoffState {
    pub fn open(&self) -> bool {
        self.dialog.is_some()
    }
}

impl Shell {
    pub(super) fn open_handoff(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(source) = self.selected else {
            return;
        };
        if self.handoff.open()
            || self.loading_task.is_some()
            || self.busy.contains(&source)
            || self.connecting.contains(&source)
            || self.creating_task
            || self.revisions.open()
            || self.explorer.modal_open()
        {
            return;
        }
        if self
            .task()
            .is_none_or(|task| task.state == TaskState::Archived)
        {
            return;
        }
        let query = cx.new(|cx| {
            TextEntry::new(
                "Find an agent or saved direct model...",
                EntryMode::SingleLine,
                32.,
                cx,
            )
        });
        let editor = cx.new(|cx| {
            TextEntry::new(
                "Review the continuation request",
                EntryMode::Editor,
                220.,
                cx,
            )
        });
        let query_subscription = cx.subscribe(&query, |_, _, event, cx| {
            if matches!(event, EntryEvent::Changed) {
                cx.notify();
            }
        });
        self.handoff.generation = self.handoff.generation.wrapping_add(1);
        let generation = self.handoff.generation;
        self.handoff.dialog = Some(Dialog {
            source,
            selection_revision: self.selection_revision,
            targets: vec![],
            query: query.clone(),
            _query_subscription: query_subscription,
            editor,
            review: None,
            error: None,
            quick: false,
        });
        self.handoff.busy = true;
        self.focus_composer = false;
        self.controls.retire();
        self.chat_tools.retire();
        window.focus(&query.read(cx).focus_handle(cx), cx);
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            let result = async {
                Ok((
                    workspace.profiles().await?,
                    workspace.direct_model_settings().await?,
                ))
            }
            .await;
            Ok(Update::Handoff(Box::new(Reply::Targets(
                generation,
                result.map_err(|e: WorkspaceError| e.to_string()),
            ))))
        });
        cx.notify();
    }
    /// Electron-style entry: header "Hand off" button opening a compact
    /// target menu. Picking a target immediately creates an unsent related
    /// conversation with the generated context; nothing is sent automatically.
    pub(super) fn open_handoff_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(source) = self.selected else {
            return;
        };
        if self.handoff.open()
            || self.loading_task.is_some()
            || self.busy.contains(&source)
            || self.connecting.contains(&source)
            || self.creating_task
            || self.revisions.open()
            || self.explorer.modal_open()
        {
            return;
        }
        if self
            .task()
            .is_none_or(|task| task.state == TaskState::Archived)
        {
            return;
        }
        let query = cx.new(|cx| {
            TextEntry::new(
                "Find an agent or saved direct model...",
                EntryMode::SingleLine,
                32.,
                cx,
            )
        });
        let editor = cx.new(|cx| {
            TextEntry::new(
                "Review the continuation request",
                EntryMode::Editor,
                220.,
                cx,
            )
        });
        let query_subscription = cx.subscribe(&query, |_, _, event, cx| {
            if matches!(event, EntryEvent::Changed) {
                cx.notify();
            }
        });
        self.handoff.generation = self.handoff.generation.wrapping_add(1);
        let generation = self.handoff.generation;
        self.handoff.dialog = Some(Dialog {
            source,
            selection_revision: self.selection_revision,
            targets: vec![],
            query: query.clone(),
            _query_subscription: query_subscription,
            editor,
            review: None,
            error: None,
            quick: true,
        });
        self.handoff.busy = true;
        self.focus_composer = false;
        self.controls.retire();
        self.chat_tools.retire();
        window.focus(&query.read(cx).focus_handle(cx), cx);
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            let result = async {
                Ok((
                    workspace.profiles().await?,
                    workspace.direct_model_settings().await?,
                ))
            }
            .await;
            Ok(Update::Handoff(Box::new(Reply::Targets(
                generation,
                result.map_err(|e: WorkspaceError| e.to_string()),
            ))))
        });
        cx.notify();
    }
    pub(super) fn toggle_handoff_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(dialog) = &self.handoff.dialog {
            if dialog.quick && !self.handoff.busy && !self.handoff.creating {
                self.dismiss_handoff(cx);
            }
            return;
        }
        self.open_handoff_menu(window, cx);
    }
    fn switch_handoff_to_review(&mut self, cx: &mut Context<Self>) {
        if self.handoff.busy || self.handoff.creating {
            return;
        }
        if let Some(dialog) = self.handoff.dialog.as_mut() {
            dialog.quick = false;
        }
        cx.notify();
    }
    fn handoff_short_label(label: &str) -> &str {
        label
            .strip_prefix("Agent: ")
            .or_else(|| label.strip_prefix("Direct: "))
            .unwrap_or(label)
    }
    /// Header "Hand off" button mirroring Electron's chat-header placement.
    /// Hidden on empty conversations like the goal/debug/recap bars.
    pub(super) fn handoff_menu_button(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let empty = self
            .thread
            .as_ref()
            .is_some_and(|t| t.timeline.is_empty() && t.plan.is_empty());
        if empty {
            return div().into_any_element();
        }
        let open = self.handoff.dialog.as_ref().is_some_and(|d| d.quick);
        ui::header_action(
            "handoff-menu",
            "Hand off",
            Some(Glyph::Handoff),
            open,
            cx.listener(|this, _: &(), window, cx| this.toggle_handoff_menu(window, cx)),
        )
        .relative()
        .child(ui::layout_probe("handoff-menu"))
        .into_any_element()
    }
    fn review_handoff_target(&mut self, choice: HandoffTarget, cx: &mut Context<Self>) {
        if self.handoff.busy {
            return;
        }
        let Some(dialog) = &self.handoff.dialog else {
            return;
        };
        if dialog.review.is_some()
            || self.selected != Some(dialog.source)
            || self.selection_revision != dialog.selection_revision
        {
            return;
        }
        let id = dialog.source;
        self.handoff.busy = true;
        let generation = self.handoff.generation;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::Handoff(Box::new(Reply::Reviewed(
                generation,
                workspace
                    .review_handoff(id, choice)
                    .await
                    .map(Box::new)
                    .map_err(|e| e.to_string()),
            ))))
        });
        cx.notify();
    }
    fn confirm_handoff(&mut self, cx: &mut Context<Self>) {
        if self.handoff.busy || self.creating_task {
            return;
        }
        let Some(dialog) = &self.handoff.dialog else {
            return;
        };
        let Some(review) = dialog.review.clone() else {
            return;
        };
        if self.selected != Some(dialog.source)
            || self.selection_revision != dialog.selection_revision
            || dialog.editor.read(cx).is_composing()
        {
            return;
        }
        let draft = dialog.editor.read(cx).text().to_owned();
        self.handoff.busy = true;
        self.handoff.creating = true;
        self.creating_task = true;
        let generation = self.handoff.generation;
        let controller = self.controller.clone();
        self.job(async move {
            Ok(Update::Handoff(Box::new(Reply::Created(
                generation,
                controller
                    .continue_with(review, draft)
                    .await
                    .map_err(|e| e.to_string()),
            ))))
        });
        cx.notify();
    }
    fn dismiss_handoff(&mut self, cx: &mut Context<Self>) {
        if self.handoff.creating {
            return;
        }
        self.handoff.generation = self.handoff.generation.wrapping_add(1);
        self.handoff.dialog = None;
        self.handoff.busy = false;
        self.handoff.focus_editor = false;
        self.focus_composer = true;
        cx.notify();
    }
    pub(super) fn load_handoff_origin(&mut self, id: TaskId) {
        self.handoff.origin = None;
        let revision = self.selection_revision;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            let origin = workspace.thread_origin(id).await.ok().flatten();
            Ok(Update::Handoff(Box::new(Reply::Origin(
                id, revision, origin,
            ))))
        });
    }
    pub(super) fn handoff_reply(&mut self, reply: Reply, cx: &mut Context<Self>) {
        if let Reply::Origin(id, revision, origin) = reply {
            if self.selected == Some(id) && self.selection_revision == revision {
                self.handoff.origin = origin
                    .filter(|o| o.kind == RelatedThreadKind::Handoff)
                    .map(|o| (id, o));
            }
            return;
        }
        let generation = match &reply {
            Reply::Targets(g, _) | Reply::Reviewed(g, _) | Reply::Created(g, _) => *g,
            Reply::Origin(..) => unreachable!(),
        };
        if self.handoff.generation != generation || self.handoff.dialog.is_none() {
            return;
        }
        self.handoff.busy = false;
        match reply {
            Reply::Targets(_, result) => {
                let dialog = self.handoff.dialog.as_mut().unwrap();
                match result {
                    Ok((agents, settings)) => {
                        dialog.targets = agents
                            .into_iter()
                            .map(|agent| Target {
                                label: format!("Agent: {}", agent.name),
                                choice: HandoffTarget::Agent(agent.id),
                            })
                            .collect();
                        // Bound the rendered candidate inventory. The settings
                        // validator already bounds saved providers/model metadata.
                        for profile in settings.providers {
                            for model in profile.models {
                                if dialog.targets.len() >= 4096 {
                                    break;
                                }
                                let limit = model
                                    .capabilities
                                    .max_output_tokens
                                    .unwrap_or(1024)
                                    .min(1024) as u32;
                                dialog.targets.push(Target {
                                    label: format!("Direct: {} / {}", profile.name, model.id),
                                    choice: HandoffTarget::Direct(ModelSelection {
                                        history_turns: None,
                                        provider_id: profile.id.clone(),
                                        model_id: model.id,
                                        max_output_tokens: limit.max(1),
                                        reasoning_effort: None,
                                        output: Default::default(),
                                    }),
                                });
                            }
                        }
                    }
                    Err(error) => dialog.error = Some(error),
                }
            }
            Reply::Reviewed(_, result) => {
                let dialog = self.handoff.dialog.as_mut().unwrap();
                match result {
                    Ok(review) if review.source().id == dialog.source => {
                        dialog.editor.update(cx, |entry, cx| {
                            entry.set_text(review.context().to_owned(), cx)
                        });
                        dialog.review = Some(*review);
                        dialog.error = None;
                        self.handoff.focus_editor = true;
                    }
                    Ok(_) => {
                        dialog.error =
                            Some("The reviewed source no longer matches this conversation.".into())
                    }
                    Err(error) => dialog.error = Some(error),
                }
                // Quick menu: create immediately with the generated context,
                // like Electron's "Handoff to X". The child stays unsent.
                let quick = self
                    .handoff
                    .dialog
                    .as_ref()
                    .is_some_and(|d| d.quick && d.review.is_some());
                if quick {
                    self.confirm_handoff(cx);
                }
            }
            Reply::Created(_, result) => {
                self.handoff.creating = false;
                self.creating_task = false;
                match result {
                    Ok(task) => {
                        let dialog = self.handoff.dialog.take().unwrap();
                        let select = self.selected == Some(dialog.source)
                            && self.selection_revision == dialog.selection_revision;
                        let id = task.id;
                        self.replace_task(task);
                        if select && self.select_task(id, cx) {
                            self.show_conversation(cx);
                        }
                        self.notice = Some("Continuation created as a new unsent conversation. The original session is unchanged. Both conversations use the same working folder.".into());
                    }
                    Err(error) => self.handoff.dialog.as_mut().unwrap().error = Some(error),
                }
            }
            Reply::Origin(..) => {}
        }
        cx.notify();
    }
    pub(super) fn restore_handoff_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.handoff.focus_editor {
            self.handoff.focus_editor = false;
            if let Some(dialog) = &self.handoff.dialog {
                window.focus(&dialog.editor.read(cx).focus_handle(cx), cx);
            }
        }
    }
    pub(super) fn handoff_source_row(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some((id, origin)) = self
            .handoff
            .origin
            .as_ref()
            .filter(|(id, _)| Some(*id) == self.selected)
        else {
            return div().into_any_element();
        };
        let _ = id;
        let source = origin.parent;
        div()
            .px_5()
            .py_1()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .text_size(px(12.))
            .text_color(rgb(palette().muted))
            .child("Related continuation. Shared folder, independent session.")
            .children(
                self.catalog
                    .tasks
                    .iter()
                    .any(|task| task.id == source)
                    .then(|| {
                        ui::action(
                            "handoff-open-source",
                            "Open original",
                            Some(Glyph::Back),
                            false,
                            cx.listener(move |this, _: &(), _, cx| {
                                if this.select_task(source, cx) {
                                    this.show_conversation(cx);
                                }
                            }),
                        )
                        .relative()
                        .child(ui::layout_probe("handoff-open-source"))
                    }),
            )
            .into_any_element()
    }
    pub(super) fn handoff_overlay(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(dialog) = &self.handoff.dialog else {
            return div().into_any_element();
        };
        if dialog.quick {
            return self.handoff_menu(cx);
        }
        let mut page = div().id("handoff-dialog").role(gpui::Role::Dialog).aria_label("Continue with another provider")
            .tab_group().w_full().max_w(px(720.)).max_h(px(650.)).p_4().flex().flex_col().gap_3()
            .bg(ui::surface(palette().overlay)).border_1().border_color(rgb(palette().border))
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape" && !this.handoff.creating {
                    let pristine = this.handoff.dialog.as_ref().is_none_or(|d| !d.editor.read(cx).is_composing() && !d.query.read(cx).is_composing() && d.review.as_ref().is_none_or(|r| r.context() == d.editor.read(cx).text()));
                    if pristine { this.dismiss_handoff(cx); }
                    cx.stop_propagation();
                }
                // Ordinary key events must reach the platform character-input
                // fallback. The shell capture guard already excludes this modal.
            }))
            .child(div().text_size(px(21.)).child("Continue with..."))
            .child(div().text_size(px(13.)).text_color(rgb(palette().muted))
                .child("Create a related conversation, not a transferable provider session. The original stays intact. Files and Git working state are shared, not cloned or reverted. No approvals, secrets, hidden reasoning, attachments or tool state are copied."));
        if let Some(review) = &dialog.review {
            page = page.child(div().text_size(px(13.)).child(format!("Target: {}", review.target_label())))
                .child(div().text_size(px(12.)).text_color(rgb(palette().muted)).child(format!("{} recent messages, {} omitted. Working folder: {}", review.included_messages(), review.omitted_messages(), review.source().working_directory.display())))
                .child(div().h(px(230.)).min_h(px(230.)).flex_shrink_0().flex().flex_col().relative()
                    .child(dialog.editor.clone()).child(ui::layout_probe("handoff-draft")))
                .child(div().text_size(px(12.)).text_color(rgb(palette().muted)).child("Edit the context and request above. Creating the conversation does not send it. Use Send explicitly after reviewing the new draft."));
        } else {
            let query = dialog.query.read(cx).text().trim().to_lowercase();
            let matches: Vec<_> = dialog
                .targets
                .iter()
                .filter(|target| target.label.to_lowercase().contains(&query))
                .collect();
            page = page.child(div().relative().child(dialog.query.clone()).child(ui::layout_probe("handoff-query")))
                .child(div().id("handoff-targets").h(px(300.)).min_h(px(200.)).overflow_y_scroll()
                    .children(matches.iter().take(50).enumerate().map(|(index, target)| {
                        let choice = target.choice.clone();
                        ui::action(("handoff-target", index), target.label.clone(), None, false,
                            cx.listener(move |this, _: &(), _, cx| this.review_handoff_target(choice.clone(), cx)))
                            .w_full().rounded_none().border_b_1().border_color(rgb(palette().border))
                            .relative().child(ui::layout_probe_slot("handoff-target", index))
                    })))
                .child(div().text_size(px(12.)).text_color(rgb(palette().muted)).child(format!("{} matching targets. Showing at most 50 of the first 4096 saved targets; narrow the search. Configure additional models in Direct models settings.", matches.len())));
        }
        page = page
            .children(dialog.error.as_ref().map(|error| {
                div()
                    .text_size(px(12.))
                    .text_color(rgb(palette().error))
                    .child(error.clone())
            }))
            .children(
                self.handoff
                    .busy
                    .then(|| div().text_size(px(12.)).child("Working...")),
            )
            .child(
                div()
                    .pt_3()
                    .border_t_1()
                    .border_color(rgb(palette().border))
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(ui::action(
                        "handoff-cancel",
                        "Discard review",
                        None,
                        false,
                        cx.listener(|this, _: &(), _, cx| this.dismiss_handoff(cx)),
                    ))
                    .children(dialog.review.is_some().then(|| {
                        ui::action(
                            "handoff-copy",
                            "Copy draft",
                            Some(Glyph::Copy),
                            false,
                            cx.listener(|this, _: &(), _, cx| {
                                if let Some(dialog) = &this.handoff.dialog {
                                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                        dialog.editor.read(cx).text().to_owned(),
                                    ));
                                }
                            }),
                        )
                    }))
                    .child(div().flex_1())
                    .children(dialog.review.is_some().then(|| {
                        ui::action(
                            "handoff-create",
                            "Create unsent continuation",
                            Some(Glyph::Compose),
                            false,
                            cx.listener(|this, _: &(), _, cx| this.confirm_handoff(cx)),
                        )
                        .relative()
                        .child(ui::layout_probe("handoff-create"))
                    })),
            );
        div()
            .id("handoff-backdrop")
            .absolute()
            .inset_0()
            .occlude()
            .bg(gpui::rgba(0x00000088))
            .flex()
            .items_center()
            .justify_center()
            .child(page)
            .into_any_element()
    }
    /// Compact Electron-style target menu: brand rows reading
    /// "Handoff to X" that create the unsent continuation on click.
    pub(super) fn handoff_menu(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(dialog) = &self.handoff.dialog else {
            return div().into_any_element();
        };
        let idle = !self.handoff.busy && !self.handoff.creating;
        let query = dialog.query.read(cx).text().trim().to_lowercase();
        let total = dialog
            .targets
            .iter()
            .filter(|target| target.label.to_lowercase().contains(&query))
            .count();
        let rows: Vec<(usize, String, Glyph, HandoffTarget)> = dialog
            .targets
            .iter()
            .enumerate()
            .filter(|(_, target)| target.label.to_lowercase().contains(&query))
            .take(50)
            .map(|(index, target)| {
                let icon = match target.choice {
                    HandoffTarget::Agent(_) => Glyph::Agent,
                    HandoffTarget::Direct(_) => Glyph::Brain,
                };
                (
                    index,
                    format!("Handoff to {}", Self::handoff_short_label(&target.label)),
                    icon,
                    target.choice.clone(),
                )
            })
            .collect();
        let mut panel = div()
            .id("handoff-menu")
            .role(gpui::Role::Menu)
            .aria_label("Hand off thread")
            .w(px(248.))
            .max_h(px(320.))
            .flex()
            .flex_col()
            .overflow_hidden()
            .rounded(px(14.))
            .bg(rgb(palette().overlay))
            .border_1()
            .border_color(rgb(palette().border))
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape" && !this.handoff.busy && !this.handoff.creating {
                    this.dismiss_handoff(cx);
                    cx.stop_propagation();
                }
            }))
            .child(
                div()
                    .p_2()
                    .pb_1()
                    .relative()
                    .child(dialog.query.clone())
                    .child(ui::layout_probe("handoff-menu-query")),
            )
            .child(
                div()
                    .id("handoff-quick-targets")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px_2()
                    .pb_1()
                    .children(rows.into_iter().map(|(index, label, icon, choice)| {
                        ui::action(
                            ("handoff-quick-target", index),
                            label,
                            Some(icon),
                            false,
                            cx.listener(move |this, _: &(), _, cx| {
                                this.review_handoff_target(choice.clone(), cx)
                            }),
                        )
                        .w_full()
                        .rounded(px(10.))
                        .relative()
                        .child(ui::layout_probe_slot("handoff-quick-target", index))
                    }))
                    .children((total == 0 && idle).then(|| {
                        div()
                            .px_2()
                            .py_2()
                            .text_size(px(12.))
                            .text_color(rgb(palette().muted))
                            .child("No matching agents or models.")
                    })),
            );
        if self.handoff.creating {
            panel = panel.child(
                div()
                    .px_3()
                    .py_1()
                    .text_size(px(12.))
                    .child("Creating continuation..."),
            );
        } else if self.handoff.busy {
            panel = panel.child(
                div()
                    .px_3()
                    .py_1()
                    .text_size(px(12.))
                    .text_color(rgb(palette().muted))
                    .child("Preparing..."),
            );
        }
        if let Some(error) = &dialog.error {
            panel = panel.child(
                div()
                    .px_3()
                    .py_1()
                    .text_size(px(12.))
                    .text_color(rgb(palette().error))
                    .child(error.clone()),
            );
        }
        panel = panel.child(
            div().px_2().pb_2().child(
                ui::action(
                    "handoff-to-review",
                    "Review...",
                    Some(Glyph::Notebook),
                    false,
                    cx.listener(|this, _: &(), _, cx| this.switch_handoff_to_review(cx)),
                )
                .w_full()
                .rounded(px(10.))
                .relative()
                .child(ui::layout_probe("handoff-to-review")),
            ),
        );
        div()
            .id("handoff-menu-backdrop")
            .absolute()
            .inset_0()
            .occlude()
            .bg(gpui::rgba(0x00000000))
            .flex()
            .items_start()
            .justify_end()
            .pt(px(82.))
            .pr(px(247.))
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if !this.handoff.busy && !this.handoff.creating {
                        this.dismiss_handoff(cx);
                    }
                }),
            )
            .child(panel)
            .into_any_element()
    }
}
