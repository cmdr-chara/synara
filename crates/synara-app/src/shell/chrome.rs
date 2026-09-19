use super::*;
use crate::ui::{self, DARK, Glyph};

impl Shell {
    fn dismiss_tools(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.navigation.menu_open = false;
        window.focus(&self.navigation.tools_focus, cx);
        cx.notify();
    }

    fn toolbar(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let project = self
            .catalog
            .projects
            .iter()
            .find(|project| Some(project.id) == self.project)
            .map_or_else(|| "Synara".to_owned(), |project| project.name.clone());
        div()
            .h(px(ui::CHROME_HEIGHT))
            .flex_shrink_0()
            .px_3()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .border_b_1()
            .border_color(rgb(DARK.border))
            .child(
                div()
                    .flex()
                    .min_w_0()
                    .items_center()
                    .gap_2()
                    .child(ui::action(
                        "sidebar-toggle",
                        "Sidebar",
                        Some(Glyph::Panel),
                        false,
                        cx.listener(|this, _: &(), _, cx| {
                            this.navigation.visible = !this.navigation.visible;
                            cx.notify();
                        }),
                    ))
                    .child(
                        div()
                            .min_w_0()
                            .text_ellipsis()
                            .text_color(rgb(DARK.muted))
                            .child(project),
                    ),
            )
            .child(
                div()
                    .tab_group()
                    .flex_shrink_0()
                    .flex()
                    .gap_1()
                    .children(
                        [
                            (Panel::Conversation, "Conversation", Glyph::Compose),
                            (Panel::Files, "Files", Glyph::Files),
                            (Panel::Changes, "Changes", Glyph::Changes),
                            (Panel::Terminal, "Terminal", Glyph::Terminal),
                        ]
                        .into_iter()
                        .map(|(panel, label, glyph)| {
                            ui::action(
                                label,
                                label,
                                Some(glyph),
                                self.panel == panel,
                                cx.listener(move |this, _: &(), _, cx| this.set_panel(panel, cx)),
                            )
                        }),
                    )
                    .child(
                        ui::action(
                            "workspace-tools",
                            "More",
                            Some(Glyph::More),
                            self.navigation.menu_open,
                            cx.listener(|this, _: &(), window, cx| {
                                if this.navigation.menu_open {
                                    this.dismiss_tools(window, cx);
                                } else {
                                    this.controls.retire();
                                    this.navigation.menu_index = 0;
                                    this.navigation.menu_open = true;
                                    window.focus(&this.navigation.menu_focus[0], cx);
                                    cx.notify();
                                }
                            }),
                        )
                        .track_focus(&self.navigation.tools_focus),
                    ),
            )
            .into_any_element()
    }

    fn tools_overlay(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        div()
            .id("tools-backdrop")
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .occlude()
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _, window, cx| this.dismiss_tools(window, cx)),
            )
            .child(
                div()
                    .id("tools-menu")
                    .role(gpui::Role::Menu)
                    .aria_label("Workspace tools")
                    .tab_group()
                    .absolute()
                    .top(px(ui::CHROME_HEIGHT - 5.0))
                    .right(px(12.0))
                    .w(px(216.0))
                    .p_2()
                    .rounded_lg()
                    .border_1()
                    .border_color(rgb(DARK.border))
                    .bg(rgb(DARK.overlay))
                    .occlude()
                    .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .children(
                        [
                            (Panel::Inspector, "Inspector"),
                            (Panel::Registry, "Agents"),
                            (Panel::Remote, "Remote"),
                        ]
                        .into_iter()
                        .enumerate()
                        .map(|(index, (panel, label))| {
                            ui::action(
                                ("tools-item", index),
                                label,
                                None,
                                self.panel == panel,
                                cx.listener(move |this, _: &(), window, cx| {
                                    this.set_panel(panel, cx);
                                    this.dismiss_tools(window, cx);
                                }),
                            )
                            .role(gpui::Role::MenuItem)
                            .track_focus(&self.navigation.menu_focus[index])
                        }),
                    ),
            )
            .into_any_element()
    }
}

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.close != CloseState::Open || self.terminal_closing {
            return self.close_panel(cx);
        }
        if !self.navigation.initialized {
            self.navigation.initialized = true;
            let root_focus = self.navigation.root_focus.clone();
            self._subscriptions.push(cx.on_focus_out(
                &root_focus,
                window,
                |this, event, window, cx| {
                    if this.close != CloseState::Open || this.terminal_closing {
                        return;
                    }
                    // A removed transient button can leave a live focus ID with
                    // no dispatch path. Restore only that retired focus (or no
                    // focus), never a deliberate move into another modal/view.
                    if window
                        .focused(cx)
                        .is_some_and(|focus| focus != event.blurred)
                    {
                        return;
                    }
                    tracing::debug!(target: "synara_ui_layout", "retired-focus-restored");
                    if this.controls.is_open() {
                        return;
                    } else if this.navigation.menu_open {
                        window.focus(&this.navigation.menu_focus[this.navigation.menu_index], cx);
                    } else if this.panel == Panel::Registry {
                        window.focus(&this.registry.query.read(cx).focus_handle(cx), cx);
                    } else {
                        window.focus(&this.navigation.root_focus, cx);
                    }
                    cx.notify();
                },
            ));
            window.focus(&root_focus, cx);
        }
        // Backend completion may request composer focus. Keep that request pending
        // while a menu owns focus, rather than stealing focus from its keyboard user.
        if self.focus_composer && !self.navigation.menu_open && !self.controls.is_open() {
            let focus = self.composer.read(cx).focus_handle(cx);
            window.focus(&focus, cx);
            self.focus_composer = false;
        }
        div()
            .id("synara-shell")
            .role(gpui::Role::Application)
            .aria_label("Synara")
            .track_focus(&self.navigation.root_focus)
            .tab_group()
            .tab_stop(false)
            .relative()
            .size_full()
            .capture_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                let modifiers = event.keystroke.modifiers;
                let key = event.keystroke.key.as_str();
                if this.navigation.menu_open {
                    if key == "escape" {
                        this.dismiss_tools(window, cx);
                        cx.stop_propagation();
                        return;
                    }
                    if !modifiers.control
                        && !modifiers.platform
                        && !modifiers.alt
                        && matches!(key, "tab" | "up" | "down")
                    {
                        let backwards = key == "up" || (key == "tab" && modifiers.shift);
                        this.navigation.menu_index =
                            (this.navigation.menu_index + if backwards { 2 } else { 1 }) % 3;
                        window.focus(&this.navigation.menu_focus[this.navigation.menu_index], cx);
                        cx.stop_propagation();
                        return;
                    }
                }
                if (modifiers.control || modifiers.platform) && !modifiers.alt && !modifiers.shift {
                    let panel = match key {
                        "1" => Some(Panel::Conversation),
                        "2" => Some(Panel::Files),
                        "3" => Some(Panel::Changes),
                        "4" => Some(Panel::Terminal),
                        "5" => Some(Panel::Inspector),
                        "6" => Some(Panel::Settings),
                        "7" => Some(Panel::Registry),
                        "8" => Some(Panel::Remote),
                        _ => None,
                    };
                    if let Some(panel) = panel {
                        this.set_panel(panel, cx);
                        if this.navigation.menu_open {
                            this.dismiss_tools(window, cx);
                        }
                        cx.stop_propagation();
                    }
                }
            }))
            // Bubble, do not capture: editor/terminal Tab input keeps its existing owner.
            .on_key_down(|event: &gpui::KeyDownEvent, window, cx| {
                let modifiers = event.keystroke.modifiers;
                if event.keystroke.key == "tab"
                    && !modifiers.control
                    && !modifiers.platform
                    && !modifiers.alt
                {
                    if modifiers.shift {
                        window.focus_prev(cx);
                    } else {
                        window.focus_next(cx);
                    }
                    cx.stop_propagation();
                }
            })
            .flex()
            .flex_col()
            .bg(rgb(DARK.canvas))
            .text_color(rgb(DARK.text))
            .font_family(ui::UI_FONT)
            .text_sm()
            .child(self.toolbar(cx))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .children(self.navigation.visible.then(|| self.sidebar(cx)))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .children(self.error.as_ref().map(|error| {
                                div()
                                    .px_4()
                                    .py_2()
                                    .bg(rgb(DARK.error_surface))
                                    .text_color(rgb(DARK.error))
                                    .child(error.clone())
                            }))
                            .children(self.notice.as_ref().map(|notice| {
                                div()
                                    .px_4()
                                    .py_2()
                                    .bg(rgb(DARK.notice_surface))
                                    .child(notice.clone())
                            }))
                            .child(match self.panel {
                                Panel::Conversation => self.conversation(window, cx),
                                Panel::Files => self.files_panel(cx),
                                Panel::Changes => self.git_panel(cx),
                                Panel::Terminal => self.terminal_panel(cx),
                                Panel::Inspector => self.inspector_panel(cx),
                                Panel::Settings => self.settings_panel(cx),
                                Panel::Registry => self.registry_panel(cx),
                                Panel::Remote => self.remote_panel(cx),
                            }),
                    ),
            )
            .child(
                div()
                    .h(px(ui::STATUS_HEIGHT))
                    .flex_shrink_0()
                    .px_4()
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_t_1()
                    .border_color(rgb(DARK.border))
                    .text_xs()
                    .text_color(rgb(DARK.muted))
                    .child(
                        div()
                            .min_w_0()
                            .text_ellipsis()
                            .child(self.root().map_or_else(
                                || "No workspace selected".into(),
                                |path| path.display().to_string(),
                            )),
                    )
                    .child(self.details.as_ref().map_or_else(
                        || "ACP · disconnected".into(),
                        |details| format!("ACP · {:?}", details.connection.state),
                    )),
            )
            .children(self.navigation.menu_open.then(|| self.tools_overlay(cx)))
            .children(self.controls.is_open().then(|| self.control_overlay(cx)))
            .into_any_element()
    }
}
