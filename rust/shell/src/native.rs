//! Native gpui-kit workspace: sidebar (spaces/projects/threads) + transcript,
//! fed by rust/app's native WS-RPC (getShellSnapshot / getThreadDetailSnapshot).

use serde_json::Value;
use std::sync::{Arc, Mutex};

use gpui_kit::component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    scroll::ScrollableElement as _,
    v_flex,
    ActiveTheme as _,
};
use gpui_kit::*;

use crate::ws_client::Client;

pub struct Native {
    focus_handle: FocusHandle,
    client: Arc<Mutex<Option<Client>>>,
    snapshot: Option<Value>,
    detail: Option<Value>,
    selected_thread: Option<String>,
    loading: bool,
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

impl Native {
    pub fn new(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let this = Self {
                focus_handle: cx.focus_handle(),
                client: Arc::new(Mutex::new(None)),
                snapshot: None,
                detail: None,
                selected_thread: None,
                loading: false,
            };
            this.refresh(cx);
            this
        })
    }

    /// (Re)fetch the shell snapshot off the UI thread.
    fn refresh(&self, cx: &mut Context<Self>) {
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let snapshot = cx
                .background_executor()
                .spawn(async move {
                    let mut guard = client.lock().ok()?;
                    if guard.is_none() {
                        *guard = Client::connect();
                    }
                    guard.as_mut()?.call("orchestration.getShellSnapshot", Value::Null)
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.loading = false;
                match snapshot {
                    Some(s) => this.snapshot = Some(s),
                    None => eprintln!("native: getShellSnapshot returned None"),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn open_thread(&mut self, thread_id: String, cx: &mut Context<Self>) {
        self.selected_thread = Some(thread_id.clone());
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let detail = cx
                .background_executor()
                .spawn(async move {
                    let mut guard = client.lock().ok()?;
                    if guard.is_none() {
                        *guard = Client::connect();
                    }
                    guard.as_mut()?.call(
                        "orchestration.getThreadDetailSnapshot",
                        json_payload(&thread_id),
                    )
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.detail = detail;
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
}

fn json_payload(thread_id: &str) -> Value {
    serde_json::json!({"threadId": thread_id})
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
                    .child("Select a thread"),
            );
        }
        for m in messages {
            let role = str_at(m, "role");
            let text = str_at(m, "text");
            if text.trim().is_empty() {
                continue;
            }
            let label = if role == "assistant" { "assistant" } else { role };
            pane = pane.child(
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.muted_foreground)
                            .child(label.to_uppercase()),
                    )
                    .child(div().text_sm().child(text.to_string())),
            );
        }

        h_flex()
            .size_full()
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(sidebar.border_r_1().border_color(theme.border))
            .child(pane.overflow_y_scrollbar())
    }
}
