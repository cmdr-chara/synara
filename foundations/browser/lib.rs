//! Browser-host consent and lifecycle policy, independent of GPUI and ACP.
//!
//! This is not a browser engine or URL parser. A native host must obtain canonical
//! origins from its browser engine, bind the actor to the authenticated command
//! source, and enforce this policy before dispatching a privileged operation.
#![forbid(unsafe_code)]

use std::collections::BTreeMap;

const MAX_TABS: usize = 32;
const MAX_CONSENTS: usize = 128;
const CONSENT_TTL_MS: u64 = 60_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Scheme {
    Http,
    Https,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Origin {
    pub scheme: Scheme,
    host: String,
    port: u16,
}
impl Origin {
    /// `host` must come from a standards-compliant native/URL origin parser, not
    /// a substring of a URL or a host claimed by page JavaScript. This validation
    /// is an additional shape check, not a replacement for that parser.
    pub fn from_canonical_parts(scheme: Scheme, host: &str, port: u16) -> Result<Self> {
        let domain = !host.is_empty()
            && host.len() <= 253
            && host.is_ascii()
            && host.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b".-".contains(&b))
            && host.split('.').all(|part| {
                !part.is_empty() && part.len() <= 63 && !part.starts_with('-') && !part.ends_with('-')
            });
        let ip = host.parse::<std::net::IpAddr>().is_ok();
        if port == 0 || (!domain && !ip) {
            return Err(Error::InvalidOrigin);
        }
        Ok(Self { scheme, host: host.to_owned(), port })
    }
    pub fn host(&self) -> &str {
        &self.host
    }
    pub fn port(&self) -> u16 {
        self.port
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct TabId(u64);
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NavigationId {
    tab: TabId,
    generation: u64,
}
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RequestId(u64);
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Grant {
    id: RequestId,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Context {
    Manual,
    AgentTask(u128),
    Authentication(u128),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Action {
    ReadDocument,
    Screenshot,
    Input,
    Download,
    Upload,
    ClipboardRead,
    ClipboardWrite,
    Navigate(Origin),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    InvalidOrigin,
    MissingTab,
    NotReady,
    WrongContext,
    Stale,
    Expired,
    Denied,
    Limit,
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug)]
pub struct ConsentPrompt {
    pub id: RequestId,
    pub tab: TabId,
    pub task: u128,
    pub action: Action,
    pub origin: Origin,
    pub expires_at_ms: u64,
    generation: u64,
}
#[derive(Clone, Debug)]
struct Tab {
    context: Context,
    generation: u64,
    document: Option<Origin>,
    loading: bool,
    crashed: bool,
}
#[derive(Debug, Default)]
pub struct BrowserPolicy {
    tabs: BTreeMap<TabId, Tab>,
    pending: BTreeMap<RequestId, ConsentPrompt>,
    grants: BTreeMap<RequestId, ConsentPrompt>,
    next_tab: u64,
    next_request: u64,
}
impl BrowserPolicy {
    pub fn open_tab(&mut self, context: Context) -> Result<TabId> {
        if self.tabs.len() >= MAX_TABS {
            return Err(Error::Limit);
        }
        self.next_tab = self.next_tab.checked_add(1).ok_or(Error::Limit)?;
        let id = TabId(self.next_tab);
        self.tabs.insert(id, Tab {
            context, generation: 0, document: None, loading: false, crashed: false,
        });
        Ok(id)
    }

    pub fn close_tab(&mut self, tab: TabId) -> Result<()> {
        self.tabs.remove(&tab).ok_or(Error::MissingTab)?;
        self.revoke_tab(tab);
        Ok(())
    }

    /// Invoke on every top-level provisional navigation, including redirects,
    /// reload and history navigation. Same-origin navigation also revokes grants.
    pub fn begin_navigation(&mut self, tab: TabId) -> Result<NavigationId> {
        let state = self.tabs.get_mut(&tab).ok_or(Error::MissingTab)?;
        state.generation = state.generation.checked_add(1).ok_or(Error::Limit)?;
        state.document = None;
        state.loading = true;
        state.crashed = false;
        let id = NavigationId { tab, generation: state.generation };
        self.revoke_tab(tab);
        Ok(id)
    }

    /// Commit only the browser-engine origin of the final document. Stale native
    /// callbacks must not revive a replaced or crashed document.
    pub fn commit_navigation(&mut self, navigation: NavigationId, origin: Origin) -> Result<()> {
        let state = self.tabs.get_mut(&navigation.tab).ok_or(Error::MissingTab)?;
        if state.generation != navigation.generation || !state.loading || state.crashed {
            return Err(Error::Stale);
        }
        state.document = Some(origin);
        state.loading = false;
        Ok(())
    }

    pub fn crash(&mut self, tab: TabId) -> Result<()> {
        let state = self.tabs.get_mut(&tab).ok_or(Error::MissingTab)?;
        state.crashed = true;
        state.loading = false;
        state.document = None;
        self.revoke_tab(tab);
        Ok(())
    }

    /// Request a one-operation grant scoped to one task, tab, origin and document.
    /// Manual and authentication contexts cannot be accessed by this agent path.
    pub fn request(&mut self, tab: TabId, task: u128, action: Action, now_ms: u64) -> Result<ConsentPrompt> {
        self.prune(now_ms);
        if self.pending.len() + self.grants.len() >= MAX_CONSENTS {
            return Err(Error::Limit);
        }
        let state = self.tabs.get(&tab).ok_or(Error::MissingTab)?;
        if state.context != Context::AgentTask(task) {
            return Err(Error::WrongContext);
        }
        if state.loading || state.crashed {
            return Err(Error::NotReady);
        }
        let origin = state.document.clone().ok_or(Error::NotReady)?;
        self.next_request = self.next_request.checked_add(1).ok_or(Error::Limit)?;
        let prompt = ConsentPrompt {
            id: RequestId(self.next_request), tab, task, action, origin,
            expires_at_ms: now_ms.checked_add(CONSENT_TTL_MS).ok_or(Error::Limit)?,
            generation: state.generation,
        };
        self.pending.insert(prompt.id, prompt.clone());
        Ok(prompt)
    }

    /// Only a native user response may resolve a request. A denied or expired
    /// request cannot later be approved by replaying the same request ID.
    pub fn resolve(&mut self, request: RequestId, allow: bool, now_ms: u64) -> Result<Option<Grant>> {
        let prompt = self.pending.remove(&request).ok_or(Error::Stale)?;
        self.validate_prompt(&prompt, now_ms)?;
        if !allow {
            return Ok(None);
        }
        self.grants.insert(request, prompt);
        Ok(Some(Grant { id: request }))
    }

    /// Consume BEFORE dispatch. Even a failed native operation needs new consent
    /// on retry. A wrong operation burns the grant rather than widening its scope.
    pub fn consume(&mut self, grant: Grant, tab: TabId, task: u128, action: &Action, now_ms: u64) -> Result<()> {
        let prompt = self.grants.remove(&grant.id).ok_or(Error::Stale)?;
        self.validate_prompt(&prompt, now_ms)?;
        if prompt.tab != tab || prompt.task != task || &prompt.action != action {
            return Err(Error::Denied);
        }
        Ok(())
    }

    pub fn revoke_task(&mut self, task: u128) {
        self.pending.retain(|_, prompt| prompt.task != task);
        self.grants.retain(|_, prompt| prompt.task != task);
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
    pub fn grant_count(&self) -> usize {
        self.grants.len()
    }
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    fn revoke_tab(&mut self, tab: TabId) {
        self.pending.retain(|_, prompt| prompt.tab != tab);
        self.grants.retain(|_, prompt| prompt.tab != tab);
    }
    fn prune(&mut self, now_ms: u64) {
        self.pending.retain(|_, prompt| now_ms < prompt.expires_at_ms);
        self.grants.retain(|_, prompt| now_ms < prompt.expires_at_ms);
    }
    fn validate_prompt(&self, prompt: &ConsentPrompt, now_ms: u64) -> Result<()> {
        if now_ms >= prompt.expires_at_ms {
            return Err(Error::Expired);
        }
        let state = self.tabs.get(&prompt.tab).ok_or(Error::MissingTab)?;
        if state.loading || state.crashed || state.generation != prompt.generation
            || state.context != Context::AgentTask(prompt.task)
            || state.document.as_ref() != Some(&prompt.origin)
        {
            return Err(Error::Stale);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn origin() -> Origin {
        Origin::from_canonical_parts(Scheme::Https, "example.test", 443).unwrap()
    }
    fn loaded(policy: &mut BrowserPolicy, context: Context) -> TabId {
        let tab = policy.open_tab(context).unwrap();
        let navigation = policy.begin_navigation(tab).unwrap();
        policy.commit_navigation(navigation, origin()).unwrap();
        tab
    }
    fn approved(policy: &mut BrowserPolicy, tab: TabId) -> Grant {
        let prompt = policy.request(tab, 7, Action::Screenshot, 0).unwrap();
        policy.resolve(prompt.id, true, 1).unwrap().unwrap()
    }

    #[test]
    fn consent_is_one_shot_and_never_implicit() {
        let mut policy = BrowserPolicy::default();
        let tab = loaded(&mut policy, Context::AgentTask(7));
        let prompt = policy.request(tab, 7, Action::Screenshot, 0).unwrap();
        assert_eq!(policy.grant_count(), 0);
        let grant = policy.resolve(prompt.id, true, 1).unwrap().unwrap();
        assert_eq!(policy.consume(grant, tab, 7, &Action::Screenshot, 2), Ok(()));
        assert_eq!(policy.consume(grant, tab, 7, &Action::Screenshot, 3), Err(Error::Stale));
    }
    #[test]
    fn denial_cannot_be_replayed_as_approval() {
        let mut policy = BrowserPolicy::default();
        let tab = loaded(&mut policy, Context::AgentTask(7));
        let prompt = policy.request(tab, 7, Action::Input, 0).unwrap();
        assert_eq!(policy.resolve(prompt.id, false, 1), Ok(None));
        assert_eq!(policy.resolve(prompt.id, true, 2), Err(Error::Stale));
    }
    #[test]
    fn browsing_authentication_and_other_tasks_are_isolated() {
        let mut policy = BrowserPolicy::default();
        for context in [Context::Manual, Context::Authentication(7), Context::AgentTask(8)] {
            let tab = loaded(&mut policy, context);
            assert_eq!(policy.request(tab, 7, Action::ReadDocument, 0).unwrap_err(), Error::WrongContext);
        }
    }
    #[test]
    fn grant_cannot_change_action_task_or_tab() {
        let mut policy = BrowserPolicy::default();
        let tab = loaded(&mut policy, Context::AgentTask(7));
        let other = loaded(&mut policy, Context::AgentTask(7));
        for (target, task, action) in [(other, 7, Action::Screenshot), (tab, 8, Action::Screenshot), (tab, 7, Action::Input)] {
            let grant = approved(&mut policy, tab);
            assert_eq!(policy.consume(grant, target, task, &action, 2), Err(Error::Denied));
        }
    }
    #[test]
    fn same_origin_reload_revokes_old_document_authority() {
        let mut policy = BrowserPolicy::default();
        let tab = loaded(&mut policy, Context::AgentTask(7));
        let grant = approved(&mut policy, tab);
        let navigation = policy.begin_navigation(tab).unwrap();
        policy.commit_navigation(navigation, origin()).unwrap();
        assert_eq!(policy.consume(grant, tab, 7, &Action::Screenshot, 2), Err(Error::Stale));
    }
    #[test]
    fn stale_navigation_cannot_replace_the_current_document() {
        let mut policy = BrowserPolicy::default();
        let tab = policy.open_tab(Context::AgentTask(7)).unwrap();
        let stale = policy.begin_navigation(tab).unwrap();
        let current = policy.begin_navigation(tab).unwrap();
        assert_eq!(policy.commit_navigation(stale, origin()), Err(Error::Stale));
        policy.commit_navigation(current, origin()).unwrap();
        assert_eq!(policy.commit_navigation(current, origin()), Err(Error::Stale));
    }
    #[test]
    fn loading_and_crashed_tabs_have_no_usable_document() {
        let mut policy = BrowserPolicy::default();
        let tab = loaded(&mut policy, Context::AgentTask(7));
        let navigation = policy.begin_navigation(tab).unwrap();
        assert_eq!(policy.request(tab, 7, Action::Screenshot, 0).unwrap_err(), Error::NotReady);
        policy.crash(tab).unwrap();
        assert_eq!(policy.commit_navigation(navigation, origin()), Err(Error::Stale));
        assert_eq!(policy.request(tab, 7, Action::Screenshot, 0).unwrap_err(), Error::NotReady);
    }
    #[test]
    fn closing_a_tab_revokes_grants_without_reusing_its_identity() {
        let mut policy = BrowserPolicy::default();
        let tab = loaded(&mut policy, Context::AgentTask(7));
        let grant = approved(&mut policy, tab);
        policy.close_tab(tab).unwrap();
        let replacement = loaded(&mut policy, Context::AgentTask(7));
        assert_ne!(replacement, tab);
        assert_eq!(policy.consume(grant, replacement, 7, &Action::Screenshot, 2), Err(Error::Stale));
    }
    #[test]
    fn independent_tabs_keep_independent_authority() {
        let mut policy = BrowserPolicy::default();
        let first = loaded(&mut policy, Context::AgentTask(7));
        let second = loaded(&mut policy, Context::AgentTask(7));
        let grant = approved(&mut policy, second);
        policy.crash(first).unwrap();
        assert_eq!(policy.consume(grant, second, 7, &Action::Screenshot, 2), Ok(()));
    }
    #[test]
    fn expiry_is_checked_at_approval_and_dispatch() {
        let mut policy = BrowserPolicy::default();
        let tab = loaded(&mut policy, Context::AgentTask(7));
        let prompt = policy.request(tab, 7, Action::Input, 0).unwrap();
        assert_eq!(policy.resolve(prompt.id, true, CONSENT_TTL_MS), Err(Error::Expired));
        let grant = approved(&mut policy, tab);
        assert_eq!(policy.consume(grant, tab, 7, &Action::Screenshot, CONSENT_TTL_MS), Err(Error::Expired));
    }
    #[test]
    fn pending_and_approved_consent_share_a_resource_limit() {
        let mut policy = BrowserPolicy::default();
        let tab = loaded(&mut policy, Context::AgentTask(7));
        for _ in 0..MAX_CONSENTS {
            approved(&mut policy, tab);
        }
        assert_eq!(policy.request(tab, 7, Action::Input, 2).unwrap_err(), Error::Limit);
        assert!(policy.request(tab, 7, Action::Input, CONSENT_TTL_MS).is_ok());
        policy.revoke_task(7);
        assert_eq!(policy.pending_count() + policy.grant_count(), 0);
    }
    #[test]
    fn tab_limit_and_blank_document_are_explicit() {
        let mut policy = BrowserPolicy::default();
        let tab = policy.open_tab(Context::AgentTask(7)).unwrap();
        assert_eq!(policy.request(tab, 7, Action::Input, 0).unwrap_err(), Error::NotReady);
        for _ in 1..MAX_TABS { policy.open_tab(Context::Manual).unwrap(); }
        assert_eq!(policy.open_tab(Context::Manual), Err(Error::Limit));
    }
    #[test]
    fn origin_parts_are_not_url_or_script_inputs() {
        for host in ["", "https://example.test", "example.test@evil.test", "example.test/path", "EXAMPLE.TEST", "a\nb", "-x.test"] {
            assert_eq!(Origin::from_canonical_parts(Scheme::Https, host, 443), Err(Error::InvalidOrigin));
        }
        assert!(Origin::from_canonical_parts(Scheme::Http, "127.0.0.1", 3000).is_ok());
        assert!(Origin::from_canonical_parts(Scheme::Http, "::1", 3000).is_ok());
        assert!(Origin::from_canonical_parts(Scheme::Https, "example.test", 0).is_err());
    }
}
