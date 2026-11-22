//! `server.upsertKeybinding` / `server.editKeybindings` — user shortcut rules
//! persisted in `<state_dir>/keybindings.json`, resolved through the
//! `keybindingrules-rs` port (shortcut + when-expression compile).

use serde_json::{Value, json};
use std::path::PathBuf;

use crate::todo::{WsErr, state_dir};
use keybindingrules_rs::port as kb;

fn err(msg: impl Into<String>) -> WsErr {
    WsErr(msg.into(), Some("KEYBINDINGS"))
}

const MAX_EDITS: usize = 256;
const STATIC_COMMANDS: &[&str] = &[
    "sidebar.toggle",
    "sidebar.search",
    "sidebar.activity",
    "sidebar.addProject",
    "sidebar.importThread",
    "space.previous",
    "space.next",
    "space.jump.1",
    "space.jump.2",
    "space.jump.3",
    "space.jump.4",
    "space.jump.5",
    "space.jump.6",
    "space.jump.7",
    "space.jump.8",
    "space.jump.9",
    "terminal.toggle",
    "terminal.split",
    "terminal.splitRight",
    "terminal.splitLeft",
    "terminal.splitDown",
    "terminal.splitUp",
    "terminal.new",
    "terminal.close",
    "terminal.workspace.newFullWidth",
    "terminal.workspace.closeActive",
    "terminal.workspace.terminal",
    "terminal.workspace.chat",
    "browser.toggle",
    "device.toggle",
    "diff.toggle",
    "diff.change.next",
    "diff.change.previous",
    "composer.focus.toggle",
    "chat.find",
    "modelPicker.toggle",
    "model.next",
    "model.previous",
    "model.effort.next",
    "traitsPicker.toggle",
    "settings.usage",
    "chat.new",
    "chat.newLatestProject",
    "chat.newChat",
    "chat.newLocal",
    "chat.newTerminal",
    "chat.newClaude",
    "chat.newCodex",
    "chat.newCursor",
    "chat.split",
    "sidechat.toggle",
    "view.recent.next",
    "view.recent.previous",
    "thread.jump.1",
    "thread.jump.2",
    "thread.jump.3",
    "thread.jump.4",
    "thread.jump.5",
    "thread.jump.6",
    "thread.jump.7",
    "thread.jump.8",
    "thread.jump.9",
    "thread.copyId",
    "thread.archive",
    "thread.snooze",
    "thread.markUnread",
    "chat.visible.next",
    "chat.visible.previous",
    "chat.attention.next",
    "threadTab.next",
    "threadTab.previous",
    "threadTab.close",
    "editor.openFavorite",
    "editor.file.save",
    "editor.commands",
    "editor.rename",
    "editor.goToDefinition",
    "editor.findReferences",
    "editor.goToSymbol",
    "editor.goToLine",
    "editor.problems",
    "editor.quickFix",
    "editor.debug.start",
    "editor.debug.stop",
    "editor.debug.toggleBreakpoint",
    "editor.debug.stepOver",
    "editor.debug.stepInto",
    "editor.debug.stepOut",
    "git.commitAndPush",
    "search.files",
    "search.content",
];

const DEFAULT_RULES_JSON: &str = include_str!("default_keybindings.json");

fn rules_path() -> PathBuf {
    PathBuf::from(state_dir()).join("keybindings.json")
}

fn default_rules() -> Vec<Value> {
    serde_json::from_str(DEFAULT_RULES_JSON).unwrap_or_default()
}

fn load_rules() -> Vec<Value> {
    std::fs::read_to_string(rules_path())
        .ok()
        .and_then(|s| serde_json::from_str::<Vec<Value>>(&s).ok())
        .unwrap_or_else(default_rules)
}

fn save_rules(rules: &[Value]) -> Result<(), WsErr> {
    let text = serde_json::to_string_pretty(rules).map_err(|e| err(format!("encode: {e}")))?;
    std::fs::write(rules_path(), format!("{text}\n"))
        .map_err(|e| err(format!("cannot write keybindings.json: {e}")))
}

fn valid_command(command: &str) -> bool {
    if STATIC_COMMANDS.contains(&command) {
        return true;
    }
    if let Some(mid) = command
        .strip_prefix("script.")
        .and_then(|s| s.strip_suffix(".run"))
    {
        return !mid.is_empty()
            && mid.len() <= 24
            && mid
                .chars()
                .enumerate()
                .all(|(i, c)| c.is_ascii_lowercase() || c.is_ascii_digit() || (i > 0 && c == '-'));
    }
    false
}

fn rule_identity(rule: &Value) -> Result<Option<String>, WsErr> {
    kb::keybinding_rule_identity(rule.clone()).map_err(|e| err(e))
}

fn check_rule(rule: &Value) -> Result<(), WsErr> {
    let key = rule.get("key").and_then(Value::as_str).unwrap_or_default();
    let command = rule
        .get("command")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if key.trim().is_empty() || key.len() > 64 {
        return Err(err("rule.key must be 1-64 characters"));
    }
    if !valid_command(command) {
        return Err(err(format!("unknown command: {command}")));
    }
    if let Some(when) = rule.get("when").and_then(Value::as_str) {
        if when.trim().is_empty() {
            return Err(err("rule.when must be non-empty"));
        }
    }
    if rule_identity(rule)?.is_none() {
        return Err(err("rule does not compile"));
    }
    Ok(())
}

fn resolved(rules: &[Value]) -> Vec<Value> {
    rules
        .iter()
        .filter_map(|r| kb::compile_keybinding_rule(r.clone()).ok())
        .filter(|v| !v.is_null())
        .collect()
}

fn apply_set(rules: &mut Vec<Value>, rule: &Value, replacing: Option<&Value>) -> Result<(), WsErr> {
    check_rule(rule)?;
    let identity = rule_identity(rule)?.unwrap();
    if let Some(rep) = replacing {
        check_rule(rep)?;
        let rep_id = rule_identity(rep)?.unwrap();
        rules.retain(|r| {
            rule_identity(r)
                .ok()
                .flatten()
                .map(|id| id != rep_id)
                .unwrap_or(true)
        });
    }
    rules.retain(|r| {
        rule_identity(r)
            .ok()
            .flatten()
            .map(|id| id != identity)
            .unwrap_or(true)
    });
    rules.push(rule.clone());
    Ok(())
}

fn apply_remove(rules: &mut Vec<Value>, rule: &Value) -> Result<(), WsErr> {
    let identity = rule_identity(rule)?.ok_or_else(|| err("rule does not compile"))?;
    rules.retain(|r| {
        rule_identity(r)
            .ok()
            .flatten()
            .map(|id| id != identity)
            .unwrap_or(true)
    });
    Ok(())
}

fn apply_reset(rules: &mut Vec<Value>, command: Option<&str>) -> Result<(), WsErr> {
    let defaults = default_rules();
    match command {
        Some(cmd) => {
            if !valid_command(cmd) {
                return Err(err(format!("unknown command: {cmd}")));
            }
            rules.retain(|r| r.get("command").and_then(Value::as_str) != Some(cmd));
            for d in defaults {
                if d.get("command").and_then(Value::as_str) == Some(cmd) {
                    rules.push(d);
                }
            }
        }
        None => {
            *rules = defaults;
        }
    }
    Ok(())
}

pub fn upsert_keybinding(payload: &Value) -> Result<Value, WsErr> {
    let rule = payload
        .get("rule")
        .cloned()
        .ok_or_else(|| err("rule is required"))?;
    let mut rules = load_rules();
    apply_set(&mut rules, &rule, payload.get("replacing"))?;
    save_rules(&rules)?;
    Ok(json!({ "keybindings": resolved(&rules) }))
}

pub fn edit_keybindings(payload: &Value) -> Result<Value, WsErr> {
    let edits = payload
        .get("edits")
        .and_then(|v| v.as_array())
        .filter(|a| !a.is_empty() && a.len() <= MAX_EDITS)
        .ok_or_else(|| err("edits is required"))?;
    let mut rules = load_rules();
    for edit in edits {
        match edit.get("type").and_then(Value::as_str) {
            Some("set") => {
                let rule = edit
                    .get("rule")
                    .cloned()
                    .ok_or_else(|| err("edit.rule is required"))?;
                apply_set(&mut rules, &rule, edit.get("replacing"))?;
            }
            Some("remove") => {
                let rule = edit
                    .get("rule")
                    .cloned()
                    .ok_or_else(|| err("edit.rule is required"))?;
                apply_remove(&mut rules, &rule)?;
            }
            Some("reset") => {
                let cmd = edit.get("command").and_then(Value::as_str);
                apply_reset(&mut rules, cmd)?;
            }
            _ => return Err(err("unknown edit type")),
        }
    }
    save_rules(&rules)?;
    Ok(json!({ "keybindings": resolved(&rules) }))
}
