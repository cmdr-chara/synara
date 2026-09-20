//! Multiple native file buffers. The active Shell fields mirror exactly one tab.
//! Each retained input owns its undo history, selection, scrolling and IME state.
use super::*;
use crate::ui::{self, Glyph, palette};

pub(super) const MAX_TABS: usize = 24;

pub(super) struct EditorTab {
    id: u64,
    document: Document,
    input: Entity<TextEntry>,
    _subscriptions: Vec<Subscription>,
}
impl EditorTab {
    fn dirty(&self, cx: &App) -> bool {
        self.input.read(cx).text() != self.document.snapshot.text
    }
}

pub(super) struct EditorState {
    pub tabs: Vec<EditorTab>,
    active: Option<u64>,
    next_id: u64,
    generation: u64,
    pub loading: Option<PathBuf>,
    close: Option<(u64, bool)>,
    pub tree_visible: bool,
    pub show_hidden: bool,
    pub preview: bool,
    pub find_open: bool,
    replace_open: bool,
    goto_open: bool,
    query: Entity<TextEntry>,
    replacement: Entity<TextEntry>,
    line: Entity<TextEntry>,
    focus_editor: bool,
    pub jump_after_open: Option<(PathBuf, usize)>,
    _subscriptions: Vec<Subscription>,
}
impl EditorState {
    pub fn new(cx: &mut Context<Shell>) -> Self {
        let query = cx.new(|cx| TextEntry::new("Find exact text", EntryMode::SingleLine, 32., cx));
        let replacement = cx.new(|cx| TextEntry::new("Replace with", EntryMode::SingleLine, 32., cx));
        let line = cx.new(|cx| TextEntry::new("Line:column", EntryMode::SingleLine, 32., cx));
        let subscriptions = vec![
            cx.subscribe(&query, |this, _, event, cx| {
                if matches!(event, EntryEvent::Submit) { this.find_in_editor(false, cx); }
                cx.notify();
            }),
            cx.subscribe(&replacement, |this, _, event, cx| {
                if matches!(event, EntryEvent::Submit) { this.replace_in_editor(false, cx); }
                cx.notify();
            }),
            cx.subscribe(&line, |this, _, event, cx| {
                if matches!(event, EntryEvent::Submit) { this.goto_editor_line(cx); }
            }),
        ];
        Self {
            tabs: Vec::new(), active: None, next_id: 0, generation: 0,
            loading: None, close: None, tree_visible: true, show_hidden: true,
            preview: false, find_open: false, replace_open: false, goto_open: false,
            query, replacement, line, focus_editor: false, jump_after_open: None, _subscriptions: subscriptions,
        }
    }
    pub fn dirty(&self, cx: &App) -> bool { self.tabs.iter().any(|tab| tab.dirty(cx)) }
    pub fn index(&self, path: &std::path::Path) -> Option<usize> {
        self.tabs.iter().position(|tab| tab.document.path == path)
    }
    pub fn request_open(&mut self, path: PathBuf) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.loading = Some(path);
        self.generation
    }
    pub fn finish_open(&mut self, generation: u64) -> bool {
        if self.generation != generation { return false; }
        self.loading = None;
        true
    }
}

impl Shell {
    pub(super) fn restore_editor_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.editors.focus_editor && self.panel == Panel::Files && !self.studio.open
            && !self.command_palette.open && !self.explorer.modal_open()
            && !self.controls.is_open() && !self.chat_tools.menu_open()
            && self.kanban.dialog.is_none() && self.organization.dialog.is_none()
            && self.saved_context.dialog.is_none() && self.settings.popup.is_none()
        {
            self.editors.focus_editor = false;
            window.focus(&self.editor.read(cx).focus_handle(cx), cx);
        }
    }
    pub(super) fn open_editor_paths(&self) -> Vec<PathBuf> {
        self.editors.tabs.iter().map(|tab| tab.document.path.clone()).collect()
    }
    pub(super) fn editor_preview_available(&self) -> bool {
        self.document.as_ref().and_then(|document| document.path.extension()).and_then(|value| value.to_str())
            .is_some_and(|extension| matches!(extension.to_ascii_lowercase().as_str(), "md" | "markdown" | "mdown"))
    }
    pub(super) fn active_document_dirty(&self, cx: &App) -> bool {
        self.document.as_ref().is_some_and(|document| self.editor.read(cx).text() != document.snapshot.text)
    }
    pub(super) fn reset_editor_tabs(&mut self) {
        self.editors.tabs.clear();
        self.editors.active = None;
        self.editors.close = None;
        self.editors.generation = self.editors.generation.wrapping_add(1);
        self.editors.loading = None;
        self.editors.jump_after_open = None;
        self.editors.preview = false;
        self.editors.focus_editor = false;
    }
    pub(super) fn sync_editor_document(&mut self) {
        if let Some(document) = &self.document
            && let Some(tab) = self.editors.tabs.iter_mut().find(|tab| Some(tab.id) == self.editors.active)
        { tab.document = document.clone(); }
    }
    pub(super) fn install_editor_document(&mut self, document: Document, cx: &mut Context<Self>) {
        if let Some(index) = self.editors.index(&document.path) {
            self.activate_editor(index, cx);
            return;
        }
        if self.editors.tabs.len() >= MAX_TABS {
            self.notice = Some("The file is available in Explorer. Close a tab to open another buffer.".into());
            return;
        }
        let input = cx.new(|cx| {
            let mut input = TextEntry::new("", EntryMode::Editor, 480., cx);
            input.set_text(document.snapshot.text.clone(), cx);
            input
        });
        let subscriptions = vec![
            cx.subscribe(&input, |this, input, event, cx| {
                if input.entity_id() == this.editor.entity_id() && matches!(event, EntryEvent::Save) {
                    this.save_file(cx);
                }
                cx.notify();
            }),
            cx.observe(&input, |_, _, cx| cx.notify()),
        ];
        self.editors.next_id = self.editors.next_id.wrapping_add(1);
        self.editors.tabs.push(EditorTab { id: self.editors.next_id, document, input, _subscriptions: subscriptions });
        self.activate_editor(self.editors.tabs.len() - 1, cx);
    }
    pub(super) fn activate_editor(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.saving || self.explorer.modal_open() || self.editor.read(cx).is_composing() { return; }
        self.sync_editor_document();
        let Some(tab) = self.editors.tabs.get(index) else { return };
        self.editors.active = Some(tab.id);
        self.editor = tab.input.clone();
        self.document = Some(tab.document.clone());
        self.editors.generation = self.editors.generation.wrapping_add(1);
        self.editors.loading = None;
        self.editors.close = None;
        self.focus_composer = false;
        self.editors.focus_editor = true;
        if let Some((path, line)) = self.editors.jump_after_open.take()
            && self.document.as_ref().is_some_and(|document| document.path == path)
        {
            self.editor.update(cx, |input, cx| { input.go_to_line(line, 1, cx); });
        }
        cx.notify();
    }
    pub(super) fn replace_editor_document(&mut self, document: Document, cx: &mut Context<Self>) {
        if let Some(tab) = self.editors.tabs.iter_mut().find(|tab| Some(tab.id) == self.editors.active) {
            tab.input.update(cx, |input, cx| input.set_text(document.snapshot.text.clone(), cx));
            tab.document = document.clone();
            self.document = Some(document);
        } else { self.install_editor_document(document, cx); }
    }
    pub(super) fn remove_active_editor(&mut self, cx: &mut Context<Self>) {
        let Some(index) = self.editors.tabs.iter().position(|tab| Some(tab.id) == self.editors.active) else {
            self.document = None;
            return;
        };
        self.editors.tabs.remove(index);
        self.editors.active = None;
        self.document = None;
        self.editors.close = None;
        if !self.editors.tabs.is_empty() { self.activate_editor(index.min(self.editors.tabs.len() - 1), cx); }
        cx.notify();
    }
    pub(super) fn discard_active_document(&mut self, cx: &mut Context<Self>) {
        if self.saving { return; }
        if let Some(document) = &self.document {
            self.editor.update(cx, |input, cx| input.set_text(document.snapshot.text.clone(), cx));
        }
        cx.notify();
    }
    pub(super) fn reveal_dirty_editor(&mut self, cx: &mut Context<Self>) {
        if self.saving || self.active_document_dirty(cx) { return; }
        if let Some(index) = self.editors.tabs.iter().position(|tab| tab.dirty(cx)) {
            self.activate_editor(index, cx);
        }
    }
    fn request_editor_close(&mut self, id: u64, cx: &mut Context<Self>) {
        if self.saving || self.explorer.modal_open() || self.editor.read(cx).is_composing() { return; }
        let Some(index) = self.editors.tabs.iter().position(|tab| tab.id == id) else { return };
        self.activate_editor(index, cx);
        if self.active_document_dirty(cx) { self.editors.close = Some((id, false)); }
        else { self.remove_active_editor(cx); }
        cx.notify();
    }
    fn save_and_close_editor(&mut self, cx: &mut Context<Self>) {
        if let Some((id, _)) = self.editors.close && self.editors.active == Some(id) {
            self.editors.close = Some((id, true));
            self.save_file(cx);
        }
    }
    pub(super) fn finish_editor_close(&mut self, cx: &mut Context<Self>) {
        if let Some((id, true)) = self.editors.close
            && self.editors.active == Some(id) && !self.active_document_dirty(cx)
        { self.remove_active_editor(cx); }
    }
    pub(super) fn cycle_editor(&mut self, backwards: bool, cx: &mut Context<Self>) {
        if self.editors.tabs.is_empty() { return; }
        let index = self.editors.tabs.iter().position(|tab| Some(tab.id) == self.editors.active).unwrap_or(0);
        let count = self.editors.tabs.len();
        self.activate_editor(if backwards { (index + count - 1) % count } else { (index + 1) % count }, cx);
    }
    pub(super) fn open_editor_find(&mut self, replace: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.document.is_none() || self.editor.read(cx).is_composing() { return; }
        self.editors.find_open = true;
        self.editors.replace_open = replace;
        self.editors.preview = false;
        let selected = self.editor.read(cx).selected_text().to_owned();
        if !selected.is_empty() && !selected.contains(['\r', '\n']) && selected.len() <= 4096 {
            self.editors.query.update(cx, |input, cx| input.set_text(selected, cx));
        }
        window.focus(&self.editors.query.read(cx).focus_handle(cx), cx);
        cx.notify();
    }
    fn find_in_editor(&mut self, backwards: bool, cx: &mut Context<Self>) {
        let query = self.editors.query.read(cx).text().to_owned();
        let found = self.editor.update(cx, |input, cx| input.find_literal(&query, backwards, cx));
        if !found && !query.is_empty() { self.notice = Some("No exact match in the current file.".into()); }
        cx.notify();
    }
    fn replace_in_editor(&mut self, all: bool, cx: &mut Context<Self>) {
        if self.document.is_none() || self.saving { return; }
        let query = self.editors.query.read(cx).text().to_owned();
        let replacement = self.editors.replacement.read(cx).text().to_owned();
        let count = self.editor.update(cx, |input, cx| input.replace_literal(&query, &replacement, all, cx));
        self.notice = Some(format!("Replaced {count} matches in the editor. Save to write the file."));
        cx.notify();
    }
    pub(super) fn open_editor_goto(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.document.is_none() || self.editor.read(cx).is_composing() { return; }
        self.editors.goto_open = true;
        self.editors.preview = false;
        window.focus(&self.editors.line.read(cx).focus_handle(cx), cx);
        cx.notify();
    }
    fn goto_editor_line(&mut self, cx: &mut Context<Self>) {
        let value = self.editors.line.read(cx).text().trim().to_owned();
        let (line, column) = value.split_once(':').unwrap_or((&value, "1"));
        let valid = match (line.parse::<usize>(), column.parse::<usize>()) {
            (Ok(line), Ok(column)) => self.editor.update(cx, |input, cx| input.go_to_line(line, column, cx)),
            _ => false,
        };
        if valid { self.editors.goto_open = false; self.editors.focus_editor = true; }
        else { self.notice = Some("Enter an existing line number, optionally followed by :column.".into()); }
        cx.notify();
    }
    pub(super) fn editor_shortcut(&mut self, event: &gpui::KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.panel != Panel::Files || self.studio.open || event.prefer_character_input || event.is_held
            || self.editor.read(cx).is_composing() || self.editors.query.read(cx).is_composing()
            || self.editors.replacement.read(cx).is_composing() || self.editors.line.read(cx).is_composing()
            || self.controls.is_open() || self.chat_tools.menu_open() || self.navigation.menu_open
            || self.environment.menu_open() || self.settings.popup.is_some()
            || self.terminal_view.read(cx).focus_handle(cx).is_focused(window)
        { return false; }
        let modifiers = event.keystroke.modifiers;
        let command = modifiers.control || modifiers.platform;
        match event.keystroke.key.as_str() {
            "f" if command && !modifiers.alt && !modifiers.shift => self.open_editor_find(false, window, cx),
            "h" if command && !modifiers.alt && !modifiers.shift => self.open_editor_find(true, window, cx),
            "g" if command && !modifiers.alt && !modifiers.shift => self.open_editor_goto(window, cx),
            "tab" if command && !modifiers.alt => { self.cycle_editor(modifiers.shift, cx); window.focus(&self.editor.read(cx).focus_handle(cx), cx); }
            "w" if command && !modifiers.alt && !modifiers.shift => {
                if let Some(id) = self.editors.active { self.request_editor_close(id, cx); }
            }
            "f3" if !command && !modifiers.alt && self.editors.find_open => self.find_in_editor(modifiers.shift, cx),
            "escape" if !command && !modifiers.alt && !modifiers.shift
                && (self.editors.find_open || self.editors.goto_open || self.editors.close.is_some()) => {
                self.editors.find_open = false;
                self.editors.goto_open = false;
                self.editors.close = None;
                window.focus(&self.editor.read(cx).focus_handle(cx), cx);
            }
            _ => return false,
        }
        cx.notify();
        true
    }
    pub(super) fn editor_tabs(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        div().id("editor-tabs").flex().items_center().min_w_0().flex_shrink_0().overflow_x_scroll()
            .border_b_1().border_color(rgb(palette().border))
            .children(self.editors.tabs.iter().enumerate().map(|(index, tab)| {
                let id = tab.id;
                let active = self.editors.active == Some(id);
                let dirty = tab.dirty(cx);
                let name = tab.document.path.file_name().map_or_else(String::new, |name| name.to_string_lossy().into_owned());
                div().id(("editor-tab", index)).flex().items_center().flex_shrink_0().max_w(px(230.))
                    .when(active, |el| el.bg(rgb(palette().overlay)).border_b_2().border_color(rgb(palette().focus)))
                    .child(ui::action(("editor-select", index), format!("{name}{}", if dirty { " *" } else { "" }), Some(Glyph::Files), active,
                        cx.listener(move |this, _: &(), window, cx| { if let Some(index) = this.editors.tabs.iter().position(|tab| tab.id == id) { this.activate_editor(index, cx); } window.focus(&this.editor.read(cx).focus_handle(cx), cx); }))
                        .h(px(33.)).min_w_0().text_size(px(12.)).aria_label(tab.document.path.display().to_string()))
                    .child(ui::button_shell(("editor-close", index), "Close file", false)
                        .size(px(22.)).p_0().flex().items_center().justify_center().child(ui::icon(Glyph::Close))
                        .on_click(cx.listener(move |this, _, _, cx| this.request_editor_close(id, cx))))
            })).into_any_element()
    }
    pub(super) fn editor_tools(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let markdown = self.document.as_ref().and_then(|document| document.path.extension()).and_then(|value| value.to_str())
            .is_some_and(|extension| matches!(extension.to_ascii_lowercase().as_str(), "md" | "markdown" | "mdown"));
        div().flex().flex_col().gap_2().flex_shrink_0()
            .child(div().flex().items_center().flex_wrap().gap_1()
                .child(ui::chrome_button("editor-find", "Find in file (Ctrl/Cmd+F)", Glyph::Search, false,
                    cx.listener(|this, _: &(), window, cx| this.open_editor_find(false, window, cx))))
                .child(ui::action("editor-replace", "Replace", None, self.editors.replace_open && self.editors.find_open,
                    cx.listener(|this, _: &(), window, cx| this.open_editor_find(true, window, cx))).text_size(px(11.)))
                .child(ui::action("editor-goto", "Go to line", None, self.editors.goto_open,
                    cx.listener(|this, _: &(), window, cx| this.open_editor_goto(window, cx))).text_size(px(11.)))
                .children(markdown.then(|| ui::action("editor-preview", if self.editors.preview { "Edit Markdown" } else { "Preview Markdown" }, Some(Glyph::Notebook), self.editors.preview,
                    cx.listener(|this, _: &(), _, cx| { this.editors.preview = !this.editors.preview; cx.notify(); })).text_size(px(11.))))
                .child(self.editor_actions(cx)))
            .children(self.editors.find_open.then(|| {
                let query = self.editors.query.read(cx).text();
                let count = if query.is_empty() { 0 } else { self.editor.read(cx).text().match_indices(query).count() };
                div().flex().flex_col().gap_1()
                    .child(div().flex().items_center().gap_1()
                        .child(div().flex_1().min_w_0().child(self.editors.query.clone()))
                        .child(div().text_size(px(11.)).text_color(rgb(palette().muted)).child(format!("{count}")))
                        .child(ui::chrome_button("editor-find-prev", "Previous exact match", Glyph::Back, false, cx.listener(|this, _: &(), _, cx| this.find_in_editor(true, cx))))
                        .child(ui::chrome_button("editor-find-next", "Next exact match", Glyph::Forward, false, cx.listener(|this, _: &(), _, cx| this.find_in_editor(false, cx))))
                        .child(ui::chrome_button("editor-find-close", "Close find", Glyph::Close, false, cx.listener(|this, _: &(), _, cx| { this.editors.find_open = false; cx.notify(); }))))
                    .children(self.editors.replace_open.then(|| div().flex().items_center().flex_wrap().gap_1()
                        .child(div().flex_1().min_w(px(120.)).child(self.editors.replacement.clone()))
                        .child(ui::action("editor-replace-one", "Replace", None, false, cx.listener(|this, _: &(), _, cx| this.replace_in_editor(false, cx))).text_size(px(11.)))
                        .child(ui::action("editor-replace-all", "Replace all", None, false, cx.listener(|this, _: &(), _, cx| this.replace_in_editor(true, cx))).text_size(px(11.)))))
                    .child(div().text_size(px(10.)).text_color(rgb(palette().muted)).child("Case-sensitive literal text. Replacements remain unsaved and support Undo."))
            }))
            .children(self.editors.goto_open.then(|| div().flex().items_center().gap_1()
                .child(div().flex_1().min_w_0().child(self.editors.line.clone()))
                .child(ui::action("editor-goto-confirm", "Go", None, false, cx.listener(|this, _: &(), window, cx| {
                    this.goto_editor_line(cx);
                    if !this.editors.goto_open { window.focus(&this.editor.read(cx).focus_handle(cx), cx); }
                })))))
            .children(self.editors.close.map(|(_, saving)| div().p_2().rounded_md().bg(rgb(palette().notice_surface)).flex().flex_col().gap_1()
                .child(div().text_size(px(12.)).child("This file has unsaved changes."))
                .child(div().flex().flex_wrap().gap_1()
                    .child(ui::action("editor-save-close", if saving { "Saving..." } else { "Save and close" }, None, false,
                        cx.listener(|this, _: &(), _, cx| this.save_and_close_editor(cx))))
                    .child(ui::action("editor-discard-close", "Discard and close", None, false, cx.listener(|this, _: &(), _, cx| {
                        if !this.saving { this.remove_active_editor(cx); }
                    })))
                    .child(ui::action("editor-keep-open", "Keep open", None, false, cx.listener(|this, _: &(), _, cx| { this.editors.close = None; cx.notify(); }))))))
            .into_any_element()
    }
    pub(super) fn editor_content(&self) -> gpui::AnyElement {
        div().flex().flex_col().flex_1().min_h_0().child(self.editor.clone()).into_any_element()
    }
    pub(super) fn editor_preview(&self, cx: &App) -> gpui::AnyElement {
        div().id("editor-markdown-preview").flex_1().min_h_0().min_w_0().overflow_y_scroll().p_3()
            .child(ui::markdown::render(self.editor.read(cx).text(), "editor-preview"))
            .into_any_element()
    }
    pub(super) fn editor_status(&self, cx: &App) -> gpui::AnyElement {
        let entry = self.editor.read(cx);
        let (line, column) = entry.cursor_position();
        let selection = entry.selected_text().chars().count();
        let newlines = if entry.text().contains("\r\n") { "CRLF" } else { "LF" };
        div().flex().items_center().flex_wrap().gap_3().text_size(px(11.)).text_color(rgb(palette().muted)).flex_shrink_0()
            .child(format!("Ln {line}, Col {column}"))
            .child(format!("{} bytes", entry.text().len()))
            .child(format!("UTF-8 · {newlines}"))
            .children((selection > 0).then(|| div().child(format!("{selection} selected"))))
            .into_any_element()
    }
}
