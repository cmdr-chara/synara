use super::*;
impl Shell {
    pub(super) fn files_panel(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let dirty = self.dirty(cx);
        div().flex().flex_1().min_h_0().p_4().gap_4()
            .child(div().w(px(260.)).flex_shrink_0().flex().flex_col().gap_3()
                .child(div().flex().gap_2().child(button("file-up","Up",false).on_click(cx.listener(|this,_,_,_|{this.directory.pop();this.refresh_files();})))
                    .child(button("files-refresh","Refresh",false).on_click(cx.listener(|this,_,_,_|this.refresh_files()))))
                .child(div().text_xs().text_color(rgb(0xa2aec0)).child(if self.directory.as_os_str().is_empty(){"Project root".into()}else{self.directory.display().to_string()}))
                .child(div().id("file-list").flex_1().min_h_0().overflow_y_scroll().flex().flex_col().gap_1().children(self.files.iter().skip(self.file_page*400).take(400).enumerate().map(|(index,file)|{
                    let path=file.relative_path.clone();let directory=file.directory;let symlink=file.symlink;
                    div().id(("file",index)).p_2().rounded_sm().cursor_pointer().hover(|s|s.bg(rgb(0x2c394a)))
                        .on_click(cx.listener(move |this,_,_,cx|{
                            if symlink{this.error=Some("Symlink navigation is disabled at the workspace boundary.".into());cx.notify();}
                            else if directory{this.directory=path.clone();this.refresh_files();}
                            else{this.open_file(path.clone(),cx);}
                        }))
                        .child(format!("{} {}",if symlink{"[link]"}else if directory{"[dir]"}else{""},file.name))
                })))
                .child(div().flex().gap_2().children((self.file_page>0).then(||button("previous-files","Previous",false).on_click(cx.listener(|this,_,_,cx|{this.file_page=this.file_page.saturating_sub(1);cx.notify();}))))
                    .children(((self.file_page+1)*400<self.files.len()).then(||button("next-files","Next",false).on_click(cx.listener(|this,_,_,cx|{this.file_page+=1;cx.notify();}))))))
            .child(div().flex_1().min_w_0().flex().flex_col().gap_3()
                .child(div().flex().items_center().justify_between().gap_3()
                    .child(div().font_weight(gpui::FontWeight::SEMIBOLD).child(self.document.as_ref().map_or_else(||"Native editor".into(),|d|format!("{}{}",d.path.display(),if dirty{" *"}else{""}))))
                    .child(div().flex().gap_2().child(button("save-document",if self.saving{"Saving..."}else{"Save"},dirty).on_click(cx.listener(|this,_,_,cx|this.save_file(cx))))
                        .child(button("discard-document","Discard edits",false).on_click(cx.listener(|this,_,_,cx|{if !this.saving{if let Some(document)=&this.document{this.editor.update(cx,|entry,cx|entry.set_text(document.snapshot.text.clone(),cx));}cx.notify();}})))))
                .child(div().text_xs().text_color(rgb(0x94a3b8)).child("UTF-8 · guarded saves · undo/redo · Ctrl+S to save · up to 1 MiB"))
                .child(self.editor.clone())
                .children(self.editor.read(cx).error.as_ref().map(|error|div().text_color(rgb(0xffb1b5)).child(error.clone())))
                .children(self.document.is_none().then(||div().p_3().child("Select a file on the left. Binary, oversized and symlink targets produce explicit errors rather than being silently converted."))))
            .into_any_element()
    }
    pub(super) fn git_panel(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        div().flex().flex_col().flex_1().min_h_0().p_4().gap_3()
            .child(div().flex().justify_between().items_center().child(div().text_lg().child(format!("Changes · {}",self.git.branch)))
                .child(div().flex().gap_2().child(button("unstaged-diff","Unstaged",!self.staged).on_click(cx.listener(|this,_,_,cx|{this.staged=false;this.refresh_git();cx.notify();})))
                    .child(button("staged-diff","Staged",self.staged).on_click(cx.listener(|this,_,_,cx|{this.staged=true;this.refresh_git();cx.notify();})))
                    .child(button("refresh-git","Refresh",false).on_click(cx.listener(|this,_,_,_|this.refresh_git())))))
            .child(div().flex().flex_1().min_h_0().gap_4()
                .child(div().id("git-file-list").w(px(300.)).flex_shrink_0().overflow_y_scroll().flex().flex_col().gap_2().children(self.git.entries.iter().enumerate().map(|(index,entry)|{
                    let path=entry.path.clone();let unstage=entry.staged();
                    div().p_2().rounded_md().bg(rgb(0x1b2532)).child(format!("{}{}  {}",entry.index_status,entry.worktree_status,entry.path.display()))
                        .child(button(("git-action",index),if unstage{"Unstage"}else{"Stage"},false).mt_2().on_click(cx.listener(move |this,_,_,_|{
                            if let Some(root)=this.root(){let path=path.clone();this.job(async move {let git=GitService::new(root);if unstage{git.unstage(path).await?;}else{git.stage(path).await?;}Ok(Update::Done("Index updated".into()))});}
                        })))
                })).children(self.git.entries.is_empty().then(||div().p_3().child("No changed files"))))
                .child(div().id("diff-output").flex_1().min_w_0().overflow_y_scroll().p_3().rounded_md().bg(rgb(0x151e28)).font_family("DejaVu Sans Mono").text_xs().child(if self.diff.is_empty(){"No diff in this view. Untracked files must be staged before Git can show their diff.".into()}else{truncate(&self.diff,256*1024)})))
            .child(self.commit_message.clone())
            .child(div().flex().justify_between().items_center().child(div().text_xs().text_color(rgb(0x96a5b9)).child("Commit writes only to the selected local workspace. Hooks and signing are disabled for this action."))
                .child(button("commit-staged","Commit staged changes",false).on_click(cx.listener(|this,_,_,cx|{
                    let Some(root)=this.root() else{return};let message=this.commit_message.read(cx).text().to_owned();
                    if message.trim().is_empty(){this.error=Some("Enter a commit message first.".into());cx.notify();return;}
                    this.job(async move {GitService::new(root).commit(message).await?;Ok(Update::Done("Commit created locally".into()))});
                }))))
            .into_any_element()
    }
    pub(super) fn terminal_panel(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let terminal = self.terminal.clone();
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
                                button("start-shell", "Start shell", false).on_click(
                                    cx.listener(|this, _, _, cx| this.start_terminal(cx)),
                                ),
                            )
                            .child(button("interrupt-shell", "Interrupt", false).on_click(
                                cx.listener(|this, _, _, _| this.terminal_input(vec![3])),
                            ))
                            .child(button("stop-shell", "Stop shell", false).on_click(
                                cx.listener(move |this, _, _, _| {
                                    if let Some(terminal) = terminal.clone() {
                                        this.job(async move {
                                            tokio::task::spawn_blocking(move || terminal.kill())
                                                .await
                                                .map_err(|_| WorkspaceError::Worker)??;
                                            Ok(Update::Done("Shell stopped".into()))
                                        });
                                    }
                                }),
                            )),
                    ),
            )
            .child(div().text_xs().text_color(rgb(0x9cabbd)).child(
                self.terminal_root.as_ref().map_or_else(
                    || "No running shell".into(),
                    |root| format!("Shell directory: {}", root.display()),
                ),
            ))
            .child(
                div()
                    .id("terminal-screen")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .p_4()
                    .rounded_md()
                    .bg(rgb(0x0b1017))
                    .font_family("DejaVu Sans Mono")
                    .child(self.terminal_snapshot.as_ref().map_or_else(
                        || "Start a shell to run commands in this workspace.".into(),
                        |s| s.text.clone(),
                    )),
            )
            .children(
                self.terminal_snapshot
                    .as_ref()
                    .and_then(|s| s.exit_code)
                    .map(|code| {
                        div().text_xs().child(format!(
                            "Shell exited with status {code}. Historical output is retained."
                        ))
                    }),
            )
            .children(
                self.terminal_snapshot
                    .as_ref()
                    .and_then(|s| s.error.as_ref())
                    .map(|error| div().text_color(rgb(0xffb1b5)).child(error.clone())),
            )
            .child(self.terminal_command.clone())
            .child(
                div()
                    .flex()
                    .justify_between()
                    .items_center()
                    .child("Enter sends the command to the shell.")
                    .child(
                        button("terminal-send", "Run command", false)
                            .on_click(cx.listener(|this, _, _, cx| this.send_terminal(cx))),
                    ),
            )
            .into_any_element()
    }
    pub(super) fn inspector_panel(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let trace = serde_json::to_string_pretty(&self.trace).unwrap_or_default();
        let mut panel=div().flex().flex_col().flex_1().min_h_0().p_4().gap_3()
            .child(div().text_lg().child("ACP inspector"))
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
                            .font_family("DejaVu Sans Mono")
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
                            .font_family("DejaVu Sans Mono")
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
    pub(super) fn settings_panel(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        div().id("settings-view").flex_1().min_h_0().overflow_y_scroll().p_5().flex().flex_col().gap_3()
            .child(div().text_2xl().child("Agent settings"))
            .child("Every profile runs through the same ACP backend. Configure an installed executable, its arguments, and any environment variable names it is allowed to inherit.")
            .child("Models, modes and authentication are discovered from the connected agent. Synara does not maintain vendor model catalogs.")
            .child(div().p_3().rounded_md().bg(rgb(0x263043)).font_family("DejaVu Sans Mono").text_xs().child("inherit_env contains names only, for example [\"MY_AGENT_TOKEN\"]. Values are read from the launch environment and never saved to the profile database. Do not place secrets in command arguments."))
            .child(self.profile_editor.clone())
            .child(div().flex().gap_2().child(button("apply-profiles","Save profiles",true).on_click(cx.listener(|this,_,_,cx|this.save_profiles(cx))))
                .child(button("reset-profiles","Load launch presets",false).on_click(cx.listener(|this,_,_,cx|{this.profile_editor.update(cx,|entry,cx|entry.set_text(serde_json::to_string_pretty(&default_profiles()).unwrap_or_default(),cx));cx.notify();}))))
            .child(div().mt_4().text_lg().child("Storage and safety"))
            .child("Workspaces, tasks and normalized conversation events are saved locally in SQLite. Use --data-dir to select a separate data directory. Pending permissions are never restored as approvals after an interrupted session.")
            .child("Filesystem callbacks are constrained to the selected workspace. Native file editing preserves UTF-8 BOMs and checks for external modifications before saving. Symlink navigation is disabled.")
            .into_any_element()
    }
}
