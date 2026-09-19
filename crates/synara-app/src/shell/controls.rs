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
    Agent,
    Mode,
    Model,
    Options,
}
impl ControlKind {
    fn index(self) -> usize {
        match self {
            Self::Agent => 0,
            Self::Mode => 1,
            Self::Model => 2,
            Self::Options => 3,
        }
    }
    fn title(self) -> &'static str {
        match self {
            Self::Agent => "Coding agent",
            Self::Mode => "Session mode",
            Self::Model => "Model",
            Self::Options => "Session options",
        }
    }
    fn id(self) -> &'static str {
        match self {
            Self::Agent => "agent-picker",
            Self::Mode => "mode-picker",
            Self::Model => "model-picker",
            Self::Options => "options-picker",
        }
    }
}
#[derive(Clone, PartialEq)]
enum ControlAction {
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
    task: TaskId,
    agent: String,
    session: Option<String>,
    connection: Option<ConnectionId>,
    kind: ControlKind,
    choices: Vec<ControlAction>,
    view: Entity<ChoiceMenu>,
    _subscription: Subscription,
}
pub(super) struct ControlState {
    open: Option<OpenControl>,
    triggers: [Trigger; 4],
    pending: HashSet<TaskId>,
}
impl ControlState {
    pub fn new(cx: &mut Context<Shell>) -> Self {
        Self {
            open: None,
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
                },
                ControlAction::Model(model.value.clone()),
            )
        }));
    }
    result
}

impl Shell {
    fn control_choices(&self, kind: ControlKind) -> Vec<(Choice, ControlAction)> {
        if kind == ControlKind::Agent {
            return self
                .profiles
                .iter()
                .map(|profile| {
                    (
                        Choice {
                            label: profile.name.clone(),
                            detail: String::new(),
                            selected: self.task().is_some_and(|task| task.agent_id == profile.id),
                        },
                        ControlAction::Agent(profile.id.clone()),
                    )
                })
                .collect();
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
        if self.controls_blocked() {
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
        let Some(task) = self.task().cloned() else {
            return;
        };
        let (items, choices): (Vec<_>, Vec<_>) = self.control_choices(kind).into_iter().unzip();
        if items.is_empty() {
            return;
        }
        self.navigation.menu_open = false;
        let view = cx.new(|cx| ChoiceMenu::new(kind.title().into(), items, cx));
        let subscription = cx.subscribe_in(&view, window, |this, _, event, window, cx| {
            this.control_event(*event, window, cx);
        });
        window.focus(&view.read(cx).focus_handle(cx), cx);
        self.controls.open = Some(OpenControl {
            task: task.id,
            agent: task.agent_id,
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
        let valid_context = self.selected == Some(open.task)
            && self.task().is_some_and(|task| task.agent_id == open.agent)
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
        let blocked = self.controls_blocked();
        self.dismiss_control(window, cx);
        if !valid_context || blocked || current.is_none() {
            self.error = Some("Session choices changed. Open the selector again.".into());
            cx.notify();
            return;
        }
        let (choice, action) = current.unwrap();
        if choice.selected {
            return;
        }
        self.controls.pending.insert(task);
        self.error = None;
        let controller = self.controller.clone();
        self.job(async move {
            let result = match action {
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
    fn control_trigger(
        &self,
        kind: ControlKind,
        label: String,
        available: bool,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let trigger = &self.controls.triggers[kind.index()];
        let bounds = trigger.bounds.clone();
        let redraw = cx.entity().downgrade();
        let disabled = self.controls_blocked() || !available;
        ui::button(kind.id(), label, false).flex().items_center().gap_1().relative().track_focus(&trigger.focus)
            .aria_label(kind.title()).accessibility_id(kind.id()).when(disabled, |el| el.aria_description("Unavailable while the session is busy, or when no choices are advertised"))
            .text_size(px(12.)).max_w(px(215.)).min_w_0().text_ellipsis()
            .bg(gpui::rgba(0)).when(disabled, |el| el.opacity(0.5).cursor_default())
            .on_click(cx.listener(move |this, _, window, cx| this.open_control(kind, window, cx)))
            .child(ui::icon(ui::Glyph::Chevron))
            .child(canvas(move |new_bounds, _, cx| {
                if bounds.get() != new_bounds {
                    bounds.set(new_bounds);
                    let _ = redraw.update(cx, |this, cx| {
                        if this.controls.is_open() { cx.notify(); }
                    });
                    tracing::debug!(target: "synara_ui_layout", control = kind.id(), x = f32::from(new_bounds.origin.x), y = f32::from(new_bounds.origin.y), width = f32::from(new_bounds.size.width), height = f32::from(new_bounds.size.height), "control-layout");
                }
            }, |_, _, _, _| {}).absolute().size_full().top_0().left_0())
            .into_any_element()
    }
    pub(super) fn session_controls(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let modes = self.control_choices(ControlKind::Mode);
        let agents = self.control_choices(ControlKind::Agent);
        let models = self.control_choices(ControlKind::Model);
        let options = self.control_choices(ControlKind::Options);
        let label = |choices: &[(Choice, ControlAction)], fallback: &str| {
            choices
                .iter()
                .find(|(choice, _)| choice.selected)
                .map_or_else(|| fallback.to_owned(), |(choice, _)| choice.label.clone())
        };
        let connected = self
            .details
            .as_ref()
            .is_some_and(|details| details.connection.state == ConnectionState::Connected);
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_1()
            .min_w_0()
            .children(
                (!modes.is_empty()).then(|| {
                    self.control_trigger(ControlKind::Mode, label(&modes, "Mode"), true, cx)
                }),
            )
            .child(self.control_trigger(
                ControlKind::Agent,
                label(&agents, "Choose agent"),
                !agents.is_empty(),
                cx,
            ))
            .children(connected.then(|| {
                self.control_trigger(
                    ControlKind::Model,
                    label(&models, "No models advertised"),
                    !models.is_empty(),
                    cx,
                )
            }))
            .children(
                (!options.is_empty()).then(|| {
                    self.control_trigger(ControlKind::Options, "Options".into(), true, cx)
                }),
            )
            .children((!connected).then(|| {
                let connecting = self
                    .selected
                    .is_some_and(|task| self.connecting.contains(&task));
                ui::button(
                    "connect-agent",
                    if connecting {
                        "Connecting..."
                    } else {
                        "Connect"
                    },
                    false,
                )
                .text_size(px(12.))
                .when(self.controls_blocked(), |el| {
                    el.aria_description("A session operation is in progress")
                })
                .on_click(cx.listener(|this, _, _, cx| {
                    if !this.controls_blocked() {
                        this.connect("connect", cx);
                    }
                }))
            }))
            .into_any_element()
    }
    pub(super) fn control_overlay(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(open) = &self.controls.open else {
            return div().into_any_element();
        };
        let bounds = self.controls.triggers[open.kind.index()].bounds.get();
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
                    .offset(point(px(0.), px(-6.)))
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
