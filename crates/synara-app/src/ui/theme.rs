//! Cached projection of the source-derived Electron theme into native paint.
//! The existing semantic Palette remains the compatibility boundary for views
//! not yet using alpha-aware roles. The color derivation itself is shared with
//! persistence and tested independently against the pinned JavaScript helpers.
use super::Palette;
use gpui::Rgba;
use std::cell::{Cell, RefCell};
use synara_workspace::{AppearanceSettings, Colorway, ThemePack, ThemeTokens, ThemeVariant};

struct CachedTheme {
    variant: ThemeVariant,
    pack: ThemePack,
    tokens: ThemeTokens,
}
thread_local! {
    static ACTIVE: RefCell<Option<CachedTheme>> = const { RefCell::new(None) };
    static VARIANT: Cell<ThemeVariant> = const { Cell::new(ThemeVariant::Light) };
    static EXACT_ROLES: Cell<bool> = const { Cell::new(false) };
}

pub fn active_variant() -> ThemeVariant { VARIANT.get() }

pub fn configure(appearance: &AppearanceSettings, dark: bool) -> Option<Palette> {
    let variant = if dark { ThemeVariant::Dark } else { ThemeVariant::Light };
    VARIANT.set(variant);
    EXACT_ROLES.set(!appearance.high_contrast
        && appearance.personalization.colorway == Colorway::Original
        && appearance.personalization.accent.is_none());
    let Some(preferences) = &appearance.electron_theme else {
        ACTIVE.with(|active| *active.borrow_mut() = None);
        return None;
    };
    let pack = preferences.pack(variant);
    ACTIVE.with(|active| {
        let mut active = active.borrow_mut();
        if !active.as_ref().is_some_and(|cached| cached.variant == variant && cached.pack == *pack) {
            *active = Some(CachedTheme { variant, pack: pack.clone(), tokens: ThemeTokens::derive(&pack.theme, variant) });
        }
        active.as_ref().map(|cached| project_palette(&cached.tokens))
    })
}

fn project_palette(tokens: &ThemeTokens) -> Palette {
    let color = |name| tokens.color_on_surface(name).unwrap_or(tokens.surface);
    Palette {
        canvas: tokens.surface,
        sidebar: tokens.surface,
        overlay: color("controlBackgroundOpaque"),
        hover: color("buttonSecondaryBackgroundHover"),
        selected: color("buttonSecondaryBackground"),
        border: color("border"),
        text: color("textForeground"),
        muted: color("textForegroundSecondary"),
        focus: color("textAccent"),
        error: color("diffRemoved"),
        error_surface: color("editorRemoved"),
        notice_surface: color("accentBackground"),
    }
}

/// Alpha is retained until native compositing rather than flattened twice.
/// Explicit high-contrast/colorway overrides continue to use their own palette.
pub fn paint(role: &str, fallback: u32) -> Rgba {
    let found = EXACT_ROLES.get().then(|| ACTIVE.with(|active| {
        active.borrow().as_ref().and_then(|cached| cached.tokens.paint(role))
    })).flatten();
    if let Some(color) = found {
        Rgba {
            r: ((color.rgb >> 16) & 255) as f32 / 255.0,
            g: ((color.rgb >> 8) & 255) as f32 / 255.0,
            b: (color.rgb & 255) as f32 / 255.0,
            a: color.alpha as f32,
        }
    } else { gpui::rgb(fallback) }
}

pub fn composer_focus(fallback: u32) -> u32 {
    ACTIVE.with(|active| active.borrow().as_ref().map_or(fallback, |cached| cached.tokens.composer_focus_border))
}

/// The native theme editor and imported packs name installed font families.
/// CSS stacks are not executable native font expressions: use their first
/// explicit family, with generic families resolved to the existing OS default.
pub fn family(value: &str, default: &str) -> String {
    let value = value.trim();
    if value.is_empty() { return default.into(); }
    let mut quoted = None;
    let mut end = value.len();
    for (index, character) in value.char_indices() {
        if let Some(quote) = quoted {
            if character == quote { quoted = None; }
        } else if character == '\'' || character == '"' {
            quoted = Some(character);
        } else if character == ',' { end = index; break; }
    }
    let family = value[..end].trim().trim_matches(['\'', '"']).trim();
    if family.is_empty() || family.contains('(') || matches!(family,
        "inherit" | "initial" | "unset" | "revert" | "revert-layer" |
        "system-ui" | "sans-serif" | "monospace" | "ui-monospace" | "ui-sans-serif") {
        default.into()
    } else { family.into() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use synara_workspace::ThemePreferences;

    #[test]
    fn stock_projection_uses_primary_surfaces_and_real_accent_in_both_modes() {
        let appearance = AppearanceSettings::default();
        let light = configure(&appearance, false).unwrap();
        assert_eq!(light.canvas, 0xffffff);
        assert_eq!(light.sidebar, light.canvas);
        assert_eq!(light.text, 0x0d0d0d);
        assert_eq!(light.focus, 0x0169cc);
        assert!((paint("border", 0).a - 0.069).abs() < 0.0001);
        let dark = configure(&appearance, true).unwrap();
        assert_eq!(dark.canvas, 0x111111);
        assert_eq!(dark.sidebar, dark.canvas);
        assert_eq!(dark.text, 0xfcfcfc);
        assert!((paint("border", 0).a - 0.072).abs() < 0.0001);
        assert_eq!(active_variant(), ThemeVariant::Dark);
    }

    #[test]
    fn explicit_accessibility_overrides_and_legacy_profiles_do_not_leave_stale_theme_roles() {
        let mut appearance = AppearanceSettings::default();
        configure(&appearance, false).unwrap();
        appearance.high_contrast = true;
        configure(&appearance, false).unwrap();
        assert_eq!(paint("border", 0x112233), gpui::rgb(0x112233));
        appearance.electron_theme = None;
        assert!(configure(&appearance, true).is_none());
        assert_eq!(paint("textForeground", 0xabcdef), gpui::rgb(0xabcdef));
        appearance.high_contrast = false;
        let mut themes = ThemePreferences::default();
        themes.select("dracula", ThemeVariant::Dark).unwrap();
        appearance.electron_theme = Some(themes);
        assert_eq!(configure(&appearance, true).unwrap().canvas, 0x282a36);
    }

    #[test]
    fn css_font_stack_parsing_never_treats_expressions_as_native_families() {
        assert_eq!(family("\"My Font\", sans-serif", "System"), "My Font");
        assert_eq!(family("'Font, With Comma', serif", "System"), "Font, With Comma");
        assert_eq!(family("var(--font-family)", "System"), "System");
        assert_eq!(family("ui-monospace", "Mono"), "Mono");
    }
}
