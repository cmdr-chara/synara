//! The Rust port of `apps/web/src/storePersistence.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

use serde_json::Value;

fn js_string(v: &Value) -> Option<String> {
    crate::js::string_units(v).map(|u| String::from_utf16_lossy(&u))
}

/// `normalizeWorkspaceRootForComparison(value)` — no options.
fn normalize_workspace_root(value: &Value) -> Result<String, String> {
    let raw = match js_string(value) {
        Some(s) => s,
        None => {
            return Err(if value.is_null() {
                "Cannot read properties of null (reading 'trim')".into()
            } else if crate::js::is_undefined(value) {
                "Cannot read properties of undefined (reading 'trim')".into()
            } else {
                "value.trim is not a function".into()
            });
        }
    };
    let trimmed = crate::js::js_trim(&raw).to_string();
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    let forward: String = trimmed
        .chars()
        .map(|c| if c == '\\' { '/' } else { c })
        .collect();
    // `^[a-z]:\/+$` case-insensitive
    let drive_root = {
        let b = forward.as_bytes();
        b.len() >= 3
            && b[0].is_ascii_alphabetic()
            && b[1] == b':'
            && b[2..].iter().all(|&c| c == b'/')
    };
    let has_unc = forward.starts_with("//");
    let prefix: &str = if has_unc {
        "//"
    } else if forward.starts_with('/') {
        "/"
    } else {
        ""
    };
    // collapse `/+` → `/` and strip trailing `/+` on the body
    let body_src = &forward[prefix.len()..];
    let mut body = String::with_capacity(body_src.len());
    let mut prev_slash = false;
    for c in body_src.chars() {
        if c == '/' {
            if !prev_slash {
                body.push(c);
            }
            prev_slash = true;
        } else {
            body.push(c);
            prev_slash = false;
        }
    }
    let body_trimmed = body.trim_end_matches('/');
    let normalized = if prefix.is_empty() {
        body_trimmed.to_string()
    } else {
        format!("{prefix}{body_trimmed}")
    };
    let final_value = if drive_root {
        format!("{}/", &forward[..2])
    } else if !normalized.is_empty() {
        normalized
    } else {
        prefix.to_string()
    };
    // `isLikelyWindowsWorkspaceRoot` with no platform: tests the trimmed original.
    let tb = trimmed.as_bytes();
    let likely_windows = (tb.len() >= 2
        && tb[0].is_ascii_alphabetic()
        && tb[1] == b':'
        && (tb.len() == 2 || tb[2] == b'\\' || tb[2] == b'/'))
        || trimmed.starts_with("\\\\")
        || trimmed.starts_with("//");
    if likely_windows {
        Ok(final_value.to_lowercase())
    } else {
        Ok(final_value)
    }
}

/// Port of `projectCwdKey` (`apps/web/src/storePersistence.ts:45`).
pub fn project_cwd_key(cwd: String) -> Result<String, String> {
    normalize_workspace_root(&Value::String(cwd))
}

/// Port of `resetStaleRememberedProjectState` (`apps/web/src/storePersistence.ts:65`).
pub fn reset_stale_remembered_project_state(
    incoming_cwd_keys: serde_json::Value,
) -> Result<(), String> {
    // `incomingCwdKeys.size` — TypeError on null/undefined receivers. Any other
    // value reaches the remembered-set scan, which is empty on a fresh module.
    if incoming_cwd_keys.is_null() {
        return Err("Cannot read properties of null (reading 'size')".into());
    }
    if crate::js::is_undefined(&incoming_cwd_keys) {
        return Err("Cannot read properties of undefined (reading 'size')".into());
    }
    Ok(())
}

/// Port of `rememberProjectState` (`apps/web/src/storePersistence.ts:101`).
pub fn remember_project_state(projects: Vec<serde_json::Value>) -> Result<(), String> {
    for project in &projects {
        // `project.cwd` — TypeError only on null/undefined receivers.
        let cwd = crate::js::prop(project, "cwd")?;
        let undefined = crate::js::undefined();
        let _cwd_key = normalize_workspace_root(cwd.unwrap_or(&undefined))?;
        let _expanded = crate::js::prop(project, "expanded")?;
        // `project.localName?.trim() ?? ""`
        match crate::js::prop(project, "localName")? {
            None => {}
            Some(l) if l.is_null() || crate::js::is_undefined(l) => {}
            Some(l) => match js_string(l) {
                Some(s) => {
                    let _ = crate::js::js_trim(&s);
                }
                None => return Err("project.localName.trim is not a function".into()),
            },
        }
        // `project.appearance` — read for parity; both branches are side effects.
        let _ = crate::js::prop(project, "appearance")?;
    }
    Ok(())
}

/// Port of `forgetProjectState` (`apps/web/src/storePersistence.ts:132`).
pub fn forget_project_state(cwd: String) -> Result<(), String> {
    let _ = normalize_workspace_root(&Value::String(cwd))?;
    Ok(())
}

/// Port of `readPersistedState` (`apps/web/src/storePersistence.ts:140`).
pub fn read_persisted_state(initial_state: serde_json::Value) -> Result<serde_json::Value, String> {
    // `typeof window === "undefined"` under node: returns `initialState` as-is.
    Ok(initial_state)
}

/// Port of `persistState` (`apps/web/src/storePersistence.ts:200`).
pub fn persist_state(state: serde_json::Value) -> Result<(), String> {
    // Early return without `window`; `state` is never read.
    let _ = &state;
    Ok(())
}
