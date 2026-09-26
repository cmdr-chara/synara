//! Pane-local native controls over the existing browser owner, never a web UI.
mod authentication;
mod native;
use super::*;
use crate::ui::{self, Glyph, palette};
use browser_domain::{
    BrowserProfile, HostTabId, NavigationKind,
    session::{RequestState, RuntimeDiagnosticKind, TabView},
    session_restore::ManualTabRestoreStore,
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
    restore: Option<ManualTabRestoreStore>,
    pub(super) busy: bool,
    diagnostics_open: bool,
    runtime_diagnostics_open: bool,
    authentication_flows: BTreeMap<u128, authentication::Flow>,
    next_authentication_flow: u128,
    _subscription: Subscription,
}
impl BrowserView {
    pub fn new(controller: &Arc<Controller>, root: PathBuf, cx: &mut Context<Shell>) -> Self {
        let (restore, restore_error) = match ManualTabRestoreStore::open(root.clone()) {
            Ok(store) => (Some(store), None),
            Err(error) => (None, Some(error.to_string())),
        };
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
        #[cfg(target_os = "linux")]
        let (restored_selected, restore_start_error) = {
            let urls = restore
                .as_ref()
                .filter(|store| store.enabled())
                .map(|store| store.urls().to_vec())
                .unwrap_or_default();
            let mut last = None;
            let mut failure = None;
            if !urls.is_empty() {
                let result = controller.browser.with(|session, now| {
                    for url in &urls {
                        let tab = session.open(BrowserProfile::Manual)?;
                        if let Err(error) =
                            session.user_navigate(tab, url, NavigationKind::Push, now)
                        {
                            let _ = session.close(tab);
                            failure = Some(error.to_string());
                            break;
                        }
                        last = Some(tab);
                    }
                    Ok(())
                });
                if let Err(error) = result {
                    failure = Some(error.to_string());
                }
            }
            (last, failure)
        };
        #[cfg(not(target_os = "linux"))]
        let (restored_selected, restore_start_error): (Option<HostTabId>, Option<String>) =
            (None, None);
        let restored_address = if restored_selected.is_some() {
            restore
                .as_ref()
                .and_then(|store| store.urls().last().cloned())
                .unwrap_or_default()
        } else {
            String::new()
        };
        let address = cx.new(|cx| {
            let mut entry = TextEntry::new(
                "https://... or http://localhost:port",
                EntryMode::SingleLine,
                34.,
                cx,
            );
            if !restored_address.is_empty() {
                entry.set_text(restored_address, cx);
            }
            entry
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
            selected: restored_selected,
            error: {
                let restore_error = restore_start_error.or(restore_error);
                #[cfg(target_os = "linux")]
                {
                    restore_error.or(install_error)
                }
                #[cfg(not(target_os = "linux"))]
                {
                    restore_error
                }
            },
            confirmation: None,
            restore,
            busy: false,
            diagnostics_open: false,
            runtime_diagnostics_open: false,
            authentication_flows: BTreeMap::new(),
            next_authentication_flow: 0,
            _subscription: sub,
        }
    }
}
impl Shell {
    fn browser_manual_restore_urls(&self) -> Result<Vec<String>, String> {
        self.controller
            .browser
            .with(|session, _| {
                let mut urls = session
                    .tabs()
                    .into_iter()
                    .filter(|tab| tab.profile == BrowserProfile::Manual)
                    .filter_map(|tab| tab.url)
                    .collect::<Vec<_>>();
                if urls.len() > 16 {
                    urls.drain(0..urls.len() - 16);
                }
                Ok(urls)
            })
            .map_err(|error| error.to_string())
    }

    pub(super) fn browser_save_manual_restore(&mut self) {
        let Some(enabled) = self
            .browser
            .restore
            .as_ref()
            .map(ManualTabRestoreStore::enabled)
        else {
            return;
        };
        if !enabled {
            return;
        }
        let urls = match self.browser_manual_restore_urls() {
            Ok(urls) => urls,
            Err(error) => {
                self.browser.error = Some(error);
                return;
            }
        };
        if let Some(store) = self.browser.restore.as_mut()
            && let Err(error) = store.save_current(&urls)
        {
            self.browser.error = Some(error.to_string());
        }
    }

    fn browser_toggle_manual_restore(&mut self, cx: &mut Context<Self>) {
        let Some(enabled) = self
            .browser
            .restore
            .as_ref()
            .map(ManualTabRestoreStore::enabled)
        else {
            self.browser.error =
                Some("Manual tab restore storage is unavailable; no URLs were saved.".into());
            cx.notify();
            return;
        };
        let enable = !enabled;
        let urls = if enable {
            match self.browser_manual_restore_urls() {
                Ok(urls) => urls,
                Err(error) => {
                    self.browser.error = Some(error);
                    cx.notify();
                    return;
                }
            }
        } else {
            Vec::new()
        };
        match self
            .browser
            .restore
            .as_mut()
            .expect("store was present above")
            .set_enabled(enable, &urls)
        {
            Ok(()) => {
                self.browser.error = None;
            }
            Err(error) => self.browser.error = Some(error.to_string()),
        }
        cx.notify();
    }

    fn browser_open_popup(&mut self, source: HostTabId, revision: u64, cx: &mut Context<Self>) {
        self.browser_tick_authentication(cx);
        if self.browser.selected != Some(source) {
            return;
        }
        match self
            .controller
            .browser
            .with(|s, now| s.open_reviewed_popup(source, revision, now))
        {
            Ok((tab, url)) => {
                self.browser_select(tab, cx);
                self.browser
                    .address
                    .update(cx, |entry, cx| entry.set_text(url, cx));
                self.browser_save_manual_restore();
            }
            Err(error) => {
                self.browser.error = Some(error.to_string());
                cx.notify();
            }
        }
    }
    fn browser_open(&mut self, cx: &mut Context<Self>) {
        match self
            .controller
            .browser
            .with(|s, _| s.open(BrowserProfile::Manual))
        {
            Ok(tab) => {
                self.browser_select(tab, cx);
                self.browser_save_manual_restore();
            }
            Err(e) => self.browser.error = Some(e.to_string()),
        }
        cx.notify();
    }
    fn browser_select(&mut self, tab: HostTabId, cx: &mut Context<Self>) {
        self.browser.selected = Some(tab);
        if let Ok(tabs) = self.controller.browser.with(|s, _| Ok(s.tabs()))
            && let Some(tab) = tabs.iter().find(|t| t.id == tab)
        {
            self.browser.address.update(cx, |entry, cx| {
                entry.set_text(tab.url.clone().unwrap_or_default(), cx)
            });
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
                match tab.profile {
                    BrowserProfile::AgentTask { .. } => "Agent: ",
                    BrowserProfile::Authentication { .. } => "Sign-in: ",
                    BrowserProfile::Manual => "",
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
                            this.browser_close_tab(id, cx);
                        }),
                    )
                    .relative()
                    .child(ui::layout_probe_slot("browser-close", slot)),
                );
        }
        tabbar = tabbar
            .child(
                ui::action(
                    "browser-new",
                    "New tab",
                    Some(Glyph::Plus),
                    false,
                    cx.listener(|this, _: &(), _, cx| this.browser_open(cx)),
                )
                .relative()
                .child(ui::layout_probe("browser-new")),
            )
            .child(ui::action(
                "browser-manual-restore-toggle",
                if self
                    .browser
                    .restore
                    .as_ref()
                    .is_some_and(ManualTabRestoreStore::enabled)
                {
                    "Restore Manual tabs: On"
                } else {
                    "Restore Manual tabs: Off"
                },
                None,
                self.browser
                    .restore
                    .as_ref()
                    .is_some_and(ManualTabRestoreStore::enabled),
                cx.listener(|this, _: &(), _, cx| this.browser_toggle_manual_restore(cx)),
            ));
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
            .child(
                div().text_xs().text_color(rgb(palette().muted)).child(
                    "Manual tab restore saves up to 16 committed HTTP(S) URLs in Synara's private local browser data. Query strings and fragments are removed; paths remain and may contain secrets. Cookies stay in WebKit's existing local profile; cookie values are never read or exported. AgentTask and Authentication tabs are never restored. Restored pages load when you open the Browser panel.",
                ),
            )
            .child(tabbar)
            .child(toolbar);
        pane = pane.child(self.browser_authentication_controls(cx));
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
        if let Some(tab) = active.filter(|tab| {
            matches!(
                tab.profile,
                BrowserProfile::Manual | BrowserProfile::Authentication { .. }
            )
        }) {
            let tab_id = tab.id;
            if let Ok((Some(preview), revision)) = self
                .controller
                .browser
                .with(|s, _| Ok((s.popup_preview(tab_id)?, s.popup_revision(tab_id)?)))
            {
                let popup = div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(format!("This page requested a new tab: {preview}"))
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(ui::action(
                                "browser-popup-open",
                                "Open in Synara tab",
                                None,
                                false,
                                cx.listener(move |this, _: &(), _, cx| {
                                    this.browser_open_popup(tab_id, revision, cx)
                                }),
                            ))
                            .child(ui::action(
                                "browser-popup-dismiss",
                                "Dismiss",
                                None,
                                false,
                                cx.listener(move |this, _: &(), _, cx| {
                                    this.browser.error = this
                                        .controller
                                        .browser
                                        .with(|s, _| s.dismiss_reviewed_popup(tab_id, revision))
                                        .err()
                                        .map(|e| e.to_string());
                                    cx.notify();
                                }),
                            )),
                    );
                pane = pane.child(popup);
            }
        }
        if let Some(tab) = active.filter(|tab| tab.profile == BrowserProfile::Manual) {
            let tab_id = tab.id;
            let diagnostics = self
                .controller
                .browser
                .with(|s, _| s.manual_diagnostics(tab_id));
            if let Ok(diagnostics) = diagnostics {
                pane = pane.child(
                    div()
                        .flex()
                        .gap_2()
                        .child(ui::action(
                            "browser-diagnostics-toggle",
                            format!("Network diagnostics ({})", diagnostics.len()),
                            None,
                            self.browser.diagnostics_open,
                            cx.listener(|this, _: &(), _, cx| {
                                this.browser.diagnostics_open = !this.browser.diagnostics_open;
                                cx.notify();
                            }),
                        ))
                        .child(ui::action(
                            "browser-diagnostics-clear",
                            "Clear",
                            None,
                            false,
                            cx.listener(move |this, _: &(), _, cx| {
                                this.browser.error = this
                                    .controller
                                    .browser
                                    .with(|s, _| s.clear_manual_diagnostics(tab_id))
                                    .err()
                                    .map(|e| e.to_string());
                                cx.notify();
                            }),
                        )),
                );
                if self.browser.diagnostics_open {
                    pane = pane.child(div().id("browser-network-diagnostics").max_h(px(160.)).overflow_y_scroll().flex().flex_col()
                        .child(div().text_xs().text_color(rgb(palette().muted)).child("Manual tab only. URL credentials, query values and fragments are omitted; request headers and bodies are never captured."))
                        .children(diagnostics.iter().rev().take(30).map(|entry| {
                            div().text_xs().child(format!("{} | {}{}", entry.status.map(|v| v.to_string()).unwrap_or_else(|| "—".into()), entry.url, entry.error.as_ref().map(|v| format!(" | {v}")).unwrap_or_default()))
                        })));
                }
            }
            let runtime_diagnostics = self
                .controller
                .browser
                .with(|s, _| s.manual_runtime_diagnostics(tab_id));
            if let Ok(runtime_diagnostics) = runtime_diagnostics {
                let controls = div()
                    .flex()
                    .gap_2()
                    .child(ui::action(
                        "browser-runtime-diagnostics-toggle",
                        format!("Runtime errors ({})", runtime_diagnostics.len()),
                        None,
                        self.browser.runtime_diagnostics_open,
                        cx.listener(|this, _: &(), _, cx| {
                            this.browser.runtime_diagnostics_open =
                                !this.browser.runtime_diagnostics_open;
                            cx.notify();
                        }),
                    ))
                    .child(ui::action(
                        "browser-runtime-diagnostics-clear",
                        "Clear runtime errors",
                        None,
                        false,
                        cx.listener(move |this, _: &(), _, cx| {
                            this.browser.error = this
                                .controller
                                .browser
                                .with(|s, _| s.clear_manual_runtime_diagnostics(tab_id))
                                .err()
                                .map(|e| e.to_string());
                            cx.notify();
                        }),
                    ));
                #[cfg(target_os = "linux")]
                let controls = controls.child(ui::action(
                    "browser-open-web-inspector",
                    "Open Web Inspector",
                    Some(Glyph::Debug),
                    false,
                    cx.listener(move |this, _: &(), _, cx| {
                        this.browser.error = this
                            .browser
                            .native
                            .borrow()
                            .open_manual_inspector(tab_id)
                            .err()
                            .map(|e| e.to_string());
                        cx.notify();
                    }),
                ));
                pane = pane.child(controls);
                if self.browser.runtime_diagnostics_open {
                    pane = pane.child(
                        div()
                            .id("browser-runtime-diagnostics")
                            .max_h(px(160.))
                            .overflow_y_scroll()
                            .flex()
                            .flex_col()
                            .child(
                                div().text_xs().text_color(rgb(palette().muted)).child(
                                    "Manual tab only. Captures uncaught exceptions and unhandled promise rejections with line numbers. Message text, rejection values, URLs, headers and page content are discarded. Open Web Inspector to inspect the full console locally.",
                                ),
                            )
                            .children(runtime_diagnostics.iter().rev().take(30).map(|entry| {
                                let kind = match entry.kind {
                                    RuntimeDiagnosticKind::UncaughtException => {
                                        "Uncaught JavaScript exception"
                                    }
                                    RuntimeDiagnosticKind::UnhandledRejection => {
                                        "Unhandled promise rejection"
                                    }
                                };
                                let location = match (entry.line, entry.column) {
                                    (Some(line), Some(column)) => {
                                        format!(" | line {line}, column {column}")
                                    }
                                    (Some(line), None) => format!(" | line {line}"),
                                    _ => String::new(),
                                };
                                div().text_xs().child(format!("{kind}{location}"))
                            })),
                    );
                }
            }
        }
        pane = pane.child(self.native_browser_surface());
        pane = pane.child(div().text_xs().text_color(rgb(palette().muted)).child("On Linux/X11, use a manual page's context menu to copy its viewport image or save a linked HTTP(S) file. Agent tabs remain isolated: no manual cookies, file picker, clipboard or download access."));
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
