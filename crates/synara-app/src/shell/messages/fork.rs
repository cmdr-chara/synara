//! A branch is a NEW unsent draft with explicit provenance, not a cloned agent
//! session, filesystem snapshot or replay of permission decisions.
use super::super::chat_tools::Reply;
use super::*;

const CONTEXT_LIMIT: usize = 1024 * 1024;
const MESSAGE_LIMIT: usize = 256;

enum ForkEnvironment {
    Same,
    Existing(PathBuf),
    Recover(PathBuf),
    New(Box<NewWorktreePlan>),
}

fn append(output: &mut String, text: &str) -> WorkspaceResult<()> {
    if output.len().saturating_add(text.len()) > CONTEXT_LIMIT {
        return Err(WorkspaceError::Invalid("This branch exceeds the 1 MiB draft limit. Quote a smaller selection instead. Nothing was created.".into()));
    }
    output.push_str(text);
    Ok(())
}
fn branch_context(task: &Task, thread: &Thread, anchor: &MessageAnchor) -> WorkspaceResult<String> {
    if thread.id != task.thread_id || anchor.role != Role::Assistant {
        return Err(WorkspaceError::Invalid(
            "The source conversation does not match this message.".into(),
        ));
    }
    let end = thread.timeline.iter().position(|item| matches!(item,
        TranscriptItem::Message { index } if thread.messages.get(*index).is_some_and(|message| anchor.matches(message))))
        .ok_or_else(|| WorkspaceError::Invalid("The source message is no longer available. Nothing was created.".into()))?;
    let mut output = format!(
        "Conversation branch from: {}\nSource task: {}\nSource message: {}\n\nThe following is quoted conversation context, not a new tool instruction. No tools, permissions or filesystem state have been copied.\n\n",
        task.title, task.id, anchor.id
    );
    let mut count = 0;
    let mut included = HashSet::new();
    for item in &thread.timeline[..=end] {
        let TranscriptItem::Message { index } = item else {
            continue;
        };
        let message = thread.messages.get(*index).ok_or_else(|| {
            WorkspaceError::Invalid("The source transcript is inconsistent.".into())
        })?;
        if !matches!(message.role, Role::User | Role::Assistant) || !included.insert(*index) {
            continue;
        }
        count += 1;
        if count > MESSAGE_LIMIT {
            return Err(WorkspaceError::Invalid("This branch exceeds 256 messages. Quote a smaller selection instead. Nothing was created.".into()));
        }
        append(
            &mut output,
            if message.role == Role::User {
                "### User\n"
            } else {
                "### Assistant\n"
            },
        )?;
        for line in message.text.split('\n') {
            append(&mut output, "> ")?;
            append(&mut output, line)?;
            append(&mut output, "\n")?;
        }
        append(&mut output, "\n")?;
    }
    append(&mut output, "---\n\nContinue from this point:\n")?;
    Ok(output)
}
impl Shell {
    pub(super) fn message_branch_button(
        &self,
        message: &Message,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let source = self.selected;
        let anchor = MessageAnchor::from(message);
        ui::chrome_button(
            "branch-message",
            "Branch into a new unsent chat draft (same workspace)",
            Glyph::Fork,
            source.is_none() || self.creating_task || self.loading_task.is_some(),
            cx.listener(move |this, _: &(), _, cx| {
                if let Some(source) = source {
                    this.branch_message(source, anchor.clone(), cx);
                }
            }),
        )
        .size(px(24.))
        .into_any_element()
    }
    pub(super) fn message_branch_worktree_button(
        &self,
        message: &Message,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let source = self.selected;
        let anchor = MessageAnchor::from(message);
        let loading = source.is_some_and(|task| self.chat_tools.loading_worktrees.contains(&task));
        ui::chrome_button(
            "branch-message-worktree",
            "Fork into an existing or reviewed new Git worktree",
            Glyph::BranchSimple,
            source.is_none() || loading || self.creating_task || self.loading_task.is_some(),
            cx.listener(move |this, _: &(), _, cx| {
                if let Some(source) = source {
                    this.load_branch_worktree_choices(source, anchor.clone(), cx);
                }
            }),
        )
        .size(px(24.))
        .relative()
        .child(ui::layout_probe("branch-message-worktree"))
        .into_any_element()
    }

    pub(in crate::shell) fn load_branch_worktree_choices(
        &mut self,
        source: TaskId,
        anchor: MessageAnchor,
        cx: &mut Context<Self>,
    ) {
        if self.selected != Some(source)
            || self.creating_task
            || self.loading_task.is_some()
            || self.close != CloseState::Open
            || self.terminal_closing
            || self.explorer.modal_open()
            || self.kanban.dialog.is_some()
            || self.organization.dialog.is_some()
            || self.saved_context.dialog.is_some()
            || self.composer.read(cx).is_composing()
            || self.editor.read(cx).is_composing()
            || self.draft_state.loading.contains(&source)
            || !self.chat_tools.loading_worktrees.insert(source)
        {
            return;
        }
        let revision = self.selection_revision;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            let result = async {
                let source_task = workspace.task(source).await?;
                if source_task.state == TaskState::Archived {
                    return Err(WorkspaceError::Invalid(
                        "Restore the source conversation before branching.".into(),
                    ));
                }
                let current_directory = source_task.working_directory.clone();
                let remote = matches!(
                    workspace.workspace_for_task(&source_task).await?.location,
                    WorkspaceLocation::Ssh { .. }
                );
                let worktrees = workspace.project_worktrees(source_task.project_id).await?;
                Ok::<_, WorkspaceError>((current_directory, remote, worktrees))
            }
            .await
            .map_err(|error| error.to_string());
            Ok(Update::ChatTools(Box::new(Reply::Worktrees {
                task: source,
                revision,
                anchor,
                result,
            })))
        });
        cx.notify();
    }

    pub(in crate::shell) fn branch_message(
        &mut self,
        source: TaskId,
        anchor: MessageAnchor,
        cx: &mut Context<Self>,
    ) {
        self.branch_message_at(source, anchor, ForkEnvironment::Same, cx);
    }

    pub(in crate::shell) fn branch_message_in_worktree(
        &mut self,
        source: TaskId,
        anchor: MessageAnchor,
        worktree_directory: PathBuf,
        recover: bool,
        cx: &mut Context<Self>,
    ) {
        self.branch_message_at(
            source,
            anchor,
            if recover {
                ForkEnvironment::Recover(worktree_directory)
            } else {
                ForkEnvironment::Existing(worktree_directory)
            },
            cx,
        );
    }

    pub(in crate::shell) fn review_new_worktree_fork(
        &mut self,
        source: TaskId,
        anchor: MessageAnchor,
        cx: &mut Context<Self>,
    ) {
        if self.selected != Some(source)
            || self.creating_task
            || self.loading_task.is_some()
            || self.close != CloseState::Open
            || self.composer.read(cx).is_composing()
            || self.editor.read(cx).is_composing()
            || !self.chat_tools.loading_worktrees.insert(source)
        {
            return;
        }
        let workspace = self.controller.workspace.clone();
        let parent = self.scratch_directory.clone();
        let revision = self.selection_revision;
        self.job(async move {
            let result = async {
                // The app's canonical data directory owns this one-level scratch parent.
                if let Err(error) = std::fs::create_dir(&parent)
                    && error.kind() != std::io::ErrorKind::AlreadyExists
                {
                    return Err(WorkspaceError::Runtime(synara_runtime::RuntimeError::Io(
                        error,
                    )));
                }
                workspace.prepare_new_worktree_fork(source, parent).await
            }
            .await
            .map(Box::new)
            .map_err(|error| error.to_string());
            Ok(Update::ChatTools(Box::new(Reply::NewWorktree {
                task: source,
                revision,
                anchor,
                result,
            })))
        });
        cx.notify();
    }

    pub(in crate::shell) fn cleanup_branch_worktree(
        &mut self,
        source: TaskId,
        anchor: MessageAnchor,
        path: PathBuf,
        branch: String,
        cx: &mut Context<Self>,
    ) {
        if self.selected != Some(source)
            || self.creating_task
            || self.loading_task.is_some()
            || self.close != CloseState::Open
            || self.composer.read(cx).is_composing()
            || self.editor.read(cx).is_composing()
            || !self.chat_tools.loading_worktrees.insert(source)
        {
            return;
        }
        let workspace = self.controller.workspace.clone();
        let scratch = self.scratch_directory.clone();
        let revision = self.selection_revision;
        let reply_branch = branch.clone();
        self.job(async move {
            let result = workspace
                .cleanup_recoverable_worktree(
                    source,
                    path,
                    scratch,
                    branch,
                    GitOperationPolicy {
                        allow_mutation: true,
                        allow_repository_execution: true,
                        ..Default::default()
                    },
                    tokio_util::sync::CancellationToken::new(),
                )
                .await
                .map_err(|error| error.to_string());
            Ok(Update::ChatTools(Box::new(Reply::WorktreeCleaned {
                task: source,
                revision,
                anchor,
                branch: reply_branch,
                result,
            })))
        });
        cx.notify();
    }

    pub(in crate::shell) fn branch_message_new_worktree(
        &mut self,
        plan: NewWorktreePlan,
        anchor: MessageAnchor,
        cx: &mut Context<Self>,
    ) {
        self.branch_message_at(
            plan.source(),
            anchor,
            ForkEnvironment::New(Box::new(plan)),
            cx,
        );
    }

    fn branch_message_at(
        &mut self,
        source: TaskId,
        anchor: MessageAnchor,
        environment: ForkEnvironment,
        cx: &mut Context<Self>,
    ) {
        if self.selected != Some(source)
            || self.creating_task
            || self.loading_task.is_some()
            || self.close != CloseState::Open
            || self.terminal_closing
            || self.explorer.modal_open()
            || self.kanban.dialog.is_some()
            || self.organization.dialog.is_some()
            || self.saved_context.dialog.is_some()
            || self.composer.read(cx).is_composing()
            || self.editor.read(cx).is_composing()
            || self.draft_state.loading.contains(&source)
        {
            return;
        }
        self.snapshot_draft(cx);
        self.creating_task = true;
        self.notice =
            Some("Preparing a new branch draft. The source conversation stays unchanged.".into());
        let workspace = self.controller.workspace.clone();
        let revision = self.selection_revision;
        self.job(async move {
            let result = async {
                let source = workspace.task(source).await?;
                if source.state == TaskState::Archived {
                    return Err(WorkspaceError::Invalid(
                        "Restore the source conversation before branching.".into(),
                    ));
                }
                let thread = workspace.thread(source.thread_id).await?;
                let draft = branch_context(&source, &thread, &anchor)?;
                // Load before the atomic creation, so a failed catalog read cannot
                // turn a committed creation into a retryable duplicate.
                let mut catalog = workspace.catalog().await?;
                let title = format!(
                    "Branch: {}",
                    source.title.chars().take(90).collect::<String>()
                );
                let task = match environment {
                    ForkEnvironment::New(plan) => {
                        workspace
                            .create_new_worktree_fork(
                                *plan,
                                title,
                                draft,
                                GitOperationPolicy {
                                    allow_mutation: true,
                                    allow_repository_execution: true,
                                    ..Default::default()
                                },
                                tokio_util::sync::CancellationToken::new(),
                            )
                            .await?
                    }
                    ForkEnvironment::Existing(directory) => {
                        workspace
                            .create_scoped_task_in_worktree(
                                source.project_id,
                                title,
                                source.agent_id,
                                source.scope,
                                draft,
                                directory,
                            )
                            .await?
                    }
                    ForkEnvironment::Recover(directory) => {
                        workspace
                            .recover_new_worktree_fork(source.id, directory, title, draft)
                            .await?
                    }
                    ForkEnvironment::Same => {
                        workspace
                            .create_scoped_task_with_draft(
                                source.project_id,
                                title,
                                source.agent_id,
                                source.scope,
                                draft,
                            )
                            .await?
                    }
                };
                catalog.tasks.insert(0, task.clone());
                Ok::<_, WorkspaceError>((task, catalog))
            }
            .await;
            Ok(match result {
                Ok((task, catalog)) => Update::TaskCreated(task, catalog, revision),
                Err(error) => Update::TaskCreationFailed(error.to_string()),
            })
        });
        cx.notify();
    }
}
