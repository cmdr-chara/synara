//! Transient native editors and previews. Durable workflows and execution belong
//! to WorkspaceService/Controller. No renderer callback performs blocking I/O.
use super::*;
use crate::ui::{self, Glyph, palette};
use std::time::{Duration, Instant};
use synara_runtime::{ComputerAction, ComputerTools, SnapWindow};
use synara_workspace::AutonomyId as Uuid;
use tokio_util::sync::CancellationToken;
mod computer_view;
mod gateway_view;
mod workflow_view;

pub(super) struct AutonomyView {
    root: Option<TaskId>,
    epoch: u64,
    pub(super) busy: bool,
    loading: bool,
    last_poll: Instant,
    value: Option<Workflow>,
    parent: Option<TaskId>,
    status: serde_json::Value,
    editor: Entity<TextEntry>,
    review: Entity<TextEntry>,
    input: Entity<TextEntry>,
    name: Entity<TextEntry>,
    steer: Option<usize>,
    submitted: Option<(String, Option<usize>)>,
    request: Option<GatewayRequest>,
    computer_review: Option<(Uuid, ComputerAction)>,
    computer_ui: computer_view::ComputerUi,
    windows: Vec<SnapWindow>,
    tools: Option<ComputerTools>,
    preview: Option<(Uuid, Arc<gpui::Image>)>,
    io_cancel: CancellationToken,
    error: Option<String>,
    notice: Option<String>,
    _subscriptions: Vec<Subscription>,
}
impl AutonomyView {
    pub fn new(cx: &mut Context<Shell>) -> Self {
        let editor = cx.new(|cx| {
            TextEntry::new(
                "Review a workflow specification before creating unsent children",
                EntryMode::Editor,
                220.,
                cx,
            )
        });
        let review =
            cx.new(|cx| TextEntry::new("Frozen incoming request", EntryMode::Editor, 220., cx));
        let input =
            cx.new(|cx| TextEntry::new("Review window input JSON", EntryMode::Editor, 110., cx));
        let name =
            cx.new(|cx| TextEntry::new("Incoming client name", EntryMode::SingleLine, 32., cx));
        let subscriptions = [&editor, &review, &input, &name]
            .into_iter()
            .map(|entity| cx.subscribe(entity, |_, _, _, cx| cx.notify()))
            .collect();
        Self {
            root: None,
            epoch: 0,
            busy: false,
            loading: false,
            last_poll: Instant::now(),
            value: None,
            parent: None,
            status: serde_json::Value::Null,
            editor,
            review,
            input,
            name,
            steer: None,
            submitted: None,
            request: None,
            computer_review: None,
            computer_ui: computer_view::ComputerUi::new(cx),
            windows: vec![],
            tools: None,
            preview: None,
            io_cancel: CancellationToken::new(),
            error: None,
            notice: None,
            _subscriptions: subscriptions,
        }
    }
}
impl Drop for AutonomyView {
    fn drop(&mut self) {
        self.io_cancel.cancel();
    }
}
pub(super) struct Reply {
    root: TaskId,
    epoch: u64,
    read: bool,
    result: Result<Outcome, String>,
}
pub(super) enum Outcome {
    Loaded(Box<Snapshot>),
    Changed,
    Windows(ComputerTools, Vec<SnapWindow>),
    Frame(ComputerFrame),
}
pub(super) struct Snapshot {
    value: Option<Workflow>,
    parent: Option<TaskId>,
    status: serde_json::Value,
    catalog: Catalog,
}
impl Shell {
    pub(super) fn autonomy_pending(&self, cx: &App) -> bool {
        !self.autonomy.editor.read(cx).text().is_empty()
            || self.autonomy.request.is_some()
            || self.autonomy.computer_review.is_some()
            || self.autonomy.editor.read(cx).is_composing()
            || self.autonomy.review.read(cx).is_composing()
    }
    pub(super) fn autonomy_navigation_blocked(&mut self, cx: &mut Context<Self>) -> bool {
        if self.autonomy_pending(cx) {
            self.notice = Some("Create/save or explicitly discard the workflow draft, and finish or cancel the open approval review before changing tasks.".into());
            cx.notify();
            return true;
        }
        false
    }
    pub(super) fn retire_autonomy_selection(&mut self) {
        self.autonomy.io_cancel.cancel();
        if let Some(root) = self.selected {
            self.controller.autonomy.computer.revoke(root);
        }
    }
    fn sync_autonomy(&mut self, cx: &mut Context<Self>) {
        if self.autonomy.root == self.selected {
            return;
        }
        self.autonomy.io_cancel.cancel();
        self.autonomy.io_cancel = CancellationToken::new();
        self.autonomy.epoch = self.autonomy.epoch.wrapping_add(1);
        self.autonomy.root = self.selected;
        self.autonomy.value = None;
        self.autonomy.parent = None;
        self.autonomy.status = serde_json::Value::Null;
        self.autonomy.loading = false;
        self.autonomy.busy = false;
        self.autonomy.request = None;
        self.autonomy.computer_review = None;
        self.autonomy.computer_ui.reset(cx);
        self.autonomy.steer = None;
        self.autonomy.submitted = None;
        self.autonomy.windows.clear();
        self.autonomy.tools = None;
        self.autonomy.preview = None;
        self.autonomy.error = None;
        self.autonomy.notice = None;
        self.autonomy.editor.update(cx, |e, cx| e.clear(cx));
        self.autonomy.review.update(cx, |e, cx| e.clear(cx));
        self.autonomy.input.update(cx, |e, cx| e.clear(cx));
    }
    pub(super) fn refresh_autonomy(&mut self, cx: &mut Context<Self>) {
        self.sync_autonomy(cx);
        let Some(root) = self.selected else { return };
        if self.autonomy.loading || self.close != CloseState::Open {
            return;
        }
        self.autonomy.loading = true;
        self.autonomy.last_poll = Instant::now();
        let epoch = self.autonomy.epoch;
        let controller = self.controller.clone();
        self.job(async move {
            let result = async {
                Ok(Outcome::Loaded(Box::new(Snapshot {
                    value: controller.workspace.workflow(root).await?,
                    parent: controller.workspace.workflow_parent(root).await?,
                    status: controller.gateway_status(root).await?,
                    catalog: controller.workspace.catalog().await?,
                })))
            }
            .await;
            Ok(Update::Autonomy(Box::new(Reply {
                root,
                epoch,
                read: true,
                result: result.map_err(|e: WorkspaceError| e.to_string()),
            })))
        });
        cx.notify();
    }
    pub(super) fn tick_autonomy(&mut self, cx: &mut Context<Self>) {
        if self.close != CloseState::Open {
            return;
        }
        if self.panel == Panel::Settings
            && self.autonomy.last_poll.elapsed() >= Duration::from_secs(1)
        {
            self.refresh_autonomy(cx);
        }
        if let Some(root) = self.selected {
            if let Some(frame) = self.controller.autonomy.computer.frame(root) {
                if self
                    .autonomy
                    .preview
                    .as_ref()
                    .is_none_or(|(id, _)| *id != frame.id)
                {
                    self.autonomy.preview = Some((
                        frame.id,
                        Arc::new(gpui::Image::from_bytes(
                            gpui::ImageFormat::Png,
                            frame.png.as_ref().clone(),
                        )),
                    ));
                    cx.notify();
                }
            } else if self.autonomy.preview.take().is_some() {
                cx.notify();
            }
            if self.controller.autonomy.gateway.pending_count(root) > 0 {
                cx.notify();
            }
        }
    }
    fn autonomy_job(
        &mut self,
        work: impl std::future::Future<Output = WorkspaceResult<Outcome>> + Send + 'static,
        cx: &mut Context<Self>,
    ) {
        if self.autonomy.busy || self.close != CloseState::Open {
            return;
        }
        let Some(root) = self.selected.filter(|id| Some(*id) == self.autonomy.root) else {
            return;
        };
        self.autonomy.busy = true;
        self.autonomy.error = None;
        let epoch = self.autonomy.epoch;
        self.job(async move {
            Ok(Update::Autonomy(Box::new(Reply {
                root,
                epoch,
                read: false,
                result: work.await.map_err(|e| e.to_string()),
            })))
        });
        cx.notify();
    }
    pub(super) fn autonomy_reply(&mut self, reply: Reply, cx: &mut Context<Self>) {
        if self.selected != Some(reply.root)
            || self.autonomy.root != Some(reply.root)
            || self.autonomy.epoch != reply.epoch
        {
            return;
        }
        if reply.read {
            self.autonomy.loading = false;
        } else {
            self.autonomy.busy = false;
        }
        match reply.result {
            Ok(Outcome::Loaded(snapshot)) => {
                self.autonomy.value = snapshot.value;
                self.autonomy.parent = snapshot.parent;
                self.autonomy.status = snapshot.status;
                for task in snapshot.catalog.tasks {
                    if !self.catalog.tasks.iter().any(|old| old.id == task.id) {
                        self.catalog.tasks.push(task);
                    }
                }
            }
            Ok(Outcome::Changed) => {
                if let Some((text, step)) = self.autonomy.submitted.take()
                    && self.autonomy.editor.read(cx).text() == text
                    && self.autonomy.steer == step
                {
                    self.autonomy.editor.update(cx, |e, cx| e.clear(cx));
                    self.autonomy.steer = None;
                }
                self.autonomy.notice = Some(
                    "Operation returned. Review the current task, workflow or receipt state below."
                        .into(),
                );
                self.refresh_autonomy(cx);
            }
            Ok(Outcome::Windows(tools, windows)) => {
                self.autonomy.tools = Some(tools);
                self.autonomy.windows = windows;
                self.autonomy.notice = Some(
                    "Discovery read window identities only. Select one application explicitly."
                        .into(),
                );
            }
            Ok(Outcome::Frame(frame)) => {
                self.autonomy.preview = Some((
                    frame.id,
                    Arc::new(gpui::Image::from_bytes(
                        gpui::ImageFormat::Png,
                        frame.png.as_ref().clone(),
                    )),
                ));
                self.autonomy.notice = Some("Window observed. An input review expires after 60 seconds and consumes this frame once.".into());
            }
            Err(error) => {
                if !reply.read {
                    self.autonomy.submitted = None;
                }
                self.autonomy.error = Some(error);
            }
        }
        cx.notify();
    }
    fn autonomy_preamble(&self) -> gpui::Div {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .children(self.autonomy.error.as_ref().map(|e| {
                div()
                    .text_color(rgb(palette().error))
                    .child(e.clone())
                    .relative()
                    .child(ui::layout_probe("autonomy-error"))
            }))
            .children(
                self.autonomy
                    .notice
                    .as_ref()
                    .map(|s| div().text_color(rgb(palette().muted)).child(s.clone())),
            )
            .children(self.autonomy.busy.then(|| {
                div().child("Operation in progress. Pause, Stop and revoke remain available.")
            }))
    }
    pub(super) fn autonomy_button(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        if self.selected.is_none() {
            return div().into_any_element();
        }
        let pending = self
            .selected
            .map_or(0, |id| self.controller.autonomy.gateway.pending_count(id));
        let label = if pending > 0 {
            format!("Workflows · {pending} approvals")
        } else {
            "Subagents & workflows".into()
        };
        ui::action(
            "autonomy-open",
            label,
            Some(Glyph::Blocks),
            false,
            cx.listener(|this, _, _, cx| {
                this.set_panel(Panel::Settings, cx);
                this.open_settings_section(settings::Section::Workflows, cx);
            }),
        )
        .relative()
        .child(ui::layout_probe("autonomy-open"))
        .into_any_element()
    }
    fn open_workflow_child(&mut self, task: TaskId, cx: &mut Context<Self>) {
        if self.select_task(task, cx) {
            self.show_conversation(cx);
        }
    }
}
