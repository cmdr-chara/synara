//! The Rust port of `apps/server/src/betaImport.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

use serde_json::Value;

fn nullish(v: &Value) -> bool {
    v.is_null() || crate::js::is_undefined(v)
}

fn prop_or_err(v: &Value, key: &str) -> Result<Value, String> {
    if crate::js::is_undefined(v) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        ));
    }
    match v {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        Value::Object(o) => Ok(o.get(key).cloned().unwrap_or_else(crate::js::undefined)),
        _ => Ok(crate::js::undefined()),
    }
}

fn resolve_path(p: &str) -> String {
    let p = if p.starts_with('~') {
        p.replacen('~', "/Users/devin", 1)
    } else {
        p.to_string()
    };
    if p.starts_with('/') {
        normalize(&p)
    } else {
        normalize(&format!("/Users/devin/repos/synara/{p}"))
    }
}

fn normalize(p: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    for part in p.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                out.pop();
            }
            _ => out.push(part),
        }
    }
    format!("/{}", out.join("/"))
}

/// Port of `allowedImportSourceHomes` (`apps/server/src/betaImport.ts:343`).
pub fn allowed_import_source_homes(env: serde_json::Value) -> Result<Vec<String>, String> {
    let env = if crate::js::is_undefined(&env) {
        serde_json::json!({})
    } else {
        env
    };
    let v = prop_or_err(&env, "SYNARA_STABLE_HOME")?;
    let trimmed = if nullish(&v) {
        None
    } else {
        match &v {
            serde_json::Value::String(s) => Some(s.trim().to_string()),
            _ => return Err("env[SYNARA_STABLE_HOME]?.trim is not a function".to_string()),
        }
    };
    let home = match trimmed {
        Some(t) if !t.is_empty() => resolve_path(&t),
        _ => "/Users/devin/.synara".to_string(),
    };
    Ok(vec![home])
}
