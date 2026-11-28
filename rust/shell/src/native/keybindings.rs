//! Extracted from native/mod.rs during the surface split — see mod.rs.
#![allow(unused_imports)]

use super::*;
use gpui::prelude::FluentBuilder as _;
use gpui_kit::assets::IconName as AssetIcon;
use gpui_kit::base::Disableable as _;
use gpui_kit::component::WindowExt as _;
use gpui_kit::component::{
    button::{Button, ButtonCustomVariant, ButtonRounded, ButtonVariant, ButtonVariants as _},
    h_flex,
    input::{Editor, EditorState, Input, InputEvent, InputState},
    menu::{DropdownMenu as _, PopupMenuItem},
    scroll::ScrollableElement as _,
    slider::{Slider, SliderEvent, SliderState},
    spinner::Spinner,
    text::TextView,
    theme::Theme,
    v_flex, ActiveTheme as _, Icon, IconName, Sizable as _,
};
use gpui_kit::*;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

impl Native {
    /// Open the recorder dialog for a shortcut row. `replacing` holds the
    /// resolved rules of the binding being edited (empty when adding one).
    pub(super) fn sc_open(
        &mut self,
        label: String,
        commands: Vec<String>,
        numbered: bool,
        replacing: Vec<Value>,
        cx: &mut Context<Self>,
    ) {
        self.sc_rec = Some(ScRec {
            label,
            commands,
            numbered,
            replacing,
            captured: None,
        });
        cx.notify();
    }

    /// `evaluateRecordedShortcut`: what saving `captured` would do.
    /// Returns (problem_message, conflicts) — empty problem + empty
    /// conflicts means "{chord} is available."
    pub(super) fn sc_evaluate(&self, rec: &ScRec) -> (Option<String>, Vec<(String, Value)>) {
        let Some(cap) = &rec.captured else {
            return (None, Vec::new());
        };
        let label = sc_fmt_captured(cap);
        if rec.numbered
            && !(cap.key.len() == 1 && cap.key.as_bytes()[0].is_ascii_digit() && cap.key != "0")
        {
            return (
                Some("Hold the modifiers and press a number key.".to_string()),
                Vec::new(),
            );
        }
        if !(cap.m_mod || cap.m_ctrl || cap.m_alt) && !sc_is_fkey(&cap.key) {
            return (
                Some(
                    "Add \u{2318}, \u{2325} or \u{2303} to that key, or use an F-key on its own."
                        .to_string(),
                ),
                Vec::new(),
            );
        }
        let enc = sc_enc_captured(cap);
        let mut conflicts: Vec<(String, Value)> = Vec::new();
        for r in &self.kb_rules {
            if rec
                .replacing
                .iter()
                .any(|rep| std::ptr::eq(rep, r) || rep == r)
            {
                continue;
            }
            let sc = r.get("shortcut").cloned().unwrap_or(Value::Null);
            if sc_enc_shortcut(&sc) != enc {
                continue;
            }
            let rc = r
                .get("command")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            if rec.commands.iter().any(|c| *c == rc) {
                return (
                    Some(format!("{label} is already a shortcut for this command.")),
                    Vec::new(),
                );
            }
            if !conflicts.iter().any(|(c, _)| *c == rc) {
                conflicts.push((rc, r.clone()));
            }
        }
        (None, conflicts)
    }

    /// Command label for a conflict message: the row title, else the raw
    /// command id.
    pub(super) fn sc_command_label(cmd: &str) -> String {
        for (title, _d, _k, commands) in &SHORTCUT_ROWS {
            if commands.split(' ').any(|c| c == cmd) {
                return title.to_string();
            }
        }
        cmd.to_string()
    }

    /// `shortcutSaveEdits` equivalent: remove conflicts, then set the new
    /// rule(s) replacing the edited binding.
    pub(super) fn sc_save(&mut self, cx: &mut Context<Self>) {
        let Some(rec) = self.sc_rec.as_ref() else {
            return;
        };
        let commands = rec.commands.clone();
        let numbered = rec.numbered;
        let replacing = rec.replacing.clone();
        let Some(cap) = rec.captured.clone() else {
            return;
        };
        let (problem, conflicts) = self.sc_evaluate(rec);
        if problem.is_some() {
            return;
        }
        let mut edits: Vec<Value> = Vec::new();
        for (_cmd, rule) in &conflicts {
            edits.push(json!({"type": "remove", "rule": sc_rule_from_resolved(rule)}));
        }
        let enc = sc_enc_captured(&cap);
        if numbered {
            // The family gets one rule per member on the captured modifiers.
            let prefix = enc
                .rsplit_once('+')
                .map(|(p, _)| format!("{p}+"))
                .unwrap_or_default();
            for (i, cmd) in commands.iter().enumerate() {
                let mut edit = json!({"type": "set", "rule": {"command": cmd, "key": format!("{prefix}{}", i + 1)}});
                if let Some(rep) = replacing
                    .iter()
                    .find(|r| r.get("command").and_then(Value::as_str) == Some(cmd.as_str()))
                {
                    edit["replacing"] = sc_rule_from_resolved(rep);
                }
                edits.push(edit);
            }
        } else {
            for (i, cmd) in commands.iter().enumerate() {
                let mut edit = json!({"type": "set", "rule": {"command": cmd, "key": enc}});
                if let Some(rep) = replacing.get(i).or_else(|| replacing.first()) {
                    edit["replacing"] = sc_rule_from_resolved(rep);
                }
                edits.push(edit);
            }
        }
        self.apply_kb_edits(edits, cx);
    }

    pub(super) fn sc_remove_binding(&mut self, rules: &[Value], cx: &mut Context<Self>) {
        let edits: Vec<Value> = rules
            .iter()
            .map(|r| json!({"type": "remove", "rule": sc_rule_from_resolved(r)}))
            .collect();
        self.apply_kb_edits(edits, cx);
    }

    /// Apply a batch of keybinding edits through `server.editKeybindings`
    /// and adopt the resolved rules it returns.
    pub(super) fn apply_kb_edits(&mut self, edits: Vec<Value>, cx: &mut Context<Self>) {
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else {
                return;
            };
            let reply = cx
                .background_executor()
                .spawn(
                    async move { client.call("server.editKeybindings", json!({"edits": edits})) },
                )
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if let Some(rules) =
                    reply.and_then(|v| v.get("keybindings").and_then(|k| k.as_array()).cloned())
                {
                    this.kb_rules = rules.clone();
                    if !this.settings_data.is_null() {
                        if let Some(cfg) = this.settings_data.get_mut("config") {
                            cfg["keybindings"] = json!(rules);
                        }
                    }
                }
                this.sc_rec = None;
                cx.notify();
            });
        })
        .detach();
    }

    /// Route a key event while the recorder is open: Escape cancels, Enter
    /// saves, everything else is captured as the candidate chord.
    pub(super) fn sc_rec_key(
        &mut self,
        event: &KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let m = event.keystroke.modifiers;
        let key = event.keystroke.key.as_str().to_string();
        match key.as_str() {
            "escape" => {
                self.sc_rec = None;
                cx.notify();
            }
            "enter" => self.sc_save(cx),
            "shift" | "control" | "alt" | "meta" | "caps_lock" | "tab" | "function" => {}
            _ => {
                if let Some(r) = &mut self.sc_rec {
                    r.captured = Some(ScCaptured {
                        key,
                        m_mod: m.platform,
                        m_ctrl: m.control,
                        m_alt: m.alt,
                        m_shift: m.shift,
                    });
                }
                cx.notify();
            }
        }
    }

    /// Resolve a key event to a command using the resolved keybinding
    /// rules (mac: `mod`/`meta` both mean the platform ⌘ key).
    pub(super) fn kb_command_for_event(&self, event: &KeyDownEvent) -> Option<String> {
        let m = event.keystroke.modifiers;
        let key = sc_norm_rule_key(&event.keystroke.key);
        for r in &self.kb_rules {
            if !sc_when_ok(r.get("when").and_then(Value::as_str)) {
                continue;
            }
            let sc = r.get("shortcut").cloned().unwrap_or(Value::Null);
            let cmd_key = sc.get("key").and_then(Value::as_str).unwrap_or("");
            if cmd_key.is_empty() || sc_norm_rule_key(cmd_key) != key {
                continue;
            }
            if (sc_flag(&sc, "modKey") || sc_flag(&sc, "metaKey")) != m.platform {
                continue;
            }
            if sc_flag(&sc, "ctrlKey") != m.control
                || sc_flag(&sc, "altKey") != m.alt
                || sc_flag(&sc, "shiftKey") != m.shift
            {
                continue;
            }
            if let Some(c) = r.get("command").and_then(Value::as_str) {
                return Some(c.to_string());
            }
        }
        None
    }

    /// Run a keybinding command. Returns false for commands the shell has
    /// no action for (model pickers, editor.*, snooze, …) so the caller can
    /// try the next matching rule.
    pub(super) fn run_command(
        &mut self,
        cmd: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        match cmd {
            "chat.new"
            | "chat.newChat"
            | "chat.newLocal"
            | "chat.newClaude"
            | "chat.newCodex"
            | "chat.newCursor"
            | "chat.newLatestProject" => self.new_thread(cx),
            "threadTab.close" | "terminal.close" | "terminal.workspace.closeActive" => {
                if self.split_open && self.focused_pane == 1 {
                    if self.split_thread.is_some() {
                        self.split_thread = None;
                        self.split_detail = None;
                    } else {
                        self.split_open = false;
                    }
                    self.focused_pane = 0;
                    cx.notify();
                } else if let Some(t) = self.selected_thread.clone() {
                    self.close_tab(t, cx);
                }
            }
            "chat.split" => {
                if self.split_open {
                    self.close_split(cx);
                } else if self.selected_thread.is_some() {
                    self.split_open = true;
                    cx.notify();
                }
            }
            "composer.focus.toggle" => {
                let f = self.composer.read(cx).focus_handle(cx);
                window.focus(&f, cx);
            }
            "sidebar.search" => {
                if self.palette_mode == Some("search") {
                    self.close_palette(window, cx);
                } else {
                    self.open_palette("search", window, cx);
                }
            }
            "search.files" => self.open_palette("files", window, cx),
            "search.content" => self.open_palette("snippets", window, cx),
            "sidebar.toggle" => {
                self.sidebar_collapsed = !self.sidebar_collapsed;
                cx.notify();
            }
            "settings.usage" => {
                self.clear_views();
                self.view_settings = true;
                self.settings_section = "usage";
                if self.settings_data.is_null() {
                    self.fetch_settings(cx);
                }
                cx.notify();
            }
            "chat.find" => self.toggle_find(window, cx),
            "chat.newTerminal"
            | "terminal.new"
            | "terminal.toggle"
            | "terminal.workspace.newFullWidth"
            | "terminal.workspace.terminal" => self.toggle_terminal(window, cx),
            "terminal.workspace.chat" => {
                if self.view_terminal {
                    self.toggle_terminal(window, cx);
                }
            }
            "sidebar.activity" => {
                self.clear_views();
                self.view_inbox = true;
                self.fetch_inbox(cx);
                cx.notify();
            }
            "thread.copyId" => {
                if let Some(t) = self.selected_thread.clone() {
                    cx.write_to_clipboard(ClipboardItem::new_string(t));
                }
            }
            "thread.archive" => self.archive_selected(cx),
            "chat.visible.previous"
            | "view.recent.previous"
            | "threadTab.previous"
            | "space.previous" => self.cycle_tab(-1, cx),
            "chat.visible.next"
            | "view.recent.next"
            | "threadTab.next"
            | "chat.attention.next"
            | "space.next" => self.cycle_tab(1, cx),
            "diff.toggle" | "git.commitAndPush" => self.toggle_changes(cx),
            "computer.toggle" => self.toggle_cua(window, cx),
            "browser.toggle" => self.toggle_browser(window, cx),
            c if c.starts_with("thread.jump.") => {
                if let Ok(i) = c["thread.jump.".len()..].parse::<usize>() {
                    if let Some(t) = self.open_threads.get(i.saturating_sub(1)).cloned() {
                        self.open_thread(t, cx);
                    }
                }
            }
            _ => return false,
        }
        true
    }

    pub(super) fn handle_chord(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // A docked blocking userInput form owns plain digits (upstream
        // global digit shortcuts pick option 1–9) and Enter advances or
        // submits — but only while no input holds focus, where digits stay
        // composer text (the custom answer).
        let m = event.keystroke.modifiers;
        if !m.platform && !m.control && !m.alt && !m.function {
            let tv = self
                .detail
                .as_ref()
                .map(|d| get(d, "thread"))
                .cloned()
                .unwrap_or(Value::Null);
            if let Some((rid, gen, questions)) = active_pending_input(&tv) {
                let other_focused = window
                    .focused(cx)
                    .map(|f| f != self.focus_handle)
                    .unwrap_or(false);
                if !other_focused {
                    let key = event.keystroke.key.as_str();
                    if key == "enter" || key == "return" {
                        self.pi_advance(&rid, &questions, gen, cx);
                        return;
                    }
                    if key.len() == 1 {
                        let digit = key.as_bytes()[0];
                        if digit.is_ascii_digit() && digit != b'0' {
                            let oi = (digit - b'1') as usize;
                            let qidx = *self.pi_qidx.get(&rid).unwrap_or(&0);
                            if let Some(q) = questions.get(qidx) {
                                let multi = get(q, "multiSelect").as_bool().unwrap_or(false);
                                if let Some(opt) = arr(q, "options").get(oi) {
                                    let qid = str_at(q, "id").to_string();
                                    let label = str_at(opt, "label").to_string();
                                    self.pi_toggle(&rid, &qid, &label, multi, cx);
                                    if !multi {
                                        self.pi_advance(&rid, &questions, gen, cx);
                                    }
                                }
                            }
                            return;
                        }
                    }
                }
            }
        }
        // ⌘S saves the open file while it is being edited (upstream
        // useCodeEditorSaveKeyDownHandler).
        if event.keystroke.modifiers.platform
            && event.keystroke.key.as_str() == "s"
            && !event.keystroke.modifiers.shift
            && !event.keystroke.modifiers.control
            && !event.keystroke.modifiers.alt
            && self.editing_file
        {
            self.save_file(cx);
            return;
        }
        // While the recorder dialog is open every key is input for it.
        if self.sc_rec.is_some() {
            self.sc_rec_key(event, window, cx);
            return;
        }
        // Once keybinding rules are loaded they own shortcut dispatch
        // (they already include the defaults). ⌘, is not a server rule —
        // keep it hardcoded.
        if !self.kb_rules.is_empty() {
            let m = event.keystroke.modifiers;
            if m.function {
                return;
            }
            if let Some(cmd) = self.kb_command_for_event(event) {
                let _ = self.run_command(&cmd, window, cx);
            } else {
                let key = event.keystroke.key.as_str();
                if m.platform && !m.control && !m.alt && !m.shift && key == "," {
                    self.clear_views();
                    self.view_settings = true;
                    if self.settings_data.is_null() {
                        self.fetch_settings(cx);
                    }
                    cx.notify();
                } else if m.platform && !m.control && !m.alt && !m.shift && key == "/" {
                    self.open_sc_dialog(window, cx);
                }
            }
            return;
        }
        let m = event.keystroke.modifiers;
        if !m.platform || m.function {
            return;
        }
        let key = event.keystroke.key.as_str();
        match (m.control, m.alt, m.shift, key) {
            // ⌘ — chat.new / chat.newChat
            (false, false, false, "n") | (false, true, false, "n") => self.new_thread(cx),
            // ⌘W — threadTab.close (split: closes the focused pane first)
            (false, false, false, "w") => {
                if self.split_open && self.focused_pane == 1 {
                    if self.split_thread.is_some() {
                        self.split_thread = None;
                        self.split_detail = None;
                    } else {
                        self.split_open = false;
                    }
                    self.focused_pane = 0;
                    cx.notify();
                } else if let Some(t) = self.selected_thread.clone() {
                    self.close_tab(t, cx);
                }
            }
            // ⌘\ — chat.split
            (false, false, false, "\\") => {
                if self.split_open {
                    self.close_split(cx);
                } else if self.selected_thread.is_some() {
                    self.split_open = true;
                    cx.notify();
                }
            }
            // ⌘L — composer.focus.toggle
            (false, false, false, "l") => {
                let f = self.composer.read(cx).focus_handle(cx);
                window.focus(&f, cx);
            }
            // ⌘K — sidebar.search: chats+projects palette (toggles).
            (false, false, false, "k") => {
                if self.palette_mode == Some("search") {
                    self.close_palette(window, cx);
                } else {
                    self.open_palette("search", window, cx);
                }
            }
            // ⌘P — search.files: workspace file-name palette.
            (false, false, false, "p") => self.open_palette("files", window, cx),
            // ⌘/ — upstream keyboard-shortcuts-help dialog.
            (false, false, false, "/") => self.open_sc_dialog(window, cx),
            // ⌘⇧F — search.content: workspace code-snippet palette.
            (false, false, true, "f") => self.open_palette("snippets", window, cx),
            // ⌘B — sidebar collapse
            (false, false, false, "b") => {
                self.sidebar_collapsed = !self.sidebar_collapsed;
                cx.notify();
            }
            // ⌘, — settings
            (false, false, false, ",") => {
                self.clear_views();
                self.view_settings = true;
                if self.settings_data.is_null() {
                    self.fetch_settings(cx);
                }
                cx.notify();
            }
            // ⌘1..⌘9 — thread.jump.N
            (false, false, false, d) if d.len() == 1 => {
                let i = d
                    .as_bytes()
                    .first()
                    .filter(|b| b.is_ascii_digit() && **b != b'0')
                    .map(|b| (b - b'1') as usize);
                if let Some(t) = i.and_then(|i| self.open_threads.get(i).cloned()) {
                    self.open_thread(t, cx);
                }
            }
            // ⌘⇧T / ⌘⇧J — chat.newTerminal / terminal.workspace.newFullWidth
            (false, false, true, "t") | (false, false, true, "j") => {
                self.toggle_terminal(window, cx)
            }
            // ⌘⇧U — settings.usage
            (false, false, true, "u") => {
                self.clear_views();
                self.view_settings = true;
                self.settings_section = "usage";
                if self.settings_data.is_null() {
                    self.fetch_settings(cx);
                }
                cx.notify();
            }
            // ⌘⇧C — thread.copyId
            (false, false, true, "c") => {
                if let Some(t) = self.selected_thread.clone() {
                    cx.write_to_clipboard(ClipboardItem::new_string(t));
                }
            }
            // ⌘⌃→ / ⌘⌃Tab — threadTab.next / view.recent.next
            (true, false, false, "right") | (true, false, false, "tab") => self.cycle_tab(1, cx),
            // ⌘⌃← / ⌘⌃⇧Tab — threadTab.previous / view.recent.previous
            (true, false, false, "left") | (true, false, true, "tab") => self.cycle_tab(-1, cx),
            // ⌘⌃P — git.commitAndPush opens the changes pane
            (true, false, false, "p") => self.toggle_changes(cx),
            // ⌘⌥U — sidebar.activity → inbox
            (false, true, false, "u") => {
                self.clear_views();
                self.view_inbox = true;
                self.fetch_inbox(cx);
                cx.notify();
            }
            // ⌘⌥A — chat.attention.next → cycle open threads
            (false, true, false, "a") => self.cycle_tab(1, cx),
            // ⌘⌥⇧A — thread.archive
            (false, true, true, "a") => self.archive_selected(cx),
            _ => {}
        }
    }

    /// Upstream ShortcutsDialog (isKeyboardShortcutsHelpShortcut, mod+/).
    pub(super) fn open_sc_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Search shortcuts..."));
        cx.subscribe_in(&input, window, |_this, _s, event, _w, cx| {
            if let InputEvent::Change = event {
                cx.notify();
            }
        })
        .detach();
        self.sc_dialog_input = Some(input.clone());
        self.sc_dialog_open = true;
        let focus = input.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        cx.notify();
    }

    /// Effective kbd label for a SHORTCUT_ROWS entry: the resolved binding
    /// when rules loaded, else the documented default.
    pub(super) fn sc_row_label(&self, commands: &str, fallback: &str) -> String {
        if !self.kb_rules.is_empty() {
            for r in &self.kb_rules {
                let rc = r.get("command").and_then(Value::as_str).unwrap_or("");
                if commands.split(' ').any(|c| c == rc) {
                    let lbl = sc_fmt_label(&r.get("shortcut").cloned().unwrap_or(Value::Null));
                    if !lbl.is_empty() {
                        return lbl;
                    }
                }
            }
            return String::new();
        }
        fallback.to_string()
    }

    /// Upstream rail "…" checkbox: toggles `key` inside
    /// settings.railShortcuts and persists via server.updateSettings.
    pub(super) fn toggle_rail_shortcut(&mut self, key: &str, cx: &mut Context<Self>) {
        let mut next: Vec<String> = arr(get(&self.settings_data, "settings"), "railShortcuts")
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        if let Some(pos) = next.iter().position(|k| k == key) {
            next.remove(pos);
        } else {
            next.push(key.to_string());
        }
        if let Some(obj) =
            get_mut(&mut self.settings_data, "settings").and_then(|s| s.as_object_mut())
        {
            obj.insert(
                "railShortcuts".to_string(),
                json!(next.iter().map(|s| s.as_str()).collect::<Vec<_>>()),
            );
        }
        cx.notify();
        let cell = self.client.clone();
        let payload = json!({"railShortcuts": next});
        cx.spawn(async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
            cx.background_executor()
                .spawn(async move {
                    let _ =
                        ensure_client(&cell).and_then(|c| c.call("server.updateSettings", payload));
                })
                .await;
        })
        .detach();
    }
}
