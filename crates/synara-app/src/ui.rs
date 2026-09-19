//! Native presentation primitives. Product state and operations stay in the controller.
pub mod menu;
use gpui::{
    Context, Div, ElementId, FontWeight, PathBuilder, SharedString, Stateful, Window, canvas, div,
    point, prelude::*, px, rgb, rgba,
};

pub const CHAT_WIDTH: f32 = 768.0;
pub const COMPOSER_INPUT_HEIGHT: f32 = 82.0;
pub const MENU_WIDTH: f32 = 304.0;
pub const MENU_ROW_HEIGHT: f32 = 42.0;
pub const MENU_MAX_HEIGHT: f32 = 294.0;
pub const SIDEBAR_WIDTH: f32 = 238.0;
pub const ROW_HEIGHT: f32 = 30.0;
pub const CHROME_HEIGHT: f32 = 53.0;
pub const STATUS_HEIGHT: f32 = 27.0;
pub const UI_FONT: &str = if cfg!(target_os = "windows") {
    "Segoe UI"
} else if cfg!(target_os = "macos") {
    "Helvetica Neue"
} else {
    "DejaVu Sans"
};

/// Semantic material roles, not per-screen RGB literals.
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
    hover: 0x33333e,
    selected: 0x3b3b47,
    border: 0x3a3a44,
    text: 0xe5e4e8,
    muted: 0xa19fa9,
    focus: 0x9bb6e8,
    error: 0xffb9c0,
    error_surface: 0x432c35,
    notice_surface: 0x303a4a,
};

/// GPUI maps unmodified Enter/Space press-release and accessibility clicks to
/// on_click. Do not add a second keydown/accessibility callback: that bypasses
/// release/blur cancellation and can dispatch a backend operation twice.
pub fn button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    selected: bool,
) -> Stateful<Div> {
    let label = label.into();
    let hint = label.clone();
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
            DARK.selected
        } else {
            DARK.overlay
        }))
        .text_color(rgb(DARK.text))
        .text_sm()
        .cursor_pointer()
        .hover(|style| style.bg(rgb(DARK.hover)))
        .active(|style| style.bg(rgb(DARK.selected)))
        .focus(|style| style.border_color(rgb(DARK.focus)))
        .tooltip(move |_, cx| cx.new(|_| Tooltip(hint.clone())).into())
        .child(label)
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
    let hint = label.clone();
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
        .gap_2()
        .rounded_md()
        .border_1()
        .border_color(rgba(0x00000000))
        .bg(rgba(if selected {
            (DARK.selected << 8) | 0xff
        } else {
            0
        }))
        .text_color(rgb(DARK.text))
        .text_size(px(13.0))
        .cursor_pointer()
        .hover(|style| style.bg(rgb(DARK.hover)))
        .active(|style| style.bg(rgb(DARK.selected)))
        .focus(|style| style.border_color(rgb(DARK.focus)))
        .on_click(move |_, window, cx| {
            activate(&(), window, cx);
            cx.stop_propagation();
        })
        .tooltip(move |_, cx| cx.new(|_| Tooltip(hint.clone())).into())
        .children(glyph.map(icon))
        .child(div().flex_1().min_w_0().text_ellipsis().child(label))
}

struct Tooltip(SharedString);
impl Render for Tooltip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .max_w(px(400.0))
            .px_2()
            .py_1()
            .rounded_md()
            .border_1()
            .border_color(rgb(DARK.border))
            .bg(rgb(DARK.overlay))
            .font_family(UI_FONT)
            .text_size(px(12.0))
            .text_color(rgb(DARK.text))
            .child(self.0.clone())
    }
}

pub fn section_label(label: &'static str) -> Div {
    div()
        .px_2()
        .py_1()
        .text_size(px(12.0))
        .font_weight(FontWeight::MEDIUM)
        .text_color(rgb(DARK.muted))
        .child(label)
}

/// Independently authored simple line geometry. No screenshot or legacy assets.
#[derive(Clone, Copy)]
pub enum Glyph {
    Folder,
    Compose,
    Chevron,
    ChevronRight,
    Settings,
    More,
    Panel,
    Terminal,
    Files,
    Changes,
    Send,
    Stop,
    Copy,
}
impl Glyph {
    fn paths(self) -> &'static [&'static [(f32, f32)]] {
        match self {
            Self::Copy => &[
                &[(8., 8.), (21., 8.), (21., 21.), (8., 21.), (8., 8.)],
                &[(4., 16.), (3., 16.), (3., 3.), (16., 3.), (16., 4.)],
            ],
            Self::Send => &[
                &[(12., 20.), (12., 4.)],
                &[(5., 11.), (12., 4.), (19., 11.)],
            ],
            Self::Stop => &[&[(5., 5.), (19., 5.), (19., 19.), (5., 19.), (5., 5.)]],
            Self::Folder => &[&[
                (2., 5.),
                (8., 5.),
                (10., 8.),
                (22., 8.),
                (22., 20.),
                (2., 20.),
                (2., 5.),
            ]],
            Self::Compose => &[
                &[(9., 4.), (4., 4.), (4., 20.), (20., 20.), (20., 15.)],
                &[
                    (10., 15.),
                    (11., 10.),
                    (19., 2.),
                    (22., 5.),
                    (14., 13.),
                    (10., 15.),
                ],
            ],
            Self::Chevron => &[&[(6., 9.), (12., 15.), (18., 9.)]],
            Self::ChevronRight => &[&[(9., 6.), (15., 12.), (9., 18.)]],
            Self::Settings => &[
                &[
                    (8., 3.),
                    (16., 3.),
                    (21., 8.),
                    (21., 16.),
                    (16., 21.),
                    (8., 21.),
                    (3., 16.),
                    (3., 8.),
                    (8., 3.),
                ],
                &[(9., 9.), (15., 9.), (15., 15.), (9., 15.), (9., 9.)],
            ],
            Self::More => &[
                &[(4., 11.), (4., 13.)],
                &[(12., 11.), (12., 13.)],
                &[(20., 11.), (20., 13.)],
            ],
            Self::Panel => &[
                &[(3., 4.), (21., 4.), (21., 20.), (3., 20.), (3., 4.)],
                &[(9., 4.), (9., 20.)],
            ],
            Self::Terminal => &[
                &[(4., 6.), (10., 12.), (4., 18.)],
                &[(13., 18.), (21., 18.)],
            ],
            Self::Files => &[
                &[
                    (6., 2.),
                    (15., 2.),
                    (21., 8.),
                    (21., 20.),
                    (6., 20.),
                    (6., 2.),
                ],
                &[(15., 2.), (15., 8.), (21., 8.)],
                &[(2., 6.), (2., 23.), (17., 23.)],
            ],
            Self::Changes => &[
                &[(6., 3.), (6., 21.)],
                &[(18., 3.), (18., 21.)],
                &[(3., 6.), (9., 6.)],
                &[(15., 16.), (21., 16.)],
            ],
        }
    }
}

pub fn icon(glyph: Glyph) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let mut path = PathBuilder::stroke(px(1.3));
            for points in glyph.paths() {
                for (index, &(x, y)) in points.iter().enumerate() {
                    let position = bounds.origin + point(px(x * 14.0 / 24.0), px(y * 14.0 / 24.0));
                    if index == 0 {
                        path.move_to(position);
                    } else {
                        path.line_to(position);
                    }
                }
            }
            if let Ok(path) = path.build() {
                window.paint_path(path, rgb(DARK.muted));
            }
        },
    )
    .size(px(14.0))
    .flex_shrink_0()
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
        .bg(rgb(DARK.selected))
        .cursor_pointer()
        .hover(|style| style.bg(rgb(DARK.hover)))
        .focus(|style| style.border_color(rgb(DARK.focus)))
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
