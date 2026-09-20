use super::*;
use crate::ui::{self, Glyph, palette};
use gpui::AnimationExt;

impl Shell {
    fn dismiss_tools(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.navigation.menu_open = false;
        window.focus(&self.navigation.brand_focus, cx);
        cx.notify();
    }

    fn toolbar(
        &self,
        sidebar_fraction: f32,
        dock_width: f32,
        viewport_width: f32,
        maximized: bool,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let docked = dock_width > 0.;
        let navigation_width = (ui::SIDEBAR_WIDTH * sidebar_fraction).max(112.);
        let has_chat = !self.environment.maximized
            && (self.panel == Panel::Conversation || docked)
            && self
                .thread
                .as_ref()
                .is_some_and(|thread| !thread.timeline.is_empty());
        let title = self
            .task()
            .map_or("New thread", |task| task.title.as_str())
            .to_owned();
        div()
            .id("window-toolbar")
            .relative()
            .h(px(ui::CHROME_HEIGHT))
            .flex_shrink_0()
            .flex()
            .items_center()
            .border_b_1()
            .border_color(gpui::rgba(0xffffff06))
            .child(
                div()
                    .w(px(navigation_width))
                    .flex_shrink_0()
                    .px_3()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(ui::chrome_button(
                        "sidebar-toggle",
                        "Toggle sidebar",
                        Glyph::Panel,
                        false,
                        cx.listener(|this, _: &(), _, cx| this.toggle_sidebar(cx)),
                    ))
                    .child(ui::chrome_button(
                        "history-back",
                        "Go back",
                        Glyph::Back,
                        self.navigation.history_index == 0,
                        cx.listener(|this, _: &(), _, cx| this.history_back(true, cx)),
                    ))
                    .child(ui::chrome_button(
                        "history-forward",
                        "Go forward",
                        Glyph::Forward,
                        self.navigation.history_index + 1 >= self.navigation.history.len(),
                        cx.listener(|this, _: &(), _, cx| this.history_back(false, cx)),
                    )),
            )
            .child(
                div()
                    .w(px((viewport_width - navigation_width - dock_width).max(0.)))
                    .flex_shrink_0()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .items_center()
                    .pl_4()
                    .pr_2()
                    .gap_2()
                    .when(!docked && !cfg!(target_os = "macos"), |el| el.pr(px(152.)))
                    .when(docked && self.environment.maximized, |el| {
                        el.px_0().overflow_hidden()
                    })
                    .children(
                        (self.panel == Panel::Kanban && self.kanban.project.is_some()).then(|| {
                            ui::chrome_button(
                                "kanban-all-projects",
                                "All projects",
                                Glyph::Back,
                                false,
                                cx.listener(|this, _: &(), _, cx| this.kanban_back(cx)),
                            )
                        }),
                    )
                    .child(
                        div()
                            .id("window-drag-region")
                            .h_full()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap_2()
                            .on_mouse_down(gpui::MouseButton::Left, |event, window, _| {
                                if event.click_count == 2 {
                                    window.zoom_window();
                                } else {
                                    window.start_window_move();
                                }
                            })
                            .children((self.panel == Panel::Kanban).then(|| {
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .text_size(px(14.))
                                    .child(self.kanban_heading())
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(rgb(palette().muted))
                                            .child(format!("{} tasks", self.kanban_count())),
                                    )
                            }))
                            .children(has_chat.then(|| ui::icon(self.selected_agent_glyph())))
                            .children(has_chat.then(|| {
                                div()
                                    .min_w_0()
                                    .text_ellipsis()
                                    .text_size(px(15.))
                                    .child(title)
                            })),
                    )
                    .children((self.panel == Panel::Kanban).then(|| {
                        ui::action(
                            "kanban-new-task",
                            "New task",
                            Some(Glyph::Plus),
                            false,
                            cx.listener(|this, _: &(), _, cx| this.open_task_dialog(false, cx)),
                        )
                        .relative()
                        .child(ui::layout_probe("kanban-new-task"))
                    }))
                    .children(has_chat.then(|| {
                        ui::unavailable_action(
                            "handoff",
                            if docked { "" } else { "Hand off" },
                            Glyph::Handoff,
                            "Agent handoff is not available in this native build yet.",
                        )
                        .aria_label("Hand off, unavailable")
                    }))
                    .children(
                        (!matches!(self.panel, Panel::Settings | Panel::Kanban)).then(|| {
                            ui::chrome_button(
                                "Terminal",
                                "Terminal",
                                Glyph::Dock,
                                false,
                                cx.listener(|this, _: &(), _, cx| {
                                    if this.panel == Panel::Terminal {
                                        this.hide_environment(cx);
                                    } else {
                                        this.set_panel(Panel::Terminal, cx);
                                    }
                                }),
                            )
                        }),
                    )
                    .children(
                        (!matches!(self.panel, Panel::Settings | Panel::Kanban)).then(|| {
                            ui::chrome_button(
                                "Files",
                                "Toggle workspace pane",
                                Glyph::PanelRight,
                                false,
                                cx.listener(|this, _: &(), _, cx| {
                                    if this.dock_open() {
                                        this.hide_environment(cx);
                                    } else {
                                        this.set_panel(Panel::Dock, cx);
                                    }
                                }),
                            )
                            .when(docked, |el| el.bg(rgb(palette().overlay)))
                        }),
                    ),
            )
            .children(docked.then(|| {
                div()
                    .w(px(dock_width))
                    .flex_shrink_0()
                    .min_w_0()
                    .h_full()
                    .border_l_1()
                    .border_color(rgb(palette().border))
                    .pl_2()
                    .pr(px(if cfg!(target_os = "macos") { 8. } else { 144. }))
                    .child(self.environment_header(dock_width < 560., cx))
            }))
            .children((!cfg!(target_os = "macos")).then(|| {
                div()
                    .absolute()
                    .right_1()
                    .top(px(8.))
                    .flex()
                    .gap_2()
                    .child(
                        ui::chrome_button(
                            "window-minimize",
                            "Minimize",
                            Glyph::Minimize,
                            false,
                            |_, window, _| window.minimize_window(),
                        )
                        .w(px(38.)),
                    )
                    .child(
                        ui::chrome_button(
                            "window-maximize",
                            "Maximize or restore",
                            if maximized {
                                Glyph::Restore
                            } else {
                                Glyph::Maximize
                            },
                            false,
                            |_, window, _| window.zoom_window(),
                        )
                        .w(px(38.)),
                    )
                    .child(
                        ui::chrome_button(
                            "window-close",
                            "Close window",
                            Glyph::Close,
                            false,
                            cx.listener(|this, _: &(), window, cx| {
                                this.request_close(window, cx);
                            }),
                        )
                        .w(px(38.)),
                    )
            }))
            .into_any_element()
    }

    fn tools_overlay(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        div()
            .id("mode-backdrop")
            .absolute()
            .inset_0()
            .occlude()
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _, window, cx| this.dismiss_tools(window, cx)),
            )
            .child(
                div()
                    .id("mode-switcher")
                    .role(gpui::Role::Menu)
                    .aria_label("Synara and Studio")
                    .tab_group()
                    .absolute()
                    .top(px(ui::CHROME_HEIGHT + 36.))
                    .left(px(6.))
                    .w(px(256.))
                    .p_1()
                    .rounded_2xl()
                    .border_1()
                    .border_color(rgb(palette().border))
                    .bg(rgb(palette().overlay))
                    .occlude()
                    .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .children(
                        [
                            (false, "Synara", "Build, debug, and ship"),
                            (true, "Studio", "Open-ended agent work"),
                        ]
                        .into_iter()
                        .enumerate()
                        .filter(|(_, (studio, _, _))| {
                            !*studio || self.settings.value.general.show_studio
                        })
                        .map(|(index, (studio, label, detail))| {
                            ui::button_shell(
                                ("mode-choice", index),
                                label,
                                self.navigation.studio == studio,
                            )
                            .role(gpui::Role::MenuItem)
                            .track_focus(&self.navigation.menu_focus[index])
                            .w_full()
                            .h(px(48.))
                            .px_3()
                            .py_2()
                            .rounded_xl()
                            .border_0()
                            .when(self.navigation.studio != studio, |row| {
                                row.bg(gpui::rgba(0))
                            })
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.))
                                    .text_size(px(13.))
                                    .child(label)
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(rgb(palette().muted))
                                            .child(detail),
                                    ),
                            )
                            .children(
                                (self.navigation.studio == studio)
                                    .then(|| ui::icon(Glyph::Check).size(px(14.))),
                            )
                            .relative()
                            .child(ui::layout_probe_slot("mode-choice", index))
                            .on_click(cx.listener(
                                move |this, _, window, cx| {
                                    this.switch_mode(studio, cx);
                                    window.focus(&this.navigation.brand_focus, cx);
                                },
                            ))
                        }),
                    ),
            )
            .map(|menu| {
                if cx.reduce_motion() {
                    menu.into_any_element()
                } else {
                    menu.with_animation(
                        "mode-switcher-entry",
                        gpui::Animation::new(std::time::Duration::from_millis(150))
                            .with_easing(ui::motion::ease_out),
                        |element, delta| element.opacity(delta),
                    )
                    .into_any_element()
                }
            })
            .into_any_element()
    }
}

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        ui::configure(&self.settings.value.appearance, window.appearance());
        if self.draft_state.quitting || self.environment.quitting {
            window.focus(&self.close_focus, cx);
            return self.draft_close_panel(cx);
        }
        if self.close != CloseState::Open || self.terminal_closing {
            return self.close_panel(cx);
        }
        if !self.navigation.initialized {
            self.navigation.initialized = true;
            let weak = cx.entity().downgrade();
            self._subscriptions
                .push(window.observe_window_appearance(move |_, cx| {
                    weak.update(cx, |_, cx| cx.notify()).ok();
                }));
            let root_focus = self.navigation.root_focus.clone();
            self._subscriptions.push(cx.on_focus_out(
                &root_focus,
                window,
                |this, event, window, cx| {
                    if this.close != CloseState::Open
                        || this.terminal_closing
                        || this.draft_state.quitting
                        || this.environment.quitting
                    {
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
                    if this.controls.is_open()
                        || this.environment.menu_open()
                        || this.chat_tools.menu_open()
                    {
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
        if self.focus_composer
            && !self.navigation.menu_open
            && !self.controls.is_open()
            && self.kanban.dialog.is_none()
            && self.organization.dialog.is_none()
            && !self.environment.menu_open()
            && !self.chat_tools.menu_open()
            && !self.chat_tools.find_open
            && !(self.dock_open() && self.environment.maximized)
        {
            let focus = self.composer.read(cx).focus_handle(cx);
            window.focus(&focus, cx);
            self.focus_composer = false;
        }
        self.consume_chat_action(window, cx);
        self.restore_environment_focus(window, cx);
        self.restore_organization_focus(window, cx);
        let now = std::time::Instant::now();
        if !cx.reduce_motion() && self.transcript.advance_animations(now) {
            window.request_animation_frame();
        }
        let sidebar_fraction = if cx.reduce_motion() {
            if self.navigation.visible { 1.0 } else { 0.0 }
        } else {
            if self.navigation.drawer.running(now) {
                window.request_animation_frame();
            }
            self.navigation.drawer.value(now)
        };
        let dock_open = self.dock_open();
        if dock_open {
            self.dock_panel = self.panel;
        }
        if self.dock_motion.target_open() != dock_open {
            self.dock_motion
                .set_open(dock_open, now, cx.reduce_motion());
        }
        let dock_fraction = if self.panel != Panel::Conversation && !dock_open {
            0.
        } else {
            self.dock_motion.value(now)
        };
        if self.dock_motion.running(now) {
            cx.on_next_frame(window, |_, _, cx| cx.notify());
        }
        let viewport_width = f32::from(window.viewport_size().width);
        let available_width = (viewport_width - ui::SIDEBAR_WIDTH * sidebar_fraction).max(0.);
        let dock_target_width = environment::split_width(
            available_width,
            self.environment.value.width_ratio,
            self.environment.maximized && dock_open,
        );
        let dock_width = dock_target_width * dock_fraction;
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
                if this.kanban.dialog.is_some() || this.organization.dialog.is_some() {
                    return;
                }
                if this.organization_shortcut(event, window, cx) {
                    cx.stop_propagation();
                    return;
                }
                if this.chat_tools_shortcut(event, window, cx) {
                    cx.stop_propagation();
                    return;
                }
                if this.panel == Panel::Kanban
                    && key == "t"
                    && modifiers.alt
                    && (modifiers.control || modifiers.platform)
                    && !modifiers.shift
                    && !event.is_held
                {
                    this.open_task_dialog(false, cx);
                    cx.stop_propagation();
                    return;
                }
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
                        this.navigation.menu_index = (this.navigation.menu_index + 1)
                            % if this.settings.value.general.show_studio {
                                2
                            } else {
                                1
                            };
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
                        "9" => Some(Panel::Kanban),
                        _ => None,
                    };
                    if let Some(panel) = panel {
                        if panel == Panel::Conversation && this.dock_open() {
                            this.hide_environment(cx);
                        } else {
                            this.set_panel(panel, cx);
                        }
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
            .bg(rgb(palette().canvas))
            .text_color(rgb(palette().text))
            .font_family(ui::ui_font())
            .text_sm()
            .child(self.toolbar(
                sidebar_fraction,
                dock_width,
                viewport_width,
                window.is_maximized(),
                cx,
            ))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .children((sidebar_fraction > 0.0).then(|| {
                        div()
                            .id("sidebar-drawer")
                            .relative()
                            .flex_shrink_0()
                            .min_h_0()
                            .h_full()
                            .w(px(ui::SIDEBAR_WIDTH * sidebar_fraction))
                            .overflow_hidden()
                            .child(
                                div()
                                    .absolute()
                                    .top_0()
                                    .bottom_0()
                                    .left(px(ui::SIDEBAR_WIDTH * (sidebar_fraction - 1.0)))
                                    .w(px(ui::SIDEBAR_WIDTH))
                                    .child(if self.panel == Panel::Settings {
                                        self.settings_sidebar(cx)
                                    } else {
                                        self.sidebar(cx)
                                    }),
                            )
                            .child(ui::layout_probe("sidebar-drawer"))
                    }))
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
                                    .bg(rgb(palette().error_surface))
                                    .text_color(rgb(palette().error))
                                    .child(error.clone())
                            }))
                            .children(self.notice.as_ref().map(|notice| {
                                div()
                                    .px_4()
                                    .py_2()
                                    .bg(rgb(palette().notice_surface))
                                    .child(notice.clone())
                            }))
                            .child(self.main_surface(
                                window,
                                dock_width,
                                dock_target_width,
                                available_width,
                                cx,
                            )),
                    ),
            )
            .children(
                self.environment
                    .menu_open()
                    .then(|| self.environment_overlay(cx)),
            )
            .children(
                self.environment
                    .resizing()
                    .then(|| self.environment_drag_overlay(cx)),
            )
            .children(
                self.chat_tools
                    .menu_open()
                    .then(|| self.chat_tools_overlay(cx)),
            )
            .children(self.kanban.dialog.clone())
            .children(self.organization.dialog.clone())
            .children(self.navigation.menu_open.then(|| self.tools_overlay(cx)))
            .children(self.controls.is_open().then(|| self.control_overlay(cx)))
            .children(
                self.settings
                    .popup
                    .is_some()
                    .then(|| self.settings_overlay(cx)),
            )
            .into_any_element()
    }
}
