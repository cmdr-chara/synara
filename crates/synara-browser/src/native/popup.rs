//! Hold the native new-window decision before creating a window or networking.
//! Approval resumes the original request into a related view, preserving opener.
use super::*;

pub(super) struct Pending {
    pub epoch: u64,
    pub url: String,
    pub expires: Instant,
    pub decision: Option<webkit2gtk::PolicyDecision>,
}
impl Drop for Pending {
    fn drop(&mut self) {
        // WebKit defaults an undecided, released policy to ALLOW, so every
        // replacement, cancellation, stale epoch and timeout explicitly denies.
        if let Some(decision) = self.decision.take() {
            decision.ignore();
        }
    }
}
pub(super) struct Approved {
    pub source_epoch: u64,
    pub tab: HostTabId,
    pub epoch: u64,
    pub url: String,
    pub webview: webkit2gtk::WebView,
    pub expires: Instant,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn defer(
    web: &webkit2gtk::WebView,
    pending: Rc<RefCell<BTreeMap<HostTabId, Pending>>>,
    shared: Arc<bridge::Shared>,
    events: Events,
    ready: Rc<Cell<bool>>,
    tab: HostTabId,
    navigation: HostNavigationId,
    epoch: u64,
) {
    web.connect_decide_policy(move |_, decision, kind| {
        if kind != webkit2gtk::PolicyDecisionType::NewWindowAction {
            return false;
        }
        let target = decision
            .dynamic_cast_ref::<webkit2gtk::NavigationPolicyDecision>()
            .and_then(|policy| policy.navigation_action())
            .and_then(|action| action.request())
            .and_then(|request| request.uri())
            .and_then(|uri| CommittedDocument::parse(uri.as_str()).ok());
        let Some(target) = target.filter(|_| ready.get() && shared.epoch(tab) == Some(epoch))
        else {
            decision.ignore();
            return true;
        };
        pending.borrow_mut().insert(
            tab,
            Pending {
                epoch,
                url: target.canonical_url.clone(),
                expires: Instant::now() + Duration::from_secs(60),
                decision: Some(decision.clone()),
            },
        );
        events.emit(Event::PopupRequested {
            tab,
            navigation,
            url: target.canonical_url,
        });
        true
    });
}
