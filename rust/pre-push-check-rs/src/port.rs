//! The Rust port of `scripts/pre-push-check.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

const TYPECHECK_PROJECTS: &[&str] = &[
    "apps/analytics",
    "apps/api",
    "apps/api/cloudflare/tsconfig.json",
    "apps/desktop",
    "apps/desktop/tsconfig.fixtures.json",
    "apps/e2e",
    "apps/marketing",
    "apps/profiles",
    "apps/server",
    "apps/web",
    "packages/contracts",
    "packages/profile-ui",
    "packages/shared",
    "scripts",
    "scripts/tsconfig.computer-measurement.json",
];

fn is_js_ws(c: char) -> bool {
    matches!(
        c,
        '\u{0009}'..='\u{000D}'
            | '\u{0020}'
            | '\u{00A0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200A}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202F}'
            | '\u{205F}'
            | '\u{3000}'
            | '\u{FEFF}'
    )
}

fn is_shared_path(file: &str) -> bool {
    file.starts_with("packages/")
        || file.starts_with("scripts/lib/")
        || file == "tsconfig.base.json"
        || file == "tsconfig.check.json"
        || file == "package.json"
        || file == "bun.lock"
}

fn is_typed_file(file: &str) -> bool {
    file.ends_with(".js")
        || file.ends_with(".ts")
        || file.ends_with(".jsx")
        || file.ends_with(".tsx")
        || file.ends_with(".cjs")
        || file.ends_with(".cts")
        || file.ends_with(".mjs")
        || file.ends_with(".mts")
        || file.ends_with(".json")
}

fn workspace_of(project: &str) -> String {
    let take = if project.starts_with("scripts") { 1 } else { 2 };
    project.split('/').take(take).collect::<Vec<_>>().join("/")
}

/// Port of `parsePushedRefs` (`scripts/pre-push-check.ts:23`).
pub fn parse_pushed_refs(stdin: String) -> Result<Vec<Value>, String> {
    let mut refs = Vec::new();
    for line in stdin.split('\n') {
        let trimmed = line.trim_matches(is_js_ws);
        let parts: Vec<&str> = trimmed.split(is_js_ws).filter(|p| !p.is_empty()).collect();
        if parts.len() == 4 {
            refs.push(json!({
                "localOid": parts[1],
                "remoteOid": parts[3],
            }));
        }
    }
    Ok(refs)
}

/// Port of `affectedProjects` (`scripts/pre-push-check.ts:74`).
pub fn affected_projects(files: Vec<String>) -> Result<Vec<String>, String> {
    if files.iter().any(|f| is_shared_path(f)) {
        return Ok(TYPECHECK_PROJECTS.iter().map(|s| s.to_string()).collect());
    }
    let typed: Vec<&String> = files.iter().filter(|f| is_typed_file(f)).collect();
    let affects_scripts = typed
        .iter()
        .any(|f| f.starts_with("apps/server/") || f.starts_with("apps/desktop/"));
    let out: Vec<String> = TYPECHECK_PROJECTS
        .iter()
        .filter(|project| {
            let workspace = workspace_of(project);
            typed
                .iter()
                .any(|f| f.starts_with(&format!("{workspace}/")))
                || (workspace == "scripts" && affects_scripts)
        })
        .map(|s| s.to_string())
        .collect();
    Ok(out)
}
