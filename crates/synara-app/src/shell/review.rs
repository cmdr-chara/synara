//! Native Git review inside the existing Environment. Every asynchronous result
//! belongs to a project/root and a request generation, never to the visible tab.
pub(super) mod diff;
mod repository;
use super::*;
use crate::ui::{self, Glyph, palette};
use std::time::{Duration, Instant};

#[derive(Default)]
pub(super) struct ReviewState {
    sessions: HashMap<ReviewScope, ReviewSession>,
    repositories: HashMap<ReviewScope, Entity<repository::RepositoryPanel>>,
    repository_open: bool,
}
struct ReviewSession {
    target: WorkspaceTarget,
    preferences: ReviewPreferences,
    input: Entity<TextEntry>,
    _subscription: Subscription,
    loaded: bool,
    restoring: bool,
    restore_error: Option<String>,
    edits: u64,
    saved_edits: u64,
    message_edits: u64,
    changed_at: Instant,
    saving: bool,
    save_error: Option<String>,
    reload_confirmation: bool,
    reads: ReadGeneration,
    mutating: bool,
    error: Option<String>,
    status: GitStatus,
    raw: String,
    diff: diff::Diff,
    scroll: gpui::UniformListScrollHandle,
}
impl ReviewSession {
    fn changed(&mut self) {
        self.edits = self.edits.wrapping_add(1);
        self.changed_at = Instant::now();
        // A failed revision check requires explicit recovery, not a retry loop.
    }
    fn dirty(&self) -> bool {
        self.edits != self.saved_edits
    }
    fn ready(&self) -> bool {
        self.loaded
            && !self.restoring
            && !self.reads.running
            && !self.mutating
            && self.error.is_none()
    }
}

#[derive(Default)]
struct ReadGeneration {
    requested: u64,
    running: bool,
}
impl ReadGeneration {
    fn request(&mut self) -> Option<u64> {
        self.requested = self.requested.wrapping_add(1);
        if self.running {
            return None;
        }
        self.running = true;
        Some(self.requested)
    }
    fn finish(&mut self, generation: u64) -> bool {
        self.running = false;
        generation == self.requested
    }
}

pub(super) enum Reply {
    Loaded {
        scope: ReviewScope,
        result: Result<ReviewPreferences, String>,
    },
    Saved {
        scope: ReviewScope,
        edits: u64,
        result: Result<ReviewPreferences, String>,
    },
    Refreshed {
        scope: ReviewScope,
        generation: u64,
        result: Result<(GitStatus, Option<PathBuf>, String), String>,
    },
    Mutated {
        scope: ReviewScope,
        submitted: Option<(u64, String)>,
        result: Result<(), String>,
    },
}
fn visible(entry: &GitEntry, staged: bool) -> bool {
    if staged {
        entry.staged()
    } else {
        entry.unstaged()
    }
}
fn conflicting(entry: &GitEntry) -> bool {
    entry.index_status == 'U'
        || entry.worktree_status == 'U'
        || matches!(
            (entry.index_status, entry.worktree_status),
            ('A', 'A') | ('D', 'D')
        )
}
fn current_selection(status: &GitStatus, path: Option<PathBuf>, staged: bool) -> Option<PathBuf> {
    path.filter(|path| {
        status
            .entries
            .iter()
            .any(|entry| entry.path == *path && visible(entry, staged))
    })
}
fn next_selection(
    status: &GitStatus,
    path: Option<&PathBuf>,
    staged: bool,
    backwards: bool,
) -> Option<PathBuf> {
    let entries: Vec<_> = status
        .entries
        .iter()
        .filter(|entry| visible(entry, staged))
        .collect();
    let index = path.and_then(|path| entries.iter().position(|entry| &entry.path == path));
    let next = match (index, backwards) {
        (Some(index), true) => index.saturating_sub(1),
        (Some(index), false) => (index + 1).min(entries.len().saturating_sub(1)),
        (None, true) => entries.len().saturating_sub(1),
        (None, false) => 0,
    };
    entries.get(next).map(|entry| entry.path.clone())
}
fn clear_submitted(current_edits: u64, current_text: &str, submitted: &(u64, String)) -> bool {
    current_edits == submitted.0 && current_text == submitted.1
}

impl Shell {
    pub(super) fn prepare_worktree_settings(&mut self, cx: &mut Context<Self>) {
        self.open_repository(cx);
        if let Some(scope) = self.review_scope()
            && let Some(panel) = self.review.repositories.get(&scope).cloned()
        {
            panel.update(cx, |panel, cx| panel.show_worktrees(cx));
        }
    }
    pub(super) fn worktree_settings_view(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let scope = self.review_scope();
        let panel = scope
            .as_ref()
            .and_then(|scope| self.review.repositories.get(scope));
        let cleanup = settings::card().child(settings::row(
            "Delete worktree on archive",
            "After Archive, remove a clean managed worktree only if the task has stopped and no other task uses it. Its branch remains available for recovery.",
            self.toggle(
                "archive-delete-worktree",
                "Delete worktree on archive",
                self.settings.value.general.delete_worktree_on_archive,
                |settings| {
                    settings.general.delete_worktree_on_archive =
                        !settings.general.delete_worktree_on_archive;
                },
                cx,
            ),
        ));
        match panel {
            Some(panel) => div().w_full().flex().flex_col().gap_3()
                .child(cleanup)
                .child("Worktrees in the selected repository. Existing execution and removal confirmations still apply.")
                .child(div().h(px(520.)).min_h(px(300.)).w_full().border_1().border_color(rgb(palette().border)).rounded_xl().overflow_hidden().child(panel.clone()))
                .into_any_element(),
            None => div().p_6().flex().flex_col().gap_3()
                .child(cleanup)
                .child(div().text_color(rgb(palette().muted)).child("Select a project before managing its worktrees. This page does not create a workspace or start a tool implicitly."))
                .into_any_element(),
        }
    }
    fn open_repository(&mut self, cx: &mut Context<Self>) {
        let (Some(scope), Some(target)) = (self.review_scope(), self.workspace_target()) else {
            return;
        };
        if self
            .review
            .sessions
            .get(&scope)
            .is_some_and(|session| session.mutating)
        {
            return;
        }
        let workspace = self.controller.workspace.clone();
        let runtime = self.runtime.clone();
        self.review
            .repositories
            .entry(scope.clone())
            .or_insert_with(|| {
                cx.new(|cx| {
                    repository::RepositoryPanel::new(target, scope.project, workspace, runtime, cx)
                })
            });
        if let Some(panel) = self.review.repositories.get(&scope).cloned() {
            panel.update(cx, |panel, cx| panel.show_repository_tabs(cx));
        }
        self.review.repository_open = true;
        self.focus_composer = false;
        cx.notify();
    }
    fn review_scope(&self) -> Option<ReviewScope> {
        Some(ReviewScope {
            project: self.project?,
            root: self.root()?,
        })
    }
    fn ensure_review(&mut self, cx: &mut Context<Self>) -> Option<ReviewScope> {
        let scope = self.review_scope()?;
        if !self.review.sessions.contains_key(&scope) {
            let target = self.workspace_target()?;
            let input =
                cx.new(|cx| TextEntry::new("Commit message", EntryMode::SingleLine, 36., cx));
            let key = scope.clone();
            let subscription = cx.subscribe(&input, move |this, input, event, cx| {
                if matches!(event, EntryEvent::Changed)
                    && let Some(session) = this.review.sessions.get_mut(&key)
                    && session.loaded
                {
                    let text = input.read(cx).text().to_owned();
                    if text != session.preferences.commit_message {
                        session.preferences.commit_message = text;
                        session.message_edits = session.message_edits.wrapping_add(1);
                        session.changed();
                    }
                }
                cx.notify();
            });
            self.review.sessions.insert(
                scope.clone(),
                ReviewSession {
                    target,
                    preferences: ReviewPreferences::default(),
                    input,
                    _subscription: subscription,
                    loaded: false,
                    restoring: false,
                    restore_error: None,
                    edits: 0,
                    saved_edits: 0,
                    message_edits: 0,
                    changed_at: Instant::now(),
                    saving: false,
                    save_error: None,
                    reload_confirmation: false,
                    reads: ReadGeneration::default(),
                    mutating: false,
                    error: None,
                    status: GitStatus::default(),
                    raw: String::new(),
                    diff: diff::Diff::default(),
                    scroll: gpui::UniformListScrollHandle::new(),
                },
            );
            self.load_review(scope.clone(), cx);
        }
        Some(scope)
    }
    fn load_review(&mut self, scope: ReviewScope, cx: &mut Context<Self>) {
        let Some(session) = self.review.sessions.get_mut(&scope) else {
            return;
        };
        if session.restoring || session.saving || session.mutating {
            return;
        }
        session.restoring = true;
        session.restore_error = None;
        session.reload_confirmation = false;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            let result = workspace
                .review_preferences(scope.clone())
                .await
                .map_err(|e| e.to_string());
            Ok(Update::Review(Box::new(Reply::Loaded { scope, result })))
        });
        cx.notify();
    }
    pub(super) fn refresh_git(&mut self, cx: &mut Context<Self>) {
        let Some(scope) = self.ensure_review(cx) else {
            return;
        };
        self.refresh_review_scope(scope, cx);
    }
    fn refresh_review_scope(&mut self, scope: ReviewScope, cx: &mut Context<Self>) {
        let Some(session) = self.review.sessions.get_mut(&scope) else {
            return;
        };
        if !session.loaded || session.mutating {
            return;
        }
        let Some(generation) = session.reads.request() else {
            cx.notify();
            return;
        };
        session.error = None;
        session.raw.clear();
        session.diff = diff::Diff::default();
        let target = session.target.clone();
        let staged = session.preferences.staged;
        let path = session.preferences.path.clone();
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            let result = async {
                let git = git_service(workspace, target).await?;
                let status = git.status().await?;
                let path = current_selection(&status, path, staged);
                let raw = git.diff(staged, path.clone()).await?;
                Ok::<_, WorkspaceError>((status, path, raw))
            }
            .await
            .map_err(|error| error.to_string());
            Ok(Update::Review(Box::new(Reply::Refreshed {
                scope,
                generation,
                result,
            })))
        });
        cx.notify();
    }
    pub(super) fn tick_review(&mut self, cx: &mut Context<Self>) {
        self.flush_review(false);
        if self.panel == Panel::Changes {
            self.ensure_review(cx);
        }
    }
    fn flush_review(&mut self, force: bool) {
        let pending: Vec<_> = self
            .review
            .sessions
            .iter_mut()
            .filter_map(|(scope, session)| {
                if !session.loaded
                    || !session.dirty()
                    || session.saving
                    || session.restoring
                    || session.save_error.is_some()
                    || (!force && session.changed_at.elapsed() < Duration::from_millis(500))
                {
                    return None;
                }
                session.saving = true;
                Some((scope.clone(), session.preferences.clone(), session.edits))
            })
            .collect();
        for (scope, value, edits) in pending {
            let workspace = self.controller.workspace.clone();
            self.job(async move {
                let result = workspace
                    .save_review_preferences(scope.clone(), value.revision, value)
                    .await
                    .map_err(|error| error.to_string());
                Ok(Update::Review(Box::new(Reply::Saved {
                    scope,
                    edits,
                    result,
                })))
            });
        }
    }
    pub(super) fn review_before_quit(&mut self, cx: &mut Context<Self>) -> bool {
        self.flush_review(true);
        let pending = self
            .review
            .repositories
            .values()
            .any(|panel| panel.read(cx).pending())
            || self
                .review
                .sessions
                .values()
                .any(|session| session.dirty() || session.saving || session.mutating);
        if pending {
            self.notice = Some("Git review still has unsaved drafts or an active operation. Finish recovery or wait for saving, then close again. No draft has been discarded.".into());
            self.close.cancel();
            cx.notify();
        }
        pending
    }
    pub(super) fn review_reply(&mut self, reply: Reply, cx: &mut Context<Self>) {
        match reply {
            Reply::Loaded { scope, result } => {
                let Some(session) = self.review.sessions.get_mut(&scope) else {
                    return;
                };
                session.restoring = false;
                match result {
                    Ok(value) => {
                        session.input.update(cx, |entry, cx| {
                            entry.set_text(value.commit_message.clone(), cx)
                        });
                        session.preferences = value;
                        session.loaded = true;
                        session.saved_edits = session.edits;
                        session.save_error = None;
                        session.message_edits = session.message_edits.wrapping_add(1);
                        self.refresh_review_scope(scope, cx);
                    }
                    Err(error) => session.restore_error = Some(error),
                }
            }
            Reply::Saved {
                scope,
                edits,
                result,
            } => {
                let Some(session) = self.review.sessions.get_mut(&scope) else {
                    return;
                };
                session.saving = false;
                match result {
                    Ok(value) => {
                        session.preferences.revision = value.revision;
                        session.saved_edits = edits;
                        session.save_error = None;
                    }
                    Err(error) => {
                        session.save_error = Some(error);
                        self.notice = Some("A Git review draft could not be saved. It remains open in Changes. Copy it or resolve the saved-version conflict before closing.".into());
                    }
                }
            }
            Reply::Refreshed {
                scope,
                generation,
                result,
            } => {
                let Some(session) = self.review.sessions.get_mut(&scope) else {
                    return;
                };
                if !session.reads.finish(generation) {
                    self.refresh_review_scope(scope, cx);
                    return;
                }
                match result {
                    Ok((status, path, raw)) => {
                        session.status = status;
                        if session.preferences.path != path {
                            session.preferences.path = path;
                            session.changed();
                        }
                        session.diff = diff::Diff::parse(&raw);
                        session.scroll.scroll_to_item(0, gpui::ScrollStrategy::Top);
                        session.raw = raw;
                        tracing::debug!(target: "synara_ui_layout", generation, staged = session.preferences.staged, selected = session.preferences.path.is_some(), "review-ready");
                    }
                    Err(error) => {
                        session.error = Some(error);
                        tracing::debug!(target: "synara_ui_layout", "review-failed");
                    }
                }
            }
            Reply::Mutated {
                scope,
                submitted,
                result,
            } => {
                let Some(session) = self.review.sessions.get_mut(&scope) else {
                    return;
                };
                session.mutating = false;
                match result {
                    Ok(()) => {
                        if let Some(submitted) = &submitted
                            && clear_submitted(
                                session.message_edits,
                                &session.preferences.commit_message,
                                submitted,
                            )
                        {
                            session.preferences.commit_message.clear();
                            session
                                .input
                                .update(cx, |entry, cx| entry.set_text(String::new(), cx));
                            session.message_edits = session.message_edits.wrapping_add(1);
                            session.changed();
                        }
                        self.refresh_review_scope(scope, cx);
                    }
                    Err(error) => {
                        session.error = Some(error);
                        tracing::debug!(target: "synara_ui_layout", "review-failed");
                    }
                }
            }
        }
        cx.notify();
    }
    fn review_select(&mut self, path: Option<PathBuf>, cx: &mut Context<Self>) {
        let Some(scope) = self.review_scope() else {
            return;
        };
        let Some(session) = self.review.sessions.get_mut(&scope) else {
            return;
        };
        if !session.loaded
            || session.restoring
            || session.mutating
            || session.preferences.path == path
        {
            return;
        }
        session.preferences.path = path;
        session.changed();
        self.refresh_review_scope(scope, cx);
    }
    fn review_side(&mut self, staged: bool, cx: &mut Context<Self>) {
        let Some(scope) = self.review_scope() else {
            return;
        };
        let Some(session) = self.review.sessions.get_mut(&scope) else {
            return;
        };
        if !session.loaded
            || session.restoring
            || session.mutating
            || session.preferences.staged == staged
        {
            return;
        }
        session.preferences.staged = staged;
        session.changed();
        self.refresh_review_scope(scope, cx);
    }
    fn review_index(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let Some(scope) = self.review_scope() else {
            return;
        };
        let Some(session) = self.review.sessions.get_mut(&scope) else {
            return;
        };
        let staged = session.preferences.staged;
        if !session.ready()
            || !session
                .status
                .entries
                .iter()
                .any(|entry| entry.path == path && visible(entry, staged))
        {
            return;
        }
        session.mutating = true;
        session.error = None;
        let target = session.target.clone();
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            let result = async {
                let git = git_service(workspace, target).await?;
                if staged {
                    git.unstage(path).await
                } else {
                    git.stage(path).await
                }
            }
            .await
            .map_err(|error| error.to_string());
            Ok(Update::Review(Box::new(Reply::Mutated {
                scope,
                submitted: None,
                result,
            })))
        });
        cx.notify();
    }
    fn review_commit(&mut self, cx: &mut Context<Self>) {
        let Some(scope) = self.review_scope() else {
            return;
        };
        let Some(session) = self.review.sessions.get_mut(&scope) else {
            return;
        };
        if !session.ready()
            || !session.status.entries.iter().any(GitEntry::staged)
            || session.status.entries.iter().any(conflicting)
        {
            return;
        }
        let message = session.preferences.commit_message.clone();
        if message.trim().is_empty() {
            self.notice = Some("Enter a commit message first.".into());
            cx.notify();
            return;
        }
        if let Err(error) = session.preferences.validate() {
            self.notice = Some(error.to_string());
            cx.notify();
            return;
        }
        session.mutating = true;
        let submitted = Some((session.message_edits, message.clone()));
        let target = session.target.clone();
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            let result = async { git_service(workspace, target).await?.commit(message).await }
                .await
                .map_err(|error| error.to_string());
            Ok(Update::Review(Box::new(Reply::Mutated {
                scope,
                submitted,
                result,
            })))
        });
        cx.notify();
    }
    fn review_to_draft(&mut self, cx: &mut Context<Self>) {
        if self.selected.is_none() || self.loading_task.is_some() || self.close != CloseState::Open
        {
            return;
        }
        let Some(scope) = self.review_scope() else {
            return;
        };
        let Some(session) = self
            .review
            .sessions
            .get(&scope)
            .filter(|session| session.ready())
        else {
            return;
        };
        let title = format!(
            "{} / {}",
            if session.preferences.staged {
                "staged"
            } else {
                "worktree"
            },
            session.preferences.path.as_ref().map_or_else(
                || "all files".into(),
                |path| serde_json::to_string(path).unwrap_or_else(|_| "selected file".into())
            )
        );
        match diff::draft_with_diff(self.composer.read(cx).text(), &title, &session.raw) {
            Ok(text) => {
                self.composer
                    .update(cx, |entry, cx| entry.set_text(text, cx));
                self.snapshot_draft(cx);
                self.notice = Some("Diff added to the unsent chat draft. Nothing was sent.".into());
            }
            Err(error) => self.notice = Some(error.into()),
        }
        cx.notify();
    }

    fn review_open_selected(&mut self, line: Option<usize>, cx: &mut Context<Self>) {
        if self.saving || self.explorer.modal_open() || self.close != CloseState::Open {
            return;
        }
        let Some(scope) = self.review_scope() else {
            return;
        };
        let Some(session) = self
            .review
            .sessions
            .get(&scope)
            .filter(|session| session.ready())
        else {
            return;
        };
        let Some(path) = session.preferences.path.as_ref() else {
            return;
        };
        if !session
            .status
            .entries
            .iter()
            .any(|entry| entry.path == *path && entry.worktree_status != 'D')
        {
            return;
        }
        let path = path.clone();
        if self.editors.index(&path).is_none() && self.editors.tabs.len() >= editors::MAX_TABS {
            self.notice = Some("Close an editor tab before opening another file.".into());
            cx.notify();
            return;
        }
        self.set_panel(Panel::Files, cx);
        if self.panel != Panel::Files {
            return;
        }
        self.editors.jump_after_open = line.map(|line| (path.clone(), line));
        self.open_file(path, cx);
    }

    pub(super) fn git_panel(&self, width: f32, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(scope) = self.review_scope() else {
            return div()
                .p_4()
                .child("Open a project to review its changes.")
                .into_any_element();
        };
        if self.review.repository_open
            && let Some(panel) = self.review.repositories.get(&scope)
        {
            let repository = panel.clone();
            return div()
                .flex()
                .flex_col()
                .flex_1()
                .min_h_0()
                .min_w_0()
                .child(
                    ui::action(
                        "repository-back",
                        "Back to changes",
                        Some(Glyph::Back),
                        false,
                        cx.listener(move |this, _: &(), _, cx| {
                            if !repository.read(cx).busy() {
                                this.review.repository_open = false;
                                this.refresh_git(cx);
                                cx.notify();
                            }
                        }),
                    )
                    .text_size(px(12.)),
                )
                .child(panel.clone())
                .into_any_element();
        }
        let Some(session) = self.review.sessions.get(&scope) else {
            return div()
                .p_4()
                .child("Opening Git review...")
                .into_any_element();
        };
        if !session.loaded {
            let key = scope.clone();
            return div()
                .p_4()
                .flex()
                .flex_col()
                .gap_3()
                .child(if session.restoring {
                    "Restoring review and commit draft..."
                } else {
                    "The saved review could not be restored. It has not been replaced."
                })
                .children(
                    session
                        .restore_error
                        .as_ref()
                        .map(|error| div().text_color(rgb(palette().error)).child(error.clone())),
                )
                .child(
                    ui::action(
                        "review-restore",
                        "Retry restore",
                        None,
                        false,
                        cx.listener(move |this, _: &(), _, cx| this.load_review(key.clone(), cx)),
                    )
                    .relative()
                    .child(ui::layout_probe("review-restore")),
                )
                .into_any_element();
        }
        let staged = session.preferences.staged;
        let selected = session.preferences.path.as_ref();
        let entries: Vec<_> = session
            .status
            .entries
            .iter()
            .filter(|entry| visible(entry, staged))
            .collect();
        let narrow = width < 680.;
        let busy = session.reads.running || session.mutating;
        let action_disabled = !session.ready();
        let root = div()
            .id("git-review")
            .relative()
            .child(ui::layout_probe("git-review"))
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .bg(gpui::rgba(0));
        let header = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(rgb(palette().border))
            .child(
                ui::action(
                    "review-repository",
                    "Repository",
                    Some(Glyph::BranchSimple),
                    false,
                    cx.listener(|this, _: &(), _, cx| this.open_repository(cx)),
                )
                .text_size(px(12.))
                .relative()
                .child(ui::layout_probe("review-repository")),
            )
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .text_ellipsis()
                    .text_size(px(13.))
                    .child(if session.status.branch.is_empty() {
                        "Changes".into()
                    } else {
                        session.status.branch.clone()
                    }),
            )
            .child(
                ui::action(
                    "review-worktree",
                    format!(
                        "Worktree {}",
                        session
                            .status
                            .entries
                            .iter()
                            .filter(|entry| entry.unstaged())
                            .count()
                    ),
                    None,
                    !staged,
                    cx.listener(|this, _: &(), _, cx| this.review_side(false, cx)),
                )
                .text_size(px(12.))
                .relative()
                .child(ui::layout_probe("review-worktree")),
            )
            .child(
                ui::action(
                    "review-staged",
                    format!(
                        "Staged {}",
                        session
                            .status
                            .entries
                            .iter()
                            .filter(|entry| entry.staged())
                            .count()
                    ),
                    None,
                    staged,
                    cx.listener(|this, _: &(), _, cx| this.review_side(true, cx)),
                )
                .text_size(px(12.))
                .relative()
                .child(ui::layout_probe("review-staged")),
            )
            .child(ui::chrome_button(
                "review-refresh",
                "Refresh changes",
                Glyph::Restore,
                session.mutating,
                cx.listener(|this, _: &(), _, cx| this.refresh_git(cx)),
            ));
        let files = div()
            .id("review-file-list")
            .role(gpui::Role::Group)
            .aria_label("Changed files. Use Up and Down to select a diff.")
            .relative()
            .child(ui::layout_probe("review-file-list"))
            .flex()
            .flex_col()
            .gap_1()
            .p_2()
            .min_w_0()
            .min_h_0()
            .overflow_y_scroll()
            .when(narrow, |el| el.h(px(142.)).flex_shrink_0().border_b_1())
            .when(!narrow, |el| el.w(px(218.)).flex_shrink_0().border_r_1())
            .border_color(rgb(palette().border))
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                let backwards = match event.keystroke.key.as_str() {
                    "up" => true,
                    "down" => false,
                    _ => return,
                };
                let Some(scope) = this.review_scope() else {
                    return;
                };
                let Some(session) = this.review.sessions.get(&scope) else {
                    return;
                };
                let path = next_selection(
                    &session.status,
                    session.preferences.path.as_ref(),
                    session.preferences.staged,
                    backwards,
                );
                this.review_select(path, cx);
                cx.stop_propagation();
            }))
            .child(
                ui::action(
                    "review-all",
                    "All changed files",
                    Some(Glyph::Changes),
                    selected.is_none(),
                    cx.listener(|this, _: &(), _, cx| this.review_select(None, cx)),
                )
                .text_size(px(12.))
                .relative()
                .child(ui::layout_probe("review-all")),
            )
            .children(entries.iter().enumerate().map(|(index, entry)| {
                let path = entry.path.clone();
                let action_path = path.clone();
                let label = format!(
                    "{}{}  {}",
                    entry.index_status,
                    entry.worktree_status,
                    entry.path.display()
                );
                div()
                    .flex()
                    .items_center()
                    .min_w_0()
                    .gap_1()
                    .child(
                        ui::action(
                            ("review-file", index),
                            label,
                            None,
                            selected == Some(&path),
                            cx.listener(move |this, _: &(), _, cx| {
                                this.review_select(Some(path.clone()), cx)
                            }),
                        )
                        .flex_1()
                        .text_size(px(12.))
                        .relative()
                        .child(ui::layout_probe_slot("review-file", index)),
                    )
                    .child(
                        ui::button_shell(
                            ("review-index", index),
                            if staged {
                                "Unstage entire file"
                            } else {
                                "Stage entire file"
                            },
                            false,
                        )
                        .size(px(26.))
                        .p_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(ui::icon(if staged { Glyph::Minimize } else { Glyph::Plus }))
                        .when(action_disabled, |el| el.opacity(0.4).cursor_default())
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if !action_disabled {
                                this.review_index(action_path.clone(), cx);
                            }
                        }))
                        .relative()
                        .child(ui::layout_probe_slot("review-index", index)),
                    )
            }))
            .children(
                (entries.is_empty() && !busy && session.error.is_none()).then(|| {
                    div()
                        .p_2()
                        .text_size(px(12.))
                        .text_color(rgb(palette().muted))
                        .child("No files in this view.")
                }),
            );
        let title = selected.map_or_else(
            || "All changed files".into(),
            |path| path.display().to_string(),
        );
        let file = selected.and_then(|path| {
            session
                .status
                .entries
                .iter()
                .find(|entry| &entry.path == path)
        });
        let toolbar = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_ellipsis()
                    .text_size(px(12.))
                    .child(title),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(rgb(palette().muted))
                    .child(format!(
                        "+{} / -{}",
                        session.diff.added, session.diff.removed
                    )),
            )
            .child(ui::chrome_button(
                "review-copy",
                "Copy exact diff",
                Glyph::Copy,
                action_disabled || session.raw.is_empty(),
                cx.listener(|this, _: &(), _, cx| {
                    if let Some(scope) = this.review_scope()
                        && let Some(session) = this
                            .review
                            .sessions
                            .get(&scope)
                            .filter(|session| session.ready())
                    {
                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(session.raw.clone()));
                    }
                }),
            ))
            .child(ui::chrome_button(
                "review-to-chat",
                "Add diff to unsent chat draft",
                Glyph::Chat,
                action_disabled || session.raw.is_empty() || self.selected.is_none(),
                cx.listener(|this, _: &(), _, cx| this.review_to_draft(cx)),
            ))
            .child(ui::chrome_button(
                "review-open",
                "Open selected file in Explorer",
                Glyph::Files,
                action_disabled
                    || selected.is_none()
                    || file.is_some_and(|entry| entry.worktree_status == 'D'),
                cx.listener(|this, _: &(), _, cx| this.review_open_selected(None, cx)),
            ))
            .child(
                ui::action(
                    "review-raw",
                    if session.preferences.raw {
                        "Raw"
                    } else {
                        "Unified"
                    },
                    None,
                    session.preferences.raw,
                    cx.listener(|this, _: &(), _, cx| {
                        if let Some(scope) = this.review_scope()
                            && let Some(session) = this.review.sessions.get_mut(&scope)
                        {
                            session.preferences.raw = !session.preferences.raw;
                            session.changed();
                            cx.notify();
                        }
                    }),
                )
                .text_size(px(11.))
                .relative()
                .child(ui::layout_probe("review-raw")),
            );
        let detail = div().flex_1().min_w_0().min_h_0().flex().flex_col().child(toolbar)
            .children(file.and_then(|entry| entry.original_path.as_ref()).map(|original|
                div().px_3().pb_2().text_size(px(12.)).text_color(rgb(palette().muted)).child(format!("Renamed from {}", original.display()))))
            .children(file.filter(|entry| conflicting(entry)).map(|_|
                div().px_3().pb_2().text_size(px(12.)).text_color(rgb(palette().error)).child("Unresolved conflict. Resolve the file before staging. Commits are blocked while conflicts remain.")))
            .children(session.error.as_ref().map(|error|
                div().p_3().text_size(px(12.)).text_color(rgb(palette().error)).child(format!("Could not complete Git review: {error}. Refresh to retry."))))
            .child(if busy {
                div().flex_1().p_4().text_size(px(12.)).text_color(rgb(palette().muted)).child(
                    if session.mutating { "Updating Git. Working files are not discarded." } else { "Reading changes..." }).into_any_element()
            } else if session.raw.is_empty() {
                div().flex_1().p_4().text_size(px(12.)).text_color(rgb(palette().muted)).child(
                    if session.error.is_some() { "The failed read is not a clean-repository result." }
                    else if file.is_some_and(|entry| entry.index_status == '?') { "Untracked file. Git has no text diff until the file is staged. Open it in Explorer to inspect it without staging." }
                    else { "No text diff in this view. Select another file or switch between Worktree and Staged." }).into_any_element()
            } else { self.review_diff(&scope, session, if narrow { width } else { width - 218. }, cx) })
            .children(session.diff.limited.then(|| div().px_3().py_1().text_size(px(11.)).text_color(rgb(palette().muted))
                .child("Preview limited to 6000 lines and 2000 characters per line. Copy retains the exact bounded Git response.")));
        let status = if session.saving {
            "Saving draft..."
        } else if session.save_error.is_some() {
            "Draft not saved"
        } else if session.dirty() {
            "Unsaved draft"
        } else {
            "Draft saved"
        };
        let key = scope.clone();
        let key_reload = scope.clone();
        let footer = div().px_3().py_2().border_t_1().border_color(rgb(palette().border)).flex().flex_col().gap_2()
            .child(if session.restoring { div().text_size(px(12.)).child("Reloading saved draft...") }
                else { div().relative().child(ui::layout_probe("review-commit-input")).child(session.input.clone()) })
            .child(div().flex().flex_wrap().items_center().gap_2()
                .child(div().flex_1().text_size(px(11.)).text_color(rgb(palette().muted)).child(status))
                .child(ui::chrome_button("review-copy-draft", "Copy commit draft", Glyph::Copy, false,
                    cx.listener(move |this, _: &(), _, cx| {
                        if let Some(session) = this.review.sessions.get(&key) {
                            cx.write_to_clipboard(gpui::ClipboardItem::new_string(session.preferences.commit_message.clone()));
                        }
                    })))
                .child(ui::action("review-commit", "Commit staged", Some(Glyph::Check), false,
                    cx.listener(|this, _: &(), _, cx| this.review_commit(cx))).text_size(px(12.)).relative().child(ui::layout_probe("review-commit"))
                    .when(action_disabled || !session.status.entries.iter().any(GitEntry::staged) || session.status.entries.iter().any(conflicting), |el| el.opacity(0.45)))
            )
            .child(div().text_size(px(11.)).text_color(rgb(palette().muted)).child("Local commit only. Hooks and signing are disabled. No push or agent permission is implied."))
            .children(session.save_error.as_ref().map(|error| div().text_size(px(12.)).text_color(rgb(palette().error)).child(error.clone())))
            .when(session.save_error.is_some(), |el| el.child(div().flex().flex_wrap().gap_2()
                .child(ui::action("review-retry-save", "Retry save", None, false, cx.listener(|this, _: &(), _, cx| {
                    if let Some(scope) = this.review_scope() && let Some(session) = this.review.sessions.get_mut(&scope) { session.save_error = None; }
                    this.flush_review(true); cx.notify();
                })))
                .child(ui::action("review-reload", if session.reload_confirmation { "Discard local draft and reload" } else { "Reload saved version..." }, None, false,
                    cx.listener(move |this, _: &(), _, cx| {
                        let Some(session) = this.review.sessions.get_mut(&key_reload) else { return };
                        if session.saving || session.mutating { return; }
                        if session.reload_confirmation { this.load_review(key_reload.clone(), cx); }
                        else { session.reload_confirmation = true; cx.notify(); }
                    })).relative().child(ui::layout_probe("review-reload")))
                .when(session.reload_confirmation, |el| el.child(ui::action("review-keep-draft", "Keep local draft", None, false,
                    cx.listener(|this, _: &(), _, cx| {
                        if let Some(scope) = this.review_scope() && let Some(session) = this.review.sessions.get_mut(&scope) { session.reload_confirmation = false; cx.notify(); }
                    }))))));
        root.child(header)
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .when(narrow, |el| el.flex_col())
                    .child(files)
                    .child(detail),
            )
            .child(footer)
            .into_any_element()
    }
    fn review_diff(
        &self,
        scope: &ReviewScope,
        session: &ReviewSession,
        width: f32,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let entity = cx.entity();
        let scope = scope.clone();
        let raw = session.preferences.raw;
        let columns = session
            .diff
            .lines
            .iter()
            .map(|line| line.text.chars().count())
            .max()
            .unwrap_or(0);
        let content_width = (columns as f32 * 8. + if raw { 24. } else { 112. }).max(width);
        let list = gpui::uniform_list(
            "review-diff",
            session.diff.lines.len(),
            move |range, _, cx| {
                entity.update(cx, |this, cx| {
                    let Some(session) = this.review.sessions.get(&scope) else {
                        return Vec::new();
                    };
                    range
                        .filter_map(|index| session.diff.lines.get(index).map(|line| (index, line)))
                        .map(|(index, line)| {
                            let color = match line.kind {
                                diff::Kind::Added => palette().focus,
                                diff::Kind::Removed => palette().error,
                                diff::Kind::Hunk => palette().muted,
                                _ => palette().text,
                            };
                            let line_number =
                                line.new.and_then(|number| usize::try_from(number).ok());
                            let jump = session.preferences.path.is_some() && line_number.is_some();
                            div()
                                .id(("review-diff-line", index))
                                .h(px(21.))
                                .w_full()
                                .min_w_0()
                                .flex()
                                .items_center()
                                .font_family(ui::code_font())
                                .text_size(px(12.))
                                .text_color(rgb(if raw { palette().text } else { color }))
                                .when(!raw && line.kind == diff::Kind::Added, |el| {
                                    el.bg(rgb(palette().notice_surface))
                                })
                                .when(!raw && line.kind == diff::Kind::Removed, |el| {
                                    el.bg(rgb(palette().error_surface))
                                })
                                .when(!raw, |el| {
                                    el.child(
                                        div()
                                            .w(px(44.))
                                            .flex_shrink_0()
                                            .text_right()
                                            .pr_2()
                                            .text_color(rgb(palette().muted))
                                            .child(
                                                line.old
                                                    .map_or_else(String::new, |n| n.to_string()),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .w(px(44.))
                                            .flex_shrink_0()
                                            .text_right()
                                            .pr_2()
                                            .text_color(rgb(palette().muted))
                                            .child(
                                                line.new
                                                    .map_or_else(String::new, |n| n.to_string()),
                                            ),
                                    )
                                })
                                .child(div().px_2().child(line.text.clone()))
                                .when(jump, |el| {
                                    el.cursor_pointer().on_click(cx.listener(
                                        move |this, _, _, cx| {
                                            this.review_open_selected(line_number, cx);
                                        },
                                    ))
                                })
                        })
                        .collect::<Vec<_>>()
                })
            },
        )
        .track_scroll(&session.scroll)
        .w(px(content_width))
        .h_full();
        div()
            .id("review-diff-scroll")
            .flex_1()
            .min_h_0()
            .min_w_0()
            .overflow_x_scroll()
            .child(list)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rapid_selection_coalesces_reads_and_rejects_the_stale_result() {
        let mut reads = ReadGeneration::default();
        let first = reads.request().unwrap();
        assert!(reads.request().is_none());
        assert!(reads.request().is_none());
        assert!(!reads.finish(first));
        let latest = reads.request().unwrap();
        assert!(reads.finish(latest));
        assert!(!reads.running);
    }
    fn status() -> GitStatus {
        GitStatus {
            branch: "main".into(),
            entries: vec![
                GitEntry {
                    path: "both.rs".into(),
                    original_path: None,
                    index_status: 'M',
                    worktree_status: 'M',
                },
                GitEntry {
                    path: "staged.rs".into(),
                    original_path: None,
                    index_status: 'A',
                    worktree_status: ' ',
                },
                GitEntry {
                    path: "new.rs".into(),
                    original_path: None,
                    index_status: '?',
                    worktree_status: '?',
                },
            ],
        }
    }
    #[test]
    fn review_filters_both_sides_and_keeps_selection_only_when_present() {
        let value = status();
        assert_eq!(
            value
                .entries
                .iter()
                .filter(|entry| visible(entry, true))
                .count(),
            2
        );
        assert_eq!(
            value
                .entries
                .iter()
                .filter(|entry| visible(entry, false))
                .count(),
            2
        );
        assert_eq!(
            current_selection(&value, Some("staged.rs".into()), false),
            None
        );
        assert_eq!(
            current_selection(&value, Some("both.rs".into()), true),
            Some("both.rs".into())
        );
    }
    #[test]
    fn review_keyboard_selection_is_bounded_and_side_specific() {
        let value = status();
        assert_eq!(
            next_selection(&value, None, false, false),
            Some("both.rs".into())
        );
        assert_eq!(
            next_selection(&value, Some(&"both.rs".into()), false, false),
            Some("new.rs".into())
        );
        assert_eq!(
            next_selection(&value, Some(&"new.rs".into()), false, false),
            Some("new.rs".into())
        );
        assert_eq!(
            next_selection(&GitStatus::default(), None, true, false),
            None
        );
    }
    #[test]
    fn successful_commit_never_clears_a_newer_draft_even_if_text_matches_again() {
        let submitted = (7, "Commit this".into());
        assert!(clear_submitted(7, "Commit this", &submitted));
        assert!(!clear_submitted(8, "Commit this", &submitted));
        assert!(!clear_submitted(7, "New draft", &submitted));
    }
}
