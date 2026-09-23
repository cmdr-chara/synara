//! Show reported context separately from cumulative input/output token totals.
use super::*;

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

impl Shell {
    pub(in crate::shell) fn context_usage_view(&self, cx: &mut Context<Self>) -> gpui::Div {
        let mut body = div().px_2().py_1().flex().flex_col().gap_1();
        let Some(thread) = self
            .thread
            .as_ref()
            .filter(|thread| self.task().is_some_and(|task| task.thread_id == thread.id))
        else {
            return body;
        };
        let (label, warning) =
            context_summary(thread.usage.context_used, thread.usage.context_limit);
        body = body.child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .flex_wrap()
                .gap_1()
                .text_size(px(11.))
                .text_color(rgb(if warning {
                    palette().error
                } else {
                    palette().muted
                }))
                .child(label)
                .child(ui::action(
                    "composer-context-details",
                    "Usage details",
                    None,
                    false,
                    cx.listener(|this, _, _, cx| {
                        this.open_settings_section(settings::Section::Usage, cx);
                    }),
                )),
        );
        if warning {
            body = body.child(
                div()
                    .text_size(px(11.))
                    .text_color(rgb(palette().error))
                    .child("Near or above the reported context limit. Review model/context settings before continuing. This warning does not compact or remove messages."),
            );
        }
        body
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
