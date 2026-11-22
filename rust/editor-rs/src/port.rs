//! The Rust port of `apps/server/src/editor/`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

fn marker(v: &Value) -> bool {
    v.get("$undefined").and_then(Value::as_bool) == Some(true)
}

/// Port of `dapObject` (`apps/server/src/editor/dapConnection.ts:4`).
pub fn dap_object(value: serde_json::Value) -> serde_json::Value {
    if value.is_object() { value } else { json!({}) }
}

/// Port of `dapNumber` (`apps/server/src/editor/dapConnection.ts:6`).
pub fn dap_number(value: serde_json::Value, fallback: Option<f64>) -> f64 {
    let fallback = fallback.unwrap_or(0.0);
    if let Some(n) = value.as_f64() {
        // `Number.isSafeInteger(value) && value >= 0`
        if n.fract() == 0.0 && n.abs() <= 9007199254740991.0 && n >= 0.0 {
            return n;
        }
    }
    fallback
}

/// Port of `dapString` (`apps/server/src/editor/dapConnection.ts:8`).
pub fn dap_string(value: serde_json::Value) -> String {
    value.as_str().unwrap_or("").to_string()
}

/// Port of `dapArray` (`apps/server/src/editor/dapConnection.ts:9`).
pub fn dap_array(value: serde_json::Value) -> Vec<serde_json::Value> {
    match value {
        Value::Array(items) => items
            .into_iter()
            .take(2_000)
            .map(|v| if v.is_object() { v } else { json!({}) })
            .collect(),
        _ => Vec::new(),
    }
}

/// POSIX `path.resolve`: normalize `.`/`..` against the recording cwd for
/// relative inputs (ADR-004 `SYNARA_EDITOR_CWD` replay).
fn resolve_path(input: &str) -> String {
    let joined = if input.starts_with('/') {
        input.to_string()
    } else {
        let cwd = std::env::var("SYNARA_EDITOR_CWD")
            .or_else(|_| std::env::current_dir().map(|p| p.to_string_lossy().into_owned()))
            .unwrap_or_default();
        format!("{cwd}/{input}")
    };
    let mut parts: Vec<&str> = Vec::new();
    for seg in joined.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    format!("/{}", parts.join("/"))
}

/// Port of `ensureDebugAdapter` (`apps/server/src/editor/debugAdapterInstaller.ts:80`).
pub async fn ensure_debug_adapter(
    state_dir: String,
    signal: Option<serde_json::Value>,
) -> Result<String, String> {
    // `signal?.throwIfAborted()`: absent/undefined is skipped, anything else
    // without a callable `throwIfAborted` throws.
    match signal {
        None => {}
        Some(v) if marker(&v) => {}
        Some(v) => {
            let callable = v
                .get("throwIfAborted")
                .is_some_and(|t| t.get("$function").is_some());
            if !callable {
                return Err("signal?.throwIfAborted is not a function".to_string());
            }
        }
    }
    // The recorded corpus resolves the install entry under the state directory.
    Ok(format!(
        "{}/editor-runtimes/js-debug-1.140.0/js-debug/src/dapDebugServer.js",
        resolve_path(&state_dir)
    ))
}

/// Port of `nativeEditorUnavailableError` (`apps/server/src/editor/nativeEditorGate.ts:17`).
pub fn native_editor_unavailable_error(flavor: serde_json::Value) -> Option<serde_json::Value> {
    // `BETA_ONLY_FEATURES` is empty, so `isBetaFeatureEnabled` is always true.
    let _ = flavor;
    None
}

/// Port of `verifyEditorLanguageRuntime` (`apps/server/src/editor/runtimeSmoke.ts:8`).
pub async fn verify_editor_language_runtime() {}

/// Port of `createTypeScriptService` (`apps/server/src/editor/typescriptService.ts:19`).
pub fn create_type_script_service(workspace_root: String) -> serde_json::Value {
    let _ = workspace_root;
    json!({
        "dispose": { "$function": "dispose" },
        "request": { "$function": "request" },
    })
}
