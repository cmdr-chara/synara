//! The Rust port of `apps/desktop/src/migrationBundleIdentity.ts`.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::{Value, json};

const MIGRATION_RUNTIME_SOURCE_RELATIVE_PATH: &str = "apps/server/src/persistence/Migrations.ts";

/// `typeof __SYNARA_MIGRATION_RUNTIME_SOURCE_DIGEST__ === "string" ? … : null`
/// — the build-time constant is not defined outside the bundled artifact.
pub fn embedded_desktop_migration_runtime_source_digest() -> Result<Option<String>, String> {
    Ok(None)
}

fn prop<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    if js::is_undefined(v) {
        return None;
    }
    v.as_object().and_then(|o| o.get(key))
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => !js::is_undefined(v),
    }
}

fn sha256_hex(text: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(text.as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// `path.join(root, rel)` for a single base.
fn join_path(root: Option<&Value>) -> Result<String, String> {
    match root {
        Some(Value::String(s)) => {
            if s.is_empty() {
                Ok(MIGRATION_RUNTIME_SOURCE_RELATIVE_PATH.to_string())
            } else {
                Ok(format!("{s}/{MIGRATION_RUNTIME_SOURCE_RELATIVE_PATH}"))
            }
        }
        None => Err("\"path\" argument must be of type string. Received undefined".to_string()),
        Some(v @ Value::Object(_)) if js::is_undefined(v) => {
            Err("\"path\" argument must be of type string. Received undefined".to_string())
        }
        Some(Value::Null) => {
            Err("\"path\" argument must be of type string. Received null".to_string())
        }
        Some(Value::Bool(b)) => Err(format!(
            "\"path\" argument must be of type string. Received type boolean ({b})"
        )),
        Some(Value::Number(n)) => Err(format!(
            "\"path\" argument must be of type string. Received type number ({})",
            js::number_to_string(n.as_f64().unwrap_or(f64::NAN))
        )),
        Some(_) => Err("\"path\" argument must be of type string. Received undefined".to_string()),
    }
}

/// Port of `inspectDesktopMigrationRuntimeIdentity` (`apps/desktop/src/migrationBundleIdentity.ts:18`).
pub fn inspect_desktop_migration_runtime_identity(input: Value) -> Result<Option<Value>, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'embeddedDigest')".to_string());
    }
    let embedded = prop(&input, "embeddedDigest");
    if matches!(embedded, Some(Value::Null)) {
        return Err("The desktop bundle has no embedded migration source identity.".to_string());
    }
    if prop(&input, "isPackaged").is_some_and(truthy) {
        return Ok(None);
    }

    let source_path = join_path(prop(&input, "appRoot"))?;
    let source_text = match std::fs::read_to_string(&source_path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };

    let digest = sha256_hex(&source_text);
    let embedded_v = embedded.unwrap_or(&Value::Null);
    if embedded_v.as_str() != Some(digest.as_str()) {
        return Ok(Some(json!({
            "kind": "source-bundle",
            "expectedDigest": digest,
            "actualDigest": embedded_v,
        })));
    }
    Ok(None)
}
