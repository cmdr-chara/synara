//! shell.openInEditor — open a directory in an external editor, detached.
//! Mirrors open.ts: prefer a PATH-resolvable editor command, then the macOS
//! application via `open -a`, then fail with the editor's own error message.

use serde_json::Value;

struct Editor {
    id: &'static str,
    commands: &'static [&'static str],
    mac_app: &'static str,
}

const EDITORS: &[Editor] = &[
    Editor {
        id: "cursor",
        commands: &["cursor"],
        mac_app: "Cursor",
    },
    Editor {
        id: "trae",
        commands: &["trae"],
        mac_app: "Trae",
    },
    Editor {
        id: "vscode",
        commands: &["code"],
        mac_app: "Visual Studio Code",
    },
];

fn on_path(cmd: &str) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(cmd);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if candidate.is_file()
                && std::fs::metadata(&candidate)
                    .map(|m| m.permissions().mode() & 0o111 != 0)
                    .unwrap_or(false)
            {
                return Some(candidate.to_string_lossy().into_owned());
            }
        }
        #[cfg(not(unix))]
        if candidate.is_file() {
            return Some(candidate.to_string_lossy().into_owned());
        }
    }
    None
}

fn spawn_detached(command: &str, args: &[String]) -> Result<(), String> {
    std::process::Command::new(command)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("failed to spawn detached process: {e}"))
}

pub fn open_in_editor(payload: &Value) -> Result<Value, crate::todo::WsErr> {
    let editor = payload
        .get("editor")
        .and_then(|v| v.as_str())
        .unwrap_or("vscode");
    let cwd = payload.get("cwd").and_then(|v| v.as_str()).unwrap_or("");

    if cwd.is_empty() {
        return Err(crate::todo::err_pub("Missing required field: cwd"));
    }

    if editor == "system-default" {
        return spawn_detached("open", &[cwd.to_string()])
            .map(|()| Value::Null)
            .map_err(crate::todo::err_pub);
    }

    let Some(def) = EDITORS.iter().find(|e| e.id == editor) else {
        return Err(crate::todo::err_pub(format!(
            "Unsupported editor: {editor}"
        )));
    };

    for cmd in def.commands {
        if let Some(bin) = on_path(cmd) {
            // launchStyle "goto" with a bare directory opens the directory.
            return spawn_detached(&bin, &[cwd.to_string()])
                .map(|()| Value::Null)
                .map_err(crate::todo::err_pub);
        }
    }

    #[cfg(target_os = "macos")]
    {
        if on_path("open").is_some() {
            return spawn_detached(
                "open",
                &["-a".to_string(), def.mac_app.to_string(), cwd.to_string()],
            )
            .map(|()| Value::Null)
            .map_err(crate::todo::err_pub);
        }
    }

    let first_cmd = def.commands.first().copied().unwrap_or("open");
    Err(crate::todo::err_pub(format!(
        "Editor command not found: {first_cmd}"
    )))
}
