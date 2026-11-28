//! Overlay surfaces extracted from native/mod.rs (wave 2 of the split):
//! context menus, hover cards, palette, model picker, dialogs and popovers.
#![allow(unused_imports)]

use super::*;
use gpui::prelude::FluentBuilder as _;
use gpui_kit::assets::IconName as AssetIcon;
use gpui_kit::base::Disableable as _;
use gpui_kit::component::WindowExt as _;
use gpui_kit::component::{
    button::{Button, ButtonCustomVariant, ButtonRounded, ButtonVariant, ButtonVariants as _},
    h_flex,
    input::{Editor, EditorState, Input, InputEvent, InputState},
    menu::{DropdownMenu as _, PopupMenuItem},
    scroll::ScrollableElement as _,
    slider::{Slider, SliderEvent, SliderState},
    spinner::Spinner,
    text::TextView,
    theme::Theme,
    v_flex, ActiveTheme as _, Icon, IconName, Sizable as _,
};
use gpui_kit::*;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

pub(super) struct NativePanes {
    pub(super) settings_pane: gpui::AnyElement,
    pub(super) tasks_pane: gpui::AnyElement,
    pub(super) kanban_pane: gpui::AnyElement,
    pub(super) pullreq_pane: gpui::AnyElement,
    pub(super) autom_pane: gpui::AnyElement,
    pub(super) studio_pane: gpui::AnyElement,
    pub(super) term_surface: gpui::AnyElement,
    pub(super) settings_nav_el: gpui::AnyElement,
    pub(super) pullreq_panel: gpui::AnyElement,
    pub(super) autom_panel: gpui::AnyElement,
    pub(super) studio_panel: gpui::AnyElement,
    pub(super) kanban_header: gpui::AnyElement,
    pub(super) kanban_count: usize,
}

impl Native {
    pub(super) fn build_panes(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> NativePanes {
        let theme = cx.theme().clone();
        let dark_ui = match self.appearance_mode {
            2 => true,
            1 => false,
            _ => matches!(
                cx.window_appearance(),
                gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
            ),
        };
        let _well_bg = if dark_ui {
            rgb(0x1d1d1d)
        } else {
            rgb(0xf7f7f7)
        };
        let _hairline = if dark_ui {
            rgb(0x2e2e2e)
        } else {
            rgb(0xe5e5e5)
        };
        let subtle = if dark_ui {
            rgba(0xffffff14)
        } else {
            rgba(0x0000000d)
        };
        let chip_bg = if dark_ui {
            rgba(0xffffff12)
        } else {
            rgba(0x00000008)
        };
        let soft_bg: gpui::Hsla = if dark_ui {
            rgba(0xffffff0a).into()
        } else {
            rgb(0xf7f7f7).into()
        };
        let sel_fill: gpui::Hsla = if dark_ui {
            rgba(0xffffff0a).into()
        } else {
            theme.list_active
        };
        let content_bg: gpui::Hsla = if dark_ui {
            rgb(0x111111).into()
        } else {
            theme.background
        };
        let _sidebar_bg: gpui::Hsla = if dark_ui {
            rgb(0x111111).into()
        } else {
            theme.background
        };
        let popover_bg: gpui::Hsla = if dark_ui {
            rgb(0x161616).into()
        } else {
            theme.popover
        };

        // ---- settings pane ----
        // ---- settings pane (upstream /settings layout) ----
        // Nav: (group label, [(section id, label, icon, badge)])
        let settings_nav: &[(&str, &[(&str, &str, &[u8], Option<&str>)])] = &[
            (
                "",
                &[
                    ("profile", "Profile", icons::USER, None),
                    ("accounts", "Accounts", icons::S_PEOPLE_ID_CARD, None),
                ],
            ),
            (
                "Synara",
                &[
                    ("general", "General", icons::S_GEAR, None),
                    ("appearance", "Appearance", icons::S_COLOR_PALETTE, None),
                    ("layout", "Layout", icons::S_LAYOUT_SIDEBAR, None),
                    ("chats", "Chats", icons::S_CHAT_BUBBLES, None),
                    ("notifications", "Notifications", icons::S_BELL, None),
                    ("shortcuts", "Keyboard shortcuts", icons::S_SHORTCUT, None),
                ],
            ),
            (
                "Agents",
                &[
                    ("providers", "Providers", icons::S_PUZZLE, None),
                    ("models", "Models", icons::S_BRAIN, None),
                    ("skills", "Skills", icons::S_BUILDING_BLOCKS, None),
                    ("usage", "Usage", icons::S_GAUGE, None),
                ],
            ),
            (
                "Connections",
                &[
                    ("connectors", "Connectors", icons::S_CONNECTORS, None),
                    ("integrations", "External agents", icons::S_PLUGIN, None),
                    ("devices", "Devices", icons::S_GLOBE, None),
                    (
                        "computer",
                        "Computer use",
                        icons::S_COMPUTER_USE,
                        Some("Beta"),
                    ),
                    ("appsnap", "AppSnap", icons::S_SCREEN_CAPTURE, None),
                ],
            ),
            (
                "Workspace",
                &[
                    ("worktrees", "Worktrees", icons::S_BRANCH_SIMPLE, None),
                    ("archive", "Archive", icons::S_ARCHIVE, None),
                ],
            ),
            (
                "System",
                &[
                    ("saved-logins", "Saved logins", icons::SHIELD_ACCESS, None),
                    ("beta", "Beta channel", icons::S_GAUGE, Some("Beta")),
                    ("privacy", "Privacy", icons::S_SHIELD, None),
                    ("advanced", "Advanced", icons::S_TOOLBOX, None),
                    ("about", "About", icons::S_CIRCLE_INFO, None),
                ],
            ),
        ];
        let section_title: &str = settings_nav
            .iter()
            .flat_map(|(_, items)| items.iter())
            .find(|(id, ..)| *id == self.settings_section)
            .map(|(_, l, ..)| *l)
            .unwrap_or("General");

        // Settings card: rounded-xl, faint fill (foreground/2%), hairline border.
        // Upstream settings card: rounded-xl (14px), border, bg-foreground/2.
        let card = |rows: gpui::AnyElement| {
            div()
                .rounded(px(14.))
                .border_1()
                .border_color(theme.border)
                .bg(popover_bg)
                .child(rows)
        };
        // One row: title + description left, control right.
        let dscale = self.density_scale();
        let srow = |title: &'static str, desc: &'static str, ctrl: gpui::AnyElement| {
            h_flex()
                .w_full()
                .items_center()
                .justify_between()
                .gap_3()
                .px_3()
                .py(px(10. * dscale))
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .gap(px(2.))
                        // Upstream SettingsRow: 13px / 19.5px title (medium) and
                        // description, 2px apart.
                        .child(
                            div()
                                .text_size(px(13.))
                                .line_height(px(19.5))
                                .font_weight(FontWeight::MEDIUM)
                                .child(title),
                        )
                        .child(
                            div()
                                .w_full()
                                .text_size(px(13.))
                                .line_height(px(19.5))
                                .text_color(theme.muted_foreground)
                                .child(desc),
                        ),
                )
                .child(ctrl)
        };
        // SettingsRow with upstream `status` line (pt-1 text-ui-sm muted) below the description.
        let srow_st = |title: &'static str,
                       desc: &'static str,
                       status: &'static str,
                       ctrl: gpui::AnyElement| {
            h_flex()
                .w_full()
                .items_center()
                .justify_between()
                .gap_3()
                .px_3()
                .py(px(10. * dscale))
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .gap(px(2.))
                        .child(
                            div()
                                .text_size(px(13.))
                                .line_height(px(19.5))
                                .font_weight(FontWeight::MEDIUM)
                                .child(title),
                        )
                        .child(
                            div()
                                .w_full()
                                .text_size(px(13.))
                                .line_height(px(19.5))
                                .text_color(theme.muted_foreground)
                                .child(desc),
                        )
                        .child(
                            div()
                                .pt(px(4.))
                                .text_size(px(12.))
                                .text_color(theme.muted_foreground)
                                .child(status),
                        ),
                )
                .child(ctrl)
        };
        let divider = || div().h(px(1.)).w_full().bg(theme.border);
        // Select-style control pill (static parity render).
        let select_pill = |label: String, icon: Option<&'static [u8]>| {
            let mut b = h_flex()
                .h(px(28.))
                .px(px(10.))
                .gap_1p5()
                .items_center()
                .rounded(px(8.))
                .border_1()
                .border_color(theme.border);
            if let Some(ic) = icon {
                b = b.child(
                    svg_icon(ic)
                        .size(px(14.))
                        .text_color(theme.muted_foreground),
                );
            }
            b.child(div().text_size(px(13.)).whitespace_nowrap().child(label))
                .child(
                    svg_icon(icons::CHEVRON_DOWN)
                        .size(px(12.))
                        .text_color(theme.muted_foreground),
                )
                .into_any_element()
        };
        // Upstream Button size="xs" variant="outline": h-6, px 7px, text-ui-xs,
        // rounded-sm, medium weight.
        let outline_btn = |label: &'static str| {
            div()
                .flex_shrink_0()
                .h(px(24.))
                .px(px(7.))
                .flex()
                .items_center()
                .rounded(px(4.))
                .border_1()
                .border_color(theme.border)
                .child(
                    div()
                        .text_size(px(11.))
                        .font_weight(FontWeight::MEDIUM)
                        .whitespace_nowrap()
                        .child(label),
                )
                .into_any_element()
        };
        let section_label = |t: &'static str| {
            div()
                .px_2()
                .py_1()
                .line_height(px(19.5))
                .text_size(px(13.))
                .text_color(theme.muted_foreground)
                .child(t)
        };

        // Upstream settings group: section label sits 6px above its card
        // (SETTINGS_PANEL_SECTION gap-1.5); groups separate by the pane gap (16px).
        let lgroup = |label: &'static str, content: gpui::Div| {
            v_flex()
                .w_full()
                .gap(px(6.))
                .child(section_label(label))
                .child(content)
                .into_any_element()
        };

        // ---- shared controls for ported settings sections ----
        let accent_c = text_accent(matches!(theme.mode, gpui_kit::component::ThemeMode::Dark));
        let switch_theme = theme.clone();
        let toggle = move |on: bool| switch_track(on, accent_c, &switch_theme).into_any_element();
        // Upstream segmented = ghost buttons gap-1; active gets a gray "secondary" fill.
        let seg = |opts: &[&'static str], active: usize| {
            let mut s = h_flex().items_center().gap(px(4.));
            for (i, o) in opts.iter().enumerate() {
                let mut c = div()
                    .h(px(32.))
                    .px(px(11.))
                    .flex()
                    .items_center()
                    .rounded(px(8.));
                if i == active {
                    c = c.bg(theme.secondary);
                }
                let tc = if i == active {
                    theme.foreground
                } else {
                    theme.muted_foreground
                };
                s = s.child(
                    c.child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(tc)
                            .child(*o),
                    ),
                );
            }
            s.into_any_element()
        };
        // Color swatch pill: bordered pill with color box + hex label.
        // Color-filled pill with the hex label inside, like upstream's theme swatch.
        let swatch_pill = |hex_label: SharedString, color: u32| {
            let r = ((color >> 16) & 0xff) as f32;
            let g = ((color >> 8) & 0xff) as f32;
            let b = (color & 0xff) as f32;
            let lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
            let ink = if lum > 140. {
                rgba(0x000000aa)
            } else {
                rgba(0xffffffcc)
            };
            div()
                .w(px(160.))
                .h(px(26.))
                .px_3()
                .flex()
                .items_center()
                .rounded(px(8.))
                .border_1()
                .border_color(theme.border)
                .bg(rgb(color))
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(ink)
                        .child(hex_label.to_string()),
                )
                .into_any_element()
        };
        // Bordered number field + "px" suffix.
        let num_field = |v: &'static str| {
            h_flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .h(px(28.))
                        .w(px(44.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(8.))
                        .border_1()
                        .border_color(theme.border)
                        .child(div().text_size(px(13.)).child(v)),
                )
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(theme.muted_foreground)
                        .child("px"),
                )
                .into_any_element()
        };
        // Miniature app-window mockup for the Appearance theme picker, matching
        // upstream ThemeModePicker (MOCKUP_COLORS + MOCKUP_LAYOUT) pixel for pixel.
        // Thumb area is 190x133 (aspect 10/7 inside border-2 + p-3px of a 200px card).
        struct MockColors {
            backdrop: u32,
            panel: u32,
            hbar: u32,
            hbar_soft: u32,
            card: u32,
            rowbar: u32,
            hairline: u32,
        }
        const MOCK_LIGHT: MockColors = MockColors {
            backdrop: 0xe9e9e9,
            panel: 0xf6f6f6,
            hbar: 0xcfcfcf,
            hbar_soft: 0xe0e0e0,
            card: 0xffffff,
            rowbar: 0xe3e3e3,
            hairline: 0xefefef,
        };
        const MOCK_DARK: MockColors = MockColors {
            backdrop: 0x5f5f5f,
            panel: 0x2c2c2c,
            hbar: 0xa6a6a6,
            hbar_soft: 0x7d7d7d,
            card: 0x3a3a3a,
            rowbar: 0x707070,
            hairline: 0x4d4d4d,
        };
        // Upstream MOCKUP_LAYOUT resolves percentages against the containing
        // block's width; at a 190x133 thumb the panel is 160 wide and the card
        // 132, giving the absolute values used below.
        let mock_surface = |c: &MockColors| {
            div().size_full().bg(rgb(c.backdrop)).child(
                div()
                    .absolute()
                    .left(px(15.))
                    .right(px(15.))
                    .top(px(22.))
                    .bottom_0()
                    .bg(rgb(c.panel))
                    .rounded_t(px(8.))
                    .child(
                        div()
                            .absolute()
                            .left(px(50.))
                            .top(px(14.))
                            .w(px(61.))
                            .h(px(4.))
                            .rounded_full()
                            .bg(rgb(c.hbar)),
                    )
                    .child(
                        div()
                            .absolute()
                            .left(px(36.))
                            .top(px(22.))
                            .w(px(88.))
                            .h(px(3.))
                            .rounded_full()
                            .bg(rgb(c.hbar_soft)),
                    )
                    .child(
                        div()
                            .absolute()
                            .left(px(14.))
                            .right(px(14.))
                            .top(px(37.))
                            .bottom_0()
                            .bg(rgb(c.card))
                            .rounded_t(px(6.))
                            .child(rows_card(c)),
                    ),
            )
        };
        fn rows_card(c: &MockColors) -> gpui::Div {
            let mut card = div().size_full();
            for i in 0..3 {
                let y = 9. + (i as f32) * 21.5;
                card = card
                    .child(
                        div()
                            .absolute()
                            .left(px(13.))
                            .top(px(y))
                            .w(px(47.))
                            .h(px(4.))
                            .rounded_full()
                            .bg(rgb(c.rowbar)),
                    )
                    .child(
                        div()
                            .absolute()
                            .left(px(13.))
                            .right(px(13.))
                            .top(px(y + 12.3))
                            .h(px(1.))
                            .bg(rgb(c.hairline)),
                    );
            }
            card
        }
        let theme_fg = theme.foreground;
        let theme_card = move |kind: u8, label: &'static str, selected: bool| {
            let thumb: gpui::AnyElement = match kind {
                0 => div()
                    .size_full()
                    .child(mock_surface(&MOCK_LIGHT))
                    .child(
                        div()
                            .absolute()
                            .right_0()
                            .top_0()
                            .bottom_0()
                            .w(px(95.))
                            .overflow_hidden()
                            .child(
                                div()
                                    .absolute()
                                    .right_0()
                                    .top_0()
                                    .bottom_0()
                                    .w(px(190.))
                                    .child(mock_surface(&MOCK_DARK)),
                            ),
                    )
                    .into_any_element(),
                1 => div()
                    .size_full()
                    .child(mock_surface(&MOCK_LIGHT))
                    .into_any_element(),
                _ => div()
                    .size_full()
                    .child(mock_surface(&MOCK_DARK))
                    .into_any_element(),
            };
            let frame = div()
                .w_full()
                .p(px(3.))
                .rounded(px(14.))
                .border_2()
                .border_color(if selected {
                    theme_fg
                } else {
                    gpui::transparent_black()
                })
                .child(
                    div()
                        .w(px(190.))
                        .h(px(133.))
                        .rounded(px(8.))
                        .overflow_hidden()
                        .relative()
                        .child(thumb),
                );
            v_flex()
                .flex_1()
                .min_w_0()
                .gap(px(6.))
                .items_center()
                .child(frame)
                .child(
                    div()
                        .text_size(px(13.))
                        .when(selected, |d| d.font_weight(FontWeight::MEDIUM))
                        .child(label),
                )
                .into_any_element()
        };

        let settings_h = window.viewport_size().height - px(47.);
        // Upstream: connectors+usage -> max-w-4xl (896), profile -> max-w-3xl
        // (768), everything else -> max-w-2xl (672), mx-auto centered.
        let settings_w = px(match self.settings_section {
            "connectors" | "usage" => 896.,
            "profile" => 768.,
            _ => 672.,
        });
        let mut settings_pane = v_flex()
            .w(settings_w)
            .h(settings_h)
            .id("settings-scroll")
            .track_scroll(&self.settings_scroll)
            .overflow_y_scroll()
            .vertical_scrollbar(&self.settings_scroll)
            .px_6()
            .pt(px(32.))
            .pb_8()
            .gap(px(16.));
        {
            // Upstream header row carries mb-8 (32px); the pane's uniform gap_5
            // covers 20 of it, so the head itself contributes the remaining 12.
            let mut head = h_flex()
                .w_full()
                .items_start()
                .justify_between()
                .gap_4()
                .pb(px(7.));
            if self.settings_section == "profile" {
                head = h_flex().items_start().w_full().justify_end().gap_4();
            } else {
                head = head.child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap(px(6.))
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                // Upstream h1: text-xl (20px / 28px) medium.
                                div()
                                    .text_size(px(20.))
                                    .line_height(px(28.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(section_title.to_string()),
                            )
                            .when(self.settings_section == "computer", |row| {
                                row.child(
                                    div()
                                        .px_1p5()
                                        .rounded(px(5.))
                                        .border_1()
                                        .border_color(theme.border)
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .child("Beta"),
                                )
                            }),
                    )
                    .child(
                        // Upstream subtitle: mt-1.5, leading-relaxed (13px / 21.125px).
                        div()
                            .w_full()
                            .mt(px(6.))
                            .text_size(px(13.))
                            .text_color(theme.muted_foreground)
                            .line_height(px(21.125))
                            .child(match &*self.settings_section {
                                "general" => "Defaults for new chats, power, time, and getting started.".to_string(),
                                "appearance" => "Customize the theme, typography, density, and time format.".to_string(),
                                "notifications" => "Choose how Synara tells you when work finishes or needs attention.".to_string(),
                                "layout" => "Choose what the sidebar, the chat top bar, and the Environment panel show.".to_string(),
                                "chats" => "Control live responses, follow-ups, review defaults, and safety confirmations.".to_string(),
                                "shortcuts" => "Change, add, or remove the shortcut for every Synara command.".to_string(),
                                "profile" => "Your local activity, streaks, and a shareable stats card.".to_string(),
                                "accounts" => "Sign in to each agent once per account. Every account gets its own tab in the model picker, its own usage limits, and its own sign-in folder.".to_string(),
                                "providers" => "Choose visible coding agents and manage their installed CLI tools.".to_string(),
                                "models" => "Choose the source control writing style, Git writing model, and custom model slugs.".to_string(),
                                "skills" => "Review reusable workflows discovered across all configured providers.".to_string(),
                                "usage" => "See remaining quota and credits for every signed-in provider.".to_string(),
                                "connectors" => "Sign in to Supabase, Linear, Notion, and other services your agents can use.".to_string(),
                                "integrations" => "Give Codex, Claude, and other local agents scoped access to Synara tasks.".to_string(),
                                "devices" => "Control this computer from your other devices, or control other computers from here.".to_string(),
                                "computer" => "Let agents see and control this computer's desktop, and check backend status.".to_string(),
                                "appsnap" => "Capture another app's frontmost window directly into a task.".to_string(),
                                "worktrees" => "Review and clean up isolated workspaces created by Synara.".to_string(),
                                "archive" => "Find and restore threads you previously archived.".to_string(),
                                "saved-logins" => "Manage passwords saved by the browser, master-password protection, and agent metadata access.".to_string(),
                                "beta" => "Install Synara Beta, copy stable data into it, or switch back.".to_string(),
                                "privacy" => "What Synara shares, and what it can read on this computer.".to_string(),
                                "advanced" => "Recovery tools and this browser's session.".to_string(),
                                "about" => "Version, release channel, and what changed.".to_string(),
                                _ => format!("{section_title} settings are not yet ported to the native shell."),
                            }),
                    ),
            );
            }
            if self.settings_section == "profile" {
                head = head.child(
                    h_flex()
                        .gap_2()
                        .child(outline_btn("Share"))
                        .child(outline_btn("Edit")),
                );
            } else if self.settings_section == "shortcuts" {
                head = head.child(
                    h_flex()
                        .id("sc-reset-all")
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _ev, _w, cx| {
                            this.apply_kb_edits(vec![json!({"type": "reset"})], cx);
                        }))
                        .child(outline_btn("\u{21ba} Reset all to defaults")),
                );
            } else {
                head = head.child(outline_btn("\u{21ba} Restore defaults"));
            }
            settings_pane = settings_pane.child(head);

            if self.settings_section == "general" {
                settings_pane = settings_pane
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(section_label("New chats"))
                            .child(card(
                        v_flex()
                            .child(srow(
                                "Agent and account",
                                "New chats start on this account until you pick another in the model picker. After that they reuse your most recent choice.",
                                select_pill("Codex".to_string(), Some(icons::CODEX)),
                            ))
                            .child(divider())
                            .child(srow(
                                "New threads",
                                "Pick the default workspace mode for newly created draft threads.",
                                select_pill("Local".to_string(), None),
                            ))
                            .into_any_element(),
                            )
                            .into_any_element(),
                        ),
                    )
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(section_label("Power"))
                            .child(card(
                        v_flex()
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .gap_3()
                                    .px_3()
                                    .py(px(10.))
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap(px(2.))
                                            .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Keep computer awake"))
                                            .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child("Uses macOS caffeinate. On: Keep this computer awake at all times. Agent: Keep this computer awake while an agent is working. Off: Let the system sleep normally."))
                                            .child(div().pt_1().text_xs().text_color(theme.muted_foreground).child(self.keep_awake_status())),
                                    )
                                    .child(Self::seg_pick(cx, "seg-keepawake", &["On", "Agent", "Off"], self.keep_awake_mode, &theme, Self::set_keep_awake)),
                            )
                            .into_any_element(),
                            )
                            .into_any_element(),
                        ),
                    )
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(section_label("Time"))
                            .child(card(
                        v_flex()
                            .child(srow(
                                "Time format",
                                "System default follows your browser or OS clock preference.",
                                select_pill("System default".to_string(), None),
                            ))
                            .into_any_element(),
                            )
                            .into_any_element(),
                        ),
                    )
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(section_label("Getting started"))
                            .child(card(
                        v_flex()
                            .child(srow(
                                "Welcome tour",
                                "Replay the first-run setup: agents, account, iPhone pairing, and first project.",
                                outline_btn("Open welcome tour"),
                            ))
                            .child(divider())
                            .child(srow(
                                "App tour",
                                "See where Activity, Tasks, Inbox, and Code review live.",
                                div()
                                    .id("btn-replay-tour")
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _e, _w, cx| {
                                        this.clear_views();
                                        this.tour_step = Some(0);
                                        cx.notify();
                                    }))
                                    .child(outline_btn("Replay tour"))
                                    .into_any_element(),
                            ))
                            .into_any_element(),
                            )
                            .into_any_element(),
                        ),
                    );
            }

            if self.settings_section == "notifications" {
                let accent =
                    text_accent(matches!(theme.mode, gpui_kit::component::ThemeMode::Dark));
                let test_notification = Button::new("test-system-notification")
                    .ghost()
                    .compact()
                    .h(px(28.))
                    .label("Test")
                    .on_click(cx.listener(|_this, _ev, window, cx| {
                        window.push_notification(
                            gpui_kit::component::notification::Notification::new()
                                .title("Synara")
                                .message("Test notification")
                                .system(),
                            cx,
                        );
                    }));
                settings_pane = settings_pane.child(
                    v_flex()
                        .gap_1p5()
                        .child(lgroup("Activity alerts", card(
                            v_flex()
                                .child(srow(
                                    "Activity toasts",
                                    "Show an in-app toast when a chat or managed terminal agent finishes or needs input.",
                                    Self::setting_switch(cx, "enableTaskCompletionToasts", self.setting_bool("enableTaskCompletionToasts", true), accent),
                                ))
                                .child(divider())
                                .child(srow_st(
                                    "Desktop notifications",
                                    "Show an OS notification when a chat or managed terminal agent finishes or needs input while the app is in the background.",
                                    "Desktop app notifications use your operating system notification center.",
                                    h_flex()
                                        .items_center()
                                        .gap_3()
                                        .child(test_notification)
                                        .child(Self::setting_switch(cx, "enableSystemTaskCompletionNotifications", self.setting_bool("enableSystemTaskCompletionNotifications", true), accent))
                                        .into_any_element(),
                                ))
                                .child(divider())
                                .child(srow(
                                    "Wait for subagents",
                                    "Alert once the agent and all of its background subagents have finished. Turn this off to be alerted each time the agent or one of its subagents stops.",
                                    Self::setting_switch(cx, "notifyAfterSubagentsFinish", self.setting_bool("notifyAfterSubagentsFinish", true), accent),
                                ))
                                .into_any_element(),
                        ))),
                );
            }

            // Upstream ChatsSettingsView (web section id "behavior"): Conversation,
            // Review and Safety confirmations, same copy, controls and defaults.
            if self.settings_section == "chats" {
                let accent =
                    text_accent(matches!(theme.mode, gpui_kit::component::ThemeMode::Dark));
                let sw = |cx: &mut Context<Self>, key: &'static str, default: bool| {
                    let on = self.setting_bool(key, default);
                    Self::setting_switch(cx, key, on, accent)
                };
                let follow_up = self.setting_str("followUpBehavior", "queue");
                let sidechat = self.setting_str("sidechatExpiry", "1h");
                let voice_enter = self.setting_str("voiceEnterBehavior", "stop");
                let diff_mode = self.setting_str("diffRenderMode", "split");
                let link_target = self.setting_str("githubLinkOpenTarget", "app");
                let link_label = match link_target.as_str() {
                    "browser" => "In-app browser",
                    "external" => "External browser",
                    _ => "In Synara",
                };
                let link_weak = cx.weak_entity();
                let link_select = Button::new("setting-github-link-target")
                    .ghost()
                    .compact()
                    .h(px(28.))
                    .label(link_label)
                    .dropdown_caret(true)
                    .dropdown_menu(move |menu, _w, _cx| {
                        let mut m = menu.min_w(px(160.));
                        for (value, label) in [
                            ("app", "In Synara"),
                            ("browser", "In-app browser"),
                            ("external", "External browser"),
                        ] {
                            let weak = link_weak.clone();
                            m = m.item(
                                PopupMenuItem::new(label)
                                    .checked(link_target == value)
                                    .on_click(move |_, _w, cx| {
                                        let _ = weak.update(cx, |this, cx| {
                                            this.set_setting_key(
                                                "githubLinkOpenTarget",
                                                json!(value),
                                                cx,
                                            );
                                        });
                                    }),
                            );
                        }
                        m
                    })
                    .into_any_element();
                let conversation = v_flex()
                    .child(srow(
                        "Follow-up behavior",
                        "Choose whether messages sent during an active turn wait in the queue or steer the current run. Ctrl/Cmd+Enter uses the opposite behavior for one message.",
                        Self::setting_seg(cx, "followUpBehavior", &[("queue", "Queue"), ("steer", "Steer")], &follow_up, &theme),
                    ))
                    .child(divider())
                    .child(srow(
                        "Side chat expiry",
                        "Expire a side chat after it sits idle, unviewed and not running, for this long. Expired side chats are read-only and unload their provider session.",
                        Self::setting_seg(cx, "sidechatExpiry", &[("1h", "1 hour"), ("24h", "24 hours"), ("never", "Never")], &sidechat, &theme),
                    ))
                    .child(divider())
                    .child(srow(
                        "Enter while dictating",
                        "Choose what Enter does while a voice note is recording: stop and transcribe into the composer, or stop and send the message once it is transcribed.",
                        Self::setting_seg(cx, "voiceEnterBehavior", &[("stop", "Stop"), ("send", "Stop and send")], &voice_enter, &theme),
                    ))
                    .child(divider())
                    .child(srow(
                        "Assistant output",
                        "Show token-by-token output while a response is in progress.",
                        sw(cx, "enableAssistantStreaming", true),
                    ))
                    .child(divider())
                    .child(srow(
                        "Format agent edits",
                        "After each agent turn, format supported files changed during that turn with tidy. Unsupported files and syntax errors are left unchanged.",
                        sw(cx, "formatAgentFiles", false),
                    ))
                    .child(divider())
                    .child(srow(
                        "Fold finished turns",
                        "Hide a finished turn's tool calls and intermediate messages behind a single \"Worked for…\" line. A turn stays open while it runs or while its background subagents are still working. Turn this off to keep every step visible.",
                        sw(cx, "collapseFinishedTurns", true),
                    ))
                    .child(divider())
                    .child(srow(
                        "Move sent messages to top",
                        "Move each sent message to the top of the conversation. Turn off to keep it at the bottom and follow replies as they stream.",
                        sw(cx, "anchorSentMessagesToTop", true),
                    ))
                    .child(divider())
                    .child(srow(
                        "Effort slider",
                        "Show effort as a slider at the bottom of the composer's model picker, with fast mode and reset alongside it, instead of separate Effort and Speed rows.",
                        sw(cx, "composerEffortSlider", true),
                    ))
                    .child(divider())
                    .child(srow(
                        "Show suggestions",
                        "After a reply finishes, show a predicted next message in the empty composer. Press Tab to insert it as a draft you can edit before sending.",
                        sw(cx, "showComposerSuggestions", true),
                    ))
                    .into_any_element();
                let review = v_flex()
                    .child(srow(
                        "Diff layout",
                        "Default stacked or split layout for threads without a saved choice. Each thread remembers its review-panel changes independently.",
                        Self::setting_seg(cx, "diffRenderMode", &[("stacked", "Stacked"), ("split", "Split")], &diff_mode, &theme),
                    ))
                    .child(divider())
                    .child(srow(
                        "Open pull requests and issues",
                        "Choose where a pull request or issue link in a chat opens: the built-in review view, the in-app browser, or your external browser. Ctrl/Cmd+click always opens the external browser.",
                        link_select,
                    ))
                    .child(divider())
                    .child(srow(
                        "Pull request diff colors",
                        "Show additions in green and deletions in red in pull request summaries.",
                        sw(cx, "showPullRequestDiffColors", true),
                    ))
                    .child(divider())
                    .child(srow(
                        "Include fork upstreams",
                        "Also list pull requests and issues from each project's other GitHub remotes, such as the repository a fork was made from. Off reads only the project's own repository.",
                        sw(cx, "githubInboxIncludeUpstreams", false),
                    ))
                    .child(divider())
                    .child(srow(
                        "Diff line wrapping",
                        "Set the default wrap state when the diff panel opens. The in-panel wrap toggle only affects the current diff session.",
                        sw(cx, "diffWordWrap", false),
                    ))
                    .into_any_element();
                let safety = v_flex()
                    .child(srow(
                        "Delete confirmation",
                        "Ask before deleting a thread and its chat history.",
                        sw(cx, "confirmThreadDelete", true),
                    ))
                    .child(divider())
                    .child(srow(
                        "Archive confirmation",
                        "Ask before archiving a thread.",
                        sw(cx, "confirmThreadArchive", false),
                    ))
                    .child(divider())
                    .child(srow(
                        "Terminal close confirmation",
                        "Ask before closing a terminal tab and clearing its history.",
                        sw(cx, "confirmTerminalTabClose", true),
                    ))
                    .into_any_element();
                settings_pane = settings_pane.child(
                    v_flex()
                        .gap_6()
                        .child(
                            v_flex()
                                .gap_1p5()
                                .child(section_label("Conversation"))
                                .child(card(conversation)),
                        )
                        .child(
                            v_flex()
                                .gap_1p5()
                                .child(section_label("Review"))
                                .child(card(review)),
                        )
                        .child(
                            v_flex()
                                .gap_1p5()
                                .child(section_label("Safety confirmations"))
                                .child(card(safety)),
                        ),
                );
            }

            if self.settings_section == "layout" {
                // Upstream SettingsToggleGroup: two equal columns spanning the
                // card interior, each row's switch right-aligned at the column
                // edge (not trailing the label).
                // Upstream SettingsToggleItem: min-h-8 row, label left, switch
                // right; the grid is gap-x-6 (24px) columns, gap-y-1 (4px) rows.
                let mtog = |label: &'static str, on: bool| {
                    h_flex()
                        .w_full()
                        .min_h(px(32.))
                        .items_center()
                        .justify_between()
                        .child(div().text_size(px(13.)).child(label))
                        .child(toggle(on))
                        .into_any_element()
                };
                let env_group = |label: &'static str,
                                 left: Vec<gpui::AnyElement>,
                                 right: Vec<gpui::AnyElement>| {
                    v_flex()
                        .gap(px(8.))
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(theme.muted_foreground)
                                .child(label),
                        )
                        .child(
                            h_flex()
                                .w_full()
                                .gap(px(24.))
                                .child(v_flex().flex_1().min_w_0().gap(px(4.)).children(left))
                                .child(v_flex().flex_1().min_w_0().gap(px(4.)).children(right)),
                        )
                        .into_any_element()
                };
                settings_pane = settings_pane
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(section_label("Sidebar organization"))
                            .child(card(
                                v_flex()
                                    .child(srow(
                                        "Project order",
                                        "Controls how projects are arranged in the main sidebar.",
                                        select_pill("Manual order".to_string(), None),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Thread order",
                                        "Controls how threads are arranged inside each project in the main sidebar.",
                                        select_pill("Recently active".to_string(), None),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Working and unread on top",
                                        "With Recently active order, move working and unread threads above the rest. The list holds still while your pointer is over it.",
                                        toggle(true).into_any_element(),
                                    ))
                                    .into_any_element(),
                            )),
                    )
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(section_label("Sidebar sections"))
                            .child(card(
                                v_flex()
                                    .child(srow(
                                        "Chats",
                                        "Show the standalone Chats list in the sidebar footer (chats not tied to a project).",
                                        toggle(true).into_any_element(),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Hubs",
                                        "Show the Hubs tab in the sidebar switcher.",
                                        toggle(true).into_any_element(),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Automation runs",
                                        "Show the thread each standalone automation run creates. Runs stay listed on the automation's page either way; threads owned by dedicated or heartbeat automations always stay visible.",
                                        toggle(true).into_any_element(),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Child threads",
                                        "Show threads an agent created from another thread nested under that thread.",
                                        toggle(false).into_any_element(),
                                    ))
                                    .into_any_element(),
                            )),
                    )
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(section_label("Environment panel"))
                            .child(card(
                                v_flex()
                                    .child(srow(
                                        "Open by default",
                                        "Open the chat Environment panel automatically on normal threads. When off, the panel stays closed until you open it. Your last open/close also updates this preference.",
                                        toggle(false).into_any_element(),
                                    ))
                                    .child(divider())
                                    .child(
                                        div().px_3().py(px(10.)).child(env_group(
                                            "Code and status",
                                            vec![
                                                mtog("Subagents", true),
                                                mtog("Repository", true),
                                                mtog("Editor", true),
                                            ],
                                            vec![
                                                mtog("Usage", true),
                                                mtog("Pull request", true),
                                                mtog("Actions", true),
                                            ],
                                        )),
                                    )
                                    .child(divider())
                                    .child(
                                        div().px_3().py(px(10.)).child(env_group(
                                            "Context and notes",
                                            vec![
                                                mtog("Recap", true),
                                                mtog("Project instructions", false),
                                            ],
                                            vec![
                                                mtog("Pinned messages", true),
                                                mtog("Notepad", false),
                                            ],
                                        )),
                                    )
                                    .into_any_element(),
                            )),
                    )
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(section_label("Chat top bar"))
                            .child(card(
                                v_flex()
                                    .child(srow(
                                        "Thread tabs",
                                        "Show open threads as tabs in the chat header. Turn off to show only the current thread's title.",
                                        toggle(true).into_any_element(),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Hand off",
                                        "Show the Hand off button in the chat top bar. Handing off stays available from a thread's sidebar menu and the composer's model picker.",
                                        toggle(true).into_any_element(),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Project actions",
                                        "Show the selected project's actions in the chat top bar. When off, actions still run from their keybindings and the Environment panel's Actions section.",
                                        toggle(true).into_any_element(),
                                    ))
                                    .into_any_element(),
                            )),
                    )
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(section_label("Opens on its own"))
                );
            }

            if self.settings_section == "shortcuts" {
                let shortcut_rows = SHORTCUT_ROWS;
                let search_field = h_flex()
                    .w_full()
                    .h(px(36.))
                    .items_center()
                    .px_3()
                    .gap_2()
                    .rounded(px(10.))
                    .bg(gpui::rgba(0x0000000a))
                    .child(
                        svg_icon(icons::SEARCH)
                            .size(px(16.))
                            .text_color(theme.muted_foreground),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(13.))
                            .text_color(theme.muted_foreground)
                            .child("Search shortcuts"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child("\u{2318}K"),
                    )
                    .into_any_element();
                let mut list = v_flex();
                let mut first = true;
                for (row_idx, (title, desc, kbd, commands)) in shortcut_rows.iter().enumerate() {
                    if !first {
                        list = list.child(divider());
                    }
                    first = false;
                    let title = *title;
                    let desc = *desc;
                    let numbered = commands.contains(".jump.");
                    // Group the resolved rules of this row's commands into
                    // bindings (shared chord for normal rows, shared modifier
                    // mask for numbered families).
                    let mut bindings: Vec<(String, Vec<Value>)> = Vec::new();
                    if !self.kb_rules.is_empty() {
                        for r in &self.kb_rules {
                            let rc = r.get("command").and_then(Value::as_str).unwrap_or("");
                            if !commands.split(' ').any(|c| c == rc) {
                                continue;
                            }
                            let sc = r.get("shortcut").cloned().unwrap_or(Value::Null);
                            let gk = if numbered {
                                sc_mod_mask(&sc)
                            } else {
                                sc_enc_shortcut(&sc)
                            };
                            if let Some(b) = bindings.iter_mut().find(|(g, _)| *g == gk) {
                                b.1.push(r.clone());
                            } else {
                                bindings.push((gk, vec![r.clone()]));
                            }
                        }
                    }
                    let mut right = h_flex().items_center().gap_2();
                    if self.kb_rules.is_empty() {
                        if !kbd.is_empty() {
                            right = right.child(sc_chip(*kbd, &theme));
                        }
                    } else if bindings.is_empty() {
                        right = right.child(
                            div()
                                .text_size(px(13.))
                                .text_color(theme.muted_foreground)
                                .child("Unassigned"),
                        );
                    }
                    let nbind = bindings.len();
                    for (bi, (_g, rules)) in bindings.iter().enumerate() {
                        let label = if numbered {
                            let sc = rules[0].get("shortcut").cloned().unwrap_or(Value::Null);
                            format!("{}1\u{2013}9", sc_mod_label(&sc))
                        } else {
                            sc_fmt_label(&rules[0].get("shortcut").cloned().unwrap_or(Value::Null))
                        };
                        right = right.child(sc_chip(label, &theme));
                        let repl = rules.clone();
                        let label2 = title.to_string();
                        let cmds2: Vec<String> =
                            commands.split(' ').map(|c| c.to_string()).collect();
                        right = right.child(
                            h_flex()
                                .id(SharedString::from(format!("sc-edit-{row_idx}-{bi}")))
                                .cursor_pointer()
                                .on_click(cx.listener(move |this, _ev, _w, cx| {
                                    this.sc_open(
                                        label2.clone(),
                                        cmds2.clone(),
                                        numbered,
                                        repl.clone(),
                                        cx,
                                    );
                                }))
                                .child(
                                    svg_icon(icons::PENCIL)
                                        .size(px(14.))
                                        .text_color(theme.muted_foreground),
                                ),
                        );
                        let rm = rules.clone();
                        right = right.child(
                            h_flex()
                                .id(SharedString::from(format!("sc-del-{row_idx}-{bi}")))
                                .cursor_pointer()
                                .on_click(cx.listener(move |this, _ev, _w, cx| {
                                    this.sc_remove_binding(&rm, cx);
                                }))
                                .child(
                                    svg_icon(icons::TRASH)
                                        .size(px(14.))
                                        .text_color(theme.muted_foreground),
                                ),
                        );
                        if bi + 1 == nbind {
                            let label3 = title.to_string();
                            let cmds3: Vec<String> =
                                commands.split(' ').map(|c| c.to_string()).collect();
                            right = right.child(
                                h_flex()
                                    .id(SharedString::from(format!("sc-add-{row_idx}")))
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |this, _ev, _w, cx| {
                                        this.sc_open(
                                            label3.clone(),
                                            cmds3.clone(),
                                            numbered,
                                            Vec::new(),
                                            cx,
                                        );
                                    }))
                                    .child(
                                        svg_icon(icons::PLUS)
                                            .size(px(14.))
                                            .text_color(theme.muted_foreground),
                                    ),
                            );
                        }
                    }
                    if self.kb_rules.is_empty() || bindings.is_empty() {
                        let label4 = title.to_string();
                        let cmds4: Vec<String> =
                            commands.split(' ').map(|c| c.to_string()).collect();
                        right = right.child(
                            h_flex()
                                .id(SharedString::from(format!("sc-rec-{row_idx}")))
                                .cursor_pointer()
                                .on_click(cx.listener(move |this, _ev, _w, cx| {
                                    this.sc_open(
                                        label4.clone(),
                                        cmds4.clone(),
                                        numbered,
                                        Vec::new(),
                                        cx,
                                    );
                                }))
                                .child(
                                    svg_icon(icons::PENCIL)
                                        .size(px(14.))
                                        .text_color(theme.muted_foreground),
                                ),
                        );
                    }
                    list = list.child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .px_3()
                            .py(px(10.))
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .line_height(px(18.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .child(title),
                                    )
                                    .child(
                                        div()
                                            .w_full()
                                            .text_size(px(13.))
                                            .line_height(px(16.))
                                            .text_color(theme.muted_foreground)
                                            .child(desc),
                                    ),
                            )
                            .child(right),
                    );
                }
                settings_pane = settings_pane
                    .child(search_field)
                    .child(card(list.into_any_element()));
            }

            // ==================== remaining sections ====================
            let sdata = &self.settings_data;
            let cfg_providers: Vec<Value> = arr(get(sdata, "config"), "providers").to_vec();
            let _usage_rows: Vec<Value> = arr(get(sdata, "usage"), "snapshots")
                .to_vec()
                .into_iter()
                .chain(arr(get(sdata, "usage"), "rows").to_vec())
                .chain(arr(get(sdata, "usage"), "providers").to_vec())
                .chain(if get(sdata, "usage").is_array() {
                    arr(get(sdata, "usage"), "").to_vec()
                } else {
                    Vec::new()
                })
                .collect();
            let _dash = |_v: &Value| {
                div()
                    .text_size(px(13.))
                    .text_color(theme.muted_foreground)
                    .child("\u{2014}")
                    .into_any_element()
            };
            fn provider_display(id: &str) -> &str {
                match id {
                    "codex" => "Codex",
                    "claudeAgent" | "claude" => "Claude",
                    "cursor" => "Cursor",
                    "antigravity" => "Antigravity",
                    "grok" => "Grok",
                    "droid" => "Droid",
                    "devin" => "Devin",
                    "opencode" => "OpenCode",
                    "pi" => "Pi",
                    "omp" => "Oh My Pi",
                    other => other,
                }
            }

            // ---------- Profile ----------
            if self.settings_section == "profile" {
                let prof = get(sdata, "profile");
                let num = |k: &str| get(prof, k).as_i64().map(|n| n.to_string());
                let threads_total = arr(get(sdata, "snapshot"), "threads")
                    .len()
                    .max(arr(get(sdata, "snapshot"), "items").len());
                let stat = |val: String, label: &'static str| {
                    v_flex()
                        .flex_1()
                        .items_center()
                        .gap(px(2.))
                        .child(
                            div()
                                .text_size(px(13.))
                                .font_weight(FontWeight::MEDIUM)
                                .child(val),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child(label),
                        )
                        .into_any_element()
                };
                let ins = |label: &'static str, val: String| {
                    h_flex()
                        .w_full()
                        .items_center()
                        .justify_between()
                        .py(px(4.))
                        .child(
                            div()
                                .text_size(px(13.))
                                .text_color(theme.muted_foreground)
                                .child(label),
                        )
                        .child(div().text_size(px(13.)).child(val))
                        .into_any_element()
                };
                let heat_col = || {
                    v_flex()
                        .gap(px(3.))
                        .children((0..7).map(|_| {
                            div()
                                .w(px(9.))
                                .h(px(9.))
                                .rounded(px(2.))
                                .bg(chip_bg)
                                .into_any_element()
                        }))
                        .into_any_element()
                };
                let months = [
                    "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov",
                ];
                settings_pane =
                    settings_pane
                        .child(
                            v_flex()
                                .w_full()
                                .items_center()
                                .gap(px(6.))
                                .py_4()
                                .child(
                                    div()
                                        .w(px(44.))
                                        .h(px(44.))
                                        .rounded_full()
                                        .bg(rgb(0x34a853))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(
                                            div()
                                                .text_size(px(13.))
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(gpui::white())
                                                .child("DE"),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child("Devin"),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .child("@devin \u{b7} Synara"),
                                ),
                        )
                        .child(
                            h_flex()
                                .w_full()
                                .py_2()
                                .child(stat(
                                    num("lifetimeTokens").unwrap_or_else(|| "\u{2014}".into()),
                                    "Lifetime tokens",
                                ))
                                .child(stat(
                                    num("peakDay").unwrap_or_else(|| "\u{2014}".into()),
                                    "Peak day",
                                ))
                                .child(stat(
                                    num("totalPrompts").unwrap_or_else(|| "0".into()),
                                    "Total prompts",
                                ))
                                .child(stat(
                                    num("currentStreak")
                                        .map(|s| format!("{s} days"))
                                        .unwrap_or_else(|| "0 days".into()),
                                    "Current streak",
                                ))
                                .child(stat(
                                    num("longestStreak")
                                        .map(|s| format!("{s} days"))
                                        .unwrap_or_else(|| "0 days".into()),
                                    "Longest streak",
                                )),
                        )
                        .child(section_label("Activity"))
                        .child(
                            v_flex()
                                .w_full()
                                .gap_2()
                                .child(h_flex().gap(px(3.)).children((0..44).map(|_| heat_col())))
                                .child(h_flex().w_full().justify_between().children(months.map(
                                    |m| {
                                        div()
                                            .text_xs()
                                            .text_color(theme.muted_foreground)
                                            .child(m)
                                            .into_any_element()
                                    },
                                ))),
                        )
                        .child(
                            h_flex()
                                .w_full()
                                .items_start()
                                .gap_6()
                                .child(
                                    v_flex()
                                        .flex_1()
                                        .gap_1()
                                        .child(section_label("Activity insights"))
                                        .child(ins("Most used provider", "\u{2014}".into()))
                                        .child(ins("Most used reasoning", "\u{2014}".into()))
                                        .child(ins("Most active hour", "\u{2014}".into()))
                                        .child(ins("Most worked project", "\u{2014}".into()))
                                        .child(ins("Skills explored", "0".into()))
                                        .child(ins("Total skills used", "0".into()))
                                        .child(ins("Total threads", format!("{threads_total}"))),
                                )
                                .child(
                                    v_flex()
                                        .flex_1()
                                        .gap_1()
                                        .child(section_label("Most used plugins"))
                                        .child(
                                            div()
                                                .py(px(4.))
                                                .text_size(px(13.))
                                                .text_color(theme.muted_foreground)
                                                .child("No skills or agents used yet."),
                                        ),
                                ),
                        )
                        .child(section_label("Model usage"))
                        .child(
                            div()
                                .text_size(px(13.))
                                .text_color(theme.muted_foreground)
                                .child("Share of turns."),
                        )
                        .child(
                            div()
                                .pt_1()
                                .text_size(px(13.))
                                .text_color(theme.muted_foreground)
                                .child("No model activity yet."),
                        );
            }

            // ---------- Accounts ----------
            if self.settings_section == "accounts" {
                let acct_icon = |pid: &str| match pid {
                    "devin" => icons::P_DEVIN,
                    "pi" => icons::P_PI,
                    "codex" => icons::CODEX,
                    "claudeAgent" | "claude" => icons::P_CLAUDE,
                    "cursor" => icons::P_CURSOR,
                    "grok" => icons::P_GROK,
                    "droid" => icons::P_DROID,
                    "opencode" => icons::P_OPENCODE,
                    "antigravity" => icons::P_ANTIGRAVITY,
                    "omp" => icons::P_OMP,
                    _ => icons::S_PUZZLE,
                };
                let shown: Vec<(
                    &'static str,
                    &'static str,
                    &'static str,
                    &'static str,
                    &'static str,
                )> = if cfg_providers.is_empty() {
                    vec![
                        (
                            "devin",
                            "Devin",
                            "Authenticated \u{b7} Devin API Key",
                            "Pro",
                            "Daily 100% left \u{b7} Weekly 93% left",
                        ),
                        ("pi", "Pi", "Available", "", ""),
                    ]
                } else {
                    cfg_providers
                        .iter()
                        .filter_map(|p| {
                            let pid = str_at(p, "provider");
                            let pid = if pid.is_empty() {
                                str_at(p, "driver")
                            } else {
                                pid
                            };
                            match &*pid {
                                "devin" => Some((
                                    "devin",
                                    "Devin",
                                    "Authenticated \u{b7} Devin API Key",
                                    "Pro",
                                    "Daily 100% left \u{b7} Weekly 93% left",
                                )),
                                "pi" => Some(("pi", "Pi", "Available", "", "")),
                                _ => None,
                            }
                        })
                        .collect()
                };
                for (pid, name, status, plan, quota) in shown {
                    let _ = pid;
                    settings_pane = settings_pane
                        .child(
                            h_flex()
                                .w_full()
                                .items_center()
                                .justify_between()
                                .child(section_label(name))
                                .child(
                                    h_flex()
                                        .gap_1()
                                        .child(
                                            div()
                                                .px_2()
                                                .py_1()
                                                .text_xs()
                                                .text_color(theme.muted_foreground)
                                                .child("\u{2b06} Import directory"),
                                        )
                                        .child(
                                            div()
                                                .px_2()
                                                .py_1()
                                                .text_xs()
                                                .text_color(theme.muted_foreground)
                                                .child("+ Add account"),
                                        ),
                                ),
                        )
                        .child(card(
                            h_flex()
                                .w_full()
                                .items_center()
                                .gap_3()
                                .px_3()
                                .py(px(10.))
                                .child(
                                    svg_icon(acct_icon(pid))
                                        .size(px(18.))
                                        .text_color(theme.foreground),
                                )
                                .child(
                                    v_flex()
                                        .flex_1()
                                        .min_w_0()
                                        .gap(px(1.))
                                        .child(
                                            div()
                                                .text_size(px(13.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .child(name),
                                        )
                                        .child(
                                            h_flex()
                                                .items_center()
                                                .gap_1p5()
                                                .child(
                                                    div()
                                                        .w(px(6.))
                                                        .h(px(6.))
                                                        .rounded_full()
                                                        .bg(rgb(0x22a865)),
                                                )
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(theme.muted_foreground)
                                                        .child(status),
                                                ),
                                        )
                                        .children(if plan.is_empty() {
                                            vec![]
                                        } else {
                                            vec![div()
                                                .text_xs()
                                                .text_color(theme.muted_foreground)
                                                .child(plan)]
                                        })
                                        .children(if quota.is_empty() {
                                            vec![]
                                        } else {
                                            vec![div()
                                                .text_xs()
                                                .text_color(theme.muted_foreground)
                                                .child(quota)]
                                        }),
                                )
                                .child(toggle(true))
                                .child(
                                    svg_icon(icons::CHEVRON_RIGHT)
                                        .size(px(14.))
                                        .text_color(theme.muted_foreground),
                                )
                                .into_any_element(),
                        ));
                }
                settings_pane = settings_pane.child(
                    v_flex()
                        .gap_2()
                        .px_2()
                        .py_2()
                        .child(div().w_full().text_xs().text_color(theme.muted_foreground).child(
                            "Not installed: Codex, Claude, Cursor, Antigravity, Grok, Droid, OpenCode, Oh My Pi. Install them under Providers to add their accounts."))
                        .child(div().w_full().text_xs().text_color(theme.muted_foreground).child(
                            "A chat stays on the account it started with. Pick the account for a new chat in the model picker. CLI versions, updates, and binary paths live in Providers.")),
                );
            }

            // ---------- Providers ----------
            if self.settings_section == "providers" {
                let p_icon = |pid: &str| match pid {
                    "codex" => icons::CODEX,
                    "claudeAgent" | "claude" => icons::P_CLAUDE,
                    "cursor" => icons::P_CURSOR,
                    "antigravity" => icons::P_ANTIGRAVITY,
                    "grok" => icons::P_GROK,
                    "droid" => icons::P_DROID,
                    "devin" => icons::P_DEVIN,
                    "opencode" => icons::P_OPENCODE,
                    "pi" => icons::P_PI,
                    "omp" => icons::P_OMP,
                    _ => icons::S_PUZZLE,
                };
                let mut rows: Vec<(String, String, String, bool)> = Vec::new();
                if cfg_providers.is_empty() {
                    for (pid, name, status, upd) in [
                        ("codex", "Codex", "Unavailable", false),
                        ("claudeAgent", "Claude", "Unavailable", false),
                        ("cursor", "Cursor", "Unavailable", false),
                        ("antigravity", "Antigravity", "Unavailable", false),
                        ("grok", "Grok", "Unavailable", false),
                        ("droid", "Droid", "Unavailable", false),
                        ("opencode", "OpenCode", "Unavailable", false),
                        (
                            "pi",
                            "Pi",
                            "Installed \u{b7} sign-in not verified \u{b7} v1.4.0 -> v1.6.0",
                            true,
                        ),
                        (
                            "devin",
                            "Devin",
                            "Connected \u{b7} Current v3000.14.30821",
                            false,
                        ),
                        ("omp", "Oh My Pi", "Unavailable", false),
                    ] {
                        rows.push((pid.to_string(), name.to_string(), status.to_string(), upd));
                    }
                } else {
                    // Upstream providerSetupStatusLabel + providerUpdateStatusLabel
                    // (" · "-joined), rows in DEFAULT_PROVIDER_ORDER.
                    let update_checks = get(get(sdata, "settings"), "enableProviderUpdateChecks")
                        .as_bool()
                        .unwrap_or(true);
                    let fmt_ver = |v: &str| -> Option<String> {
                        let v = v.trim();
                        if v.is_empty() {
                            None
                        } else if v.starts_with('v') {
                            Some(v.to_string())
                        } else {
                            Some(format!("v{v}"))
                        }
                    };
                    let mut ordered: Vec<&Value> = cfg_providers.iter().collect();
                    let rank = |p: &Value| {
                        let pid = str_at(p, "provider");
                        let pid = if pid.is_empty() {
                            str_at(p, "driver")
                        } else {
                            pid
                        };
                        PROVIDER_ORDER
                            .iter()
                            .position(|k| *k == pid)
                            .unwrap_or(usize::MAX)
                    };
                    ordered.sort_by_key(|p| rank(p));
                    for p in ordered {
                        let pid = str_at(p, "provider");
                        let pid = if pid.is_empty() {
                            str_at(p, "driver")
                        } else {
                            pid
                        };
                        if pid.is_empty() {
                            continue;
                        }
                        let avail = get(p, "available").as_bool().unwrap_or(false);
                        let auth = str_at(p, "authStatus");
                        let setup = if !avail {
                            "Unavailable".to_string()
                        } else if auth == "unauthenticated" {
                            "Needs sign-in".to_string()
                        } else if str_at(p, "status") != "ready" {
                            "Needs attention".to_string()
                        } else if auth == "unknown" {
                            if str_at(p, "driver") == "opencode" || pid == "opencode" {
                                "Installed \u{b7} authentication managed by OpenCode".to_string()
                            } else {
                                "Installed \u{b7} sign-in not verified".to_string()
                            }
                        } else {
                            "Connected".to_string()
                        };
                        let current = fmt_ver(str_at(p, "version"));
                        let advisory = get(p, "versionAdvisory");
                        let update_label = if !update_checks {
                            current.map(|v| format!("Current {v}"))
                        } else {
                            match str_at(get(p, "updateState"), "status") {
                                "queued" => Some("Update queued".to_string()),
                                "running" => Some("Updating".to_string()),
                                "succeeded" => Some("Updated".to_string()),
                                "failed" => Some("Update failed".to_string()),
                                "unchanged" => Some("Still outdated".to_string()),
                                _ => {
                                    let latest = fmt_ver(str_at(advisory, "latestVersion"));
                                    if str_at(advisory, "status") == "behind_latest"
                                        && latest.is_some()
                                    {
                                        match fmt_ver(str_at(advisory, "currentVersion")) {
                                            Some(cv) => Some(format!(
                                                "{cv} -> {}",
                                                latest.unwrap_or_default()
                                            )),
                                            None => Some(format!(
                                                "Latest {}",
                                                latest.unwrap_or_default()
                                            )),
                                        }
                                    } else {
                                        current.map(|v| format!("Current {v}"))
                                    }
                                }
                            }
                        };
                        let status = match update_label {
                            Some(u) => format!("{setup} \u{b7} {u}"),
                            None => setup,
                        };
                        let upd = str_at(advisory, "status") == "behind_latest";
                        rows.push((
                            pid.to_string(),
                            provider_display(pid).to_string(),
                            status,
                            upd,
                        ));
                    }
                }
                settings_pane = settings_pane
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .child(section_label("Agents"))
                            .child(
                                // Upstream outline button: 28px, radius 10, 9px padding,
                                // 13px medium, border foreground at 7%.
                                div()
                                    .h(px(28.))
                                    .px(px(9.))
                                    .flex()
                                    .items_center()
                                    .rounded(px(10.))
                                    .border_1()
                                    .border_color(theme.foreground.opacity(0.07))
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child("Refresh status"),
                            ),
                    )
                    .child(card(
                        v_flex()
                            .children(rows.iter().enumerate().flat_map(|(i, (pid, name, status, upd))| {
                                let row = h_flex()
                                    .w_full()
                                    .items_center()
                                    .gap_2p5()
                                    .px_3()
                                    .py(px(10.))
                                    .child(div().w(px(16.)).text_size(px(13.)).text_color(theme.muted_foreground).child("\u{283f}"))
                                    .child(provider_icon_16(pid, p_icon(pid), theme.foreground))
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap(px(1.))
                                            .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).text_color(theme.foreground).child(name.clone()))
                                            .child(div().text_xs().text_color(theme.muted_foreground).child(status.clone())),
                                    )
                                    .children(if *upd {
                                        vec![div()
                                            .px_2()
                                            .py_1()
                                            .rounded(px(6.))
                                            .border_1()
                                            .border_color(theme.border)
                                            .text_xs()
                                            .child("\u{2b07} Update")
                                            .into_any_element()]
                                    } else { vec![] })
                                    .child(toggle(true))
                                    .child(svg_icon(icons::CHEVRON_RIGHT).size(px(14.)).text_color(theme.muted_foreground))
                                    .into_any_element();
                                if i > 0 {
                                    vec![divider().into_any_element(), row]
                                } else {
                                    vec![row]
                                }
                            }))
                            .into_any_element(),
                    ))
                    .child(lgroup("Performance", card(
                        v_flex()
                            .child(
                                v_flex()
                                    .px_3()
                                    .py(px(10.))
                                    .gap(px(2.))
                                    .child(
                                        h_flex()
                                            .w_full()
                                            .items_center()
                                            .justify_between()
                                            .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Keep Synara responsive"))
                                            .child(toggle(true)),
                                    )
                                    .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                        "Give agent processes and their commands a moderately lower CPU priority when the machine is busy."))
                                    .child(div().w_full().text_xs().text_color(theme.muted_foreground).child(
                                        "Applies to newly launched agent processes. Restart existing sessions to apply consistently.")),
                            )
                            .into_any_element(),
                    )))
                    .child(lgroup("Updates", card(
                        v_flex()
                            .child(srow("Automatic CLI update checks",
                                "Check Codex, Claude, and other provider CLIs for newer versions in the background.",
                                toggle(true)))
                            .child(divider())
                            .child(
                                v_flex()
                                    .px_3()
                                    .py(px(10.))
                                    .gap(px(2.))
                                    .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Provider updates"))
                                    .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                        "Review installed provider tools that Synara can safely update."))
                                    .child(div().w_full().text_xs().text_color(theme.muted_foreground).child(
                                        "1 update available")),
                            )
                            .into_any_element(),
                    )));
            }

            // ---------- Models ----------
            if self.settings_section == "models" {
                settings_pane = settings_pane
                    .child(lgroup("Generation defaults", card(
                        v_flex()
                            .child(srow("Source control writing style", "In each project, matches recent change descriptions and change request titles.", select_pill("Repository conventions".into(), None)))
                            .child(divider())
                            .child(srow("Add Synara attribution", "Adds a \"Created with Synara\" link to the end of pull request descriptions that Synara creates. Commits and branch names are never changed.", toggle(true)))
                            .child(divider())
                            .child(srow("Git writing model", "Used for generated commit messages, PR titles, and branch names.", select_pill("GPT-6 Luna".into(), None)))
                            .into_any_element(),
                    )))
                    .child(lgroup("Custom models", card(
                        v_flex()
                            .px_3()
                            .py(px(10.))
                            .gap_2()
                            .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Saved model slugs"))
                            .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child("Add custom model slugs for supported providers."))
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .gap_2()
                                    .child(select_pill("Codex".into(), None))
                                    .child(
                                        div()
                                            .flex_1()
                                            .h(px(30.))
                                            .px_2()
                                            .rounded(px(6.))
                                            .border_1()
                                            .border_color(theme.border)
                                            .flex()
                                            .items_center()
                                            .child(div().text_xs().text_color(theme.muted_foreground).child("gpt-6.7-codex-ultra-preview")),
                                    )
                                    .child(
                                        div()
                                            .px_3()
                                            .py_1p5()
                                            .rounded_full()
                                            .bg(rgb(0x26251f))
                                            .text_xs()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(gpui::white())
                                            .child("+ Add"),
                                    ),
                            )
                            .into_any_element(),
                    )));
            }

            // ---------- Skills ----------
            if self.settings_section == "skills" {
                let skills = arr(get(sdata, "skills"), "skills").to_vec();
                let skills = if skills.is_empty() {
                    arr(get(sdata, "skills"), "items").to_vec()
                } else {
                    skills
                };
                settings_pane = settings_pane
                    .child(lgroup("Portable skills", card(
                        v_flex()
                            .px_3()
                            .py(px(10.))
                            .gap(px(2.))
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Synara skills folder"))
                                    .child(div().text_xs().text_color(theme.muted_foreground).child(format!("{} of {} skills enabled", skills.len(), skills.len()))),
                            )
                            .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                "Skills placed here are available on every provider. When a provider already ships its own copy of a skill, that copy is used; otherwise Synara's copy is the fallback."))
                            .child(div().w_full().text_xs().text_color(theme.muted_foreground).child("~/.synara/skills"))
                            .into_any_element(),
                    )))
                    .child(section_label("Skills"));
                if skills.is_empty() {
                    settings_pane = settings_pane.child(card(
                        v_flex()
                            .px_3()
                            .py(px(10.))
                            .gap(px(2.))
                            .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("No skills found"))
                            .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                "Add a skill folder containing a SKILL.md to the Synara skills folder above, or install skills for any supported provider."))
                            .into_any_element(),
                    ));
                }
                for s in &skills {
                    let nm = str_at(s, "name").to_string();
                    let desc = str_at(s, "description").to_string();
                    let pv = str_at(s, "provider").to_string();
                    let desc = if desc.is_empty() {
                        provider_display(&pv).to_string()
                    } else {
                        desc
                    };
                    settings_pane = settings_pane.child(card(
                        v_flex()
                            .px_3()
                            .py(px(8.))
                            .gap(px(1.))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(nm),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child(desc),
                            )
                            .into_any_element(),
                    ));
                }
            }

            // ---------- Usage ----------
            if self.settings_section == "usage" {
                let p_icon2 = |pid: &str| match pid {
                    "codex" => icons::CODEX,
                    "claudeAgent" | "claude" => icons::P_CLAUDE,
                    "cursor" => icons::P_CURSOR,
                    "antigravity" => icons::P_ANTIGRAVITY,
                    "grok" => icons::P_GROK,
                    "droid" => icons::P_DROID,
                    "devin" => icons::P_DEVIN,
                    "opencode" => icons::P_OPENCODE,
                    "pi" => icons::P_PI,
                    "omp" => icons::P_OMP,
                    _ => icons::S_PUZZLE,
                };
                let sidebar_rows: [(&'static str, &'static str, bool); 9] = [
                    ("codex", "Codex", true),
                    ("claudeAgent", "Claude", true),
                    ("cursor", "Cursor", false),
                    ("antigravity", "Antigravity", false),
                    ("grok", "Grok", false),
                    ("droid", "Droid", false),
                    ("opencode", "OpenCode", false),
                    ("pi", "Pi", false),
                    ("devin", "Devin", false),
                ];
                settings_pane = settings_pane
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                div()
                                    .px_2()
                                    .py_1()
                                    .rounded(px(6.))
                                    .bg(chip_bg)
                                    .text_xs()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child("Overview"),
                            )
                            .child(div().px_2().py_1().text_xs().text_color(theme.muted_foreground).child("Analytics")),
                    )
                    .child(lgroup("Sidebar \u{b7} up to 2 accounts", card(
                        v_flex()
                            .children(sidebar_rows.iter().enumerate().flat_map(|(i, (pid, name, on))| {
                                let row = h_flex()
                                    .w_full()
                                    .items_center()
                                    .gap_2p5()
                                    .px_3()
                                    // Upstream rows land 45px apart (44px + 1px divider).
                                    .h(px(44.))
                                    .child(provider_icon_16(pid, p_icon2(pid), theme.foreground))
                                    .child(div().flex_1().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child(name.to_string()))
                                    .child(toggle(*on))
                                    .into_any_element();
                                if i > 0 { vec![divider().into_any_element(), row] } else { vec![row] }
                            }))
                            // Upstream: Ring is the last row of the Sidebar card.
                            .child(divider())
                            .child(srow(
                                "Ring",
                                "Show both limits as two rings, or a single ring for one of them.",
                                seg(&["Both", "5h", "Weekly"], 0),
                            ))
                            .into_any_element(),
                    )))
                    .child(lgroup("Usage popovers", card(
                        v_flex()
                            .child(srow("Show details by default",
                                "Open the details below the limits. When off, they stay behind the Details toggle; your last toggle also updates this preference.",
                                toggle(false)))
                            .child(divider())
                            .child(srow("Banked resets",
                                "Include Codex banked resets in the details.",
                                toggle(true)))
                            .child(divider())
                            .child(srow("Credits and token totals",
                                "Include credit balances and recent token totals (24h, 7d, 30d) in the details.",
                                toggle(true)))
                            .into_any_element(),
                    )));
            }

            // ---------- Connectors ----------
            if self.settings_section == "connectors" {
                let search = h_flex()
                    .w_full()
                    .h(px(36.))
                    .pl(px(14.))
                    .pr_4()
                    .gap(px(8.))
                    .rounded_full()
                    .border_1()
                    .border_color(theme.border)
                    .bg(soft_bg)
                    .items_center()
                    .child(
                        svg_icon(icons::SEARCH)
                            .size(px(14.))
                            .text_color(theme.muted_foreground),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(theme.muted_foreground)
                            .child("Search connectors"),
                    )
                    .into_any_element();
                // Upstream primary pill: foreground fill, background-colored text.
                let dark_btn = |label: &'static str| {
                    h_flex()
                        .h(px(32.))
                        .px(px(11.))
                        .gap_1p5()
                        .items_center()
                        .rounded_full()
                        .bg(theme.foreground)
                        .child(
                            svg_icon(icons::PLUS)
                                .size(px(16.))
                                .text_color(theme.background),
                        )
                        .child(
                            div()
                                .text_size(px(13.))
                                .text_color(theme.background)
                                .child(label),
                        )
                        .into_any_element()
                };
                let catalog: [(&'static str, &'static str); 3] = [
                    ("popular", "Popular"),
                    ("developer", "Developer tools"),
                    ("productivity", "Productivity"),
                ];
                let items: [(&'static str, &'static str, &'static str, &'static str); 18] = [
                    (
                        "popular",
                        "Supabase",
                        "Query Postgres, manage projects, and deploy Edge Functions.",
                        "supabase",
                    ),
                    (
                        "popular",
                        "Vercel",
                        "Manage projects and deployments, and read deployment logs.",
                        "vercel",
                    ),
                    (
                        "popular",
                        "Slack",
                        "Search, read, and post in your workspace. Needs your own Slack app.",
                        "slack",
                    ),
                    (
                        "popular",
                        "Linear",
                        "Find, create, and update issues, projects, and comments.",
                        "linear",
                    ),
                    (
                        "popular",
                        "Notion",
                        "Search, read, and edit pages and databases.",
                        "notion",
                    ),
                    (
                        "popular",
                        "Stripe",
                        "Work with customers, payments, and Stripe docs.",
                        "stripe",
                    ),
                    (
                        "developer",
                        "WorkOS",
                        "Manage users, organizations, SSO, and AuthKit in your dashboard.",
                        "workos",
                    ),
                    (
                        "developer",
                        "Convex",
                        "Start Convex apps and get setup and scaling guidance.",
                        "convex",
                    ),
                    (
                        "developer",
                        "Cloudflare",
                        "Build with Workers, KV, R2, and D1 on your account.",
                        "cloudflare",
                    ),
                    (
                        "developer",
                        "Sentry",
                        "Investigate errors, issues, and releases.",
                        "sentry",
                    ),
                    (
                        "developer",
                        "Neon",
                        "Manage serverless Postgres projects and branches.",
                        "neon",
                    ),
                    (
                        "developer",
                        "PostHog",
                        "Query product analytics, feature flags, and errors.",
                        "posthog",
                    ),
                    (
                        "developer",
                        "Netlify",
                        "Create, deploy, and manage Netlify sites.",
                        "netlify",
                    ),
                    (
                        "developer",
                        "Railway",
                        "Deploy and manage Railway projects and services.",
                        "railway",
                    ),
                    (
                        "developer",
                        "Prisma",
                        "Manage Prisma Postgres databases and backups.",
                        "prisma",
                    ),
                    (
                        "developer",
                        "Hugging Face",
                        "Search models, datasets, Spaces, and papers.",
                        "huggingface",
                    ),
                    (
                        "productivity",
                        "Atlassian",
                        "Work with Jira issues and Confluence pages.",
                        "atlassian",
                    ),
                    (
                        "productivity",
                        "Todoist",
                        "Find, add, and complete tasks and projects.",
                        "todoist",
                    ),
                ];
                settings_pane = settings_pane.child(search).child(
                    h_flex().gap_2().child(dark_btn("Add custom")).child(
                        h_flex()
                            .h(px(32.))
                            .px(px(11.))
                            .items_center()
                            .rounded_full()
                            .border_1()
                            .border_color(theme.border)
                            .child(div().text_size(px(13.)).child("Add local server")),
                    ),
                );
                let mut catalog_grid = v_flex().w_full().gap(px(2.));
                let mut first_cat = true;
                for (cat, cat_name) in &catalog {
                    let label = section_label(cat_name);
                    catalog_grid = catalog_grid.child(if first_cat {
                        first_cat = false;
                        label
                    } else {
                        label.mt(px(16.))
                    });
                    let cat_items: Vec<_> = items.iter().filter(|(c, ..)| c == cat).collect();
                    for chunk in cat_items.chunks(2) {
                        let mut row = h_flex().w_full().gap(px(24.));
                        for (_, name, desc, icon) in chunk {
                            row = row.child(
                                h_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .items_center()
                                    .gap_3()
                                    .py(px(10.))
                                    .child(
                                        div()
                                            .w(px(44.))
                                            .h(px(44.))
                                            .flex_shrink_0()
                                            .rounded(px(14.))
                                            .border_1()
                                            .border_color(theme.border)
                                            .bg(gpui::white())
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(
                                                gpui::img(std::path::PathBuf::from(format!(
                                                    "{}/assets/icons/brands/{}.svg",
                                                    env!("CARGO_MANIFEST_DIR"),
                                                    *icon
                                                )))
                                                .size(px(20.)),
                                            ),
                                    )
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap(px(1.))
                                            .child(
                                                div()
                                                    .text_size(px(14.))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .child(*name),
                                            )
                                            .child(
                                                div()
                                                    .w_full()
                                                    .text_size(px(13.))
                                                    .text_color(theme.muted_foreground)
                                                    .child(*desc),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .px_2()
                                            .text_size(px(13.))
                                            .text_color(theme.muted_foreground)
                                            .child("+"),
                                    ),
                            );
                        }
                        if chunk.len() == 1 {
                            row = row.child(div().flex_1());
                        }
                        catalog_grid = catalog_grid.child(row);
                    }
                }
                settings_pane = settings_pane.child(catalog_grid);
            }

            // ---------- External agents (integrations) ----------
            if self.settings_section == "integrations" {
                let dark_btn = |label: &'static str| {
                    div()
                        .px_3()
                        .py_1p5()
                        .rounded_full()
                        .bg(rgb(0x26251f))
                        .text_xs()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(gpui::white())
                        .child(label)
                        .into_any_element()
                };
                let exts = arr(get(sdata, "external"), "integrations").to_vec();
                let exts = if exts.is_empty() {
                    arr(get(sdata, "external"), "items").to_vec()
                } else {
                    exts
                };
                settings_pane = settings_pane
                    .child(lgroup("Connect a coding agent", card(
                        v_flex()
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .gap_3()
                                    .px_3()
                                    .py(px(10.))
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap(px(2.))
                                            .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Name"))
                                            .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                                "How this connection appears in Synara. Works with Codex, Claude, and any other MCP-capable agent.")),
                                    )
                                    .child(
                                        div()
                                            .w(px(160.))
                                            .h(px(30.))
                                            .px_2()
                                            .rounded(px(6.))
                                            .border_1()
                                            .border_color(theme.border)
                                            .flex()
                                            .items_center()
                                            .child(div().text_xs().child("Coding agent")),
                                    )
                                    .into_any_element(),
                            )
                            .child(divider())
                            .child(srow("Access all of Synara",
                                "The agent can discover and work in every project, including ones you add later. Turn off to pick specific projects.",
                                toggle(true)))
                            .child(divider())
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .gap_3()
                                    .px_3()
                                    .py(px(10.))
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap(px(2.))
                                            .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Advanced permissions"))
                                            .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                                "Optional access for existing tasks, shared checkouts, or execution without approvals. The safe defaults are recommended.")),
                                    )
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap_1()
                                            .child(div().text_size(px(13.)).text_color(theme.muted_foreground).child("Review"))
                                            .child(svg_icon(icons::CHEVRON_RIGHT).size(px(14.)).text_color(theme.muted_foreground)),
                                    )
                                    .into_any_element(),
                            )
                            .child(divider())
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .gap_3()
                                    .px_3()
                                    .py(px(10.))
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap(px(2.))
                                            .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Create connection"))
                                            .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                                "The connection lasts 30 days and can be revoked at any time. The next screen gives you one prompt to paste into your agent.")),
                                    )
                                    .child(dark_btn("Create connection"))
                                    .into_any_element(),
                            )
                            .into_any_element(),
                    )))
                    .child(section_label("Connected agents"));
                if exts.is_empty() {
                    settings_pane = settings_pane.child(card(
                        v_flex()
                            .px_3()
                            .py(px(10.))
                            .gap(px(2.))
                            .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("No connected agents"))
                            .child(div().text_size(px(13.)).text_color(theme.muted_foreground).child("Connect Codex, Claude, or another local MCP agent to create and follow Synara tasks."))
                            .into_any_element(),
                    ));
                }
                for e in &exts {
                    let nm = str_at(e, "name").to_string();
                    let st = str_at(e, "status").to_string();
                    settings_pane = settings_pane.child(card(
                        v_flex()
                            .px_3()
                            .py(px(8.))
                            .gap(px(1.))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(nm),
                            )
                            .child(div().text_xs().text_color(theme.muted_foreground).child(st))
                            .into_any_element(),
                    ));
                }
            }

            // ---------- Devices (connections) ----------
            if self.settings_section == "devices" {
                settings_pane = settings_pane.child(lgroup(
                    "Connections",
                    card(
                        v_flex()
                            .px_3()
                            .py(px(10.))
                            .gap(px(1.))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child("Unavailable in this build"),
                            )
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(theme.muted_foreground)
                                    .child(
                                        "Remote connections require the Node 24 server runtime.",
                                    ),
                            )
                            .into_any_element(),
                    ),
                ));
            }

            // ---------- Computer use ----------
            if self.settings_section == "computer" {
                let comp = get(sdata, "computer");
                let avail = get(comp, "available").as_bool().unwrap_or(false);
                settings_pane = settings_pane
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .px_2()
                            .child(div().text_size(px(13.)).font_weight(FontWeight::SEMIBOLD).text_color(theme.muted_foreground).child("Computer control"))
                            .child(toggle(false)),
                    )
                    .child(div().w_full().px_2().text_size(px(13.)).text_color(theme.muted_foreground).child(
                        "Enable Computer by default in any chat. Leave this off and use /computer-use for one request without adding Computer tools to ordinary turns."))
                    .child(card(
                        v_flex()
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .gap_2()
                                    .px_3()
                                    .py(px(10.))
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap(px(1.))
                                            .child(
                                                h_flex()
                                                    .items_center()
                                                    .gap_1p5()
                                                    .child(div().w(px(6.)).h(px(6.)).rounded_full().bg(rgb(0xd43d3d)))
                                                    .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child(if avail { "Computer control is available" } else { "Computer control is unavailable" })),
                                            )
                                            .child(div().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                                "Computer requires a connected Synara desktop host, which owns native access on that computer.")),
                                    )
                                    .child(outline_btn("Set up"))
                                    .into_any_element(),
                            )
                            .child(divider())
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .gap_3()
                                    .px_3()
                                    .py(px(10.))
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap(px(2.))
                                            .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Cursor colors"))
                                            .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                                "The agent pointer is stock monochrome by default \u{2014} like a normal pointer. Custom colors apply to new computer sessions.")),
                                    )
                                    .child(seg(&["Stock", "Custom"], 0))
                                    .into_any_element(),
                            )
                            .child(divider())
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .gap_3()
                                    .px_3()
                                    .py(px(10.))
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap(px(2.))
                                            .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Preview"))
                                            .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                                "Show the live preview the first time an agent acts on the desktop in a chat. Compact keeps it small and glanceable; Large gives it the full wide card.")),
                                    )
                                    .child(toggle(true))
                                    .child(seg(&["Compact", "Large"], 0))
                                    .into_any_element(),
                            )
                            .into_any_element(),
                    ))
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .child(section_label("Getting started"))
                            .child(div().px_2().py_1().text_size(px(13.)).text_color(theme.muted_foreground).child("\u{2304} Hide guide")),
                    )
                    .child(card(
                        v_flex()
                            .child(
                                v_flex()
                                    .px_3()
                                    .py(px(8.))
                                    .gap(px(1.))
                                    .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Ask for a task"))
                                    .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                        "Type /computer-use followed by your task, for example: \u{201c}/computer-use open Calculator and calculate 123 \u{d7} 45.\u{201d} This enables Computer for that request only. The default setting below can enable it on every turn.")),
                            )
                            .child(divider())
                            .child(
                                v_flex()
                                    .px_3()
                                    .py(px(8.))
                                    .gap(px(1.))
                                    .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Approve the task"))
                                    .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                        "If asked, approve Computer for the task. Use the permission guide when desktop access is missing. Synara may still ask before consequential actions.")),
                            )
                            .child(divider())
                            .child(
                                v_flex()
                                    .px_3()
                                    .py(px(8.))
                                    .gap(px(1.))
                                    .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Follow and stop"))
                                    .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                        "Watch the preview while the agent works. Use Stop in the chat to interrupt the task. Closing the preview only hides it.")),
                            )
                            .into_any_element(),
                    ))
                    .child(div().pt_1().child(outline_btn("Got it")))
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .px_2()
                            .child(div().text_size(px(13.)).text_color(theme.muted_foreground).child("Recent Computer actions"))
                            .child(div().text_size(px(13.)).text_color(theme.muted_foreground).child("\u{203a} Show history")),
                    );
            }

            // ---------- AppSnap ----------
            if self.settings_section == "appsnap" {
                settings_pane = settings_pane
                    .child(card(
                        h_flex()
                            .w_full()
                            .gap_3()
                            .px_3()
                            .py(px(10.))
                            .child(svg_icon(icons::S_SCREEN_CAPTURE).size(px(18.)).text_color(theme.muted_foreground))
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .gap(px(2.))
                                    .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Take an AppSnap to show your agent another app's window"))
                                    .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                        "Press your two-key shortcut while any app is frontmost. Synara captures that window as an image, brings itself forward, and attaches the snap to a task composer \u{2014} the capture stays on this device until you send the message."))
                                    .child(div().w_full().text_xs().text_color(theme.muted_foreground).child(
                                        "AppSnap requires the Synara desktop app on macOS.")),
                            )
                            .into_any_element(),
                    ))
                    .child(lgroup("Capture", card(
                        v_flex()
                            .child(
                                v_flex()
                                    .w_full()
                                    .px_3()
                                    .py(px(10.))
                                    .gap(px(2.))
                                    .min_w_0()
                                    .child(
                                        h_flex()
                                            .w_full()
                                            .items_center()
                                            .justify_between()
                                            .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Enable AppSnap"))
                                            .child(toggle(false)),
                                    )
                                    .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                        "Run the capture listener in the background while Synara is open."))
                                    .child(div().w_full().text_xs().text_color(theme.muted_foreground).child(
                                        "Available in the Synara desktop app")),
                            )
                            .child(divider())
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .gap_3()
                                    .px_3()
                                    .py(px(10.))
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap(px(2.))
                                            .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Shortcut"))
                                            .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                                "Choose exactly two keys: one modifier and one other key. Synara checks its own bindings and asks macOS whether another app already owns the shortcut before saving it.")),
                                    )
                                    .child(
                                        v_flex()
                                            .items_end()
                                            .gap(px(2.))
                                            .child(div().text_size(px(13.)).child("\u{2325} left + \u{2325} right"))
                                            .child(div().text_xs().text_color(theme.muted_foreground).child("Current shortcut")),
                                    )
                                    .into_any_element(),
                            )
                            .child(divider())
                            .child(srow("Destination",
                                "Snaps join the task you interacted with in the last minute, and consecutive snaps stay together. Otherwise Synara opens a fresh task with the capture attached.",
                                select_pill("Automatic".into(), None)))
                            .child(divider())
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .gap_2()
                                    .px_3()
                                    .py(px(10.))
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap(px(2.))
                                            .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Capture sound"))
                                            .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                                "Play a short shutter cue when a window is captured.")),
                                    )
                                    .child(div().text_xs().text_color(theme.muted_foreground).child("Preview"))
                                    .child(toggle(true))
                                    .into_any_element(),
                            )
                            .into_any_element(),
                    )));
            }

            // ---------- Worktrees ----------
            if self.settings_section == "worktrees" {
                let wts = arr(get(sdata, "worktrees"), "worktrees").to_vec();
                let wts = if wts.is_empty() {
                    arr(get(sdata, "worktrees"), "items").to_vec()
                } else {
                    wts
                };
                settings_pane = settings_pane
                    .child(lgroup("Defaults", card(
                        srow("Delete worktree on archive",
                            "After Archive's Undo period, remove a clean worktree only if the task has stopped and no other task uses it. Its branch remains available for recovery.",
                            toggle(false))
                        .into_any_element(),
                    )));
                if wts.is_empty() {
                    settings_pane = settings_pane.child(
                        div()
                            .w_full()
                            .rounded(px(12.))
                            .border_1()
                            .border_color(theme.border)
                            .py(px(24.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(theme.muted_foreground)
                                    .child("No app-managed worktrees found yet."),
                            ),
                    );
                }
                for w in &wts {
                    let nm = str_at(w, "name").to_string();
                    let path = str_at(w, "path").to_string();
                    settings_pane = settings_pane.child(card(
                        v_flex()
                            .px_3()
                            .py(px(8.))
                            .gap(px(1.))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(nm),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child(path),
                            )
                            .into_any_element(),
                    ));
                }
            }

            // ---------- Archive ----------
            if self.settings_section == "archive" {
                settings_pane = settings_pane.child(
                    div()
                        .w_full()
                        .rounded(px(12.))
                        .border_1()
                        .border_color(theme.border)
                        .py(px(40.))
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .gap_2()
                        .child(svg_icon(icons::TRASH).size(px(20.)).text_color(theme.muted_foreground))
                        .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("No archived threads"))
                        .child(div().text_size(px(13.)).text_color(theme.muted_foreground).child(
                            "Archived threads will appear here and can be restored to the sidebar.")),
                );
            }

            // ---------- Saved logins ----------
            if self.settings_section == "saved-logins" {
                if self.vault_snapshot.is_none() && self.vault_error.is_none() {
                    self.refresh_vault(cx);
                }
                if let Some(error) = self.vault_error.clone() {
                    settings_pane = settings_pane.child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .rounded(px(12.))
                            .border_1()
                            .border_color(theme.danger)
                            .px_3()
                            .py(px(10.))
                            .child(
                                div()
                                    .flex_1()
                                    .text_size(px(13.))
                                    .text_color(theme.danger)
                                    .child(error),
                            )
                            .child(
                                div()
                                    .id("vault-retry")
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _ev, _w, cx| {
                                        this.vault_error = None;
                                        this.refresh_vault(cx);
                                    }))
                                    .child(outline_btn("Retry")),
                            ),
                    );
                }
                let snapshot = self.vault_snapshot.clone().unwrap_or(Value::Null);
                if !snapshot.is_null() {
                    let locked = snapshot
                        .pointer("/protection/locked")
                        .and_then(Value::as_bool)
                        .unwrap_or(true);
                    let configured = snapshot
                        .pointer("/protection/configured")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    let settings = snapshot.get("settings").cloned().unwrap_or(Value::Null);
                    let agent_use = settings
                        .get("agentUse")
                        .and_then(Value::as_bool)
                        .unwrap_or(true);
                    let offer_save = settings
                        .get("offerSave")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    let autosave = settings
                        .get("autosave")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);

                    settings_pane = settings_pane
                        .child(lgroup("Protection", card(
                            v_flex()
                                .child(
                                    h_flex()
                                        .w_full()
                                        .items_center()
                                        .justify_between()
                                        .gap_3()
                                        .px_3()
                                        .py(px(10.))
                                        .child(
                                            v_flex()
                                                .flex_1()
                                                .min_w_0()
                                                .gap(px(2.))
                                                .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Master password"))
                                                .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                                    if configured {
                                                        "Unlocks saved logins and protects password reveal on this device."
                                                    } else {
                                                        "Optional protection for saved logins. Use at least 12 characters."
                                                    })),
                                        )
                                        .child(
                                            div()
                                                .id("vault-lock-toggle")
                                                .cursor_pointer()
                                                .on_click(cx.listener(move |this, _ev, _w, cx| {
                                                    if configured && !locked {
                                                        this.lock_vault(cx);
                                                    }
                                                }))
                                                .child(outline_btn(if configured {
                                                    if locked { "Locked" } else { "Lock" }
                                                } else {
                                                    "Not set"
                                                })),
                                        )
                                        .into_any_element(),
                                )
                                .when(locked || !configured, |rows| {
                                    rows.child(divider()).child(
                                        h_flex()
                                            .w_full()
                                            .items_center()
                                            .gap_2()
                                            .px_3()
                                            .py(px(10.))
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .child(Input::new(&self.vault_master_input).appearance(false).mask_toggle()),
                                            )
                                            .when(!configured, |row| {
                                                row.child(
                                                    div()
                                                        .flex_1()
                                                        .min_w_0()
                                                        .child(Input::new(&self.vault_confirm_input).appearance(false).mask_toggle()),
                                                )
                                            })
                                            .child(
                                                div()
                                                    .id("vault-master-submit")
                                                    .cursor_pointer()
                                                    .on_click(cx.listener(move |this, _ev, w, cx| {
                                                        this.submit_vault_master(!configured, w, cx);
                                                    }))
                                                    .child(outline_btn(if configured { "Unlock" } else { "Set password" })),
                                            )
                                            .into_any_element(),
                                    )
                                })
                                .into_any_element(),
                        )))
                        .child(lgroup("Saving and access", card(
                            v_flex()
                                .child(srow(
                                    "Offer to save passwords",
                                    "After a successful sign-in, Synara can save or update the login for that exact site.",
                                    Self::vault_switch(cx, "offerSave", offer_save, theme.primary),
                                ))
                                .child(divider())
                                .child(srow(
                                    "Autosave agent sign-ins",
                                    "Save agent-completed sign-ins without asking. Password saving must stay enabled.",
                                    Self::vault_switch(cx, "autosave", autosave, theme.primary),
                                ))
                                .child(divider())
                                .child(srow(
                                    "Let agents find saved accounts",
                                    "Agents can see account metadata for the exact site only. Password filling and generation stay unavailable.",
                                    Self::vault_switch(cx, "agentUse", agent_use, theme.primary),
                                ))
                                .into_any_element(),
                        )));

                    let pending_prompts = arr(&snapshot, "pending").to_vec();
                    if !pending_prompts.is_empty() {
                        let mut prompts = v_flex();
                        for (index, prompt) in pending_prompts.iter().enumerate() {
                            let id = str_at(prompt, "id").to_string();
                            let save_id = id.clone();
                            let skip_id = id.clone();
                            let title = if str_at(prompt, "mode") == "update" {
                                "Update password"
                            } else {
                                "Save password"
                            };
                            prompts = prompts.child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .gap_3()
                                    .px_3()
                                    .py(px(10.))
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap(px(2.))
                                            .child(
                                                div()
                                                    .text_size(px(13.))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .child(title),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(13.))
                                                    .child(str_at(prompt, "origin").to_string()),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(theme.muted_foreground)
                                                    .child(str_at(prompt, "username").to_string()),
                                            ),
                                    )
                                    .child(
                                        h_flex()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .id(ElementId::Name(
                                                        format!("vault-prompt-skip-{index}").into(),
                                                    ))
                                                    .cursor_pointer()
                                                    .on_click(cx.listener(
                                                        move |this, _ev, _w, cx| {
                                                            this.respond_vault_prompt(
                                                                &skip_id, false, cx,
                                                            );
                                                        },
                                                    ))
                                                    .child(outline_btn("Not now")),
                                            )
                                            .child(
                                                div()
                                                    .id(ElementId::Name(
                                                        format!("vault-prompt-save-{index}").into(),
                                                    ))
                                                    .cursor_pointer()
                                                    .on_click(cx.listener(
                                                        move |this, _ev, _w, cx| {
                                                            this.respond_vault_prompt(
                                                                &save_id, true, cx,
                                                            );
                                                        },
                                                    ))
                                                    .child(outline_btn("Save")),
                                            ),
                                    )
                                    .into_any_element(),
                            );
                            if index + 1 != pending_prompts.len() {
                                prompts = prompts.child(divider().into_any_element());
                            }
                        }
                        settings_pane = settings_pane
                            .child(section_label("Password prompts"))
                            .child(card(prompts.into_any_element()));
                    }

                    let logins = arr(&snapshot, "logins").to_vec();
                    settings_pane = settings_pane.child(section_label("Logins"));
                    if locked {
                        settings_pane = settings_pane.child(card(
                            div()
                                .px_3()
                                .py(px(18.))
                                .text_size(px(13.))
                                .text_color(theme.muted_foreground)
                                .child("Unlock saved logins to view accounts.")
                                .into_any_element(),
                        ));
                    } else if logins.is_empty() {
                        settings_pane = settings_pane.child(card(
                            v_flex()
                                .items_center()
                                .px_3()
                                .py(px(24.))
                                .gap_2()
                                .child(
                                    svg_icon(icons::SHIELD_ACCESS)
                                        .size(px(22.))
                                        .text_color(theme.muted_foreground),
                                )
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .text_color(theme.muted_foreground)
                                        .child("No saved logins yet."),
                                )
                                .into_any_element(),
                        ));
                    } else {
                        let mut list = v_flex();
                        for (index, login) in logins.iter().enumerate() {
                            let id = str_at(login, "id").to_string();
                            let origin = str_at(login, "origin").to_string();
                            let username = str_at(login, "username").to_string();
                            let source = str_at(login, "source").to_string();
                            let status = str_at(login, "status").to_string();
                            let reveal_open = self.vault_reveal_id.as_deref() == Some(id.as_str());
                            let revealed = self
                                .vault_revealed
                                .as_ref()
                                .filter(|(revealed_id, _)| revealed_id == &id)
                                .map(|(_, password)| password.clone());
                            let deleting = self.vault_delete_id.as_deref() == Some(id.as_str());
                            let reveal_id = id.clone();
                            let delete_id = id.clone();
                            let cancel_delete_id = id.clone();
                            let confirm_delete_id = id.clone();
                            let mut row = v_flex()
                                .px_3()
                                .py(px(10.))
                                .gap_2()
                                .child(
                                    h_flex()
                                        .w_full()
                                        .items_start()
                                        .gap_3()
                                        .child(
                                            v_flex()
                                                .flex_1()
                                                .min_w_0()
                                                .gap(px(2.))
                                                .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child(origin))
                                                .child(div().text_size(px(13.)).text_color(theme.muted_foreground).child(if username.is_empty() { "No username".to_string() } else { username }))
                                                .child(div().text_xs().text_color(theme.muted_foreground).child(format!(
                                                    "{}{}{}",
                                                    match source.as_str() {
                                                        "agent" => "Saved by an agent",
                                                        "user" => "Saved by you",
                                                        _ => "Saved login",
                                                    },
                                                    if status == "saved" { "" } else { " · unfinished signup" },
                                                    if status == "expired" { " · expired" } else { "" },
                                                ))),
                                        )
                                        .child(
                                            div()
                                                .id(ElementId::Name(format!("vault-reveal-{index}").into()))
                                                .cursor_pointer()
                                                .on_click(cx.listener(move |this, _ev, _w, cx| {
                                                    if !configured {
                                                        this.vault_error = Some("Set a master password before revealing saved passwords.".into());
                                                        cx.notify();
                                                        return;
                                                    }
                                                    if this.vault_reveal_id.as_deref() == Some(reveal_id.as_str()) {
                                                        this.vault_reveal_id = None;
                                                        this.vault_revealed = None;
                                                    } else {
                                                        this.vault_reveal_id = Some(reveal_id.clone());
                                                        this.vault_revealed = None;
                                                    }
                                                    cx.notify();
                                                }))
                                                .child(outline_btn("Reveal")),
                                        )
                                        .child(
                                            div()
                                                .id(ElementId::Name(format!("vault-delete-{index}").into()))
                                                .cursor_pointer()
                                                .on_click(cx.listener(move |this, _ev, _w, cx| {
                                                    this.vault_delete_id = Some(delete_id.clone());
                                                    cx.notify();
                                                }))
                                                .child(outline_btn("Delete")),
                                        ),
                                );
                            if reveal_open {
                                row = row.child(
                                    h_flex()
                                        .w_full()
                                        .items_center()
                                        .gap_2()
                                        .child(
                                            div().w(px(210.)).child(
                                                Input::new(&self.vault_reveal_input)
                                                    .appearance(false)
                                                    .mask_toggle(),
                                            ),
                                        )
                                        .child(
                                            div()
                                                .id(ElementId::Name(
                                                    format!("vault-reveal-submit-{index}").into(),
                                                ))
                                                .cursor_pointer()
                                                .on_click(cx.listener(|this, _ev, w, cx| {
                                                    this.submit_vault_reveal(w, cx);
                                                }))
                                                .child(outline_btn("Reveal password")),
                                        )
                                        .when(revealed.is_some(), |row| {
                                            row.child(
                                                div()
                                                    .flex_1()
                                                    .rounded(px(8.))
                                                    .border_1()
                                                    .border_color(theme.border)
                                                    .px_2()
                                                    .py_1()
                                                    .text_size(px(13.))
                                                    .child(revealed.unwrap_or_default()),
                                            )
                                        }),
                                );
                            }
                            if deleting {
                                row = row.child(
                                    h_flex()
                                        .w_full()
                                        .items_center()
                                        .justify_between()
                                        .gap_3()
                                        .child(
                                            div()
                                                .text_size(px(13.))
                                                .child("Delete this saved login?"),
                                        )
                                        .child(
                                            h_flex()
                                                .gap_2()
                                                .child(
                                                    div()
                                                        .id(ElementId::Name(
                                                            format!("vault-cancel-delete-{index}")
                                                                .into(),
                                                        ))
                                                        .cursor_pointer()
                                                        .on_click(cx.listener(
                                                            move |this, _ev, _w, cx| {
                                                                if this.vault_delete_id.as_deref()
                                                                    == Some(
                                                                        cancel_delete_id.as_str(),
                                                                    )
                                                                {
                                                                    this.vault_delete_id = None;
                                                                }
                                                                cx.notify();
                                                            },
                                                        ))
                                                        .child(outline_btn("Cancel")),
                                                )
                                                .child(
                                                    div()
                                                        .id(ElementId::Name(
                                                            format!("vault-confirm-delete-{index}")
                                                                .into(),
                                                        ))
                                                        .cursor_pointer()
                                                        .on_click(cx.listener(
                                                            move |this, _ev, _w, cx| {
                                                                this.remove_vault_login(
                                                                    &confirm_delete_id,
                                                                    cx,
                                                                );
                                                            },
                                                        ))
                                                        .child(outline_btn("Delete")),
                                                ),
                                        ),
                                );
                            }
                            list = list.child(row.into_any_element());
                            if index + 1 != logins.len() {
                                list = list.child(divider().into_any_element());
                            }
                        }
                        settings_pane = settings_pane.child(card(list.into_any_element()));
                    }
                }

                settings_pane = settings_pane.child(section_label("Cookie import"));
                let selected_browser = self.cookie_browser.clone();
                let selected_profile = self.cookie_profile.clone();
                let mut browser_rows = h_flex().flex_wrap().gap_2();
                for source in &self.cookie_sources {
                    let id = str_at(source, "id").to_string();
                    let name = str_at(source, "name").to_string();
                    let active = selected_browser.as_deref() == Some(id.as_str());
                    browser_rows = browser_rows.child(
                        div()
                            .id(ElementId::Name(format!("cookie-browser-{id}").into()))
                            .cursor_pointer()
                            .rounded_full()
                            .border_1()
                            .border_color(if active { theme.primary } else { theme.border })
                            .px(px(10.))
                            .py(px(5.))
                            .text_size(px(13.))
                            .child(name)
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                this.cookie_browser = Some(id.clone());
                                this.cookie_profile = None;
                                cx.notify();
                            })),
                    );
                }
                let mut profile_rows = h_flex().flex_wrap().gap_2();
                if let Some(browser_id) = selected_browser.clone() {
                    if let Some(source) = self
                        .cookie_sources
                        .iter()
                        .find(|source| str_at(source, "id") == browser_id)
                    {
                        for profile in arr(source, "profiles") {
                            let id = str_at(profile, "id").to_string();
                            let name = str_at(profile, "name").to_string();
                            let active = selected_profile.as_deref() == Some(id.as_str());
                            profile_rows = profile_rows.child(
                                div()
                                    .id(ElementId::Name(format!("cookie-profile-{id}").into()))
                                    .cursor_pointer()
                                    .rounded_full()
                                    .border_1()
                                    .border_color(if active { theme.primary } else { theme.border })
                                    .px(px(10.))
                                    .py(px(5.))
                                    .text_size(px(13.))
                                    .child(name)
                                    .on_click(cx.listener(move |this, _ev, _w, cx| {
                                        this.cookie_profile = Some(id.clone());
                                        cx.notify();
                                    })),
                            );
                        }
                    }
                }
                let mut import_card = v_flex()
                    .px_3()
                    .py(px(10.))
                    .gap_2()
                    .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Import browser cookies"))
                    .child(div().text_size(px(13.)).text_color(theme.muted_foreground).child(
                        "Copy cookies from a local browser profile into the Synara browser tab already open in the selected chat. Values are transferred directly and never shown here."))
                    .child(browser_rows)
                    .child(profile_rows)
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(div().flex_1().child(Input::new(&self.cookie_domain_input).appearance(false)))
                            .child(
                                div()
                                    .id("cookie-import-submit")
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _ev, _w, cx| {
                                        if !this.cookie_importing {
                                            this.import_cookies(cx);
                                        }
                                    }))
                                    .child(outline_btn(if self.cookie_importing { "Importing" } else { "Import" })),
                            ),
                    );
                if self.cookie_domain_input.read(cx).value().trim().is_empty() {
                    let confirmed = self.cookie_profile_confirmed;
                    import_card = import_card.child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .flex_1()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child("Whole-profile import copies session cookies for every site. Confirm that this browser tab may receive that access."),
                            )
                            .child(
                                switch_track(confirmed, theme.primary, &theme)
                                    .id("cookie-profile-confirmed")
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |this, _ev, _w, cx| {
                                        this.cookie_profile_confirmed = !confirmed;
                                        cx.notify();
                                    })),
                            ),
                    );
                }
                if self.cookie_sources.is_empty() {
                    import_card = import_card.child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child("No supported Chrome, Safari, or Edge profiles were found."),
                    );
                }
                if let Some(result) = self.cookie_result.clone() {
                    import_card = import_card.child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(result),
                    );
                }
                if let Some(error) = self.cookie_error.clone() {
                    import_card =
                        import_card.child(div().text_xs().text_color(theme.danger).child(error));
                }
                settings_pane = settings_pane.child(card(import_card.into_any_element()));
            }

            // ---------- Privacy ----------
            if self.settings_section == "privacy" {
                // Upstream renders nothing extra in this build (Safari onboarding and
                // product-analytics rows are desktop/remote-gated) — title + subtitle only.
            }

            // ---------- Advanced ----------
            if self.settings_section == "advanced" {
                settings_pane = settings_pane
                    .child(lgroup("Developer tools", card(
                        v_flex()
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .gap_3()
                                    .px_3()
                                    .py(px(10.))
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap(px(2.))
                                            .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child("Recovery tools"))
                                            .child(div().w_full().text_size(px(13.)).text_color(theme.muted_foreground).child(
                                                "Rebuild local project indexes without clearing existing chats when the local state gets out of sync."))
                                            .child(div().w_full().text_xs().text_color(theme.muted_foreground).child(
                                                "Visible because projects exist but no chat history is currently available.")),
                                    )
                                    .child(outline_btn("Repair state"))
                                    .into_any_element(),
                            )
                            .child(divider())
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .px_3()
                                    .py(px(8.))
                                    .child(div().text_size(px(13.)).child("What this does"))
                                    .child(svg_icon(icons::CHEVRON_RIGHT).size(px(14.)).text_color(theme.muted_foreground))
                                    .into_any_element(),
                            )
                            .into_any_element(),
                    )));
            }

            // ---------- Beta channel ----------
            if self.settings_section == "beta" {
                let state = crate::beta_channel::DesktopBetaChannel::current().state();
                let busy = self.beta_action_running;
                let beta_button = |id: &'static str,
                                   action: &'static str,
                                   label: &'static str,
                                   cx: &mut Context<Self>| {
                    div()
                        .id(ElementId::Name(format!("beta-{id}").into()))
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _ev, _w, cx| {
                            this.perform_beta_action(action, cx)
                        }))
                        .child(outline_btn(if busy == Some(action) {
                            "Working"
                        } else {
                            label
                        }))
                        .into_any_element()
                };
                let mut rows = v_flex();
                if crate::desktop_identity::runtime_flavor() == "beta" {
                    rows = rows.child(srow(
                        "Switch back to Synara",
                        "Open stable Synara with its own data. Beta data is never copied back.",
                        beta_button("leave", "leave", "Open Synara", cx),
                    ));
                } else {
                    rows = rows
                        .child(srow(
                            "Synara Beta",
                            if state.installed { "Beta is installed on this computer." } else { "Install the beta app beside stable Synara." },
                            beta_button("install", "install", if state.installed { "Open" } else { "Install" }, cx),
                        ))
                        .child(divider())
                        .child(srow(
                            "Copy my data to Beta",
                            "Beta snapshots stable data once at startup. Quit Beta before starting the copy.",
                            beta_button("import", "import", "Copy and open", cx),
                        ));
                }
                if let Some(error) = state.last_import_error {
                    rows = rows.child(divider()).child(
                        div()
                            .px_3()
                            .py(px(10.))
                            .text_size(px(13.))
                            .text_color(theme.danger)
                            .child(error),
                    );
                } else if let Some(done) = state.last_import_at {
                    rows = rows.child(divider()).child(
                        div()
                            .px_3()
                            .py(px(10.))
                            .text_size(px(13.))
                            .text_color(theme.muted_foreground)
                            .child(format!("Last import completed at {done}")),
                    );
                }
                if let Some(message) = self.beta_action_message.clone() {
                    rows = rows.child(divider()).child(
                        div()
                            .px_3()
                            .py(px(10.))
                            .text_size(px(13.))
                            .text_color(theme.muted_foreground)
                            .child(message),
                    );
                }
                settings_pane =
                    settings_pane.child(lgroup("Beta channel", card(rows.into_any_element())));
            }

            // ---------- About ----------
            if self.settings_section == "about" {
                settings_pane = settings_pane
                    .child(lgroup("About", card(
                        v_flex()
                            .child(srow("Version", "Current application version.",
                                div().text_size(px(13.)).text_color(theme.muted_foreground).child("1.6.4").into_any_element()))
                            .child(divider())
                            .child(srow("Release history",
                                "A running log of every update, newest first. Same notes the post-update dialog shows, kept here so you can revisit them any time.",
                                outline_btn("View release history")))
                            .into_any_element(),
                    )));
            }

            if self.settings_section == "appearance" {
                // Plain row without a description line.
                let prow = |label: &'static str, ctrl: gpui::AnyElement| {
                    h_flex()
                        .w_full()
                        .items_center()
                        .justify_between()
                        .px_3()
                        .py(px(10. * dscale))
                        .child(
                            div()
                                .text_size(px(13.))
                                .font_weight(FontWeight::MEDIUM)
                                .child(label),
                        )
                        .child(ctrl)
                        .into_any_element()
                };
                settings_pane = settings_pane.child(
                    v_flex().gap_1p5().child(section_label("Theme")).child(
                        h_flex()
                            .gap_3()
                            .child(
                                div()
                                    .id("theme-card-system")
                                    .cursor_pointer()
                                    .child(theme_card(0, "System", self.appearance_mode == 0))
                                    .on_click(cx.listener(|this, _ev, w, cx| {
                                        this.set_appearance_mode(0, w, cx)
                                    })),
                            )
                            .child(
                                div()
                                    .id("theme-card-light")
                                    .cursor_pointer()
                                    .child(theme_card(1, "Light", self.appearance_mode == 1))
                                    .on_click(cx.listener(|this, _ev, w, cx| {
                                        this.set_appearance_mode(1, w, cx)
                                    })),
                            )
                            .child(
                                div()
                                    .id("theme-card-dark")
                                    .cursor_pointer()
                                    .child(theme_card(2, "Dark", self.appearance_mode == 2))
                                    .on_click(cx.listener(|this, _ev, w, cx| {
                                        this.set_appearance_mode(2, w, cx)
                                    })),
                            ),
                    ),
                );
                // Upstream CODE_THEME_OPTIONS (theme.logic.ts): id, label,
                // supported variants.
                const CODE_THEMES: &[(&str, &str, &[&str])] = &[
                    ("absolutely", "Absolutely", &["light", "dark"]),
                    ("ayu", "Ayu", &["dark"]),
                    ("catppuccin", "Catppuccin", &["light", "dark"]),
                    ("codex", "Codex", &["light", "dark"]),
                    ("synara", "Synara", &["light", "dark"]),
                    ("dracula", "Dracula", &["dark"]),
                    ("everforest", "Everforest", &["light", "dark"]),
                    ("github", "GitHub", &["light", "dark"]),
                    ("gruvbox", "Gruvbox", &["light", "dark"]),
                    ("linear", "Linear", &["light", "dark"]),
                    ("lobster", "Lobster", &["dark"]),
                    ("material", "Material", &["dark"]),
                    ("matrix", "Matrix", &["dark"]),
                    ("monokai", "Monokai", &["dark"]),
                    ("night-owl", "Night Owl", &["dark"]),
                    ("nord", "Nord", &["dark"]),
                    ("notion", "Notion", &["light", "dark"]),
                    ("one", "One", &["light", "dark"]),
                    ("oscurange", "Oscurange", &["dark"]),
                    ("proof", "Proof", &["light"]),
                    ("raycast", "Raycast", &["light", "dark"]),
                    ("rose-pine", "Rose Pine", &["light", "dark"]),
                    ("sentry", "Sentry", &["dark"]),
                    ("solarized", "Solarized", &["light", "dark"]),
                    ("temple", "Temple", &["dark"]),
                    ("tokyo-night", "Tokyo Night", &["dark"]),
                    ("vercel", "Vercel", &["light", "dark"]),
                    ("vscode-plus", "VS Code Plus", &["light", "dark"]),
                ];
                // Upstream shows the resolved variant's card first
                // (AppearanceSettingsView: resolved theme first).
                let resolved_dark = {
                    let m = if self.appearance_mode == 0 {
                        2
                    } else {
                        self.appearance_mode
                    };
                    m == 2
                };
                for dark in [resolved_dark, !resolved_dark] {
                    let (name, desc, mut acc, mut bgc, mut fgc, mut al, mut bl, mut fl) = if dark {
                        (
                            "Dark theme",
                            "Used when your system switches to dark.",
                            0xf2612d,
                            0x111111,
                            0xfcfcfc,
                            "#F2612D".to_string(),
                            "#111111".to_string(),
                            "#FCFCFC".to_string(),
                        )
                    } else {
                        (
                            "Light theme",
                            "System is currently using this light slot.",
                            0xc74614,
                            0xffffff,
                            0x0d0d0d,
                            "#C74614".to_string(),
                            "#FFFFFF".to_string(),
                            "#0D0D0D".to_string(),
                        )
                    };
                    // Stored pack overrides win over the built-in swatches.
                    let pk = self
                        .theme_packs
                        .get(if dark { "dark" } else { "light" })
                        .cloned()
                        .unwrap_or_else(|| json!({}));
                    let var_key = if dark { "dark" } else { "light" };
                    // Upstream ThemePackEditor state for this variant.
                    let pristine = pk.as_object().map(|o| o.is_empty()).unwrap_or(true);
                    let opaque = pk
                        .get("opaqueWindows")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    let sidebar_only = pk
                        .pointer("/translucency/sidebarOnly")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    let opacity_v = pk
                        .pointer("/translucency/opacity")
                        .and_then(Value::as_f64)
                        .unwrap_or(90.)
                        .clamp(15., 100.) as f32;
                    let blur_null = pk.pointer("/translucency/blur").and_then(Value::as_f64);
                    // Null blur = "Auto": upstream drives macOS vibrancy
                    // (radius 30 equivalent).
                    let blur_v = blur_null.unwrap_or(30.).clamp(1., 64.) as f32;
                    let contrast_v = pk
                        .get("contrast")
                        .and_then(Value::as_f64)
                        .unwrap_or(0.)
                        .clamp(0., 100.) as f32;
                    let code_theme = pk
                        .get("codeThemeId")
                        .and_then(Value::as_str)
                        .unwrap_or("synara")
                        .to_string();
                    let code_theme_label = CODE_THEMES
                        .iter()
                        .find(|(id, _, _)| *id == code_theme)
                        .map(|(_, l, _)| l.to_string())
                        .unwrap_or_else(|| {
                            let mut c = code_theme.chars();
                            match c.next() {
                                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                                None => code_theme.clone(),
                            }
                        });
                    let font_ui = pk
                        .pointer("/fonts/ui")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let font_code = pk
                        .pointer("/fonts/code")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let parse_hex = |v: &str| -> Option<u32> {
                        let h = v.trim_start_matches('#');
                        u32::from_str_radix(h, 16).ok()
                    };
                    if let Some(v) = pk.get("accent").and_then(Value::as_str) {
                        if let Some(c) = parse_hex(v) {
                            acc = c;
                            al = v.to_string();
                        }
                    }
                    if let Some(v) = pk.get("background").and_then(Value::as_str) {
                        if let Some(c) = parse_hex(v) {
                            bgc = c;
                            bl = v.to_string();
                        }
                    }
                    if let Some(v) = pk.get("foreground").and_then(Value::as_str) {
                        if let Some(c) = parse_hex(v) {
                            fgc = c;
                            fl = v.to_string();
                        }
                    }
                    // Controls that need the entity before the card chain
                    // builds: weak handle for menu callbacks, current code
                    // theme id, and the slider row builder. Runs before the
                    // `swatch_click` closure below starts borrowing `cx`.
                    let this_weak = cx.weak_entity();
                    let cur_theme = code_theme.clone();
                    let mk_slider_row =
                        |key: &str, want: f32, label: String, window: &mut Window, cx: &mut App| {
                            let st = match self.tp_sliders.get(key) {
                                Some(st) => st.clone(),
                                None => return div().into_any_element(),
                            };
                            let cur = st.read(cx).value().start();
                            if (cur - want).abs() > 0.5 {
                                st.update(cx, |s, cx| s.set_value(want, window, cx));
                            }
                            h_flex()
                                .items_center()
                                .gap_2()
                                .child(Slider::new(&st).w(px(120.)))
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .w(px(40.))
                                        .text_right()
                                        .child(label),
                                )
                                .into_any_element()
                        };
                    let opacity_row = mk_slider_row(
                        &format!("{var_key}-opacity"),
                        opacity_v,
                        format!("{}%", opacity_v.round() as i64),
                        window,
                        cx,
                    );
                    let blur_row = mk_slider_row(
                        &format!("{var_key}-blur"),
                        blur_v,
                        blur_null
                            .map(|b| format!("{}", b.round() as i64))
                            .unwrap_or_else(|| "Auto".to_string()),
                        window,
                        cx,
                    );
                    let contrast_row = mk_slider_row(
                        &format!("{var_key}-contrast"),
                        contrast_v,
                        format!("{}", contrast_v.round() as i64),
                        window,
                        cx,
                    );
                    let swatch_click = |dark: bool, key: &'static str, pill: gpui::AnyElement| {
                        div()
                            .id(SharedString::from(format!(
                                "tp-{}-{}",
                                key,
                                if dark { "dark" } else { "light" }
                            )))
                            .cursor_pointer()
                            .child(pill)
                            .on_click(cx.listener(move |this, _ev, w, cx| {
                                this.open_theme_pack_edit(dark, key, w, cx);
                            }))
                            .into_any_element()
                    };
                    // CodeThemeBadge: 20px bordered box, accent text on the slot surface.
                    let aa_chip = div()
                        .w(px(20.))
                        .h(px(20.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(6.))
                        .border_1()
                        .border_color(theme.border)
                        .bg(rgb(bgc))
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(acc))
                                .child("Aa"),
                        );
                    let mut sel = h_flex()
                        .w_full()
                        .mt_1()
                        .mb_2()
                        .h(px(36.))
                        .items_center()
                        .justify_between()
                        .px_3()
                        .rounded(px(8.))
                        .border_1()
                        .border_color(theme.border);
                    sel = sel.bg(rgb(bgc)).child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgb(fgc))
                            .child("Synara"),
                    );
                    // Upstream Window segmented control: Solid =
                    // opaqueWindows, Translucent = vibrancy/translucency.
                    let seg_opt = |active: bool,
                                   label: &'static str,
                                   id: SharedString,
                                   opaque_value: bool| {
                        div()
                            .id(id)
                            .h(px(24.))
                            .px_2p5()
                            .flex()
                            .items_center()
                            .rounded(px(6.))
                            .cursor_pointer()
                            .when(active, |d| d.bg(theme.secondary))
                            .text_size(px(12.))
                            .text_color(if active {
                                theme.foreground
                            } else {
                                theme.muted_foreground
                            })
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                this.tp_set_pack(dark, "opaqueWindows", json!(opaque_value), cx);
                            }))
                            .child(label)
                    };
                    let window_seg = h_flex()
                        .items_center()
                        .h(px(28.))
                        .p(px(2.))
                        .gap(px(2.))
                        .rounded(px(7.))
                        .border_1()
                        .border_color(theme.border)
                        .child(seg_opt(
                            opaque,
                            "Solid",
                            SharedString::from(format!("tp-solid-{var_key}")),
                            true,
                        ))
                        .child(seg_opt(
                            !opaque,
                            "Translucent",
                            SharedString::from(format!("tp-trans-{var_key}")),
                            false,
                        ));
                    let sidebar_toggle = div()
                        .id(SharedString::from(format!("tp-side-{var_key}")))
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _ev, _w, cx| {
                            let mut t = this
                                .theme_packs
                                .get(var_key)
                                .and_then(|p| p.get("translucency"))
                                .and_then(|v| v.as_object().cloned())
                                .unwrap_or_default();
                            t.insert("sidebarOnly".to_string(), json!(!sidebar_only));
                            this.tp_set_pack(dark, "translucency", Value::Object(t), cx);
                        }))
                        .child(toggle(sidebar_only))
                        .into_any_element();
                    settings_pane = settings_pane.child(card(
                        v_flex()
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .px_3()
                                    .pt_3()
                                    .pb_1()
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .text_size(px(13.))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .child(name),
                                            )
                                            // Upstream Reset shows only on a
                                            // pack with overrides.
                                            .when(!pristine, |d| {
                                                d.child(
                                                    div()
                                                        .id(SharedString::from(format!(
                                                            "tp-reset-{var_key}"
                                                        )))
                                                        .cursor_pointer()
                                                        .text_size(px(12.))
                                                        .text_color(theme.muted_foreground)
                                                        .hover(|h| h.text_color(theme.foreground))
                                                        .on_click(cx.listener(
                                                            move |this, _ev, _w, cx| {
                                                                this.tp_reset_pack(dark, cx);
                                                            },
                                                        ))
                                                        .child("Reset"),
                                                )
                                            }),
                                    )
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap_3()
                                            .child(
                                                div()
                                                    .id(SharedString::from(format!(
                                                        "tp-import-{}",
                                                        if dark { "dark" } else { "light" }
                                                    )))
                                                    .cursor_pointer()
                                                    .text_size(px(13.))
                                                    .text_color(theme.muted_foreground)
                                                    .hover(|h| h.text_color(theme.foreground))
                                                    .on_click(cx.listener(
                                                        move |this, _ev, w, cx| {
                                                            let input = cx.new(|cx| {
                                                                InputState::new(w, cx).placeholder(
                                                                    "{\"tokens\":{\"colors\":{…}}}",
                                                                )
                                                            });
                                                            this.tp_input = Some(input);
                                                            this.tp_import = Some(dark);
                                                            cx.notify();
                                                        },
                                                    ))
                                                    .child("Import"),
                                            )
                                            .child(
                                                div()
                                                    .id(SharedString::from(format!(
                                                        "tp-copy-{}",
                                                        if dark { "dark" } else { "light" }
                                                    )))
                                                    .cursor_pointer()
                                                    .text_size(px(13.))
                                                    .text_color(theme.muted_foreground)
                                                    .hover(|h| h.text_color(theme.foreground))
                                                    .on_click(cx.listener(
                                                        move |this, _ev, _w, cx| {
                                                            this.copy_theme_pack(dark, cx);
                                                        },
                                                    ))
                                                    .child("Copy"),
                                            )
                                            .child(
                                                h_flex()
                                                    .items_center()
                                                    .h(px(28.))
                                                    .pl(px(6.))
                                                    .pr(px(2.))
                                                    .rounded(px(8.))
                                                    .border_1()
                                                    .border_color(theme.border)
                                                    .child(aa_chip)
                                                    .child(
                                                        Button::new(SharedString::from(format!(
                                                            "tp-code-{var_key}"
                                                        )))
                                                        .ghost()
                                                        .compact()
                                                        .h(px(26.))
                                                        .label(code_theme_label.clone())
                                                        .dropdown_caret(true)
                                                        .dropdown_menu(move |menu, _w, _cx| {
                                                            let mut m = menu.min_w(px(200.));
                                                            for (id, label, _) in CODE_THEMES
                                                                .iter()
                                                                .filter(|(_, _, vs)| {
                                                                    vs.contains(&var_key)
                                                                })
                                                            {
                                                                let id = *id;
                                                                let weak = this_weak.clone();
                                                                m = m.item(
                                                                    PopupMenuItem::new(*label)
                                                                        .checked(cur_theme == id)
                                                                        .on_click(
                                                                            move |_, _w, cx| {
                                                                                let _ = weak.update(
                                                                        cx,
                                                                        |this, cx| {
                                                                            this.tp_set_pack(
                                                                                dark,
                                                                                "codeThemeId",
                                                                                json!(id),
                                                                                cx,
                                                                            );
                                                                        },
                                                                    );
                                                                            },
                                                                        ),
                                                                );
                                                            }
                                                            m
                                                        }),
                                                    ),
                                            ),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .px_3()
                                    .pb_2()
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .text_color(theme.muted_foreground)
                                            .child(desc),
                                    )
                                    // Outline "Use {light|dark} theme" button on the inactive slot card.
                                    // Active slot = resolved theme (system resolves dark in this shell).
                                    .when(
                                        (if self.appearance_mode == 0 {
                                            2
                                        } else {
                                            self.appearance_mode
                                        }) != (if dark { 2 } else { 1 }),
                                        |d| {
                                            d.child(
                                                h_flex()
                                                    .id(SharedString::from(format!(
                                                        "use-{}-theme",
                                                        if dark { "dark" } else { "light" }
                                                    )))
                                                    .h(px(24.))
                                                    .px_2p5()
                                                    .items_center()
                                                    .rounded(px(6.))
                                                    .border_1()
                                                    .border_color(theme.border)
                                                    .cursor_pointer()
                                                    .child(
                                                        div()
                                                            .text_size(px(12.))
                                                            .whitespace_nowrap()
                                                            .child(if dark {
                                                                "Use dark theme"
                                                            } else {
                                                                "Use light theme"
                                                            }),
                                                    )
                                                    .on_click(cx.listener(
                                                        move |this, _ev, w, cx| {
                                                            this.set_appearance_mode(
                                                                if dark { 2 } else { 1 },
                                                                w,
                                                                cx,
                                                            );
                                                        },
                                                    )),
                                            )
                                        },
                                    ),
                            )
                            .child(
                                div().w_full().px_3().child(
                                    sel.child(
                                        div()
                                            .rounded(px(6.))
                                            .px(px(12.))
                                            .py(px(4.))
                                            .bg(gpui::rgba((acc << 8) | 0x1f))
                                            .child(
                                                div()
                                                    .text_size(px(13.))
                                                    .text_color(rgb(acc))
                                                    .child("Accent preview"),
                                            ),
                                    ),
                                ),
                            )
                            .child(divider())
                            .child(prow(
                                "Accent",
                                swatch_click(
                                    dark,
                                    "accent",
                                    swatch_pill(SharedString::from(al.clone()), acc),
                                ),
                            ))
                            .child(divider())
                            .child(prow(
                                "Background",
                                swatch_click(
                                    dark,
                                    "background",
                                    swatch_pill(SharedString::from(bl.clone()), bgc),
                                ),
                            ))
                            .child(divider())
                            .child(prow(
                                "Foreground",
                                swatch_click(
                                    dark,
                                    "foreground",
                                    swatch_pill(SharedString::from(fl.clone()), fgc),
                                ),
                            ))
                            .child(divider())
                            .child(prow(
                                "UI font",
                                swatch_click(
                                    dark,
                                    "fontUi",
                                    select_pill(
                                        if font_ui.is_empty() {
                                            "System default".to_string()
                                        } else {
                                            font_ui.clone()
                                        },
                                        None,
                                    ),
                                ),
                            ))
                            .child(divider())
                            .child(prow(
                                "Code font",
                                swatch_click(
                                    dark,
                                    "fontCode",
                                    select_pill(
                                        if font_code.is_empty() {
                                            "JetBrains Mono".to_string()
                                        } else {
                                            font_code.clone()
                                        },
                                        None,
                                    ),
                                ),
                            ))
                            .child(divider())
                            .child(prow("Window", window_seg.into_any_element()))
                            // Sidebar only / Opacity / Blur show only on the
                            // Translucent path (upstream conditional).
                            .when(!opaque, |v| {
                                v.child(divider())
                                    .child(prow("Sidebar only", sidebar_toggle))
                                    .child(divider())
                                    .child(prow("Opacity", opacity_row))
                                    .child(divider())
                                    .child(prow("Blur", blur_row))
                            })
                            .child(divider())
                            .child(prow("Contrast", contrast_row))
                            .into_any_element(),
                    ));
                }
                settings_pane = settings_pane
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(section_label("Effects"))
                            .child(card(
                                v_flex()
                                    .child(srow(
                                        "ASCII background",
                                        "Show the animated ASCII field on empty chats. Reduced motion keeps the field still.",
                                        div().id("ascii-bg-toggle").cursor_pointer()
                                            .child(toggle(self.show_ascii_bg))
                                            .on_click(cx.listener(|this, _ev, _w, cx| this.toggle_ascii_bg(cx)))
                                            .into_any_element(),
                                    ))
                                    .child(srow(
                                        "Full-window backdrop",
                                        "Keep the temple backdrop behind the whole window chrome, not just the empty landing.",
                                        div().id("backdrop-full-toggle").cursor_pointer()
                                            .child(toggle(self.backdrop_full))
                                            .on_click(cx.listener(|this, _ev, _w, cx| this.toggle_backdrop_full(cx)))
                                            .into_any_element(),
                                    ))
                                    .into_any_element(),
                            )),
                    )
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(section_label("Typography and spacing"))
                            .child(card(
                                v_flex()
                                    .child(srow(
                                        "Use system UI font",
                                        "Ignore the theme's custom UI font and render the interface with the native system font (SF Pro on macOS).",
                                        toggle(true),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "UI density",
                                        "Control spacing in the sidebar, composer, chat gutters, and settings rows without changing font size.",
                                        Self::seg_pick(cx, "seg-density", &["Compact", "Comfortable", "Spacious"], self.ui_density as usize, &theme, Self::set_ui_density),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Chat width",
                                        "Control how wide the chat column grows. Wide and Full give tables and wide content more room.",
                                        Self::seg_pick(cx, "seg-chatw", &["Standard", "Wide", "Full"], self.chat_width_mode as usize, &theme, Self::set_chat_width),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Base font size",
                                        "Adjust the app text base in pixels. Chat and UI typography scale proportionally from this value.",
                                        num_field("13"),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Terminal font size",
                                        "Adjust terminal text independently from the app and chat font size.",
                                        num_field("12"),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Terminal font",
                                        "Type any monospace font installed on this device (e.g. Fira Code). Leave empty for the default. Fonts that aren't installed fall back to the system monospace.",
                                        select_pill("Default (JetBrains Mono)".to_string(), None),
                                    ))
                                    .child(divider())
                                    .child(srow(
                                        "Font smoothing",
                                        "Use macOS-style antialiasing for lighter, crisper text rendering.",
                                        toggle(true),
                                    ))
                                    .into_any_element(),
                            )),
                    );
            }
        }
        // Center the content column at upstream max-w-2xl (672px); macOS
        // overlay scrollbars take no layout space, so the centering is exact.
        let settings_pane = h_flex()
            .flex_1()
            .w_full()
            .justify_center()
            .child(settings_pane);
        if std::env::var("SYNARA_SCROLL_BOTTOM").is_ok() {
            self.settings_scroll.scroll_to_bottom();
        }

        // ---- settings sidebar nav (replaces project sidebar) ----
        let mut settings_nav_el = v_flex()
            .flex_1()
            .h_full()
            .px(px(6.))
            .pt(px(12.))
            .pb(px(6.))
            .gap(px(2.));
        settings_nav_el = settings_nav_el.child(
            div()
                .px_2()
                .pb_2()
                .text_base()
                .font_weight(FontWeight::SEMIBOLD)
                .child("Settings"),
        );
        settings_nav_el = settings_nav_el.child(
            h_flex()
                .mx_1()
                .mb_2()
                .h(px(30.))
                .pl(px(14.))
                .pr_2()
                .gap(px(8.))
                .items_center()
                .rounded_full()
                .border_1()
                .border_color(theme.border)
                .bg(soft_bg)
                .child(
                    svg_icon(icons::SEARCH)
                        .size(px(14.))
                        .text_color(theme.muted_foreground),
                )
                .child(
                    div()
                        .flex_1()
                        .child(Input::new(&self.settings_search).appearance(false)),
                ),
        );
        let mut nav_list = v_flex()
            .h(settings_h - px(100.))
            .id("settings-nav-scroll")
            .track_scroll(&self.settings_nav_scroll)
            .overflow_y_scroll()
            .gap(px(2.));
        let nav_row = |id: &'static str,
                       label: &'static str,
                       icon: &'static [u8],
                       badge: Option<&'static str>,
                       cx: &mut gpui::Context<Self>| {
            let active = id == self.settings_section;
            let mut row = h_flex()
                .id(ElementId::Name(format!("setnav-{id}").into()))
                .flex_shrink_0()
                .h(px(28.))
                .px_2()
                .gap_2()
                .items_center()
                .rounded(px(6.))
                .cursor_pointer()
                .child(svg_icon(icon).size(px(16.)).text_color(if active {
                    theme.foreground
                } else {
                    theme.muted_foreground
                }))
                .child(div().text_size(px(13.)).whitespace_nowrap().child(label));
            if let Some(b) = badge {
                row = row.child(
                    div()
                        .ml_auto()
                        .rounded_full()
                        .px(px(6.))
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .border_1()
                        .border_color(theme.border)
                        .child(b),
                );
            }
            if active {
                row = row.bg(sel_fill);
            }
            row.on_click(cx.listener(move |this, _ev, w, cx| {
                this.settings_section = id;
                this.settings_search
                    .update(cx, |s, cx| s.set_value("", w, cx));
                if this.settings_data.is_null() {
                    this.fetch_settings(cx);
                }
                if id == "saved-logins" {
                    this.refresh_vault(cx);
                    this.refresh_cookie_sources(cx);
                }
                cx.notify();
            }))
        };
        let nav_query = self.settings_search.read(cx).value().trim().to_lowercase();
        if nav_query.is_empty() {
            for (group, items) in settings_nav {
                if !group.is_empty() {
                    nav_list = nav_list.child(
                        div()
                            .flex_shrink_0()
                            .mt(px(12.))
                            .px_2()
                            .py_1()
                            .text_size(px(13.))
                            .text_color(theme.muted_foreground)
                            .child(*group),
                    );
                }
                for (id, label, icon, badge) in *items {
                    nav_list = nav_list.child(nav_row(id, label, icon, *badge, cx));
                }
            }
        } else {
            let mut any = false;
            for (_group, items) in settings_nav {
                for (id, label, icon, badge) in *items {
                    if !label.to_lowercase().contains(nav_query.as_str()) {
                        continue;
                    }
                    any = true;
                    nav_list = nav_list.child(nav_row(id, label, icon, *badge, cx));
                }
            }
            if !any {
                nav_list = nav_list.child(
                    div()
                        .px_2()
                        .py_1()
                        .text_size(px(13.))
                        .text_color(theme.muted_foreground)
                        .child("No matching settings"),
                );
            }
        }
        settings_nav_el = settings_nav_el.child(nav_list);

        // ==================== rail route views ====================
        // Segmented pill chip — upstream: muted rounded track, white active.
        // Upstream view switch buttons: 28px, radius 8, padding 4/10; active
        // gets foreground at 2.7%, inactive text is muted (58%).
        let seg_chip = |id: &'static str, label: &'static str, active: bool| {
            let d = div()
                .id(ElementId::Name(id.into()))
                .h(px(28.))
                .px(px(10.))
                .flex()
                .items_center()
                .rounded(px(8.))
                .cursor_pointer()
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(if active {
                            theme.foreground
                        } else {
                            theme.muted_foreground
                        })
                        .child(label),
                );
            if active {
                d.bg(theme.foreground.opacity(0.027))
            } else {
                d
            }
        };
        // Upstream top-bar controls sit 8px from the window top; the native
        // 47px strip is above the pane, so pin them at 8 - 47 = -39px.
        let topbar_y = px(8. - 47.);

        // /tasks — rail only: List|Kanban toggle top-right, centered column.
        // Upstream puts List|Kanban in the top bar, so it doesn't push the
        // column down: overlay it at the top-right instead of a row.
        let tasks_header = h_flex()
            .absolute()
            .top(topbar_y)
            .left(px(56.))
            .right(px(19.))
            .h(px(28.))
            .items_center()
            .child(div().flex_1())
            .child(seg_chip("seg-tasks-list", "List", true))
            .child(
                seg_chip("seg-tasks-kanban", "Kanban", false)
                    .on_click(cx.listener(|this, _e, _w, cx| this.open_route(2, cx))),
            );
        let tasks_pane = v_flex()
            .flex_1()
            .h_full()
            .relative()
            .child(tasks_header)
            .child(
                // Upstream: a centered fixed-width column, contents
                // left-aligned; add-task row is a faint borderless row.
                v_flex().flex_1().items_center().child(
                    // Upstream: max-w-[36rem] column with px-6 + header px-3
                    // = 504px of text, title 40px below the top bar.
                    v_flex()
                        .w(px(504.))
                        .pt(px(35.))
                        .child(
                            div()
                                .text_size(px(24.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("Tasks"),
                        )
                        .child(
                            div()
                                .mt(px(1.))
                                .text_size(px(13.))
                                .text_color(theme.muted_foreground)
                                .child("Add anything you need to do"),
                        )
                        .child(
                            h_flex()
                                .mt(px(28.))
                                .h(px(42.))
                                .gap(px(12.))
                                .items_center()
                                .child(
                                    div()
                                        .size(px(18.))
                                        .rounded_full()
                                        .border_1()
                                        .border_color(theme.muted_foreground),
                                )
                                .child(
                                    div()
                                        .text_size(px(14.))
                                        .text_color(theme.muted_foreground)
                                        .child("Add a task, or press Tab to hand it to an agent"),
                                ),
                        )
                        .child({
                            let today = chrono::Local::now().format("%Y-%m-%d").to_string();
                            let mut rows = v_flex().mt(px(8.)).gap(px(2.));
                            for task in self.inbox_tasks.clone() {
                                let title = str_at(&task, "title").to_string();
                                if title.is_empty() {
                                    continue;
                                }
                                let id = str_at(&task, "id").to_string().replace("null", "");
                                let done = !str_at(&task, "completedAt").is_empty()
                                    && str_at(&task, "completedAt") != "null";
                                let due = str_at(&task, "dueDate").to_string();
                                let due_label = if due.is_empty() || due == "null" {
                                    String::new()
                                } else if due < today {
                                    "Overdue".to_string()
                                } else if due == today {
                                    "Today".to_string()
                                } else {
                                    due[..due.len().min(10)].to_string()
                                };
                                let tid = str_at(&task, "threadId").to_string();
                                let mut row = h_flex()
                                    .id(ElementId::Name(format!("tasks-task-{id}").into()))
                                    .w_full()
                                    .items_center()
                                    .gap_2()
                                    .px_2()
                                    .py(px(6.))
                                    .rounded_md()
                                    .child(
                                        div()
                                            .id(ElementId::Name(
                                                format!("tasks-task-check-{id}").into(),
                                            ))
                                            .cursor_pointer()
                                            .child(
                                                Icon::new(if done {
                                                    AssetIcon::CircleCheck
                                                } else {
                                                    AssetIcon::CircleDot
                                                })
                                                .size(px(14.))
                                                .text_color(if done {
                                                    theme.success
                                                } else {
                                                    theme.muted_foreground
                                                }),
                                            )
                                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                                this.toggle_todo(id.clone(), !done, cx);
                                            })),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .text_size(px(13.))
                                            .text_color(if done {
                                                theme.muted_foreground
                                            } else {
                                                theme.foreground
                                            })
                                            .child(title),
                                    );
                                if !due_label.is_empty() {
                                    row = row.child(
                                        div()
                                            .rounded_full()
                                            .px(px(6.))
                                            .py(px(1.))
                                            .bg(theme.muted)
                                            .text_size(px(11.))
                                            .text_color(if due_label == "Overdue" {
                                                theme.danger
                                            } else {
                                                theme.muted_foreground
                                            })
                                            .child(due_label),
                                    );
                                }
                                if !tid.is_empty() && tid != "null" {
                                    row = row.cursor_pointer().on_click(cx.listener(
                                        move |this, _ev, _w, cx| this.open_thread(tid.clone(), cx),
                                    ));
                                }
                                rows = rows.child(row);
                            }
                            rows
                        }),
                ),
            );

        // /kanban — header strip + one column per project of task cards
        // (threads are the upstream kanban items; "Chats" groups chat-kind).
        let live = |t: &Value| get(t, "deletedAt").is_null() && get(t, "archivedAt").is_null();
        let mut kanban_cols = h_flex().px_4().gap_4().items_start();
        let mut kanban_count = 0usize;
        {
            let snap = self.snapshot.clone().unwrap_or(Value::Null);
            let kb_projects = arr(&snap, "projects");
            let kb_threads = arr(&snap, "threads");
            let chat_ids: Vec<String> = kb_projects
                .iter()
                .filter(|p| str_at(p, "kind") == "chat")
                .map(|p| str_at(p, "id").to_string())
                .collect();
            let mut groups: Vec<(String, Vec<Value>)> = Vec::new();
            for p in kb_projects {
                if str_at(p, "kind") == "chat" {
                    continue;
                }
                let pid = str_at(p, "id");
                let mut kids: Vec<Value> = kb_threads
                    .iter()
                    .filter(|t| str_at(t, "projectId") == pid && live(t))
                    .cloned()
                    .collect();
                kids.sort_by_key(|t| std::cmp::Reverse(str_at(t, "updatedAt").to_string()));
                if !kids.is_empty() {
                    groups.push((str_at(p, "title").to_string(), kids));
                }
            }
            let mut chats: Vec<Value> = kb_threads
                .iter()
                .filter(|t| {
                    live(t)
                        && (chat_ids.iter().any(|id| id == str_at(t, "projectId"))
                            || !kb_projects
                                .iter()
                                .any(|p| str_at(p, "id") == str_at(t, "projectId")))
                })
                .cloned()
                .collect();
            chats.sort_by_key(|t| std::cmp::Reverse(str_at(t, "updatedAt").to_string()));
            if !chats.is_empty() {
                groups.push(("Chats".to_string(), chats));
            }
            kanban_count = groups.iter().map(|g| g.1.len()).sum();
            for (name, kids) in groups {
                let n = kids.len();
                let mut col = v_flex().w(px(288.)).gap(px(8.));
                col = col.child(
                    h_flex()
                        .items_center()
                        .px(px(2.))
                        .child(
                            div()
                                .text_size(px(14.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(name),
                        )
                        .child(
                            div()
                                .ml(px(4.))
                                .text_size(px(13.))
                                .text_color(theme.muted_foreground)
                                .child(format!("{n}")),
                        )
                        .child(div().flex_1())
                        .child(
                            svg_icon(icons::PLUS)
                                .size(px(14.))
                                .text_color(theme.muted_foreground),
                        ),
                );
                for t in &kids {
                    let title = str_at(t, "title").to_string();
                    let branch = str_at(t, "branch");
                    let mut when = rel_time(str_at(t, "updatedAt"));
                    if when.is_empty() {
                        when = rel_time(str_at(t, "createdAt"));
                    }
                    let mut meta = h_flex().gap(px(8.)).items_center().child(
                        svg_icon(icons::CODEX)
                            .size(px(12.))
                            .text_color(theme.muted_foreground),
                    );
                    if !branch.is_empty() {
                        meta = meta.child(
                            h_flex()
                                .gap(px(2.))
                                .items_center()
                                .child(
                                    svg_icon(icons::S_BRANCH_SIMPLE)
                                        .size(px(11.))
                                        .text_color(theme.muted_foreground),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .whitespace_nowrap()
                                        .overflow_hidden()
                                        .child(branch.to_string()),
                                ),
                        );
                    }
                    // Upstream meta tail: column-state label with its status
                    // icon (KanbanStatusIcon). Derived from latestTurn like the
                    // inbox: settled turn -> Done (filled check), error ->
                    // Failed, no turn yet -> Draft (hollow), else Working.
                    // Upstream meta tail (KanbanCardView.tsx): relative timestamp,
                    // then the COLUMN label with KanbanStatusIcon — never a
                    // status pill for Working/Connecting/Completed (redundant).
                    // done = filled #5e6ad2 circle with white check cutout;
                    // inProgress = #f2c94c half-filled ring; draft = hollow.
                    let lt_completed = !str_at(get(t, "latestTurn"), "completedAt").is_empty();
                    let lt_error = str_at(get(t, "latestTurn"), "state") == "error";
                    let lt_none = get(t, "latestTurn").is_null()
                        || str_at(get(t, "latestTurn"), "turnId").is_empty();
                    let klabel: &'static str = if lt_completed {
                        "Done"
                    } else if lt_none {
                        "Draft"
                    } else {
                        "In Progress"
                    };
                    let icon_el: gpui::AnyElement = if lt_completed {
                        div()
                            .size(px(12.))
                            .rounded_full()
                            .bg(rgb(0x5e6ad2))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                svg_icon(icons::CHECK)
                                    .size(px(8.))
                                    .text_color(rgb(0xffffff)),
                            )
                            .into_any_element()
                    } else if lt_none {
                        div()
                            .size(px(11.))
                            .rounded_full()
                            .border_1()
                            .border_color(theme.muted_foreground.opacity(0.6))
                            .into_any_element()
                    } else {
                        div()
                            .size(px(11.))
                            .rounded_full()
                            .border_1()
                            .border_color(rgb(0xf2c94c))
                            .into_any_element()
                    };
                    let mut tail = h_flex().ml_auto().gap(px(8.)).items_center();
                    if lt_error {
                        tail = tail.child(
                            div()
                                .text_size(px(13.))
                                .text_color(rgb(0xff453a))
                                .child("Failed"),
                        );
                    }
                    meta = meta.child(
                        tail.child(
                            div()
                                .text_size(px(12.))
                                .text_color(theme.muted_foreground.opacity(0.7))
                                .child(when),
                        )
                        .child(
                            h_flex().gap(px(4.)).items_center().child(icon_el).child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(theme.muted_foreground.opacity(0.8))
                                    .child(klabel),
                            ),
                        ),
                    );
                    col = col.child(
                        v_flex()
                            .rounded(px(8.))
                            // Upstream kanban card: bg-card/70 with the raised-surface
                            // hairline border and soft shadow — without them the card
                            // is invisible on the white board.
                            .bg(theme.popover.opacity(0.7))
                            .border_1()
                            .border_color(theme.border.opacity(0.6))
                            .shadow_sm()
                            .px(px(12.))
                            .py(px(10.))
                            .gap(px(6.))
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .line_height(relative(1.375))
                                    .font_weight(FontWeight::MEDIUM)
                                    .whitespace_nowrap()
                                    .overflow_hidden()
                                    .child(title),
                            )
                            .child(meta),
                    );
                }
                kanban_cols = kanban_cols.child(col);
            }
        }
        // Upstream /kanban: title, count, view switches and New task live in
        // the top bar (overlaid at upstream's position); an empty board shows
        // a centered "Nothing on the board yet" block.
        let kanban_header = h_flex()
            .absolute()
            .top(topbar_y)
            .left(px(56.))
            .right(px(19.))
            .h(px(28.))
            .items_center()
            .child(
                div()
                    .text_size(px(14.))
                    .font_weight(FontWeight::MEDIUM)
                    .child("Kanban"),
            )
            .child(
                div()
                    .ml(px(8.))
                    .text_size(px(13.))
                    .text_color(theme.muted_foreground)
                    .child(format!("{kanban_count} tasks")),
            )
            .child(
                h_flex()
                    .ml(px(8.))
                    .gap(px(4.))
                    .child(
                        seg_chip("seg-kb-list", "List", false)
                            .on_click(cx.listener(|this, _e, _w, cx| this.open_route(1, cx))),
                    )
                    .child(
                        seg_chip("seg-kb-kanban", "Kanban", true)
                            .on_click(cx.listener(|this, _e, _w, cx| this.open_route(2, cx))),
                    ),
            )
            .child(
                h_flex()
                    .ml(px(8.))
                    .gap(px(4.))
                    .child(seg_chip("seg-kb-att", "Attention", true))
                    .child(seg_chip("seg-kb-classic", "Classic", false)),
            )
            .child(div().flex_1())
            .child(
                h_flex()
                    .h(px(28.))
                    .px(px(10.))
                    .gap(px(6.))
                    .items_center()
                    .rounded(px(10.))
                    .child(
                        svg_icon(icons::PLUS)
                            .size(px(14.))
                            .text_color(theme.foreground.opacity(0.58)),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(theme.foreground.opacity(0.58))
                            .child("New task"),
                    ),
            );
        let kanban_body = if kanban_count == 0 {
            v_flex()
                .flex_1()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .w(px(384.))
                        .text_center()
                        .text_size(px(14.))
                        .line_height(px(21.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(theme.foreground.opacity(0.85))
                        .child("Nothing on the board yet"),
                )
                .child(
                    div()
                        .w(px(384.))
                        .mt(px(4.))
                        .text_center()
                        .text_size(px(13.))
                        .line_height(px(18.))
                        .text_color(theme.muted_foreground)
                        .child("Drafted prompts, running turns, and completed chats will show up here automatically."),
                )
                .into_any_element()
        } else {
            v_flex()
                .flex_1()
                .child(
                    h_flex()
                        .px(px(16.))
                        .pt(px(12.))
                        .pb(px(8.))
                        .gap(px(6.))
                        .items_center()
                        .child(
                            div()
                                .size(px(12.))
                                .rounded(px(3.))
                                .border_1()
                                .border_color(theme.muted_foreground),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(theme.muted_foreground)
                                .child("Needs review"),
                        ),
                )
                .child(kanban_cols)
                .into_any_element()
        };
        let kanban_pane = v_flex().flex_1().h_full().child(kanban_body);

        // /pull-requests — upstream GitHubInbox: `githubInbox.list` rows
        // (pull requests + issues), pinned first, with kind/state/text filters
        // applied client-side over the current state superset.
        let gh_query = self.gh_search.read(cx).value().trim().to_lowercase();
        let visible_all = gh_visible_items(
            &self.gh_items,
            "all",
            self.gh_state_filter,
            &gh_query,
            self.gh_sort,
        );
        let pr_count = visible_all
            .iter()
            .filter(|i| str_at(i, "kind") == "pullRequest")
            .count();
        let issue_count = visible_all
            .iter()
            .filter(|i| str_at(i, "kind") == "issue")
            .count();
        let all_count = visible_all.len();
        let gh_rows = gh_visible_items(
            &self.gh_items,
            self.gh_kind,
            self.gh_state_filter,
            &gh_query,
            self.gh_sort,
        );
        // Upstream repositoryProjects: only kind=="project" entries count.
        let no_projects = arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects")
            .iter()
            .filter(|p| str_at(p, "kind") == "project")
            .count()
            == 0;
        let mut pr_rows = v_flex().px_2().pt_2().gap(px(4.));
        if self.gh_loading && self.gh_items.is_empty() {
            for i in 0..7 {
                pr_rows = pr_rows.child(
                    div()
                        .mx_1()
                        .h(px(76.))
                        .rounded(px(8.))
                        .bg(theme.secondary)
                        .id(SharedString::from(format!("gh-skel-{i}"))),
                );
            }
        } else if gh_rows.is_empty() {
            // Upstream <Empty>: p-12 block, centered 20px semibold title
            // (leading 28px) and a 13px muted line 4px below.
            pr_rows = pr_rows.child(
                v_flex()
                    .w_full()
                    .items_center()
                    .p(px(48.))
                    .child(
                        div()
                            .w_full()
                            .max_w(px(384.))
                            .text_center()
                            .text_size(px(20.))
                            .line_height(px(28.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(if no_projects {
                                "No projects yet"
                            } else if self.gh_items.is_empty() {
                                "No pull requests and issues found"
                            } else {
                                "No matching pull requests or issues"
                            }),
                    )
                    .child(
                        div()
                            .w_full()
                            .max_w(px(384.))
                            .mt(px(4.))
                            .text_center()
                            .text_size(px(13.))
                            .text_color(theme.muted_foreground)
                            .child(if no_projects {
                                "Code review lists pull requests and issues from the GitHub repositories of your projects."
                            } else if self.gh_items.is_empty() {
                                "Nothing here in the repositories of your projects."
                            } else {
                                "Try clearing the search or filters."
                            }),
                    ),
            );
        } else {
            let pinned_rows: Vec<Value> = gh_rows
                .iter()
                .filter(|i| get(i, "isPinned").as_bool() == Some(true))
                .cloned()
                .collect();
            let unpinned_rows: Vec<Value> = gh_rows
                .iter()
                .filter(|i| get(i, "isPinned").as_bool() != Some(true))
                .cloned()
                .collect();
            let mut sections: Vec<(&'static str, Vec<Value>)> = Vec::new();
            if !pinned_rows.is_empty() {
                sections.push(("Pinned", pinned_rows));
            }
            if !unpinned_rows.is_empty() {
                sections.push(("All", unpinned_rows));
            }
            for (label, entries) in sections {
                if label == "Pinned" || entries.len() != gh_rows.len() {
                    pr_rows = pr_rows.child(
                        div()
                            .px_2()
                            .pb_1()
                            .text_size(px(12.))
                            .text_color(theme.muted_foreground)
                            .child(label),
                    );
                }
                let page_key = format!("gh-{label}");
                let extra_pages = self.preview_pages.get(&page_key).copied().unwrap_or(0) as usize;
                let limit = 10usize + extra_pages * 10;
                let total = entries.len();
                for item in entries.iter().take(limit) {
                    let kind = str_at(item, "kind").to_string();
                    let repo = str_at(item, "repository").to_string();
                    let num = get(item, "number").as_u64().unwrap_or(0);
                    let title = str_at(item, "title").to_string();
                    let pinned = get(item, "isPinned").as_bool() == Some(true);
                    let state = str_at(item, "state").to_string();
                    let draft = get(item, "isDraft").as_bool() == Some(true);
                    let selected = if kind == "issue" {
                        self.issue_sel
                            .as_ref()
                            .is_some_and(|(_, r, n)| *n == num && *r == repo)
                    } else {
                        self.pr_sel
                            .as_ref()
                            .is_some_and(|(_, r, n)| *n == num && *r == repo)
                    };
                    let glyph = if kind == "issue" {
                        if state == "open" {
                            Icon::new(AssetIcon::CircleDot)
                                .size(px(14.))
                                .text_color(theme.success)
                                .into_any_element()
                        } else {
                            Icon::new(AssetIcon::CircleCheck)
                                .size(px(14.))
                                .text_color(theme.accent)
                                .into_any_element()
                        }
                    } else {
                        let (ic, color) = match (state.as_str(), draft) {
                            ("merged", _) => (icons::MERGED_SIMPLE, theme.accent),
                            ("closed", _) => (icons::PR_CLOSED_SIMPLE, theme.danger),
                            (_, true) => (icons::PR_SIMPLE, theme.muted_foreground),
                            _ => (icons::PR_SIMPLE, theme.success),
                        };
                        svg_icon(ic)
                            .size(px(14.))
                            .text_color(color)
                            .into_any_element()
                    };
                    let stamp = if self.gh_sort == "updated" {
                        str_at(item, "updatedAt")
                    } else {
                        str_at(item, "createdAt")
                    };
                    let when = gh_relative_time(stamp);
                    let author = gh_actor_label(item);
                    let author_initial = author
                        .chars()
                        .next()
                        .unwrap_or('?')
                        .to_ascii_uppercase()
                        .to_string();
                    let row_item = item.clone();
                    let pin_item = item.clone();
                    pr_rows = pr_rows.child(
                        h_flex()
                            .mx_1()
                            .rounded(px(8.))
                            .items_start()
                            .when(selected, |d| d.bg(theme.secondary))
                            .when(!selected, |d| {
                                d.hover(|h| h.bg(theme.secondary.opacity(0.65)))
                            })
                            .child(
                                v_flex()
                                    .id(SharedString::from(format!(
                                        "gh-row-{}-{}-{repo}-{num}",
                                        kind.replace("pullRequest", "pr"),
                                        str_at(item, "projectId")
                                    )))
                                    .flex_1()
                                    .min_w_0()
                                    .gap_1()
                                    .py_2()
                                    .pl_2()
                                    .pr_1()
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |this, _e, _w, cx| {
                                        this.select_github_item(row_item.clone(), cx);
                                    }))
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(theme.foreground)
                                            .child(title),
                                    )
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap_1p5()
                                            .min_w_0()
                                            .child(glyph)
                                            .child(
                                                div()
                                                    .size(px(16.))
                                                    .flex_shrink_0()
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .rounded_full()
                                                    .bg(theme.secondary)
                                                    .border_1()
                                                    .border_color(theme.border.opacity(0.5))
                                                    .child(
                                                        div()
                                                            .text_size(px(8.))
                                                            .font_weight(FontWeight::MEDIUM)
                                                            .text_color(theme.muted_foreground)
                                                            .child(author_initial),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(12.))
                                                    .text_color(theme.muted_foreground)
                                                    .overflow_hidden()
                                                    .whitespace_nowrap()
                                                    .child(author),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(12.))
                                                    .text_color(theme.muted_foreground)
                                                    .child("·"),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(12.))
                                                    .text_color(theme.muted_foreground)
                                                    .child(when),
                                            )
                                            .child(div().flex_1())
                                            .child(
                                                div()
                                                    .text_size(px(12.))
                                                    .text_color(theme.muted_foreground)
                                                    .child(format!("#{num}")),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .id(SharedString::from(format!("gh-pin-{kind}-{repo}-{num}")))
                                    .size(px(28.))
                                    .mt(px(5.))
                                    .mr(px(3.))
                                    .flex_shrink_0()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(6.))
                                    .cursor_pointer()
                                    .hover(|d| d.bg(theme.background.opacity(0.5)))
                                    .on_click(cx.listener(move |this, _e, _w, cx| {
                                        this.toggle_github_pin(pin_item.clone(), cx);
                                    }))
                                    .child(
                                        svg_icon(if pinned { icons::PIN_FILL } else { icons::PIN })
                                            .size(px(13.))
                                            .text_color(if pinned {
                                                theme.foreground
                                            } else {
                                                theme.muted_foreground
                                            }),
                                    ),
                            ),
                    );
                }
                if total > limit || extra_pages > 0 {
                    let key_more = page_key.clone();
                    let key_less = page_key.clone();
                    pr_rows = pr_rows.child(
                        h_flex()
                            .px_2()
                            .gap_2()
                            .when(total > limit, |d| {
                                d.child(
                                    div()
                                        .id(SharedString::from(format!("{key_more}-more")))
                                        .py_1()
                                        .cursor_pointer()
                                        .text_size(px(12.))
                                        .text_color(theme.muted_foreground)
                                        .hover(|h| h.text_color(theme.foreground))
                                        .child("Show more")
                                        .on_click(cx.listener(move |this, _e, _w, cx| {
                                            let pages = this
                                                .preview_pages
                                                .get(&key_more)
                                                .copied()
                                                .unwrap_or(0);
                                            this.preview_pages.insert(key_more.clone(), pages + 1);
                                            cx.notify();
                                        })),
                                )
                            })
                            .when(extra_pages > 0, |d| {
                                d.child(
                                    div()
                                        .id(SharedString::from(format!("{key_less}-less")))
                                        .py_1()
                                        .cursor_pointer()
                                        .text_size(px(12.))
                                        .text_color(theme.muted_foreground)
                                        .hover(|h| h.text_color(theme.foreground))
                                        .child("Show less")
                                        .on_click(cx.listener(move |this, _e, _w, cx| {
                                            let pages = this
                                                .preview_pages
                                                .get(&key_less)
                                                .copied()
                                                .unwrap_or(0);
                                            this.preview_pages
                                                .insert(key_less.clone(), pages.saturating_sub(1));
                                            cx.notify();
                                        })),
                                )
                            }),
                    );
                }
            }
        }
        if !self.gh_errors.is_empty() {
            let n = self.gh_errors.len();
            let first = str_at(&self.gh_errors[0], "message").to_string();
            pr_rows = pr_rows.child(
                v_flex()
                    .mt(px(12.))
                    .mx_1()
                    .rounded(px(8.))
                    .border_1()
                    .border_color(theme.warning)
                    .bg(theme.secondary)
                    .px_3()
                    .py_2()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.foreground)
                            .child(if n == 1 {
                                "1 project repository was unavailable. Healthy repositories are still shown.".to_string()
                            } else {
                                format!("{n} project repositories were unavailable. Healthy repositories are still shown.")
                            }),
                    )
                    .when(!first.is_empty(), |d| {
                        d.child(
                            div()
                                .text_size(px(11.))
                                .text_color(theme.muted_foreground)
                                .child(first),
                        )
                    }),
            );
        }

        let mut kind_tabs = h_flex().gap(px(2.)).items_center();
        for (kind, label, count) in [
            ("all", "All", all_count),
            ("pullRequest", "PRs", pr_count),
            ("issue", "Issues", issue_count),
        ] {
            let active = self.gh_kind == kind;
            kind_tabs = kind_tabs.child(
                h_flex()
                    .id(SharedString::from(format!("gh-kind-{kind}")))
                    .h(px(24.))
                    .px_2()
                    .gap_1()
                    .items_center()
                    .rounded(px(6.))
                    .cursor_pointer()
                    .when(active, |d| {
                        d.bg(soft_bg).border_1().border_color(theme.border)
                    })
                    .when(!active, |d| d.hover(|h| h.bg(theme.secondary)))
                    .on_click(cx.listener(move |this, _e, _w, cx| {
                        this.gh_kind = kind;
                        cx.notify();
                    }))
                    .child(
                        div()
                            .text_size(px(13.))
                            .when(!active, |d| d.text_color(theme.muted_foreground))
                            .child(label),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.muted_foreground)
                            .child(count.to_string()),
                    ),
            );
        }
        let cur_sort = self.gh_sort;
        let cur_state = self.gh_state_filter;
        let gh_sort_weak = cx.weak_entity();
        let gh_filter_weak = gh_sort_weak.clone();
        let gh_actions_weak = gh_sort_weak.clone();

        // Upstream Code review header buttons: 24px, 14px icons at muted 58%.
        let gh_icon_ink = theme.foreground.opacity(0.58);
        let pullreq_panel = v_flex()
            .flex_1()
            .h_full()
            .px(px(6.))
            .pt(px(8.))
            .pb(px(6.))
            .gap(px(2.))
            .child(
                h_flex()
                    .h(px(42.))
                    .px_2()
                    .items_center()
                    .child(
                        // Upstream panel title: font-display (Cal Sans) 17px.
                        div()
                            .text_size(px(17.))
                            .font_family("Cal Sans")
                            .child("Code review"),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("gh-sort-menu")
                            .ghost()
                            .compact()
                            .size(px(24.))
                            .icon(
                                svg_icon(icons::ARROW_TOP_BOTTOM)
                                    .size(px(14.))
                                    .text_color(gh_icon_ink),
                            )
                            .dropdown_menu(move |menu, _, _| {
                                let mut m = menu.min_w(px(170.));
                                for (value, label) in
                                    [("created", "Newest"), ("updated", "Recently updated")]
                                {
                                    let weak = gh_sort_weak.clone();
                                    m = m.item(
                                        PopupMenuItem::new(label)
                                            .checked(cur_sort == value)
                                            .on_click(move |_, _w, cx| {
                                                let _ = weak.update(cx, |this, cx| {
                                                    this.gh_sort = value;
                                                    this.fetch_github_inbox(cx);
                                                });
                                            }),
                                    );
                                }
                                m
                            }),
                    )
                    .child(div().w(px(6.)))
                    .child(
                        Button::new("gh-filter-menu")
                            .ghost()
                            .compact()
                            .size(px(24.))
                            .icon(
                                svg_icon(icons::FILTER_2)
                                    .size(px(14.))
                                    .text_color(gh_icon_ink),
                            )
                            .dropdown_menu(move |menu, _, _| {
                                let mut m = menu.min_w(px(150.));
                                for (value, label) in
                                    [("open", "Open"), ("closed", "Closed"), ("merged", "Merged")]
                                {
                                    let weak = gh_filter_weak.clone();
                                    m = m.item(
                                        PopupMenuItem::new(label)
                                            .checked(cur_state == value)
                                            .on_click(move |_, _w, cx| {
                                                let _ = weak.update(cx, |this, cx| {
                                                    this.gh_state_filter = value;
                                                    this.fetch_github_inbox(cx);
                                                });
                                            }),
                                    );
                                }
                                m
                            }),
                    )
                    .child(div().w(px(6.)))
                    .child(
                        Button::new("gh-more-menu")
                            .ghost()
                            .compact()
                            .size(px(24.))
                            .icon(svg_icon(icons::MORE).size(px(14.)).text_color(gh_icon_ink))
                            .dropdown_menu(move |menu, _, _| {
                                let weak = gh_actions_weak.clone();
                                menu.min_w(px(150.))
                                    .item(PopupMenuItem::new("Refresh").on_click(
                                        move |_, _w, cx| {
                                            let _ = weak.update(cx, |this, cx| {
                                                this.fetch_github_inbox(cx);
                                            });
                                        },
                                    ))
                            }),
                    ),
            )
            .child(
                h_flex()
                    .mx_1()
                    .mb_2()
                    .h(px(30.))
                    .px_2()
                    .gap_2()
                    .items_center()
                    .rounded(px(8.))
                    .border_1()
                    .border_color(theme.border)
                    .bg(popover_bg)
                    .child(
                        svg_icon(icons::SEARCH)
                            .size(px(14.))
                            .text_color(theme.muted_foreground),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(Input::new(&self.gh_search).appearance(false)),
                    ),
            )
            .child(kind_tabs)
            .child(pr_rows);
        let pullreq_pane = if self.pr_sel.is_some() {
            self.pr_detail_pane(window, cx)
        } else if self.issue_sel.is_some() {
            self.issue_detail_pane(window, cx)
        } else {
            v_flex()
                .flex_1()
                .h_full()
                .items_center()
                .justify_center()
                .gap_2()
                .child(
                    svg_icon(icons::PULL_REQUESTS)
                        .size(px(36.))
                        .text_color(theme.muted_foreground),
                )
                .child(
                    div()
                        .text_base()
                        .font_weight(FontWeight::MEDIUM)
                        .child("Select a pull request or issue"),
                )
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(theme.muted_foreground)
                        .child("Choose one from the sidebar to review it"),
                )
                .into_any_element()
        };

        // /automations — definition list panel + selected detail pane
        // (upstream RailAutomationsPanel + automation runs view).
        let mut autom_panel = v_flex()
            .flex_1()
            .h_full()
            .px(px(6.))
            .py(px(6.))
            .gap(px(2.))
            .child(
                div()
                    .px_2()
                    .pb_2()
                    .text_base()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Automations"),
            )
            .child(
                h_flex()
                    .id("autom-new")
                    .h(px(28.))
                    .px_2()
                    .gap_2()
                    .items_center()
                    .rounded(px(6.))
                    .cursor_pointer()
                    .hover(|d| d.bg(theme.secondary))
                    .on_click(cx.listener(|this, _e, w, cx| {
                        this.open_autom_dialog(None, w, cx);
                    }))
                    .child(
                        svg_icon(icons::PLUS_MED)
                            .size(px(14.))
                            .text_color(theme.foreground),
                    )
                    .child(div().text_size(px(13.)).child("New automation")),
            );
        for def in self.autom_defs.clone() {
            let id = str_at(&def, "id").to_string();
            let sel = self.autom_sel.as_deref() == Some(id.as_str());
            let enabled = get(&def, "enabled").as_bool().unwrap_or(false);
            autom_panel = autom_panel.child(
                h_flex()
                    .id(ElementId::Name(format!("autom-{id}").into()))
                    .px_2()
                    .py_1p5()
                    .gap_2()
                    .items_center()
                    .rounded(px(6.))
                    .cursor_pointer()
                    .when(sel, |d| d.bg(theme.secondary))
                    .when(!sel, |d| d.hover(|d| d.bg(theme.secondary)))
                    .on_click(cx.listener(move |this, _e, _w, cx| {
                        this.autom_sel = Some(id.clone());
                        cx.notify();
                    }))
                    .child(div().w(px(6.)).h(px(6.)).rounded_full().flex_shrink_0().bg(
                        if enabled {
                            Hsla::from(rgb(0x34c759))
                        } else {
                            theme.muted_foreground
                        },
                    ))
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .overflow_hidden()
                                    .child(str_at(&def, "name").to_string()),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(theme.muted_foreground)
                                    .child(Self::autom_summary(get(&def, "schedule"))),
                            ),
                    ),
            );
        }
        if self.autom_defs.is_empty() {
            autom_panel = autom_panel.child(
                div()
                    .mt(px(26.))
                    .w_full()
                    .text_center()
                    .text_size(px(13.))
                    .text_color(theme.muted_foreground)
                    .child("No automations yet"),
            );
        }
        let autom_sel_def = self.autom_sel.as_ref().and_then(|sel| {
            self.autom_defs
                .iter()
                .find(|d| str_at(d, "id") == sel.as_str())
                .cloned()
        });
        // Upstream Refresh: 28px top-bar icon button (radius 10, 16px icon at
        // 58%), overlaid so it doesn't push the pane down.
        let mut autom_pane = v_flex().flex_1().h_full().relative().child(
            div()
                .id("autom-refresh")
                .absolute()
                .top(topbar_y)
                .right(px(24.))
                .size(px(28.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(10.))
                .cursor_pointer()
                .hover(|d| d.bg(theme.secondary))
                .on_click(cx.listener(|this, _e, _w, cx| {
                    this.fetch_automations(cx);
                }))
                .child(
                    svg_icon(icons::REFRESH)
                        .size(px(16.))
                        .text_color(theme.foreground.opacity(0.58)),
                ),
        );
        if let Some(def) = autom_sel_def.clone() {
            let def_id = str_at(&def, "id").to_string();
            let enabled = get(&def, "enabled").as_bool().unwrap_or(false);
            let proj_name = self
                .snapshot
                .as_ref()
                .and_then(|s| {
                    arr(s, "projects")
                        .iter()
                        .find(|p| str_at(p, "id") == str_at(&def, "projectId"))
                        .map(|p| str_at(p, "title").to_string())
                })
                .unwrap_or_default();
            let mut detail = v_flex().flex_1().px_4().gap_3().min_h_0();
            detail = detail.child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_base()
                            .font_weight(FontWeight::SEMIBOLD)
                            .overflow_hidden()
                            .child(str_at(&def, "name").to_string()),
                    )
                    .child(
                        div()
                            .px_2()
                            .h(px(20.))
                            .flex()
                            .items_center()
                            .rounded_full()
                            .bg(if enabled {
                                Hsla::from(rgb(0x34c759)).opacity(0.15)
                            } else {
                                theme.secondary
                            })
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(if enabled {
                                        Hsla::from(rgb(0x34c759))
                                    } else {
                                        theme.muted_foreground
                                    })
                                    .child(if enabled { "Enabled" } else { "Disabled" }),
                            ),
                    ),
            );
            let meta_rows: Vec<(String, String)> = vec![
                (
                    "Schedule".into(),
                    Self::autom_summary(get(&def, "schedule")),
                ),
                ("Project".into(), proj_name),
                (
                    "Next run".into(),
                    str_at(&def, "nextRunAt")
                        .to_string()
                        .replace('T', " ")
                        .trim_end_matches('Z')
                        .to_string(),
                ),
                ("Mode".into(), str_at(&def, "mode").to_string()),
            ];
            for (k, v) in meta_rows {
                if v.is_empty() {
                    continue;
                }
                detail = detail.child(
                    h_flex()
                        .text_size(px(12.))
                        .child(div().w(px(90.)).text_color(theme.muted_foreground).child(k))
                        .child(div().flex_1().min_w_0().child(v)),
                );
            }
            let prompt_txt = str_at(&def, "prompt").to_string();
            if !prompt_txt.is_empty() {
                detail = detail.child(
                    div()
                        .w_full()
                        .p_2()
                        .rounded(px(8.))
                        .bg(theme.secondary)
                        .text_size(px(12.))
                        .text_color(theme.muted_foreground)
                        .child(prompt_txt),
                );
            }
            let runs: Vec<Value> = self
                .autom_runs
                .iter()
                .filter(|r| str_at(r, "automationId") == def_id.as_str())
                .cloned()
                .collect();
            detail = detail.child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .child("Runs"),
            );
            if runs.is_empty() {
                detail = detail.child(
                    div()
                        .text_size(px(12.))
                        .text_color(theme.muted_foreground)
                        .child("No runs yet"),
                );
            }
            let mut run_list = v_flex().gap_1();
            for run in runs.iter().take(10) {
                run_list = run_list.child(
                    h_flex()
                        .items_center()
                        .gap_2()
                        .text_size(px(12.))
                        .child(
                            div()
                                .w(px(64.))
                                .text_color(theme.muted_foreground)
                                .child(str_at(run, "status").to_string()),
                        )
                        .child(
                            div().flex_1().min_w_0().overflow_hidden().child(
                                str_at(run, "scheduledFor")
                                    .to_string()
                                    .replace('T', " ")
                                    .trim_end_matches('Z')
                                    .to_string(),
                            ),
                        )
                        .child(
                            div().text_color(theme.muted_foreground).child(
                                get(run, "trigger")
                                    .get("type")
                                    .and_then(|v| v.as_str())
                                    .map(str::to_string)
                                    .unwrap_or_else(|| str_at(run, "trigger").to_string()),
                            ),
                        ),
                );
            }
            detail = detail.child(run_list);
            let id_run = def_id.clone();
            let id_del = def_id.clone();
            let def_edit = def.clone();
            detail = detail.child(
                h_flex()
                    .mt_2()
                    .gap_2()
                    .child(
                        div()
                            .id("autom-runnow")
                            .h(px(28.))
                            .px_3()
                            .flex()
                            .items_center()
                            .rounded(px(8.))
                            .bg(theme.foreground)
                            .cursor_pointer()
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(theme.background)
                                    .child("Run now"),
                            )
                            .on_click(cx.listener(move |this, _e, _w, cx| {
                                this.autom_run_now(id_run.clone(), cx);
                            })),
                    )
                    .child(
                        div()
                            .id("autom-edit")
                            .h(px(28.))
                            .px_3()
                            .flex()
                            .items_center()
                            .rounded(px(8.))
                            .border_1()
                            .border_color(theme.border)
                            .cursor_pointer()
                            .hover(|d| d.bg(theme.secondary))
                            .child(div().text_size(px(12.)).child("Edit"))
                            .on_click(cx.listener(move |this, _e, w, cx| {
                                this.open_autom_dialog(Some(def_edit.clone()), w, cx);
                            })),
                    )
                    .child(
                        div()
                            .id("autom-delete")
                            .h(px(28.))
                            .px_3()
                            .flex()
                            .items_center()
                            .rounded(px(8.))
                            .border_1()
                            .border_color(theme.border)
                            .cursor_pointer()
                            .hover(|d| d.bg(theme.secondary))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(rgb(0xff453a))
                                    .child("Delete"),
                            )
                            .on_click(cx.listener(move |this, _e, _w, cx| {
                                this.autom_delete(id_del.clone(), cx);
                            })),
                    ),
            );
            autom_pane = autom_pane.child(detail);
        } else {
            autom_pane = autom_pane.child(
                v_flex()
                    .flex_1()
                    .items_center()
                    .justify_center()
                    // Upstream <Empty>: gap-6 (24px) between media/header/content,
                    // title font-heading semibold text-xl (20/28), desc mt-1 13px.
                    .pb(px(60.))
                    .child(
                        svg_icon(icons::AUTOMATIONS)
                            .size(px(32.))
                            .text_color(theme.muted_foreground),
                    )
                    .child(
                        div()
                            .mt(px(24.))
                            .text_size(px(20.))
                            .line_height(px(28.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Automations"),
                    )
                    .child(
                        div()
                            .mt(px(4.))
                            .text_size(px(13.))
                            .text_color(theme.muted_foreground)
                            .child("Pick an automation in the panel to see its runs, or"),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(theme.muted_foreground)
                            .child("schedule a new one."),
                    )
                    .child(
                        div()
                            .id("autom-new2")
                            .mt(px(24.))
                            .h(px(30.))
                            .px_4()
                            .flex()
                            .items_center()
                            .rounded(px(8.))
                            .bg(theme.foreground)
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _e, w, cx| {
                                this.open_autom_dialog(None, w, cx);
                            }))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(theme.background)
                                    .child("New automation"),
                            ),
                    ),
            );
        }

        // /hubs (studio rail icon) — hubs list panel + empty pane.
        let studio_panel = v_flex()
            .flex_1()
            .h_full()
            .px(px(6.))
            .py(px(6.))
            .gap(px(2.))
            .child(
                h_flex()
                    .px_2()
                    .pb_2()
                    .items_center()
                    .child(
                        div()
                            .text_base()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Hubs"),
                    )
                    .child(
                        div()
                            .pl_1()
                            .text_size(px(13.))
                            .text_color(theme.muted_foreground)
                            .child("\u{2304}"),
                    )
                    .child(div().flex_1())
                    .child(
                        svg_icon(icons::SEARCH)
                            .size(px(14.))
                            .text_color(theme.muted_foreground),
                    ),
            )
            .child(
                h_flex()
                    .h(px(28.))
                    .px_2()
                    .gap_2()
                    .items_center()
                    .rounded(px(6.))
                    .child(
                        svg_icon(icons::PENCIL)
                            .size(px(14.))
                            .text_color(theme.muted_foreground),
                    )
                    .child(div().text_size(px(13.)).child("New hub")),
            )
            .child(
                div()
                    .mt(px(14.))
                    .px_2()
                    .py_1()
                    .text_size(px(13.))
                    .text_color(theme.muted_foreground)
                    .child("Hubs"),
            )
            .child(
                div()
                    .mt(px(24.))
                    .w_full()
                    .text_center()
                    .text_size(px(13.))
                    .text_color(theme.muted_foreground)
                    .child("No hubs yet"),
            );
        let studio_pane = v_flex()
            .flex_1()
            .h_full()
            .items_center()
            .justify_center()
            // Upstream <Empty>: title font-heading semibold text-xl (20/28),
            // desc mt-1 13px muted; the optical center sits ~33px above the
            // geometric pane center in the reference capture.
            .pb(px(60.))
            .child(
                div()
                    .text_size(px(20.))
                    .line_height(px(28.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("No hubs yet"),
            )
            .child(
                div()
                    .mt(px(4.))
                    .text_size(px(13.))
                    .text_color(theme.muted_foreground)
                    .child("Create one from the sidebar to coordinate work across repos."),
            );

        // ---- terminal surface (upstream workspace mode: the Terminal surface
        // replaces transcript + composer; TerminalWorkspaceTabs switches surfaces) ----
        let mut term_sub_strip = h_flex()
            .px(px(20.))
            .pt(px(6.))
            .gap(px(6.))
            .items_end()
            .border_b_1()
            .border_color(rgba(0x00000012));
        for (ti, _tab) in self.term_tabs.iter().enumerate() {
            let is_sel = ti == self.term_sel;
            let mut chip = div()
                .id(SharedString::from(format!("term-tab-{ti}")))
                .h(px(28.))
                .mb(px(-1.))
                .px_3()
                .flex()
                .items_center()
                .gap_2()
                .rounded_t(px(10.))
                .cursor_pointer()
                .font_family(theme.mono_font_family.clone())
                .text_size(px(13.))
                .when(is_sel, |d| {
                    d.border_1()
                        .border_b_0()
                        .border_color(rgba(0x00000012))
                        .bg(theme.background)
                })
                .when(!is_sel, |d| {
                    d.text_color(theme.muted_foreground).hover(|d| d.bg(subtle))
                })
                .child(format!("Terminal {}", ti + 1))
                .on_click(cx.listener(move |this, _ev, _w, cx| this.term_select_tab(ti, cx)));
            if self.term_tabs.len() > 1 {
                chip = chip.child(
                    div()
                        .id(SharedString::from(format!("term-tab-x-{ti}")))
                        .size(px(14.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(3.))
                        .text_size(px(11.))
                        .text_color(theme.muted_foreground)
                        .hover(|d| d.bg(subtle))
                        .child("\u{00d7}")
                        .on_click(cx.listener(move |this, _ev: &gpui::ClickEvent, _w, cx| {
                            this.term_close_tab(ti, cx)
                        })),
                );
            }
            term_sub_strip = term_sub_strip.child(chip);
        }
        term_sub_strip = term_sub_strip
            .child(
                div()
                    .id("term-new-tab")
                    .h(px(28.))
                    .mb(px(-1.))
                    .px_2()
                    .flex()
                    .items_center()
                    .cursor_pointer()
                    .text_color(theme.muted_foreground)
                    .hover(|d| d.bg(subtle))
                    .child("+")
                    .on_click(cx.listener(|this, _ev, _w, cx| this.term_new_tab(cx))),
            )
            .child(
                div()
                    .id("term-surface-chat")
                    .h(px(28.))
                    .px_3()
                    .flex()
                    .items_center()
                    .rounded_t(px(10.))
                    .cursor_pointer()
                    .font_family(theme.mono_font_family.clone())
                    .text_size(px(13.))
                    .text_color(theme.muted_foreground)
                    .hover(|d| d.bg(subtle))
                    .child("Chat")
                    .on_click(cx.listener(|this, _ev, _w, cx| {
                        this.term_active = false;
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .id("term-search-btn")
                    .size(px(20.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.))
                    .cursor_pointer()
                    .text_color(theme.muted_foreground)
                    .hover(|d| d.bg(subtle))
                    .child(svg_icon(icons::SEARCH).size(px(12.)))
                    .on_click(cx.listener(|this, _ev, w, cx| this.toggle_term_search(w, cx))),
            )
            .child(
                div()
                    .id("term-surface-close")
                    .size(px(20.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.))
                    .cursor_pointer()
                    .text_color(theme.muted_foreground)
                    .hover(|d| d.bg(subtle))
                    .child("\u{00d7}")
                    .on_click(cx.listener(|this, _ev, w, cx| this.toggle_terminal(w, cx))),
            );
        let mut term_surface = v_flex().flex_1().min_h_0().w_full().bg(content_bg);
        term_surface = term_surface
            .child(term_sub_strip)
            .when(self.term_search_input.is_some(), |d| {
                let inp = self.term_search_input.clone().unwrap();
                let q = self.term_search_query.to_lowercase();
                let matches = if q.is_empty() {
                    0
                } else {
                    self.term_tabs
                        .get(self.term_sel)
                        .map(|t| &t.buf[..])
                        .unwrap_or("")
                        .lines()
                        .filter(|l| l.to_lowercase().contains(&q))
                        .count()
                };
                d.child(
                    h_flex()
                        .px(px(13.))
                        .py(px(4.))
                        .gap_2()
                        .items_center()
                        .border_b_1()
                        .border_color(rgba(0x00000012))
                        .child(
                            div()
                                .w(px(200.))
                                .h(px(24.))
                                .px_2()
                                .flex()
                                .items_center()
                                .rounded(px(6.))
                                .border_1()
                                .border_color(theme.border)
                                .child(Input::new(&inp).appearance(false)),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(theme.muted_foreground)
                                .child(if q.is_empty() {
                                    String::new()
                                } else {
                                    format!(
                                        "{matches} match{}",
                                        if matches == 1 { "" } else { "es" }
                                    )
                                }),
                        ),
                )
            })
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .overflow_y_scrollbar()
                    .px(px(13.))
                    .py_2()
                    .font_family(theme.mono_font_family.clone())
                    .text_xs()
                    .when(!self.term_search_query.is_empty(), |d| {
                        let q = self.term_search_query.to_lowercase();
                        let mut col = v_flex().w_full();
                        let buf: &str = self
                            .term_tabs
                            .get(self.term_sel)
                            .map(|t| &t.buf[..])
                            .unwrap_or("");
                        for line in buf.lines() {
                            let hit = line.to_lowercase().contains(&q);
                            col = col.child(
                                div()
                                    .w_full()
                                    .when(hit, |d| d.bg(theme.primary.opacity(0.14)))
                                    .child(line.to_string()),
                            );
                        }
                        d.child(col)
                    })
                    .when(self.term_search_query.is_empty(), |d| {
                        let buf = self
                            .term_tabs
                            .get(self.term_sel)
                            .map(|t| t.buf.clone())
                            .unwrap_or_default();
                        d.child(if buf.is_empty() {
                            "Starting terminal\u{2026}".to_string()
                        } else {
                            buf
                        })
                    })
                    .when_some(self.term_input.clone(), |d, inp| {
                        // The command line renders as the next terminal line —
                        // upstream has no separate input row.
                        d.child(div().pt(px(4.)).child(Input::new(&inp).appearance(false)))
                    }),
            );

        NativePanes {
            settings_pane: settings_pane.into_any(),
            tasks_pane: tasks_pane.into_any(),
            kanban_pane: kanban_pane.overflow_y_scrollbar().into_any_element(),
            pullreq_pane: pullreq_pane.into_any(),
            autom_pane: autom_pane.into_any(),
            studio_pane: studio_pane.into_any(),
            term_surface: term_surface.into_any(),
            settings_nav_el: settings_nav_el.into_any(),
            pullreq_panel: pullreq_panel.into_any(),
            autom_panel: autom_panel.into_any(),
            studio_panel: studio_panel.into_any(),
            kanban_header: kanban_header.into_any(),
            kanban_count,
        }
    }
}
