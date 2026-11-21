//! The Rust port of `scripts/ci-install-key.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

// `process.versions.modules` of the Node runtime the behavior was recorded under.
const RECORDED_MODULES: &str = "137";

// `fileURLToPath(new URL("../", import.meta.url))` for `scripts/ci-install-key.ts`:
// the repository root. Computed from this crate's own manifest dir (`rust/<crate>`).
fn repository() -> PathBuf {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    base.canonicalize().unwrap_or(base)
}

fn received_desc(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => format!("type boolean ({b})"),
        Value::Number(_) => format!(
            "type number ({})",
            crate::js::number_to_string(v.as_f64().unwrap_or(0.0))
        ),
        Value::Array(_) => "an instance of Array".to_string(),
        Value::Object(_) => "an instance of Object".to_string(),
        Value::String(s) => format!("type string ('{s}')"),
    }
}

// `join(root, name)` — Node path.join on posix: joins with '/', skips empty
// segments, collapses repeated separators.
fn join(root: &str, name: &str) -> String {
    let mut parts: Vec<&str> = root.split('/').chain(name.split('/')).collect();
    parts.retain(|s| !s.is_empty());
    let joined = parts.join("/");
    if root.starts_with('/') {
        format!("/{joined}")
    } else {
        joined
    }
}

fn read_bytes(path: &str) -> Result<Vec<u8>, String> {
    match std::fs::read(path) {
        Ok(b) => Ok(b),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Err(format!("ENOENT: no such file or directory, open '{path}'"))
        }
        Err(e) => Err(format!("{e}")),
    }
}

fn read_utf8(path: &str) -> Result<String, String> {
    read_bytes(path).map(|b| String::from_utf8_lossy(&b).into_owned())
}

fn string_list(v: Option<&Value>) -> Vec<String> {
    v.and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|w| w.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// Port of `workspaceInstallFingerprint` (`scripts/ci-install-key.ts:12`).
pub fn workspace_install_fingerprint(root: Value) -> Result<String, String> {
    let root_str = match &root {
        v if crate::js::is_undefined(v) => repository().to_string_lossy().into_owned(),
        Value::String(s) => s.clone(),
        other => {
            return Err(format!(
                "The \"path\" argument must be of type string. Received {}",
                received_desc(other)
            ));
        }
    };
    let root_manifest = read_utf8(&join(&root_str, "package.json"))?;
    let configuration: Value = serde_json::from_str(&root_manifest).map_err(|e| format!("{e}"))?;
    let workspaces = match configuration.get("workspaces") {
        Some(Value::Array(_)) => string_list(configuration.get("workspaces")),
        Some(other) => match other.get("packages") {
            Some(_) => string_list(other.get("packages")),
            None => {
                return Err("Cannot read properties of undefined (reading 'packages')".to_string());
            }
        },
        None => {
            return Err("Cannot read properties of undefined (reading 'packages')".to_string());
        }
    };
    let patterns: Vec<String> = workspaces
        .iter()
        .map(|w| format!("{w}/package.json"))
        .collect();
    let mut manifests = jscore::glob_sync(&patterns, Path::new(&root_str));
    manifests.sort();
    let mut patches: Vec<String> = match configuration.get("patchedDependencies") {
        Some(Value::Object(m)) => m
            .values()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        _ => Vec::new(),
    };
    patches.sort();
    let mut hash = Sha256::new();
    let mut feed = |bytes: &[u8]| {
        hash.update(bytes);
        hash.update(b"\0");
    };
    feed(RECORDED_MODULES.as_bytes());
    feed(root_manifest.as_bytes());
    let lockfile = read_utf8(&join(&root_str, "bun.lock"))?;
    feed(jscore::normalize_bun_lockfile(&lockfile).as_bytes());
    for name in ["bunfig.toml", "scripts/patch-workspace-typescript.ts"]
        .iter()
        .map(|s| s.to_string())
        .chain(patches)
    {
        feed(name.as_bytes());
        feed(&read_bytes(&join(&root_str, &name))?);
    }
    for name in manifests {
        let manifest = read_utf8(&join(&root_str, &name))?;
        let mut manifest: Value = serde_json::from_str(&manifest).map_err(|e| format!("{e}"))?;
        if let Value::Object(m) = &mut manifest
            && m.get("version").is_some_and(Value::is_string)
        {
            m.shift_remove("version");
        }
        feed(name.as_bytes());
        feed(jscore::json_stringify(&manifest).as_bytes());
    }
    Ok(hash
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>())
}
