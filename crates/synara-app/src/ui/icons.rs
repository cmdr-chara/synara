//! Exact Synara glyph assets and its pinned Tabler/Simple Icons choices.
//! Asset origins and hashes are in assets/icons/manifest.json.
use gpui::{Styled, Svg, px, rgb, svg};
use std::borrow::Cow;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Glyph {
    Back,
    Forward,
    Send,
    Plus,
    Search,
    Folder,
    Folders,
    Compose,
    Chevron,
    ChevronRight,
    Settings,
    More,
    Panel,
    PanelRight,
    Dock,
    Terminal,
    Files,
    Changes,
    Handoff,
    Copy,
    Stop,
    Kanban,
    PullRequest,
    Clock,
    Help,
    Notebook,
    Chat,
    Shield,
    Mic,
    Minimize,
    Maximize,
    Restore,
    Check,
    Close,
    Agent,
    Fork,
    Pin,
    Browser,
    Attach,
    Goal,
    Plan,
    Debug,
    Down,
    Window,
    Error,
    OpenAI,
    OpenCode,
    Archive,
    Toolbox,
    BranchSimple,
    Blocks,
    Brain,
    Puzzle,
    Plugin,
    Capture,
    Gauge,
    Shortcut,
    Sliders,
    Bell,
    Palette,
    User,
}
impl Glyph {
    fn path(self) -> &'static str {
        match self {
            Self::Back => "icons/tabler/arrow-left.svg",
            Self::Forward => "icons/tabler/arrow-right.svg",
            Self::Send => "icons/synara/arrow-up.svg",
            Self::Plus => "icons/tabler/plus.svg",
            Self::Search => "icons/synara/magnifying-glass.svg",
            Self::Folder => "icons/synara/folder-2.svg",
            Self::Folders => "icons/synara/folders.svg",
            Self::Compose => "icons/synara/compose-pencil.svg",
            Self::Chevron => "icons/tabler/chevron-down.svg",
            Self::ChevronRight => "icons/tabler/chevron-right.svg",
            Self::Settings => "icons/synara/settings-gear-4.svg",
            Self::More => "icons/tabler/dots.svg",
            Self::Panel => "icons/synara/sidebar-simple-left-wide.svg",
            Self::PanelRight => "icons/synara/sidebar-simple-right-wide.svg",
            Self::Dock => "icons/synara/window.svg",
            Self::Terminal => "icons/synara/console.svg",
            Self::Files => "icons/tabler/file.svg",
            Self::Changes => "icons/synara/changes.svg",
            Self::Handoff => "icons/synara/arrow-left-right.svg",
            Self::Copy => "icons/synara/square-behind-square-6.svg",
            Self::Stop => "icons/synara/stop-fill.svg",
            Self::Kanban => "icons/synara/columns-3-wide.svg",
            Self::PullRequest => "icons/synara/pull-request.svg",
            Self::Clock => "icons/synara/clock.svg",
            Self::Help => "icons/synara/circle-questionmark.svg",
            Self::Notebook => "icons/synara/notes.svg",
            Self::Chat => "icons/synara/chat-bubble-7.svg",
            Self::Shield => "icons/synara/shield-code.svg",
            Self::Mic => "icons/synara/microphone.svg",
            Self::Minimize => "icons/tabler/minus.svg",
            Self::Restore => "icons/tabler/minimize.svg",
            Self::Check => "icons/tabler/check.svg",
            Self::Maximize => "icons/tabler/maximize.svg",
            Self::Close => "icons/tabler/x.svg",
            Self::Agent => "icons/synara/robot.svg",
            Self::Fork => "icons/synara/branch.svg",
            Self::Pin => "icons/synara/pin.svg",
            Self::Browser => "icons/synara/globe.svg",
            Self::Attach => "icons/tabler/paperclip.svg",
            Self::Goal => "icons/synara/target-arrow.svg",
            Self::Plan => "icons/tabler/list-details.svg",
            Self::Debug => "icons/tabler/bug.svg",
            Self::Down => "icons/tabler/arrow-down.svg",
            Self::Window => "icons/synara/window.svg",
            Self::Error => "icons/tabler/alert-circle.svg",
            Self::OpenAI => "icons/providers/openai.svg",
            Self::User => "icons/synara/user.svg",
            Self::Palette => "icons/synara/color-palette.svg",
            Self::Bell => "icons/synara/bell.svg",
            Self::Sliders => "icons/synara/settings-slider-hor.svg",
            Self::Shortcut => "icons/synara/shortcut.svg",
            Self::Gauge => "icons/synara/gauge.svg",
            Self::Capture => "icons/synara/screen-capture.svg",
            Self::Plugin => "icons/synara/plugin-1.svg",
            Self::Puzzle => "icons/synara/puzzle.svg",
            Self::Brain => "icons/synara/brain.svg",
            Self::Blocks => "icons/synara/building-blocks.svg",
            Self::BranchSimple => "icons/synara/branch-simple.svg",
            Self::Toolbox => "icons/synara/toolbox.svg",
            Self::Archive => "icons/synara/archive.svg",
            Self::OpenCode => "icons/synara/opencode.svg",
        }
    }
}

pub fn icon(glyph: Glyph) -> Svg {
    svg()
        .path(glyph.path())
        .size(px(16.))
        .flex_shrink_0()
        .text_color(rgb(super::palette().muted))
}

pub fn provider_glyph(id: &str, executable: Option<&str>) -> Glyph {
    // Only known identities get provider branding; custom agents retain Synara's
    // own generic robot. A profile display name is not a trusted provider ID.
    for identity in [Some(id), executable].into_iter().flatten() {
        match identity.to_ascii_lowercase().as_str() {
            "codex"
            | "codex-acp"
            | "@zed-industries/codex-acp"
            | "registry-codex"
            | "registry-codex-acp" => return Glyph::OpenAI,
            "opencode" | "opencode-acp" | "registry-opencode" | "registry-opencode-acp" => {
                return Glyph::OpenCode;
            }
            _ => {}
        }
    }
    Glyph::Agent
}

pub(super) fn load(path: &str) -> Option<Cow<'static, [u8]>> {
    let bytes: &'static [u8] = match path {
        "icons/providers/openai.svg" => include_bytes!("../../assets/icons/providers/openai.svg"),
        "icons/synara/arrow-left-right.svg" => {
            include_bytes!("../../assets/icons/synara/arrow-left-right.svg")
        }
        "icons/synara/arrow-up.svg" => include_bytes!("../../assets/icons/synara/arrow-up.svg"),
        "icons/synara/branch.svg" => include_bytes!("../../assets/icons/synara/branch.svg"),
        "icons/synara/changes.svg" => include_bytes!("../../assets/icons/synara/changes.svg"),
        "icons/synara/chat-bubble-7.svg" => {
            include_bytes!("../../assets/icons/synara/chat-bubble-7.svg")
        }
        "icons/synara/circle-questionmark.svg" => {
            include_bytes!("../../assets/icons/synara/circle-questionmark.svg")
        }
        "icons/synara/clock.svg" => include_bytes!("../../assets/icons/synara/clock.svg"),
        "icons/synara/columns-3-wide.svg" => {
            include_bytes!("../../assets/icons/synara/columns-3-wide.svg")
        }
        "icons/synara/compose-pencil.svg" => {
            include_bytes!("../../assets/icons/synara/compose-pencil.svg")
        }
        "icons/synara/console.svg" => include_bytes!("../../assets/icons/synara/console.svg"),
        "icons/synara/folder-2.svg" => include_bytes!("../../assets/icons/synara/folder-2.svg"),
        "icons/synara/folders.svg" => include_bytes!("../../assets/icons/synara/folders.svg"),
        "icons/synara/globe.svg" => include_bytes!("../../assets/icons/synara/globe.svg"),
        "icons/synara/magnifying-glass.svg" => {
            include_bytes!("../../assets/icons/synara/magnifying-glass.svg")
        }
        "icons/synara/microphone.svg" => include_bytes!("../../assets/icons/synara/microphone.svg"),
        "icons/synara/notes.svg" => include_bytes!("../../assets/icons/synara/notes.svg"),
        "icons/synara/user.svg" => include_bytes!("../../assets/icons/synara/user.svg"),
        "icons/synara/color-palette.svg" => {
            include_bytes!("../../assets/icons/synara/color-palette.svg")
        }
        "icons/synara/bell.svg" => include_bytes!("../../assets/icons/synara/bell.svg"),
        "icons/synara/settings-slider-hor.svg" => {
            include_bytes!("../../assets/icons/synara/settings-slider-hor.svg")
        }
        "icons/synara/shortcut.svg" => include_bytes!("../../assets/icons/synara/shortcut.svg"),
        "icons/synara/gauge.svg" => include_bytes!("../../assets/icons/synara/gauge.svg"),
        "icons/synara/screen-capture.svg" => {
            include_bytes!("../../assets/icons/synara/screen-capture.svg")
        }
        "icons/synara/plugin-1.svg" => include_bytes!("../../assets/icons/synara/plugin-1.svg"),
        "icons/synara/puzzle.svg" => include_bytes!("../../assets/icons/synara/puzzle.svg"),
        "icons/synara/brain.svg" => include_bytes!("../../assets/icons/synara/brain.svg"),
        "icons/synara/building-blocks.svg" => {
            include_bytes!("../../assets/icons/synara/building-blocks.svg")
        }
        "icons/synara/branch-simple.svg" => {
            include_bytes!("../../assets/icons/synara/branch-simple.svg")
        }
        "icons/synara/toolbox.svg" => include_bytes!("../../assets/icons/synara/toolbox.svg"),
        "icons/synara/archive.svg" => include_bytes!("../../assets/icons/synara/archive.svg"),
        "icons/synara/opencode.svg" => include_bytes!("../../assets/icons/synara/opencode.svg"),
        "icons/synara/pin.svg" => include_bytes!("../../assets/icons/synara/pin.svg"),
        "icons/synara/pull-request.svg" => {
            include_bytes!("../../assets/icons/synara/pull-request.svg")
        }
        "icons/synara/robot.svg" => include_bytes!("../../assets/icons/synara/robot.svg"),
        "icons/synara/settings-gear-4.svg" => {
            include_bytes!("../../assets/icons/synara/settings-gear-4.svg")
        }
        "icons/synara/shield-code.svg" => {
            include_bytes!("../../assets/icons/synara/shield-code.svg")
        }
        "icons/synara/sidebar-simple-left-wide.svg" => {
            include_bytes!("../../assets/icons/synara/sidebar-simple-left-wide.svg")
        }
        "icons/synara/sidebar-simple-right-wide.svg" => {
            include_bytes!("../../assets/icons/synara/sidebar-simple-right-wide.svg")
        }
        "icons/synara/square-behind-square-6.svg" => {
            include_bytes!("../../assets/icons/synara/square-behind-square-6.svg")
        }
        "icons/synara/stop-fill.svg" => include_bytes!("../../assets/icons/synara/stop-fill.svg"),
        "icons/synara/target-arrow.svg" => {
            include_bytes!("../../assets/icons/synara/target-arrow.svg")
        }
        "icons/synara/window.svg" => include_bytes!("../../assets/icons/synara/window.svg"),
        "icons/tabler/alert-circle.svg" => {
            include_bytes!("../../assets/icons/tabler/alert-circle.svg")
        }
        "icons/tabler/arrow-down.svg" => include_bytes!("../../assets/icons/tabler/arrow-down.svg"),
        "icons/tabler/arrow-left.svg" => include_bytes!("../../assets/icons/tabler/arrow-left.svg"),
        "icons/tabler/arrow-right.svg" => {
            include_bytes!("../../assets/icons/tabler/arrow-right.svg")
        }
        "icons/tabler/bug.svg" => include_bytes!("../../assets/icons/tabler/bug.svg"),
        "icons/tabler/chevron-down.svg" => {
            include_bytes!("../../assets/icons/tabler/chevron-down.svg")
        }
        "icons/tabler/chevron-right.svg" => {
            include_bytes!("../../assets/icons/tabler/chevron-right.svg")
        }
        "icons/tabler/dots.svg" => include_bytes!("../../assets/icons/tabler/dots.svg"),
        "icons/tabler/file.svg" => include_bytes!("../../assets/icons/tabler/file.svg"),
        "icons/tabler/list-details.svg" => {
            include_bytes!("../../assets/icons/tabler/list-details.svg")
        }
        "icons/tabler/minus.svg" => include_bytes!("../../assets/icons/tabler/minus.svg"),
        "icons/tabler/paperclip.svg" => include_bytes!("../../assets/icons/tabler/paperclip.svg"),
        "icons/tabler/plus.svg" => include_bytes!("../../assets/icons/tabler/plus.svg"),
        "icons/tabler/x.svg" => include_bytes!("../../assets/icons/tabler/x.svg"),
        "icons/tabler/maximize.svg" => include_bytes!("../../assets/icons/tabler/maximize.svg"),
        "icons/tabler/minimize.svg" => include_bytes!("../../assets/icons/tabler/minimize.svg"),
        "icons/tabler/check.svg" => include_bytes!("../../assets/icons/tabler/check.svg"),
        _ => return None,
    };
    Some(Cow::Borrowed(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn provider_identity_controls_branding_without_guessing_from_display_labels() {
        assert_eq!(provider_glyph("codex", None), Glyph::OpenAI);
        assert_eq!(
            provider_glyph("registry-codex-acp", Some("npx")),
            Glyph::OpenAI
        );
        assert_eq!(provider_glyph("registry-opencode", None), Glyph::OpenCode);
        assert_eq!(provider_glyph("custom", Some("opencode")), Glyph::OpenCode);
        assert_eq!(provider_glyph("fixture-alpha", None), Glyph::Agent);
        assert_eq!(provider_glyph("my-codex-notes", None), Glyph::Agent);
    }
}
