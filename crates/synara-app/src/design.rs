//! Synara-owned native presentation vocabulary. Domain state lives elsewhere.
use gpui::{Div, FontWeight, div, prelude::*, px, rgb};
pub const CANVAS: u32 = 0x17191f;
pub const NAVIGATION: u32 = 0x121419;
pub const SURFACE: u32 = 0x1e2129;
pub const ELEVATED: u32 = 0x252933;
pub const HOVER: u32 = 0x2b303c;
pub const SELECTED: u32 = 0x303044;
pub const BORDER: u32 = 0x343944;
pub const SEPARATOR: u32 = 0x252a33;
pub const TEXT: u32 = 0xe4e7ed;
pub const SECONDARY: u32 = 0xa6aebb;
pub const MUTED: u32 = 0x8792a3;
pub const ACCENT: u32 = 0xb5adff;
pub const ON_ACCENT: u32 = 0x252237;
pub const SUCCESS: u32 = 0x93c9ad;
pub const DANGER: u32 = 0xf0a1a4;
pub const WARNING: u32 = 0xd9bf8b;
pub const ERROR_SURFACE: u32 = 0x342429;
pub const CAUTION_SURFACE: u32 = 0x2d2922;
pub const CODE: u32 = 0x13161b;
pub const ADDED_SURFACE: u32 = 0x182c26;

pub const BODY: f32 = 13.;
pub const READING: f32 = 14.;
pub const CAPTION: f32 = 11.;
pub const TITLE: f32 = 16.;
pub const DISPLAY: f32 = 27.;
pub const CONTROL: f32 = 30.;
pub const CONTROL_RADIUS: f32 = 6.;
pub const COMPOSER_RADIUS: f32 = 11.;
pub const READING_WIDTH: f32 = 856.;
pub const NAV_WIDTH: f32 = 232.;
pub const GUTTER: f32 = 26.;
pub const SPACE: [f32; 6] = [4., 8., 12., 18., 26., 36.];
#[cfg(target_os = "macos")]
pub const UI_FONT: &str = ".SystemUIFont";
#[cfg(target_os = "windows")]
pub const UI_FONT: &str = "Segoe UI";
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub const UI_FONT: &str = "DejaVu Sans";
#[cfg(target_os = "macos")]
pub const MONO_FONT: &str = "Menlo";
#[cfg(target_os = "windows")]
pub const MONO_FONT: &str = "Consolas";
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub const MONO_FONT: &str = "DejaVu Sans Mono";
pub fn caption(text: impl Into<gpui::SharedString>) -> Div {
    div()
        .text_size(px(CAPTION))
        .text_color(rgb(SECONDARY))
        .child(text.into())
}
pub fn heading(text: impl Into<gpui::SharedString>) -> Div {
    div()
        .text_size(px(TITLE))
        .font_weight(FontWeight::SEMIBOLD)
        .child(text.into())
}
pub fn hairline() -> Div {
    div().h(px(1.)).w_full().flex_shrink_0().bg(rgb(SEPARATOR))
}
pub fn dot(color: u32) -> Div {
    div()
        .size(px(5.))
        .flex_shrink_0()
        .rounded_full()
        .bg(rgb(color))
}
/// These glyphs were independently drawn on a small coordinate grid for Synara.
#[derive(Clone, Copy)]
pub enum Symbol {
    Add,
    Chevron,
    Folder,
    Chat,
    File,
    Changes,
    Terminal,
    Agent,
    Settings,
    Remote,
    Close,
    Check,
    Up,
    Sidebar,
    Search,
    More,
    Shield,
    Copy,
    Refresh,
}
impl Symbol {
    pub fn view(self, color: u32) -> gpui::Svg {
        let path = match self {
            Self::Add => "icons/add.svg",
            Self::Chevron => "icons/chevron.svg",
            Self::Folder => "icons/folder.svg",
            Self::Chat => "icons/chat.svg",
            Self::File => "icons/file.svg",
            Self::Changes => "icons/changes.svg",
            Self::Terminal => "icons/terminal.svg",
            Self::Agent => "icons/agent.svg",
            Self::Settings => "icons/settings.svg",
            Self::Remote => "icons/remote.svg",
            Self::Close => "icons/close.svg",
            Self::Check => "icons/check.svg",
            Self::Up => "icons/up.svg",
            Self::Sidebar => "icons/sidebar.svg",
            Self::Search => "icons/search.svg",
            Self::More => "icons/more.svg",
            Self::Shield => "icons/shield.svg",
            Self::Copy => "icons/copy.svg",
            Self::Refresh => "icons/refresh.svg",
        };
        gpui::svg()
            .path(path)
            .size(px(16.))
            .flex_shrink_0()
            .text_color(rgb(color))
    }
}
pub struct Assets;
impl gpui::AssetSource for Assets {
    fn load(&self, path: &str) -> gpui::Result<Option<std::borrow::Cow<'static, [u8]>>> {
        let bytes: &'static [u8] = match path {
            "icons/add.svg" => include_bytes!("../assets/icons/add.svg"),
            "icons/chevron.svg" => include_bytes!("../assets/icons/chevron.svg"),
            "icons/folder.svg" => include_bytes!("../assets/icons/folder.svg"),
            "icons/chat.svg" => include_bytes!("../assets/icons/chat.svg"),
            "icons/file.svg" => include_bytes!("../assets/icons/file.svg"),
            "icons/changes.svg" => include_bytes!("../assets/icons/changes.svg"),
            "icons/terminal.svg" => include_bytes!("../assets/icons/terminal.svg"),
            "icons/agent.svg" => include_bytes!("../assets/icons/agent.svg"),
            "icons/settings.svg" => include_bytes!("../assets/icons/settings.svg"),
            "icons/remote.svg" => include_bytes!("../assets/icons/remote.svg"),
            "icons/close.svg" => include_bytes!("../assets/icons/close.svg"),
            "icons/check.svg" => include_bytes!("../assets/icons/check.svg"),
            "icons/up.svg" => include_bytes!("../assets/icons/up.svg"),
            "icons/sidebar.svg" => include_bytes!("../assets/icons/sidebar.svg"),
            "icons/search.svg" => include_bytes!("../assets/icons/search.svg"),
            "icons/more.svg" => include_bytes!("../assets/icons/more.svg"),
            "icons/shield.svg" => include_bytes!("../assets/icons/shield.svg"),
            "icons/copy.svg" => include_bytes!("../assets/icons/copy.svg"),
            "icons/refresh.svg" => include_bytes!("../assets/icons/refresh.svg"),
            "synara-mark.svg" => include_bytes!("../assets/synara-mark.svg"),
            _ => return Ok(None),
        };
        Ok(Some(std::borrow::Cow::Borrowed(bytes)))
    }
    fn list(&self, _: &str) -> gpui::Result<Vec<gpui::SharedString>> {
        Ok(vec![])
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn luminance(color: u32) -> f64 {
        [16, 8, 0]
            .into_iter()
            .zip([0.2126, 0.7152, 0.0722])
            .map(|(shift, weight)| {
                let n = ((color >> shift) & 255) as f64 / 255.;
                weight
                    * if n <= 0.04045 {
                        n / 12.92
                    } else {
                        ((n + 0.055) / 1.055).powf(2.4)
                    }
            })
            .sum()
    }
    #[test]
    fn text_roles_remain_readable_on_application_surfaces() {
        for text in [TEXT, SECONDARY, MUTED, ACCENT] {
            for surface in [CANVAS, NAVIGATION, SURFACE] {
                assert!(
                    (luminance(text) + 0.05) / (luminance(surface) + 0.05) >= 4.5,
                    "{text:x} on {surface:x}"
                );
            }
        }
    }
    #[test]
    fn spacing_has_a_single_ordered_scale() {
        assert!(SPACE.windows(2).all(|p| p[0] < p[1]));
        assert!(CONTROL >= 28.);
    }
}
