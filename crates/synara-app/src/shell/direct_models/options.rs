//! These controls edit the existing review draft only. Confirmation remains the
//! sole route mutation and Send remains the sole inference boundary.
use super::*;

enum Edit {
    History(Option<u16>),
    Output(u32),
    Effort(Option<String>),
}
impl Shell {
    fn edit_direct_option(&mut self, edit: Edit, cx: &mut Context<Self>) {
        if self.direct_models.busy
            || !matches!(
                self.direct_models.review,
                Some(Review::Route {
                    selection: Some(_),
                    ..
                })
            )
        {
            return;
        }
        let parsed =
            serde_json::from_str::<ModelSelection>(self.direct_models.options.read(cx).text());
        let mut selection = match parsed {
            Ok(value) => value,
            Err(_) => {
                self.direct_models.error = Some(
                    "Fix the model options JSON before choosing a preset. Nothing was replaced."
                        .into(),
                );
                cx.notify();
                return;
            }
        };
        match edit {
            Edit::History(value) => selection.history_turns = value,
            Edit::Output(value) => selection.max_output_tokens = value,
            Edit::Effort(value) => selection.reasoning_effort = value,
        }
        match serde_json::to_string_pretty(&selection) {
            Ok(text) => {
                self.direct_models
                    .options
                    .update(cx, |entry, cx| entry.set_text(text, cx));
                self.direct_models.error = None;
            }
            Err(_) => {
                self.direct_models.error =
                    Some("Model options could not be encoded. Nothing was changed.".into())
            }
        }
        cx.notify();
    }
    pub(super) fn direct_option_presets(&self, cx: &mut Context<Self>) -> gpui::Div {
        let selection =
            serde_json::from_str::<ModelSelection>(self.direct_models.options.read(cx).text()).ok();
        let mut body = div().flex().flex_col().gap_2()
            .child("Context sent on the next explicit Send")
            .child(div().text_sm().text_color(rgb(palette().muted)).child(
                "History is never deleted. null sends all history, 0 sends the current prompt only, and 1-256 retains that many prior user turns with their replies and images. Oversized requests fail rather than silently truncate."
            ));
        let mut history = div().flex().flex_wrap().gap_2();
        for (id, label, value) in [
            ("direct-history-all", "All history", None),
            ("direct-history-10", "Last 10 turns", Some(10)),
            ("direct-history-current", "Current message only", Some(0)),
        ] {
            history = history.child(
                ui::action(
                    id,
                    label,
                    None,
                    selection.as_ref().is_some_and(|s| s.history_turns == value),
                    cx.listener(move |this, _, _, cx| {
                        this.edit_direct_option(Edit::History(value), cx)
                    }),
                )
                .relative()
                .child(ui::layout_probe(id)),
            );
        }
        body = body.child(history);
        let profile = selection.as_ref().and_then(|s| {
            self.direct_models
                .value
                .as_ref()
                .and_then(|v| v.providers.iter().find(|p| p.id == s.provider_id))
        });
        let model = selection
            .as_ref()
            .zip(profile)
            .and_then(|(s, p)| p.models.iter().find(|m| m.id == s.model_id));
        let maximum = model
            .and_then(|m| m.capabilities.max_output_tokens)
            .unwrap_or(131072)
            .min(131072);
        if let Some(window) = model.and_then(|m| m.capabilities.context_window) {
            let reserved = selection
                .as_ref()
                .map_or(0, |s| u64::from(s.max_output_tokens));
            body = body.child(
                div().text_sm().text_color(rgb(palette().muted)).child(format!(
                    "Model context: {window} tokens. Requested output: {reserved}. Remaining context is shared by history, attachments and the current prompt."
                )),
            );
            if let (Some(selection), Some(thread)) = (selection.as_ref(), self.thread.as_ref()) {
                let fitted = synara_workspace::suggested_direct_history_turns(
                    thread,
                    window,
                    selection.max_output_tokens,
                );
                let label = match fitted {
                    None => "Fit context: all history".to_owned(),
                    Some(0) => "Fit context: current message only".to_owned(),
                    Some(turns) => format!("Fit context: last {turns} turns"),
                };
                body = body
                    .child(
                        ui::action(
                            "direct-history-fit",
                            label,
                            None,
                            selection.history_turns == fitted,
                            cx.listener(move |this, _, _, cx| {
                                this.edit_direct_option(Edit::History(fitted), cx)
                            }),
                        )
                        .relative()
                        .child(ui::layout_probe("direct-history-fit")),
                    )
                    .child(
                        div().text_xs().text_color(rgb(palette().muted)).child(
                            "Fit context uses a conservative byte upper bound against the reviewed model window, reserves output plus space for the next prompt, and only changes the reviewed history window. It never deletes or summarizes the local transcript.",
                        ),
                    );
            }
        }
        let mut output = div()
            .flex()
            .flex_wrap()
            .gap_2()
            .child("Maximum output tokens");
        for (value, id) in [
            (1024u32, "direct-output-1024"),
            (4096, "direct-output-4096"),
            (8192, "direct-output-8192"),
        ] {
            if u64::from(value) <= maximum {
                output = output.child(
                    ui::action(
                        id,
                        value.to_string(),
                        None,
                        selection
                            .as_ref()
                            .is_some_and(|s| s.max_output_tokens == value),
                        cx.listener(move |this, _, _, cx| {
                            this.edit_direct_option(Edit::Output(value), cx)
                        }),
                    )
                    .relative()
                    .child(ui::layout_probe(id)),
                );
            }
        }
        body = body.child(output);
        if profile.is_some_and(|p| p.protocol == synara_model::ProtocolFamily::OpenAiChat) {
            let mut efforts = div()
                .flex()
                .flex_wrap()
                .gap_2()
                .child("Reasoning effort")
                .child(ui::action(
                    "direct-effort-default",
                    "Provider default",
                    None,
                    selection
                        .as_ref()
                        .is_some_and(|s| s.reasoning_effort.is_none()),
                    cx.listener(|this, _, _, cx| this.edit_direct_option(Edit::Effort(None), cx)),
                ));
            if let Some(model) = model {
                let offered = &model.capabilities.reasoning_efforts;
                let fast = ["low", "minimal", "none"]
                    .iter()
                    .find_map(|name| offered.iter().find(|effort| effort.as_str() == *name))
                    .cloned();
                let balanced = offered
                    .iter()
                    .find(|effort| effort.as_str() == "medium")
                    .cloned();
                let thinking = ["high", "xhigh", "max"]
                    .iter()
                    .find_map(|name| offered.iter().find(|effort| effort.as_str() == *name))
                    .cloned();
                if let Some(value) = fast {
                    let selected = selection
                        .as_ref()
                        .is_some_and(|s| s.reasoning_effort.as_ref() == Some(&value));
                    efforts = efforts.child(ui::action(
                        "direct-preset-fast",
                        "Fast",
                        None,
                        selected,
                        cx.listener(move |this, _, _, cx| {
                            this.edit_direct_option(Edit::Effort(Some(value.clone())), cx)
                        }),
                    ));
                }
                if let Some(value) = balanced {
                    let selected = selection
                        .as_ref()
                        .is_some_and(|s| s.reasoning_effort.as_ref() == Some(&value));
                    efforts = efforts.child(ui::action(
                        "direct-preset-balanced",
                        "Balanced",
                        None,
                        selected,
                        cx.listener(move |this, _, _, cx| {
                            this.edit_direct_option(Edit::Effort(Some(value.clone())), cx)
                        }),
                    ));
                }
                if let Some(value) = thinking {
                    let selected = selection
                        .as_ref()
                        .is_some_and(|s| s.reasoning_effort.as_ref() == Some(&value));
                    efforts = efforts.child(ui::action(
                        "direct-preset-thinking",
                        "Thinking",
                        None,
                        selected,
                        cx.listener(move |this, _, _, cx| {
                            this.edit_direct_option(Edit::Effort(Some(value.clone())), cx)
                        }),
                    ));
                }
                for (index, effort) in model.capabilities.reasoning_efforts.iter().enumerate() {
                    let value = effort.clone();
                    efforts = efforts.child(ui::action(
                        ("direct-effort", index),
                        effort.clone(),
                        None,
                        selection
                            .as_ref()
                            .is_some_and(|s| s.reasoning_effort.as_ref() == Some(effort)),
                        cx.listener(move |this, _, _, cx| {
                            this.edit_direct_option(Edit::Effort(Some(value.clone())), cx)
                        }),
                    ));
                }
            }
            body = body.child(efforts).child(
                div().text_xs().text_color(rgb(palette().muted)).child(
                    "Presets select only reasoning levels offered by this model. Confirm the route to save; inference starts only when you Send.",
                ),
            );
        }
        body
    }
    pub(in crate::shell) fn direct_attachment_error(&self, images: bool) -> Option<&'static str> {
        if !images {
            return None;
        }
        let profile = self
            .selected
            .and_then(|id| self.direct_models.bindings.get(&id))
            .and_then(Option::as_ref)
            .and_then(|binding| {
                self.direct_models
                    .value
                    .as_ref()
                    .and_then(|settings| binding.profile(settings).ok())
                    .and_then(|p| p.model(&binding.selection.model_id).ok())
            });
        match profile {
            Some(model) if model.capabilities.images == synara_model::Support::Supported => None,
            Some(_) => Some(
                "This direct model has not been reviewed as image-capable. Remove the images or review a compatible model.",
            ),
            None => Some(
                "Reload Direct models settings and review the current model before sending images.",
            ),
        }
    }
}
