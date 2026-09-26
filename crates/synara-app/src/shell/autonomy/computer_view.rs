use super::*;
use std::{cell::Cell, rc::Rc};
use synara_runtime::{ComputerButton, ComputerKey, computer_preview_point};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum PointerAction {
    #[default]
    Click,
    RightClick,
    MiddleClick,
    DoubleClick,
    Move,
    Drag,
    ScrollUp,
    ScrollDown,
    ScrollLeft,
    ScrollRight,
}

pub(super) struct ComputerUi {
    bounds: Rc<Cell<gpui::Bounds<gpui::Pixels>>>,
    focus: gpui::FocusHandle,
    filter: Entity<TextEntry>,
    text: Entity<TextEntry>,
    mode: PointerAction,
    steps: u8,
    point: Option<(Uuid, u32, u32)>,
    drag_start: Option<(Uuid, u32, u32)>,
    expanded: bool,
    _subscriptions: Vec<Subscription>,
}

impl ComputerUi {
    pub(super) fn new(cx: &mut Context<Shell>) -> Self {
        let filter = cx.new(|cx| {
            TextEntry::new(
                "Filter by window title, application or PID",
                EntryMode::SingleLine,
                32.,
                cx,
            )
        });
        let text = cx.new(|cx| {
            TextEntry::new(
                "Text to prepare for the selected window",
                EntryMode::SingleLine,
                32.,
                cx,
            )
        });
        let subscriptions = [&filter, &text]
            .into_iter()
            .map(|entry| cx.subscribe(entry, |_, _, _, cx| cx.notify()))
            .collect();
        Self {
            bounds: Rc::new(Cell::new(gpui::Bounds::default())),
            focus: cx.focus_handle(),
            filter,
            text,
            mode: PointerAction::default(),
            steps: 3,
            point: None,
            drag_start: None,
            expanded: false,
            _subscriptions: subscriptions,
        }
    }

    fn clear_points(&mut self) {
        self.point = None;
        self.drag_start = None;
    }

    pub(super) fn reset(&mut self, cx: &mut Context<Shell>) {
        self.clear_points();
        self.mode = PointerAction::default();
        self.filter.update(cx, |entry, cx| entry.clear(cx));
        self.text.update(cx, |entry, cx| entry.clear(cx));
    }
}

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
        if self.autonomy.busy {
            return;
        }
        let Some(root) = self.selected else { return };
        self.autonomy.computer_review = None;
        self.autonomy.computer_ui.clear_points();
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
        self.prepare_computer_action(ComputerAction::Key { key }, cx);
    }

    fn prepare_computer_action(&mut self, action: ComputerAction, cx: &mut Context<Self>) {
        if self.autonomy.busy || self.autonomy.input.read(cx).is_composing() {
            return;
        }
        if matches!(
            action,
            ComputerAction::Key { .. } | ComputerAction::Type { .. }
        ) {
            self.autonomy.computer_ui.clear_points();
        }
        if let Ok(text) = serde_json::to_string_pretty(&action) {
            self.autonomy.input.update(cx, |e, cx| e.set_text(text, cx));
            self.autonomy.computer_review = None;
            self.autonomy.error = None;
            self.autonomy.notice =
                Some("Input prepared. Review the action and apply it explicitly.".into());
            cx.notify();
        }
    }

    fn pick_computer_point(
        &mut self,
        frame_id: Uuid,
        position: gpui::Point<gpui::Pixels>,
        cx: &mut Context<Self>,
    ) {
        if self.autonomy.busy || self.autonomy.input.read(cx).is_composing() {
            return;
        }
        let Some(root) = self.selected else { return };
        let Some(frame) = self
            .controller
            .autonomy
            .computer
            .frame(root)
            .filter(|frame| {
                frame.id == frame_id && frame.captured.elapsed() < Duration::from_secs(60)
            })
        else {
            self.autonomy.error = Some("Observe the window again before choosing a point.".into());
            self.autonomy.computer_review = None;
            self.autonomy.computer_ui.clear_points();
            cx.notify();
            return;
        };
        let bounds = self.autonomy.computer_ui.bounds.get();
        let Some((x, y)) = computer_preview_point(
            f32::from(position.x - bounds.origin.x),
            f32::from(position.y - bounds.origin.y),
            f32::from(bounds.size.width),
            f32::from(bounds.size.height),
            frame.window.width,
            frame.window.height,
        ) else {
            return;
        };
        self.autonomy.computer_ui.point = Some((frame_id, x, y));
        self.autonomy.computer_review = None;
        let steps = self.autonomy.computer_ui.steps;
        let action = match self.autonomy.computer_ui.mode {
            PointerAction::Click => ComputerAction::Click {
                x,
                y,
                button: ComputerButton::Left,
            },
            PointerAction::RightClick => ComputerAction::Click {
                x,
                y,
                button: ComputerButton::Right,
            },
            PointerAction::MiddleClick => ComputerAction::Click {
                x,
                y,
                button: ComputerButton::Middle,
            },
            PointerAction::DoubleClick => ComputerAction::DoubleClick {
                x,
                y,
                button: ComputerButton::Left,
            },
            PointerAction::Move => ComputerAction::Move { x, y },
            PointerAction::ScrollUp => ComputerAction::Scroll {
                x,
                y,
                down: false,
                steps,
            },
            PointerAction::ScrollDown => ComputerAction::Scroll {
                x,
                y,
                down: true,
                steps,
            },
            PointerAction::ScrollLeft => ComputerAction::HorizontalScroll {
                x,
                y,
                right: false,
                steps,
            },
            PointerAction::ScrollRight => ComputerAction::HorizontalScroll {
                x,
                y,
                right: true,
                steps,
            },
            PointerAction::Drag => {
                if let Some((start_frame, from_x, from_y)) = self
                    .autonomy
                    .computer_ui
                    .drag_start
                    .take()
                    .filter(|(id, _, _)| *id == frame_id)
                {
                    debug_assert_eq!(start_frame, frame_id);
                    ComputerAction::Drag {
                        from_x,
                        from_y,
                        to_x: x,
                        to_y: y,
                        button: ComputerButton::Left,
                    }
                } else {
                    self.autonomy.computer_ui.drag_start = Some((frame_id, x, y));
                    self.autonomy.input.update(cx, |entry, cx| entry.clear(cx));
                    self.autonomy.notice = Some(format!(
                        "Drag starts at ({x}, {y}). Choose its endpoint in this same frame."
                    ));
                    self.autonomy.error = None;
                    cx.notify();
                    return;
                }
            }
        };
        self.prepare_computer_action(action, cx);
    }

    fn change_computer_scroll_steps(&mut self, increase: bool, cx: &mut Context<Self>) {
        if self.autonomy.busy || self.autonomy.input.read(cx).is_composing() {
            return;
        }
        let current = self.autonomy.computer_ui.steps;
        let steps = if increase {
            (current + 1).min(8)
        } else {
            current.saturating_sub(1).max(1)
        };
        self.autonomy.computer_ui.steps = steps;
        if let Ok(mut action) =
            serde_json::from_str::<ComputerAction>(self.autonomy.input.read(cx).text())
        {
            match &mut action {
                ComputerAction::Scroll { steps: count, .. }
                | ComputerAction::HorizontalScroll { steps: count, .. } => {
                    *count = steps;
                    self.prepare_computer_action(action, cx);
                }
                _ => {}
            }
        }
        cx.notify();
    }
    fn review_computer_input(&mut self, cx: &mut Context<Self>) {
        if self.autonomy.busy || self.autonomy.input.read(cx).is_composing() {
            return;
        }
        let Some(root) = self.selected else { return };
        self.autonomy.computer_review = None;
        self.autonomy.error = None;
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
        self.autonomy.computer_ui.clear_points();
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
        self.autonomy.computer_ui.clear_points();
        self.autonomy.windows.clear();
        self.autonomy.tools = None;
        self.autonomy.notice = Some("Control revoked. No input or observation is retried. Events already delivered cannot be undone. Discover and select again to re-enable.".into());
        cx.notify();
    }
    fn computer_preview_controls(
        &self,
        root: TaskId,
        frame: &ComputerFrame,
        image: Arc<gpui::Image>,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let frame_id = frame.id;
        let width = frame.window.width;
        let height = frame.window.height;
        let mut modes = div().flex().flex_wrap().gap_2();
        for (index, (label, mode)) in [
            ("Left click", PointerAction::Click),
            ("Right click", PointerAction::RightClick),
            ("Middle click", PointerAction::MiddleClick),
            ("Double-click", PointerAction::DoubleClick),
            ("Move", PointerAction::Move),
            ("Drag", PointerAction::Drag),
            ("Scroll up", PointerAction::ScrollUp),
            ("Scroll down", PointerAction::ScrollDown),
            ("Scroll left", PointerAction::ScrollLeft),
            ("Scroll right", PointerAction::ScrollRight),
        ]
        .into_iter()
        .enumerate()
        {
            modes = modes.child(ui::action(
                ("computer-pointer-mode", index),
                label,
                None,
                self.autonomy.computer_ui.mode == mode,
                cx.listener(move |this, _, _, cx| {
                    if this.selected != Some(root) || this.autonomy.busy {
                        return;
                    }
                    this.autonomy.computer_ui.mode = mode;
                    this.autonomy.computer_ui.clear_points();
                    this.autonomy.computer_review = None;
                    this.autonomy.notice =
                        Some("Choose a point in the screenshot to prepare this action.".into());
                    cx.notify();
                }),
            ));
        }
        let bounds = self.autonomy.computer_ui.bounds.clone();
        let mut markers: Vec<_> = [
            self.autonomy.computer_ui.point,
            self.autonomy.computer_ui.drag_start,
        ]
        .into_iter()
        .flatten()
        .filter(|(id, _, _)| *id == frame_id)
        .collect();
        if self
            .autonomy
            .computer_ui
            .point
            .is_some_and(|(id, _, _)| id == frame_id)
            && let Ok(ComputerAction::Drag {
                from_x,
                from_y,
                to_x,
                to_y,
                ..
            }) = serde_json::from_str::<ComputerAction>(self.autonomy.input.read(cx).text())
            && from_x < width
            && to_x < width
            && from_y < height
            && to_y < height
        {
            markers = vec![(frame_id, from_x, from_y), (frame_id, to_x, to_y)];
        }
        let viewer = div().id("computer-preview-target")
            .role(gpui::Role::Group)
            .aria_label("Observed window. Select an action and click to prepare its coordinates. Escape revokes control.")
            .tab_index(0).track_focus(&self.autonomy.computer_ui.focus)
            .relative().w_full().h(px(if self.autonomy.computer_ui.expanded { 520. } else { 280. }))
            .flex_shrink_0().overflow_hidden().cursor_pointer()
            .child(gpui::img(image).size_full().object_fit(gpui::ObjectFit::Contain))
            .child(gpui::canvas(move |area, _, _| bounds.set(area), move |area, _, window, _| {
                let scale = (f32::from(area.size.width) / width as f32)
                    .min(f32::from(area.size.height) / height as f32);
                let left = (f32::from(area.size.width) - width as f32 * scale) / 2.;
                let top = (f32::from(area.size.height) - height as f32 * scale) / 2.;
                for (_, x, y) in &markers {
                    let x = area.origin.x + px(left + (*x as f32 + 0.5) * scale);
                    let y = area.origin.y + px(top + (*y as f32 + 0.5) * scale);
                    for (dx, dy, w, h) in [(-7., -1., 14., 2.), (-1., -7., 2., 14.)] {
                        window.paint_quad(gpui::fill(gpui::Bounds::new(
                            gpui::point(x + px(dx), y + px(dy)), gpui::size(px(w), px(h)),
                        ), rgb(palette().focus)));
                    }
                }
            }).absolute().inset_0().size_full())
            .child(ui::layout_probe("computer-preview"))
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |this, event: &gpui::MouseDownEvent, window, cx| {
                if this.selected == Some(root) {
                    window.focus(&this.autonomy.computer_ui.focus, cx);
                    this.pick_computer_point(frame_id, event.position, cx);
                }
                cx.stop_propagation();
            }))
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape" {
                    this.takeover_computer(cx);
                    cx.stop_propagation();
                }
            }));
        let remaining = 60u64.saturating_sub(frame.captured.elapsed().as_secs());
        let status = if remaining == 0 {
            "Frame expired. Observe the window again to prepare and apply input.".to_owned()
        } else {
            format!(
                "{} x {} pixels · frame age {} seconds · one action per frame. Click prepares input for review.",
                width,
                height,
                frame.captured.elapsed().as_secs()
            )
        };
        let mut controls = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(modes)
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(ui::action(
                        "computer-preview-size",
                        if self.autonomy.computer_ui.expanded {
                            "Compact preview"
                        } else {
                            "Enlarge preview"
                        },
                        None,
                        false,
                        cx.listener(|this, _, _, cx| {
                            this.autonomy.computer_ui.expanded =
                                !this.autonomy.computer_ui.expanded;
                            cx.notify();
                        }),
                    ))
                    .child(ui::action(
                        "computer-scroll-less",
                        "Fewer scroll steps",
                        None,
                        false,
                        cx.listener(|this, _, _, cx| {
                            this.change_computer_scroll_steps(false, cx);
                        }),
                    ))
                    .child(div().child(format!("{} steps", self.autonomy.computer_ui.steps)))
                    .child(ui::action(
                        "computer-scroll-more",
                        "More scroll steps",
                        None,
                        false,
                        cx.listener(|this, _, _, cx| {
                            this.change_computer_scroll_steps(true, cx);
                        }),
                    )),
            )
            .child(viewer)
            .child(div().text_sm().child(status));
        if let Some((_, x, y)) = self
            .autonomy
            .computer_ui
            .point
            .filter(|(id, _, _)| *id == frame_id)
        {
            controls = controls.child(div().text_sm().child(format!(
                "Selected pixel: ({x}, {y}). Coordinates use the original window size."
            )));
        }
        if let Some((_, x, y)) = self
            .autonomy
            .computer_ui
            .drag_start
            .filter(|(id, _, _)| *id == frame_id)
        {
            controls = controls.child(div().text_sm().child(format!(
                "Drag start: ({x}, {y}). Click the endpoint to prepare the drag."
            )));
        }
        controls.into_any_element()
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
        let readiness = match ComputerTools::support() {
            Ok(()) => {
                "Linux/X11 helpers ready. Capture and input require an explicit window selection."
                    .to_owned()
            }
            Err(error) => error.to_string(),
        };
        let mut body = self.autonomy_preamble().gap_3()
            .child("Computer Use · selected-window control")
            .child(div().text_sm().text_color(rgb(palette().muted)).child(readiness))
            .child(div().text_sm().text_color(rgb(palette().muted)).child("Every action needs a fresh frame and your approval. Selection expires after 15 minutes, and an input frame expires after 60 seconds. Control permission is never restored after restart. Full-desktop control, Wayland, macOS and Windows are unavailable."))
            .child(div().flex().flex_wrap().gap_2()
                .child(ui::action("computer-discover", "Discover application windows", Some(Glyph::Window), false, cx.listener(|this, _, _, cx| this.discover_computer_windows(cx))).relative().child(ui::layout_probe("computer-discover")))
                .child(ui::action("computer-observe", "Observe selected window", Some(Glyph::Capture), false, cx.listener(|this, _, _, cx| this.observe_computer(cx))).relative().child(ui::layout_probe("computer-observe")))
                .child(ui::action("computer-takeover", "Take over / revoke control", Some(Glyph::Stop), false, cx.listener(|this, _, _, cx| this.takeover_computer(cx))).relative().child(ui::layout_probe("computer-takeover"))));
        let selected = self.controller.autonomy.computer.selected(root);
        if !self.autonomy.windows.is_empty() {
            body = body.child(self.autonomy.computer_ui.filter.clone());
            let query = self
                .autonomy
                .computer_ui
                .filter
                .read(cx)
                .text()
                .trim()
                .to_lowercase();
            let mut matched = 0;
            for (index, window) in self.autonomy.windows.iter().enumerate() {
                if !query.is_empty()
                    && !format!("{} {} {}", window.title, window.class, window.pid)
                        .to_lowercase()
                        .contains(&query)
                {
                    continue;
                }
                matched += 1;
                let window = window.clone();
                let label = format!("{} · {}", window.title, window.identity());
                let is_selected = selected.as_ref() == Some(&window);
                body = body.child(
                    ui::action(
                        ("computer-select", index),
                        label,
                        None,
                        is_selected,
                        cx.listener(move |this, _, _, cx| {
                            if this.autonomy.busy || this.selected != Some(root) {
                                return;
                            }
                            let Some(tools) = this.autonomy.tools.clone() else {
                                return;
                            };
                            match this.controller.autonomy.computer.select(
                                root,
                                tools,
                                window.clone(),
                            ) {
                                Ok(()) => {
                                    this.autonomy.preview = None;
                                    this.autonomy.computer_review = None;
                                    this.autonomy.computer_ui.clear_points();
                                    this.autonomy.error = None;
                                    this.autonomy.notice = Some(
                                        "Window selected. Observe it to prepare input.".into(),
                                    );
                                }
                                Err(error) => this.autonomy.error = Some(error.to_string()),
                            }
                            cx.notify();
                        }),
                    )
                    .relative()
                    .child(ui::layout_probe_slot("computer-select", index)),
                );
            }
            if matched == 0 {
                body = body.child("No windows match the filter.");
            }
        }
        if let Some(window) = selected {
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
        if let Some(frame) = self.controller.autonomy.computer.frame(root)
            && let Some((id, image)) = &self.autonomy.preview
            && *id == frame.id
        {
            body = body.child(self.computer_preview_controls(root, &frame, image.clone(), cx));
        }
        body = body.child(div().text_sm().child("Prepare text, a key, or a screenshot action below. Preparing does not send input. Typing accepts up to 512 UTF-8 bytes without controls or line separators. Enter and Tab are separate reviewed actions."))
            .child(self.autonomy.computer_ui.text.clone())
            .child(ui::action("computer-prepare-text", "Prepare typed text", None, false, cx.listener(move |this, _, _, cx| {
                if this.selected != Some(root) || this.autonomy.computer_ui.text.read(cx).is_composing() { return; }
                let action = ComputerAction::Type { text: this.autonomy.computer_ui.text.read(cx).text().to_owned() };
                match action.validate(1, 1) {
                    Ok(()) => this.prepare_computer_action(action, cx),
                    Err(error) => { this.autonomy.error = Some(error.to_string()); cx.notify(); }
                }
            })));
        let mut presets = div().flex().flex_wrap().gap_2();
        for (index, (label, key)) in [
            ("Enter", ComputerKey::Enter),
            ("Tab", ComputerKey::Tab),
            ("Escape", ComputerKey::Escape),
            ("Backspace", ComputerKey::Backspace),
            ("Delete", ComputerKey::Delete),
            ("Back tab", ComputerKey::BackTab),
            ("Up", ComputerKey::Up),
            ("Down", ComputerKey::Down),
            ("Left", ComputerKey::Left),
            ("Right", ComputerKey::Right),
            ("Page up", ComputerKey::PageUp),
            ("Page down", ComputerKey::PageDown),
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
        body = body.child(presets)
            .child(div().text_sm().child("Exact action JSON can also be edited below. Pointer coordinates are relative to the observed window. Named keys include insert, f1-f12, shift_enter and editing combinations. Arbitrary key sequences and Alt/Meta shortcuts are unavailable."))
            .child(div().relative().h(px(130.)).flex_shrink_0().flex().flex_col()
                .child(self.autonomy.input.clone()).child(ui::layout_probe("computer-input-editor")))
            .child(ui::action("computer-review-input", "Review input against this frame", None, false,
                cx.listener(|this, _, _, cx| this.review_computer_input(cx))).relative().child(ui::layout_probe("computer-review-input")));
        if let Some((frame, action)) = &self.autonomy.computer_review {
            body = body.child(div().p_3().border_1().border_color(rgb(palette().focus)).flex().flex_col().gap_2()
                .child(format!("Approve frame {frame}: {}", serde_json::to_string(action).unwrap_or_default()))
                .child("This one-shot action revalidates the selected window and observed image. The preview is untrusted application content. Synthetic input may be rejected by the application. Observe again to verify the result.")
                .child(div().flex().gap_2()
                    .child(ui::action("computer-confirm-input", "Apply this input once", None, false, cx.listener(|this, _, _, cx| this.apply_computer_input(cx))).relative().child(ui::layout_probe("computer-confirm-input")))
                    .child(ui::action("computer-cancel-input", "Cancel review", None, false, cx.listener(|this, _, _, cx| { this.autonomy.computer_review = None; cx.notify(); })).relative().child(ui::layout_probe("computer-cancel-input")))));
        }
        body.child(self.autonomy_gateway_settings(cx))
            .into_any_element()
    }
}
