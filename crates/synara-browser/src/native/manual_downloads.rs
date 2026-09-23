//! Human-requested link downloads. Wry's shared-context callbacks consult one
//! profile gate, while the reviewed destination and transfer belong to one tab.
use super::{CommittedDocument, HostTabId, Ordering, bridge};
use gtk::{gio, glib, prelude::*};
use std::{
    cell::{Cell, RefCell},
    fs,
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
    time::Duration,
};
use webkit2gtk::{
    ContextMenuExt, ContextMenuItem, DownloadExt, HitTestResultExt, URIRequestExt, URIResponseExt,
    WebContextExt, WebViewExt,
};

const MAX_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Default)]
pub(super) struct Gate {
    active: RefCell<Option<Rc<Transfer>>>,
    hook: RefCell<Option<(glib::WeakRef<webkit2gtk::WebContext>, glib::SignalHandlerId)>>,
}
struct Ui {
    web: glib::WeakRef<webkit2gtk::WebView>,
    shared: Arc<bridge::Shared>,
    tab: HostTabId,
    epoch: u64,
    alive: Cell<bool>,
    chooser: RefCell<Option<gtk::FileChooserNative>>,
}
struct Transfer {
    ui: Rc<Ui>,
    source: String,
    target: PathBuf,
    staging: tempfile::TempDir,
    download: RefCell<Option<webkit2gtk::Download>>,
    claimed: Cell<bool>,
    failed: Cell<bool>,
    done: Cell<bool>,
    watchdog: RefCell<Option<glib::SourceId>>,
}
pub(super) struct Owner {
    ui: Rc<Ui>,
    gate: Rc<Gate>,
    handler: Option<glib::SignalHandlerId>,
}
impl Ui {
    fn current(&self) -> bool {
        self.alive.get()
            && self.shared.ready.load(Ordering::Acquire)
            && self.shared.epoch(self.tab) == Some(self.epoch)
    }
    fn notice(&self, text: &str) {
        if self.current()
            && let Some(web) = self.web.upgrade()
        {
            web.set_tooltip_text(Some(text));
        }
    }
}
impl Gate {
    fn attach(self: &Rc<Self>, web: &webkit2gtk::WebView) {
        if self.hook.borrow().is_some() {
            return;
        }
        let Some(context) = web.context() else { return };
        let weak = Rc::downgrade(self);
        let handler = context.connect_download_started(move |_, download| {
            let Some(gate) = weak.upgrade() else {
                download.cancel();
                return;
            };
            let transfer = gate.active.borrow().clone();
            // Stop can revoke the epoch before WebKit emits download-started.
            // Retire the ticket as well as cancelling this late native object,
            // otherwise the private staging and profile slot wait for timeout.
            if let Some(transfer) = transfer.as_ref().filter(|t| !t.ui.current()) {
                gate.cancel(transfer, "Download cancelled: the browser tab changed.");
                download.cancel();
                return;
            }
            let Some(transfer) = transfer.filter(|t| {
                t.ui.current()
                    && !t.done.get()
                    && t.download.borrow().is_none()
                    && download
                        .web_view()
                        .is_some_and(|web| t.ui.web.upgrade().as_ref() == Some(&web))
                    && download.request().and_then(|r| r.uri()).as_deref()
                        == Some(t.source.as_str())
            }) else {
                download.cancel();
                return;
            };
            *transfer.download.borrow_mut() = Some(download.clone());
            download.set_allow_overwrite(false);
            let weak_transfer = Rc::downgrade(&transfer);
            download.connect_failed(move |_, _| {
                if let Some(t) = weak_transfer.upgrade() {
                    t.failed.set(true);
                }
            });
            let weak_transfer = Rc::downgrade(&transfer);
            let weak_gate = Rc::downgrade(&gate);
            download.connect_received_data(move |download, _| {
                if let (Some(t), Some(gate)) = (weak_transfer.upgrade(), weak_gate.upgrade()) {
                    let too_large = download.received_data_length() > MAX_BYTES
                        || download
                            .response()
                            .is_some_and(|r| r.content_length() > MAX_BYTES);
                    if !t.ui.current() || too_large {
                        gate.cancel(
                            &t,
                            "Download cancelled: tab changed or 256 MiB limit exceeded.",
                        );
                    } else {
                        t.ui.notice(&format!(
                            "Downloading: {} KiB. Right-click to cancel.",
                            download.received_data_length() / 1024
                        ));
                    }
                }
            });
            let weak_transfer = Rc::downgrade(&transfer);
            let weak_gate = Rc::downgrade(&gate);
            download.connect_finished(move |_| {
                if let (Some(t), Some(gate)) = (weak_transfer.upgrade(), weak_gate.upgrade()) {
                    gate.finish(&t);
                }
            });
        });
        *self.hook.borrow_mut() = Some((context.downgrade(), handler));
    }

    /// All Wry handlers in this manual profile share this gate. Older tabs must
    /// not retain authority or steal the destination from a newer tab's request.
    pub(super) fn destination(&self, source: &str, destination: &mut PathBuf) -> bool {
        let transfer = self.active.borrow().clone();
        let Some(t) = transfer else { return false };
        let download = t.download.borrow().clone();
        if !t.ui.current()
            || t.done.get()
            || t.failed.get()
            || source != t.source
            || download.is_none()
            || t.claimed.replace(true)
        {
            return false;
        }
        *destination = t.staging.path().join("payload");
        true
    }

    fn begin(self: &Rc<Self>, ui: Rc<Ui>, source: &str, target: &Path) -> Result<(), String> {
        if !ui.current() || self.active.borrow().is_some() {
            return Err("Another download is active, or this tab changed.".into());
        }
        let document = CommittedDocument::parse(source)
            .map_err(|_| "Only HTTP(S) links without embedded credentials can be saved.")?;
        let mut source = url::Url::parse(&document.canonical_url).map_err(|e| e.to_string())?;
        source.set_fragment(None);
        let source = source.to_string();
        let web = ui.web.upgrade().ok_or("The browser tab was closed.")?;
        if !web.is_visible() || !web.is_mapped() || web.is_loading() {
            return Err("Wait for this visible page to finish loading.".into());
        }
        let (target, staging) = prepare_destination(target)?;
        let transfer = Rc::new(Transfer {
            ui,
            source,
            target,
            staging,
            download: RefCell::new(None),
            claimed: Cell::new(false),
            failed: Cell::new(false),
            done: Cell::new(false),
            watchdog: RefCell::new(None),
        });
        *self.active.borrow_mut() = Some(transfer.clone());
        let weak_gate = Rc::downgrade(self);
        let weak_transfer = Rc::downgrade(&transfer);
        *transfer.watchdog.borrow_mut() = Some(glib::timeout_add_local_once(
            Duration::from_secs(120),
            move || {
                if let (Some(gate), Some(t)) = (weak_gate.upgrade(), weak_transfer.upgrade()) {
                    t.watchdog.borrow_mut().take();
                    gate.cancel(&t, "Download timed out. No destination file was saved.");
                }
            },
        ));
        transfer
            .ui
            .notice("Downloading. Right-click the page to cancel.");
        // Explicit GET only. No intercepted form POST is replayed as a GET.
        if web.download_uri(&transfer.source).is_none() {
            self.cancel(&transfer, "The browser could not start this download.");
            return Err("The browser could not start this download.".into());
        }
        Ok(())
    }
    fn cancel(&self, transfer: &Rc<Transfer>, message: &str) {
        transfer.failed.set(true);
        let download = transfer.download.borrow().clone();
        if let Some(download) = download {
            download.cancel();
        }
        self.finish(transfer);
        transfer.ui.notice(message);
    }
    fn finish(&self, transfer: &Rc<Transfer>) {
        if transfer.done.replace(true) {
            return;
        }
        if let Some(timer) = transfer.watchdog.borrow_mut().take() {
            timer.remove();
        }
        let valid_response = transfer
            .download
            .borrow()
            .as_ref()
            .and_then(|d| d.response())
            .is_some_and(|r| (200..300).contains(&r.status_code()));
        let result = if transfer.ui.current()
            && transfer.claimed.get()
            && !transfer.failed.get()
            && valid_response
        {
            publish(&transfer.staging.path().join("payload"), &transfer.target)
        } else {
            Err("Download failed or was cancelled. No destination file was saved.".into())
        };
        match result {
            Ok(()) => transfer.ui.notice(&format!(
                "Saved {}. The file was not opened.",
                transfer.target.display()
            )),
            Err(error) => transfer.ui.notice(&error),
        }
        let mut active = self.active.borrow_mut();
        if active
            .as_ref()
            .is_some_and(|current| Rc::ptr_eq(current, transfer))
        {
            active.take();
        }
        // Private staging is removed by TempDir when the last callback returns.
    }
}
impl Drop for Gate {
    fn drop(&mut self) {
        if let Some((context, handler)) = self.hook.get_mut().take()
            && let Some(context) = context.upgrade()
        {
            context.disconnect(handler);
        }
    }
}
impl Owner {
    pub(super) fn new(
        web: &webkit2gtk::WebView,
        shared: Arc<bridge::Shared>,
        tab: HostTabId,
        epoch: u64,
        gate: Rc<Gate>,
    ) -> Self {
        gate.attach(web);
        let ui = Rc::new(Ui {
            web: web.downgrade(),
            shared,
            tab,
            epoch,
            alive: Cell::new(true),
            chooser: RefCell::new(None),
        });
        let menu_ui = ui.clone();
        let menu_gate = gate.clone();
        let handler = web.connect_context_menu(move |_, menu, _, hit| {
            if let Some(source) = hit
                .link_uri()
                .filter(|url| CommittedDocument::parse(url).is_ok())
            {
                let action = gio::SimpleAction::new("synara-save-link", None);
                action.set_enabled(
                    menu_ui.current()
                        && menu_ui.chooser.borrow().is_none()
                        && menu_gate.active.borrow().is_none(),
                );
                let ui = menu_ui.clone();
                let gate = menu_gate.clone();
                action.connect_activate(move |_, _| {
                    choose(ui.clone(), gate.clone(), source.as_str())
                });
                menu.append(&ContextMenuItem::new_separator());
                menu.append(&ContextMenuItem::from_gaction(
                    &action,
                    "Save linked file as...",
                    None,
                ));
            }
            let active = menu_gate.active.borrow().clone();
            if let Some(transfer) = active.filter(|t| Rc::ptr_eq(&t.ui, &menu_ui)) {
                let action = gio::SimpleAction::new("synara-cancel-download", None);
                let gate = menu_gate.clone();
                action.connect_activate(move |_, _| {
                    gate.cancel(
                        &transfer,
                        "Download cancelled. No destination file was saved.",
                    )
                });
                menu.append(&ContextMenuItem::from_gaction(
                    &action,
                    "Cancel download",
                    None,
                ));
            }
            false
        });
        Self {
            ui,
            gate,
            handler: Some(handler),
        }
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        self.ui.alive.set(false);
        let chooser = self.ui.chooser.borrow_mut().take();
        if let Some(chooser) = chooser {
            chooser.destroy();
        }
        let transfer = self.gate.active.borrow().clone();
        if let Some(t) = transfer.filter(|t| Rc::ptr_eq(&t.ui, &self.ui)) {
            self.gate.cancel(&t, "Download cancelled.");
        }
        if let Some(web) = self.ui.web.upgrade()
            && let Some(handler) = self.handler.take()
        {
            web.disconnect(handler);
        }
    }
}
fn choose(ui: Rc<Ui>, gate: Rc<Gate>, source: &str) {
    if !ui.current() || ui.chooser.borrow().is_some() || gate.active.borrow().is_some() {
        return;
    }
    let Some(web) = ui.web.upgrade() else { return };
    if !web.is_visible() || !web.is_mapped() || web.is_loading() {
        return;
    }
    let chooser = gtk::FileChooserNative::new(
        Some("Save linked file (maximum 256 MiB, new filename required)"),
        None::<&gtk::Window>,
        gtk::FileChooserAction::Save,
        Some("Save"),
        Some("Cancel"),
    );
    chooser.set_local_only(true);
    chooser.set_current_name("download");
    let weak_ui = Rc::downgrade(&ui);
    let source = source.to_owned();
    chooser.connect_response(move |chooser, response| {
        if let Some(ui) = weak_ui.upgrade() {
            ui.chooser.borrow_mut().take();
            if response == gtk::ResponseType::Accept && ui.current() {
                let result = chooser
                    .filename()
                    .ok_or_else(|| "Choose a local destination file.".to_owned())
                    .and_then(|path| gate.begin(ui.clone(), &source, &path));
                if let Err(error) = result {
                    ui.notice(&error);
                }
            }
        }
        chooser.destroy();
    });
    *ui.chooser.borrow_mut() = Some(chooser.clone());
    chooser.show();
}
pub(super) fn prepare_destination(target: &Path) -> Result<(PathBuf, tempfile::TempDir), String> {
    if !target.is_absolute() {
        return Err("Choose an absolute local filename.".into());
    }
    let name = target
        .file_name()
        .ok_or("Choose a filename, not a directory.")?;
    let parent = target
        .parent()
        .ok_or("Choose a local directory.")?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let target = parent.join(name);
    match fs::symlink_metadata(&target) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
        _ => {
            return Err(
                "Destination already exists or is inaccessible. Choose a new filename.".into(),
            );
        }
    }
    let staging = tempfile::Builder::new()
        .prefix(".synara-download-")
        .tempdir_in(parent)
        .map_err(|e| e.to_string())?;
    Ok((target, staging))
}
pub(super) fn publish(source: &Path, target: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let metadata = fs::symlink_metadata(source).map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > MAX_BYTES {
        return Err("Download is not a regular file within the 256 MiB limit.".into());
    }
    fs::set_permissions(source, fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())?;
    // Staging is in the selected parent. A hard link publishes atomically without
    // replacing existing files or following an existing destination symlink.
    fs::hard_link(source, target).map_err(|e| format!("Could not save without overwriting: {e}"))
}

#[cfg(test)]
mod tests;
