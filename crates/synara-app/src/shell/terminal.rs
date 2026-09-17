use gpui::{
    App, Bounds, ClipboardItem, ContentMask, Context, ElementInputHandler, EntityInputHandler,
    FocusHandle, Focusable, FontStyle, FontWeight, KeyDownEvent, MouseButton, Pixels, Point,
    SharedString, TextAlign, UTF16Selection, UnderlineStyle, Window, canvas, div, prelude::*, px,
    rgb,
};
use std::{ops::Range, sync::Arc};
use synara_runtime::{
    NativeTerminal, PasteDecision, PreparedPaste, RuntimeError, TerminalColor, TerminalGrid,
    TerminalKey, TerminalModifiers, TerminalRenderSnapshot,
};

const CELL_WIDTH: f32 = 8.45;
const LINE_HEIGHT: f32 = 18.0;
const DEFAULT_FOREGROUND: u32 = 0xd7dae0;
const DEFAULT_BACKGROUND: u32 = 0x0b1017;
const SELECTION_BACKGROUND: u32 = 0x315580;
const CURSOR_BACKGROUND: u32 = 0xd7dae0;
const CURSOR_UNFOCUSED: u32 = 0x596779;

#[derive(Clone)]
pub(super) enum TerminalSession {
    Local(Arc<NativeTerminal>),
    #[cfg(unix)]
    Remote(Arc<synara_runtime::RemoteTerminal>),
}

impl TerminalSession {
    pub(super) fn key(
        &self,
        key: TerminalKey,
        modifiers: TerminalModifiers,
    ) -> Result<(), RuntimeError> {
        match self {
            Self::Local(terminal) => terminal.key(key, modifiers),
            #[cfg(unix)]
            Self::Remote(terminal) => terminal.key(key, modifiers),
        }
    }

    pub(super) fn text(&self, text: &str) -> Result<(), RuntimeError> {
        match self {
            Self::Local(terminal) => terminal.text(text),
            #[cfg(unix)]
            Self::Remote(terminal) => terminal.text(text),
        }
    }

    pub(super) fn paste(
        &self,
        paste: PreparedPaste,
        decision: PasteDecision,
    ) -> Result<(), RuntimeError> {
        match self {
            Self::Local(terminal) => terminal.paste(paste, decision),
            #[cfg(unix)]
            Self::Remote(terminal) => terminal.paste(paste, decision),
        }
    }

    pub(super) fn resize(&self, rows: u16, columns: u16) -> Result<(), RuntimeError> {
        match self {
            Self::Local(terminal) => terminal.resize(rows, columns),
            #[cfg(unix)]
            Self::Remote(terminal) => terminal.resize(rows, columns),
        }
    }

    pub(super) fn scrollback(&self, offset: usize) -> Result<(), RuntimeError> {
        match self {
            Self::Local(terminal) => terminal.scrollback(offset),
            #[cfg(unix)]
            Self::Remote(terminal) => terminal.scrollback(offset),
        }
    }

    pub(super) fn render_snapshot(&self) -> Result<TerminalRenderSnapshot, RuntimeError> {
        match self {
            Self::Local(terminal) => terminal.render_snapshot(),
            #[cfg(unix)]
            Self::Remote(terminal) => terminal.render_snapshot(),
        }
    }

    pub(super) fn kill(&self) -> Result<(), RuntimeError> {
        match self {
            Self::Local(terminal) => terminal.kill(),
            #[cfg(unix)]
            Self::Remote(terminal) => terminal.kill(),
        }
    }

    pub(super) async fn wait(&self) -> Result<u32, RuntimeError> {
        match self {
            Self::Local(terminal) => terminal.wait().await,
            #[cfg(unix)]
            Self::Remote(terminal) => terminal.wait().await,
        }
    }
}

pub(super) struct TerminalView {
    focus: FocusHandle,
    session: Option<TerminalSession>,
    snapshot: Option<TerminalRenderSnapshot>,
    bounds: Bounds<Pixels>,
    selection: Option<((u16, u16), (u16, u16))>,
    dragging: bool,
    pending_paste: Option<PreparedPaste>,
    preedit: String,
    last_size: Option<(u16, u16)>,
    scrollback: usize,
    error: Option<String>,
}

impl Focusable for TerminalView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl TerminalView {
    pub(super) fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle(),
            session: None,
            snapshot: None,
            bounds: Bounds::default(),
            selection: None,
            dragging: false,
            pending_paste: None,
            preedit: String::new(),
            last_size: None,
            scrollback: 0,
            error: None,
        }
    }

    pub(super) fn set_session(&mut self, session: TerminalSession, cx: &mut Context<Self>) {
        self.session = Some(session);
        self.snapshot = None;
        self.selection = None;
        self.pending_paste = None;
        self.preedit.clear();
        self.last_size = None;
        self.scrollback = 0;
        self.error = None;
        cx.notify();
    }

    pub(super) fn set_snapshot(
        &mut self,
        snapshot: TerminalRenderSnapshot,
        cx: &mut Context<Self>,
    ) {
        let changed = self.snapshot.as_ref().is_none_or(|old| {
            old.grid.revision != snapshot.grid.revision
                || old.exit_code != snapshot.exit_code
                || old.error != snapshot.error
        });
        if changed {
            self.scrollback = snapshot.grid.scrollback;
            self.snapshot = Some(snapshot);
            cx.notify();
        }
    }

    pub(super) fn clear_session(&mut self, cx: &mut Context<Self>) {
        self.session = None;
        self.pending_paste = None;
        self.preedit.clear();
        cx.notify();
    }

    pub(super) fn exit_code(&self) -> Option<u32> {
        self.snapshot.as_ref().and_then(|snapshot| snapshot.exit_code)
    }

    pub(super) fn terminal_error(&self) -> Option<&str> {
        self.snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.error.as_deref())
            .or(self.error.as_deref())
    }

    pub(super) fn title(&self) -> Option<&str> {
        self.snapshot
            .as_ref()
            .map(|snapshot| snapshot.grid.title.as_str())
            .filter(|title| !title.is_empty())
    }

    pub(super) fn cwd_hint(&self) -> Option<&str> {
        self.snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.grid.cwd_hint.as_deref())
    }

    fn prepare(&mut self, bounds: Bounds<Pixels>) {
        self.bounds = bounds;
        let columns = ((bounds.size.width / px(CELL_WIDTH)).floor() as u16).clamp(1, 500);
        let rows = ((bounds.size.height / px(LINE_HEIGHT)).floor() as u16).clamp(1, 200);
        if u32::from(rows) * u32::from(columns) > 40_000 {
            return;
        }
        let size = (rows, columns);
        if self.last_size != Some(size) {
            self.last_size = Some(size);
            if let Some(session) = &self.session
                && let Err(error) = session.resize(rows, columns)
            {
                self.error = Some(error.to_string());
            }
        }
    }

    fn cell_at(&self, point: Point<Pixels>) -> Option<(u16, u16)> {
        let grid = &self.snapshot.as_ref()?.grid;
        if point.x < self.bounds.left()
            || point.x >= self.bounds.right()
            || point.y < self.bounds.top()
            || point.y >= self.bounds.bottom()
        {
            return None;
        }
        let column = ((point.x - self.bounds.left()) / px(CELL_WIDTH)).floor() as u16;
        let row = ((point.y - self.bounds.top()) / px(LINE_HEIGHT)).floor() as u16;
        Some((
            row.min(grid.rows.saturating_sub(1)),
            column.min(grid.columns.saturating_sub(1)),
        ))
    }

    fn selected_text(&self) -> Option<String> {
        let (start, end) = self.selection?;
        let grid = &self.snapshot.as_ref()?.grid;
        let text = grid.selected_text(start, end);
        (!text.is_empty()).then_some(text)
    }

    fn copy_selection(&self, cx: &mut Context<Self>) -> bool {
        let Some(text) = self.selected_text() else {
            return false;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        true
    }

    fn request_paste(&mut self, cx: &mut Context<Self>) {
        let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
            return;
        };
        let paste = match PreparedPaste::new(&text) {
            Ok(paste) => paste,
            Err(error) => {
                self.error = Some(error.to_string());
                cx.notify();
                return;
            }
        };
        if paste.requires_review() {
            self.pending_paste = Some(paste);
            cx.notify();
            return;
        }
        self.send_paste(paste, PasteDecision::Unreviewed, cx);
    }

    fn send_paste(
        &mut self,
        paste: PreparedPaste,
        decision: PasteDecision,
        cx: &mut Context<Self>,
    ) {
        let Some(session) = &self.session else {
            return;
        };
        match session.paste(paste, decision) {
            Ok(()) => {
                self.error = None;
                self.selection = None;
            }
            Err(error) => self.error = Some(error.to_string()),
        }
        cx.notify();
    }

    fn approve_paste(&mut self, cx: &mut Context<Self>) {
        if let Some(paste) = self.pending_paste.take() {
            self.send_paste(paste, PasteDecision::Approved, cx);
        }
    }

    fn cancel_paste(&mut self, cx: &mut Context<Self>) {
        self.pending_paste = None;
        cx.notify();
    }

    fn send_key(
        &mut self,
        key: TerminalKey,
        modifiers: TerminalModifiers,
        cx: &mut Context<Self>,
    ) {
        let Some(session) = &self.session else {
            return;
        };
        match session.key(key, modifiers) {
            Ok(()) => {
                self.error = None;
                self.selection = None;
            }
            Err(error) => self.error = Some(error.to_string()),
        }
        cx.notify();
    }

    fn key(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let modifiers = event.keystroke.modifiers;
        let copy = (modifiers.platform && !modifiers.control && !modifiers.alt)
            || (modifiers.control && modifiers.shift && !modifiers.alt && !modifiers.platform);
        if copy && event.keystroke.key.eq_ignore_ascii_case("c") && self.copy_selection(cx) {
            cx.stop_propagation();
            return;
        }
        if copy && event.keystroke.key.eq_ignore_ascii_case("v") {
            self.request_paste(cx);
            cx.stop_propagation();
            return;
        }
        if modifiers.platform {
            return;
        }
        if event.prefer_character_input && !modifiers.control {
            return;
        }

        let terminal_modifiers = TerminalModifiers {
            shift: modifiers.shift,
            control: modifiers.control,
            alt: modifiers.alt,
        };
        let key = match event.keystroke.key.as_str() {
            "enter" => Some(TerminalKey::Enter),
            "backspace" => Some(TerminalKey::Backspace),
            "tab" => Some(TerminalKey::Tab),
            "escape" => Some(TerminalKey::Escape),
            "up" => Some(TerminalKey::Up),
            "down" => Some(TerminalKey::Down),
            "left" => Some(TerminalKey::Left),
            "right" => Some(TerminalKey::Right),
            "home" => Some(TerminalKey::Home),
            "end" => Some(TerminalKey::End),
            "insert" => Some(TerminalKey::Insert),
            "delete" => Some(TerminalKey::Delete),
            "pageup" => Some(TerminalKey::PageUp),
            "pagedown" => Some(TerminalKey::PageDown),
            value if value
                .strip_prefix('f')
                .and_then(|number| number.parse::<u8>().ok())
                .is_some_and(|number| (1..=12).contains(&number)) =>
            {
                Some(TerminalKey::Function(
                    value[1..].parse::<u8>().expect("validated function key"),
                ))
            }
            _ if modifiers.control || modifiers.alt => event
                .keystroke
                .key_char
                .as_deref()
                .or(Some(event.keystroke.key.as_str()))
                .and_then(single_character)
                .map(TerminalKey::Character),
            _ => None,
        };
        if let Some(key) = key {
            self.send_key(key, terminal_modifiers, cx);
            cx.stop_propagation();
        }
    }

    fn scroll(&mut self, event: &gpui::ScrollWheelEvent, cx: &mut Context<Self>) {
        let delta = event.delta.pixel_delta(px(LINE_HEIGHT)).y;
        let next = if delta > px(0.) {
            self.scrollback.saturating_add(3)
        } else if delta < px(0.) {
            self.scrollback.saturating_sub(3)
        } else {
            return;
        };
        if let Some(session) = &self.session {
            match session.scrollback(next) {
                Ok(()) => {
                    self.scrollback = next;
                    self.error = None;
                }
                Err(error) => self.error = Some(error.to_string()),
            }
        }
        cx.notify();
        cx.stop_propagation();
    }

    fn cursor_bounds(&self) -> Bounds<Pixels> {
        let cursor = self
            .snapshot
            .as_ref()
            .map(|snapshot| snapshot.grid.cursor)
            .unwrap_or_default();
        Bounds::new(
            gpui::point(
                self.bounds.left() + px(f32::from(cursor.1) * CELL_WIDTH),
                self.bounds.top() + px(f32::from(cursor.0) * LINE_HEIGHT),
            ),
            gpui::size(px(CELL_WIDTH), px(LINE_HEIGHT)),
        )
    }
}

impl gpui::Render for TerminalView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        let paint_entity = entity.clone();
        let pending = self.pending_paste.clone();
        div()
            .id("terminal-direct-input")
            .key_context("SynaraTerminal")
            .track_focus(&self.focus)
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(DEFAULT_BACKGROUND))
            .font_family("DejaVu Sans Mono")
            .cursor_text()
            .on_key_down(cx.listener(Self::key))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &gpui::MouseDownEvent, window, cx| {
                    window.focus(&this.focus, cx);
                    if let Some(cell) = this.cell_at(event.position) {
                        if event.modifiers.shift {
                            let start = this.selection.map_or(cell, |selection| selection.0);
                            this.selection = Some((start, cell));
                        } else {
                            this.selection = Some((cell, cell));
                        }
                        this.dragging = true;
                        cx.notify();
                    }
                    cx.stop_propagation();
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &gpui::MouseMoveEvent, _, cx| {
                if this.dragging
                    && event.dragging()
                    && let Some(cell) = this.cell_at(event.position)
                    && let Some((start, _)) = this.selection
                {
                    this.selection = Some((start, cell));
                    cx.notify();
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.dragging = false),
            )
            .on_scroll_wheel(cx.listener(|this, event, _, cx| this.scroll(event, cx)))
            .child(
                canvas(
                    move |bounds, _, cx| {
                        entity.update(cx, |this, _| this.prepare(bounds));
                    },
                    move |bounds, _, window, cx| {
                        let (focus, snapshot, selection, preedit) = {
                            let this = paint_entity.read(cx);
                            (
                                this.focus.clone(),
                                this.snapshot.clone(),
                                this.selection,
                                this.preedit.clone(),
                            )
                        };
                        window.handle_input(
                            &focus,
                            ElementInputHandler::new(bounds, paint_entity.clone()),
                            cx,
                        );
                        window.with_content_mask(Some(ContentMask { bounds }), |window| {
                            let Some(snapshot) = snapshot else {
                                return;
                            };
                            paint_grid(
                                &snapshot.grid,
                                selection,
                                focus.is_focused(window),
                                bounds,
                                window,
                                cx,
                            );
                            if !preedit.is_empty() && !snapshot.grid.cursor_hidden {
                                paint_preedit(
                                    &preedit,
                                    snapshot.grid.cursor,
                                    bounds,
                                    window,
                                    cx,
                                );
                            }
                        });
                    },
                )
                .flex_1()
                .min_h_0(),
            )
            .children(pending.map(|paste| {
                let preview = preview(paste.text(), 1_500);
                div()
                    .p_3()
                    .border_t_1()
                    .border_color(rgb(0x53657a))
                    .bg(rgb(0x182433))
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .font_family("DejaVu Sans")
                            .text_color(rgb(0xffd7a8))
                            .child(format!(
                                "Review clipboard paste. It contains line breaks, tabs, or removed controls ({} removed). Without bracketed-paste support, approved line breaks may execute commands.",
                                paste.removed_controls()
                            )),
                    )
                    .child(
                        div()
                            .max_h(px(120.))
                            .overflow_y_scroll()
                            .whitespace_pre()
                            .child(preview),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                div()
                                    .id("terminal-paste-approve")
                                    .px_3()
                                    .py_1()
                                    .rounded_md()
                                    .bg(rgb(0x2b4665))
                                    .cursor_pointer()
                                    .child("Paste reviewed text")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.approve_paste(cx)
                                    })),
                            )
                            .child(
                                div()
                                    .id("terminal-paste-cancel")
                                    .px_3()
                                    .py_1()
                                    .rounded_md()
                                    .bg(rgb(0x2b3038))
                                    .cursor_pointer()
                                    .child("Cancel")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.cancel_paste(cx)
                                    })),
                            ),
                    )
            }))
    }
}

impl EntityInputHandler for TerminalView {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        adjusted: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let byte_range = utf16_range_to_bytes(&self.preedit, range.clone())?;
        *adjusted = Some(range);
        Some(self.preedit[byte_range].to_owned())
    }

    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: 0..0,
            reversed: false,
        })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        (!self.preedit.is_empty()).then(|| 0..self.preedit.encode_utf16().count())
    }

    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.preedit.clear();
        cx.notify();
    }

    fn replace_text_in_range(
        &mut self,
        _: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.preedit.clear();
        if text.is_empty() {
            cx.notify();
            return;
        }
        let Some(session) = &self.session else {
            return;
        };
        match session.text(text) {
            Ok(()) => {
                self.error = None;
                self.selection = None;
            }
            Err(error) => self.error = Some(error.to_string()),
        }
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        _: Option<Range<usize>>,
        text: &str,
        _: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.preedit.clear();
        self.preedit.push_str(text);
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        _: Range<usize>,
        _: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        Some(self.cursor_bounds())
    }

    fn character_index_for_point(
        &mut self,
        _: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        Some(0)
    }

    fn set_selected_text_range(
        &mut self,
        _: Range<usize>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) {
    }

    fn text_length_utf16(&mut self, _: &mut Window, _: &mut Context<Self>) -> Option<usize> {
        Some(self.preedit.encode_utf16().count())
    }
}

fn single_character(text: &str) -> Option<char> {
    let mut chars = text.chars();
    let character = chars.next()?;
    chars.next().is_none().then_some(character)
}

fn utf16_range_to_bytes(text: &str, range: Range<usize>) -> Option<Range<usize>> {
    if range.start > range.end {
        return None;
    }
    let byte_at = |target: usize| {
        let mut units = 0;
        for (byte, character) in text.char_indices() {
            if units == target {
                return Some(byte);
            }
            units += character.len_utf16();
            if units > target {
                return None;
            }
        }
        (units == target).then_some(text.len())
    };
    Some(byte_at(range.start)?..byte_at(range.end)?)
}

fn preview(text: &str, limit: usize) -> String {
    let mut result = text.chars().take(limit).collect::<String>();
    if text.chars().count() > limit {
        result.push_str("\n[preview truncated]");
    }
    result
}

fn paint_grid(
    grid: &TerminalGrid,
    selection: Option<((u16, u16), (u16, u16))>,
    focused: bool,
    bounds: Bounds<Pixels>,
    window: &mut Window,
    cx: &mut App,
) {
    for row in 0..grid.rows {
        let mut text = String::new();
        let mut runs = Vec::new();
        for column in 0..grid.columns {
            let Some(cell) = grid.cell(row, column) else {
                continue;
            };
            if cell.continuation {
                continue;
            }
            let contents = if cell.text.is_empty() {
                " "
            } else {
                cell.text.as_str()
            };
            text.push_str(contents);
            let mut run = window.text_style().to_run(contents.len());
            let (mut foreground, mut background) = (
                terminal_color(cell.foreground, DEFAULT_FOREGROUND),
                terminal_color(cell.background, DEFAULT_BACKGROUND),
            );
            if cell.inverse {
                std::mem::swap(&mut foreground, &mut background);
            }
            if cell.dim {
                foreground = dim(foreground);
            }
            if selected(selection, row, column) {
                background = SELECTION_BACKGROUND;
            }
            if !grid.cursor_hidden && grid.cursor == (row, column) {
                background = if focused {
                    CURSOR_BACKGROUND
                } else {
                    CURSOR_UNFOCUSED
                };
                foreground = DEFAULT_BACKGROUND;
            }
            run.color = rgb(foreground).into();
            run.background_color = Some(rgb(background).into());
            if cell.bold {
                run.font.weight = FontWeight::BOLD;
            }
            if cell.italic {
                run.font.style = FontStyle::Italic;
            }
            if cell.underline {
                run.underline = Some(UnderlineStyle {
                    thickness: px(1.),
                    color: Some(rgb(foreground).into()),
                    wavy: false,
                });
            }
            runs.push(run);
        }
        if text.is_empty() {
            continue;
        }
        let line = window
            .text_system()
            .shape_line(SharedString::from(text), px(14.), &runs, None);
        let origin = gpui::point(
            bounds.left(),
            bounds.top() + px(f32::from(row) * LINE_HEIGHT),
        );
        let _ = line.paint_background(
            origin,
            px(LINE_HEIGHT),
            TextAlign::Left,
            Some(bounds),
            window,
            cx,
        );
        let _ = line.paint(
            origin,
            px(LINE_HEIGHT),
            TextAlign::Left,
            Some(bounds),
            window,
            cx,
        );
    }
}

fn paint_preedit(
    text: &str,
    cursor: (u16, u16),
    bounds: Bounds<Pixels>,
    window: &mut Window,
    cx: &mut App,
) {
    let mut run = window.text_style().to_run(text.len());
    run.color = rgb(DEFAULT_FOREGROUND).into();
    run.background_color = Some(rgb(0x263244).into());
    run.underline = Some(UnderlineStyle {
        thickness: px(1.),
        color: Some(rgb(0x8bb9f5).into()),
        wavy: false,
    });
    let line = window.text_system().shape_line(
        SharedString::from(text.to_owned()),
        px(14.),
        &[run],
        None,
    );
    let origin = gpui::point(
        bounds.left() + px(f32::from(cursor.1) * CELL_WIDTH),
        bounds.top() + px(f32::from(cursor.0) * LINE_HEIGHT),
    );
    let _ = line.paint_background(
        origin,
        px(LINE_HEIGHT),
        TextAlign::Left,
        Some(bounds),
        window,
        cx,
    );
    let _ = line.paint(
        origin,
        px(LINE_HEIGHT),
        TextAlign::Left,
        Some(bounds),
        window,
        cx,
    );
}

fn selected(
    selection: Option<((u16, u16), (u16, u16))>,
    row: u16,
    column: u16,
) -> bool {
    let Some((start, end)) = selection else {
        return false;
    };
    let (start, end) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };
    (row, column) >= start && (row, column) <= end
}

fn dim(color: u32) -> u32 {
    let red = ((color >> 16) & 0xff) / 2;
    let green = ((color >> 8) & 0xff) / 2;
    let blue = (color & 0xff) / 2;
    (red << 16) | (green << 8) | blue
}

fn terminal_color(color: TerminalColor, default: u32) -> u32 {
    match color {
        TerminalColor::Default => default,
        TerminalColor::Rgb(red, green, blue) => {
            (u32::from(red) << 16) | (u32::from(green) << 8) | u32::from(blue)
        }
        TerminalColor::Indexed(index @ 0..=15) => [
            0x000000, 0xcd0000, 0x00cd00, 0xcdcd00, 0x0000ee, 0xcd00cd, 0x00cdcd, 0xe5e5e5,
            0x7f7f7f, 0xff0000, 0x00ff00, 0xffff00, 0x5c5cff, 0xff00ff, 0x00ffff, 0xffffff,
        ][usize::from(index)],
        TerminalColor::Indexed(index @ 16..=231) => {
            let value = index - 16;
            let levels = [0u32, 95, 135, 175, 215, 255];
            let red = levels[usize::from(value / 36)];
            let green = levels[usize::from((value / 6) % 6)];
            let blue = levels[usize::from(value % 6)];
            (red << 16) | (green << 8) | blue
        }
        TerminalColor::Indexed(index) => {
            let level = 8 + u32::from(index - 232) * 10;
            (level << 16) | (level << 8) | level
        }
    }
}
