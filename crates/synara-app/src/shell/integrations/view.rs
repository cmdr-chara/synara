use super::super::settings::Section;
use super::*;

fn note(text: impl Into<SharedString>) -> gpui::Div {
    div()
        .text_sm()
        .text_color(rgb(palette().muted))
        .child(text.into())
}
fn section(title: &str) -> gpui::Div {
    div()
        .mt_4()
        .mb_2()
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .child(title.to_owned())
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
    pub(in crate::shell) fn integration_settings(
        &self,
        page: Section,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let state = &self.integrations;
        let toolbar = div()
            .flex()
            .gap_2()
            .items_center()
            .child(
                div()
                    .flex_1()
                    .relative()
                    .child(state.query.clone())
                    .child(ui::layout_probe("integrations-search")),
            )
            .child(
                ui::button("integrations-reload", "Reload", false)
                    .relative()
                    .child(ui::layout_probe("integrations-reload"))
                    .on_click(cx.listener(|this, _, _, cx| this.load_integrations(cx))),
            );
        let filters = div().flex().gap_2().children(
            [
                (None, "All"),
                (Some(true), "Enabled"),
                (Some(false), "Disabled"),
            ]
            .into_iter()
            .enumerate()
            .map(|(i, (filter, label))| {
                ui::button(("integration-filter", i), label, state.filter == filter).on_click(
                    cx.listener(move |this, _, _, cx| {
                        this.integrations.filter = filter;
                        cx.notify();
                    }),
                )
            }),
        );
        let mut body = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(toolbar)
            .child(filters)
            .children(
                state
                    .error
                    .as_ref()
                    .map(|error| div().text_color(rgb(palette().error)).child(error.clone())),
            )
            .children(state.notice.as_ref().map(|text| note(text.clone())))
            .children(state.busy.then(|| {
                note("Working on the requested operation... No other integration action is queued.")
            }));
        if let Some(confirm) = &state.confirm {
            let message = match confirm {
                Confirm::Skill(_) => {
                    "Remove this document from Synara's library? The original file, provider skills and previously inserted drafts will not be changed."
                }
                Confirm::Mcp(_) => {
                    "Remove this task-scoped connection? Synara will stop sharing it with new sessions after the old session is retired. Revoke the external token separately at its provider. The credential-store entry is not deleted."
                }
                Confirm::Disconnect(_) => {
                    "Disconnect this agent process? Other tasks may share it. Active prompts must finish first. No tasks will restart automatically and no provider credential will be revoked."
                }
            };
            body = body.child(
                row().child(note(message)).child(
                    div()
                        .flex()
                        .gap_2()
                        .child(
                            ui::button("integration-confirm", "Confirm", false)
                                .relative()
                                .child(ui::layout_probe("integration-confirm"))
                                .on_click(
                                    cx.listener(|this, _, _, cx| this.confirm_integration(cx)),
                                ),
                        )
                        .child(
                            ui::button("integration-cancel-confirm", "Cancel", false)
                                .relative()
                                .child(ui::layout_probe("integration-cancel-confirm"))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if !this.integrations.busy {
                                        this.integrations.confirm = None;
                                    }
                                    cx.notify();
                                })),
                        ),
                ),
            );
        }
        let Some(value) = &state.value else {
            return body.child(note("Integration settings are not loaded. Reload to read them. No repository configuration is imported automatically.")).into_any_element();
        };
        match page {
            Section::Plugins => {
                body=body.child(section("Built into Synara"))
                    .child(note("These management surfaces ship with Synara. They are not downloaded provider plugins."));
                for (name, description, target) in [
                    (
                        "MCP connections",
                        "Explicit HTTP discovery, task/agent consent and credential references.",
                        Section::Mcp,
                    ),
                    (
                        "Skill documents",
                        "Reviewed Markdown imports with origin receipts. Explicit draft insertion only.",
                        Section::Skills,
                    ),
                ] {
                    if state.matches(true, &[name, description], cx) {
                        body = body.child(
                            row()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .child(name)
                                        .child(
                                            ui::button(
                                                SharedString::from(format!("manage-{name}")),
                                                "Manage",
                                                false,
                                            )
                                            .on_click(
                                                cx.listener(move |this, _, _, cx| {
                                                    this.open_settings_section(target, cx)
                                                }),
                                            ),
                                        ),
                                )
                                .child(note(format!("Synara-owned | Available | {description}"))),
                        );
                    }
                }
                body = body.child(section("Synara-managed inventory"));
                let mut count = 0;
                for item in &value.mcp {
                    if state.matches(
                        item.enabled,
                        &[&item.name, &item.endpoint, &item.agent_id],
                        cx,
                    ) {
                        count += 1;
                        body = body.child(row().child(item.name.clone()).child(note(format!(
                            "MCP configuration | {} | Task {} | Agent {}",
                            if item.enabled {
                                "Enabled for future sessions"
                            } else {
                                "Disabled"
                            },
                            item.task,
                            item.agent_id
                        ))));
                    }
                }
                for item in &value.skills {
                    if state.matches(
                        item.enabled,
                        &[&item.title, &item.description, &item.origin.path],
                        cx,
                    ) {
                        count += 1;
                        body = body.child(row().child(item.title.clone()).child(note(format!(
                            "Skill document | Installed in Synara | {}",
                            if item.enabled {
                                "Draft insertion enabled"
                            } else {
                                "Disabled"
                            }
                        ))));
                    }
                }
                if count == 0 {
                    body = body.child(note("No managed items match this filter."));
                }
                body=body.child(section("External ownership"))
                    .child(note("Provider-owned extensions are installed and controlled by that provider. The current generic agent contract does not expose a plugin catalog or lifecycle API. Synara cannot claim their installed state or offer install, enable, revoke or remove switches."))
                    .child(ui::button("integration-agent-profiles","Open agent profiles",false).relative().child(ui::layout_probe("integration-agent-profiles" )).on_click(cx.listener(|this,_,_,cx|this.open_settings_section(Section::Providers,cx))));
                body = body.child(section("Reported by the selected agent"));
                if let Some(details) = &self.details {
                    body=body.child(note(format!("Connection: {:?}. Negotiated HTTP MCP configuration: {}. Negotiated SSE configuration: {}. These are protocol capabilities, not installed plugins or a successful MCP connection.",details.connection.state,details.connection.capabilities.mcp_http,details.connection.capabilities.mcp_sse)));
                } else {
                    body=body.child(note("No current negotiation evidence. Opening this page does not connect an agent."));
                }
            }
            Section::Skills => {
                body=body.child(note("Synara-owned document library. Enable only permits explicit insertion into a visible draft. Native provider skill compatibility is unknown and is never inferred from an agent name."))
                    .child(ui::button("skill-import","Review local skill document",false).relative().child(ui::layout_probe("skill-import" )).on_click(cx.listener(|this,_,_,cx|this.choose_skill(None,cx))))
                    .child(div().flex().gap_2()
                        .child(div().flex_1().relative().child(state.skill_path.clone()).child(ui::layout_probe("skill-path")))
                        .child(ui::button("skill-review-path","Review path",false).relative().child(ui::layout_probe("skill-review-path" )).on_click(cx.listener(|this,_,_,cx|this.review_skill_path(cx)))));
                if let Some((review, _)) = &state.review {
                    body=body.child(section("Review before installation"))
                        .child(self.skill_preview(review.document()))
                        .child(note("Only this Markdown document will be stored, as plaintext instructions. It must not contain secrets. Bundled scripts/assets are not installed or executed. Review all instructions before approval."))
                        .child(div().flex().gap_2()
                            .child(ui::button("skill-approve","Install document disabled",false).relative().child(ui::layout_probe("skill-approve" )).on_click(cx.listener(|this,_,_,cx|this.approve_skill(cx))))
                            .child(ui::button("skill-cancel","Cancel review",false).relative().child(ui::layout_probe("skill-cancel" )).on_click(cx.listener(|this,_,_,cx|{if !this.integrations.busy{this.integrations.review=None;}cx.notify();}))));
                }
                if let Some(preview) = &state.preview {
                    body = body.child(self.skill_preview(preview)).child(
                        ui::button("skill-close-preview", "Close preview", false)
                            .relative()
                            .child(ui::layout_probe("skill-close-preview"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.integrations.preview = None;
                                cx.notify();
                            })),
                    );
                }
                body = body.child(section("Installed documents"));
                let mut count = 0;
                for item in &value.skills {
                    if !state.matches(
                        item.enabled,
                        &[&item.title, &item.description, &item.origin.path],
                        cx,
                    ) {
                        continue;
                    }
                    count += 1;
                    let preview = item.clone();
                    let toggle = item.id.clone();
                    let remove = item.id.clone();
                    let update = item.id.clone();
                    let insert = item.id.clone();
                    let enabled = item.enabled;
                    let mut actions = div()
                        .flex()
                        .flex_wrap()
                        .gap_2()
                        .child(
                            ui::button(("skill-preview", count), "Review", false)
                                .relative()
                                .child(ui::layout_probe_slot("skill-preview", count))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.integrations.preview = Some(preview.clone());
                                    cx.notify();
                                })),
                        )
                        .child(
                            ui::button(
                                ("skill-toggle", count),
                                if enabled {
                                    "Disable"
                                } else {
                                    "Enable for drafts"
                                },
                                false,
                            )
                            .relative()
                            .child(ui::layout_probe_slot("skill-toggle", count))
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.change_skill(
                                        SkillEdit::SetEnabled {
                                            id: toggle.clone(),
                                            enabled: !enabled,
                                        },
                                        cx,
                                    )
                                },
                            )),
                        )
                        .child(
                            ui::button(("skill-update", count), "Review update", false)
                                .relative()
                                .child(ui::layout_probe_slot("skill-update", count))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.choose_skill(Some(update.clone()), cx)
                                })),
                        )
                        .child(
                            ui::button(("skill-remove", count), "Remove", false)
                                .relative()
                                .child(ui::layout_probe_slot("skill-remove", count))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    if !this.integrations.busy {
                                        this.integrations.confirm =
                                            Some(Confirm::Skill(remove.clone()));
                                    }
                                    cx.notify();
                                })),
                        );
                    if enabled && self.selected.is_some() {
                        actions = actions.child(
                            ui::button(("skill-insert", count), "Insert into draft", false)
                                .relative()
                                .child(ui::layout_probe_slot("skill-insert", count))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.insert_skill(insert.clone(), cx)
                                })),
                        );
                    }
                    body = body.child(
                        row()
                            .child(item.title.clone())
                            .child(note(item.description.clone()))
                            .child(note(format!(
                                "Synara-owned | {} | Version {} | {}",
                                if enabled {
                                    "Enabled for drafts"
                                } else {
                                    "Disabled"
                                },
                                item.origin.version.as_deref().unwrap_or("not declared"),
                                item.origin.path
                            )))
                            .child(note(format!("SHA-256 {}", item.origin.sha256)))
                            .child(actions),
                    );
                }
                if count == 0 {
                    body = body.child(note("No installed skill documents match this filter."));
                }
                body=body.child(section("Available sources"))
                    .child(note("Choose a local Markdown document to discover and review it. No authoritative cross-agent remote skill catalog is configured. Agent-owned skills remain in their provider's tooling. Synara does not install package managers or execute repository installers."));
            }
            Section::Mcp => {
                body=body.child(note("Synara-managed Streamable HTTP connections. Every row belongs to one task and one exact agent profile, not all tasks, projects or Hubs. Editing disables it. Enabling authorizes sharing with that task's next agent session, subject to negotiated support."))
                    .child(note(format!("Credential store: {:?}. Enter references only, never plaintext tokens. OAuth pairing, managed process/SSE transports and SSH-scoped MCP are not implemented.",self.controller.integration_secret_state())))
                    .child(ui::button("mcp-add","Add HTTP connection",false).relative().child(ui::layout_probe("mcp-add" )).on_click(cx.listener(|this,_,_,cx|this.edit_mcp_form(None,cx))));
                if let Some(form) = &state.form {
                    body=body.child(section("Connection configuration"))
                        .child(note(format!("Scope: task {} | agent {}. This scope cannot change during editing.",form.config.task,form.config.agent_id)))
                        .child(note("Name")).child(div().relative().child(form.name.clone()).child(ui::layout_probe("mcp-name-input")))
                        .child(note("Endpoint. HTTPS or literal loopback HTTP. No URL credentials, queries or fragments.")).child(div().relative().child(form.endpoint.clone()).child(ui::layout_probe("mcp-endpoint-input")))
                        .child(note("Optional bearer credential reference: service and account. The token must already exist in the OS secret store."))
                        .child(div().relative().child(form.service.clone()).child(ui::layout_probe("mcp-service-input"))).child(div().relative().child(form.account.clone()).child(ui::layout_probe("mcp-account-input")))
                        .child(div().flex().gap_2()
                            .child(ui::button("mcp-save","Save disabled",false).relative().child(ui::layout_probe("mcp-save" )).on_click(cx.listener(|this,_,_,cx|this.save_mcp_form(cx))))
                            .child(ui::button("mcp-cancel","Discard form",false).relative().child(ui::layout_probe("mcp-cancel" )).on_click(cx.listener(|this,_,_,cx|{if !this.integrations.busy{this.integrations.form=None;}cx.notify();}))));
                }
                let mut count = 0;
                for item in &value.mcp {
                    if !state.matches(
                        item.enabled,
                        &[&item.name, &item.endpoint, &item.agent_id],
                        cx,
                    ) {
                        continue;
                    }
                    count += 1;
                    let edit = item.clone();
                    let toggle = item.id.clone();
                    let remove = item.id.clone();
                    let test = item.id.clone();
                    let enabled = item.enabled;
                    let task = item.task;
                    let known_unsupported = self.selected == Some(item.task)
                        && self
                            .task()
                            .is_some_and(|task| task.agent_id == item.agent_id)
                        && self
                            .details
                            .as_ref()
                            .is_some_and(|d| !d.connection.capabilities.mcp_http);
                    let mut actions = div()
                        .flex()
                        .flex_wrap()
                        .gap_2()
                        .child(
                            ui::button(("mcp-edit", count), "Edit", false)
                                .relative()
                                .child(ui::layout_probe_slot("mcp-edit", count))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.edit_mcp_form(Some(edit.clone()), cx)
                                })),
                        )
                        .child(
                            ui::button(("mcp-test", count), "Test and discover", false)
                                .relative()
                                .child(ui::layout_probe_slot("mcp-test", count))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.test_mcp_row(test.clone(), cx)
                                })),
                        )
                        .child(
                            ui::button(("mcp-remove", count), "Remove", false)
                                .relative()
                                .child(ui::layout_probe_slot("mcp-remove", count))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    if !this.integrations.busy {
                                        this.integrations.confirm =
                                            Some(Confirm::Mcp(remove.clone()));
                                    }
                                    cx.notify();
                                })),
                        )
                        .child(
                            ui::button(("mcp-disconnect", count), "Disconnect agent...", false)
                                .relative()
                                .child(ui::layout_probe_slot("mcp-disconnect", count))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    if !this.integrations.busy {
                                        this.integrations.confirm = Some(Confirm::Disconnect(task));
                                    }
                                    cx.notify();
                                })),
                        );
                    if enabled || !known_unsupported {
                        actions = actions.child(
                            ui::button(
                                ("mcp-enable", count),
                                if enabled {
                                    "Disable"
                                } else {
                                    "Enable for this scope"
                                },
                                false,
                            )
                            .relative()
                            .child(ui::layout_probe_slot("mcp-enable", count))
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.change_mcp(
                                        McpEdit::SetEnabled {
                                            id: toggle.clone(),
                                            enabled: !enabled,
                                        },
                                        cx,
                                    )
                                },
                            )),
                        );
                    }
                    let mut entry=row().child(item.name.clone()).child(note(item.endpoint.clone()))
                        .child(note(format!("Synara-managed | {} | Task {} | Agent {}",if enabled{"Enabled, not proof of connection"}else{"Disabled"},item.task,item.agent_id)))
                        .child(note(item.bearer.as_ref().map_or_else(||"Authentication: no bearer credential configured".into(),|r|format!("Bearer reference: {} / {}",r.service,r.account))))
                        .children(known_unsupported.then(||note("The selected agent does not advertise HTTP MCP configuration support.")))
                        .child(actions);
                    if let Some((_, report)) = state.reports.get(&item.id) {
                        entry=entry.child(match report {
                            Err(error)=>div().relative().child(ui::layout_probe("mcp-probe-error")).text_color(rgb(palette().error)).child(error.clone()).into_any_element(),
                            Ok(report)=>{
                                let expanded=state.expanded.as_ref()==Some(&item.id);let expand=item.id.clone();
                                div().flex().flex_col().gap_1().relative().child(ui::layout_probe("mcp-probe-success"))
                                    .child(note(format!("Last explicit probe succeeded from this app in {} ms. Protocol {}. {} tools. Agent reachability is not established.",report.elapsed_ms,report.protocol,report.tools.len())))
                                    .child(note(format!("Self-reported server: {} | Capabilities: {}",if report.server.is_empty(){"not reported"}else{&report.server},report.capabilities.join(", "))))
                                    .children(report.cleanup_warning.then(||note("The server did not confirm deletion of the temporary legacy probe session. No persistent client session is retained.")))
                                    .child(ui::button(("mcp-tools",count),if expanded{"Hide tools"}else{"Show tools"},false).relative().child(ui::layout_probe_slot("mcp-tools",count)).on_click(cx.listener(move |this,_,_,cx|{this.integrations.expanded=if this.integrations.expanded.as_ref()==Some(&expand){None}else{Some(expand.clone())};cx.notify();})))
                                    .children(expanded.then(||div().id(("mcp-tool-list",count)).max_h(px(240.)).overflow_y_scroll().children(report.tools.iter().map(|tool|note(format!("{}: {}",tool.name,tool.description))))))
                                    .into_any_element()
                            },
                        });
                    } else {
                        entry=entry.child(note("Not tested in this view. Saving and enabling do not prove a connection."));
                    }
                    body = body.child(entry);
                }
                if count == 0 {
                    body = body.child(note("No configured MCP connections match this filter."));
                }
                body=body.child(section("Agent-managed MCP"))
                    .child(note("An external agent may own other MCP configuration. ACP HTTP/SSE capability flags do not enumerate those connections. Synara neither edits their files nor claims to test or revoke them."));
            }
            _ => {}
        }
        body.into_any_element()
    }
    fn skill_preview(&self, skill: &InstalledSkill) -> gpui::AnyElement {
        row().child(skill.title.clone()).child(note(format!("Origin: {} | SHA-256 {}",skill.origin.path,skill.origin.sha256)))
            .child(note(format!("Retained update receipts: {}. Publisher identity and provider compatibility are not verified.",skill.previous_origins.len())))
            .child(div().id("skill-document-preview").max_h(px(320.)).overflow_y_scroll().text_sm().child(skill.markdown.clone()))
            .into_any_element()
    }
}
