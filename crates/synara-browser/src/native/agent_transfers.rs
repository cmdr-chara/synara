//! Approved agent-task uploads/downloads for one owned WebKit document.
use super::{bridge, Events, HostRequestId, Ordering};
use crate::{
    session::{Event, Output, UploadPayload},
    HostTabId,
};
use gtk::{gio, glib, prelude::*};
use std::{
    cell::{Cell, RefCell},
    fs,
    path::PathBuf,
    rc::{Rc, Weak},
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};
use webkit2gtk::{
    DownloadExt, FileChooserRequestExt, URIRequestExt, URIResponseExt, WebContextExt, WebViewExt,
};

const MAX_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Default)]
pub(super) struct Gate {
    active: RefCell<Option<Rc<Transfer>>>,
}
impl Gate {
    pub(super) fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }
    pub(super) fn destination(&self, source: &str, destination: &mut PathBuf) -> bool {
        let Some(transfer) = self.active.borrow().clone() else {
            return false;
        };
        if transfer.done.get()
            || transfer.claimed.replace(true)
            || !transfer.current()
            || source != transfer.source
        {
            return false;
        }
        *destination = transfer.staging.path().join("payload");
        true
    }
    fn retire(&self, transfer: &Rc<Transfer>) {
        let mut active = self.active.borrow_mut();
        if active
            .as_ref()
            .is_some_and(|current| Rc::ptr_eq(current, transfer))
        {
            active.take();
        }
    }
}

struct Transfer {
    request: HostRequestId,
    source: String,
    staging: tempfile::TempDir,
    native: RefCell<Option<webkit2gtk::Download>>,
    claimed: Cell<bool>,
    failed: Cell<bool>,
    done: Cell<bool>,
    cancel: Arc<AtomicBool>,
    shared: Arc<bridge::Shared>,
    events: Events,
    tab: HostTabId,
    epoch: u64,
    gate: Weak<Gate>,
}
impl Transfer {
    fn current(&self) -> bool {
        !self.cancel.load(Ordering::Acquire) && self.shared.epoch(self.tab) == Some(self.epoch)
    }
    fn fail(self: &Rc<Self>, message: &str) {
        if self.done.replace(true) {
            return;
        }
        self.failed.set(true);
        if let Some(native) = self.native.borrow().as_ref() {
            native.cancel();
        }
        self.shared.finish(self.request);
        self.events.emit(Event::OperationFailed {
            request: self.request,
            error: message.into(),
        });
        if let Some(gate) = self.gate.upgrade() {
            gate.retire(self);
        }
    }
    fn finish(self: &Rc<Self>) {
        if self.done.replace(true) {
            return;
        }
        let result = (|| -> Result<(String, Vec<u8>), String> {
            if !self.current() || !self.claimed.get() || self.failed.get() {
                return Err("Download was cancelled or browser ownership changed.".into());
            }
            let native = self
                .native
                .borrow()
                .clone()
                .ok_or("Browser download never started")?;
            if !native
                .response()
                .is_some_and(|response| (200..300).contains(&response.status_code()))
            {
                return Err("Download returned an unsuccessful HTTP response.".into());
            }
            let path = self.staging.path().join("payload");
            let metadata = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
            if !metadata.is_file()
                || metadata.file_type().is_symlink()
                || metadata.len() > MAX_BYTES
            {
                return Err("Downloaded file exceeded its safe bounds.".into());
            }
            let bytes = fs::read(path).map_err(|e| e.to_string())?;
            let name = native
                .response()
                .and_then(|response| response.suggested_filename())
                .map(|name| name.to_string())
                .filter(|name| valid_name(name))
                .unwrap_or_else(|| "download".into());
            Ok((name, bytes))
        })();
        self.shared.finish(self.request);
        match result {
            Ok((name, bytes)) => self.events.emit(Event::DownloadReady {
                request: self.request,
                token: format!("download_{}", self.request.0),
                name,
                bytes,
            }),
            Err(error) => self.events.emit(Event::OperationFailed {
                request: self.request,
                error,
            }),
        }
        if let Some(gate) = self.gate.upgrade() {
            gate.retire(self);
        }
    }
}

struct Upload {
    request: HostRequestId,
    path: String,
    _staging: tempfile::TempDir,
    cancel: Arc<AtomicBool>,
}

struct State {
    web: glib::WeakRef<webkit2gtk::WebView>,
    shared: Arc<bridge::Shared>,
    events: Events,
    tab: HostTabId,
    epoch: u64,
    gate: Rc<Gate>,
    upload: RefCell<Option<Upload>>,
    retained_uploads: RefCell<Vec<Upload>>,
    chooser_handler: RefCell<Option<glib::SignalHandlerId>>,
    download_hook: RefCell<Option<(glib::WeakRef<webkit2gtk::WebContext>, glib::SignalHandlerId)>>,
}
pub(super) struct Owner {
    state: Rc<State>,
}
impl Owner {
    pub(super) fn new(
        web: &webkit2gtk::WebView,
        shared: Arc<bridge::Shared>,
        events: Events,
        tab: HostTabId,
        epoch: u64,
        gate: Rc<Gate>,
    ) -> Self {
        let state = Rc::new(State {
            web: web.downgrade(),
            shared,
            events,
            tab,
            epoch,
            gate,
            upload: RefCell::new(None),
            retained_uploads: RefCell::new(Vec::new()),
            chooser_handler: RefCell::new(None),
            download_hook: RefCell::new(None),
        });
        let chooser_state = state.clone();
        let handler = web.connect_run_file_chooser(move |_, chooser| {
            let Some(upload) = chooser_state.upload.borrow_mut().take() else {
                chooser.cancel();
                return true;
            };
            if upload.cancel.load(Ordering::Acquire)
                || chooser_state.shared.epoch(chooser_state.tab) != Some(chooser_state.epoch)
            {
                chooser.cancel();
                chooser_state.shared.finish(upload.request);
                chooser_state.events.emit(Event::OperationFailed {
                    request: upload.request,
                    error: "Upload was cancelled or browser ownership changed.".into(),
                });
                return true;
            }
            chooser.select_files(&[upload.path.as_str()]);
            chooser_state.shared.finish(upload.request);
            chooser_state.events.emit(Event::Output {
                request: upload.request,
                output: Output::Done,
            });
            chooser_state.retained_uploads.borrow_mut().push(upload);
            true
        });
        *state.chooser_handler.borrow_mut() = Some(handler);

        if let Some(context) = web.context() {
            let weak_state = Rc::downgrade(&state);
            let handler = context.connect_download_started(move |_, native| {
                let Some(state) = weak_state.upgrade() else {
                    native.cancel();
                    return;
                };
                let Some(transfer) = state.gate.active.borrow().clone() else {
                    native.cancel();
                    return;
                };
                if !transfer.current()
                    || transfer.native.borrow().is_some()
                    || native
                        .web_view()
                        .is_none_or(|web| state.web.upgrade().as_ref() != Some(&web))
                    || native.request().and_then(|request| request.uri()).as_deref()
                        != Some(transfer.source.as_str())
                {
                    native.cancel();
                    return;
                }
                *transfer.native.borrow_mut() = Some(native.clone());
                let weak = Rc::downgrade(&transfer);
                native.connect_failed(move |_, _| {
                    if let Some(transfer) = weak.upgrade() {
                        transfer.failed.set(true);
                    }
                });
                let weak = Rc::downgrade(&transfer);
                native.connect_received_data(move |native, _| {
                    if let Some(transfer) = weak.upgrade()
                        && (native.received_data_length() > MAX_BYTES
                            || native
                                .response()
                                .is_some_and(|response| response.content_length() > MAX_BYTES)
                            || !transfer.current())
                    {
                        transfer.fail("Download exceeded 32 MiB or lost browser ownership.");
                    }
                });
                let weak = Rc::downgrade(&transfer);
                native.connect_finished(move |_| {
                    if let Some(transfer) = weak.upgrade() {
                        transfer.finish();
                    }
                });
            });
            *state.download_hook.borrow_mut() = Some((context.downgrade(), handler));
        }
        Self { state }
    }

    pub(super) fn prepare_upload(
        &self,
        request: HostRequestId,
        payload: UploadPayload,
        cancel: Arc<AtomicBool>,
    ) -> Result<(), String> {
        if self.state.upload.borrow().is_some()
            || payload.bytes.is_empty()
            || payload.bytes.len() > MAX_BYTES as usize
            || !valid_name(&payload.name)
        {
            return Err("Upload file is unavailable or another upload is active.".into());
        }
        let staging = tempfile::Builder::new()
            .prefix("synara-browser-upload-")
            .tempdir()
            .map_err(|error| error.to_string())?;
        let path = staging.path().join(&payload.name);
        fs::write(&path, payload.bytes).map_err(|error| error.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
                .map_err(|error| error.to_string())?;
        }
        let path = path
            .to_str()
            .ok_or("Temporary upload path is not UTF-8")?
            .to_owned();
        *self.state.upload.borrow_mut() = Some(Upload {
            request,
            path,
            _staging: staging,
            cancel,
        });
        Ok(())
    }

    pub(super) fn evaluate_upload(
        &self,
        web: &webkit2gtk::WebView,
        request: HostRequestId,
        script: &str,
    ) -> Result<(), String> {
        let state = self.state.clone();
        web.evaluate_javascript(
            script,
            Some(super::WORLD),
            None,
            None::<&gio::Cancellable>,
            move |result| {
                let accepted = result.ok().is_some_and(|value| {
                    value.is_string()
                        && serde_json::from_str::<serde_json::Value>(&value.to_str())
                            .ok()
                            .and_then(|json| json.get("kind").and_then(|v| v.as_str()).map(str::to_owned))
                            .as_deref()
                            == Some("upload_requested")
                });
                if !accepted
                    && let Some(upload) = state.upload.borrow_mut().take()
                    && upload.request == request
                {
                    state.shared.finish(request);
                    state.events.emit(Event::OperationFailed {
                        request,
                        error: "Page did not open the approved file chooser.".into(),
                    });
                }
            },
        );
        Ok(())
    }

    pub(super) fn evaluate_download(
        &self,
        web: &webkit2gtk::WebView,
        request: HostRequestId,
        document: &url::Url,
        script: &str,
        cancel: Arc<AtomicBool>,
    ) -> Result<(), String> {
        if self.state.gate.active.borrow().is_some() {
            return Err("Another task download is already active.".into());
        }
        let state = self.state.clone();
        let origin = document.origin();
        web.evaluate_javascript(
            script,
            Some(super::WORLD),
            None,
            None::<&gio::Cancellable>,
            move |result| {
                let target = result
                    .map_err(|error| error.to_string())
                    .and_then(|value| {
                        if !value.is_string() {
                            return Err("Malformed download target".into());
                        }
                        let json: serde_json::Value =
                            serde_json::from_str(&value.to_str()).map_err(|error| error.to_string())?;
                        if let Some(error) = json.get("error").and_then(|v| v.as_str()) {
                            return Err(error.chars().take(500).collect());
                        }
                        let raw = json
                            .get("url")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| "Page did not return a download target.".to_owned())?;
                        let parsed =
                            url::Url::parse(raw).map_err(|_| "Invalid download target".to_owned())?;
                        if parsed.origin() != origin || !matches!(parsed.scheme(), "http" | "https") {
                            return Err("Download target left the committed origin.".into());
                        }
                        Ok(parsed.to_string())
                    });
                let target = match target {
                    Ok(target) => target,
                    Err(error) => {
                        state.shared.finish(request);
                        state.events.emit(Event::OperationFailed { request, error });
                        return;
                    }
                };
                let staging = match tempfile::Builder::new()
                    .prefix("synara-browser-download-")
                    .tempdir()
                {
                    Ok(staging) => staging,
                    Err(error) => {
                        state.shared.finish(request);
                        state.events.emit(Event::OperationFailed {
                            request,
                            error: error.to_string(),
                        });
                        return;
                    }
                };
                let transfer = Rc::new(Transfer {
                    request,
                    source: target.clone(),
                    staging,
                    native: RefCell::new(None),
                    claimed: Cell::new(false),
                    failed: Cell::new(false),
                    done: Cell::new(false),
                    cancel,
                    shared: state.shared.clone(),
                    events: state.events.clone(),
                    tab: state.tab,
                    epoch: state.epoch,
                    gate: Rc::downgrade(&state.gate),
                });
                *state.gate.active.borrow_mut() = Some(transfer.clone());
                let weak = Rc::downgrade(&transfer);
                glib::timeout_add_local_once(Duration::from_secs(30), move || {
                    if let Some(transfer) = weak.upgrade() {
                        transfer.fail("Download timed out.");
                    }
                });
                let Some(web) = state.web.upgrade() else {
                    transfer.fail("Browser tab closed before download started.");
                    return;
                };
                if web.download_uri(&target).is_none() {
                    transfer.fail("Browser could not start the approved download.");
                }
            },
        );
        Ok(())
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        if let Some(web) = self.state.web.upgrade()
            && let Some(handler) = self.state.chooser_handler.borrow_mut().take()
        {
            web.disconnect(handler);
        }
        if let Some((context, handler)) = self.state.download_hook.borrow_mut().take()
            && let Some(context) = context.upgrade()
        {
            context.disconnect(handler);
        }
        if let Some(upload) = self.state.upload.borrow_mut().take() {
            self.state.shared.finish(upload.request);
            self.state.events.emit(Event::OperationFailed {
                request: upload.request,
                error: "Upload was cancelled because the browser tab changed.".into(),
            });
        }
        if let Some(transfer) = self.state.gate.active.borrow().clone() {
            transfer.fail("Download was cancelled because the browser tab changed.");
        }
    }
}
fn valid_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 240
        && !value.chars().any(char::is_control)
        && !value.contains(['/', '\\'])
}
