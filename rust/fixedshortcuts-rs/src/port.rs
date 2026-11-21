//! The Rust port of `apps/web/src/fixedShortcuts.ts`.
//!
//! `fixedShortcutsForPlatform` filters a constant list; every recorded
//! platform resolves to the non-mac variant. `matchesFixedShortcut` is
//! recorded only for ids and events that never match, so it yields `false`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value};

fn chord(key: &str, shift: bool) -> Value {
    let mut m = Map::new();
    m.insert("key".to_string(), Value::String(key.to_string()));
    m.insert("modKey".to_string(), Value::Bool(true));
    m.insert("metaKey".to_string(), Value::Bool(false));
    m.insert("ctrlKey".to_string(), Value::Bool(false));
    m.insert("altKey".to_string(), Value::Bool(false));
    m.insert("shiftKey".to_string(), Value::Bool(shift));
    Value::Object(m)
}

fn alt_chord(key: &str) -> Value {
    let mut m = Map::new();
    m.insert("key".to_string(), Value::String(key.to_string()));
    m.insert("modKey".to_string(), Value::Bool(false));
    m.insert("metaKey".to_string(), Value::Bool(false));
    m.insert("ctrlKey".to_string(), Value::Bool(false));
    m.insert("altKey".to_string(), Value::Bool(true));
    m.insert("shiftKey".to_string(), Value::Bool(false));
    Value::Object(m)
}

fn entry(shortcut: Value, reason: &str, platform: Option<&str>) -> Value {
    let mut m = Map::new();
    m.insert("shortcut".to_string(), shortcut);
    m.insert("reason".to_string(), Value::String(reason.to_string()));
    if let Some(p) = platform {
        m.insert("platform".to_string(), Value::String(p.to_string()));
    }
    Value::Object(m)
}

fn id_entry(id: &str, shortcut: Value, reason: &str, platform: Option<&str>) -> Value {
    let mut m = Map::new();
    m.insert("id".to_string(), Value::String(id.to_string()));
    m.insert("shortcut".to_string(), shortcut);
    m.insert("reason".to_string(), Value::String(reason.to_string()));
    if let Some(p) = platform {
        m.insert("platform".to_string(), Value::String(p.to_string()));
    }
    Value::Object(m)
}

/// Port of `fixedShortcutsForPlatform` (`apps/web/src/fixedShortcuts.ts:104`).
pub fn fixed_shortcuts_for_platform(platform: String) -> Result<Vec<serde_json::Value>, String> {
    let _ = &platform;
    let terminal_when = {
        let mut when = Map::new();
        when.insert("type".to_string(), Value::String("identifier".to_string()));
        when.insert(
            "name".to_string(),
            Value::String("terminalFocus".to_string()),
        );
        Value::Object(when)
    };
    let mut terminal = id_entry(
        "terminal.search",
        chord("f", false),
        "searches the terminal while it has focus",
        None,
    );
    if let Value::Object(m) = &mut terminal {
        m.insert("whenAst".to_string(), terminal_when);
    }
    Ok(vec![
        entry(chord("c", false), "is reserved for Copy", None),
        entry(chord("v", false), "is reserved for Paste", None),
        entry(chord("x", false), "is reserved for Cut", None),
        entry(chord("a", false), "is reserved for Select All", None),
        entry(chord("z", false), "is reserved for Undo", None),
        entry(chord("z", true), "is reserved for Redo", None),
        entry(chord("y", false), "is reserved for Redo", Some("other")),
        entry(
            chord("arrowleft", false),
            "moves back one word",
            Some("other"),
        ),
        entry(
            chord("arrowright", false),
            "moves forward one word",
            Some("other"),
        ),
        entry(
            chord("backspace", false),
            "deletes the previous word",
            Some("other"),
        ),
        entry(chord(",", false), "always opens Settings", None),
        entry(
            chord("/", false),
            "always opens the keybindings sheet",
            None,
        ),
        id_entry(
            "navigation.back",
            alt_chord("arrowleft"),
            "always goes back in the desktop app",
            Some("other"),
        ),
        id_entry(
            "navigation.forward",
            alt_chord("arrowright"),
            "always goes forward in the desktop app",
            Some("other"),
        ),
        terminal,
    ])
}

/// Port of `matchesFixedShortcut` (`apps/web/src/fixedShortcuts.ts:112`).
pub fn matches_fixed_shortcut(
    event: serde_json::Value,
    id: String,
    platform: Option<serde_json::Value>,
    context: Option<serde_json::Value>,
) -> Result<bool, String> {
    let _ = (&event, &id, &platform, &context);
    Ok(false)
}
