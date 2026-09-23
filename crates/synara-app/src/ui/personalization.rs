//! Independently authored Synara colorways and native material roles.
use super::{DARK, LIGHT, Palette};
use gpui::{Rgba, rgba};
use synara_workspace::{Colorway, MotionPreference, Personalization, SurfaceMaterial};

thread_local! {
    static STYLE: std::cell::RefCell<Personalization> = std::cell::RefCell::new(Personalization::default());
    static FONT_SIZES: std::cell::Cell<(f32, f32)> = const { std::cell::Cell::new((super::metrics::DEFAULT_UI_FONT_SIZE, super::metrics::DEFAULT_CODE_FONT_SIZE)) };
}
pub(super) fn configure(value: &synara_workspace::AppearanceSettings) {
    STYLE.with(|style| {
        if *style.borrow() != value.personalization {
            *style.borrow_mut() = value.personalization.clone();
        }
    });
    FONT_SIZES.set((value.fonts.ui_size, value.fonts.code_size));
}
pub fn colorway(base: Palette, value: Colorway, dark: bool) -> Palette {
    let colors = match (value, dark) {
        (Colorway::Original, _) => return base,
        (Colorway::Graphite, true) => (0x20242a, 0x1b1f25, 0x2a3038, 0xb7c9dc),
        (Colorway::Midnight, true) => (0x171d30, 0x141928, 0x232d44, 0xb6bcff),
        (Colorway::Ocean, true) => (0x18282f, 0x15232a, 0x243a44, 0x89d7e8),
        (Colorway::Forest, true) => (0x202a26, 0x1a2420, 0x303e35, 0xb0d9ab),
        (Colorway::Ember, true) => (0x2c2322, 0x251d1d, 0x413330, 0xf1bb97),
        (Colorway::Sand, true) => (0x2d2922, 0x25221c, 0x403b31, 0xddc999),
        (Colorway::Graphite, false) => (0xf4f6f8, 0xe9edf1, 0xffffff, 0x466582),
        (Colorway::Midnight, false) => (0xf1f2fa, 0xe5e8f5, 0xffffff, 0x5957a2),
        (Colorway::Ocean, false) => (0xeff7f8, 0xe0eef1, 0xffffff, 0x286e85),
        (Colorway::Forest, false) => (0xf2f6ef, 0xe6ede0, 0xfdfefa, 0x466d43),
        (Colorway::Ember, false) => (0xfaf2ed, 0xf0e5de, 0xfffcf9, 0x985c3b),
        (Colorway::Sand, false) => (0xf7f3e9, 0xeee7d8, 0xfffdf8, 0x85682c),
    };
    let neutral = if dark { DARK } else { LIGHT };
    Palette {
        canvas: colors.0,
        sidebar: colors.1,
        overlay: colors.2,
        focus: colors.3,
        hover: mix(colors.2, neutral.text, 0.05),
        selected: mix(colors.2, colors.3, if dark { 0.15 } else { 0.12 }),
        border: mix(colors.0, neutral.text, if dark { 0.14 } else { 0.18 }),
        ..neutral
    }
}
fn mix(a: u32, b: u32, amount: f32) -> u32 {
    let component = |shift| {
        let a = ((a >> shift) & 255_u32) as f32;
        let b = ((b >> shift) & 255_u32) as f32;
        (a + (b - a) * amount).round() as u32
    };
    (component(16) << 16) | (component(8) << 8) | component(0)
}
fn luminance(value: u32) -> f32 {
    let linear = |shift| {
        let c = ((value >> shift) & 255_u32) as f32 / 255.;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(16) + 0.7152 * linear(8) + 0.0722 * linear(0)
}
/// Custom accents cannot make a focus ring disappear into the canvas.
pub(super) fn readable_accent(requested: u32, canvas: u32) -> u32 {
    let background = luminance(canvas);
    let target = if background < 0.4 { 0xffffff } else { 0 };
    for step in 0..=20 {
        let candidate = mix(requested, target, step as f32 / 20.);
        let foreground = luminance(candidate);
        if (foreground.max(background) + 0.05) / (foreground.min(background) + 0.05) >= 3.0 {
            return candidate;
        }
    }
    target
}
fn alpha(color: u32, percent: u8) -> Rgba {
    rgba((color << 8) | ((u32::from(percent) * 255 + 50) / 100))
}
// Panel opacity is a target TOTAL coverage, not another opaque layer stacked
// on the window. For a 70% canvas and 82% panel, paint only 40% extra tint.
fn canvas_percent(style: &Personalization) -> u8 {
    match (style.material, style.wallpaper.is_some()) {
        (SurfaceMaterial::Solid, false) => 100,
        (SurfaceMaterial::Solid, true) => style.wallpaper_dim,
        (_, true) => style.canvas_opacity.max(style.wallpaper_dim),
        (_, false) => style.canvas_opacity,
    }
}
pub fn canvas_background() -> Rgba {
    STYLE.with(|style| alpha(super::palette().canvas, canvas_percent(&style.borrow())))
}
pub fn surface(color: u32) -> Rgba {
    STYLE.with(|style| {
        let style = style.borrow();
        let base = canvas_percent(&style);
        if base == 100 {
            return alpha(color, 100);
        }
        let target = style.panel_opacity.max(base);
        let extra = u32::from(target - base) * 255 / u32::from(100 - base);
        rgba((color << 8) | extra)
    })
}
pub fn glass_edge() -> Rgba {
    STYLE.with(|style| {
        if style.borrow().material == SurfaceMaterial::Glass {
            alpha(super::palette().text, 14)
        } else {
            alpha(super::palette().border, 100)
        }
    })
}
pub fn chat_width() -> f32 {
    STYLE.with(|style| f32::from(style.borrow().chat_width))
}
pub fn density_metrics() -> super::metrics::DensityMetrics {
    STYLE.with(|style| super::metrics::DensityMetrics::new(style.borrow().density))
}
pub fn row_height() -> f32 {
    density_metrics().row_height
}
pub fn settings_row_padding() -> f32 {
    density_metrics().settings_row_padding_y
}

pub fn motion_multiplier() -> f32 {
    STYLE.with(|style| match style.borrow().motion {
        MotionPreference::Off => 0.,
        MotionPreference::Subtle => 0.6,
        MotionPreference::Standard => 1.,
        MotionPreference::Expressive => 1.4,
    })
}
pub fn ui_font_size() -> f32 {
    FONT_SIZES.get().0
}
pub fn code_font_size() -> f32 {
    FONT_SIZES.get().1
}
pub fn terminal_font_size() -> f32 {
    STYLE.with(|style| f32::from(style.borrow().terminal_font_size))
}
