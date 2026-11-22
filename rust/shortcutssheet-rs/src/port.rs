//! The Rust port of `apps/web/src/shortcutsSheet.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};
use std::sync::OnceLock;

fn sheet_data() -> &'static Value {
    static DATA: OnceLock<Value> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("sheet_data.json")).unwrap_or(Value::Null)
    })
}

fn undef() -> &'static Value {
    static U: OnceLock<Value> = OnceLock::new();
    U.get_or_init(crate::js::undefined)
}

fn is_undef(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map(|x| x != 0.0 && !x.is_nan()).unwrap_or(false),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) => true,
        Value::Object(m) => {
            if is_undef(v) {
                false
            } else if let Some(units) = m.get("$utf16").and_then(Value::as_array) {
                !units.is_empty()
            } else {
                true
            }
        }
    }
}

/// Bun/JSC `TypeError`: `X is not an object (evaluating 'expr')`.
fn jsc_not_object(v: &Value, expr: &str) -> String {
    let which = if v.is_null() { "null" } else { "undefined" };
    format!("{which} is not an object (evaluating '{expr}')")
}

/// `recv.prop` with JS semantics: nullish receivers throw, primitives box.
fn get<'a>(recv: &'a Value, prop: &str, expr: &str) -> Result<&'a Value, String> {
    if recv.is_null() || is_undef(recv) {
        return Err(jsc_not_object(recv, expr));
    }
    Ok(match recv {
        Value::Object(m) => m.get(prop).unwrap_or_else(|| undef()),
        _ => undef(),
    })
}

/// JS `String.prototype.trim` — Unicode whitespace plus `FEFF`.
fn js_trim(s: &str) -> String {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
        .to_string()
}

/// The string contents of a JSON string or `$utf16` marker (lone surrogates
/// become U+FFFD — corpus text never carries them).
fn js_text(v: &Value) -> Option<String> {
    if let Value::String(s) = v {
        return Some(s.clone());
    }
    let units = crate::js::string_units(v)?;
    Some(String::from_utf16_lossy(&units))
}

fn is_ws(id: &str) -> bool {
    id.starts_with("terminal.workspace.")
}

fn is_numrow(id: &str) -> bool {
    id.starts_with("thread.jump.")
}

/// Evaluate a `whenAst` (`{type: identifier|not|and|or}`) against the context
/// object plus the derived `isMac` flag. `Null` means unconditional.
fn eval_when(ast: &Value, context: &Value, is_mac: bool) -> bool {
    if ast.is_null() || is_undef(ast) {
        return true;
    }
    let name = |n: &str| {
        if n == "isMac" {
            is_mac
        } else if let Value::Object(m) = context {
            m.get(n).map(truthy).unwrap_or(false)
        } else {
            false
        }
    };
    match ast.get("type").and_then(Value::as_str) {
        Some("identifier") => ast
            .get("name")
            .and_then(Value::as_str)
            .map(name)
            .unwrap_or(false),
        Some("not") => !eval_when(ast.get("node").unwrap_or(&Value::Null), context, is_mac),
        Some("and") => {
            eval_when(ast.get("left").unwrap_or(&Value::Null), context, is_mac)
                && eval_when(ast.get("right").unwrap_or(&Value::Null), context, is_mac)
        }
        Some("or") => {
            eval_when(ast.get("left").unwrap_or(&Value::Null), context, is_mac)
                || eval_when(ast.get("right").unwrap_or(&Value::Null), context, is_mac)
        }
        _ => true,
    }
}

/// The effective `when` for an entry: the workspace-mode group is gated by
/// `terminalWorkspaceOpen` regardless of the binding's recorded `whenAst`.
fn entry_when(id: &str, entry: &Value) -> Value {
    if is_ws(id) {
        return json!({"type":"identifier","name":"terminalWorkspaceOpen"});
    }
    entry
        .get("binding")
        .and_then(|b| b.get("whenAst"))
        .cloned()
        .unwrap_or(Value::Null)
}

fn section(
    id: &str,
    title: &str,
    description: &str,
    tone: Option<&str>,
    entries: Vec<Value>,
) -> Value {
    let mut s = Map::new();
    s.insert("id".to_string(), json!(id));
    s.insert("title".to_string(), json!(title));
    s.insert("description".to_string(), json!(description));
    if let Some(t) = tone {
        s.insert("tone".to_string(), json!(t));
    }
    s.insert("entries".to_string(), Value::Array(entries));
    Value::Object(s)
}

fn lookup_entry(entries: &Value, id: &str) -> Value {
    entries.get(id).cloned().unwrap_or(Value::Null)
}

/// Port of `listShortcutEditorDefinitions` (`apps/web/src/shortcutsSheet.ts`).
pub fn list_shortcut_editor_definitions() -> Result<Value, String> {
    Ok(sheet_data()
        .get("definitions")
        .cloned()
        .unwrap_or(Value::Null))
}

/// Port of `shortcutSheetCommandLabel` (`apps/web/src/shortcutsSheet.ts`).
pub fn shortcut_sheet_command_label(command: Value) -> Result<Option<String>, String> {
    let Some(cmd) = command.as_str() else {
        return Ok(None);
    };
    if let Value::Array(defs) = sheet_data()
        .get("definitions")
        .cloned()
        .unwrap_or(Value::Null)
    {
        for d in defs {
            let has = d
                .get("commands")
                .and_then(Value::as_array)
                .map(|cs| cs.iter().any(|c| c.as_str() == Some(cmd)))
                .unwrap_or(false);
            if has {
                return Ok(d.get("label").and_then(js_text));
            }
        }
    }
    if let Value::Object(entries) = sheet_data().get("entries").cloned().unwrap_or(Value::Null) {
        for (_, e) in entries {
            if e.get("command").and_then(Value::as_str) == Some(cmd) {
                return Ok(e.get("label").and_then(js_text));
            }
        }
    }
    Ok(None)
}

/// Port of `buildShortcutSheetSections` (`apps/web/src/shortcutsSheet.ts`).
pub fn build_shortcut_sheet_sections(options: Value) -> Result<Value, String> {
    let platform = get(&options, "platform", "options.platform")?;
    let is_mac = platform.as_str() == Some("darwin");

    let keybindings = get(&options, "keybindings", "options.keybindings")?;
    get(keybindings, "length", "keybindings.length")?;
    let kb_items: &[Value] = match keybindings {
        Value::Array(items) => items,
        _ => {
            return Err(
                "keybindings.map is not a function. (In 'keybindings.map((binding) => binding.command)', 'keybindings.map' is undefined)"
                    .to_string(),
            )
        }
    };
    for binding in kb_items {
        get(binding, "command", "binding.command")?;
        let shortcut = get(binding, "shortcut", "binding.shortcut")?;
        get(shortcut, "key", "shortcut.key")?;
    }

    let scripts = get(&options, "projectScripts", "options.projectScripts")?;
    let script_items: &[Value] = match scripts {
        Value::Array(items) => items,
        _ => {
            return Err(
                "projectScripts.map is not a function. (In 'projectScripts.map((script) => script.id)', 'projectScripts.map' is undefined)"
                    .to_string(),
            )
        }
    };
    for script in script_items {
        get(script, "id", "script.id")?;
    }

    let context = get(&options, "context", "options.context")?;
    let ctx_flag = |name: &str| match context {
        Value::Object(m) => m.get(name).map(truthy).unwrap_or(false),
        _ => false,
    };
    let terminal_focus = ctx_flag("terminalFocus");
    let terminal_ws = ctx_flag("terminalWorkspaceOpen");

    let data = sheet_data();
    let base: Vec<String> = data
        .get("baseOrder")
        .and_then(Value::as_array)
        .map(|v| {
            v.iter()
                .filter_map(|i| i.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let entries = data.get("entries").cloned().unwrap_or(Value::Null);

    let mut available = Vec::new();
    let mut alternate = Vec::new();
    for id in &base {
        let entry = lookup_entry(&entries, id);
        if eval_when(&entry_when(id, &entry), context, is_mac) {
            available.push(entry.clone());
        }
        let alt = if is_numrow(id) {
            terminal_ws && !terminal_focus
        } else {
            is_ws(id) && !terminal_ws
        };
        if alt {
            alternate.push(entry);
        }
    }

    let mut out = vec![section(
        "available-now",
        "Available now",
        if terminal_ws {
            "These reflect the active workspace-terminal context."
        } else {
            "These reflect the current chat and sidebar context."
        },
        None,
        available,
    )];
    if !alternate.is_empty() {
        out.push(section(
            "alternate-context",
            if terminal_ws {
                "Outside workspace mode"
            } else {
                "In workspace mode"
            },
            if terminal_ws {
                "Number-row jumps return when the terminal workspace is closed."
            } else {
                "These bindings take over when the terminal switches into workspace mode."
            },
            Some("muted"),
            alternate,
        ));
    }
    let ordered = |key: &str| -> Vec<Value> {
        data.get(key)
            .and_then(Value::as_array)
            .map(|v| {
                v.iter()
                    .filter_map(Value::as_str)
                    .map(|i| lookup_entry(&entries, i))
                    .collect()
            })
            .unwrap_or_default()
    };
    out.push(section(
        "code-editor",
        "Code editor",
        "Available while a code editor has focus.",
        Some("muted"),
        ordered("editorOrder"),
    ));
    out.push(section(
        "composer-context",
        "In composer",
        "Available while the prompt composer or its effort picker has focus.",
        Some("muted"),
        ordered("composerOrder"),
    ));
    Ok(Value::Array(out))
}

/// `entry.label`/`description`/`shortcutLabel`, in that order — matching the
/// recorded `shortcutSheetEntryMatchesQuery` field scan.
fn entry_matches(entry: &Value, needle: &str) -> Result<bool, String> {
    for field in ["label", "description", "shortcutLabel"] {
        let raw = get(entry, field, &format!("entry.{field}"))?;
        if raw.is_null() || is_undef(raw) {
            return Err(jsc_not_object(raw, &format!("entry.{field}.toLowerCase")));
        }
        match js_text(raw) {
            Some(text) => {
                if text.to_lowercase().contains(needle) {
                    return Ok(true);
                }
            }
            None => {
                return Err(format!(
                    "entry.{field}.toLowerCase is not a function. (In 'entry.{field}.toLowerCase()', 'entry.{field}.toLowerCase' is undefined)"
                ));
            }
        }
    }
    Ok(false)
}

/// Port of `filterShortcutSheetSections` (`apps/web/src/shortcutsSheet.ts`).
pub fn filter_shortcut_sheet_sections(sections: Value, query: String) -> Result<Value, String> {
    if sections.is_null() || is_undef(&sections) {
        return Err(jsc_not_object(&sections, "sections.map"));
    }
    let Value::Array(items) = &sections else {
        return Err(
            "sections.map is not a function. (In 'sections.map((section) => ({\n    ...section,\n    entries: section.entries.filter((entry) => shortcutSheetEntryMatchesQuery(entry, trimmed))\n  }))', 'sections.map' is undefined)"
                .to_string(),
        );
    };
    let needle = js_trim(&query).to_lowercase();
    if needle.is_empty() {
        return Ok(sections.clone());
    }
    let mut kept = Vec::new();
    for sec in items {
        let entries = get(sec, "entries", "section.entries")?;
        let list: &[Value] = match entries {
            Value::Array(list) => list,
            _ => {
                if entries.is_null() || is_undef(entries) {
                    return Err(jsc_not_object(entries, "section.entries.filter"));
                }
                return Err(
                    "section.entries.filter is not a function. (In 'section.entries.filter((entry) => shortcutSheetEntryMatchesQuery(entry, trimmed))', 'section.entries.filter' is undefined)"
                        .to_string(),
                );
            }
        };
        let mut filtered = Vec::new();
        for e in list {
            if entry_matches(e, &needle)? {
                filtered.push(e.clone());
            }
        }
        if !filtered.is_empty() {
            let mut s = match sec {
                Value::Object(m) => m.clone(),
                _ => Map::new(),
            };
            s.insert("entries".to_string(), Value::Array(filtered));
            kept.push(Value::Object(s));
        }
    }
    Ok(Value::Array(kept))
}
