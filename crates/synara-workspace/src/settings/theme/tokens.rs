//! Electron's sRGB theme derivation, including its zero-contrast curve.
//! Adapted from the pinned theme.logic.ts under the retained MIT license.
use super::{ChromeTheme, ThemeHex, ThemeVariant};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct ThemePaint {
    pub rgb: u32,
    pub alpha: f64,
}
impl ThemePaint {
    pub fn opaque(rgb: u32) -> Self { Self { rgb, alpha: 1.0 } }
    pub fn rgba(rgb: u32, alpha: f64) -> Self {
        // The Electron CSS formatter rounds alpha to three decimal places.
        Self { rgb, alpha: (alpha.clamp(0.0, 1.0) * 1000.0).round() / 1000.0 }
    }
    pub fn over(self, background: u32) -> u32 { mix(background, self.rgb, self.alpha) }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ThemeTokens {
    pub contrast: f64,
    pub surface: u32,
    pub surface_under: u32,
    pub panel: u32,
    pub editor_background: u32,
    pub composer_focus_border: u32,
    pub colors: BTreeMap<&'static str, ThemePaint>,
}
impl ThemeTokens {
    pub fn derive(theme: &ChromeTheme, variant: ThemeVariant) -> Self {
        let dark = variant == ThemeVariant::Dark;
        let baseline = if dark { 60.0 } else { 45.0 };
        let value = f64::from(theme.contrast.min(100));
        let curved = value / 100.0 + ((value - baseline) / 60.0) * 0.7;
        let c = if value <= baseline { curved } else { baseline / 100.0 + (curved - baseline / 100.0) * 2.0 };
        let surface = theme.surface.value();
        let ink = theme.ink.value();
        let accent = theme.accent.value();
        let white = 0xffffff;
        let black = 0;
        let surface_under = mix(surface, if dark { black } else { ink },
            if dark { 0.16 + (value - baseline) * 0.0015 } else { 0.04 + (value - baseline) * 0.0012 });
        let panel = mix(surface, if dark { ink } else { white }, if dark { 0.03 + c * 0.03 } else { 0.18 + c * 0.008 });
        let editor_background = mix(surface, if dark { ink } else { white }, if dark { 0.07 } else { 0.12 });
        let composer_focus_border = mix(panel, if dark { white } else { ink }, if dark { 0.12 + c * 0.06 } else { 0.1 + c * 0.05 });
        let mut colors = BTreeMap::new();
        let opaque = ThemePaint::opaque;
        let alpha = ThemePaint::rgba;
        if dark {
            let control = mix(surface, ink, 0.06 + c * 0.05);
            let focus = mix(accent, white, 0.3 + c * 0.15);
            let elevated = mix(surface, ink, 0.08 + c * 0.08);
            colors.extend([
                ("accentBackground", opaque(mix(black, accent, 0.2 + c * 0.08))),
                ("accentBackgroundActive", opaque(mix(black, accent, 0.22 + c * 0.12))),
                ("accentBackgroundHover", opaque(mix(black, accent, 0.21 + c * 0.1))),
                ("border", alpha(ink, 0.1 + c * 0.04)),
                ("borderFocus", alpha(focus, 0.7 + c * 0.1)),
                ("borderHeavy", alpha(ink, 0.16 + c * 0.06)),
                ("borderLight", alpha(ink, 0.06 + c * 0.02)),
                ("buttonPrimaryBackground", opaque(ink)),
                ("buttonPrimaryBackgroundActive", alpha(ink, 0.07 + c * 0.05)),
                ("buttonPrimaryBackgroundHover", alpha(ink, 0.04 + c * 0.03)),
                ("buttonPrimaryBackgroundInactive", alpha(ink, 0.02 + c * 0.02)),
                ("buttonSecondaryBackground", alpha(ink, 0.04 + c * 0.02)),
                ("buttonSecondaryBackgroundActive", alpha(ink, 0.09 + c * 0.05)),
                ("buttonSecondaryBackgroundHover", alpha(ink, 0.06 + c * 0.03)),
                ("buttonSecondaryBackgroundInactive", alpha(ink, 0.02 + c * 0.03)),
                ("buttonTertiaryBackground", alpha(ink, 0.02 + c * 0.015)),
                ("buttonTertiaryBackgroundActive", alpha(ink, 0.07 + c * 0.05)),
                ("buttonTertiaryBackgroundHover", alpha(ink, 0.05 + c * 0.03)),
                ("controlBackground", alpha(control, 0.96)),
                ("controlBackgroundOpaque", opaque(control)),
                ("elevatedPrimary", alpha(elevated, 0.96)),
                ("elevatedPrimaryOpaque", opaque(elevated)),
                ("elevatedSecondary", alpha(ink, 0.02 + c * 0.02)),
                ("elevatedSecondaryOpaque", opaque(mix(surface, ink, 0.04 + c * 0.05))),
                ("iconAccent", opaque(focus)),
                ("iconPrimary", alpha(ink, 0.82 + c * 0.14)),
                ("iconSecondary", alpha(ink, 0.65 + c * 0.1)),
                ("iconTertiary", alpha(ink, 0.45 + c * 0.1)),
                ("simpleScrim", alpha(ink, 0.08 + c * 0.04)),
                ("textAccent", opaque(focus)),
                ("textButtonPrimary", opaque(surface)),
                ("textButtonSecondary", opaque(mix(ink, surface, 0.7 + c * 0.1))),
                ("textButtonTertiary", alpha(ink, 0.45 + c * 0.1)),
                ("textForeground", opaque(ink)),
                ("textForegroundSecondary", alpha(ink, 0.65 + c * 0.1)),
                ("textForegroundTertiary", alpha(ink, 0.42 + c * 0.13)),
            ]);
        } else {
            let control = mix(surface, white, 0.09 + c * 0.04);
            let secondary = mix(surface, white, 0.08 + c * 0.08);
            let primary = mix(surface, white, 0.16 + c * 0.12);
            colors.extend([
                ("accentBackground", opaque(mix(surface, accent, 0.11 + c * 0.04))),
                ("accentBackgroundActive", opaque(mix(surface, accent, 0.13 + c * 0.05))),
                ("accentBackgroundHover", opaque(mix(surface, accent, 0.12 + c * 0.045))),
                ("border", alpha(ink, 0.09 + c * 0.04)),
                ("borderFocus", opaque(accent)),
                ("borderHeavy", alpha(ink, 0.09 + c * 0.06)),
                ("borderLight", alpha(ink, 0.07 + c * 0.02)),
                ("buttonPrimaryBackground", opaque(ink)),
                ("buttonPrimaryBackgroundActive", alpha(ink, 0.1 + c * 0.12)),
                ("buttonPrimaryBackgroundHover", alpha(ink, 0.05 + c * 0.06)),
                ("buttonPrimaryBackgroundInactive", alpha(ink, 0.18 + c * 0.14)),
                ("buttonSecondaryBackground", alpha(ink, 0.03)),
                ("buttonSecondaryBackgroundActive", alpha(ink, 0.03 + c * 0.02)),
                ("buttonSecondaryBackgroundHover", alpha(ink, 0.03)),
                ("buttonSecondaryBackgroundInactive", alpha(ink, 0.01 + c * 0.02)),
                ("buttonTertiaryBackground", alpha(ink, 0.0)),
                ("buttonTertiaryBackgroundActive", alpha(ink, 0.16 + c * 0.08)),
                ("buttonTertiaryBackgroundHover", alpha(ink, 0.08 + c * 0.04)),
                ("controlBackground", alpha(control, 0.96)),
                ("controlBackgroundOpaque", opaque(control)),
                ("elevatedPrimary", alpha(primary, 0.96)),
                ("elevatedPrimaryOpaque", opaque(primary)),
                ("elevatedSecondary", alpha(ink, 0.04)),
                ("elevatedSecondaryOpaque", opaque(secondary)),
                ("iconAccent", opaque(accent)),
                ("iconPrimary", opaque(ink)),
                ("iconSecondary", alpha(ink, 0.65 + c * 0.1)),
                ("iconTertiary", alpha(ink, 0.45 + c * 0.1)),
                ("simpleScrim", alpha(black, 0.08 + c * 0.04)),
                ("textAccent", opaque(accent)),
                ("textButtonPrimary", opaque(surface)),
                ("textButtonSecondary", opaque(ink)),
                ("textButtonTertiary", alpha(ink, 0.45 + c * 0.1)),
                ("textForeground", opaque(ink)),
                ("textForegroundSecondary", alpha(ink, 0.65 + c * 0.1)),
                ("textForegroundTertiary", alpha(ink, 0.45 + c * 0.1)),
            ]);
        }
        colors.extend([
            ("diffAdded", opaque(theme.semantic_colors.diff_added.value())),
            ("diffRemoved", opaque(theme.semantic_colors.diff_removed.value())),
            ("skill", opaque(theme.semantic_colors.skill.value())),
            ("editorAdded", alpha(theme.semantic_colors.diff_added.value(), if dark { 0.23 } else { 0.15 })),
            ("editorRemoved", alpha(theme.semantic_colors.diff_removed.value(), if dark { 0.23 } else { 0.15 })),
        ]);
        Self { contrast: c, surface, surface_under, panel, editor_background, composer_focus_border, colors }
    }
    pub fn paint(&self, role: &str) -> Option<ThemePaint> { self.colors.get(role).copied() }
    pub fn color_on_surface(&self, role: &str) -> Option<u32> { self.paint(role).map(|paint| paint.over(self.surface)) }
}

pub fn mix(from: u32, to: u32, amount: f64) -> u32 {
    let amount = amount.clamp(0.0, 1.0);
    let channel = |shift| {
        let a = f64::from((from >> shift) & 255_u32);
        let b = f64::from((to >> shift) & 255_u32);
        (a + (b - a) * amount).round() as u32
    };
    (channel(16) << 16) | (channel(8) << 8) | channel(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ThemePack, theme_catalog};

    #[test]
    fn zero_contrast_is_not_the_curve_baseline() {
        let light = ThemeTokens::derive(&ThemePack::codex(ThemeVariant::Light).theme, ThemeVariant::Light);
        let dark = ThemeTokens::derive(&ThemePack::codex(ThemeVariant::Dark).theme, ThemeVariant::Dark);
        assert!((light.contrast + 0.525).abs() < 1e-10);
        assert!((dark.contrast + 0.7).abs() < 1e-10);
        assert_eq!(light.surface, 0xffffff);
        assert_eq!(dark.surface, 0x111111);
        assert_eq!(light.surface_under, 0xffffff);
        assert_eq!(dark.surface_under, 0x101010);
        assert_eq!(light.paint("border").unwrap().alpha, 0.069);
        assert_eq!(dark.paint("border").unwrap().alpha, 0.072);
        assert_eq!(dark.paint("textForegroundSecondary").unwrap().alpha, 0.58);
    }

    #[test]
    fn every_catalog_variant_has_the_same_complete_finite_role_set_at_all_contrasts() {
        let mut expected = None;
        for variants in theme_catalog().unwrap().values() {
            for (&variant, seed) in variants {
                for contrast in 0..=100 {
                    let mut theme = seed.clone();
                    theme.contrast = contrast;
                    let tokens = ThemeTokens::derive(&theme, variant);
                    let keys: Vec<_> = tokens.colors.keys().copied().collect();
                    if let Some(expected) = &expected { assert_eq!(&keys, expected); }
                    else { expected = Some(keys); }
                    assert_eq!(tokens.colors.len(), 41);
                    for paint in tokens.colors.values() {
                        assert!(paint.rgb <= 0xffffff);
                        assert!(paint.alpha.is_finite() && (0.0..=1.0).contains(&paint.alpha));
                    }
                }
            }
        }
    }

    #[test]
    fn srgba_rounding_and_compositing_match_the_reference_color_model() {
        assert_eq!(mix(0x000000, 0xffffff, 0.5), 0x808080);
        assert_eq!(mix(0xffffff, 0, -0.2), 0xffffff);
        assert_eq!(mix(0, 0xffffff, 1.2), 0xffffff);
        assert_eq!(ThemePaint::rgba(0xff0000, 0.12345).alpha, 0.123);
        assert_eq!(ThemePaint::rgba(0xff0000, 0.5).over(0xffffff), 0xff8080);
        let mut theme = ThemePack::codex(ThemeVariant::Dark).theme;
        theme.surface = ThemeHex::rgb(0x262626);
        assert_eq!(ThemeTokens::derive(&theme, ThemeVariant::Dark).surface, 0x262626);
    }
}
