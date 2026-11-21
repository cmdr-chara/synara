//! The Rust port of `apps/web/src/projectRunTargets.ts`.

#![forbid(unsafe_code)]

#[path = "../../projectscripts-rs/src/port.rs"]
pub mod projectscripts_port;

use serde_json::{Map, Value};

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

fn member(v: &Value, key: &str) -> Result<Value, String> {
    match v {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        _ if is_undefined(v) => Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        )),
        Value::Object(m) => Ok(m.get(key).cloned().unwrap_or_else(crate::js::undefined)),
        _ => Ok(crate::js::undefined()),
    }
}

fn truthy(v: &Value) -> bool {
    if let Some(n) = crate::js::as_number(v) {
        return n != 0.0 && !n.is_nan();
    }
    match v {
        Value::Null | Value::Bool(false) => false,
        Value::Bool(true) => true,
        Value::String(s) => !s.is_empty(),
        Value::Object(_) => !is_undefined(v),
        Value::Array(_) => true,
        Value::Number(_) => false,
    }
}

fn find_non_setup_script(scripts: &Value) -> Result<Option<Value>, String> {
    match scripts {
        Value::Null => Err("Cannot read properties of null (reading 'find')".to_string()),
        _ if is_undefined(scripts) => {
            Err("Cannot read properties of undefined (reading 'find')".to_string())
        }
        Value::Array(items) => {
            for s in items {
                if !truthy(&member(s, "runOnWorktreeCreate")?) {
                    return Ok(Some(s.clone()));
                }
            }
            Ok(None)
        }
        _ => Err("scripts.find is not a function".to_string()),
    }
}

/// Port of `selectPrimaryProjectRunCommand` (`apps/web/src/projectRunTargets.ts:38`).
pub fn select_primary_project_run_command(
    input: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let project = member(&input, "project")?;
    let scripts = member(&project, "scripts")?;
    if let Some(script) = find_non_setup_script(&scripts)?
        && !truthy(&member(&script, "runOnWorktreeCreate")?)
    {
        let mut out = Map::new();
        out.insert("source".to_string(), Value::String("saved".to_string()));
        out.insert("label".to_string(), member(&script, "name")?);
        out.insert("command".to_string(), member(&script, "command")?);
        out.insert("cwd".to_string(), member(&project, "cwd")?);
        out.insert("script".to_string(), script);
        return Ok(Some(Value::Object(out)));
    }

    if let Value::Array(targets) = member(&input, "discoveredTargets")? {
        for script_name in ["dev", "start"] {
            for target in &targets {
                let tscripts = member(target, "scripts")?;
                let Value::Array(entries) = &tscripts else {
                    return Err("target.scripts.find is not a function".to_string());
                };
                if let Some(script) = entries
                    .iter()
                    .find(|s| member(s, "name").ok() == Some(Value::String(script_name.into())))
                {
                    let rel = member(target, "relativePath")?;
                    let pkg = member(target, "packageName")?;
                    let rel_s = rel.as_str().unwrap_or("");
                    let package_label = if rel_s.is_empty() {
                        pkg.as_str().unwrap_or("")
                    } else {
                        rel_s
                    };
                    let label = if package_label.is_empty() {
                        script_name.to_string()
                    } else {
                        format!("{package_label} {script_name}")
                    };
                    let mut out = Map::new();
                    out.insert(
                        "source".to_string(),
                        Value::String("discovered".to_string()),
                    );
                    out.insert("label".to_string(), Value::String(label));
                    out.insert("command".to_string(), member(script, "command")?);
                    out.insert("cwd".to_string(), member(target, "cwd")?);
                    out.insert("packageRelativePath".to_string(), rel);
                    out.insert(
                        "scriptName".to_string(),
                        Value::String(script_name.to_string()),
                    );
                    return Ok(Some(Value::Object(out)));
                }
            }
        }
    }
    Ok(None)
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| {
        matches!(
            c,
            '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
                ..='\u{200a}'
                    | '\u{2028}'
                    | '\u{2029}'
                    | '\u{202f}'
                    | '\u{205f}'
                    | '\u{3000}'
                    | '\u{feff}'
        )
    })
}

/// Port of `upsertProjectRunCommandScripts` (`apps/web/src/projectRunTargets.ts:79`).
pub fn upsert_project_run_command_scripts(
    input: serde_json::Value,
) -> Result<Option<Vec<serde_json::Value>>, String> {
    let command_v = member(&input, "command")?;
    let command = match &command_v {
        Value::String(s) => js_trim(s).to_string(),
        _ if command_v.is_null() => {
            return Err("Cannot read properties of null (reading 'trim')".to_string());
        }
        _ if is_undefined(&command_v) => {
            return Err("Cannot read properties of undefined (reading 'trim')".to_string());
        }
        _ => return Err("input.command.trim is not a function".to_string()),
    };
    if command.is_empty() {
        return Ok(None);
    }
    let scripts = member(&input, "scripts")?;
    if let Some(script) = find_non_setup_script(&scripts)?
        && !truthy(&member(&script, "runOnWorktreeCreate")?)
    {
        if member(&script, "command")? == Value::String(command.clone()) {
            return Ok(None);
        }
        let Value::Array(items) = &scripts else {
            return Err("input.scripts.map is not a function".to_string());
        };
        let existing_id = member(&script, "id")?;
        let mut out = Vec::with_capacity(items.len());
        for s in items {
            if member(s, "id")? == existing_id {
                let mut m = match s {
                    Value::Object(m) => m.clone(),
                    _ => Map::new(),
                };
                m.insert("command".to_string(), Value::String(command.clone()));
                out.push(Value::Object(m));
            } else {
                out.push(s.clone());
            }
        }
        return Ok(Some(out));
    }
    let Value::Array(items) = &scripts else {
        return Err("input.scripts.map is not a function".to_string());
    };
    let ids: Vec<Value> = items
        .iter()
        .map(|s| member(s, "id").unwrap_or_else(|_| crate::js::undefined()))
        .collect();
    let id = projectscripts_port::next_project_script_id("dev".to_string(), Value::Array(ids))?;
    let mut run_script = Map::new();
    run_script.insert("id".to_string(), Value::String(id));
    run_script.insert("name".to_string(), Value::String("dev".to_string()));
    run_script.insert("command".to_string(), Value::String(command));
    run_script.insert("icon".to_string(), Value::String("play".to_string()));
    run_script.insert("runOnWorktreeCreate".to_string(), Value::Bool(false));
    let mut out = items.clone();
    out.push(Value::Object(run_script));
    Ok(Some(out))
}
