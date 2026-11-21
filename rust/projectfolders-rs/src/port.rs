//! The Rust port of `packages/shared/src/projectFolders.ts`.
//!
//! Clean-room implementation (ADR-001): written from the recorded behavior in
//! `cases.jsonl` and the exported signatures, not from the source body.

#![forbid(unsafe_code)]

use serde_json::Value;

fn is_undefined(v: &Value) -> bool {
    matches!(v, Value::Object(m) if m.get("$undefined").and_then(Value::as_bool) == Some(true))
}

/// `String(v)` for the values these functions coerce into messages.
fn js_string(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => {
            let f = n.as_f64().unwrap_or(f64::NAN);
            if f.fract() == 0.0 && f.abs() < 9e15 {
                format!("{}", f as i64)
            } else {
                format!("{}", f)
            }
        }
        Value::String(s) => s.clone(),
        Value::Object(m) if is_undefined(v) => {
            let _ = m;
            "undefined".into()
        }
        _ => "[object Object]".into(),
    }
}

fn js_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
        Value::String(s) => !s.is_empty(),
        Value::Object(_) if is_undefined(v) => false,
        _ => true,
    }
}

/// Object property read; `None` = JS `undefined`.
fn field<'a>(v: &'a Value, name: &str) -> Option<&'a Value> {
    match v {
        Value::Object(m) if !is_undefined(v) => m.get(name),
        _ => None,
    }
}

fn is_absolute(p: &str) -> bool {
    let b = p.as_bytes();
    if b.first() == Some(&b'/') || b.first() == Some(&b'\\') {
        return true;
    }
    b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b[2] == b'\\' || b[2] == b'/')
}

fn normalize(p: &str) -> &str {
    p.trim_end_matches(['/', '\\'])
}

fn basename(p: &str) -> &str {
    normalize(p).rsplit(['/', '\\']).next().unwrap_or(p)
}

/// Path segments split on `/` and `\`.
fn segments(p: &str) -> Vec<&str> {
    normalize(p).split(['/', '\\']).collect()
}

/// Label a path by its shortest unambiguous trailing segment count.
fn derive_labels(paths: &[String]) -> Vec<String> {
    let segs: Vec<Vec<&str>> = paths.iter().map(|p| segments(p)).collect();
    (0..paths.len())
        .map(|i| {
            for k in 1..=segs[i].len() {
                let suffix = &segs[i][segs[i].len() - k..];
                let unique = (0..paths.len()).all(|j| {
                    j == i || segs[j].len() < k || segs[j][segs[j].len() - k..] != *suffix
                });
                if unique {
                    return suffix.join("/");
                }
            }
            segs[i].join("/")
        })
        .collect()
}

/// Provider kind id → display name ("opencode" → "OpenCode").
fn display_name(kind: &str) -> &str {
    match kind {
        "codex" => "Codex",
        "claudeAgent" => "Claude",
        "cursor" => "Cursor",
        "antigravity" => "Antigravity",
        "grok" => "Grok",
        "droid" => "Droid",
        "opencode" => "OpenCode",
        "pi" => "Pi",
        "devin" => "Devin",
        "omp" => "Omp",
        other => other,
    }
}

/// Port of `projectFoldersSessionIssue` (`packages/shared/src/projectFolders.ts:21`).
pub fn project_folders_session_issue(input: serde_json::Value) -> Result<Option<String>, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'provider')".into());
    }
    if is_undefined(&input) {
        return Err("Cannot read properties of undefined (reading 'provider')".into());
    }
    let provider = match field(&input, "provider") {
        Some(v) if v.as_str() == Some("codex") || v.as_str() == Some("claudeAgent") => {
            return Ok(None);
        }
        Some(v) => v
            .as_str()
            .map(display_name)
            .map(str::to_string)
            .unwrap_or_else(|| js_string(v)),
        None => "undefined".to_string(),
    };
    Ok(Some(format!(
        "{provider} cannot access a project's additional folders. Use Codex or Claude for this multi-folder project."
    )))
}

/// Port of `findProjectFolderProblem` (`packages/shared/src/projectFolders.ts:42`).
pub fn find_project_folder_problem(
    paths: Vec<String>,
    options: Option<serde_json::Value>,
) -> Result<Option<String>, String> {
    let _ = options; // platform selection accepted; both separators are checked.
    if paths.is_empty() {
        return Ok(Some("Add at least one folder.".into()));
    }
    for p in &paths {
        if !is_absolute(p) {
            return Ok(Some(format!("Use an absolute path: {p}")));
        }
    }
    let norm: Vec<&str> = paths.iter().map(|p| normalize(p)).collect();
    for i in 0..norm.len() {
        for j in (i + 1)..norm.len() {
            if norm[i] == norm[j] {
                return Ok(Some(format!(
                    "{} is already in this project.",
                    basename(norm[i])
                )));
            }
            for (inner, outer) in [(i, j), (j, i)] {
                for sep in ['/', '\\'] {
                    if norm[inner].starts_with(&format!("{}{}", norm[outer], sep)) {
                        return Ok(Some(format!(
                            "{} is inside {}. Add only one of them.",
                            basename(norm[inner]),
                            basename(norm[outer])
                        )));
                    }
                }
            }
        }
    }
    Ok(None)
}

/// Port of `deriveProjectFolderLabels` (`packages/shared/src/projectFolders.ts:73`).
pub fn derive_project_folder_labels(paths: Vec<String>) -> Result<Vec<String>, String> {
    Ok(derive_labels(&paths))
}

/// Port of `buildProjectFoldersPreamble` (`packages/shared/src/projectFolders.ts:96`).
pub fn build_project_folders_preamble(input: serde_json::Value) -> Result<Option<String>, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'additionalFolders')".into());
    }
    if is_undefined(&input) {
        return Err("Cannot read properties of undefined (reading 'additionalFolders')".into());
    }
    let additional = field(&input, "additionalFolders")
        .cloned()
        .unwrap_or(Value::Object(serde_json::Map::from_iter([(
            "$undefined".to_string(),
            Value::Bool(true),
        )])));
    if is_undefined(&additional) {
        return Err("Cannot read properties of undefined (reading 'length')".into());
    }
    if additional.is_null() {
        return Err("Cannot read properties of null (reading 'length')".into());
    }
    let Value::Array(folders) = &additional else {
        return Err("input.additionalFolders.map is not a function".into());
    };
    if folders.is_empty() {
        return Ok(None);
    }
    let primary = field(&input, "primaryFolder")
        .map(js_string)
        .unwrap_or_default();
    let mut paths = vec![primary.clone()];
    paths.extend(folders.iter().map(js_string));
    let labels = derive_labels(&paths);

    static NULL: Value = Value::Null;
    let worktree = field(&input, "worktreeOf").unwrap_or(&NULL);
    let primary_tag = if js_truthy(worktree) {
        format!(" (primary, worktree of {})", js_string(worktree))
    } else {
        " (primary)".to_string()
    };
    let worktree_note = if js_truthy(worktree) {
        "Edit the primary folder only inside its worktree path above, never in the original checkout. The other folders are not isolated: edits there apply directly.\n"
    } else {
        ""
    };
    let mut out = String::from(
        "<project_folders>\nThis project spans several folders. You can read and edit all of them:\n",
    );
    out.push_str(&format!("- {}: {}{}\n", labels[0], paths[0], primary_tag));
    for i in 1..paths.len() {
        out.push_str(&format!("- {}: {}\n", labels[i], paths[i]));
    }
    out.push_str(worktree_note);
    out.push_str("Use absolute paths for files outside the primary folder.\n</project_folders>");
    Ok(Some(out))
}
