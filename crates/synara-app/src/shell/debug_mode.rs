//! Upstream parity: Plan and Debug are persisted per-task interaction modes.
//! They prepend provider-independent instructions to provider-bound prompts
//! (`with_plan_prompt`/`with_debug_prompt`); they grant no extra permissions and
//! are not ACP session modes.
use super::controls::ControlKind;
use super::*;
use crate::ui;

impl Shell {
    pub(super) fn load_interaction_mode(&mut self, task: TaskId, _cx: &mut Context<Self>) {
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            let mode = workspace.interaction_mode(task).await?;
            Ok(Update::InteractionMode { task, mode })
        });
    }
    /// /plan, /debug, /default, the mode-menu rows and the badge all land here.
    pub(super) fn set_interaction_mode(
        &mut self,
        task: TaskId,
        mode: InteractionMode,
        cx: &mut Context<Self>,
    ) -> bool {
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            workspace.set_interaction_mode(task, mode).await?;
            Ok(Update::InteractionMode { task, mode })
        });
        cx.notify();
        true
    }
    /// Slash-command entries. The mode applies to the next provider turn, so it
    /// may change while a turn is in flight.
    pub(super) fn debug_mode_command(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(task) = self.selected else {
            self.error = Some("Create or select a task first.".into());
            cx.notify();
            return false;
        };
        self.set_interaction_mode(task, InteractionMode::Debug, cx)
    }
    /// Upstream /plan switches the thread into the plan interaction mode — a
    /// prompt shim, not an ACP session mode.
    pub(super) fn plan_mode_command(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(task) = self.selected else {
            self.error = Some("Create or select a task first.".into());
            cx.notify();
            return false;
        };
        self.set_interaction_mode(task, InteractionMode::Plan, cx)
    }
    /// Upstream /default also returns the provider session to its default mode
    /// where the session advertises one.
    pub(super) fn default_mode_command(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(task) = self.selected else {
            self.error = Some("Create or select a task first.".into());
            cx.notify();
            return false;
        };
        if self
            .mode_tasks
            .get(&task)
            .is_some_and(|mode| *mode != InteractionMode::Default)
        {
            self.set_interaction_mode(task, InteractionMode::Default, cx);
        }
        if !self.controls_blocked()
            && let Some((choice, action)) = self
                .control_choices(ControlKind::Mode)
                .into_iter()
                .find(|(choice, _)| choice.label.eq_ignore_ascii_case("default"))
            && choice.unavailable.is_none()
            && !choice.selected
        {
            self.apply_session_control(task, action, cx);
        }
        true
    }
    pub(super) fn interaction_mode_reply(
        &mut self,
        task: TaskId,
        mode: InteractionMode,
        cx: &mut Context<Self>,
    ) {
        if mode == InteractionMode::Default {
            self.mode_tasks.remove(&task);
        } else {
            self.mode_tasks.insert(task, mode);
        }
        cx.notify();
    }
    /// Mode badge on the shared workflow strip; selecting it returns to
    /// Default, matching the upstream composer badge.
    pub(super) fn mode_compact(&self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let task = self.selected?;
        let mode = *self.mode_tasks.get(&task)?;
        if mode == InteractionMode::Default {
            return None;
        }
        Some(
            ui::header_action(
                "interaction-mode-off",
                mode.label(),
                None,
                false,
                cx.listener(|this, _: &(), _, cx| {
                    if let Some(task) = this.selected {
                        this.set_interaction_mode(task, InteractionMode::Default, cx);
                    }
                }),
            )
            .child(ui::layout_probe("interaction-mode-off"))
            .into_any_element(),
        )
    }
}
