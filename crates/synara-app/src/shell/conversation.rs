use super::*;
use gpui::{Animation, AnimationExt};
impl Shell {
    pub(super) fn conversation(&self, window: &Window, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(thread) = &self.thread else {
            return div()
                .flex()
                .flex_col()
                .flex_1()
                .min_h_0()
                .child(self.welcome())
                .child(self.composer_panel(window, cx))
                .into_any_element();
        };
        let empty = thread.timeline.is_empty() && thread.plan.is_empty();
        let mut root = div().flex().flex_col().flex_1().min_h_0().min_w_0();
        if let Some(details) = &self.details {
            match details.connection.state {
                ConnectionState::Authenticating => {
                    root = root.child(div().px_5().py_2().child("Authentication in progress..."));
                }
                ConnectionState::AuthenticationRequired => {
                    root = root.child(
                        div().px_5().py_2().flex().gap_2().child("Authentication required:")
                            .children(details.connection.authentication.iter().enumerate().map(|(index, method)| {
                                let id = method.id.clone();
                                button(("login", index), method.name.clone(), false)
                                    .on_click(cx.listener(move |this, _, _, cx| this.authenticate(id.clone(), cx)))
                            }))
                            .when(details.connection.authentication.is_empty(), |el| {
                                el.child("No supported login flow was advertised. Authenticate the agent externally, then restart.")
                            }),
                    );
                }
                _ => {}
            }
        }
        // Login/configuration questions have no durable task thread yet. Render them
        // only for the selected connection, using the same native question panel.
        if let Some(connection_id) = self.details.as_ref().map(|details| details.connection.id) {
            let mut login_questions: Vec<_> = self
                .pending
                .iter()
                .filter(|(_, interaction)| {
                    interaction.is_active()
                        && interaction.context().scope
                            == InteractionScope::Connection(connection_id)
                })
                .map(|(key, _)| key.clone())
                .collect();
            login_questions.sort();
            if !login_questions.is_empty() {
                root = root.child(
                    div()
                        .px_5()
                        .py_3()
                        .max_h(px(360.))
                        .id("connection-questions")
                        .overflow_y_scroll()
                        .children(
                            login_questions
                                .into_iter()
                                .map(|key| self.input_request(key, cx)),
                        ),
                );
            }
        }
        root = root.child(self.chat_tools_bar(cx));
        if self.chat_tools.find_open {
            root = root.child(self.message_find_bar(cx));
        }
        root = root.child(if empty {
            self.welcome()
        } else {
            self.virtual_transcript(cx)
        });
        root = root.child(self.composer_panel(window, cx));
        root.with_animation(
            SharedString::from(format!("conversation-entry-{}", thread.id)),
            Animation::new(crate::ui::motion::PANE_DURATION)
                .with_easing(crate::ui::motion::ease_out),
            |el, progress| {
                tracing::debug!(target: "synara_ui_layout", surface = "conversation", progress, "motion-frame");
                el.opacity(progress)
            },
        )
        .into_any_element()
    }
    pub(super) fn transcript_item(
        &self,
        thread: &Thread,
        index: usize,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        match &thread.timeline[index] {
            TranscriptItem::Message { index: message } => {
                self.message_row(&thread.messages[*message], index, cx)
            }
            TranscriptItem::Tool { id } => {
                let Some(tool) = thread.tools.get(id) else {
                    return div().into_any_element();
                };
                div()
                    .p_3()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(crate::ui::palette().border))
                    .bg(rgb(crate::ui::palette().canvas))
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .child(format!("{} · {:?}", tool.title, tool.status)),
                    )
                    .children(tool.output.iter().map(|output| {
                        match output {
                            ToolOutput::Text { text } => div()
                                .mt_2()
                                .font_family(crate::ui::code_font())
                                .text_xs()
                                .child(truncate(text, 16 * 1024)),
                            ToolOutput::Diff {
                                path,
                                before,
                                after,
                            } => div()
                                .mt_2()
                                .font_family(crate::ui::code_font())
                                .text_xs()
                                .child(format!(
                                    "{}\n{}\n{}",
                                    path,
                                    before.as_deref().map_or(String::new(), |s| format!(
                                        "Before:\n{}",
                                        truncate(s, 8000)
                                    )),
                                    after.as_deref().map_or(String::new(), |s| format!(
                                        "After:\n{}",
                                        truncate(s, 8000)
                                    ))
                                )),
                            ToolOutput::Terminal { id } => div()
                                .mt_2()
                                .font_family(crate::ui::code_font())
                                .text_xs()
                                .child(thread.terminals.get(id).map_or_else(
                                    || format!("Terminal {id}"),
                                    |record| {
                                        format!(
                                            "Terminal {} · exit {:?}\n{}",
                                            id,
                                            record.exit_code,
                                            truncate(&record.text, 16000)
                                        )
                                    },
                                )),
                            ToolOutput::Resource { uri, name } => {
                                div().mt_2().child(format!("{name} · {uri}"))
                            }
                        }
                    }))
                    .into_any_element()
            }
            TranscriptItem::Permission { id } => {
                let key = (thread.id, id.clone());
                let request =
                    self.pending
                        .get(&key)
                        .filter(|p| p.is_active())
                        .and_then(|p| match p {
                            UiInteraction::Permission { request, .. } => Some(request),
                            _ => None,
                        });
                if let Some(request) = request {
                    div()
                        .p_4()
                        .rounded_md()
                        .bg(rgb(0x3a3020))
                        .border_1()
                        .border_color(rgb(0x88703f))
                        .child(
                            div()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .child(request.title.clone()),
                        )
                        .child(
                            div()
                                .text_xs()
                                .mt_1()
                                .child("The agent is waiting for your decision."),
                        )
                        .child(
                            div()
                                .mt_3()
                                .flex()
                                .flex_wrap()
                                .gap_2()
                                .children(request.choices.iter().enumerate().map(
                                    |(index, choice)| {
                                        let key = key.clone();
                                        let selected = choice.id.clone();
                                        button(
                                            ("permission-choice", index),
                                            choice.label.clone(),
                                            false,
                                        )
                                        .relative()
                                        .child(crate::ui::layout_probe(match choice.kind {
                                            PermissionKind::AllowOnce => "permission-allow-once",
                                            PermissionKind::AllowAlways => {
                                                "permission-allow-always"
                                            }
                                            PermissionKind::DenyOnce => "permission-deny-once",
                                            PermissionKind::DenyAlways => "permission-deny-always",
                                        }))
                                        .on_click(
                                            cx.listener(move |this, _, _, cx| {
                                                this.answer_permission(
                                                    key.clone(),
                                                    Some(selected.clone()),
                                                    cx,
                                                )
                                            }),
                                        )
                                    },
                                ))
                                .child(
                                    button("cancel-permission", "Cancel request", false).on_click(
                                        cx.listener(move |this, _, _, cx| {
                                            this.answer_permission(key.clone(), None, cx)
                                        }),
                                    ),
                                ),
                        )
                        .into_any_element()
                } else {
                    div()
                        .px_3()
                        .py_1()
                        .text_xs()
                        .text_color(rgb(0x94a2b4))
                        .child("Permission request resolved or expired")
                        .into_any_element()
                }
            }
            TranscriptItem::Input { id } => self.input_request((thread.id, id.clone()), cx),
            TranscriptItem::Notice { text, is_error } => div()
                .flex()
                .items_start()
                .gap_2()
                .text_size(px(15.))
                .line_height(px(24.))
                .text_color(rgb(crate::ui::palette().muted))
                .children(is_error.then(|| crate::ui::icon(crate::ui::Glyph::Error).mt_1()))
                .child(truncate(text, 16000))
                .into_any_element(),
        }
    }
    fn answer_permission(
        &mut self,
        key: InteractionKey,
        selected: Option<String>,
        cx: &mut Context<Self>,
    ) {
        if self
            .pending
            .get(&key)
            .is_none_or(|request| !request.is_active())
        {
            self.pending.remove(&key);
            self.error = Some("This permission request is no longer active.".into());
        } else if let Some(UiInteraction::Permission { response, .. }) = self.pending.remove(&key)
            && response.send(selected).is_err()
        {
            self.error = Some("This permission request is no longer active.".into());
        }
        cx.notify();
    }
    fn input_request(&self, key: InteractionKey, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(form) = self
            .forms
            .get(&key)
            .filter(|_| self.pending.get(&key).is_some_and(UiInteraction::is_active))
        else {
            return div()
                .text_xs()
                .text_color(rgb(0x94a2b4))
                .child("User input request resolved or expired")
                .into_any_element();
        };
        let mut panel = div()
            .p_4()
            .rounded_md()
            .border_1()
            .border_color(rgb(0x657fad))
            .bg(rgb(0x1e2b40))
            .child(
                div()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(form.request.message.clone()),
            );
        if let Some(url) = &form.request.url {
            let url = url.clone();
            let url_key = key.clone();
            let destination = url.split(['?', '#']).next().unwrap_or_default().to_owned();
            panel = panel.child(
                div()
                    .mt_2()
                    .text_xs()
                    .child(format!("Agent-provided website: {destination}")),
            );
            panel = panel.child(
                button("open-input-url", "Open requested website", false)
                    .mt_2()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if !this
                            .pending
                            .get(&url_key)
                            .is_some_and(UiInteraction::is_active)
                        {
                            this.error = Some("This website request is no longer active.".into());
                        } else if synara_agent::validate_web_url(&url).is_ok() {
                            cx.open_url(&url);
                        } else {
                            this.error =
                                Some("The requested website address is not supported.".into());
                        }
                        cx.notify();
                    })),
            );
        }
        for field in &form.request.fields {
            let mut row = div().mt_3().flex().flex_col().gap_1().child(format!(
                "{}{}",
                field.label,
                if field.required { " *" } else { "" }
            ));
            if let Some(input) = form.inputs.get(&field.id) {
                row = row.child(input.clone());
            }
            match &field.kind {
                InputFieldKind::Boolean => {
                    let key = key.clone();
                    let field_id = field.id.clone();
                    let value =
                        matches!(form.values.get(&field.id), Some(InputValue::Boolean(true)));
                    row = row.child(
                        button(
                            SharedString::from(field.id.clone()),
                            if value { "Yes" } else { "No" },
                            value,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if let Some(form) = this.forms.get_mut(&key) {
                                form.values
                                    .insert(field_id.clone(), InputValue::Boolean(!value));
                            }
                            cx.notify();
                        })),
                    );
                }
                InputFieldKind::Choice { options }
                | InputFieldKind::MultiChoice { options, .. } => {
                    let multi = matches!(field.kind, InputFieldKind::MultiChoice { .. });
                    row = row.child(div().flex().flex_wrap().gap_2().children(
                        options.iter().enumerate().map(|(index, option)| {
                            let selected = match form.values.get(&field.id) {
                                Some(InputValue::Text(value)) => value == &option.value,
                                Some(InputValue::Strings(values)) => values.contains(&option.value),
                                _ => false,
                            };
                            let key = key.clone();
                            let field = field.id.clone();
                            let value = option.value.clone();
                            button(("input-option", index), option.label.clone(), selected)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    if let Some(form) = this.forms.get_mut(&key) {
                                        if multi {
                                            let mut values = match form.values.get(&field) {
                                                Some(InputValue::Strings(values)) => values.clone(),
                                                _ => vec![],
                                            };
                                            if selected {
                                                values.retain(|v| v != &value);
                                            } else {
                                                values.push(value.clone());
                                            }
                                            form.values
                                                .insert(field.clone(), InputValue::Strings(values));
                                        } else {
                                            form.values.insert(
                                                field.clone(),
                                                InputValue::Text(value.clone()),
                                            );
                                        }
                                    }
                                    cx.notify();
                                }))
                        }),
                    ));
                }
                _ => {}
            }
            panel = panel.child(row);
        }
        if let Some(error) = &form.error {
            panel = panel.child(div().mt_2().text_color(rgb(0xffb8bc)).child(error.clone()));
        }
        let decline = key.clone();
        let cancel = key.clone();
        panel
            .child(
                div()
                    .mt_3()
                    .flex()
                    .gap_2()
                    .child(button("submit-input", "Submit", true).on_click(cx.listener(
                        move |this, _, _, cx| {
                            this.answer_input(key.clone(), true, UserInputResponse::Cancel, cx)
                        },
                    )))
                    .child(
                        button("decline-input", "Decline", false).on_click(cx.listener(
                            move |this, _, _, cx| {
                                this.answer_input(
                                    decline.clone(),
                                    false,
                                    UserInputResponse::Decline,
                                    cx,
                                )
                            },
                        )),
                    )
                    .child(
                        button("cancel-input", "Cancel", false).on_click(cx.listener(
                            move |this, _, _, cx| {
                                this.answer_input(
                                    cancel.clone(),
                                    false,
                                    UserInputResponse::Cancel,
                                    cx,
                                )
                            },
                        )),
                    ),
            )
            .into_any_element()
    }
    fn answer_input(
        &mut self,
        key: InteractionKey,
        accept: bool,
        fallback: UserInputResponse,
        cx: &mut Context<Self>,
    ) {
        if !self.pending.get(&key).is_some_and(UiInteraction::is_active) {
            self.pending.remove(&key);
            self.forms.remove(&key);
            self.error = Some("This input request is no longer active.".into());
            cx.notify();
            return;
        }
        let response = if accept {
            let Some(form) = self.forms.get_mut(&key) else {
                return;
            };
            let mut values = form.values.clone();
            for field in &form.request.fields {
                if let Some(input) = form.inputs.get(&field.id) {
                    let text = input.read(cx).text();
                    if text.is_empty() && !field.required {
                        continue;
                    }
                    let value = match field.kind {
                        InputFieldKind::Number { .. } => match text.parse::<f64>() {
                            Ok(value) => InputValue::Number(value),
                            Err(_) => {
                                form.error = Some(format!("{} must be a number", field.label));
                                cx.notify();
                                return;
                            }
                        },
                        _ => InputValue::Text(text.into()),
                    };
                    values.insert(field.id.clone(), value);
                }
            }
            if let Err(error) = synara_agent::validate_input(&form.request, &values) {
                form.error = Some(error.to_string());
                cx.notify();
                return;
            }
            UserInputResponse::Accept { values }
        } else {
            fallback
        };
        if let Some(UiInteraction::Input {
            response: sender, ..
        }) = self.pending.remove(&key)
            && sender.send(response).is_err()
        {
            self.error = Some("This input request is no longer active.".into());
        }
        self.forms.remove(&key);
        cx.notify();
    }
}
