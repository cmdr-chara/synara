use super::*;
impl Shell {
    fn enable_incoming_client(&mut self, kind: GatewayClientKind, cx: &mut Context<Self>) {
        if self.autonomy.busy || self.autonomy.name.read(cx).is_composing() {
            return;
        }
        let Some(root) = self.selected else { return };
        let text = self.autonomy.name.read(cx).text().trim();
        let name = if text.is_empty() {
            if kind == GatewayClientKind::External {
                "Reviewed local client"
            } else {
                "Selected ACP agent"
            }
            .to_owned()
        } else {
            text.to_owned()
        };
        let controller = self.controller.clone();
        self.autonomy_job(
            async move {
                controller.enable_gateway(root, kind, name).await?;
                Ok(Outcome::Changed)
            },
            cx,
        );
    }
    fn review_incoming_request(&mut self, request: GatewayRequest, cx: &mut Context<Self>) {
        if self.autonomy.busy
            || self.autonomy.request.is_some()
            || self.selected != Some(request.parent)
        {
            return;
        }
        let text = match serde_json::to_string_pretty(&request.operation) {
            Ok(v) => v,
            Err(_) => return,
        };
        self.autonomy
            .review
            .update(cx, |e, cx| e.set_text(text, cx));
        self.autonomy.request = Some(request);
        cx.notify();
    }
    fn approve_incoming_request(&mut self, cx: &mut Context<Self>) {
        if self.autonomy.busy || self.autonomy.review.read(cx).is_composing() {
            return;
        }
        let Some(request) = self.autonomy.request.clone() else {
            return;
        };
        if self.selected != Some(request.parent) {
            return;
        }
        let expected = serde_json::to_string_pretty(&request.operation).unwrap_or_default();
        if self.autonomy.review.read(cx).text() != expected {
            self.autonomy.error = Some("The displayed operation was edited. Cancel review and ask the client for a new request. Nothing was approved.".into());
            cx.notify();
            return;
        }
        let controller = self.controller.clone();
        self.autonomy.request = None;
        self.autonomy.review.update(cx, |e, cx| e.clear(cx));
        self.autonomy_job(
            async move {
                controller
                    .approve_gateway(request.parent, request.id)
                    .await?;
                Ok(Outcome::Changed)
            },
            cx,
        );
    }
    pub(in crate::shell) fn autonomy_gateway_settings(
        &self,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let Some(root) = self.selected else {
            return div()
                .child("Open a task to enroll incoming clients.")
                .into_any_element();
        };
        let mut body = div().mt_4().flex().flex_col().gap_3()
            .child(div().text_size(px(17.)).child("Agent Gateway and incoming MCP clients"))
            .child(div().text_sm().text_color(rgb(palette().muted)).child("This is inbound access TO Synara, separate from MCP servers passed to agents below. Enable grants only this task's workflow metadata and permission to propose requests. Creation, running, steering, screenshots and input still require native approval. Leases last 15 minutes, are not saved, and never auto-reconnect."))
            .child(div().relative().child(self.autonomy.name.clone()).child(ui::layout_probe("gateway-client-name")))
            .child(div().flex().flex_wrap().gap_2()
                .child(ui::action("gateway-enable-external", "Enable local external MCP client", Some(Glyph::Plugin), false, cx.listener(|this, _, _, cx| this.enable_incoming_client(GatewayClientKind::External, cx))).relative().child(ui::layout_probe("gateway-enable-external")))
                .child(ui::action("gateway-connect-agent", "Connect selected ACP agent", None, false, cx.listener(|this, _, _, cx| {
                    let Some(root) = this.selected else { return }; let controller = this.controller.clone();
                    this.autonomy_job(async move { controller.connect(root).await?; Ok(Outcome::Changed) }, cx);
                })).relative().child(ui::layout_probe("gateway-connect-agent")))
                .child(ui::action("gateway-enable-agent", "Enable negotiated Agent Gateway", None, false, cx.listener(|this, _, _, cx| this.enable_incoming_client(GatewayClientKind::Agent, cx))).relative().child(ui::layout_probe("gateway-enable-agent"))))
            .child(div().text_sm().text_color(rgb(palette().muted)).child("Agent Gateway requires an explicitly connected local ACP agent with HTTP MCP support. Enabling closes this task's current session and selects a fresh session for the next Send. The local transcript is retained. No agent is launched by external-client enrollment."));
        for (index, client) in self
            .controller
            .autonomy
            .gateway
            .clients(root)
            .into_iter()
            .enumerate()
        {
            let id = client.id;
            body = body.child(div().flex().flex_col().gap_1().relative().child(ui::layout_probe_slot("gateway-client", index))
                .child(format!("{} · {:?} · {} seconds remaining", client.name, client.kind, client.remaining_seconds))
                .child(div().text_sm().child(client.url))
                .child(div().flex().gap_2()
                    .child(ui::action(("gateway-copy", index), "Copy private client configuration", Some(Glyph::Copy), false, cx.listener(move |this, _, _, cx| {
                        match this.controller.autonomy.gateway.configuration(root, id) {
                            Ok(value) => { cx.write_to_clipboard(gpui::ClipboardItem::new_string(value.to_string())); this.autonomy.notice = Some("Private short-lived client configuration copied. Treat its bearer token as a secret. It was not saved to disk or the transcript.".into()); }
                            Err(error) => this.autonomy.error = Some(error.to_string()),
                        } cx.notify();
                    })).relative().child(ui::layout_probe_slot("gateway-copy", index)))
                    .child(ui::action(("gateway-revoke", index), "Revoke and cancel requests", Some(Glyph::Stop), false, cx.listener(move |this, _, _, cx| {
                        this.controller.autonomy.gateway.revoke(root, Some(id));
                        if this.autonomy.request.as_ref().is_some_and(|r| r.client == id) { this.autonomy.request = None; this.autonomy.review.update(cx, |e, cx| e.clear(cx)); }
                        this.autonomy.notice = Some("Client revoked. In-flight work was asked to stop. Effects already completed are retained.".into()); cx.notify();
                    })).relative().child(ui::layout_probe_slot("gateway-revoke", index)))));
        }
        if let Some(request) = &self.autonomy.request {
            body = body.child(div().p_3().border_1().border_color(rgb(palette().focus)).flex().flex_col().gap_2()
                .child(format!("Review {} from {} ({:?})", request.operation.label(), request.client_name, request.kind))
                .child("Review the complete frozen JSON. This approval is one-shot and cannot be transferred to changed arguments. Text and screenshot content are untrusted.")
                .children(self.controller.autonomy.computer.selected(root).map(|w| div().child(format!("Current computer target: {} ({})", w.title, w.identity()))))
                .child(div().relative().h(px(260.)).flex_shrink_0().flex().flex_col().child(self.autonomy.review.clone()).child(ui::layout_probe("gateway-review-editor")))
                .child(div().flex().gap_2()
                    .child(ui::action("gateway-approve", "Approve this operation once", None, false, cx.listener(|this, _, _, cx| this.approve_incoming_request(cx))).relative().child(ui::layout_probe("gateway-approve")))
                    .child(ui::action("gateway-review-cancel", "Cancel review", None, false, cx.listener(|this, _, _, cx| { this.autonomy.request = None; this.autonomy.review.update(cx, |e, cx| e.clear(cx)); cx.notify(); })).relative().child(ui::layout_probe("gateway-review-cancel")))));
        }
        for (index, request) in self
            .controller
            .autonomy
            .gateway
            .requests(root)
            .into_iter()
            .enumerate()
        {
            let pending = request.state == GatewayRequestState::Pending;
            let id = request.id;
            let clone = request.clone();
            body = body.child(
                div()
                    .py_2()
                    .border_b_1()
                    .border_color(rgb(palette().border))
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(format!(
                        "{} · {} · {:?}",
                        request.client_name,
                        request.operation.label(),
                        request.state
                    ))
                    .children(
                        request
                            .error
                            .map(|e| div().text_color(rgb(palette().error)).child(e)),
                    )
                    .children(pending.then(|| {
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                ui::action(
                                    ("gateway-review", index),
                                    "Review exact request",
                                    None,
                                    false,
                                    cx.listener(move |this, _, _, cx| {
                                        this.review_incoming_request(clone.clone(), cx)
                                    }),
                                )
                                .relative()
                                .child(ui::layout_probe_slot("gateway-review", index)),
                            )
                            .child(
                                ui::action(
                                    ("gateway-deny", index),
                                    "Deny",
                                    None,
                                    false,
                                    cx.listener(move |this, _, _, cx| {
                                        if let Err(error) =
                                            this.controller.autonomy.gateway.deny(root, id)
                                        {
                                            this.autonomy.error = Some(error.to_string());
                                        }
                                        cx.notify();
                                    }),
                                )
                                .relative()
                                .child(ui::layout_probe_slot("gateway-deny", index)),
                            )
                    })),
            );
        }
        body.into_any_element()
    }
}
