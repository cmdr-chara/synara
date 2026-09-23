//! Explicit manual PNG export using the capture owner's epoch and cancellation.
//! Encoding runs asynchronously and the selected filename is published only once.
use super::{State, allowed_size};
use crate::native::manual_downloads;
use gtk::{gdk, gdk_pixbuf, gio, glib, prelude::*};
use std::{cell::RefCell, path::Path, rc::Rc, time::Duration};
use webkit2gtk::{ContextMenuExt, ContextMenuItem, SnapshotOptions, SnapshotRegion, WebViewExt};

const MAX_PNG_BYTES: u64 = 72 * 1024 * 1024;
type Chooser = Rc<RefCell<Option<gtk::FileChooserNative>>>;

pub(super) struct Owner {
    web: glib::WeakRef<webkit2gtk::WebView>,
    chooser: Chooser,
    handler: Option<glib::SignalHandlerId>,
}
impl Owner {
    pub(super) fn new(web: &webkit2gtk::WebView, state: Rc<State>) -> Self {
        let chooser = Chooser::default();
        let slot = chooser.clone();
        let handler = web.connect_context_menu(move |web, menu, _, _| {
            let action = gio::SimpleAction::new("synara-save-page-image", None);
            action.set_enabled(state.current() && !state.busy.get());
            let state = state.clone();
            let slot = slot.clone();
            let weak = web.downgrade();
            action.connect_activate(move |_, _| {
                if let Some(web) = weak.upgrade() {
                    choose(&web, state.clone(), slot.clone());
                }
            });
            menu.append(&ContextMenuItem::from_gaction(
                &action,
                "Save visible page image as PNG...",
                None,
            ));
            false
        });
        Self {
            web: web.downgrade(),
            chooser,
            handler: Some(handler),
        }
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        let chooser = self.chooser.borrow_mut().take();
        if let Some(chooser) = chooser {
            chooser.destroy();
        }
        if let Some(web) = self.web.upgrade()
            && let Some(handler) = self.handler.take()
        {
            web.disconnect(handler);
        }
    }
}
fn visible(web: &webkit2gtk::WebView, state: &State) -> bool {
    state.current() && web.is_visible() && web.is_mapped() && !web.is_loading()
}
fn finish(web: &webkit2gtk::WebView, state: &State, message: &str) {
    state.clear_watchdog();
    state.busy.set(false);
    if state.current() {
        web.set_tooltip_text(Some(message));
    }
}
fn choose(web: &webkit2gtk::WebView, state: Rc<State>, slot: Chooser) {
    if !visible(web, &state) || state.busy.get() || slot.borrow().is_some() {
        return;
    }
    let chooser = gtk::FileChooserNative::new(
        Some("Save visible page image (new filename required)"),
        None::<&gtk::Window>,
        gtk::FileChooserAction::Save,
        Some("Save PNG"),
        Some("Cancel"),
    );
    chooser.set_local_only(true);
    chooser.set_current_name("synara-page.png");
    let filter = gtk::FileFilter::new();
    filter.set_name(Some("PNG image"));
    filter.add_pattern("*.png");
    chooser.add_filter(filter);
    let weak = web.downgrade();
    let weak_slot = Rc::downgrade(&slot);
    state.busy.set(true);
    chooser.connect_response(move |chooser, response| {
        if let Some(slot) = weak_slot.upgrade() {
            slot.borrow_mut().take();
        }
        state.busy.set(false);
        if response == gtk::ResponseType::Accept
            && let Some(web) = weak.upgrade()
            && visible(&web, &state)
        {
            if let Some(path) = chooser.filename() {
                save(&web, state.clone(), &path);
            } else {
                finish(
                    &web,
                    &state,
                    "Choose a local PNG filename. Nothing was saved.",
                );
            }
        }
        chooser.destroy();
    });
    *slot.borrow_mut() = Some(chooser.clone());
    chooser.show();
}
pub(super) fn save(web: &webkit2gtk::WebView, state: Rc<State>, target: &Path) {
    if !visible(web, &state) || state.busy.get() {
        return;
    }
    if !allowed_size(
        web.allocated_width(),
        web.allocated_height(),
        web.scale_factor(),
    ) {
        finish(
            web,
            &state,
            "Page image is too large. Reduce the viewport before saving.",
        );
        return;
    }
    let (target, staging) = match manual_downloads::prepare_destination(target) {
        Ok(destination) => destination,
        Err(error) => {
            finish(web, &state, &error);
            return;
        }
    };
    state.busy.set(true);
    let timeout = state.clone();
    let timeout_web = web.downgrade();
    *state.watchdog.borrow_mut() = Some(glib::timeout_add_local_once(
        Duration::from_secs(30),
        move || {
            timeout.watchdog.borrow_mut().take();
            if timeout.busy.replace(false) && timeout.current() {
                timeout.cancelled.cancel();
                if let Some(web) = timeout_web.upgrade() {
                    web.set_tooltip_text(Some("PNG export timed out. Reload this tab to retry."));
                }
            }
        },
    ));
    let weak = web.downgrade();
    let pending = state.clone();
    web.snapshot(
        SnapshotRegion::Visible,
        SnapshotOptions::empty(),
        Some(&state.cancelled),
        move |result| {
            let Some(web) = weak.upgrade() else {
                pending.clear_watchdog();
                pending.busy.set(false);
                return;
            };
            if !visible(&web, &pending) {
                finish(
                    &web,
                    &pending,
                    "PNG export cancelled because the page changed.",
                );
                return;
            }
            let image = result
                .ok()
                .and_then(|surface| gtk::cairo::ImageSurface::try_from(surface).ok())
                .filter(|surface| allowed_size(surface.width(), surface.height(), 1))
                .and_then(|surface| {
                    gdk::pixbuf_get_from_surface(&surface, 0, 0, surface.width(), surface.height())
                });
            let Some(image) = image else {
                finish(&web, &pending, "Page capture failed. Nothing was saved.");
                return;
            };
            encode(&web, pending, image, target, staging);
        },
    );
}
fn encode(
    web: &webkit2gtk::WebView,
    state: Rc<State>,
    image: gdk_pixbuf::Pixbuf,
    target: std::path::PathBuf,
    staging: tempfile::TempDir,
) {
    let payload = staging.path().join("payload");
    let output = match gio::File::for_path(&payload)
        .create(gio::FileCreateFlags::PRIVATE, Some(&state.cancelled))
    {
        Ok(output) => output,
        Err(error) => {
            finish(
                web,
                &state,
                &format!("Could not prepare PNG export: {error}"),
            );
            return;
        }
    };
    let weak = web.downgrade();
    let stream = output.clone();
    let pending = state.clone();
    image.save_to_streamv_async(
        &output,
        "png",
        &[],
        Some(&state.cancelled),
        move |encoded| {
            // Closing after async encoding flushes only our private staging file.
            // Neither an error nor cancellation ever publishes the selected path.
            let closed = stream.close(None::<&gio::Cancellable>);
            let _staging = staging;
            let Some(web) = weak.upgrade() else {
                pending.clear_watchdog();
                pending.busy.set(false);
                return;
            };
            if !visible(&web, &pending) {
                finish(
                    &web,
                    &pending,
                    "PNG export cancelled because the page changed.",
                );
                return;
            }
            let result = encoded
                .and(closed)
                .map_err(|error| error.to_string())
                .and_then(|_| {
                    let size = std::fs::metadata(&payload)
                        .map_err(|error| error.to_string())?
                        .len();
                    if size == 0 || size > MAX_PNG_BYTES {
                        return Err(
                            "Encoded PNG exceeds the export limit. Nothing was saved.".into()
                        );
                    }
                    manual_downloads::publish(&payload, &target)
                });
            match result {
                Ok(()) => finish(
                    &web,
                    &pending,
                    "Visible page image saved as PNG. The file was not opened.",
                ),
                Err(error) => finish(&web, &pending, &format!("PNG export failed: {error}")),
            }
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        HostTabId, StoragePartition,
        native::{bridge, manual_capture},
        session::{Command, NativePort},
    };
    use std::{sync::atomic::Ordering, time::Instant};

    #[test]
    #[ignore = "requires real WebKitGTK and a private X11 display"]
    fn real_png_export_preserves_pixels_and_refuses_revoked_publication() {
        gtk::init().unwrap();
        let (mut port, _receiver, shared) = bridge::channel();
        let tab = HostTabId(1);
        port.send(Command::Open {
            tab,
            partition: StoragePartition::Manual,
        })
        .unwrap();
        shared.ready.store(true, Ordering::Release);
        assert!(!port.capabilities().capture);
        let web = webkit2gtk::WebView::new();
        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        window.set_default_size(640, 480);
        window.add(&web);
        window.show_all();
        let owner = manual_capture::Owner::new(&web, shared, tab, 0);
        web.load_html(
            "<!doctype html><title>PNG fixture</title><h1>Export these pixels</h1>",
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
        while web.is_loading() || !web.is_mapped() || web.title().as_deref() != Some("PNG fixture")
        {
            pump();
            assert!(Instant::now() < end, "PNG fixture did not load");
        }
        let settle = || {
            let end = Instant::now() + Duration::from_secs(10);
            while owner.state.busy.get() {
                pump();
                assert!(Instant::now() < end, "PNG export did not settle");
            }
        };
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("image.png");
        let clipboard = gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD);
        clipboard.set_text("clipboard untouched by PNG export");
        save(&web, owner.state.clone(), &path);
        settle();
        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
        let image = gdk_pixbuf::Pixbuf::from_file(&path).unwrap();
        assert!(image.width() >= 640 && image.height() >= 480);
        assert_eq!(
            clipboard.wait_for_text().as_deref(),
            Some("clipboard untouched by PNG export")
        );
        save(&web, owner.state.clone(), &path);
        settle();
        assert_eq!(
            std::fs::read(&path).unwrap(),
            bytes,
            "existing image was replaced"
        );
        let stale = root.path().join("stale.png");
        save(&web, owner.state.clone(), &stale);
        port.send(Command::Stop { tab }).unwrap();
        settle();
        assert!(!stale.exists(), "revoked capture published a file");
        drop(owner);
        window.close();
        assert_eq!(
            std::fs::read_dir(root.path()).unwrap().count(),
            1,
            "private staging leaked"
        );
        println!(
            "PNG_EXPORT_ACCEPTANCE: real PNG pixels, clipboard preservation, no overwrite, stale epoch refusal and staging cleanup passed"
        );
    }
}
