use super::*;
use synara_registry::{
    HttpsDownloader, InstallPlan, InstalledAgent, Platform, Registry, RegistryReference,
    RegistryStore,
};

pub(super) struct RegistryState {
    pub directory: PathBuf,
    pub query: Entity<TextEntry>,
    catalog: Option<Registry>,
    installed: Vec<InstalledAgent>,
    busy: bool,
    loaded: bool,
    review: Option<InstallPlan>,
    remove: Option<RegistryReference>,
    error: Option<String>,
}
pub(super) enum RegistryReply {
    Loaded(Option<Registry>, Vec<InstalledAgent>),
    Changed(Vec<InstalledAgent>, Vec<AgentProfile>, String),
    Failed(String),
}
impl RegistryState {
    pub fn new(directory: PathBuf, cx: &mut Context<Shell>) -> Self {
        Self {
            directory,
            query: cx.new(|cx| {
                TextEntry::new(
                    "Search agents by name or description",
                    EntryMode::SingleLine,
                    36.,
                    cx,
                )
            }),
            catalog: None,
            installed: vec![],
            busy: false,
            loaded: false,
            review: None,
            remove: None,
            error: None,
        }
    }
}
impl Shell {
    fn registry_job(
        &mut self,
        job: impl std::future::Future<Output = Result<RegistryReply, String>> + Send + 'static,
        cx: &mut Context<Self>,
    ) {
        if self.registry.busy {
            return;
        }
        self.registry.busy = true;
        self.registry.error = None;
        let sender = self.sender.clone();
        self.runtime.spawn(async move {
            let reply = job.await.unwrap_or_else(RegistryReply::Failed);
            let _ = sender.send(Update::Registry(Box::new(reply))).await;
        });
        cx.notify();
    }
    pub(super) fn load_registry_if_needed(&mut self, cx: &mut Context<Self>) {
        if !self.registry.loaded {
            self.load_registry(false, cx);
        }
    }
    fn load_registry(&mut self, network: bool, cx: &mut Context<Self>) {
        let root = self.registry.directory.clone();
        self.registry_job(
            async move {
                tokio::task::spawn_blocking(move || {
                    let store = RegistryStore::open(root)?;
                    let catalog = if network {
                        Some(store.refresh(&HttpsDownloader::default())?)
                    } else {
                        store.cached()?
                    };
                    Ok::<_, synara_registry::RegistryError>(RegistryReply::Loaded(
                        catalog,
                        store.installed()?,
                    ))
                })
                .await
                .map_err(|_| "Registry worker stopped".to_string())?
                .map_err(|e| e.to_string())
            },
            cx,
        );
    }
    pub(super) fn registry_reply(&mut self, reply: RegistryReply, cx: &mut Context<Self>) {
        self.registry.busy = false;
        self.registry.loaded = true;
        match reply {
            RegistryReply::Loaded(catalog, installed) => {
                self.registry.catalog = catalog;
                self.registry.installed = installed;
            }
            RegistryReply::Changed(installed, profiles, message) => {
                self.registry.installed = installed;
                self.profiles = profiles;
                self.profile_editor.update(cx, |entry, cx| {
                    entry.set_text(
                        serde_json::to_string_pretty(&self.profiles).unwrap_or_default(),
                        cx,
                    )
                });
                self.notice = Some(message);
                self.registry.review = None;
                self.registry.remove = None;
            }
            RegistryReply::Failed(message) => self.registry.error = Some(message),
        }
        cx.notify();
    }
    fn install_reviewed_agent(&mut self, cx: &mut Context<Self>) {
        if self.registry.busy {
            return;
        }
        if !self.busy.is_empty() || !self.connecting.is_empty() {
            self.registry.error = Some(
                "Finish active prompts and connection operations before changing installed agents."
                    .into(),
            );
            cx.notify();
            return;
        }
        let Some(plan) = self.registry.review.clone() else {
            return;
        };
        let root = self.registry.directory.clone();
        let workspace = self.controller.workspace.clone();
        self.registry_job(async move {
            let installed=tokio::task::spawn_blocking(move ||RegistryStore::open(&root)?.install(&plan,&HttpsDownloader::default()))
                .await.map_err(|_|"Installer worker stopped".to_string())?.map_err(|e|e.to_string())?;
            let root=installed.reference.directory.parent().ok_or("Invalid installation directory")?.to_owned();
            let profiles=workspace.register_installation(installed.reference).await
                .map_err(|e|format!("Installation saved, but the agent could not be enabled: {e}. Reload installed agents to retry Enable."))?;
            let list=tokio::task::spawn_blocking(move ||RegistryStore::open(root)?.installed()).await.map_err(|_|"Registry worker stopped")?.map_err(|e|e.to_string())?;
            Ok(RegistryReply::Changed(list,profiles,"Agent added. Select it for a task to connect. No agent was started by installation.".into()))
        },cx);
    }
    fn enable_registry_agent(&mut self, reference: RegistryReference, cx: &mut Context<Self>) {
        if !self.busy.is_empty() || !self.connecting.is_empty() {
            self.registry.error =
                Some("Finish active agent operations before enabling another version.".into());
            cx.notify();
            return;
        }
        let root = self.registry.directory.clone();
        let workspace = self.controller.workspace.clone();
        self.registry_job(
            async move {
                let profiles = workspace
                    .register_installation(reference)
                    .await
                    .map_err(|e| e.to_string())?;
                let list =
                    tokio::task::spawn_blocking(move || RegistryStore::open(root)?.installed())
                        .await
                        .map_err(|_| "Registry worker stopped")?
                        .map_err(|e| e.to_string())?;
                Ok(RegistryReply::Changed(
                    list,
                    profiles,
                    "Installed agent enabled. It will start only when you connect a task.".into(),
                ))
            },
            cx,
        );
    }
    fn remove_reviewed_agent(&mut self, cx: &mut Context<Self>) {
        if !self.busy.is_empty() || !self.connecting.is_empty() {
            self.registry.error =
                Some("Finish active agent operations before removing an installation.".into());
            cx.notify();
            return;
        }
        let Some(reference) = self.registry.remove.clone() else {
            return;
        };
        let root = self.registry.directory.clone();
        let workspace = self.controller.workspace.clone();
        self.registry_job(async move {
            let profiles=workspace.unregister_installation(reference.clone()).await.map_err(|e|e.to_string())?;
            let list=tokio::task::spawn_blocking(move ||{
                let store=RegistryStore::open(root)?;store.remove(&reference)?;store.installed()
            }).await.map_err(|_|"Registry worker stopped")?.map_err(|e|format!("Profile unregistered, but removal failed: {e}"))?;
            Ok(RegistryReply::Changed(list,profiles,"Installation removed. Vendor authentication and npm/uv caches were not deleted.".into()))
        },cx);
    }
    pub(super) fn registry_panel(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let query = self.registry.query.read(cx).text().trim().to_lowercase();
        let mut body=div().id("registry-content").flex().flex_col().flex_1().min_h_0().overflow_y_scroll().gap_3().p_4()
            .child(div().flex().justify_between().items_center().child(div().text_lg().child("ACP Agent Registry"))
                .child(div().flex().gap_2()
                    .child(button("registry-reload","Reload installed",false).on_click(cx.listener(|this,_,_,cx|this.load_registry(false,cx))))
                    .child(button("registry-refresh",if self.registry.busy{"Working..."}else{"Refresh catalog"},false).on_click(cx.listener(|this,_,_,cx|this.load_registry(true,cx))))))
            .child(div().text_sm().text_color(rgb(0xa7b5c7)).child("Official ACP registry · external executables · explicit approval required. Package launchers require npm or uv and download their pinned package on first connection."))
            .child(self.registry.query.clone());
        if let Some(error) = &self.registry.error {
            body = body.child(
                div()
                    .p_3()
                    .rounded_md()
                    .bg(rgb(0x41272d))
                    .child(error.clone()),
            );
        }
        if let Some(plan) = &self.registry.review {
            let license = plan.license_url().map(str::to_owned);
            let mut review=div().p_4().rounded_md().border_1().border_color(rgb(0x628db9)).flex().flex_col().gap_2()
                .child(div().text_lg().child(format!("Review {} {}",plan.name(),plan.version())))
                .child(div().text_sm().child(format!("Target: {}",plan.platform().key())))
                .child(div().text_sm().child(format!("Origin: {}",plan.origin())))
                .child(div().text_sm().child(format!("Arguments: {:?}",plan.args())))
                .child(div().text_sm().child(format!("Public environment defaults: {:?}",plan.environment())))
                .child(div().text_sm().child(plan.checksum().map_or_else(||"Package-manager integrity, not a Synara-verified binary archive. The package and its dependencies can execute code when first connected.".into(),|hash|format!("Required SHA-256: {hash}"))))
                .child(div().text_sm().child("An agent runs with your account's permissions. Workspace callback containment is not an operating-system sandbox. Approve only publishers you trust."));
            if let Some(license) = license {
                review = review.child(
                    button("registry-license", "Read license / terms", false)
                        .on_click(move |_, _, cx| cx.open_url(&license)),
                );
            }
            review = review.child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        button(
                            "registry-confirm",
                            if plan.package_managed() {
                                "Approve pinned launcher"
                            } else {
                                "Approve download and install"
                            },
                            true,
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.install_reviewed_agent(cx))),
                    )
                    .child(
                        button("registry-cancel", "Cancel", false).on_click(cx.listener(
                            |this, _, _, cx| {
                                if !this.registry.busy {
                                    this.registry.review = None;
                                    cx.notify();
                                }
                            },
                        )),
                    ),
            );
            body = body.child(review);
        }
        if self.registry.remove.is_some() {
            body=body.child(div().p_3().rounded_md().bg(rgb(0x41272d)).flex().flex_col().gap_2()
                .child("Remove this installation? Assigned tasks must first be switched to a different agent. Saved conversations are not deleted.")
                .child(div().flex().gap_2()
                    .child(button("registry-remove-confirm","Remove installation",false).on_click(cx.listener(|this,_,_,cx|this.remove_reviewed_agent(cx))))
                    .child(button("registry-remove-cancel","Keep",false).on_click(cx.listener(|this,_,_,cx|{if !this.registry.busy{this.registry.remove=None;cx.notify();}})))));
        }
        if !self.registry.installed.is_empty() {
            body = body.child(
                div()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Installed / approved launchers"),
            );
            for (index, agent) in self.registry.installed.iter().enumerate() {
                let enabled = self
                    .profiles
                    .iter()
                    .any(|p| p.registry.as_ref() == Some(&agent.reference));
                let enable = agent.reference.clone();
                let remove = enable.clone();
                body =
                    body.child(
                        div()
                            .p_3()
                            .bg(rgb(0x1b2532))
                            .rounded_md()
                            .flex()
                            .justify_between()
                            .gap_3()
                            .child(div().flex_1().min_w_0().child(format!(
                                "{} {} · {}",
                                agent.plan.name(),
                                agent.plan.version(),
                                if enabled {
                                    "Enabled"
                                } else {
                                    "Not selected as current version"
                                }
                            )))
                            .child(
                                div()
                                    .flex()
                                    .gap_2()
                                    .children((!enabled).then(|| {
                                        button(("registry-enable", index), "Enable", false)
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.enable_registry_agent(enable.clone(), cx)
                                            }))
                                    }))
                                    .child(
                                        button(("registry-remove", index), "Remove", false)
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                if !this.registry.busy {
                                                    this.registry.remove = Some(remove.clone());
                                                    this.registry.review = None;
                                                    cx.notify();
                                                }
                                            })),
                                    ),
                            ),
                    );
            }
        }
        if let Some(catalog) = &self.registry.catalog {
            body = body.child(div().font_weight(gpui::FontWeight::SEMIBOLD).child(format!(
                "Catalog · {} {}",
                catalog.agents.len(),
                if catalog.agents.len() == 1 {
                    "agent"
                } else {
                    "agents"
                }
            )));
            for (index, entry) in catalog.agents.iter().enumerate().filter(|(_, a)| {
                query.is_empty()
                    || format!("{} {} {}", a.name, a.id, a.description)
                        .to_lowercase()
                        .contains(&query)
            }) {
                let plan = Platform::current().and_then(|platform| entry.plan(platform));
                let installed = self
                    .registry
                    .installed
                    .iter()
                    .any(|a| a.plan.id() == entry.id && a.plan.version() == entry.version);
                let update = self.registry.installed.iter().any(|a| {
                    a.plan.id() == entry.id
                        && entry
                            .version_key()
                            .ok()
                            .zip(a.plan.version_key().ok())
                            .is_some_and(|(new, old)| new > old)
                });
                let mut row = div()
                    .p_3()
                    .rounded_md()
                    .bg(rgb(0x1b2532))
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(div().font_weight(gpui::FontWeight::SEMIBOLD).child(format!(
                        "{} · {}{}",
                        entry.name,
                        entry.version,
                        if update { " · Update available" } else { "" }
                    )))
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0xa7b5c7))
                            .child(entry.description.clone()),
                    );
                match plan {
                    Ok(plan) if !installed => {
                        let label = if plan.package_managed() {
                            "Review launcher"
                        } else {
                            "Review download"
                        };
                        row = row.child(button(("registry-review", index), label, false).on_click(
                            cx.listener(move |this, _, _, cx| {
                                if !this.registry.busy {
                                    this.registry.review = Some(plan.clone());
                                    this.registry.remove = None;
                                    this.registry.error = None;
                                    cx.notify();
                                }
                            }),
                        ));
                    }
                    Ok(_) => row = row.child(div().text_sm().child("This version is installed")),
                    Err(error) => {
                        row = row.child(
                            div()
                                .text_sm()
                                .text_color(rgb(0xc9b7a2))
                                .child(error.to_string()),
                        )
                    }
                }
                body = body.child(row);
            }
        } else {
            body=body.child(div().p_4().child("No cached catalog. Refresh catalog to retrieve the official index. Browsing does not install or start agents."));
        }
        body.into_any_element()
    }
}
