//! Stacks reuse the loaded PR scope, process owner and cancellation generation.
use super::*;

impl Shell {
    pub(super) fn pr_stack_load(&mut self, cx: &mut Context<Self>) {
        if !self.pr_require_scope(cx) || self.pull_requests.busy {
            return;
        }
        let v = &mut self.pull_requests;
        let (Some(client), Some(repo), Some(detail)) =
            (v.client.clone(), v.repo.clone(), v.detail.as_ref())
        else {
            return;
        };
        let number = detail.pr.number;
        v.stack = None;
        v.stack_progress = None;
        let (generation, cancel) = v.begin();
        let sender = self.sender.clone();
        self.runtime.spawn(async move {
            let result = client
                .review_stack(&repo, number, cancel)
                .await
                .map(Outcome::Stack);
            let _ = sender
                .send(Update::PullRequests(Box::new(Reply { generation, result })))
                .await;
        });
        cx.notify();
    }
    fn pr_stack_prepare(&mut self, cx: &mut Context<Self>) {
        if !self.pr_require_scope(cx) || self.pull_requests.busy {
            return;
        }
        let v = &mut self.pull_requests;
        if let Some(review) = &v.stack {
            if review.ready() {
                v.confirmation = None;
                v.stack_confirmation = Some(review.clone());
            } else {
                v.error = Some("The selected prefix has blocked or unknown readiness. Refresh after checks finish.".into());
            }
        }
        cx.notify();
    }
    fn pr_stack_confirm(&mut self, cx: &mut Context<Self>) {
        if !self.pr_require_scope(cx) || self.pull_requests.busy {
            return;
        }
        let v = &mut self.pull_requests;
        let (Some(client), Some(repo), Some(review)) = (
            v.client.clone(),
            v.repo.clone(),
            v.stack_confirmation.take(),
        ) else {
            return;
        };
        let (generation, cancel) = v.begin();
        v.stack_writing = true;
        v.stack_progress = None;
        let sender = self.sender.clone();
        self.runtime.spawn(async move {
            let result = client
                .merge_stack_prefix(&repo, review, true, cancel)
                .await
                .map(Outcome::StackWritten);
            let _ = sender
                .send(Update::PullRequests(Box::new(Reply { generation, result })))
                .await;
        });
        cx.notify();
    }
    pub(super) fn pr_stack_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let v = &self.pull_requests;
        let mut pane = div()
            .flex()
            .flex_col()
            .gap_1()
            .border_b_1()
            .border_color(rgb(palette().border))
            .pb_2();
        pane = pane.child(
            div()
                .relative()
                .child(ui::layout_probe("pr-stack-load"))
                .child(ui::action(
                    "pr-stack-load",
                    "Discover / refresh stack",
                    Some(Glyph::PullRequest),
                    false,
                    cx.listener(|this, _: &(), _, cx| this.pr_stack_load(cx)),
                )),
        );
        if let Some(progress) = &v.stack_progress {
            pane = pane.child(
                div()
                    .relative()
                    .child(ui::layout_probe("pr-stack-progress"))
                    .text_sm()
                    .child(progress.summary()),
            );
        }
        if let Some(review) = &v.stack {
            let prefix = review.prefix().unwrap_or_default();
            pane = pane.child(format!(
                "Stack: selected #{} | position {} on its path | {} PRs in this stack",
                review.selected(),
                prefix.len(),
                review.rows().len()
            ));
            let mut rows = div()
                .id("pr-stack-rows")
                .max_h(px(170.))
                .overflow_y_scroll();
            for (at, row) in review.rows().iter().enumerate() {
                let number = row.number();
                let parent = row
                    .parent
                    .map(|n| format!("parent #{n}"))
                    .unwrap_or_else(|| "root".into());
                let state = row.blocked.as_deref().unwrap_or("checks ready");
                let label = format!(
                    "{}#{} {} <- {} | {} | {} | {}",
                    "  ".repeat(row.depth),
                    number,
                    row.branch(),
                    row.base(),
                    parent,
                    &row.head()[..8],
                    state
                );
                rows = rows.child(
                    div()
                        .relative()
                        .child(ui::layout_probe_slot("pr-stack-row", at))
                        .child(ui::action(
                            format!("pr-stack-row-{number}"),
                            label,
                            None,
                            number == review.selected(),
                            cx.listener(move |this, _: &(), _, cx| {
                                if this.pull_requests.busy || !this.pr_require_scope(cx) {
                                    return;
                                }
                                if let Some(review) = &mut this.pull_requests.stack {
                                    let _ = review.select(number);
                                }
                                this.pull_requests.stack_confirmation = None;
                                this.pr_load(Some(number), cx);
                            }),
                        )),
                );
            }
            pane = pane.child(rows).child(
                div()
                    .relative()
                    .child(ui::layout_probe("pr-stack-prepare"))
                    .child(ui::action(
                        "pr-stack-prepare",
                        "Review selected prefix merge...",
                        Some(Glyph::Shield),
                        false,
                        cx.listener(|this, _: &(), _, cx| this.pr_stack_prepare(cx)),
                    )),
            );
            if let Some(confirmation) = &v.stack_confirmation
                && let Ok(prefix) = confirmation.prefix()
            {
                let base = prefix[0].base();
                let mut notice = div().relative().child(ui::layout_probe("pr-stack-confirmation")).text_sm()
                        .child(format!("Confirm {}: merge this prefix in order into {base}, using merge commits only. Retarget each child to {base}. No force, branch deletion, checkout or local Git mutation. This is not atomic: Stop keeps confirmed earlier writes. New checks after retargeting may stop the prefix.",confirmation.repository.slug()));
                for row in prefix {
                    notice = notice.child(div().font_family("monospace").text_xs().child(format!(
                        "#{} {} | {} -> {base}",
                        row.number(),
                        row.head(),
                        row.base()
                    )));
                }
                pane = pane.child(notice).child(
                    div()
                        .flex()
                        .gap_2()
                        .child(
                            div()
                                .relative()
                                .child(ui::layout_probe("pr-stack-confirm"))
                                .child(ui::action(
                                    "pr-stack-confirm",
                                    "Confirm reviewed prefix",
                                    Some(Glyph::Shield),
                                    false,
                                    cx.listener(|this, _: &(), _, cx| this.pr_stack_confirm(cx)),
                                )),
                        )
                        .child(
                            div()
                                .relative()
                                .child(ui::layout_probe("pr-stack-dismiss"))
                                .child(ui::action(
                                    "pr-stack-dismiss",
                                    "Cancel review",
                                    None,
                                    false,
                                    cx.listener(|this, _: &(), _, cx| {
                                        this.pull_requests.stack_confirmation = None;
                                        cx.notify();
                                    }),
                                )),
                        ),
                );
            }
        }
        pane.into_any_element()
    }
}
