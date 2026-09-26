//! Related OAuth windows stay hidden and resource-blocked until explicit review.
use super::*;

#[derive(Clone, Copy)]
struct Authority {
    tab: HostTabId,
    navigation: HostNavigationId,
    epoch: u64,
}
#[derive(Default)]
struct Gate {
    authority: Option<Authority>,
    decision: Option<webkit2gtk::PolicyDecision>,
    cancelled: bool,
}
pub(super) struct Pending {
    pub epoch: u64,
    pub url: String,
    pub expires: Instant,
    pub view: Option<WebView>,
    pub completed: Rc<Cell<bool>>,
    gate: Rc<RefCell<Gate>>,
    filter: webkit2gtk::UserContentFilter,
}
impl Pending {
    pub fn activate(
        &self,
        web: &webkit2gtk::WebView,
        tab: HostTabId,
        navigation: HostNavigationId,
        epoch: u64,
    ) {
        let decision = {
            let mut gate = self.gate.borrow_mut();
            gate.authority = Some(Authority {
                tab,
                navigation,
                epoch,
            });
            gate.decision.take()
        };
        if let Some(manager) = web.user_content_manager() {
            manager.remove_filter(&self.filter);
        }
        if let Some(settings) = WebViewExt::settings(web) {
            settings.set_enable_javascript(true);
        }
        if let Some(decision) = decision {
            decision.use_();
        }
    }
}
impl Drop for Pending {
    fn drop(&mut self) {
        if self.view.is_some() {
            let mut gate = self.gate.borrow_mut();
            gate.cancelled = true;
            if let Some(decision) = gate.decision.take() {
                decision.ignore();
            }
            if let Some(view) = &self.view {
                view.webview().stop_loading();
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn create(
    url: String,
    opener: webkit2gtk::WebView,
    pending: Rc<RefCell<BTreeMap<HostTabId, Pending>>>,
    filter: Rc<RefCell<Option<webkit2gtk::UserContentFilter>>>,
    shared: Arc<bridge::Shared>,
    events: Events,
    source: HostTabId,
    navigation: HostNavigationId,
    epoch: u64,
) -> wry::NewWindowResponse {
    let Ok(document) = CommittedDocument::parse(&url) else {
        return wry::NewWindowResponse::Deny;
    };
    let Some(rule) = filter.borrow().clone() else {
        return wry::NewWindowResponse::Deny;
    };
    if shared.epoch(source) != Some(epoch) {
        return wry::NewWindowResponse::Deny;
    }
    let Some(container) = opener
        .parent()
        .and_then(|widget| widget.downcast::<gtk::Container>().ok())
    else {
        return wry::NewWindowResponse::Deny;
    };
    let gate = Rc::new(RefCell::new(Gate::default()));
    let completed = Rc::new(Cell::new(false));
    let child_gate = gate.clone();
    let child_complete = completed.clone();
    let child_pending = pending.clone();
    let child_filter = filter.clone();
    let child_shared = shared.clone();
    let child_events = events.clone();
    let result = WebViewBuilder::new()
        .with_related_view(opener)
        .with_visible(false)
        .with_focused(false)
        .with_devtools(false)
        .with_new_window_req_handler(move |url, features| {
            let authority = child_gate.borrow().authority;
            if let Some(authority) = authority.filter(|_| child_complete.get()) {
                create(
                    url,
                    features.opener.webview,
                    child_pending.clone(),
                    child_filter.clone(),
                    child_shared.clone(),
                    child_events.clone(),
                    authority.tab,
                    authority.navigation,
                    authority.epoch,
                )
            } else {
                wry::NewWindowResponse::Deny
            }
        })
        .with_download_started_handler(|_, _| false)
        .build_gtk(&container);
    let Ok(view) = result else {
        return wry::NewWindowResponse::Deny;
    };
    let web = view.webview();
    WebViewExt::set_settings(&web, &webkit2gtk::Settings::new());
    harden(&web, StoragePartition::Authentication(0));
    let Some(manager) = web.user_content_manager() else {
        return wry::NewWindowResponse::Deny;
    };
    manager.add_filter(&rule);
    if let Some(settings) = WebViewExt::settings(&web) {
        settings.set_enable_javascript(false);
    }
    let policy_gate = gate.clone();
    let policy_shared = shared.clone();
    let expected = document.canonical_url.clone();
    web.connect_decide_policy(move |_, decision, kind| {
        if kind != webkit2gtk::PolicyDecisionType::NavigationAction {
            return false;
        }
        let target = decision
            .dynamic_cast_ref::<webkit2gtk::NavigationPolicyDecision>()
            .and_then(|policy| policy.navigation_action())
            .and_then(|action| action.request())
            .and_then(|request| request.uri())
            .and_then(|uri| CommittedDocument::parse(uri.as_str()).ok());
        let mut gate = policy_gate.borrow_mut();
        if gate.cancelled || target.is_none() {
            decision.ignore();
            return true;
        }
        if let Some(authority) = gate.authority {
            let allowed = policy_shared.epoch(authority.tab) == Some(authority.epoch);
            drop(gate);
            if allowed {
                decision.use_();
            } else {
                decision.ignore();
            }
        } else if gate.decision.is_none()
            && target.is_some_and(|target| target.canonical_url == expected)
        {
            gate.decision = Some(decision.clone());
        } else {
            decision.ignore();
        }
        true
    });
    pending.borrow_mut().insert(
        source,
        Pending {
            epoch,
            url: document.canonical_url.clone(),
            expires: Instant::now() + Duration::from_secs(60),
            view: Some(view),
            completed,
            gate,
            filter: rule,
        },
    );
    events.emit(Event::PopupRequested {
        tab: source,
        navigation,
        url: document.canonical_url,
    });
    wry::NewWindowResponse::Create { webview: web }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn protected_load(
    web: webkit2gtk::WebView,
    url: String,
    root: PathBuf,
    filter: Rc<RefCell<Option<webkit2gtk::UserContentFilter>>>,
    shared: Arc<bridge::Shared>,
    events: Events,
    tab: HostTabId,
    navigation: HostNavigationId,
    epoch: u64,
) -> std::result::Result<(), String> {
    if filter.borrow().is_some() {
        web.load_uri(&url);
        return Ok(());
    }
    let root = root.join("popup-rules");
    private_directory(&root)?;
    let store = webkit2gtk::UserContentFilterStore::new(
        root.to_str().ok_or("Invalid popup rule directory")?,
    );
    let rules = gtk::glib::Bytes::from_static(
        br#"[{"trigger":{"url-filter":".*","resource-type":["image","style-sheet","script","font","raw","svg-document","media","popup"]},"action":{"type":"block"}}]"#,
    );
    store.save(
        "synara-popup-quarantine",
        &rules,
        None::<&gio::Cancellable>,
        move |result| {
            if shared.epoch(tab) != Some(epoch) {
                return;
            }
            match result {
                Ok(rule) => {
                    *filter.borrow_mut() = Some(rule);
                    web.load_uri(&url);
                }
                Err(_) => events.emit(Event::Failed {
                    tab,
                    navigation,
                    error: "Could not protect sign-in popups".into(),
                }),
            }
        },
    );
    Ok(())
}
