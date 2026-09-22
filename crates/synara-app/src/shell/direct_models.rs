//! Direct HTTP models share the existing composer and transcript, not ACP sessions.
use super::*;
use crate::ui::{self, palette};
use synara_model::{
    HttpModelProvider, ModelInfo, OutputFormat, ProviderCatalog, custom_profile_example,
};
mod view;

pub(super) enum Reply {
    Loaded(ProviderSettings),
    Saved(ProviderSettings),
    Binding(TaskId, u64, Result<Option<DirectModelBinding>, String>),
    Selected(TaskId, Option<DirectModelBinding>),
    Catalog(ProviderCatalog),
    Discovered(String, u64, Vec<ModelInfo>),
    KeyChanged,
    Failed(String),
}
#[derive(Clone)]
enum Review {
    Route {
        task: TaskId,
        title: String,
        selection: Option<ModelSelection>,
        revision: u64,
        sequence: u64,
        endpoint: String,
    },
    Key {
        provider: String,
        revision: u64,
        endpoint: String,
        delete: bool,
    },
}
pub(super) struct DirectModelState {
    value: Option<ProviderSettings>,
    bindings: HashMap<TaskId, Option<DirectModelBinding>>,
    generations: HashMap<TaskId, u64>,
    pub busy: bool,
    mutating_task: Option<TaskId>,
    error: Option<String>,
    notice: Option<String>,
    query: Entity<TextEntry>,
    editor: Entity<TextEntry>,
    options: Entity<TextEntry>,
    editing: bool,
    catalog: Option<ProviderCatalog>,
    expanded: Option<String>,
    review: Option<Review>,
    _subscription: Subscription,
}
impl DirectModelState {
    pub fn new(cx: &mut Context<Shell>) -> Self {
        let query =
            cx.new(|cx| TextEntry::new("Search provider or model", EntryMode::SingleLine, 32., cx));
        let editor = cx.new(|cx| {
            TextEntry::new(
                "Provider settings JSON, never API keys",
                EntryMode::Editor,
                320.,
                cx,
            )
        });
        let options =
            cx.new(|cx| TextEntry::new("Reviewed model options", EntryMode::Editor, 180., cx));
        let subscription = cx.subscribe(&query, |_, _, _, cx| cx.notify());
        Self {
            value: None,
            bindings: HashMap::new(),
            generations: HashMap::new(),
            busy: false,
            mutating_task: None,
            error: None,
            notice: None,
            query,
            editor,
            options,
            editing: false,
            catalog: None,
            expanded: None,
            review: None,
            _subscription: subscription,
        }
    }
    pub fn pending(&self) -> bool {
        self.busy || self.editing || self.review.is_some()
    }
}
impl Shell {
    pub(super) fn direct_route_loading(&self) -> bool {
        self.selected
            .is_some_and(|task| !self.direct_models.bindings.contains_key(&task))
    }
    pub(super) fn uses_direct_model(&self) -> bool {
        self.selected
            .and_then(|task| self.direct_models.bindings.get(&task))
            .is_some_and(Option::is_some)
    }
    pub(super) fn load_direct_binding(&mut self, task: TaskId) {
        self.direct_models.review = None;
        let generation = self.direct_models.generations.entry(task).or_default();
        *generation = generation.wrapping_add(1);
        let generation = *generation;
        self.direct_models.bindings.remove(&task);
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::DirectModels(Box::new(Reply::Binding(
                task,
                generation,
                workspace
                    .direct_model_binding(task)
                    .await
                    .map_err(|e| e.to_string()),
            ))))
        });
    }
    fn direct_model_job(
        &mut self,
        work: impl std::future::Future<Output = Result<Reply, String>> + Send + 'static,
        cx: &mut Context<Self>,
    ) {
        if self.direct_models.busy || self.close != CloseState::Open {
            return;
        }
        self.direct_models.busy = true;
        self.direct_models.error = None;
        self.direct_models.notice = None;
        let sender = self.sender.clone();
        self.runtime.spawn(async move {
            let reply = work.await.unwrap_or_else(Reply::Failed);
            let _ = sender.send(Update::DirectModels(Box::new(reply))).await;
        });
        cx.notify();
    }
    pub(super) fn load_direct_models(&mut self, cx: &mut Context<Self>) {
        if self.direct_models.busy
            || self.direct_models.editing
            || self.direct_models.review.is_some()
        {
            return;
        }
        let workspace = self.controller.workspace.clone();
        self.direct_model_job(
            async move {
                workspace
                    .direct_model_settings()
                    .await
                    .map(Reply::Loaded)
                    .map_err(|e| e.to_string())
            },
            cx,
        );
    }
    fn edit_direct_models(&mut self, example: bool, cx: &mut Context<Self>) {
        if self.direct_models.busy
            || self.direct_models.editing
            || self.direct_models.review.is_some()
        {
            return;
        }
        let Some(mut value) = self.direct_models.value.clone() else {
            return;
        };
        if example && !value.providers.iter().any(|p| p.id == "local-compatible") {
            value.providers.push(custom_profile_example());
        }
        self.direct_models.editor.update(cx, |entry, cx| {
            entry.set_text(serde_json::to_string_pretty(&value).unwrap_or_default(), cx)
        });
        self.direct_models.editing = true;
        cx.notify();
    }
    fn edit_google_provider(&mut self, cx: &mut Context<Self>) {
        if self.direct_models.busy || self.direct_models.editing || self.direct_models.review.is_some() { return; }
        let Some(mut value) = self.direct_models.value.clone() else { return; };
        if !value.providers.iter().any(|p| p.id == "google-direct") {
            value.providers.push(synara_model::google_profile_example());
        }
        self.direct_models.editor.update(cx, |entry, cx| {
            entry.set_text(serde_json::to_string_pretty(&value).unwrap_or_default(), cx)
        });
        self.direct_models.editing = true;
        cx.notify();
    }
    fn save_direct_models(&mut self, cx: &mut Context<Self>) {
        if self.direct_models.busy || !self.direct_models.editing {
            return;
        }
        let text = self.direct_models.editor.read(cx).text();
        let parsed = serde_json::from_str::<ProviderSettings>(text);
        let value = match parsed {
            Ok(value) if text.len() <= synara_model::MAX_REQUEST_BYTES => value,
            _ => {
                self.direct_models.error=Some("Invalid provider JSON. Use the displayed schema and never put API keys in this editor.".into());
                cx.notify();
                return;
            }
        };
        if let Err(error) = value.validate() {
            self.direct_models.error = Some(error.to_string());
            cx.notify();
            return;
        }
        let controller = self.controller.clone();
        self.direct_model_job(
            async move {
                controller
                    .save_direct_model_settings(value)
                    .await
                    .map(Reply::Saved)
                    .map_err(|e| e.to_string())
            },
            cx,
        );
    }
    fn load_direct_catalog(&mut self, cx: &mut Context<Self>) {
        if self.direct_models.editing || self.direct_models.review.is_some() {
            return;
        }
        self.direct_model_job(
            async move {
                HttpModelProvider::new()
                    .map_err(|e| e.to_string())?
                    .catalog(Default::default())
                    .await
                    .map(Reply::Catalog)
                    .map_err(|e| e.to_string())
            },
            cx,
        );
    }
    fn review_catalog_provider(&mut self, id: String, cx: &mut Context<Self>) {
        if self.direct_models.busy
            || self.direct_models.editing
            || self.direct_models.review.is_some()
        {
            return;
        }
        let Some(profile) = self
            .direct_models
            .catalog
            .as_ref()
            .and_then(|c| c.providers.iter().find(|p| p.id == id))
            .and_then(|p| p.profile.clone())
        else {
            return;
        };
        let Some(mut settings) = self.direct_models.value.clone() else {
            return;
        };
        if settings.providers.iter().any(|p| p.id == id) {
            self.direct_models.error=Some("This profile already exists. Use Configure providers to review an update without replacing it implicitly.".into());
            cx.notify();
            return;
        }
        settings.providers.push(profile);
        match serde_json::to_string_pretty(&settings) {
            Ok(text) if text.len() <= synara_model::MAX_REQUEST_BYTES => {
                self.direct_models.editor.update(cx,|entry,cx|entry.set_text(text,cx));
                self.direct_models.editing=true;
                self.direct_models.notice=Some("Review the catalog endpoint, authentication and models before saving. Metadata is not verified provider interoperability.".into());
            }
            _ => self.direct_models.error=Some("This catalog entry is too large for one reviewed configuration. Configure a smaller model list manually.".into()),
        }
        cx.notify();
    }
    fn discover_direct_models(&mut self, id: String, cx: &mut Context<Self>) {
        if self.direct_models.editing || self.direct_models.review.is_some() {
            return;
        }
        let Some(value) = self.direct_models.value.as_ref() else {
            return;
        };
        let revision = value.revision;
        let controller = self.controller.clone();
        self.direct_model_job(
            async move {
                controller
                    .discover_direct_models(id.clone(), revision)
                    .await
                    .map(|models| Reply::Discovered(id, revision, models))
                    .map_err(|e| e.to_string())
            },
            cx,
        );
    }
    fn review_direct_route(&mut self, selection: Option<ModelSelection>, cx: &mut Context<Self>) {
        if self.direct_models.busy || self.direct_models.editing || self.controls_blocked() {
            return;
        }
        let (Some(task), Some(thread), Some(value)) = (
            self.task(),
            self.thread.as_ref(),
            self.direct_models.value.as_ref(),
        ) else {
            return;
        };
        let endpoint = selection
            .as_ref()
            .and_then(|s| value.providers.iter().find(|p| p.id == s.provider_id))
            .map(|p| p.endpoint.clone())
            .unwrap_or_else(|| "New ACP coding-agent session".into());
        let review = Review::Route {
            task: task.id,
            title: task.title.clone(),
            selection: selection.clone(),
            revision: value.revision,
            sequence: thread.last_sequence,
            endpoint,
        };
        self.direct_models.options.update(cx, |entry, cx| {
            entry.set_text(
                serde_json::to_string_pretty(&selection).unwrap_or_default(),
                cx,
            )
        });
        self.direct_models.review = Some(review);
        cx.notify();
    }
    fn review_direct_key(&mut self, provider: String, delete: bool, cx: &mut Context<Self>) {
        if self.direct_models.busy || self.direct_models.editing {
            return;
        }
        let Some(value) = &self.direct_models.value else {
            return;
        };
        let Some(profile) = value
            .providers
            .iter()
            .find(|p| p.id == provider && p.requires_key)
        else {
            return;
        };
        self.direct_models.review = Some(Review::Key {
            provider,
            revision: value.revision,
            endpoint: profile.endpoint.clone(),
            delete,
        });
        cx.notify();
    }
    fn confirm_direct_review(&mut self, cx: &mut Context<Self>) {
        if self.direct_models.busy {
            return;
        }
        let Some(review) = self.direct_models.review.clone() else {
            return;
        };
        let controller = self.controller.clone();
        match review {
            Review::Route {
                task,
                selection,
                revision,
                sequence,
                ..
            } => {
                if self.selected != Some(task) {
                    self.direct_models.review = None;
                    cx.notify();
                    return;
                }
                let reviewed: Option<ModelSelection> =
                    match serde_json::from_str(self.direct_models.options.read(cx).text()) {
                        Ok(value) => value,
                        Err(_) => {
                            self.direct_models.error = Some("Invalid model options JSON.".into());
                            cx.notify();
                            return;
                        }
                    };
                if reviewed.as_ref().map(|s| (&s.provider_id, &s.model_id))
                    != selection.as_ref().map(|s| (&s.provider_id, &s.model_id))
                {
                    self.direct_models.error=Some("The provider/model identity changed. Cancel and review that model from its row.".into());
                    cx.notify();
                    return;
                }
                // Invalidate older binding reads before this mutation can complete.
                let generation = self.direct_models.generations.entry(task).or_default();
                *generation = generation.wrapping_add(1);
                self.direct_models.bindings.remove(&task);
                self.direct_models.mutating_task = Some(task);
                self.direct_model_job(
                    async move {
                        controller
                            .select_direct_model(task, reviewed, revision, sequence)
                            .await
                            .map(|binding| Reply::Selected(task, binding))
                            .map_err(|e| e.to_string())
                    },
                    cx,
                );
            }
            Review::Key {
                provider,
                revision,
                delete,
                ..
            } => {
                let value = if delete {
                    None
                } else {
                    let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
                        self.direct_models.error =
                            Some("The clipboard has no text API key. Nothing was stored.".into());
                        cx.notify();
                        return;
                    };
                    if text.len() > 8192
                        || text.is_empty()
                        || !text.bytes().all(|b| (33..=126).contains(&b))
                    {
                        self.direct_models.error=Some("The clipboard key must be printable ASCII, without whitespace, at most 8 KiB.".into());
                        cx.notify();
                        return;
                    }
                    match synara_runtime::SecretValue::new(text.into_bytes()) {
                        Ok(value) => Some(value),
                        Err(_) => {
                            self.direct_models.error =
                                Some("The key could not be accepted.".into());
                            cx.notify();
                            return;
                        }
                    }
                };
                self.direct_model_job(
                    async move {
                        controller
                            .direct_model_key(provider, revision, value)
                            .await
                            .map(|_| Reply::KeyChanged)
                            .map_err(|e| e.to_string())
                    },
                    cx,
                );
            }
        }
    }
    pub(super) fn direct_model_reply(&mut self, reply: Reply, cx: &mut Context<Self>) {
        if let Reply::Binding(task, generation, result) = reply {
            if self.direct_models.generations.get(&task) != Some(&generation) {
                return;
            }
            match result {
                Ok(binding) => {
                    self.direct_models.bindings.insert(task, binding);
                }
                Err(error) => {
                    if self.selected == Some(task) {
                        self.error = Some(error);
                    }
                }
            }
            cx.notify();
            return;
        }
        self.direct_models.busy = false;
        match reply {
            Reply::Loaded(value) => self.direct_models.value = Some(value),
            Reply::Saved(value) => {
                self.direct_models.value = Some(value);
                self.direct_models.editing = false;
                self.direct_models.notice=Some("Provider metadata saved. Changed profiles require model re-selection. No request was sent.".into());
            }
            Reply::Selected(task, binding) => {
                self.direct_models.mutating_task = None;
                self.direct_models.bindings.insert(task, binding);
                self.direct_models.review = None;
                if self.selected == Some(task) {
                    self.details = None;
                    self.controls.retire();
                }
                self.direct_models.notice=Some("Route selected. Nothing was sent or executed. Return to the conversation and explicitly send when ready.".into());
            }
            Reply::Catalog(catalog) => self.direct_models.catalog = Some(catalog),
            Reply::Discovered(id, revision, models) => {
                if let Some(mut value) = self
                    .direct_models
                    .value
                    .clone()
                    .filter(|v| v.revision == revision)
                {
                    if let Some(profile) = value.providers.iter_mut().find(|p| p.id == id) {
                        for mut model in models {
                            if let Some(existing) = profile.models.iter().find(|m| m.id == model.id)
                            {
                                model.capabilities = existing.capabilities.clone();
                            }
                            if !profile.models.iter().any(|m| m.id == model.id) {
                                profile.models.push(model);
                            }
                        }
                    }
                    if value.validate().is_ok() {
                        self.direct_models.editor.update(cx, |entry, cx| {
                            entry.set_text(
                                serde_json::to_string_pretty(&value).unwrap_or_default(),
                                cx,
                            )
                        });
                        self.direct_models.editing = true;
                        self.direct_models.notice=Some("Review discovered identities before saving. Only a bounded first page was read. Unreported capabilities remain unknown.".into());
                    } else {
                        self.direct_models.error = Some(
                            "Discovery exceeded profile limits. Existing settings were retained."
                                .into(),
                        );
                    }
                }
            }
            Reply::KeyChanged => {
                self.direct_models.review = None;
                self.direct_models.notice=Some("OS credential operation completed. No key was copied into settings or transcript. Clipboard contents were left unchanged.".into());
            }
            Reply::Failed(error) => {
                self.direct_models.error = Some(error);
                if let Some(task) = self.direct_models.mutating_task.take() {
                    // Failed route changes leave durable ownership authoritative.
                    // Never leave the composer using a stale or unknown route.
                    self.load_direct_binding(task);
                }
            }
            Reply::Binding(..) => unreachable!(),
        }
        cx.notify();
    }
    pub(super) fn direct_model_controls(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let label = if self.direct_route_loading() {
            "Loading conversation route...".into()
        } else if let Some(binding) = self
            .selected
            .and_then(|id| self.direct_models.bindings.get(&id))
            .and_then(Option::as_ref)
        {
            format!(
                "Direct · {} / {}",
                binding.selection.provider_id, binding.selection.model_id
            )
        } else {
            "Direct models".into()
        };
        ui::action(
            "direct-model-controls",
            label,
            None,
            false,
            cx.listener(|this, _, _, cx| {
                this.set_panel(Panel::Settings, cx);
                this.open_settings_section(settings::Section::DirectModels, cx);
            }),
        )
        .relative()
        .child(ui::layout_probe("direct-model-controls"))
        .into_any_element()
    }
}
