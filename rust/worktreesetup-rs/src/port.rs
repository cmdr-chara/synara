//! The Rust port of `apps/server/src/worktreeSetup.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

const WORKTREE_SETUP_TIMEOUT_MS: u64 = 10 * 60_000;

fn js_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map(|x| x != 0.0 && !x.is_nan()).unwrap_or(false),
        Value::String(s) => !s.is_empty(),
        _ if crate::js::is_undefined(v) => false,
        _ if crate::js::as_number(v).is_some() => {
            let n = crate::js::as_number(v).unwrap_or(0.0);
            n != 0.0 && !n.is_nan()
        }
        _ => true,
    }
}

fn find_script(scripts: &Value) -> Result<Option<Value>, String> {
    match scripts {
        Value::Null => Err("Cannot read properties of null (reading 'find')".to_string()),
        _ if crate::js::is_undefined(scripts) => {
            Err("Cannot read properties of undefined (reading 'find')".to_string())
        }
        Value::Array(items) => Ok(items
            .iter()
            .find(|s| s.get("runOnWorktreeCreate").map(js_truthy).unwrap_or(false))
            .cloned()),
        _ => Err("scripts.find is not a function".to_string()),
    }
}

/// Port of `findWorktreeSetupScript` (`apps/server/src/worktreeSetup.ts:7`).
pub fn find_worktree_setup_script(scripts: Value) -> Result<Option<Value>, String> {
    find_script(&scripts)
}

/// Port of `runWorktreeSetupScript` (`apps/server/src/worktreeSetup.ts:14`).
pub async fn run_worktree_setup_script(
    scripts: Value,
    cwd: String,
    signal: Option<Value>,
) -> Result<(), String> {
    let _ = &signal;
    let Some(script) = find_script(&scripts)? else {
        return Ok(());
    };
    let command = script
        .get("command")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    let status = std::process::Command::new(&shell)
        .arg("-lc")
        .arg(&command)
        .current_dir(&cwd)
        .env(
            "SYNARA_RUN_TIMEOUT_MS",
            WORKTREE_SETUP_TIMEOUT_MS.to_string(),
        )
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("Command failed with exit code {:?}", status.code()))
    }
}
