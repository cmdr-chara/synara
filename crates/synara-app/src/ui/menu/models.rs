//! Provider tabs and stars decorate immutable controller action indices.
use super::*;
use synara_workspace::{ModelFavorite, SessionModelPreset, WorkspaceService};
use tokio::runtime::Handle;
pub struct ModelSource {
    pub name: String,
    pub icon: Glyph,
}
pub struct ModelRow {
    pub source: usize,
    pub favorite: Option<ModelFavorite>,
    pub preset: Option<SessionModelPreset>,
}
pub(super) struct ModelState {
    sources: Vec<ModelSource>,
    rows: Vec<ModelRow>,
    tab: Option<usize>,
    favorites: Vec<ModelFavorite>,
    current_agent: String,
    current_source: usize,
    presets: Vec<SessionModelPreset>,
    ready: bool,
    saving: bool,
    error: Option<String>,
    workspace: WorkspaceService,
    runtime: Handle,
}
impl ModelState {
    pub(super) fn preset(&self, index: usize) -> Option<SessionModelPreset> {
        self.rows.get(index)?.preset.clone()
    }
    pub(super) fn includes(&self, index: usize) -> bool {
        self.rows.get(index).is_some_and(|row| match self.tab {
            Some(source) => row.source == source && row.preset.is_none(),
            None => {
                row.preset
                    .as_ref()
                    .is_some_and(|preset| self.presets.contains(preset))
                    || row
                        .favorite
                        .as_ref()
                        .is_some_and(|favorite| self.favorites.contains(favorite))
            }
        })
    }
}
impl ChoiceMenu {
    pub fn with_models(
        mut self,
        sources: Vec<ModelSource>,
        rows: Vec<ModelRow>,
        selection: (usize, String),
        workspace: WorkspaceService,
        runtime: Handle,
        cx: &mut Context<Self>,
    ) -> Self {
        let (current, current_agent) = selection;
        debug_assert_eq!(rows.len(), self.choices.len());
        self.search = cx.new(|cx| {
            TextEntry::new("Search models...", EntryMode::SingleLine, 32., cx)
                .with_leading_icon(Glyph::Search)
                .picker_chrome()
        });
        self._search_events = cx.subscribe(&self.search, |this, entry, event, cx| {
            if matches!(event, EntryEvent::Changed) {
                this.filter(entry.read(cx).text().to_owned(), cx);
            }
        });
        self.searchable = true;
        self.models = Some(ModelState {
            sources,
            rows,
            tab: Some(current),
            favorites: vec![],
            current_agent,
            current_source: current,
            presets: vec![],
            ready: false,
            saving: false,
            error: None,
            workspace: workspace.clone(),
            runtime: runtime.clone(),
        });
        self.filter(String::new(), cx);
        let job = runtime.spawn(async move {
            tokio::join!(
                workspace.model_favorites(),
                workspace.session_model_presets()
            )
        });
        cx.spawn(async move |view, cx| {
            let (favorites, presets) = match job.await {
                Ok(results) => results,
                Err(_) => {
                    let _ = view.update(cx, |this, cx| {
                        if let Some(state) = &mut this.models {
                            state.error = Some(
                                "Could not load saved models. Reopen the picker to retry.".into(),
                            );
                        }
                        this.filter(this.query.clone(), cx);
                    });
                    return;
                }
            };
            let _ = view.update(cx, |this, cx| {
                let Ok(favorites) = favorites else {
                    if let Some(state) = &mut this.models {
                        state.error =
                            Some("Could not load saved models. Reopen the picker to retry.".into());
                    }
                    this.filter(this.query.clone(), cx);
                    return;
                };
                let Ok(presets) = presets else {
                    if let Some(state) = &mut this.models {
                        state.error =
                            Some("Could not load saved models. Reopen the picker to retry.".into());
                    }
                    this.filter(this.query.clone(), cx);
                    return;
                };
                let mut rows_to_append = Vec::new();
                if let Some(state) = &mut this.models {
                    state.favorites = favorites;
                    let current_presets: Vec<_> = presets
                        .into_iter()
                        .filter(|preset| preset.agent == state.current_agent)
                        .collect();
                    state.presets = current_presets.clone();
                    rows_to_append = current_presets
                        .into_iter()
                        .map(|preset| (state.current_source, preset))
                        .collect();
                    state.ready = true;
                }
                for (source, preset) in rows_to_append {
                    let label = preset.model.clone();
                    let detail = preset.effort.as_ref().map_or_else(
                        || "Saved model preset".to_owned(),
                        |effort| format!("Saved preset · effort {effort}"),
                    );
                    this.choices.push(Choice {
                        label,
                        detail,
                        selected: false,
                        ..Default::default()
                    });
                    if let Some(state) = &mut this.models {
                        state.rows.push(ModelRow {
                            source,
                            favorite: None,
                            preset: Some(preset),
                        });
                    }
                }
                this.filter(this.query.clone(), cx);
            });
        })
        .detach();
        self
    }
    pub(super) fn model_title(&self) -> String {
        self.models.as_ref().map_or_else(
            || self.title.clone(),
            |state| {
                state
                    .tab
                    .and_then(|i| state.sources.get(i))
                    .map_or_else(|| "Starred models".into(), |s| s.name.clone())
            },
        )
    }
    fn choose_source(&mut self, tab: Option<usize>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(state) = &mut self.models else {
            return;
        };
        if tab.is_some_and(|i| i >= state.sources.len()) {
            return;
        }
        state.tab = tab;
        self.search
            .update(cx, |e, cx| e.set_text(String::new(), cx));
        self.filter(String::new(), cx);
        window.focus(&self.search.read(cx).focus_handle(cx), cx);
    }
    pub(super) fn model_source_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(state) = &self.models else {
            return false;
        };
        let m = event.keystroke.modifiers;
        if !m.alt
            || m.control
            || m.platform
            || !matches!(event.keystroke.key.as_str(), "left" | "right")
        {
            return false;
        }
        let count = state.sources.len() + 1;
        let index = state.tab.map_or(0, |i| i + 1);
        let next = if event.keystroke.key == "left" {
            (index + count - 1) % count
        } else {
            (index + 1) % count
        };
        self.choose_source(next.checked_sub(1), window, cx);
        cx.stop_propagation();
        true
    }
    pub(super) fn model_tabs(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(state) = &self.models else {
            return div().into_any_element();
        };
        let entries = std::iter::once((None, "Starred".to_owned(), Glyph::StarFilled)).chain(
            state
                .sources
                .iter()
                .enumerate()
                .map(|(i, s)| (Some(i), s.name.clone(), s.icon)),
        );
        div()
            .id("model-sources")
            .role(gpui::Role::TabList)
            .aria_label("Model sources")
            .flex()
            .gap(px(2.))
            .p(px(6.))
            .border_b_1()
            .border_color(rgb(palette().border))
            .overflow_x_scroll()
            .children(entries.enumerate().map(|(slot, (tab, label, glyph))| {
                let active = state.tab == tab;
                let tooltip = label.clone();
                button_shell(
                    SharedString::from(format!("source-{slot}")),
                    label.clone(),
                    active,
                )
                .role(gpui::Role::Tab)
                .aria_label(label)
                .aria_selected(active)
                .size(px(28.))
                .p_0()
                .relative()
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .bg(rgba(0))
                .child(icon(glyph).size(px(16.)))
                .child(layout_probe_slot("model-source", slot))
                .children(active.then(|| {
                    div()
                        .absolute()
                        .bottom(px(-3.))
                        .left(px(6.))
                        .right(px(6.))
                        .h(px(2.))
                        .rounded_full()
                        .bg(rgb(palette().focus))
                }))
                .tooltip(move |_, cx| cx.new(|_| Tooltip(tooltip.clone().into())).into())
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.choose_source(tab, window, cx);
                    cx.stop_propagation();
                }))
            }))
            .into_any_element()
    }
    pub(super) fn model_favorite_button(
        &self,
        index: usize,
        position: usize,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        let state = self.models.as_ref()?;
        let row = state.rows.get(index)?;
        if let Some(preset) = row.preset.as_ref() {
            let starred = state.presets.contains(preset);
            let unavailable = !state.ready || state.saving;
            let label = format!(
                "{} model preset {}",
                if starred { "Remove" } else { "Save" },
                self.choices[index].label
            );
            return Some(
                button_shell(
                    SharedString::from(format!("preset-{index}")),
                    label.clone(),
                    false,
                )
                .aria_label(label)
                .size(px(24.))
                .p_0()
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .bg(rgba(0))
                .relative()
                .when(unavailable, |el| {
                    el.opacity(0.4)
                        .aria_description("Waiting for model preset storage")
                })
                .child(
                    icon(if starred {
                        Glyph::StarFilled
                    } else {
                        Glyph::Star
                    })
                    .size(px(14.)),
                )
                .child(layout_probe_slot("model-preset-star", position))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.star_preset(index, cx);
                    cx.stop_propagation();
                }))
                .into_any_element(),
            );
        }
        let favorite = row.favorite.as_ref()?;
        let starred = state.favorites.contains(favorite);
        let unavailable = !state.ready || state.saving;
        let label = format!(
            "{} {}",
            if starred { "Unstar" } else { "Star" },
            self.choices[index].label
        );
        Some(
            button_shell(
                SharedString::from(format!("favorite-{index}")),
                label.clone(),
                false,
            )
            .aria_label(label)
            .size(px(24.))
            .p_0()
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0))
            .relative()
            .when(unavailable, |el| {
                el.opacity(0.4)
                    .aria_description("Waiting for favorites storage")
            })
            .child(
                icon(if starred {
                    Glyph::StarFilled
                } else {
                    Glyph::Star
                })
                .size(px(14.)),
            )
            .child(layout_probe_slot("model-star", position))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.star_model(index, cx);
                cx.stop_propagation();
            }))
            .into_any_element(),
        )
    }
    fn star_model(&mut self, index: usize, cx: &mut Context<Self>) {
        self.navigation.armed = None;
        let Some(state) = &mut self.models else {
            return;
        };
        if !state.ready || state.saving {
            return;
        }
        let Some(favorite) = state.rows.get(index).and_then(|r| r.favorite.clone()) else {
            return;
        };
        let enabled = !state.favorites.contains(&favorite);
        let workspace = state.workspace.clone();
        state.saving = true;
        state.error = None;
        let job = state
            .runtime
            .spawn(async move { workspace.set_model_favorite(favorite, enabled).await });
        cx.spawn(async move |view, cx| {
            let result = job.await;
            let _ = view.update(cx, |this, cx| {
                if let Some(state) = &mut this.models {
                    state.saving = false;
                    match result {
                        Ok(Ok(favorites)) => state.favorites = favorites,
                        _ => state.error = Some("Favorite was not saved. Try again.".into()),
                    }
                }
                this.filter(this.query.clone(), cx);
            });
        })
        .detach();
        cx.notify();
    }
    fn star_preset(&mut self, index: usize, cx: &mut Context<Self>) {
        self.navigation.armed = None;
        let Some(state) = &mut self.models else {
            return;
        };
        if !state.ready || state.saving {
            return;
        }
        let Some(preset) = state.rows.get(index).and_then(|row| row.preset.clone()) else {
            return;
        };
        let enabled = !state.presets.contains(&preset);
        let workspace = state.workspace.clone();
        state.saving = true;
        state.error = None;
        let job = state
            .runtime
            .spawn(async move { workspace.set_session_model_preset(preset, enabled).await });
        cx.spawn(async move |view, cx| {
            let result = job.await;
            let _ = view.update(cx, |this, cx| {
                if let Some(state) = &mut this.models {
                    state.saving = false;
                    match result {
                        Ok(Ok(presets)) => {
                            state.presets = presets
                                .into_iter()
                                .filter(|preset| preset.agent == state.current_agent)
                                .collect()
                        }
                        _ => state.error = Some("Preset was not saved. Try again.".into()),
                    }
                }
                this.filter(this.query.clone(), cx);
            });
        })
        .detach();
        cx.notify();
    }
    pub(super) fn model_status(&self) -> gpui::AnyElement {
        let Some(state) = &self.models else {
            return div().into_any_element();
        };
        let status = state.error.as_deref().unwrap_or(if state.saving {
            "Saving model preferences..."
        } else if !state.ready {
            "Loading saved models..."
        } else {
            "Star the selected model to save its advertised effort; apply presets from Starred."
        });
        div()
            .px_2()
            .py_1()
            .text_size(px(10.))
            .text_color(rgb(palette().muted))
            .child(status.to_owned())
            .into_any_element()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn provider_filter_and_stars_never_fabricate_actions() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let a = ModelFavorite {
            agent: "a".into(),
            option: Some("model".into()),
            value: "same".into(),
        };
        let b = ModelFavorite {
            agent: "b".into(),
            ..a.clone()
        };
        let preset = SessionModelPreset {
            agent: "a".into(),
            model_option: Some("model".into()),
            model: "gpt-live".into(),
            effort_option: Some("reasoning_effort".into()),
            effort: Some("high".into()),
        };
        let mut s = ModelState {
            sources: vec![],
            rows: vec![
                ModelRow {
                    source: 0,
                    favorite: None,
                    preset: None,
                },
                ModelRow {
                    source: 0,
                    favorite: Some(a.clone()),
                    preset: None,
                },
                ModelRow {
                    source: 1,
                    favorite: Some(b.clone()),
                    preset: None,
                },
                ModelRow {
                    source: 0,
                    favorite: None,
                    preset: Some(preset.clone()),
                },
            ],
            tab: Some(0),
            favorites: vec![b],
            current_agent: "a".into(),
            current_source: 0,
            presets: vec![preset],
            ready: true,
            saving: false,
            error: None,
            workspace: WorkspaceService::memory().unwrap(),
            runtime: runtime.handle().clone(),
        };
        assert!(s.includes(0) && s.includes(1) && !s.includes(2));
        s.tab = None;
        assert!(!s.includes(0) && !s.includes(1) && s.includes(2));
        assert!(s.includes(3));
        s.tab = Some(0);
        assert!(!s.includes(3));
        s.tab = None;
        s.favorites.push(a);
        assert!(s.includes(1));
        assert!(!s.includes(100));
    }
}
