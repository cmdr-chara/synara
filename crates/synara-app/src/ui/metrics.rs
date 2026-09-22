//! Source-derived desktop measurements for the pinned Electron reference.
//!
//! Reference: Emanuele-web04/synara eaa61eded31b6755d4f30ba8eabc5d905cf817cb,
//! apps/web/src/lib/{appDensity,appTypography}.ts and appSettings.ts (MIT).
//! Values are logical pixels, not screenshot pixels. Native font overrides above
//! the Electron range remain usable for accessibility instead of being discarded.
use synara_workspace::DensityPreference;

pub const DEFAULT_UI_FONT_SIZE: f32 = 13.0;
pub const DEFAULT_CODE_FONT_SIZE: f32 = 12.0;
pub const DEFAULT_TERMINAL_FONT_SIZE: u8 = 12;
pub const STANDARD_CHAT_WIDTH: f32 = 736.0;
pub const WIDE_CHAT_WIDTH: f32 = 1152.0;

pub fn density_scale(density: DensityPreference) -> f32 {
    match density {
        DensityPreference::Compact => 0.85,
        DensityPreference::Comfortable => 1.0,
        DensityPreference::Spacious => 1.15,
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DensityMetrics {
    pub row_height: f32,
    pub row_padding_y: f32,
    pub row_gap: f32,
    pub settings_row_padding_y: f32,
    pub chat_gutter: f32,
    pub chat_gutter_large: f32,
    pub composer_top: f32,
    pub composer_bottom: f32,
    pub composer_x: f32,
    pub composer_end: f32,
    pub composer_footer: f32,
    pub composer_footer_end: f32,
}

impl DensityMetrics {
    pub fn new(density: DensityPreference) -> Self {
        let scale = density_scale(density);
        Self {
            row_height: 28.0 * scale,
            row_padding_y: 2.0 * scale,
            row_gap: 8.0 * scale,
            settings_row_padding_y: 10.0 * scale,
            chat_gutter: 12.0 * scale,
            chat_gutter_large: 20.0 * scale,
            composer_top: 12.0 * scale,
            composer_bottom: 8.0 * scale,
            composer_x: 12.0 * scale,
            composer_end: 14.0 * scale,
            composer_footer: 6.0 * scale,
            composer_footer_end: 8.0 * scale,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Typography {
    pub ui: f32,
    pub large: f32,
    pub small: f32,
    pub extra_small: f32,
    pub tiny: f32,
    pub timestamp: f32,
    pub code: f32,
    pub chat_meta: f32,
    pub chat_tiny: f32,
}

impl Typography {
    pub fn from_base(base: f32) -> Self {
        let base = if base.is_finite() {
            base.round().clamp(8.0, 72.0)
        } else {
            DEFAULT_UI_FONT_SIZE
        };
        // The reference caps derived sizes at MAX_CHAT_FONT_SIZE_PX + 2.
        // Preserve explicit larger native accessibility preferences as well.
        let maximum = 20.0_f32.max((base * 1.08).ceil());
        let scaled = |factor: f32, minimum: f32| (base * factor).round().clamp(minimum, maximum);
        Self {
            ui: base,
            large: scaled(1.08, base),
            small: scaled(0.92, 10.0),
            extra_small: scaled(0.84, 10.0),
            tiny: scaled(0.76, 9.0),
            timestamp: scaled(0.72, 8.0),
            code: scaled(0.95, 10.0),
            chat_meta: scaled(0.72, 8.0),
            chat_tiny: scaled(0.66, 8.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use synara_workspace::{AppSettings, FontPreferences, Personalization};

    #[test]
    fn comfortable_density_matches_the_electron_rem_contract() {
        let metrics = DensityMetrics::new(DensityPreference::Comfortable);
        assert_eq!(metrics.row_height, 28.0);
        assert_eq!(metrics.row_padding_y, 2.0);
        assert_eq!(metrics.row_gap, 8.0);
        assert_eq!(metrics.settings_row_padding_y, 10.0);
        assert_eq!(metrics.chat_gutter, 12.0);
        assert_eq!(metrics.chat_gutter_large, 20.0);
        assert_eq!(metrics.composer_top, 12.0);
        assert_eq!(metrics.composer_bottom, 8.0);
        assert_eq!(metrics.composer_x, 12.0);
        assert_eq!(metrics.composer_end, 14.0);
        assert_eq!(metrics.composer_footer, 6.0);
        assert_eq!(metrics.composer_footer_end, 8.0);
    }

    #[test]
    fn density_scales_every_surface_continuously() {
        for (density, expected) in [
            (DensityPreference::Compact, 0.85),
            (DensityPreference::Comfortable, 1.0),
            (DensityPreference::Spacious, 1.15),
        ] {
            let metrics = DensityMetrics::new(density);
            assert!((metrics.row_height - 28.0 * expected).abs() < 0.001);
            assert!((metrics.settings_row_padding_y - 10.0 * expected).abs() < 0.001);
            assert!((metrics.composer_x - 12.0 * expected).abs() < 0.001);
        }
    }

    #[test]
    fn default_typography_matches_electron_and_persisted_defaults() {
        assert_eq!(FontPreferences::default().ui_size, DEFAULT_UI_FONT_SIZE);
        assert_eq!(FontPreferences::default().code_size, DEFAULT_CODE_FONT_SIZE);
        assert_eq!(
            Personalization::default().terminal_font_size,
            DEFAULT_TERMINAL_FONT_SIZE
        );
        assert_eq!(
            f32::from(Personalization::default().chat_width),
            STANDARD_CHAT_WIDTH
        );
        assert!(WIDE_CHAT_WIDTH > STANDARD_CHAT_WIDTH);
        assert_eq!(
            Typography::from_base(13.0),
            Typography {
                ui: 13.0,
                large: 14.0,
                small: 12.0,
                extra_small: 11.0,
                tiny: 10.0,
                timestamp: 9.0,
                code: 12.0,
                chat_meta: 9.0,
                chat_tiny: 9.0,
            }
        );
    }

    #[test]
    fn all_electron_base_sizes_use_the_reference_rounding() {
        for base in 11..=18 {
            let base = base as f32;
            let scale = Typography::from_base(base);
            assert_eq!(scale.ui, base);
            assert_eq!(scale.large, (base * 1.08).round().clamp(base, 20.0));
            assert_eq!(scale.small, (base * 0.92).round().clamp(10.0, 20.0));
            assert_eq!(scale.extra_small, (base * 0.84).round().clamp(10.0, 20.0));
            assert_eq!(scale.tiny, (base * 0.76).round().clamp(9.0, 20.0));
            assert_eq!(scale.timestamp, (base * 0.72).round().clamp(8.0, 20.0));
            assert_eq!(scale.code, (base * 0.95).round().clamp(10.0, 20.0));
        }
    }

    #[test]
    fn saved_native_fonts_are_not_silently_migrated() {
        let settings: AppSettings = serde_json::from_value(serde_json::json!({
            "version": 1,
            "appearance": { "fonts": {
                "ui_family": "Liberation Sans", "ui_size": 14.0,
                "code_family": null, "code_size": 13.0
            }}
        }))
        .unwrap();
        settings.validate().unwrap();
        assert_eq!(settings.appearance.fonts.ui_size, 14.0);
        assert_eq!(settings.appearance.fonts.code_size, 13.0);
        assert_eq!(Typography::from_base(72.0).ui, 72.0);
        assert_eq!(Typography::from_base(f32::NAN).ui, DEFAULT_UI_FONT_SIZE);
    }
}
