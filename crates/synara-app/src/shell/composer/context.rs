//! Reported context as a ring meter in the composer footer, matching upstream's
//! `ContextWindowMeter`: rendered only when the session reports context usage,
//! fills toward the reported limit, turns red near capacity, and opens the
//! Usage settings on click.
use super::*;

const METER_SIZE: f32 = 16.;
const RING_RADIUS: f32 = 6.;
const RING_WIDTH: f32 = 1.6;

fn context_summary(used: Option<u64>, limit: Option<u64>) -> (String, bool) {
    match (used, limit.filter(|n| *n > 0)) {
        (Some(used), Some(limit)) => {
            // Widen before multiplication. Provider values are untrusted u64s.
            let percent = u128::from(used) * 100 / u128::from(limit);
            (
                format!("Last reported context: {used} / {limit} tokens ({percent}%)"),
                percent >= 80,
            )
        }
        (Some(used), None) => (
            format!("Last reported context: {used} tokens. Capacity not reported."),
            false,
        ),
        (None, Some(limit)) => (
            format!("Context usage not reported. Reported capacity: {limit} tokens."),
            false,
        ),
        (None, None) => ("Context usage not reported by this session.".into(), false),
    }
}

/// Full ring outline drawn as two semicircle strokes.
fn ring(size: f32, radius: f32, width: f32) -> Option<gpui::Path<gpui::Pixels>> {
    let center = px(size / 2.);
    let radii = gpui::point(px(radius), px(radius));
    let mut builder = gpui::PathBuilder::stroke(px(width));
    builder.move_to(gpui::point(center + px(radius), center));
    builder.arc_to(
        radii,
        px(0.),
        false,
        true,
        gpui::point(center - px(radius), center),
    );
    builder.arc_to(
        radii,
        px(0.),
        false,
        true,
        gpui::point(center + px(radius), center),
    );
    builder.build().ok()
}

/// Ring sector stroked clockwise from twelve o'clock over `fraction` of the
/// circumference (endpoint arcs, at most 180° each).
fn arc(size: f32, radius: f32, width: f32, fraction: f32) -> Option<gpui::Path<gpui::Pixels>> {
    if fraction <= 0. {
        return None;
    }
    if fraction >= 0.999 {
        return ring(size, radius, width);
    }
    let center = size / 2.;
    let radii = gpui::point(px(radius), px(radius));
    let mut builder = gpui::PathBuilder::stroke(px(width));
    builder.move_to(gpui::point(px(center), px(center - radius)));
    let total = fraction.clamp(0., 1.) * 360.;
    let mut swept = 0f32;
    while swept < total {
        let step = (total - swept).min(180.);
        let theta = (swept + step).to_radians();
        builder.arc_to(
            radii,
            px(0.),
            step > 180.,
            true,
            gpui::point(
                px(center + radius * theta.sin()),
                px(center - radius * theta.cos()),
            ),
        );
        swept += step;
    }
    builder.build().ok()
}

impl Shell {
    pub(in crate::shell) fn context_meter(
        &self,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        let thread = self
            .thread
            .as_ref()
            .filter(|thread| self.task().is_some_and(|task| task.thread_id == thread.id))?;
        // Upstream renders the meter only once a session reports usage.
        let used = thread.usage.context_used?;
        let limit = thread.usage.context_limit.filter(|limit| *limit > 0);
        let fraction = limit.map_or(0., |limit| {
            (used as f64 / limit as f64).clamp(0., 1.) as f32
        });
        let (summary, warning) = context_summary(Some(used), limit);
        let track_color = palette().border;
        let meter_color = if warning {
            palette().error
        } else {
            palette().focus
        };
        Some(
            div()
                .id("composer-context-meter")
                .role(gpui::Role::Button)
                .aria_label(summary.clone())
                .tab_index(0)
                .size(px(20.))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .cursor_pointer()
                .hover(|style| style.bg(rgb(palette().hover)))
                .focus_visible(|style| style.border_1().border_color(rgb(palette().focus)))
                .tooltip(move |_, cx| cx.new(|_| ui::Tooltip(summary.clone().into())).into())
                .child(
                    gpui::canvas(
                        |_, _, _| (),
                        move |_, _, window, _| {
                            if let Some(path) = ring(METER_SIZE, RING_RADIUS, RING_WIDTH) {
                                window.paint_path(path, rgb(track_color));
                            }
                            if let Some(path) = arc(METER_SIZE, RING_RADIUS, RING_WIDTH, fraction) {
                                window.paint_path(path, rgb(meter_color));
                            }
                        },
                    )
                    .size(px(METER_SIZE)),
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.open_settings_section(settings::Section::Usage, cx);
                }))
                .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::context_summary;

    #[test]
    fn unknown_capacity_is_not_zero_and_large_reports_do_not_overflow() {
        assert!(!context_summary(None, None).1);
        assert!(
            context_summary(None, Some(100))
                .0
                .contains("usage not reported")
        );
        assert!(
            context_summary(Some(40), Some(0))
                .0
                .contains("Capacity not reported")
        );
        assert!(!context_summary(Some(79), Some(100)).1);
        assert!(context_summary(Some(80), Some(100)).1);
        assert!(context_summary(Some(120), Some(100)).0.contains("120%"));
        assert!(context_summary(Some(u64::MAX), Some(1)).1);
        assert!(context_summary(Some(0), Some(u64::MAX)).0.contains("(0%)"));
    }
}
