//! Native navigation bindings. Provider/editor protocols are not rebound here.
use super::KeyBinding;
use crate::{WorkspaceError, WorkspaceResult};

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

/// Contexts where an application shortcut may run. Global navigation remains
/// active across views. Composer and editor actions only run while that input
/// owner has focus, so those two contexts may reuse a shortcut.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum KeybindingContext {
    Global,
    Composer,
    Editor,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContextualCommand {
    pub id: &'static str,
    pub label: &'static str,
    pub context: KeybindingContext,
    /// `None` means the existing product setting owns the default behavior.
    pub default: Option<&'static str>,
}

pub const CONTEXTUAL_COMMANDS: &[ContextualCommand] = &[
    ContextualCommand {
        id: "composer.send",
        label: "Send message",
        context: KeybindingContext::Composer,
        default: None,
    },
    ContextualCommand {
        id: "model.next",
        label: "Next model",
        context: KeybindingContext::Composer,
        default: Some("Alt+]"),
    },
    ContextualCommand {
        id: "model.previous",
        label: "Previous model",
        context: KeybindingContext::Composer,
        default: Some("Alt+["),
    },
    ContextualCommand {
        id: "editor.save",
        label: "Save current file",
        context: KeybindingContext::Editor,
        default: Some("Primary+S"),
    },
];

/// A normalized shortcut shape shared by settings validation and app input.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct KeybindingStroke {
    pub key: String,
    pub primary: bool,
    pub alt: bool,
    pub shift: bool,
}
impl KeybindingStroke {
    pub fn parse(text: &str) -> WorkspaceResult<Self> {
        if text.len() > 128 || text.chars().any(char::is_control) {
            return Err(invalid_contextual());
        }
        let lower = text.to_lowercase().replace('-', "+");
        let mut tokens: Vec<_> = lower.split('+').map(str::trim).collect();
        let key = tokens
            .pop()
            .filter(|key| valid_key_name(key))
            .ok_or_else(invalid_contextual)?
            .to_string();
        let (mut primary, mut alt, mut shift) = (false, false, false);
        for token in tokens {
            let flag = match token {
                "primary" | "ctrl" | "control" | "cmd" | "command" => &mut primary,
                "alt" | "option" => &mut alt,
                "shift" => &mut shift,
                _ => return Err(invalid_contextual()),
            };
            if *flag {
                return Err(invalid_contextual());
            }
            *flag = true;
        }
        Ok(Self {
            key,
            primary,
            alt,
            shift,
        })
    }

    pub fn display(&self) -> String {
        let mut parts = Vec::new();
        if self.primary {
            parts.push("Primary".to_owned());
        }
        if self.alt {
            parts.push("Alt".to_owned());
        }
        if self.shift {
            parts.push("Shift".to_owned());
        }
        parts.push(match self.key.as_str() {
            "enter" => "Enter".to_owned(),
            "escape" => "Escape".to_owned(),
            "tab" => "Tab".to_owned(),
            "space" => "Space".to_owned(),
            "backspace" => "Backspace".to_owned(),
            "delete" => "Delete".to_owned(),
            "left" => "Left".to_owned(),
            "right" => "Right".to_owned(),
            "up" => "Up".to_owned(),
            "down" => "Down".to_owned(),
            "home" => "Home".to_owned(),
            "end" => "End".to_owned(),
            "pageup" => "PageUp".to_owned(),
            "pagedown" => "PageDown".to_owned(),
            "[" | "]" => self.key.clone(),
            key => key.to_uppercase(),
        });
        parts.join("+")
    }
}

fn valid_key_name(key: &str) -> bool {
    (key.len() == 1
        && key
            .as_bytes()
            .first()
            .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        || matches!(
            key,
            "[" | "]" | "enter" | "escape" | "tab" | "space" | "backspace" | "delete"
                | "left" | "right" | "up" | "down" | "home" | "end" | "pageup" | "pagedown"
        ))
        || key.strip_prefix('f').is_some_and(|number| {
            number
                .parse::<u8>()
                .is_ok_and(|number| (1..=12).contains(&number) && key == format!("f{number}"))
        })
}

fn invalid_contextual() -> WorkspaceError {
    WorkspaceError::Invalid(
        "Use a named key, letter, digit, bracket or F1..F12 with Primary, Alt or Shift modifiers".into(),
    )
}

pub fn contextual_binding<'a>(bindings: &'a [KeyBinding], command: &str) -> Option<String> {
    let binding = bindings.iter().find(|binding| binding.command == command);
    binding
        .map(|binding| binding.shortcut.clone())
        .or_else(|| {
            CONTEXTUAL_COMMANDS
                .iter()
                .find(|candidate| candidate.id == command)
                .and_then(|candidate| candidate.default.map(str::to_owned))
        })
}

pub fn has_contextual_override(bindings: &[KeyBinding], command: &str) -> bool {
    bindings.iter().any(|binding| binding.command == command)
}

pub fn parse_contextual_shortcut(command_id: &str, text: &str) -> WorkspaceResult<String> {
    let command = CONTEXTUAL_COMMANDS
        .iter()
        .find(|command| command.id == command_id)
        .ok_or_else(|| {
            WorkspaceError::Invalid("Unknown context-aware keybinding command".into())
        })?;
    let key = KeybindingStroke::parse(text)?;
    if !contextual_shortcut_is_allowed(command, &key) {
        return Err(WorkspaceError::Invalid(format!(
            "{} is not valid in the {:?} context",
            command.label, command.context
        )));
    }
    Ok(key.display())
}

pub fn contextual_command_for_key(
    bindings: &[KeyBinding],
    context: KeybindingContext,
    stroke: &KeybindingStroke,
) -> Option<&'static str> {
    CONTEXTUAL_COMMANDS
        .iter()
        .filter(|command| command.context == context)
        .find(|command| {
            contextual_binding(bindings, command.id)
                .and_then(|shortcut| KeybindingStroke::parse(&shortcut).ok())
                .is_some_and(|candidate| &candidate == stroke)
        })
        .map(|command| command.id)
}

fn contextual_shortcut_is_allowed(command: &ContextualCommand, key: &KeybindingStroke) -> bool {
    let safe_primary_alt_letter = key.primary
        && key.alt
        && !key.shift
        && key.key.len() == 1
        && key.key.as_bytes()[0].is_ascii_lowercase()
        && !matches!(key.key.as_str(), "t" | "z");
    let primary_function = key.primary
        && !key.alt
        && !key.shift
        && key.key.starts_with('f');
    match command.id {
        "composer.send" => key.key == "enter" && !key.shift,
        "model.next" => {
            (key.alt && !key.primary && !key.shift && key.key == "]")
                || safe_primary_alt_letter
                || primary_function
        }
        "model.previous" => {
            (key.alt && !key.primary && !key.shift && key.key == "[")
                || safe_primary_alt_letter
                || primary_function
        }
        "editor.save" => {
            (key.primary && !key.alt && !key.shift && key.key == "s")
                || (key.key == "enter" && !key.shift && (key.primary ^ key.alt))
                || safe_primary_alt_letter
                || primary_function
        }
        _ => false,
    }
}
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
    let mut effective = Vec::new();
    for command in NAVIGATION_COMMANDS {
        let key = NavigationKeystroke::parse(navigation_binding(bindings, command))?;
        effective.push((
            KeybindingContext::Global,
            KeybindingStroke {
                key: key.key,
                primary: true,
                alt: key.alt,
                shift: key.shift,
            },
            command.label,
        ));
    }
    for command in CONTEXTUAL_COMMANDS {
        let Some(shortcut) = contextual_binding(bindings, command.id) else {
            continue;
        };
        let display = parse_contextual_shortcut(command.id, &shortcut)?;
        let key = KeybindingStroke::parse(&display)?;
        effective.push((command.context, key, command.label));
    }
    for binding in bindings {
        if binding.command.starts_with("navigation.")
            && !NAVIGATION_COMMANDS
                .iter()
                .any(|command| command.id == binding.command)
        {
            return Err(WorkspaceError::Invalid(
                "Unknown native navigation command".into(),
            ));
        }
        if ["composer.", "editor.", "model."]
            .iter()
            .any(|namespace| binding.command.starts_with(namespace))
            && !CONTEXTUAL_COMMANDS
                .iter()
                .any(|command| command.id == binding.command)
        {
            return Err(WorkspaceError::Invalid(
                "Unknown context-aware keybinding command".into(),
            ));
        }
    }
    for right in 0..effective.len() {
        for left in 0..right {
            if effective[left].1 == effective[right].1
                && contexts_overlap(effective[left].0, effective[right].0)
            {
                return Err(WorkspaceError::Invalid(format!(
                    "{} conflicts with {} in an active input context",
                    effective[right].2, effective[left].2
                )));
            }
        }
    }
    Ok(())
}

fn contexts_overlap(left: KeybindingContext, right: KeybindingContext) -> bool {
    left == right || left == KeybindingContext::Global || right == KeybindingContext::Global
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

    #[test]
    fn contextual_commands_resolve_only_in_their_active_input_scope() {
        let bindings = [
            KeyBinding {
                command: "composer.send".into(),
                shortcut: "Alt+Enter".into(),
            },
            KeyBinding {
                command: "editor.save".into(),
                shortcut: "Alt+Enter".into(),
            },
        ];
        assert!(validate_navigation_bindings(&bindings).is_ok());
        let key = KeybindingStroke::parse("Alt+Enter").unwrap();
        assert_eq!(
            contextual_command_for_key(&bindings, KeybindingContext::Composer, &key),
            Some("composer.send")
        );
        assert_eq!(
            contextual_command_for_key(&bindings, KeybindingContext::Editor, &key),
            Some("editor.save")
        );
        assert_eq!(
            contextual_command_for_key(&[], KeybindingContext::Composer, &key),
            None
        );
    }

    #[test]
    fn contextual_defaults_and_normalized_conflicts_are_checked() {
        let default = contextual_binding(&[], "model.next").unwrap();
        assert_eq!(KeybindingStroke::parse(&default).unwrap().display(), "Alt+]");
        assert!(validate_navigation_bindings(&[
            KeyBinding {
                command: "model.next".into(),
                shortcut: "ctrl-alt-a".into(),
            },
            KeyBinding {
                command: "model.previous".into(),
                shortcut: "Primary+Option+A".into(),
            },
        ])
        .is_err());
        assert!(validate_navigation_bindings(&[
            KeyBinding {
                command: "composer.send".into(),
                shortcut: "ctrl+1".into(),
            },
        ])
        .is_err());
        assert!(validate_navigation_bindings(&[
            KeyBinding {
                command: "editor.save".into(),
                shortcut: "enter".into(),
            },
        ])
        .is_err());
        assert!(validate_navigation_bindings(&[
            KeyBinding {
                command: "model.typo".into(),
                shortcut: "Primary+Alt+A".into(),
            },
        ])
        .is_err());
    }
}
