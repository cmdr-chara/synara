//! Shared Environment chrome. Tabs select existing native services, never launch
//! tools. Layout saves are coalesced and serialized independently of settings.
use super::*;
use crate::ui::{
    self, Glyph,
    menu::{Choice, ChoiceEvent, ChoiceMenu},
    palette,
};
use gpui::{FocusHandle, MouseButton, Pixels, Point};

struct Popup {
    view: Entity<ChoiceMenu>,
    position: Point<Pixels>,
    _subscription: Subscription,
}
#[derive(Default)]
struct SaveState {
    dirty: bool,
    saving: bool,
    failed: bool,
}
impl SaveState {
    fn changed(&mut self) {
        self.dirty = true;
        self.failed = false;
    }
    fn start(&mut self, force: bool) -> bool {
        if !self.dirty || self.saving || (self.failed && !force) {
            return false;
        }
        self.dirty = false;
        self.saving = true;
        true
    }
    fn finish(&mut self, failed: bool) {
        self.saving = false;
        self.failed = failed;
        self.dirty |= failed;
    }
    fn pending(&self) -> bool {
        self.dirty || self.saving
    }
}
#[derive(Clone, Copy)]
struct Resize {
    origin: f32,
    ratio: f32,
    available: f32,
}
pub(super) struct EnvironmentState {
    pub value: EnvironmentLayout,
    pub recovery: Option<String>,
    save: SaveState,
    pub quitting: bool,
    pub maximized: bool,
    resize: Option<Resize>,
    popup: Option<Popup>,
    add_focus: FocusHandle,
    add_bounds: std::rc::Rc<std::cell::Cell<gpui::Bounds<Pixels>>>,
    restore_focus: Option<Option<EnvironmentTab>>,
    divider_focus: FocusHandle,
    tabs_focus: [FocusHandle; 3],
}
impl EnvironmentState {
    pub(super) fn new(loaded: LoadedEnvironmentLayout, cx: &mut Context<Shell>) -> Self {
        Self {
            value: loaded.layout,
            recovery: loaded.recovery,
            save: SaveState::default(),
            quitting: false,
            maximized: false,
            resize: None,
            popup: None,
            add_focus: cx.focus_handle(),
            add_bounds: std::rc::Rc::new(std::cell::Cell::new(gpui::Bounds::default())),
            restore_focus: None,
            divider_focus: cx.focus_handle(),
            tabs_focus: std::array::from_fn(|_| cx.focus_handle()),
        }
    }
    pub(super) fn menu_open(&self) -> bool {
        self.popup.is_some()
    }
    pub(super) fn retire_popup(&mut self) {
        self.popup = None;
    }
    pub(super) fn resizing(&self) -> bool {
        self.resize.is_some()
    }
}
fn tab_panel(tab: EnvironmentTab) -> Panel {
    match tab {
        EnvironmentTab::Terminal => Panel::Terminal,
        EnvironmentTab::Explorer => Panel::Files,
        EnvironmentTab::Changes => Panel::Changes,
    }
}
fn panel_tab(panel: Panel) -> Option<EnvironmentTab> {
    match panel {
        Panel::Terminal => Some(EnvironmentTab::Terminal),
        Panel::Files => Some(EnvironmentTab::Explorer),
        Panel::Changes => Some(EnvironmentTab::Changes),
        _ => None,
    }
}
fn tab_info(tab: EnvironmentTab) -> (&'static str, Glyph, usize) {
    match tab {
        EnvironmentTab::Terminal => ("Terminal", Glyph::Terminal, 0),
        EnvironmentTab::Explorer => ("Explorer", Glyph::Folders, 1),
        EnvironmentTab::Changes => ("Changes", Glyph::BranchSimple, 2),
    }
}
/// Preserve usable panes at intermediate widths without overwriting the saved
/// preference simply because the current window is narrower than before.
pub(super) fn split_width(available: f32, ratio: f32, maximized: bool) -> f32 {
    let available = if available.is_finite() {
        available.max(0.)
    } else {
        0.
    };
    if maximized {
        return available;
    }
    let minimum = 320_f32.min(available * 0.5);
    (available * ratio.clamp(0.2, 0.8)).clamp(minimum, available - minimum)
}
fn resized_ratio(start: f32, delta: f32, available: f32) -> f32 {
    if available <= 0. || !available.is_finite() || !delta.is_finite() {
        return start;
    }
    (start - delta / available).clamp(0.2, 0.8)
}
fn tab_close_blocked(
    tab: EnvironmentTab,
    dirty: bool,
    saving: bool,
    starting: bool,
    running: bool,
) -> Option<&'static str> {
    match tab {
        EnvironmentTab::Explorer if saving => {
            Some("Wait for the file save to finish before closing Explorer.")
        }
        EnvironmentTab::Explorer if dirty => {
            Some("Save or discard the edited file in Explorer before closing its tab.")
        }
        EnvironmentTab::Terminal if starting => {
            Some("Wait for shell startup to finish before closing Terminal.")
        }
        EnvironmentTab::Terminal if running => Some(
            "Stop the running shell before closing its tab. Closing never silently kills a process.",
        ),
        _ => None,
    }
}
impl Shell {
    /// Called only for explicit panel navigation, not as a side effect of render.
    pub(super) fn track_environment_panel(&mut self, panel: Panel) -> Panel {
        if !matches!(
            panel,
            Panel::Dock | Panel::Terminal | Panel::Files | Panel::Changes
        ) {
            self.environment.resize = None;
            return panel;
        }
        let before = self.environment.value.clone();
        if let Some(tab) = panel_tab(panel) {
            self.environment.value.select(tab);
        }
        self.environment.value.open_by_default = true;
        if self.environment.value != before {
            self.environment.save.changed();
        }
        let target = if panel == Panel::Dock {
            self.environment
                .value
                .active
                .map(tab_panel)
                .unwrap_or(Panel::Dock)
        } else {
            panel
        };
        self.dock_panel = target;
        target
    }
    /// Navigation into a chat honors the remembered preference. Studio remains
    /// deliberate: its independent workspace does not auto-open this pane.
    pub(super) fn show_conversation(&mut self, cx: &mut Context<Self>) {
        self.environment.maximized = false;
        let panel = if self.environment.value.open_by_default
            && self
                .task()
                .is_some_and(|task| task.scope != TaskScope::Studio)
        {
            self.environment
                .value
                .active
                .map(tab_panel)
                .unwrap_or(Panel::Dock)
        } else {
            Panel::Conversation
        };
        self.set_panel(panel, cx);
        self.focus_composer = true;
    }
    pub(super) fn hide_environment(&mut self, cx: &mut Context<Self>) {
        self.environment.maximized = false;
        self.environment.resize = None;
        self.environment.value.open_by_default = false;
        self.environment.save.changed();
        self.set_panel(Panel::Conversation, cx);
        self.focus_composer = true;
    }
    pub(super) fn flush_environment(&mut self, force: bool) {
        if self.environment.recovery.is_some()
            || self.environment.resizing()
            || !self.environment.save.start(force)
        {
            return;
        }
        let value = self.environment.value.clone();
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::EnvironmentSaved(
                workspace
                    .save_environment_layout(value)
                    .await
                    .err()
                    .map(|error| error.to_string()),
            ))
        });
    }
    pub(super) fn environment_saved(&mut self, error: Option<String>, cx: &mut Context<Self>) {
        self.environment.save.finish(error.is_some());
        if let Some(error) = error {
            self.error = Some(format!(
                "Environment layout could not be saved: {error}. Your tools and chat are still open."
            ));
            self.environment.quitting = false;
            self.close.cancel();
        } else if self.environment.quitting {
            self.flush_environment(true);
            if !self.environment.save.pending() {
                self.environment.quitting = false;
                self.begin_quit(cx);
            }
        }
        cx.notify();
    }
    pub(super) fn save_environment_before_quit(&mut self) -> bool {
        self.environment.resize = None;
        if self.environment.recovery.is_some() || !self.environment.save.pending() {
            return false;
        }
        self.environment.quitting = true;
        self.flush_environment(true);
        true
    }
    pub(super) fn reset_environment_layout(&mut self, cx: &mut Context<Self>) {
        self.environment.value = EnvironmentLayout::default();
        self.environment.recovery = None;
        self.environment.maximized = false;
        self.environment.save.changed();
        // Only chrome is reset. Keep editor contents, terminal and drafts owned.
        if self.dock_open() {
            self.panel = Panel::Conversation;
            self.dock_panel = Panel::Dock;
            self.focus_composer = true;
        }
        cx.notify();
    }
    pub(super) fn environment_preference_toggle(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let enabled = self.environment.value.open_by_default;
        ui::button_shell(
            "environment-default",
            "Open Environment by default",
            enabled,
        )
        .role(gpui::Role::Switch)
        .aria_label("Open Environment by default")
        .aria_toggled(if enabled {
            gpui::Toggled::True
        } else {
            gpui::Toggled::False
        })
        .w(px(32.))
        .h(px(20.))
        .p(px(2.))
        .rounded_full()
        .border_0()
        .bg(rgb(if enabled {
            palette().focus
        } else {
            palette().border
        }))
        .flex()
        .items_center()
        .when(enabled, |el| el.justify_end())
        .child(div().size(px(16.)).rounded_full().bg(rgb(palette().text)))
        .relative()
        .child(ui::layout_probe("environment-default"))
        .on_click(cx.listener(|this, _, _, cx| {
            this.environment.value.open_by_default = !this.environment.value.open_by_default;
            this.environment.save.changed();
            cx.notify();
        }))
        .into_any_element()
    }
    pub(super) fn reset_environment_default(&mut self) {
        self.environment.value.open_by_default = false;
        self.environment.save.changed();
    }
    fn select_environment_tab(
        &mut self,
        tab: EnvironmentTab,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.focus_composer = false;
        self.set_panel(tab_panel(tab), cx);
        window.focus(&self.environment.tabs_focus[tab_info(tab).2], cx);
    }
    fn close_environment_tab(
        &mut self,
        tab: EnvironmentTab,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.close != CloseState::Open
            || self.terminal_closing
            || !self.environment.value.tabs.contains(&tab)
        {
            return;
        }
        let running = self.terminal.is_some() && self.terminal_view.read(cx).exit_code().is_none();
        if let Some(reason) = tab_close_blocked(
            tab,
            self.dirty(cx),
            self.saving,
            self.terminal_starting,
            running,
        ) {
            self.select_environment_tab(tab, window, cx);
            self.notice = Some(reason.into());
            cx.notify();
            return;
        }
        if self.environment.value.close(tab) {
            self.environment.save.changed();
            self.environment.retire_popup();
            if let Some(active) = self.environment.value.active {
                self.select_environment_tab(active, window, cx);
            } else {
                self.focus_composer = false;
                self.set_panel(Panel::Dock, cx);
                window.focus(&self.environment.add_focus, cx);
            }
            cx.notify();
        }
    }
    fn move_environment_tab(
        &mut self,
        tab: EnvironmentTab,
        backwards: bool,
        cx: &mut Context<Self>,
    ) {
        if self.environment.value.move_tab(tab, backwards) {
            self.environment.save.changed();
            cx.notify();
        }
    }
    fn environment_tab_key(
        &mut self,
        tab: EnvironmentTab,
        event: &gpui::KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let modifiers = event.keystroke.modifiers;
        if modifiers.control || modifiers.platform || event.prefer_character_input {
            return;
        }
        if modifiers.alt {
            if !modifiers.shift && matches!(event.keystroke.key.as_str(), "left" | "right") {
                self.move_environment_tab(tab, event.keystroke.key == "left", cx);
                cx.stop_propagation();
            }
            return;
        }
        if event.keystroke.key == "delete" && !modifiers.shift {
            if !event.is_held {
                self.close_environment_tab(tab, window, cx);
            }
            cx.stop_propagation();
            return;
        }
        let tabs = &self.environment.value.tabs;
        let Some(index) = tabs.iter().position(|candidate| *candidate == tab) else {
            return;
        };
        let next = match event.keystroke.key.as_str() {
            "left" => (index + tabs.len() - 1) % tabs.len(),
            "right" => (index + 1) % tabs.len(),
            "home" => 0,
            "end" => tabs.len() - 1,
            _ => return,
        };
        self.select_environment_tab(tabs[next], window, cx);
        cx.stop_propagation();
    }
    fn open_environment_menu(
        &mut self,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.controls.retire();
        self.settings.popup = None;
        self.focus_composer = false;
        let view = cx.new(|cx| {
            ChoiceMenu::new(
                "Environment".into(),
                vec![
                    Choice {
                        label: "Terminal".into(),
                        icon: Some(Glyph::Terminal),
                        detail: "Open the native terminal. Starting a shell is a separate action."
                            .into(),
                        ..Default::default()
                    },
                    Choice {
                        label: "Browser".into(),
                        icon: Some(Glyph::Browser),
                        unavailable: Some(
                            "Embedded browsing is not available in this native build yet.".into(),
                        ),
                        ..Default::default()
                    },
                    Choice {
                        label: "Explorer".into(),
                        icon: Some(Glyph::Folders),
                        detail: "Browse and edit files in the selected workspace.".into(),
                        ..Default::default()
                    },
                    Choice {
                        label: "Side chats".into(),
                        icon: Some(Glyph::Chat),
                        unavailable: Some(
                            "Side chats are not available in this native build yet.".into(),
                        ),
                        ..Default::default()
                    },
                    Choice {
                        label: "Changes".into(),
                        icon: Some(Glyph::BranchSimple),
                        detail: "Review local Git changes and staged files.".into(),
                        ..Default::default()
                    },
                ],
                cx,
            )
        });
        let subscription = cx.subscribe(&view, |this, _, event, cx| {
            this.environment.popup = None;
            this.environment.restore_focus = Some(None);
            if let ChoiceEvent::Selected(index) = event {
                let tab = match index {
                    0 => Some(EnvironmentTab::Terminal),
                    2 => Some(EnvironmentTab::Explorer),
                    4 => Some(EnvironmentTab::Changes),
                    _ => None,
                };
                if let Some(tab) = tab {
                    this.environment.restore_focus = Some(Some(tab));
                    this.set_panel(tab_panel(tab), cx);
                }
            }
            cx.notify();
        });
        window.focus(&view.read(cx).focus_handle(cx), cx);
        self.environment.popup = Some(Popup {
            view,
            position,
            _subscription: subscription,
        });
        cx.notify();
    }
    pub(super) fn restore_environment_focus(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.environment.menu_open()
            && let Some(tab) = self.environment.restore_focus.take()
            && self.dock_open()
        {
            let focus = tab
                .map(|tab| &self.environment.tabs_focus[tab_info(tab).2])
                .unwrap_or(&self.environment.add_focus);
            window.focus(focus, cx);
        }
    }
    pub(super) fn environment_header(
        &self,
        compact: bool,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let add_bounds = self.environment.add_bounds.clone();
        div()
            .id("environment-toolbar")
            .relative()
            .h_full()
            .w_full()
            .flex()
            .items_center()
            .gap_1()
            .min_w_0()
            .child(ui::layout_probe("environment-toolbar"))
            .child(
                div()
                    .id("environment-tabs")
                    .role(gpui::Role::TabList)
                    .aria_label("Environment tools")
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap_1()
                    .overflow_x_scroll()
                    .children(self.environment.value.tabs.iter().map(|tab| {
                        let tab = *tab;
                        let (label, glyph, index) = tab_info(tab);
                        let active = panel_tab(self.dock_panel) == Some(tab);
                        div().id(("environment-tab-group", index)).flex().items_center().flex_shrink_0()
                            .rounded_md().bg(if active { rgb(palette().selected) } else { rgb(palette().canvas) })
                            .child(ui::button_shell(
                                SharedString::from(format!("environment-tab-{index}")), label, active,
                            )
                            .role(gpui::Role::Tab).aria_label(label).aria_selected(active)
                            .aria_description("Left/Right select. Alt+Left/Right reorder. Delete closes a stopped, saved tool.")
                            .track_focus(&self.environment.tabs_focus[index]).tab_stop(active)
                            .h(px(28.)).px_2().py_0().flex_shrink_0().flex().items_center().gap_1()
                            .text_size(px(12.)).relative()
                            .child(ui::layout_probe_slot("environment-tab", index))
                            .child(ui::icon(glyph).size(px(14.)))
                            .children((!compact).then_some(label))
                            .when(compact, |el| el.px(px(5.)))
                            .tooltip(move |_, cx| cx.new(|_| ui::Tooltip(label.into())).into())
                            .on_key_down(cx.listener(move |this, event, window, cx| this.environment_tab_key(tab, event, window, cx)))
                            .on_click(cx.listener(move |this, _, window, cx| this.select_environment_tab(tab, window, cx))))
                            .child(ui::chrome_button(
                                ["environment-terminal-close", "environment-explorer-close", "environment-changes-close"][index],
                                ["Close Terminal tab", "Close Explorer tab", "Close Changes tab"][index], Glyph::Close, false,
                                cx.listener(move |this, _: &(), window, cx| this.close_environment_tab(tab, window, cx)),
                            ).size(px(20.)).tab_stop(active).relative()
                                .child(ui::layout_probe_slot("environment-tab-close", index)))
                    }))
                    .children(
                        self.environment
                            .value
                            .tabs
                            .is_empty()
                            .then(|| div().px_2().text_size(px(13.)).child("Environment")),
                    ),
            )
            .child(
                ui::chrome_button(
                    "environment-add",
                    "Add Environment tool",
                    Glyph::Plus,
                    false,
                    cx.listener(|this, _: &(), window, cx| {
                        // Position relative to the actual toolbar, not screenshot pixels.
                        let bounds = this.environment.add_bounds.get();
                        let position = gpui::point(bounds.right(), bounds.bottom() + px(4.));
                        this.open_environment_menu(position, window, cx);
                    }),
                )
                .size(px(24.))
                .track_focus(&self.environment.add_focus)
                .relative()
                .child(
                    gpui::canvas(move |bounds, _, _| add_bounds.set(bounds), |_, _, _, _| {})
                        .absolute()
                        .inset_0()
                        .size_full(),
                ),
            )
            .child(
                ui::chrome_button(
                    "environment-maximize",
                    if self.environment.maximized {
                        "Restore split workspace"
                    } else {
                        "Maximize Environment"
                    },
                    if self.environment.maximized {
                        Glyph::Restore
                    } else {
                        Glyph::Maximize
                    },
                    false,
                    cx.listener(|this, _: &(), window, cx| {
                        this.environment.maximized = !this.environment.maximized;
                        this.focus_composer = false;
                        window.focus(&this.environment.add_focus, cx);
                        cx.notify();
                    }),
                )
                .size(px(24.)),
            )
            .child(
                ui::chrome_button(
                    "environment-close",
                    "Hide Environment (running tools stay open)",
                    Glyph::Close,
                    false,
                    cx.listener(|this, _: &(), _, cx| this.hide_environment(cx)),
                )
                .size(px(24.)),
            )
            .into_any_element()
    }
    pub(super) fn environment_overlay(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(popup) = &self.environment.popup else {
            return div().into_any_element();
        };
        div()
            .absolute()
            .inset_0()
            .occlude()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    this.environment.popup = None;
                    window.focus(&this.environment.add_focus, cx);
                    cx.notify();
                    cx.stop_propagation();
                }),
            )
            .child(
                gpui::anchored()
                    .anchor(gpui::Anchor::TopRight)
                    .position(popup.position)
                    .snap_to_window_with_margin(px(8.))
                    .child(popup.view.clone()),
            )
            .into_any_element()
    }
    pub(super) fn environment_divider(
        &self,
        available: f32,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        div()
            .id("environment-divider")
            .role(gpui::Role::Splitter)
            .aria_label("Resize chat and Environment")
            .aria_description("Left and Right resize. Home or double click resets equal widths.")
            .track_focus(&self.environment.divider_focus)
            .tab_stop(true)
            .absolute()
            .left(px(-4.))
            .top_0()
            .bottom_0()
            .w(px(8.))
            .cursor(gpui::CursorStyle::ResizeLeftRight)
            .hover(|style| style.bg(gpui::rgba((palette().focus << 8) | 0x55)))
            .focus_visible(|style| style.bg(gpui::rgba((palette().focus << 8) | 0x88)))
            .child(ui::layout_probe("environment-divider"))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &gpui::MouseDownEvent, window, cx| {
                    this.focus_composer = false;
                    window.focus(&this.environment.divider_focus, cx);
                    if event.click_count == 2 {
                        this.environment.value.width_ratio = 0.5;
                        this.environment.resize = None;
                        this.environment.save.changed();
                    } else {
                        this.environment.resize = Some(Resize {
                            origin: f32::from(event.position.x),
                            ratio: split_width(
                                available,
                                this.environment.value.width_ratio,
                                false,
                            ) / available.max(1.),
                            available,
                        });
                    }
                    cx.notify();
                    cx.stop_propagation();
                }),
            )
            .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, _, cx| {
                let modifiers = event.keystroke.modifiers;
                if modifiers.alt || modifiers.control || modifiers.platform {
                    return;
                }
                let step = if modifiers.shift { 0.1 } else { 0.03 };
                let current = split_width(available, this.environment.value.width_ratio, false)
                    / available.max(1.);
                let ratio = match event.keystroke.key.as_str() {
                    "left" => current + step,
                    "right" => current - step,
                    "home" => 0.5,
                    _ => return,
                };
                this.environment.value.width_ratio = ratio.clamp(0.2, 0.8);
                this.environment.save.changed();
                cx.notify();
                cx.stop_propagation();
            }))
            .into_any_element()
    }
    pub(super) fn environment_drag_overlay(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        div()
            .absolute()
            .inset_0()
            .occlude()
            .cursor(gpui::CursorStyle::ResizeLeftRight)
            .on_mouse_move(cx.listener(|this, event: &gpui::MouseMoveEvent, _, cx| {
                if let Some(resize) = this.environment.resize {
                    if event.dragging() {
                        this.environment.value.width_ratio = resized_ratio(
                            resize.ratio,
                            f32::from(event.position.x) - resize.origin,
                            resize.available,
                        );
                        this.environment.save.changed();
                    } else {
                        this.environment.resize = None;
                    }
                    cx.notify();
                    cx.stop_propagation();
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.environment.resize = None;
                    this.flush_environment(false);
                    cx.notify();
                    cx.stop_propagation();
                }),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn closing_tools_requires_explicit_save_and_stop_not_hidden_data_loss() {
        use EnvironmentTab::{Changes, Explorer, Terminal};
        assert!(tab_close_blocked(Explorer, true, false, false, false).is_some());
        assert!(tab_close_blocked(Explorer, false, true, false, false).is_some());
        assert!(tab_close_blocked(Terminal, false, false, true, false).is_some());
        assert!(tab_close_blocked(Terminal, false, false, false, true).is_some());
        assert!(tab_close_blocked(Changes, true, true, true, true).is_none());
        assert!(tab_close_blocked(Explorer, false, false, true, true).is_none());
        assert!(tab_close_blocked(Terminal, true, true, false, false).is_none());
    }
    #[test]
    fn split_respects_both_panes_at_intermediate_widths() {
        for available in [0., 100., 640., 700., 1020., 1660.] {
            for ratio in [0.2, 0.5, 0.8] {
                let width = split_width(available, ratio, false);
                assert!((0. ..=available).contains(&width));
                if available >= 640. {
                    assert!(width >= 320. && available - width >= 320.);
                }
                assert_eq!(split_width(available, ratio, true), available);
            }
        }
        assert_eq!(split_width(1020., 0.5, false), 510.);
        assert_eq!(split_width(f32::NAN, 0.5, false), 0.);
    }
    #[test]
    fn resizing_is_bounded_and_drag_direction_is_correct() {
        assert_eq!(resized_ratio(0.5, -100., 1000.), 0.6);
        assert_eq!(resized_ratio(0.5, 100., 1000.), 0.4);
        assert_eq!(resized_ratio(0.5, -5000., 1000.), 0.8);
        assert_eq!(resized_ratio(0.5, 5000., 1000.), 0.2);
        assert_eq!(resized_ratio(0.5, 100., 0.), 0.5);
    }
    #[test]
    fn layout_saves_coalesce_without_overtaking_inflight_writes() {
        let mut state = SaveState::default();
        state.changed();
        assert!(state.start(false));
        state.changed();
        state.changed();
        assert!(!state.start(true));
        state.finish(false);
        assert!(state.start(false));
        state.finish(false);
        assert!(!state.pending());
    }
    #[test]
    fn failed_saves_wait_for_an_edit_or_explicit_close_retry() {
        let mut state = SaveState::default();
        state.changed();
        state.start(false);
        state.finish(true);
        assert!(state.pending());
        assert!(!state.start(false));
        assert!(state.start(true));
        state.finish(true);
        state.changed();
        assert!(state.start(false));
    }
}
