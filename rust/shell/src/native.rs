//! Native gpui-kit workspace: sidebar (spaces/projects/threads) + transcript +
//! composer, fed by rust/app's native WS-RPC (getShellSnapshot /
//! getThreadDetailSnapshot / dispatchCommand relayed upstream).

use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

use gpui_kit::component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputEvent, InputState},
    scroll::ScrollableElement as _,
    v_flex,
    ActiveTheme as _,
};
use gpui_kit::*;

use crate::ws_client::Client;

type SharedClient = Arc<Mutex<Option<Arc<Client>>>>;

pub struct Native {
    focus_handle: FocusHandle,
    client: SharedClient,
    snapshot: Option<Value>,
    detail: Option<Value>,
    selected_thread: Option<String>,
    thread_stream_id: Option<String>,
    composer: Entity<InputState>,
    sending: bool,
}

fn get<'a>(v: &'a Value, k: &str) -> &'a Value {
    v.get(k).unwrap_or(&Value::Null)
}

fn str_at<'a>(v: &'a Value, k: &str) -> &'a str {
    get(v, k).as_str().unwrap_or("")
}

fn arr<'a>(v: &'a Value, k: &str) -> &'a [Value] {
    get(v, k).as_array().map(Vec::as_slice).unwrap_or(&[])
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Blocking: connect (once) and clone the shared client. Run on a background
/// executor.
fn ensure_client(cell: &SharedClient) -> Option<Arc<Client>> {
    let mut guard = cell.lock().ok()?;
    if guard.is_none() {
        *guard = Client::connect();
    }
    guard.clone()
}

impl Native {
    pub fn new(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let composer = cx.new(|cx| {
                InputState::new(window, cx).placeholder("Do anything in Synara")
            });
            cx.subscribe_in(
                &composer,
                window,
                |this: &mut Self, _state, event, window, cx| {
                    if let InputEvent::PressEnter { shift, .. } = event {
                        if !shift {
                            this.send(window, cx);
                        }
                    }
                },
            )
            .detach();
            let this = Self {
                focus_handle: cx.focus_handle(),
                client: Arc::new(Mutex::new(None)),
                snapshot: None,
                detail: None,
                selected_thread: None,
                thread_stream_id: None,
                composer,
                sending: false,
            };
            this.refresh(cx);
            this
        })
    }

    /// (Re)fetch the shell snapshot, then keep it live via subscribeShell.
    fn refresh(&self, cx: &mut Context<Self>) {
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else {
                eprintln!("native: ws connect failed");
                return;
            };
            let c = client.clone();
            let snapshot = cx
                .background_executor()
                .spawn(async move { c.call("orchestration.getShellSnapshot", json!({})) })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if let Some(s) = snapshot {
                    this.snapshot = Some(s);
                }
                cx.notify();
            });
            // Live updates: subscribeShell streams shell-domain events; on each
            // chunk just refetch the snapshot.
            let (tx, rx) = async_channel::unbounded::<()>();
            let c = client.clone();
            cx.background_executor()
                .spawn(async move {
                    c.call_stream("orchestration.subscribeShell", json!({}), move |_| {
                        let _ = tx.send_blocking(());
                    });
                })
                .detach();
            while rx.recv().await.is_ok() {
                let c = client.clone();
                let s = cx
                    .background_executor()
                    .spawn(async move { c.call("orchestration.getShellSnapshot", json!({})) })
                    .await;
                let _ = this.update(cx, |this: &mut Self, cx| {
                    if let Some(s) = s {
                        this.snapshot = Some(s);
                    }
                    cx.notify();
                });
            }
        })
        .detach();
        cx.notify();
    }

    fn open_thread(&mut self, thread_id: String, cx: &mut Context<Self>) {
        // Close the previous thread stream if any.
        let old = self.thread_stream_id.take();
        self.selected_thread = Some(thread_id.clone());
        self.detail = None;
        let client = self.client.clone();
        let tid = thread_id;
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            if let Some(old) = old {
                client.interrupt(&old);
            }
            let payload = json!({"threadId": tid});
            let c = client.clone();
            let p = payload.clone();
            let detail = cx
                .background_executor()
                .spawn(async move { c.call("orchestration.getThreadDetailSnapshot", p) })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if this.selected_thread.as_deref() == Some(tid.as_str()) {
                    this.detail = detail;
                }
                cx.notify();
            });
            // Live transcript via subscribeThread: each chunk triggers a detail
            // refetch (chunk shapes vary; refetch is schema-free).
            let c = client.clone();
            let (rid, frames) = match c.request("orchestration.subscribeThread", payload.clone()) {
                Some(v) => v,
                None => return,
            };
            let rid_for_ui = rid.clone();
            let _ = this.update(cx, |this: &mut Self, _| {
                this.thread_stream_id = Some(rid_for_ui);
            });
            let (tx, rx) = async_channel::unbounded::<()>();
            let c = client.clone();
            cx.background_executor()
                .spawn(async move {
                    while let Ok(msg) = frames.recv() {
                        match msg.get("_tag").and_then(Value::as_str) {
                            Some("Chunk") => {
                                let _ = tx.send_blocking(());
                            }
                            Some("Exit") => break,
                            _ => {}
                        }
                    }
                    c.finish_request(&rid);
                })
                .detach();
            while rx.recv().await.is_ok() {
                let c = client.clone();
                let p = payload.clone();
                let d = cx
                    .background_executor()
                    .spawn(async move { c.call("orchestration.getThreadDetailSnapshot", p) })
                    .await;
                let _ = this.update(cx, |this: &mut Self, cx| {
                    if this.selected_thread.as_deref() == Some(tid.as_str()) {
                        this.detail = d;
                    }
                    cx.notify();
                });
            }
        })
        .detach();
        cx.notify();
    }

    /// Composer: send the draft as a turn on the selected thread, or create a
    /// thread in the first project then start the turn.
    fn send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.composer.read(cx).value().to_string();
        let text = text.trim().to_string();
        if text.is_empty() || self.sending {
            return;
        }
        self.sending = true;
        self.composer.update(cx, |s, cx| s.clean(window, cx));
        let selected = self.selected_thread.clone();
        let project_id = arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects")
            .first()
            .and_then(|p| p.get("id"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else {
                eprintln!("native: send: no ws client");
                let _ = this.update(cx, |this: &mut Self, cx| {
                    this.sending = false;
                    cx.notify();
                });
                return;
            };
            let model_selection = json!({
                "provider": "codex",
                "instanceId": "codex",
                "model": "gpt-6-astra",
            });
            let tid = selected.clone().unwrap_or_else(uuid);
            let mut cmds = Vec::new();
            if selected.is_none() {
                if let Some(pid) = project_id {
                    cmds.push(json!({
                        "type": "thread.create",
                        "commandId": uuid(),
                        "threadId": tid,
                        "projectId": pid,
                        "title": text.chars().take(60).collect::<String>(),
                        "modelSelection": model_selection,
                        "runtimeMode": "full-access",
                        "interactionMode": "default",
                        "envMode": "local",
                        "branch": null,
                        "worktreePath": null,
                        "createdAt": now_iso(),
                    }));
                }
            }
            cmds.push(json!({
                "type": "thread.turn.start",
                "commandId": uuid(),
                "threadId": tid,
                "message": {
                    "messageId": uuid(),
                    "role": "user",
                    "text": text,
                    "attachments": [],
                },
                "runtimeMode": "full-access",
                "interactionMode": "default",
                "createdAt": now_iso(),
            }));
            for cmd in cmds {
                let c = client.clone();
                let r = cx
                    .background_executor()
                    .spawn(async move { c.call("orchestration.dispatchCommand", cmd) })
                    .await;
                if r.is_none() {
                    eprintln!("native: dispatchCommand failed");
                } else {
                }
            }
            // Refresh snapshot + detail.
            let c = client.clone();
            let s = cx
                .background_executor()
                .spawn(async move { c.call("orchestration.getShellSnapshot", json!({})) })
                .await;
            let p = json!({"threadId": tid});
            let c = client.clone();
            let d = cx
                .background_executor()
                .spawn(async move { c.call("orchestration.getThreadDetailSnapshot", p) })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.sending = false;
                if let Some(s) = s {
                    this.snapshot = Some(s);
                }
                this.selected_thread = Some(tid.clone());
                if let Some(d) = d {
                    this.detail = Some(d);
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
}

impl Focusable for Native {
    fn focus_handle(&self, _cx: &gpui_kit::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

fn row(
    id: impl Into<ElementId>,
    title: String,
    subtitle: String,
    selected: bool,
    cx: &mut Context<Native>,
    on_click: impl Fn(&mut Native, &ClickEvent, &mut Window, &mut Context<Native>) + 'static,
) -> impl IntoElement {
    let theme = cx.theme();
    let mut el = v_flex()
        .id(id)
        .w_full()
        .px_3()
        .py_2()
        .gap_1()
        .cursor_pointer()
        .rounded_md()
        .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(title))
        .child(div().text_xs().text_color(theme.muted_foreground).child(subtitle));
    if selected {
        el = el.bg(theme.list_active);
    }
    el.on_click(cx.listener(on_click))
}

impl Render for Native {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        // ---- sidebar ----
        let mut sidebar = v_flex().w(px(280.)).h_full().gap_1().p_2();
        for space in arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "spaces") {
            sidebar = sidebar.child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.muted_foreground)
                    .child(str_at(space, "name").to_uppercase()),
            );
        }
        for project in arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects") {
            sidebar = sidebar.child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(str_at(project, "title").to_string()),
            );
            let pid = str_at(project, "id").to_string();
            for thread in arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "threads")
                .iter()
                .filter(|t| str_at(t, "projectId") == pid)
                .cloned()
                .collect::<Vec<_>>()
            {
                let tid = str_at(&thread, "id").to_string();
                let title = str_at(&thread, "title");
                let preview = str_at(&thread, "latestReplyPreview");
                let selected = self.selected_thread.as_deref() == Some(tid.as_str());
                sidebar = sidebar.child(row(
                    ElementId::Name(format!("thread-{tid}").into()),
                    if title.is_empty() { "(untitled)".to_string() } else { title.to_string() },
                    preview.chars().take(80).collect(),
                    selected,
                    cx,
                    move |this, _ev, _w, cx| this.open_thread(tid.clone(), cx),
                ));
            }
        }
        sidebar = sidebar.child(
            Button::new("refresh")
                .ghost()
                .label("Refresh")
                .on_click(cx.listener(|this, _ev, _w, cx| this.refresh(cx))),
        );

        // ---- transcript ----
        let mut pane = v_flex().flex_1().h_full().p_4().gap_2();
        let messages = arr(get(self.detail.as_ref().unwrap_or(&Value::Null), "thread"), "messages");
        if messages.is_empty() {
            pane = pane.child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("Select a thread or type below"),
            );
        }
        for m in messages {
            let role = str_at(m, "role");
            let text = str_at(m, "text");
            if text.trim().is_empty() {
                continue;
            }
            pane = pane.child(
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.muted_foreground)
                            .child(role.to_uppercase()),
                    )
                    .child(div().text_sm().child(text.to_string())),
            );
        }

        // ---- composer ----
        let composer_bar = h_flex()
            .w_full()
            .p_3()
            .gap_2()
            .border_t_1()
            .border_color(theme.border)
            .child(div().flex_1().child(Input::new(&self.composer)))
            .child(
                Button::new("send")
                    .primary()
                    .label(if self.sending { "Sending..." } else { "Send" })
                    .on_click(cx.listener(|this, _ev, w, cx| this.send(w, cx))),
            );

        let main = v_flex()
            .flex_1()
            .h_full()
            .child(div().flex_1().child(pane.overflow_y_scrollbar()))
            .child(composer_bar);

        h_flex()
            .size_full()
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(sidebar.border_r_1().border_color(theme.border))
            .child(main)
    }
}
