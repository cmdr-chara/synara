//! Linux/X11 WebKitGTK child surfaces for the existing Session owner.
//! Wry owns only the OS webview. There is no agent page RPC, WebDriver or second
//! session store. Manual tabs have an isolated one-way channel for value-free
//! JavaScript runtime error metadata.
mod bridge;
mod manual_capture;
mod manual_downloads;
mod surface;
#[cfg(test)]
mod tests;

use crate::{
    session::{Capabilities, Command, Event, NativePort, Output, RuntimeDiagnostic},
    *,
};
use gtk::{gio, prelude::*};
use javascriptcore::ValueExt;
use raw_window_handle::HasWindowHandle;
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    path::PathBuf,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
use webkit2gtk::SettingsExt;
use webkit2gtk::*;
use wry::{Rect, WebContext, WebView, WebViewBuilder, WebViewBuilderExtUnix, WebViewExtUnix};

/// Logical bounds supplied by GPUI after layout.
pub struct ViewportRect;
impl ViewportRect {
    pub fn logical(x: f32, y: f32, width: f32, height: f32) -> Rect {
        Rect {
            position: wry::dpi::LogicalPosition::new(f64::from(x), f64::from(y)).into(),
            size: wry::dpi::LogicalSize::new(f64::from(width.max(1.)), f64::from(height.max(1.)))
                .into(),
        }
    }
}
const MAX_TABS: usize = 64;
const MAX_RUNTIME_MESSAGES_PER_DOCUMENT: usize = 100;
const MAX_QUEUED_RUNTIME_DIAGNOSTICS: usize = 32;
const WORLD: &str = "synara-browser-actions-v1";
const MANUAL_RUNTIME_WORLD: &str = "synara-manual-runtime-diagnostics-v1";
const MANUAL_RUNTIME_HANDLER: &str = "synaraRuntimeDiagnostics";
const MANUAL_RUNTIME_SCRIPT: &str = r#"(() => {
  const handler = globalThis.webkit?.messageHandlers?.synaraRuntimeDiagnostics;
  if (!handler) return;
  let reported = 0;
  const report = (kind, event) => {
    if (reported >= 100) return;
    reported += 1;
    const line = Number.isSafeInteger(event?.lineno) ? event.lineno : undefined;
    const column = Number.isSafeInteger(event?.colno) ? event.colno : undefined;
    try {
      handler.postMessage(JSON.stringify({ kind, line, column }));
    } catch (_) {}
  };
  globalThis.addEventListener("error", event => {
    if (event instanceof ErrorEvent) report("uncaught_exception", event);
  }, true);
  globalThis.addEventListener("unhandledrejection", event => {
    report("unhandled_rejection", event);
  }, true);
})();"#;
const SCRIPT: &str = include_str!("../../../../assets/native-browser/actions.js");
struct Profile {
    context: WebContext,
    downloads: Rc<manual_downloads::Gate>,
    _directory: Option<tempfile::TempDir>,
}
struct NativeTab {
    webview: WebView,
    epoch: u64,
    partition: StoragePartition,
    document: Rc<RefCell<Option<CommittedDocument>>>,
    capture: Option<manual_capture::Owner>,
    downloads: Option<manual_downloads::Owner>,
}
impl Drop for NativeTab {
    fn drop(&mut self) {
        drop(self.capture.take());
        drop(self.downloads.take());
        let web = self.webview.webview();
        if self.partition == StoragePartition::Manual
            && let Some(inspector) = web.inspector()
        {
            inspector.close();
        }
        web.stop_loading();
    }
}
struct Running {
    tab: HostTabId,
    epoch: u64,
    flag: Arc<AtomicBool>,
    cancellable: gio::Cancellable,
}
#[derive(Clone)]
struct Events {
    sender: mpsc::SyncSender<Event>,
    overflow: Arc<AtomicBool>,
    runtime_pending: Arc<std::sync::atomic::AtomicUsize>,
}
impl Events {
    fn emit(&self, event: Event) {
        if self.sender.try_send(event).is_err() {
            self.overflow.store(true, Ordering::Release);
        }
    }
    /// Runtime events are best-effort telemetry. A noisy page must not turn a
    /// full event queue into a browser-host failure that closes unrelated tabs.
    fn emit_runtime_diagnostic(
        &self,
        tab: HostTabId,
        navigation: HostNavigationId,
        diagnostic: RuntimeDiagnostic,
    ) {
        if self
            .runtime_pending
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |pending| {
                (pending < MAX_QUEUED_RUNTIME_DIAGNOSTICS).then_some(pending + 1)
            })
            .is_err()
        {
            return;
        }
        if self
            .sender
            .try_send(Event::RuntimeDiagnostic {
                tab,
                navigation,
                diagnostic,
            })
            .is_err()
        {
            self.runtime_pending.fetch_sub(1, Ordering::AcqRel);
        }
    }
}
/// This value stays on the main thread. Only `Port` is Send.
pub struct NativeHost {
    views: BTreeMap<HostTabId, NativeTab>,
    profiles: BTreeMap<String, Profile>,
    tabs: BTreeMap<HostTabId, StoragePartition>,
    running: Rc<RefCell<BTreeMap<HostRequestId, Running>>>,
    commands: mpsc::Receiver<bridge::Delivery>,
    shared: Arc<bridge::Shared>,
    events: Events,
    receiver: mpsc::Receiver<Event>,
    root: PathBuf,
    surface: Option<surface::ChildSurface>,
    initialized: bool,
    error: Option<String>,
    selected: Option<HostTabId>,
    viewport: Option<Rect>,
}
impl NativeHost {
    pub fn new(root: PathBuf) -> (Self, Box<dyn NativePort>) {
        let (port, commands, shared) = bridge::channel();
        let (sender, receiver) = mpsc::sync_channel(256);
        (
            Self {
                views: BTreeMap::new(),
                profiles: BTreeMap::new(),
                tabs: BTreeMap::new(),
                running: Rc::new(RefCell::new(BTreeMap::new())),
                commands,
                shared,
                events: Events {
                    sender,
                    overflow: Arc::new(AtomicBool::new(false)),
                    runtime_pending: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
                },
                receiver,
                root,
                surface: None,
                initialized: false,
                error: None,
                selected: None,
                viewport: None,
            },
            Box::new(port),
        )
    }
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
    pub fn open_manual_inspector(&self, tab: HostTabId) -> Result<()> {
        let view = self.views.get(&tab).ok_or(BrowserError::MissingTab)?;
        if view.partition != StoragePartition::Manual {
            return Err(BrowserError::WrongContext);
        }
        let inspector = view
            .webview
            .webview()
            .inspector()
            .ok_or(BrowserError::Unavailable)?;
        inspector.show();
        Ok(())
    }
    pub fn has_tabs(&self) -> bool {
        !self.tabs.is_empty()
    }
    fn initialize(&mut self, parent: &impl HasWindowHandle) {
        if self.initialized {
            return;
        }
        self.initialized = true;
        let result = (|| -> std::result::Result<(), String> {
            let handle = parent.window_handle().map_err(|e| e.to_string())?;
            let parent_id = surface::parent_id(handle.as_raw())?;
            // GPUI does not use GDK. Select X11 before initializing this child-widget toolkit.
            gtk::gdk::set_allowed_backends("x11");
            gtk::init().map_err(|e| format!("WebKitGTK initialization failed: {e}"))?;
            private_directory(&self.root)?;
            self.surface = Some(surface::ChildSurface::new(parent_id)?);
            Ok(())
        })();
        match result {
            Ok(()) => self.shared.ready.store(true, Ordering::Release),
            Err(e) => self.error = Some(e),
        }
    }
    /// Drive a bounded amount of GTK work alongside GPUI, never a nested blocking event loop.
    pub fn pump(&mut self, parent: &impl HasWindowHandle) -> Vec<Event> {
        self.initialize(parent);
        if self.shared.ready.load(Ordering::Acquire) {
            self.reap();
            let container = self
                .surface
                .as_ref()
                .expect("initialized surface")
                .container
                .clone();
            for _ in 0..64 {
                let Ok(delivery) = self.commands.try_recv() else {
                    break;
                };
                self.dispatch(delivery, |builder| builder.build_gtk(&container));
            }
            let until = Instant::now() + Duration::from_millis(4);
            for _ in 0..32 {
                if Instant::now() >= until || !gtk::events_pending() {
                    break;
                }
                gtk::main_iteration_do(false);
            }
            self.reap();
        }
        self.drain_events()
    }
    fn drain_events(&mut self) -> Vec<Event> {
        let mut events: Vec<_> = self.receiver.try_iter().take(256).collect();
        let runtime_count = events
            .iter()
            .filter(|event| matches!(event, Event::RuntimeDiagnostic { .. }))
            .count();
        if runtime_count > 0 {
            self.events
                .runtime_pending
                .fetch_sub(runtime_count, Ordering::AcqRel);
        }
        if self.events.overflow.swap(false, Ordering::AcqRel) {
            self.shared.ready.store(false, Ordering::Release);
            self.error = Some(
                "Native browser event limit exceeded. Close and restart Synara before retrying."
                    .into(),
            );
            events.clear();
            events.extend(self.tabs.keys().map(|tab| Event::Crashed { tab: *tab }));
            self.views.clear();
            self.sync_surface();
        }
        events
    }
    fn reap(&mut self) {
        self.views
            .retain(|tab, view| self.shared.epoch(*tab) == Some(view.epoch));
        self.tabs.retain(|tab, _| self.shared.epoch(*tab).is_some());
        self.running.borrow_mut().retain(|id, run| {
            if run.flag.load(Ordering::Acquire) || self.shared.epoch(run.tab) != Some(run.epoch) {
                run.cancellable.cancel();
                self.shared.finish(*id);
                false
            } else {
                true
            }
        });
        // Drop a task's data only after its last view is destroyed.
        self.profiles
            .retain(|key, _| self.tabs.values().any(|p| profile_key(*p) == *key));
        self.sync_surface();
    }
    /// Selection alone does not imply rendered content. Blank, stopped and
    /// cancelled tabs have no live WebKit view, even while their tab row exists.
    fn visible_bounds(&self) -> Option<Rect> {
        let tab = self.selected?;
        let view = self.views.get(&tab)?;
        if !self.shared.ready.load(Ordering::Acquire) || self.shared.epoch(tab) != Some(view.epoch)
        {
            return None;
        }
        self.viewport
    }
    fn sync_surface(&self) {
        if let Some(surface) = &self.surface {
            // Hiding only the widget leaves the shared X11 parent's backing
            // pixels visible. Unmap that surface when there is no live view.
            surface.viewport(self.visible_bounds());
        }
    }
    pub fn viewport(&mut self, tab: Option<HostTabId>, bounds: Option<Rect>) {
        self.selected = tab;
        self.viewport = bounds;
        let visible_bounds = self.visible_bounds();
        if visible_bounds.is_none() {
            self.sync_surface();
        }
        for (id, view) in &self.views {
            let visible = Some(*id) == tab && visible_bounds.is_some();
            if visible && let Some(bounds) = bounds {
                let _ = view.webview.set_bounds(content_bounds(bounds));
            }
            let _ = view.webview.set_visible(visible);
        }
        self.sync_surface();
    }
    fn context(
        &mut self,
        partition: StoragePartition,
    ) -> std::result::Result<&mut Profile, String> {
        let key = profile_key(partition);
        if !self.profiles.contains_key(&key) {
            let (path, directory) = if partition == StoragePartition::Manual {
                let path = self.root.join("manual");
                private_directory(&path)?;
                (path, None)
            } else {
                let dir = tempfile::Builder::new()
                    .prefix("isolated-")
                    .tempdir_in(&self.root)
                    .map_err(|e| e.to_string())?;
                (dir.path().to_path_buf(), Some(dir))
            };
            let mut context = WebContext::new(Some(path));
            context.set_allows_automation(false);
            self.profiles.insert(
                key.clone(),
                Profile {
                    context,
                    downloads: Rc::new(manual_downloads::Gate::default()),
                    _directory: directory,
                },
            );
        }
        Ok(self.profiles.get_mut(&key).expect("inserted profile"))
    }
    fn dispatch(
        &mut self,
        delivery: bridge::Delivery,
        build: impl FnOnce(WebViewBuilder<'_>) -> wry::Result<WebView>,
    ) {
        let epoch = delivery.epoch;
        match delivery.command {
            Command::Open { tab, partition } => {
                if self.shared.epoch(tab).is_some() {
                    self.tabs.insert(tab, partition);
                }
            }
            Command::Close { .. } | Command::Stop { .. } | Command::Cancel { .. } => self.reap(),
            Command::Navigate {
                tab,
                navigation,
                document,
                partition,
                allowed_origin,
            } => {
                if self.shared.epoch(tab) != Some(epoch) {
                    return;
                }
                if self.tabs.get(&tab) != Some(&partition) {
                    return;
                }
                let result = self.navigate(
                    tab,
                    epoch,
                    navigation,
                    document,
                    partition,
                    allowed_origin,
                    build,
                );
                if let Err(error) = result {
                    self.events.emit(Event::Failed {
                        tab,
                        navigation,
                        error,
                    });
                }
            }
            Command::Operation {
                request,
                command,
                max_output_bytes,
            } => {
                if delivery.cancelled.load(Ordering::Acquire)
                    || self.shared.epoch(command.tab) != Some(epoch)
                {
                    self.shared.finish(request);
                    return;
                }
                if let Err(error) = self.operation(
                    request,
                    command,
                    epoch,
                    delivery.cancelled,
                    max_output_bytes,
                ) {
                    self.shared.finish(request);
                    self.events.emit(Event::OperationFailed { request, error });
                }
            }
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn navigate(
        &mut self,
        tab: HostTabId,
        epoch: u64,
        navigation: HostNavigationId,
        document: CommittedDocument,
        partition: StoragePartition,
        allowed: Option<CanonicalOrigin>,
        build: impl FnOnce(WebViewBuilder<'_>) -> wry::Result<WebView>,
    ) -> std::result::Result<(), String> {
        document.validate().map_err(|e| e.to_string())?;
        let agent = matches!(partition, StoragePartition::AgentTask(_));
        let manual = partition == StoragePartition::Manual;
        let popup_review = matches!(
            partition,
            StoragePartition::Manual | StoragePartition::Authentication(_)
        );
        if agent && allowed.as_ref() != Some(&document.origin) {
            return Err("Missing approved navigation origin".into());
        }
        self.views.remove(&tab);
        self.sync_surface();
        let shared = self.shared.clone();
        let events = self.events.clone();
        let completed = Rc::new(Cell::new(false));
        let finished = completed.clone();
        let popup_ready = completed.clone();
        let popup_shared = shared.clone();
        let popup_events = events.clone();
        let allowed_navigation = allowed.clone();
        let downloads = self.context(partition)?.downloads.clone();
        let download_route = downloads.clone();
        let builder = WebViewBuilder::new_with_web_context(&mut self.context(partition)?.context)
            .with_focused(false)
            .with_visible(false)
            .with_devtools(manual)
            .with_new_window_req_handler(move |url, _| {
                if popup_review
                    && popup_ready.get()
                    && popup_shared.epoch(tab) == Some(epoch)
                {
                    popup_events.emit(Event::PopupRequested {
                        tab,
                        navigation,
                        url,
                    });
                }
                wry::NewWindowResponse::Deny
            })
            .with_download_started_handler(move |url, path| {
                manual && download_route.destination(&url, path)
            })
            .with_navigation_handler(move |url| {
                if shared.epoch(tab) != Some(epoch) {
                    return false;
                }
                let Ok(doc) = CommittedDocument::parse(&url) else {
                    return false;
                };
                if finished.get() {
                    if partition == StoragePartition::Manual {
                        events.emit(Event::ManualNavigation {
                            tab,
                            navigation,
                            url: doc.canonical_url,
                        });
                    }
                    return false;
                }
                let allow = approved_origin_allows(allowed_navigation.as_ref(), &doc);
                if !allow {
                    events.emit(Event::Failed {
                        tab,
                        navigation,
                        error: "Navigation to an unapproved origin was denied".into(),
                    });
                }
                allow
            });
        let view =
            build(builder).map_err(|e| format!("Could not create native WebKit view: {e}"))?;
        let web = view.webview();
        harden(&web, partition);
        if partition == StoragePartition::Manual {
            observe_manual_runtime_diagnostics(
                &web,
                self.shared.clone(),
                self.events.clone(),
                tab,
                navigation,
                epoch,
            );
            let shared = self.shared.clone();
            let events = self.events.clone();
            web.connect_resource_load_started(move |_, resource, _| {
                let done = Rc::new(Cell::new(false));
                let finished = done.clone();
                let shared_finished = shared.clone();
                let events_finished = events.clone();
                resource.connect_finished(move |resource| {
                    if finished.replace(true) || shared_finished.epoch(tab) != Some(epoch) {
                        return;
                    }
                    if let Some(uri) = resource.uri() {
                        events_finished.emit(Event::NetworkDiagnostic {
                            tab,
                            navigation,
                            url: uri.to_string(),
                            status: resource
                                .response()
                                .and_then(|response| u16::try_from(response.status_code()).ok()),
                            error: None,
                        });
                    }
                });
                let shared_failed = shared.clone();
                let events_failed = events.clone();
                resource.connect_failed(move |resource, _| {
                    if done.replace(true) || shared_failed.epoch(tab) != Some(epoch) {
                        return;
                    }
                    if let Some(uri) = resource.uri() {
                        events_failed.emit(Event::NetworkDiagnostic {
                            tab,
                            navigation,
                            url: uri.to_string(),
                            status: None,
                            error: Some("Request failed".into()),
                        });
                    }
                });
            });
        }
        if agent {
            // Retain the named world for the lifetime of this document, not only an
            // individual evaluation. A page-world script cannot modify its inventory.
            let manager = web
                .user_content_manager()
                .ok_or("Native script world is unavailable")?;
            manager.add_script(&webkit2gtk::UserScript::for_world(
                "globalThis.__synaraRefs = new Map();",
                webkit2gtk::UserContentInjectedFrames::TopFrame,
                webkit2gtk::UserScriptInjectionTime::Start,
                WORLD,
                &[],
                &[],
            ));
        }
        let committed = Rc::new(RefCell::new(None));
        let current_document = committed.clone();
        let shared = self.shared.clone();
        let events = self.events.clone();
        let done = completed.clone();
        web.connect_load_changed(move |web, event| {
            if event != webkit2gtk::LoadEvent::Finished
                || done.get()
                || shared.epoch(tab) != Some(epoch)
            {
                return;
            }
            let Some(uri) = web.uri() else {
                return;
            };
            let Ok(doc) = CommittedDocument::parse(uri.as_str()) else {
                return;
            };
            if allowed.as_ref().is_some_and(|origin| origin != &doc.origin) {
                events.emit(Event::Failed {
                    tab,
                    navigation,
                    error: "Navigation escaped its approved origin".into(),
                });
                return;
            }
            done.set(true);
            *current_document.borrow_mut() = Some(doc.clone());
            events.emit(Event::Committed {
                tab,
                navigation,
                url: doc.canonical_url,
                title: bounded_title(web.title().as_deref().unwrap_or("")),
            });
        });
        let shared = self.shared.clone();
        let events = self.events.clone();
        let failed = completed.clone();
        web.connect_load_failed(move |_, _, _, error| {
            if shared.epoch(tab) == Some(epoch) && !failed.replace(true) {
                events.emit(Event::Failed {
                    tab,
                    navigation,
                    error: error.to_string().chars().take(500).collect(),
                });
            }
            true // Suppress engine-generated error pages, keep the native error row authoritative.
        });
        let shared = self.shared.clone();
        let events = self.events.clone();
        web.connect_web_process_terminated(move |_, _| {
            if shared.epoch(tab) == Some(epoch) {
                events.emit(Event::DocumentCrashed { tab, navigation });
            }
        });
        let shared = self.shared.clone();
        let events = self.events.clone();
        web.connect_title_notify(move |web| {
            if completed.get() && shared.epoch(tab) == Some(epoch) {
                events.emit(Event::Title {
                    tab,
                    navigation,
                    title: bounded_title(web.title().as_deref().unwrap_or("")),
                });
            }
        });
        let capture =
            manual.then(|| manual_capture::Owner::new(&web, self.shared.clone(), tab, epoch));
        let downloads = manual.then(|| {
            manual_downloads::Owner::new(&web, self.shared.clone(), tab, epoch, downloads)
        });
        // The native navigation policy is installed before the first network request.
        // Acceptance tests assert that a cross-origin redirect never reaches its target.
        web.load_uri(&document.canonical_url);
        self.views.insert(
            tab,
            NativeTab {
                webview: view,
                epoch,
                partition,
                document: committed,
                capture,
                downloads,
            },
        );
        // Creation may occur after the canvas selected a blank tab. Reapply its
        // requested geometry now that a view exists so first navigation maps it.
        self.viewport(self.selected, self.viewport);
        Ok(())
    }
    fn operation(
        &mut self,
        request: HostRequestId,
        command: NativeCommand,
        epoch: u64,
        flag: Arc<AtomicBool>,
        max_bytes: usize,
    ) -> std::result::Result<(), String> {
        command.validate().map_err(|e| e.to_string())?;
        let view = self
            .views
            .get(&command.tab)
            .ok_or("Native view is not ready")?;
        if view.partition != command.partition
            || !matches!(view.partition, StoragePartition::AgentTask(_))
        {
            return Err("Wrong native storage partition".into());
        }
        let document = view
            .document
            .borrow()
            .clone()
            .ok_or("Native document is not ready")?;
        if !matches!(
            command.operation,
            BrowserOperation::ReadDocument
                | BrowserOperation::Click { .. }
                | BrowserOperation::Fill { .. }
                | BrowserOperation::Input {
                    event: InputEvent::Scroll { .. }
                }
        ) {
            return Err("Native operation is not supported".into());
        }
        let origin = url::Url::parse(&document.canonical_url)
            .map_err(|e| e.to_string())?
            .origin()
            .ascii_serialization();
        let args = serde_json::json!({"origin": origin, "nonce": request.0.to_string(), "operation": command.operation});
        let script = format!("{SCRIPT}({args})");
        let cancelled = gio::Cancellable::new();
        let running = self.running.clone();
        running.borrow_mut().insert(
            request,
            Running {
                tab: command.tab,
                epoch,
                flag: flag.clone(),
                cancellable: cancelled.clone(),
            },
        );
        let shared = self.shared.clone();
        let events = self.events.clone();
        view.webview.webview().evaluate_javascript(
            &script,
            Some(WORLD),
            None,
            Some(&cancelled),
            move |result| {
                running.borrow_mut().remove(&request);
                shared.finish(request);
                if flag.load(Ordering::Acquire) || shared.epoch(command.tab) != Some(epoch) {
                    return;
                }
                let output = result.map_err(|e| e.to_string()).and_then(|value| {
                    if !value.is_string() {
                        return Err("Malformed native action result".into());
                    }
                    let text = value.to_str();
                    if text.len() > max_bytes.min(MAX_IPC_FRAME_BYTES) {
                        return Err("Native result exceeds its byte limit".into());
                    }
                    let json: serde_json::Value =
                        serde_json::from_str(&text).map_err(|e| e.to_string())?;
                    if let Some(error) = json.get("error").and_then(|e| e.as_str()) {
                        return Err(error.chars().take(500).collect());
                    }
                    serde_json::from_value::<Output>(json).map_err(|e| e.to_string())
                });
                match output {
                    Ok(output) => events.emit(Event::Output { request, output }),
                    Err(error) => events.emit(Event::OperationFailed { request, error }),
                }
            },
        );
        Ok(())
    }
}
impl Drop for NativeHost {
    fn drop(&mut self) {
        self.shared.ready.store(false, Ordering::Release);
        for run in self.running.borrow().values() {
            run.flag.store(true, Ordering::Release);
            run.cancellable.cancel();
        }
        self.views.clear();
        self.profiles.clear();
        self.surface.take();
    }
}
fn harden(web: &webkit2gtk::WebView, partition: StoragePartition) {
    // Only human-operated tabs receive interactive native dialogs and inspection.
    // Authentication is a separate partition, not an implicit grant of manual authority.
    let manual = partition == StoragePartition::Manual;
    if let Some(settings) = webkit2gtk::WebViewExt::settings(web) {
        settings.set_enable_developer_extras(manual);
        settings.set_javascript_can_access_clipboard(false);
        settings.set_javascript_can_open_windows_automatically(false);
        settings.set_allow_file_access_from_file_urls(false);
        settings.set_allow_universal_access_from_file_urls(false);
    }
    web.connect_permission_request(|_, permission| {
        permission.deny();
        true
    });
    web.connect_run_file_chooser(move |_, request| {
        if manual {
            // Let WebKit present its native picker. No path is selected on the user's behalf.
            false
        } else {
            request.cancel();
            true
        }
    });
    web.connect_enter_fullscreen(|_| true);
    web.connect_script_dialog(move |_, dialog| {
        if manual {
            // Preserve the engine's confirm/prompt UI instead of silently dismissing forms.
            false
        } else {
            dialog.close();
            true
        }
    });
    if matches!(partition, StoragePartition::AgentTask(_)) {
        web.set_sensitive(false);
    }
}
fn observe_manual_runtime_diagnostics(
    web: &webkit2gtk::WebView,
    shared: Arc<bridge::Shared>,
    events: Events,
    tab: HostTabId,
    navigation: HostNavigationId,
    epoch: u64,
) {
    let Some(manager) = WebViewExt::user_content_manager(web) else {
        return;
    };
    if !manager
        .register_script_message_handler_in_world(MANUAL_RUNTIME_HANDLER, MANUAL_RUNTIME_WORLD)
    {
        return;
    }
    let reported = Rc::new(Cell::new(0usize));
    let reported_by_handler = reported.clone();
    let current_session = shared.clone();
    manager.connect_script_message_received(Some(MANUAL_RUNTIME_HANDLER), move |_, result| {
        if current_session.epoch(tab) != Some(epoch)
            || reported_by_handler.get() >= MAX_RUNTIME_MESSAGES_PER_DOCUMENT
        {
            return;
        }
        let Some(value) = result.js_value() else {
            return;
        };
        if !value.is_string() {
            return;
        }
        let raw = value.to_str();
        if raw.len() > 128 {
            return;
        }
        let Ok(diagnostic) = serde_json::from_str::<RuntimeDiagnostic>(&raw) else {
            return;
        };
        if !diagnostic.is_valid() {
            return;
        }
        reported_by_handler.set(reported_by_handler.get() + 1);
        events.emit_runtime_diagnostic(tab, navigation, diagnostic);
    });
    manager.add_script(&webkit2gtk::UserScript::for_world(
        MANUAL_RUNTIME_SCRIPT,
        webkit2gtk::UserContentInjectedFrames::AllFrames,
        webkit2gtk::UserScriptInjectionTime::Start,
        MANUAL_RUNTIME_WORLD,
        &[],
        &[],
    ));
}
fn profile_key(partition: StoragePartition) -> String {
    match partition {
        StoragePartition::Manual => "manual".into(),
        StoragePartition::AgentTask(id) => format!("task-{id}"),
        StoragePartition::Authentication(id) => format!("auth-{id}"),
    }
}
fn bounded_title(title: &str) -> String {
    title
        .chars()
        .filter(|c| !c.is_control())
        .take(512)
        .collect()
}
fn approved_origin_allows(allowed: Option<&CanonicalOrigin>, document: &CommittedDocument) -> bool {
    allowed.is_none_or(|origin| origin == &document.origin)
}

fn private_directory(path: &std::path::Path) -> std::result::Result<(), String> {
    use std::{fs, io::ErrorKind, os::unix::fs::PermissionsExt};
    match fs::symlink_metadata(path) {
        Ok(meta) if !meta.is_dir() || meta.file_type().is_symlink() => {
            return Err("Browser data path must be a directory, not a symbolic link.".into());
        }
        Ok(_) => (),
        Err(e) if e.kind() == ErrorKind::NotFound => {
            fs::create_dir_all(path).map_err(|e| e.to_string())?
        }
        Err(e) => return Err(e.to_string()),
    }
    let meta = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err("Browser data directory changed while opening it.".into());
    }
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|e| e.to_string())
}

/// The outer child is already positioned at the pane origin. The WebKit widget
/// must occupy local coordinates inside its fixed container, not repeat that offset.
fn content_bounds(bounds: Rect) -> Rect {
    Rect {
        position: wry::dpi::LogicalPosition::new(0., 0.).into(),
        size: bounds.size,
    }
}
