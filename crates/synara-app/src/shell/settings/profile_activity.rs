//! Local profile activity visualizations built from durable thread turn timestamps.
use super::*;

pub(super) fn active_hours(activity: Option<&ProfileActivity>, loading: bool) -> gpui::AnyElement {
    let Some(activity) = activity else {
        return div()
            .text_color(rgb(palette().muted))
            .child(if loading {
                "Loading local activity…"
            } else {
                "Activity has not been loaded yet."
            })
            .into_any_element();
    };

    let max_count = activity.hours.iter().copied().max().unwrap_or_default();
    let peak = activity
        .hours
        .iter()
        .enumerate()
        .max_by_key(|(_, count)| **count)
        .filter(|(_, count)| **count > 0)
        .map(|(hour, count)| format!("{hour:02}:00 UTC · {count} prompts"))
        .unwrap_or_else(|| "No recorded turn starts.".into());

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(palette().muted))
                .child(peak),
        )
        .child(
            div()
                .id("profile-active-hours")
                .w_full()
                .h(px(82.))
                .flex()
                .items_end()
                .gap(px(3.))
                .children(activity.hours.iter().enumerate().map(|(hour, count)| {
                    let height = if max_count == 0 || *count == 0 {
                        2.
                    } else {
                        6. + (*count as f32 / max_count as f32) * 68.
                    };
                    div()
                        .id(("profile-active-hour", hour))
                        .flex_1()
                        .h_full()
                        .flex()
                        .items_end()
                        .child(div().w_full().h(px(height)).rounded(px(3.)).bg(rgb(
                            if *count > 0 {
                                palette().focus
                            } else {
                                palette().border
                            },
                        )))
                })),
        )
        .child(div().w_full().flex().children((0..24).map(|hour| {
            div()
                .flex_1()
                .text_size(px(10.))
                .text_color(rgb(palette().muted))
                .child(if hour % 6 == 0 {
                    format!("{hour:02}")
                } else {
                    String::new()
                })
        })))
        .child(
            div()
                .text_size(px(11.))
                .text_color(rgb(palette().muted))
                .child("Turn start times from this installation · UTC"),
        )
        .into_any_element()
}
