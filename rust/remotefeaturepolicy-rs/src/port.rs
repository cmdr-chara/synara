//! The Rust port of `apps/server/src/remoteFeaturePolicy.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

use std::path::Path;
use std::sync::OnceLock;

/// `REMOTE_IMPORT_SUSPENSION_FILE`.
const REMOTE_IMPORT_SUSPENSION_FILE: &str = "remote-suspended-after-import.json";

/// `REMOTE_V2_QUALIFIED` — build-time constant upstream.
const REMOTE_V2_QUALIFIED: bool = true;

/// `ACCOUNT_URL_ENV_NAME` (`@synara/shared/account`).
const ACCOUNT_URL_ENV_NAME: &str = "SYNARA_ACCOUNT_URL";

/// `process.env[name]` — `None` when unset.
fn env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

/// `String.prototype.trim` — the JS whitespace set (`jscore::is_space`); Rust's
/// `char::is_whitespace` also covers U+0085, which JS does not trim.
fn js_trim(s: &str) -> &str {
    s.trim_matches(jscore::is_space)
}

/// `desktopFlavorFromBundleId(process.env.SYNARA_DESKTOP_BUNDLE_ID)`, read per
/// call like the source: blank or unrecognized maps to `"unknown"`.
fn desktop_flavor() -> &'static str {
    match env("SYNARA_DESKTOP_BUNDLE_ID").as_deref().map(js_trim) {
        Some("com.emanueledipietro.synara") => "production",
        Some("com.emanueledipietro.synara.dev") => "development",
        Some("com.emanueledipietro.synara.canary") => "canary",
        Some("com.emanueledipietro.synara.cua") => "cua",
        Some("com.emanueledipietro.synara.remote") => "remote",
        Some("com.emanueledipietro.synara.beta") => "beta",
        _ => "unknown",
    }
}

/// `resolveConfiguredAccountUrl({})` — with no `flag` and no `env` override it
/// reads `SYNARA_ACCOUNT_URL`, trimmed, empty mapping to `undefined`.
fn configured_account_url() -> Option<String> {
    let raw = env(ACCOUNT_URL_ENV_NAME)?;
    let trimmed = js_trim(&raw);
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// `path.posix.normalize` (the recorded host is darwin → posix rules), copied
/// from accountauth-rs.
fn path_normalize(path: &str) -> String {
    let is_abs = path.starts_with('/');
    let trailing = path.len() > 1 && path.ends_with('/');
    let mut segs: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => match segs.last() {
                Some(&"..") if !is_abs => segs.push(".."),
                Some(_) => {
                    segs.pop();
                }
                None if !is_abs => segs.push(".."),
                None => {}
            },
            s => segs.push(s),
        }
    }
    let mut out = segs.join("/");
    if is_abs {
        out = format!("/{out}");
    }
    if out.is_empty() {
        ".".to_string()
    } else if trailing && out != "/" {
        format!("{out}/")
    } else {
        out
    }
}

/// `path.posix.join(a, b)` — joins with `/` then normalizes.
fn path_join(a: &str, b: &str) -> String {
    if a.is_empty() {
        path_normalize(b)
    } else {
        path_normalize(&format!("{a}/{b}"))
    }
}

/// `process.versions.bun` — truthy when the host executable is Bun (`bun`,
/// `bunx`, `bun-*`). Native code has no `process.versions`; the addon runs
/// inside the host process, so its executable name is the check.
fn is_bun_runtime() -> bool {
    static IS_BUN: OnceLock<bool> = OnceLock::new();
    *IS_BUN.get_or_init(|| {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.file_stem().map(|n| n.to_string_lossy().to_lowercase()))
            .is_some_and(|n| n.starts_with("bun"))
    })
}

/// `Number(process.versions.node.split(".")[0])` — the host executable is the
/// Node binary, so its `--version` output (`vX.Y.Z`) carries the same major.
/// `None` (unreadable) behaves like the check's `NaN`: `!== 24` holds.
fn node_major_version() -> Option<u32> {
    static NODE_MAJOR: OnceLock<Option<u32>> = OnceLock::new();
    *NODE_MAJOR.get_or_init(|| {
        let out = std::process::Command::new(std::env::current_exe().ok()?)
            .arg("--version")
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&out.stdout);
        text.trim()
            .strip_prefix('v')?
            .split('.')
            .next()?
            .parse()
            .ok()
    })
}

/// Port of `remoteConnectionsUnavailableReason` (`apps/server/src/remoteFeaturePolicy.ts:11`).
pub fn remote_connections_unavailable_reason(
    state_dir: Option<String>,
) -> Result<Option<String>, String> {
    let flavor = desktop_flavor();
    if flavor == "production" {
        // Stable needs both an explicit opt-in and a configured HTTPS account origin.
        let configured = configured_account_url();
        let https = configured
            .as_deref()
            .and_then(|s| url::Url::parse(s).ok())
            .filter(|u| u.scheme() == "https");
        let at_origin = https.as_ref().is_some_and(|u| {
            let origin = u.origin().ascii_serialization();
            configured.as_deref() == Some(origin.as_str())
                || configured == Some(format!("{origin}/"))
        });
        if env("SYNARA_REMOTE_CONNECTIONS").as_deref() != Some("1") || https.is_none() || !at_origin
        {
            return Ok(Some(
                "Remote connections are unavailable in Stable.".to_string(),
            ));
        }
    }
    if is_bun_runtime() || node_major_version() != Some(24) {
        return Ok(Some(
            "Remote connections require the Node 24 server runtime.".to_string(),
        ));
    }
    if flavor == "unknown" && env("SYNARA_REMOTE_CONNECTIONS").as_deref() != Some("1") {
        return Ok(Some(
            "Set SYNARA_REMOTE_CONNECTIONS=1 to opt in to remote connections.".to_string(),
        ));
    }
    if let Some(dir) = state_dir.as_deref().filter(|d| !d.is_empty())
        && Path::new(&path_join(dir, REMOTE_IMPORT_SUSPENSION_FILE)).exists()
    {
        return Ok(Some(
            "Reconnect this installation after the data import.".to_string(),
        ));
    }
    if !REMOTE_V2_QUALIFIED {
        return Ok(Some(
            "Remote protocol v2 and owner pairing are not qualified in this build.".to_string(),
        ));
    }
    Ok(None)
}

// Keep existing non-Stable reporting behavior while applying the Stable rollout policy.
/// Port of `accountLiveActivityEnabled` (`apps/server/src/remoteFeaturePolicy.ts:37`).
pub fn account_live_activity_enabled(state_dir: Option<String>) -> Result<bool, String> {
    Ok(desktop_flavor() != "production"
        || remote_connections_unavailable_reason(state_dir)?.is_none())
}

/// Port of `requireRemoteConnections` (`apps/server/src/remoteFeaturePolicy.ts:42`).
pub fn require_remote_connections(state_dir: Option<String>) -> Result<(), String> {
    match remote_connections_unavailable_reason(state_dir)? {
        Some(reason) => Err(reason),
        None => Ok(()),
    }
}

/// Port of `accountProfileSyncUnavailableReason` (`apps/server/src/remoteFeaturePolicy.ts:47`).
pub fn account_profile_sync_unavailable_reason() -> Result<Option<String>, String> {
    // Stable and Remote offer account profiles on sign-in. Other hosts retain
    // operator opt-in, and an explicit opt-out is honored in every flavor.
    let sync_env = env("SYNARA_ACCOUNT_PROFILE_SYNC");
    let enabled = sync_env.as_deref() == Some("1")
        || (matches!(desktop_flavor(), "production" | "remote") && sync_env.is_none());
    Ok(if enabled {
        None
    } else {
        Some(
            "Account profile sync is unavailable until SYNARA_ACCOUNT_PROFILE_SYNC=1 is set."
                .to_string(),
        )
    })
}

/// Port of `requireAccountProfileSync` (`apps/server/src/remoteFeaturePolicy.ts:60`).
pub fn require_account_profile_sync() -> Result<(), String> {
    match account_profile_sync_unavailable_reason()? {
        Some(reason) => Err(reason),
        None => Ok(()),
    }
}

/// Port of `requireHostSecretsSync` (`apps/server/src/remoteFeaturePolicy.ts:65`).
pub fn require_host_secrets_sync() -> Result<(), String> {
    Err("Host secrets sync is unavailable in this build.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_resolves_dot_segments() {
        assert_eq!(path_normalize("a/b/../c"), "a/c");
        assert_eq!(path_normalize("/a//b/"), "/a/b/");
        assert_eq!(path_normalize("../x"), "../x");
        assert_eq!(path_normalize("/../x"), "/x");
        assert_eq!(path_normalize(""), ".");
    }

    #[test]
    fn join_normalizes_the_pair() {
        assert_eq!(path_join("a/b", "../c"), "a/c");
        assert_eq!(path_join("", "f.json"), "f.json");
        assert_eq!(
            path_join("d", "remote-suspended-after-import.json"),
            "d/remote-suspended-after-import.json"
        );
    }

    #[test]
    fn trim_uses_js_whitespace() {
        assert_eq!(js_trim("﻿bom﻿"), "bom");
        assert_eq!(js_trim("  x\u{85}"), "x\u{85}");
        assert_eq!(js_trim(""), "");
    }
}
