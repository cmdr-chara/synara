//! Claude per-account environment isolation, ported from upstream
//! `claudeEnvironment.ts`. Each provider instance gets its own isolated home
//! (`provider-homes/claude/instance-<hex(instanceId)>` under the userdata
//! root) so the CLI's credentials, projects, and sessions never mix across
//! accounts. The default instance keeps the user's real home.

/// Hex-encoded instance scope, upstream `claudeInstanceHomeScope`.
fn instance_scope(provider_instance_id: Option<&str>) -> String {
    match provider_instance_id
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(id) => format!(
            "instance-{}",
            id.bytes().map(|b| format!("{b:02x}")).collect::<String>()
        ),
        None => "environment-only".to_string(),
    }
}

/// Isolated home path, upstream `claudeIsolatedHomePath`:
/// `<isolationRoot|~/ .synara/userdata>/provider-homes/claude/<scope>`.
pub(crate) fn isolated_home_path(
    userdata_root: &str,
    provider_instance_id: Option<&str>,
) -> String {
    format!(
        "{}/provider-homes/claude/{}",
        userdata_root.trim_end_matches('/'),
        instance_scope(provider_instance_id)
    )
}

/// Home environment map, upstream `claudeHomeEnvironment`. On non-Windows it
/// is just HOME; Windows mirrors the profile variables because the CLI can
/// read profile directories outside HOME.
pub(crate) fn home_environment(home_path: &str) -> Vec<(String, String)> {
    // This port targets macOS/Linux first; the Windows mirroring is a
    // documented gap until the Windows runtime lands.
    vec![("HOME".to_string(), home_path.to_string())]
}

/// Whether an instance needs the isolated home: any explicit per-instance
/// environment or a non-default instance id forces isolation so accounts
/// never share the CLI state, upstream `needsIsolatedInstanceHome`.
pub(crate) fn needs_isolated_home(
    explicit_home: Option<&str>,
    has_instance_environment: bool,
    provider_instance_id: Option<&str>,
    default_instance_id: &str,
) -> bool {
    if explicit_home
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .is_some()
    {
        return false;
    }
    has_instance_environment
        || provider_instance_id
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|id| id != default_instance_id)
            .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_hex_encodes_instance_id() {
        // "acc" → 61 63 63
        assert_eq!(instance_scope(Some("acc")), "instance-616363");
        assert_eq!(instance_scope(None), "environment-only");
        assert_eq!(instance_scope(Some("  ")), "environment-only");
    }

    #[test]
    fn isolated_path_layout() {
        assert_eq!(
            isolated_home_path("/home/u/.synara/userdata", Some("acc")),
            "/home/u/.synara/userdata/provider-homes/claude/instance-616363"
        );
    }

    #[test]
    fn default_instance_needs_no_isolation() {
        assert!(!needs_isolated_home(
            None,
            false,
            Some("default"),
            "default"
        ));
        assert!(needs_isolated_home(None, false, Some("work"), "default"));
        assert!(needs_isolated_home(None, true, Some("default"), "default"));
        assert!(!needs_isolated_home(
            Some("/tmp/x"),
            true,
            Some("work"),
            "default"
        ));
    }
}
