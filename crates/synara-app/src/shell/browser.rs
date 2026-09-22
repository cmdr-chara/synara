//! Pane-local native controls over the existing browser owner, never a web UI.
mod native;
use super::*;
use crate::ui::{self, Glyph, palette};
use browser_domain::{
    BrowserProfile, HostTabId, NavigationKind,
    session::{RequestState, TabView},
};
use gpui::AnyElement;
pub(super) struct BrowserView {
    #[cfg(target_os = "linux")]
    native: native::Host,
    #[cfg(target_os = "linux")]
    native_task: Option<gpui::Task<()>>,
    address: Entity<TextEntry>,
    selected: Option<HostTabId>,
    pub(super) error: Option<String>,
    confirmation: Option<TaskId>,
    pub(super) busy: bool,
    _subscription: Subscription,
}
impl BrowserView {
    pub fn new(controller: &Arc<Controller>, root: PathBuf, cx: &mut Context<Shell>) -> Self {
        #[cfg(target_os = "linux")]
        let (native, install_error) = {
            let (host, port) = browser_domain::native::NativeHost::new(root);
            let error = controller
                .browser
                .with(|s, _| s.install_port(port))
                .err()
                .map(|e| e.to_string());
            (std::rc::Rc::new(std::cell::RefCell::new(host)), error)
        };
        #[cfg(not(target_os = "linux"))]
        let _ = (controller, root);
        let address = cx.new(|cx| {
            TextEntry::new(
                "https://... or http://localhost:port",
                EntryMode::SingleLine,
                34.,
                cx,
            )
        });
        let sub = cx.subscribe(&address, |this, _, event, cx| {
            if matches!(event, EntryEvent::Submit) {
                this.browser_navigate(NavigationKind::Push, cx);
            }
            cx.notify();
        });
        Self {
            #[cfg(target_os = "linux")]
            native,
            #[cfg(target_os = "linux")]
            native_task: None,
            address,
            selected: None,
            error: {
                #[cfg(target_os = "linux")]
                {
                    install_error
                }
                #[cfg(not(target_os = "linux"))]
                {
                    None
                }
            },
            confirmation: None,
            busy: false,
            _subscription: sub,
        }
    }
}
impl Shell {
    fn browser_open(&mut self, cx: &mut Context<Self>) {
        match self
            .controller
            .browser
            .with(|s, _| s.open(BrowserProfile::Manual))
        {
            Ok(tab) => self.browser_select(tab, cx),
            Err(e) => self.browser.error = Some(e.to_string()),
        }
        cx.notify();
    }
    fn browser_select(&mut self, tab: HostTabId, cx: &mut Context<Self>) {
        self.browser.selected = Some(tab);
        if let Ok(tabs) = self.controller.browser.with(|s, _| Ok(s.tabs())) {
            if let Some(tab) = tabs.iter().find(|t| t.id == tab) {
                self.browser.address.update(cx, |entry, cx| {
                    entry.set_text(tab.url.clone().unwrap_or_default(), cx)
                });
            }
        }
        self.browser.error = None;
        cx.notify();
    }
    fn browser_navigate(&mut self, kind: NavigationKind, cx: &mut Context<Self>) {
        let Some(tab) = self.browser.selected else {
            self.browser.error = Some("Create or select a tab first.".into());
            cx.notify();
            return;
        };
        if matches!(
            kind,
            NavigationKind::Back | NavigationKind::Forward | NavigationKind::Reload
        ) {
            let allowed = self.controller.browser.with(|s, _| {
                Ok(s.tabs()
                    .iter()
                    .any(|state| state.id == tab && history_action_allowed(state, kind)))
            });
            match allowed {
                Ok(true) => (),
                Ok(false) => return,
                Err(error) => {
                    self.browser.error = Some(error.to_string());
                    cx.notify();
                    return;
                }
            }
        }
        let url = self.browser.address.read(cx).text().to_owned();
        self.browser.error = self
            .controller
            .browser
            .with(|s, n| s.user_navigate(tab, &url, kind, n))
            .err()
            .map(|e| e.to_string());
        cx.notify();
    }
    fn browser_history_control(
        &self,
        id: &'static str,
        label: &'static str,
        glyph: Option<Glyph>,
        kind: NavigationKind,
        active: Option<&TabView>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let enabled = active.is_some_and(|tab| history_action_allowed(tab, kind));
        ui::action(
            id,
            label,
            glyph,
            false,
            cx.listener(move |this, _: &(), _, cx| {
                if enabled {
                    this.browser_navigate(kind, cx);
                }
            }),
        )
        .when(!enabled, |control| {
            control
                .opacity(0.4)
                .cursor_default()
                .aria_description(
                    "Unavailable until navigation completes and this history entry exists",
                )
                .tab_index(-1)
        })
        .relative()
        .child(browser_state_probe(id, enabled))
        .into_any_element()
    }
    fn browser_enable(&mut self, cx: &mut Context<Self>) {
        if self.browser.busy {
            return;
        }
        let Some(task) = self.browser.confirmation.take() else {
            return;
        };
        // Confirmation pins the task, not whichever thread happens to be selected later.
        self.browser.busy = true;
        let controller = self.controller.clone();
        let sender = self.sender.clone();
        self.runtime.spawn(async move {
            let result = controller
                .configure_browser_use(task, true)
                .await
                .map_err(|e| e.to_string());
            let _ = sender.send(Update::BrowserConfigured(result)).await;
        });
        cx.notify();
    }
    pub(super) fn browser_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let snapshot = self
            .controller
            .browser
            .with(|s, _| Ok((s.tabs(), s.requests(), s.capabilities())));
        let (tabs, requests, _capabilities) = match snapshot {
            Ok(v) => v,
            Err(e) => return div().p_4().child(e.to_string()).into_any_element(),
        };
        let active = tabs.iter().find(|t| Some(t.id) == self.browser.selected);
        let mut tabbar = div()
            .flex()
            .items_center()
            .gap_1()
            .flex_wrap()
            .border_b_1()
            .border_color(rgb(palette().border));
        for (slot, tab) in tabs.iter().enumerate() {
            let id = tab.id;
            let label = format!(
                "{}{}",
                if matches!(tab.profile, BrowserProfile::AgentTask { .. }) {
                    "Agent: "
                } else {
                    ""
                },
                tab.title
            );
            tabbar = tabbar
                .child(
                    ui::action(
                        format!("browser-tab-{id:?}"),
                        label,
                        Some(Glyph::Browser),
                        Some(id) == self.browser.selected,
                        cx.listener(move |this, _: &(), _, cx| this.browser_select(id, cx)),
                    )
                    .relative()
                    .child(ui::layout_probe_slot("browser-tab", slot)),
                )
                .child(
                    ui::action(
                        format!("browser-close-{id:?}"),
                        "Close tab",
                        Some(Glyph::Close),
                        false,
                        cx.listener(move |this, _: &(), _, cx| {
                            this.browser.error = this
                                .controller
                                .browser
                                .with(|s, _| s.close(id))
                                .err()
                                .map(|e| e.to_string());
                            if this.browser.selected == Some(id) {
                                this.browser.selected = None;
                            }
                            cx.notify();
                        }),
                    )
                    .relative()
                    .child(ui::layout_probe_slot("browser-close", slot)),
                );
        }
        tabbar = tabbar.child(
            ui::action(
                "browser-new",
                "New tab",
                Some(Glyph::Plus),
                false,
                cx.listener(|this, _: &(), _, cx| this.browser_open(cx)),
            )
            .relative()
            .child(ui::layout_probe("browser-new")),
        );
        let toolbar = div()
            .flex()
            .items_center()
            .gap_1()
            .py_2()
            .child(self.browser_history_control(
                "browser-back",
                "Back",
                Some(Glyph::Back),
                NavigationKind::Back,
                active,
                cx,
            ))
            .child(self.browser_history_control(
                "browser-forward",
                "Forward",
                Some(Glyph::Forward),
                NavigationKind::Forward,
                active,
                cx,
            ))
            .child(self.browser_history_control(
                "browser-reload",
                "Reload",
                None,
                NavigationKind::Reload,
                active,
                cx,
            ))
            .child(
                ui::action(
                    "browser-stop",
                    "Stop",
                    Some(Glyph::Stop),
                    false,
                    cx.listener(|this, _: &(), _, cx| {
                        if let Some(id) = this.browser.selected {
                            this.browser.error = this
                                .controller
                                .browser
                                .with(|s, _| s.stop(id))
                                .err()
                                .map(|e| e.to_string());
                        }
                        cx.notify();
                    }),
                )
                .relative()
                .child(ui::layout_probe("browser-stop")),
            )
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_w_0()
                    .child(self.browser.address.clone())
                    .child(ui::layout_probe("browser-address")),
            )
            .child(
                ui::action(
                    "browser-go",
                    "Go",
                    None,
                    false,
                    cx.listener(|this, _: &(), _, cx| {
                        this.browser_navigate(NavigationKind::Push, cx)
                    }),
                )
                .relative()
                .child(ui::layout_probe("browser-go")),
            );
        let mut pane = div()
            .id("browser-panel")
            .flex()
            .flex_col()
            .size_full()
            .min_h_0()
            .p_3()
            .gap_2()
            .child(tabbar)
            .child(toolbar);
        if let Some(tab) = active {
            pane = pane.child(
                div()
                    .relative()
                    .text_sm()
                    .child(format!(
                        "{} | {} | {}",
                        tab.state,
                        tab.title,
                        tab.url.as_deref().unwrap_or("No committed URL")
                    ))
                    .child(browser_state_probe("browser-ready", tab.state == "ready")),
            );
            if let Some(error) = &tab.error {
                pane = pane.child(div().text_color(rgb(palette().error)).child(error.clone()));
            }
        }
        if let Some(error) = &self.browser.error {
            pane = pane.child(div().text_color(rgb(palette().error)).child(error.clone()));
        }
        pane = pane.child(self.native_browser_surface());
        pane = pane.child(div().text_xs().text_color(rgb(palette().muted)).child("Agent browser use uses isolated task storage. Manual cookies, authentication, clipboard and uploads are not exposed. Captures/downloads require bounded native handles; export is not available in this build."));
        if let Some(task) = self.selected {
            let enabled = self.controller.browser_use_enabled(task);
            pane = pane.child(ui::action(
                "browser-enable",
                if enabled {
                    "Revoke browser use for this task"
                } else {
                    "Enable browser use for this task..."
                },
                Some(Glyph::Shield),
                enabled,
                cx.listener(move |this, _: &(), _, cx| {
                    if enabled {
                        this.controller.revoke_browser_use(task);
                        this.browser.confirmation = None;
                    } else {
                        this.browser.confirmation = Some(task);
                    }
                    cx.notify();
                }),
            ));
        }
        if let Some(task) = self.browser.confirmation {
            pane = pane.child(div().text_sm().child(format!("Grant browser-use tools to task {task}? This closes its idle agent session. Every operation still requires separate approval. No saved permission survives restart.")))
                .child(div().flex().gap_2()
                    .child(ui::action("browser-confirm", "Confirm enable", None, false, cx.listener(|this, _: &(), _, cx| this.browser_enable(cx))))
                    .child(ui::action("browser-dismiss", "Cancel", None, false, cx.listener(|this, _: &(), _, cx| { this.browser.confirmation = None; cx.notify(); }))));
        }
        let mut pending = div()
            .id("browser-permissions")
            .overflow_y_scroll()
            .max_h(px(190.))
            .flex_shrink_0()
            .min_h_0();
        for request in requests {
            let id = request.id;
            let task = request.task;
            let mut row = div()
                .border_b_1()
                .border_color(rgb(palette().border))
                .py_2()
                .gap_1()
                .flex()
                .flex_col()
                .child(format!("Task {task} | {id:?} | {:?}", request.state))
                .child(
                    serde_json::to_string(&request.operation)
                        .unwrap_or_else(|_| "Invalid operation".into()),
                );
            if matches!(request.state, RequestState::AwaitingConsent) {
                for (name, allow) in [("Allow once", true), ("Deny", false)] {
                    row = row.child(ui::action(
                        format!("browser-{id:?}-{allow}"),
                        name,
                        None,
                        false,
                        cx.listener(move |this, _: &(), _, cx| {
                            this.browser.error = this
                                .controller
                                .browser
                                .with(|s, n| s.decide(id, allow, n))
                                .err()
                                .map(|e| e.to_string());
                            cx.notify();
                        }),
                    ));
                }
            }
            row = row.child(ui::action(
                format!("browser-cancel-{id:?}"),
                "Cancel request",
                None,
                false,
                cx.listener(move |this, _: &(), _, cx| {
                    this.browser.error = this
                        .controller
                        .browser
                        .with(|s, n| s.cancel(task, id, n))
                        .err()
                        .map(|e| e.to_string());
                    cx.notify();
                }),
            ));
            if !matches!(
                request.state,
                RequestState::AwaitingConsent | RequestState::Running
            ) {
                row = row.child(ui::action(
                    format!("browser-forget-{id:?}"),
                    "Dismiss receipt",
                    None,
                    false,
                    cx.listener(move |this, _: &(), _, cx| {
                        this.browser.error = this
                            .controller
                            .browser
                            .with(|s, _| s.forget(task, id))
                            .err()
                            .map(|e| e.to_string());
                        cx.notify();
                    }),
                ));
            }
            pending = pending.child(row);
        }
        pane.child(pending).into_any_element()
    }
}

// History is committed by native callbacks, not by request dispatch or network receipt.
fn history_action_allowed(tab: &TabView, kind: NavigationKind) -> bool {
    if matches!(tab.profile, BrowserProfile::AgentTask { .. }) || tab.state == "loading" {
        return false;
    }
    match kind {
        NavigationKind::Back => tab.back,
        NavigationKind::Forward => tab.forward,
        NavigationKind::Reload => tab.url.is_some(),
        _ => false,
    }
}

// Opt-in native test metadata. No URL, title, page content, task or token is logged.
fn browser_state_probe(control: &'static str, enabled: bool) -> impl IntoElement {
    gpui::canvas(
        move |bounds, _, _| {
            tracing::debug!(target: "synara_ui_layout", control, enabled,
            x = f32::from(bounds.origin.x), y = f32::from(bounds.origin.y),
            width = f32::from(bounds.size.width), height = f32::from(bounds.size.height),
            "control-layout");
        },
        |_, _, _, _| {},
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tab(profile: BrowserProfile) -> TabView {
        let mut session = browser_domain::session::Session::default();
        session.open(profile).unwrap();
        let mut tab = session.tabs().remove(0);
        tab.state = "ready".into();
        tab.url = Some("https://example.test/".into());
        tab.back = true;
        tab.forward = true;
        tab
    }
    #[test]
    fn history_controls_wait_for_native_commit_even_when_old_history_flags_are_true() {
        let mut tab = tab(BrowserProfile::Manual);
        for kind in [
            NavigationKind::Back,
            NavigationKind::Forward,
            NavigationKind::Reload,
        ] {
            assert!(history_action_allowed(&tab, kind));
        }
        tab.state = "loading".into();
        for kind in [
            NavigationKind::Back,
            NavigationKind::Forward,
            NavigationKind::Reload,
        ] {
            assert!(!history_action_allowed(&tab, kind));
        }
        tab.state = "ready".into();
        tab.back = false;
        assert!(!history_action_allowed(&tab, NavigationKind::Back));
        assert!(history_action_allowed(&tab, NavigationKind::Forward));
    }
    #[test]
    fn manual_history_controls_do_not_grant_agent_navigation_authority() {
        let tab = tab(BrowserProfile::AgentTask { task: 42 });
        for kind in [
            NavigationKind::Back,
            NavigationKind::Forward,
            NavigationKind::Reload,
        ] {
            assert!(!history_action_allowed(&tab, kind));
        }
    }
}
