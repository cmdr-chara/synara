//! An isolated native task composer. Opening and editing never start an agent.
use super::*;
use crate::input::{EntryEvent, EntryMode, TextEntry};
use gpui::{Entity, EntityInputHandler, EventEmitter, FocusHandle, Focusable, Subscription};
use synara_core::ProjectId;

pub struct NewTaskRequest {
    pub project: ProjectId,
    pub agent: String,
    pub text: String,
    pub send: bool,
}
pub enum TaskDialogEvent {
    Create(NewTaskRequest),
    Dismissed,
}
#[derive(Clone, Copy)]
enum Field {
    Project,
    Agent,
}

pub struct TaskDialogConfig {
    pub projects: Vec<(ProjectId, String)>,
    pub agents: Vec<(String, String, Glyph)>,
    pub initial_project: Option<ProjectId>,
    pub default_agent: Option<String>,
    pub draft: bool,
    pub send_on_enter: bool,
}
pub struct TaskDialog {
    prompt: Entity<TextEntry>,
    projects: Vec<(ProjectId, String)>,
    agents: Vec<(String, String, Glyph)>,
    project: usize,
    agent: usize,
    draft: bool,
    busy: bool,
    discard: bool,
    error: Option<String>,
    popup: Option<Entity<menu::ChoiceMenu>>,
    popup_subscription: Option<Subscription>,
    needs_focus: bool,
    _prompt_subscription: Subscription,
}
impl EventEmitter<TaskDialogEvent> for TaskDialog {}
impl Focusable for TaskDialog {
    fn focus_handle(&self, cx: &gpui::App) -> FocusHandle {
        self.prompt.read(cx).focus_handle(cx)
    }
}
impl TaskDialog {
    pub fn new(config: TaskDialogConfig, cx: &mut Context<Self>) -> Self {
        let TaskDialogConfig {
            projects,
            agents,
            initial_project,
            default_agent,
            draft,
            send_on_enter,
        } = config;
        let prompt = cx.new(|cx| {
            let mut entry = TextEntry::new("Describe the task...", EntryMode::Composer, 112., cx);
            entry.set_send_on_enter(send_on_enter);
            entry
        });
        let subscription = cx.subscribe(&prompt, |this, _, event, cx| {
            if matches!(event, EntryEvent::Submit) && this.popup.is_none() && !this.discard {
                this.create(cx);
            }
            cx.notify();
        });
        let project = projects
            .iter()
            .position(|(id, _)| Some(*id) == initial_project)
            .unwrap_or(0);
        let agent = agents
            .iter()
            .position(|(id, _, _)| Some(id.as_str()) == default_agent.as_deref())
            .unwrap_or(0);
        Self {
            prompt,
            projects,
            agents,
            project,
            agent,
            draft,
            busy: false,
            discard: false,
            error: None,
            popup: None,
            popup_subscription: None,
            needs_focus: true,
            _prompt_subscription: subscription,
        }
    }
    pub fn failed(&mut self, error: String, cx: &mut Context<Self>) {
        self.busy = false;
        self.error = Some(error);
        self.needs_focus = true;
        cx.notify();
    }
    pub fn has_text(&self, cx: &gpui::App) -> bool {
        !self.prompt.read(cx).text().is_empty()
    }
    fn create(&mut self, cx: &mut Context<Self>) {
        if self.busy || self.discard || self.prompt.read(cx).text().trim().is_empty() {
            return;
        }
        let (Some((project, _)), Some((agent, _, _))) =
            (self.projects.get(self.project), self.agents.get(self.agent))
        else {
            return;
        };
        let request = NewTaskRequest {
            project: *project,
            agent: agent.clone(),
            text: self.prompt.read(cx).text().to_owned(),
            send: !self.draft,
        };
        self.busy = true;
        self.error = None;
        self.popup = None;
        cx.emit(TaskDialogEvent::Create(request));
        cx.notify();
    }
    fn dismiss(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        if self.has_text(cx) {
            self.discard = true;
            self.popup = None;
            cx.notify();
        } else {
            cx.emit(TaskDialogEvent::Dismissed);
        }
    }
    fn open_field(&mut self, field: Field, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy || self.discard {
            return;
        }
        let choices = match field {
            Field::Project => self
                .projects
                .iter()
                .enumerate()
                .map(|(index, (_, name))| menu::Choice {
                    label: name.clone(),
                    selected: index == self.project,
                    icon: Some(Glyph::Folder),
                    ..Default::default()
                })
                .collect(),
            Field::Agent => self
                .agents
                .iter()
                .enumerate()
                .map(|(index, (_, name, icon))| menu::Choice {
                    label: name.clone(),
                    selected: index == self.agent,
                    icon: Some(*icon),
                    detail: "Use this agent's default model. Configure models in the task.".into(),
                    ..Default::default()
                })
                .collect(),
        };
        let popup = cx.new(|cx| {
            menu::ChoiceMenu::new(
                match field {
                    Field::Project => "Project",
                    Field::Agent => "Agent",
                }
                .into(),
                choices,
                cx,
            )
        });
        self.popup_subscription = Some(cx.subscribe(&popup, move |this, _, event, cx| {
            if let menu::ChoiceEvent::Selected(index) = event {
                match field {
                    Field::Project if *index < this.projects.len() => this.project = *index,
                    Field::Agent if *index < this.agents.len() => this.agent = *index,
                    _ => {}
                }
            }
            this.popup = None;
            this.needs_focus = true;
            cx.notify();
        }));
        window.focus(&popup.read(cx).focus_handle(cx), cx);
        self.popup = Some(popup);
        cx.notify();
    }
}
impl gpui::Render for TaskDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.needs_focus && self.popup.is_none() {
            window.focus(&self.prompt.read(cx).focus_handle(cx), cx);
            self.needs_focus = false;
        }
        let can_create = !self.busy
            && !self.discard
            && !self.projects.is_empty()
            && !self.agents.is_empty()
            && !self.prompt.read(cx).text().trim().is_empty();
        let project = self
            .projects
            .get(self.project)
            .map_or("Choose project", |(_, name)| name.as_str());
        let (agent, glyph) = self
            .agents
            .get(self.agent)
            .map_or(("No configured agent", Glyph::Agent), |(_, name, glyph)| {
                (name.as_str(), *glyph)
            });
        let modal = div().id("new-task-dialog").role(gpui::Role::Dialog).aria_label("New task")
            .tab_group().relative().w_full().max_w(px(768.)).rounded(px(22.))
            .border_1().border_color(rgb(palette().border)).bg(rgb(palette().overlay)).shadow_lg().occlude()
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .capture_key_down(cx.listener(|this, _, _, cx| { if this.busy { cx.stop_propagation(); } }))
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                if event.prefer_character_input || this.prompt.update(cx, |entry, cx| entry.marked_text_range(window, cx).is_some()) {
                    return;
                }
                if event.keystroke.key == "escape" {
                    if this.popup.take().is_some() { this.needs_focus = true; }
                    else if this.discard { this.discard = false; this.needs_focus = true; }
                    else { this.dismiss(cx); }
                    cx.notify(); cx.stop_propagation();
                }
            }))
            .child(layout_probe("new-task-dialog"))
            .child(div().h(px(46.)).px_4().flex().items_center().gap_2()
                .child(action("task-project", project.to_owned(), Some(Glyph::Folder), false,
                    cx.listener(|this, _: &(), window, cx| this.open_field(Field::Project, window, cx)))
                    .relative().child(layout_probe("task-project")))
                .child(icon(Glyph::ChevronRight).size(px(12.)))
                .child(div().text_size(px(13.)).child("New task"))
                .child(div().flex_1())
                .child(chrome_button("task-close", "Close new task", Glyph::Close, self.busy,
                    cx.listener(|this, _: &(), _, cx| this.dismiss(cx)))))
            .child(div().px_4().relative().child(layout_probe("task-prompt"))
                .child(if self.busy {
                    div().h(px(112.)).overflow_hidden().text_size(px(13.))
                        .child(self.prompt.read(cx).text().to_owned()).into_any_element()
                } else { self.prompt.clone().into_any_element() }))
            .children(self.error.as_ref().map(|error| div().px_4().py_2().text_color(rgb(palette().error)).child(error.clone())))
            .child(div().h(px(42.)).px_4().flex().items_center().gap_2()
                .child(icon(Glyph::Shield).size(px(14.)))
                .child(div().text_size(px(12.)).text_color(rgb(palette().muted)).child("Ask permission"))
                .child(div().flex_1())
                .child(action("task-agent", agent.to_owned(), Some(glyph), false,
                    cx.listener(|this, _: &(), window, cx| this.open_field(Field::Agent, window, cx)))
                    .relative().child(layout_probe("task-agent"))
                    .aria_description("Agent default model. Model and session options are available after connecting in the task.")))
            .child(div().h(px(48.)).px_4().border_t_1().border_color(rgb(palette().border))
                .flex().items_center().gap_2()
                .child(div().text_size(px(11.)).text_color(rgb(palette().muted)).child("Shift+Enter: new line"))
                .child(div().flex_1())
                .child(button_shell("task-as-draft", "Send as draft", self.draft)
                    .role(gpui::Role::CheckBox)
                    .aria_toggled(if self.draft { gpui::Toggled::True } else { gpui::Toggled::False })
                    .aria_label(if self.draft { "Send as draft, on" } else { "Send as draft, off" })
                    .relative().child(layout_probe("task-as-draft"))
                    .flex().items_center().gap_2().border_0().bg(rgba(0))
                    .child(div().w(px(28.)).h(px(16.)).rounded_full().px(px(2.)).flex().items_center()
                        .when(self.draft, |el| el.justify_end())
                        .bg(rgb(if self.draft { palette().focus } else { palette().muted }))
                        .child(div().size(px(12.)).rounded_full().bg(rgb(palette().text))))
                    .child(div().text_size(px(12.)).child("Send as draft"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        if !this.busy && !this.discard { this.draft = !this.draft; cx.notify(); }
                    })))
                .child(button("task-create", if self.busy { "Creating..." } else { "Create task" }, false)
                    .relative().child(layout_probe("task-create"))
                    .bg(rgb(palette().text)).text_color(rgb(palette().canvas))
                    .hover(|style| style.bg(rgb(palette().text)).opacity(0.9))
                    .when(!can_create, |el| el.opacity(0.4).cursor_default())
                    .on_click(cx.listener(|this, _, _, cx| this.create(cx)))))
            .children(self.discard.then(|| div().p_4().border_t_1().border_color(rgb(palette().border))
                .flex().items_center().gap_3().child("Discard the unfinished task?")
                .child(button("task-keep-editing", "Keep editing", false).on_click(cx.listener(|this, _, _, cx| {
                    this.discard = false; this.needs_focus = true; cx.notify();
                })))
                .child(button("task-discard", "Discard draft", false).relative().child(layout_probe("task-discard"))
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(TaskDialogEvent::Dismissed))))))
            .children(self.popup.as_ref().map(|popup| div().absolute().left_4().top(px(44.)).child(popup.clone())));
        let modal = if cx.reduce_motion() {
            modal.into_any_element()
        } else {
            use gpui::AnimationExt;
            modal
                .with_animation(
                    "new-task-entry",
                    gpui::Animation::new(std::time::Duration::from_millis(150))
                        .with_easing(motion::ease_out),
                    |element, value| element.opacity(value),
                )
                .into_any_element()
        };
        div()
            .absolute()
            .inset_0()
            .size_full()
            .px_4()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x00000088))
            .occlude()
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.dismiss(cx);
                    cx.stop_propagation();
                }),
            )
            .child(modal)
    }
}
