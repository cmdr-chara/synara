#![forbid(unsafe_code)]

//! Typed browser-host domain state.
//!
//! Native web engines remain platform adapters. This crate owns tab/session state,
//! navigation/history, consent binding and the narrow command vocabulary crossing
//! the Rust/native boundary. It deliberately exposes no generic page-to-host RPC.

#[path = "../../../foundations/browser/lib.rs"]
pub mod policy;

use policy::{Action, BrowserPolicy, Context, Grant, NavigationId, Origin, Scheme, TabId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

const MAX_CANONICAL_URL_BYTES: usize = 8 * 1024;
const MAX_INPUT_BYTES: usize = 16 * 1024;
const MAX_IPC_FRAME_BYTES: usize = 256 * 1024;
const MAX_HISTORY: usize = 512;
const MAX_PENDING: usize = 128;

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum BrowserError {
    #[error("browser policy rejected the operation: {0:?}")]
    Policy(policy::Error),
    #[error("browser tab does not exist")]
    MissingTab,
    #[error("browser navigation does not exist")]
    MissingNavigation,
    #[error("browser request does not exist")]
    MissingRequest,
    #[error("browser operation is invalid")]
    Invalid,
    #[error("browser resource limit reached")]
    Limit,
    #[error("browser context does not own this operation")]
    WrongContext,
    #[error("browser IPC frame is malformed")]
    MalformedFrame,
}
impl From<policy::Error> for BrowserError {
    fn from(value: policy::Error) -> Self {
        Self::Policy(value)
    }
}
pub type Result<T> = std::result::Result<T, BrowserError>;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct HostTabId(u64);
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct HostNavigationId(u64);
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct HostRequestId(u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserProfile {
    Manual,
    AgentTask { task: u128 },
    Authentication { flow: u128 },
}
impl BrowserProfile {
    fn policy_context(self) -> Context {
        match self {
            Self::Manual => Context::Manual,
            Self::AgentTask { task } => Context::AgentTask(task),
            Self::Authentication { flow } => Context::Authentication(flow),
        }
    }
    pub fn storage_partition(self) -> StoragePartition {
        match self {
            Self::Manual => StoragePartition::Manual,
            Self::AgentTask { task } => StoragePartition::AgentTask(task),
            Self::Authentication { flow } => StoragePartition::Authentication(flow),
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoragePartition {
    Manual,
    AgentTask(u128),
    Authentication(u128),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeScheme {
    Http,
    Https,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalOrigin {
    pub scheme: NativeScheme,
    pub host: String,
    pub port: u16,
}
impl CanonicalOrigin {
    fn policy_origin(&self) -> Result<Origin> {
        let scheme = match self.scheme {
            NativeScheme::Http => Scheme::Http,
            NativeScheme::Https => Scheme::Https,
        };
        Origin::from_canonical_parts(scheme, &self.host, self.port).map_err(Into::into)
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommittedDocument {
    pub origin: CanonicalOrigin,
    /// Canonical display/navigation URL reported by the native browser engine.
    /// It is not used as an authority source.
    pub canonical_url: String,
}
impl CommittedDocument {
    fn validate(&self) -> Result<()> {
        self.origin.policy_origin()?;
        if self.canonical_url.is_empty()
            || self.canonical_url.len() > MAX_CANONICAL_URL_BYTES
            || self.canonical_url.chars().any(char::is_control)
        {
            return Err(BrowserError::Invalid);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NavigationKind {
    Push,
    Replace,
    Reload,
    Back,
    Forward,
    Redirect,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HistorySnapshot {
    pub entries: Vec<CommittedDocument>,
    pub current: Option<usize>,
}
#[derive(Clone, Debug)]
struct Tab {
    policy: TabId,
    profile: BrowserProfile,
    history: VecDeque<CommittedDocument>,
    current: Option<usize>,
}
#[derive(Clone, Copy, Debug)]
struct PendingNavigation {
    tab: HostTabId,
    policy: NavigationId,
    kind: NavigationKind,
    target: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputEvent {
    Text(String),
    Key { key: String, pressed: bool },
    Pointer { x: i32, y: i32, button: u8, pressed: bool },
    Scroll { x: i32, y: i32 },
}
impl InputEvent {
    fn validate(&self) -> Result<()> {
        match self {
            Self::Text(value) if value.len() <= MAX_INPUT_BYTES && !value.contains('\0') => Ok(()),
            Self::Key { key, .. }
                if !key.is_empty()
                    && key.len() <= 128
                    && !key.chars().any(char::is_control) =>
            {
                Ok(())
            }
            Self::Pointer { button, .. } if *button <= 8 => Ok(()),
            Self::Scroll { .. } => Ok(()),
            _ => Err(BrowserError::Invalid),
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum BrowserOperation {
    ReadDocument,
    Screenshot { full_page: bool },
    Input { event: InputEvent },
    Download { download_id: String },
    Upload { chooser_id: String, file_token: String },
    ClipboardRead,
    ClipboardWrite { text: String },
}
impl BrowserOperation {
    fn validate(&self) -> Result<()> {
        let token = |value: &str| {
            !value.is_empty()
                && value.len() <= 256
                && !value.chars().any(char::is_control)
                && !value.contains('/')
                && !value.contains('\\')
        };
        match self {
            Self::Input { event } => event.validate(),
            Self::Download { download_id } if token(download_id) => Ok(()),
            Self::Upload {
                chooser_id,
                file_token,
            } if token(chooser_id) && token(file_token) => Ok(()),
            Self::ClipboardWrite { text }
                if text.len() <= MAX_INPUT_BYTES && !text.contains('\0') =>
            {
                Ok(())
            }
            Self::ReadDocument
            | Self::Screenshot { .. }
            | Self::ClipboardRead => Ok(()),
            _ => Err(BrowserError::Invalid),
        }
    }
    fn action(&self) -> Action {
        match self {
            Self::ReadDocument => Action::ReadDocument,
            Self::Screenshot { .. } => Action::Screenshot,
            Self::Input { .. } => Action::Input,
            Self::Download { .. } => Action::Download,
            Self::Upload { .. } => Action::Upload,
            Self::ClipboardRead => Action::ClipboardRead,
            Self::ClipboardWrite { .. } => Action::ClipboardWrite,
        }
    }
}

#[derive(Clone, Debug)]
struct PendingOperation {
    policy_request: policy::RequestId,
    tab: HostTabId,
    task: u128,
    operation: BrowserOperation,
}
#[derive(Debug)]
pub struct ApprovedOperation {
    policy_grant: Grant,
    tab: HostTabId,
    task: u128,
    operation: BrowserOperation,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeCommand {
    pub tab: HostTabId,
    pub partition: StoragePartition,
    pub operation: BrowserOperation,
}
impl NativeCommand {
    fn validate(&self) -> Result<()> {
        self.operation.validate()
    }
}

#[derive(Debug, Default)]
pub struct BrowserHost {
    policy: BrowserPolicy,
    tabs: BTreeMap<HostTabId, Tab>,
    navigations: BTreeMap<HostNavigationId, PendingNavigation>,
    pending: BTreeMap<HostRequestId, PendingOperation>,
    next_tab: u64,
    next_navigation: u64,
    next_request: u64,
}
impl BrowserHost {
    pub fn open_tab(&mut self, profile: BrowserProfile) -> Result<HostTabId> {
        self.next_tab = self.next_tab.checked_add(1).ok_or(BrowserError::Limit)?;
        let id = HostTabId(self.next_tab);
        let policy = self.policy.open_tab(profile.policy_context())?;
        self.tabs.insert(
            id,
            Tab {
                policy,
                profile,
                history: VecDeque::new(),
                current: None,
            },
        );
        Ok(id)
    }

    /// A native popup inherits the exact storage/authority profile of its opener.
    pub fn open_popup(&mut self, opener: HostTabId) -> Result<HostTabId> {
        let profile = self.tabs.get(&opener).ok_or(BrowserError::MissingTab)?.profile;
        self.open_tab(profile)
    }

    pub fn close_tab(&mut self, tab: HostTabId) -> Result<()> {
        let state = self.tabs.remove(&tab).ok_or(BrowserError::MissingTab)?;
        self.policy.close_tab(state.policy)?;
        self.navigations.retain(|_, value| value.tab != tab);
        self.pending.retain(|_, value| value.tab != tab);
        Ok(())
    }

    pub fn profile(&self, tab: HostTabId) -> Result<BrowserProfile> {
        Ok(self.tabs.get(&tab).ok_or(BrowserError::MissingTab)?.profile)
    }

    pub fn begin_navigation(
        &mut self,
        tab: HostTabId,
        kind: NavigationKind,
    ) -> Result<HostNavigationId> {
        let state = self.tabs.get_mut(&tab).ok_or(BrowserError::MissingTab)?;
        let target = match kind {
            NavigationKind::Back => state.current.and_then(|index| index.checked_sub(1)),
            NavigationKind::Forward => state.current.and_then(|index| index.checked_add(1)),
            _ => None,
        };
        if matches!(kind, NavigationKind::Back | NavigationKind::Forward)
            && target.is_none_or(|index| index >= state.history.len())
        {
            return Err(BrowserError::Invalid);
        }
        let policy = self.policy.begin_navigation(state.policy)?;
        self.navigations.retain(|_, value| value.tab != tab);
        self.next_navigation = self
            .next_navigation
            .checked_add(1)
            .ok_or(BrowserError::Limit)?;
        let id = HostNavigationId(self.next_navigation);
        self.navigations.insert(
            id,
            PendingNavigation {
                tab,
                policy,
                kind,
                target,
            },
        );
        self.pending.retain(|_, value| value.tab != tab);
        Ok(id)
    }

    pub fn commit_navigation(
        &mut self,
        navigation: HostNavigationId,
        document: CommittedDocument,
    ) -> Result<()> {
        document.validate()?;
        let pending = self
            .navigations
            .remove(&navigation)
            .ok_or(BrowserError::MissingNavigation)?;
        let policy_origin = document.origin.policy_origin()?;
        {
            let state = self
                .tabs
                .get(&pending.tab)
                .ok_or(BrowserError::MissingTab)?;
            match pending.kind {
                NavigationKind::Back | NavigationKind::Forward => {
                    let target = pending.target.ok_or(BrowserError::Invalid)?;
                    if state.history.get(target) != Some(&document) {
                        return Err(BrowserError::Invalid);
                    }
                }
                NavigationKind::Reload => {
                    let current = state.current.ok_or(BrowserError::Invalid)?;
                    if state.history.get(current) != Some(&document) {
                        return Err(BrowserError::Invalid);
                    }
                }
                _ => {}
            }
        }
        self.policy.commit_navigation(pending.policy, policy_origin)?;
        let state = self
            .tabs
            .get_mut(&pending.tab)
            .ok_or(BrowserError::MissingTab)?;
        match pending.kind {
            NavigationKind::Back | NavigationKind::Forward => {
                state.current = pending.target;
            }
            NavigationKind::Reload => {}
            NavigationKind::Replace | NavigationKind::Redirect => {
                if let Some(current) = state.current {
                    state.history[current] = document;
                } else {
                    state.history.push_back(document);
                    state.current = Some(0);
                }
            }
            NavigationKind::Push => {
                if let Some(current) = state.current {
                    state.history.truncate(current + 1);
                }
                if state.history.len() >= MAX_HISTORY {
                    state.history.pop_front();
                }
                state.history.push_back(document);
                state.current = Some(state.history.len() - 1);
            }
        }
        Ok(())
    }

    pub fn crash(&mut self, tab: HostTabId) -> Result<()> {
        let state = self.tabs.get_mut(&tab).ok_or(BrowserError::MissingTab)?;
        self.policy.crash(state.policy)?;
        self.navigations.retain(|_, value| value.tab != tab);
        self.pending.retain(|_, value| value.tab != tab);
        Ok(())
    }

    pub fn history(&self, tab: HostTabId) -> Result<HistorySnapshot> {
        let state = self.tabs.get(&tab).ok_or(BrowserError::MissingTab)?;
        Ok(HistorySnapshot {
            entries: state.history.iter().cloned().collect(),
            current: state.current,
        })
    }

    /// Freeze an agent operation before presenting native consent. Callers cannot
    /// replace the payload when the approval is later consumed.
    pub fn request_agent_operation(
        &mut self,
        tab: HostTabId,
        task: u128,
        operation: BrowserOperation,
        now_ms: u64,
    ) -> Result<HostRequestId> {
        operation.validate()?;
        if self.pending.len() >= MAX_PENDING {
            return Err(BrowserError::Limit);
        }
        let state = self.tabs.get(&tab).ok_or(BrowserError::MissingTab)?;
        if state.profile != (BrowserProfile::AgentTask { task }) {
            return Err(BrowserError::WrongContext);
        }
        let prompt = self
            .policy
            .request(state.policy, task, operation.action(), now_ms)?;
        self.next_request = self.next_request.checked_add(1).ok_or(BrowserError::Limit)?;
        let id = HostRequestId(self.next_request);
        self.pending.insert(
            id,
            PendingOperation {
                policy_request: prompt.id,
                tab,
                task,
                operation,
            },
        );
        Ok(id)
    }

    pub fn resolve_agent_operation(
        &mut self,
        request: HostRequestId,
        allow: bool,
        now_ms: u64,
    ) -> Result<Option<ApprovedOperation>> {
        let pending = self
            .pending
            .remove(&request)
            .ok_or(BrowserError::MissingRequest)?;
        let grant = self
            .policy
            .resolve(pending.policy_request, allow, now_ms)?;
        Ok(grant.map(|policy_grant| ApprovedOperation {
            policy_grant,
            tab: pending.tab,
            task: pending.task,
            operation: pending.operation,
        }))
    }

    pub fn dispatch_agent_operation(
        &mut self,
        approved: ApprovedOperation,
        now_ms: u64,
    ) -> Result<NativeCommand> {
        let state = self
            .tabs
            .get(&approved.tab)
            .ok_or(BrowserError::MissingTab)?;
        if state.profile != (BrowserProfile::AgentTask { task: approved.task }) {
            return Err(BrowserError::WrongContext);
        }
        self.policy.consume(
            approved.policy_grant,
            state.policy,
            approved.task,
            &approved.operation.action(),
            now_ms,
        )?;
        Ok(NativeCommand {
            tab: approved.tab,
            partition: state.profile.storage_partition(),
            operation: approved.operation,
        })
    }

    /// Manual/authentication commands are only accepted from those native
    /// controller contexts. Agent-task tabs must use the consent flow above.
    pub fn dispatch_user_operation(
        &self,
        tab: HostTabId,
        operation: BrowserOperation,
    ) -> Result<NativeCommand> {
        operation.validate()?;
        let state = self.tabs.get(&tab).ok_or(BrowserError::MissingTab)?;
        if matches!(state.profile, BrowserProfile::AgentTask { .. }) {
            return Err(BrowserError::WrongContext);
        }
        Ok(NativeCommand {
            tab,
            partition: state.profile.storage_partition(),
            operation,
        })
    }

    pub fn shutdown_task(&mut self, task: u128) {
        self.policy.revoke_task(task);
        self.pending.retain(|_, value| value.task != task);
    }
}

/// Encode only the reviewed native command vocabulary. No method string, shell
/// command, filesystem path or arbitrary host-RPC payload can be injected.
pub fn encode_command(command: &NativeCommand) -> Result<Vec<u8>> {
    command.validate()?;
    let body = serde_json::to_vec(command).map_err(|_| BrowserError::MalformedFrame)?;
    if body.len() > MAX_IPC_FRAME_BYTES {
        return Err(BrowserError::Limit);
    }
    let length = u32::try_from(body.len()).map_err(|_| BrowserError::Limit)?;
    let mut frame = Vec::with_capacity(4 + body.len());
    frame.extend_from_slice(&length.to_le_bytes());
    frame.extend_from_slice(&body);
    Ok(frame)
}

pub fn decode_command(frame: &[u8]) -> Result<NativeCommand> {
    if frame.len() < 4 || frame.len() > MAX_IPC_FRAME_BYTES + 4 {
        return Err(BrowserError::MalformedFrame);
    }
    let length = u32::from_le_bytes(frame[..4].try_into().unwrap()) as usize;
    if length > MAX_IPC_FRAME_BYTES || frame.len() != length + 4 {
        return Err(BrowserError::MalformedFrame);
    }
    let command: NativeCommand =
        serde_json::from_slice(&frame[4..]).map_err(|_| BrowserError::MalformedFrame)?;
    command.validate()?;
    Ok(command)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(host: &str, path: &str) -> CommittedDocument {
        CommittedDocument {
            origin: CanonicalOrigin {
                scheme: NativeScheme::Https,
                host: host.into(),
                port: 443,
            },
            canonical_url: format!("https://{host}{path}"),
        }
    }
    fn loaded(host: &mut BrowserHost, profile: BrowserProfile) -> HostTabId {
        let tab = host.open_tab(profile).unwrap();
        let nav = host.begin_navigation(tab, NavigationKind::Push).unwrap();
        host.commit_navigation(nav, doc("example.test", "/")).unwrap();
        tab
    }

    #[test]
    fn profiles_and_popups_never_cross_storage_contexts() {
        let mut host = BrowserHost::default();
        let manual = loaded(&mut host, BrowserProfile::Manual);
        let agent = loaded(&mut host, BrowserProfile::AgentTask { task: 7 });
        let auth = loaded(&mut host, BrowserProfile::Authentication { flow: 9 });
        let manual_popup = host.open_popup(manual).unwrap();
        let agent_popup = host.open_popup(agent).unwrap();
        let auth_popup = host.open_popup(auth).unwrap();
        assert_eq!(host.profile(manual_popup).unwrap(), BrowserProfile::Manual);
        assert_eq!(
            host.profile(agent_popup).unwrap(),
            BrowserProfile::AgentTask { task: 7 }
        );
        assert_eq!(
            host.profile(auth_popup).unwrap(),
            BrowserProfile::Authentication { flow: 9 }
        );
        assert!(matches!(
            host.dispatch_user_operation(agent, BrowserOperation::Screenshot { full_page: false }),
            Err(BrowserError::WrongContext)
        ));
    }

    #[test]
    fn history_back_forward_reload_replace_and_redirect_are_stateful() {
        let mut host = BrowserHost::default();
        let tab = loaded(&mut host, BrowserProfile::Manual);
        for path in ["/two", "/three"] {
            let nav = host.begin_navigation(tab, NavigationKind::Push).unwrap();
            host.commit_navigation(nav, doc("example.test", path)).unwrap();
        }
        let nav = host.begin_navigation(tab, NavigationKind::Back).unwrap();
        host.commit_navigation(nav, doc("example.test", "/two")).unwrap();
        assert_eq!(host.history(tab).unwrap().current, Some(1));
        let nav = host.begin_navigation(tab, NavigationKind::Forward).unwrap();
        host.commit_navigation(nav, doc("example.test", "/three")).unwrap();
        let nav = host.begin_navigation(tab, NavigationKind::Reload).unwrap();
        host.commit_navigation(nav, doc("example.test", "/three")).unwrap();
        let nav = host.begin_navigation(tab, NavigationKind::Replace).unwrap();
        host.commit_navigation(nav, doc("example.test", "/replaced")).unwrap();
        let nav = host.begin_navigation(tab, NavigationKind::Redirect).unwrap();
        host.commit_navigation(nav, doc("redirect.test", "/final")).unwrap();
        let history = host.history(tab).unwrap();
        assert_eq!(history.entries.len(), 3);
        assert_eq!(history.entries[2], doc("redirect.test", "/final"));
    }

    #[test]
    fn approval_freezes_payload_and_is_revoked_by_navigation_or_crash() {
        let mut host = BrowserHost::default();
        let tab = loaded(&mut host, BrowserProfile::AgentTask { task: 7 });
        let request = host
            .request_agent_operation(
                tab,
                7,
                BrowserOperation::ClipboardWrite {
                    text: "approved".into(),
                },
                0,
            )
            .unwrap();
        let approved = host.resolve_agent_operation(request, true, 1).unwrap().unwrap();
        let command = host.dispatch_agent_operation(approved, 2).unwrap();
        assert_eq!(
            command.operation,
            BrowserOperation::ClipboardWrite {
                text: "approved".into()
            }
        );

        let request = host
            .request_agent_operation(
                tab,
                7,
                BrowserOperation::Screenshot { full_page: false },
                3,
            )
            .unwrap();
        let _ = host.begin_navigation(tab, NavigationKind::Reload).unwrap();
        assert!(host.resolve_agent_operation(request, true, 4).is_err());

        let nav = host.begin_navigation(tab, NavigationKind::Push).unwrap();
        host.commit_navigation(nav, doc("example.test", "/after")).unwrap();
        let request = host
            .request_agent_operation(
                tab,
                7,
                BrowserOperation::Screenshot { full_page: true },
                5,
            )
            .unwrap();
        host.crash(tab).unwrap();
        assert!(host.resolve_agent_operation(request, true, 6).is_err());
    }

    #[test]
    fn task_shutdown_revokes_pending_consent() {
        let mut host = BrowserHost::default();
        let tab = loaded(&mut host, BrowserProfile::AgentTask { task: 7 });
        let request = host
            .request_agent_operation(tab, 7, BrowserOperation::ReadDocument, 0)
            .unwrap();
        host.shutdown_task(7);
        assert!(matches!(
            host.resolve_agent_operation(request, true, 1),
            Err(BrowserError::MissingRequest)
        ));
    }

    #[test]
    fn native_frames_are_typed_bounded_and_exact_length() {
        let command = NativeCommand {
            tab: HostTabId(1),
            partition: StoragePartition::Manual,
            operation: BrowserOperation::Input {
                event: InputEvent::Text("hello".into()),
            },
        };
        let frame = encode_command(&command).unwrap();
        assert_eq!(decode_command(&frame).unwrap(), command);

        let mut trailing = frame.clone();
        trailing.push(0);
        assert_eq!(decode_command(&trailing), Err(BrowserError::MalformedFrame));

        let unknown = br#"{"tab":1,"partition":"manual","operation":{"operation":"shell","command":"rm"}}"#;
        let mut frame = Vec::new();
        frame.extend_from_slice(&(unknown.len() as u32).to_le_bytes());
        frame.extend_from_slice(unknown);
        assert_eq!(decode_command(&frame), Err(BrowserError::MalformedFrame));

        let oversized = NativeCommand {
            tab: HostTabId(1),
            partition: StoragePartition::Manual,
            operation: BrowserOperation::ClipboardWrite {
                text: "x".repeat(MAX_INPUT_BYTES + 1),
            },
        };
        assert_eq!(encode_command(&oversized), Err(BrowserError::Invalid));
    }

    #[test]
    fn hostile_origins_urls_tokens_and_context_claims_fail_closed() {
        let mut host = BrowserHost::default();
        let tab = host.open_tab(BrowserProfile::AgentTask { task: 7 }).unwrap();
        let nav = host.begin_navigation(tab, NavigationKind::Push).unwrap();
        let mut invalid = doc("example.test", "/");
        invalid.origin.host = "example.test@evil.test".into();
        assert!(host.commit_navigation(nav, invalid).is_err());

        let nav = host.begin_navigation(tab, NavigationKind::Push).unwrap();
        let mut invalid = doc("example.test", "/");
        invalid.canonical_url.push('\n');
        assert!(host.commit_navigation(nav, invalid).is_err());

        let nav = host.begin_navigation(tab, NavigationKind::Push).unwrap();
        host.commit_navigation(nav, doc("example.test", "/")).unwrap();
        assert!(matches!(
            host.request_agent_operation(tab, 8, BrowserOperation::ReadDocument, 0),
            Err(BrowserError::WrongContext)
        ));
        assert!(host
            .request_agent_operation(
                tab,
                7,
                BrowserOperation::Upload {
                    chooser_id: "chooser".into(),
                    file_token: "../secret".into(),
                },
                0,
            )
            .is_err());
    }

    #[test]
    fn history_and_pending_operations_are_bounded() {
        let mut host = BrowserHost::default();
        let tab = loaded(&mut host, BrowserProfile::Manual);
        for index in 0..(MAX_HISTORY + 20) {
            let nav = host.begin_navigation(tab, NavigationKind::Push).unwrap();
            host.commit_navigation(nav, doc("example.test", &format!("/{index}")))
                .unwrap();
        }
        assert_eq!(host.history(tab).unwrap().entries.len(), MAX_HISTORY);

        let tab = loaded(&mut host, BrowserProfile::AgentTask { task: 7 });
        for _ in 0..MAX_PENDING {
            host.request_agent_operation(tab, 7, BrowserOperation::ReadDocument, 0)
                .unwrap();
        }
        assert_eq!(
            host.request_agent_operation(tab, 7, BrowserOperation::ReadDocument, 0),
            Err(BrowserError::Limit)
        );
    }
}
