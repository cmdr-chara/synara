//! Explicit reviewed recap generation, separate from normal Send and task sessions.
//! Native entry points retain existing task and storage ownership.
use super::*;
use crate::ui::{self, Glyph, palette};
use gpui::FocusHandle;
use tokio_util::sync::CancellationToken;

pub(super) enum Reply {
    Loaded(TaskId, u64, Result<RecapReview, String>),
    Generated(TaskId, u64, Result<ThreadRecap, String>),
}
struct RecapDialog {
    task: TaskId,
    title: String,
    review: Option<RecapReview>,
    selected_model: Option<(String, String)>,
    query: Entity<TextEntry>,
    _subscription: Subscription,
    show_source: bool,
    busy: bool,
    cancel: Option<CancellationToken>,
    error: Option<String>,
    notice: Option<String>,
}
#[derive(Default)]
pub(super) struct RecapState {
    dialog: Option<RecapDialog>,
    generation: u64,
    previous_focus: Option<FocusHandle>,
    restore_focus: bool,
}
impl RecapState {
    pub fn open(&self) -> bool {
        self.dialog.is_some()
    }
}
impl Shell {
    pub(super) fn open_thread_recap(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(task) = self.task().cloned() else {
            return;
        };
        if self.recaps.open()
            || self.workflows.open()
            || self.file_comments.open()
            || self.revisions.open()
            || self.handoff.open()
            || self.loading_task.is_some()
            || self.close != CloseState::Open
            || self.explorer.modal_open()
            || self.saved_context.dialog.is_some()
            || self.organization.dialog.is_some()
            || self.kanban.dialog.is_some()
        {
            return;
        }
        let query = cx.new(|cx| {
            TextEntry::new(
                "Filter configured providers and models...",
                EntryMode::SingleLine,
                36.,
                cx,
            )
        });
        let subscription = cx.subscribe(&query, |_, _, _, cx| cx.notify());
        self.recaps.previous_focus = window.focused(cx);
        self.recaps.dialog = Some(RecapDialog {
            task: task.id,
            title: task.title,
            review: None,
            selected_model: None,
            query: query.clone(),
            _subscription: subscription,
            show_source: false,
            busy: false,
            cancel: None,
            error: None,
            notice: None,
        });
        self.focus_composer = false;
        self.controls.retire();
        self.chat_tools.retire();
        self.environment.retire_popup();
        self.navigation.menu_open = false;
        self.settings.popup = None;
        window.focus(&query.read(cx).focus_handle(cx), cx);
        self.reload_thread_recap(cx);
    }
    fn reload_thread_recap(&mut self, cx: &mut Context<Self>) {
        let Some(d) = self.recaps.dialog.as_mut() else {
            return;
        };
        if d.busy {
            return;
        }
        d.busy = true;
        d.error = None;
        d.selected_model = None;
        self.recaps.generation = self.recaps.generation.wrapping_add(1);
        let epoch = self.recaps.generation;
        let id = d.task;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::Recap(Box::new(Reply::Loaded(
                id,
                epoch,
                workspace
                    .review_thread_recap(id)
                    .await
                    .map_err(|e| e.to_string()),
            ))))
        });
        cx.notify();
    }
    fn choose_recap_model(&mut self, provider: String, model: String, cx: &mut Context<Self>) {
        if let Some(d) = self.recaps.dialog.as_mut() {
            if !d.busy {
                d.selected_model = Some((provider, model));
            }
        }
        cx.notify();
    }
    fn generate_recap(&mut self, cx: &mut Context<Self>) {
        let Some(d) = self.recaps.dialog.as_mut() else {
            return;
        };
        if d.busy
            || self.selected != Some(d.task)
            || self.close != CloseState::Open
            || d.query.read(cx).is_composing()
        {
            return;
        }
        let (Some(review), Some((provider, model))) = (d.review.clone(), d.selected_model.clone())
        else {
            return;
        };
        let cancel = CancellationToken::new();
        d.cancel = Some(cancel.clone());
        d.busy = true;
        d.error = None;
        d.notice = Some("Generating a separate recap. No conversation prompt was sent. Stop cannot undo processing already performed by the remote model.".into());
        self.recaps.generation = self.recaps.generation.wrapping_add(1);
        let epoch = self.recaps.generation;
        let id = d.task;
        let controller = self.controller.clone();
        self.job(async move {
            Ok(Update::Recap(Box::new(Reply::Generated(
                id,
                epoch,
                controller
                    .generate_thread_recap(review, provider, model, cancel)
                    .await
                    .map_err(|e| e.to_string()),
            ))))
        });
        cx.notify();
    }
    fn stop_recap(&mut self, cx: &mut Context<Self>) {
        if let Some(d) = self.recaps.dialog.as_mut() {
            if let Some(cancel) = &d.cancel {
                cancel.cancel();
                d.notice = Some("Stopping the separate model request. The previous cache remains unless a completed save already committed.".into());
            }
        }
        cx.notify();
    }
    pub(super) fn recap_reply(&mut self, reply: Reply, cx: &mut Context<Self>) {
        let (id, epoch) = match &reply {
            Reply::Loaded(id, epoch, _) | Reply::Generated(id, epoch, _) => (*id, *epoch),
        };
        if epoch != self.recaps.generation
            || self.selected != Some(id)
            || self.close != CloseState::Open
        {
            return;
        }
        let Some(d) = self
            .recaps
            .dialog
            .as_mut()
            .filter(|d| d.task == id && d.busy)
        else {
            return;
        };
        d.busy = false;
        d.cancel = None;
        match reply {
            Reply::Loaded(_, _, Ok(review)) => {
                d.review = Some(review);
                d.notice = None;
            }
            Reply::Generated(_, _, Ok(saved)) => {
                if let Some(review) = &mut d.review {
                    review.cached = Some(saved);
                }
                d.notice = Some("Recap cached separately. The source transcript, session, files and normal draft were not changed.".into());
            }
            Reply::Loaded(_, _, Err(error)) | Reply::Generated(_, _, Err(error)) => {
                d.error = Some(error);
                d.notice = None;
            }
        }
        cx.notify();
    }
    fn dismiss_recap(&mut self, cx: &mut Context<Self>) {
        let Some(d) = &self.recaps.dialog else {
            return;
        };
        if d.busy || d.query.read(cx).is_composing() {
            self.stop_recap(cx);
            return;
        }
        self.recaps.dialog = None;
        self.recaps.generation = self.recaps.generation.wrapping_add(1);
        self.recaps.restore_focus = true;
        cx.notify();
    }
    pub(super) fn restore_recap_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.recaps.restore_focus {
            self.recaps.restore_focus = false;
            if let Some(focus) = self.recaps.previous_focus.take() {
                window.focus(&focus, cx);
            }
        }
    }
    pub(super) fn recap_overlay(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(d) = &self.recaps.dialog else {
            return div().into_any_element();
        };
        let mut content = div()
            .id("recap-body")
            .min_h_0()
            .flex_1()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_2();
        if let Some(review) = &d.review {
            let source = &review.snapshot.source;
            content = content.child(div().text_xs().child(format!(
                "{} whole visible messages included, {} omitted; sequence {}. No reasoning, tools, approvals, attachments or file contents. Source SHA-256 {}",
                source.included_messages, source.omitted_messages, source.sequence, source.sha256)))
                .child(ui::action("recap-source", if d.show_source { "Hide reviewed source" } else { "Inspect exact source to send" }, None, false,
                    cx.listener(|this, _: &(), _, cx| { if let Some(d) = this.recaps.dialog.as_mut() { d.show_source = !d.show_source; } cx.notify(); }))
                    .relative().child(ui::layout_probe("recap-source")));
            if d.show_source {
                content = content.child(
                    div()
                        .id("recap-source-text")
                        .h(px(160.))
                        .min_h(px(160.))
                        .flex_shrink_0()
                        .overflow_y_scroll()
                        .border_1()
                        .border_color(rgb(palette().border))
                        .p_2()
                        .text_xs()
                        .child(review.snapshot.text.clone()),
                );
            }
            content = content.child(
                div()
                    .relative()
                    .h(px(38.))
                    .min_h(px(38.))
                    .flex_shrink_0()
                    .child(d.query.clone())
                    .child(ui::layout_probe("recap-model-filter")),
            );
            let query = d.query.read(cx).text().trim().to_lowercase();
            let mut shown = 0;
            let mut matched = 0;
            let mut models = div()
                .id("recap-model-list")
                .max_h(px(140.))
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .gap_1();
            for p in &review.settings.providers {
                for m in &p.models {
                    let label = format!("{} / {}", p.name, m.name);
                    if !format!("{label} {} {}", p.id, m.id)
                        .to_lowercase()
                        .contains(&query)
                    {
                        continue;
                    }
                    matched += 1;
                    if shown >= 40 {
                        continue;
                    }
                    let provider = p.id.clone();
                    let model = m.id.clone();
                    let selected = d
                        .selected_model
                        .as_ref()
                        .is_some_and(|(a, b)| a == &p.id && b == &m.id);
                    models = models.child(
                        ui::action(
                            format!("recap-model-{shown}"),
                            label,
                            None,
                            selected,
                            cx.listener(move |this, _: &(), _, cx| {
                                this.choose_recap_model(provider.clone(), model.clone(), cx)
                            }),
                        )
                        .relative()
                        .child(ui::layout_probe_slot("recap-model", shown)),
                    );
                    shown += 1;
                }
            }
            content = content.child(models).child(div().text_xs().child(format!("Showing {shown} of {matched} matching configured models. Filter to narrow. No provider session is transferred.")));
            if review.settings.providers.is_empty() {
                content = content.child(div().relative().child("No direct model is configured. Close this dialog and configure one in Settings > Direct models. Reading an existing cache never requires a provider.")
                    .child(ui::layout_probe("recap-provider-unavailable")));
            }
            if let Some((provider, model)) = &d.selected_model {
                if let Some(p) = review.settings.providers.iter().find(|p| &p.id == provider) {
                    content = content.child(div().relative().text_xs().child(format!(
                        "Explicit destination: {} / {} at {}. Generating sends the reviewed text to this endpoint and may incur provider charges.", p.name, model, p.endpoint))
                        .child(ui::layout_probe("recap-destination")));
                }
            }
            if !d.busy && d.selected_model.is_some() && source.included_messages > 0 {
                content = content.child(
                    ui::action(
                        "recap-generate",
                        if review.cached.is_some() {
                            "Regenerate and replace cache"
                        } else {
                            "Generate and cache recap"
                        },
                        Some(Glyph::Brain),
                        false,
                        cx.listener(|this, _: &(), _, cx| this.generate_recap(cx)),
                    )
                    .relative()
                    .child(ui::layout_probe("recap-generate")),
                );
            } else if source.included_messages == 0 {
                content = content.child(div().text_xs().child("No whole visible messages fit the recap limits. There is no source text to generate from."));
            }
            if let Some(saved) = &review.cached {
                let stale = saved.source != *source;
                content = content.child(div().relative().text_xs().child(format!(
                    "{} Model-generated, not authoritative transcript history. {} / {}; generated Unix ms {}; source sequence {}.",
                    if stale { "STALE: the conversation has changed." } else { "Cached recap for this reviewed source." },
                    saved.provider_id, saved.model_id, saved.generated_at_ms, saved.source.sequence))
                    .child(ui::layout_probe(if stale { "recap-stale" } else { "recap-cached" })))
                    .child(div().id("recap-cached-text").h(px(160.)).min_h(px(100.)).flex_shrink_0().overflow_y_scroll().p_2().border_1()
                        .border_color(rgb(palette().border)).child(saved.text.clone()));
            }
        }
        let page = div()
            .id("recap-dialog")
            .role(gpui::Role::Dialog)
            .aria_label("Thread recap")
            .tab_group()
            .w_full()
            .max_w(px(780.))
            .h(px(740.))
            .max_h_full()
            .p_4()
            .flex()
            .flex_col()
            .gap_2()
            .bg(ui::surface(palette().overlay))
            .border_1()
            .border_color(rgb(palette().border))
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape" && !event.prefer_character_input {
                    this.dismiss_recap(cx);
                    cx.stop_propagation();
                }
            }))
            .child(
                div()
                    .text_size(px(20.))
                    .child(format!("Thread recap: {}", d.title)),
            )
            .child(content)
            .children(d.error.as_ref().map(|error| {
                div()
                    .relative()
                    .text_xs()
                    .text_color(rgb(palette().error))
                    .child(error.clone())
                    .child(ui::layout_probe("recap-error"))
            }))
            .children(
                d.notice
                    .as_ref()
                    .map(|notice| div().text_xs().child(notice.clone())),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_2()
                    .border_t_1()
                    .border_color(rgb(palette().border))
                    .pt_2()
                    .child(
                        ui::action(
                            "recap-close",
                            "Close",
                            None,
                            false,
                            cx.listener(|this, _: &(), _, cx| this.dismiss_recap(cx)),
                        )
                        .relative()
                        .child(ui::layout_probe("recap-close")),
                    )
                    .child(
                        ui::action(
                            "recap-reload",
                            if d.busy {
                                "Working..."
                            } else {
                                "Reload source and cache"
                            },
                            None,
                            false,
                            cx.listener(|this, _: &(), _, cx| this.reload_thread_recap(cx)),
                        )
                        .relative()
                        .child(ui::layout_probe("recap-reload")),
                    )
                    .children(
                        d.review
                            .as_ref()
                            .and_then(|r| r.cached.as_ref())
                            .map(|saved| {
                                let text = saved.text.clone();
                                ui::action(
                                    "recap-copy",
                                    "Copy cached recap",
                                    Some(Glyph::Copy),
                                    false,
                                    cx.listener(move |_, _: &(), _, cx| {
                                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                            text.clone(),
                                        ))
                                    }),
                                )
                                .relative()
                                .child(ui::layout_probe("recap-copy"))
                            }),
                    )
                    .children(d.cancel.as_ref().map(|_| {
                        ui::action(
                            "recap-stop",
                            "Stop recap",
                            Some(Glyph::Stop),
                            false,
                            cx.listener(|this, _: &(), _, cx| this.stop_recap(cx)),
                        )
                        .relative()
                        .child(ui::layout_probe("recap-stop"))
                    })),
            );
        div()
            .id("recap-backdrop")
            .absolute()
            .inset_0()
            .occlude()
            .bg(gpui::rgba(0x00000088))
            .flex()
            .items_center()
            .justify_center()
            .p_3()
            .child(page)
            .into_any_element()
    }
}
