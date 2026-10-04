//! Explicit URL elicitation handoff into request-owned ephemeral browser tabs.
use super::*;

pub(super) struct Flow {
    pub(super) key: InteractionKey,
    scope: InteractionScope,
    session: String,
    cancelled: tokio_util::sync::CancellationToken,
    pub(super) url: String,
    initial: bool,
    expires: u64,
    label: String,
}

impl Shell {
    fn browser_authentication_active(&self, flow: &Flow) -> bool {
        !flow.cancelled.is_cancelled()
            && self.controller.browser.now() < flow.expires
            && self.pending.get(&flow.key).is_some_and(|pending| {
                pending.is_active()
                    && pending.context().scope == flow.scope
                    && pending.context().session_id == flow.session
                    && matches!(pending, UiInteraction::Input { request, .. }
                        if request.url.as_deref() == Some(flow.url.as_str()) && request.fields.is_empty())
            })
    }

    pub(in crate::shell) fn browser_authentication_request(
        &mut self,
        key: InteractionKey,
        url: String,
        cx: &mut Context<Self>,
    ) {
        if !cfg!(target_os = "linux") {
            self.error = Some("Private sign-in tabs require the Linux native browser. Use the external browser action on this platform.".into());
            cx.notify();
            return;
        }
        let Some(UiInteraction::Input {
            context, request, ..
        }) = self.pending.get(&key).filter(|pending| pending.is_active())
        else {
            return;
        };
        if request.url.as_deref() != Some(url.as_str())
            || !request.fields.is_empty()
            || browser_domain::CommittedDocument::parse(&url).is_err()
        {
            self.error = Some("This sign-in website request is no longer available.".into());
            cx.notify();
            return;
        }
        if self
            .browser
            .authentication_flows
            .values()
            .any(|flow| flow.key == key)
        {
            self.panel = Panel::Browser;
            cx.notify();
            return;
        }
        let Some(id) = self.browser.next_authentication_flow.checked_add(1) else {
            return;
        };
        let task = self
            .catalog
            .tasks
            .iter()
            .find(|task| task.thread_id == key.0);
        let label = task
            .map(|task| format!("{} / {}", task.agent_id, task.title))
            .unwrap_or_else(|| format!("Thread {}", key.0));
        let url = task
            .and_then(|task| {
                self.browser
                    .owned_restore
                    .as_ref()
                    .and_then(|store| store.auth_url(task.id.0.as_u128(), &url))
            })
            .unwrap_or(url);
        self.browser.next_authentication_flow = id;
        self.browser.authentication_flows.insert(
            id,
            Flow {
                key,
                scope: context.scope,
                session: context.session_id.clone(),
                cancelled: context.cancelled.clone(),
                url,
                initial: true,
                expires: self.controller.browser.now().saturating_add(300_000),
                label,
            },
        );
        self.panel = Panel::Browser;
        self.browser.error = None;
        cx.notify();
    }

    fn browser_close_authentication_flow(&mut self, id: u128) {
        self.browser.authentication_flows.remove(&id);
        if self.browser.cookie_importing == Some(id) {
            self.browser.cookie_importing = None;
        }
        let mut selected_closed = false;
        let selected = self.browser.selected;
        let result = self.controller.browser.with(|session, _| {
            for tab in session
                .tabs()
                .into_iter()
                .filter(|tab| tab.profile == (BrowserProfile::Authentication { flow: id }))
            {
                selected_closed |= selected == Some(tab.id);
                session.close(tab.id)?;
            }
            Ok(())
        });
        if selected_closed {
            self.browser.selected = None;
        }
        if let Err(error) = result {
            self.browser.error = Some(error.to_string());
        }
    }

    pub(in crate::shell) fn browser_close_authentication_request(&mut self, key: &InteractionKey) {
        let ids: Vec<_> = self
            .browser
            .authentication_flows
            .iter()
            .filter_map(|(id, flow)| (&flow.key == key).then_some(*id))
            .collect();
        for id in ids {
            self.browser_close_authentication_flow(id);
        }
    }

    pub(super) fn browser_tick_authentication(&mut self, cx: &mut Context<Self>) {
        let stale: Vec<_> = self
            .browser
            .authentication_flows
            .iter()
            .filter_map(|(id, flow)| (!self.browser_authentication_active(flow)).then_some(*id))
            .collect();
        for id in stale {
            self.browser_close_authentication_flow(id);
            cx.notify();
        }
        let ready = self
            .controller
            .browser
            .with(|session, _| Ok(session.capabilities().navigation))
            .unwrap_or(false);
        if !ready {
            return;
        }
        let queued: Vec<_> = self
            .browser
            .authentication_flows
            .iter()
            .filter_map(|(id, flow)| flow.initial.then_some((*id, flow.url.clone())))
            .collect();
        for (flow, url) in queued {
            let result = self.controller.browser.with(|session, now| {
                let tab = session.open(BrowserProfile::Authentication { flow })?;
                if let Err(error) = session.user_navigate(tab, &url, NavigationKind::Push, now) {
                    let _ = session.close(tab);
                    return Err(error);
                }
                Ok(tab)
            });
            match result {
                Ok(tab) => {
                    if let Some(flow) = self.browser.authentication_flows.get_mut(&flow) {
                        flow.initial = false;
                    }
                    self.browser_select(tab, cx);
                }
                Err(error) => {
                    self.browser_close_authentication_flow(flow);
                    self.browser.error = Some(error.to_string());
                    cx.notify();
                }
            }
        }
    }

    fn browser_finish_authentication(&mut self, id: u128, accepted: bool, cx: &mut Context<Self>) {
        let Some(flow) = self.browser.authentication_flows.get(&id) else {
            return;
        };
        if accepted && flow.initial {
            return;
        }
        let active = self.browser_authentication_active(flow);
        let key = flow.key.clone();
        self.browser_close_authentication_flow(id);
        if active {
            if let Some(UiInteraction::Input { response, .. }) = self.pending.remove(&key) {
                let result = response.send(if accepted {
                    UserInputResponse::Accept {
                        values: BTreeMap::new(),
                    }
                } else {
                    UserInputResponse::Cancel
                });
                if result.is_err() {
                    self.browser.error = Some("The sign-in request expired.".into());
                }
            }
            self.forms.remove(&key);
            self.transcript.interaction_changed(&key);
        }
        cx.notify();
    }

    pub(super) fn browser_close_tab(&mut self, id: HostTabId, cx: &mut Context<Self>) {
        if let Ok(tabs) = self
            .controller
            .browser
            .with(|session, _| Ok(session.tabs()))
            && let Some(tab) = tabs.iter().find(|tab| tab.id == id)
            && let BrowserProfile::Authentication { flow } = tab.profile
            && tabs
                .iter()
                .filter(|other| other.profile == tab.profile)
                .count()
                == 1
            && self.browser.authentication_flows.contains_key(&flow)
        {
            self.browser_finish_authentication(flow, false, cx);
            return;
        }
        self.browser.error = self
            .controller
            .browser
            .with(|session, _| session.close(id))
            .err()
            .map(|error| error.to_string());
        if self.browser.selected == Some(id) {
            self.browser.selected = None;
        }
        self.browser_save_manual_restore();
        self.browser_save_owned_restore();
        cx.notify();
    }

    fn browser_import_authentication_cookies(&mut self, id: u128, cx: &mut Context<Self>) {
        if self.browser.cookie_importing.is_some() {
            return;
        }
        let Some(flow) = self.browser.authentication_flows.get(&id) else {
            return;
        };
        if flow.initial || !self.browser_authentication_active(flow) {
            self.browser.error =
                Some("Open the private sign-in page before importing cookies.".into());
            cx.notify();
            return;
        }
        self.browser.cookie_importing = Some(id);
        let picker = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(
                "Import a Netscape/Mozilla cookies.txt file into this private sign-in flow".into(),
            ),
        });
        cx.spawn(async move |view, cx| {
            let selected = picker.await;
            let _ = view.update(cx, |this, cx| {
                if this.browser.cookie_importing != Some(id) {
                    return;
                }
                this.browser.cookie_importing = None;
                let Some(flow) = this.browser.authentication_flows.get(&id) else {
                    return;
                };
                if !this.browser_authentication_active(flow) || flow.initial {
                    this.browser.error = Some("The sign-in request changed before cookies could be imported.".into());
                    cx.notify();
                    return;
                }
                let path = match selected {
                    Ok(Ok(Some(paths))) if paths.len() == 1 && paths[0].is_file() => paths[0].clone(),
                    Ok(Ok(None)) => {
                        cx.notify();
                        return;
                    }
                    _ => {
                        this.browser.error = Some("Choose one readable Netscape/Mozilla cookies.txt file.".into());
                        cx.notify();
                        return;
                    }
                };
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|duration| duration.as_secs())
                    .unwrap_or(0);
                #[cfg_attr(not(target_os = "linux"), allow(unused_variables))]
                let jar = match browser_domain::cookie_import::read_netscape_cookie_jar(&path, now) {
                    Ok(jar) => jar,
                    Err(error) => {
                        this.browser.error = Some(error.to_string());
                        cx.notify();
                        return;
                    }
                };

                #[cfg(target_os = "linux")]
                let imported = this
                    .browser
                    .native
                    .borrow_mut()
                    .import_authentication_cookies(id, &jar);
                #[cfg(not(target_os = "linux"))]
                let imported: browser_domain::Result<usize> = Err(browser_domain::BrowserError::Unavailable);

                let count = match imported {
                    Ok(count) => count,
                    Err(error) => {
                        this.browser.error = Some(error.to_string());
                        cx.notify();
                        return;
                    }
                };

                let reload = this.controller.browser.with(|session, now| {
                    let tabs = session
                        .tabs()
                        .into_iter()
                        .filter(|tab| tab.profile == (BrowserProfile::Authentication { flow: id }))
                        .map(|tab| tab.id)
                        .collect::<Vec<_>>();
                    if tabs.is_empty() {
                        return Err(browser_domain::BrowserError::MissingTab);
                    }
                    for tab in tabs {
                        session.user_navigate(tab, "", NavigationKind::Reload, now)?;
                    }
                    Ok(())
                });
                match reload {
                    Ok(()) => {
                        this.browser.error = None;
                        this.notice = Some(format!(
                            "Imported {count} protected cookies into this private sign-in flow. The sign-in page is reloading."
                        ));
                    }
                    Err(error) => this.browser.error = Some(error.to_string()),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn browser_retry_authentication(&mut self, id: u128, cx: &mut Context<Self>) {
        let Some(flow) = self.browser.authentication_flows.get(&id) else {
            return;
        };
        let key = flow.key.clone();
        let url = flow.url.clone();
        self.browser_close_authentication_flow(id);
        self.browser_authentication_request(key, url, cx);
    }

    pub(super) fn browser_authentication_controls(&self, cx: &mut Context<Self>) -> AnyElement {
        div().flex().flex_col().gap_2().children(self.browser.authentication_flows.iter().map(|(id, flow)| {
            let id = *id;
            let importing = self.browser.cookie_importing == Some(id);
            let import_disabled = flow.initial || self.browser.cookie_importing.is_some();
            div().flex().flex_col().gap_1()
                .child(format!("Private sign-in {id}: {}", flow.label))
                .child(div().text_xs().child("Only this request shares these sign-in tabs. You can import a reviewed Netscape/Mozilla cookies.txt file into this temporary flow; imported cookies never enter Manual or agent browser profiles and are destroyed with the flow."))
                .child(div().flex().gap_2()
                    .child(ui::action(format!("browser-auth-import-{id}"), if importing { "Importing cookies..." } else { "Import cookies..." }, None, import_disabled,
                        cx.listener(move |this, _: &(), _, cx| this.browser_import_authentication_cookies(id, cx))))
                    .child(ui::action(format!("browser-auth-retry-{id}"), "Restart sign-in", None, false,
                        cx.listener(move |this, _: &(), _, cx| this.browser_retry_authentication(id, cx))))
                    .child(ui::action(format!("browser-auth-finish-{id}"), "I finished sign-in", None, false,
                        cx.listener(move |this, _: &(), _, cx| this.browser_finish_authentication(id, true, cx))))
                    .child(ui::action(format!("browser-auth-cancel-{id}"), "Cancel sign-in", None, false,
                        cx.listener(move |this, _: &(), _, cx| this.browser_finish_authentication(id, false, cx)))))
        })).into_any_element()
    }
}
