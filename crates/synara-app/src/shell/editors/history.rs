//! Task/root/tab-scoped, inert committed-file inspection. Editor buffers remain owned
//! by the existing editor while a read-only historical snapshot is visible.
use super::*;
use tokio_util::sync::CancellationToken;

#[derive(Clone, PartialEq, Eq)]
pub(super) struct Scope {
    task: Option<TaskId>,
    project: Option<ProjectId>,
    root: PathBuf,
    path: PathBuf,
    tab: u64,
}
pub(super) enum Data {
    History(GitFileHistory),
    Revision(GitFileRevision),
    Blame(GitFileBlame),
}
pub(in crate::shell) struct Reply {
    scope: Scope,
    generation: u64,
    result: Result<Data, String>,
}
struct Revision {
    source: GitFileRevision,
    lines: Vec<String>,
    limited: bool,
    blame: Option<GitFileBlame>,
}
#[derive(Default)]
pub(super) struct State {
    scope: Option<Scope>,
    generation: u64,
    cancel: CancellationToken,
    open: bool,
    pending: bool,
    error: Option<String>,
    history: Option<GitFileHistory>,
    revision: Option<Revision>,
}
impl State {
    pub fn clear(&mut self) {
        let generation = self.generation.wrapping_add(1);
        self.cancel.cancel();
        *self = Self::default();
        self.generation = generation;
    }
}
impl Drop for State {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}
impl Shell {
    pub(in crate::shell) fn cancel_editor_history(&mut self) {
        self.editors.history.clear();
    }
    fn history_scope(&self) -> Option<Scope> {
        let WorkspaceTarget::Local { root } = self.workspace_target()? else {
            return None;
        };
        Some(Scope {
            task: self.selected,
            project: self.project,
            root,
            path: self.document.as_ref()?.path.clone(),
            tab: self.editors.active?,
        })
    }
    fn history_matches(&self) -> bool {
        self.editors.history.scope.is_some() && self.history_scope() == self.editors.history.scope
    }
    pub(super) fn open_editor_history(&mut self, cx: &mut Context<Self>) {
        if self.close != CloseState::Open || self.editor.read(cx).is_composing() {
            return;
        }
        let Some(scope) = self.history_scope() else {
            self.notice = Some("File history currently requires a local repository.".into());
            cx.notify();
            return;
        };
        self.editors.history.clear();
        let state = &mut self.editors.history;
        state.scope = Some(scope.clone());
        state.open = true;
        state.pending = true;
        let generation = state.generation;
        let cancel = state.cancel.clone();
        self.job(async move {
            let result = GitService::new(scope.root.clone())
                .file_history(scope.path.clone(), &cancel)
                .await
                .map(Data::History)
                .map_err(|e| e.to_string());
            Ok(Update::EditorHistory(Box::new(Reply {
                scope,
                generation,
                result,
            })))
        });
        cx.notify();
    }
    fn read_editor_revision(
        &mut self,
        commit: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.history_matches()
            || self.close != CloseState::Open
            || self.editor.read(cx).is_composing()
        {
            return;
        }
        let state = &mut self.editors.history;
        let (Some(scope), Some(history)) = (state.scope.clone(), state.history.clone()) else {
            return;
        };
        if state.pending {
            return;
        }
        window.focus(&self.navigation.root_focus, cx);
        self.editors.focus_editor = false;
        state.cancel.cancel();
        state.cancel = Default::default();
        let cancel = state.cancel.clone();
        state.generation = state.generation.wrapping_add(1);
        let generation = state.generation;
        state.pending = true;
        state.error = None;
        state.revision = None;
        self.job(async move {
            let result = GitService::new(scope.root.clone())
                .file_revision(&history, &commit, &cancel)
                .await
                .map(Data::Revision)
                .map_err(|e| e.to_string());
            Ok(Update::EditorHistory(Box::new(Reply {
                scope,
                generation,
                result,
            })))
        });
        cx.notify();
    }
    fn read_editor_blame(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.history_matches()
            || self.close != CloseState::Open
            || self.editor.read(cx).is_composing()
        {
            return;
        }
        let state = &mut self.editors.history;
        let (Some(scope), Some(history), Some(revision)) = (
            state.scope.clone(),
            state.history.clone(),
            state.revision.as_ref(),
        ) else {
            return;
        };
        if state.pending || revision.blame.is_some() {
            return;
        }
        let commit = revision.source.commit.clone();
        window.focus(&self.navigation.root_focus, cx);
        self.editors.focus_editor = false;
        state.cancel.cancel();
        state.cancel = Default::default();
        let cancel = state.cancel.clone();
        state.generation = state.generation.wrapping_add(1);
        let generation = state.generation;
        state.pending = true;
        state.error = None;
        self.job(async move {
            let result = GitService::new(scope.root.clone())
                .file_blame(&history, &commit, &cancel)
                .await
                .map(Data::Blame)
                .map_err(|e| e.to_string());
            Ok(Update::EditorHistory(Box::new(Reply {
                scope,
                generation,
                result,
            })))
        });
        cx.notify();
    }
    pub(in crate::shell) fn editor_history_reply(&mut self, reply: Reply, cx: &mut Context<Self>) {
        if reply.generation != self.editors.history.generation
            || Some(reply.scope) != self.history_scope()
            || !self.history_matches()
        {
            return;
        }
        let state = &mut self.editors.history;
        state.pending = false;
        match reply.result {
            Ok(Data::History(history)) => state.history = Some(history),
            Ok(Data::Revision(source)) => {
                let limited = source.text.lines().count() > 6000
                    || source.text.lines().any(|line| line.chars().count() > 2000);
                let lines = source
                    .text
                    .lines()
                    .take(6000)
                    .map(|line| line.chars().take(2000).collect())
                    .collect();
                state.revision = Some(Revision {
                    source,
                    lines,
                    limited,
                    blame: None,
                });
            }
            Ok(Data::Blame(source)) => {
                let current = state
                    .history
                    .as_ref()
                    .is_some_and(|history| history.head == source.head)
                    && state.revision.as_ref().is_some_and(|revision| {
                        revision.source.commit == source.commit
                            && revision.lines.len() == source.lines.len()
                    });
                if current {
                    if let Some(revision) = state.revision.as_mut() {
                        revision.blame = Some(source);
                    }
                } else {
                    state.error = Some(
                        "Line blame is stale. Select the revision again and retry.".into(),
                    );
                }
            }
            Err(error) => state.error = Some(error),
        }
        cx.notify();
    }
    pub(super) fn editor_history_toolbar(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let state = &self.editors.history;
        let mut panel = div().flex().flex_col().gap_1();
        if !state.open || !self.history_matches() {
            return panel.into_any_element();
        }
        panel = panel.child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_size(px(11.))
                        .flex_1()
                        .child("Committed file history. Your current buffer is unchanged."),
                )
                .child(
                    ui::button("editor-history-close", "Back to buffer", false)
                        .relative()
                        .child(ui::layout_probe("editor-history-close"))
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.editors.history.clear();
                            window.focus(&this.editor.read(cx).focus_handle(cx), cx);
                            cx.notify();
                        })),
                ),
        );
        if state.pending {
            panel = panel.child("Reading Git objects...");
        }
        if let Some(error) = &state.error {
            panel = panel.child(
                div()
                    .text_size(px(11.))
                    .text_color(rgb(palette().error))
                    .child(error.clone()),
            );
        }
        if let Some(history) = &state.history {
            panel = panel.child(div().text_size(px(11.)).text_color(rgb(palette().muted)).child(format!("HEAD {} · exact-path history{}", &history.head[..8], if history.limited { " · latest 50 commits only" } else { "" })))
                .child(div().id("editor-history-list").relative().child(ui::layout_probe("editor-history-list")).h(px(125.)).overflow_y_scroll().flex().flex_col()
                    .children(history.commits.iter().enumerate().map(|(index, commit)| {
                        let id = commit.id.clone();
                        ui::button(("editor-history-commit", index), format!("{}  {}  {}  {}", &id[..8], commit.author, commit.date.as_deref().unwrap_or("Date unavailable"), commit.subject), false)
                            .text_size(px(11.)).flex_shrink_0().relative().child(ui::layout_probe_slot("editor-history-commit", index))
                            .on_click(cx.listener(move |this, _, window, cx| this.read_editor_revision(id.clone(), window, cx)))
                    })).children(history.commits.is_empty().then(|| div().child("No committed history at this exact path. Renames are not followed."))));
        }
        panel.into_any_element()
    }
    pub(in crate::shell) fn editor_history_content(
        &self,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        if !self.history_matches() {
            return None;
        }
        let revision = self.editors.history.revision.as_ref()?;
        let entity = cx.entity();
        let width = (revision
            .lines
            .iter()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0) as f32
            * 8.
            + if revision.blame.is_some() { 300. } else { 70. })
            .max(300.);
        let list = gpui::uniform_list(
            "editor-history-lines",
            revision.lines.len(),
            move |range, _, cx| {
                entity.update(cx, |this, _| {
                    let Some(revision) = &this.editors.history.revision else {
                        return Vec::new();
                    };
                    range
                        .filter_map(|index| {
                            revision.lines.get(index).map(|text| {
                                let attribution = revision
                                    .blame
                                    .as_ref()
                                    .and_then(|blame| blame.lines.get(index))
                                    .map(|line| {
                                        let author = line.author.chars().take(20).collect::<String>();
                                        format!("{} {:<20}  ", &line.commit[..8], author)
                                    })
                                    .unwrap_or_default();
                                div()
                                    .h(px(20.))
                                    .w_full()
                                    .font_family(ui::code_font())
                                    .text_size(px(12.))
                                    .child(format!("{:>5}  {}{}", index + 1, attribution, text))
                            })
                        })
                        .collect::<Vec<_>>()
                })
            },
        )
        .w(px(width))
        .h_full();
        Some(
            div()
                .id("editor-history-preview")
                .relative()
                .child(ui::layout_probe("editor-history-preview"))
                .flex_1()
                .min_h_0()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .items_center()
                        .child(div().text_size(px(11.)).flex_1().child(format!(
                            "Read-only revision {}{}",
                            &revision.source.commit[..8],
                            if revision.limited {
                                " · preview shortened, Copy retains complete text"
                            } else {
                                ""
                            }
                        )))
                        .child(
                            ui::button(
                                "editor-history-blame",
                                if revision
                                    .blame
                                    .as_ref()
                                    .is_some_and(|blame| blame.limited)
                                {
                                    "Blame first 6000 lines"
                                } else if revision.blame.is_some() {
                                    "Line blame loaded"
                                } else if self.editors.history.pending {
                                    "Loading line blame..."
                                } else {
                                    "Show line blame"
                                },
                                self.editors.history.pending || revision.blame.is_some(),
                            )
                            .relative()
                            .child(ui::layout_probe("editor-history-blame"))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.read_editor_blame(window, cx)
                            })),
                        )
                        .child(
                            ui::button("editor-history-copy", "Copy revision", false)
                                .relative()
                                .child(ui::layout_probe("editor-history-copy"))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if this.history_matches()
                                        && let Some(revision) = &this.editors.history.revision
                                    {
                                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                            revision.source.text.clone(),
                                        ));
                                    }
                                })),
                        ),
                )
                .child(
                    div()
                        .id("editor-history-scroll")
                        .flex_1()
                        .min_h_0()
                        .min_w_0()
                        .overflow_x_scroll()
                        .child(list),
                )
                .into_any_element(),
        )
    }
}
