use super::*;
fn note(text: impl Into<SharedString>) -> gpui::Div {
    let text: SharedString = text.into();
    div().text_sm().text_color(rgb(palette().muted)).child(text)
}
fn row() -> gpui::Div {
    div()
        .py_3()
        .border_b_1()
        .border_color(rgb(palette().border))
        .flex()
        .flex_col()
        .gap_2()
}
const CHUNK: usize = 4096;
impl Shell {
    pub(in crate::shell) fn project_import_settings(
        &self,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let state = &self.project_import;
        let mut body=div().id("project-import-settings").flex().flex_col().gap_2()
            .child(note("Import reviewed local Codex or Claude JSONL history into a standalone Synara chat. Only visible user/assistant text is supported. Source files are never changed, moved or executed."))
            .children(state.error.as_ref().map(|error|div().relative().text_color(rgb(palette().error)).child(error.clone()).child(ui::layout_probe("import-error"))))
            .children(state.notice.as_ref().map(|notice|note(notice.clone())));
        if state.busy {
            body=body.child(note(if state.committing{"Committing the reviewed text and duplicate receipt atomically. Do not assume cancellation or retry until the result is shown."}else{"Reading only the explicitly selected local history. No provider request or process is started."}));
            if !state.committing {
                body = body.child(ui::action(
                    "import-cancel",
                    "Cancel read",
                    None,
                    false,
                    cx.listener(|this, _, _, cx| {
                        this.project_import.token.cancel();
                        cx.notify();
                    }),
                ));
            }
            return body.into_any_element();
        }
        if let Some(review) = &state.review {
            body=body.child(row().child("Confirm local history import")
                .child(note(format!("Source: {}\nProvider/session: {} / {}\nVisible messages: {}\nSelected branch: {}",review.preview().source_path().display(),review.preview().provider().label(),review.preview().session_id(),review.preview().messages().len(),review.preview().selected_leaf().unwrap_or("single rollout"))))
                .child(note(format!("Destination: {}\nWorking directory: {}\nFuture ACP agent: {}",review.destination().name,review.directory().display(),review.agent_name())))
                .child(note("This creates a new unsent chat, not a restored provider session. No approvals, credentials, tool state, attachments, Hub context or filesystem changes are transferred. Future ACP Send starts a fresh session and does not automatically replay this history. Direct-model use requires separate disclosure and model selection."))
                .children(review.prior().map(|receipt|note(format!("Existing import receipt: {} messages, native task {}. Confirmation will return that existing import instead of creating a duplicate.",receipt.message_count,receipt.task))))
                .child(div().flex().gap_2()
                    .child(ui::action("import-confirm","Import reviewed history",None,false,cx.listener(|this,_,_,cx|this.confirm_history(cx))).relative().child(ui::layout_probe("import-confirm")))
                    .child(ui::action("import-back-preview","Back to preview",None,false,cx.listener(|this,_,_,cx|{this.project_import.review=None;cx.notify();})).relative().child(ui::layout_probe("import-back-preview")))));
            return body.into_any_element();
        }
        if state.result.is_some() {
            body = body.child(
                ui::action(
                    "import-open-chat",
                    "Open imported chat",
                    None,
                    false,
                    cx.listener(|this, _, _, cx| this.open_imported_chat(cx)),
                )
                .relative()
                .child(ui::layout_probe("import-open-chat")),
            );
        }
        body=body.child(div().flex().gap_2()
            .children([HistoryProvider::Codex,HistoryProvider::Claude].into_iter().enumerate().map(|(index,provider)|ui::action(("import-provider",index),provider.label(),None,state.provider==provider,cx.listener(move|this,_,_,cx| {
                if !this.project_import.pending(){this.project_import.provider=provider;this.project_import.clear_source();}cx.notify();
            })).relative().child(ui::layout_probe(if provider==HistoryProvider::Codex{"import-codex"}else{"import-claude"})))))
            .child(note("Choose your sessions/history folder explicitly. Typical locations: ~/.codex/sessions or ~/.claude/projects. Expand '~' to your home directory when entering a path. Discovery is bounded and skips symlinks, credentials and subagent folders."))
            .child(div().relative().child(state.root.clone()).child(ui::layout_probe("import-root")))
            .child(div().flex().gap_2()
                .child(ui::action("import-folder","Choose folder...",None,false,cx.listener(|this,_,_,cx|this.import_pick_folder(cx))))
                .child(ui::action("import-scan","Discover histories",None,false,cx.listener(|this,_,_,cx|this.scan_history(cx))).relative().child(ui::layout_probe("import-scan"))));
        if let Some(scan) = &state.scan {
            body = body.child(note(format!(
                "Reviewed folder: {} · {}\n{} JSONL candidates · {} skipped entries{}",
                scan.source.root().display(),
                scan.source.provider().label(),
                scan.files.len(),
                scan.skipped,
                if scan.limited {
                    " · scan limit reached, select a smaller folder for other histories"
                } else {
                    ""
                }
            )));
            for (index, file) in scan
                .files
                .iter()
                .enumerate()
                .skip(state.file_page * 12)
                .take(12)
            {
                let file = file.clone();
                let picked = file.clone();
                body = body.child(
                    row()
                        .child(note(format!(
                            "{} · {} bytes",
                            file.relative_path.display(),
                            file.bytes
                        )))
                        .child(
                            ui::action(
                                ("import-file", index),
                                "Preview history",
                                None,
                                false,
                                cx.listener(move |this, _, _, cx| {
                                    this.preview_history(picked.clone(), None, cx)
                                }),
                            )
                            .relative()
                            .child(ui::layout_probe("import-preview-file")),
                        ),
                );
            }
            if scan.files.len() > 12 {
                body = body.child(
                    div()
                        .flex()
                        .gap_2()
                        .child(ui::action(
                            "import-files-prev",
                            "Previous files",
                            None,
                            false,
                            cx.listener(|this, _, _, cx| {
                                this.project_import.file_page =
                                    this.project_import.file_page.saturating_sub(1);
                                cx.notify();
                            }),
                        ))
                        .child(ui::action(
                            "import-files-next",
                            "Next files",
                            None,
                            false,
                            cx.listener(|this, _, _, cx| {
                                let count = this
                                    .project_import
                                    .scan
                                    .as_ref()
                                    .map_or(0, |s| s.files.len());
                                if (this.project_import.file_page + 1) * 12 < count {
                                    this.project_import.file_page += 1;
                                }
                                cx.notify();
                            }),
                        )),
                );
            }
        }
        if let Some(preview) = &state.preview {
            body = body.child(
                row()
                    .child(format!("Preview: {}", preview.title()))
                    .child(note(format!(
                        "Source: {}\nSession: {}\nSource working directory (metadata only): {}",
                        preview.source_path().display(),
                        preview.session_id(),
                        preview.source_cwd().unwrap_or("not recorded")
                    )))
                    .children(
                        preview
                            .warnings()
                            .iter()
                            .map(|warning| note(warning.clone())),
                    ),
            );
            if preview.leaves().len() > 1 {
                body = body.child(note(
                    "Choose one branch ending. Sibling branches are never merged.",
                ));
                for (index, leaf) in preview.leaves().iter().enumerate() {
                    let id = leaf.id.clone();
                    let file = preview.file().clone();
                    body = body.child(
                        ui::action(
                            ("import-leaf", index),
                            format!("{} · {}", leaf.label, leaf.id),
                            None,
                            preview.selected_leaf() == Some(leaf.id.as_str()),
                            cx.listener(move |this, _, _, cx| {
                                this.preview_history(file.clone(), Some(id.clone()), cx)
                            }),
                        )
                        .relative()
                        .child(ui::layout_probe("import-leaf"))
                        .children((index == 0).then(|| ui::layout_probe("import-first-leaf"))),
                    );
                }
            }
            if let Some(message) = preview.messages().get(state.message) {
                let length = message.text.chars().count();
                let text: String = message
                    .text
                    .chars()
                    .skip(state.chunk * CHUNK)
                    .take(CHUNK)
                    .collect();
                body = body.child(
                    row()
                        .child(format!(
                            "Message {} of {} · {:?} · part {} of {}",
                            state.message + 1,
                            preview.messages().len(),
                            message.role,
                            state.chunk + 1,
                            length.div_ceil(CHUNK).max(1)
                        ))
                        .child(div().whitespace_normal().child(text))
                        .child(
                            div()
                                .flex()
                                .flex_wrap()
                                .gap_2()
                                .child(ui::action(
                                    "import-message-prev",
                                    "Previous message",
                                    None,
                                    false,
                                    cx.listener(|this, _, _, cx| {
                                        this.project_import.message =
                                            this.project_import.message.saturating_sub(1);
                                        this.project_import.chunk = 0;
                                        cx.notify();
                                    }),
                                ))
                                .child(ui::action(
                                    "import-message-next",
                                    "Next message",
                                    None,
                                    false,
                                    cx.listener(|this, _, _, cx| {
                                        let count = this
                                            .project_import
                                            .preview
                                            .as_ref()
                                            .map_or(0, |p| p.messages().len());
                                        if this.project_import.message + 1 < count {
                                            this.project_import.message += 1;
                                            this.project_import.chunk = 0;
                                        }
                                        cx.notify();
                                    }),
                                ))
                                .children((length > CHUNK).then(|| {
                                    ui::action(
                                        "import-part-prev",
                                        "Previous part",
                                        None,
                                        false,
                                        cx.listener(|this, _, _, cx| {
                                            this.project_import.chunk =
                                                this.project_import.chunk.saturating_sub(1);
                                            cx.notify();
                                        }),
                                    )
                                }))
                                .children((length > CHUNK).then(|| {
                                    ui::action(
                                        "import-part-next",
                                        "Next part",
                                        None,
                                        false,
                                        cx.listener(move |this, _, _, cx| {
                                            if (this.project_import.chunk + 1) * CHUNK < length {
                                                this.project_import.chunk += 1;
                                            }
                                            cx.notify();
                                        }),
                                    )
                                })),
                        ),
                );
            }
            body = body.child(note(
                "Destination project / working folder (choose explicitly)",
            ));
            for (index, project) in self.catalog.projects.iter().enumerate().filter(|(_, p)| {
                self.catalog.workspaces.iter().any(|w| {
                    w.id == p.workspace_id && matches!(w.location, WorkspaceLocation::Local { .. })
                })
            }) {
                let id = project.id;
                body = body.child(
                    ui::action(
                        ("import-destination", index),
                        project.name.clone(),
                        None,
                        state.destination == Some(id),
                        cx.listener(move |this, _, _, cx| {
                            this.project_import.destination = Some(id);
                            cx.notify();
                        }),
                    )
                    .relative()
                    .child(ui::layout_probe("import-destination")),
                );
            }
            body = body.child(note("Future coding agent (not started by importing)"));
            for (index, profile) in self.profiles.iter().enumerate() {
                let id = profile.id.clone();
                body = body.child(
                    ui::action(
                        ("import-agent", index),
                        profile.name.clone(),
                        None,
                        state.agent.as_ref() == Some(&id),
                        cx.listener(move |this, _, _, cx| {
                            this.project_import.agent = Some(id.clone());
                            cx.notify();
                        }),
                    )
                    .relative()
                    .child(ui::layout_probe("import-agent")),
                );
            }
            body = body.child(
                ui::action(
                    "import-review-destination",
                    "Review destination and import",
                    None,
                    false,
                    cx.listener(|this, _, _, cx| this.review_history(cx)),
                )
                .relative()
                .child(ui::layout_probe("import-review-destination")),
            );
        }
        body.into_any_element()
    }
}
