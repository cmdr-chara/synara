//! The Rust port of `packages/shared/src/threadWorkspace.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

use crate::js;

const SCRATCH_DIR: &str = "synara-codex-workspaces";

fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t'..='\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}'
            | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'
    )
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_space)
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0 && !f.is_nan()),
        Value::String(s) => !s.is_empty(),
        _ if js::is_undefined(v) => false,
        _ => true,
    }
}

fn member(v: &Value, k: &str) -> Value {
    match v {
        Value::Object(o) => o.get(k).cloned().unwrap_or_else(js::undefined),
        _ => js::undefined(),
    }
}

/// `input` member access for the derive functions: `null` throws first.
fn input_member(input: &Value, k: &str) -> Result<Value, String> {
    if input.is_null() {
        return Err(format!("Cannot read properties of null (reading '{k}')"));
    }
    Ok(member(input, k))
}

/// `/^[a-z]:([\\/]|$)/i` on `value`, `\\` or `//` prefix.
fn is_likely_windows_root(value: &str, platform: &Value) -> bool {
    if platform.as_str() == Some("win32") {
        return true;
    }
    if truthy(platform) && platform.as_str() != Some("win32") {
        return false;
    }
    let b = value.as_bytes();
    (b.len() >= 2
        && b[0].is_ascii_alphabetic()
        && b[1] == b':'
        && (b.len() == 2 || b[2] == b'\\' || b[2] == b'/'))
        || value.starts_with("\\\\")
        || value.starts_with("//")
}

fn normalize_root(value: &str, platform: &Value) -> String {
    let trimmed = js_trim(value);
    if trimmed.is_empty() {
        return String::new();
    }
    let fwd = trimmed.replace('\\', "/");
    // `/^[a-z]:\/+$/i`
    let fb = fwd.as_bytes();
    let is_drive_root = fb.len() >= 3
        && fb[0].is_ascii_alphabetic()
        && fb[1] == b':'
        && fb[2..].iter().all(|&c| c == b'/');
    let has_unc = fwd.starts_with("//");
    let prefix = if has_unc {
        "//"
    } else if fwd.starts_with('/') {
        "/"
    } else {
        ""
    };
    let mut body = String::with_capacity(fwd.len());
    let mut prev_slash = false;
    for c in fwd[prefix.len()..].chars() {
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
    let body = body.trim_end_matches('/');
    let normalized = if !prefix.is_empty() {
        format!("{prefix}{body}")
    } else {
        body.to_string()
    };
    let mut final_value = if is_drive_root {
        format!("{}/", &fwd[..2])
    } else if !normalized.is_empty() {
        normalized
    } else {
        prefix.to_string()
    };

    if platform.as_str() == Some("darwin")
        && (final_value.starts_with("/private/var/") || final_value.starts_with("/private/tmp/"))
    {
        final_value = final_value["/private".len()..].to_string();
    }

    if is_likely_windows_root(trimmed, platform) {
        final_value.to_lowercase()
    } else {
        final_value
    }
}

fn options_platform(options: &Option<Value>) -> Value {
    match options {
        Some(o) => member(o, "platform"),
        None => js::undefined(),
    }
}

/// Port of `normalizeWorkspaceRootForComparison` (`packages/shared/src/threadWorkspace.ts:33`).
pub fn normalize_workspace_root_for_comparison(
    value: String,
    options: Option<serde_json::Value>,
) -> Result<String, String> {
    Ok(normalize_root(&value, &options_platform(&options)))
}

/// Port of `workspaceRootsEqual` (`packages/shared/src/threadWorkspace.ts:72`).
pub fn workspace_roots_equal(
    left: String,
    right: String,
    options: Option<serde_json::Value>,
) -> Result<bool, String> {
    let platform = options_platform(&options);
    Ok(normalize_root(&left, &platform) == normalize_root(&right, &platform))
}

/// Port of `isWorkspaceRootWithin` (`packages/shared/src/threadWorkspace.ts:87`).
pub fn is_workspace_root_within(
    candidate: String,
    ancestor_root: String,
    options: Option<serde_json::Value>,
) -> Result<bool, String> {
    let platform = options_platform(&options);
    let candidate_n = normalize_root(&candidate, &platform);
    let ancestor_n = normalize_root(&ancestor_root, &platform);
    if candidate_n.is_empty() || ancestor_n.is_empty() {
        return Ok(false);
    }
    if candidate_n == ancestor_n {
        return Ok(true);
    }
    let prefix = if ancestor_n.ends_with('/') {
        ancestor_n
    } else {
        format!("{ancestor_n}/")
    };
    Ok(candidate_n.starts_with(&prefix))
}

/// Port of `isScratchWorkspacePath` (`packages/shared/src/threadWorkspace.ts:113`).
pub fn is_scratch_workspace_path(file_path: String) -> Result<bool, String> {
    let normalized = js_trim(&file_path).replace('\\', "/");
    let b = normalized.as_bytes();
    let is_absolute = normalized.starts_with('/')
        || (b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && b[2] == b'/');
    Ok(is_absolute && normalized.contains(&format!("/{SCRATCH_DIR}/")))
}

fn is_undef(v: &Value) -> bool {
    js::is_undefined(v)
}

fn nullish_or(v: &Value, default: Value) -> Value {
    if v.is_null() || is_undef(v) {
        default
    } else {
        v.clone()
    }
}

/// Port of `deriveAssociatedWorktreeMetadata` (`packages/shared/src/threadWorkspace.ts:119`).
pub fn derive_associated_worktree_metadata(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let awp = input_member(&input, "associatedWorktreePath")?;
    let awb = input_member(&input, "associatedWorktreeBranch")?;
    let awr = input_member(&input, "associatedWorktreeRef")?;
    let branch = input_member(&input, "branch")?;
    let worktree = input_member(&input, "worktreePath")?;

    let derived_branch = if truthy(&worktree) {
        nullish_or(&branch, Value::Null)
    } else {
        Value::Null
    };
    let path = if !is_undef(&awp) {
        awp
    } else {
        nullish_or(&worktree, Value::Null)
    };
    let branch_out = if !is_undef(&awb) {
        awb.clone()
    } else {
        derived_branch.clone()
    };
    let ref_out = if !is_undef(&awr) {
        awr
    } else if !is_undef(&awb) {
        awb
    } else {
        derived_branch
    };
    Ok(serde_json::json!({
        "associatedWorktreePath": path,
        "associatedWorktreeBranch": branch_out,
        "associatedWorktreeRef": ref_out,
    }))
}

/// Port of `deriveAssociatedWorktreeMetadataPatch` (`packages/shared/src/threadWorkspace.ts:152`).
pub fn derive_associated_worktree_metadata_patch(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let awp = input_member(&input, "associatedWorktreePath")?;
    let awb = input_member(&input, "associatedWorktreeBranch")?;
    let awr = input_member(&input, "associatedWorktreeRef")?;
    let branch = input_member(&input, "branch")?;
    let worktree = input_member(&input, "worktreePath")?;
    let worktree_set = !is_undef(&worktree) && !worktree.is_null();

    let mut patch = serde_json::Map::new();
    if !is_undef(&awp) {
        patch.insert("associatedWorktreePath".into(), awp);
    } else if worktree_set {
        patch.insert("associatedWorktreePath".into(), worktree.clone());
    }
    if !is_undef(&awb) {
        patch.insert("associatedWorktreeBranch".into(), awb.clone());
    } else if worktree_set {
        patch.insert(
            "associatedWorktreeBranch".into(),
            nullish_or(&branch, Value::Null),
        );
    }
    if !is_undef(&awr) {
        patch.insert("associatedWorktreeRef".into(), awr);
    } else if !is_undef(&awb) {
        patch.insert("associatedWorktreeRef".into(), awb);
    } else if worktree_set {
        patch.insert(
            "associatedWorktreeRef".into(),
            nullish_or(&branch, Value::Null),
        );
    }
    Ok(Value::Object(patch))
}
