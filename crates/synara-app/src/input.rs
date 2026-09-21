use gpui::{
    App, Bounds, ClipboardItem, ContentMask, Context, ElementInputHandler, EntityInputHandler,
    EventEmitter, FocusHandle, Focusable, KeyDownEvent, MouseButton, Pixels, Point, SharedString,
    TextAlign, UTF16Selection, UnderlineStyle, Window, WrappedLine, canvas, div, fill, point,
    prelude::*, px, rgb, size,
};
use std::{ops::Range, rc::Rc};
use synara_core::TextBuffer;

mod policy;
mod navigation;

const MAX_INPUT: usize = 1024 * 1024;
const HISTORY_BYTES: usize = 16 * 1024 * 1024;
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EntryMode {
    SingleLine,
    Composer,
    Editor,
}
pub enum EntryEvent {
    Changed,
    Submit,
    Save,
}
#[derive(Clone)]
struct Line {
    shaped: Rc<WrappedLine>,
    start: usize,
    origin: Point<Pixels>,
}
/// Native platform input with UTF-16/UTF-8 conversion, IME composition and retained selection.
pub struct TextEntry {
    buffer: TextBuffer,
    focus: FocusHandle,
    placeholder: String,
    leading_icon: Option<crate::ui::Glyph>,
    picker_chrome: bool,
    send_on_enter: bool,
    mode: EntryMode,
    height: f32,
    lines: Vec<Line>,
    bounds: Bounds<Pixels>,
    scroll_y: Pixels,
    content_height: Pixels,
    anchor: Option<usize>,
    reversed: bool,
    dragging: bool,
    ensure_caret: bool,
    undo: Vec<TextBuffer>,
    redo: Vec<TextBuffer>,
    history_bytes: usize,
    pub error: Option<String>,
}
impl EventEmitter<EntryEvent> for TextEntry {}
impl Focusable for TextEntry {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
impl TextEntry {
    fn visible_height(&self) -> f32 {
        if self.mode == EntryMode::Composer {
            (f32::from(self.content_height) + 16.).clamp(self.height.max(self.line_height() + 12.), 196.)
        } else { self.height.max(self.line_height() + 12.) }
    }
    fn line_height(&self) -> f32 {
        let font = match self.mode {
            EntryMode::Editor => crate::ui::code_font_size(),
            EntryMode::Composer => crate::ui::ui_font_size() + 1.,
            EntryMode::SingleLine => crate::ui::ui_font_size(),
        };
        (font * 1.5).max(18.)
    }

    pub fn new(placeholder: &str, mode: EntryMode, height: f32, cx: &mut Context<Self>) -> Self {
        Self {
            buffer: TextBuffer::default(),
            focus: cx.focus_handle(),
            placeholder: placeholder.into(),
            leading_icon: None,
            picker_chrome: false,
            send_on_enter: true,
            mode,
            height,
            lines: vec![],
            bounds: Bounds::default(),
            scroll_y: px(0.),
            content_height: px(0.),
            anchor: None,
            reversed: false,
            dragging: false,
            ensure_caret: true,
            undo: vec![],
            redo: vec![],
            history_bytes: 0,
            error: None,
        }
    }
    pub fn with_leading_icon(mut self, icon: crate::ui::Glyph) -> Self {
        self.leading_icon = Some(icon);
        self
    }

    pub fn picker_chrome(mut self) -> Self {
        self.picker_chrome = true;
        self
    }

    pub fn set_send_on_enter(&mut self, enabled: bool) {
        self.send_on_enter = enabled;
    }

    /// Whether a platform IME currently owns marked text. Presentation shortcuts
    /// must leave Enter/Escape and candidate selection to that composition.
    pub fn is_composing(&self) -> bool {
        self.buffer.marked().is_some()
    }

    pub fn selected_text(&self) -> &str {
        self.buffer
            .text()
            .get(self.buffer.selection())
            .unwrap_or_default()
    }
    pub fn text(&self) -> &str {
        self.buffer.text()
    }
    pub fn set_text(&mut self, text: String, cx: &mut Context<Self>) {
        tracing::debug!(target: "synara_ui_layout", composer = self.mode == EntryMode::Composer, editor = self.mode == EntryMode::Editor, empty = text.is_empty(), "input-replaced");
        self.buffer = TextBuffer::new(text);
        self.undo.clear();
        self.redo.clear();
        self.history_bytes = 0;
        self.anchor = None;
        self.reversed = false;
        self.scroll_y = px(0.);
        self.ensure_caret = true;
        self.error = None;
        cx.notify();
    }
    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.set_text(String::new(), cx);
        cx.emit(EntryEvent::Changed);
    }
    fn checkpoint(&mut self) {
        self.redo.clear();
        if self.buffer.marked().is_some() {
            return;
        }
        self.undo.push(self.buffer.clone());
        self.history_bytes += self.buffer.text().len();
        while self.undo.len() > 128 || self.history_bytes > HISTORY_BYTES {
            self.history_bytes -= self.undo.remove(0).text().len();
        }
    }
    fn edit(&mut self, range: Range<usize>, text: &str, cx: &mut Context<Self>) {
        let text = if self.mode == EntryMode::SingleLine {
            text.replace(['\r', '\n'], " ")
        } else {
            text.to_owned()
        };
        if self
            .buffer
            .text()
            .len()
            .saturating_sub(range.len())
            .saturating_add(text.len())
            > MAX_INPUT
        {
            self.error = Some("Input exceeds 1 MiB".into());
            cx.notify();
            return;
        }
        if range.end > self.buffer.text().len()
            || !self.buffer.text().is_char_boundary(range.start)
            || !self.buffer.text().is_char_boundary(range.end)
        {
            return;
        }
        self.checkpoint();
        if self.buffer.replace(range, &text).is_ok() {
            self.changed(cx);
        }
    }
    fn changed(&mut self, cx: &mut Context<Self>) {
        self.anchor = None;
        self.reversed = false;
        self.ensure_caret = true;
        self.error = None;
        cx.emit(EntryEvent::Changed);
        cx.notify();
    }
    fn caret(&self) -> usize {
        if self.reversed {
            self.buffer.selection().start
        } else {
            self.buffer.selection().end
        }
    }
    fn select_to(&mut self, index: usize, extend: bool, cx: &mut Context<Self>) {
        let index = index.min(self.buffer.text().len());
        let anchor = if extend {
            *self.anchor.get_or_insert(self.caret())
        } else {
            self.anchor = None;
            index
        };
        if self
            .buffer
            .select(index.min(anchor)..index.max(anchor))
            .is_ok()
        {
            self.reversed = index < anchor;
            self.ensure_caret = true;
            cx.notify();
        }
    }
    fn position(&self, index: usize) -> Point<Pixels> {
        for line in &self.lines {
            if index >= line.start
                && index <= line.start + line.shaped.len()
                && let Some(position) = line
                    .shaped
                    .position_for_index(index - line.start, px(self.line_height()))
            {
                return line.origin + position;
            }
        }
        self.lines
            .last()
            .map_or(self.bounds.origin, |line| line.origin)
    }
    fn index_at(&self, position: Point<Pixels>) -> usize {
        for line in &self.lines {
            if position.y < line.origin.y + line.shaped.size(px(self.line_height())).height {
                let local = point(
                    (position.x - line.origin.x).max(px(0.)),
                    (position.y - line.origin.y).max(px(0.)),
                );
                let index = line
                    .shaped
                    .closest_index_for_position(local, px(self.line_height()))
                    .unwrap_or_else(|index| index);
                return (line.start + index).min(self.buffer.text().len());
            }
        }
        self.buffer.text().len()
    }
    fn key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.prefer_character_input {
            return;
        }
        let modifiers = event.keystroke.modifiers;
        let command = modifiers.control || modifiers.platform;
        let shift = modifiers.shift;
        let key = event.keystroke.key.as_str();
        if self.buffer.marked().is_some() && matches!(key, "enter" | "escape") {
            return;
        }
        match (command, key) {
            (true, "a") => {
                self.buffer.select_all();
                self.anchor = Some(0);
                self.reversed = false;
                cx.notify();
            }
            (true, "c") | (true, "x") => {
                let range = self.buffer.selection();
                if !range.is_empty() {
                    cx.write_to_clipboard(ClipboardItem::new_string(
                        self.buffer.text()[range.clone()].to_owned(),
                    ));
                    if key == "x" {
                        self.edit(range, "", cx);
                    }
                }
            }
            (true, "v") => {
                if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                    self.edit(self.buffer.selection(), &text, cx);
                }
            }
            (true, "z") | (true, "y") => {
                let redo = key == "y" || shift;
                if redo {
                    if let Some(previous) = self.redo.pop() {
                        self.history_bytes += self.buffer.text().len();
                        self.undo.push(self.buffer.clone());
                        self.buffer = previous;
                        self.changed(cx);
                    }
                } else if let Some(previous) = self.undo.pop() {
                    self.history_bytes = self.history_bytes.saturating_sub(previous.text().len());
                    self.redo.push(self.buffer.clone());
                    self.buffer = previous;
                    self.changed(cx);
                }
            }
            (true, "s") if self.mode == EntryMode::Editor => cx.emit(EntryEvent::Save),
            (_, "enter") => {
                if policy::submits_enter(
                    self.mode,
                    self.send_on_enter,
                    command,
                    shift,
                    modifiers.alt,
                ) {
                    if !event.is_held {
                        cx.emit(EntryEvent::Submit);
                    }
                } else {
                    self.edit(self.buffer.selection(), "\n", cx);
                }
            }
            (_, "backspace") | (_, "delete") => {
                self.checkpoint();
                let result = if key == "backspace" {
                    self.buffer.backspace()
                } else {
                    self.buffer.delete_forward()
                };
                if result.is_ok() {
                    self.changed(cx);
                }
            }
            (_, "left") | (_, "right") => {
                let old = self.caret();
                let mut cursor = self.buffer.clone();
                if shift {
                    let _ = cursor.select(old..old);
                }
                if key == "left" {
                    cursor.move_left();
                } else {
                    cursor.move_right();
                }
                self.select_to(cursor.selection().end, shift, cx);
            }
            (_, "home") | (_, "end") => {
                let index = self.caret();
                let text = self.buffer.text();
                let next = if key == "home" {
                    if command {
                        0
                    } else {
                        text[..index].rfind('\n').map_or(0, |n| n + 1)
                    }
                } else if command {
                    text.len()
                } else {
                    text[index..].find('\n').map_or(text.len(), |n| index + n)
                };
                self.select_to(next, shift, cx);
            }
            (_, "up") | (_, "down") => {
                let mut position = self.position(self.caret());
                position.y += px(if key == "up" {
                    -self.line_height()
                } else {
                    self.line_height()
                });
                position.y += px(self.line_height() / 2.);
                let next = if position.y < self.bounds.origin.y - self.scroll_y {
                    0
                } else {
                    self.index_at(position)
                };
                self.select_to(next, shift, cx);
            }
            (_, "tab") if self.mode == EntryMode::Editor => {
                self.edit(self.buffer.selection(), "    ", cx)
            }
            _ => return,
        }
        let _ = window;
        cx.stop_propagation();
    }
    fn prepare(&mut self, bounds: Bounds<Pixels>, window: &mut Window) {
        if self.bounds != bounds {
            tracing::debug!(target: "synara_ui_layout", composer = self.mode == EntryMode::Composer, editor = self.mode == EntryMode::Editor, ?bounds, "input-layout");
        }
        self.bounds = bounds;
        let empty = self.buffer.text().is_empty();
        let text: SharedString = if empty {
            self.placeholder.clone()
        } else {
            self.buffer.text().to_owned()
        }
        .into();
        let mut points = vec![0, text.len()];
        let selection = self.buffer.selection();
        let marked = self.buffer.marked();
        if !empty {
            points.extend([selection.start, selection.end]);
            if let Some(marked) = &marked {
                points.extend([marked.start, marked.end]);
            }
        }
        points.sort_unstable();
        points.dedup();
        let runs = points
            .windows(2)
            .map(|range| {
                let mut run = window.text_style().to_run(range[1] - range[0]);
                run.color = rgb(if empty {
                    crate::ui::palette().muted
                } else {
                    crate::ui::palette().text
                })
                .into();
                if !empty
                    && range[0] >= selection.start
                    && range[1] <= selection.end
                    && !selection.is_empty()
                {
                    run.background_color = Some(rgb(crate::ui::palette().selected).into());
                }
                if marked
                    .as_ref()
                    .is_some_and(|m| range[0] >= m.start && range[1] <= m.end)
                {
                    run.underline = Some(UnderlineStyle {
                        thickness: px(1.),
                        color: Some(rgb(crate::ui::palette().focus).into()),
                        wavy: false,
                    });
                }
                run
            })
            .collect::<Vec<_>>();
        let wrap = if self.mode == EntryMode::SingleLine {
            None
        } else {
            Some(bounds.size.width.max(px(8.)))
        };
        let shaped = match window.text_system().shape_text(
            text,
            px(match self.mode {
                EntryMode::Composer => crate::ui::ui_font_size() + 1.,
                EntryMode::Editor => crate::ui::code_font_size(),
                EntryMode::SingleLine => crate::ui::ui_font_size(),
            }),
            &runs,
            wrap,
            None,
        ) {
            Ok(lines) => lines,
            Err(_) => {
                self.lines.clear();
                return;
            }
        };
        let mut start = 0;
        let mut y = bounds.origin.y;
        self.lines = shaped
            .into_iter()
            .map(|shaped| {
                let origin = point(bounds.origin.x, y);
                y += shaped.size(px(self.line_height())).height;
                let line = Line {
                    start,
                    shaped: Rc::new(shaped),
                    origin,
                };
                start += line.shaped.len() + 1;
                line
            })
            .collect();
        self.content_height = (y - bounds.origin.y).max(px(self.line_height()));
        if self.ensure_caret {
            let caret = self.position(self.caret()).y - bounds.origin.y;
            if caret < self.scroll_y {
                self.scroll_y = caret;
            } else if caret + px(self.line_height()) > self.scroll_y + bounds.size.height {
                self.scroll_y = caret + px(self.line_height()) - bounds.size.height;
            }
            self.ensure_caret = false;
        }
        self.scroll_y = self.scroll_y.clamp(
            px(0.),
            (self.content_height - bounds.size.height).max(px(0.)),
        );
        // Single-line fields keep the insertion point in view horizontally as well.
        let x = if self.mode == EntryMode::SingleLine {
            (self.position(self.caret()).x - bounds.origin.x - bounds.size.width + px(4.))
                .max(px(0.))
        } else {
            px(0.)
        };
        for line in &mut self.lines {
            line.origin.x -= x;
            line.origin.y -= self.scroll_y;
        }
    }
}
impl Render for TextEntry {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        let paint_entity = entity.clone();
        div()
            .id("text-entry")
            .relative()
            .key_context("SynaraTextEntry")
            .track_focus(&self.focus)
            .tab_index(0)
            .w_full()
            .h(px(self.visible_height()))
            .p_2()
            .bg(crate::ui::surface(crate::ui::palette().canvas))
            .border_1()
            .border_color(rgb(if self.error.is_some() {
                0xb85e65
            } else {
                crate::ui::palette().border
            }))
            .rounded_md()
            .when(self.mode == EntryMode::SingleLine, |el| el.py_1().rounded_lg())
            .when(self.mode == EntryMode::Composer, |el| el
                .bg(gpui::rgba(0)).border_0().rounded_none().font_family(crate::ui::ui_font()))
            .when_some(self.leading_icon, |el, icon| el.pl(px(32.)).child(div().absolute().left(px(10.)).top(px(7.)).child(crate::ui::icon(icon))))
            .when(self.mode == EntryMode::Editor, |el| el.font_family(crate::ui::code_font()).flex_1().min_h_0().h_full())
            .when(self.picker_chrome, |el| el.bg(gpui::rgba(0)).border_0().rounded_none())
            .cursor_text()
            .when(self.mode == EntryMode::Composer, |el| el.child(crate::ui::layout_probe("composer-input")))
            .when(self.mode == EntryMode::Editor, |el| el.child(crate::ui::layout_probe("editor-input")))
            .on_key_down(cx.listener(Self::key))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &gpui::MouseDownEvent, window, cx| {
                    tracing::debug!(target: "synara_ui_layout", composer = this.mode == EntryMode::Composer, editor = this.mode == EntryMode::Editor, position = ?event.position, "input-mouse-focus");
                    window.focus(&this.focus, cx);
                    let index = this.index_at(event.position);
                    this.select_to(index, event.modifiers.shift, cx);
                    this.anchor = Some(if event.modifiers.shift {
                        this.anchor.unwrap_or(index)
                    } else {
                        index
                    });
                    this.dragging = true;
                    cx.stop_propagation();
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &gpui::MouseMoveEvent, _, cx| {
                if this.dragging && event.dragging() {
                    let index = this.index_at(event.position);
                    this.select_to(index, true, cx);
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.dragging = false),
            )
            .on_scroll_wheel(cx.listener(|this, event: &gpui::ScrollWheelEvent, _, cx| {
                this.scroll_y = (this.scroll_y - event.delta.pixel_delta(px(this.line_height())).y).clamp(
                    px(0.),
                    (this.content_height - this.bounds.size.height).max(px(0.)),
                );
                this.ensure_caret = false;
                cx.notify();
                cx.stop_propagation();
            }))
            .child(
                canvas(
                    move |bounds, window, cx| {
                        entity.update(cx, |this, cx| {
                            let previous = this.visible_height();
                            this.prepare(bounds, window);
                            if this.mode == EntryMode::Composer && (this.visible_height() - previous).abs() > 0.5 {
                                cx.notify();
                            }
                        });
                    },
                    move |bounds, _, window, cx| {
                        let focus = paint_entity.read(cx).focus.clone();
                        window.handle_input(
                            &focus,
                            ElementInputHandler::new(bounds, paint_entity.clone()),
                            cx,
                        );
                        window.with_content_mask(Some(ContentMask { bounds }), |window| {
                            let (lines, caret, line_height) = {
                                let this = paint_entity.read(cx);
                                (this.lines.clone(), this.position(this.caret()), this.line_height())
                            };
                            for line in &lines {
                                if line.origin.y + line.shaped.size(px(line_height)).height
                                    >= bounds.top()
                                    && line.origin.y < bounds.bottom()
                                {
                                    let _ = line.shaped.paint_background(
                                        line.origin,
                                        px(line_height),
                                        TextAlign::Left,
                                        Some(bounds),
                                        window,
                                        cx,
                                    );
                                    let _ = line.shaped.paint(
                                        line.origin,
                                        px(line_height),
                                        TextAlign::Left,
                                        Some(bounds),
                                        window,
                                        cx,
                                    );
                                }
                            }
                            if focus.is_focused(window) {
                                window.paint_quad(fill(
                                    Bounds::new(caret, size(px(1.5), px(line_height))),
                                    rgb(crate::ui::palette().focus),
                                ));
                            }
                        });
                    },
                )
                .size_full(),
            )
    }
}
impl EntityInputHandler for TextEntry {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        adjusted: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let bytes = self.buffer.utf16_range_to_bytes(range.clone()).ok()?;
        *adjusted = Some(range);
        Some(self.buffer.text()[bytes].into())
    }
    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let range = self.buffer.selection();
        Some(UTF16Selection {
            range: self.buffer.byte_to_utf16(range.start).ok()?
                ..self.buffer.byte_to_utf16(range.end).ok()?,
            reversed: self.reversed,
        })
    }
    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        let range = self.buffer.marked()?;
        Some(
            self.buffer.byte_to_utf16(range.start).ok()?
                ..self.buffer.byte_to_utf16(range.end).ok()?,
        )
    }
    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.buffer.unmark();
        cx.notify();
    }
    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = match range {
            Some(range) => match self.buffer.utf16_range_to_bytes(range) {
                Ok(r) => r,
                Err(_) => return,
            },
            None => self
                .buffer
                .marked()
                .unwrap_or_else(|| self.buffer.selection()),
        };
        self.edit(range, text, cx);
    }
    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        selected: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = match range {
            Some(r) => match self.buffer.utf16_range_to_bytes(r) {
                Ok(r) => r,
                Err(_) => return,
            },
            None => self
                .buffer
                .marked()
                .unwrap_or_else(|| self.buffer.selection()),
        };
        let selected = match selected {
            Some(r) => match TextBuffer::new(text.into()).utf16_range_to_bytes(r) {
                Ok(r) => Some(r),
                Err(_) => return,
            },
            None => None,
        };
        if self
            .buffer
            .text()
            .len()
            .saturating_sub(range.len())
            .saturating_add(text.len())
            > MAX_INPUT
        {
            return;
        }
        let start = range.start;
        self.checkpoint();
        if self.buffer.replace(range, text).is_err() {
            return;
        }
        if !text.is_empty() {
            let _ = self.buffer.mark(start..start + text.len());
        }
        if let Some(r) = selected {
            let _ = self.buffer.select(start + r.start..start + r.end);
        }
        self.changed(cx);
    }
    fn bounds_for_range(
        &mut self,
        range: Range<usize>,
        _: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let range = self.buffer.utf16_range_to_bytes(range).ok()?;
        let start = self.position(range.start);
        let end = self.position(range.end);
        Some(Bounds::new(
            start,
            size(
                if start.y == end.y {
                    (end.x - start.x).max(px(1.))
                } else {
                    px(1.)
                },
                px(self.line_height()),
            ),
        ))
    }
    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        self.buffer.byte_to_utf16(self.index_at(point)).ok()
    }
    fn set_selected_text_range(
        &mut self,
        range: Range<usize>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Ok(range) = self.buffer.utf16_range_to_bytes(range) {
            let _ = self.buffer.select(range);
            self.reversed = false;
            self.anchor = None;
            self.ensure_caret = true;
            cx.notify();
        }
    }
    fn text_length_utf16(&mut self, _: &mut Window, _: &mut Context<Self>) -> Option<usize> {
        Some(self.buffer.text().encode_utf16().count())
    }
}
