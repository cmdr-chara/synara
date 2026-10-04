//! UI-thread ownership and geometry for the real child surface.
use super::*;
#[cfg(target_os = "linux")]
use std::{cell::RefCell, rc::Rc, time::Duration};

#[cfg(target_os = "linux")]
pub(super) type Host = Rc<RefCell<browser_domain::native::NativeHost>>;

impl Shell {
    pub(in crate::shell) fn prepare_native_browser(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        #[cfg(target_os = "linux")]
        {
            if self.browser.native_task.is_none() && self.panel != Panel::Browser {
                return;
            }
            if self.browser.native_task.is_none() {
                self.pump_native_browser(window, cx);
                self.browser.native_task = Some(cx.spawn_in(window, async move |weak, cx| {
                    let mut interval = Duration::from_millis(16);
                    loop {
                        cx.background_executor().timer(interval).await;
                        let result = weak.update_in(cx, |this, window, cx| {
                            this.pump_native_browser(window, cx);
                            this.browser.native.borrow().has_tabs()
                        });
                        match result {
                            Ok(true) => interval = Duration::from_millis(16),
                            Ok(false) => interval = Duration::from_millis(100),
                            Err(_) => break,
                        }
                    }
                }));
            }
            if !self.native_browser_visible() {
                self.browser.native.borrow_mut().viewport(None, None);
            }
        }
        #[cfg(not(target_os = "linux"))]
        let _ = (window, cx);
    }
    #[cfg(target_os = "linux")]
    fn pump_native_browser(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.browser_tick_authentication(cx);
        let events = self.browser.native.borrow_mut().pump(window);
        self.browser_tick_authentication(cx);
        let changed = !events.is_empty();
        for event in events {
            // Stale callbacks after cancellation/close are expected and never restore authority.
            let _ = self
                .controller
                .browser
                .with(|session, now| session.event_at(event, now));
        }
        if changed {
            self.browser_save_manual_restore();
            self.browser_save_owned_restore();
        }
        let _ = self.controller.browser.with(|_, _| Ok(()));
        if changed {
            cx.notify();
        }
    }
    #[cfg(target_os = "linux")]
    fn native_browser_visible(&self) -> bool {
        self.panel == Panel::Browser
            && !self.zen_active()
            && self.close == CloseState::Open
            && !self.terminal_layout_quitting
            && !self.terminal_closing
            && !self.draft_state.quitting
            && !self.environment.quitting
            && !self.command_palette.open
            && !self.navigation.menu_open
            && !self.settings.personalization.attention_open
            && !self.controls.is_open()
            && !self.revisions.open()
            && self.kanban.dialog.is_none()
            && self.organization.dialog.is_none()
            && self.saved_context.dialog.is_none()
            && !self.explorer.modal_open()
            && !self.environment.menu_open()
            && !self.chat_tools.menu_open()
            && self.settings.popup.is_none()
    }
    pub(super) fn native_browser_surface(&self) -> AnyElement {
        #[cfg(target_os = "linux")]
        {
            let host = self.browser.native.clone();
            let selected = self.browser.selected;
            let visible = self.native_browser_visible();
            let error = host.borrow().error().map(str::to_owned);
            let mut surface = div()
                .id("browser-native-surface")
                .relative()
                .flex_1()
                .min_h(px(160.))
                .min_w_0()
                .overflow_hidden()
                .child(ui::layout_probe("browser-viewport"));
            if let Some(error) = error {
                surface = surface.child(div().p_3().child(error));
            } else if selected.is_none() {
                surface = surface.child(
                    div()
                        .p_3()
                        .child("Create a tab and enter an HTTP(S) address."),
                );
            }
            surface
                .child(
                    gpui::canvas(
                        move |bounds, _, _| {
                            use browser_domain::native::ViewportRect;
                            let rect = visible.then(|| {
                                ViewportRect::logical(
                                    f32::from(bounds.origin.x),
                                    f32::from(bounds.origin.y),
                                    f32::from(bounds.size.width),
                                    f32::from(bounds.size.height),
                                )
                            });
                            host.borrow_mut().viewport(selected, rect);
                        },
                        |_, _, _, _| {},
                    )
                    .absolute()
                    .inset_0()
                    .size_full(),
                )
                .into_any_element()
        }
        #[cfg(not(target_os = "linux"))]
        div().flex_1().min_h(px(160.)).p_3().child("This build does not yet include a Windows or macOS native browser adapter. Linux/X11 uses the real WebKitGTK host.").into_any_element()
    }
}
