use super::*;
use synara_runtime::ComputerKey;

impl Shell {
    fn discover_computer_windows(&mut self, cx: &mut Context<Self>) {
        if self.autonomy.busy {
            return;
        }
        self.autonomy.io_cancel.cancel();
        self.autonomy.io_cancel = CancellationToken::new();
        self.autonomy.windows.clear();
        self.autonomy.computer_review = None;
        let cancel = self.autonomy.io_cancel.clone();
        self.autonomy_job(
            async move {
                let tools = ComputerTools::setup()?;
                let windows = tools.discover(&cancel).await?;
                Ok(Outcome::Windows(tools, windows))
            },
            cx,
        );
    }
    fn observe_computer(&mut self, cx: &mut Context<Self>) {
        let Some(root) = self.selected else { return };
        self.autonomy.computer_review = None;
        let owner = self.controller.autonomy.computer.clone();
        let Some(target) = owner.selection_id(root) else {
            self.autonomy.error = Some("Select a computer target first.".into());
            cx.notify();
            return;
        };
        let cancel = self.autonomy.io_cancel.clone();
        self.autonomy_job(
            async move { Ok(Outcome::Frame(owner.observe(root, target, cancel).await?)) },
            cx,
        );
    }
    fn prepare_computer_key(&mut self, key: ComputerKey, cx: &mut Context<Self>) {
        if self.autonomy.busy || self.autonomy.input.read(cx).is_composing() {
            return;
        }
        if let Ok(text) = serde_json::to_string_pretty(&ComputerAction::Key { key }) {
            self.autonomy.input.update(cx, |e, cx| e.set_text(text, cx));
            self.autonomy.computer_review = None;
            self.autonomy.error = None;
            self.autonomy.notice = Some(
                "Key input prepared, not sent. Observe, review and apply it explicitly.".into(),
            );
            cx.notify();
        }
    }
    fn review_computer_input(&mut self, cx: &mut Context<Self>) {
        if self.autonomy.busy || self.autonomy.input.read(cx).is_composing() {
            return;
        }
        let Some(root) = self.selected else { return };
        let Some(frame) = self.controller.autonomy.computer.frame(root) else {
            self.autonomy.error = Some("Observe the selected window first.".into());
            cx.notify();
            return;
        };
        let parsed = serde_json::from_str::<ComputerAction>(self.autonomy.input.read(cx).text());
        match parsed {
            Ok(action) => match self
                .controller
                .autonomy
                .computer
                .validate_action(root, frame.id, &action)
            {
                Ok(()) => self.autonomy.computer_review = Some((frame.id, action)),
                Err(error) => self.autonomy.error = Some(error.to_string()),
            },
            Err(error) => self.autonomy.error = Some(format!("Invalid input JSON: {error}")),
        }
        cx.notify();
    }
    fn apply_computer_input(&mut self, cx: &mut Context<Self>) {
        if self.autonomy.busy || self.autonomy.input.read(cx).is_composing() {
            return;
        }
        let Some(root) = self.selected else { return };
        let Some((frame, action)) = self.autonomy.computer_review.clone() else {
            return;
        };
        if serde_json::from_str::<ComputerAction>(self.autonomy.input.read(cx).text())
            .ok()
            .as_ref()
            != Some(&action)
        {
            self.autonomy.computer_review = None;
            self.autonomy.error = Some("Input changed after review. Review again.".into());
            cx.notify();
            return;
        }
        self.autonomy.computer_review = None;
        let controller = self.controller.clone();
        let cancel = self.autonomy.io_cancel.clone();
        self.autonomy_job(async move {
            let task = controller.workspace.task(root).await?;
            controller.workspace.record(task.thread_id, ThreadEvent::Notice { message: format!("Native window-input approval for frame {frame}. One-shot target input, no automatic retry.") }).await?;
            controller.autonomy.computer.act(root, frame, action, cancel).await?;
            controller.workspace.record(task.thread_id, ThreadEvent::Notice { message: "Window input delivery returned. Application success is not verified. Observe the window again before another action.".into() }).await?;
            Ok(Outcome::Changed)
        }, cx);
    }
    fn takeover_computer(&mut self, cx: &mut Context<Self>) {
        if let Some(root) = self.selected {
            self.controller.autonomy.computer.revoke(root);
        }
        self.autonomy.io_cancel.cancel();
        self.autonomy.io_cancel = CancellationToken::new();
        self.autonomy.preview = None;
        self.autonomy.computer_review = None;
        self.autonomy.windows.clear();
        self.autonomy.tools = None;
        self.autonomy.notice = Some("Control revoked. No input or observation is retried. Events already delivered cannot be undone. Discover and select again to re-enable.".into());
        cx.notify();
    }
    pub(in crate::shell) fn autonomy_computer_settings(
        &self,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let Some(root) = self.selected else {
            return div()
                .child("Open a chat to own Computer Use observations and approvals.")
                .into_any_element();
        };
        let mut body = self.autonomy_preamble().gap_3()
            .child("Computer Use · explicit single-window control")
            .child(div().text_sm().text_color(rgb(palette().muted)).child("Linux/X11 only. Requires installed x11-utils, ImageMagick and xdotool. Synara's own approval window, root desktop and dock windows are excluded. Every input needs a fresh observed frame, exact target revalidation and a separate approval. Selection expires after 15 minutes. No control permission survives restart."))
            .child(div().flex().flex_wrap().gap_2()
                .child(ui::action("computer-discover", "Discover application windows", Some(Glyph::Window), false, cx.listener(|this, _, _, cx| this.discover_computer_windows(cx))).relative().child(ui::layout_probe("computer-discover")))
                .child(ui::action("computer-observe", "Observe selected window", Some(Glyph::Capture), false, cx.listener(|this, _, _, cx| this.observe_computer(cx))).relative().child(ui::layout_probe("computer-observe")))
                .child(ui::action("computer-takeover", "Take over / revoke control", Some(Glyph::Stop), false, cx.listener(|this, _, _, cx| this.takeover_computer(cx))).relative().child(ui::layout_probe("computer-takeover"))));
        for (index, window) in self.autonomy.windows.iter().enumerate() {
            let window = window.clone();
            let label = format!("Select {} · {}", window.title, window.identity());
            body = body.child(ui::action(("computer-select", index), label, None, false, cx.listener(move |this, _, _, cx| {
                if this.autonomy.busy || this.selected != Some(root) { return; }
                let Some(tools) = this.autonomy.tools.clone() else { return };
                match this.controller.autonomy.computer.select(root, tools, window.clone()) {
                    Ok(()) => { this.autonomy.preview = None; this.autonomy.computer_review = None; this.autonomy.notice = Some("Window selected without capture or input. Observe and review before acting.".into()); }
                    Err(error) => this.autonomy.error = Some(error.to_string()),
                } cx.notify();
            })).relative().child(ui::layout_probe_slot("computer-select", index)));
        }
        if let Some(window) = self.controller.autonomy.computer.selected(root) {
            body = body.child(
                div()
                    .relative()
                    .child(ui::layout_probe("computer-selected"))
                    .child(format!(
                        "Selected target: {} · {}",
                        window.title,
                        window.identity()
                    )),
            );
        }
        if let Some((id, image)) = &self.autonomy.preview {
            body = body.child(div().relative().flex_shrink_0().child(ui::layout_probe("computer-preview"))
                .child(gpui::img(image.clone()).w_full().h(px(240.)).object_fit(gpui::ObjectFit::Contain)))
                .child(div().text_sm().child(format!("Frame {id}. Input lease: 60 seconds, one action. Preview is untrusted application content.")));
        }
        body = body.child(div().text_sm().child("Input examples: {\"action\":\"type\",\"text\":\"hello\"}, {\"action\":\"key\",\"key\":\"enter\"}, {\"action\":\"click\",\"x\":20,\"y\":40,\"button\":\"left\"}. Scroll uses x, y, down and steps 1-8. Typing accepts up to 512 UTF-8 bytes, including Unicode, but rejects control characters, line separators and bidirectional formatting characters. Use separate reviewed keys for Enter and Tab. X11 keymap or application support can vary; delivery does not prove the text was accepted. Keys include insert, f1-f12, back_tab, shift_enter and named editing combinations. No arbitrary key sequences, clipboard operations or Alt/Meta shortcuts."));
        let mut presets = div().flex().flex_wrap().gap_2();
        for (index, (label, key)) in [
            ("Enter", ComputerKey::Enter),
            ("Tab", ComputerKey::Tab),
            ("Back tab", ComputerKey::BackTab),
            ("Select all", ComputerKey::SelectAll),
            ("Undo", ComputerKey::Undo),
            ("Redo", ComputerKey::Redo),
            ("Previous word", ComputerKey::WordLeft),
            ("Next word", ComputerKey::WordRight),
            ("Select previous word", ComputerKey::SelectWordLeft),
            ("Select next word", ComputerKey::SelectWordRight),
        ]
        .into_iter()
        .enumerate()
        {
            presets = presets.child(ui::action(
                ("computer-prepare-key", index),
                label,
                None,
                false,
                cx.listener(move |this, _, _, cx| {
                    if this.selected == Some(root) {
                        this.prepare_computer_key(key, cx);
                    }
                }),
            ));
        }
        body = body
            .child(div().text_sm().child(
                "Prepare a key action below, then review and apply. Buttons do not send input.",
            ))
            .child(presets)
            .child(
                div()
                    .relative()
                    .h(px(130.))
                    .flex_shrink_0()
                    .flex()
                    .flex_col()
                    .child(self.autonomy.input.clone())
                    .child(ui::layout_probe("computer-input-editor")),
            )
            .child(
                ui::action(
                    "computer-review-input",
                    "Review input against this frame",
                    None,
                    false,
                    cx.listener(|this, _, _, cx| this.review_computer_input(cx)),
                )
                .relative()
                .child(ui::layout_probe("computer-review-input")),
            );
        if let Some((frame, action)) = &self.autonomy.computer_review {
            body = body.child(div().p_3().border_1().border_color(rgb(palette().focus)).flex().flex_col().gap_2()
                .child(format!("Approve frame {frame}: {}", serde_json::to_string(action).unwrap_or_default()))
                .child("Click and scroll move the shared pointer to the reviewed window coordinates. Input is explicitly window-addressed. Some applications reject synthetic window events or use different editing shortcuts. Delivery does not prove success. A changing screen is refused rather than acted on using stale coordinates.")
                .child(div().flex().gap_2()
                    .child(ui::action("computer-confirm-input", "Apply this input once", None, false, cx.listener(|this, _, _, cx| this.apply_computer_input(cx))).relative().child(ui::layout_probe("computer-confirm-input")))
                    .child(ui::action("computer-cancel-input", "Cancel review", None, false, cx.listener(|this, _, _, cx| { this.autonomy.computer_review = None; cx.notify(); })).relative().child(ui::layout_probe("computer-cancel-input")))));
        }
        body.child(self.autonomy_gateway_settings(cx))
            .into_any_element()
    }
}
