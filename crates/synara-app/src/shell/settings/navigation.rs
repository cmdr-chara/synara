//! Settings taxonomy adapted from the pinned Electron settingsNavigation.ts.
//! Copyright (c) 2026 T3 Tools Inc. and Emanuele Di Pietro. MIT.
//! See docs/ui/electron-reference-LICENSE.txt for the retained license.
use super::{Glyph, Section, SectionInfo};

const fn item(
    section: Section,
    id: &'static str,
    group: &'static str,
    label: &'static str,
    icon: Glyph,
    description: &'static str,
) -> SectionInfo {
    SectionInfo {
        section,
        id,
        group,
        label,
        icon,
        description,
    }
}

pub(super) const SECTIONS: &[SectionInfo] = &[
    item(
        Section::Onboarding,
        "onboarding",
        "Personal",
        "Getting started",
        Glyph::Help,
        "Replay the welcome tour, check local agent commands, and set up your workspace.",
    ),
    item(
        Section::General,
        "general",
        "Personal",
        "General",
        Glyph::Settings,
        "Choose defaults for new chats, navigation, and the Environment panel.",
    ),
    item(
        Section::Profile,
        "profile",
        "Personal",
        "Profile",
        Glyph::User,
        "Your local activity, streaks, and a shareable stats card.",
    ),
    item(
        Section::Appearance,
        "appearance",
        "Personal",
        "Appearance",
        Glyph::Palette,
        "Customize the theme, typography, density, and time format.",
    ),
    item(
        Section::Notifications,
        "notifications",
        "Personal",
        "Notifications",
        Glyph::Bell,
        "Choose how Synara tells you when work finishes or needs attention.",
    ),
    item(
        Section::Behavior,
        "behavior",
        "Personal",
        "Chat behavior",
        Glyph::Sliders,
        "Control live responses, follow-ups, review defaults, and safety confirmations.",
    ),
    item(
        Section::Keybindings,
        "keybindings",
        "Personal",
        "Keybindings",
        Glyph::Shortcut,
        "Capture, customize, and add shortcuts for every Synara command.",
    ),
    item(
        Section::Usage,
        "usage",
        "Personal",
        "Usage & limits",
        Glyph::Gauge,
        "See remaining quota and credits for every signed-in provider.",
    ),
    item(
        Section::AppSnap,
        "appsnap",
        "Integrations",
        "AppSnap",
        Glyph::Capture,
        "Capture another app's frontmost window directly into a task.",
    ),
    item(
        Section::Computer,
        "computer",
        "Integrations",
        "Computer use",
        Glyph::Window,
        "Let agents see and control this computer's desktop, and check backend status.",
    ),
    item(
        Section::Mcp,
        "mcp",
        "Integrations",
        "MCP connections",
        Glyph::Plugin,
        "Give Codex, Claude, and other local agents scoped access to Synara tasks.",
    ),
    item(
        Section::Providers,
        "providers",
        "Coding",
        "Agent providers",
        Glyph::Puzzle,
        "Choose visible coding agents and manage their installed CLI tools.",
    ),
    item(
        Section::Models,
        "models",
        "Coding",
        "Models & writing",
        Glyph::Brain,
        "Choose the model used for Git writing and add custom model slugs.",
    ),
    item(
        Section::Skills,
        "skills",
        "Coding",
        "Agent skills",
        Glyph::Blocks,
        "Review reusable workflows discovered across all configured providers.",
    ),
    item(
        Section::Worktrees,
        "worktrees",
        "Coding",
        "Managed worktrees",
        Glyph::BranchSimple,
        "Review and clean up isolated workspaces created by Synara.",
    ),
    item(
        Section::System,
        "system",
        "System",
        "System tools",
        Glyph::Toolbox,
        "Manage sessions, recovery tools, low-level keybindings, and version details.",
    ),
    item(
        Section::Archived,
        "archived",
        "Archived",
        "Archived threads",
        Glyph::Archive,
        "Find and restore threads you previously archived.",
    ),
    // Native-specific capabilities remain reachable from System tools and search.
    // They do not displace the reference's primary navigation or lose saved state.
    item(
        Section::ProjectImport,
        "project-import",
        "Integrations",
        "Project import",
        Glyph::Folder,
        "Discover and review local Codex or Claude histories. Import unsent standalone chats without changing source files.",
    ),
    item(
        Section::DirectModels,
        "direct-models",
        "Integrations",
        "Direct models",
        Glyph::Brain,
        "Direct provider endpoints, secure API keys and reviewed model selection. Separate from ACP coding agents.",
    ),
    item(
        Section::Device,
        "device",
        "Integrations",
        "Device / capture",
        Glyph::Window,
        "Installed device helpers, captures, permissions and supported controls.",
    ),
    item(
        Section::Plugins,
        "plugins",
        "Integrations",
        "Plugins & integrations",
        Glyph::Plugin,
        "Synara-managed integrations and reported agent capabilities, with explicit ownership.",
    ),
    item(
        Section::Privacy,
        "privacy",
        "System",
        "Privacy & security",
        Glyph::Settings,
        "Local data, protocol diagnostics, secret-store status and safe deletion.",
    ),
    item(
        Section::Workflows,
        "workflows",
        "Integrations",
        "Subagents & workflows",
        Glyph::Blocks,
        "Create reviewed child-agent workflows, follow dependencies and usage, pause or stop, and review incoming client requests.",
    ),
];

pub(super) fn primary_section(section: Section) -> bool {
    !matches!(
        section,
        Section::ProjectImport
            | Section::DirectModels
            | Section::Device
            | Section::Plugins
            | Section::Privacy
            | Section::Workflows
    )
}

/// Preserve native accessibility/test identities while exposing the reference taxonomy.
#[cfg(test)]
pub(super) fn reference_section_id(section: Section) -> &'static str {
    match section {
        Section::Keybindings => "shortcuts",
        Section::Mcp => "integrations",
        Section::System => "advanced",
        _ => SECTIONS
            .iter()
            .find(|item| item.section == section)
            .map_or("general", |item| item.id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn primary_navigation_includes_onboarding_before_sixteen_reference_sections() {
        let actual: Vec<_> = SECTIONS
            .iter()
            .filter(|item| primary_section(item.section))
            .map(|item| (reference_section_id(item.section), item.group, item.label))
            .collect();
        assert_eq!(
            actual,
            vec![
                ("onboarding", "Personal", "Getting started"),
                ("general", "Personal", "General"),
                ("profile", "Personal", "Profile"),
                ("appearance", "Personal", "Appearance"),
                ("notifications", "Personal", "Notifications"),
                ("behavior", "Personal", "Chat behavior"),
                ("shortcuts", "Personal", "Keybindings"),
                ("usage", "Personal", "Usage & limits"),
                ("appsnap", "Integrations", "AppSnap"),
                ("computer", "Integrations", "Computer use"),
                ("integrations", "Integrations", "MCP connections"),
                ("providers", "Coding", "Agent providers"),
                ("models", "Coding", "Models & writing"),
                ("skills", "Coding", "Agent skills"),
                ("worktrees", "Coding", "Managed worktrees"),
                ("advanced", "System", "System tools"),
                ("archived", "Archived", "Archived threads"),
            ]
        );
    }

    #[test]
    fn native_extensions_and_stable_control_ids_are_preserved() {
        let ids: HashSet<_> = SECTIONS.iter().map(|item| item.id).collect();
        assert_eq!(ids.len(), SECTIONS.len());
        assert_eq!(SECTIONS.len(), 23);
        for id in [
            "workflows",
            "project-import",
            "direct-models",
            "device",
            "privacy",
            "plugins",
        ] {
            let item = SECTIONS.iter().find(|item| item.id == id).unwrap();
            assert!(!primary_section(item.section));
            assert!(!item.description.is_empty());
        }
        for id in ["keybindings", "mcp", "system"] {
            assert!(ids.contains(id));
        }
    }
}
