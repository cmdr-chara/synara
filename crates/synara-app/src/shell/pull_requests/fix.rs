//! Reviewed context is added only to the exact task and unchanged unsent draft.
use super::*;
#[derive(Clone)]
pub(in crate::shell) struct FixDraft {
    task: TaskId,
    selection: u64,
    review: ReviewFix,
    // Captured only for an explicit recheck, never inferred from a later edit.
    instruction: Option<String>,
}
impl Shell {
    pub(super) fn pr_fix_collect(&mut self, cx: &mut Context<Self>) {
        if !self.pr_require_scope(cx) || self.pull_requests.busy || self.loading_task.is_some() {
            return;
        }
        let Some(task) = self
            .task()
            .filter(|t| Some(t.project_id) == self.project && t.state != TaskState::Archived)
        else {
            self.pull_requests.error = Some("Select the destination conversation in this project before collecting PR Fix context.".into());
            cx.notify();
            return;
        };
        let task = task.id;
        let view = &mut self.pull_requests;
        let (Some(client), Some(repo), Some(detail)) =
            (view.client.clone(), view.repo.clone(), view.detail.as_ref())
        else {
            return;
        };
        let number = detail.pr.number;
        let head = text(&detail.pr.head, "sha");
        view.fix = None;
        let (generation, cancel) = view.begin();
        let selection = self.selection_revision;
        let sender = self.sender.clone();
        self.runtime.spawn(async move {
            let result = client
                .review_fix(&repo, number, &head, cancel)
                .await
                .map(|review| {
                    Outcome::FixReviewed(FixDraft {
                        task,
                        selection,
                        review,
                        instruction: None,
                    })
                });
            let _ = sender
                .send(Update::PullRequests(Box::new(Reply { generation, result })))
                .await;
        });
        cx.notify();
    }
    pub(super) fn pr_fix_reviewed(&mut self, draft: FixDraft, cx: &mut Context<Self>) {
        if Some(draft.task) != self.selected || draft.selection != self.selection_revision {
            self.pull_requests.error = Some(
                "Destination task changed. Collect a fresh PR Fix for the intended task.".into(),
            );
            return;
        }
        self.pull_requests.fix_text.update(cx, |input,cx| input.set_text("Investigate and address the unresolved review comments. Verify each claim against the current files and report any outdated locations. Do not post comments or change PR state.".into(),cx));
        self.pull_requests.fix = Some(draft);
        cx.notify();
    }
    pub(super) fn pr_fix_add(&mut self, cx: &mut Context<Self>) {
        if !self.pr_require_scope(cx)
            || self.pull_requests.busy
            || self.composer.read(cx).is_composing()
            || self.pull_requests.fix_text.read(cx).is_composing()
            || self.close != CloseState::Open
        {
            return;
        }
        let view = &mut self.pull_requests;
        let (Some(mut review), Some(client)) = (view.fix.clone(), view.client.clone()) else {
            return;
        };
        if Some(review.task) != self.selected
            || review.selection != self.selection_revision
            || self.loading_task.is_some()
        {
            view.error =
                Some("Destination changed. Collect a fresh PR Fix. Nothing was added.".into());
            cx.notify();
            return;
        }
        let expected = self.composer.read(cx).text().to_owned();
        let instruction = view.fix_text.read(cx).text().to_owned();
        let draft = match review.review.draft(&instruction, &expected) {
            Ok(draft) => draft,
            Err(error) => {
                view.error = Some(error);
                cx.notify();
                return;
            }
        };
        review.instruction = Some(instruction);
        let (generation, cancel) = view.begin();
        let sender = self.sender.clone();
        self.runtime.spawn(async move {
            let result =
                client
                    .validate_fix(&review.review, cancel)
                    .await
                    .map(|_| Outcome::FixValidated {
                        review,
                        expected,
                        draft,
                    });
            let _ = sender
                .send(Update::PullRequests(Box::new(Reply { generation, result })))
                .await;
        });
        cx.notify();
    }
    pub(super) fn pr_fix_validated(
        &mut self,
        review: FixDraft,
        expected: String,
        draft: String,
        cx: &mut Context<Self>,
    ) {
        if Some(review.task) != self.selected
            || review.selection != self.selection_revision
            || self.loading_task.is_some()
            || self.composer.read(cx).is_composing()
            || self.composer.read(cx).text() != expected
            || !instruction_is_current(
                review.instruction.as_deref(),
                self.pull_requests.fix_text.read(cx).text(),
                self.pull_requests.fix_text.read(cx).is_composing(),
            )
            || self.close != CloseState::Open
        {
            self.pull_requests.error=Some("Task, draft or PR Fix instruction changed during validation. Nothing was added. Review and try again explicitly.".into());
            cx.notify();
            return;
        }
        self.composer
            .update(cx, |input, cx| input.set_text(draft, cx));
        self.remember_draft(cx);
        self.pull_requests.fix = None;
        self.show_conversation(cx);
        self.focus_composer = true;
        self.notice=Some("PR Fix context added to this unsent draft. Review it and use Send explicitly. No provider or local Git state was changed.".into());
        cx.notify();
    }
    pub(super) fn pr_fix_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let view = &self.pull_requests;
        let mut pane = div()
            .flex()
            .flex_col()
            .gap_2()
            .border_b_1()
            .border_color(rgb(palette().border))
            .pb_2()
            .child(
                div()
                    .relative()
                    .child(ui::action(
                        "pr-fix-collect",
                        "Collect unresolved review comments",
                        Some(Glyph::Debug),
                        false,
                        cx.listener(|this, _: &(), _, cx| this.pr_fix_collect(cx)),
                    ))
                    .child(crate::ui::layout_probe("pr-fix-collect")),
            );
        if view.busy {
            pane = pane.child(
                div()
                    .relative()
                    .child("Reading review state...")
                    .child(crate::ui::layout_probe("pr-fix-busy")),
            );
        }
        if let Some(error) = &view.error {
            pane = pane.child(
                div()
                    .relative()
                    .child(error.clone())
                    .child(crate::ui::layout_probe("pr-fix-error")),
            );
        }
        if let Some(fix) = &view.fix {
            pane=pane.child(format!("PR Fix: {} #{} | {} unresolved threads | head {} | destination task {}",fix.review.repository.slug(),fix.review.number,fix.review.unresolved_threads,fix.review.head,fix.task))
                .child("Review the instruction and immutable provider context. Adding rechecks all comments and the PR head. Nothing is sent automatically.")
                .child(div().relative().min_h(px(110.)).child(view.fix_text.clone()).child(crate::ui::layout_probe("pr-fix-instruction")))
                .child(div().id("pr-fix-context").max_h(px(170.)).overflow_y_scroll().text_xs().child(fix.review.context().to_owned()))
                .child(div().flex().gap_2()
                    .child(div().relative().child(ui::action("pr-fix-add","Recheck and add to unsent draft",None,false,cx.listener(|this,_:&(),_,cx| this.pr_fix_add(cx)))).child(crate::ui::layout_probe("pr-fix-add")))
                    .child(div().relative().child(ui::action("pr-fix-discard","Cancel PR Fix",None,false,cx.listener(|this,_:&(),_,cx| { this.pull_requests.retire(); cx.notify(); }))).child(crate::ui::layout_probe("pr-fix-discard"))));
        }
        pane.into_any_element()
    }
}

// A successful network response does not authorize replacing edits made while
// it was in flight. IME composition is also user work, even before text changes.
fn instruction_is_current(expected: Option<&str>, current: &str, composing: bool) -> bool {
    !composing && expected == Some(current)
}

#[cfg(test)]
mod tests {
    use super::instruction_is_current;

    #[test]
    fn pr_fix_recheck_requires_an_explicit_instruction_snapshot() {
        assert!(!instruction_is_current(None, "", false));
        assert!(!instruction_is_current(None, "Review this change", false));
    }

    #[test]
    fn pr_fix_recheck_rejects_instruction_edits() {
        assert!(!instruction_is_current(Some("Before"), "After", false));
        assert!(!instruction_is_current(Some("Before"), "", false));
        assert!(!instruction_is_current(Some("Review"), "Review ", false));
    }

    #[test]
    fn pr_fix_recheck_never_overrides_ime_composition() {
        assert!(!instruction_is_current(Some("Review"), "Review", true));
    }

    #[test]
    fn pr_fix_recheck_preserves_exact_multiline_unicode_instruction() {
        let instruction = "Review the file\nKeep caf\u{e9} unchanged";
        assert!(instruction_is_current(
            Some(instruction),
            instruction,
            false
        ));
    }
}
