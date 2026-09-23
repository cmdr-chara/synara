//! Metrics shared with the pinned Electron reference.
//!
//! Source: Emanuele-web04/synara eaa61eded31b6755d4f30ba8eabc5d905cf817cb,
//! apps/web/src/lib/appDensity.ts and apps/web/src/lib/appTypography.ts.
//! These functions do not alter stored preferences or provider behavior.
use synara_workspace::DensityPreference;

pub(super) fn density_scale(value: DensityPreference) -> f32 {
    match value {
        DensityPreference::Compact => 0.85,
        DensityPreference::Comfortable => 1.0,
        DensityPreference::Spacious => 1.15,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn density_matches_the_reference_scale_without_rounding_drift() {
        for (density, scale) in [
            (DensityPreference::Compact, 0.85),
            (DensityPreference::Comfortable, 1.0),
            (DensityPreference::Spacious, 1.15),
        ] {
            assert!((density_scale(density) - scale).abs() < f32::EPSILON);
            assert!((28.0 * density_scale(density) - 28.0 * scale).abs() < 0.001);
            assert!((10.0 * density_scale(density) - 10.0 * scale).abs() < 0.001);
        }
    }

    #[test]
    fn codex_chrome_uses_the_reference_canvas_ink_and_accent() {
        let light = super::super::LIGHT;
        let dark = super::super::DARK;
        assert_eq!(
            (light.canvas, light.sidebar, light.text),
            (0xffffff, 0xffffff, 0x0d0d0d)
        );
        assert_eq!(
            (dark.canvas, dark.sidebar, dark.text),
            (0x111111, 0x111111, 0xfcfcfc)
        );
        assert_eq!((light.focus, dark.focus), (0x0169cc, 0x0169cc));
    }

    #[test]
    fn default_neutral_materials_do_not_reintroduce_a_violet_tint() {
        for palette in [super::super::LIGHT, super::super::DARK] {
            for color in [
                palette.canvas,
                palette.sidebar,
                palette.overlay,
                palette.hover,
                palette.selected,
                palette.border,
                palette.text,
                palette.muted,
            ] {
                assert_eq!((color >> 16) & 255, (color >> 8) & 255);
                assert_eq!((color >> 8) & 255, color & 255);
            }
        }
    }
}
