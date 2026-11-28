//! Installation-scoped "app tour seen" persistence.
//!
//! Upstream stores FeatureTourDialog's seen installation keys in browser
//! localStorage. The native shell has no browser storage, so it keeps the same
//! installation-scoped array in the Synara home instead.

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

pub fn load_seen() -> Vec<String> {
    load_seen_from(&state_path())
}

pub fn mark_seen(installation: &str) {
    if installation.is_empty() {
        return;
    }
    let path = state_path();
    let mut seen = load_seen_from(&path);
    if seen.iter().any(|item| item == installation) {
        return;
    }
    seen.push(installation.to_string());
    save_seen_to(&path, &seen);
}

fn state_path() -> PathBuf {
    crate::backend::configured_home_dir()
        .join("userdata")
        .join("feature-tour-seen.json")
}

fn load_seen_from(path: &Path) -> Vec<String> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return Vec::new();
    };
    value
        .get("seenInstallations")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn save_seen_to(path: &Path, seen: &[String]) {
    let Some(parent) = path.parent() else {
        return;
    };
    if fs::create_dir_all(parent).is_err() {
        return;
    }
    let payload = serde_json::json!({
        "version": 1,
        "seenInstallations": seen,
    });
    let temp = path.with_extension("json.tmp");
    if fs::write(&temp, payload.to_string().as_bytes()).is_ok() {
        let _ = fs::rename(&temp, path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seen_installations_round_trip_without_duplicates() {
        let root =
            std::env::temp_dir().join(format!("synara-feature-tour-test-{}", std::process::id()));
        let path = root.join("feature-tour-seen.json");
        save_seen_to(&path, &["install-a".to_string()]);
        assert_eq!(load_seen_from(&path), vec!["install-a".to_string()]);

        let mut seen = load_seen_from(&path);
        if !seen.iter().any(|item| item == "install-a") {
            seen.push("install-a".to_string());
        }
        seen.push("install-b".to_string());
        save_seen_to(&path, &seen);
        assert_eq!(
            load_seen_from(&path),
            vec!["install-a".to_string(), "install-b".to_string()]
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn invalid_state_reads_as_unseen() {
        let root = std::env::temp_dir().join(format!(
            "synara-feature-tour-invalid-{}",
            std::process::id()
        ));
        let path = root.join("feature-tour-seen.json");
        fs::create_dir_all(&root).unwrap();
        fs::write(&path, "not json").unwrap();
        assert!(load_seen_from(&path).is_empty());
        let _ = fs::remove_dir_all(root);
    }
}
