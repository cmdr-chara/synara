//! Native presentation primitives. Product state and operations stay in the controller.
mod icons;
pub mod markdown;
pub use icons::{Glyph, icon, provider_glyph};
pub mod menu;
pub mod motion;
pub mod task_dialog;
use gpui::{
    Context, Div, ElementId, SharedString, Stateful, Window, canvas, div, prelude::*, px, rgb, rgba,
};

pub const CHAT_WIDTH: f32 = 736.0;
pub const COMPOSER_INPUT_HEIGHT: f32 = 51.0;
pub const MENU_WIDTH: f32 = 304.0;
pub const MENU_ROW_HEIGHT: f32 = 42.0;
pub const MENU_MAX_HEIGHT: f32 = 294.0;
pub const SIDEBAR_WIDTH: f32 = 256.0;
pub const ROW_HEIGHT: f32 = 30.0;
pub const CHROME_HEIGHT: f32 = 46.0;
pub const UI_FONT: &str = if cfg!(target_os = "windows") {
    "Segoe UI"
} else if cfg!(target_os = "macos") {
    "Helvetica Neue"
} else {
    "Liberation Sans"
};

/// Semantic material roles, not per-screen RGB literals.
#[derive(Clone, Copy)]
pub struct Palette {
    pub canvas: u32,
    pub sidebar: u32,
    pub overlay: u32,
    pub hover: u32,
    pub selected: u32,
    pub border: u32,
    pub text: u32,
    pub muted: u32,
    pub focus: u32,
    pub error: u32,
    pub error_surface: u32,
    pub notice_surface: u32,
}
pub const DARK: Palette = Palette {
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
};

pub const LIGHT: Palette = Palette {
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
};
// Synara currently owns one application window. All native views, including
// menus and text entries, paint on the UI thread and share its current palette.
thread_local! {
    static PALETTE: std::cell::Cell<Palette> = const { std::cell::Cell::new(DARK) };
    static UI_FAMILY: std::cell::RefCell<SharedString> = std::cell::RefCell::new(UI_FONT.into());
    static CODE_FAMILY: std::cell::RefCell<SharedString> = std::cell::RefCell::new("DejaVu Sans Mono".into());
}
pub fn palette() -> Palette {
    PALETTE.with(std::cell::Cell::get)
}
pub fn ui_font() -> SharedString {
    UI_FAMILY.with(|family| family.borrow().clone())
}
pub fn code_font() -> SharedString {
    CODE_FAMILY.with(|family| family.borrow().clone())
}
pub fn configure(
    appearance: &synara_workspace::AppearanceSettings,
    system: gpui::WindowAppearance,
) {
    use synara_workspace::{DarkThemePreference, ThemePreference};
    let dark = match appearance.theme {
        ThemePreference::System => matches!(
            system,
            gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
        ),
        ThemePreference::Light => false,
        ThemePreference::Dark => true,
    };
    let palette = if !dark {
        LIGHT
    } else if appearance.dark_theme == DarkThemePreference::Dracula {
        Palette {
            canvas: 0x282a36,
            sidebar: 0x252731,
            overlay: 0x30323f,
            hover: 0x30323c,
            selected: 0x393b49,
            border: 0x393b46,
            text: 0xf8f8f2,
            muted: 0xa4a3ae,
            focus: 0xff79c6,
            ..DARK
        }
    } else {
        DARK
    };
    PALETTE.set(palette);
    UI_FAMILY.with(|family| {
        *family.borrow_mut() = appearance
            .fonts
            .ui_family
            .clone()
            .map(SharedString::from)
            .unwrap_or_else(|| UI_FONT.into())
    });
    CODE_FAMILY.with(|family| {
        *family.borrow_mut() = appearance
            .fonts
            .code_family
            .clone()
            .map(SharedString::from)
            .unwrap_or_else(|| "DejaVu Sans Mono".into())
    });
}

/// GPUI maps unmodified Enter/Space press-release and accessibility clicks to
/// on_click. Do not add a second keydown/accessibility callback: that bypasses
/// release/blur cancellation and can dispatch a backend operation twice.
pub fn button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    selected: bool,
) -> Stateful<Div> {
    let label = label.into();
    button_shell(id, label.clone(), selected).child(label)
}

pub fn button_shell(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    selected: bool,
) -> Stateful<Div> {
    let label = label.into();
    div()
        .id(id)
        .role(gpui::Role::Button)
        .aria_label(label.clone())
        .tab_index(0)
        .px_3()
        .py_1()
        .rounded_md()
        .border_1()
        .border_color(rgba(0x00000000))
        .bg(rgb(if selected {
            palette().selected
        } else {
            palette().overlay
        }))
        .text_color(rgb(palette().text))
        .text_sm()
        .cursor_pointer()
        .hover(|style| style.bg(rgb(palette().hover)))
        .active(|style| style.bg(rgb(palette().selected)))
        .focus_visible(|style| style.border_color(rgb(palette().focus)))
}

/// Mouse, native keyboard activation and assistive activation use one callback.
pub fn action(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    glyph: Option<Glyph>,
    selected: bool,
    activate: impl Fn(&(), &mut Window, &mut gpui::App) + 'static,
) -> Stateful<Div> {
    let label = label.into();
    div()
        .id(id)
        .role(gpui::Role::Button)
        .aria_label(label.clone())
        .tab_index(0)
        .h(px(ROW_HEIGHT))
        .min_w_0()
        .px_2()
        .flex()
        .items_center()
        .gap(px(6.))
        .rounded_md()
        .border_1()
        .border_color(rgba(0x00000000))
        .bg(rgba(if selected {
            (palette().selected << 8) | 0xff
        } else {
            0
        }))
        .text_color(rgb(palette().text))
        .text_size(px(15.0))
        .cursor_pointer()
        .hover(|style| style.bg(rgb(palette().hover)))
        .active(|style| style.bg(rgb(palette().selected)))
        .focus_visible(|style| style.border_color(rgb(palette().focus)))
        .on_click(move |_, window, cx| {
            activate(&(), window, cx);
            cx.stop_propagation();
        })
        .children(glyph.map(icon))
        .child(div().flex_1().min_w_0().text_ellipsis().child(label))
}

pub(crate) struct Tooltip(pub(crate) SharedString);

/// Preserve the navigation landmark while accurately exposing unavailable capabilities.
pub fn unavailable_action(
    id: &'static str,
    label: &'static str,
    glyph: Glyph,
    reason: &'static str,
) -> Stateful<Div> {
    div()
        .id(id)
        .role(gpui::Role::Label)
        .aria_label(format!("{label}, unavailable"))
        .aria_description(reason)
        .tab_index(0)
        .h(px(ROW_HEIGHT))
        .px_2()
        .flex()
        .items_center()
        .gap(px(6.))
        .text_size(px(15.))
        .text_color(rgb(palette().text))
        .opacity(0.78)
        .rounded_md()
        .border_1()
        .border_color(rgba(0))
        .focus_visible(|style| style.border_color(rgb(palette().focus)))
        .cursor_default()
        .tooltip(move |_, cx| cx.new(|_| Tooltip(reason.into())).into())
        .child(icon(glyph))
        .child(label)
}
impl Render for Tooltip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .max_w(px(400.0))
            .px_2()
            .py_1()
            .rounded_md()
            .border_1()
            .border_color(rgb(palette().border))
            .bg(rgb(palette().overlay))
            .font_family(ui_font())
            .text_size(px(12.0))
            .text_color(rgb(palette().text))
            .child(self.0.clone())
    }
}

/// A named compact action, with native press/release behavior and guarded disablement.
pub fn icon_button(
    id: &'static str,
    label: &'static str,
    glyph: Glyph,
    disabled: bool,
    activate: impl Fn(&(), &mut Window, &mut gpui::App) + 'static,
) -> Stateful<Div> {
    div()
        .id(id)
        .role(gpui::Role::Button)
        .aria_label(label)
        .when(disabled, |el| el.aria_description("Currently unavailable"))
        .tab_index(0)
        .size(px(30.))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .border_1()
        .border_color(rgba(0))
        .bg(rgb(palette().selected))
        .cursor_pointer()
        .hover(|style| style.bg(rgb(palette().hover)))
        .focus_visible(|style| style.border_color(rgb(palette().focus)))
        .when(disabled, |el| el.opacity(0.4).cursor_default())
        .tooltip(move |_, cx| cx.new(|_| Tooltip(label.into())).into())
        .on_click(move |_, window, cx| {
            if !disabled {
                activate(&(), window, cx);
            }
        })
        .relative()
        .child(icon(glyph))
}

/// Opt-in geometry diagnostics only. Never logs labels, paths, prompts or request IDs.
pub fn layout_probe(id: &'static str) -> impl IntoElement {
    canvas(move |bounds, _, _| {
        tracing::debug!(target: "synara_ui_layout", control = id,
            x = f32::from(bounds.origin.x), y = f32::from(bounds.origin.y),
            width = f32::from(bounds.size.width), height = f32::from(bounds.size.height), "control-layout");
    }, |_, _, _, _| {}).absolute().top_0().left_0().size_full()
}

pub fn layout_probe_slot(id: &'static str, slot: usize) -> impl IntoElement {
    canvas(move |bounds, _, _| {
        tracing::debug!(target: "synara_ui_layout", control = id, slot,
            x = f32::from(bounds.origin.x), y = f32::from(bounds.origin.y),
            width = f32::from(bounds.size.width), height = f32::from(bounds.size.height), "control-layout");
    }, |_, _, _, _| {}).absolute().top_0().left_0().size_full()
}

/// Quiet chrome action; its accessible name remains available without a text label.
pub fn chrome_button(
    id: &'static str,
    label: &'static str,
    glyph: Glyph,
    disabled: bool,
    activate: impl Fn(&(), &mut Window, &mut gpui::App) + 'static,
) -> Stateful<Div> {
    icon_button(id, label, glyph, disabled, activate)
        .rounded_md()
        .bg(rgba(0))
        .child(layout_probe(id))
}

pub struct Assets;
impl gpui::AssetSource for Assets {
    fn load(&self, path: &str) -> anyhow::Result<Option<std::borrow::Cow<'static, [u8]>>> {
        Ok(match path {
            "brand/synara.svg" => Some(std::borrow::Cow::Borrowed(include_bytes!(
                "../assets/synara.svg"
            ))),
            _ => icons::load(path),
        })
    }

    fn list(&self, path: &str) -> anyhow::Result<Vec<SharedString>> {
        Ok(if "brand/synara.svg".starts_with(path) {
            vec!["brand/synara.svg".into()]
        } else {
            vec![]
        })
    }
}
