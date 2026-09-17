//! Legacy xterm input encoding and a separate, explicit clipboard trust boundary.
use crate::RuntimeError;

/// Input and clipboard messages are bounded before they reach the PTY queue.
pub const MAX_TERMINAL_INPUT_BYTES: usize = 256 * 1024;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TerminalModifiers {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
}
impl TerminalModifiers {
    fn parameter(self) -> u8 {
        1 + u8::from(self.shift) + 2 * u8::from(self.alt) + 4 * u8::from(self.control)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalKey {
    Character(char),
    Enter,
    Backspace,
    Tab,
    Escape,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    Insert,
    Delete,
    PageUp,
    PageDown,
    Function(u8),
}

/// Encode a physical key using the negotiated application-cursor mode.
///
/// Printable text normally arrives through the platform text-input/IME callback.
/// Use `Character` for modified keys only, to avoid sending a key twice. This
/// implements legacy xterm sequences, not unnegotiated kitty/CSI-u extensions.
pub fn encode_terminal_key(
    key: TerminalKey,
    modifiers: TerminalModifiers,
    application_cursor: bool,
) -> Option<Vec<u8>> {
    let parameter = modifiers.parameter();
    let cursor = |code: char| {
        if parameter != 1 {
            format!("\x1b[1;{parameter}{code}").into_bytes()
        } else if application_cursor {
            format!("\x1bO{code}").into_bytes()
        } else {
            format!("\x1b[{code}").into_bytes()
        }
    };
    let tilde = |number: u8| {
        if parameter == 1 {
            format!("\x1b[{number}~").into_bytes()
        } else {
            format!("\x1b[{number};{parameter}~").into_bytes()
        }
    };
    let mut bytes = match key {
        TerminalKey::Up => return Some(cursor('A')),
        TerminalKey::Down => return Some(cursor('B')),
        TerminalKey::Right => return Some(cursor('C')),
        TerminalKey::Left => return Some(cursor('D')),
        TerminalKey::Home => return Some(cursor('H')),
        TerminalKey::End => return Some(cursor('F')),
        TerminalKey::Insert => return Some(tilde(2)),
        TerminalKey::Delete => return Some(tilde(3)),
        TerminalKey::PageUp => return Some(tilde(5)),
        TerminalKey::PageDown => return Some(tilde(6)),
        TerminalKey::Function(number @ 1..=4) => {
            let code = char::from(b'P' + number - 1);
            return Some(if parameter == 1 {
                format!("\x1bO{code}").into_bytes()
            } else {
                format!("\x1b[1;{parameter}{code}").into_bytes()
            });
        }
        TerminalKey::Function(number @ 5..=12) => {
            return Some(tilde(
                [15, 17, 18, 19, 20, 21, 23, 24][usize::from(number - 5)],
            ));
        }
        TerminalKey::Function(_) => return None,
        TerminalKey::Enter => vec![b'\r'],
        TerminalKey::Backspace => vec![if modifiers.control { 8 } else { 127 }],
        TerminalKey::Tab if modifiers.shift => b"\x1b[Z".to_vec(),
        TerminalKey::Tab => vec![b'\t'],
        TerminalKey::Escape => vec![27],
        TerminalKey::Character(character) if modifiers.control => {
            vec![control_character(character)?]
        }
        TerminalKey::Character(character) if !character.is_control() => {
            character.to_string().into_bytes()
        }
        TerminalKey::Character(_) => return None,
    };
    if modifiers.alt {
        bytes.insert(0, 27);
    }
    Some(bytes)
}

fn control_character(character: char) -> Option<u8> {
    Some(match character.to_ascii_uppercase() {
        'A'..='Z' => character.to_ascii_uppercase() as u8 - b'A' + 1,
        '@' | ' ' | '2' => 0,
        '[' | '3' => 27,
        '\\' | '4' => 28,
        ']' | '5' => 29,
        '^' | '6' => 30,
        '_' | '7' | '/' => 31,
        '?' | '8' => 127,
        _ => return None,
    })
}

/// Encode committed platform text, not preedit text or clipboard contents.
/// Control keys have their own typed path above. Clipboard text must use
/// `PreparedPaste`, even when the platform delivers it through a text callback.
pub fn encode_terminal_text(text: &str) -> Result<Vec<u8>, RuntimeError> {
    if text.len() > MAX_TERMINAL_INPUT_BYTES {
        return Err(RuntimeError::Limit);
    }
    if text.chars().any(char::is_control) {
        return Err(RuntimeError::Denied(
            "text input contains terminal controls".into(),
        ));
    }
    Ok(text.as_bytes().to_vec())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PasteDecision {
    Unreviewed,
    Approved,
}

/// Sanitized immutable clipboard data. Constructing this object sends nothing.
/// The GUI must show `text()` and obtain deliberate consent when review is
/// required. Approval is for this exact object, not for subsequent clipboard data.
#[derive(Clone, Debug)]
pub struct PreparedPaste {
    text: String,
    requires_review: bool,
    removed_controls: usize,
}
impl PreparedPaste {
    pub fn new(clipboard: &str) -> Result<Self, RuntimeError> {
        if clipboard.len() > MAX_TERMINAL_INPUT_BYTES {
            return Err(RuntimeError::Limit);
        }
        let mut text = String::with_capacity(clipboard.len());
        let mut requires_review = false;
        let mut removed_controls = 0;
        let mut characters = clipboard.chars().peekable();
        while let Some(character) = characters.next() {
            match character {
                '\r' => {
                    requires_review = true;
                    if characters.peek() == Some(&'\n') {
                        characters.next();
                    }
                    text.push('\n');
                }
                '\n' | '\t' => {
                    requires_review = true;
                    text.push(character);
                }
                '\u{2028}' | '\u{2029}' => {
                    requires_review = true;
                    text.push('\n');
                }
                c if c.is_control() || is_bidi_control(c) => {
                    requires_review = true;
                    removed_controls += 1;
                }
                c => text.push(c),
            }
        }
        Ok(Self {
            text,
            requires_review,
            removed_controls,
        })
    }

    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn requires_review(&self) -> bool {
        self.requires_review
    }
    pub fn removed_controls(&self) -> usize {
        self.removed_controls
    }

    /// Do not append Enter. Without negotiated bracketed paste, approved line
    /// breaks may execute commands, which the review UI must explicitly explain.
    /// Clipboard data cannot introduce ESC or C1, including its own bracketed
    /// paste terminator. The framing here is the only source of escape bytes.
    pub fn encode(
        &self,
        decision: PasteDecision,
        bracketed: bool,
    ) -> Result<Vec<u8>, RuntimeError> {
        if self.requires_review && decision != PasteDecision::Approved {
            return Err(RuntimeError::Denied(
                "clipboard paste requires review".into(),
            ));
        }
        if self.text.is_empty() {
            return Ok(Vec::new());
        }
        let mut bytes = Vec::with_capacity(self.text.len() + 12);
        if bracketed {
            bytes.extend_from_slice(b"\x1b[200~");
        }
        bytes.extend_from_slice(self.text.as_bytes());
        if bracketed {
            bytes.extend_from_slice(b"\x1b[201~");
        }
        Ok(bytes)
    }
}

fn is_bidi_control(character: char) -> bool {
    matches!(character, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_and_modified_keys_follow_xterm_modes() {
        let plain = TerminalModifiers::default();
        assert_eq!(
            encode_terminal_key(TerminalKey::Up, plain, false).unwrap(),
            b"\x1b[A"
        );
        assert_eq!(
            encode_terminal_key(TerminalKey::Up, plain, true).unwrap(),
            b"\x1bOA"
        );
        let modified = TerminalModifiers {
            shift: true,
            control: true,
            alt: true,
        };
        assert_eq!(
            encode_terminal_key(TerminalKey::Left, modified, true).unwrap(),
            b"\x1b[1;8D"
        );
        assert_eq!(
            encode_terminal_key(TerminalKey::Delete, modified, false).unwrap(),
            b"\x1b[3;8~"
        );
        assert_eq!(
            encode_terminal_key(TerminalKey::Function(1), plain, false).unwrap(),
            b"\x1bOP"
        );
        assert_eq!(
            encode_terminal_key(TerminalKey::Function(12), plain, false).unwrap(),
            b"\x1b[24~"
        );
        assert!(encode_terminal_key(TerminalKey::Function(99), plain, false).is_none());
    }

    #[test]
    fn control_alt_and_unicode_input_are_distinct() {
        let control = TerminalModifiers {
            control: true,
            ..Default::default()
        };
        for (character, expected) in [
            ('c', 3),
            ('Z', 26),
            (' ', 0),
            ('[', 27),
            ('\\', 28),
            ('_', 31),
            ('?', 127),
        ] {
            assert_eq!(
                encode_terminal_key(TerminalKey::Character(character), control, false).unwrap(),
                [expected]
            );
        }
        let alt = TerminalModifiers {
            alt: true,
            ..Default::default()
        };
        assert_eq!(
            encode_terminal_key(TerminalKey::Character('界'), alt, false).unwrap(),
            "\x1b界".as_bytes()
        );
        assert_eq!(
            encode_terminal_text("æ中文e\u{301}👩‍💻").unwrap(),
            "æ中文e\u{301}👩‍💻".as_bytes()
        );
        assert!(encode_terminal_text("bad\x1b[201~").is_err());
        assert!(encode_terminal_text("line\n").is_err());
    }

    #[test]
    fn basic_keys_and_shift_tab_are_not_command_submission() {
        let plain = TerminalModifiers::default();
        for (key, expected) in [
            (TerminalKey::Enter, b'\r'),
            (TerminalKey::Backspace, 127),
            (TerminalKey::Tab, b'\t'),
            (TerminalKey::Escape, 27),
        ] {
            assert_eq!(encode_terminal_key(key, plain, false).unwrap(), [expected]);
        }
        let shift = TerminalModifiers {
            shift: true,
            ..Default::default()
        };
        assert_eq!(
            encode_terminal_key(TerminalKey::Tab, shift, false).unwrap(),
            b"\x1b[Z"
        );
        assert_eq!(
            encode_terminal_key(TerminalKey::Character('x'), plain, false).unwrap(),
            b"x"
        );
    }

    #[test]
    fn plain_paste_never_appends_enter() {
        let paste = PreparedPaste::new("printf '中文'").unwrap();
        assert!(!paste.requires_review());
        assert_eq!(
            paste.encode(PasteDecision::Unreviewed, false).unwrap(),
            "printf '中文'".as_bytes()
        );
        assert_eq!(
            paste.encode(PasteDecision::Unreviewed, true).unwrap(),
            "\x1b[200~printf '中文'\x1b[201~".as_bytes()
        );
    }

    #[test]
    fn multiline_tabs_and_controls_always_require_review() {
        for text in [
            "one\ntwo",
            "one\r\ntwo",
            "one\rtwo",
            "one\ttwo",
            "\x03cancel",
            "\u{202e}hidden",
            "one\u{2028}two",
        ] {
            let paste = PreparedPaste::new(text).unwrap();
            assert!(paste.requires_review(), "{text:?}");
            for bracketed in [true, false] {
                assert!(paste.encode(PasteDecision::Unreviewed, bracketed).is_err());
                assert!(paste.encode(PasteDecision::Approved, bracketed).is_ok());
            }
        }
    }

    #[test]
    fn clipboard_cannot_synthesize_bracketed_paste_end_or_other_controls() {
        let paste = PreparedPaste::new("safe\x1b[201~\r\nid\n\u{009b}201~\0\x7f\x08").unwrap();
        assert_eq!(paste.text(), "safe[201~\nid\n201~");
        assert_eq!(paste.removed_controls(), 5);
        let encoded = paste.encode(PasteDecision::Approved, true).unwrap();
        assert_eq!(encoded.iter().filter(|byte| **byte == 27).count(), 2);
        assert_eq!(
            encoded
                .windows(6)
                .filter(|window| *window == b"\x1b[201~")
                .count(),
            1
        );
        assert!(
            !paste
                .text()
                .chars()
                .any(|c| c.is_control() && !matches!(c, '\n' | '\t'))
        );
    }

    #[test]
    fn every_c0_c1_and_del_control_is_removed_except_reviewed_whitespace() {
        let hostile: String = (0..=0x9f).filter_map(char::from_u32).collect();
        let paste = PreparedPaste::new(&hostile).unwrap();
        assert!(paste.requires_review());
        assert!(
            paste
                .text()
                .chars()
                .all(|c| !c.is_control() || matches!(c, '\n' | '\t'))
        );
        assert!(PreparedPaste::new(&"x".repeat(MAX_TERMINAL_INPUT_BYTES + 1)).is_err());
        assert!(encode_terminal_text(&"x".repeat(MAX_TERMINAL_INPUT_BYTES + 1)).is_err());
        assert!(
            PreparedPaste::new("")
                .unwrap()
                .encode(PasteDecision::Unreviewed, true)
                .unwrap()
                .is_empty()
        );
    }
}
