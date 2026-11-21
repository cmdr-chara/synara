//! The Rust port of `apps/server/src/migrationBundleIdentity.ts`.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::Value;

const MIGRATION_RUNTIME_SOURCE_RELATIVE_PATH: &str = "apps/server/src/persistence/Migrations.ts";

/// `typeof __SYNARA_MIGRATION_RUNTIME_SOURCE_DIGEST__ === "string" ? … : null`
/// — the build-time constant is not defined outside the bundled artifact.
pub fn embedded_migration_runtime_source_digest() -> Result<Value, String> {
    Ok(Value::Null)
}

fn prop<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    if js::is_undefined(v) {
        return None;
    }
    v.as_object().and_then(|o| o.get(key))
}

fn sha256_hex(text: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(text.as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

fn mismatch_error(kind: &str, expected: &Value, actual: &Value) -> String {
    let as_text = |v: &Value| match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        _ if js::is_undefined(v) => "undefined".to_string(),
        _ => serde_json::to_string(v).unwrap_or_default(),
    };
    let (relationship, recovery) = if kind == "launcher-bundle" {
        (
            "desktop and server bundles were built from different migration sources",
            "Rebuild with bun run build:desktop before starting Synara.",
        )
    } else {
        (
            "the server bundle was built from a different migration source than this checkout",
            "Rebuild the server with bun run build before starting Synara.",
        )
    };
    format!(
        "Refusing database startup because {relationship}. Expected {}, but the server bundle contains {}. {recovery}",
        as_text(expected),
        as_text(actual)
    )
}

/// POSIX `path.resolve` of a single segment against the process cwd.
fn resolve_cwd(cwd: Option<&Value>) -> Result<String, String> {
    match cwd {
        Some(Value::String(s)) => {
            if s.starts_with('/') {
                Ok(s.clone())
            } else {
                let base = std::env::current_dir().unwrap_or_default();
                Ok(format!("{}/{s}", base.to_string_lossy()))
            }
        }
        None => Err("\"paths[0]\" argument must be of type string. Received undefined".to_string()),
        Some(v @ Value::Object(_)) if js::is_undefined(v) => {
            Err("\"paths[0]\" argument must be of type string. Received undefined".to_string())
        }
        Some(v) => Err(format!(
            "\"paths[0]\" argument must be of type string. Received {}",
            type_name_of(v)
        )),
    }
}

fn type_name_of(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => format!("type boolean ({b})"),
        Value::Number(n) => {
            format!(
                "type number ({})",
                js::number_to_string(n.as_f64().unwrap_or(f64::NAN))
            )
        }
        Value::String(s) => format!("type string ('{s}')"),
        _ => "type undefined".to_string(),
    }
}

fn find_source_checkout_root(start: &str) -> Option<String> {
    let mut candidate = start.to_string();
    loop {
        let pkg_path = format!("{candidate}/package.json");
        if let Ok(text) = std::fs::read_to_string(&pkg_path)
            && let Ok(pkg) = serde_json::from_str::<Value>(&text)
            && pkg.get("name").and_then(Value::as_str) == Some("@synara/monorepo")
        {
            return Some(candidate);
        }
        let parent = std::path::Path::new(&candidate)
            .parent()
            .map(|p| p.to_string_lossy().to_string());
        match parent {
            Some(p) if p != candidate => candidate = p,
            _ => return None,
        }
    }
}

/// Port of `verifyMigrationRuntimeIdentity` (`apps/server/src/migrationBundleIdentity.ts:41`).
pub fn verify_migration_runtime_identity(input: Value) -> Result<Value, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'embeddedDigest')".to_string());
    }
    let embedded = prop(&input, "embeddedDigest");
    let launcher = prop(&input, "launcherDigest");
    let launcher_defined = launcher.is_some_and(|v| !js::is_undefined(v));

    if matches!(embedded, Some(Value::Null)) {
        if launcher_defined {
            return Err(mismatch_error(
                "launcher-bundle",
                launcher.unwrap_or(&Value::Null),
                &Value::String("no embedded migration source identity".to_string()),
            ));
        }
        return Ok(js::undefined());
    }

    if launcher_defined {
        let launcher_v = launcher.unwrap_or(&Value::Null);
        let embedded_v = embedded.unwrap_or(&Value::Null);
        if launcher_v != embedded_v {
            return Err(mismatch_error("launcher-bundle", launcher_v, embedded_v));
        }
        return Ok(js::undefined());
    }

    let resolved = resolve_cwd(prop(&input, "cwd"))?;
    let source_text: Option<String> = match find_source_checkout_root(&resolved) {
        None => None,
        Some(root) => {
            let source_path = format!("{root}/{MIGRATION_RUNTIME_SOURCE_RELATIVE_PATH}");
            match std::fs::read_to_string(&source_path) {
                Ok(t) => Some(t),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                Err(e) => return Err(e.to_string()),
            }
        }
    };

    if let Some(text) = source_text {
        let digest = sha256_hex(&text);
        let embedded_v = embedded.unwrap_or(&Value::Null);
        if embedded_v.as_str() != Some(digest.as_str()) {
            return Err(mismatch_error(
                "source-bundle",
                &Value::String(digest),
                embedded_v,
            ));
        }
    }
    Ok(js::undefined())
}
