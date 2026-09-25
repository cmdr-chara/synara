//! Explicit, window-addressed X11 input. Never falls back to ambient XTEST input.
//! xdotool --window uses SendEvent, which some applications intentionally reject.
//! Command completion proves delivery attempt, not application-level success.
use crate::{RuntimeError, SnapTools, SnapWindow, device_tools::command};
use serde::{Deserialize, Serialize};
use std::{path::Path, time::Duration};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComputerButton {
    Left,
    Middle,
    Right,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComputerKey {
    #[serde(alias = "return")]
    Enter,
    Tab,
    #[serde(alias = "esc")]
    Escape,
    Backspace,
    #[serde(alias = "del")]
    Delete,
    #[serde(alias = "arrowleft")]
    Left,
    #[serde(alias = "arrowright")]
    Right,
    #[serde(alias = "arrowup")]
    Up,
    #[serde(alias = "arrowdown")]
    Down,
    Home,
    End,
    #[serde(alias = "pageup")]
    PageUp,
    #[serde(alias = "pagedown")]
    PageDown,
    #[serde(alias = "spacebar")]
    Space,
    Insert,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    // Named editing chords only. Never accept caller-supplied key sequences,
    // Alt/Meta shortcuts, held modifiers or clipboard operations here.
    BackTab,
    ShiftEnter,
    SelectAll,
    Undo,
    Redo,
    WordLeft,
    WordRight,
    SelectLeft,
    SelectRight,
    SelectUp,
    SelectDown,
    SelectWordLeft,
    SelectWordRight,
    SelectHome,
    SelectEnd,
    DocumentHome,
    DocumentEnd,
    SelectDocumentHome,
    SelectDocumentEnd,
    DeleteWordLeft,
    DeleteWordRight,
}
impl ComputerKey {
    fn sequence(self) -> &'static str {
        match self {
            Self::Enter => "Return",
            Self::Tab => "Tab",
            Self::Escape => "Escape",
            Self::Backspace => "BackSpace",
            Self::Delete => "Delete",
            Self::Left => "Left",
            Self::Right => "Right",
            Self::Up => "Up",
            Self::Down => "Down",
            Self::Home => "Home",
            Self::End => "End",
            Self::PageUp => "Prior",
            Self::PageDown => "Next",
            Self::Space => "space",
            Self::Insert => "Insert",
            Self::F1 => "F1",
            Self::F2 => "F2",
            Self::F3 => "F3",
            Self::F4 => "F4",
            Self::F5 => "F5",
            Self::F6 => "F6",
            Self::F7 => "F7",
            Self::F8 => "F8",
            Self::F9 => "F9",
            Self::F10 => "F10",
            Self::F11 => "F11",
            Self::F12 => "F12",
            Self::BackTab => "shift+Tab",
            Self::ShiftEnter => "shift+Return",
            Self::SelectAll => "ctrl+a",
            Self::Undo => "ctrl+z",
            Self::Redo => "ctrl+shift+z",
            Self::WordLeft => "ctrl+Left",
            Self::WordRight => "ctrl+Right",
            Self::SelectLeft => "shift+Left",
            Self::SelectRight => "shift+Right",
            Self::SelectUp => "shift+Up",
            Self::SelectDown => "shift+Down",
            Self::SelectWordLeft => "ctrl+shift+Left",
            Self::SelectWordRight => "ctrl+shift+Right",
            Self::SelectHome => "shift+Home",
            Self::SelectEnd => "shift+End",
            Self::DocumentHome => "ctrl+Home",
            Self::DocumentEnd => "ctrl+End",
            Self::SelectDocumentHome => "ctrl+shift+Home",
            Self::SelectDocumentEnd => "ctrl+shift+End",
            Self::DeleteWordLeft => "ctrl+BackSpace",
            Self::DeleteWordRight => "ctrl+Delete",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum ComputerAction {
    Move {
        x: u32,
        y: u32,
    },
    Click {
        x: u32,
        y: u32,
        button: ComputerButton,
    },
    DoubleClick {
        x: u32,
        y: u32,
        button: ComputerButton,
    },
    Drag {
        from_x: u32,
        from_y: u32,
        to_x: u32,
        to_y: u32,
        button: ComputerButton,
    },
    Scroll {
        x: u32,
        y: u32,
        down: bool,
        steps: u8,
    },
    HorizontalScroll {
        x: u32,
        y: u32,
        right: bool,
        steps: u8,
    },
    Type {
        text: String,
    },
    Key {
        key: ComputerKey,
    },
}
impl ComputerAction {
    pub fn validate(&self, width: u32, height: u32) -> Result<(), RuntimeError> {
        match self {
            Self::Move { x, y }
            | Self::Click { x, y, .. }
            | Self::DoubleClick { x, y, .. }
            | Self::Scroll { x, y, .. }
            | Self::HorizontalScroll { x, y, .. }
                if *x >= width || *y >= height =>
            {
                Err(RuntimeError::Invalid(
                    "Input coordinates are outside the observed window".into(),
                ))
            }
            Self::Drag {
                from_x,
                from_y,
                to_x,
                to_y,
                ..
            } if *from_x >= width
                || *from_y >= height
                || *to_x >= width
                || *to_y >= height =>
            {
                Err(RuntimeError::Invalid(
                    "Drag coordinates are outside the observed window".into(),
                ))
            }
            Self::Scroll { steps, .. } | Self::HorizontalScroll { steps, .. }
                if !(1..=8).contains(steps) =>
            {
                Err(RuntimeError::Limit)
            }
            Self::Type { text } if !valid_typed_text(text) => Err(RuntimeError::Invalid("Window typing accepts 1-512 UTF-8 bytes of text without controls, line separators, or bidirectional formatting characters. Use separate reviewed keys for Enter or Tab".into())),
            _ if width == 0 || height == 0 => Err(RuntimeError::Invalid("Window dimensions must be positive".into())),
            _ => Ok(()),
        }
    }
    fn commands(&self, id: u32) -> Vec<Vec<String>> {
        let window = id.to_string();
        let move_to = |x: u32, y: u32| {
            vec![
                "mousemove".into(),
                "--window".into(),
                window.clone(),
                x.to_string(),
                y.to_string(),
            ]
        };
        let click = |button: u8, repeats: u8| {
            vec![
                "click".into(),
                "--window".into(),
                window.clone(),
                "--repeat".into(),
                repeats.to_string(),
                "--delay".into(),
                "20".into(),
                button.to_string(),
            ]
        };
        let button_number = |button: &ComputerButton| match button {
            ComputerButton::Left => 1,
            ComputerButton::Middle => 2,
            ComputerButton::Right => 3,
        };
        match self {
            Self::Move { x, y } => vec![move_to(*x, *y)],
            Self::Click { x, y, button } | Self::DoubleClick { x, y, button } => vec![
                move_to(*x, *y),
                click(
                    button_number(button),
                    if matches!(self, Self::DoubleClick { .. }) {
                        2
                    } else {
                        1
                    },
                ),
            ],
            Self::Drag {
                from_x,
                from_y,
                to_x,
                to_y,
                button,
            } => vec![
                move_to(*from_x, *from_y),
                vec![
                    "mousedown".into(),
                    "--window".into(),
                    window.clone(),
                    button_number(button).to_string(),
                ],
                move_to(*to_x, *to_y),
                vec![
                    "mouseup".into(),
                    "--window".into(),
                    window.clone(),
                    button_number(button).to_string(),
                ],
            ],
            Self::Scroll { x, y, down, steps } => {
                vec![move_to(*x, *y), click(if *down { 5 } else { 4 }, *steps)]
            }
            Self::HorizontalScroll { x, y, right, steps } => {
                vec![move_to(*x, *y), click(if *right { 7 } else { 6 }, *steps)]
            }
            Self::Type { text } => vec![vec![
                "type".into(),
                "--window".into(),
                window,
                "--delay".into(),
                "2".into(),
                "--".into(),
                text.clone(),
            ]],
            Self::Key { key } => vec![vec![
                "key".into(),
                "--window".into(),
                window,
                "--".into(),
                key.sequence().into(),
            ]],
        }
    }
}

fn valid_typed_text(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 512
        && !text.chars().any(|character| {
            character.is_control()
                || matches!(
                    character,
                    '\u{061c}'
                        | '\u{200e}'
                        | '\u{200f}'
                        | '\u{2028}'
                        | '\u{2029}'
                        | '\u{202a}'..='\u{202e}'
                        | '\u{2066}'..='\u{206f}'
                )
        })
}
#[derive(Clone)]
pub struct ComputerTools {
    capture: SnapTools,
}
impl ComputerTools {
    /// Setup inspects installed helpers only. Nothing is downloaded or launched.
    pub fn setup() -> Result<Self, RuntimeError> {
        let capture = SnapTools::setup()?;
        if !Path::new("/usr/bin/xdotool").is_file() {
            return Err(RuntimeError::Unsupported("Computer Use requires the installed /usr/bin/xdotool helper in addition to AppSnap. No automatic install or fallback".into()));
        }
        Ok(Self { capture })
    }
    pub async fn discover(
        &self,
        cancel: &CancellationToken,
    ) -> Result<Vec<SnapWindow>, RuntimeError> {
        Ok(self
            .capture
            .discover(cancel)
            .await?
            .into_iter()
            .filter(|w| w.pid != std::process::id())
            .collect())
    }
    pub async fn observe(
        &self,
        window: &SnapWindow,
        cancel: &CancellationToken,
    ) -> Result<Vec<u8>, RuntimeError> {
        if window.pid == std::process::id() {
            return Err(RuntimeError::Denied(
                "Synara cannot grant control over its own approval window".into(),
            ));
        }
        self.capture.capture(window, cancel).await
    }
    pub async fn act(
        &self,
        window: &SnapWindow,
        action: &ComputerAction,
        cancel: &CancellationToken,
    ) -> Result<(), RuntimeError> {
        if window.pid == std::process::id() {
            return Err(RuntimeError::Denied(
                "Synara is not a computer-control target".into(),
            ));
        }
        action.validate(window.width, window.height)?;
        let drag_button = match action {
            ComputerAction::Drag { button, .. } => Some(match button {
                ComputerButton::Left => 1,
                ComputerButton::Middle => 2,
                ComputerButton::Right => 3,
            }),
            _ => None,
        };
        let mut pressed = false;
        for args in action.commands(window.native_id()) {
            let is_down = args.first().is_some_and(|arg| arg == "mousedown");
            let is_up = args.first().is_some_and(|arg| arg == "mouseup");
            let step = async {
                if cancel.is_cancelled() {
                    return Err(RuntimeError::Closed);
                }
                self.capture.validate_window(window, cancel).await?;
                command::run(Path::new("/usr/bin/xdotool"), args, 1024, cancel).await
            }
            .await;
            if let Err(error) = step {
                if pressed {
                    if let Some(button) = drag_button {
                        let release = vec![
                            "mouseup".into(),
                            "--window".into(),
                            window.native_id().to_string(),
                            button.to_string(),
                        ];
                        let cleanup = CancellationToken::new();
                        let _ = tokio::time::timeout(
                            Duration::from_secs(1),
                            command::run(Path::new("/usr/bin/xdotool"), release, 1024, &cleanup),
                        )
                        .await;
                    }
                }
                return Err(error);
            }
            if is_down {
                pressed = true;
            } else if is_up {
                pressed = false;
            }
        }
        if cancel.is_cancelled() {
            return Err(RuntimeError::Closed);
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_input_command_is_explicitly_window_addressed() {
        for action in [
            ComputerAction::Move { x: 4, y: 8 },
            ComputerAction::Click {
                x: 5,
                y: 9,
                button: ComputerButton::Left,
            },
            ComputerAction::DoubleClick {
                x: 7,
                y: 11,
                button: ComputerButton::Left,
            },
            ComputerAction::Drag {
                from_x: 3,
                from_y: 4,
                to_x: 30,
                to_y: 40,
                button: ComputerButton::Left,
            },
            ComputerAction::Scroll {
                x: 1,
                y: 2,
                down: true,
                steps: 8,
            },
            ComputerAction::HorizontalScroll {
                x: 1,
                y: 2,
                right: true,
                steps: 8,
            },
            ComputerAction::Type {
                text: "literal --window 0; echo unchanged".into(),
            },
            ComputerAction::Key {
                key: ComputerKey::Enter,
            },
            ComputerAction::Key {
                key: ComputerKey::SelectAll,
            },
            ComputerAction::Key {
                key: ComputerKey::SelectWordRight,
            },
            ComputerAction::Key {
                key: ComputerKey::BackTab,
            },
        ] {
            action.validate(100, 100).unwrap();
            for args in action.commands(42) {
                assert_eq!(&args[1..3], &["--window", "42"]);
                assert!(!args.iter().any(|a| {
                    [
                        "windowactivate",
                        "windowfocus",
                        "--clearmodifiers",
                        "keydown",
                    ]
                    .contains(&a.as_str())
                }));
            }
        }
        let action = ComputerAction::Drag {
            from_x: 3,
            from_y: 4,
            to_x: 30,
            to_y: 40,
            button: ComputerButton::Left,
        };
        assert_eq!(
            action.commands(42),
            vec![
                vec!["mousemove", "--window", "42", "3", "4"],
                vec!["mousedown", "--window", "42", "1"],
                vec!["mousemove", "--window", "42", "30", "40"],
                vec!["mouseup", "--window", "42", "1"],
            ]
        );

        let action = ComputerAction::HorizontalScroll {
            x: 7,
            y: 11,
            right: true,
            steps: 3,
        };
        assert_eq!(
            action.commands(42),
            vec![
                vec!["mousemove", "--window", "42", "7", "11"],
                vec![
                    "click", "--window", "42", "--repeat", "3", "--delay", "20", "7"
                ],
            ]
        );

        let action = ComputerAction::DoubleClick {
            x: 7,
            y: 11,
            button: ComputerButton::Left,
        };
        assert_eq!(
            action.commands(42),
            vec![
                vec!["mousemove", "--window", "42", "7", "11"],
                vec![
                    "click", "--window", "42", "--repeat", "2", "--delay", "20", "1"
                ],
            ]
        );
        let action = ComputerAction::Key {
            key: ComputerKey::SelectWordRight,
        };
        assert_eq!(
            action.commands(42),
            vec![vec!["key", "--window", "42", "--", "ctrl+shift+Right"]]
        );
    }
    #[test]
    fn invalid_input_cannot_become_global_shortcuts_or_command_chains() {
        for value in [
            r#"{"action":"key","key":"ctrl+alt+Delete"}"#,
            r#"{"action":"key","key":"alt+Tab"}"#,
            r#"{"action":"key","key":"select_all","modifiers":["alt"]}"#,
            r#"{"action":"type","text":"ok","window":0}"#,
            r#"{"action":"click","x":-1,"y":0,"button":"left"}"#,
        ] {
            assert!(serde_json::from_str::<ComputerAction>(value).is_err());
        }
        for (name, key) in [
            ("return", ComputerKey::Enter),
            ("arrowleft", ComputerKey::Left),
            ("select_word_right", ComputerKey::SelectWordRight),
            ("f12", ComputerKey::F12),
        ] {
            let action: ComputerAction = serde_json::from_value(serde_json::json!({
                "action": "key", "key": name
            }))
            .unwrap();
            assert_eq!(action, ComputerAction::Key { key });
            action.validate(10, 10).unwrap();
        }
        for text in [
            "",
            "line\ncommand",
            "\u{1b}",
            "\u{2028}",
            "review\u{202e}txt",
        ] {
            assert!(
                ComputerAction::Type { text: text.into() }
                    .validate(10, 10)
                    .is_err()
            );
        }
        for text in ["café", "日本語", "مرحبا", "👋"] {
            ComputerAction::Type { text: text.into() }
                .validate(10, 10)
                .unwrap();
        }
        ComputerAction::Type {
            text: "é".repeat(256),
        }
        .validate(10, 10)
        .unwrap();
        assert!(
            ComputerAction::Type {
                text: "é".repeat(257),
            }
            .validate(10, 10)
            .is_err()
        );
        assert!(
            ComputerAction::Type {
                text: "a".repeat(513)
            }
            .validate(10, 10)
            .is_err()
        );
        assert!(
            ComputerAction::Move { x: 10, y: 0 }
                .validate(10, 10)
                .is_err()
        );
        assert!(
            ComputerAction::DoubleClick {
                x: 10,
                y: 0,
                button: ComputerButton::Left
            }
            .validate(10, 10)
            .is_err()
        );
        assert!(
            ComputerAction::Drag {
                from_x: 0,
                from_y: 0,
                to_x: 10,
                to_y: 9,
                button: ComputerButton::Left
            }
            .validate(10, 10)
            .is_err()
        );
        assert!(
            ComputerAction::Click {
                x: 10,
                y: 0,
                button: ComputerButton::Left
            }
            .validate(10, 10)
            .is_err()
        );
        assert!(
            ComputerAction::Scroll {
                x: 0,
                y: 0,
                down: true,
                steps: 9
            }
            .validate(10, 10)
            .is_err()
        );
        assert!(
            ComputerAction::HorizontalScroll {
                x: 0,
                y: 0,
                right: true,
                steps: 9
            }
            .validate(10, 10)
            .is_err()
        );
        assert!(
            ComputerAction::HorizontalScroll {
                x: 10,
                y: 0,
                right: false,
                steps: 1
            }
            .validate(10, 10)
            .is_err()
        );
    }

    #[test]
    fn unicode_text_remains_one_literal_argument_to_the_selected_window() {
        let action = ComputerAction::Type {
            text: "こんにちは --window 0; $HOME".into(),
        };
        action.validate(100, 100).unwrap();
        assert_eq!(
            action.commands(42),
            vec![vec![
                "type",
                "--window",
                "42",
                "--delay",
                "2",
                "--",
                "こんにちは --window 0; $HOME",
            ]]
        );
    }
}
