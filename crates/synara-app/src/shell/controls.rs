//! Presentation-only session pickers. The controller owns every applied value.
use super::*;
use crate::ui::{
    self,
    menu::{Choice, ChoiceEvent, ChoiceMenu},
};
use gpui::{Bounds, FocusHandle, Pixels, canvas, point};
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ControlKind {
    Project,
    Agent,
    Mode,
    Model,
    Options,
    Extras,
    Access,
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
        }
    }
}
#[derive(Clone, PartialEq)]
enum ControlAction {
    DebugWorkflow,
    Project(ProjectId),
    BrowseWorkspace,
    AddReferences,
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
    triggers: [Trigger; 7],
    pub composer_bounds: Rc<Cell<Bounds<Pixels>>>,
    pending: HashSet<TaskId>,
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
    }
    pub fn completed(&mut self, task: TaskId) {
        self.pending.remove(&task);
    }
}

fn option_kind(option: &SessionOption) -> ControlKind {
    match option.category.as_deref() {
        Some("model") => ControlKind::Model,
        Some("mode") => ControlKind::Mode,
        _ => ControlKind::Options,
    }
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

impl Shell {
    fn control_choices(&self, kind: ControlKind) -> Vec<(Choice, ControlAction)> {
        if kind == ControlKind::Access {
            return vec![(
                Choice {
                    label: "Ask permission".into(),
                    detail: "Agent approval requests require your confirmation.".into(),
                    selected: true,
                    ..Default::default()
                },
                ControlAction::AccessInfo,
            )];
        }
        if kind == ControlKind::Extras {
            let modes = self.control_choices(ControlKind::Mode);
            let plan = modes
                .iter()
                .find(|(choice, _)| choice.label.eq_ignore_ascii_case("plan"));
            let in_plan = plan.is_some_and(|(choice, _)| choice.selected);
            let target = if in_plan {
                modes
                    .iter()
                    .find(|(choice, _)| !choice.label.eq_ignore_ascii_case("plan"))
            } else {
                plan
            };
            let mut rows = vec![
                (
                    Choice {
                        label: "Files and folders".into(),
                        icon: Some(ui::Glyph::Attach),
                        unavailable: (!matches!(
                            self.workspace_target(),
                            Some(WorkspaceTarget::Local { .. })
                        ))
                        .then(|| "Choose a local project to add file references.".into()),
                        ..Default::default()
                    },
                    ControlAction::AddReferences,
                ),
                (
                    Choice {
                        label: "Attach window".into(),
                        detail: "Capture an open app window".into(),
                        icon: Some(ui::Glyph::Window),
                        unavailable: Some(
                            "Window capture is not available in this native build yet.".into(),
                        ),
                        ..Default::default()
                    },
                    ControlAction::Unavailable,
                ),
                (
                    Choice {
                        label: "Goal".into(),
                        detail: "Set a goal to keep pursuing".into(),
                        icon: Some(ui::Glyph::Goal),
                        unavailable: Some(
                            "Goals are not available in this native build yet.".into(),
                        ),
                        ..Default::default()
                    },
                    ControlAction::Unavailable,
                ),
            ];
            rows.push((
                Choice {
                    label: "Plan mode".into(),
                    detail: format!("Turn plan mode {}", if in_plan { "off" } else { "on" }),
                    icon: Some(ui::Glyph::Plan),
                    unavailable: target
                        .is_none()
                        .then(|| "Connect an agent that advertises a plan mode.".into()),
                    ..Default::default()
                },
                target.map_or(ControlAction::Unavailable, |(_, action)| action.clone()),
            ));
            rows.push((
                Choice {
                    label: "Debug mode".into(),
                    detail: "Observe, reproduce, investigate, fix and verify with saved evidence"
                        .into(),
                    icon: Some(ui::Glyph::Debug),
                    ..Default::default()
                },
                ControlAction::DebugWorkflow,
            ));
            return rows;
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
                            label: project.name.clone(),
                            detail: String::new(),
                            selected: self.project == Some(project.id),
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
            let mut choices: Vec<_> = self
                .profiles
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
    pub(super) fn dismiss_control(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(open) = self.controls.open.take() {
            window.focus(&self.controls.triggers[open.kind.index()].focus, cx);
            cx.notify();
        }
    }
    fn open_control(&mut self, kind: ControlKind, window: &mut Window, cx: &mut Context<Self>) {
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
            if kind == ControlKind::Extras {
                view.add_layout(self.controls.composer_bounds.clone())
            } else if kind == ControlKind::Agent {
                let current = self
                    .profiles
                    .iter()
                    .position(|p| task.as_ref().is_some_and(|t| t.agent_id == p.id))
                    .unwrap_or(0);
                let sources = self
                    .profiles
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
                            ControlAction::Agent(id) => self
                                .profiles
                                .iter()
                                .position(|p| &p.id == id)
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
                        ui::menu::ModelRow { source, favorite }
                    })
                    .collect();
                view.with_models(
                    sources,
                    rows,
                    current,
                    self.controller.workspace.clone(),
                    self.runtime.clone(),
                    cx,
                )
            } else {
                view
            }
        });
        let subscription = cx.subscribe_in(&view, window, |this, _, event, window, cx| {
            this.control_event(*event, window, cx);
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
        let ChoiceEvent::Selected(index) = event else {
            self.dismiss_control(window, cx);
            return;
        };
        let Some(open) = self.controls.open.as_ref() else {
            return;
        };
        let action = open.choices.get(index).cloned();
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
        if !valid_context || blocked || current.is_none() {
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
            ControlAction::DebugWorkflow => {
                self.open_debug(cx);
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
            ControlAction::AddReferences => {
                self.add_file_references(cx);
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
    pub(in crate::shell) fn native_plan_mode(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(task) = self.selected else {
            return false;
        };
        if self.controls_blocked() || self.uses_direct_model() {
            self.error = Some(
                "Plan mode requires an idle connected ACP session advertising that mode.".into(),
            );
            return false;
        }
        let Some((choice, action)) = self
            .control_choices(ControlKind::Mode)
            .into_iter()
            .find(|(choice, _)| choice.label.eq_ignore_ascii_case("plan"))
        else {
            self.error = Some(
                "This session does not advertise a Plan mode. No mode or prompt was changed."
                    .into(),
            );
            return false;
        };
        if let Some(reason) = choice.unavailable {
            self.error = Some(reason);
            return false;
        }
        if !choice.selected {
            self.apply_session_control(task, action, cx);
        }
        true
    }
    fn apply_session_control(
        &mut self,
        task: TaskId,
        action: ControlAction,
        cx: &mut Context<Self>,
    ) {
        self.controls.pending.insert(task);
        self.error = None;
        let controller = self.controller.clone();
        self.job(async move {
            let result = match action {
                ControlAction::Project(_)
                | ControlAction::DebugWorkflow
                | ControlAction::BrowseWorkspace
                | ControlAction::AddReferences
                | ControlAction::Unavailable
                | ControlAction::AccessInfo
                | ControlAction::Connect => {
                    unreachable!("project controls are handled before session dispatch")
                }
                ControlAction::Agent(agent) => controller.switch_agent(task, agent).await.map(Some),
                ControlAction::Mode(mode) => controller.set_mode(task, mode).await.map(|_| None),
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
            .when(kind == ControlKind::Access, |el| el.text_color(rgb(0xf0844b)))
            .when(kind == ControlKind::Extras, |el| el.size(px(30.)).p_0().justify_center())
            .on_click(cx.listener(move |this, _, window, cx| this.open_control(kind, window, cx)))
            .children(match kind {
                ControlKind::Project => Some(ui::icon(ui::Glyph::Folder)),
                ControlKind::Agent => Some(ui::icon(self.selected_agent_glyph())),
                ControlKind::Access => Some(ui::icon(ui::Glyph::Shield).text_color(rgb(0xf0844b))),
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

    // The ACP text prompt supports workspace references; a picker never sends
    // content or starts a turn. Recheck task/root after the asynchronous dialog.
    fn add_file_references(&mut self, cx: &mut Context<Self>) {
        let Some(WorkspaceTarget::Local { root }) = self.workspace_target() else {
            return;
        };
        let selected = self.selected;
        let picker = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: true,
            multiple: true,
            prompt: Some("Add workspace references".into()),
        });
        cx.spawn(async move |view, cx| {
            let result = picker.await;
            let _ = view.update(cx, |this, cx| {
                if this.selected != selected || !matches!(this.workspace_target(), Some(WorkspaceTarget::Local { root: current }) if current == root) { return; }
                match result {
                    Ok(Ok(Some(paths))) => {
                        let root = root.canonicalize();
                        let references: Result<Vec<_>, _> = paths.into_iter().map(|path| {
                            let path = path.canonicalize().map_err(|_| ())?;
                            let relative = path.strip_prefix(root.as_ref().map_err(|_| ())?).map_err(|_| ())?;
                            Ok::<_, ()>(format!("@{}", relative.display()))
                        }).collect();
                        match references {
                            Ok(references) => this.composer.update(cx, |entry, cx| {
                                let mut text = entry.text().to_owned();
                                if !text.is_empty() && !text.ends_with(char::is_whitespace) { text.push(' '); }
                                text.push_str(&references.join(" "));
                                entry.set_text(text, cx);
                            }),
                            Err(()) => this.error = Some("Choose files or folders inside the current project.".into()),
                        }
                    }
                    Ok(Ok(None)) => {},
                    _ => this.error = Some("The system file picker could not open. Type a workspace path in the message instead.".into()),
                }
                this.snapshot_draft(cx);
                cx.notify();
            });
        }).detach();
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
            .child(self.control_trigger(ControlKind::Access, "Ask permission".into(), true, cx))
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
                    |project| project.name.clone(),
                )
        };
        self.control_trigger(ControlKind::Project, label, true, cx)
    }

    pub(super) fn control_overlay(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(open) = &self.controls.open else {
            return div().into_any_element();
        };
        let is_add = open.kind == ControlKind::Extras;
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
}
