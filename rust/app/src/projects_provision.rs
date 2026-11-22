//! `projects.provisionFromGitHub` — clone a GitHub repository into the
//! chosen parent directory, reusing the checkout when it already exists.
//! Clone goes through `gh repo clone` when gh is on PATH (it carries the
//! user's auth), else plain `git clone` of the public HTTPS URL.

use serde_json::{Value, json};
use std::path::PathBuf;
use std::process::Command;

use crate::todo::WsErr;

fn err(msg: impl Into<String>) -> WsErr {
    WsErr(msg.into(), Some("PROVISION"))
}

pub fn handles(tag: &str) -> bool {
    tag == "projects.provisionFromGitHub"
}

pub fn handle(_tag: &str, payload: &Value) -> Result<Value, WsErr> {
    let operation_id = str_req(payload, "operationId")?;
    let repository = str_req(payload, "repository")?;
    let destination_parent = str_req(payload, "destinationParent")?;
    let directory_name = str_req(payload, "directoryName")?;
    let project_id = str_req(payload, "projectId")?;

    let repo_re = regex_lite(&repository);
    if !repo_re {
        return Err(err("repository must be an owner/name pair"));
    }
    if directory_name.contains('/')
        || directory_name.contains('\\')
        || directory_name.starts_with('.')
        || directory_name.len() > 128
    {
        return Err(WsErr(
            "directoryName must be a single directory name".into(),
            Some("VALIDATION"),
        ));
    }
    let dest = PathBuf::from(&destination_parent).join(&directory_name);

    let checkout = if dest.join(".git").exists() {
        "reused"
    } else if dest.exists() {
        return Err(err(format!(
            "{dest:?} exists and is not a git checkout"
        )));
    } else {
        std::fs::create_dir_all(&destination_parent)
            .map_err(|e| err(format!("cannot create {destination_parent}: {e}")))?;
        clone(&repository, &dest)?;
        "created"
    };
    let workspace_root = dest
        .canonicalize()
        .unwrap_or(dest)
        .to_string_lossy()
        .to_string();

    Ok(json!({
        "operationId": operation_id,
        "repository": repository,
        "workspaceRoot": workspace_root,
        "projectId": project_id,
        "checkout": checkout,
    }))
}

fn str_req(payload: &Value, k: &str) -> Result<String, WsErr> {
    payload
        .get(k)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .ok_or_else(|| WsErr(format!("{k} is required"), Some("VALIDATION")))
}

fn regex_lite(repo: &str) -> bool {
    let mut parts = repo.split('/');
    let (Some(owner), Some(name), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    let ok = |s: &str| {
        !s.is_empty()
            && s.len() <= 128
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    };
    ok(owner) && ok(name)
}

fn clone(repo: &str, dest: &std::path::Path) -> Result<(), WsErr> {
    // Prefer gh (authenticated) and fall back to a public git clone.
    let gh = Command::new("gh")
        .args(["repo", "clone", repo])
        .arg(dest)
        .output();
    if let Ok(out) = gh {
        if out.status.success() {
            return Ok(());
        }
        // Fall through to git clone for repos gh cannot reach.
        let stderr = String::from_utf8_lossy(&out.stderr);
        if stderr.contains("Could not resolve") || stderr.contains("not found") {
            // Try public clone anyway below.
        }
    }
    let url = format!("https://github.com/{repo}.git");
    let out = Command::new("git")
        .args(["clone", "--", &url])
        .arg(dest)
        .output()
        .map_err(|e| err(format!("git clone failed to launch: {e}")))?;
    if out.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let _ = std::fs::remove_dir_all(dest);
        Err(err(format!(
            "clone of {repo} failed: {}",
            stderr.trim().chars().take(400).collect::<String>()
        )))
    }
}
