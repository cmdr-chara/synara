//! The Rust port of `apps/web/src/settingsNavigation.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value};

const SETTINGS_SECTION_IDS: &[&str] = &[
    "general",
    "profile",
    "accounts",
    "appearance",
    "layout",
    "notifications",
    "behavior",
    "appsnap",
    "computer",
    "shortcuts",
    "worktrees",
    "archived",
    "models",
    "providers",
    "skills",
    "usage",
    "connectors",
    "integrations",
    "connections",
    "privacy",
    "advanced",
    "about",
];

const HOST_SETTINGS_SECTIONS: &[&str] = &[
    "accounts",
    "worktrees",
    "archived",
    "models",
    "providers",
    "skills",
    "usage",
    "connectors",
    "integrations",
];

/// `isSettingsSectionAvailable` on Stable: "connectors" is Beta-only.
fn is_settings_section_available(section: &str) -> bool {
    section != "connectors"
}

fn slugify(lowered: &str) -> String {
    let mut slug = String::new();
    let mut pending_dash = false;
    for ch in lowered.chars() {
        if ch.is_ascii_alphanumeric() {
            if pending_dash && !slug.is_empty() {
                slug.push('-');
            }
            pending_dash = false;
            slug.push(ch);
        } else {
            pending_dash = true;
        }
    }
    slug
}

/// Port of `settingRowAnchorId` (`apps/web/src/settingsNavigation.ts:295`):
/// `setting-` + lowercased title with `[^a-z0-9]+` → `-`, edges trimmed.
pub fn setting_row_anchor_id(title: String) -> Result<String, String> {
    Ok(format!("setting-{}", slugify(&title.to_lowercase())))
}

/// Port of `providerAccountAnchorId` (`apps/web/src/settingsNavigation.ts:304`):
/// same slug but edge dashes are kept (`replace` only, no trim).
pub fn provider_account_anchor_id(instance_id: String) -> Result<String, String> {
    let lowered = instance_id.to_lowercase();
    let mut slug = String::new();
    let mut in_dash = false;
    for ch in lowered.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            in_dash = false;
        } else if !in_dash {
            slug.push('-');
            in_dash = true;
        }
    }
    Ok(format!("setting-account-{slug}"))
}

/// Port of `settingsPageSearch` (`apps/web/src/settingsNavigation.ts:313`).
/// `section` is a `Value` (not `Option`) so an explicit `null` stays `null`
/// in the output while a missing/`$undefined` arg serializes back as the
/// undefined marker.
pub fn settings_page_search(
    section: serde_json::Value,
    target: Option<String>,
) -> Result<serde_json::Value, String> {
    let mut out = Map::new();
    let section_out = match &section {
        Value::String(s) if s == "general" => crate::js::undefined(),
        _ => section,
    };
    out.insert("section".to_string(), section_out);
    out.insert(
        "target".to_string(),
        target.map_or_else(crate::js::undefined, Value::String),
    );
    out.insert("account".to_string(), crate::js::undefined());
    out.insert("provider".to_string(), crate::js::undefined());
    Ok(Value::Object(out))
}

/// Port of `providerAccountsSectionId` (`apps/web/src/settingsNavigation.ts:323`).
pub fn provider_accounts_section_id(provider: String) -> Result<String, String> {
    Ok(format!("accounts-{}", provider.to_lowercase()))
}

/// Port of `normalizeSettingsSection` (`apps/web/src/settingsNavigation.ts:327`).
pub fn normalize_settings_section(value: serde_json::Value) -> Result<serde_json::Value, String> {
    if let Value::String(s) = &value
        && SETTINGS_SECTION_IDS.contains(&s.as_str())
        && is_settings_section_available(s)
    {
        return Ok(value);
    }
    Ok(Value::String("general".to_string()))
}

/// Port of `isHostSettingsSection` (`apps/web/src/settingsNavigation.ts:351`).
pub fn is_host_settings_section(section: serde_json::Value) -> Result<bool, String> {
    Ok(matches!(&section, Value::String(s) if HOST_SETTINGS_SECTIONS.contains(&s.as_str())))
}
