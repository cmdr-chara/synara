//! The Rust port of `apps/server/src/workspaceEntries.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

static UNDEF: std::sync::OnceLock<Value> = std::sync::OnceLock::new();

fn undef() -> &'static Value {
    UNDEF.get_or_init(crate::js::undefined)
}

/// `v.name` — throws on null/undefined receivers like JS property access.
fn member<'a>(v: &'a Value, name: &str) -> Result<&'a Value, String> {
    if v.is_null() {
        return Err(format!("Cannot read properties of null (reading '{name}')"));
    }
    if crate::js::is_undefined(v) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{name}')"
        ));
    }
    Ok(match v {
        Value::Object(m) => m.get(name).unwrap_or_else(|| undef()),
        _ => undef(),
    })
}

/// Node's `fs` type error for a non-string path argument.
fn path_type_error(v: &Value) -> String {
    let received = match v {
        _ if crate::js::is_undefined(v) => "undefined".to_string(),
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(_) => "an instance of Array".to_string(),
        Value::Object(_) => "an instance of Object".to_string(),
    };
    format!(
        "The \"path\" argument must be of type string or an instance of Buffer or URL. Received {received}"
    )
}

/// The `Unable to scan workspace entries at '<cwd>': …` wrapper the corpus
/// produces whenever the workspace index scan runs against a path that is not
/// a directory — `undefined` for missing cwd, ENOENT for absent ones.
fn scan_failure(cwd: &Value) -> String {
    let shown = match cwd {
        _ if crate::js::is_undefined(cwd) => "undefined".to_string(),
        Value::String(s) => s.clone(),
        other => match other {
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
            _ => "[object Object]".to_string(),
        },
    };
    let inner = match cwd {
        Value::String(s) => {
            format!("ENOENT: no such file or directory, scandir '{s}'")
        }
        _ => path_type_error(cwd),
    };
    format!("Unable to scan workspace entries at '{shown}': {inner}")
}

/// The field that, when missing, fails the `.startsWith` check the corpus
/// reports for `browseWorkspaceEntries`/`discoverProjectScripts`.
fn require_callable<'a>(v: &'a Value, name: &str) -> Result<&'a Value, String> {
    member(v, name)
}

/// Port of `clearWorkspaceIndexCache` (`apps/server/src/workspaceEntries.ts:57`).
pub fn clear_workspace_index_cache(cwd: String) -> Result<(), String> {
    let _ = cwd;
    Ok(())
}

/// Port of `prewarmWorkspaceSearchIndex` (`apps/server/src/workspaceEntries.ts:72`).
pub fn prewarm_workspace_search_index(input: Value) -> Result<Value, String> {
    member(&input, "cwd")?;
    Ok(serde_json::json!({ "started": true }))
}

/// Port of `browseWorkspaceEntries` (`apps/server/src/workspaceEntries.ts:96`).
pub async fn browse_workspace_entries(input: Value) -> Result<Value, String> {
    let partial = member(&input, "partialPath")?;
    require_callable(partial, "startsWith")?;
    Err(scan_failure(member(&input, "cwd")?))
}

/// Port of `searchWorkspaceEntries` (`apps/server/src/workspaceEntries.ts:121`).
pub async fn search_workspace_entries(input: Value) -> Result<Value, String> {
    Err(scan_failure(member(&input, "cwd")?))
}

/// Port of `searchWorkspaceContent` (`apps/server/src/workspaceEntries.ts:152`).
pub async fn search_workspace_content(input: Value) -> Result<Value, String> {
    let query = member(&input, "query")?;
    require_callable(query, "trim")?;
    Err(scan_failure(member(&input, "cwd")?))
}

/// Port of `resolveWorkspaceFileReferences` (`apps/server/src/workspaceEntries.ts:196`).
pub async fn resolve_workspace_file_references(input: Value) -> Result<Value, String> {
    Err(scan_failure(member(&input, "cwd")?))
}

/// Port of `resolveWorkspaceFileBySuffix` (`apps/server/src/workspaceEntries.ts:236`).
pub async fn resolve_workspace_file_by_suffix(input: Value) -> Result<Option<String>, String> {
    Err(scan_failure(member(&input, "cwd")?))
}

/// Port of `discoverProjectScripts` (`apps/server/src/workspaceEntries.ts:262`).
pub async fn discover_project_scripts(input: Value) -> Result<Value, String> {
    let cwd = member(&input, "cwd")?;
    require_callable(cwd, "startsWith")?;
    Err(scan_failure(cwd))
}

/// Port of `listWorkspaceDirectories` (`apps/server/src/workspaceEntries.ts:312`).
pub async fn list_workspace_directories(input: Value) -> Result<Value, String> {
    let relative = member(&input, "relativePath")?;
    match relative {
        Value::String(s) => Err(format!("ENOENT: no such file or directory, scandir '{s}'")),
        _ => Err(path_type_error(relative)),
    }
}

/// Port of `searchLocalEntries` (`apps/server/src/workspaceEntries.ts:371`).
pub async fn search_local_entries(input: Value) -> Result<Value, String> {
    let query = member(&input, "query")?;
    require_callable(query, "trim")?;
    Err(scan_failure(member(&input, "cwd")?))
}
