//! Native navigation bindings. Provider/editor protocols are not rebound here.
use super::KeyBinding;
use crate::{WorkspaceError, WorkspaceResult};
use std::collections::HashSet;

pub struct NavigationCommand {
    pub id: &'static str,
    pub label: &'static str,
    pub default: &'static str,
}
pub const NAVIGATION_COMMANDS: &[NavigationCommand] = &[
    NavigationCommand {
        id: "navigation.chat",
        label: "Conversation",
        default: "Primary+1",
    },
    NavigationCommand {
        id: "navigation.files",
        label: "Explorer",
        default: "Primary+2",
    },
    NavigationCommand {
        id: "navigation.changes",
        label: "Changes",
        default: "Primary+3",
    },
    NavigationCommand {
        id: "navigation.terminal",
        label: "Terminal",
        default: "Primary+4",
    },
    NavigationCommand {
        id: "navigation.inspector",
        label: "Session inspector",
        default: "Primary+5",
    },
    NavigationCommand {
        id: "navigation.settings",
        label: "Settings",
        default: "Primary+6",
    },
    NavigationCommand {
        id: "navigation.agents",
        label: "Agent providers",
        default: "Primary+7",
    },
    NavigationCommand {
        id: "navigation.remote",
        label: "Remote workspaces",
        default: "Primary+8",
    },
    NavigationCommand {
        id: "navigation.kanban",
        label: "Kanban",
        default: "Primary+9",
    },
    NavigationCommand {
        id: "navigation.device",
        label: "Device viewer",
        default: "Primary+0",
    },
];
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct NavigationKeystroke {
    pub key: String,
    pub alt: bool,
    pub shift: bool,
}
impl NavigationKeystroke {
    pub fn parse(text: &str) -> WorkspaceResult<Self> {
        if text.len() > 128 || text.chars().any(char::is_control) {
            return Err(invalid());
        }
        let lower = text.to_lowercase().replace('-', "+");
        let mut tokens: Vec<_> = lower.split('+').map(str::trim).collect();
        let key = tokens
            .pop()
            .filter(|key| !key.is_empty())
            .ok_or_else(invalid)?
            .to_string();
        let (mut primary, mut alt, mut shift) = (false, false, false);
        for token in tokens {
            let flag = match token {
                "primary" | "ctrl" | "control" | "cmd" | "command" => &mut primary,
                "alt" | "option" => &mut alt,
                "shift" => &mut shift,
                _ => return Err(invalid()),
            };
            if *flag {
                return Err(invalid());
            }
            *flag = true;
        }
        let digit = key.len() == 1 && key.as_bytes()[0].is_ascii_digit();
        let function = key
            .strip_prefix('f')
            .and_then(|key| key.parse::<u8>().ok())
            .is_some_and(|n| (1..=12).contains(&n) && key == format!("f{n}"));
        let alt_letter = alt
            && key.len() == 1
            && key.as_bytes()[0].is_ascii_lowercase()
            && key != "z"
            && key != "t";
        // Space navigation owns Primary+Alt+digits. Ordinary letters, Enter,
        // Escape and text-editing combinations retain their native owners.
        if !primary || !((function || digit) && !alt || alt_letter) {
            return Err(invalid());
        }
        Ok(Self { key, alt, shift })
    }
    pub fn display(&self) -> String {
        format!(
            "Primary+{}{}{}",
            if self.alt { "Alt+" } else { "" },
            if self.shift { "Shift+" } else { "" },
            self.key.to_uppercase()
        )
    }
}
fn invalid() -> WorkspaceError {
    WorkspaceError::Invalid("Use Primary+digit, Primary+F1..F12, or Primary+Alt+letter (except T/Z). Native editing, Space, Zen and system shortcuts are reserved".into())
}
pub fn navigation_binding<'a>(
    bindings: &'a [KeyBinding],
    command: &'a NavigationCommand,
) -> &'a str {
    bindings
        .iter()
        .find(|binding| binding.command == command.id)
        .map_or(command.default, |binding| binding.shortcut.as_str())
}
pub fn validate_navigation_bindings(bindings: &[KeyBinding]) -> WorkspaceResult<()> {
    let mut seen = HashSet::new();
    for command in NAVIGATION_COMMANDS {
        let key = NavigationKeystroke::parse(navigation_binding(bindings, command))?;
        if !seen.insert(key) {
            return Err(WorkspaceError::Invalid(format!(
                "{} conflicts with another effective navigation shortcut, including defaults",
                command.label
            )));
        }
    }
    for binding in bindings
        .iter()
        .filter(|binding| binding.command.starts_with("navigation."))
    {
        if !NAVIGATION_COMMANDS
            .iter()
            .any(|command| command.id == binding.command)
        {
            return Err(WorkspaceError::Invalid(
                "Unknown native navigation command".into(),
            ));
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalizes_aliases_order_case_and_separators() {
        assert_eq!(
            NavigationKeystroke::parse("ctrl-alt-D").unwrap(),
            NavigationKeystroke::parse("Option+Command+d").unwrap()
        );
        assert_eq!(
            NavigationKeystroke::parse("shift+ctrl+2")
                .unwrap()
                .display(),
            "Primary+Shift+2"
        );
    }
    #[test]
    fn rejects_default_collisions_reserved_bindings_and_typographical_commands() {
        assert!(validate_navigation_bindings(&[]).is_ok());
        assert!(
            validate_navigation_bindings(&[KeyBinding {
                command: "navigation.chat".into(),
                shortcut: "cmd+2".into()
            }])
            .is_err()
        );
        assert!(
            validate_navigation_bindings(&[KeyBinding {
                command: "navigation.typo".into(),
                shortcut: "ctrl+0".into()
            }])
            .is_err()
        );
        for key in [
            "ctrl+s",
            "ctrl+alt+1",
            "ctrl+alt+z",
            "ctrl+alt+t",
            "enter",
            "cmd+shift+p",
            "ctrl+f13",
            "ctrl+f01",
            "ctrl+alt+f1",
            "ctrl+alt+shift+f12",
            "ctrl+ctrl+1",
            "ctrl+1\n",
        ] {
            assert!(NavigationKeystroke::parse(key).is_err(), "{key}");
        }
    }
    #[test]
    fn legacy_unimplemented_bindings_are_preserved_not_executed() {
        assert!(
            validate_navigation_bindings(&[KeyBinding {
                command: "conversation.cancel".into(),
                shortcut: "ctrl+escape".into()
            }])
            .is_ok()
        );
    }
}
