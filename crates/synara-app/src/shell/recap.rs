//! Recaps use a visible, unsent related conversation and the existing Send/Stop owners.
use super::*;
use crate::ui::{self, Glyph, palette};

pub(super) struct RecapState {
    task: Option<TaskId>,
    epoch: u64,
    pub(super) open: bool,
    busy: bool,
    loading: bool,
    creating: bool,
    cached: Option<ThreadRecap>,
    origin: Option<ThreadOrigin>,
    review: Option<HandoffReview>,
    editor: Entity<TextEntry>,
    error: Option<String>,
    _subscription: Subscription,
}
pub(super) enum Outcome {
    Loaded(Option<ThreadRecap>, Option<ThreadOrigin>),
    Reviewed(Box<HandoffReview>),
    Created(Task),
    Saved(ThreadRecap),
}
pub(super) struct Reply {
    task: TaskId,
    epoch: u64,
    result: Result<Outcome, String>,
}
impl RecapState {
    pub fn new(cx: &mut Context<Shell>) -> Self {
        let editor = cx.new(|cx| {
            TextEntry::new(
                "Review recap request before creating an unsent conversation",
                EntryMode::Editor,
                180.,
                cx,
            )
        });
        let subscription = cx.subscribe(&editor, |_, _, _, cx| cx.notify());
        Self {
            task: None,
            epoch: 0,
            open: false,
            busy: false,
            loading: false,
            creating: false,
            cached: None,
            origin: None,
            review: None,
            editor,
            error: None,
            _subscription: subscription,
        }
    }
    pub fn pending(&self) -> bool {
        // Cache/origin reads carry a task/epoch fence and own no unsaved work.
        self.review.is_some() || self.busy && !self.loading
    }
}
impl Shell {
    fn clear_finished_recap_navigation_notice(&mut self) {
        if !self.recap.pending()
            && self.error.as_deref()
                == Some(
                    "Create or cancel the reviewed recap request before leaving this conversation.",
                )
        {
            self.error = None;
        }
    }
    pub(super) fn load_recap(&mut self, task: TaskId) {
        self.recap.task = Some(task);
        self.recap.epoch = self.recap.epoch.wrapping_add(1);
        self.recap.open = false;
        self.recap.busy = true;
        self.recap.loading = true;
        self.recap.cached = None;
        self.recap.origin = None;
        self.recap.review = None;
        self.recap.error = None;
        let epoch = self.recap.epoch;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            let result = async {
                Ok(Outcome::Loaded(
                    workspace.thread_recap(task).await?,
                    workspace.thread_origin(task).await?,
                ))
            }
            .await;
            Ok(Update::Recap(Box::new(Reply {
                task,
                epoch,
                result: result.map_err(|e: WorkspaceError| e.to_string()),
            })))
        });
    }
    pub(super) fn open_recap(&mut self, cx: &mut Context<Self>) {
        if self.selected.is_some() {
            self.show_conversation(cx);
            // Accordion: one workflow expanded at a time (see open_goals).
            self.goals.open = false;
            self.recap.open = true;
            cx.notify();
        }
    }
    fn review_recap(&mut self, cx: &mut Context<Self>) {
        let Some(task) = self.selected.filter(|id| Some(*id) == self.recap.task) else {
            return;
        };
        if self.recap.busy
            || self.recap.pending()
            || self.busy.contains(&task)
            || self.connecting.contains(&task)
            || self.loading_task.is_some()
            || self.close != CloseState::Open
        {
            return;
        }
        self.recap.busy = true;
        self.recap.error = None;
        let epoch = self.recap.epoch;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::Recap(Box::new(Reply {
                task,
                epoch,
                result: workspace
                    .review_recap(task)
                    .await
                    .map(Box::new)
                    .map(Outcome::Reviewed)
                    .map_err(|e| e.to_string()),
            })))
        });
        cx.notify();
    }
    fn create_recap_request(&mut self, cx: &mut Context<Self>) {
        let Some(task) = self.selected.filter(|id| Some(*id) == self.recap.task) else {
            return;
        };
        if self.recap.busy
            || self.creating_task
            || self.recap.editor.read(cx).is_composing()
            || self.close != CloseState::Open
        {
            return;
        }
        let Some(review) = self.recap.review.clone() else {
            return;
        };
        let draft = self.recap.editor.read(cx).text().to_owned();
        self.recap.busy = true;
        self.recap.creating = true;
        self.creating_task = true;
        let epoch = self.recap.epoch;
        let controller = self.controller.clone();
        self.job(async move {
            Ok(Update::Recap(Box::new(Reply {
                task,
                epoch,
                result: controller
                    .continue_with(review, draft)
                    .await
                    .map(Outcome::Created)
                    .map_err(|e| e.to_string()),
            })))
        });
        cx.notify();
    }
    fn cache_recap(&mut self, cx: &mut Context<Self>) {
        let Some(task) = self.selected.filter(|id| Some(*id) == self.recap.task) else {
            return;
        };
        if self.recap.busy
            || self.recap.pending()
            || self.busy.contains(&task)
            || self.loading_task.is_some()
            || self.close != CloseState::Open
        {
            return;
        }
        let Some(thread) = &self.thread else { return };
        let sequence = thread.last_sequence;
        self.recap.busy = true;
        self.recap.error = None;
        let epoch = self.recap.epoch;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::Recap(Box::new(Reply {
                task,
                epoch,
                result: workspace
                    .save_thread_recap(task, sequence)
                    .await
                    .map(Outcome::Saved)
                    .map_err(|e| e.to_string()),
            })))
        });
        cx.notify();
    }
    pub(super) fn recap_reply(&mut self, reply: Reply, cx: &mut Context<Self>) {
        if self.recap.task != Some(reply.task) || self.recap.epoch != reply.epoch {
            return;
        }
        self.recap.busy = false;
        self.recap.loading = false;
        if self.recap.creating {
            self.creating_task = false;
            self.recap.creating = false;
        }
        match reply.result {
            Ok(Outcome::Loaded(cache, origin)) => {
                self.recap.cached = cache;
                self.recap.origin = origin.filter(|o| o.kind == RelatedThreadKind::Recap);
            }
            Ok(Outcome::Reviewed(review)) => {
                self.recap
                    .editor
                    .update(cx, |entry, cx| entry.set_text(review.context().into(), cx));
                self.recap.review = Some(*review);
            }
            Ok(Outcome::Created(task)) => {
                self.creating_task = false;
                self.recap.review = None;
                let id = task.id;
                self.replace_task(task);
                if self.selected == Some(reply.task) && self.select_task(id, cx) {
                    self.show_conversation(cx);
                }
                self.notice=Some("Recap request created, not sent. Review the composer and Send explicitly. After completion, open Recap and Save recap to cache the result on the original thread.".into());
            }
            Ok(Outcome::Saved(value)) => {
                self.recap.error = None;
                if self.select_task(value.source, cx) {
                    self.show_conversation(cx);
                    self.recap.open = true;
                }
                self.notice = Some(
                    "Generated recap saved. Original messages, session and draft were not changed."
                        .into(),
                );
            }
            Err(error) => {
                self.recap.error = Some(error);
            }
        }
        self.clear_finished_recap_navigation_notice();
        cx.notify();
    }
    /// Collapsed idle recap entry for the shared one-row workflow strip.
    pub(super) fn recap_compact(&self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let view = &self.recap;
        if self.selected.is_none() || self.selected != view.task {
            return None;
        }
        if self
            .thread
            .as_ref()
            .is_some_and(|t| t.timeline.is_empty() && t.plan.is_empty())
        {
            return None;
        }
        if view.open {
            return None;
        }
        Some(
            ui::header_action(
                "recap-open",
                "Recap",
                Some(Glyph::Notebook),
                false,
                cx.listener(|this, _: &(), _, cx| this.open_recap(cx)),
            )
            .child(ui::layout_probe("recap-open"))
            .into_any_element(),
        )
    }
    pub(super) fn recap_bar(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let view = &self.recap;
        if self.selected.is_none() || self.selected != view.task {
            return div().into_any_element();
        }
        // Collapsed idle state lives in the shared workflow strip.
        if !view.open {
            return div().into_any_element();
        }
        let mut root = div()
            .px_4()
            .py_1()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .gap_1()
            .child(
                ui::action(
                    "recap-open",
                    "Recap",
                    Some(Glyph::Notebook),
                    false,
                    cx.listener(|this, _: &(), _, cx| this.open_recap(cx)),
                )
                .child(ui::layout_probe("recap-open")),
            );
        if let Some(origin) = &view.origin {
            let source = origin.parent;
            root=root.child(div().text_xs().child(format!("Recap request for {source}. Send through the normal composer. Only a successful completed reply can be cached.")))
                .child(div().flex().gap_2()
                    .child(ui::action("recap-cache","Save recap to source",None,view.pending(),cx.listener(|this,_:&(),_,cx|this.cache_recap(cx))).child(ui::layout_probe("recap-cache")))
                    .child(ui::action("recap-source","Open source",None,view.pending(),cx.listener(move|this,_:&(),_,cx|{if this.select_task(source,cx){this.show_conversation(cx);}})).child(ui::layout_probe("recap-source"))));
        }
        if let Some(cached) = &view.cached {
            let stale = self
                .thread
                .as_ref()
                .is_none_or(|thread| thread.last_sequence != cached.source_sequence);
            root = root
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(palette().muted))
                        .child(format!(
                            "Saved generated recap | source sequence {}{}",
                            cached.source_sequence,
                            if stale {
                                " | Older snapshot: regenerate to include new messages"
                            } else {
                                ""
                            }
                        )),
                )
                .child(
                    div()
                        .id("recap-text")
                        .max_h(px(160.))
                        .overflow_y_scroll()
                        .text_sm()
                        .child(cached.text.clone())
                        .child(ui::layout_probe("recap-text")),
                );
        }
        if let Some(review) = &view.review {
            root=root.child(div().text_xs().child(format!("Target: {} | {} recent visible messages, {} omitted. Original transcript and draft stay unchanged. No session or approvals transfer.",review.target_label(),review.included_messages(),review.omitted_messages())))
                .child(div().id("recap-editor").h(px(180.)).relative().child(view.editor.clone()).child(ui::layout_probe("recap-editor")))
                .child(ui::action("recap-create","Create reviewed request (not sent)",None,view.busy,cx.listener(|this,_:&(),_,cx|this.create_recap_request(cx))).child(ui::layout_probe("recap-create")));
        } else {
            root = root.child(
                ui::action(
                    "recap-review",
                    if view.cached.is_some() {
                        "Regenerate: review request"
                    } else {
                        "Generate: review request"
                    },
                    None,
                    view.busy,
                    cx.listener(|this, _: &(), _, cx| this.review_recap(cx)),
                )
                .child(ui::layout_probe("recap-review")),
            );
        }
        root = root.child(
            ui::action(
                "recap-cancel",
                if view.review.is_some() {
                    "Cancel request"
                } else {
                    "Close"
                },
                None,
                view.busy,
                cx.listener(|this, _: &(), _, cx| {
                    if !this.recap.busy {
                        this.recap.review = None;
                        this.recap.open = false;
                        this.recap.error = None;
                        this.clear_finished_recap_navigation_notice();
                        cx.notify();
                    }
                }),
            )
            .child(ui::layout_probe("recap-cancel")),
        );
        if let Some(error) = &view.error {
            root = root.child(
                div()
                    .text_xs()
                    .text_color(rgb(palette().error))
                    .child(error.clone()),
            );
        }
        root.into_any_element()
    }
}
