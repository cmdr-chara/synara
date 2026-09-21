//! The single browser owner. Native adapters enqueue commands and report trusted
//! events. Neither page scripts nor agent RPC may resolve approval requests.
use super::*;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct Capabilities {
    pub navigation: bool, pub document: bool, pub input: bool,
    pub capture: bool, pub downloads: bool,
}
/// Nonblocking native adapter. Enforce partitions and limits. Block agent
/// cross-origin redirects BEFORE networking. No arbitrary host IPC or evaluation.
pub trait NativePort: Send {
    fn capabilities(&self) -> Capabilities;
    fn send(&mut self, command: Command) -> Result<()>;
}
pub struct UnavailablePort;
impl NativePort for UnavailablePort {
    fn capabilities(&self) -> Capabilities { Capabilities::default() }
    fn send(&mut self, _: Command) -> Result<()> { Err(BrowserError::Unavailable) }
}
#[derive(Clone, Debug, Serialize)]
pub enum Command {
    Open { tab: HostTabId, partition: StoragePartition },
    Close { tab: HostTabId },
    Navigate { tab: HostTabId, navigation: HostNavigationId, document: CommittedDocument,
        partition: StoragePartition, allowed_origin: Option<CanonicalOrigin> },
    Stop { tab: HostTabId },
    Operation { request: HostRequestId, command: NativeCommand, max_output_bytes: usize },
    Cancel { request: HostRequestId },
}
#[derive(Clone, Debug, Serialize)]
pub struct TabView {
    pub id: HostTabId, pub profile: BrowserProfile, pub url: Option<String>,
    pub title: String, pub state: String, pub error: Option<String>,
    pub back: bool, pub forward: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Element { pub id: String, pub role: String, pub name: String }
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Output {
    Done,
    Document { text: String, elements: Vec<Element> },
    /// Scoped opaque handles, never arbitrary local paths.
    Capture { token: String, bytes: usize },
    Download { token: String, bytes: usize },
}
impl Output {
    fn validate(&self, operation: &BrowserOperation) -> Result<()> {
        let token = |v: &str| !v.is_empty() && v.len() <= 128
            && v.bytes().all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b));
        match (self, operation) {
            (Self::Done, BrowserOperation::Navigate { .. } | BrowserOperation::Click { .. }
                | BrowserOperation::Fill { .. } | BrowserOperation::Input { .. }) => (),
            (Self::Document { text, elements }, BrowserOperation::ReadDocument) => {
                if text.len() > 64 * 1024 || elements.len() > 512 { return Err(BrowserError::Limit); }
                let mut ids = BTreeSet::new();
                let mut bytes = text.len();
                for e in elements {
                    if !token(&e.id) || e.role.len() > 128 || e.name.len() > 1024
                        || !ids.insert(e.id.as_str()) { return Err(BrowserError::Invalid); }
                    bytes += e.id.len() + e.role.len() + e.name.len();
                }
                if bytes > 192 * 1024 { return Err(BrowserError::Limit); }
            },
            (Self::Capture { token: t, bytes }, BrowserOperation::Screenshot { .. })
                if token(t) && *bytes <= 8 * 1024 * 1024 => (),
            (Self::Download { token: t, bytes }, BrowserOperation::Download { .. })
                if token(t) && *bytes <= 32 * 1024 * 1024 => (),
            _ => return Err(BrowserError::Invalid),
        }
        if serde_json::to_vec(self).map_err(|_| BrowserError::Invalid)?.len() > 256 * 1024 {
            return Err(BrowserError::Limit);
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "state", content = "result", rename_all = "snake_case")]
pub enum RequestState { AwaitingConsent, Running, Complete(Output), Denied, Cancelled, Failed(String) }
impl RequestState {
    fn active(&self) -> bool { matches!(self, Self::AwaitingConsent | Self::Running) }
}
#[derive(Clone, Debug, Serialize)]
pub struct RequestView {
    pub id: HostRequestId, pub tab: HostTabId, pub task: u128,
    pub operation: BrowserOperation, pub state: RequestState, pub expires_ms: u64,
}
struct TabState {
    view: TabView,
    navigation: Option<(HostNavigationId, u64, Option<CanonicalOrigin>, Option<HostRequestId>)>,
    elements: BTreeSet<String>,
}
pub enum Event {
    Committed { tab: HostTabId, navigation: HostNavigationId, url: String, title: String },
    Failed { tab: HostTabId, navigation: HostNavigationId, error: String },
    Crashed { tab: HostTabId },
    Output { request: HostRequestId, output: Output },
    OperationFailed { request: HostRequestId, error: String },
}
pub struct Session {
    host: BrowserHost, port: Box<dyn NativePort>,
    tabs: BTreeMap<HostTabId, TabState>, requests: BTreeMap<HostRequestId, RequestView>,
}
impl Default for Session { fn default() -> Self { Self::new(Box::new(UnavailablePort)) } }
impl Session {
    pub fn new(port: Box<dyn NativePort>) -> Self {
        Self { host: BrowserHost::default(), port, tabs: BTreeMap::new(), requests: BTreeMap::new() }
    }
    pub fn capabilities(&self) -> Capabilities { self.port.capabilities() }
    pub fn tabs(&self) -> Vec<TabView> { self.tabs.values().map(|t| t.view.clone()).collect() }
    pub fn requests(&self) -> Vec<RequestView> { self.requests.values().cloned().collect() }
    pub fn task_tabs(&self, task: u128) -> Vec<HostTabId> {
        self.tabs.values().filter(|t| t.view.profile == BrowserProfile::AgentTask { task })
            .map(|t| t.view.id).collect()
    }
    pub fn open(&mut self, profile: BrowserProfile) -> Result<HostTabId> {
        let id = self.host.open_tab(profile)?;
        let error = self.port.send(Command::Open { tab: id, partition: profile.storage_partition() }).err();
        self.tabs.insert(id, TabState { view: TabView { id, profile, url: None, title: "New tab".into(),
            state: if error.is_some() { "unavailable" } else { "blank" }.into(),
            error: error.map(|e| e.to_string()), back: false, forward: false }, navigation: None,
            elements: BTreeSet::new() });
        Ok(id)
    }
    pub fn close(&mut self, tab: HostTabId) -> Result<()> {
        self.stop(tab)?; self.host.close_tab(tab)?; self.tabs.remove(&tab);
        let _ = self.port.send(Command::Close { tab }); Ok(())
    }
    /// Revoke grants even when the adapter cannot acknowledge cancellation.
    pub fn stop(&mut self, tab: HostTabId) -> Result<()> {
        self.host.crash(tab)?; self.invalidate(tab, None);
        let state = self.tabs.get_mut(&tab).ok_or(BrowserError::MissingTab)?;
        state.navigation = None; state.elements.clear(); state.view.state = "stopped".into();
        let _ = self.port.send(Command::Stop { tab }); Ok(())
    }
    pub fn shutdown_task(&mut self, task: u128) {
        for tab in self.task_tabs(task) { let _ = self.close(tab); }
        self.host.shutdown_task(task); self.requests.retain(|_, r| r.task != task);
    }
    pub fn user_navigate(&mut self, tab: HostTabId, url: &str, kind: NavigationKind, now: u64) -> Result<()> {
        if matches!(self.host.profile(tab)?, BrowserProfile::AgentTask { .. }) { return Err(BrowserError::WrongContext); }
        let document = if matches!(kind, NavigationKind::Back | NavigationKind::Forward | NavigationKind::Reload) {
            let h = self.host.history(tab)?; let at = h.current.ok_or(BrowserError::Invalid)?;
            let at = match kind { NavigationKind::Back => at.checked_sub(1),
                NavigationKind::Forward => at.checked_add(1), _ => Some(at) }.ok_or(BrowserError::Invalid)?;
            h.entries.get(at).cloned().ok_or(BrowserError::Invalid)?
        } else { CommittedDocument::parse(url)? };
        self.navigate(tab, document, kind, now, None)
    }
    fn navigate(&mut self, tab: HostTabId, document: CommittedDocument, kind: NavigationKind,
        now: u64, request: Option<HostRequestId>) -> Result<()> {
        if !self.capabilities().navigation { return Err(BrowserError::Unavailable); }
        let profile = self.host.profile(tab)?; let nav = self.host.begin_navigation(tab, kind)?;
        self.invalidate(tab, request);
        let allowed = matches!(profile, BrowserProfile::AgentTask { .. }).then(|| document.origin.clone());
        let state = self.tabs.get_mut(&tab).ok_or(BrowserError::MissingTab)?;
        state.elements.clear(); state.view.state = "loading".into(); state.view.error = None;
        state.navigation = Some((nav, now.saturating_add(30_000), allowed.clone(), request));
        if let Err(e) = self.port.send(Command::Navigate { tab, navigation: nav, document,
            partition: profile.storage_partition(), allowed_origin: allowed }) {
            self.fail_tab(tab, &e.to_string()); return Err(e);
        }
        Ok(())
    }
    fn supported(&self, op: &BrowserOperation) -> bool {
        let c = self.capabilities();
        match op { BrowserOperation::Navigate { .. } => c.navigation,
            BrowserOperation::ReadDocument => c.document,
            BrowserOperation::Click { .. } | BrowserOperation::Fill { .. } => c.input,
            BrowserOperation::Screenshot { .. } => c.capture,
            BrowserOperation::Download { .. } => c.downloads,
            _ => false }
    }
    pub fn request(&mut self, task: u128, tab: HostTabId, op: BrowserOperation, now: u64) -> Result<HostRequestId> {
        self.tick(now);
        if !self.supported(&op) { return Err(BrowserError::Unavailable); }
        let state = self.tabs.get(&tab).ok_or(BrowserError::MissingTab)?;
        if let BrowserOperation::Click { element } | BrowserOperation::Fill { element, .. } = &op {
            if !state.elements.contains(element) { return Err(BrowserError::Invalid); }
        }
        if self.requests.len() >= MAX_PENDING { return Err(BrowserError::Limit); }
        let id = self.host.request_agent_operation(tab, task, op.clone(), now)?;
        self.requests.insert(id, RequestView { id, tab, task, operation: op,
            state: RequestState::AwaitingConsent, expires_ms: now.saturating_add(60_000) });
        Ok(id)
    }
    /// Trusted native UI only. Deliberately absent from agent RPC.
    pub fn decide(&mut self, request: HostRequestId, allow: bool, now: u64) -> Result<()> {
        let view = self.requests.get(&request).ok_or(BrowserError::MissingRequest)?;
        if !matches!(view.state, RequestState::AwaitingConsent) { return Err(BrowserError::Invalid); }
        let result = (|| {
            let Some(grant) = self.host.resolve_agent_operation(request, allow, now)? else {
                self.requests.get_mut(&request).unwrap().state = RequestState::Denied; return Ok(());
            };
            let command = self.host.dispatch_agent_operation(grant, now)?;
            let r = self.requests.get_mut(&request).unwrap();
            r.state = RequestState::Running; r.expires_ms = now.saturating_add(30_000);
            if let BrowserOperation::Navigate { url } = &command.operation {
                self.navigate(command.tab, CommittedDocument::parse(url)?, NavigationKind::Push, now, Some(request))
            } else {
                self.port.send(Command::Operation { request, command, max_output_bytes: MAX_IPC_FRAME_BYTES })
            }
        })();
        if let Err(e) = &result { self.requests.get_mut(&request).unwrap().state = RequestState::Failed(e.to_string()); }
        result
    }
    pub fn result(&self, task: u128, request: HostRequestId) -> Result<RequestView> {
        let r = self.requests.get(&request).ok_or(BrowserError::MissingRequest)?;
        if r.task != task { return Err(BrowserError::WrongContext); } Ok(r.clone())
    }
    pub fn cancel(&mut self, task: u128, request: HostRequestId, now: u64) -> Result<()> {
        let r = self.result(task, request)?;
        match r.state {
            RequestState::AwaitingConsent => { let _ = self.host.resolve_agent_operation(request, false, now); },
            RequestState::Running => {
                if matches!(r.operation, BrowserOperation::Navigate { .. }) { self.stop(r.tab)?; }
                let _ = self.port.send(Command::Cancel { request });
            }, _ => return Ok(())
        }
        self.requests.get_mut(&request).unwrap().state = RequestState::Cancelled; Ok(())
    }
    pub fn forget(&mut self, task: u128, request: HostRequestId) -> Result<()> {
        if self.result(task, request)?.state.active() { return Err(BrowserError::Invalid); }
        self.requests.remove(&request); Ok(())
    }
    pub fn tick(&mut self, now: u64) {
        let expired: Vec<_> = self.requests.values().filter(|r| r.state.active() && now >= r.expires_ms)
            .map(|r| (r.task, r.id)).collect();
        for (task, id) in expired { let _ = self.cancel(task, id, now);
            if let Some(r) = self.requests.get_mut(&id) { r.state = RequestState::Failed(BrowserError::Timeout.to_string()); } }
        let expired: Vec<_> = self.tabs.iter().filter(|(_, t)| t.navigation.as_ref().is_some_and(|n| now >= n.1)).map(|(id, _)| *id).collect();
        for id in expired { self.fail_tab(id, "Navigation timed out"); }
    }
    fn invalidate(&mut self, tab: HostTabId, except: Option<HostRequestId>) {
        for r in self.requests.values_mut().filter(|r| r.tab == tab && Some(r.id) != except && r.state.active()) {
            let _ = self.port.send(Command::Cancel { request: r.id }); r.state = RequestState::Cancelled;
        }
    }
    fn fail_tab(&mut self, tab: HostTabId, error: &str) {
        let request = self.tabs.get(&tab).and_then(|t| t.navigation.as_ref()).and_then(|n| n.3);
        let _ = self.stop(tab);
        if let Some(t) = self.tabs.get_mut(&tab) { t.view.state = "failed".into(); t.view.error = Some(error.chars().take(1024).collect()); }
        if let Some(id) = request { if let Some(r) = self.requests.get_mut(&id) { r.state = RequestState::Failed(error.chars().take(1024).collect()); } }
    }
    /// Authenticated native host callbacks only, never page JavaScript or MCP.
    pub fn event(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Committed { tab, navigation, url, title } => {
                let t = self.tabs.get(&tab).ok_or(BrowserError::MissingTab)?;
                let (nav, _, allowed, request) = t.navigation.clone().ok_or(BrowserError::MissingNavigation)?;
                if nav != navigation { return Err(BrowserError::MissingNavigation); }
                let doc = CommittedDocument::parse(&url)?;
                if title.len() > 4096 || allowed.as_ref().is_some_and(|a| a != &doc.origin) {
                    self.fail_tab(tab, "Native navigation violated its approved origin"); return Err(BrowserError::WrongContext);
                }
                self.host.commit_navigation(navigation, doc.clone())?;
                let h = self.host.history(tab)?; let t = self.tabs.get_mut(&tab).unwrap(); t.navigation = None;
                t.view.url = Some(doc.canonical_url); t.view.title = title; t.view.state = "ready".into(); t.view.error = None;
                t.view.back = h.current.is_some_and(|i| i > 0); t.view.forward = h.current.is_some_and(|i| i + 1 < h.entries.len());
                if let Some(id) = request { self.requests.get_mut(&id).ok_or(BrowserError::MissingRequest)?.state = RequestState::Complete(Output::Done); }
            },
            Event::Failed { tab, navigation, error } => {
                if self.tabs.get(&tab).and_then(|t| t.navigation.as_ref()).is_none_or(|n| n.0 != navigation) { return Err(BrowserError::MissingNavigation); }
                self.fail_tab(tab, &error);
            },
            Event::Crashed { tab } => { self.fail_tab(tab, "Native browser process crashed");
                if let Some(t) = self.tabs.get_mut(&tab) { t.view.state = "crashed".into(); } },
            Event::Output { request, output } => {
                let r = self.requests.get(&request).ok_or(BrowserError::MissingRequest)?;
                if !matches!(r.state, RequestState::Running) || matches!(r.operation, BrowserOperation::Navigate { .. }) { return Err(BrowserError::Invalid); }
                if let Err(e) = output.validate(&r.operation) { self.requests.get_mut(&request).unwrap().state = RequestState::Failed(e.to_string()); return Err(e); }
                if let Output::Document { elements, .. } = &output {
                    self.tabs.get_mut(&r.tab).ok_or(BrowserError::MissingTab)?.elements = elements.iter().map(|e| e.id.clone()).collect();
                }
                self.requests.get_mut(&request).unwrap().state = RequestState::Complete(output);
            },
            Event::OperationFailed { request, error } => {
                let r = self.requests.get_mut(&request).ok_or(BrowserError::MissingRequest)?;
                if !matches!(r.state, RequestState::Running) { return Err(BrowserError::Invalid); }
                r.state = RequestState::Failed(error.chars().take(1024).collect());
            }
        }
        Ok(())
    }
}
#[cfg(test)] mod tests;
