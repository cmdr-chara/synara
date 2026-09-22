//! A reversible presentation of the existing Synara/Studio task, not another scope.
use super::*;
use crate::ui::{self, Glyph, palette};

impl Shell {
    pub(super) fn zen_active(&self) -> bool {
        self.settings.value.appearance.personalization.zen_mode
            && matches!(self.panel, Panel::Conversation | Panel::Dock | Panel::Files | Panel::Changes | Panel::Terminal | Panel::Device | Panel::SideChats)
    }
    pub(super) fn toggle_zen(&mut self, cx: &mut Context<Self>) {
        if self.settings.saving || self.settings.personalization.busy
            || self.close != CloseState::Open || self.terminal_closing || self.explorer.modal_open()
            || self.kanban.dialog.is_some() || self.organization.dialog.is_some()
            || self.saved_context.dialog.is_some() || self.composer.read(cx).is_composing()
            || self.editor.read(cx).is_composing() || self.terminal_view.read(cx).has_pending_input()
            || self.hubs.pending(cx)
        { return; }
        self.settings.personalization.navigation_shown = false;
        self.settings.personalization.tools_shown = false;
        self.settings.personalization.details_shown = false;
        self.controls.retire();
        self.environment.retire_popup();
        self.chat_tools.retire();
        self.focus_composer = matches!(self.panel, Panel::Conversation | Panel::Dock | Panel::Files | Panel::Changes | Panel::Terminal | Panel::Device | Panel::SideChats);
        self.save_setting(|s| s.appearance.personalization.zen_mode = !s.appearance.personalization.zen_mode, cx);
    }
    pub(super) fn zen_shortcut(&mut self, event: &gpui::KeyDownEvent, cx: &mut Context<Self>) -> bool {
        let m = event.keystroke.modifiers;
        if event.is_held || event.prefer_character_input || self.composer.read(cx).is_composing()
            || self.editor.read(cx).is_composing() || self.command_palette.open
            || self.controls.is_open() || self.environment.menu_open() || self.chat_tools.menu_open()
            || self.settings.popup.is_some() || self.navigation.menu_open
        { return false; }
        if (m.control || m.platform) && m.alt && !m.shift && event.keystroke.key == "z" {
            self.toggle_zen(cx);
            return true;
        }
        false
    }
    pub(super) fn toggle_zen_tools(&mut self, cx: &mut Context<Self>) {
        self.settings.personalization.tools_shown = !self.settings.personalization.tools_shown;
        if self.settings.personalization.tools_shown && !self.dock_open() {
            self.set_panel(Panel::Dock, cx);
        }
        self.focus_composer = !self.settings.personalization.tools_shown;
        cx.notify();
    }
    fn attention_tasks(&self) -> Vec<(TaskId, String, String, usize)> {
        let mut rows: Vec<_> = self.catalog.tasks.iter().filter_map(|task| {
            let requests = self.pending.values().filter(|request| request.is_active() && (request.context().thread_id == task.thread_id
                || self.selected == Some(task.id) && self.details.as_ref().is_some_and(|details|
                    request.context().scope == InteractionScope::Connection(details.connection.id)))).count();
            let running = self.busy.contains(&task.id) || self.connecting.contains(&task.id);
            if requests == 0 && !running { return None; }
            let project = self.catalog.projects.iter().find(|project| project.id == task.project_id)
                .map_or("Standalone", |project| project.name.as_str());
            let state = if requests > 0 { format!("{requests} pending decision(s)") }
                else if self.connecting.contains(&task.id) { "Connecting".into() } else { "Working".into() };
            Some((task.id, task.title.clone(), format!("{project} · {state}"), requests))
        }).collect();
        rows.sort_by(|a, b| b.3.cmp(&a.3).then_with(|| a.1.cmp(&b.1)));
        rows
    }
    pub(super) fn open_attention(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.controls.retire(); self.environment.retire_popup(); self.chat_tools.retire();
        self.navigation.menu_open = false;
        self.settings.popup = None;
        self.settings.personalization.attention_open = true;
        self.focus_composer = false;
        window.focus(&self.settings.personalization.attention_focus, cx);
        cx.notify();
    }
    pub(super) fn zen_toolbar(&self, viewport: f32, maximized: bool, cx: &mut Context<Self>) -> gpui::AnyElement {
        let compact = viewport < 820.;
        let waiting = self.pending.values().filter(|request| request.is_active()).count();
        let title = self.task().map_or("New thread", |task| task.title.as_str());
        div().id("zen-toolbar").relative().h(px(ui::CHROME_HEIGHT)).w_full().flex_shrink_0()
            .flex().items_center().px_2().gap_1()
            .child(ui::chrome_button("zen-navigation", "Show or hide navigation", Glyph::Panel, false,
                cx.listener(|this, _: &(), _, cx| {
                    this.settings.personalization.navigation_shown = !this.settings.personalization.navigation_shown;
                    this.focus_composer = false; cx.notify();
                })))
            .child(div().id("zen-drag-region").flex_1().min_w_0().h_full().flex().items_center().gap_2()
                .on_mouse_down(gpui::MouseButton::Left, |event, window, _| {
                    if event.click_count == 2 { window.zoom_window(); } else { window.start_window_move(); }
                })
                .child(gpui::svg().path("brand/synara.svg").size(px(20.)).text_color(rgb(palette().text)))
                .children((!compact).then(|| div().font_family("Cal Sans").text_size(px(16.)).child(if self.navigation.studio { "Hubs" } else { "Synara" })))
                .child(div().min_w_0().text_ellipsis().text_size(px(12.)).text_color(rgb(palette().muted)).child(title.to_owned())))
            .child(ui::action("zen-attention", if waiting > 0 { format!("{waiting}") } else { String::new() }, Some(Glyph::Bell), waiting > 0,
                cx.listener(|this, _: &(), window, cx| this.open_attention(window, cx))).aria_label("Active tasks and pending decisions"))
            .child(ui::chrome_button("zen-command-search", "Search commands and threads", Glyph::Search, false,
                cx.listener(|this, _: &(), window, cx| this.open_command_palette(window, cx))))
            .child(ui::chrome_button("zen-new-chat", "New chat in Synara or the current Hub", Glyph::Compose, false,
                cx.listener(|this, _: &(), _, cx| {
                    this.start_new_chat(cx);
                })))
            .child(ui::chrome_button("zen-tools", "Show or hide Environment", Glyph::PanelRight, false,
                cx.listener(|this, _: &(), _, cx| this.toggle_zen_tools(cx))))
            .child(ui::chrome_button("zen-details", "Show or hide conversation actions", Glyph::More, false,
                cx.listener(|this, _: &(), _, cx| {
                    this.settings.personalization.details_shown = !this.settings.personalization.details_shown;
                    cx.notify();
                })))
            .child(ui::chrome_button("zen-appearance", "Customize appearance", Glyph::Palette, false,
                cx.listener(|this, _: &(), _, cx| this.open_appearance(cx))))
            .child(ui::action("exit-zen", "Exit Zen", None, false,
                cx.listener(|this, _: &(), _, cx| this.toggle_zen(cx)))
                .w(px(76.)).text_size(px(12.)).aria_label("Exit Zen mode, Ctrl/Cmd+Alt+Z"))
            .children((!cfg!(target_os = "macos")).then(|| div().flex().items_center().gap_1()
                .child(ui::chrome_button("zen-window-minimize", "Minimize", Glyph::Minimize, false, |_, window, _| window.minimize_window()))
                .child(ui::chrome_button("zen-window-maximize", "Maximize or restore", if maximized { Glyph::Restore } else { Glyph::Maximize }, false, |_, window, _| window.zoom_window()))
                .child(ui::chrome_button("zen-window-close", "Close window", Glyph::Close, false, cx.listener(|this, _: &(), window, cx| { this.request_close(window, cx); })))))
            .into_any_element()
    }
    pub(super) fn attention_overlay(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let rows = self.attention_tasks();
        div().absolute().inset_0().occlude().bg(gpui::rgba(0x00000066)).flex().justify_center().items_center().p_4()
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, window, cx| {
                this.settings.personalization.attention_open = false;
                window.focus(&this.navigation.root_focus, cx); cx.notify(); cx.stop_propagation();
            }))
            .child(div().id("attention-dialog").role(gpui::Role::Dialog).aria_label("Active tasks and pending decisions")
                .track_focus(&self.settings.personalization.attention_focus).tab_group().occlude()
                .w(px(560.)).max_w_full().max_h(px(560.)).rounded_xl().border_1().border_color(rgb(palette().border))
                .bg(rgb(palette().overlay)).p_4().flex().flex_col().gap_2()
                .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                    if event.keystroke.key == "escape" {
                        this.settings.personalization.attention_open = false;
                        window.focus(&this.navigation.root_focus, cx); cx.notify(); cx.stop_propagation();
                    }
                }))
                .child(div().flex().items_center().gap_2()
                    .child(div().flex_1().text_size(px(18.)).child("Active tasks"))
                    .child(ui::chrome_button("attention-close", "Close active tasks", Glyph::Close, false,
                        cx.listener(|this, _: &(), window, cx| {
                            this.settings.personalization.attention_open = false;
                            window.focus(&this.navigation.root_focus, cx); cx.notify();
                        }))))
                .child(div().text_size(px(12.)).text_color(rgb(palette().muted)).child("Open a conversation to review its requests. Nothing is approved or cancelled here."))
                .child(div().id("attention-rows").max_h(px(400.)).overflow_y_scroll().flex().flex_col().gap_1()
                    .children(rows.iter().enumerate().map(|(index, (id, title, detail, requests))| {
                        let id = *id;
                        ui::action(("attention-task", index), title.clone(), Some(if *requests > 0 { Glyph::Shield } else { Glyph::Agent }), false,
                            cx.listener(move |this, _: &(), _, cx| {
                                if this.select_task(id, cx) {
                                    this.settings.personalization.attention_open = false;
                                    this.show_conversation(cx);
                                    this.settings.personalization.tools_shown = false;
                                    this.transcript.follow();
                                }
                                cx.notify();
                            })).w_full().h_auto().min_h(px(44.)).flex_wrap()
                            .child(div().w_full().text_size(px(11.)).text_color(rgb(palette().muted)).child(detail.clone()))
                    }))
                    .children(rows.is_empty().then(|| div().p_4().text_size(px(13.)).child("No running chats or pending task decisions.")))))
            .into_any_element()
    }
}
