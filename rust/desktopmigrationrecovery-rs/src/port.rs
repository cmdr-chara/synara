//! The Rust port of `apps/desktop/src/desktopMigrationRecovery.ts`.
//!
//! Detects pending desktop migration recovery and verifies the server-owned restore.

#![forbid(unsafe_code)]

#[path = "../../migrationrecovery-rs/src/port.rs"]
pub mod migrationrecovery_port;

use serde_json::{Value, json};
use std::fs;

fn is_undefined(v: &Value) -> bool {
    matches!(v, Value::Object(m) if m.get("$undefined") == Some(&Value::Bool(true)))
}

fn nullish(v: &Value) -> bool {
    v.is_null() || is_undefined(v)
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) => true,
        Value::Object(_) => !is_undefined(v),
    }
}

/// JS `obj[key]`: throws on null/undefined receivers, reads members on objects.
fn member(v: &Value, key: &str) -> Result<Value, String> {
    if v.is_null() {
        return Err(format!("Cannot read properties of null (reading '{key}')"));
    }
    if is_undefined(v) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        ));
    }
    Ok(match v {
        Value::Object(m) => m.get(key).cloned().unwrap_or_else(crate::js::undefined),
        _ => crate::js::undefined(),
    })
}

/// `Path.join` with Node's normalization: `.`/`..` resolve, `//` collapses, "" -> ".".
fn js_path_normalize(p: &str) -> String {
    let absolute = p.starts_with('/');
    let trailing = p.len() > 1 && p.ends_with('/');
    let mut out: Vec<&str> = Vec::new();
    for seg in p.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if matches!(out.last(), Some(&s) if s != "..") {
                    out.pop();
                } else if !absolute {
                    out.push("..");
                }
            }
            s => out.push(s),
        }
    }
    let mut joined = out.join("/");
    if absolute {
        joined.insert(0, '/');
    }
    if joined.is_empty() {
        joined = ".".to_string();
    } else if trailing && !joined.ends_with('/') {
        joined.push('/');
    }
    joined
}

/// `path.join` on string segments: empties drop out before normalizing.
fn js_join_str(parts: &[&str]) -> String {
    let segs: Vec<&&str> = parts.iter().filter(|p| !p.is_empty()).collect();
    js_path_normalize(&segs.iter().map(|s| **s).collect::<Vec<_>>().join("/"))
}

fn join(parts: &[&Value]) -> Result<String, String> {
    let mut segments: Vec<String> = Vec::new();
    for p in parts {
        match p {
            Value::String(s) if !s.is_empty() => segments.push(s.clone()),
            Value::String(_) => {}
            _ => {
                return Err(
                    "The \"path\" argument must be of type string. Received undefined".into(),
                );
            }
        }
    }
    Ok(js_path_normalize(&segments.join("/")))
}

/// Port of `resolveDesktopMigrationRecoveryPaths` (`apps/desktop/src/desktopMigrationRecovery.ts:26`).
pub fn resolve_desktop_migration_recovery_paths(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let base_dir = member(&input, "baseDir")?;
    let app_root = member(&input, "appRoot")?;
    let is_development = truthy(&member(&input, "isDevelopment")?);
    let state_dir = join(&[
        &base_dir,
        &Value::String(if is_development { "dev" } else { "userdata" }.to_string()),
    ])?;
    let db_path = js_join_str(&[&state_dir, "state.sqlite"]);
    let marker_path = migrationrecovery_port::migration_recovery_marker_path(db_path.clone())?;
    let provenance_path =
        migrationrecovery_port::migration_backup_provenance_path(db_path.clone())?;
    let restore_entry_path = match app_root {
        Value::String(s) => js_join_str(&[&s, "apps/server/dist/restoreMigrationBackup.mjs"]),
        _ => return Err("The \"path\" argument must be of type string. Received undefined".into()),
    };
    Ok(json!({
        "dbPath": db_path,
        "markerPath": marker_path,
        "provenancePath": provenance_path,
        "restoreEntryPath": restore_entry_path,
    }))
}

/// Port of `invalidMigrationStartupRecoveryChoices` (`apps/desktop/src/desktopMigrationRecovery.ts:59`).
pub fn invalid_migration_startup_recovery_choices(
    input: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    let mut choices = Vec::new();
    if truthy(&member(&input, "canInstallUpdate")?) {
        choices.push(json!({ "label": "Update Synara and restart", "decision": "install-update" }));
    }
    if truthy(&member(&input, "canOpenReleasePage")?) {
        choices
            .push(json!({ "label": "Download latest release", "decision": "open-release-page" }));
    }
    choices.push(json!({ "label": "Open logs", "decision": "open-logs" }));
    choices.push(json!({ "label": "Quit", "decision": "quit" }));
    Ok(choices)
}

/// Port of `hasPendingDesktopMigrationRecovery` (`apps/desktop/src/desktopMigrationRecovery.ts:172`).
pub fn has_pending_desktop_migration_recovery(paths: serde_json::Value) -> Result<bool, String> {
    let marker = member(&paths, "markerPath")?;
    let Value::String(marker) = marker else {
        return Ok(false);
    };
    Ok(fs::metadata(marker).is_ok())
}

/// Port of `hasVerifiedDesktopMigrationRestore` (`apps/desktop/src/desktopMigrationRecovery.ts:176`).
pub fn has_verified_desktop_migration_restore(
    paths: serde_json::Value,
    expected_backup_path: String,
) -> Result<bool, String> {
    let provenance_path = match member(&paths, "provenancePath") {
        Ok(v) => v,
        Err(_) => return Ok(false),
    };
    let Value::String(provenance_path) = provenance_path else {
        return Ok(false);
    };
    let Ok(text) = fs::read_to_string(provenance_path) else {
        return Ok(false);
    };
    let Ok(provenance) = serde_json::from_str::<Value>(&text) else {
        return Ok(false);
    };
    let db_path = match member(&paths, "dbPath") {
        Ok(v) => v,
        Err(_) => return Ok(false),
    };
    Ok(member(&provenance, "databasePath")? == db_path
        && member(&provenance, "backupPath")? == Value::String(expected_backup_path)
        && member(&provenance, "phase")? == Value::String("migration-restored".into())
        && member(&provenance, "restoredAt")
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .is_some_and(|s| !s.is_empty()))
}

/// Port of `resolveDesktopMigrationRestoreCandidate` (`apps/desktop/src/desktopMigrationRecovery.ts:206`).
pub fn resolve_desktop_migration_restore_candidate(
    paths: serde_json::Value,
    block: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    if member(&block, "databasePath")? != member(&paths, "dbPath")? {
        return Ok(None);
    }
    let recovery = member(&block, "recovery")?;
    if member(&recovery, "kind")? != Value::String("restore-available".into()) {
        return Ok(None);
    }
    if member(&recovery, "provenancePath")? != member(&paths, "provenancePath")? {
        return Ok(None);
    }
    Ok(Some(recovery))
}

/// Port of `requiresDesktopMigrationRecovery` (`apps/desktop/src/desktopMigrationRecovery.ts:226`).
pub fn requires_desktop_migration_recovery(paths: serde_json::Value) -> Result<bool, String> {
    let marker = match member(&paths, "markerPath") {
        Ok(v) => v,
        Err(_) => return Ok(true),
    };
    let Value::String(marker) = marker else {
        return Ok(true);
    };
    let text = match fs::read_to_string(marker) {
        Ok(t) => t,
        Err(e) => return Ok(e.raw_os_error() != Some(2)),
    };
    let parsed = migrationrecovery_port::parse_migration_recovery_resume_state(text)?;
    Ok(match parsed.as_ref().and_then(|p| p.get("exhausted")) {
        Some(v) if !nullish(v) => truthy(v),
        _ => true,
    })
}
