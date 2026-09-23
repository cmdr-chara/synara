//! Human-only, bounded viewport capture. No agent capability or filesystem grant.
mod export;
use super::{HostTabId, Ordering, bridge};
use gtk::{gdk, gio, glib, prelude::*};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
    time::Duration,
};
use webkit2gtk::{ContextMenuExt, ContextMenuItem, SnapshotOptions, SnapshotRegion, WebViewExt};

const MAX_CAPTURE_PIXELS: i64 = 16 * 1024 * 1024;

pub(super) struct Owner {
    _exporter: export::Owner,
    state: Rc<State>,
    web: glib::WeakRef<webkit2gtk::WebView>,
    handler: Option<glib::SignalHandlerId>,
}
struct State {
    shared: Arc<bridge::Shared>,
    tab: HostTabId,
    epoch: u64,
    cancelled: gio::Cancellable,
    busy: Cell<bool>,
    watchdog: RefCell<Option<glib::SourceId>>,
}
impl State {
    fn current(&self) -> bool {
        !self.cancelled.is_cancelled()
            && self.shared.ready.load(Ordering::Acquire)
            && self.shared.epoch(self.tab) == Some(self.epoch)
    }
    fn clear_watchdog(&self) {
        if let Some(source) = self.watchdog.borrow_mut().take() {
            source.remove();
        }
    }
}
impl Owner {
    pub(super) fn new(
        web: &webkit2gtk::WebView,
        shared: Arc<bridge::Shared>,
        tab: HostTabId,
        epoch: u64,
    ) -> Self {
        let state = Rc::new(State {
            shared,
            tab,
            epoch,
            cancelled: gio::Cancellable::new(),
            busy: Cell::new(false),
            watchdog: RefCell::new(None),
        });
        let menu_state = state.clone();
        let handler = web.connect_context_menu(move |web, menu, _, _| {
            let action = gio::SimpleAction::new("synara-copy-page-image", None);
            action.set_enabled(menu_state.current() && !menu_state.busy.get());
            let weak = web.downgrade();
            let state = menu_state.clone();
            action.connect_activate(move |_, _| {
                if let Some(web) = weak.upgrade() {
                    copy(&web, state.clone());
                }
            });
            menu.append(&ContextMenuItem::new_separator());
            menu.append(&ContextMenuItem::from_gaction(
                &action,
                "Copy visible page image",
                None,
            ));
            false
        });
        Self {
            _exporter: export::Owner::new(web, state.clone()),
            state,
            web: web.downgrade(),
            handler: Some(handler),
        }
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        self.state.clear_watchdog();
        self.state.cancelled.cancel();
        if let Some(web) = self.web.upgrade()
            && let Some(handler) = self.handler.take()
        {
            web.disconnect(handler);
        }
    }
}

fn allowed_size(width: i32, height: i32, scale: i32) -> bool {
    width > 0
        && height > 0
        && (1..=8).contains(&scale)
        && i64::from(width) * i64::from(scale) <= 8192
        && i64::from(height) * i64::from(scale) <= 8192
        && i64::from(width) * i64::from(height) * i64::from(scale).pow(2) <= MAX_CAPTURE_PIXELS
}

fn copy(web: &webkit2gtk::WebView, state: Rc<State>) {
    if !state.current() || state.busy.get() || !web.is_visible() || !web.is_mapped() {
        return;
    }
    if web.is_loading()
        || !allowed_size(
            web.allocated_width(),
            web.allocated_height(),
            web.scale_factor(),
        )
    {
        web.set_tooltip_text(Some(
            "Page capture unavailable: wait for loading or reduce the viewport.",
        ));
        return;
    }
    state.busy.set(true);
    let weak = web.downgrade();
    let timeout_web = weak.clone();
    let timeout = state.clone();
    *state.watchdog.borrow_mut() = Some(glib::timeout_add_local_once(
        Duration::from_secs(10),
        move || {
            timeout.watchdog.borrow_mut().take();
            if timeout.current() && timeout.busy.replace(false) {
                timeout.cancelled.cancel();
                if let Some(web) = timeout_web.upgrade() {
                    web.set_tooltip_text(Some("Page capture timed out. Reload this tab to retry."));
                }
            }
        },
    ));
    let finished = state.clone();
    web.snapshot(
        SnapshotRegion::Visible,
        SnapshotOptions::empty(),
        Some(&state.cancelled),
        move |result| {
            finished.clear_watchdog();
            finished.busy.set(false);
            if !finished.current() {
                return;
            }
            let Some(web) = weak.upgrade() else {
                return;
            };
            // Hiding/replacing the pane during capture must not publish a stale image.
            if !web.is_visible() || !web.is_mapped() || web.is_loading() {
                return;
            }
            let image = result
                .ok()
                .and_then(|surface| gtk::cairo::ImageSurface::try_from(surface).ok())
                .filter(|surface| allowed_size(surface.width(), surface.height(), 1))
                .and_then(|surface| {
                    gdk::pixbuf_get_from_surface(&surface, 0, 0, surface.width(), surface.height())
                });
            match image {
                Some(image) => {
                    gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD).set_image(&image);
                    web.set_tooltip_text(Some("Visible page image copied to the clipboard."));
                }
                None => web
                    .set_tooltip_text(Some("Page capture failed. The clipboard was not changed.")),
            }
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        StoragePartition,
        session::{Command, NativePort},
    };
    use std::time::Instant;

    #[test]
    fn capture_budget_checks_dimensions_and_device_scale_before_allocation() {
        assert!(allowed_size(1920, 1080, 2));
        assert!(!allowed_size(8192, 8192, 1));
        assert!(!allowed_size(i32::MAX, i32::MAX, 8));
        assert!(!allowed_size(1, 1, 0));
        assert!(!allowed_size(0, 1080, 1));
    }

    #[test]
    #[ignore = "requires WebKitGTK and a private X11 display"]
    fn real_capture_copies_pixels_and_refuses_revoked_epochs() {
        gtk::init().unwrap();
        let (mut port, _receiver, shared) = bridge::channel();
        let tab = HostTabId(1);
        port.send(Command::Open {
            tab,
            partition: StoragePartition::Manual,
        })
        .unwrap();
        shared.ready.store(true, Ordering::Release);
        assert!(
            !port.capabilities().capture,
            "manual copy must not enable agent capture"
        );
        let web = webkit2gtk::WebView::new();
        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        window.set_default_size(640, 480);
        window.add(&web);
        window.show_all();
        let owner = Owner::new(&web, shared.clone(), tab, 0);
        web.load_html(
            "<!doctype html><title>Capture fixture</title><h1>Visible capture fixture</h1>",
            None,
        );
        let pump = || {
            for _ in 0..32 {
                if !gtk::events_pending() {
                    break;
                }
                gtk::main_iteration_do(false);
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        let end = Instant::now() + Duration::from_secs(20);
        while web.is_loading()
            || web.title().as_deref() != Some("Capture fixture")
            || !web.is_mapped()
        {
            pump();
            assert!(Instant::now() < end, "capture fixture did not load");
        }
        let clipboard = gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD);
        clipboard.set_text("capture sentinel");
        copy(&web, owner.state.clone());
        while owner.state.busy.get() {
            pump();
            assert!(Instant::now() < end, "capture did not complete");
        }
        let image = clipboard
            .wait_for_image()
            .expect("real pixels were not copied");
        assert!(image.width() >= 640 && image.height() >= 480);
        clipboard.set_text("revoked capture sentinel");
        copy(&web, owner.state.clone());
        port.send(Command::Stop { tab }).unwrap();
        while owner.state.busy.get() {
            pump();
            assert!(Instant::now() < end, "revoked capture did not settle");
        }
        assert_eq!(
            clipboard.wait_for_text().as_deref(),
            Some("revoked capture sentinel")
        );
        assert!(clipboard.wait_for_image().is_none());
        drop(owner);
        window.close();
        println!(
            "MANUAL_CAPTURE_ACCEPTANCE: real viewport pixels and revoked-epoch clipboard fence passed"
        );
    }
}
