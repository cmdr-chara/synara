use super::*;
impl Shell {
    pub(in crate::shell) fn attachments_view(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let state = &self.attachments;
        let task = self.selected;
        let value = state.value.as_ref();
        let pending = value.map_or(&[][..], |v| v.pending.as_slice());
        let recent = if self.settings.value.chat.show_recent_attachments {
            value.map_or(&[][..], |v| v.recent.as_slice())
        } else {
            &[][..]
        };
        let changing = task.is_some_and(|t| state.changing(t));
        let error = task.and_then(|t| state.errors.get(&t));
        let mut root = div()
            .id("composer-attachments")
            .min_w_0()
            .flex()
            .flex_col()
            .gap_1()
            .children((!pending.is_empty() || !recent.is_empty()).then(|| {
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_1()
                    .children(pending.iter().enumerate().map(|(slot, info)| {
                        let preview = info.id.clone();
                        let remove = info.id.clone();
                        div()
                            .flex()
                            .items_center()
                            .max_w(px(220.))
                            .gap_1()
                            .child(
                                ui::action(
                                    ("attachment", slot),
                                    info.name.clone(),
                                    Some(if info.is_folder_snapshot() {
                                        Glyph::Folder
                                    } else if info.kind.is_image() {
                                        Glyph::Capture
                                    } else {
                                        Glyph::Files
                                    }),
                                    false,
                                    cx.listener(move |this, _: &(), _, cx| {
                                        this.preview_attachment(preview.clone(), cx)
                                    }),
                                )
                                .min_w_0()
                                .max_w(px(185.))
                                .text_size(px(12.)),
                            )
                            .child(
                                ui::chrome_button(
                                    "remove-attachment",
                                    "Remove attachment",
                                    Glyph::Close,
                                    changing,
                                    cx.listener(move |this, _: &(), _, cx| {
                                        this.change_attachments(
                                            AttachmentEdit::Remove(remove.clone()),
                                            cx,
                                        )
                                    }),
                                )
                                .id(("remove-attachment", slot))
                                .size(px(22.)),
                            )
                    }))
                    .children((!recent.is_empty()).then(|| {
                        ui::action(
                            "recent-attachments",
                            format!("Recent ({})", recent.len()),
                            None,
                            state.recent_open,
                            cx.listener(|this, _: &(), _, cx| {
                                this.attachments.recent_open = !this.attachments.recent_open;
                                cx.notify();
                            }),
                        )
                        .text_size(px(12.))
                        .relative()
                        .child(ui::layout_probe("recent-attachments"))
                    }))
            }))
            .children((state.loading || changing).then(|| {
                div()
                    .px_2()
                    .text_size(px(12.))
                    .text_color(rgb(palette().muted))
                    .child(if state.loading {
                        "Loading saved attachments..."
                    } else {
                        "Saving attachments..."
                    })
            }))
            .children((!pending.is_empty()).then(|| {
                div()
                    .px_2()
                    .text_size(px(11.))
                    .text_color(rgb(palette().muted))
                    .child(format!(
                        "{} files · {} KiB · Snapshots will be sent to the selected agent{}{}",
                        pending.len(),
                        pending
                            .iter()
                            .map(|a| a.bytes)
                            .sum::<usize>()
                            .div_ceil(1024),
                        if pending.iter().any(|a| a.is_folder_snapshot()) {
                            " · Folder snapshots show names and item types only"
                        } else {
                            ""
                        },
                        if pending.iter().any(|a| a.kind == AttachmentKind::Webp) {
                            " · Still WebP images convert to PNG for prompts; animation is unsupported"
                        } else {
                            ""
                        },
                    ))
            }))
            .children(
                self.details
                    .as_ref()
                    .and_then(|details| {
                        value.and_then(|v| v.unsupported(&details.connection.capabilities))
                    })
                    .map(|message| {
                        div()
                            .px_2()
                            .text_size(px(12.))
                            .text_color(rgb(palette().error))
                            .child(message)
                    }),
            )
            .children(error.map(|error| {
                div()
                    .px_2()
                    .text_size(px(12.))
                    .text_color(rgb(palette().error))
                    .child(error.clone())
            }));
        if task.is_some_and(|t| state.imports.contains_key(&t))
            && !task.is_some_and(|t| state.writes.contains_key(&t))
        {
            root = root.child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(ui::action(
                        "retry-attachments",
                        "Retry import",
                        Some(Glyph::Restore),
                        false,
                        cx.listener(|this, _: &(), _, cx| {
                            let Some(task) = this.selected else { return };
                            let Some(inputs) = this.attachments.imports.get(&task).cloned() else {
                                return;
                            };
                            let Some(value) = this.attachments.value.as_ref() else {
                                return;
                            };
                            this.import_attachments(task, value.revision, inputs, cx);
                        }),
                    ))
                    .child(ui::action(
                        "discard-attachment-import",
                        "Discard failed import",
                        None,
                        false,
                        cx.listener(|this, _: &(), _, cx| {
                            if let Some(task) = this.selected {
                                this.attachments.imports.remove(&task);
                                this.attachments.errors.remove(&task);
                            }
                            cx.notify();
                        }),
                    )),
            );
        }
        if error.is_some() || value.is_none() && !state.loading {
            root = root.child(
                ui::action(
                    "reload-attachments",
                    "Reload saved attachments",
                    Some(Glyph::Restore),
                    false,
                    cx.listener(|this, _: &(), _, cx| {
                        if let Some(task) = this.selected
                            && !this.attachments.writes.contains_key(&task)
                        {
                            this.load_attachments(task);
                            cx.notify();
                        }
                    }),
                )
                .text_size(px(12.)),
            );
        }
        for (index, other) in state
            .imports
            .keys()
            .filter(|other| Some(**other) != task)
            .copied()
            .enumerate()
        {
            let name = self
                .catalog
                .tasks
                .iter()
                .find(|task| task.id == other)
                .map_or("removed thread", |task| task.title.as_str());
            let writing = state.writes.contains_key(&other);
            root = root.child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_1()
                    .child(
                        ui::action(
                            ("other-attachment-import", index),
                            format!("Attachment import: {name}"),
                            Some(Glyph::Attach),
                            false,
                            cx.listener(move |this, _: &(), _, cx| {
                                if this.select_task(other, cx) {
                                    this.show_conversation(cx);
                                }
                            }),
                        )
                        .flex_1()
                        .text_size(px(12.)),
                    )
                    .children((!writing).then(|| {
                        ui::action(
                            ("discard-other-import", index),
                            "Discard failed import",
                            None,
                            false,
                            cx.listener(move |this, _: &(), _, cx| {
                                this.attachments.imports.remove(&other);
                                this.attachments.errors.remove(&other);
                                cx.notify();
                            }),
                        )
                        .text_size(px(12.))
                    })),
            );
        }
        if state.recent_open && self.settings.value.chat.show_recent_attachments {
            root=root.child(div().id("recent-attachment-list").max_h(px(150.)).overflow_y_scroll().flex().flex_col().gap_1()
                .children(recent.iter().rev().enumerate().map(|(slot,info)|{
                    let preview=info.id.clone();let reuse=info.id.clone();
                    div().flex().items_center().gap_2()
                        .child(ui::action(("recent-file",slot),info.name.clone(),Some(if info.is_folder_snapshot(){Glyph::Folder}else{Glyph::Files}),false,
                            cx.listener(move |this,_:&(),_,cx|this.preview_attachment(preview.clone(),cx))).flex_1().min_w_0().text_size(px(12.)))
                        .child(ui::action(("reuse-attachment",slot),"Attach again",None,false,
                            cx.listener(move |this,_:&(),_,cx|this.change_attachments(AttachmentEdit::Reuse(reuse.clone()),cx))).text_size(px(12.)))
                }))
                .child(ui::action("forget-recent-attachments","Clear recent snapshots",None,false,
                    cx.listener(|this,_:&(),_,cx|this.change_attachments(AttachmentEdit::ForgetRecent,cx))).text_size(px(11.)).relative().child(ui::layout_probe("forget-recent-attachments")))
                .child(div().px_2().text_size(px(11.)).text_color(rgb(palette().muted)).child("Recent is a bounded local cache, not delivery confirmation. Reattaching does not send.")));
        }
        let folder_preview = state
            .preview_id
            .as_ref()
            .and_then(|id| {
                value.and_then(|draft| {
                    draft
                        .pending
                        .iter()
                        .chain(&draft.recent)
                        .find(|info| info.id == *id)
                })
            })
            .is_some_and(|info| info.is_folder_snapshot());
        if state.preview_loading || state.preview.is_some() {
            root = root
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(div().flex_1().text_size(px(12.)).child(if folder_preview {
                            "Folder snapshot preview · names and types only"
                        } else {
                            "Attachment preview"
                        }))
                        .child(
                            ui::chrome_button(
                                "close-attachment-preview",
                                "Close preview",
                                Glyph::Close,
                                false,
                                cx.listener(|this, _: &(), _, cx| {
                                    this.attachments.preview = None;
                                    this.attachments.preview_id = None;
                                    this.attachments.preview_loading = false;
                                    cx.notify();
                                }),
                            )
                            .size(px(24.)),
                        ),
                )
                .child(match &state.preview {
                    Some(Preview::Image(image, (w, h))) => div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            gpui::img(image.clone())
                                .w_full()
                                .h(px(150.))
                                .object_fit(gpui::ObjectFit::Contain),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(rgb(palette().muted))
                                .child(format!("{w} × {h}")),
                        )
                        .into_any_element(),
                    Some(Preview::Text(text)) => div()
                        .id("attachment-text-preview")
                        .max_h(px(150.))
                        .overflow_y_scroll()
                        .p_2()
                        .font_family(ui::code_font())
                        .text_size(px(12.))
                        .child(truncate(text, 64 * 1024))
                        .into_any_element(),
                    None => div()
                        .p_2()
                        .text_size(px(12.))
                        .child("Loading preview...")
                        .into_any_element(),
                });
        }
        root.into_any_element()
    }
}
