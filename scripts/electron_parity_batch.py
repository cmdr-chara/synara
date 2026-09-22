#!/usr/bin/env python3
"""Apply the reviewed Electron parity batch to a clean, isolated native checkout.

This task-scoped development helper is not imported by the product. Every edit
has an exact source anchor and an idempotent postcondition. It never fetches or
executes instructions from the Electron reference, and never changes lockfiles,
provider policy, user databases, or application architecture.
"""
from __future__ import annotations

import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
CHANGED: set[str] = set()


def edit(path: str, before: str, after: str, count: int = 1) -> None:
    target = ROOT / path
    original = target.read_text(encoding="utf-8")
    found = original.count(before)
    if found == 0 and original.count(after) >= count:
        return
    if found != count:
        raise RuntimeError(f"{path}: expected {count} exact source anchors, found {found}")
    target.write_text(original.replace(before, after), encoding="utf-8")
    CHANGED.add(path)


def create(path: str, content: str) -> None:
    target = ROOT / path
    if target.exists():
        if target.read_text(encoding="utf-8") != content:
            raise RuntimeError(f"{path}: refusing to replace an existing independent file")
        return
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(content, encoding="utf-8")
    CHANGED.add(path)


def foundation() -> None:
    ui = "crates/synara-app/src/ui.rs"
    edit(ui, "mod personalization;\n", "mod personalization;\nmod reference;\n")
    edit(ui, "    surface, terminal_font_size, ui_font_size,\n", "    density_scale, settings_row_padding, surface, terminal_font_size, ui_font_size,\n")
    edit(ui, "pub const ROW_HEIGHT: f32 = 30.0;", "pub const ROW_HEIGHT: f32 = 28.0;")
    old_dark = """pub const DARK: Palette = Palette {
    canvas: 0x272731,
    sidebar: 0x25252f,
    overlay: 0x30303a,
    hover: 0x2e2e38,
    selected: 0x383843,
    border: 0x34343f,
    text: 0xe8e6e1,
    muted: 0xa19fa9,
    focus: 0x9bb6e8,
    error: 0xffb9c0,
    error_surface: 0x432c35,
    notice_surface: 0x303a4a,
};"""
    new_dark = """// Electron Codex chrome uses neutral surfaces, not the older violet shell.
// Custom colorways, custom accents and the accessibility override are applied
// after these defaults in configure(), preserving saved personalization.
pub const DARK: Palette = Palette {
    canvas: 0x111111,
    sidebar: 0x111111,
    overlay: 0x181818,
    hover: 0x1f1f1f,
    selected: 0x2d2d2d,
    border: 0x2d2d2d,
    text: 0xfcfcfc,
    muted: 0x9e9e9e,
    focus: 0x0169cc,
    error: 0xff8583,
    error_surface: 0x351f1f,
    notice_surface: 0x152334,
};"""
    edit(ui, old_dark, new_dark)
    old_light = """pub const LIGHT: Palette = Palette {
    canvas: 0xfafafa,
    sidebar: 0xf3f3f3,
    overlay: 0xf0f0f0,
    hover: 0xececec,
    selected: 0xe1e1e5,
    border: 0xdddddf,
    text: 0x26262a,
    muted: 0x6d6d76,
    focus: 0x825b9e,
    error: 0x99283b,
    error_surface: 0xffe4e8,
    notice_surface: 0xe6edf7,
};"""
    new_light = """pub const LIGHT: Palette = Palette {
    canvas: 0xffffff,
    sidebar: 0xffffff,
    overlay: 0xf8f8f8,
    hover: 0xf1f1f1,
    selected: 0xe2e2e2,
    border: 0xeeeeee,
    text: 0x0d0d0d,
    muted: 0x6e6e6e,
    focus: 0x0169cc,
    error: 0xe02e2a,
    error_surface: 0xfdeeed,
    notice_surface: 0xf0f5fb,
};"""
    edit(ui, old_light, new_light)
    edit(ui, "        .text_size(px(ui_font_size() + 1.))", "        .text_size(px(ui_font_size()))", 2)
    edit(ui, "        .text_color(rgb(palette().text))\n        .text_sm()\n        .cursor_pointer()", "        .text_color(rgb(palette().text))\n        .text_size(px(ui_font_size()))\n        .cursor_pointer()")
    edit(ui, "        .text_size(px(15.))\n        .text_color(rgb(palette().text))\n        .opacity(0.78)", "        .text_size(px(ui_font_size()))\n        .text_color(rgb(palette().text))\n        .opacity(0.78)")
    personalization = "crates/synara-app/src/ui/personalization.rs"
    edit(personalization, "std::cell::Cell::new((14., 13.))", "std::cell::Cell::new((13., 13.))")
    edit(personalization, """pub fn row_height() -> f32 {
    STYLE.with(|style| match style.borrow().density {
        DensityPreference::Compact => 28.,
        DensityPreference::Comfortable => 30.,
        DensityPreference::Spacious => 36.,
    })
}""", """pub fn density_scale() -> f32 {
    STYLE.with(|style| super::reference::density_scale(style.borrow().density))
}
pub fn row_height() -> f32 {
    super::ROW_HEIGHT * density_scale()
}
pub fn settings_row_padding() -> f32 {
    10.0 * density_scale()
}""")
    # DensityPreference is now consumed by the reference metric owner.
    edit(personalization, "    Colorway, DensityPreference, MotionPreference, Personalization, SurfaceMaterial,", "    Colorway, MotionPreference, Personalization, SurfaceMaterial,")
    edit("crates/synara-workspace/src/settings.rs", "            ui_size: 14.0,", "            ui_size: 13.0,")
    create("crates/synara-app/src/ui/reference.rs", '''//! Metrics shared with the pinned Electron reference.
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
        assert_eq!((light.canvas, light.sidebar, light.text), (0xffffff, 0xffffff, 0x0d0d0d));
        assert_eq!((dark.canvas, dark.sidebar, dark.text), (0x111111, 0x111111, 0xfcfcfc));
        assert_eq!((light.focus, dark.focus), (0x0169cc, 0x0169cc));
    }

    #[test]
    fn default_neutral_materials_do_not_reintroduce_a_violet_tint() {
        for palette in [super::super::LIGHT, super::super::DARK] {
            for color in [palette.canvas, palette.sidebar, palette.overlay, palette.hover,
                          palette.selected, palette.border, palette.text, palette.muted] {
                assert_eq!((color >> 16) & 255, (color >> 8) & 255);
                assert_eq!((color >> 8) & 255, color & 255);
            }
        }
    }
}
''')
    create("crates/synara-workspace/tests/electron_parity_settings.rs", '''use synara_workspace::AppSettings;

#[test]
fn first_run_uses_the_electron_ui_base_size() {
    let settings = AppSettings::default();
    assert_eq!(settings.appearance.fonts.ui_size, 13.0);
    assert_eq!(settings.appearance.fonts.code_size, 13.0);
    settings.validate().unwrap();
}

#[test]
fn existing_explicit_font_sizes_are_not_migrated_or_overwritten() {
    let mut original = AppSettings::default();
    original.appearance.fonts.ui_size = 14.0;
    original.appearance.fonts.code_size = 17.0;
    original.appearance.fonts.ui_family = Some("Operator font".into());
    let decoded: AppSettings = serde_json::from_str(&serde_json::to_string(&original).unwrap()).unwrap();
    assert_eq!(decoded, original);
    decoded.validate().unwrap();
}

#[test]
fn_old_minimal_settings_receive_defaults_without_changing_the_schema_version() {
    let decoded: AppSettings = serde_json::from_str(r#"{"version":1}"#).unwrap();
    assert_eq!(decoded.appearance.fonts.ui_size, 13.0);
    assert_eq!(decoded.version, 1);
    decoded.validate().unwrap();
}
'''.replace('fn_old_minimal', 'fn old_minimal'))


def settings_geometry() -> None:
    path = "crates/synara-app/src/shell/settings.rs"
    edit(path, """fn card() -> gpui::Div {
    div()
        .w_full()
        .rounded_2xl()
        .border_1()""", """fn card() -> gpui::Div {
    div()
        .w_full()
        .rounded_xl()
        .bg(ui::surface(palette().overlay))
        .border_1()""")
    edit(path, """fn heading(label: &'static str) -> gpui::Div {
    div()
        .mt_7()
        .mb_3()
        .px_2()
        .text_color(rgb(palette().muted))
        .child(label)
}""", """fn heading(label: &'static str) -> gpui::Div {
    div()
        .mt_4()
        .mb(px(6.))
        .text_size(px(ui::ui_font_size()))
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(rgb(palette().text))
        .child(label)
}""")
    edit(path, """    div()
        .px_3()
        .py_3()
        .border_b_1()""", """    div()
        .px_3()
        .py(px(ui::settings_row_padding()))
        .text_size(px(ui::ui_font_size()))
        .border_b_1()""")
    edit(path, """                .gap_1()
                .child(title.into())
                .children((!description.is_empty()).then(|| {
                    div()
                        .text_color(rgb(palette().muted))
                        .line_height(px(22.))""", """                .gap(px(2.))
                .child(div().font_weight(gpui::FontWeight::MEDIUM).child(title.into()))
                .children((!description.is_empty()).then(|| {
                    div()
                        .text_color(rgb(palette().muted))
                        .line_height(px(ui::ui_font_size() * 1.625))""")
    edit(path, """            .track_scroll(&self.settings.scroll)
            .mt(px(-30.))
            .text_size(px(15.))""", """            .track_scroll(&self.settings.scroll)
            .text_size(px(ui::ui_font_size()))""")
    edit(path, """            .overflow_y_scroll()
            .px_6()
            .pb_10()""", """            .overflow_y_scroll()
            .px_6()
            .pt_8()
            .pb_10()""")
    edit(path, """                        div()
                            .pt_3()
                            .pb_3()
                            .flex()
                            .items_center()
                            .gap_4()
                            .child(div().text_size(px(20.)).flex_1().child(info.label))""", """                        div()
                            .pb(px(6.))
                            .flex()
                            .items_center()
                            .gap_4()
                            .child(div().text_size(px(20.)).font_weight(gpui::FontWeight::MEDIUM).flex_1().child(info.label))""")
    edit(path, """                            .line_height(px(22.))
                            .mb_3()
                            .child(info.description)""", """                            .line_height(px(ui::ui_font_size() * 1.625))
                            .mb_4()
                            .child(info.description)""")
    edit(path, """                                            .h(px(30.))
                                            .relative()""", """                                            .h(px(ui::row_height()))
                                            .relative()""")


def documentation() -> None:
    create("docs/ui/parity/README.md", '''# Electron-to-GPUI parity

## Pinned product reference

- Reference: `Emanuele-web04/synara`, `main` at `eaa61eded31b6755d4f30ba8eabc5d905cf817cb` (Electron v0.9.1).
- Native baseline: `03fe420bd6379113e59ea64c574dd04254d36a3b` on `astra/gpui-clean-rewrite`.
- Implementation branch: `astra/electron-1to1`.
- Evidence supplied for the task: 157 Electron screenshots and 51 native CI screenshots. Counts describe inputs, not completed features.
- Product and bundled asset notices remain in their existing locations. No screenshot is used as an interactive application surface.

## Completion contract

The requested outcome is visual and behavioral parity, not merely the presence of similar controls. Every feature family remains open until the same state, content, theme, viewport, and interactions have been exercised in both applications.

| Required outcome | State | Deciding check |
| --- | --- | --- |
| Shell, home, navigation, project and thread menus | OPEN | Same-state runtime captures and keyboard/pointer journeys |
| Composer, agent/model/permission controls, active transcript, follow-ups | OPEN | Actual provider-capability-aware interactions plus captures |
| Project creation/import, environment and worktree workflows | OPEN | Persist/reopen and filesystem behavior plus captures |
| Kanban, pull requests, automations | OPEN | Real native mutations, empty/error/populated states and captures |
| Terminal, browser, files, editor, changes and Git | OPEN | Real native tools, lifecycle/recovery and captures |
| All Electron settings sections and controls | OPEN | Defaults, persistence, consumers, reset, navigation and captures |
| Light/dark themes, typography and density | OPEN | Matched captures at 1100x780, 1600x1000 and intermediate widths |
| Onboarding and welcome flow | OPEN | Every step, skip/back/finish, restart and keyboard checks |
| Integration, accessibility and platform checks | OPEN | Native focus/activation, regression suite and platform evidence |

The initial foundation batch changes neutral chrome defaults, the UI base size, density scaling and settings geometry. Existing explicitly saved font sizes, custom colorways, custom accents, high-contrast settings, provider choices and permission policies are preserved. Compilation and unit tests do not close a visual or feature gate.

The existing feature-gap inventory remains authoritative for unimplemented capabilities. Computer control, external client integrations, and authenticated provider/platform behavior must not be marked complete using placeholders, fixture agents or screenshots alone.
''')


def main() -> None:
    status = subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT, text=True)
    if status.strip():
        raise RuntimeError("Refusing to apply a parity batch to a dirty working tree")
    foundation()
    settings_geometry()
    documentation()
    print(json.dumps({"changed": sorted(CHANGED), "reference": "eaa61eded31b6755d4f30ba8eabc5d905cf817cb"}, indent=2))
    subprocess.run(["git", "diff", "--check"], cwd=ROOT, check=True)


if __name__ == "__main__":
    main()
