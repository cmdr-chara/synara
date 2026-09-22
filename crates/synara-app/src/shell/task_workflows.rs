//! Native task-local workflow controls. Service mutations are revision checked;
//! drafting instructions is separate from the normal explicit Send action.
use super::*;
use crate::ui::{self, Glyph, palette};
use gpui::FocusHandle;

pub(super) enum Reply {
    Summary(TaskId, u64, Result<DebugWorkflow, String>),
    Record(TaskId, u64, bool, Result<DebugWorkflow, String>),
}
struct DebugDialog {
    task: TaskId,
    title: String,
    base: Option<DebugWorkflow>,
    problem: Entity<TextEntry>,
    evidence: Entity<TextEntry>,
    _subscriptions: Vec<Subscription>,
    busy: bool,
    error: Option<String>,
    notice: Option<String>,
    confirm_discard: bool,
    confirm_clear: bool,
    inserted_revision: Option<u64>,
}
#[derive(Default)]
pub(super) struct WorkflowState {
    dialog: Option<DebugDialog>,
    generation: u64,
    summary: Option<(TaskId, DebugWorkflow)>,
    previous_focus: Option<FocusHandle>,
    restore_focus: bool,
    focus_input: bool,
}
impl WorkflowState {
    pub fn open(&self) -> bool {
        self.dialog.is_some()
    }
}

impl Shell {
    pub(super) fn load_workflow_summary(&mut self, task: TaskId) {
        self.workflows.summary = None;
        let workspace = self.controller.workspace.clone();
        let selection = self.selection_revision;
        self.job(async move {
            Ok(Update::Workflows(Box::new(Reply::Summary(
                task,
                selection,
                workspace
                    .debug_workflow(task)
                    .await
                    .map_err(|e| e.to_string()),
            ))))
        });
    }
    pub(super) fn open_debug_workflow(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(task) = self.task().cloned() else {
            return;
        };
        if self.workflows.open()
            || self.file_comments.open()
            || self.loading_task.is_some()
            || self.close != CloseState::Open
            || self.revisions.open()
            || self.handoff.open()
            || self.explorer.modal_open()
            || self.saved_context.dialog.is_some()
            || self.organization.dialog.is_some()
            || self.kanban.dialog.is_some()
        {
            return;
        }
        let problem = cx.new(|cx| {
            TextEntry::new(
                "Observed problem and expected behavior...",
                EntryMode::Editor,
                100.,
                cx,
            )
        });
        let evidence = cx.new(|cx| {
            TextEntry::new(
                "Record actual evidence, reproduction results or verification output...",
                EntryMode::Editor,
                110.,
                cx,
            )
        });
        let subscriptions = vec![
            cx.subscribe(&problem, |_, _, _, cx| cx.notify()),
            cx.subscribe(&evidence, |_, _, _, cx| cx.notify()),
        ];
        self.workflows.previous_focus = window.focused(cx);
        self.workflows.dialog = Some(DebugDialog {
            task: task.id,
            title: task.title,
            base: None,
            problem: problem.clone(),
            evidence,
            _subscriptions: subscriptions,
            busy: false,
            error: None,
            notice: None,
            confirm_discard: false,
            confirm_clear: false,
            inserted_revision: None,
        });
        self.focus_composer = false;
        self.controls.retire();
        self.chat_tools.retire();
        self.environment.retire_popup();
        self.navigation.menu_open = false;
        self.settings.popup = None;
        window.focus(&problem.read(cx).focus_handle(cx), cx);
        self.reload_debug_workflow(cx);
    }
    fn reload_debug_workflow(&mut self, cx: &mut Context<Self>) {
        let Some(dialog) = self.workflows.dialog.as_mut() else {
            return;
        };
        if dialog.busy {
            return;
        }
        dialog.busy = true;
        dialog.error = None;
        self.workflows.generation = self.workflows.generation.wrapping_add(1);
        let generation = self.workflows.generation;
        let task = dialog.task;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::Workflows(Box::new(Reply::Record(
                task,
                generation,
                false,
                workspace
                    .debug_workflow(task)
                    .await
                    .map_err(|e| e.to_string()),
            ))))
        });
        cx.notify();
    }
    fn edit_debug_workflow(&mut self, edit: DebugEdit, cx: &mut Context<Self>) {
        let Some(dialog) = self.workflows.dialog.as_mut() else {
            return;
        };
        if dialog.busy
            || self.selected != Some(dialog.task)
            || dialog.problem.read(cx).is_composing()
            || dialog.evidence.read(cx).is_composing()
        {
            return;
        }
        let Some(base) = &dialog.base else {
            return;
        };
        let task = dialog.task;
        let revision = base.revision;
        dialog.busy = true;
        dialog.error = None;
        dialog.notice = None;
        dialog.confirm_clear = false;
        self.workflows.generation = self.workflows.generation.wrapping_add(1);
        let generation = self.workflows.generation;
        let clear = matches!(edit, DebugEdit::Evidence { .. } | DebugEdit::Start { .. });
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::Workflows(Box::new(Reply::Record(
                task,
                generation,
                clear,
                workspace
                    .edit_debug_workflow(task, revision, edit)
                    .await
                    .map_err(|e| e.to_string()),
            ))))
        });
        cx.notify();
    }
    pub(super) fn workflow_reply(&mut self, reply: Reply, cx: &mut Context<Self>) {
        match reply {
            Reply::Summary(task, selection, result) => {
                if self.selected != Some(task) || self.selection_revision != selection {
                    return;
                }
                if let Ok(value) = result {
                    if self
                        .workflows
                        .summary
                        .as_ref()
                        .is_none_or(|(id, old)| *id != task || old.revision <= value.revision)
                    {
                        self.workflows.summary = Some((task, value));
                    }
                }
            }
            Reply::Record(task, generation, clear, result) => {
                if self.workflows.generation != generation {
                    return;
                }
                let Some(dialog) = self.workflows.dialog.as_mut().filter(|d| d.task == task) else {
                    return;
                };
                dialog.busy = false;
                match result {
                    Ok(value) => {
                        self.workflows.summary = Some((task, value.clone()));
                        self.workflows.focus_input = dialog.base.is_none();
                        dialog.base = Some(value);
                        dialog.error = None;
                        if clear {
                            dialog.problem.update(cx, |e, cx| e.clear(cx));
                            dialog.evidence.update(cx, |e, cx| e.clear(cx));
                            dialog.notice = Some("Saved locally. No message was sent.".into());
                        }
                    }
                    Err(error) => dialog.error = Some(error),
                }
            }
        }
        cx.notify();
    }
    fn insert_debug_instructions(&mut self, cx: &mut Context<Self>) {
        let Some(dialog) = self.workflows.dialog.as_mut() else {
            return;
        };
        if dialog.busy
            || self.selected != Some(dialog.task)
            || self.loading_task.is_some()
            || self.draft_state.loading.contains(&dialog.task)
            || self.composer.read(cx).is_composing()
        {
            return;
        }
        let Some(base) = &dialog.base else {
            return;
        };
        if dialog.inserted_revision == Some(base.revision) {
            return;
        }
        let result = base
            .current
            .as_ref()
            .ok_or("Start a Debug run first.".to_owned())
            .and_then(|run| run.draft_instructions().map_err(|e| e.to_string()));
        match result {
            Ok(text) => {
                let current = self.composer.read(cx).text().to_owned();
                let separator = if current.is_empty() { "" } else { "\n\n" };
                if current
                    .len()
                    .saturating_add(separator.len())
                    .saturating_add(text.len())
                    > 1024 * 1024
                {
                    dialog.error =
                        Some("Combined draft exceeds 1 MiB. Nothing was inserted.".into());
                } else {
                    let revision = base.revision;
                    self.composer.update(cx, |entry, cx| {
                        entry.set_text(format!("{current}{separator}{text}"), cx)
                    });
                    dialog.inserted_revision = Some(revision);
                    dialog.notice = Some("Saved phase instructions added to the editable composer. Review and use Send explicitly. Unsaved evidence is not included.".into());
                    self.remember_draft(cx);
                }
            }
            Err(error) => dialog.error = Some(error),
        }
        cx.notify();
    }
    fn dismiss_debug_workflow(&mut self, discard: bool, cx: &mut Context<Self>) {
        let Some(dialog) = self.workflows.dialog.as_mut() else {
            return;
        };
        if dialog.busy
            || dialog.problem.read(cx).is_composing()
            || dialog.evidence.read(cx).is_composing()
        {
            return;
        }
        if !discard
            && (!dialog.problem.read(cx).text().is_empty()
                || !dialog.evidence.read(cx).text().is_empty())
        {
            dialog.confirm_discard = true;
            cx.notify();
            return;
        }
        self.workflows.dialog = None;
        self.workflows.generation = self.workflows.generation.wrapping_add(1);
        self.workflows.restore_focus = true;
        cx.notify();
    }
    pub(super) fn restore_workflow_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.workflows.restore_focus {
            self.workflows.restore_focus = false;
            if let Some(focus) = self.workflows.previous_focus.take() {
                window.focus(&focus, cx);
            }
        }
        if self.workflows.focus_input {
            self.workflows.focus_input = false;
            if let Some(dialog) = &self.workflows.dialog {
                let entry = if dialog.base.as_ref().is_some_and(|b| b.current.is_some()) {
                    &dialog.evidence
                } else {
                    &dialog.problem
                };
                window.focus(&entry.read(cx).focus_handle(cx), cx);
            }
        }
    }
    pub(super) fn workflow_status(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some((_, value)) = self
            .workflows
            .summary
            .as_ref()
            .filter(|(id, _)| Some(*id) == self.selected)
        else {
            return div().into_any_element();
        };
        let Some(run) = &value.current else {
            return div().into_any_element();
        };
        let label = format!(
            "Debug: {}{}",
            run.phase.label(),
            if run.paused { " (paused)" } else { "" }
        );
        ui::action(
            "debug-status",
            label,
            Some(Glyph::Debug),
            false,
            cx.listener(|this, _: &(), window, cx| this.open_debug_workflow(window, cx)),
        )
        .w_full()
        .relative()
        .child(ui::layout_probe("debug-status"))
        .into_any_element()
    }
    pub(super) fn workflow_overlay(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(dialog) = &self.workflows.dialog else {
            return div().into_any_element();
        };
        let mut content = div()
            .id("debug-body")
            .min_h_0()
            .flex_1()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_2();
        if let Some(base) = &dialog.base {
            if let Some(run) = &base.current {
                content = content
                    .child(div().text_size(px(17.)).child(format!(
                        "{}{}",
                        run.phase.label(),
                        if run.paused { " (paused)" } else { "" }
                    )))
                    .child(div().text_size(px(13.)).child(run.problem.clone()))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(palette().muted))
                            .child(run.phase.guidance()),
                    )
                    .child(
                        div()
                            .id("debug-evidence-list")
                            .h(px(145.))
                            .min_h(px(80.))
                            .overflow_y_scroll()
                            .border_y_1()
                            .border_color(rgb(palette().border))
                            .py_2()
                            .children(run.evidence.iter().enumerate().map(|(i, e)| {
                                div().py_1().text_size(px(12.)).child(format!(
                                    "{}. {} / visit {}\n{}",
                                    i + 1,
                                    e.phase.label(),
                                    e.visit,
                                    e.text
                                ))
                            })),
                    );
                if run.phase != DebugPhase::Complete {
                    content = content.child(
                        div()
                            .h(px(114.))
                            .min_h(px(114.))
                            .flex_shrink_0()
                            .relative()
                            .child(dialog.evidence.clone())
                            .child(ui::layout_probe("debug-evidence")),
                    );
                    let has_text = !dialog.evidence.read(cx).text().trim().is_empty();
                    content = content.child(div().flex().flex_wrap().gap_2()
                        .children((!dialog.busy && has_text).then(|| ui::action("debug-add-evidence","Save evidence",Some(Glyph::Plus),false,
                            cx.listener(|this,_:&(),_,cx| { if let Some(d)=&this.workflows.dialog { let text=d.evidence.read(cx).text().to_owned(); this.edit_debug_workflow(DebugEdit::Evidence { text },cx); } }))
                            .relative().child(ui::layout_probe("debug-add-evidence"))))
                        .children((!dialog.busy && run.can_advance()).then(|| ui::action("debug-advance",if run.phase==DebugPhase::Verification { "Mark verified by me" } else { "Next phase" },Some(Glyph::Check),false,
                            cx.listener(|this,_:&(),_,cx| this.edit_debug_workflow(DebugEdit::Advance,cx))).relative().child(ui::layout_probe("debug-advance"))))
                        .children((!dialog.busy).then(|| { let paused=run.paused; ui::action("debug-pause",if paused { "Resume" } else { "Pause" },None,false,
                            cx.listener(move |this,_:&(),_,cx| this.edit_debug_workflow(DebugEdit::Pause(!paused),cx))).relative().child(ui::layout_probe("debug-pause")) })))
                        .children((!run.can_advance()).then(|| div().text_size(px(11.)).text_color(rgb(palette().muted)).child("Save evidence for this phase before advancing. Paused phases must be resumed.")));
                }
                content = content.child(div().flex().flex_wrap().gap_2()
                    .children((!dialog.busy && !run.paused && run.phase!=DebugPhase::Complete && dialog.inserted_revision!=Some(base.revision)).then(||
                        ui::action("debug-insert","Add phase instructions to draft",Some(Glyph::Compose),false,cx.listener(|this,_:&(),_,cx| this.insert_debug_instructions(cx)))
                            .relative().child(ui::layout_probe("debug-insert"))))
                    .children((!dialog.busy && matches!(run.phase,DebugPhase::Fix|DebugPhase::Verification|DebugPhase::Complete)).then(||
                        ui::action("debug-reopen","Reinvestigate",Some(Glyph::Restore),false,cx.listener(|this,_:&(),_,cx| this.edit_debug_workflow(DebugEdit::Reinvestigate,cx)))
                            .relative().child(ui::layout_probe("debug-reopen")))))
                    .child(div().text_size(px(11.)).text_color(rgb(palette().muted)).child(format!("{} evidence records. User-recorded results, not independent certification. No agent or permission mode was changed.",run.evidence.len())));
            } else {
                content = content.child(div().text_size(px(13.)).child("Observe, reproduce, investigate, fix, then verify. Each phase needs your recorded evidence. Starting does not execute anything."))
                    .child(div().h(px(105.)).min_h(px(105.)).flex_shrink_0().relative().child(dialog.problem.clone()).child(ui::layout_probe("debug-problem")))
                    .children((!dialog.busy && !dialog.problem.read(cx).text().trim().is_empty()).then(|| ui::action("debug-start","Start Debug workflow",Some(Glyph::Debug),false,
                        cx.listener(|this,_:&(),_,cx| { if let Some(d)=&this.workflows.dialog { let problem=d.problem.read(cx).text().to_owned(); this.edit_debug_workflow(DebugEdit::Start { problem },cx); } }))
                        .relative().child(ui::layout_probe("debug-start"))));
            }
            if !base.history.is_empty() {
                content = content
                    .child(
                        div()
                            .text_size(px(12.))
                            .child("Recent closed runs (compact summaries)"),
                    )
                    .children(base.history.iter().rev().map(|h| {
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(palette().muted))
                            .child(format!(
                                "{} · {} evidence records · {}",
                                h.last_phase.label(),
                                h.evidence_count,
                                h.problem
                            ))
                    }));
            }
        }
        let mut page = div()
            .id("debug-dialog")
            .role(gpui::Role::Dialog)
            .aria_label("Debug workflow")
            .tab_group()
            .w_full()
            .max_w(px(720.))
            .h(px(710.))
            .max_h_full()
            .p_4()
            .flex()
            .flex_col()
            .gap_2()
            .bg(ui::surface(palette().overlay))
            .border_1()
            .border_color(rgb(palette().border))
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape" && !event.prefer_character_input {
                    this.dismiss_debug_workflow(false, cx);
                    cx.stop_propagation();
                }
            }))
            .child(div().text_size(px(20.)).child("Debug workflow"))
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(rgb(palette().muted))
                    .child(dialog.title.clone()),
            )
            .child(content)
            .children(dialog.busy.then(|| {
                div()
                    .text_size(px(12.))
                    .child("Saving or loading local state...")
            }))
            .children(dialog.error.as_ref().map(|e| {
                div()
                    .text_size(px(12.))
                    .text_color(rgb(palette().error))
                    .child(e.clone())
            }))
            .children(
                dialog
                    .notice
                    .as_ref()
                    .map(|n| div().text_size(px(12.)).child(n.clone())),
            );
        if dialog.confirm_discard {
            page = page.child(div().text_size(px(12.)).child("Discard the unsaved text in this dialog? Saved evidence and the chat draft are kept."))
                .child(ui::action("debug-discard","Discard unsaved text and close",None,false,cx.listener(|this,_:&(),_,cx| this.dismiss_debug_workflow(true,cx))))
                .child(ui::action("debug-keep","Keep editing",None,false,cx.listener(|this,_:&(),_,cx| { if let Some(d)=this.workflows.dialog.as_mut() { d.confirm_discard=false; } cx.notify(); })));
        }
        if dialog.confirm_clear {
            page = page.child(div().text_size(px(12.)).child("Close this run and replace its detailed evidence with a compact history summary? Copy any evidence you need first. No files or transcript will change."))
                .child(ui::action("debug-confirm-clear","Close run and keep summary",None,false,cx.listener(|this,_:&(),_,cx| this.edit_debug_workflow(DebugEdit::Clear,cx)))
                    .relative().child(ui::layout_probe("debug-confirm-clear")));
        }
        page = page.child(
            div()
                .pt_2()
                .border_t_1()
                .border_color(rgb(palette().border))
                .flex()
                .flex_wrap()
                .gap_2()
                .child(
                    ui::action(
                        "debug-close",
                        "Close",
                        None,
                        false,
                        cx.listener(|this, _: &(), _, cx| this.dismiss_debug_workflow(false, cx)),
                    )
                    .relative()
                    .child(ui::layout_probe("debug-close")),
                )
                .children(dialog.base.as_ref().map(|base| {
                    let text = serde_json::to_string_pretty(base).unwrap_or_default();
                    ui::action(
                        "debug-copy",
                        "Copy saved evidence",
                        Some(Glyph::Copy),
                        false,
                        cx.listener(move |_, _: &(), _, cx| {
                            cx.write_to_clipboard(gpui::ClipboardItem::new_string(text.clone()))
                        }),
                    )
                }))
                .children((!dialog.busy).then(|| {
                    ui::action(
                        "debug-reload",
                        "Reload saved state",
                        Some(Glyph::Restore),
                        false,
                        cx.listener(|this, _: &(), _, cx| this.reload_debug_workflow(cx)),
                    )
                }))
                .children(
                    (!dialog.busy && dialog.base.as_ref().is_some_and(|b| b.current.is_some()))
                        .then(|| {
                            ui::action(
                                "debug-clear",
                                "Close run...",
                                None,
                                false,
                                cx.listener(|this, _: &(), _, cx| {
                                    if let Some(d) = this.workflows.dialog.as_mut() {
                                        d.confirm_clear = true;
                                    }
                                    cx.notify();
                                }),
                            )
                            .relative()
                            .child(ui::layout_probe("debug-clear"))
                        }),
                ),
        );
        div()
            .id("debug-backdrop")
            .absolute()
            .inset_0()
            .occlude()
            .bg(gpui::rgba(0x00000088))
            .flex()
            .items_center()
            .justify_center()
            .p_3()
            .child(page)
            .into_any_element()
    }
}
