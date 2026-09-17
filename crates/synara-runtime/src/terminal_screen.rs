//! Bounded terminal state, independent of any desktop layout or process host.
use crate::{BoundedBytes, RuntimeError};

pub const TERMINAL_HISTORY_ROWS: usize = 5_000;
const CONTROL_STRING_LIMIT: usize = 8_192;
const REPLY_LIMIT: usize = 16_384;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TerminalColor {
    #[default]
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}
impl From<vt100::Color> for TerminalColor {
    fn from(value: vt100::Color) -> Self {
        match value {
            vt100::Color::Default => Self::Default,
            vt100::Color::Idx(index) => Self::Indexed(index),
            vt100::Color::Rgb(r, g, b) => Self::Rgb(r, g, b),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TerminalCell {
    pub text: String,
    pub foreground: TerminalColor,
    pub background: TerminalColor,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub inverse: bool,
    pub wide: bool,
    pub continuation: bool,
}

/// An immutable visible viewport. Coordinates are cells, not UTF-8 byte offsets.
#[derive(Clone, Debug)]
pub struct TerminalGrid {
    pub rows: u16,
    pub columns: u16,
    pub cells: Vec<TerminalCell>,
    pub wrapped: Vec<bool>,
    pub cursor: (u16, u16),
    pub cursor_hidden: bool,
    pub alternate_screen: bool,
    pub application_cursor: bool,
    pub bracketed_paste: bool,
    pub scrollback: usize,
    pub title: String,
    /// Untrusted OSC 7 metadata. Never use this to change an execution host or
    /// perform filesystem operations. It is a display hint only.
    pub cwd_hint: Option<String>,
    pub revision: u64,
    pub discarded_control_strings: u64,
}
impl TerminalGrid {
    pub fn cell(&self, row: u16, column: u16) -> Option<&TerminalCell> {
        if row >= self.rows || column >= self.columns {
            return None;
        }
        self.cells
            .get(usize::from(row) * usize::from(self.columns) + usize::from(column))
    }

    /// Copy an inclusive cell selection from this exact snapshot. Never includes
    /// SGR/OSC framing, and wide continuations cannot duplicate a character.
    pub fn selected_text(&self, start: (u16, u16), end: (u16, u16)) -> String {
        if self.rows == 0 || self.columns == 0 {
            return String::new();
        }
        let clamp = |(row, col): (u16, u16)| (row.min(self.rows - 1), col.min(self.columns - 1));
        let (start, end) = (clamp(start), clamp(end));
        let (start, end) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        let mut text = String::new();
        for row in start.0..=end.0 {
            let mut first = if row == start.0 { start.1 } else { 0 };
            let last = if row == end.0 {
                end.1
            } else {
                self.columns - 1
            };
            if first > 0 && self.cell(row, first).is_some_and(|cell| cell.continuation) {
                first -= 1;
            }
            let line_start = text.len();
            for col in first..=last {
                if let Some(cell) = self.cell(row, col) {
                    if cell.continuation {
                        continue;
                    }
                    if cell.text.is_empty() {
                        text.push(' ');
                    } else {
                        text.extend(cell.text.chars().filter(|c| !c.is_control()));
                    }
                }
            }
            // Trim padding at hard line endings, not whitespace within soft wraps.
            let wrapped = self.wrapped.get(usize::from(row)).copied().unwrap_or(false);
            if !wrapped || row == end.0 {
                while text.len() > line_start && text.ends_with(' ') {
                    text.pop();
                }
            }
            if row != end.0 && !wrapped {
                text.push('\n');
            }
        }
        text
    }

    pub fn plain_text(&self) -> String {
        self.selected_text(
            (0, 0),
            (self.rows.saturating_sub(1), self.columns.saturating_sub(1)),
        )
        .trim_end_matches('\n')
        .to_owned()
    }
}

#[derive(Default)]
struct ScreenEvents {
    title: String,
    cwd_hint: Option<String>,
    replies: Vec<u8>,
}
impl ScreenEvents {
    fn reply(&mut self, bytes: &[u8]) {
        if self.replies.len().saturating_add(bytes.len()) <= REPLY_LIMIT {
            self.replies.extend_from_slice(bytes);
        }
    }
}
impl vt100::Callbacks for ScreenEvents {
    fn set_window_title(&mut self, _: &mut vt100::Screen, title: &[u8]) {
        self.title = display_hint(title, 256);
    }
    fn unhandled_osc(&mut self, _: &mut vt100::Screen, params: &[&[u8]]) {
        if let [b"7", value] = params {
            let hint = display_hint(value, 2_048);
            if hint.starts_with("file://") {
                self.cwd_hint = Some(hint);
            }
        }
    }
    fn unhandled_csi(
        &mut self,
        screen: &mut vt100::Screen,
        i1: Option<u8>,
        i2: Option<u8>,
        params: &[&[u16]],
        code: char,
    ) {
        if i2.is_some() {
            return;
        }
        let first = params.first().and_then(|p| p.first()).copied().unwrap_or(0);
        if params.len() > 1 {
            return;
        }
        match (i1, code, first) {
            (None, 'n', 5) => self.reply(b"\x1b[0n"),
            (None, 'n', 6) | (Some(b'?'), 'n', 6) => {
                let (row, col) = screen.cursor_position();
                let private = if i1.is_some() { "?" } else { "" };
                self.reply(
                    format!(
                        "\x1b[{private}{};{}R",
                        row.saturating_add(1),
                        col.saturating_add(1)
                    )
                    .as_bytes(),
                );
            }
            // VT100 basic attributes only. Do not advertise sixel or extensions
            // that this terminal does not implement.
            (None, 'c', 0) => self.reply(b"\x1b[?1;0c"),
            (Some(b'>'), 'c', 0) => self.reply(b"\x1b[>0;0;0c"),
            _ => {}
        }
    }
    // OSC 52 is deliberately not implemented. A child cannot read or write the
    // system clipboard through terminal output.
}

fn display_hint(bytes: &[u8], max_chars: usize) -> String {
    String::from_utf8_lossy(bytes).chars()
        .filter(|c| !c.is_control() && !matches!(*c, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'))
        .take(max_chars).collect()
}

/// Buffer OSC strings before passing them to vte's growable OSC buffer. An
/// unterminated or oversized string must never grow process memory indefinitely.
#[derive(Default)]
enum ControlState {
    #[default]
    Ground,
    Escape,
    Osc {
        bytes: Vec<u8>,
        discarded: bool,
    },
}
#[derive(Default)]
struct ControlBound {
    state: ControlState,
    discarded: u64,
}
impl ControlBound {
    fn filter(&mut self, input: &[u8]) -> Vec<u8> {
        let mut result = Vec::with_capacity(input.len().saturating_add(CONTROL_STRING_LIMIT));
        for &byte in input {
            match &mut self.state {
                ControlState::Ground if byte == 27 => self.state = ControlState::Escape,
                ControlState::Ground => result.push(byte),
                ControlState::Escape if byte == b']' => {
                    self.state = ControlState::Osc {
                        bytes: b"\x1b]".to_vec(),
                        discarded: false,
                    };
                }
                ControlState::Escape => {
                    result.push(27);
                    if byte != 27 {
                        result.push(byte);
                        self.state = ControlState::Ground;
                    }
                }
                ControlState::Osc { bytes, discarded } => {
                    if matches!(byte, 7 | 24 | 26 | 27) {
                        if !*discarded {
                            result.extend_from_slice(bytes);
                        }
                        if byte == 27 {
                            self.state = ControlState::Escape;
                        } else {
                            if !*discarded {
                                result.push(byte);
                            }
                            self.state = ControlState::Ground;
                        }
                    } else if !*discarded {
                        if bytes.len() < CONTROL_STRING_LIMIT {
                            bytes.push(byte);
                        } else {
                            bytes.clear();
                            *discarded = true;
                            self.discarded = self.discarded.saturating_add(1);
                        }
                    }
                }
            }
        }
        result
    }
}

pub(crate) struct TerminalScreen {
    parser: vt100::Parser<ScreenEvents>,
    tail: BoundedBytes,
    controls: ControlBound,
    revision: u64,
}
impl TerminalScreen {
    pub(crate) fn new(rows: u16, columns: u16) -> Result<Self, RuntimeError> {
        check_terminal_size(rows, columns)?;
        Ok(Self {
            parser: vt100::Parser::new_with_callbacks(
                rows,
                columns,
                TERMINAL_HISTORY_ROWS,
                ScreenEvents::default(),
            ),
            tail: BoundedBytes::new(1024 * 1024),
            controls: ControlBound::default(),
            revision: 0,
        })
    }
    pub(crate) fn process(&mut self, bytes: &[u8]) -> Vec<u8> {
        self.tail.push(bytes);
        self.parser.process(&self.controls.filter(bytes));
        self.revision = self.revision.wrapping_add(1);
        std::mem::take(&mut self.parser.callbacks_mut().replies)
    }
    pub(crate) fn resize(&mut self, rows: u16, columns: u16) -> Result<(), RuntimeError> {
        check_terminal_size(rows, columns)?;
        self.parser.screen_mut().set_size(rows, columns);
        self.revision = self.revision.wrapping_add(1);
        Ok(())
    }
    pub(crate) fn scroll(&mut self, offset: usize) {
        if !self.parser.screen().alternate_screen() {
            self.parser
                .screen_mut()
                .set_scrollback(offset.min(TERMINAL_HISTORY_ROWS));
            self.revision = self.revision.wrapping_add(1);
        }
    }
    pub(crate) fn application_cursor(&self) -> bool {
        self.parser.screen().application_cursor()
    }
    pub(crate) fn bracketed_paste(&self) -> bool {
        self.parser.screen().bracketed_paste()
    }
    pub(crate) fn raw_tail(&self) -> Vec<u8> {
        self.tail.bytes()
    }
    pub(crate) fn truncated(&self) -> bool {
        self.tail.truncated()
    }
    pub(crate) fn text(&self) -> String {
        self.parser.screen().contents()
    }
    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }
    pub(crate) fn grid(&self) -> TerminalGrid {
        let screen = self.parser.screen();
        let (rows, columns) = screen.size();
        let mut cells = Vec::with_capacity(usize::from(rows) * usize::from(columns));
        for row in 0..rows {
            for column in 0..columns {
                let cell = screen.cell(row, column).expect("validated viewport cell");
                cells.push(TerminalCell {
                    text: cell.contents().to_owned(),
                    foreground: cell.fgcolor().into(),
                    background: cell.bgcolor().into(),
                    bold: cell.bold(),
                    dim: cell.dim(),
                    italic: cell.italic(),
                    underline: cell.underline(),
                    inverse: cell.inverse(),
                    wide: cell.is_wide(),
                    continuation: cell.is_wide_continuation(),
                });
            }
        }
        TerminalGrid {
            rows,
            columns,
            cells,
            wrapped: (0..rows).map(|row| screen.row_wrapped(row)).collect(),
            cursor: screen.cursor_position(),
            cursor_hidden: screen.hide_cursor() || screen.scrollback() > 0,
            alternate_screen: screen.alternate_screen(),
            application_cursor: screen.application_cursor(),
            bracketed_paste: screen.bracketed_paste(),
            scrollback: screen.scrollback(),
            title: self.parser.callbacks().title.clone(),
            cwd_hint: self.parser.callbacks().cwd_hint.clone(),
            revision: self.revision,
            discarded_control_strings: self.controls.discarded,
        }
    }
}

pub(crate) fn check_terminal_size(rows: u16, columns: u16) -> Result<(), RuntimeError> {
    // Cell budget also bounds the retained history at the maximum column count.
    if rows == 0
        || columns == 0
        || rows > 200
        || columns > 500
        || u32::from(rows) * u32::from(columns) > 40_000
    {
        return Err(RuntimeError::Invalid("invalid terminal dimensions".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cells_preserve_styles_wide_unicode_cursor_and_modes() {
        let mut s = TerminalScreen::new(4, 20).unwrap();
        s.process(b"\x1b[31;44;1;3;4m");
        s.process("A界e\u{301}".as_bytes());
        s.process(b"\x1b[?1h\x1b[?2004h");
        let g = s.grid();
        let c = g.cell(0, 0).unwrap();
        assert_eq!(c.foreground, TerminalColor::Indexed(1));
        assert_eq!(c.background, TerminalColor::Indexed(4));
        assert!(c.bold && c.italic && c.underline);
        assert_eq!(g.cell(0, 1).unwrap().text, "界");
        assert!(g.cell(0, 1).unwrap().wide);
        assert!(g.cell(0, 2).unwrap().continuation);
        assert_eq!(g.cell(0, 3).unwrap().text, "e\u{301}");
        assert_eq!(g.cursor, (0, 4));
        assert!(g.application_cursor && g.bracketed_paste);
        assert_eq!(g.selected_text((0, 0), (0, 3)), "A界e\u{301}");
        assert_eq!(g.selected_text((0, 2), (0, 2)), "界");
        assert!(!g.plain_text().contains('\x1b'));
    }
    #[test]
    fn alternate_screen_restores_main_and_resize_is_bounded() {
        let mut s = TerminalScreen::new(3, 10).unwrap();
        s.process(b"main\x1b[?1049h\x1b[2J\x1b[Halternate");
        assert!(s.grid().alternate_screen);
        assert!(s.text().contains("alternate"));
        s.process(b"\x1b[?1049l");
        assert!(!s.grid().alternate_screen);
        assert!(s.text().contains("main"));
        s.resize(5, 12).unwrap();
        assert_eq!((s.grid().rows, s.grid().columns), (5, 12));
        assert!(s.resize(0, 12).is_err());
        assert!(s.resize(200, 500).is_err());
    }
    #[test]
    fn user_scrollback_does_not_follow_new_output() {
        let mut s = TerminalScreen::new(2, 10).unwrap();
        for n in 0..20 {
            s.process(format!("{n:02}\r\n").as_bytes());
        }
        s.scroll(10);
        let old = s.grid().plain_text();
        s.process(b"20\r\n");
        assert_eq!(s.grid().plain_text(), old);
        assert!(s.grid().cursor_hidden);
        s.scroll(0);
        assert!(s.grid().plain_text().contains("20"));
    }
    #[test]
    fn replies_are_bounded_and_clipboard_output_is_inert() {
        let mut s = TerminalScreen::new(3, 10).unwrap();
        assert_eq!(
            s.process(b"xy\x1b[6n\x1b[5n\x1b[c"),
            b"\x1b[1;3R\x1b[0n\x1b[?1;0c"
        );
        assert!(
            s.process(b"\x1b]52;c;?\x07\x1b]52;c;c2VjcmV0\x07")
                .is_empty()
        );
        let flood = b"\x1b[6n".repeat(10_000);
        assert!(s.process(&flood).len() <= REPLY_LIMIT);
    }
    #[test]
    fn untrusted_titles_and_unterminated_osc_are_bounded() {
        let mut s = TerminalScreen::new(3, 10).unwrap();
        s.process(b"\x1b]2;small title\x07\x1b]7;file://host/project\x1b\\ok");
        assert_eq!(s.grid().title, "small title");
        assert_eq!(s.grid().cwd_hint.as_deref(), Some("file://host/project"));
        s.process(b"\x1b]2;");
        for _ in 0..300 {
            s.process(&[b'x'; 8192]);
        }
        s.process(b"\x07after");
        assert_eq!(s.grid().title, "small title");
        assert_eq!(s.grid().discarded_control_strings, 1);
        assert!(s.text().contains("after"));
        assert!(s.truncated());
        assert!(s.raw_tail().len() <= 1024 * 1024);
    }
}
