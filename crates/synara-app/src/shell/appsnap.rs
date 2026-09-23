//! Transient discovery and explicit one-shot capture into the existing draft owner.
use super::*;
use crate::ui::{self, Glyph, palette};
use synara_runtime::{DeviceCancellation, SnapTools, SnapWindow};
#[derive(Default)]
pub(super) struct SnapView {
    open: bool,
    epoch: u64,
    cancel: DeviceCancellation,
    busy: bool,
    tools: Option<SnapTools>,
    windows: Vec<SnapWindow>,
    selected: Option<usize>,
    error: Option<String>,
    message: String,
}
impl SnapView {
    pub fn retire(&mut self) {
        self.cancel.cancel();
        self.cancel = DeviceCancellation::new();
        self.epoch = self.epoch.wrapping_add(1);
        self.open = false;
        self.busy = false;
        self.tools = None;
        self.windows.clear();
        self.selected = None;
        self.error = None;
        self.message.clear();
    }
}
impl Drop for SnapView {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}
pub(super) struct Reply {
    epoch: u64,
    task: TaskId,
    revision: u64,
    result: Result<Outcome, String>,
}
enum Outcome {
    Discovery(SnapTools, Vec<SnapWindow>),
    Capture(SnapWindow, Vec<u8>),
}
impl Shell {
    pub(super) fn open_appsnap_from_settings(&mut self, cx: &mut Context<Self>) {
        if self.selected.is_none() || self.close != CloseState::Open {
            return;
        }
        self.set_panel(Panel::Conversation, cx);
        self.appsnap.retire();
        self.appsnap.open = true;
        cx.notify();
    }
    pub(super) fn toggle_appsnap(&mut self, cx: &mut Context<Self>) {
        let open = !self.appsnap.open;
        self.appsnap.retire();
        self.appsnap.open = open;
        cx.notify();
    }
    fn appsnap_job(
        &mut self,
        work: impl std::future::Future<Output = Result<Outcome, String>> + Send + 'static,
        cx: &mut Context<Self>,
    ) {
        let Some(task) = self.selected else { return };
        self.appsnap.busy = true;
        self.appsnap.error = None;
        let epoch = self.appsnap.epoch;
        let revision = self.selection_revision;
        self.job(async move {
            Ok(Update::AppSnap(Box::new(Reply {
                epoch,
                task,
                revision,
                result: work.await,
            })))
        });
        cx.notify();
    }
    fn discover_appsnap(&mut self, cx: &mut Context<Self>) {
        if self.close != CloseState::Open || self.appsnap.busy {
            return;
        }
        self.appsnap.retire();
        self.appsnap.open = true;
        let cancel = self.appsnap.cancel.clone();
        self.appsnap.message =
            "Inspecting visible window identities only. No pixels are captured by discovery."
                .into();
        self.appsnap_job(
            async move {
                let tools = SnapTools::setup().map_err(|e| e.to_string())?;
                let windows = tools.discover(&cancel).await.map_err(|e| e.to_string())?;
                Ok(Outcome::Discovery(tools, windows))
            },
            cx,
        );
    }
    fn capture_appsnap(&mut self, cx: &mut Context<Self>) {
        if self.close != CloseState::Open || self.appsnap.busy || self.attachment_send_blocked() {
            return;
        }
        let Some(window) = self
            .appsnap
            .selected
            .and_then(|i| self.appsnap.windows.get(i))
            .cloned()
        else {
            return;
        };
        let Some(tools) = self.appsnap.tools.clone() else {
            return;
        };
        let cancel = self.appsnap.cancel.clone();
        // Selection is consumed even if the native helper fails. A second capture
        // requires a fresh explicit selection, never an automatic retry.
        self.appsnap.selected = None;
        self.appsnap.message =
            "Capturing the reviewed window once. Stop discards an unfinished result.".into();
        self.appsnap_job(
            async move {
                let png = tools
                    .capture(&window, &cancel)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(Outcome::Capture(window, png))
            },
            cx,
        );
    }
    pub(super) fn appsnap_reply(&mut self, reply: Reply, cx: &mut Context<Self>) {
        if !self.appsnap.open
            || self.appsnap.epoch != reply.epoch
            || self.selected != Some(reply.task)
            || self.selection_revision != reply.revision
            || self.close != CloseState::Open
        {
            return;
        }
        self.appsnap.busy = false;
        match reply.result {
            Ok(Outcome::Discovery(tools, windows)) => {
                self.appsnap.message = if windows.is_empty() {
                    "No supported visible application windows with process identities were found."
                        .into()
                } else {
                    format!(
                        "{} visible windows. Select one, then explicitly capture it.",
                        windows.len()
                    )
                };
                self.appsnap.tools = Some(tools);
                self.appsnap.windows = windows;
            }
            Ok(Outcome::Capture(window, png)) => {
                let name = format!(
                    "AppSnap-PID{}-{}x{}.png",
                    window.pid, window.width, window.height
                );
                self.appsnap.retire();
                self.attach_capture(name, png, ImageSource::AppSnap, cx);
                self.notice = Some(format!(
                    "AppSnap captured {} ({}). Review the pending attachment before Send. No input authority was granted.",
                    window.title,
                    window.identity()
                ));
            }
            Err(error) => {
                self.appsnap.selected = None;
                self.appsnap.error = Some(format!(
                    "AppSnap stopped: {error}. OS permission status is not reported by X11. Nothing was attached. Refresh and reselect before retrying."
                ));
            }
        }
        cx.notify();
    }
    pub(super) fn appsnap_view(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        if !self.appsnap.open {
            return div().into_any_element();
        }
        let state = &self.appsnap;
        let mut root=div().relative().flex().flex_col().gap_1().min_w_0().text_size(px(11.))
            .child(ui::layout_probe("appsnap-panel"))
            .child(div().flex().items_center().gap_2()
                .child(div().flex_1().child("AppSnap | one window, one capture"))
                .child(ui::chrome_button("appsnap-close","Close AppSnap and revoke selection",Glyph::Close,false,
                    cx.listener(|this,_:&(),_,cx|{this.appsnap.retire();cx.notify();})).size(px(24.))))
            .child(div().text_color(rgb(palette().muted)).child("OS permission: not reported by X11. Synara consent: this capture only. No desktop capture or input control. Setup and selections are not restored."));
        if let Err(error) = SnapTools::support() {
            return root
                .child(div().child(error.to_string()))
                .into_any_element();
        }
        if state.busy {
            root = root.child(
                ui::action(
                    "appsnap-stop",
                    "Stop capture / discovery",
                    Some(Glyph::Stop),
                    false,
                    cx.listener(|this, _: &(), _, cx| {
                        this.appsnap.retire();
                        this.appsnap.open = true;
                        this.appsnap.message =
                            "Stopped. Selection and consent were discarded. No automatic retry."
                                .into();
                        cx.notify();
                    }),
                )
                .relative()
                .child(ui::layout_probe("appsnap-stop")),
            );
        } else {
            root = root.child(
                ui::action(
                    "appsnap-setup",
                    if state.tools.is_some() {
                        "Refresh visible windows"
                    } else {
                        "Set up Linux/X11 and discover windows"
                    },
                    Some(Glyph::Restore),
                    false,
                    cx.listener(|this, _: &(), _, cx| this.discover_appsnap(cx)),
                )
                .relative()
                .child(ui::layout_probe("appsnap-setup")),
            );
        }
        root = root
            .child(div().child(state.message.clone()))
            .children(state.error.as_ref().map(|e| {
                div()
                    .relative()
                    .child(ui::layout_probe("appsnap-error"))
                    .text_color(rgb(palette().error))
                    .child(e.clone())
            }));
        if !state.busy {
            root = root.child(
                div()
                    .id("appsnap-windows")
                    .max_h(px(85.))
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .children(state.windows.iter().enumerate().map(|(slot, window)| {
                        ui::action(
                            ("appsnap-window", slot),
                            format!("{} | {}", window.title, window.identity()),
                            None,
                            state.selected == Some(slot),
                            cx.listener(move |this, _: &(), _, cx| {
                                this.appsnap.selected = Some(slot);
                                this.appsnap.error = None;
                                cx.notify();
                            }),
                        )
                        .text_size(px(11.))
                        .relative()
                        .child(ui::layout_probe_slot("appsnap-window", slot))
                    })),
            );
            if let Some(window) = state.selected.and_then(|i| state.windows.get(i)) {
                root = root
                    .child(
                        div()
                            .relative()
                            .child(ui::layout_probe("appsnap-reviewed"))
                            .child(format!(
                                "Capture only: {} | {}",
                                window.title,
                                window.identity()
                            )),
                    )
                    .child(
                        ui::chrome_button(
                            "appsnap-capture",
                            "Capture selected window and attach once",
                            Glyph::Capture,
                            self.attachment_send_blocked(),
                            cx.listener(|this, _: &(), _, cx| this.capture_appsnap(cx)),
                        )
                        .w_full(),
                    );
            }
        }
        root.into_any_element()
    }
}
