//! The Rust port of `apps/web/src/shortcutsSheet.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};

const NATIVE_EDITOR_ON: bool = true;

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(true),
        Value::String(s) => !s.is_empty(),
        _ => true,
    }
}

fn is_undefined(v: &Value) -> bool {
    matches!(v, Value::Object(o) if o.len() == 1 && o.get("$undefined") == Some(&Value::Bool(true)))
}

fn js_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Array(a) => a.iter().map(js_str).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".into(),
    }
}

fn is_mac(platform: &Value) -> bool {
    let p = js_str(platform).to_lowercase();
    p.contains("mac")
        || p.contains("darwin")
        || p.contains("iphone")
        || p.contains("ipad")
        || p.contains("ipod")
}

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

const EFFORT_CYCLE: Def = Def {
    commands: &["model.effort.next"],
    label: "Next model effort",
    description: "Cycle the model's available effort levels and briefly show the selected level.",
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

// — when-node helpers —

fn ident(name: &str) -> Value {
    json!({"type": "identifier", "name": name})
}
fn wnot(n: Value) -> Value {
    json!({"type": "not", "node": n})
}
fn wand(l: Value, r: Value) -> Value {
    json!({"type": "and", "left": l, "right": r})
}
fn wor(l: Value, r: Value) -> Value {
    json!({"type": "or", "left": l, "right": r})
}

fn when_mod_chord_allowed() -> Value {
    wor(wnot(ident("terminalFocus")), ident("isMac"))
}
fn when_not_terminal_focus() -> Value {
    wnot(ident("terminalFocus"))
}
fn when_thread_jump_available() -> Value {
    wor(
        wand(
            when_not_terminal_focus(),
            wnot(ident("terminalWorkspaceOpen")),
        ),
        ident("isMac"),
    )
}

fn shortcut(key: &str, meta: bool, ctrl: bool, shift: bool, alt: bool, modk: bool) -> Value {
    json!({
        "key": key,
        "metaKey": meta,
        "ctrlKey": ctrl,
        "shiftKey": shift,
        "altKey": alt,
        "modKey": modk,
    })
}

fn command_shortcut(
    key: &str,
    meta: bool,
    ctrl: bool,
    shift: bool,
    alt: bool,
    modk: bool,
) -> Value {
    // commandShortcut defaults all modifier flags false and modKey true.
    shortcut(key, meta, ctrl, shift, alt, modk)
}

fn rule(command: &str, shortcut: Value, when_ast: Option<Value>) -> Value {
    let mut o = Map::new();
    o.insert("command".into(), json!(command));
    o.insert("shortcut".into(), shortcut);
    if let Some(w) = when_ast {
        o.insert("whenAst".into(), w);
    }
    Value::Object(o)
}

fn default_fallbacks() -> Vec<Value> {
    let mc = || when_mod_chord_allowed();
    let ntf = || when_not_terminal_focus();
    let cs = command_shortcut;
    let mut v = vec![
        rule(
            "thread.archive",
            cs("a", false, false, true, true, true),
            Some(mc()),
        ),
        rule(
            "thread.snooze",
            cs("s", false, false, true, true, true),
            Some(mc()),
        ),
        rule(
            "thread.markUnread",
            cs("u", false, false, true, true, true),
            Some(mc()),
        ),
        rule(
            "sidechat.toggle",
            cs("s", false, false, false, true, true),
            Some(mc()),
        ),
        rule(
            "sidebar.activity",
            cs("u", false, false, false, true, true),
            Some(mc()),
        ),
        rule(
            "chat.attention.next",
            cs("a", false, false, false, true, true),
            Some(mc()),
        ),
        rule(
            "sidebar.addProject",
            cs("o", false, false, true, false, true),
            Some(ntf()),
        ),
        rule(
            "sidebar.importThread",
            cs("i", false, false, false, false, true),
            Some(ntf()),
        ),
        rule(
            "chat.new",
            cs("n", false, false, false, false, true),
            Some(mc()),
        ),
        rule(
            "chat.newLatestProject",
            cs("n", false, false, true, false, true),
            Some(mc()),
        ),
        rule(
            "chat.newClaude",
            cs("c", false, false, false, true, true),
            Some(mc()),
        ),
        rule(
            "chat.newChat",
            cs("n", false, false, false, true, true),
            Some(mc()),
        ),
        rule(
            "chat.newTerminal",
            cs("t", false, false, true, false, true),
            Some(mc()),
        ),
        rule(
            "chat.newCodex",
            cs("x", false, false, false, true, true),
            Some(mc()),
        ),
        rule(
            "chat.newCursor",
            cs("r", false, false, false, true, true),
            Some(mc()),
        ),
        rule(
            "chat.split",
            cs("\\", false, false, false, false, true),
            Some(mc()),
        ),
        rule(
            "view.recent.next",
            cs("tab", false, true, false, false, false),
            None,
        ),
        rule(
            "view.recent.previous",
            cs("tab", false, true, true, false, false),
            None,
        ),
        rule(
            "modelPicker.toggle",
            cs("m", false, false, true, false, true),
            Some(ntf()),
        ),
        rule(
            "model.next",
            cs("]", false, false, false, true, false),
            Some(ntf()),
        ),
        rule(
            "model.previous",
            cs("[", false, false, false, true, false),
            Some(ntf()),
        ),
        rule(
            "model.effort.next",
            cs("tab", false, false, true, false, false),
            Some(ident("composerFocus")),
        ),
        rule(
            "traitsPicker.toggle",
            cs("e", false, false, true, false, true),
            Some(ntf()),
        ),
        rule(
            "diff.change.next",
            cs("arrowdown", false, false, false, true, false),
            Some(ntf()),
        ),
        rule(
            "diff.change.previous",
            cs("arrowup", false, false, false, true, false),
            Some(ntf()),
        ),
        rule(
            "composer.focus.toggle",
            cs("l", true, false, false, false, false),
            Some(ntf()),
        ),
        rule(
            "chat.find",
            cs("f", false, false, false, false, true),
            Some(ntf()),
        ),
        rule(
            "search.files",
            cs("p", false, false, false, false, true),
            Some(ntf()),
        ),
        rule(
            "search.content",
            cs("f", false, false, true, false, true),
            Some(ntf()),
        ),
        rule(
            "settings.usage",
            cs("u", false, false, true, false, true),
            Some(ntf()),
        ),
        rule(
            "git.commitAndPush",
            cs("p", true, true, false, false, false),
            Some(wand(ntf(), ident("isMac"))),
        ),
        rule(
            "git.commitAndPush",
            cs("p", false, true, false, true, false),
            Some(wand(ntf(), wnot(ident("isMac")))),
        ),
        rule(
            "threadTab.next",
            cs("arrowright", false, true, false, false, true),
            Some(ident("isMac")),
        ),
        rule(
            "threadTab.previous",
            cs("arrowleft", false, true, false, false, true),
            Some(ident("isMac")),
        ),
        rule(
            "threadTab.next",
            cs("pagedown", false, true, false, false, false),
            Some(wand(ntf(), wnot(ident("isMac")))),
        ),
        rule(
            "threadTab.previous",
            cs("pageup", false, true, false, false, false),
            Some(wand(ntf(), wnot(ident("isMac")))),
        ),
        rule(
            "threadTab.close",
            cs("w", false, false, false, false, true),
            Some(wand(ntf(), wnot(ident("terminalWorkspaceOpen")))),
        ),
    ];
    for (i, command) in SPACE_JUMP_COMMANDS.iter().enumerate() {
        v.push(rule(
            command,
            cs(
                match i {
                    0 => "1",
                    1 => "2",
                    2 => "3",
                    3 => "4",
                    4 => "5",
                    5 => "6",
                    6 => "7",
                    7 => "8",
                    _ => "9",
                },
                false,
                false,
                false,
                true,
                true,
            ),
            Some(mc()),
        ));
    }
    for (i, command) in THREAD_JUMP_COMMANDS.iter().enumerate() {
        v.push(rule(
            command,
            cs(
                match i {
                    0 => "1",
                    1 => "2",
                    2 => "3",
                    3 => "4",
                    4 => "5",
                    5 => "6",
                    6 => "7",
                    7 => "8",
                    _ => "9",
                },
                false,
                false,
                false,
                false,
                true,
            ),
            Some(when_thread_jump_available()),
        ));
    }
    v.extend([
        rule(
            "thread.copyId",
            cs("c", false, false, true, false, true),
            Some(mc()),
        ),
        rule(
            "terminal.workspace.newFullWidth",
            cs("j", false, false, true, false, true),
            None,
        ),
        rule(
            "terminal.workspace.closeActive",
            cs("w", false, false, false, false, true),
            Some(ident("terminalWorkspaceOpen")),
        ),
        rule(
            "terminal.workspace.terminal",
            cs("1", false, true, false, false, false),
            Some(ident("terminalWorkspaceOpen")),
        ),
        rule(
            "terminal.workspace.chat",
            cs("2", false, true, false, false, false),
            Some(ident("terminalWorkspaceOpen")),
        ),
        rule(
            "editor.file.save",
            cs("s", false, false, false, false, true),
            Some(ntf()),
        ),
    ]);
    for (command, key, shift) in [
        ("editor.commands", "f1", false),
        ("editor.rename", "f2", false),
        ("editor.goToDefinition", "f12", false),
        ("editor.findReferences", "f12", true),
        ("editor.debug.start", "f5", false),
        ("editor.debug.stop", "f5", true),
        ("editor.debug.toggleBreakpoint", "f9", false),
        ("editor.debug.stepOver", "f10", false),
        ("editor.debug.stepInto", "f11", false),
        ("editor.debug.stepOut", "f11", true),
    ] {
        v.push(rule(
            command,
            cs(key, false, false, shift, false, false),
            Some(ntf()),
        ));
    }
    v.push(rule(
        "editor.quickFix",
        cs(".", false, false, false, false, true),
        Some(ntf()),
    ));
    v
}

// — resolution —

fn eval_when(node: &Value, context: &Map<String, Value>) -> bool {
    let Some(t) = node.get("type").and_then(|t| t.as_str()) else {
        return false;
    };
    match t {
        "identifier" => {
            let name = node.get("name").and_then(|n| n.as_str()).unwrap_or("");
            if name == "true" {
                return true;
            }
            if name == "false" {
                return false;
            }
            context.get(name).is_some_and(truthy)
        }
        "not" => node
            .get("node")
            .map(|n| !eval_when(n, context))
            .unwrap_or(false),
        "and" => {
            eval_when(node.get("left").unwrap_or(&Value::Null), context)
                && eval_when(node.get("right").unwrap_or(&Value::Null), context)
        }
        "or" => {
            eval_when(node.get("left").unwrap_or(&Value::Null), context)
                || eval_when(node.get("right").unwrap_or(&Value::Null), context)
        }
        _ => false,
    }
}

fn resolve_active_context(
    options: Option<&Value>,
    platform: &Value,
) -> Result<Map<String, Value>, String> {
    let mut ctx = Map::new();
    ctx.insert("terminalFocus".into(), json!(false));
    ctx.insert("terminalOpen".into(), json!(false));
    ctx.insert("isMac".into(), json!(is_mac(platform)));
    if let Some(Value::Object(extra)) = options.and_then(|o| o.get("context")) {
        for (k, v) in extra {
            ctx.insert(k.clone(), v.clone());
        }
    }
    Ok(ctx)
}

fn conflict_key(shortcut: &Value, use_meta_for_mod: bool) -> Result<String, String> {
    let getb = |name: &str| truthy(shortcut.get(name).unwrap_or(&Value::Null));
    let key = match shortcut.get("key") {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) | None => String::new(),
        Some(other) => js_str(other),
    };
    let meta = getb("metaKey") || (getb("modKey") && use_meta_for_mod);
    let ctrl = getb("ctrlKey") || (getb("modKey") && !use_meta_for_mod);
    Ok(format!(
        "{}|{}|{}|{}|{}",
        key,
        if meta { "meta" } else { "" },
        if ctrl { "ctrl" } else { "" },
        if getb("shiftKey") { "shift" } else { "" },
        if getb("altKey") { "alt" } else { "" },
    ))
}

fn find_effective(
    keybindings: &Value,
    command: &str,
    platform: &Value,
    context: &Map<String, Value>,
) -> Result<Option<Value>, String> {
    let arr = match keybindings {
        Value::Array(a) => a,
        Value::Null => return Err("null is not an object (evaluating 'keybindings.length')".into()),
        v if is_undefined(v) => {
            return Err("undefined is not an object (evaluating 'keybindings.length')".into());
        }
        _ => return Ok(None), // primitive: length undefined -> loop skipped
    };
    let use_meta_for_mod = is_mac(platform);
    let mut claimed: std::collections::HashSet<String> = std::collections::HashSet::new();
    for binding in arr.iter().rev() {
        if !truthy(binding) {
            continue;
        }
        // isUnassignedKeybinding: reads binding.shortcut.key
        let shortcut_v = binding.get("shortcut");
        let is_unassigned = match shortcut_v {
            None => return Err("undefined is not an object (evaluating 'shortcut.key')".into()),
            Some(Value::Null) => {
                return Err("null is not an object (evaluating 'shortcut.key')".into());
            }
            Some(s) => s.get("key") == Some(&json!("unassigned")),
        };
        if is_unassigned {
            continue;
        }
        match binding.get("command") {
            Some(Value::String(c)) => {
                if c.starts_with("terminal.split") {
                    continue;
                }
            }
            Some(Value::Null) | None => {
                return Err(
                    "undefined is not an object (evaluating 'binding.command.startsWith')".into(),
                );
            }
            Some(other) => {
                if other.is_object() {
                    return Err("binding.command.startsWith is not a function. (In 'binding.command.startsWith(\"terminal.split\")', 'binding.command.startsWith' is undefined)".into());
                }
                return Err("binding.command.startsWith is not a function. (In 'binding.command.startsWith(\"terminal.split\")', 'binding.command.startsWith' is undefined)".into());
            }
        }
        let when_ok = match binding.get("whenAst") {
            None | Some(Value::Null) => true,
            Some(w) => eval_when(w, context),
        };
        if !when_ok {
            continue;
        }
        let key = conflict_key(
            binding.get("shortcut").unwrap_or(&Value::Null),
            use_meta_for_mod,
        )?;
        if claimed.contains(&key) {
            continue;
        }
        claimed.insert(key);
        if binding.get("command") == Some(&json!(command)) {
            return Ok(Some(binding.clone()));
        }
    }
    Ok(None)
}

fn get_fallback_bindings(keybindings: &Value) -> Result<Vec<Value>, String> {
    match keybindings {
        Value::Array(arr) => {
            let mut configured = std::collections::HashSet::new();
            for b in arr {
                match b.get("command") {
                    Some(c) => {
                        configured.insert(c.clone());
                    }
                    None => {
                        return if b.is_null() {
                            Err("null is not an object (evaluating 'binding.command')".into())
                        } else {
                            Err("undefined is not an object (evaluating 'binding.command')".into())
                        };
                    }
                }
            }
            Ok(default_fallbacks()
                .into_iter()
                .filter(|b| !configured.contains(&b["command"]))
                .collect())
        }
        Value::Null => Err("null is not an object (evaluating 'keybindings.map')".into()),
        v if is_undefined(v) => {
            Err("undefined is not an object (evaluating 'keybindings.map')".into())
        }
        _ => Err("keybindings.map is not a function. (In 'keybindings.map((binding) => binding.command)', 'keybindings.map' is undefined)".into()),
    }
}

fn resolve_keybinding_for_command(
    keybindings: &Value,
    command: &str,
    options: Option<&Value>,
    platform: &Value,
) -> Result<Option<Value>, String> {
    let context = resolve_active_context(options, platform)?;
    if let Some(b) = find_effective(keybindings, command, platform, &context)? {
        return Ok(Some(b));
    }
    let fallbacks = get_fallback_bindings(keybindings)?;
    find_effective(&Value::Array(fallbacks), command, platform, &context)
}

fn key_label(key: &str) -> String {
    if key == " " {
        return "Space".into();
    }
    if key.chars().count() == 1 {
        return key.to_uppercase();
    }
    match key {
        "escape" => "Esc".into(),
        "arrowup" => "Up".into(),
        "arrowdown" => "Down".into(),
        "arrowleft" => "Left".into(),
        "arrowright" => "Right".into(),
        "pageup" => "PgUp".into(),
        "pagedown" => "PgDn".into(),
        _ => {
            let mut chars = key.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        }
    }
}

fn build_shortcut_text(shortcut: &Value, platform: &Value) -> String {
    let key = shortcut.get("key").and_then(|k| k.as_str()).unwrap_or("");
    let label = key_label(key);
    let use_meta_for_mod = is_mac(platform);
    let getb = |name: &str| truthy(shortcut.get(name).unwrap_or(&Value::Null));
    let show_meta = getb("metaKey") || (getb("modKey") && use_meta_for_mod);
    let show_ctrl = getb("ctrlKey") || (getb("modKey") && !use_meta_for_mod);
    let show_alt = getb("altKey");
    let show_shift = getb("shiftKey");
    if use_meta_for_mod {
        return format!(
            "{}{}{}{}{}",
            if show_ctrl { "\u{2303}" } else { "" },
            if show_alt { "\u{2325}" } else { "" },
            if show_shift { "\u{21e7}" } else { "" },
            if show_meta { "\u{2318}" } else { "" },
            label
        );
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
    parts.push(label);
    parts.join("+")
}

// — editor definitions —

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

pub fn list_shortcut_editor_definitions() -> Result<Vec<Value>, String> {
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
    Ok(definitions)
}

pub fn shortcut_sheet_command_label(command: Value) -> Result<Option<String>, String> {
    let Some(cmd) = command.as_str() else {
        return Ok(None);
    };
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
                return Ok(Some(d.label.to_string()));
            }
        }
    }
    Ok(None)
}

// — sections —

fn definition_to_entry(
    def: &Def,
    keybindings: &Value,
    options: Option<&Value>,
    platform: &Value,
) -> Result<Option<Value>, String> {
    let mut binding: Option<Value> = None;
    for command in def.commands {
        binding = resolve_keybinding_for_command(keybindings, command, options, platform)?;
        if binding.is_some() {
            break;
        }
    }
    let Some(binding) = binding else {
        return Ok(None);
    };
    let command = binding.get("command").cloned().unwrap_or(Value::Null);
    let shortcut_label =
        build_shortcut_text(binding.get("shortcut").unwrap_or(&Value::Null), platform);
    Ok(Some(json!({
        "id": command,
        "command": command,
        "binding": binding,
        "label": def.label,
        "description": def.description,
        "shortcutLabel": shortcut_label,
    })))
}

fn definitions_to_entries(
    defs: &[&Def],
    keybindings: &Value,
    options: Option<&Value>,
    platform: &Value,
) -> Result<Vec<Value>, String> {
    let mut out = Vec::new();
    for d in defs {
        if let Some(e) = definition_to_entry(d, keybindings, options, platform)? {
            out.push(e);
        }
    }
    Ok(out)
}

pub fn build_shortcut_sheet_sections(options: Value) -> Result<Vec<Value>, String> {
    // modSlashLabel(options.platform) runs first in the entries literal.
    let platform = match options.get("platform") {
        _ if options.is_null() => {
            return Err("null is not an object (evaluating 'options.platform')".into());
        }
        _ if is_undefined(&options) => {
            return Err("undefined is not an object (evaluating 'options.platform')".into());
        }
        Some(p) => p.clone(),
        None => Value::Null,
    };
    let keybindings = options.get("keybindings").cloned().unwrap_or(Value::Null);
    let context_options =
        json!({"context": options.get("context").cloned().unwrap_or(Value::Null)});
    let opts = Some(&context_options);

    let mut sections: Vec<Value> = Vec::new();
    let mut current_entries: Vec<Value> = vec![json!({
        "id": "shortcuts.show",
        "command": Value::Null,
        "binding": Value::Null,
        "label": "Show keybindings",
        "description": "Open this sheet from anywhere without leaving your current context.",
        "shortcutLabel": if is_mac(&platform) { "⌘/" } else { "Ctrl+/" },
    })];

    let available = available_now_defs();
    let avail_refs: Vec<&Def> = available.iter().collect();
    current_entries.extend(definitions_to_entries(
        &avail_refs,
        &keybindings,
        opts,
        &platform,
    )?);
    if let Some(entry) = definition_to_entry(&SIDEBAR_TOGGLE, &keybindings, opts, &platform)? {
        current_entries.insert(1, entry);
    }

    let two_open = options
        .get("context")
        .and_then(|c| c.get("terminalWorkspaceOpen"))
        .is_some_and(truthy);
    let thread_defs = thread_jump_defs();
    let thread_refs: Vec<&Def> = thread_defs.iter().collect();
    let ws_refs: Vec<&Def> = WORKSPACE_DEFS.iter().collect();
    let nav_entries = if two_open {
        definitions_to_entries(&ws_refs, &keybindings, opts, &platform)?
    } else {
        definitions_to_entries(&thread_refs, &keybindings, opts, &platform)?
    };
    current_entries.extend(nav_entries);

    sections.push(json!({
        "id": "available-now",
        "title": "Available now",
        "description": if two_open {
            "These reflect the active workspace-terminal context."
        } else {
            "These reflect the current chat and sidebar context."
        },
        "entries": current_entries.clone(),
    }));

    // alternate context
    let base_ctx = options.get("context").cloned().unwrap_or(Value::Null);
    let mut alt_ctx = match base_ctx.clone() {
        Value::Object(m) => m,
        _ => Map::new(),
    };
    if two_open {
        alt_ctx.insert("terminalWorkspaceOpen".into(), json!(false));
    } else {
        alt_ctx.insert("terminalOpen".into(), json!(true));
        alt_ctx.insert("terminalWorkspaceOpen".into(), json!(true));
    }
    let alt_options = json!({"context": Value::Object(alt_ctx)});
    let alternate = if two_open {
        definitions_to_entries(&thread_refs, &keybindings, Some(&alt_options), &platform)?
    } else {
        definitions_to_entries(&ws_refs, &keybindings, Some(&alt_options), &platform)?
    };
    if !alternate.is_empty() {
        sections.push(json!({
            "id": "alternate-context",
            "title": if two_open { "Outside workspace mode" } else { "In workspace mode" },
            "description": if two_open {
                "Number-row jumps return when the terminal workspace is closed."
            } else {
                "These bindings take over when the terminal switches into workspace mode."
            },
            "tone": "muted",
            "entries": alternate,
        }));
    }

    // project scripts — resolve with no context
    let project_scripts = match options.get("projectScripts") {
        Some(Value::Array(a)) => a.clone(),
        Some(Value::Null) => {
            return Err("null is not an object (evaluating 'options.projectScripts.map')".into())
        }
        None => Vec::new(),
        Some(_) => {
            return Err("options.projectScripts.map is not a function. (In 'options.projectScripts.map((script) => {', 'options.projectScripts.map' is undefined)".into())
        }
    };
    let mut script_entries: Vec<Value> = Vec::new();
    for script in &project_scripts {
        let id = match script.get("id") {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Null) => {
                return Err("null is not an object (evaluating 'script.id')".into());
            }
            None if script.is_null() => {
                return Err("null is not an object (evaluating 'script.id')".into());
            }
            _ => String::new(),
        };
        let command = format!("script.{}.run", id);
        if let Some(binding) =
            resolve_keybinding_for_command(&keybindings, &command, None, &platform)?
        {
            let name = script.get("name").cloned().unwrap_or(Value::Null);
            let setup = script.get("runOnWorktreeCreate").is_some_and(truthy);
            let name_str = name.as_str().unwrap_or("");
            script_entries.push(json!({
                "id": script.get("id").cloned().unwrap_or(Value::Null),
                "command": command,
                "binding": binding.clone(),
                "label": if setup { format!("{} setup script", name_str) } else { name_str.to_string() },
                "description": if setup {
                    "Run the project setup script directly from the keyboard."
                } else {
                    "Run this project script without opening the scripts menu."
                },
                "shortcutLabel": build_shortcut_text(binding.get("shortcut").unwrap_or(&Value::Null), &platform),
            }));
        }
    }
    if !script_entries.is_empty() {
        sections.push(json!({
            "id": "project-scripts",
            "title": "Project scripts",
            "description": "Custom shortcuts defined for the active project's scripts.",
            "entries": script_entries,
        }));
    }

    // code editor — context terminalFocus:false
    if NATIVE_EDITOR_ON {
        let mut ce_ctx = match base_ctx.clone() {
            Value::Object(m) => m,
            _ => Map::new(),
        };
        ce_ctx.insert("terminalFocus".into(), json!(false));
        let ce_options = json!({"context": Value::Object(ce_ctx)});
        let ce_refs: Vec<&Def> = CODE_EDITOR_DEFS.iter().collect();
        let ce_entries =
            definitions_to_entries(&ce_refs, &keybindings, Some(&ce_options), &platform)?;
        if !ce_entries.is_empty() {
            sections.push(json!({
                "id": "code-editor",
                "title": "Code editor",
                "description": "Available while a code editor has focus.",
                "tone": "muted",
                "entries": ce_entries,
            }));
        }
    }

    // composer context
    let effort_bound = current_entries
        .iter()
        .any(|e| e.get("command") == Some(&json!("model.effort.next")));
    if !effort_bound {
        let mut comp_ctx = match base_ctx {
            Value::Object(m) => m,
            _ => Map::new(),
        };
        comp_ctx.insert("composerFocus".into(), json!(true));
        comp_ctx.insert("terminalFocus".into(), json!(false));
        let comp_options = json!({"context": Value::Object(comp_ctx)});
        let comp_entries = definitions_to_entries(
            &[&EFFORT_CYCLE],
            &keybindings,
            Some(&comp_options),
            &platform,
        )?;
        if !comp_entries.is_empty() {
            sections.push(json!({
                "id": "composer-context",
                "title": "In composer",
                "description": "Available while the prompt composer or its effort picker has focus.",
                "tone": "muted",
                "entries": comp_entries,
            }));
        }
    }

    Ok(sections)
}

pub fn filter_shortcut_sheet_sections(
    sections: Value,
    query: String,
) -> Result<Vec<Value>, String> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return match sections {
            Value::Array(a) => Ok(a),
            Value::Null => Err("null is not an object (evaluating 'sections.map')".into()),
            v if is_undefined(&v) => {
                Err("undefined is not an object (evaluating 'sections.map')".into())
            }
            _ => Err("sections.map is not a function. (In 'sections.map((section) => ({\n    ...section,\n    entries: section.entries.filter((entry) => shortcutSheetEntryMatchesQuery(entry, trimmed))\n  }))', 'sections.map' is undefined)".into()),
        };
    }
    let sections = match &sections {
        Value::Array(a) => a.clone(),
        Value::Null => return Err("null is not an object (evaluating 'sections.map')".into()),
        v if is_undefined(v) => {
            return Err("undefined is not an object (evaluating 'sections.map')".into())
        }
        _ => return Err("sections.map is not a function. (In 'sections.map((section) => ({\n    ...section,\n    entries: section.entries.filter((entry) => shortcutSheetEntryMatchesQuery(entry, trimmed))\n  }))', 'sections.map' is undefined)".into()),
    };
    let mut out = Vec::new();
    for section in &sections {
        let Value::Object(map) = section else {
            return Err("undefined is not an object (evaluating 'section.entries.filter')".into());
        };
        let entries = match map.get("entries") {
            Some(Value::Array(a)) => a,
            Some(Value::Null) | None => {
                return Err(
                    "undefined is not an object (evaluating 'section.entries.filter')".into(),
                )
            }
            Some(_) => {
                return Err("section.entries.filter is not a function. (In 'section.entries.filter((entry) => shortcutSheetEntryMatchesQuery(entry, trimmed))', 'section.entries.filter' is undefined)".into())
            }
        };
        let mut kept = Vec::new();
        for entry in entries {
            let lower = |field: &str| -> Result<String, String> {
                match entry.get(field) {
                    Some(Value::String(s)) => Ok(s.to_lowercase()),
                    Some(Value::Null) | None => Err(format!(
                        "undefined is not an object (evaluating 'entry.{}.toLowerCase')",
                        field
                    )),
                    Some(_) => Err(format!(
                        "entry.{0}.toLowerCase is not a function. (In 'entry.{0}.toLowerCase()', 'entry.{0}.toLowerCase' is undefined)",
                        field
                    )),
                }
            };
            let label = lower("label")?;
            let description = lower("description")?;
            let shortcut = lower("shortcutLabel")?;
            if label.contains(&needle)
                || description.contains(&needle)
                || shortcut.contains(&needle)
            {
                kept.push(entry.clone());
            }
        }
        if !kept.is_empty() {
            let mut m = map.clone();
            m.insert("entries".into(), Value::Array(kept));
            out.push(Value::Object(m));
        }
    }
    Ok(out)
}
