//! Collapsed work summaries preserve the underlying tools and approval rows.
use super::*;
use crate::ui::{self, Glyph, palette};

pub(super) fn turn_for_row(thread: &Thread, index: usize) -> Option<usize> {
    thread
        .turns
        .iter()
        .position(|turn| (turn.first_timeline_index..turn.end_timeline_index).contains(&index))
}

pub(super) fn activity_anchor(thread: &Thread, turn: &TurnSummary) -> Option<usize> {
    let range = turn.first_timeline_index..turn.end_timeline_index.min(thread.timeline.len());
    range
        .clone()
        .find(|index| {
            matches!(&thread.timeline[*index],
        TranscriptItem::Message { index } if thread.messages[*index].role == Role::Assistant)
        })
        .or_else(|| {
            range
                .into_iter()
                .find(|index| match &thread.timeline[*index] {
                    TranscriptItem::Tool { .. } => true,
                    TranscriptItem::Message { index } => {
                        thread.messages[*index].role == Role::Reasoning
                    }
                    _ => false,
                })
        })
}

pub(super) fn answer_index(thread: &Thread, turn: &TurnSummary) -> Option<usize> {
    (turn.first_timeline_index..turn.end_timeline_index.min(thread.timeline.len()))
        .rev().find(|index| matches!(&thread.timeline[*index], TranscriptItem::Message { index } if thread.messages[*index].role == Role::Assistant))
}

impl Shell {
    pub(super) fn activity_summary(
        &self,
        thread: &Thread,
        turn_index: usize,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let turn = &thread.turns[turn_index];
        let answer = answer_index(thread, turn);
        let key = (thread.id, turn.id.clone());
        let expanded = self.expanded_activity.contains(&key);
        let seconds = turn
            .finished_at_ms
            .map(|end| end.saturating_sub(turn.started_at_ms) / 1000);
        let label = match seconds {
            Some(seconds) if seconds >= 60 => {
                format!("Worked for {}m {}s", seconds / 60, seconds % 60)
            }
            Some(seconds) => format!("Worked for {seconds}s"),
            None => "Working…".into(),
        };
        let has_answer = (turn.first_timeline_index..turn.end_timeline_index).any(|index| {
            matches!(&thread.timeline[index], TranscriptItem::Message { index } if thread.messages[*index].role == Role::Assistant)
        });
        let tools: Vec<_> = thread.timeline[turn.first_timeline_index..turn.end_timeline_index]
            .iter()
            .filter_map(|item| match item {
                TranscriptItem::Tool { id } => thread.tools.get(id),
                _ => None,
            })
            .collect();
        let label = if !has_answer && !tools.is_empty() && turn.finished_at_ms.is_some() {
            let command = tools
                .iter()
                .all(|tool| tool.kind.as_deref() == Some("execute"));
            format!(
                "Ran {} {}{}",
                tools.len(),
                if command { "command" } else { "action" },
                if tools.len() == 1 { "" } else { "s" }
            )
        } else {
            label
        };
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap_2()
            .pb_3()
            .when(has_answer, |el| {
                el.border_b_1().border_color(gpui::rgba(0xffffff0c)).mb_3()
            })
            .child(
                ui::button_shell(("work-summary", turn_index), label.clone(), false)
                    .bg(gpui::rgba(0))
                    .ml(px(-2.))
                    .border_0()
                    .p_0()
                    .flex()
                    .items_center()
                    .gap_1()
                    .text_size(px(15.))
                    .text_color(rgb(palette().muted))
                    .relative()
                    .child(ui::layout_probe_slot("work-summary", turn_index))
                    .children((!has_answer).then(|| ui::icon(Glyph::Terminal)))
                    .child(label)
                    .child(
                        ui::icon(if expanded {
                            Glyph::Chevron
                        } else {
                            Glyph::ChevronRight
                        })
                        .size(px(12.)),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if !this.expanded_activity.remove(&key) {
                            this.expanded_activity.insert(key.clone());
                        }
                        this.transcript.invalidate_activity(&key.1);
                        cx.notify();
                    })),
            )
            .children(expanded.then(|| {
                div()
                    .id(("activity-details", turn_index))
                    .max_h(px(320.))
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .py_2()
                    .children(
                        thread.timeline[turn.first_timeline_index..turn.end_timeline_index]
                            .iter()
                            .enumerate()
                            .filter_map(|(offset, item)| match item {
                                TranscriptItem::Tool { id } => thread
                                    .tools
                                    .get(id)
                                    .map(|tool| self.tool_detail(thread, tool)),
                                TranscriptItem::Message { index }
                                    if thread.messages[*index].role != Role::User
                                        && Some(turn.first_timeline_index + offset) != answer =>
                                {
                                    Some(
                                        div()
                                            .text_size(px(14.))
                                            .text_color(rgb(palette().muted))
                                            .child(ui::markdown::render(
                                                &thread.messages[*index].text,
                                                &thread.messages[*index].id,
                                            ))
                                            .into_any_element(),
                                    )
                                }
                                _ => None,
                            }),
                    )
            }))
            .into_any_element()
    }

    pub(super) fn tool_detail(&self, thread: &Thread, tool: &Tool) -> gpui::AnyElement {
        div()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_1()
            .text_size(px(13.))
            .child(
                div()
                    .text_color(rgb(palette().muted))
                    .child(format!("{} · {:?}", tool.title, tool.status)),
            )
            .children(tool.output.iter().map(|output| {
                let text = match output {
                    ToolOutput::Text { text } => truncate(text, 16 * 1024),
                    ToolOutput::Diff {
                        path,
                        before,
                        after,
                    } => format!(
                        "{path}\n{}\n{}",
                        before.as_deref().unwrap_or_default(),
                        after.as_deref().unwrap_or_default()
                    ),
                    ToolOutput::Terminal { id } => thread.terminals.get(id).map_or_else(
                        || format!("Terminal {id}"),
                        |record| truncate(&record.text, 16000),
                    ),
                    ToolOutput::Resource { uri, name } => format!("{name} · {uri}"),
                };
                div()
                    .p_2()
                    .rounded_md()
                    .bg(rgb(palette().overlay))
                    .font_family(crate::ui::code_font())
                    .text_size(px(12.))
                    .child(text)
            }))
            .into_any_element()
    }
}
