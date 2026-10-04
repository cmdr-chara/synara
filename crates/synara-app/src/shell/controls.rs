//! Presentation-only session pickers. The controller owns every applied value.
use super::*;
use crate::ui::{
    self,
    menu::{Choice, ChoiceEvent, ChoiceMenu},
};
use gpui::{Bounds, FocusHandle, Pixels, canvas, point};
use std::{cell::Cell, rc::Rc};
use synara_workspace::SessionModelPreset;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ControlKind {
    Project,
    Agent,
    Mode,
    Model,
    Options,
    Extras,
    Access,
    PlanImplement,
}
impl ControlKind {
    fn index(self) -> usize {
        match self {
            Self::Project => 4,
            Self::Agent => 0,
            Self::Mode => 1,
            Self::Model => 2,
            Self::Options => 3,
            Self::Extras => 5,
            Self::Access => 6,
            Self::PlanImplement => 7,
        }
    }
    fn title(self) -> &'static str {
        match self {
            Self::Project => "Work in a project",
            Self::Agent => "Model and agent",
            Self::Mode => "Session mode",
            Self::Model => "Model",
            Self::Options => "Session options",
            Self::Extras => "Add",
            Self::Access => "Agent permissions",
            Self::PlanImplement => "Implement",
        }
    }
    fn id(self) -> &'static str {
        match self {
            Self::Project => "project-picker",
            Self::Agent => "agent-picker",
            Self::Mode => "mode-picker",
            Self::Model => "model-picker",
            Self::Options => "options-picker",
            Self::Extras => "composer-extras",
            Self::Access => "access-picker",
            Self::PlanImplement => "plan-implement-menu",
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub(in crate::shell) enum ControlAction {
    PlanMode,
    DebugMode,
    Project(ProjectId),
    BrowseWorkspace,
    AttachFiles,
    AttachWindow,
    AttachWindowIndex(usize),
    ExtrasBack,
    ImplementPlanNewThread,
    Goal,
    Unavailable,
    AccessInfo,
    Connect,
    Agent(String),
    Mode(String),
    Model(String),
    Option(String, ConfigValue),
}
struct Trigger {
    focus: FocusHandle,
    bounds: Rc<Cell<Bounds<Pixels>>>,
}
struct OpenControl {
    task: Option<TaskId>,
    agent: Option<String>,
    session: Option<String>,
    connection: Option<ConnectionId>,
    kind: ControlKind,
    choices: Vec<ControlAction>,
    view: Entity<ChoiceMenu>,
    _subscription: Subscription,
}
pub(super) struct ControlState {
    open: Option<OpenControl>,
    triggers: [Trigger; 8],
    pub composer_bounds: Rc<Cell<Bounds<Pixels>>>,
    pending: HashSet<TaskId>,
    extras_windows: bool,
    /// Mode id waiting on a successful dispatch; folded into
    /// `catalog.runtime_modes` when ControlFinished lands.
    pub pending_mode: Option<(TaskId, String)>,
}
impl ControlState {
    pub fn new(cx: &mut Context<Shell>) -> Self {
        Self {
            open: None,
            composer_bounds: Rc::new(Cell::new(Bounds::default())),
            triggers: std::array::from_fn(|_| Trigger {
                focus: cx.focus_handle(),
                bounds: Rc::new(Cell::new(Bounds::default())),
            }),
            pending: HashSet::new(),
            extras_windows: false,
            pending_mode: None,
        }
    }
    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }
    pub fn is_pending(&self, task: TaskId) -> bool {
        self.pending.contains(&task)
    }
    pub fn retire(&mut self) {
        self.open = None;
        self.extras_windows = false;
    }
    pub fn completed(&mut self, task: TaskId) {
        self.pending.remove(&task);
    }
}

/// Upstream `RUNTIME_MODE_PRESENTATION` copy for provider session modes whose
/// ids map onto the three runtime modes (verbatim labels/descriptions).
fn access_mode_copy(mode_id: &str) -> Option<(&'static str, &'static str)> {
    match mode_id {
        "approval-required" => Some((
            "Ask for approval",
            "Always ask to edit external files and use the internet",
        )),
        "auto" => Some((
            "Approve for me",
            "Only ask for actions detected as potentially unsafe",
        )),
        "full-access" => Some((
            "Full access",
            "Unrestricted access to the internet and any file on your computer",
        )),
        _ => None,
    }
}

fn option_kind(option: &SessionOption) -> ControlKind {
    match option.category.as_deref() {
        Some("model") => ControlKind::Model,
        Some("mode") => ControlKind::Mode,
        _ => ControlKind::Options,
    }
}
fn ordered_profiles<'a>(
    profiles: &'a [AgentProfile],
    preferred: &[String],
) -> Vec<&'a AgentProfile> {
    let mut ordered: Vec<_> = profiles.iter().enumerate().collect();
    ordered.sort_by_key(|(registry_index, profile)| {
        (
            preferred
                .iter()
                .position(|id| id == &profile.id)
                .unwrap_or(usize::MAX),
            *registry_index,
        )
    });
    ordered.into_iter().map(|(_, profile)| profile).collect()
}
fn is_effort_option(option: &SessionOption) -> bool {
    if !matches!(option.current, ConfigValue::Select { .. }) {
        return false;
    }
    let contains_effort = |text: &str| {
        text.split(|ch: char| !ch.is_ascii_alphanumeric())
            .any(|part| part.eq_ignore_ascii_case("effort"))
    };
    option.category.as_deref().is_some_and(contains_effort)
        || contains_effort(&option.id)
        || contains_effort(&option.name)
}
fn current_model_preset(
    agent: &str,
    configuration: &SessionConfiguration,
) -> Option<SessionModelPreset> {
    let (model_option, model) = session_choices(configuration, ControlKind::Model)
        .into_iter()
        .find(|(choice, _)| choice.selected)
        .and_then(|(_, action)| match action {
            ControlAction::Model(model) => Some((None, model)),
            ControlAction::Option(option, ConfigValue::Select { value }) => {
                Some((Some(option), value))
            }
            _ => None,
        })?;
    let effort = configuration.options.iter().find_map(|option| {
        if !is_effort_option(option) {
            return None;
        }
        let ConfigValue::Select { value } = &option.current else {
            return None;
        };
        option
            .choices
            .iter()
            .any(|choice| choice.value == *value)
            .then(|| (option.id.clone(), value.clone()))
    });
    Some(SessionModelPreset {
        agent: agent.into(),
        model_option,
        model,
        effort_option: effort.as_ref().map(|(option, _)| option.clone()),
        effort: effort.map(|(_, value)| value),
    })
}
fn model_action_for_preset(preset: &SessionModelPreset) -> ControlAction {
    match &preset.model_option {
        Some(option) => ControlAction::Option(
            option.clone(),
            ConfigValue::Select {
                value: preset.model.clone(),
            },
        ),
        None => ControlAction::Model(preset.model.clone()),
    }
}
fn preset_model_action(
    preset: &SessionModelPreset,
    agent: &str,
    configuration: &SessionConfiguration,
) -> Option<Option<ControlAction>> {
    if preset.agent != agent || preset.effort_option.is_some() != preset.effort.is_some() {
        return None;
    }
    let model = model_action_for_preset(preset);
    let model_choice = session_choices(configuration, ControlKind::Model)
        .into_iter()
        .find(|(_, candidate)| *candidate == model)?;
    Some((!model_choice.0.selected).then_some(model))
}
fn advertised_effort_action(
    preset: &SessionModelPreset,
    configuration: &SessionConfiguration,
) -> Option<(ControlAction, bool)> {
    if let (Some(option), Some(effort)) = (&preset.effort_option, &preset.effort) {
        if !configuration
            .options
            .iter()
            .any(|candidate| candidate.id == *option && is_effort_option(candidate))
        {
            return None;
        }
        let action = ControlAction::Option(
            option.clone(),
            ConfigValue::Select {
                value: effort.clone(),
            },
        );
        let choice = session_choices(configuration, ControlKind::Options)
            .into_iter()
            .find(|(_, candidate)| *candidate == action)?;
        return Some((action, !choice.0.selected));
    }
    None
}
fn session_choices(
    configuration: &SessionConfiguration,
    kind: ControlKind,
) -> Vec<(Choice, ControlAction)> {
    let mut result = Vec::new();
    for option in configuration
        .options
        .iter()
        .filter(|option| option_kind(option) == kind)
    {
        let detail = option.description.as_ref().map_or_else(
            || option.name.clone(),
            |description| format!("{} · {}", option.name, description),
        );
        match &option.current {
            ConfigValue::Boolean { value } => {
                for next in [true, false] {
                    result.push((
                        Choice {
                            label: if next { "On" } else { "Off" }.into(),
                            detail: detail.clone(),
                            selected: *value == next,
                            ..Default::default()
                        },
                        ControlAction::Option(
                            option.id.clone(),
                            ConfigValue::Boolean { value: next },
                        ),
                    ));
                }
            }
            ConfigValue::Select { value } => {
                for choice in &option.choices {
                    result.push((
                        Choice {
                            label: choice.label.clone(),
                            detail: choice.group.as_ref().map_or_else(
                                || detail.clone(),
                                |group| format!("{group} · {detail}"),
                            ),
                            selected: choice.value == *value,
                            ..Default::default()
                        },
                        ControlAction::Option(
                            option.id.clone(),
                            ConfigValue::Select {
                                value: choice.value.clone(),
                            },
                        ),
                    ));
                }
            }
        }
    }
    // Config-option categories replace the legacy selector even when empty.
    // Do not invent choices or fall back to stale legacy capability data.
    let category_present = configuration
        .options
        .iter()
        .any(|option| option_kind(option) == kind);
    if !category_present && kind == ControlKind::Mode {
        result.extend(configuration.modes.iter().map(|mode| {
            (
                Choice {
                    label: mode.name.clone(),
                    detail: mode.description.clone().unwrap_or_default(),
                    selected: configuration.current_mode.as_deref() == Some(&mode.id),
                    ..Default::default()
                },
                ControlAction::Mode(mode.id.clone()),
            )
        }));
    }
    if !category_present && kind == ControlKind::Model {
        result.extend(configuration.models.iter().map(|model| {
            (
                Choice {
                    label: model.label.clone(),
                    detail: model.group.clone().unwrap_or_default(),
                    selected: configuration.current_model.as_deref() == Some(&model.value),
                    ..Default::default()
                },
                ControlAction::Model(model.value.clone()),
            )
        }));
    }
    result
}

fn cycle_model_action(choices: &[(Choice, ControlAction)], forward: bool) -> Option<ControlAction> {
    let available: Vec<_> = choices
        .iter()
        .filter(|(choice, action)| {
            choice.unavailable.is_none()
                && matches!(
                    action,
                    ControlAction::Model(_) | ControlAction::Option(_, ConfigValue::Select { .. })
                )
        })
        .collect();
    if available.len() < 2 {
        return None;
    }
    let option = match &available.first()?.1 {
        ControlAction::Model(_) => None,
        ControlAction::Option(option, ConfigValue::Select { .. }) => Some(option.as_str()),
        _ => return None,
    };
    if available.iter().any(|(_, action)| match (option, action) {
        (None, ControlAction::Model(_)) => false,
        (Some(expected), ControlAction::Option(candidate, ConfigValue::Select { .. })) => {
            candidate != expected
        }
        _ => true,
    }) {
        return None;
    }
    let current = available.iter().position(|(choice, _)| choice.selected);
    let next = match (current, forward) {
        (Some(index), true) => (index + 1) % available.len(),
        (Some(index), false) => (index + available.len() - 1) % available.len(),
        (None, true) => 0,
        (None, false) => available.len() - 1,
    };
    Some(available[next].1.clone())
}

impl Shell {
    pub(super) fn cycle_session_model(&mut self, forward: bool, cx: &mut Context<Self>) -> bool {
        if self.controls_blocked() || self.loading_task.is_some() {
            return false;
        }
        let models = self.control_choices(ControlKind::Model);
        let Some(action) = cycle_model_action(&models, forward) else {
            return false;
        };
        if let Some(task) = self.selected {
            self.apply_session_control(task, action, cx);
            return self.controls.is_pending(task);
        }
        false
    }

    /// The configured model cycle, only inside the main composer.
    /// Text/IME, side-chat, terminal, editor and open menus keep their input.
    pub(super) fn model_cycle_shortcut(
        &mut self,
        event: &gpui::KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if event.prefer_character_input
            || event.is_held
            || self.close != CloseState::Open
            || self.hubs.pending(cx)
            || self.command_palette.open
            || self.controls.is_open()
            || self.chat_tools.menu_open()
            || self.navigation.menu_open
            || self.environment.menu_open()
            || self.settings.popup.is_some()
            || self.composer.read(cx).is_composing()
            || !self.composer.read(cx).focus_handle(cx).is_focused(window)
            || (self.panel != Panel::Conversation && !self.dock_open())
        {
            return false;
        }
        let Some(stroke) = crate::input::keybinding_stroke(event) else {
            return false;
        };
        let forward = match contextual_command_for_key(
            &self.settings.value.keybindings,
            KeybindingContext::Composer,
            &stroke,
        ) {
            Some("model.next") => true,
            Some("model.previous") => false,
            _ => return false,
        };
        self.cycle_session_model(forward, cx)
    }

    pub(in crate::shell) fn control_choices(
        &self,
        kind: ControlKind,
    ) -> Vec<(Choice, ControlAction)> {
        if kind == ControlKind::Access {
            let modes = self.control_choices(ControlKind::Mode);
            if modes.is_empty() {
                // Upstream's access picker lists the three runtime modes even
                // before a session exists — the choice is draft state, applied
                // when the session starts (DEFAULT_RUNTIME_MODE = full-access).
                let Some(task) = self.task() else {
                    return vec![(
                        Choice {
                            label: "Ask for approval".into(),
                            detail: "Always ask to edit external files and use the internet".into(),
                            selected: true,
                            ..Default::default()
                        },
                        ControlAction::AccessInfo,
                    )];
                };
                let stored = self.catalog.runtime_modes.get(&task.id).cloned();
                return ["approval-required", "auto", "full-access"]
                    .into_iter()
                    .map(|id| {
                        let (label, detail) = access_mode_copy(id).unwrap_or((id, ""));
                        (
                            Choice {
                                label: label.into(),
                                detail: detail.into(),
                                selected: stored.as_deref().unwrap_or("full-access") == id,
                                ..Default::default()
                            },
                            ControlAction::Mode(id.into()),
                        )
                    })
                    .collect();
            }
            return modes
                .into_iter()
                .map(|(mut choice, action)| {
                    if let ControlAction::Mode(id) = &action
                        && let Some((label, detail)) = access_mode_copy(id)
                    {
                        choice.label = label.into();
                        if choice.detail.is_empty() {
                            choice.detail = detail.into();
                        }
                    }
                    (choice, action)
                })
                .collect();
        }
        if kind == ControlKind::Extras {
            if self.controls.extras_windows {
                let mut rows = vec![(
                    Choice {
                        label: "Back".into(),
                        icon: Some(ui::Glyph::Back),
                        ..Default::default()
                    },
                    ControlAction::ExtrasBack,
                )];
                if self.appsnap.busy {
                    rows.push((
                        Choice {
                            label: "Finding windows…".into(),
                            unavailable: Some("Window discovery is still running.".into()),
                            ..Default::default()
                        },
                        ControlAction::Unavailable,
                    ));
                } else if self.appsnap.windows.is_empty() {
                    rows.push((
                        Choice {
                            label: "No capturable windows".into(),
                            detail: "No supported visible application windows were found.".into(),
                            unavailable: Some("Nothing to capture.".into()),
                            ..Default::default()
                        },
                        ControlAction::Unavailable,
                    ));
                } else {
                    rows.extend(
                        self.appsnap
                            .windows
                            .iter()
                            .enumerate()
                            .map(|(index, window)| {
                                (
                                    Choice {
                                        label: if window.title.is_empty() {
                                            window.class.clone()
                                        } else {
                                            window.title.clone()
                                        },
                                        detail: format!(
                                            "{} · {}×{}",
                                            window.class, window.width, window.height
                                        ),
                                        icon: Some(ui::Glyph::Window),
                                        ..Default::default()
                                    },
                                    ControlAction::AttachWindowIndex(index),
                                )
                            }),
                    );
                }
                return rows;
            }
            let in_plan = self
                .selected
                .is_some_and(|task| self.mode_tasks.get(&task) == Some(&InteractionMode::Plan));
            let in_debug = self
                .selected
                .is_some_and(|task| self.mode_tasks.get(&task) == Some(&InteractionMode::Debug));
            let mut rows = vec![
                (
                    Choice {
                        label: "Files and folders".into(),
                        icon: Some(ui::Glyph::Attach),
                        ..Default::default()
                    },
                    ControlAction::AttachFiles,
                ),
                (
                    Choice {
                        label: "Attach window".into(),
                        detail: "Capture an open app window".into(),
                        icon: Some(ui::Glyph::Window),
                        unavailable: self
                            .selected
                            .is_none()
                            .then(|| "Select a task first.".into()),
                        ..Default::default()
                    },
                    ControlAction::AttachWindow,
                ),
                (
                    Choice {
                        label: "Goal".into(),
                        detail: "Set a goal to keep pursuing".into(),
                        icon: Some(ui::Glyph::Goal),
                        unavailable: self
                            .selected
                            .is_none()
                            .then(|| "Select a task first.".into()),
                        ..Default::default()
                    },
                    ControlAction::Goal,
                ),
            ];
            rows.push((
                Choice {
                    label: "Plan mode".into(),
                    detail: format!("Turn plan mode {}", if in_plan { "off" } else { "on" }),
                    icon: Some(ui::Glyph::Plan),
                    selected: in_plan,
                    unavailable: self
                        .selected
                        .is_none()
                        .then(|| "Select a task first.".into()),
                    ..Default::default()
                },
                ControlAction::PlanMode,
            ));
            rows.push((
                Choice {
                    label: "Debug mode".into(),
                    detail: format!("Turn debug mode {}", if in_debug { "off" } else { "on" }),
                    icon: Some(ui::Glyph::Debug),
                    selected: in_debug,
                    unavailable: self
                        .selected
                        .is_none()
                        .then(|| "Select a task first.".into()),
                    ..Default::default()
                },
                ControlAction::DebugMode,
            ));
            return rows;
        }
        // Upstream: the empty-draft plan follow-up submit is a split button
        // whose chevron menu carries "Implement in a new thread".
        if kind == ControlKind::PlanImplement {
            return vec![(
                Choice {
                    label: "Implement in a new thread".into(),
                    ..Default::default()
                },
                ControlAction::ImplementPlanNewThread,
            )];
        }
        if kind == ControlKind::Project {
            let mut choices: Vec<_> = self
                .catalog
                .projects
                .iter()
                .filter(|project| !self.is_chat_workspace(project))
                .map(|project| {
                    (
                        Choice {
                            label: self.project_name(project).to_string(),
                            detail: String::new(),
                            selected: self.project == Some(project.id),
                            custom_icon: Some(self.project_choice_icon(project)),
                            ..Default::default()
                        },
                        ControlAction::Project(project.id),
                    )
                })
                .collect();
            choices.push((
                Choice {
                    label: "Open a project…".into(),
                    detail: "Choose a folder".into(),
                    selected: false,
                    ..Default::default()
                },
                ControlAction::BrowseWorkspace,
            ));
            return choices;
        }
        if kind == ControlKind::Agent {
            let models = self.control_choices(ControlKind::Model);
            let profiles =
                ordered_profiles(&self.profiles, &self.settings.value.general.provider_order);
            let mut choices: Vec<_> = profiles
                .iter()
                .map(|profile| {
                    (
                        Choice {
                            label: profile.name.clone(),
                            detail: "Coding agent".into(),
                            icon: Some(self.agent_glyph(&profile.id)),
                            selected: models.is_empty()
                                && self.task().is_some_and(|task| task.agent_id == profile.id),
                            ..Default::default()
                        },
                        ControlAction::Agent(profile.id.clone()),
                    )
                })
                .collect();
            choices.extend(models);
            for kind in [ControlKind::Mode, ControlKind::Options] {
                choices.extend(self.control_choices(kind).into_iter().map(
                    |(mut choice, action)| {
                        choice.label = format!(
                            "{} · {}",
                            if choice.detail.is_empty() {
                                kind.title()
                            } else {
                                &choice.detail
                            },
                            choice.label
                        );
                        choice.icon = Some(ui::Glyph::Sliders);
                        (choice, action)
                    },
                ));
            }
            if self.details.as_ref().is_none_or(|d| {
                d.connection.state != ConnectionState::Connected || d.session_id.is_none()
            }) {
                choices.push((
                    Choice {
                        label: "Connect and load models".into(),
                        detail: "Start the selected agent explicitly".into(),
                        icon: Some(ui::Glyph::Agent),
                        ..Default::default()
                    },
                    ControlAction::Connect,
                ));
            }
            return choices;
        }
        self.details
            .as_ref()
            .filter(|details| {
                details.connection.state == ConnectionState::Connected
                    && details.session_id.is_some()
            })
            .and(self.thread.as_ref())
            .map_or_else(Vec::new, |thread| {
                session_choices(&thread.configuration, kind)
            })
    }
    pub(super) fn controls_blocked(&self) -> bool {
        self.selected.is_none_or(|task| {
            self.busy.contains(&task)
                || self.connecting.contains(&task)
                || self.controls.is_pending(task)
        })
    }
    /// AppSnap discovery/capture finished while the extras menu sat in its
    /// windows view — repopulate the open menu in place.
    pub(super) fn refresh_extras_menu(&mut self, cx: &mut Context<Self>) {
        if !self.controls.extras_windows
            || !self
                .controls
                .open
                .as_ref()
                .is_some_and(|open| open.kind == ControlKind::Extras)
        {
            return;
        }
        let (items, choices): (Vec<_>, Vec<_>) = self
            .control_choices(ControlKind::Extras)
            .into_iter()
            .unzip();
        if let Some(open) = self.controls.open.as_mut() {
            open.choices = choices;
            open.view.update(cx, |menu, cx| menu.set_choices(items, cx));
        }
    }
    pub(super) fn dismiss_control(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.controls.extras_windows = false;
        if let Some(open) = self.controls.open.take() {
            window.focus(&self.controls.triggers[open.kind.index()].focus, cx);
            cx.notify();
        }
    }
    pub(super) fn open_control(
        &mut self,
        kind: ControlKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if kind != ControlKind::Project && self.controls_blocked() {
            return;
        }
        if self
            .controls
            .open
            .as_ref()
            .is_some_and(|open| open.kind == kind)
        {
            self.dismiss_control(window, cx);
            return;
        }
        let task = self.task().cloned();
        let (items, choices): (Vec<_>, Vec<_>) = self.control_choices(kind).into_iter().unzip();
        if items.is_empty() {
            return;
        }
        self.navigation.menu_open = false;
        let view = cx.new(|cx| {
            let view = ChoiceMenu::new(kind.title().into(), items, cx);
            if matches!(kind, ControlKind::Extras | ControlKind::PlanImplement) {
                view.add_layout(self.controls.composer_bounds.clone())
            } else if kind == ControlKind::Agent {
                let profiles =
                    ordered_profiles(&self.profiles, &self.settings.value.general.provider_order);
                let current = profiles
                    .iter()
                    .position(|p| task.as_ref().is_some_and(|t| t.agent_id == p.id))
                    .unwrap_or(0);
                let sources = profiles
                    .iter()
                    .map(|p| ui::menu::ModelSource {
                        name: p.name.clone(),
                        icon: self.agent_glyph(&p.id),
                    })
                    .collect();
                let models = self.control_choices(ControlKind::Model);
                let rows = choices
                    .iter()
                    .map(|action| {
                        let source = match action {
                            ControlAction::Agent(id) => profiles
                                .iter()
                                .position(|profile| &profile.id == id)
                                .unwrap_or(current),
                            _ => current,
                        };
                        let favorite = if models.iter().any(|(_, a)| a == action) {
                            task.as_ref().and_then(|task| match action {
                                ControlAction::Model(value) => Some(ModelFavorite {
                                    agent: task.agent_id.clone(),
                                    option: None,
                                    value: value.clone(),
                                }),
                                ControlAction::Option(key, ConfigValue::Select { value }) => {
                                    Some(ModelFavorite {
                                        agent: task.agent_id.clone(),
                                        option: Some(key.clone()),
                                        value: value.clone(),
                                    })
                                }
                                _ => None,
                            })
                        } else {
                            None
                        };
                        let preset = task.as_ref().and_then(|task| {
                            let preset = self
                                .thread
                                .as_ref()
                                .filter(|_| {
                                    self.details.as_ref().is_some_and(|details| {
                                        details.connection.state == ConnectionState::Connected
                                            && details.session_id.is_some()
                                    })
                                })
                                .and_then(|thread| {
                                    current_model_preset(&task.agent_id, &thread.configuration)
                                })?;
                            (action == &model_action_for_preset(&preset)).then_some(preset)
                        });
                        ui::menu::ModelRow {
                            source,
                            favorite,
                            preset,
                        }
                    })
                    .collect();
                view.with_models(
                    sources,
                    rows,
                    (
                        current,
                        task.as_ref()
                            .map_or_else(String::new, |task| task.agent_id.clone()),
                    ),
                    self.controller.workspace.clone(),
                    self.runtime.clone(),
                    cx,
                )
            } else {
                view
            }
        });
        let subscription = cx.subscribe_in(&view, window, |this, _, event, window, cx| {
            this.control_event(event.clone(), window, cx);
        });
        window.focus(&view.read(cx).focus_handle(cx), cx);
        self.controls.open = Some(OpenControl {
            task: task.as_ref().map(|task| task.id),
            agent: task.map(|task| task.agent_id),
            session: self
                .details
                .as_ref()
                .and_then(|details| details.session_id.clone()),
            connection: self.details.as_ref().map(|details| details.connection.id),
            kind,
            choices,
            view,
            _subscription: subscription,
        });
        cx.notify();
    }
    fn control_event(&mut self, event: ChoiceEvent, window: &mut Window, cx: &mut Context<Self>) {
        let (selected_index, selected_preset) = match event {
            ChoiceEvent::Selected(index) => (Some(index), None),
            ChoiceEvent::PresetSelected(preset) => (None, Some(preset)),
            ChoiceEvent::Dismissed => {
                self.dismiss_control(window, cx);
                return;
            }
        };
        let Some(open) = self.controls.open.as_ref() else {
            return;
        };
        let action = selected_index.and_then(|index| open.choices.get(index).cloned());
        let valid_context = self.selected == open.task
            && self.task().map(|task| &task.agent_id) == open.agent.as_ref()
            && self
                .details
                .as_ref()
                .and_then(|details| details.session_id.clone())
                == open.session
            && self.details.as_ref().map(|details| details.connection.id) == open.connection;
        let current = action.as_ref().and_then(|action| {
            self.control_choices(open.kind)
                .into_iter()
                .find(|(_, candidate)| candidate == action)
        });
        let task = open.task;
        let blocked = open.kind != ControlKind::Project && self.controls_blocked();
        self.dismiss_control(window, cx);
        if !valid_context || blocked {
            self.error = Some("Session choices changed. Open the selector again.".into());
            cx.notify();
            return;
        }
        if let Some(preset) = selected_preset {
            let Some(task) = task else {
                return;
            };
            let model_action = self.thread.as_ref().and_then(|thread| {
                let agent = self.task()?.agent_id.as_str();
                preset_model_action(&preset, agent, &thread.configuration)
            });
            let Some(model_action) = model_action else {
                self.error = Some(
                    "This preset is no longer advertised by the selected agent. No setting was changed."
                        .into(),
                );
                cx.notify();
                return;
            };
            self.apply_model_preset(task, preset, model_action, cx);
            return;
        }
        if current.is_none() {
            self.error = Some("Session choices changed. Open the selector again.".into());
            cx.notify();
            return;
        }
        let (choice, action) = current.unwrap();
        if choice.unavailable.is_some()
            || choice.selected
            || matches!(&action, ControlAction::Agent(agent) if self.task().is_some_and(|task| &task.agent_id == agent))
        {
            return;
        }
        match action {
            ControlAction::PlanMode => {
                let Some(task) = self.selected else {
                    return;
                };
                let on = self.mode_tasks.get(&task) != Some(&InteractionMode::Plan);
                self.set_interaction_mode(
                    task,
                    if on {
                        InteractionMode::Plan
                    } else {
                        InteractionMode::Default
                    },
                    cx,
                );
                return;
            }
            ControlAction::DebugMode => {
                let Some(task) = self.selected else {
                    return;
                };
                let on = self.mode_tasks.get(&task) != Some(&InteractionMode::Debug);
                self.set_interaction_mode(
                    task,
                    if on {
                        InteractionMode::Debug
                    } else {
                        InteractionMode::Default
                    },
                    cx,
                );
                return;
            }
            ControlAction::Project(id) => {
                self.navigate_project(id, cx);
                return;
            }
            ControlAction::BrowseWorkspace => {
                self.browse_workspace(cx);
                return;
            }
            ControlAction::AttachFiles => {
                self.choose_attachments(cx);
                return;
            }
            ControlAction::Goal => {
                self.composer.update(cx, |entry, cx| {
                    let text = entry.text().to_owned();
                    let objective = text
                        .strip_prefix("/synara/goal set")
                        .map(str::trim_start)
                        .unwrap_or(&text);
                    entry.set_text(format!("/synara/goal set {objective}"), cx);
                });
                self.focus_composer = true;
                cx.notify();
                return;
            }
            ControlAction::AttachWindow => {
                self.controls.extras_windows = true;
                self.discover_appsnap_menu(cx);
                self.open_control(ControlKind::Extras, window, cx);
                return;
            }
            ControlAction::AttachWindowIndex(index) => {
                self.capture_appsnap_window(index, cx);
                return;
            }
            ControlAction::ExtrasBack => {
                self.open_control(ControlKind::Extras, window, cx);
                return;
            }
            ControlAction::ImplementPlanNewThread => {
                self.implement_plan_in_new_thread(cx);
                return;
            }

            ControlAction::Connect => {
                self.connect("connect", cx);
                return;
            }
            ControlAction::AccessInfo | ControlAction::Unavailable => return,
            _ => {}
        }
        let Some(task) = task else {
            return;
        };
        self.apply_session_control(task, action, cx);
    }
    pub(in crate::shell) fn apply_session_control(
        &mut self,
        task: TaskId,
        action: ControlAction,
        cx: &mut Context<Self>,
    ) {
        self.controls.pending.insert(task);
        self.error = None;
        if let ControlAction::Mode(mode) = &action {
            self.controls.pending_mode = Some((task, mode.clone()));
        }
        let controller = self.controller.clone();
        self.job(async move {
            let result = match action {
                ControlAction::Project(_)
                | ControlAction::PlanMode
                | ControlAction::DebugMode
                | ControlAction::BrowseWorkspace
                | ControlAction::AttachFiles
                | ControlAction::AttachWindow
                | ControlAction::AttachWindowIndex(_)
                | ControlAction::ExtrasBack
                | ControlAction::ImplementPlanNewThread
                | ControlAction::Goal
                | ControlAction::Unavailable
                | ControlAction::AccessInfo
                | ControlAction::Connect => {
                    unreachable!("project controls are handled before session dispatch")
                }
                ControlAction::Agent(agent) => controller.switch_agent(task, agent).await.map(Some),
                ControlAction::Mode(mode) => {
                    // Apply to a live session only when it advertises the mode;
                    // the choice itself is per-thread state that persists and
                    // re-applies on session creation (upstream `runtimeMode`).
                    let advertised = controller
                        .details(task)
                        .await
                        .ok()
                        .flatten()
                        .is_some_and(|d| d.configuration.modes.iter().any(|m| m.id == mode));
                    if advertised {
                        controller.set_mode(task, mode.clone()).await?;
                    }
                    controller
                        .workspace
                        .save_task_runtime_mode(task, mode)
                        .await
                        .map(|_| None)
                }
                ControlAction::Model(model) => {
                    controller.set_model(task, model).await.map(|_| None)
                }
                ControlAction::Option(key, value) => {
                    controller.set_option(task, key, value).await.map(|_| None)
                }
            };
            let details = controller.details(task).await.ok().flatten();
            Ok(Update::ControlFinished {
                task,
                result,
                details,
            })
        });
        cx.notify();
    }
    fn apply_model_preset(
        &mut self,
        task: TaskId,
        preset: SessionModelPreset,
        model_action: Option<ControlAction>,
        cx: &mut Context<Self>,
    ) {
        self.controls.pending.insert(task);
        self.error = None;
        let controller = self.controller.clone();
        self.job(async move {
            let result = async {
                if let Some(action) = model_action {
                    match action {
                        ControlAction::Model(model) => controller.set_model(task, model).await?,
                        ControlAction::Option(option, value) => {
                            controller.set_option(task, option, value).await?
                        }
                        _ => unreachable!("validated model preset contains only model and effort"),
                    }
                }
                if preset.effort.is_some() {
                    let details = controller
                        .details(task)
                        .await?
                        .ok_or_else(|| WorkspaceError::Invalid("the agent session is no longer connected".into()))?;
                    let Some((action, should_apply)) =
                        advertised_effort_action(&preset, &details.configuration)
                    else {
                        return Err(WorkspaceError::Invalid(
                            "the selected model does not advertise this effort value; the model was selected, but the effort was left unchanged".into(),
                        ));
                    };
                    if should_apply {
                        match action {
                            ControlAction::Option(option, value) => {
                                controller.set_option(task, option, value).await?
                            }
                            _ => unreachable!("validated effort is a select option"),
                        }
                    }
                }
                Ok(None)
            }
            .await;
            let details = controller.details(task).await.ok().flatten();
            Ok(Update::ControlFinished {
                task,
                result,
                details,
            })
        });
        cx.notify();
    }
    pub(super) fn control_trigger(
        &self,
        kind: ControlKind,
        label: String,
        available: bool,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let trigger = &self.controls.triggers[kind.index()];
        let bounds = trigger.bounds.clone();
        let redraw = cx.entity().downgrade();
        let disabled = (kind != ControlKind::Project && self.controls_blocked()) || !available;
        let model_alias =
            kind == ControlKind::Agent && !self.control_choices(ControlKind::Model).is_empty();
        ui::button_shell(kind.id(), kind.title(), false).flex().items_center().gap(px(6.)).relative().track_focus(&trigger.focus)
            .aria_label(kind.title()).accessibility_id(kind.id()).when(disabled, |el| el.aria_description("Unavailable while the session is busy, or when no choices are advertised"))
            .text_size(px(14.)).max_w(px(320.)).min_w_0().text_ellipsis().px_2().py_1()
            .bg(gpui::rgba(0)).when(disabled, |el| el.opacity(0.5).cursor_default())
            .when(kind == ControlKind::Extras, |el| el.size(px(30.)).p_0().justify_center())
            .on_click(cx.listener(move |this, _, window, cx| this.open_control(kind, window, cx)))
            .children(match kind {
                ControlKind::Project => Some(ui::icon(ui::Glyph::Folder)),
                ControlKind::Agent => Some(ui::icon(self.selected_agent_glyph())),
                ControlKind::Extras => Some(ui::icon(ui::Glyph::Plus).size(px(17.))),
                _ => None,
            })
            .children((kind != ControlKind::Extras).then(|| div().min_w_0().text_ellipsis().child(label)))
            .children((kind != ControlKind::Extras && kind != ControlKind::Project).then(|| ui::icon(ui::Glyph::Chevron).size(px(11.))))
            .child(canvas(move |new_bounds, _, cx| {
                if bounds.get() != new_bounds {
                    bounds.set(new_bounds);
                    let _ = redraw.update(cx, |this, cx| { if this.controls.is_open() { cx.notify(); } });
                }
                for control in std::iter::once(kind.id()).chain(model_alias.then_some("model-picker")) {
                    tracing::debug!(target: "synara_ui_layout", control, enabled = !disabled, x = f32::from(new_bounds.origin.x), y = f32::from(new_bounds.origin.y), width = f32::from(new_bounds.size.width), height = f32::from(new_bounds.size.height), "control-layout");
                }
            }, |_, _, _, _| {}).absolute().size_full().top_0().left_0())
            .into_any_element()
    }
    pub(super) fn configuration_controls(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        div()
            .flex()
            .gap_2()
            .child(
                ui::button("connect-agent", "Connect and load models", false)
                    .on_click(cx.listener(|this, _, _, cx| this.connect("connect", cx))),
            )
            .child(self.control_trigger(
                ControlKind::Mode,
                "Session mode".into(),
                !self.control_choices(ControlKind::Mode).is_empty(),
                cx,
            ))
            .child(self.control_trigger(
                ControlKind::Options,
                "Session options".into(),
                !self.control_choices(ControlKind::Options).is_empty(),
                cx,
            ))
            .into_any_element()
    }

    pub(super) fn session_controls(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let models = self.control_choices(ControlKind::Model);
        let label = models
            .iter()
            .find(|(choice, _)| choice.selected)
            .map(|(choice, _)| choice.label.clone())
            .or_else(|| {
                self.profiles
                    .iter()
                    .find(|profile| self.task().is_some_and(|task| task.agent_id == profile.id))
                    .map(|profile| profile.name.clone())
            })
            .unwrap_or_else(|| "Choose model".into());
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_1()
            .min_w_0()
            .child(self.control_trigger(ControlKind::Extras, String::new(), true, cx))
            .child(self.control_trigger(
                ControlKind::Access,
                // Upstream shows the thread's persisted `runtimeMode` label on
                // the composer button even before a session exists.
                if self.control_choices(ControlKind::Mode).is_empty() {
                    self.task()
                        .and_then(|task| self.catalog.runtime_modes.get(&task.id))
                        .map(|mode| {
                            access_mode_copy(mode)
                                .map(|(label, _)| label.to_owned())
                                .unwrap_or_else(|| mode.clone())
                        })
                        .unwrap_or_else(|| "Ask for approval".into())
                } else {
                    self.control_choices(ControlKind::Access)
                        .iter()
                        .find(|(choice, _)| choice.selected)
                        .map_or_else(
                            || "Ask for approval".into(),
                            |(choice, _)| choice.label.clone(),
                        )
                },
                true,
                cx,
            ))
            .children(self.context_meter(cx))
            // Upstream `sidebarAction`: "Plan details"/"Tasks" sidebar toggle.
            .children(self.plan_sidebar_toggle(cx))
            .child(div().flex_1())
            .child(self.control_trigger(ControlKind::Agent, label, !self.profiles.is_empty(), cx))
            .into_any_element()
    }

    pub(super) fn project_picker(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let empty = self
            .thread
            .as_ref()
            .is_none_or(|thread| thread.timeline.is_empty());
        let label = if empty {
            "Work in a project".to_owned()
        } else {
            self.catalog
                .projects
                .iter()
                .find(|project| Some(project.id) == self.project)
                .map_or_else(
                    || "Work in a project".to_owned(),
                    |project| self.project_name(project).to_string(),
                )
        };
        self.control_trigger(ControlKind::Project, label, true, cx)
    }

    pub(super) fn control_overlay(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(open) = &self.controls.open else {
            return div().into_any_element();
        };
        let is_add = matches!(open.kind, ControlKind::Extras | ControlKind::PlanImplement);
        let bounds = if is_add {
            self.controls.composer_bounds.get()
        } else {
            self.controls.triggers[open.kind.index()].bounds.get()
        };
        div()
            .id("session-choice-backdrop")
            .absolute()
            .size_full()
            .top_0()
            .left_0()
            .occlude()
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    this.dismiss_control(window, cx);
                    cx.stop_propagation();
                }),
            )
            .child(
                gpui::anchored()
                    .anchor(gpui::Anchor::BottomLeft)
                    .position(bounds.origin)
                    .offset(point(
                        px(if is_add { 4. } else { 0. }),
                        px(if is_add { -8. } else { -6. }),
                    ))
                    .snap_to_window_with_margin(px(8.))
                    .child(open.view.clone()),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn configuration() -> SessionConfiguration {
        SessionConfiguration {
            current_model: Some("legacy".into()),
            models: vec![SelectChoice {
                value: "legacy".into(),
                label: "Legacy".into(),
                group: None,
            }],
            options: vec![SessionOption {
                id: "model-choice".into(),
                name: "Model".into(),
                description: None,
                category: Some("model".into()),
                current: ConfigValue::Select {
                    value: "second".into(),
                },
                choices: vec![
                    SelectChoice {
                        value: "first".into(),
                        label: "First".into(),
                        group: None,
                    },
                    SelectChoice {
                        value: "second".into(),
                        label: "Second".into(),
                        group: None,
                    },
                ],
            }],
            ..Default::default()
        }
    }
    #[test]
    fn options_override_legacy_models_and_preserve_actual_values() {
        let config = configuration();
        let choices = session_choices(&config, ControlKind::Model);
        assert_eq!(choices.len(), 2);
        assert!(!choices[0].0.selected);
        assert!(choices[1].0.selected);
        assert!(
            matches!(&choices[1].1, ControlAction::Option(id, ConfigValue::Select { value }) if id == "model-choice" && value == "second")
        );
        assert_eq!(config.current_model.as_deref(), Some("legacy"));
    }
    #[test]
    fn empty_advertised_category_does_not_invent_a_legacy_fallback() {
        let mut config = configuration();
        config.options[0].choices.clear();
        assert!(session_choices(&config, ControlKind::Model).is_empty());
    }
    #[test]
    fn saved_preset_captures_only_live_selected_model_and_explicit_effort() {
        let mut config = configuration();
        config.options.push(SessionOption {
            id: "reasoning_effort".into(),
            name: "Reasoning effort".into(),
            description: None,
            category: Some("reasoning_effort".into()),
            current: ConfigValue::Select {
                value: "high".into(),
            },
            choices: ["low", "high"]
                .into_iter()
                .map(|value| SelectChoice {
                    value: value.into(),
                    label: value.into(),
                    group: None,
                })
                .collect(),
        });
        let preset = current_model_preset("codex", &config).unwrap();
        assert_eq!(preset.agent, "codex");
        assert_eq!(preset.model_option.as_deref(), Some("model-choice"));
        assert_eq!(preset.model, "second");
        assert_eq!(preset.effort_option.as_deref(), Some("reasoning_effort"));
        assert_eq!(preset.effort.as_deref(), Some("high"));
        assert!(preset_model_action(&preset, "other-agent", &config).is_none());
        assert_eq!(preset_model_action(&preset, "codex", &config), Some(None));
        assert_eq!(
            advertised_effort_action(&preset, &config),
            Some((
                ControlAction::Option(
                    "reasoning_effort".into(),
                    ConfigValue::Select {
                        value: "high".into(),
                    },
                ),
                false,
            ))
        );

        config.options[1].current = ConfigValue::Select {
            value: "low".into(),
        };
        config.options[0].current = ConfigValue::Select {
            value: "first".into(),
        };
        assert_eq!(
            preset_model_action(&preset, "codex", &config),
            Some(Some(ControlAction::Option(
                "model-choice".into(),
                ConfigValue::Select {
                    value: "second".into(),
                },
            )))
        );
        assert_eq!(
            advertised_effort_action(&preset, &config),
            Some((
                ControlAction::Option(
                    "reasoning_effort".into(),
                    ConfigValue::Select {
                        value: "high".into(),
                    },
                ),
                true,
            ))
        );
        config.options[1]
            .choices
            .retain(|choice| choice.value != "high");
        assert!(advertised_effort_action(&preset, &config).is_none());
    }
    #[test]
    fn preset_does_not_infer_effort_from_unrelated_options() {
        let mut config = configuration();
        config.options.push(SessionOption {
            id: "thinking".into(),
            name: "Thought level".into(),
            description: None,
            category: Some("mode".into()),
            current: ConfigValue::Select {
                value: "deep".into(),
            },
            choices: vec![SelectChoice {
                value: "deep".into(),
                label: "Deep".into(),
                group: None,
            }],
        });
        let preset = current_model_preset("codex", &config).unwrap();
        assert!(preset.effort_option.is_none());
        assert!(preset.effort.is_none());
    }
    #[test]
    fn provider_order_uses_preferences_then_registry_order_for_new_entries() {
        let profile = |id: &str| AgentProfile {
            registry: None,
            id: id.into(),
            name: id.into(),
            command: id.into(),
            args: vec![],
            inherit_env: vec![],
            secret_env: Default::default(),
        };
        let profiles = vec![profile("codex"), profile("claude"), profile("custom")];
        let order = ordered_profiles(&profiles, &["claude".into(), "codex".into()]);
        assert_eq!(
            order
                .iter()
                .map(|profile| profile.id.as_str())
                .collect::<Vec<_>>(),
            vec!["claude", "codex", "custom"]
        );
    }
    #[test]
    fn boolean_options_offer_explicit_values_not_a_cycle() {
        let mut config = SessionConfiguration::default();
        config.options.push(SessionOption {
            id: "review".into(),
            name: "Review first".into(),
            description: None,
            category: None,
            current: ConfigValue::Boolean { value: true },
            choices: vec![],
        });
        let choices = session_choices(&config, ControlKind::Options);
        assert_eq!(choices.len(), 2);
        assert!(choices[0].0.selected);
        assert!(!choices[1].0.selected);
        assert!(
            matches!(&choices[1].1, ControlAction::Option(id, ConfigValue::Boolean { value: false }) if id == "review")
        );
    }

    #[test]
    fn model_cycle_uses_advertised_order_and_wraps() {
        let config = SessionConfiguration {
            current_model: Some("second".into()),
            models: ["first", "second", "third"]
                .into_iter()
                .map(|value| SelectChoice {
                    value: value.into(),
                    label: value.into(),
                    group: None,
                })
                .collect(),
            ..Default::default()
        };
        let choices = session_choices(&config, ControlKind::Model);
        assert_eq!(
            cycle_model_action(&choices, true),
            Some(ControlAction::Model("third".into()))
        );
        assert_eq!(
            cycle_model_action(&choices, false),
            Some(ControlAction::Model("first".into()))
        );

        let mut last_selected = config;
        last_selected.current_model = Some("third".into());
        let choices = session_choices(&last_selected, ControlKind::Model);
        assert_eq!(
            cycle_model_action(&choices, true),
            Some(ControlAction::Model("first".into()))
        );
    }

    #[test]
    fn model_cycle_applies_only_provider_advertised_select_options() {
        let config = SessionConfiguration {
            options: vec![SessionOption {
                id: "model-choice".into(),
                name: "Model".into(),
                description: None,
                category: Some("model".into()),
                current: ConfigValue::Select {
                    value: "provider-b".into(),
                },
                choices: ["provider-a", "provider-b", "provider-c"]
                    .into_iter()
                    .map(|value| SelectChoice {
                        value: value.into(),
                        label: value.into(),
                        group: None,
                    })
                    .collect(),
            }],
            ..Default::default()
        };
        let choices = session_choices(&config, ControlKind::Model);
        assert_eq!(
            cycle_model_action(&choices, true),
            Some(ControlAction::Option(
                "model-choice".into(),
                ConfigValue::Select {
                    value: "provider-c".into()
                }
            ))
        );
        assert_eq!(
            cycle_model_action(&choices, false),
            Some(ControlAction::Option(
                "model-choice".into(),
                ConfigValue::Select {
                    value: "provider-a".into()
                }
            ))
        );
    }

    #[test]
    fn model_cycle_is_absent_without_two_advertised_models() {
        assert!(cycle_model_action(&[], true).is_none());
        let mut config = configuration();
        config.options[0].choices.truncate(1);
        let choices = session_choices(&config, ControlKind::Model);
        assert!(cycle_model_action(&choices, true).is_none());
    }

    #[test]
    fn model_cycle_does_not_merge_distinct_provider_selectors() {
        let mut config = SessionConfiguration::default();
        for (id, prefix) in [("model-a", "a"), ("model-b", "b")] {
            let values = [format!("{prefix}1"), format!("{prefix}2")];
            config.options.push(SessionOption {
                id: id.into(),
                name: id.into(),
                description: None,
                category: Some("model".into()),
                current: ConfigValue::Select {
                    value: values[0].clone(),
                },
                choices: values
                    .into_iter()
                    .map(|value| SelectChoice {
                        label: value.clone(),
                        value,
                        group: None,
                    })
                    .collect(),
            });
        }
        let choices = session_choices(&config, ControlKind::Model);
        assert!(cycle_model_action(&choices, true).is_none());
    }
}
