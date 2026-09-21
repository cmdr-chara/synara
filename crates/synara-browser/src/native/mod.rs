//! Linux/X11 WebKitGTK child surfaces for the existing Session owner.
//! Wry owns only the OS webview. There is no page IPC, WebDriver or second session store.
mod bridge;
#[cfg(test)]
mod tests;

use crate::{
    session::{Capabilities, Command, Event, NativePort, Output},
    *,
};
use gtk::{gio, glib, prelude::*};
use javascriptcore::ValueExt;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
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
use webkit2gtk::*;
use wry::{Rect, WebContext, WebView, WebViewBuilder, WebViewExtUnix};

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
const WORLD: &str = "synara-browser-actions-v1";
const SCRIPT: &str = include_str!("actions.js");
struct Profile {
    context: WebContext,
    _directory: Option<tempfile::TempDir>,
}
struct NativeTab {
    webview: WebView,
    epoch: u64,
    partition: StoragePartition,
    document: Rc<RefCell<Option<CommittedDocument>>>,
    loading: gio::Cancellable,
    _filter_directory: Option<Rc<tempfile::TempDir>>,
}
impl Drop for NativeTab {
    fn drop(&mut self) {
        self.loading.cancel();
        self.webview.webview().stop_loading();
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
}
impl Events {
    fn emit(&self, event: Event) {
        if self.sender.try_send(event).is_err() {
            self.overflow.store(true, Ordering::Release);
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
                },
                receiver,
                root,
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
            if !matches!(
                handle.as_raw(),
                RawWindowHandle::Xlib(_) | RawWindowHandle::Xcb(_)
            ) {
                return Err("Embedded browsing currently requires Linux/X11. Start Synara with its X11 backend. Native Wayland, Windows and macOS host acceptance remains open.".into());
            }
            // GPUI does not use GDK. Select X11 before initializing this child-widget toolkit.
            gtk::gdk::set_allowed_backends("x11");
            gtk::init().map_err(|e| format!("WebKitGTK initialization failed: {e}"))?;
            std::fs::create_dir_all(&self.root).map_err(|e| e.to_string())?;
            if std::fs::symlink_metadata(&self.root)
                .map_err(|e| e.to_string())?
                .file_type()
                .is_symlink()
            {
                return Err("Browser data directory must not be a symbolic link.".into());
            }
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&self.root, std::fs::Permissions::from_mode(0o700))
                .map_err(|e| e.to_string())?;
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
            for _ in 0..64 {
                let Ok(delivery) = self.commands.try_recv() else {
                    break;
                };
                self.dispatch(delivery, |builder| builder.build_as_child(parent));
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
        if self.events.overflow.swap(false, Ordering::AcqRel) {
            self.shared.ready.store(false, Ordering::Release);
            self.error = Some(
                "Native browser event limit exceeded. Close and restart Synara before retrying."
                    .into(),
            );
            events.clear();
            events.extend(self.tabs.keys().map(|tab| Event::Crashed { tab: *tab }));
            self.views.clear();
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
    }
    pub fn viewport(&mut self, tab: Option<HostTabId>, bounds: Option<Rect>) {
        self.selected = tab;
        self.viewport = bounds;
        for (id, view) in &self.views {
            let visible = Some(*id) == tab && bounds.is_some();
            if visible {
                if let Some(bounds) = bounds {
                    let _ = view.webview.set_bounds(bounds);
                }
            }
            let _ = view.webview.set_visible(visible);
        }
    }
    fn context(
        &mut self,
        partition: StoragePartition,
    ) -> std::result::Result<&mut Profile, String> {
        let key = profile_key(partition);
        if !self.profiles.contains_key(&key) {
            let (path, directory) = if partition == StoragePartition::Manual {
                (self.root.join("manual"), None)
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
        if agent && allowed.as_ref() != Some(&document.origin) {
            return Err("Missing approved navigation origin".into());
        }
        self.views.remove(&tab);
        let shared = self.shared.clone();
        let events = self.events.clone();
        let completed = Rc::new(Cell::new(false));
        let finished = completed.clone();
        let allowed_navigation = allowed.clone();
        let builder = WebViewBuilder::new()
            .with_focused(false)
            .with_visible(false)
            .with_devtools(false)
            .with_web_context(&mut self.context(partition)?.context)
            .with_new_window_req_handler(|_, _| wry::NewWindowResponse::Deny)
            .with_download_started_handler(|_, _| false)
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
                            url: doc.canonical_url,
                        });
                    }
                    return false;
                }
                allowed_navigation
                    .as_ref()
                    .is_none_or(|origin| origin == &doc.origin)
            });
        let view =
            build(builder).map_err(|e| format!("Could not create native WebKit view: {e}"))?;
        let web = view.webview();
        harden(&web, agent);
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
                events.emit(Event::Crashed { tab });
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
        let loading = gio::Cancellable::new();
        let mut filter_directory = None;
        if agent {
            // A content blocker, installed BEFORE load_uri, also fences HTTP redirects.
            // A decide-policy callback alone is not evidence of a pre-network redirect boundary.
            let rules = origin_rules(&document);
            let directory = Rc::new(
                tempfile::Builder::new()
                    .prefix("filter-")
                    .tempdir_in(&self.root)
                    .map_err(|e| e.to_string())?,
            );
            let filter_path = directory.path().to_path_buf();
            filter_directory = Some(directory.clone());
            let store = webkit2gtk::UserContentFilterStore::new(
                filter_path.to_str().ok_or("Invalid filter path")?,
            );
            let shared = self.shared.clone();
            let events = self.events.clone();
            let target = document.canonical_url;
            let web = web.clone();
            store.save(
                &format!("navigation-{}", navigation.0),
                &glib::Bytes::from_owned(rules),
                Some(&loading),
                move |result| {
                    let _directory = directory;
                    if shared.epoch(tab) != Some(epoch) {
                        return;
                    }
                    match result {
                        Ok(filter) => {
                            if let Some(manager) = web.user_content_manager() {
                                manager.add_filter(&filter);
                                web.load_uri(&target);
                            } else {
                                events.emit(Event::Failed {
                                    tab,
                                    navigation,
                                    error: "Native content filter manager is unavailable".into(),
                                });
                            }
                        }
                        Err(error) => events.emit(Event::Failed {
                            tab,
                            navigation,
                            error: format!("Could not enforce navigation filter: {error}"),
                        }),
                    }
                },
            );
        } else {
            web.load_uri(&document.canonical_url);
        }
        if self.selected == Some(tab) {
            if let Some(bounds) = self.viewport {
                let _ = view.set_bounds(bounds);
                let _ = view.set_visible(true);
            }
        }
        self.views.insert(
            tab,
            NativeTab {
                webview: view,
                epoch,
                partition,
                document: committed,
                loading,
                _filter_directory: filter_directory,
            },
        );
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
    }
}
fn harden(web: &webkit2gtk::WebView, agent: bool) {
    if let Some(settings) = webkit2gtk::WebViewExt::settings(web) {
        settings.set_enable_developer_extras(false);
        settings.set_javascript_can_access_clipboard(false);
        settings.set_javascript_can_open_windows_automatically(false);
        settings.set_allow_file_access_from_file_urls(false);
        settings.set_allow_universal_access_from_file_urls(false);
    }
    web.connect_permission_request(|_, permission| {
        permission.deny();
        true
    });
    web.connect_run_file_chooser(|_, request| {
        request.cancel();
        true
    });
    web.connect_enter_fullscreen(|_| true);
    web.connect_script_dialog(|_, dialog| {
        dialog.close();
        true
    });
    if agent {
        web.set_sensitive(false);
    }
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
fn origin_rules(document: &CommittedDocument) -> Vec<u8> {
    let origin = url::Url::parse(&document.canonical_url)
        .expect("validated document")
        .origin()
        .ascii_serialization();
    let mut pattern = String::from("^");
    for ch in origin.chars() {
        if ".*+?()[]{}^$|\\".contains(ch) {
            pattern.push('\\');
        }
        pattern.push(ch);
    }
    pattern.push('/');
    serde_json::to_vec(&serde_json::json!([
        {"trigger":{"url-filter":".*","resource-type":["document"]},"action":{"type":"block"}},
        {"trigger":{"url-filter":pattern,"resource-type":["document"]},"action":{"type":"ignore-previous-rules"}}
    ])).expect("static filter schema")
}
