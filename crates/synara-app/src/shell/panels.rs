use super::*;
impl Shell {
    pub(super) fn files_panel(&self, width: f32, cx: &mut Context<Self>) -> gpui::AnyElement {
        if self.studio.open { return self.studio_files_panel(cx); }
        use crate::ui::{self, Glyph, palette};
        let dirty = self.active_document_dirty(cx);
        let query = self.file_search.read(cx).text().trim().to_lowercase();
        let mut files: Vec<_> = self.files.iter().filter(|file| {
            (self.editors.show_hidden || !file.name.starts_with('.'))
                && (query.is_empty() || file.name.to_lowercase().contains(&query))
        }).collect();
        files.sort_by_key(|file| (!file.directory, file.name.to_lowercase()));
        let count = files.len();
        let page = self.file_page.min(count.saturating_sub(1) / 400);
        let narrow = width < 680.;
        div().flex().flex_col().flex_1().min_h_0().min_w_0()
            .child(self.editor_tabs(cx))
            .child(div().flex().items_center().flex_wrap().px_2().py_1().gap_1()
                .child(ui::chrome_button("editor-tree-toggle", "Show or hide file tree", Glyph::Folders, false,
                    cx.listener(|this, _: &(), _, cx| { this.editors.tree_visible = !this.editors.tree_visible; cx.notify(); })))
                .child(ui::action("files-root", "Workspace", None, false, cx.listener(|this, _: &(), _, cx| {
                    this.directory.clear(); this.explorer.reset_search(); this.refresh_files(); cx.notify();
                })).text_size(px(11.)))
                .children(self.directory.components().enumerate().map(|(index, component)| {
                    let path: PathBuf = self.directory.components().take(index + 1).collect();
                    ui::action(("file-breadcrumb", index), component.as_os_str().to_string_lossy().into_owned(), Some(Glyph::ChevronRight), false,
                        cx.listener(move |this, _: &(), _, cx| { this.directory = path.clone(); this.explorer.reset_search(); this.refresh_files(); cx.notify(); })).text_size(px(11.))
                }))
                .child(div().flex_1())
                .child(ui::action("files-hidden", "Dotfiles", None, self.editors.show_hidden, cx.listener(|this, _: &(), _, cx| {
                    this.editors.show_hidden = !this.editors.show_hidden; this.file_page = 0; cx.notify();
                })).text_size(px(11.))))
            .children(self.editors.loading.as_ref().map(|path| div().px_3().py_1().text_size(px(11.)).text_color(rgb(palette().muted)).child(format!("Opening {}...", path.display()))))
            .child(div().flex().flex_1().min_h_0().min_w_0().when(narrow, |el| el.flex_col())
                .children(self.editors.tree_visible.then(|| div().relative().child(ui::layout_probe("file-tree"))
                    .when(narrow, |el| el.h(px(185.)).w_full().border_b_1())
                    .when(!narrow, |el| el.w(px(220.)).border_r_1())
                    .flex_shrink_0().min_h_0().border_color(rgb(palette().border)).flex().flex_col().gap_1()
                    .child(self.explorer_toolbar(cx))
                    .when(!self.explorer.search_open, |el| el.child(div().p_2().child(self.file_search.clone())))
                    .when(self.explorer.search_open, |el| el.child(self.explorer_search_panel(cx)))
                    .when(!self.explorer.search_open, |el| el.child(div().id("file-list").flex_1().min_h_0().overflow_y_scroll().px_1().flex().flex_col()
                        .children(files.iter().skip(page * 400).take(400).enumerate().map(|(index, file)| {
                            let path = file.relative_path.clone(); let directory = file.directory; let symlink = file.symlink;
                            let active = self.document.as_ref().is_some_and(|document| document.path == path);
                            ui::action(("file", index), file.name.clone(), Some(if directory { Glyph::Folder } else { Glyph::Files }), active,
                                cx.listener(move |this, _: &(), _, cx| {
                                    if symlink { this.error = Some("Symlink navigation is disabled at the workspace boundary.".into()); cx.notify(); }
                                    else if directory { this.directory = path.clone(); this.explorer.reset_search(); this.file_search.update(cx, |input, cx| input.clear(cx)); this.refresh_files(); }
                                    else { this.open_file(path.clone(), cx); }
                                })).h(px(28.)).text_size(px(12.)).relative().child(ui::layout_probe_slot("file-row", index))
                        }))
                        .children((count == 0).then(|| div().p_3().text_size(px(12.)).text_color(rgb(palette().muted)).child("No matching files in this folder.")))))
                    .child(div().px_2().pb_2().flex().items_center().gap_2()
                        .child(ui::button("files-refresh", "Refresh", false).text_size(px(11.)).on_click(cx.listener(|this, _, _, _| this.refresh_files())))
                        .child(div().text_size(px(11.)).text_color(rgb(palette().muted)).child(format!("{count} items")))
                        .children((page > 0).then(|| ui::button("previous-files", "Previous", false).on_click(cx.listener(move |this, _, _, cx| { this.file_page = page.saturating_sub(1); cx.notify(); }))))
                        .children(((page + 1) * 400 < count).then(|| ui::button("next-files", "Next", false).on_click(cx.listener(move |this, _, _, cx| { this.file_page = page + 1; cx.notify(); })))))))
                .child(if let Some(document) = &self.document {
                    div().flex_1().min_w_0().min_h_0().flex().flex_col().p_3().gap_2()
                        .child(div().flex().items_center().gap_2()
                            .child(div().flex_1().min_w_0().text_ellipsis().text_size(px(12.)).child(document.path.display().to_string()))
                            .child(ui::button("save-document", if self.saving { "Saving..." } else { "Save" }, dirty).relative().child(ui::layout_probe("save-document")).on_click(cx.listener(|this, _, _, cx| this.save_file(cx))))
                            .child(ui::button("discard-document", "Discard", false).on_click(cx.listener(|this, _, _, cx| this.discard_active_document(cx)))))
                        .child(self.editor_tools(cx))
                        .child(if self.editors.preview && self.editor_preview_available() { self.editor_preview(cx) } else { self.editor_content() })
                        .child(self.editor_status(cx))
                        .children(self.editor.read(cx).error.as_ref().map(|error| div().text_color(rgb(palette().error)).child(error.clone())))
                } else {
                    div().flex_1().min_w_0().flex().items_center().justify_center().p_4().text_size(px(12.)).text_color(rgb(palette().muted))
                        .child(div().w_full().min_w_0().text_center().child("Open files from Explorer. Each tab keeps its text, selection and undo history."))
                }))
            .into_any_element()
    }
    pub(super) fn terminal_panel(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let terminal_view = self.terminal_view.clone();
        let exit_code = terminal_view.read(cx).exit_code();
        let terminal_error = terminal_view.read(cx).terminal_error().map(str::to_owned);
        let title = terminal_view.read(cx).title().map(str::to_owned);
        let cwd_hint = terminal_view.read(cx).cwd_hint().map(str::to_owned);
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .p_4()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_lg().child("Terminal"))
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                button(
                                    "start-shell",
                                    if self.terminal.is_some() {
                                        "Restart shell"
                                    } else {
                                        "Start shell"
                                    },
                                    false,
                                )
                                .relative()
                                .child(crate::ui::layout_probe("start-shell"))
                                .on_click(cx.listener(|this, _, _, cx| this.start_terminal(cx))),
                            )
                            .child(
                                button("interrupt-shell", "Interrupt", false)
                                    .relative()
                                    .child(crate::ui::layout_probe("interrupt-shell"))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.interrupt_terminal(cx)
                                    })),
                            )
                            .child(
                                button("stop-shell", "Stop shell", false)
                                    .relative()
                                    .child(crate::ui::layout_probe("stop-shell"))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.stop_terminal(cx)
                                    })),
                            ),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x9cabbd))
                    .child(if self.terminal_starting {
                        "Starting shell...".into()
                    } else {
                        self.terminal_root.as_ref().map_or_else(
                            || "No running shell".into(),
                            |root| {
                                let mut status = format!("Shell directory: {}", root.display());
                                if let Some(title) = title {
                                    status.push_str(&format!(" · {title}"));
                                }
                                if let Some(cwd) = cwd_hint {
                                    status.push_str(&format!(" · {cwd}"));
                                }
                                status
                            },
                        )
                    }),
            )
            .child(
                div()
                    .id("terminal-screen")
                    .relative()
                    .child(crate::ui::layout_probe("terminal-screen"))
                    .flex_1()
                    .min_h_0()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x283444))
                    .bg(rgb(0x0b1017))
                    .child(terminal_view),
            )
            .children(exit_code.map(|code| {
                div().text_xs().child(format!(
                    "Shell exited with status {code}. Historical output is retained."
                ))
            }))
            .children(
                terminal_error
                    .map(|error| div().text_color(rgb(0xffb1b5)).child(error)),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x8e9caf))
                    .child("Type directly into the terminal. Ctrl+C interrupts. Ctrl+Shift+C copies a selection and Ctrl+Shift+V pastes through review when required."),
            )
            .into_any_element()
    }
    pub(super) fn remote_panel(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let status = self.workspace_target().map_or_else(
            || "No workspace selected.".into(),
            |target| match target {
                WorkspaceTarget::Local { root } => {
                    format!("Current workspace is local: {}", root.display())
                }
                WorkspaceTarget::Ssh { workspace, root } => {
                    let label = match workspace.location {
                        WorkspaceLocation::Ssh {
                            host, port, user, ..
                        } => format!(
                            "{}{}:{port}",
                            user.map_or_else(String::new, |user| format!("{user}@")),
                            host
                        ),
                        WorkspaceLocation::Local { .. } => unreachable!(),
                    };
                    format!("Current workspace is remote: {label} · {}", root.display())
                }
            },
        );
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .p_4()
            .gap_3()
            .child(div().text_lg().child("Remote workspace"))
            .child(div().text_xs().text_color(rgb(0x9cabbd)).child(
                "SSH enrollment is fail-closed: Synara uses only the pinned known_hosts file and explicit identity below. Ambient SSH config, agent auth, forwarding and multiplexing are disabled. The remote helper must already be installed on the target host.",
            ))
            .child(div().text_xs().text_color(rgb(0xaebbd0)).child(status))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(div().flex().gap_2().child(self.remote_host.clone()).child(self.remote_port.clone()))
                    .child(self.remote_user.clone())
                    .child(self.remote_root.clone())
                    .child(self.remote_known_hosts.clone())
                    .child(self.remote_identity.clone())
                    .child(self.remote_helper.clone()),
            )
            .child(
                div()
                    .flex()
                    .justify_between()
                    .items_center()
                    .child(div().text_xs().text_color(rgb(0x8e9caf)).child(
                        "Host keys must be enrolled out-of-band. Synara never auto-accepts or writes host keys.",
                    ))
                    .child(
                        button("open-remote-workspace", "Verify and open remote workspace", false)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.open_remote_workspace(cx)
                            })),
                    ),
            )
            .into_any_element()
    }

    pub(super) fn inspector_panel(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let trace = serde_json::to_string_pretty(&self.trace).unwrap_or_default();
        let mut panel=div().flex().flex_col().flex_1().min_h_0().p_4().gap_3()
            .child(div().text_lg().child("ACP inspector"))
            .child(self.configuration_controls(cx))
            .child(div().text_xs().text_color(rgb(0x9aabc0)).child("Bounded protocol metadata. Payload values, credentials and stderr contents are redacted before retention."))
            .child(div().flex().flex_wrap().gap_2()
                .child(button("copy-trace","Copy redacted trace",false).on_click(move |_,_,cx|cx.write_to_clipboard(gpui::ClipboardItem::new_string(trace.clone()))))
                .child(button("clear-trace","Clear trace",false).on_click(cx.listener(|this,_,_,_|{if let Some(task)=this.selected{let controller=this.controller.clone();this.job(async move {let trace=controller.trace(task,true).await?;Ok(Update::Details{task,details:controller.details(task).await?,trace})});}})))
                .child(button("restart-connection","Restart connection",false).on_click(cx.listener(|this,_,_,cx|this.connect("restart",cx))))
                .child(button("fresh-session","New agent session",false).on_click(cx.listener(|this,_,_,cx|this.connect("fresh",cx)))))
            .child(div().text_xs().text_color(rgb(0xd2b587)).child("Restart disconnects every task sharing this agent process and directory. A new session preserves the local transcript."));
        if let Some(details) = &self.details {
            panel = panel.child(
                div()
                    .p_3()
                    .rounded_md()
                    .bg(rgb(0x1b2735))
                    .child(format!(
                        "State: {:?}\nHost: {}\nAgent: {}\nSession: {}",
                        details.connection.state,
                        details.connection.host,
                        details.connection.identity.as_ref().map_or_else(
                            || "Unknown".into(),
                            |i| format!("{} {}", i.name, i.version)
                        ),
                        details.session_id.as_deref().unwrap_or("Not created")
                    ))
                    .child(
                        div()
                            .mt_2()
                            .font_family(crate::ui::code_font())
                            .text_xs()
                            .child(
                                serde_json::to_string_pretty(&details.connection.capabilities)
                                    .unwrap_or_default(),
                            ),
                    ),
            );
            if !details.connection.authentication.is_empty() {
                panel =
                    panel.child(div().flex().gap_2().children(
                        details.connection.authentication.iter().enumerate().map(
                            |(index, method)| {
                                let method_id = method.id.clone();
                                button(("inspector-login", index), method.name.clone(), false)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.authenticate(method_id.clone(), cx)
                                    }))
                            },
                        ),
                    ));
            }
        }
        panel
            .child(
                div()
                    .id("trace-list")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .children(self.trace.iter().rev().enumerate().map(|(index, entry)| {
                        div()
                            .id(("trace-entry", index))
                            .p_2()
                            .rounded_md()
                            .bg(rgb(0x17212c))
                            .font_family(crate::ui::code_font())
                            .text_xs()
                            .child(format!(
                                "#{}  {}  {}  {}  {}",
                                entry.sequence,
                                entry.direction,
                                entry.kind,
                                entry.method.as_deref().unwrap_or(""),
                                entry.request_id.as_deref().unwrap_or("")
                            ))
                            .child(
                                div()
                                    .mt_1()
                                    .text_color(rgb(0x99adc7))
                                    .child(entry.shape.clone()),
                            )
                    })),
            )
            .into_any_element()
    }
}
