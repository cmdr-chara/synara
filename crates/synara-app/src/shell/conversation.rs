use super::*;
impl Shell {
    pub(super) fn conversation(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(thread) = &self.thread else {
            return div().flex_1().flex().flex_col().justify_center().items_center().gap_3().p_8()
                .child(div().text_2xl().child("Bring your work into focus"))
                .child("Open a local directory, create a task, and choose your coding agent.")
                .child("Agent commands are configured in Settings. No agent starts until you connect or send a prompt.")
                .into_any_element();
        };
        let id = thread.id;
        let configuration = &thread.configuration;
        let title = self.task().map_or("Task", |t| t.title.as_str());
        let busy = self.selected.is_some_and(|id| self.busy.contains(&id));
        let connecting = self
            .selected
            .is_some_and(|id| self.connecting.contains(&id));
        let mut root = div().flex().flex_col().flex_1().min_h_0().child(
            div()
                .px_5()
                .py_3()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .border_b_1()
                .border_color(rgb(0x293442))
                .child(
                    div()
                        .text_lg()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .child(truncate(title, 150)),
                )
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .children(self.profiles.iter().enumerate().map(|(index, profile)| {
                            let agent = profile.id.clone();
                            let active = self.task().is_some_and(|t| t.agent_id == agent);
                            button(("agent", index), profile.name.clone(), active).on_click(
                                cx.listener(move |this, _, _, cx| {
                                    if let Some(task) = this.selected {
                                        if this.busy.contains(&task)
                                            || this.connecting.contains(&task)
                                        {
                                            return;
                                        }
                                        let agent = agent.clone();
                                        let controller = this.controller.clone();
                                        this.job(async move {
                                            Ok(Update::AgentChanged(
                                                controller.switch_agent(task, agent).await?,
                                            ))
                                        });
                                    }
                                    cx.notify();
                                }),
                            )
                        }))
                        .child(
                            button(
                                "connect-agent",
                                if connecting {
                                    "Connecting..."
                                } else {
                                    "Connect"
                                },
                                false,
                            )
                            .on_click(cx.listener(|this, _, _, cx| this.connect("connect", cx))),
                        ),
                ),
        );
        if !configuration.options.is_empty()
            || !configuration.modes.is_empty()
            || !configuration.models.is_empty()
        {
            root = root.child(
                div()
                    .px_5()
                    .py_2()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .children(
                        configuration
                            .options
                            .iter()
                            .enumerate()
                            .map(|(index, option)| {
                                let key = option.id.clone();
                                let current = match &option.current {
                                    ConfigValue::Boolean { value } => {
                                        if *value {
                                            "On".into()
                                        } else {
                                            "Off".into()
                                        }
                                    }
                                    ConfigValue::Select { value } => option
                                        .choices
                                        .iter()
                                        .find(|c| c.value == *value)
                                        .map_or_else(|| value.clone(), |c| c.label.clone()),
                                };
                                let next = match &option.current {
                                    ConfigValue::Boolean { value } => {
                                        ConfigValue::Boolean { value: !*value }
                                    }
                                    ConfigValue::Select { value } => {
                                        let index = option
                                            .choices
                                            .iter()
                                            .position(|c| c.value == *value)
                                            .unwrap_or(0);
                                        ConfigValue::Select {
                                            value: option
                                                .choices
                                                .get((index + 1) % option.choices.len().max(1))
                                                .map_or_else(|| value.clone(), |c| c.value.clone()),
                                        }
                                    }
                                };
                                button(
                                    ("configuration", index),
                                    format!("{}: {}", option.name, current),
                                    false,
                                )
                                .on_click(cx.listener(
                                    move |this, _, _, _| {
                                        if let Some(id) = this.selected {
                                            let controller = this.controller.clone();
                                            let key = key.clone();
                                            let value = next.clone();
                                            this.job(async move {
                                                controller.set_option(id, key, value).await?;
                                                Ok(Update::Done("Session setting updated".into()))
                                            });
                                        }
                                    },
                                ))
                            }),
                    )
                    .children(configuration.modes.iter().enumerate().map(|(index, mode)| {
                        let key = mode.id.clone();
                        button(
                            ("mode", index),
                            mode.name.clone(),
                            configuration.current_mode.as_deref() == Some(&mode.id),
                        )
                        .on_click(cx.listener(move |this, _, _, _| {
                            if let Some(id) = this.selected {
                                let controller = this.controller.clone();
                                let key = key.clone();
                                this.job(async move {
                                    controller.set_mode(id, key).await?;
                                    Ok(Update::Done("Session mode updated".into()))
                                });
                            }
                        }))
                    }))
                    .children(
                        configuration
                            .models
                            .iter()
                            .filter(|_| {
                                !configuration
                                    .options
                                    .iter()
                                    .any(|o| o.category.as_deref() == Some("model"))
                            })
                            .enumerate()
                            .map(|(index, model)| {
                                let key = model.value.clone();
                                button(
                                    ("model", index),
                                    model.label.clone(),
                                    configuration.current_model.as_deref() == Some(&model.value),
                                )
                                .on_click(cx.listener(
                                    move |this, _, _, _| {
                                        if let Some(id) = this.selected {
                                            let controller = this.controller.clone();
                                            let key = key.clone();
                                            this.job(async move {
                                                controller.set_model(id, key).await?;
                                                Ok(Update::Done("Session model updated".into()))
                                            });
                                        }
                                    },
                                ))
                            }),
                    ),
            );
        }
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
        root = root.child(self.virtual_transcript(cx));
        root = root.child(
            div()
                .px_5()
                .py_3()
                .flex()
                .flex_col()
                .gap_2()
                .border_t_1()
                .border_color(rgb(0x2a3442))
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .items_center()
                        .child(div().text_xs().text_color(rgb(0x99aac0)).child(format!(
                                "{:?} · {} events{}",
                                thread.state,
                                thread.last_sequence,
                                thread
                                    .usage
                                    .context_used
                                    .map_or(String::new(), |n| format!(" · context {n}"))
                            )))
                        .children((!self.transcript.is_following()).then(|| {
                            button("jump-latest", "Jump to latest", false).on_click(cx.listener(
                                |this, _, _, cx| {
                                    this.transcript.follow();
                                    cx.notify();
                                },
                            ))
                        })),
                )
                .child(self.composer.clone())
                .children(
                    self.composer
                        .read(cx)
                        .error
                        .as_ref()
                        .map(|error| div().text_color(rgb(0xffa9ac)).child(error.clone())),
                )
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .items_center()
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x8f9caf))
                                .child("Enter to send  ·  Shift+Enter for a new line"),
                        )
                        .child(if busy {
                            button("cancel-prompt", "Stop", true)
                                .on_click(cx.listener(|this, _, _, cx| this.cancel(cx)))
                        } else {
                            button("send-prompt", "Send", true)
                                .on_click(cx.listener(|this, _, _, cx| this.send_prompt(cx)))
                        }),
                ),
        );
        let _ = id;
        root.into_any_element()
    }
    pub(super) fn transcript_item(
        &self,
        thread: &Thread,
        index: usize,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        match &thread.timeline[index] {
            TranscriptItem::Message { index: message } => {
                let message = &thread.messages[*message];
                let text = message.text.clone();
                div()
                    .id(("message", index))
                    .p_4()
                    .rounded_lg()
                    .bg(rgb(match message.role {
                        Role::User => 0x1e3044,
                        Role::Assistant => 0x19212c,
                        Role::Reasoning => 0x211f2c,
                    }))
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .items_center()
                            .mb_2()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(rgb(0xa7bada))
                                    .child(match message.role {
                                        Role::User => "YOU",
                                        Role::Assistant => "ASSISTANT",
                                        Role::Reasoning => "THINKING",
                                    }),
                            )
                            .child(button(("copy-message", index), "Copy", false).on_click(
                                move |_, _, cx| {
                                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                        text.clone(),
                                    ))
                                },
                            )),
                    )
                    .child(div().w_full().child(truncate(&message.text, 64 * 1024)))
                    .into_any_element()
            }
            TranscriptItem::Tool { id } => {
                let Some(tool) = thread.tools.get(id) else {
                    return div().into_any_element();
                };
                div()
                    .p_3()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x394252))
                    .bg(rgb(0x172029))
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .child(format!("{} · {:?}", tool.title, tool.status)),
                    )
                    .children(tool.output.iter().map(|output| {
                        match output {
                            ToolOutput::Text { text } => div()
                                .mt_2()
                                .font_family("DejaVu Sans Mono")
                                .text_xs()
                                .child(truncate(text, 16 * 1024)),
                            ToolOutput::Diff {
                                path,
                                before,
                                after,
                            } => div()
                                .mt_2()
                                .font_family("DejaVu Sans Mono")
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
                                .font_family("DejaVu Sans Mono")
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
                .p_3()
                .rounded_md()
                .bg(rgb(if *is_error { 0x3a242a } else { 0x1b2b38 }))
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
