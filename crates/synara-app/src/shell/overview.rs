use super::*;
use crate::ui::{self, palette};

impl Shell {
    pub(super) fn kanban_panel(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        div()
            .relative()
            .child(ui::layout_probe("kanban-board"))
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .p_6()
            .gap_5()
            .child(div().text_size(px(24.)).child("Kanban"))
            .child(
                div()
                    .id("kanban-columns")
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .gap_4()
                    .overflow_x_scroll()
                    .children(
                        [
                            ("Ready", vec![TaskState::Ready]),
                            ("In progress", vec![TaskState::Running]),
                            (
                                "Needs attention",
                                vec![TaskState::Waiting, TaskState::Failed],
                            ),
                            ("Complete", vec![TaskState::Completed]),
                        ]
                        .into_iter()
                        .enumerate()
                        .map(|(column, (label, states))| {
                            let tasks: Vec<_> = self
                                .catalog
                                .tasks
                                .iter()
                                .filter(|task| states.contains(&task.state))
                                .collect();
                            div()
                                .id(("kanban-column", column))
                                .min_w(px(210.))
                                .flex_1()
                                .min_h_0()
                                .flex()
                                .flex_col()
                                .gap_3()
                                .child(
                                    div()
                                        .flex()
                                        .justify_between()
                                        .text_color(rgb(palette().muted))
                                        .child(label)
                                        .child(tasks.len().to_string()),
                                )
                                .child(
                                    div()
                                        .id(("kanban-tasks", column))
                                        .flex_1()
                                        .overflow_y_scroll()
                                        .flex()
                                        .flex_col()
                                        .gap_2()
                                        .children(tasks.iter().map(|task| {
                                            let id = task.id;
                                            let project = self
                                                .catalog
                                                .projects
                                                .iter()
                                                .find(|project| project.id == task.project_id)
                                                .map_or("", |project| project.name.as_str());
                                            ui::button(
                                                SharedString::from(format!("kanban-task-{id}")),
                                                task.title.clone(),
                                                false,
                                            )
                                            .p_3()
                                            .flex()
                                            .flex_col()
                                            .gap_2()
                                            .text_size(px(15.))
                                            .min_w_0()
                                            .relative()
                                            .children(
                                                (column == 0)
                                                    .then(|| ui::layout_probe("kanban-ready-task")),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(12.))
                                                    .text_color(rgb(palette().muted))
                                                    .child(project.to_owned()),
                                            )
                                            .on_click(
                                                cx.listener(move |this, _, _, cx| {
                                                    if this.select_task(id, cx) {
                                                        this.set_panel(Panel::Conversation, cx);
                                                    }
                                                }),
                                            )
                                        })),
                                )
                        }),
                    ),
            )
            .into_any_element()
    }

    pub(super) fn help_panel(&self) -> gpui::AnyElement {
        div().id("help-panel").relative().child(ui::layout_probe("help-panel")).flex_1().min_h_0().overflow_y_scroll().p_6().flex().flex_col().gap_4()
            .child(div().font_family("Cal Sans").text_size(px(28.)).child("Synara"))
            .child("Keyboard shortcuts")
            .child("Ctrl/Cmd + 1: Conversation · 2: Files · 3: Changes · 4: Terminal · 5: Inspector · 6: Settings · 7: Agents · 8: Remote · 9: Kanban")
            .child("Agent approval requests are shown for your confirmation. Selecting a model or agent does not send a prompt.")
            .child(div().mt_4().text_size(px(20.)).child("Fonts and icons"))
            .child("Cal Sans · SIL Open Font License 1.1")
            .child(div().text_size(px(12.)).child(include_str!("../../assets/licenses/CalSans-OFL.txt")))
            .child("Synara's Central icons and provider artwork")
            .child(div().text_size(px(12.)).child(include_str!("../../assets/NOTICE.md")))
            .child(div().text_size(px(12.)).child(include_str!("../../assets/licenses/Synara-source-MIT.txt")))
            .child("Tabler icons · MIT license")
            .child(div().text_size(px(12.)).child(include_str!("../../assets/licenses/Tabler-MIT.txt")))
            .child("OpenAI glyph · Simple Icons (CC0), via React Icons (MIT)")
            .child(div().text_size(px(12.)).child(include_str!("../../assets/licenses/Simple-Icons-CC0.txt")))
            .child(div().text_size(px(12.)).child(include_str!("../../assets/licenses/React-Icons-MIT.txt")))
            .into_any_element()
    }
}
