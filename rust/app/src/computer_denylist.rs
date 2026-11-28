//! Computer-control denylist, exact port of upstream computer/computerDenylist.ts.
//! Password managers and OS security surfaces are refused with no override in
//! this build: control of 1Password or Keychain Access is control of every
//! credential the user has.

const DENYLIST_BUNDLE_IDS: &[&str] = &[
    "com.1password.1password",
    "com.agilebits.onepassword",
    "com.apple.keychainaccess",
    "com.apple.passwords",
    "com.apple.systempreferences",
    "com.apple.securityagent",
    "com.apple.security.authorization",
    "com.lastpass.lastpass",
    "com.bitwarden.desktop",
];

const DENYLIST_BUNDLE_PREFIXES: &[&str] = &["com.agilebits.onepassword", "com.dashlane."];

const DENYLIST_APP_NAMES: &[&str] = &[
    "1password",
    "keychain access",
    "passwords",
    "system settings",
    "system preferences",
    "securityagent",
    "securityagenthelper",
    "bitwarden",
    "dashlane",
    "lastpass",
];

pub struct DenylistMatch {
    pub app: String,
    pub matched: String,
}

fn normalize_app_name(raw: &str) -> String {
    let mut name = raw.trim().to_string();
    if name.contains('/') {
        name = name.rsplit('/').next().unwrap_or(&name).to_string();
    }
    if name.to_lowercase().ends_with(".app") {
        name = name[..name.len() - 4].to_string();
    }
    // Collapse whitespace runs exactly like upstream's replace(/\s+/g, " ").
    let mut collapsed = String::new();
    let mut pending_space = false;
    for c in name.chars() {
        if c.is_whitespace() {
            pending_space = true;
        } else {
            if pending_space && !collapsed.is_empty() {
                collapsed.push(' ');
            }
            collapsed.extend(c.to_lowercase());
            pending_space = false;
        }
    }
    collapsed
}

fn matches_denylist_string(raw: &str) -> Option<String> {
    let name = normalize_app_name(raw);
    for denied in DENYLIST_APP_NAMES {
        if name == *denied || name.starts_with(&format!("{denied} ")) {
            return Some(format!("app name {denied}"));
        }
    }
    // The same string is also tried as a bundle id: launch targets arrive in
    // both spellings.
    let bundle_id: String = name.chars().filter(|c| !c.is_whitespace()).collect();
    if DENYLIST_BUNDLE_IDS.contains(&bundle_id.as_str()) {
        return Some(format!("bundle id {bundle_id}"));
    }
    for prefix in DENYLIST_BUNDLE_PREFIXES {
        if bundle_id.starts_with(prefix) {
            return Some(format!("bundle id {prefix}*"));
        }
    }
    None
}

/// How an app identity matches the denylist, or nothing. Name wins over
/// bundle id exactly like upstream.
pub fn computer_denylist_match(
    name: Option<&str>,
    bundle_id: Option<&str>,
) -> Option<DenylistMatch> {
    if let Some(name) = name {
        if let Some(matched) = matches_denylist_string(name) {
            return Some(DenylistMatch {
                app: name.to_string(),
                matched,
            });
        }
    }
    if let Some(bundle_id) = bundle_id {
        if let Some(matched) = matches_denylist_string(bundle_id) {
            return Some(DenylistMatch {
                app: name
                    .or(Some(bundle_id))
                    .unwrap_or("the target app")
                    .to_string(),
                matched,
            });
        }
    }
    None
}

pub fn denylist_error(matched: &DenylistMatch) -> String {
    format!(
        "{} is on the computer-control denylist ({}): password managers and OS security surfaces are refused, with no override in this build.",
        matched.app, matched.matched
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_managers_match_by_name_and_bundle() {
        assert!(computer_denylist_match(Some("1Password"), None).is_some());
        assert!(computer_denylist_match(Some("Keychain Access"), None).is_some());
        assert!(computer_denylist_match(None, Some("com.bitwarden.desktop")).is_some());
        assert!(computer_denylist_match(None, Some("com.dashlane.dashlanephonefinal")).is_some());
    }

    #[test]
    fn paths_and_app_suffixes_normalize() {
        assert!(computer_denylist_match(Some("/Applications/1Password.app"), None).is_some());
        assert!(computer_denylist_match(Some("System Settings"), None).is_some());
    }

    #[test]
    fn ordinary_apps_pass() {
        assert!(computer_denylist_match(Some("Visual Studio Code"), None).is_none());
        assert!(computer_denylist_match(Some("com.microsoft.VSCode"), None).is_none());
        assert!(computer_denylist_match(Some("Finder"), None).is_none());
    }
}
