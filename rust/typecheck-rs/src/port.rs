//! The Rust port of `scripts/typecheck.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;
use std::path::{Path, PathBuf};

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

fn is_callable(v: &Value) -> bool {
    matches!(v, Value::Object(m) if m.contains_key("$function"))
}

/// `join(dirname(fileURLToPath(import.meta.url)), "..")` — the repo root,
/// located by the `tsconfig.check.json` marker. The diff harness runs the
/// native module in a scratch cwd, so fall back to the crate's own position
/// (`<root>/rust/typecheck-rs`).
fn repo_root() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        if dir.join("tsconfig.check.json").is_file() {
            return Some(dir);
        }
        if !dir.pop() {
            break;
        }
    }
    let via_manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    if via_manifest.join("tsconfig.check.json").is_file() {
        return Some(via_manifest);
    }
    None
}

/// `readFileSync(path, "utf8")`.
fn read_file(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("{}: '{}'", e, path.display()))
}

/// `path.posix`/`node:path` join for repo-relative segments.
fn join(a: &Path, b: &str) -> PathBuf {
    a.join(b)
}

/// Port of `effectProjects` (`scripts/typecheck.ts:30`): projects whose
/// tsconfig enables the Effect language-service plugin.
pub fn effect_projects(read: serde_json::Value) -> Result<Vec<String>, String> {
    // JS default params apply only when the argument is `undefined`.
    let read = read;
    let default_read = is_undefined(&read);

    // `TYPECHECK_PROJECTS` — eager module-level constant.
    let root = repo_root().ok_or_else(|| "ENOENT: no such file or directory".to_string())?;
    let check_json = read_file(&root.join("tsconfig.check.json"))?;
    let parsed: Value = serde_json::from_str(&check_json).map_err(|e| format!("{e}"))?;
    let mut projects: Vec<String> = Vec::new();
    if let Some(refs) = parsed.get("references").and_then(|r| r.as_array()) {
        for r in refs {
            if let Some(p) = r.get("path").and_then(|p| p.as_str()) {
                projects.push(p.to_string());
            }
        }
    }

    let mut out: Vec<String> = Vec::new();
    for project in projects {
        let rel = if project.ends_with(".json") {
            project.clone()
        } else {
            format!("{project}/tsconfig.json")
        };
        if !default_read {
            // `read(tsconfigPath(project))` on a non-callable.
            if !is_callable(&read) {
                return Err("read is not a function".to_string());
            }
            continue;
        }
        let contents = read_file(&join(&root, &rel))?;
        if contents.contains("\"@effect/language-service\"") {
            out.push(project);
        }
    }
    Ok(out)
}
