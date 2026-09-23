use super::*;
use crate::ui::Glyph;
fn note(text: impl Into<SharedString>) -> gpui::Div {
    let text: SharedString = text.into();
    div().text_sm().text_color(rgb(palette().muted)).child(text)
}
fn row() -> gpui::Div {
    div()
        .py_3()
        .border_b_1()
        .border_color(rgb(palette().border))
        .flex()
        .flex_col()
        .gap_2()
}
impl Shell {
    pub(in crate::shell) fn direct_model_settings(
        &self,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let state = &self.direct_models;
        let mut body=div().id("direct-model-settings").flex().flex_col().gap_2()
            .child(note("Direct HTTP models are separate from ACP coding agents. Text conversations stream into the existing transcript. No browser, filesystem, device or MCP tool authority is granted."))
            .child(note(format!("OS credential store: {:?}. This is not provider authentication or quota information.",self.controller.secret_store_state())))
            .children(state.error.as_ref().map(|s|div().text_color(rgb(palette().error)).child(s.clone())))
            .children(state.notice.as_ref().map(|s|note(s.clone())))
            .children(state.busy.then(||note("The requested operation is in progress. No additional operation is queued.")));
        if let Some(review) = &state.review {
            let (title, detail) = match review {
                Review::Route {
                    title,
                    selection,
                    endpoint,
                    ..
                } => (
                    format!("Review route for {title}"),
                    if let Some(selection) = selection {
                        format!(
                            "Model: {} / {}. Next explicit Send will share this chat's retained visible user/assistant text and user images with {endpoint}. Hidden reasoning, approvals, provider sessions and filesystem state are not transferred. Existing ACP sessions are retired. Reviewed PNG/JPEG images and plain-text attachments are included according to the history window below. Autonomous tool execution is unavailable.",
                            selection.provider_id, selection.model_id
                        )
                    } else {
                        "Return to the task's ACP coding agent using a new session. The transcript stays local, but prior direct-model messages are not automatically sent to that agent.".into()
                    },
                ),
                Review::Key {
                    endpoint, delete, ..
                } => (
                    if *delete {
                        "Delete this endpoint's OS key".into()
                    } else {
                        "Store clipboard API key".into()
                    },
                    format!(
                        "Endpoint: {endpoint}. {} No plaintext fallback is available. The clipboard is not changed.",
                        if *delete {
                            "Only this profile/endpoint credential will be removed. Provider-side revocation is separate."
                        } else {
                            "The current clipboard text will be stored in the OS credential store, never shown in an input field or transcript."
                        }
                    ),
                ),
            };
            body=body.child(row().child(title).child(note(detail))
                .children(matches!(review,Review::Route {selection:Some(_),..}).then(||div().flex().flex_col().gap_2()
                    .child(note("Model options (JSON). Identity is fixed by this review. JSON schema output is validated locally before completion. Supported: types, enum/const, object properties/required/additionalProperties, items, size limits and allOf/anyOf/oneOf/not. References, formats, regexes and numeric ranges are rejected before sending."))
                    .child(self.direct_option_presets(cx))
                    .child(div().relative().h(px(220.)).flex().flex_col().child(state.options.clone()).child(ui::layout_probe("direct-options-editor")))))
                .child(div().flex().gap_2()
                    .child(ui::action("direct-confirm","Confirm reviewed change",None,false,cx.listener(|this,_,_,cx|this.confirm_direct_review(cx))).relative().child(ui::layout_probe("direct-confirm")))
                    .child(ui::action("direct-cancel-review","Cancel",None,false,cx.listener(|this,_,_,cx|{if !this.direct_models.busy{this.direct_models.review=None;}cx.notify();})))));
            return body.into_any_element();
        }
        if state.editing {
            return body.child(note("Review the endpoint, models and capability metadata. Supported/unsupported/unknown are explicit. Save never makes a provider request. Keep API keys out of JSON. Protocols: open_ai_chat, anthropic_messages, google_generate_content. Google uses its API key header, not URL credentials. Model capabilities must be reviewed, not guessed. Changing a profile invalidates old task bindings."))
                .child(div().relative().h(px(360.)).flex().flex_col().child(state.editor.clone()).child(ui::layout_probe("direct-config-editor")))
                .child(div().flex().gap_2()
                    .child(ui::action("direct-save","Save reviewed providers",None,false,cx.listener(|this,_,_,cx|this.save_direct_models(cx))).relative().child(ui::layout_probe("direct-save")))
                    .child(ui::action("direct-discard","Discard editor",None,false,cx.listener(|this,_,_,cx|{if !this.direct_models.busy{this.direct_models.editing=false;this.direct_models.editor.update(cx,|e,cx|e.clear(cx));}cx.notify();}))))
                .into_any_element();
        }
        body = body
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(ui::action(
                        "direct-reload",
                        "Reload",
                        None,
                        false,
                        cx.listener(|this, _, _, cx| this.load_direct_models(cx)),
                    ))
                    .child(
                        ui::action(
                            "direct-configure",
                            "Configure providers",
                            None,
                            false,
                            cx.listener(|this, _, _, cx| this.edit_direct_models(false, cx)),
                        )
                        .relative()
                        .child(ui::layout_probe("direct-configure")),
                    )
                    .child(
                        ui::action(
                            "direct-custom",
                            "Add custom endpoint",
                            None,
                            false,
                            cx.listener(|this, _, _, cx| this.edit_direct_models(true, cx)),
                        )
                        .relative()
                        .child(ui::layout_probe("direct-custom")),
                    )
                    .child(
                        ui::action(
                            "direct-google",
                            "Add Google API",
                            None,
                            false,
                            cx.listener(|this, _, _, cx| this.edit_google_provider(cx)),
                        )
                        .relative()
                        .child(ui::layout_probe("direct-google")),
                    )
                    .child(ui::action(
                        "direct-catalog",
                        "Load models.dev catalog",
                        None,
                        false,
                        cx.listener(|this, _, _, cx| this.load_direct_catalog(cx)),
                    )),
            )
            .child(
                div()
                    .relative()
                    .child(state.query.clone())
                    .child(ui::layout_probe("direct-search")),
            );
        if let Some(selection) = self
            .selected
            .and_then(|id| state.bindings.get(&id))
            .and_then(Option::as_ref)
            .map(|binding| binding.selection.clone())
        {
            body = body.child(
                ui::action(
                    "direct-edit-current",
                    "Review current model options",
                    None,
                    false,
                    cx.listener(move |this, _, _, cx| {
                        this.review_direct_route(Some(selection.clone()), cx)
                    }),
                )
                .relative()
                .child(ui::layout_probe("direct-edit-current")),
            );
        }
        let query = state.query.read(cx).text().trim().to_lowercase();
        if let Some(value) = &state.value {
            body = body.child(
                row()
                    .child(div().font_weight(gpui::FontWeight::SEMIBOLD).child("Favorites"))
                    .child(note(
                        "Review a favorite to switch this conversation. The route still needs explicit confirmation before it changes.",
                    )),
            );
            if let Some(error) = &state.favorites_error {
                body = body.child(note(error.clone()));
            } else if !state.favorites_ready {
                body = body.child(note("Loading model favorites..."));
            } else {
                let favorites = state
                    .favorites
                    .iter()
                    .enumerate()
                    .filter(|(_, favorite)| {
                        direct_favorite_provider_id(favorite).is_some_and(|provider_id| {
                            let profile = value
                                .providers
                                .iter()
                                .find(|profile| profile.id == provider_id);
                            let model = profile.and_then(|profile| {
                                profile
                                    .models
                                    .iter()
                                    .find(|model| model.id == favorite.value)
                            });
                            query.is_empty()
                                || provider_id.to_lowercase().contains(&query)
                                || favorite.value.to_lowercase().contains(&query)
                                || profile.is_some_and(|profile| {
                                    profile.name.to_lowercase().contains(&query)
                                })
                                || model
                                    .is_some_and(|model| model.name.to_lowercase().contains(&query))
                        })
                    })
                    .collect::<Vec<_>>();
                if favorites.is_empty() {
                    body = body.child(note(if query.is_empty() {
                        "No direct model favorites yet. Star a model below to keep it here."
                    } else {
                        "No matching favorites."
                    }));
                }
                for (favorite_position, (favorite_index, favorite)) in
                    favorites.into_iter().enumerate()
                {
                    let Some(provider_id) = direct_favorite_provider_id(favorite) else {
                        continue;
                    };
                    let profile = value
                        .providers
                        .iter()
                        .find(|profile| profile.id == provider_id);
                    let model = profile.and_then(|profile| {
                        profile
                            .models
                            .iter()
                            .find(|model| model.id == favorite.value)
                    });
                    let model_label =
                        model.map_or(favorite.value.as_str(), |model| model.name.as_str());
                    let provider_label =
                        profile.map_or(provider_id, |profile| profile.name.as_str());
                    let mut favorite_row = row()
                        .child(
                            div()
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .child(model_label.to_owned()),
                        )
                        .child(note(format!("{provider_label} · {}", favorite.value)));
                    if let (Some(profile), Some(model)) = (profile, model) {
                        let selection = ModelSelection {
                            history_turns: None,
                            provider_id: profile.id.clone(),
                            model_id: model.id.clone(),
                            max_output_tokens: model
                                .capabilities
                                .max_output_tokens
                                .unwrap_or(4096)
                                .min(4096) as u32,
                            reasoning_effort: None,
                            output: OutputFormat::Text,
                        };
                        favorite_row = favorite_row.child(
                            ui::action(
                                SharedString::from(format!(
                                    "direct-favorite-review-{favorite_index}"
                                )),
                                "Review to switch",
                                None,
                                false,
                                cx.listener(move |this, _, _, cx| {
                                    this.review_direct_route(Some(selection.clone()), cx)
                                }),
                            )
                            .relative()
                            .children(
                                (favorite_position == 0)
                                    .then(|| ui::layout_probe("direct-first-favorite")),
                            ),
                        );
                    } else {
                        favorite_row = favorite_row.child(note(
                            "This model is no longer configured. Remove the favorite or restore its provider profile.",
                        ));
                    }
                    let provider_id = provider_id.to_owned();
                    let model_id = favorite.value.clone();
                    favorite_row = favorite_row.child(ui::action(
                        SharedString::from(format!("direct-favorite-remove-{favorite_index}")),
                        "Remove favorite",
                        None,
                        true,
                        cx.listener(move |this, _, _, cx| {
                            this.toggle_direct_model_favorite(
                                provider_id.clone(),
                                model_id.clone(),
                                cx,
                            )
                        }),
                    ));
                    body = body.child(favorite_row);
                }
            }
            if value.providers.is_empty() {
                body=body.child(note("No direct providers are configured. Review a catalog candidate or configure your own compatible endpoint. No local server is assumed to be installed."));
            }
            for (index, profile) in value.providers.iter().enumerate().filter(|(_, p)| {
                query.is_empty()
                    || p.name.to_lowercase().contains(&query)
                    || p.id.to_lowercase().contains(&query)
                    || p.models
                        .iter()
                        .any(|m| m.id.to_lowercase().contains(&query))
            }) {
                let id = profile.id.clone();
                let discover = id.clone();
                let store = id.clone();
                let delete = id.clone();
                body = body.child(
                    row()
                        .child(
                            div()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .child(profile.name.clone()),
                        )
                        .child(note(format!(
                            "{} · {:?} · {} model identities · {}",
                            profile.endpoint,
                            profile.protocol,
                            profile.models.len(),
                            if profile.requires_key {
                                "OS key required"
                            } else {
                                "explicitly no key"
                            }
                        )))
                        .child(
                            div()
                                .flex()
                                .flex_wrap()
                                .gap_2()
                                .child(
                                    ui::action(
                                        ("direct-expand", index),
                                        "Models",
                                        None,
                                        state.expanded.as_ref() == Some(&id),
                                        cx.listener(move |this, _, _, cx| {
                                            this.direct_models.expanded = Some(id.clone());
                                            cx.notify();
                                        }),
                                    )
                                    .relative()
                                    .child(ui::layout_probe("direct-expand")),
                                )
                                .child(
                                    ui::action(
                                        ("direct-discover", index),
                                        "Discover model IDs",
                                        None,
                                        false,
                                        cx.listener(move |this, _, _, cx| {
                                            this.discover_direct_models(discover.clone(), cx)
                                        }),
                                    )
                                    .relative()
                                    .child(ui::layout_probe("direct-discover")),
                                )
                                .children(profile.requires_key.then(|| {
                                    ui::action(
                                        ("direct-key", index),
                                        "Store clipboard key",
                                        None,
                                        false,
                                        cx.listener(move |this, _, _, cx| {
                                            this.review_direct_key(store.clone(), false, cx)
                                        }),
                                    )
                                }))
                                .children(profile.requires_key.then(|| {
                                    ui::action(
                                        ("direct-key-delete", index),
                                        "Delete OS key",
                                        None,
                                        false,
                                        cx.listener(move |this, _, _, cx| {
                                            this.review_direct_key(delete.clone(), true, cx)
                                        }),
                                    )
                                })),
                        ),
                );
                if state.expanded.as_ref() == Some(&profile.id) {
                    for (model_index, model) in profile
                        .models
                        .iter()
                        .enumerate()
                        .filter(|(_, m)| {
                            query.is_empty()
                                || profile.name.to_lowercase().contains(&query)
                                || profile.id.to_lowercase().contains(&query)
                                || m.id.to_lowercase().contains(&query)
                        })
                        .take(60)
                    {
                        let selection = ModelSelection {
                            history_turns: None,
                            provider_id: profile.id.clone(),
                            model_id: model.id.clone(),
                            max_output_tokens: model
                                .capabilities
                                .max_output_tokens
                                .unwrap_or(4096)
                                .min(4096) as u32,
                            reasoning_effort: None,
                            output: OutputFormat::Text,
                        };
                        let element_id =
                            SharedString::from(format!("direct-model-{index}-{model_index}"));
                        let model_favorite = direct_model_favorite(&profile.id, &model.id);
                        let is_favorite = state.favorites.contains(&model_favorite);
                        let favorite_provider = profile.id.clone();
                        let favorite_model = model.id.clone();
                        body=body.child(row().child(div().flex().gap_2().items_center()
                            .child(div().flex_1().min_w_0().child(model.name.clone()))
                            .child(ui::action(element_id,"Review for this chat",None,false,cx.listener(move|this,_,_,cx|this.review_direct_route(Some(selection.clone()),cx))).relative().children((model_index==0).then(||ui::layout_probe("direct-first-model")))))
                            .child(ui::action(
                                SharedString::from(format!("direct-model-favorite-{index}-{model_index}")),
                                if is_favorite { "Remove favorite" } else { "Add favorite" },
                                Some(if is_favorite { Glyph::StarFilled } else { Glyph::Star }),
                                is_favorite,
                                cx.listener(move |this, _, _, cx| {
                                    this.toggle_direct_model_favorite(
                                        favorite_provider.clone(),
                                        favorite_model.clone(),
                                        cx,
                                    )
                                }),
                            ))
                            .child(note(format!("{} · tools {:?} · images {:?} · structured output {:?} · context {:?}",model.id,model.capabilities.tools,model.capabilities.images,model.capabilities.structured_output,model.capabilities.context_window)))
                            .child(note(model.capabilities.source.clone())));
                    }
                    if profile.models.len() > 60 {
                        body=body.child(note("At most 60 matching models are shown. Narrow the search for another model."));
                    }
                }
            }
            if self.uses_direct_model() {
                body = body.child(
                    ui::action(
                        "direct-return-agent",
                        "Review return to ACP coding agent",
                        None,
                        false,
                        cx.listener(|this, _, _, cx| this.review_direct_route(None, cx)),
                    )
                    .relative()
                    .child(ui::layout_probe("direct-return-agent")),
                );
            }
        }
        if let Some(catalog) = &state.catalog {
            let compatible = catalog
                .providers
                .iter()
                .filter(|p| p.profile.is_some())
                .count();
            body=body.child(row().child(format!("Catalog: {} providers, {compatible} compatible metadata candidates",catalog.providers.len()))
                .child(note("Catalog breadth is not tested provider coverage. Unsupported protocols/cloud authentication remain explicit. Loading this catalog sends no chat content or provider key to models.dev.")));
            for (index, entry) in catalog
                .providers
                .iter()
                .enumerate()
                .filter(|(_, p)| {
                    query.is_empty()
                        || p.id.to_lowercase().contains(&query)
                        || p.name.to_lowercase().contains(&query)
                })
                .take(30)
            {
                let id = entry.id.clone();
                body = body.child(
                    row()
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .items_center()
                                .child(div().flex_1().child(entry.name.clone()))
                                .children(entry.profile.is_some().then(|| {
                                    ui::action(
                                        ("direct-catalog-review", index),
                                        "Review configuration",
                                        None,
                                        false,
                                        cx.listener(move |this, _, _, cx| {
                                            this.review_catalog_provider(id.clone(), cx)
                                        }),
                                    )
                                })),
                        )
                        .child(note(
                            entry
                                .unsupported_reason
                                .map(str::to_owned)
                                .unwrap_or_else(|| {
                                    format!(
                                        "{} model identities",
                                        entry.profile.as_ref().map_or(0, |p| p.models.len())
                                    )
                                }),
                        )),
                );
            }
            body = body.child(note(
                "At most 30 provider results are shown. Search for a provider to narrow the list.",
            ));
        }
        body.into_any_element()
    }
}
