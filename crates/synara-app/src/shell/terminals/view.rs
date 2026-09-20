use super::*;
use crate::ui::{self, Glyph, palette};

#[derive(Clone, Copy)]
enum Action { Start, Interrupt, Stop, Copy, Draft, Find, Rename, Close, Bottom }
impl Shell {
    fn terminal_action(&mut self, key: &Key, action: Action, window: &mut Window, cx: &mut Context<Self>) {
        if self.terminal_entry(key).is_none() || self.terminal_closing
            || self.terminals.groups.get(&key.scope).is_some_and(|group| group.loading) { return; }
        match action {
            Action::Start => self.request_terminal_action(key.scope.clone(), key.id, StopAction::Restart, cx),
            Action::Stop => self.request_terminal_action(key.scope.clone(), key.id, StopAction::Keep, cx),
            Action::Close => self.request_terminal_action(key.scope.clone(), key.id, StopAction::Close, cx),
            Action::Copy => {
                let text = self.terminal_entry(key).unwrap().view.read(cx).selection_or_viewport();
                cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
            }
            Action::Draft => self.terminal_to_draft(&key.scope, key.id, cx),
            Action::Find => {
                let entry = self.terminal_entry_mut(key).unwrap();
                entry.search = !entry.search;
                let focus = if entry.search { entry.query.read(cx).focus_handle(cx) } else { entry.view.read(cx).focus_handle(cx) };
                window.focus(&focus, cx);
            }
            Action::Rename => {
                let group = self.terminals.groups.get_mut(&key.scope).unwrap();
                let name = group.layout.tabs.iter().find(|tab| tab.id == key.id).unwrap().name.clone();
                let entry = group.entries.get(&key.id).unwrap();
                entry.name.update(cx, |input, cx| input.set_text(name, cx));
                window.focus(&entry.name.read(cx).focus_handle(cx), cx);
                group.renaming = Some(key.id);
            }
            Action::Interrupt => {
                if let Some(process) = &self.terminal_entry(key).unwrap().process
                    && let Err(error) = process.key(TerminalKey::Character('c'), TerminalModifiers { control: true, ..Default::default() })
                { self.terminal_entry_mut(key).unwrap().error = Some(error.to_string()); }
            }
            Action::Bottom => self.terminal_entry(key).unwrap().view.update(cx, |view, cx| view.follow_output(cx)),
        }
        self.focus_composer = false;
        cx.notify();
    }
    fn terminal_pane(&self, scope: &TerminalScope, id: u64, cx: &mut Context<Self>) -> gpui::AnyElement {
        let group = &self.terminals.groups[scope];
        let Some(entry) = group.entries.get(&id) else { return div().into_any_element() };
        let key = self.terminal_key(scope, id).unwrap();
        let tab = group.layout.tabs.iter().find(|tab| tab.id == id).unwrap();
        let busy = entry.starting || entry.stopping;
        let status = if entry.starting { "Starting" } else if entry.stopping { "Stopping" }
            else if entry.exited { "Exited" } else if entry.process.is_some() { "Running" } else { "Stopped" };
        let select_scope = scope.clone();
        let mut pane = div().id(("terminal-pane", id as usize)).relative()
            .child(ui::layout_probe_slot("terminal-pane", id as usize))
            .flex().flex_col().size_full().min_w_0().min_h_0()
            .border_1().border_color(rgb(palette().border)).rounded_md()
            .child(div().px_2().py_1().flex().items_center().gap_2()
                .child(ui::action(("terminal-focus", id as usize), tab.name.clone(), Some(Glyph::Terminal), group.layout.active == Some(id),
                    cx.listener(move |this, _: &(), window, cx| this.select_terminal(&select_scope, id, window, cx))).flex_1().min_w_0())
                .child(div().text_size(px(11.)).text_color(rgb(palette().muted)).child(status)))
            .child(div().px_2().pb_1().flex().flex_wrap().items_center().gap_1().children([
                ("start-shell", if entry.process.is_some() { "Restart this shell" } else { "Start this shell" }, Glyph::Terminal, Action::Start, busy),
                ("interrupt-shell", "Interrupt this shell", Glyph::Stop, Action::Interrupt, busy || entry.process.is_none() || entry.exited),
                ("stop-shell", "Stop this shell", Glyph::Close, Action::Stop, busy || entry.process.is_none() || entry.exited),
                ("terminal-find", "Find in visible rows", Glyph::Search, Action::Find, busy),
                ("terminal-copy", "Copy selection or visible rows", Glyph::Copy, Action::Copy, busy),
                ("terminal-to-chat", "Add selection or visible rows to unsent draft", Glyph::Chat, Action::Draft, busy || self.selected.is_none()),
                ("terminal-bottom", "Follow live output", Glyph::Down, Action::Bottom, busy || entry.process.is_none()),
                ("terminal-rename", "Rename terminal tab", Glyph::Compose, Action::Rename, busy),
                ("terminal-close", "Close terminal tab", Glyph::Archive, Action::Close, busy),
            ].into_iter().map(|(control, label, glyph, action, disabled)| {
                let key = key.clone();
                ui::chrome_button(control, label, glyph, disabled,
                    cx.listener(move |this, _: &(), window, cx| this.terminal_action(&key, action, window, cx)))
            })));
        if group.renaming == Some(id) {
            let save_scope = scope.clone(); let cancel_scope = scope.clone();
            pane = pane.child(div().px_2().py_1().flex().flex_wrap().gap_1()
                .child(div().flex_1().min_w(px(100.)).child(entry.name.clone()))
                .child(ui::action("terminal-save-name", "Save name", None, false,
                    cx.listener(move |this, _: &(), _, cx| this.rename_terminal(&save_scope, id, true, cx))))
                .child(ui::action("terminal-cancel-name", "Cancel", None, false,
                    cx.listener(move |this, _: &(), _, cx| this.rename_terminal(&cancel_scope, id, false, cx)))));
        }
        if entry.search {
            let previous_scope = scope.clone(); let next_scope = scope.clone();
            pane = pane.child(div().px_2().py_1().flex().flex_wrap().items_center().gap_1()
                .child(div().flex_1().min_w(px(100.)).child(entry.query.clone()))
                .child(div().text_size(px(11.)).child(format!("{}/{}", entry.matches.0, entry.matches.1)))
                .child(ui::chrome_button("terminal-find-prev", "Previous visible match", Glyph::Back, false,
                    cx.listener(move |this, _: &(), _, cx| this.search_terminal(&previous_scope, id, Some(true), cx))))
                .child(ui::chrome_button("terminal-find-next", "Next visible match", Glyph::Forward, false,
                    cx.listener(move |this, _: &(), _, cx| this.search_terminal(&next_scope, id, Some(false), cx)))))
                .child(div().px_2().pb_1().text_size(px(11.)).text_color(rgb(palette().muted)).child("Visible rows only. Scroll for earlier output."));
        }
        if let Some((confirmed_id, action)) = group.confirmation.filter(|(confirmed_id, _)| *confirmed_id == id) {
            let accept_scope = scope.clone(); let cancel_scope = scope.clone();
            let label = match action { StopAction::Keep => "Stop shell", StopAction::Close => "Stop and close", StopAction::Restart => "Stop and restart" };
            pane = pane.child(div().p_2().bg(rgb(palette().notice_surface)).flex().flex_col().gap_1()
                .child(div().text_size(px(12.)).child(format!("{label} in {}? Active commands may be interrupted. Other terminal tabs are unaffected.", tab.name)))
                .child(div().flex().flex_wrap().gap_2()
                    .child(ui::action("terminal-confirm", label, None, false, cx.listener(move |this, _: &(), _, cx| {
                        if let Some(group) = this.terminals.groups.get_mut(&accept_scope) { group.confirmation = None; }
                        this.stop_terminal_entry(accept_scope.clone(), confirmed_id, action, cx);
                    })))
                    .child(ui::action("terminal-keep", "Keep running", None, false, cx.listener(move |this, _: &(), _, cx| {
                        if let Some(group) = this.terminals.groups.get_mut(&cancel_scope) { group.confirmation = None; }
                        cx.notify();
                    })))));
        }
        let metadata = entry.view.read(cx);
        if let Some(title) = metadata.title().filter(|title| !title.is_empty()) {
            pane = pane.child(div().px_2().text_size(px(11.)).text_color(rgb(palette().muted))
                .min_w_0().text_ellipsis().child(title.chars().take(160).collect::<String>()));
        }
        if let Some(directory) = metadata.cwd_hint().filter(|directory| !directory.is_empty()) {
            pane = pane.child(div().px_2().text_size(px(11.)).text_color(rgb(palette().muted))
                .min_w_0().text_ellipsis().child(format!("Shell directory hint: {}", directory.chars().take(160).collect::<String>())));
        }
        // Render owned errors independently from an exit code. A read failure is
        // not an exited process and never silently frees its slot.
        if let Some(error) = entry.error.as_deref().or(entry.view.read(cx).terminal_error()) {
            pane = pane.child(div().px_2().py_1().text_size(px(12.)).text_color(rgb(palette().error)).child(error.to_owned()));
        }
        pane = pane.child(if busy {
            div().flex_1().min_h_0().p_3().text_size(px(12.)).child(if entry.starting { "Starting this shell..." } else { "Waiting for this shell to stop..." }).into_any_element()
        } else if entry.process.is_none() && !entry.exited {
            let start = key.clone();
            div().flex_1().min_h_0().p_3().flex().flex_col().justify_center().items_center().gap_2()
                .child(div().text_size(px(12.)).child("This tab is stopped. Restoring tabs never runs commands."))
                .child(ui::action("terminal-start-empty", "Start shell", Some(Glyph::Terminal), false,
                    cx.listener(move |this, _: &(), window, cx| this.terminal_action(&start, Action::Start, window, cx))))
                .into_any_element()
        } else {
            div().id("terminal-screen").relative().child(ui::layout_probe("terminal-screen"))
                .flex_1().min_h_0().min_w_0().overflow_hidden().child(entry.view.clone()).into_any_element()
        });
        if let Some(code) = entry.view.read(cx).exit_code() {
            pane = pane.child(div().px_2().py_1().text_size(px(11.)).text_color(rgb(palette().muted)).child(format!("Exit status {code}. Output retained until this tab is closed or restarted.")));
        }
        pane.into_any_element()
    }
    pub(in crate::shell) fn terminal_panel(&self, width: f32, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(scope) = self.terminal_scope() else { return div().p_3().child("Open a project to use terminals.").into_any_element() };
        let Some(group) = self.terminals.groups.get(&scope) else {
            return div().p_3().child("Terminal workspace is not loaded. Up to 32 project workspaces can be open in one app session.").into_any_element();
        };
        if group.loading || !group.loaded {
            let retry_scope = scope.clone();
            return div().p_3().flex().flex_col().gap_2()
                .child(if group.loading { "Restoring terminal tabs. No shell is being started." } else { "Terminal layout could not be loaded. The saved value was preserved." })
                .children(group.load_error.as_ref().map(|error| div().text_color(rgb(palette().error)).child(error.clone())))
                .child(ui::action("terminal-reload-initial", "Retry", None, false,
                    cx.listener(move |this, _: &(), _, cx| this.load_terminals(retry_scope.clone(), cx))))
                .into_any_element();
        }
        let mut root = div().id("terminal-workspace").relative().child(ui::layout_probe("terminal-workspace"))
            .flex_1().min_w_0().min_h_0().flex().flex_col().bg(rgb(palette().canvas)).p_2().gap_2()
            .child(div().flex().flex_wrap().items_center().gap_1()
                .child(ui::action("terminal-new-shell", "New shell", Some(Glyph::Plus), false,
                    cx.listener(|this, _: &(), _, cx| this.new_terminal(true, cx))))
                .child(ui::action("terminal-new-tab", "New stopped tab", None, false,
                    cx.listener(|this, _: &(), _, cx| this.new_terminal(false, cx))))
                .child(ui::action("terminal-split", if group.layout.secondary.is_some() { "Single pane" } else { "Split" }, Some(Glyph::Window), false,
                    cx.listener(|this, _: &(), _, cx| this.split_terminals(cx))))
                .children(group.layout.secondary.map(|_| ui::action("terminal-axis", if group.layout.stacked { "Side by side" } else { "Stack panes" }, None, false,
                    cx.listener(|this, _: &(), _, cx| {
                        if let Some(scope) = this.terminal_scope() && let Some(group) = this.terminals.groups.get_mut(&scope) { group.layout.stacked = !group.layout.stacked; group.changed(); }
                        cx.notify();
                    }))))
                .children(group.layout.secondary.map(|_| div().flex().items_center().gap_1().children([
                    ("terminal-shrink", "Less room for first pane", Glyph::Minimize, -5i16),
                    ("terminal-balance", "Balance terminal panes", Glyph::Restore, 0i16),
                    ("terminal-grow", "More room for first pane", Glyph::Plus, 5i16),
                ].into_iter().map(|(id, label, glyph, amount)| ui::chrome_button(id, label, glyph, false,
                    cx.listener(move |this, _: &(), _, cx| {
                        if let Some(scope) = this.terminal_scope() && let Some(group) = this.terminals.groups.get_mut(&scope) {
                            group.layout.primary_percent = if amount == 0 { 50 } else { (i16::from(group.layout.primary_percent) + amount).clamp(25, 75) as u8 };
                            group.changed();
                        }
                        cx.notify();
                    })))))))
            .child(div().text_size(px(11.)).text_color(rgb(palette().muted)).text_ellipsis()
                .child(format!("{} · {}", if matches!(group.target, WorkspaceTarget::Ssh { .. }) { "SSH" } else { "Local" }, scope.root.display())))
            .child(div().id("terminal-tabs").overflow_x_scroll().flex().gap_1().flex_shrink_0().children(group.layout.tabs.iter().map(|tab| {
                let select_scope = scope.clone(); let id = tab.id;
                let entry = &group.entries[&id];
                let running = entry.starting || entry.process.is_some() && !entry.exited;
                ui::action(("terminal-tab", id as usize), format!("{}{}", tab.name, if running { " •" } else { "" }), Some(Glyph::Terminal), group.layout.active == Some(id),
                    cx.listener(move |this, _: &(), window, cx| this.select_terminal(&select_scope, id, window, cx)))
                    .flex_shrink_0().max_w(px(180.)).relative().child(ui::layout_probe_slot("terminal-tab", id as usize))
            })));
        if let Some(error) = group.save_error.as_ref().or(group.load_error.as_ref()) {
            let copy_scope = scope.clone(); let retry_scope = scope.clone(); let reload_scope = scope.clone();
            root = root.child(div().p_2().bg(rgb(palette().error_surface)).flex().flex_col().gap_1()
                .child(div().text_size(px(12.)).child(error.clone()))
                .child(div().flex().flex_wrap().gap_2()
                    .child(ui::action("terminal-copy-layout", "Copy local layout", None, false,
                        cx.listener(move |this, _: &(), _, cx| {
                            if let Some(group) = this.terminals.groups.get(&copy_scope)
                                && let Ok(text) = serde_json::to_string_pretty(&group.layout)
                            { cx.write_to_clipboard(gpui::ClipboardItem::new_string(text)); }
                        })))
                    .child(ui::action("terminal-retry-save", "Retry saving", None, false,
                        cx.listener(move |this, _: &(), _, cx| {
                            if let Some(group) = this.terminals.groups.get_mut(&retry_scope) { group.save_error = None; }
                            this.flush_terminal_layouts(true); cx.notify();
                        })))
                    .child(ui::action("terminal-reload-layout", if group.reload_confirm { "Discard local layout and output" } else { "Reload saved layout..." }, None, false,
                        cx.listener(move |this, _: &(), _, cx| {
                            let Some(group) = this.terminals.groups.get_mut(&reload_scope) else { return };
                            if group.running() || group.saving { this.notice = Some("Stop all shells in this workspace and finish saving before reloading its layout.".into()); }
                            else if group.reload_confirm { this.load_terminals(reload_scope.clone(), cx); }
                            else { group.reload_confirm = true; }
                            cx.notify();
                        })))));
        }
        let stacked = group.layout.stacked || width < 640.;
        let ratio = f32::from(group.layout.primary_percent) / 100.;
        if let Some(primary) = group.layout.active {
            let pane = self.terminal_pane(&scope, primary, cx);
            root = root.child(if let Some(secondary) = group.layout.secondary {
                div().flex_1().min_h_0().min_w_0().flex().when(stacked, |el| el.flex_col())
                    .child(div().min_h_0().min_w_0().flex_shrink_0()
                        .when(stacked, |el| el.h(gpui::relative(ratio)).w_full())
                        .when(!stacked, |el| el.w(gpui::relative(ratio)).h_full()).child(pane))
                    .child(div().flex_1().min_h_0().min_w_0().when(stacked, |el| el.pt_1()).when(!stacked, |el| el.pl_1())
                        .child(self.terminal_pane(&scope, secondary, cx))).into_any_element()
            } else { div().flex_1().min_h_0().min_w_0().child(pane).into_any_element() });
        } else {
            root = root.child(div().flex_1().p_3().child("No terminal tabs. New stopped tab creates a workspace without executing commands."));
        }
        root.child(div().text_size(px(11.)).text_color(rgb(palette().muted))
            .child(if group.saving { "Saving tab layout..." } else if group.edits != group.saved_edits { "Tab layout has unsaved changes." } else { "Tab layout saved. Processes and output are not restored after restart." }))
            .into_any_element()
    }
}
