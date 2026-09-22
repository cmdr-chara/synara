//! PR Fix is a reviewed draft handoff, never an agent or GitHub write action.
use super::*;

pub(super) struct FixReview {
    task: Task,
    snapshot: FixSnapshot,
}

fn append_fix_draft(draft: &str, instructions: &str) -> std::result::Result<String, String> {
    if instructions.trim().is_empty()
        || instructions.len() > MAX_FIX_PROMPT_BYTES
        || instructions.contains('\0')
    {
        return Err("Fix instructions must be nonempty, NUL-free and at most 512 KiB.".into());
    }
    let separator = if draft.is_empty() { "" } else { "\n\n" };
    if draft
        .len()
        .saturating_add(separator.len())
        .saturating_add(instructions.len())
        > 1024 * 1024
    {
        return Err("Combined draft exceeds 1 MiB. Both drafts were kept unchanged.".into());
    }
    Ok(format!("{draft}{separator}{instructions}"))
}

fn same_task_owner(expected: &Task, current: &Task) -> bool {
    expected.id == current.id
        && expected.thread_id == current.thread_id
        && expected.project_id == current.project_id
        && expected.working_directory == current.working_directory
        && current.state != TaskState::Archived
}

async fn checked_task(
    service: &WorkspaceService,
    expected: &Task,
    target: &WorkspaceTarget,
) -> std::result::Result<Task, String> {
    let current = service.task(expected.id).await.map_err(|e| e.to_string())?;
    if !same_task_owner(expected, &current) || current.working_directory != *target.root() {
        return Err("The target task or working directory changed. Nothing was added.".into());
    }
    let workspace = service
        .workspace_for_task(&current)
        .await
        .map_err(|e| e.to_string())?;
    let matches = match target {
        WorkspaceTarget::Local { .. } => {
            matches!(workspace.location, WorkspaceLocation::Local { .. })
        }
        WorkspaceTarget::Ssh {
            workspace: loaded, ..
        } => workspace.id == loaded.id && workspace.location == loaded.location,
    };
    if !matches {
        return Err("The target workspace changed. Nothing was added.".into());
    }
    Ok(current)
}

impl PrView {
    pub(in crate::shell) fn has_fix_review(&self) -> bool {
        self.fix.is_some()
    }
}

impl Shell {
    fn pr_fix_owner_current(&self, task: &Task, cx: &App) -> bool {
        self.pr_scope_current()
            && self.selected == Some(task.id)
            && self.project == Some(task.project_id)
            && self
                .task()
                .is_some_and(|current| same_task_owner(task, current))
            && self.loading_task.is_none()
            && !self.draft_state.loading.contains(&task.id)
            && self.close == CloseState::Open
            && !self.composer.read(cx).is_composing()
    }

    pub(super) fn pr_collect_fixes(&mut self, cx: &mut Context<Self>) {
        if !self.pr_require_scope(cx) || self.pull_requests.busy {
            return;
        }
        if self.pull_requests.fix.is_some() {
            self.pull_requests.error = Some(
                "Add or explicitly discard the current fix review first. Its edits were kept."
                    .into(),
            );
            cx.notify();
            return;
        }
        let Some(task) = self.task().cloned() else {
            self.pull_requests.error =
                Some("Select an existing chat in this PR's project first.".into());
            cx.notify();
            return;
        };
        if !self.pr_fix_owner_current(&task, cx) {
            self.pull_requests.error = Some(
                "Select a writable, fully loaded chat in this PR's project. Nothing was collected."
                    .into(),
            );
            cx.notify();
            return;
        }
        let view = &mut self.pull_requests;
        let (Some(client), Some(repo), Some(target), Some(detail)) = (
            view.client.clone(),
            view.repo.clone(),
            view.target.clone(),
            view.detail.as_ref(),
        ) else {
            return;
        };
        if detail.pr.state != "open" || detail.pr.merged == Some(true) {
            view.error = Some("PR Fix requires an open, unmerged pull request.".into());
            cx.notify();
            return;
        }
        let number = detail.pr.number;
        let head = text(&detail.pr.head, "sha").to_owned();
        let selection = self.selection_revision;
        let service = self.controller.workspace.clone();
        let (generation, cancel) = view.begin();
        self.job(async move {
            let result = async {
                let task = checked_task(&service, &task, &target).await?;
                let snapshot = client
                    .collect_unresolved_reviews(&repo, number, &head, cancel)
                    .await?;
                // Reject an empty/incomplete/oversized result before publishing UI state.
                snapshot.instruction_set()?;
                Ok(Outcome::FixCollected {
                    task,
                    selection,
                    snapshot,
                })
            }
            .await;
            Ok(Update::PullRequests(Box::new(Reply { generation, result })))
        });
        cx.notify();
    }

    pub(super) fn pr_fixes_collected(
        &mut self,
        task: Task,
        selection: u64,
        snapshot: FixSnapshot,
        cx: &mut Context<Self>,
    ) {
        if selection != self.selection_revision || !self.pr_fix_owner_current(&task, cx) {
            self.pull_requests.error = Some(
                "Task selection changed during collection. No review or draft was replaced.".into(),
            );
            return;
        }
        if self.pull_requests.fix.is_some() {
            return;
        }
        match snapshot.instruction_set() {
            Ok(instructions) => {
                self.pull_requests
                    .fix_editor
                    .update(cx, |entry, cx| entry.set_text(instructions, cx));
                self.pull_requests.fix = Some(FixReview { task, snapshot });
                self.pull_requests.fix_discard_confirm = false;
            }
            Err(error) => self.pull_requests.error = Some(error),
        }
    }

    pub(super) fn pr_add_fixes_to_draft(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.pull_requests.busy || !self.pr_require_scope(cx) {
            return;
        }
        let view = &self.pull_requests;
        let (Some(review), Some(client), Some(target)) =
            (view.fix.as_ref(), view.client.clone(), view.target.clone())
        else {
            return;
        };
        if !self.pr_fix_owner_current(&review.task, cx) || view.fix_editor.read(cx).is_composing() {
            self.pull_requests.error = Some("Return to the original target chat and finish text composition before adding this review.".into());
            cx.notify();
            return;
        }
        let instructions = view.fix_editor.read(cx).text().to_owned();
        if instructions.trim().is_empty()
            || instructions.len() > MAX_FIX_PROMPT_BYTES
            || instructions.contains('\0')
        {
            self.pull_requests.error = Some("Fix instructions must be nonempty, NUL-free and at most 512 KiB. Your edits were kept.".into());
            cx.notify();
            return;
        }
        let task = review.task.clone();
        let snapshot = review.snapshot.clone();
        let selection = self.selection_revision;
        let service = self.controller.workspace.clone();
        let (generation, cancel) = self.pull_requests.begin();
        window.focus(&self.pull_requests.fix_focus, cx);
        self.job(async move {
            let result = async {
                checked_task(&service, &task, &target).await?;
                client
                    .verify_review_snapshot(&snapshot, cancel.clone())
                    .await?;
                let current = checked_task(&service, &task, &target).await?;
                if cancel.is_cancelled() {
                    return Err("Cancelled. No draft was changed.".into());
                }
                Ok(Outcome::FixVerified {
                    task: current,
                    selection,
                    fingerprint: snapshot.fingerprint()?,
                    instructions,
                })
            }
            .await;
            Ok(Update::PullRequests(Box::new(Reply { generation, result })))
        });
        cx.notify();
    }

    pub(super) fn pr_fixes_verified(
        &mut self,
        task: Task,
        selection: u64,
        fingerprint: String,
        instructions: String,
        cx: &mut Context<Self>,
    ) {
        let result = (|| -> std::result::Result<(), String> {
            if selection != self.selection_revision || !self.pr_fix_owner_current(&task, cx) {
                return Err("Conversation changed during verification. Your fix review was kept; no draft changed.".into());
            }
            let view = &self.pull_requests;
            let review = view.fix.as_ref().ok_or("The fix review was closed")?;
            if !same_task_owner(&review.task, &task)
                || review.snapshot.fingerprint()? != fingerprint
                || view.fix_editor.read(cx).is_composing()
                || view.fix_editor.read(cx).text() != instructions
            {
                return Err(
                    "The fix review changed during verification. Nothing was inserted.".into(),
                );
            }
            let draft = self.composer.read(cx).text().to_owned();
            let combined = append_fix_draft(&draft, &instructions)?;
            self.composer
                .update(cx, |entry, cx| entry.set_text(combined, cx));
            self.remember_draft(cx);
            self.pull_requests.fix = None;
            self.pull_requests
                .fix_editor
                .update(cx, |entry, cx| entry.set_text(String::new(), cx));
            self.pull_requests.fix_discard_confirm = false;
            self.notice = Some(format!(
                "PR Fix added to '{}'. Inspect or edit the normal chat draft before Send. Nothing was sent or executed.",
                task.title
            ));
            self.set_panel(Panel::Conversation, cx);
            self.focus_composer = true;
            Ok(())
        })();
        if let Err(error) = result {
            self.pull_requests.error = Some(error);
        }
    }

    pub(super) fn pr_fix_review_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let v = &self.pull_requests;
        let Some(review) = &v.fix else {
            return div().into_any_element();
        };
        let snapshot = &review.snapshot;
        let mut pane = div().id("pr-fix-review").track_focus(&v.fix_focus).tab_index(0)
            .flex().flex_col().flex_1().min_w_0().min_h_0().gap_2().overflow_y_scroll()
            .child(format!("PR Fix: {} #{}", snapshot.repository.slug(), snapshot.number))
            .child(div().text_xs().child(format!("Target: {} | task {} | project {}", review.task.title, review.task.id, review.task.project_id)))
            .child(div().text_xs().child(format!("Reviewed head {} | {} unresolved threads / {} comments | snapshot at {} ms UTC", snapshot.head_sha, snapshot.threads.len(), snapshot.comment_count(), snapshot.collected_ms)))
            .child("Review and edit these instructions. Add to draft rechecks the head and every comment. Nothing is sent or executed. Review edits are temporary until added to the chat's saved draft.");
        if v.busy {
            pane = pane
                .child("Rechecking the review. Editing is paused; Cancel request keeps your text.");
        } else {
            pane = pane.child(
                div()
                    .h(px(260.))
                    .min_h(px(160.))
                    .flex_shrink_0()
                    .relative()
                    .child(v.fix_editor.clone())
                    .child(ui::layout_probe("pr-fix-editor")),
            );
        }
        pane = pane.child(
            div()
                .flex()
                .gap_2()
                .flex_wrap()
                .child(
                    ui::action(
                        "pr-fix-add",
                        "Recheck and add to target draft",
                        Some(Glyph::Plus),
                        false,
                        cx.listener(|this, _: &(), window, cx| {
                            this.pr_add_fixes_to_draft(window, cx)
                        }),
                    )
                    .relative()
                    .child(ui::layout_probe("pr-fix-add")),
                )
                .child(
                    ui::action(
                        "pr-fix-discard",
                        "Discard fix review...",
                        None,
                        false,
                        cx.listener(|this, _: &(), _, cx| {
                            if !this.pull_requests.busy
                                && !this.pull_requests.fix_editor.read(cx).is_composing()
                            {
                                this.pull_requests.fix_discard_confirm = true;
                                cx.notify();
                            }
                        }),
                    )
                    .relative()
                    .child(ui::layout_probe("pr-fix-discard")),
                ),
        );
        if v.fix_discard_confirm {
            pane = pane.child("Discard the collected snapshot and all review edits? The chat's existing draft will not change.")
                .child(ui::action("pr-fix-discard-confirm", "Discard review and edits", None, false,
                    cx.listener(|this, _: &(), _, cx| {
                        if this.pull_requests.busy || this.pull_requests.fix_editor.read(cx).is_composing() { return; }
                        this.pull_requests.fix = None;
                        this.pull_requests.fix_editor.update(cx, |entry, cx| entry.set_text(String::new(), cx));
                        this.pull_requests.fix_discard_confirm = false;
                        cx.notify();
                    })).relative().child(ui::layout_probe("pr-fix-discard-confirm")))
                .child(ui::action("pr-fix-keep", "Keep editing", None, false,
                    cx.listener(|this, _: &(), _, cx| { this.pull_requests.fix_discard_confirm = false; cx.notify(); })).relative().child(ui::layout_probe("pr-fix-keep")));
        }
        pane.into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn task() -> Task {
        Task {
            id: TaskId::new(),
            project_id: ProjectId::new(),
            title: "Review".into(),
            state: TaskState::Ready,
            thread_id: ThreadId::new(),
            agent_id: "generic-agent".into(),
            working_directory: PathBuf::from("/project"),
            updated_at_ms: 0,
            scope: Default::default(),
        }
    }
    #[test]
    fn ownership_rejects_task_thread_project_directory_or_archive_changes() {
        let source = task();
        assert!(same_task_owner(&source, &source));
        let mut current = source.clone();
        current.id = TaskId::new();
        assert!(!same_task_owner(&source, &current));
        let mut current = source.clone();
        current.thread_id = ThreadId::new();
        assert!(!same_task_owner(&source, &current));
        let mut current = source.clone();
        current.project_id = ProjectId::new();
        assert!(!same_task_owner(&source, &current));
        let mut current = source.clone();
        current.working_directory = PathBuf::from("/other");
        assert!(!same_task_owner(&source, &current));
        let mut current = source.clone();
        current.state = TaskState::Archived;
        assert!(!same_task_owner(&source, &current));
        let mut current = source.clone();
        current.title = "Renamed".into();
        assert!(same_task_owner(&source, &current));
    }
    #[test]
    fn draft_handoff_preserves_existing_text_and_rejects_bad_or_oversized_instructions() {
        let draft = "Keep my existing unsent request";
        assert_eq!(
            append_fix_draft(draft, "Reviewed fix").unwrap(),
            format!("{draft}\n\nReviewed fix")
        );
        assert_eq!(
            append_fix_draft("", "Reviewed fix").unwrap(),
            "Reviewed fix"
        );
        for invalid in ["", " \n ", "bad\0text"] {
            assert!(append_fix_draft(draft, invalid).is_err());
        }
        assert!(append_fix_draft(draft, &"x".repeat(MAX_FIX_PROMPT_BYTES + 1)).is_err());
        assert!(append_fix_draft(&"x".repeat(1024 * 1024), "Reviewed fix").is_err());
        assert_eq!(draft, "Keep my existing unsent request");
    }
}
