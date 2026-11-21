//! The Rust port of `apps/web/src/keybindingEditor.ts`.
//!
//! JSC-faithful: member access on null/undefined and missing methods reproduce the
//! recorded JavaScriptCore TypeError strings; everything else follows the source.

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

fn undef() -> Value {
    crate::js::undefined()
}

fn truthy(v: &Value) -> bool {
    if is_undefined(v) {
        return false;
    }
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        _ => true,
    }
}

/// `String(v)` for the shapes this port meets.
fn js_str(v: &Value) -> String {
    if is_undefined(v) {
        return "undefined".to_string();
    }
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => match n.as_f64() {
            Some(x) if x.is_nan() => "NaN".to_string(),
            Some(x) if x == f64::INFINITY => "Infinity".to_string(),
            Some(x) if x == f64::NEG_INFINITY => "-Infinity".to_string(),
            Some(x) => {
                if x == 0.0 {
                    "0".to_string()
                } else if x.fract() == 0.0 && x.abs() < 1e21 {
                    format!("{x:.0}")
                } else {
                    x.to_string()
                }
            }
            None => "null".to_string(),
        },
        Value::String(s) => s.clone(),
        Value::Array(a) => a.iter().map(js_str).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

/// Array `.join` element coercion: null/undefined become "".
fn join_str(v: &Value) -> String {
    if v.is_null() || is_undefined(v) {
        String::new()
    } else {
        js_str(v)
    }
}

fn js_strict_eq(a: &Value, b: &Value) -> bool {
    if is_undefined(a) && is_undefined(b) {
        return true;
    }
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            x.as_f64() == y.as_f64() && !x.as_f64().is_some_and(|n| n.is_nan())
        }
        (Value::Number(_), _) | (_, Value::Number(_)) => false,
        _ => a == b,
    }
}

/// `v.key`: throws on null/undefined receivers, yields undefined on primitives.
fn member(v: &Value, key: &str, expr: &str) -> Result<Value, String> {
    match v {
        Value::Null => Err(format!("null is not an object (evaluating '{expr}')")),
        _ if is_undefined(v) => Err(format!("undefined is not an object (evaluating '{expr}')")),
        Value::Object(o) => Ok(o.get(key).cloned().unwrap_or_else(undef)),
        _ => Ok(undef()),
    }
}

/// `{recv}.{method}(...)` where the method value is missing/not callable.
fn not_a_fn(fn_expr: &str, call_src: &str) -> String {
    format!("{fn_expr} is not a function. (In '{call_src}', '{fn_expr}' is undefined)")
}

/// `{recv}.filter(...)` on the already-read receiver value.
fn call_filter(v: &Value, fn_expr: &str, call_src: &str) -> Result<Vec<Value>, String> {
    match v {
        Value::Null => Err(format!("null is not an object (evaluating '{fn_expr}')")),
        _ if is_undefined(v) => Err(format!(
            "undefined is not an object (evaluating '{fn_expr}')"
        )),
        Value::Array(a) => Ok(a.clone()),
        _ => Err(not_a_fn(fn_expr, call_src)),
    }
}

/// `{recv}.includes(item)` on the already-read receiver value.
fn call_includes(v: &Value, item: &Value, fn_expr: &str, call_src: &str) -> Result<bool, String> {
    match v {
        Value::Null => Err(format!("null is not an object (evaluating '{fn_expr}')")),
        _ if is_undefined(v) => Err(format!(
            "undefined is not an object (evaluating '{fn_expr}')"
        )),
        Value::Array(a) => Ok(a.iter().any(|x| js_strict_eq(x, item))),
        Value::String(s) => Ok(s.contains(&js_str(item))),
        _ => Err(not_a_fn(fn_expr, call_src)),
    }
}

/// `v.toLowerCase()` on the already-read value; strings only.
fn call_to_lowercase(v: &Value, fn_expr: &str, call_src: &str) -> Result<String, String> {
    match v {
        Value::Null => Err(format!("null is not an object (evaluating '{fn_expr}')")),
        _ if is_undefined(v) => Err(format!(
            "undefined is not an object (evaluating '{fn_expr}')"
        )),
        Value::String(s) => Ok(s.to_lowercase()),
        _ => Err(not_a_fn(fn_expr, call_src)),
    }
}

/// `isMacPlatform`: /mac|darwin|iphone|ipad|ipod/i.test(platform).
fn is_mac(platform: &Value) -> bool {
    let p = js_str(platform).to_lowercase();
    p.contains("mac")
        || p.contains("darwin")
        || p.contains("iphone")
        || p.contains("ipad")
        || p.contains("ipod")
}

// — editor definition tables (from shortcutsSheet) —

const NATIVE_EDITOR_ON: bool = true;

struct Def {
    commands: &'static [&'static str],
    label: &'static str,
    description: &'static str,
}

const SIDEBAR_TOGGLE: Def = Def {
    commands: &["sidebar.toggle"],
    label: "Toggle sidebar",
    description: "Collapse or reveal the sidebar shell.",
};

fn space_jump_defs() -> Vec<Def> {
    (1..=9)
        .map(|i| Def {
            commands: match i {
                1 => &["space.jump.1"],
                2 => &["space.jump.2"],
                3 => &["space.jump.3"],
                4 => &["space.jump.4"],
                5 => &["space.jump.5"],
                6 => &["space.jump.6"],
                7 => &["space.jump.7"],
                8 => &["space.jump.8"],
                _ => &["space.jump.9"],
            },
            label: if i == 1 {
                "Jump to Void"
            } else {
                match i {
                    2 => "Jump to space 2",
                    3 => "Jump to space 3",
                    4 => "Jump to space 4",
                    5 => "Jump to space 5",
                    6 => "Jump to space 6",
                    7 => "Jump to space 7",
                    8 => "Jump to space 8",
                    _ => "Jump to space 9",
                }
            },
            description: if i == 1 {
                "Switch straight to the Void tab of the space switcher."
            } else {
                "Switch straight to this tab of the space switcher."
            },
        })
        .collect()
}

fn thread_jump_defs() -> Vec<Def> {
    (1..=9)
        .map(|i| Def {
            commands: match i {
                1 => &["thread.jump.1"],
                2 => &["thread.jump.2"],
                3 => &["thread.jump.3"],
                4 => &["thread.jump.4"],
                5 => &["thread.jump.5"],
                6 => &["thread.jump.6"],
                7 => &["thread.jump.7"],
                8 => &["thread.jump.8"],
                _ => &["thread.jump.9"],
            },
            label: match i {
                1 => "Jump to visible thread 1",
                2 => "Jump to visible thread 2",
                3 => "Jump to visible thread 3",
                4 => "Jump to visible thread 4",
                5 => "Jump to visible thread 5",
                6 => "Jump to visible thread 6",
                7 => "Jump to visible thread 7",
                8 => "Jump to visible thread 8",
                _ => "Jump to visible thread 9",
            },
            description: "Focus a visible thread directly from the sidebar number row.",
        })
        .collect()
}

fn available_now_defs() -> Vec<Def> {
    let mut defs = vec![
        Def {
            commands: &["sidebar.addProject"],
            label: "Add project",
            description: "Open the Create project dialog to import a local folder.",
        },
        Def {
            commands: &["sidebar.search"],
            label: "Search projects and threads",
            description: "Open the sidebar search palette from anywhere in the app.",
        },
        Def {
            commands: &["sidebar.activity"],
            label: "Toggle Activity",
            description: "Show or hide running tasks, completed work, and items that need attention.",
        },
        Def {
            commands: &["sidebar.importThread"],
            label: "Import thread",
            description: "Bring an existing conversation into the current workspace.",
        },
        Def {
            commands: &["space.previous"],
            label: "Previous space",
            description: "Switch to the previous project space and restore its last working context.",
        },
        Def {
            commands: &["space.next"],
            label: "Next space",
            description: "Switch to the next project space and restore its last working context.",
        },
    ];
    defs.extend(space_jump_defs());
    defs.extend([
        Def {
            commands: &["chat.new"],
            label: "New thread",
            description: "Start a fresh thread in the current project, or the most recent one.",
        },
        Def {
            commands: &["chat.newLatestProject"],
            label: "New thread in latest project",
            description: "Jump back into the most recently used project with a new thread.",
        },
        Def {
            commands: &["chat.newChat", "chat.newLocal"],
            label: "New chat",
            description: "Open the empty chat landing view.",
        },
        Def {
            commands: &["chat.newTerminal"],
            label: "New terminal thread",
            description: "Create a thread that opens directly into terminal mode.",
        },
        Def {
            commands: &["chat.newClaude"],
            label: "New Claude thread",
            description: "Start a fresh thread with Claude selected.",
        },
        Def {
            commands: &["chat.newCodex"],
            label: "New Codex thread",
            description: "Start a fresh thread with Codex selected.",
        },
        Def {
            commands: &["chat.newCursor"],
            label: "New Cursor thread",
            description: "Start a fresh thread with Cursor selected.",
        },
        Def {
            commands: &["chat.split"],
            label: "Split chat",
            description: "Open the current conversation in a second pane.",
        },
        Def {
            commands: &["view.recent.previous"],
            label: "Previous recent view",
            description: "Cycle backward through recently opened primary views.",
        },
        Def {
            commands: &["view.recent.next"],
            label: "Next recent view",
            description: "Cycle forward through recently opened primary views.",
        },
        Def {
            commands: &["modelPicker.toggle"],
            label: "Model picker",
            description: "Open the composer provider and model picker.",
        },
        Def {
            commands: &["model.next"],
            label: "Next model",
            description: "Cycle to the next model for the active provider (favorites first, then remaining models).",
        },
        Def {
            commands: &["model.previous"],
            label: "Previous model",
            description: "Cycle to the previous model for the active provider (favorites first, then remaining models).",
        },
        Def {
            commands: &["model.effort.next"],
            label: "Next model effort",
            description: "Cycle the model's available effort levels and briefly show the selected level.",
        },
        Def {
            commands: &["traitsPicker.toggle"],
            label: "Reasoning picker",
            description: "Open the composer reasoning and trait controls.",
        },
        Def {
            commands: &["settings.usage"],
            label: "Open usage settings",
            description: "Open Settings → Usage for provider quota and token totals.",
        },
        Def {
            commands: &["composer.focus.toggle"],
            label: "Focus composer",
            description: "Focus or blur the chat prompt composer.",
        },
        Def {
            commands: &["chat.find"],
            label: "Find in thread",
            description: "Search the current transcript and jump to each matching message.",
        },
        Def {
            commands: &["search.files"],
            label: "Search files",
            description: "Open the workspace file-name search palette.",
        },
        Def {
            commands: &["search.content"],
            label: "Search snippets",
            description: "Open the workspace content search palette across file snippets.",
        },
        Def {
            commands: &["terminal.toggle"],
            label: "Toggle terminal",
            description: "Show or hide the terminal surface for the active thread.",
        },
        Def {
            commands: &["terminal.new"],
            label: "Focus terminal",
            description: "Open or focus the terminal panel for this thread.",
        },
        Def {
            commands: &["terminal.close"],
            label: "Close terminal",
            description: "Close the terminal session for this thread.",
        },
        Def {
            commands: &["diff.toggle"],
            label: "Toggle diff",
            description: "Open or close the working tree diff panel.",
        },
        Def {
            commands: &["diff.change.next"],
            label: "Next change",
            description: "Jump the diff viewport to the next changed file.",
        },
        Def {
            commands: &["diff.change.previous"],
            label: "Previous change",
            description: "Jump the diff viewport to the previous changed file.",
        },
        Def {
            commands: &["sidechat.toggle"],
            label: "Toggle side chat",
            description: "Open or hide a side chat beside the main conversation.",
        },
        Def {
            commands: &["browser.toggle"],
            label: "Toggle browser",
            description: "Reveal the built-in browser panel for the active thread.",
        },
        Def {
            commands: &["device.toggle"],
            label: "Toggle iOS Simulator",
            description: "Reveal the iOS Simulator panel for the active thread. macOS servers only.",
        },
        Def {
            commands: &["thread.copyId"],
            label: "Copy thread ID",
            description: "Copy the active thread's ID to the clipboard.",
        },
        Def {
            commands: &["thread.archive"],
            label: "Archive thread",
            description: "Archive the active chat, with the same confirmation and undo as the thread menu.",
        },
        Def {
            commands: &["thread.snooze"],
            label: "Snooze thread",
            description: "Open the snooze date and time picker for the active chat.",
        },
        Def {
            commands: &["thread.markUnread"],
            label: "Mark thread unread",
            description: "Mark the active chat unread and restore its dismissed notification.",
        },
        Def {
            commands: &["chat.visible.previous"],
            label: "Previous visible thread",
            description: "Cycle to the previous thread that is currently visible in the sidebar.",
        },
        Def {
            commands: &["chat.visible.next"],
            label: "Next visible thread",
            description: "Cycle to the next thread that is currently visible in the sidebar.",
        },
        Def {
            commands: &["chat.attention.next"],
            label: "Next chat needing attention",
            description: "Open the most urgent other chat: one waiting on you first, then unseen work, as Activity's Priority ranks them.",
        },
        Def {
            commands: &["threadTab.previous"],
            label: "Previous tab",
            description: "Switch to the open thread tab on the left of the active one.",
        },
        Def {
            commands: &["threadTab.next"],
            label: "Next tab",
            description: "Switch to the open thread tab on the right of the active one.",
        },
        Def {
            commands: &["threadTab.close"],
            label: "Close tab",
            description: "Close the active open-thread tab, like a browser tab.",
        },
        Def {
            commands: &["editor.openFavorite"],
            label: "Open in favorite editor",
            description: "Send the current thread or workspace target to your preferred editor.",
        },
        Def {
            commands: &["editor.file.save"],
            label: "Save file",
            description: "Write the focused editor's unsaved changes back to disk.",
        },
        Def {
            commands: &["git.commitAndPush"],
            label: "Commit and push",
            description: "Commit pending changes and push the active thread's repo.",
        },
    ]);
    defs
}

const WORKSPACE_DEFS: &[Def] = &[
    Def {
        commands: &["terminal.workspace.newFullWidth"],
        label: "Open full-width terminal workspace",
        description: "Expand the active thread into the workspace terminal layout.",
    },
    Def {
        commands: &["terminal.workspace.terminal"],
        label: "Focus terminal tab",
        description: "Switch the workspace to the terminal tab.",
    },
    Def {
        commands: &["terminal.workspace.chat"],
        label: "Focus chat tab",
        description: "Switch the workspace back to the chat tab.",
    },
    Def {
        commands: &["terminal.workspace.closeActive"],
        label: "Close active workspace panel",
        description: "Close the currently focused workspace panel or tab.",
    },
];

const CODE_EDITOR_DEFS: &[Def] = &[
    Def {
        commands: &["editor.commands"],
        label: "Editor commands",
        description: "Open the editor command picker for the focused code editor.",
    },
    Def {
        commands: &["editor.rename"],
        label: "Rename symbol",
        description: "Rename the symbol under the cursor across the workspace.",
    },
    Def {
        commands: &["editor.goToDefinition"],
        label: "Go to definition",
        description: "Jump to the definition of the symbol under the cursor.",
    },
    Def {
        commands: &["editor.findReferences"],
        label: "Find references",
        description: "List references to the symbol under the cursor.",
    },
    Def {
        commands: &["editor.goToSymbol"],
        label: "Go to symbol in file",
        description: "Jump to a symbol in the open file.",
    },
    Def {
        commands: &["editor.goToLine"],
        label: "Go to line",
        description: "Jump to a line, or line:column, in the open file.",
    },
    Def {
        commands: &["editor.problems"],
        label: "Show problems",
        description: "List diagnostics for the open file.",
    },
    Def {
        commands: &["editor.quickFix"],
        label: "Quick fix",
        description: "Show quick fixes and refactorings for the code at the cursor.",
    },
    Def {
        commands: &["editor.debug.start"],
        label: "Start or continue debugging",
        description: "Launch the debugger for the open file, or continue a paused session.",
    },
    Def {
        commands: &["editor.debug.stop"],
        label: "Stop debugging",
        description: "Stop the current debug session.",
    },
    Def {
        commands: &["editor.debug.toggleBreakpoint"],
        label: "Toggle breakpoint",
        description: "Add or remove a breakpoint on the cursor line.",
    },
    Def {
        commands: &["editor.debug.stepOver"],
        label: "Step over",
        description: "Run the next statement without entering calls.",
    },
    Def {
        commands: &["editor.debug.stepInto"],
        label: "Step into",
        description: "Enter the next call.",
    },
    Def {
        commands: &["editor.debug.stepOut"],
        label: "Step out",
        description: "Run until the current call returns.",
    },
];

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

const SPACE_JUMP_COMMANDS: &[&str] = &[
    "space.jump.1",
    "space.jump.2",
    "space.jump.3",
    "space.jump.4",
    "space.jump.5",
    "space.jump.6",
    "space.jump.7",
    "space.jump.8",
    "space.jump.9",
];
const THREAD_JUMP_COMMANDS: &[&str] = &[
    "thread.jump.1",
    "thread.jump.2",
    "thread.jump.3",
    "thread.jump.4",
    "thread.jump.5",
    "thread.jump.6",
    "thread.jump.7",
    "thread.jump.8",
    "thread.jump.9",
];

fn single_editor_defs(defs: &[&Def]) -> Vec<Value> {
    defs.iter()
        .map(|d| {
            let commands: Vec<Value> = d.commands.iter().map(|c| json!(c)).collect();
            json!({
                "id": commands.first().cloned().unwrap_or(json!(d.label)),
                "label": d.label,
                "description": d.description,
                "commands": commands,
            })
        })
        .collect()
}

fn list_shortcut_editor_definitions() -> Vec<Value> {
    let space_jump: std::collections::HashSet<&str> = SPACE_JUMP_COMMANDS.iter().copied().collect();
    let mut definitions: Vec<Value> = Vec::new();
    definitions.extend(single_editor_defs(&[&SIDEBAR_TOGGLE]));
    let available = available_now_defs();
    let filtered: Vec<&Def> = available
        .iter()
        .filter(|d| d.commands.len() > 1 || !space_jump.contains(d.commands[0]))
        .collect();
    definitions.extend(single_editor_defs(&filtered));
    if NATIVE_EDITOR_ON {
        let ce: Vec<&Def> = CODE_EDITOR_DEFS.iter().collect();
        definitions.extend(single_editor_defs(&ce));
    }
    let ws: Vec<&Def> = WORKSPACE_DEFS.iter().collect();
    definitions.extend(single_editor_defs(&ws));

    let thread_defs = thread_jump_defs();
    let thread_members: Vec<&Def> = thread_defs.iter().collect();
    definitions.push(json!({
        "id": "thread.jump",
        "label": "Jump to visible thread 1–9",
        "description": "Focus a visible thread directly from the sidebar number row.",
        "commands": THREAD_JUMP_COMMANDS,
        "members": single_editor_defs(&thread_members),
    }));
    let space_defs = space_jump_defs();
    let space_members: Vec<&Def> = space_defs.iter().collect();
    definitions.push(json!({
        "id": "space.jump",
        "label": "Jump to space 1–9",
        "description": "Switch straight to a tab of the space switcher. 1 is always Void.",
        "commands": SPACE_JUMP_COMMANDS,
        "members": single_editor_defs(&space_members),
    }));

    let covered: std::collections::HashSet<Value> = definitions
        .iter()
        .flat_map(|d| {
            d.get("commands")
                .and_then(|c| c.as_array().cloned())
                .unwrap_or_default()
        })
        .collect();
    for command in STATIC_COMMANDS {
        if covered.contains(&json!(command)) || command.starts_with("terminal.split") {
            continue;
        }
        definitions.push(json!({
            "id": command,
            "label": command,
            "description": "Assign a shortcut to this built-in command.",
            "commands": [command],
        }));
    }
    definitions
}

fn shortcut_sheet_command_label(command: &Value) -> Option<String> {
    let cmd = command.as_str()?;
    let available = available_now_defs();
    let thread = thread_jump_defs();
    let tables: [&[Def]; 5] = [
        std::slice::from_ref(&SIDEBAR_TOGGLE),
        &available,
        CODE_EDITOR_DEFS,
        WORKSPACE_DEFS,
        &thread,
    ];
    for table in tables {
        for d in table {
            if d.commands.contains(&cmd) {
                return Some(d.label.to_string());
            }
        }
    }
    None
}

// — fixed shortcuts (from fixedShortcuts) —

fn chord(key: &str, mod_key: bool) -> Value {
    json!({"key": key, "metaKey": false, "ctrlKey": false, "shiftKey": false, "altKey": false, "modKey": mod_key})
}

fn alt_chord(key: &str) -> Value {
    json!({"key": key, "metaKey": false, "ctrlKey": false, "shiftKey": false, "altKey": true, "modKey": false})
}

fn entry(shortcut: Value, reason: &str, group: Option<&str>) -> Value {
    let mut e = Map::new();
    e.insert("shortcut".to_string(), shortcut);
    e.insert("reason".to_string(), json!(reason));
    if let Some(g) = group {
        e.insert("group".to_string(), json!(g));
    }
    Value::Object(e)
}

fn id_entry(id: &str, shortcut: Value, reason: &str, group: Option<&str>) -> Value {
    let mut e = entry(shortcut, reason, group);
    if let Value::Object(m) = &mut e {
        m.insert("id".to_string(), json!(id));
    }
    e
}

fn fixed_shortcuts_for_platform() -> Vec<Value> {
    let terminal_when = json!({"type": "identifier", "name": "terminalFocus"});
    let mut terminal = id_entry(
        "terminal.search",
        chord("f", false),
        "searches the terminal while it has focus",
        None,
    );
    if let Value::Object(m) = &mut terminal {
        m.insert("whenAst".to_string(), terminal_when);
    }
    vec![
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
    ]
}

// — keybinding rule helpers (from keybindingRules / keybindings) —

const UNASSIGNED_KEYBINDING_KEY: &str = "unassigned";

fn is_unassigned_shortcut(shortcut: &Value) -> Result<bool, String> {
    Ok(js_strict_eq(
        &member(shortcut, "key", "shortcut.key")?,
        &json!(UNASSIGNED_KEYBINDING_KEY),
    ))
}

fn is_unassigned_keybinding(binding: &Value) -> Result<bool, String> {
    is_unassigned_shortcut(&member(binding, "shortcut", "binding.shortcut")?)
}

fn encode_keybinding_shortcut(shortcut: &Value) -> Result<Option<String>, String> {
    let mut mods: Vec<&str> = Vec::new();
    for (name, token) in [
        ("modKey", "mod"),
        ("metaKey", "meta"),
        ("ctrlKey", "ctrl"),
        ("altKey", "alt"),
        ("shiftKey", "shift"),
    ] {
        let flag = member(
            shortcut,
            name,
            match name {
                "modKey" => "shortcut.modKey",
                "metaKey" => "shortcut.metaKey",
                "ctrlKey" => "shortcut.ctrlKey",
                "altKey" => "shortcut.altKey",
                _ => "shortcut.shiftKey",
            },
        )?;
        if truthy(&flag) {
            mods.push(token);
        }
    }
    let key = member(shortcut, "key", "shortcut.key")?;
    if !truthy(&key) {
        return Ok(None);
    }
    // `shortcut.key !== "+" && shortcut.key.includes("+")`
    let key_str = match &key {
        Value::String(s) => s.clone(),
        _ => {
            return Err(not_a_fn(
                "shortcut.key.includes",
                "shortcut.key.includes(\"+\")",
            ));
        }
    };
    if key_str != "+" && key_str.contains('+') {
        return Ok(None);
    }
    mods.push(if key_str == " " { "space" } else { &key_str });
    Ok(Some(mods.join("+")))
}

fn when_type_prec(node: &Value) -> Result<Option<i64>, String> {
    let ty = member(node, "type", "node.type")?;
    Ok(match ty.as_str() {
        Some("or") => Some(1),
        Some("and") => Some(2),
        Some("not") => Some(3),
        Some("identifier") => Some(4),
        _ => None,
    })
}

/// `encodeKeybindingWhen`: returns the expression string, or the undefined marker
/// when the node type is not one the encoder knows.
fn encode_when(node: &Value) -> Result<Value, String> {
    let ty = member(node, "type", "node.type")?;
    let prec = when_type_prec(node)?.unwrap_or(-1);
    match ty.as_str() {
        Some("identifier") => Ok(Value::String(js_str(&member(node, "name", "node.name")?))),
        Some("not") => {
            let child = member(node, "node", "node.node")?;
            let needs = when_type_prec(&child)?.is_some_and(|p| p < prec);
            let inner = encode_when(&child)?;
            let mut text = js_str(&inner);
            if needs {
                text = format!("({text})");
            }
            Ok(Value::String(format!("!{text}")))
        }
        Some("and") | Some("or") => {
            let sep = if ty.as_str() == Some("and") {
                " && "
            } else {
                " || "
            };
            let mut sides = Vec::new();
            for (idx, side) in ["left", "right"].iter().enumerate() {
                let child = member(
                    node,
                    side,
                    match *side {
                        "left" => "node.left",
                        _ => "node.right",
                    },
                )?;
                let cp = when_type_prec(&child)?.unwrap_or(-1);
                let needs = cp >= 0 && (cp < prec || (cp == prec && idx == 1));
                let inner = encode_when(&child)?;
                let mut text = match &inner {
                    v if is_undefined(v) || v.is_null() => String::new(),
                    v => js_str(v),
                };
                if needs {
                    text = format!("({text})");
                }
                sides.push(text);
            }
            Ok(Value::String(sides.join(sep)))
        }
        _ => Ok(undef()),
    }
}

fn rule_identity(rule: &Value) -> Result<String, String> {
    let shortcut = member(rule, "shortcut", "rule.shortcut")?;
    let key = if is_unassigned_shortcut(&shortcut)? {
        UNASSIGNED_KEYBINDING_KEY.to_string()
    } else {
        match encode_keybinding_shortcut(&shortcut)? {
            Some(k) => k,
            None => js_str(&member(&shortcut, "key", "shortcut.key")?),
        }
    };
    let command = js_str(&member(rule, "command", "rule.command")?);
    let when_ast = member(rule, "whenAst", "rule.whenAst")?;
    let when = if truthy(&when_ast) {
        js_str(&encode_when(&when_ast)?)
    } else {
        String::new()
    };
    Ok(format!("{command}\u{0}{key}\u{0}{when}"))
}

fn evaluate_when_node(node: &Value, context: &Map<String, Value>) -> Result<Value, String> {
    let ty = member(node, "type", "node.type")?;
    match ty.as_str() {
        Some("identifier") => {
            let name = member(node, "name", "node.name")?;
            if name.as_str() == Some("true") {
                return Ok(Value::Bool(true));
            }
            if name.as_str() == Some("false") {
                return Ok(Value::Bool(false));
            }
            Ok(Value::Bool(truthy(
                context.get(&js_str(&name)).unwrap_or(&Value::Null),
            )))
        }
        Some("not") => {
            let inner = evaluate_when_node(&member(node, "node", "node.node")?, context)?;
            Ok(Value::Bool(!truthy(&inner)))
        }
        Some("and") => {
            let left = evaluate_when_node(&member(node, "left", "node.left")?, context)?;
            if !truthy(&left) {
                return Ok(left);
            }
            evaluate_when_node(&member(node, "right", "node.right")?, context)
        }
        Some("or") => {
            let left = evaluate_when_node(&member(node, "left", "node.left")?, context)?;
            if truthy(&left) {
                return Ok(left);
            }
            evaluate_when_node(&member(node, "right", "node.right")?, context)
        }
        _ => Ok(undef()),
    }
}

fn collect_when_names(node: &Value, names: &mut Vec<Value>) -> Result<(), String> {
    let ty = member(node, "type", "node.type")?;
    match ty.as_str() {
        Some("identifier") => {
            let name = member(node, "name", "node.name")?;
            if name.as_str() != Some("true")
                && name.as_str() != Some("false")
                && !names.iter().any(|n| js_strict_eq(n, &name))
            {
                names.push(name);
            }
        }
        Some("not") => collect_when_names(&member(node, "node", "node.node")?, names)?,
        Some("and") | Some("or") => {
            collect_when_names(&member(node, "left", "node.left")?, names)?;
            collect_when_names(&member(node, "right", "node.right")?, names)?;
        }
        _ => {}
    }
    Ok(())
}

fn is_focus_name(name: &Value) -> Result<bool, String> {
    match name {
        Value::String(s) => Ok(s.ends_with("Focus")),
        _ => Err(not_a_fn("name.endsWith", "name.endsWith(\"Focus\")")),
    }
}

const MAX_WHEN_NAMES: usize = 12;

fn conditions_hold_inner(raw_nodes: &[Value], facts: &Value) -> Result<bool, String> {
    // `conditions.filter((node) => node !== undefined)`
    let nodes: Vec<Value> = raw_nodes
        .iter()
        .filter(|n| !is_undefined(n))
        .cloned()
        .collect();
    let mut names: Vec<Value> = Vec::new();
    for node in &nodes {
        collect_when_names(node, &mut names)?;
    }
    // `free` = names not in facts; `name in facts` requires facts to be an object.
    // Arrays count as objects: their keys are the numeric indices and `length`.
    let facts_obj: Map<String, Value> = match facts {
        Value::Object(o) => o.clone(),
        Value::Array(a) => {
            let mut m = Map::new();
            for (i, v) in a.iter().enumerate() {
                m.insert(i.to_string(), v.clone());
            }
            m.insert("length".to_string(), json!(a.len()));
            m
        }
        _ => {
            if names.is_empty() {
                Map::new()
            } else {
                return Err("facts is not an Object. (evaluating 'name in facts')".to_string());
            }
        }
    };
    let mut free: Vec<Value> = Vec::new();
    for name in &names {
        if !facts_obj.contains_key(&js_str(name)) {
            free.push(name.clone());
        }
    }
    if free.len() > MAX_WHEN_NAMES {
        return Ok(true);
    }
    let mut focus_mask: u64 = 0;
    for (i, name) in free.iter().enumerate() {
        if is_focus_name(name)? {
            focus_mask |= 1 << i;
        }
    }
    let total = 1u64 << free.len();
    for mask in 0..total {
        let focused = mask & focus_mask;
        if focused.count_ones() > 1 {
            continue;
        }
        let mut context = facts_obj.clone();
        for (i, name) in free.iter().enumerate() {
            context.insert(js_str(name), Value::Bool(mask & (1 << i) != 0));
        }
        let mut all = true;
        for node in &nodes {
            if !truthy(&evaluate_when_node(node, &context)?) {
                all = false;
                break;
            }
        }
        if all {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Port of `whenConditionsCanHoldTogether` (`apps/web/src/keybindingEditor.ts:117`).
pub fn when_conditions_can_hold_together(
    conditions: serde_json::Value,
    facts: serde_json::Value,
) -> Result<bool, String> {
    let raw = call_filter(
        &conditions,
        "conditions.filter",
        "conditions.filter((node) => node !== void 0)",
    )?;
    let nodes: Vec<Value> = raw.into_iter().filter(|n| !is_undefined(n)).collect();
    conditions_hold_inner(&nodes, &facts)
}

fn platform_facts(platform: &Value) -> Value {
    json!({"isMac": is_mac(platform)})
}

fn is_shipped_rule(source: &Value, rule: &Value) -> Result<bool, String> {
    let identity = rule_identity(rule)?;
    let defaults = member(source, "defaultKeybindings", "source.defaultKeybindings")?;
    // `(source.defaultKeybindings ?? []).some(...)`
    let list = if defaults.is_null() || is_undefined(&defaults) {
        Vec::new()
    } else {
        call_filter(
            &defaults,
            "source.defaultKeybindings.some",
            "source.defaultKeybindings.some((shipped) => ruleIdentity(shipped) === identity)",
        )?
    };
    for shipped in &list {
        if rule_identity(shipped)? == identity {
            return Ok(true);
        }
    }
    Ok(false)
}

fn applies_on_platform(source: &Value, rule: &Value) -> Result<bool, String> {
    let when_ast = member(rule, "whenAst", "rule.whenAst")?;
    let platform = member(source, "platform", "source.platform")?;
    conditions_hold_inner(&[when_ast], &platform_facts(&platform))
}

fn visible_rules(source: &Value, command: &Value) -> Result<Vec<Value>, String> {
    let keybindings = member(source, "keybindings", "source.keybindings")?;
    let rules = call_filter(
        &keybindings,
        "source.keybindings.filter",
        "source.keybindings.filter((rule) => rule.command === command && !isUnassignedKeybinding(rule) && appliesOnPlatform(source, rule))",
    )?;
    let mut out = Vec::new();
    for rule in rules {
        let rc = member(&rule, "command", "rule.command")?;
        if js_strict_eq(&rc, command)
            && !is_unassigned_keybinding(&rule)?
            && applies_on_platform(source, &rule)?
        {
            out.push(rule);
        }
    }
    Ok(out)
}

fn commands_are_default(source: &Value, commands: &[Value]) -> Result<bool, String> {
    let defaults = member(source, "defaultKeybindings", "source.defaultKeybindings")?;
    if !truthy(&defaults) {
        return Ok(true);
    }
    let in_row = |rule: &Value| -> Result<bool, String> {
        let rc = member(rule, "command", "rule.command")?;
        Ok(commands.iter().any(|c| js_strict_eq(c, &rc)))
    };
    let shipped_list = call_filter(
        &defaults,
        "source.defaultKeybindings.filter",
        "source.defaultKeybindings.filter(inRow)",
    )?;
    let mut current: Vec<String> = Vec::new();
    let keybindings = member(source, "keybindings", "source.keybindings")?;
    for rule in call_filter(
        &keybindings,
        "source.keybindings.filter",
        "source.keybindings.filter(inRow)",
    )? {
        if in_row(&rule)? {
            current.push(rule_identity(&rule)?);
        }
    }
    let mut shipped: Vec<String> = Vec::new();
    for rule in shipped_list {
        if in_row(&rule)? {
            shipped.push(rule_identity(&rule)?);
        }
    }
    let key_utf16 = |s: &str| s.encode_utf16().collect::<Vec<u16>>();
    current.sort_by_key(|s| key_utf16(s));
    shipped.sort_by_key(|s| key_utf16(s));
    Ok(current == shipped)
}

fn same_modifiers(left: &Value, right: &Value) -> Result<bool, String> {
    for (name, expr) in [
        ("modKey", "shortcut.modKey"),
        ("metaKey", "shortcut.metaKey"),
        ("ctrlKey", "shortcut.ctrlKey"),
        ("altKey", "shortcut.altKey"),
        ("shiftKey", "shortcut.shiftKey"),
    ] {
        if !js_strict_eq(&member(left, name, expr)?, &member(right, name, expr)?) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn format_key_label(key: &Value) -> Result<String, String> {
    let s = match key {
        Value::String(s) => s.clone(),
        Value::Null => {
            return Err("null is not an object (evaluating 'key.slice')".to_string());
        }
        v if is_undefined(v) => {
            return Err("undefined is not an object (evaluating 'key.slice')".to_string());
        }
        Value::Array(a) if a.len() == 1 => {
            // `key.length === 1` is true for one-element arrays.
            return Err(not_a_fn("key.toUpperCase", "key.toUpperCase()"));
        }
        _ => {
            // `key.slice` doesn't exist on numbers/booleans/objects/arrays.
            return Err(not_a_fn("key.slice", "key.slice(0, 1)"));
        }
    };
    if s == " " {
        return Ok("Space".to_string());
    }
    if s.chars().count() == 1 {
        return Ok(s.to_uppercase());
    }
    Ok(match s.as_str() {
        "escape" => "Esc".to_string(),
        "arrowup" => "Up".to_string(),
        "arrowdown" => "Down".to_string(),
        "arrowleft" => "Left".to_string(),
        "arrowright" => "Right".to_string(),
        "pageup" => "PgUp".to_string(),
        "pagedown" => "PgDn".to_string(),
        _ => {
            let mut chars = s.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        }
    })
}

fn format_shortcut_label(shortcut: &Value, platform: &Value) -> Result<String, String> {
    let key = member(shortcut, "key", "shortcut.key")?;
    let key_label = format_key_label(&key)?;
    let use_meta = is_mac(platform);
    let g = |name: &str| truthy(shortcut.get(name).unwrap_or(&Value::Null));
    let show_meta = g("metaKey") || (g("modKey") && use_meta);
    let show_ctrl = g("ctrlKey") || (g("modKey") && !use_meta);
    let show_alt = g("altKey");
    let show_shift = g("shiftKey");
    if use_meta {
        return Ok(format!(
            "{}{}{}{}{}",
            if show_ctrl { "\u{2303}" } else { "" },
            if show_alt { "\u{2325}" } else { "" },
            if show_shift { "\u{21e7}" } else { "" },
            if show_meta { "\u{2318}" } else { "" },
            key_label
        ));
    }
    let mut parts: Vec<String> = Vec::new();
    if show_ctrl {
        parts.push("Ctrl".into());
    }
    if show_alt {
        parts.push("Alt".into());
    }
    if show_shift {
        parts.push("Shift".into());
    }
    if show_meta {
        parts.push("Meta".into());
    }
    parts.push(key_label);
    Ok(parts.join("+"))
}

fn shortcut_conflict_key(shortcut: &Value, platform: &Value) -> Result<String, String> {
    let key = member(shortcut, "key", "shortcut.key")?;
    let use_meta = is_mac(platform);
    let g = |name: &str| truthy(shortcut.get(name).unwrap_or(&Value::Null));
    let meta = g("metaKey") || (g("modKey") && use_meta);
    let ctrl = g("ctrlKey") || (g("modKey") && !use_meta);
    let parts = [
        join_str(&key),
        if meta {
            "meta".to_string()
        } else {
            String::new()
        },
        if ctrl {
            "ctrl".to_string()
        } else {
            String::new()
        },
        if g("shiftKey") {
            "shift".to_string()
        } else {
            String::new()
        },
        if g("altKey") {
            "alt".to_string()
        } else {
            String::new()
        },
    ];
    Ok(parts.join("|"))
}

const NUMBER_KEYS: [&str; 9] = ["1", "2", "3", "4", "5", "6", "7", "8", "9"];

fn single_binding(source: &Value, rule: &Value) -> Result<Value, String> {
    let platform = member(source, "platform", "source.platform")?;
    Ok(json!({
        "id": rule_identity(rule)?,
        "label": format_shortcut_label(&member(rule, "shortcut", "rule.shortcut")?, &platform)?,
        "rules": [rule.clone()],
    }))
}

fn when_text(node: &Value) -> Result<String, String> {
    if truthy(node) {
        Ok(js_str(&encode_when(node)?))
    } else {
        Ok(String::new())
    }
}

fn numbered_family_bindings(
    source: &Value,
    commands: &[Value],
) -> Result<Option<Vec<Value>>, String> {
    let mut rules_by_member: Vec<Vec<Value>> = Vec::new();
    for command in commands {
        rules_by_member.push(visible_rules(source, command)?);
    }
    let first = rules_by_member.first().cloned().unwrap_or_default();
    if rules_by_member.iter().any(|r| r.len() != first.len()) {
        return Ok(None);
    }
    let mut bindings: Vec<Value> = Vec::new();
    let platform = member(source, "platform", "source.platform")?;
    for (slot, lead) in first.iter().enumerate() {
        let mut rules: Vec<Value> = Vec::new();
        for member_rules in &rules_by_member {
            rules.push(member_rules.get(slot).cloned().unwrap_or_else(undef));
        }
        let mut uniform = true;
        for (index, rule) in rules.iter().enumerate() {
            if is_undefined(rule) {
                uniform = false;
                break;
            }
            let rule_key = member(
                &member(rule, "shortcut", "rule.shortcut")?,
                "key",
                "shortcut.key",
            )?;
            if rule_key.as_str() != Some(NUMBER_KEYS[index]) {
                uniform = false;
                break;
            }
            if !same_modifiers(
                &member(rule, "shortcut", "rule.shortcut")?,
                &member(lead, "shortcut", "rule.shortcut")?,
            )? {
                uniform = false;
                break;
            }
            if when_text(&member(rule, "whenAst", "rule.whenAst")?)?
                != when_text(&member(lead, "whenAst", "rule.whenAst")?)?
            {
                uniform = false;
                break;
            }
        }
        if !uniform {
            return Ok(None);
        }
        bindings.push(json!({
            "id": format!("{}:{}:{}", js_str(commands.first().unwrap_or(&Value::Null)), slot, rule_identity(lead)?),
            "label": format!("{}–9", format_shortcut_label(&member(lead, "shortcut", "rule.shortcut")?, &platform)?),
            "rules": rules,
        }));
    }
    Ok(Some(bindings))
}

fn single_row(source: &Value, definition: &Value) -> Result<Value, String> {
    let commands = member(definition, "commands", "definition.commands")?;
    let commands_arr = match &commands {
        Value::Array(a) => a.clone(),
        _ => Vec::new(),
    };
    let mut bindings: Vec<Value> = Vec::new();
    for command in &commands_arr {
        for rule in visible_rules(source, command)? {
            bindings.push(single_binding(source, &rule)?);
        }
    }
    Ok(json!({
        "id": member(definition, "id", "definition.id")?,
        "label": member(definition, "label", "definition.label")?,
        "description": member(definition, "description", "definition.description")?,
        "commands": commands,
        "numbered": false,
        "bindings": bindings,
        "isDefault": commands_are_default(source, &commands_arr)?,
    }))
}

/// Port of `buildShortcutEditorRows` (`apps/web/src/keybindingEditor.ts:249`).
pub fn build_shortcut_editor_rows(
    source: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    let _ = member(&source, "keybindings", "source.keybindings")?;
    let mut rows = Vec::new();
    for definition in list_shortcut_editor_definitions() {
        if !truthy(&member(&definition, "members", "definition.members")?) {
            rows.push(single_row(&source, &definition)?);
            continue;
        }
        let commands = member(&definition, "commands", "definition.commands")?;
        let commands_arr = match &commands {
            Value::Array(a) => a.clone(),
            _ => Vec::new(),
        };
        match numbered_family_bindings(&source, &commands_arr)? {
            None => {
                for member_def in member(&definition, "members", "definition.members")?
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                {
                    rows.push(single_row(&source, &member_def)?);
                }
            }
            Some(bindings) => {
                rows.push(json!({
                    "id": member(&definition, "id", "definition.id")?,
                    "label": member(&definition, "label", "definition.label")?,
                    "description": member(&definition, "description", "definition.description")?,
                    "commands": commands,
                    "numbered": true,
                    "bindings": bindings,
                    "isDefault": commands_are_default(&source, &commands_arr)?,
                }));
            }
        }
    }
    Ok(rows)
}

/// Port of `filterShortcutEditorRows` (`apps/web/src/keybindingEditor.ts:269`).
pub fn filter_shortcut_editor_rows(
    rows: serde_json::Value,
    query: String,
) -> Result<Vec<serde_json::Value>, String> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        // `return [...rows]` — spread, not filter.
        return match &rows {
            Value::Array(a) => Ok(a.clone()),
            _ => Err("rows is not iterable".to_string()),
        };
    }
    let items = call_filter(
        &rows,
        "rows.filter",
        "rows.filter((row) => row.label.toLowerCase().includes(needle) || row.description.toLowerCase().includes(needle) || row.bindings.some((binding) => binding.label.toLowerCase().includes(needle)))",
    )?;
    let mut out = Vec::new();
    for row in items {
        let label = member(&row, "label", "row.label")?;
        if call_to_lowercase(&label, "row.label.toLowerCase", "row.label.toLowerCase()")?
            .contains(&needle)
        {
            out.push(row);
            continue;
        }
        let description = member(&row, "description", "row.description")?;
        if call_to_lowercase(
            &description,
            "row.description.toLowerCase",
            "row.description.toLowerCase()",
        )?
        .contains(&needle)
        {
            out.push(row);
            continue;
        }
        let bindings = member(&row, "bindings", "row.bindings")?;
        let binding_list = match &bindings {
            Value::Null => {
                return Err("null is not an object (evaluating 'row.bindings.some')".to_string());
            }
            v if is_undefined(v) => {
                return Err(
                    "undefined is not an object (evaluating 'row.bindings.some')".to_string(),
                );
            }
            Value::Array(a) => a.clone(),
            _ => {
                return Err(not_a_fn(
                    "row.bindings.some",
                    "row.bindings.some((binding) => binding.label.toLowerCase().includes(needle))",
                ));
            }
        };
        let mut hit = false;
        for binding in binding_list {
            let blabel = member(&binding, "label", "binding.label")?;
            if call_to_lowercase(
                &blabel,
                "binding.label.toLowerCase",
                "binding.label.toLowerCase()",
            )?
            .contains(&needle)
            {
                hit = true;
                break;
            }
        }
        if hit {
            out.push(row);
        }
    }
    Ok(out)
}

/// Port of `shortcutModifierHint` (`apps/web/src/keybindingEditor.ts:393`).
pub fn shortcut_modifier_hint(platform: String) -> Result<String, String> {
    Ok(if is_mac(&json!(platform)) {
        "Use ⌘, ⌥ or ⌃ with any key, or an F-key on its own.".to_string()
    } else {
        "Use Ctrl or Alt with any key, or an F-key on its own.".to_string()
    })
}

fn default_when_for(source: &Value, command: &Value) -> Result<Value, String> {
    let defaults = member(source, "defaultKeybindings", "source.defaultKeybindings")?;
    let shipped_list = if defaults.is_null() || is_undefined(&defaults) {
        Vec::new()
    } else {
        match &defaults {
            Value::Array(a) => a.clone(),
            _ => {
                return Err(not_a_fn(
                    "(source.defaultKeybindings ?? []).find",
                    "(source.defaultKeybindings ?? []).find((rule) => rule.command === command && appliesOnPlatform(source, rule))",
                ));
            }
        }
    };
    let mut shipped = Value::Null;
    let mut found = false;
    for rule in shipped_list {
        if js_strict_eq(&member(&rule, "command", "rule.command")?, command)
            && applies_on_platform(source, &rule)?
        {
            shipped = rule;
            found = true;
            break;
        }
    }
    if !found {
        shipped = visible_rules(source, command)?
            .into_iter()
            .next()
            .unwrap_or(shipped);
    }
    if shipped.is_null() {
        return Ok(undef());
    }
    member(&shipped, "whenAst", "shipped.whenAst")
}

fn resolved_rule(command: &Value, shortcut: &Value, when_ast: &Value) -> Value {
    let mut o = Map::new();
    o.insert("command".to_string(), command.clone());
    o.insert("shortcut".to_string(), shortcut.clone());
    if truthy(when_ast) {
        o.insert("whenAst".to_string(), when_ast.clone());
    }
    Value::Object(o)
}

fn candidate_rules(
    source: &Value,
    row: &Value,
    replacing: &Value,
    shortcut: &Value,
) -> Result<Vec<Value>, String> {
    if !truthy(&member(row, "numbered", "row.numbered")?) {
        let replaced = if replacing.is_null() || is_undefined(replacing) {
            undef()
        } else {
            let rules = member(replacing, "rules", "replacing.rules")?;
            match &rules {
                Value::Array(a) => a.first().cloned().unwrap_or_else(undef),
                Value::String(s) => s
                    .chars()
                    .next()
                    .map(|c| json!(c.to_string()))
                    .unwrap_or_else(undef),
                _ => undef(),
            }
        };
        // `const command = replaced?.command ?? row.commands[0]`
        let mut command = undef();
        if !is_undefined(&replaced) && !replaced.is_null() {
            command = member(&replaced, "command", "replaced.command")?;
        }
        if command.is_null() || is_undefined(&command) {
            let commands = member(row, "commands", "row.commands")?;
            command = match &commands {
                Value::Array(a) => a.first().cloned().unwrap_or_else(undef),
                _ => undef(),
            };
        }
        if !truthy(&command) {
            return Ok(Vec::new());
        }
        let when_ast = if !is_undefined(&replaced) && !replaced.is_null() {
            member(&replaced, "whenAst", "replaced.whenAst")?
        } else {
            default_when_for(source, &command)?
        };
        return Ok(vec![resolved_rule(&command, shortcut, &when_ast)]);
    }
    let commands = member(row, "commands", "row.commands")?;
    let commands_arr = match &commands {
        Value::Array(a) => a.clone(),
        _ => {
            return Err(not_a_fn(
                "row.commands.map",
                "row.commands.map((command, index) => {",
            ));
        }
    };
    let replacing_rules = if replacing.is_null() || is_undefined(replacing) {
        None
    } else {
        Some(member(replacing, "rules", "replacing.rules")?)
    };
    let mut out = Vec::new();
    for (index, command) in commands_arr.iter().enumerate() {
        let replaced = match &replacing_rules {
            Some(Value::Array(a)) => a.get(index).cloned().unwrap_or_else(undef),
            _ => undef(),
        };
        let mut sc = match shortcut {
            Value::Object(o) => o.clone(),
            _ => Map::new(),
        };
        let key = if index < NUMBER_KEYS.len() {
            json!(NUMBER_KEYS[index])
        } else {
            member(shortcut, "key", "shortcut.key")?
        };
        sc.insert("key".to_string(), key);
        let when_ast = if !is_undefined(&replaced) && !replaced.is_null() {
            member(&replaced, "whenAst", "replaced.whenAst")?
        } else {
            default_when_for(source, command)?
        };
        out.push(resolved_rule(command, &Value::Object(sc), &when_ast));
    }
    Ok(out)
}

fn is_function_key(key: &Value) -> bool {
    let s = js_str(key);
    let b = s.as_bytes();
    if b.len() < 2 || b.len() > 3 || b[0] != b'f' {
        return false;
    }
    match &s[1..] {
        "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => true,
        t if t.len() == 2 && t.starts_with('1') && t.chars().all(|c| c.is_ascii_digit()) => true,
        t if t.len() == 2
            && t.starts_with('2')
            && t.chars().nth(1).is_some_and(|c| ('0'..='4').contains(&c)) =>
        {
            true
        }
        _ => false,
    }
}

fn is_project_script_command(command: &Value) -> bool {
    command.as_str().is_some_and(|c| c.starts_with("script."))
}

fn colliding_rules(
    source: &Value,
    candidates: &[Value],
    ignored: &[Value],
) -> Result<Vec<Value>, String> {
    let platform = member(source, "platform", "source.platform")?;
    let facts = platform_facts(&platform);
    let keybindings = member(source, "keybindings", "source.keybindings")?;
    let existing = call_filter(
        &keybindings,
        "source.keybindings.filter",
        "source.keybindings.filter((existing) =>",
    )?;
    let mut out = Vec::new();
    'outer: for ex in existing {
        if is_unassigned_keybinding(&ex)? {
            continue;
        }
        if ignored.iter().any(|i| i == &ex) {
            continue;
        }
        for candidate in candidates {
            if shortcut_conflict_key(
                &member(candidate, "shortcut", "candidate.shortcut")?,
                &platform,
            )? != shortcut_conflict_key(
                &member(&ex, "shortcut", "existing.shortcut")?,
                &platform,
            )? {
                continue;
            }
            if !conditions_hold_inner(
                &[
                    member(candidate, "whenAst", "candidate.whenAst")?,
                    member(&ex, "whenAst", "existing.whenAst")?,
                ],
                &facts,
            )? {
                continue;
            }
            let same_command = js_strict_eq(
                &member(candidate, "command", "candidate.command")?,
                &member(&ex, "command", "existing.command")?,
            );
            if !same_command && is_shipped_rule(source, candidate)? && is_shipped_rule(source, &ex)?
            {
                continue;
            }
            out.push(ex.clone());
            continue 'outer;
        }
    }
    Ok(out)
}

fn fixed_shortcut_reason(source: &Value, candidates: &[Value]) -> Result<Option<String>, String> {
    let platform = member(source, "platform", "source.platform")?;
    let facts = platform_facts(&platform);
    for candidate in candidates {
        let shortcut = member(candidate, "shortcut", "candidate.shortcut")?;
        let key = shortcut_conflict_key(&shortcut, &platform)?;
        for fixed in fixed_shortcuts_for_platform() {
            if shortcut_conflict_key(&member(&fixed, "shortcut", "entry.shortcut")?, &platform)?
                != key
            {
                continue;
            }
            if !conditions_hold_inner(
                &[
                    member(&fixed, "whenAst", "entry.whenAst")?,
                    member(candidate, "whenAst", "candidate.whenAst")?,
                ],
                &facts,
            )? {
                continue;
            }
            let reason = member(&fixed, "reason", "fixed.reason")?;
            return Ok(Some(format!(
                "{} {}",
                format_shortcut_label(&shortcut, &platform)?,
                js_str(&reason)
            )));
        }
    }
    Ok(None)
}

/// Port of `evaluateRecordedShortcut` (`apps/web/src/keybindingEditor.ts:403`).
pub fn evaluate_recorded_shortcut(input: serde_json::Value) -> Result<serde_json::Value, String> {
    if input.is_null() || is_undefined(&input) {
        return Err(
            "Cannot destructure property 'source' from null or undefined value".to_string(),
        );
    }
    let source = member(&input, "source", "input.source")?;
    let row = member(&input, "row", "input.row")?;
    let replacing = member(&input, "replacing", "input.replacing")?;
    let shortcut = member(&input, "shortcut", "input.shortcut")?;
    if !truthy(&shortcut) {
        return Ok(json!({"status": "idle"}));
    }

    let candidates = candidate_rules(&source, &row, &replacing, &shortcut)?;
    // `const replaced = replacing?.rules ?? []`
    let replaced_v = if replacing.is_null() || is_undefined(&replacing) {
        undef()
    } else {
        member(&replacing, "rules", "replacing.rules")?
    };
    // `replaced.length === candidates.length` — junk (non-array) replaced means
    // `undefined === n` → false.
    let mut unchanged = match &replaced_v {
        Value::Array(a) => a.len() == candidates.len(),
        v if v.is_null() || is_undefined(v) => candidates.is_empty(),
        _ => false,
    };
    let replaced: Vec<Value> = match &replaced_v {
        Value::Array(a) => a.clone(),
        _ => Vec::new(),
    };
    let platform = member(&source, "platform", "source.platform")?;
    if unchanged {
        for (index, candidate) in candidates.iter().enumerate() {
            let a = shortcut_conflict_key(
                &member(candidate, "shortcut", "candidate.shortcut")?,
                &platform,
            )?;
            let b = shortcut_conflict_key(
                &member(&replaced[index], "shortcut", "replaced[index].shortcut")?,
                &platform,
            )?;
            if a != b {
                unchanged = false;
                break;
            }
        }
    }
    if unchanged {
        return Ok(json!({"status": "idle"}));
    }

    let label = format_shortcut_label(&shortcut, &platform)?;
    let g = |name: &str| truthy(shortcut.get(name).unwrap_or(&Value::Null));
    let has_command_modifier = g("modKey") || g("metaKey") || g("ctrlKey") || g("altKey");
    if truthy(&member(&row, "numbered", "row.numbered")?)
        && !NUMBER_KEYS.iter().any(|k| {
            js_strict_eq(
                &member(&shortcut, "key", "shortcut.key").unwrap_or(Value::Null),
                &json!(k),
            )
        })
    {
        return Ok(
            json!({"status": "problem", "message": "Hold the modifiers and press a number key."}),
        );
    }
    if !has_command_modifier && !is_function_key(&member(&shortcut, "key", "shortcut.key")?) {
        let message = if is_mac(&platform) {
            "Add ⌘, ⌥ or ⌃ to that key, or use an F-key on its own."
        } else {
            "Add Ctrl or Alt to that key, or use an F-key on its own."
        };
        return Ok(json!({"status": "problem", "message": message}));
    }
    if let Some(fixed) = fixed_shortcut_reason(&source, &candidates)? {
        return Ok(json!({"status": "problem", "message": format!("{fixed}. Try another.")}));
    }

    let colliding = colliding_rules(&source, &candidates, &replaced)?;
    let row_commands = member(&row, "commands", "row.commands")?;
    let mut same_row_conflict = false;
    for rule in &colliding {
        let rc = member(rule, "command", "rule.command")?;
        if call_includes(
            &row_commands,
            &rc,
            "row.commands.includes",
            "row.commands.includes(rule.command)",
        )? {
            same_row_conflict = true;
            break;
        }
    }
    if same_row_conflict {
        return Ok(
            json!({"status": "problem", "message": format!("{label} is already a shortcut for this command.")}),
        );
    }
    if let Some(script) = colliding.iter().find(|rule| {
        member(rule, "command", "rule.command")
            .map(|c| is_project_script_command(&c))
            .unwrap_or(false)
    }) {
        let _ = script;
        return Ok(
            json!({"status": "problem", "message": format!("{label} already runs a project script. Change it from the project's scripts first.")}),
        );
    }
    let mut conflicts = Vec::new();
    for rule in &colliding {
        let command = member(rule, "command", "rule.command")?;
        let label_v = match shortcut_sheet_command_label(&command) {
            Some(l) => json!(l),
            None => command,
        };
        conflicts.push(json!({"rule": rule, "label": label_v}));
    }
    Ok(json!({"status": "ready", "candidates": candidates, "conflicts": conflicts}))
}

fn keybinding_value_from_shortcut(shortcut: &Value) -> Result<String, String> {
    let mut parts: Vec<String> = Vec::new();
    for (name, token) in [
        ("modKey", "mod"),
        ("ctrlKey", "ctrl"),
        ("metaKey", "meta"),
        ("altKey", "alt"),
        ("shiftKey", "shift"),
    ] {
        let flag = member(
            shortcut,
            name,
            match name {
                "modKey" => "shortcut.modKey",
                "ctrlKey" => "shortcut.ctrlKey",
                "metaKey" => "shortcut.metaKey",
                "altKey" => "shortcut.altKey",
                _ => "shortcut.shiftKey",
            },
        )?;
        if truthy(&flag) {
            parts.push(token.to_string());
        }
    }
    let key = member(shortcut, "key", "shortcut.key")?;
    parts.push(match key.as_str() {
        Some(" ") => "space".to_string(),
        Some("escape") => "esc".to_string(),
        _ => join_str(&key),
    });
    Ok(parts.join("+"))
}

fn keybinding_rule(rule: &Value) -> Result<Value, String> {
    // `encodeKeybindingRule(rule)`
    let shortcut = member(rule, "shortcut", "rule.shortcut")?;
    let key = if is_unassigned_shortcut(&shortcut)? {
        Some(UNASSIGNED_KEYBINDING_KEY.to_string())
    } else {
        encode_keybinding_shortcut(&shortcut)?
    };
    match key {
        Some(key) => {
            let mut o = Map::new();
            o.insert(
                "command".to_string(),
                member(rule, "command", "rule.command")?,
            );
            o.insert("key".to_string(), json!(key));
            let when_ast = member(rule, "whenAst", "rule.whenAst")?;
            if truthy(&when_ast) {
                o.insert("when".to_string(), encode_when(&when_ast)?);
            }
            Ok(Value::Object(o))
        }
        None => {
            let mut o = Map::new();
            o.insert(
                "command".to_string(),
                member(rule, "command", "rule.command")?,
            );
            o.insert(
                "key".to_string(),
                json!(keybinding_value_from_shortcut(&shortcut)?),
            );
            let when_ast = member(rule, "whenAst", "rule.whenAst")?;
            if truthy(&when_ast) {
                o.insert("when".to_string(), encode_when(&when_ast)?);
            }
            Ok(Value::Object(o))
        }
    }
}

/// Port of `shortcutSaveEdits` (`apps/web/src/keybindingEditor.ts:469`).
pub fn shortcut_save_edits(
    recording: serde_json::Value,
    replacing: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    let conflicts = member(&recording, "conflicts", "recording.conflicts")?;
    let conflict_list = call_filter(
        &conflicts,
        "recording.conflicts.map",
        "recording.conflicts.map((conflict) => ({ type: \"remove\", rule: keybindingRule(conflict.rule) }))",
    )?;
    let candidates = member(&recording, "candidates", "recording.candidates")?;
    let candidate_list = call_filter(
        &candidates,
        "recording.candidates.map",
        "recording.candidates.map((candidate, index) => ({ type: \"set\", rule: keybindingRule(candidate), ...(replaced ? { replacing: keybindingRule(replaced) } : {}) }))",
    )?;
    let replacing_rules = if !replacing.is_null() && !is_undefined(&replacing) {
        Some(member(&replacing, "rules", "replacing.rules")?)
    } else {
        None
    };
    let mut edits = Vec::new();
    for conflict in conflict_list {
        let rule = member(&conflict, "rule", "conflict.rule")?;
        edits.push(json!({"type": "remove", "rule": keybinding_rule(&rule)?}));
    }
    for (index, candidate) in candidate_list.iter().enumerate() {
        let mut edit = Map::new();
        edit.insert("type".to_string(), json!("set"));
        edit.insert("rule".to_string(), keybinding_rule(candidate)?);
        let replaced = match &replacing_rules {
            Some(Value::Array(a)) => a.get(index).cloned().unwrap_or_else(undef),
            Some(Value::Object(o)) => o.get(&index.to_string()).cloned().unwrap_or_else(undef),
            Some(Value::String(s)) => s
                .chars()
                .nth(index)
                .map(|c| json!(c.to_string()))
                .unwrap_or_else(undef),
            _ => undef(),
        };
        if truthy(&replaced) {
            edit.insert("replacing".to_string(), keybinding_rule(&replaced)?);
        }
        edits.push(Value::Object(edit));
    }
    Ok(edits)
}

/// Port of `shortcutRemoveEdits` (`apps/web/src/keybindingEditor.ts:488`).
pub fn shortcut_remove_edits(binding: serde_json::Value) -> Result<Vec<serde_json::Value>, String> {
    let rules = member(&binding, "rules", "binding.rules")?;
    let rule_list = call_filter(
        &rules,
        "binding.rules.map",
        "binding.rules.map((rule) => ({ type: \"remove\", rule: keybindingRule(rule) }))",
    )?;
    let mut edits = Vec::new();
    for rule in rule_list {
        edits.push(json!({"type": "remove", "rule": keybinding_rule(&rule)?}));
    }
    Ok(edits)
}

/// Port of `shortcutResetTakeovers` (`apps/web/src/keybindingEditor.ts:498`).
pub fn shortcut_reset_takeovers(
    source: serde_json::Value,
    row: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    let commands = member(&row, "commands", "row.commands")?;
    let defaults = member(&source, "defaultKeybindings", "source.defaultKeybindings")?;
    let shipped = if truthy(&defaults) {
        let list = call_filter(
            &defaults,
            "(source.defaultKeybindings ?? []).filter",
            "(source.defaultKeybindings ?? []).filter((rule) => row.commands.includes(rule.command) && appliesOnPlatform(source, rule))",
        )?;
        let mut kept = Vec::new();
        for rule in list {
            if call_includes(
                &commands,
                &member(&rule, "command", "rule.command")?,
                "row.commands.includes",
                "row.commands.includes(rule.command)",
            )? && applies_on_platform(&source, &rule)?
            {
                kept.push(rule);
            }
        }
        kept
    } else {
        Vec::new()
    };
    let keybindings = member(&source, "keybindings", "source.keybindings")?;
    let all = call_filter(
        &keybindings,
        "source.keybindings.filter",
        "source.keybindings.filter((rule) => row.commands.includes(rule.command))",
    )?;
    let mut own_rules = Vec::new();
    for rule in all {
        if call_includes(
            &commands,
            &member(&rule, "command", "rule.command")?,
            "row.commands.includes",
            "row.commands.includes(rule.command)",
        )? {
            own_rules.push(rule);
        }
    }
    let colliding = colliding_rules(&source, &shipped, &own_rules)?;
    let mut out = Vec::new();
    for rule in colliding {
        let command = member(&rule, "command", "rule.command")?;
        if is_project_script_command(&command) {
            continue;
        }
        let label = match shortcut_sheet_command_label(&command) {
            Some(l) => json!(l),
            None => command,
        };
        out.push(json!({"rule": rule, "label": label}));
    }
    Ok(out)
}

/// Port of `shortcutResetEdits` (`apps/web/src/keybindingEditor.ts:512`).
pub fn shortcut_reset_edits(
    source: serde_json::Value,
    row: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    let takeovers = shortcut_reset_takeovers(source.clone(), row.clone())?;
    let mut edits = Vec::new();
    for takeover in takeovers {
        let rule = member(&takeover, "rule", "takeover.rule")?;
        edits.push(json!({"type": "remove", "rule": keybinding_rule(&rule)?}));
    }
    let commands = member(&row, "commands", "row.commands")?;
    let command_list = match &commands {
        Value::Array(a) => a.clone(),
        _ => {
            return Err(not_a_fn(
                "row.commands.map",
                "row.commands.map((command) => ({ type: \"reset\", command }))",
            ));
        }
    };
    for command in command_list {
        edits.push(json!({"type": "reset", "command": command}));
    }
    Ok(edits)
}
