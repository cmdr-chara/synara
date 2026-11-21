//! The Rust port of `apps/web/src/settingsSearchIndex.ts` (`settingsSectionLabel`).

#![forbid(unsafe_code)]

use serde_json::Value;

/// `SETTINGS_NAV_ITEMS` filtered by `isSettingsSectionAvailable` in the recorded
/// environment (Stable build: the Beta-gated `connectors` section is excluded),
/// mapped through `t()` to its bundled English label.
fn settings_section_label_by_id(id: &str) -> Option<&'static str> {
    Some(match id {
        "profile" => "Profile",
        "accounts" => "Accounts",
        "general" => "General",
        "appearance" => "Appearance",
        "layout" => "Layout",
        "behavior" => "Chats",
        "notifications" => "Notifications",
        "shortcuts" => "Keyboard shortcuts",
        "providers" => "Providers",
        "models" => "Models",
        "skills" => "Skills",
        "integrations" => "External agents",
        "connections" => "Devices",
        "computer" => "Computer use",
        "appsnap" => "AppSnap",
        "worktrees" => "Worktrees",
        "archived" => "Archive",
        "privacy" => "Privacy",
        "advanced" => "Advanced",
        "about" => "About",
        // `usage`: ACCOUNT_BILLING_ENABLED is false in the recorded build
        // (no Electron, no VITE billing flags), so `settings:nav_usage`.
        "usage" => "Usage",
        _ => return None,
    })
}

/// Port of `settingsSectionLabel` (`apps/web/src/settingsSearchIndex.ts:401`).
/// `label === undefined ? section : t(label)` — unknown ids echo back the section.
pub fn settings_section_label(section: serde_json::Value) -> Result<serde_json::Value, String> {
    if let Value::String(s) = &section
        && let Some(label) = settings_section_label_by_id(s)
    {
        return Ok(serde_json::Value::String(label.to_string()));
    }
    Ok(section)
}
