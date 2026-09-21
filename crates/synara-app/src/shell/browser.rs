//! Pane-local native controls over the existing browser owner, never a web UI.
use super::*;
use crate::ui::{self, Glyph, palette};
use browser_domain::{BrowserProfile, HostTabId, NavigationKind, session::RequestState};
use gpui::AnyElement;
pub(super) struct BrowserView {
    address: Entity<TextEntry>, selected: Option<HostTabId>, pub(super) error: Option<String>,
    confirmation: Option<TaskId>, pub(super) busy: bool, _subscription: Subscription,
}
impl BrowserView {
    pub fn new(cx: &mut Context<Shell>) -> Self {
        let address = cx.new(|cx| TextEntry::new("https://... or http://localhost:port", EntryMode::SingleLine, 34., cx));
        let sub = cx.subscribe(&address, |this, _, event, cx| {
            if matches!(event, EntryEvent::Submit) { this.browser_navigate(NavigationKind::Push, cx); }
            cx.notify();
        });
        Self { address, selected: None, error: None, confirmation: None, busy: false, _subscription: sub }
    }
}
impl Shell {
    fn browser_open(&mut self, cx: &mut Context<Self>) {
        match self.controller.browser.with(|s, _| s.open(BrowserProfile::Manual)) {
            Ok(tab) => self.browser_select(tab, cx), Err(e) => self.browser.error = Some(e.to_string()),
        }
        cx.notify();
    }
    fn browser_select(&mut self, tab: HostTabId, cx: &mut Context<Self>) {
        self.browser.selected = Some(tab);
        if let Ok(tabs) = self.controller.browser.with(|s, _| Ok(s.tabs())) {
            if let Some(tab) = tabs.iter().find(|t| t.id == tab) {
                self.browser.address.update(cx, |entry, cx| entry.set_text(tab.url.clone().unwrap_or_default(), cx));
            }
        }
        self.browser.error = None; cx.notify();
    }
    fn browser_navigate(&mut self, kind: NavigationKind, cx: &mut Context<Self>) {
        let Some(tab) = self.browser.selected else { self.browser.error = Some("Create or select a tab first.".into()); cx.notify(); return; };
        let url = self.browser.address.read(cx).text().to_owned();
        self.browser.error = self.controller.browser.with(|s, n| s.user_navigate(tab, &url, kind, n)).err().map(|e| e.to_string()); cx.notify();
    }
    fn browser_enable(&mut self, cx: &mut Context<Self>) {
        if self.browser.busy { return; }
        let Some(task) = self.browser.confirmation.take() else { return; };
        // Confirmation pins the task, not whichever thread happens to be selected later.
        self.browser.busy = true;
        let controller = self.controller.clone(); let sender = self.sender.clone();
        self.runtime.spawn(async move {
            let result = controller.configure_browser_use(task, true).await.map_err(|e| e.to_string());
            let _ = sender.send(Update::BrowserConfigured(result)).await;
        }); cx.notify();
    }
    pub(super) fn browser_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let snapshot = self.controller.browser.with(|s, _| Ok((s.tabs(), s.requests(), s.capabilities())));
        let (tabs, requests, capabilities) = match snapshot {
            Ok(v) => v, Err(e) => return div().p_4().child(e.to_string()).into_any_element(),
        };
        let active = tabs.iter().find(|t| Some(t.id) == self.browser.selected);
        let mut tabbar = div().flex().items_center().gap_1().flex_wrap().border_b_1().border_color(rgb(palette().border));
        for tab in &tabs {
            let id = tab.id;
            let label = format!("{}{}", if matches!(tab.profile, BrowserProfile::AgentTask { .. }) { "Agent: " } else { "" }, tab.title);
            tabbar = tabbar.child(ui::action(format!("browser-tab-{id:?}"), label, Some(Glyph::Browser), Some(id) == self.browser.selected,
                cx.listener(move |this, _: &(), _, cx| this.browser_select(id, cx))))
                .child(ui::action(format!("browser-close-{id:?}"), "Close tab", Some(Glyph::Close), false,
                    cx.listener(move |this, _: &(), _, cx| {
                        this.browser.error = this.controller.browser.with(|s, _| s.close(id)).err().map(|e| e.to_string());
                        if this.browser.selected == Some(id) { this.browser.selected = None; } cx.notify();
                    })));
        }
        tabbar = tabbar.child(ui::action("browser-new", "New tab", Some(Glyph::Plus), false, cx.listener(|this, _: &(), _, cx| this.browser_open(cx))));
        let toolbar = div().flex().items_center().gap_1().py_2()
            .child(ui::action("browser-back", "Back", Some(Glyph::Back), false, cx.listener(|this, _: &(), _, cx| this.browser_navigate(NavigationKind::Back, cx))))
            .child(ui::action("browser-forward", "Forward", Some(Glyph::Forward), false, cx.listener(|this, _: &(), _, cx| this.browser_navigate(NavigationKind::Forward, cx))))
            .child(ui::action("browser-reload", "Reload", None, false, cx.listener(|this, _: &(), _, cx| this.browser_navigate(NavigationKind::Reload, cx))))
            .child(ui::action("browser-stop", "Stop", Some(Glyph::Stop), false, cx.listener(|this, _: &(), _, cx| {
                if let Some(id) = this.browser.selected { this.browser.error = this.controller.browser.with(|s, _| s.stop(id)).err().map(|e| e.to_string()); } cx.notify();
            })))
            .child(div().flex_1().min_w_0().child(self.browser.address.clone()))
            .child(ui::action("browser-go", "Go", None, false, cx.listener(|this, _: &(), _, cx| this.browser_navigate(NavigationKind::Push, cx))));
        let mut pane = div().id("browser-panel").flex().flex_col().size_full().min_h_0().p_3().gap_2()
            .child(tabbar).child(toolbar);
        if let Some(tab) = active {
            pane = pane.child(div().text_sm().child(format!("{} | {} | {}", tab.state, tab.title, tab.url.as_deref().unwrap_or("No committed URL"))));
            if let Some(error) = &tab.error { pane = pane.child(div().text_color(rgb(palette().error)).child(error.clone())); }
        }
        if let Some(error) = &self.browser.error { pane = pane.child(div().text_color(rgb(palette().error)).child(error.clone())); }
        pane = pane.child(div().min_h(px(90.)).border_y_1().border_color(rgb(palette().border)).p_3().child(
            if capabilities.navigation { "Native host connected. Platform viewport attachment requires native-host acceptance." }
            else { "Native webview host is not installed in this build. No rendered page is simulated. Tabs and permission state remain visible. HTTP(S) local-server addresses can be entered, but will not open without a host." }));
        pane = pane.child(div().text_xs().text_color(rgb(palette().muted)).child("Agent browser use uses isolated task storage. Manual cookies, authentication, clipboard and uploads are not exposed. Captures/downloads require bounded native handles; export is not available in this build."));
        if let Some(task) = self.selected {
            let enabled = self.controller.browser_use_enabled(task);
            pane = pane.child(ui::action("browser-enable", if enabled { "Revoke browser use for this task" } else { "Enable browser use for this task..." }, Some(Glyph::Shield), enabled,
                cx.listener(move |this, _: &(), _, cx| {
                    if enabled { this.controller.revoke_browser_use(task); this.browser.confirmation = None; }
                    else { this.browser.confirmation = Some(task); } cx.notify();
                })));
        }
        if let Some(task) = self.browser.confirmation {
            pane = pane.child(div().text_sm().child(format!("Grant browser-use tools to task {task}? This closes its idle agent session. Every operation still requires separate approval. No saved permission survives restart.")))
                .child(div().flex().gap_2()
                    .child(ui::action("browser-confirm", "Confirm enable", None, false, cx.listener(|this, _: &(), _, cx| this.browser_enable(cx))))
                    .child(ui::action("browser-dismiss", "Cancel", None, false, cx.listener(|this, _: &(), _, cx| { this.browser.confirmation = None; cx.notify(); }))));
        }
        let mut pending = div().id("browser-permissions").overflow_y_scroll().flex_1().min_h_0();
        for request in requests {
            let id = request.id; let task = request.task;
            let mut row = div().border_b_1().border_color(rgb(palette().border)).py_2().gap_1().flex().flex_col()
                .child(format!("Task {task} | {id:?} | {:?}", request.state))
                .child(serde_json::to_string(&request.operation).unwrap_or_else(|_| "Invalid operation".into()));
            if matches!(request.state, RequestState::AwaitingConsent) {
                for (name, allow) in [("Allow once", true), ("Deny", false)] {
                    row = row.child(ui::action(format!("browser-{id:?}-{allow}"), name, None, false,
                        cx.listener(move |this, _: &(), _, cx| {
                            this.browser.error = this.controller.browser.with(|s, n| s.decide(id, allow, n)).err().map(|e| e.to_string()); cx.notify();
                        })));
                }
            }
            row = row.child(ui::action(format!("browser-cancel-{id:?}"), "Cancel request", None, false,
                cx.listener(move |this, _: &(), _, cx| { this.browser.error = this.controller.browser.with(|s, n| s.cancel(task, id, n)).err().map(|e| e.to_string()); cx.notify(); })));
            if !matches!(request.state, RequestState::AwaitingConsent | RequestState::Running) {
                row = row.child(ui::action(format!("browser-forget-{id:?}"), "Dismiss receipt", None, false, cx.listener(move |this, _: &(), _, cx| {
                    this.browser.error = this.controller.browser.with(|s, _| s.forget(task, id)).err().map(|e| e.to_string()); cx.notify();
                })));
            }
            pending = pending.child(row);
        }
        pane.child(pending).into_any_element()
    }
}
