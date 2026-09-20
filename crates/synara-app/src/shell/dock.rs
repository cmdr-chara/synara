use super::*;
use crate::ui::{self, Glyph, palette};

impl Shell {
    pub(super) fn dock_open(&self) -> bool {
        matches!(
            self.panel,
            Panel::Dock | Panel::Files | Panel::Terminal | Panel::Changes
        )
    }

    pub(super) fn main_surface(
        &self,
        window: &Window,
        dock_width: f32,
        target_width: f32,
        available_width: f32,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        if self.dock_open() || dock_width > 0. {
            return div()
                .flex()
                .flex_1()
                .min_h_0()
                .min_w_0()
                .children((!self.environment.maximized || !self.dock_open()).then(|| {
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w_0()
                        .min_h_0()
                        .relative()
                        .child(ui::layout_probe("chat-pane"))
                        .child(self.conversation(window, cx))
                }))
                .child(
                    div()
                        .w(px(dock_width))
                        .flex_shrink_0()
                        .h_full()
                        .relative()
                        .child(ui::layout_probe("workspace-pane"))
                        .child(
                            div().w_full().h_full().overflow_hidden().child(
                                div()
                                    .w(px(target_width))
                                    .h_full()
                                    .min_h_0()
                                    .flex()
                                    .flex_col()
                                    .border_l_1()
                                    .border_color(rgb(palette().border))
                                    .child(match self.dock_panel {
                                        Panel::Files => self.files_panel(cx),
                                        Panel::Terminal => self.terminal_panel(cx),
                                        Panel::Changes => self.git_panel(target_width, cx),
                                        _ => self.dock_launcher(cx),
                                    }),
                            ),
                        )
                        .children(
                            (self.dock_open() && !self.environment.maximized)
                                .then(|| self.environment_divider(available_width, cx)),
                        ),
                )
                .into_any_element();
        }
        match self.panel {
            Panel::Conversation => self.conversation(window, cx),
            Panel::Kanban => self.kanban_panel(cx),
            Panel::Help => self.help_panel(),
            Panel::Inspector => self.inspector_panel(cx),
            Panel::Settings => self.settings_panel(cx),
            Panel::Registry => self.registry_panel(cx),
            Panel::Remote => self.remote_panel(cx),
            _ => unreachable!("workspace panels are rendered alongside the conversation"),
        }
    }

    fn dock_launcher(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .justify_center()
            .items_center()
            .px_6()
            .child(
                div()
                    .w_full()
                    .max_w(px(384.))
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(
                        ui::action(
                            "dock-terminal",
                            "Terminal",
                            Some(Glyph::Terminal),
                            false,
                            cx.listener(|this, _: &(), _, cx| this.set_panel(Panel::Terminal, cx)),
                        )
                        .h(px(40.))
                        .px(px(20.))
                        .gap_3()
                        .rounded_xl()
                        .bg(rgb(palette().overlay)),
                    )
                    .child(
                        ui::unavailable_action(
                            "dock-browser",
                            "Browser",
                            Glyph::Browser,
                            "An embedded browser is not available in this native build yet.",
                        )
                        .h(px(40.))
                        .px(px(20.))
                        .gap_3()
                        .rounded_xl()
                        .bg(rgb(palette().overlay)),
                    )
                    .child(
                        ui::action(
                            "dock-files",
                            "Files",
                            Some(Glyph::Folders),
                            false,
                            cx.listener(|this, _: &(), _, cx| this.set_panel(Panel::Files, cx)),
                        )
                        .h(px(40.))
                        .px(px(20.))
                        .gap_3()
                        .rounded_xl()
                        .bg(rgb(palette().overlay))
                        .relative()
                        .child(ui::layout_probe("dock-files")),
                    )
                    .child(
                        ui::unavailable_action(
                            "dock-side-chats",
                            "Side chats",
                            Glyph::Chat,
                            "Side chats are not available in this native build yet.",
                        )
                        .h(px(40.))
                        .px(px(20.))
                        .gap_3()
                        .rounded_xl()
                        .bg(rgb(palette().overlay)),
                    ),
            )
            .into_any_element()
    }
}
