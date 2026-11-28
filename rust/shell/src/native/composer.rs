//! Extracted from native/mod.rs during the surface split — see mod.rs.
#![allow(unused_imports)]

use super::*;
use gpui::prelude::FluentBuilder as _;
use gpui_kit::assets::IconName as AssetIcon;
use gpui_kit::base::Disableable as _;
use gpui_kit::component::WindowExt as _;
use gpui_kit::component::{
    button::{Button, ButtonCustomVariant, ButtonRounded, ButtonVariant, ButtonVariants as _},
    h_flex,
    input::{Editor, EditorState, Input, InputEvent, InputState},
    menu::{DropdownMenu as _, PopupMenuItem},
    scroll::ScrollableElement as _,
    slider::{Slider, SliderEvent, SliderState},
    spinner::Spinner,
    text::TextView,
    theme::Theme,
    v_flex, ActiveTheme as _, Icon, IconName, Sizable as _,
};
use gpui_kit::*;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

impl Native {
    /// "+" button: native file picker -> upload each file -> chip in composer.
    pub(super) fn attach(&mut self, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some("Attach files".into()),
        });
        let client = self.client.clone();
        if self.composer_thread_id.is_none() && self.selected_thread.is_none() {
            self.composer_thread_id = Some(uuid());
        }
        let thread_id = self
            .selected_thread
            .clone()
            .or_else(|| self.composer_thread_id.clone());
        self.uploading = true;
        cx.notify();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let paths = rx
                .await
                .ok()
                .and_then(|r| r.ok())
                .flatten()
                .unwrap_or_default();
            let cell = client.clone();
            let thread_id = thread_id.unwrap_or_default();
            let uploaded: Vec<Value> = cx
                .background_executor()
                .spawn(async move {
                    let Some(c) = ensure_client(&cell) else {
                        return Vec::new();
                    };
                    let mut out = Vec::new();
                    for p in paths {
                        let Ok(bytes) = std::fs::read(&p) else {
                            continue;
                        };
                        let name = p
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("file")
                            .to_string();
                        let ext = p
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("")
                            .to_lowercase();
                        let (kind, mime) = match ext.as_str() {
                            "png" => ("image", "image/png"),
                            "jpg" | "jpeg" => ("image", "image/jpeg"),
                            "gif" => ("image", "image/gif"),
                            "webp" => ("image", "image/webp"),
                            "pdf" => ("file", "application/pdf"),
                            "txt" | "md" => ("file", "text/plain"),
                            "json" => ("file", "application/json"),
                            "zip" => ("file", "application/zip"),
                            _ => ("file", "application/octet-stream"),
                        };
                        if let Some(att) =
                            c.upload_attachment(&thread_id, kind, &name, mime, &bytes)
                        {
                            out.push(att);
                        }
                    }
                    out
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.uploading = false;
                this.pending_attachments.extend(uploaded);
                cx.notify();
            });
        })
        .detach();
    }

    /// Composer extras "+" -> Goal: upstream inserts a literal `/goal` slash
    /// command so the goal chip flow is the same as typing it.
    pub(super) fn set_draft(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        let composer = self.composer.clone();
        self.composer.update(cx, |state, cx| {
            state.set_value(text.to_string(), window, cx);
        });
        let focus = composer.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        cx.notify();
    }

    /// Fire-and-forget `orchestration.dispatchCommand`, like upstream's
    /// `orchestration.dispatchCommand(...).forget()` call sites.
    pub(super) fn dispatch_cmd(&mut self, cmd: Value, cx: &mut Context<Self>) {
        let cell = self.client.clone();
        cx.spawn(async move |_this, cx: &mut AsyncApp| {
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            cx.background_executor()
                .spawn(async move { client.call("orchestration.dispatchCommand", cmd) })
                .detach();
        })
        .detach();
    }

    /// Composer paste-image: upstream inserts clipboard images as image
    /// attachments of the draft thread (same upload path as window captures).
    pub(super) fn attach_clipboard_image(
        &mut self,
        bytes: Vec<u8>,
        mime: String,
        ext: String,
        cx: &mut Context<Self>,
    ) {
        let client = self.client.clone();
        if self.composer_thread_id.is_none() && self.selected_thread.is_none() {
            self.composer_thread_id = Some(uuid());
        }
        let thread_id = self
            .selected_thread
            .clone()
            .or_else(|| self.composer_thread_id.clone());
        self.uploading = true;
        cx.notify();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let tid = thread_id.unwrap_or_default();
            let uploaded: Option<(Value, Option<std::sync::Arc<RenderImage>>)> = cx
                .background_executor()
                .spawn(async move {
                    let Some(c) = ensure_client(&cell) else {
                        return None;
                    };
                    let thumb = decode_bytes_to_render_image(&bytes);
                    let name = format!("pasted-image.{ext}");
                    c.upload_attachment(&tid, "image", &name, &mime, &bytes)
                        .map(|att| (att, thumb))
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.uploading = false;
                if let Some((att, thumb)) = uploaded {
                    if let Some(img) = thumb {
                        let key = str_at(&att, "id").to_string();
                        this.attachment_thumbs.insert(key, img);
                    }
                    this.pending_attachments.push(att);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Attach-window rows: capture the CG window and upload it as an image
    /// attachment, mirroring upstream appSnap.captureWindow.
    pub(super) fn capture_window(
        &mut self,
        window_id: String,
        title: String,
        cx: &mut Context<Self>,
    ) {
        let num = window_id.rsplit(':').next().unwrap_or("0").to_string();
        let client = self.client.clone();
        if self.composer_thread_id.is_none() && self.selected_thread.is_none() {
            self.composer_thread_id = Some(uuid());
        }
        let thread_id = self
            .selected_thread
            .clone()
            .or_else(|| self.composer_thread_id.clone());
        self.uploading = true;
        cx.notify();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let tid = thread_id.unwrap_or_default();
            let uploaded: Option<(Value, Option<std::sync::Arc<RenderImage>>)> = cx
                .background_executor()
                .spawn(async move {
                    let Some(c) = ensure_client(&cell) else {
                        return None;
                    };
                    let path = std::env::temp_dir().join(format!("synara-win-{num}.png"));
                    let ok = std::process::Command::new("screencapture")
                        .args(["-x", "-l", &num])
                        .arg(&path)
                        .status()
                        .map(|s| s.success())
                        .unwrap_or(false);
                    if !ok {
                        return None;
                    }
                    let Ok(bytes) = std::fs::read(&path) else {
                        return None;
                    };
                    let _ = std::fs::remove_file(&path);
                    let name = if title.is_empty() {
                        "window.png".to_string()
                    } else {
                        format!("{title}.png")
                    };
                    let thumb = decode_bytes_to_render_image(&bytes);
                    c.upload_attachment(&tid, "image", &name, "image/png", &bytes)
                        .map(|att| (att, thumb))
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.uploading = false;
                if let Some((att, thumb)) = uploaded {
                    if let Some(img) = thumb {
                        let key = str_at(&att, "id").to_string();
                        this.attachment_thumbs.insert(key, img);
                    }
                    this.pending_attachments.push(att);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Blocking RPC: list one directory level (dirs + files).
    /// Draft typing hook: keep the "@" mention browser pointed at the
    /// directory the query resolves to (upstream useComposerDiscovery).
    pub(super) fn on_composer_change(&mut self, cx: &mut Context<Self>) {
        let draft = self.composer.read(cx).value().to_string();
        if let Some(dir) = mention_browse_dir(&draft) {
            if dir != self.mention_dir {
                self.load_mention_dir(dir, cx);
            }
        }
        cx.notify();
    }

    /// Dispatch a simple thread command (interrupt, archive, delete, ...).
    pub(super) fn dispatch_thread_cmd(&mut self, cmd_type: &'static str, cx: &mut Context<Self>) {
        let Some(tid) = self.selected_thread.clone() else {
            return;
        };
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let cmd = json!({
                "type": cmd_type,
                "commandId": uuid(),
                "threadId": tid,
                "createdAt": now_iso(),
            });
            let r = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell).and_then(|c| c.call("orchestration.dispatchCommand", cmd))
                })
                .await;
            if r.is_none() {
                eprintln!("native: {cmd_type} dispatch failed");
            }
            let cell = client.clone();
            let s = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell)
                        .and_then(|c| c.call("orchestration.getShellSnapshot", json!({})))
                })
                .await;
            let cell = client.clone();
            let p = json!({"threadId": tid});
            let d = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell)
                        .and_then(|c| c.call("orchestration.getThreadDetailSnapshot", p))
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if let Some(s) = s {
                    this.snapshot = Some(s);
                }
                if let Some(d) = d {
                    this.detail = Some(d);
                    this.prefetch_md_images(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Dispatch thread.approval.respond for a pending interaction.
    pub(super) fn respond_approval(
        &mut self,
        request_id: String,
        decision: &str,
        cx: &mut Context<Self>,
    ) {
        let Some(tid) = self.selected_thread.clone() else {
            return;
        };
        let client = self.client.clone();
        let decision = decision.to_string();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let cmd = json!({
                "type": "thread.approval.respond",
                "commandId": uuid(),
                "threadId": tid,
                "requestId": request_id,
                "decision": decision,
                "createdAt": now_iso(),
            });
            let r = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell).and_then(|c| c.call("orchestration.dispatchCommand", cmd))
                })
                .await;
            if r.is_none() {
                eprintln!("native: approval.respond dispatch failed");
            }
            let cell = client.clone();
            let p = json!({"threadId": tid});
            let d = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell)
                        .and_then(|c| c.call("orchestration.getThreadDetailSnapshot", p))
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if let Some(d) = d {
                    this.detail = Some(d);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Answer an async user-input card. Upstream (useAsyncUserInputResponse)
    /// steers a turn whose message is formatAsyncUserInputResponse and whose
    /// asyncUserInputResponse binds the answers to the question message.
    pub(super) fn respond_user_input(
        &mut self,
        msg_id: String,
        questions: Vec<String>,
        answers: Vec<String>,
        cx: &mut Context<Self>,
    ) {
        let Some(tid) = self.selected_thread.clone() else {
            return;
        };
        let client = self.client.clone();
        let mode = self.runtime_mode;
        let imode = self.interaction_mode;
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let text = questions
                .iter()
                .zip(answers.iter())
                .map(|(q, a)| format!("{q}\n{a}"))
                .collect::<Vec<_>>()
                .join("\n\n");
            let cell = client.clone();
            let t2 = tid.clone();
            let cmd = json!({
                "type": "thread.turn.start",
                "commandId": uuid(),
                "threadId": t2,
                "message": {
                    "messageId": uuid(),
                    "role": "user",
                    "text": text,
                    "attachments": [],
                },
                "asyncUserInputResponse": {
                    "messageId": msg_id,
                    "answers": answers,
                },
                "dispatchMode": "steer",
                "runtimeMode": mode,
                "interactionMode": imode,
                "createdAt": now_iso(),
            });
            let r = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell).and_then(|c| c.call("orchestration.dispatchCommand", cmd))
                })
                .await;
            if r.is_none() {
                eprintln!("native: user-input respond dispatch failed");
            }
            let cell = client.clone();
            let p = json!({"threadId": tid});
            let d = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell)
                        .and_then(|c| c.call("orchestration.getThreadDetailSnapshot", p))
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if let Some(d) = d {
                    this.detail = Some(d);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Toggle one option of a blocking userInput question (upstream
    /// togglePendingUserInputOptionSelection): single-select replaces, a
    /// multi-select question keeps a set.
    pub(super) fn pi_toggle(
        &mut self,
        rid: &str,
        qid: &str,
        label: &str,
        multi: bool,
        cx: &mut Context<Self>,
    ) {
        if self.pi_responding {
            return;
        }
        let draft = self
            .pi_drafts
            .entry(rid.to_string())
            .or_default()
            .entry(qid.to_string())
            .or_default();
        draft.custom.clear();
        if multi {
            if let Some(i) = draft.options.iter().position(|o| o == label) {
                draft.options.remove(i);
            } else {
                draft.options.push(label.to_string());
            }
        } else {
            draft.options = vec![label.to_string()];
        }
        cx.notify();
    }

    /// Advance the blocking userInput form (upstream
    /// onAdvanceActivePendingUserInput): next question, or submit when the
    /// last one resolves the whole answers record.
    pub(super) fn pi_advance(
        &mut self,
        rid: &str,
        questions: &[Value],
        gen: Option<String>,
        cx: &mut Context<Self>,
    ) {
        if self.pi_responding {
            return;
        }
        let idx = *self.pi_qidx.get(rid).unwrap_or(&0);
        let last = idx + 1 >= questions.len();
        if last {
            let Some(answers) = pi_build_answers(questions, self.pi_drafts.get(rid)) else {
                return;
            };
            self.pi_respond(rid.to_string(), gen, answers, cx);
        } else {
            // Upstream enables "next" only once the active question resolves.
            let q = &questions[idx];
            let draft = self.pi_drafts.get(rid).and_then(|m| m.get(str_at(q, "id")));
            if pi_resolved(q, draft).is_none() {
                return;
            }
            self.pi_qidx.insert(rid.to_string(), idx + 1);
            cx.notify();
        }
    }

    /// Respond to a blocking userInput prompt (upstream
    /// onRespondToUserInput): dispatch thread.user-input.respond, then
    /// refresh the thread detail so the settled interaction drops out.
    pub(super) fn pi_respond(
        &mut self,
        rid: String,
        gen: Option<String>,
        answers: Value,
        cx: &mut Context<Self>,
    ) {
        let Some(tid) = self.selected_thread.clone() else {
            return;
        };
        if self.pi_responding {
            return;
        }
        self.pi_responding = true;
        cx.notify();
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cmd = json!({
                "type": "thread.user-input.respond",
                "commandId": uuid(),
                "threadId": tid,
                "requestId": rid,
                "answers": answers,
                "createdAt": now_iso(),
            });
            if let Some(g) = gen {
                cmd["lifecycleGeneration"] = json!(g);
            }
            let cell = client.clone();
            let r = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell).and_then(|c| c.call("orchestration.dispatchCommand", cmd))
                })
                .await;
            if r.is_none() {
                eprintln!("native: thread.user-input.respond dispatch failed");
            }
            let cell = client.clone();
            let p = json!({"threadId": tid});
            let d = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell)
                        .and_then(|c| c.call("orchestration.getThreadDetailSnapshot", p))
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.pi_responding = false;
                if let Some(d) = d {
                    this.detail = Some(d);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Upstream stop affordance: dispatch thread.turn.interrupt and hold the
    /// queued follow-ups (upstream pauses the waiting queue on a user Stop).
    pub(super) fn stop(&mut self, cx: &mut Context<Self>) {
        let Some(tid) = self.selected_thread.clone() else {
            return;
        };
        if !self.queued_turns.is_empty() {
            self.queue_paused = true;
        }
        let client = self.client.clone();
        let cmd = json!({
            "type": "thread.turn.interrupt",
            "requestedBy": "user",
            "commandId": uuid(),
            "threadId": tid,
            "createdAt": now_iso(),
        });
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            if let Some(client) = client {
                let c = client.clone();
                let _ = cx
                    .background_executor()
                    .spawn(async move { c.call("orchestration.dispatchCommand", cmd) })
                    .await;
                let c = client.clone();
                let p = json!({"threadId": tid});
                let d = cx
                    .background_executor()
                    .spawn(async move { c.call("orchestration.getThreadDetailSnapshot", p) })
                    .await;
                let _ = this.update(cx, |this: &mut Self, cx| {
                    this.detail = d;
                    cx.notify();
                });
            }
        })
        .detach();
        cx.notify();
    }

    /// Transcript surface shared by the primary pane and the split pane.
    /// `detail` is the thread detail snapshot (may be null); `has_thread`
    /// and `show_landing` mirror the caller's state for the empty row.
    pub(super) fn transcript_body(
        &self,
        detail: &Value,
        has_thread: bool,
        show_landing: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let theme = cx.theme().clone();
        let dark_ui = match self.appearance_mode {
            2 => true,
            1 => false,
            _ => matches!(
                cx.window_appearance(),
                gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
            ),
        };
        let subtle = if dark_ui {
            rgba(0xffffff14)
        } else {
            rgba(0x0000000d)
        };
        // Upstream fills are neutral, not the navy of gpui's default
        // list_active: selected rows = white 4%, user bubble = #181818.
        let sel_fill: gpui::Hsla = if dark_ui {
            rgba(0xffffff0a).into()
        } else {
            theme.list_active
        };
        // Upstream light bubble: rgba(13,13,13,0.03) — neutral ink, not the
        // list-selection tint.
        let user_bubble: gpui::Hsla = if dark_ui {
            rgb(0x181818).into()
        } else {
            gpui::hsla(0.0, 0.0, 0.05, 0.03)
        };
        let mut col = v_flex().gap_2();
        let thread_v = get(detail, "thread");
        let messages = arr(thread_v, "messages");
        let find_q = self.find_query.to_lowercase();
        let chat_mw = self.chat_max_w();
        if messages.is_empty() && has_thread && !show_landing {
            col = col.child(
                h_flex().w_full().justify_center().child(
                    div().w_full().max_w(chat_mw).child(
                        div()
                            .text_size(px(13.))
                            .text_color(theme.muted_foreground)
                            .child("Select a thread or type below"),
                    ),
                ),
            );
        }
        for (i, m) in messages.iter().enumerate() {
            let role = str_at(m, "role");
            let mut text = str_at(m, "text").to_string();
            if text.trim().is_empty() {
                text = get(m, "textSegments")
                    .as_array()
                    .map(|segs| {
                        segs.iter()
                            .filter_map(|s| s.get("text").and_then(Value::as_str))
                            .collect::<Vec<_>>()
                            .join("")
                    })
                    .unwrap_or_default();
            }
            // Async user-input card (upstream AsyncUserInputCard): question
            // rows with option picks, submitted as a steered turn.
            let aui = get(m, "asyncUserInput");
            let questions = arr(aui, "questions");
            if !questions.is_empty() {
                let msg_id = str_at(m, "id").to_string();
                let done: Vec<String> = arr(get(aui, "response"), "answers")
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect();
                let answered = !done.is_empty();
                let n = questions.len();
                // Upstream CollapsibleTrigger: standalone rounded-full pill,
                // panel only after a click (default closed for both states).
                let open = self.user_input_open.contains(&msg_id);

                let mut card = v_flex().w_full().gap(px(8.));
                let mut head = h_flex()
                    .id(ElementId::Name(format!("aui-pill-{msg_id}").into()))
                    .gap_2()
                    .items_center()
                    .flex_shrink_0()
                    .border_1()
                    .border_color(theme.border)
                    .rounded_full()
                    .px(px(10.))
                    .py(px(4.))
                    .cursor_pointer()
                    .child(
                        svg_icon(icons::HELP)
                            .size(px(14.))
                            .text_color(theme.muted_foreground),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(theme.muted_foreground)
                            .whitespace_nowrap()
                            .child(format!(
                                "{n} {}",
                                if n == 1 { "question" } else { "questions" }
                            )),
                    )
                    .on_mouse_down(MouseButton::Left, {
                        let mid = msg_id.clone();
                        cx.listener(move |this, _e, _w, cx| {
                            if this.user_input_open.contains(&mid) {
                                this.user_input_open.retain(|i| i != &mid);
                            } else {
                                this.user_input_open.push(mid.clone());
                            }
                            cx.notify();
                        })
                    });
                if answered {
                    head = head
                        .child(
                            svg_icon(icons::CHECK)
                                .size(px(12.))
                                .text_color(theme.muted_foreground),
                        )
                        .child(
                            div()
                                .text_size(px(13.))
                                .text_color(theme.muted_foreground)
                                .child("Answered"),
                        );
                }
                card = card.child(h_flex().child(head).child(div().flex_1()));
                if !open {
                    col = col.child(
                        h_flex()
                            .w_full()
                            .justify_center()
                            .child(div().w_full().max_w(chat_mw).child(card)),
                    );
                    continue;
                }
                let mut panel = v_flex()
                    .w_full()
                    .gap_3()
                    .border_1()
                    .border_color(theme.border)
                    .rounded(px(12.))
                    .px(px(14.))
                    .py(px(14.));

                if answered {
                    for (qi, q) in questions.iter().enumerate() {
                        let mut qa = v_flex().gap_1().child(
                            div()
                                .text_size(px(13.))
                                .font_weight(FontWeight::MEDIUM)
                                .child(str_at(q, "title").to_string()),
                        );
                        if let Some(a) = done.get(qi) {
                            qa = qa.child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(theme.muted_foreground)
                                    .child(a.clone()),
                            );
                        }
                        panel = panel.child(qa);
                    }
                } else {
                    // Effective pick per question: draft override, else the
                    // first option (upstream preselects options[0]).
                    let mut all_answerable = true;
                    for (qi, q) in questions.iter().enumerate() {
                        let title = str_at(q, "title").to_string();
                        let options: Vec<String> = arr(q, "options")
                            .iter()
                            .filter_map(Value::as_str)
                            .map(str::to_string)
                            .collect();
                        let pick = self
                            .user_input_draft
                            .iter()
                            .find(|(id, i, _)| id == &msg_id && *i == qi)
                            .map(|(_, _, l)| l.clone())
                            .or_else(|| options.first().cloned());
                        let mut qv = v_flex().gap_2().child(
                            div()
                                .text_size(px(13.))
                                .font_weight(FontWeight::MEDIUM)
                                .child(title),
                        );
                        if options.is_empty() {
                            all_answerable = false;
                            qv = qv.child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(theme.muted_foreground)
                                    .child("Type your answer in the composer below"),
                            );
                        } else {
                            let mut orow = h_flex().gap_2().flex_wrap();
                            for opt in options {
                                let chosen = pick.as_deref() == Some(opt.as_str());
                                let mid2 = msg_id.clone();
                                let opt2 = opt.clone();
                                let mut ob = div()
                                    .id(ElementId::Name(
                                        format!("uinput-{msg_id}-{qi}-{opt}").into(),
                                    ))
                                    .px(px(10.))
                                    .py(px(4.))
                                    .rounded(px(999.))
                                    .border_1()
                                    .cursor_pointer()
                                    .text_size(px(12.))
                                    .child(opt);
                                if chosen {
                                    ob = ob
                                        .border_color(theme.foreground)
                                        .bg(sel_fill)
                                        .text_color(theme.foreground);
                                } else {
                                    ob = ob
                                        .border_color(theme.border)
                                        .text_color(theme.muted_foreground)
                                        .hover(|d| d.bg(rgba(0x00000005)));
                                }
                                ob = ob.on_click(cx.listener(move |this, _ev, _w, cx| {
                                    this.user_input_draft
                                        .retain(|(id, i, _)| !(id == &mid2 && *i == qi));
                                    this.user_input_draft.push((mid2.clone(), qi, opt2.clone()));
                                    cx.notify();
                                }));
                                orow = orow.child(ob);
                            }
                            qv = qv.child(orow);
                        }
                        panel = panel.child(qv);
                    }
                    // Footer: hint + Send (disabled until every question's
                    // effective pick exists).
                    let sendable = all_answerable;
                    let mut foot = h_flex().items_center().justify_between().child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.muted_foreground)
                            .child("The agent can keep working"),
                    );
                    if sendable {
                        let mid3 = msg_id.clone();
                        let qs: Vec<String> = questions
                            .iter()
                            .map(|q| str_at(q, "title").to_string())
                            .collect();
                        let answers: Vec<String> = questions
                            .iter()
                            .enumerate()
                            .map(|(qi, q)| {
                                self.user_input_draft
                                    .iter()
                                    .find(|(id, i, _)| id == &msg_id && *i == qi)
                                    .map(|(_, _, l)| l.clone())
                                    .or_else(|| {
                                        arr(q, "options")
                                            .first()
                                            .and_then(Value::as_str)
                                            .map(str::to_string)
                                    })
                                    .unwrap_or_default()
                            })
                            .collect();
                        foot = foot.child(
                            Button::new(ElementId::Name(format!("uinput-send-{mid3}").into()))
                                .primary()
                                .label("Send answer")
                                .on_click(cx.listener(move |this, _ev, _w, cx| {
                                    this.respond_user_input(
                                        mid3.clone(),
                                        qs.clone(),
                                        answers.clone(),
                                        cx,
                                    );
                                })),
                        );
                    } else {
                        foot = foot.child(
                            div()
                                .px(px(10.))
                                .py(px(4.))
                                .rounded(px(8.))
                                .text_size(px(12.))
                                .text_color(theme.muted_foreground.opacity(0.5))
                                .child("Send answer"),
                        );
                    }
                    panel = panel.child(foot);
                }
                card = card.child(panel);
                col = col.child(
                    h_flex()
                        .w_full()
                        .justify_center()
                        .child(div().w_full().max_w(chat_mw).child(card)),
                );
                continue;
            }

            if text.trim().is_empty() {
                continue;
            }
            let is_match = !find_q.is_empty() && text.to_lowercase().contains(&find_q);

            if role == "user" {
                let mut pill = div()
                    .bg(user_bubble)
                    .rounded(px(16.))
                    .px(px(14.))
                    .py(px(10.))
                    // Upstream caps the bubble at 80% of the transcript
                    // column, not a fixed width.
                    .max_w(chat_mw * 0.8);
                if is_match {
                    pill = pill.border_1().border_color(theme.warning);
                }
                // Upstream user-message footer: right-aligned timestamp
                // (hover-reveal) + copy / edit (pencil) / revert (undo).
                let mut ufooter = h_flex().mt(px(4.)).gap(px(2.)).items_center().justify_end();
                let uts = clock_time(str_at(m, "createdAt"));
                if !uts.is_empty() {
                    ufooter = ufooter.child(
                        div()
                            .mr(px(4.))
                            .text_size(px(11.))
                            .text_color(theme.muted_foreground.opacity(0.45))
                            .child(uts),
                    );
                }
                let msg_id = str_at(m, "id").to_string();
                // Upstream revert target: the checkpoint of the turn the
                // next assistant message belongs to, minus one.
                let revert_tc: Option<i64> = (|| {
                    let turn_id = messages
                        .iter()
                        .skip(i + 1)
                        .find(|nm| str_at(nm, "role") == "assistant")
                        .map(|nm| str_at(nm, "turnId").to_string())
                        .filter(|t| !t.is_empty())?;
                    arr(thread_v, "checkpoints")
                        .iter()
                        .find(|c| str_at(c, "turnId") == turn_id)
                        .and_then(|c| c.get("checkpointTurnCount").and_then(Value::as_i64))
                        .map(|tc| (tc - 1).max(0))
                })();
                // Upstream shows the pencil only when
                // resolveTailUserMessageEditTarget would allow it: the
                // latest native-source user message whose tail spans at
                // most one turn (or carries the currently running turn).
                let msg_editable = {
                    let src = str_at(m, "source");
                    let native_src = src.is_empty() || src == "native";
                    let answered = messages.iter().any(|nm| {
                        str_at(get(get(nm, "asyncUserInput"), "response"), "messageId") == msg_id
                    });
                    let latest = messages.iter().rposition(|nm| {
                        str_at(nm, "role") == "user" && {
                            let s = str_at(nm, "source");
                            s.is_empty() || s == "native"
                        }
                    });
                    let sess = get(detail, "session");
                    let active_turn = str_at(sess, "status") == "running"
                        && !str_at(sess, "activeTurnId").is_empty();
                    let mut tail: Vec<&str> = Vec::new();
                    for nm in &messages[i..] {
                        let t = str_at(nm, "turnId");
                        if !t.is_empty() && !tail.contains(&t) {
                            tail.push(t);
                        }
                    }
                    native_src
                        && !answered
                        && latest == Some(i)
                        && (tail.len() == 1 || (tail.is_empty() && active_turn))
                };
                let mut uicons: Vec<(&'static str, &'static [u8])> =
                    vec![("umsg-copy", icons::COPY)];
                if msg_editable {
                    uicons.push(("umsg-edit", icons::NEW_THREAD));
                }
                if revert_tc.is_some() {
                    uicons.push(("umsg-revert", icons::UNDO));
                }
                for (iid, data) in uicons {
                    let msg_id2 = msg_id.clone();
                    let text2 = text.clone();
                    let tc = revert_tc;
                    ufooter = ufooter.child(
                        div()
                            .id(ElementId::Name(format!("{iid}-{msg_id}").into()))
                            .size(px(22.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(6.))
                            .cursor_pointer()
                            .hover(|d| d.bg(subtle))
                            .on_click(cx.listener(move |this, _ev, w, cx| match iid {
                                "umsg-edit" => {
                                    this.editing_message_id = Some(msg_id2.clone());
                                    this.composer.update(cx, |s, cx| {
                                        s.set_value(text2.clone(), w, cx);
                                    });
                                    let f = this.composer.read(cx).focus_handle(cx);
                                    w.focus(&f, cx);
                                    cx.notify();
                                }
                                "umsg-revert" => {
                                    if let Some(tc) = tc {
                                        this.pending_revert = Some((msg_id2.clone(), tc));
                                        cx.notify();
                                    }
                                }
                                _ => {
                                    cx.write_to_clipboard(ClipboardItem::new_string(text2.clone()));
                                }
                            }))
                            .child(
                                svg_icon(data)
                                    .size(px(13.))
                                    .text_color(theme.muted_foreground.opacity(0.45)),
                            ),
                    );
                }
                // Upstream UserDispatchModeChip: the marker sits above the
                // sent-message stack, right aligned (automation > agent >
                // steer precedence).
                let marker = match str_at(m, "dispatchOrigin") {
                    "automation" => Some(("Sent via Automation", icons::CLOCK)),
                    "agent" => Some(("Sent by agent", icons::BOT)),
                    _ => {
                        if str_at(m, "dispatchMode") == "steer" {
                            Some(("Steering conversation", icons::STEER))
                        } else {
                            None
                        }
                    }
                };
                let mut stack = v_flex().items_end();
                if let Some((mlabel, micon)) = marker {
                    stack = stack.child(
                        h_flex()
                            .mb(px(6.))
                            .gap(px(6.))
                            .items_center()
                            .child(
                                svg_icon(micon)
                                    .size(px(12.))
                                    .text_color(theme.muted_foreground.opacity(0.75)),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(theme.muted_foreground.opacity(0.78))
                                    .child(mlabel),
                            ),
                    );
                }
                let msg_hover_id = msg_id.clone();
                let msg_hover_id2 = msg_id.clone();
                let reveal = if self.hover_user_msg.as_deref() == Some(msg_id.as_str()) {
                    1.0f32
                } else {
                    0.0f32
                };
                col = col.child(
                    h_flex().w_full().justify_center().child(
                        h_flex().w_full().max_w(chat_mw).justify_end().child(
                            div()
                                .id(ElementId::Name(format!("umsg-hover-{msg_id}").into()))
                                .w_full()
                                .on_hover(cx.listener(move |this, hovered: &bool, _w, cx| {
                                    if *hovered {
                                        if this.hover_user_msg.as_deref()
                                            != Some(msg_hover_id.as_str())
                                        {
                                            this.hover_user_msg = Some(msg_hover_id.clone());
                                            cx.notify();
                                        }
                                    } else if this.hover_user_msg.as_deref()
                                        == Some(msg_hover_id2.as_str())
                                    {
                                        this.hover_user_msg = None;
                                        cx.notify();
                                    }
                                }))
                                .child(
                                    stack
                                        .child(pill.child({
                                            // User attachments (upstream UserMessageImageAttachments
                                            // + file cards): image thumbnails row, then file and
                                            // assistant-selection chips, then the text.
                                            let mut content = v_flex().gap(px(8.));
                                            let msg_atts = arr(m, "attachments");
                                            let img_atts: Vec<&Value> = msg_atts
                                                .iter()
                                                .filter(|a| str_at(a, "type") == "image")
                                                .collect();
                                            // Upstream transcript attachments render as
                                            // compact pills: tiny icon, name, size ("25 B").
                                            // Upstream stacks attachment pills vertically,
                                            // one right-aligned row per attachment.
                                            let mut pill_row = v_flex().gap(px(6.)).items_end();
                                            let n_sel = msg_atts
                                                .iter()
                                                .filter(|a| {
                                                    str_at(a, "type") == "assistant-selection"
                                                })
                                                .count();
                                            let mut has_pills = false;
                                            if n_sel > 0 {
                                                has_pills = true;
                                                pill_row = pill_row.child(
                                                    h_flex()
                                                        .gap(px(5.))
                                                        .items_center()
                                                        .rounded_full()
                                                        .border_1()
                                                        .border_color(theme.border)
                                                        .px(px(8.))
                                                        .py(px(3.))
                                                        .child(
                                                            svg_icon(icons::COPY)
                                                                .size(px(11.))
                                                                .text_color(theme.muted_foreground),
                                                        )
                                                        .child(
                                                            div()
                                                                .text_size(px(12.))
                                                                .text_color(theme.muted_foreground)
                                                                .child(format!(
                                                                    "{} selection{}",
                                                                    n_sel,
                                                                    if n_sel == 1 {
                                                                        ""
                                                                    } else {
                                                                        "s"
                                                                    }
                                                                )),
                                                        ),
                                                );
                                            }
                                            for a in msg_atts
                                                .iter()
                                                .filter(|a| str_at(a, "type") == "file")
                                            {
                                                let name = str_at(a, "name").to_string();
                                                let bytes = a
                                                    .get("sizeBytes")
                                                    .and_then(Value::as_u64)
                                                    .unwrap_or(0);
                                                let size_label = if bytes < 1024 {
                                                    format!("{bytes} B")
                                                } else if bytes < 1024 * 1024 {
                                                    format!("{} KB", bytes / 1024)
                                                } else {
                                                    format!("{:.1} MB", bytes as f64 / 1048576.0)
                                                };
                                                has_pills = true;
                                                pill_row = pill_row.child(
                                                    h_flex()
                                                        .gap(px(5.))
                                                        .items_center()
                                                        .rounded_full()
                                                        .border_1()
                                                        .border_color(theme.border)
                                                        .px(px(8.))
                                                        .py(px(3.))
                                                        .child(
                                                            svg_icon(icons::PAPERCLIP)
                                                                .size(px(11.))
                                                                .text_color(theme.muted_foreground),
                                                        )
                                                        .child(
                                                            div()
                                                                .text_size(px(12.))
                                                                .whitespace_nowrap()
                                                                .overflow_hidden()
                                                                .child(name),
                                                        )
                                                        .child(
                                                            div()
                                                                .text_size(px(12.))
                                                                .text_color(theme.muted_foreground)
                                                                .child(size_label),
                                                        ),
                                                );
                                            }
                                            if has_pills {
                                                content = content.child(pill_row);
                                            }
                                            if !img_atts.is_empty() {
                                                let mut row = h_flex().gap(px(8.)).flex_wrap();
                                                for a in &img_atts {
                                                    let aid = str_at(a, "id").to_string();
                                                    let cached = crate::ws_client::BACKEND_BASE
                                                        .get()
                                                        .and_then(|base| {
                                                            self.md_images
                                                                .lock()
                                                                .unwrap()
                                                                .get(&format!(
                                                                    "{base}/attachments/{aid}"
                                                                ))
                                                                .cloned()
                                                        });
                                                    row = row.child(match cached {
                                                        Some(render) => div()
                                                            .rounded(px(8.))
                                                            .overflow_hidden()
                                                            .child(
                                                                img(ImageSource::Render(render))
                                                                    .h(px(96.))
                                                                    .rounded(px(8.)),
                                                            )
                                                            .into_any_element(),
                                                        None => div()
                                                            .size(px(96.))
                                                            .rounded(px(8.))
                                                            .bg(subtle)
                                                            .into_any_element(),
                                                    });
                                                }
                                                content = content.child(row);
                                            }
                                            content.child(div().text_size(px(13.)).child(text))
                                        }))
                                        .child(ufooter.opacity(reveal)),
                                ),
                        ),
                    ),
                );
            } else {
                // Upstream transcript body: 13px chat font at 1.625 line-height
                // (measured computed web values); the markdown TextView inherits both.
                let mut wrap = div().text_size(px(13.)).line_height(relative(1.625));
                if is_match {
                    wrap = wrap.border_l_2().border_color(theme.warning).pl_2();
                }
                // Footer row under a settled assistant message (upstream):
                // copy / fork / pin icon buttons followed by the wall-clock time.
                let mut footer = h_flex().mt(px(4.)).gap(px(2.)).items_center();
                let msg_id = str_at(m, "id").to_string();
                let pinned_ids: Vec<String> = arr(thread_v, "pinnedMessages")
                    .iter()
                    .map(|p| str_at(p, "messageId").to_string())
                    .collect();
                for (iid, data) in [
                    ("msg-copy", icons::COPY),
                    ("msg-fork", icons::BRANCH_SIMPLE),
                    ("msg-pin", icons::PIN),
                ] {
                    let msg_id2 = msg_id.clone();
                    let text2 = text.clone();
                    let is_pinned = pinned_ids.contains(&msg_id2);
                    footer = footer.child(
                        div()
                            .id(ElementId::Name(format!("{iid}-{msg_id}").into()))
                            .size(px(22.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(6.))
                            .cursor_pointer()
                            .hover(|d| d.bg(subtle))
                            .on_click(cx.listener(move |this, _ev, _w, cx| match iid {
                                "msg-fork" => {
                                    if let Some(tid) = this.selected_thread.clone() {
                                        this.lineage(tid, false, None, cx);
                                    }
                                }
                                "msg-pin" => {
                                    if let Some(tid) = this.selected_thread.clone() {
                                        let op = if is_pinned {
                                            "thread.pinned-message.remove"
                                        } else {
                                            "thread.pinned-message.add"
                                        };
                                        let cell = this.client.clone();
                                        let mid = msg_id2.clone();
                                        cx.background_executor()
                                            .spawn(async move {
                                                let Some(client) = ensure_client(&cell) else {
                                                    return;
                                                };
                                                client.call(
                                                    "orchestration.dispatchCommand",
                                                    json!({
                                                        "type": op,
                                                        "commandId": uuid(),
                                                        "threadId": tid,
                                                        "messageId": mid,
                                                        "createdAt": now_iso(),
                                                    }),
                                                );
                                            })
                                            .detach();
                                    }
                                }
                                _ => {
                                    cx.write_to_clipboard(ClipboardItem::new_string(text2.clone()));
                                }
                            }))
                            .child(svg_icon(data).size(px(13.)).text_color(
                                if is_pinned && iid == "msg-pin" {
                                    theme.foreground
                                } else {
                                    theme.muted_foreground
                                },
                            )),
                    );
                }
                let ts = clock_time(str_at(m, "createdAt"));
                if !ts.is_empty() {
                    footer = footer.child(
                        div()
                            .ml(px(4.))
                            .text_size(px(11.))
                            .text_color(theme.muted_foreground)
                            .child(ts),
                    );
                }
                col = col.child(
                    h_flex().w_full().justify_center().child(
                        div().w_full().max_w(chat_mw).child(
                            v_flex()
                                .child(
                                    wrap.child(
                                        TextView::markdown(
                                            ElementId::Name(
                                                format!("md-{i}-{}", str_at(m, "id")).into(),
                                            ),
                                            text,
                                        )
                                        .image_source({
                                            let cache = self.md_images.clone();
                                            move |uri: &SharedUri| -> ImageSource {
                                                let key = uri.as_str();
                                                if let Some(img) =
                                                    cache.lock().unwrap().get(key).cloned()
                                                {
                                                    ImageSource::Render(img)
                                                } else if key.starts_with("http://")
                                                    || key.starts_with("https://")
                                                {
                                                    // Remote image still in flight: perpetual-loading
                                                    // custom source; the cache fill re-renders.
                                                    ImageSource::Custom(std::sync::Arc::new(
                                                        |_w, _cx| None,
                                                    ))
                                                } else {
                                                    uri.clone().into()
                                                }
                                            }
                                        })
                                        .style(markdown_style(theme.is_dark()))
                                        .code_block_actions({
                                            let muted_fg = theme.muted_foreground;
                                            move |cb, _w, _cx| {
                                                let lang = cb
                                                    .lang()
                                                    .map(|l| l.to_string())
                                                    .unwrap_or_default();
                                                h_flex()
                                                    .items_center()
                                                    .justify_between()
                                                    .child(
                                                        div()
                                                            .text_size(px(11.))
                                                            .text_color(muted_fg)
                                                            .child(lang),
                                                    )
                                                    .child(
                                                        h_flex()
                                                            .gap(px(2.))
                                                            .child(
                                                                svg_icon(icons::COPY)
                                                                    .size(px(14.))
                                                                    .text_color(muted_fg),
                                                            )
                                                            .child(
                                                                svg_icon(icons::TEXT_WRAP)
                                                                    .size(px(14.))
                                                                    .text_color(muted_fg),
                                                            ),
                                                    )
                                            }
                                        })
                                        .selectable(true),
                                    ),
                                )
                                .child(footer),
                        ),
                    ),
                );
            }
        }

        /// Verb-summary aggregation for a run of tool activities, in the grammar the
        /// web client's ToolCallGroupSummaryRow uses: verb phrases in the order the
        /// work happened ("searched 1 issue, read 2 files") with failed and running
        /// fragments appended. Kinds map to verbs loosely — the server owns the exact
        /// taxonomy, the shell only needs stable buckets.
        fn activity_verb(kind: &str) -> &'static str {
            let k = kind.to_lowercase();
            if k.contains("read") || k.contains("open") {
                "read"
            } else if k.contains("search") || k.contains("find") || k.contains("grep") {
                "searched"
            } else if k.contains("edit") || k.contains("write") || k.contains("patch") {
                "edited"
            } else if k.contains("command")
                || k.contains("shell")
                || k.contains("exec")
                || k.contains("terminal")
                || k.contains("run")
            {
                "ran"
            } else if k.contains("agent") || k.contains("task") {
                "delegated"
            } else if k.contains("fetch") || k.contains("web") || k.contains("http") {
                "fetched"
            } else if k.contains("think") || k.contains("reason") || k.contains("plan") {
                "planned"
            } else {
                "used"
            }
        }

        fn activity_noun(verb: &str, count: usize) -> String {
            let noun = match verb {
                "read" => {
                    if count == 1 {
                        "file"
                    } else {
                        "files"
                    }
                }
                "searched" => {
                    if count == 1 {
                        "search"
                    } else {
                        "searches"
                    }
                }
                "edited" => {
                    if count == 1 {
                        "file"
                    } else {
                        "files"
                    }
                }
                "ran" => {
                    if count == 1 {
                        "command"
                    } else {
                        "commands"
                    }
                }
                "delegated" => {
                    if count == 1 {
                        "agent"
                    } else {
                        "agents"
                    }
                }
                "fetched" => {
                    if count == 1 {
                        "page"
                    } else {
                        "pages"
                    }
                }
                "planned" => {
                    if count == 1 {
                        "plan"
                    } else {
                        "plans"
                    }
                }
                _ => {
                    if count == 1 {
                        "tool"
                    } else {
                        "tools"
                    }
                }
            };
            format!("{count} {noun}")
        }

        /// "Searched 1 issue, read 2 files" — verb phrases in first-seen order.
        fn tool_group_verb_summary(activities: &[Value]) -> String {
            let mut order: Vec<&'static str> = Vec::new();
            let mut counts: std::collections::HashMap<&'static str, usize> =
                std::collections::HashMap::new();
            for a in activities {
                let verb = activity_verb(str_at(a, "kind"));
                if !order.contains(&verb) {
                    order.push(verb);
                }
                *counts.entry(verb).or_insert(0) += 1;
            }
            let parts: Vec<String> = order
                .iter()
                .map(|verb| format!("{} {}", verb, activity_noun(verb, counts[verb])))
                .collect();
            if parts.is_empty() {
                return format!("{} actions", activities.len());
            }
            let mut out = parts.join(", ");
            if let Some(pos) = out.find(char::is_alphabetic) {
                let upper: String = out[pos..].chars().next().unwrap().to_uppercase().collect();
                out.replace_range(pos..pos + 1, &upper);
            }
            out
        }

        // Activities grouped in a collapsible tool-call card (web parity):
        // a single activity stays a plain row; several collapse into a card.
        let activities: Vec<Value> = arr(thread_v, "activities")
            .iter()
            .filter(|a| !str_at(a, "kind").is_empty() || !str_at(a, "summary").is_empty())
            .cloned()
            .collect();
        // Background tasks owned by this transcript (upstream workLog fold).
        let bg_tasks = derive_bg_tasks(&thread_v);
        let this_tid = str_at(thread_v, "id").to_string();
        // Upstream BackgroundTaskRow: tray icon + subject + status + elapsed,
        // Stop for running rows.
        let bg_task_row = |t: &BgTask| {
            let status_label = match t.status.as_str() {
                "running" => "running".to_string(),
                "finished" => "finished".to_string(),
                "failed" => match t.exit_code {
                    Some(c) => format!("failed · exit {c}"),
                    None => "failed".to_string(),
                },
                "stopped" => "stopped".to_string(),
                _ => t.status.clone(),
            };
            let elapsed = clock_elapsed(&t.started_at, t.completed_at.as_deref().unwrap_or(""));
            let running = t.status == "running";
            let stop_tid = t.task_id.clone();
            let stop_thread = this_tid.clone();
            h_flex()
                .w_full()
                .gap_2()
                .items_center()
                .py(px(2.))
                .child(
                    svg_icon(icons::TERMINAL)
                        .size(px(14.))
                        .text_color(theme.muted_foreground),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(theme.muted_foreground)
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .child(t.title.clone()),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(if t.status == "failed" {
                            theme.danger
                        } else {
                            theme.muted_foreground
                        })
                        .child(status_label),
                )
                .when_some(elapsed, |d, e| {
                    d.child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.muted_foreground)
                            .child(e),
                    )
                })
                .when(running, |d| {
                    d.child(
                        div()
                            .id(SharedString::from(format!("tr-bgtask-stop-{}", stop_tid)))
                            .px_1()
                            .rounded(px(4.))
                            .cursor_pointer()
                            .hover(|h| h.bg(theme.border))
                            .child(
                                svg_icon(icons::X)
                                    .size(px(11.))
                                    .text_color(theme.muted_foreground),
                            )
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                this.dispatch_cmd(
                                    json!({
                                        "type": "thread.task.stop",
                                        "commandId": uuid(),
                                        "threadId": stop_thread,
                                        "taskId": stop_tid,
                                        "createdAt": now_iso(),
                                    }),
                                    cx,
                                );
                            })),
                    )
                })
        };
        let open_acts = self.activity_open.clone();
        let activity_row = |a: &Value| {
            let kind = str_at(a, "kind");
            let summary = str_at(a, "summary");
            let color = match str_at(a, "tone") {
                "error" => theme.danger,
                _ => theme.muted_foreground,
            };
            // Upstream work rows render the activity summary only (the kind
            // is an internal discriminator, never user-facing).
            let label = if summary.is_empty() {
                kind.to_string()
            } else {
                summary.to_string()
            };
            // Upstream appends payload.detail after the summary (one muted
            // line, clipped at the card edge).
            let detail = str_at(get(a, "payload"), "detail").to_string();
            let mut row = h_flex()
                .gap_2()
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child("\u{25e6}"),
                )
                .child(div().text_xs().text_color(color).child(label));
            if !detail.is_empty() {
                row = row.child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .child(detail),
                );
            }
            // Click-to-expand detail: payload locations, content text, and
            // status — the native counterpart of upstream's per-entry detail
            // surface, rendered inline.
            let aid = str_at(a, "id").to_string();
            if !open_acts.iter().any(|x| x == &aid) {
                return v_flex().w_full().child(row).into_any_element();
            }
            let payload = get(a, "payload");
            let mut det = v_flex().w_full().gap(px(2.)).pl(px(16.)).py(px(2.));
            let status = str_at(payload, "status");
            let stext = str_at(a, "status");
            let status_line = if !status.is_empty() { status } else { stext };
            if !status_line.is_empty() {
                det = det.child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(format!("status: {}", status_line)),
                );
            }
            for loc in arr(payload, "locations").iter() {
                let p = str_at(loc, "path");
                if !p.is_empty() {
                    det = det.child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(p.to_string()),
                    );
                }
            }
            for c in arr(payload, "content").iter() {
                let inner = get(c, "content");
                let mut ct = str_at(inner, "text");
                // resource-type content carries the text one level deeper.
                if ct.is_empty() {
                    ct = str_at(get(inner, "resource"), "text");
                }
                if !ct.is_empty() {
                    det = det.child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(ct.to_string()),
                    );
                }
            }
            v_flex().w_full().child(row).child(det).into_any_element()
        };
        // Upstream withUserInputExchanges: resolved userInput pairs render as
        // Q→A bubble exchanges and consume their requested row.
        let (ui_exchanges, ui_consumed) = derive_ui_exchanges(&activities);
        let activities: Vec<Value> = activities
            .into_iter()
            .filter(|a| !ui_consumed.contains(str_at(a, "id")))
            .collect();
        // Runs of plain activities group into the collapsible actions card;
        // a resolved userInput pair breaks the run and renders its bubbles.
        let mut segments: Vec<Vec<Value>> = Vec::new();
        let mut run: Vec<Value> = Vec::new();
        for a in activities {
            if ui_exchanges.contains_key(str_at(&a, "id")) {
                if !run.is_empty() {
                    segments.push(std::mem::take(&mut run));
                }
                segments.push(vec![a]);
            } else {
                run.push(a);
            }
        }
        if !run.is_empty() {
            segments.push(run);
        }
        let n_plain_cards = segments.iter().filter(|s| s.len() > 1).count();
        let mut seen_cards = 0usize;
        for seg in &segments {
            let is_exchange = seg.len() == 1 && ui_exchanges.contains_key(str_at(&seg[0], "id"));
            if is_exchange {
                let aid = str_at(&seg[0], "id");
                let (qs, answers) = ui_exchanges.get(aid).unwrap();
                let mut exch = v_flex().w_full().max_w(chat_mw).gap_2().py(px(2.));
                for q in qs {
                    let opts: Vec<String> = arr(q, "options")
                        .iter()
                        .map(|o| str_at(o, "label").to_string())
                        .collect();
                    let answer =
                        ui_answer_text(answers.as_ref(), q).unwrap_or_else(|| "No answer".into());
                    exch = exch
                        .child(
                            h_flex().w_full().justify_start().child(
                                v_flex()
                                    .max_w(chat_mw * 0.85)
                                    .px(px(14.))
                                    .py(px(10.))
                                    .gap_1()
                                    .rounded(px(16.))
                                    .border_1()
                                    .border_dashed()
                                    .border_color(theme.border)
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(theme.muted_foreground)
                                            .child(str_at(q, "header").to_string()),
                                    )
                                    .child(div().text_sm().child(str_at(q, "question").to_string()))
                                    .when(!opts.is_empty(), |d| {
                                        d.child(
                                            div()
                                                .text_xs()
                                                .text_color(theme.muted_foreground)
                                                .child(opts.join(" · ")),
                                        )
                                    }),
                            ),
                        )
                        .child(
                            h_flex().w_full().justify_end().child(
                                v_flex()
                                    .max_w(chat_mw * 0.85)
                                    .px(px(14.))
                                    .py(px(10.))
                                    .gap_1()
                                    .rounded(px(16.))
                                    .border_1()
                                    .border_dashed()
                                    .border_color(theme.border)
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(theme.muted_foreground)
                                            .child("Answer"),
                                    )
                                    .child(div().text_sm().child(answer)),
                            ),
                        );
                }
                col = col.child(h_flex().w_full().justify_center().child(exch));
            } else if seg.len() == 1 {
                col = col.child(h_flex().w_full().justify_center().child(
                    div().w_full().max_w(chat_mw).child({
                        let aid = str_at(&seg[0], "id").to_string();
                        let aidc = aid.clone();
                        div()
                            .id(SharedString::from(format!("actrow-{}", aid)))
                            .cursor_pointer()
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                if let Some(pos) =
                                    this.activity_open.iter().position(|x| x == &aidc)
                                {
                                    this.activity_open.remove(pos);
                                } else {
                                    this.activity_open.push(aidc.clone());
                                }
                                cx.notify();
                            }))
                            .child(activity_row(&seg[0]))
                    }),
                ));
            } else {
                // Modernized tool-call group (flat, no card chrome): a single
                // verb-summary line ("Searched 1 issue, read 1 project, 1 failed,
                // 1 running") with quiet hover brightening, a rotating chevron,
                // and choreographed rows — staggered fade-in on expand, a pulsing
                // dot on the live row while the thread is working.
                let working = thread_actively_working(thread_v);
                // Upstream "keep the running tool run inspectable while it
                // runs": the live group renders expanded while the thread
                // works, whatever the disclosure toggle says; it falls back
                // to the toggle once the run settles.
                let is_live_card = working && seen_cards + 1 == n_plain_cards;
                let expanded = self.tools_expanded || is_live_card;
                let n_failed = seg.iter().filter(|a| str_at(a, "tone") == "error").count();
                let mut summary = tool_group_verb_summary(seg);
                if n_failed > 0 {
                    summary.push_str(&format!(", {} failed", n_failed));
                }
                if working {
                    summary.push_str(", 1 running");
                }
                let chevron = if expanded { "\u{25be}" } else { "\u{25b8}" };
                let header = h_flex()
                    .id("tools-card")
                    .cursor_pointer()
                    .gap_2()
                    .items_center()
                    .py(px(2.))
                    .on_click(cx.listener(|this, _ev, _w, cx| {
                        this.tools_expanded = !this.tools_expanded;
                        cx.notify();
                    }))
                    .child(
                        Icon::new(AssetIcon::Wrench)
                            .size(px(12.))
                            .text_color(theme.muted_foreground),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(summary),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(chevron),
                    );
                let mut card = v_flex().gap(px(1.)).child(header);
                seen_cards += 1;
                if expanded {
                    for (row_idx, a) in seg.iter().enumerate() {
                        // Staggered entrance: the easing holds at zero for a
                        // slice of the duration that grows with the row index,
                        // so rows cascade instead of popping in together.
                        let base_row = {
                            let aid = str_at(a, "id").to_string();
                            let aidc = aid.clone();
                            div()
                                .w_full()
                                .id(SharedString::from(format!("actrow-{}", aid)))
                                .cursor_pointer()
                                .on_click(cx.listener(move |this, _ev, _w, cx| {
                                    if let Some(pos) =
                                        this.activity_open.iter().position(|x| x == &aidc)
                                    {
                                        this.activity_open.remove(pos);
                                    } else {
                                        this.activity_open.push(aidc.clone());
                                    }
                                    cx.notify();
                                }))
                                .child(activity_row(a))
                        };
                        let row_with_live = h_flex().gap_1().child(base_row);
                        let stagger = (row_idx as f32 * 0.09).min(0.6);
                        let animated = row_with_live.with_animation(
                            SharedString::from(format!(
                                "toolrow-{}-{}",
                                str_at(a, "id"),
                                if expanded { "open" } else { "closed" }
                            )),
                            Animation::new(std::time::Duration::from_millis(650)).with_easing(
                                move |t| {
                                    let span = 1.0 - stagger;
                                    if span <= 0.0 {
                                        return 1.0;
                                    }
                                    let nt = ((t - stagger) / span).clamp(0.0, 1.0);
                                    // ease-out cubic
                                    1.0 - (1.0 - nt).powi(3)
                                },
                            ),
                            |this, delta| this.opacity(delta),
                        );
                        card = card.child(animated);
                    }
                    if seen_cards == n_plain_cards {
                        for t in &bg_tasks {
                            card = card.child(bg_task_row(t));
                        }
                    }
                }
                col = col.child(
                    h_flex()
                        .w_full()
                        .justify_center()
                        .child(div().w_full().max_w(chat_mw).child(card)),
                );
            }
        }

        // End-of-turn files card (upstream MessagesTimeline turnSummary):
        // after a settled turn with checkpoint files, an "Edited N files"
        // card with per-file rows (+adds/-dels); a row click opens the
        // precomputed checkpoint diff in the diff panel.
        for cp in arr(thread_v, "checkpoints") {
            let cp_files = arr(cp, "files");
            if cp_files.is_empty() {
                continue;
            }
            let cp_status = str_at(cp, "status");
            if cp_status == "missing" || cp_status == "error" {
                continue;
            }
            let turn_id = str_at(cp, "turnId").to_string();
            let tc = cp
                .get("checkpointTurnCount")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            if tc <= 0 {
                continue;
            }
            let collapsed = self.turn_files_collapsed.iter().any(|t| t == &turn_id);
            let total_adds: i64 = cp_files
                .iter()
                .map(|f| f.get("additions").and_then(Value::as_i64).unwrap_or(0))
                .sum();
            let total_dels: i64 = cp_files
                .iter()
                .map(|f| f.get("deletions").and_then(Value::as_i64).unwrap_or(0))
                .sum();
            let turn_id_h = turn_id.clone();
            let mut fcard =
                v_flex()
                    .w_full()
                    .max_w(chat_mw)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .p_2()
                    .gap_1()
                    .child(
                        h_flex()
                            .id(ElementId::Name(format!("turnfiles-{turn_id}").into()))
                            .cursor_pointer()
                            .gap_2()
                            .items_center()
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                if this.turn_files_collapsed.iter().any(|t| t == &turn_id_h) {
                                    this.turn_files_collapsed.retain(|t| t != &turn_id_h);
                                } else {
                                    this.turn_files_collapsed.push(turn_id_h.clone());
                                }
                                cx.notify();
                            }))
                            .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).child(
                                format!(
                                    "Edited {} {}{}",
                                    cp_files.len(),
                                    if cp_files.len() == 1 { "file" } else { "files" },
                                    if collapsed { " \u{25b8}" } else { " \u{25be}" },
                                ),
                            ))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child(format!("+{total_adds} -{total_dels}")),
                            ),
                    );
            if !collapsed {
                let review_tid = this_tid.clone();
                let thread_id_r = str_at(thread_v, "id").to_string();
                let _ = review_tid;
                fcard = fcard.child(
                    h_flex()
                        .id(ElementId::Name(
                            format!("turnfiles-review-{turn_id}").into(),
                        ))
                        .cursor_pointer()
                        .gap_1()
                        .py(px(2.))
                        .on_click(cx.listener(move |this, _ev, _w, cx| {
                            this.open_turn_diff(thread_id_r.clone(), tc - 1, tc, cx);
                        }))
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child("Review diff"),
                        ),
                );
                for f in cp_files.iter().take(8) {
                    let fpath = str_at(f, "path").to_string();
                    let adds = f.get("additions").and_then(Value::as_i64).unwrap_or(0);
                    let dels = f.get("deletions").and_then(Value::as_i64).unwrap_or(0);
                    let fkind = str_at(f, "kind");
                    let fname = fpath.rsplit('/').next().unwrap_or(&fpath).to_string();
                    let fdir = if fpath.len() > fname.len() {
                        fpath[..fpath.len() - fname.len()]
                            .trim_end_matches('/')
                            .to_string()
                    } else {
                        String::new()
                    };
                    let kind_label = match fkind {
                        "added" => "A",
                        "deleted" => "D",
                        _ => "M",
                    };
                    fcard = fcard.child(
                        h_flex()
                            .w_full()
                            .gap_2()
                            .items_center()
                            .py(px(1.))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child(kind_label),
                            )
                            .child(div().text_xs().overflow_hidden().whitespace_nowrap().child(
                                if fdir.is_empty() {
                                    fname
                                } else {
                                    format!("{fdir}/{fname}")
                                },
                            ))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child(format!("+{adds} -{dels}")),
                            ),
                    );
                }
                if cp_files.len() > 8 {
                    fcard = fcard.child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(format!("… {} more", cp_files.len() - 8)),
                    );
                }
            }
            col = col.child(h_flex().w_full().justify_center().child(fcard));
        }

        // SubagentRunCard: the thread's child threads fold into one
        // collapsible card — "N agents" header, per-agent rows when expanded.
        let subagents: Vec<Value> = arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "threads")
            .iter()
            .filter(|t| {
                !this_tid.is_empty()
                    && str_at(t, "parentThreadId") == this_tid
                    && get(t, "deletedAt").is_null()
            })
            .cloned()
            .collect();
        if !subagents.is_empty() {
            let running_n = subagents
                .iter()
                .filter(|t| thread_running_for_activity(t))
                .count();
            let mut card =
                v_flex()
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .p_2()
                    .gap_1()
                    .child(
                        h_flex()
                            .id("subagents-card")
                            .cursor_pointer()
                            .gap_2()
                            .items_center()
                            .on_click(cx.listener(|this, _ev, _w, cx| {
                                this.subagents_expanded = !this.subagents_expanded;
                                cx.notify();
                            }))
                            .child(
                                Icon::new(AssetIcon::Bot)
                                    .size(px(12.))
                                    .text_color(theme.muted_foreground),
                            )
                            .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).child(
                                format!(
                                    "{} {} {}{}",
                                    subagents.len(),
                                    if subagents.len() == 1 {
                                        "agent"
                                    } else {
                                        "agents"
                                    },
                                    if self.subagents_expanded {
                                        "\u{25be}"
                                    } else {
                                        "\u{25b8}"
                                    },
                                    if running_n > 0 {
                                        format!(" · {running_n} running")
                                    } else {
                                        String::new()
                                    },
                                ),
                            )),
                    );
            if self.subagents_expanded {
                for (si, t) in subagents.iter().enumerate() {
                    let tid = str_at(t, "id").to_string();
                    let name = {
                        let n = str_at(t, "subagentNickname");
                        if n.is_empty() {
                            let ti = str_at(t, "title");
                            if ti.is_empty() {
                                "Subagent".to_string()
                            } else {
                                ti.to_string()
                            }
                        } else {
                            n.to_string()
                        }
                    };
                    let (glyph, gcol) = if thread_running_for_activity(t) {
                        ("\u{25cf}", theme.primary)
                    } else if sess_str(t, "status") == "failed"
                        || str_at(get(t, "session"), "status") == "failed"
                    {
                        ("\u{2717}", theme.danger)
                    } else {
                        ("\u{2713}", theme.muted_foreground)
                    };
                    let elapsed = clock_elapsed(
                        str_at(t, "createdAt"),
                        if thread_running_for_activity(t) {
                            ""
                        } else {
                            str_at(t, "updatedAt")
                        },
                    );
                    card = card.child(
                        h_flex()
                            .id(SharedString::from(format!("tr-sub-{si}")))
                            .w_full()
                            .gap_2()
                            .items_center()
                            .py(px(2.))
                            .rounded(px(4.))
                            .cursor_pointer()
                            .hover(|d| d.bg(subtle))
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                this.open_thread(tid.clone(), cx)
                            }))
                            .child(div().text_xs().text_color(gcol).child(glyph))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .flex_1()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .child(name),
                            )
                            .when_some(elapsed, |d, e| {
                                d.child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(theme.muted_foreground)
                                        .child(e),
                                )
                            }),
                    );
                }
            }
            col = col.child(
                h_flex()
                    .w_full()
                    .justify_center()
                    .child(div().w_full().max_w(chat_mw).child(card)),
            );
        }

        // Active tools currently running.
        for t in arr(thread_v, "activeTools") {
            let item = str_at(t, "item_id");
            if item.is_empty() {
                continue;
            }
            col = col.child(
                h_flex().w_full().justify_center().child(
                    div().w_full().max_w(chat_mw).child(
                        div()
                            .text_xs()
                            .text_color(theme.primary)
                            .child(format!("\u{25cf} tool running: {item}")),
                    ),
                ),
            );
        }

        // Proposed plans (upstream ProposedPlanCard): a 24px-rounded card with
        // a "Plan" badge + first-heading title, download/copy icon buttons,
        // and a collapsible markdown body (10-line preview + "..." when long).
        for p in arr(thread_v, "proposedPlans") {
            let md = str_at(p, "planMarkdown").to_string();
            if md.trim().is_empty() {
                continue;
            }
            let plan_id = str_at(p, "id").to_string();
            // proposedPlanTitle: first markdown heading wins.
            let title = md
                .lines()
                .find_map(|l| {
                    let t = l.trim_start();
                    let h = t.trim_start_matches('#');
                    if h.len() < t.len() && h.starts_with(' ') {
                        let v = h.trim().to_string();
                        (!v.is_empty()).then_some(v)
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| "Proposed plan".to_string());
            // stripDisplayedPlanMarkdown: drop the first heading, then a
            // leading "Summary" heading.
            let mut body_lines: Vec<&str> = md.trim_end().lines().collect();
            if body_lines
                .first()
                .map(|l| l.trim_start().starts_with('#'))
                .unwrap_or(false)
            {
                body_lines.remove(0);
            }
            while body_lines
                .first()
                .map(|l| l.trim().is_empty())
                .unwrap_or(false)
            {
                body_lines.remove(0);
            }
            if body_lines
                .first()
                .map(|l| {
                    let h = l.trim_start().trim_start_matches('#');
                    h.len() < l.trim_start().len() && h.trim().eq_ignore_ascii_case("summary")
                })
                .unwrap_or(false)
            {
                body_lines.remove(0);
                while body_lines
                    .first()
                    .map(|l| l.trim().is_empty())
                    .unwrap_or(false)
                {
                    body_lines.remove(0);
                }
            }
            let displayed = body_lines.join("\n");
            let can_collapse = md.len() > 900 || md.lines().count() > 20;
            let expanded = self.plan_expanded.contains(&plan_id);
            let shown_md = if can_collapse && !expanded {
                // buildCollapsedProposedPlanPreviewMarkdown: 10 visible lines.
                let mut out: Vec<String> = Vec::new();
                let mut visible = 0usize;
                let mut more = false;
                for line in displayed.trim_end().lines().map(|l| l.trim_end()) {
                    if !line.trim().is_empty() && visible >= 10 {
                        more = true;
                        break;
                    }
                    out.push(line.to_string());
                    if !line.trim().is_empty() {
                        visible += 1;
                    }
                }
                while out.last().map(|l| l.trim().is_empty()).unwrap_or(false) {
                    out.pop();
                }
                if out.is_empty() {
                    title.clone()
                } else {
                    if more {
                        out.push(String::new());
                        out.push("...".to_string());
                    }
                    out.join("\n")
                }
            } else {
                displayed.clone()
            };

            let mut actions = h_flex().gap_1().flex_shrink_0();
            // Download (writes <workspace>/.plan/<slug>.md).
            {
                let md2 = format!("{}\n", md.trim_end());
                let slug = {
                    let s = title
                        .to_lowercase()
                        .replace(|c: char| "`'\".,!?()[]{}".contains(c), "")
                        .replace(|c: char| !(c.is_ascii_alphanumeric()), "-")
                        .trim_matches('-')
                        .to_string();
                    if s.is_empty() {
                        "plan".to_string()
                    } else {
                        s
                    }
                };
                actions = actions.child(
                    div()
                        .id(ElementId::Name(format!("plan-dl-{plan_id}").into()))
                        .size(px(20.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .border_1()
                        .border_color(theme.border)
                        .rounded(px(5.))
                        .text_size(px(11.))
                        .text_color(theme.muted_foreground)
                        .cursor_pointer()
                        .hover(|s| s.text_color(theme.foreground))
                        .child("\u{2193}")
                        .on_mouse_down(MouseButton::Left, {
                            let tid = str_at(thread_v, "id").to_string();
                            cx.listener(move |this, _e, _w, cx| {
                                let Some(root) = this.thread_workspace_cwd(&tid) else {
                                    this.feedback_toast = Some((
                                        "Workspace unavailable for this plan.".to_string(),
                                        true,
                                    ));
                                    cx.notify();
                                    return;
                                };
                                let dir = std::path::Path::new(&root).join(".plan");
                                let ok = std::fs::create_dir_all(&dir).is_ok()
                                    && std::fs::write(dir.join(format!("{slug}.md")), &md2).is_ok();
                                this.feedback_toast = Some((
                                    if ok {
                                        format!("Saved to .plan/{slug}.md")
                                    } else {
                                        "Could not save the plan.".to_string()
                                    },
                                    !ok,
                                ));
                                cx.notify();
                            })
                        }),
                );
            }
            // Export (native save dialog).
            {
                let md2 = format!("{}\n", md.trim_end());
                let slug2 = {
                    let s = title
                        .to_lowercase()
                        .replace(|c: char| "`'\".,!?()[]{}".contains(c), "")
                        .replace(|c: char| !(c.is_ascii_alphanumeric()), "-")
                        .trim_matches('-')
                        .to_string();
                    if s.is_empty() {
                        "plan".to_string()
                    } else {
                        s
                    }
                };
                actions = actions.child(
                    div()
                        .id(ElementId::Name(format!("plan-ex-{plan_id}").into()))
                        .size(px(20.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .border_1()
                        .border_color(theme.border)
                        .rounded(px(5.))
                        .text_size(px(11.))
                        .text_color(theme.muted_foreground)
                        .cursor_pointer()
                        .hover(|s| s.text_color(theme.foreground))
                        .child("\u{2191}")
                        .on_mouse_down(MouseButton::Left, {
                            let tid = str_at(thread_v, "id").to_string();
                            cx.listener(move |this, _e, _w, cx| {
                                let dir = this
                                    .thread_workspace_cwd(&tid)
                                    .map(std::path::PathBuf::from)
                                    .unwrap_or_else(|| std::env::temp_dir());
                                let rx = cx.prompt_for_new_path(&dir, Some(&format!("{slug2}.md")));
                                let md3 = md2.clone();
                                cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                                    let path = rx.await.ok().and_then(|r| r.ok()).flatten();
                                    let ok = match &path {
                                        Some(p) => std::fs::write(p, &md3).is_ok(),
                                        None => return,
                                    };
                                    let _ = this.update(cx, |this, cx| {
                                        this.feedback_toast = Some((
                                            if ok {
                                                "Plan exported.".to_string()
                                            } else {
                                                "Could not export the plan.".to_string()
                                            },
                                            !ok,
                                        ));
                                        cx.notify();
                                    });
                                })
                                .detach();
                            })
                        }),
                );
            }
            // Copy as markdown.
            {
                let md2 = format!("{}\n", md.trim_end());
                actions = actions.child(
                    div()
                        .id(ElementId::Name(format!("plan-cp-{plan_id}").into()))
                        .size(px(20.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .border_1()
                        .border_color(theme.border)
                        .rounded(px(5.))
                        .text_size(px(11.))
                        .text_color(theme.muted_foreground)
                        .cursor_pointer()
                        .hover(|s| s.text_color(theme.foreground))
                        .child("\u{2398}")
                        .on_mouse_down(MouseButton::Left, move |_e, _w, cx| {
                            cx.write_to_clipboard(ClipboardItem::new_string(md2.clone()));
                        }),
                );
            }

            let mut card = v_flex()
                .w_full()
                .gap_0()
                .border_1()
                .border_color(theme.border.opacity(0.8))
                .bg(theme.popover)
                .rounded(px(24.))
                .p(px(16.))
                .child(
                    h_flex()
                        .w_full()
                        .items_center()
                        .justify_between()
                        .child(
                            h_flex()
                                .min_w_0()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(theme.foreground)
                                        .bg(theme.secondary)
                                        .rounded(px(6.))
                                        .px(px(8.))
                                        .py(px(2.))
                                        .child("Plan"),
                                )
                                .child(
                                    div()
                                        .text_size(px(14.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(theme.foreground)
                                        .child(title.clone()),
                                ),
                        )
                        .child(actions),
                )
                .child(
                    div().mt(px(12.)).child(
                        TextView::markdown(
                            ElementId::Name(format!("plan-{plan_id}").into()),
                            shown_md,
                        )
                        .style({
                            let mut s = gpui_kit::component::text::TextViewStyle::default();
                            s.highlight_theme =
                                gpui_kit::component::highlighter::HighlightTheme::default_dark();
                            s
                        })
                        .selectable(true),
                    ),
                );
            if can_collapse {
                let pid = plan_id.clone();
                let was = expanded;
                card = card.child(
                    h_flex().w_full().justify_center().mt(px(12.)).child(
                        div()
                            .id(ElementId::Name(format!("plan-toggle-{pid}").into()))
                            .text_size(px(11.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.muted_foreground)
                            .border_1()
                            .border_color(theme.border)
                            .rounded(px(7.))
                            .px(px(10.))
                            .py(px(3.))
                            .cursor_pointer()
                            .hover(|s| s.text_color(theme.foreground))
                            .child(if expanded {
                                "Collapse plan"
                            } else {
                                "Expand plan"
                            })
                            .on_mouse_down(MouseButton::Left, {
                                cx.listener(move |this, _e, _w, cx| {
                                    if was {
                                        this.plan_expanded.remove(&pid);
                                    } else {
                                        this.plan_expanded.insert(pid.clone());
                                    }
                                    cx.notify();
                                })
                            }),
                    ),
                );
            }
            col = col.child(
                h_flex()
                    .w_full()
                    .justify_center()
                    .child(div().w_full().max_w(chat_mw).child(card)),
            );
        }

        // Blocking userInput prompt (upstream ComposerPendingUserInputPanel +
        // UserInputQuestionForm): a form card for the first pending request.
        let pending_input = active_pending_input(thread_v);

        // Pending interactions (approvals awaiting a decision).
        for p in arr(thread_v, "pendingInteractions") {
            if str_at(p, "status") != "pending" {
                continue;
            }
            // userInput rows render as the form card below, not a pill.
            if str_at(p, "interactionKind") == "userInput" {
                continue;
            }
            let rid = str_at(p, "requestId").to_string();
            // The matching approval.requested activity carries the
            // human-readable detail (tool name / command) upstream shows
            // on the card, plus whether a session-scoped allow exists.
            let req_act = arr(thread_v, "activities").iter().rev().find(|a| {
                str_at(a, "kind") == "approval.requested"
                    && str_at(get(a, "payload"), "requestId") == rid
            });
            let payload = req_act
                .map(|a| get(a, "payload").clone())
                .unwrap_or(Value::Null);
            let detail = {
                let d = str_at(&payload, "detail");
                if d.is_empty() {
                    let t = str_at(&payload, "toolName");
                    if t.is_empty() {
                        "approval"
                    } else {
                        t
                    }
                } else {
                    d
                }
            };
            let session_ok = get(&payload, "sessionApprovalAvailable")
                .as_bool()
                .unwrap_or(false);
            let mut row_el = h_flex().gap_2().child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.warning)
                    .child(format!("approve {detail}?")),
            );
            if !rid.is_empty() {
                let mut buttons: Vec<(&str, &str)> =
                    vec![("Approve", "accept"), ("Decline", "decline")];
                if session_ok {
                    buttons.insert(1, ("Always allow", "acceptForSession"));
                }
                for (label, decision) in buttons {
                    let r = rid.clone();
                    row_el = row_el.child(
                        Button::new(ElementId::Name(format!("approve-{decision}-{rid}").into()))
                            .ghost()
                            .label(label)
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                this.respond_approval(r.clone(), decision, cx)
                            })),
                    );
                }
            }
            col = col.child(
                h_flex()
                    .w_full()
                    .justify_center()
                    .child(div().w_full().max_w(chat_mw).child(row_el)),
            );
        }

        // Blocking userInput form card (upstream UserInputQuestionForm):
        // question header, numbered option rows, ‹ n of N › navigation,
        // key hints and a Cancel that submits empty answers.
        if let Some((rid, gen, questions)) = pending_input {
            let q_count = questions.len();
            let qidx = (*self.pi_qidx.get(&rid).unwrap_or(&0)).min(q_count - 1);
            let q = &questions[qidx];
            let qid = str_at(q, "id").to_string();
            let multi = get(q, "multiSelect").as_bool().unwrap_or(false);
            let options = arr(q, "options");
            let drafts = self.pi_drafts.get(&rid).cloned().unwrap_or_default();
            let draft = drafts.get(&qid).cloned().unwrap_or_default();
            let answered_count = questions
                .iter()
                .filter(|qq| pi_resolved(qq, drafts.get(str_at(qq, "id"))).is_some())
                .count();
            let is_last = qidx + 1 >= q_count;
            let responding = self.pi_responding;

            let mut card = v_flex()
                .w_full()
                .border_1()
                .border_color(theme.border)
                .rounded(px(12.))
                .px(px(14.))
                .pt(px(12.))
                .pb(px(6.))
                .gap_1();

            // Header: question text + ‹ n of N › navigation.
            let mut head = h_flex().items_start().justify_between().gap_3().child(
                div()
                    .text_size(px(14.))
                    .font_weight(FontWeight::MEDIUM)
                    .flex_1()
                    .min_w_0()
                    .child(str_at(q, "question").to_string()),
            );
            if q_count > 1 {
                let can_back = qidx > 0 && !responding;
                let can_fwd = !is_last && !responding;
                let rid_b = rid.clone();
                let rid_f = rid.clone();
                head = head.child(
                    h_flex()
                        .gap_1()
                        .items_center()
                        .child(
                            div()
                                .id(ElementId::Name(format!("pi-prev-{rid}").into()))
                                .px_1()
                                .rounded(px(4.))
                                .when(can_back, |d| {
                                    d.cursor_pointer().hover(|h| h.bg(subtle)).on_click(
                                        cx.listener(move |this, _ev, _w, cx| {
                                            let i = *this.pi_qidx.get(&rid_b).unwrap_or(&0);
                                            this.pi_qidx.insert(rid_b.clone(), i.saturating_sub(1));
                                            cx.notify();
                                        }),
                                    )
                                })
                                .when(!can_back, |d| d.opacity(0.3))
                                .child(
                                    svg_icon(icons::CHEVRON_LEFT)
                                        .size(px(14.))
                                        .text_color(theme.muted_foreground),
                                ),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(theme.muted_foreground)
                                .child(format!("{} of {q_count}", qidx + 1)),
                        )
                        .child(
                            div()
                                .id(ElementId::Name(format!("pi-next-{rid}").into()))
                                .px_1()
                                .rounded(px(4.))
                                .when(can_fwd, |d| d.cursor_pointer().hover(|h| h.bg(subtle)))
                                .when(!can_fwd, |d| d.opacity(0.3))
                                .child(
                                    svg_icon(icons::CHEVRON_RIGHT)
                                        .size(px(14.))
                                        .text_color(theme.muted_foreground),
                                )
                                .on_click(cx.listener(move |this, _ev, _w, cx| {
                                    if !can_fwd {
                                        return;
                                    }
                                    let i = *this.pi_qidx.get(&rid_f).unwrap_or(&0);
                                    this.pi_qidx.insert(rid_f.clone(), i + 1);
                                    cx.notify();
                                })),
                        ),
                );
            }
            card = card.child(head);
            if multi {
                card = card.child(
                    div()
                        .text_size(px(12.))
                        .text_color(theme.muted_foreground)
                        .child("Select one or more"),
                );
            }

            // Option rows (upstream ComposerChoiceRow "stacked"): number
            // chip, label (+Recommended badge), description, trailing check.
            if !options.is_empty() {
                let mut optcol = v_flex().mt(px(4.)).gap_px();
                for (oi, opt) in options.iter().enumerate() {
                    let raw_label = str_at(opt, "label").to_string();
                    // Split a trailing "(Recommended)" exactly like
                    // upstream splitRecommendedOptionLabel.
                    let (label, recommended) = {
                        let suffix = " (recommended)";
                        if raw_label.to_lowercase().ends_with(suffix) {
                            (
                                raw_label[..raw_label.len() - suffix.len()].to_string(),
                                true,
                            )
                        } else {
                            (raw_label.clone(), false)
                        }
                    };
                    let desc = str_at(opt, "description").to_string();
                    let selected = draft.options.iter().any(|o| o == &raw_label);
                    let rid_c = rid.clone();
                    let qid_c = qid.clone();
                    let label_c = raw_label.clone();
                    let qs_c = questions.clone();
                    let gen_c = gen.clone();
                    let mut row = h_flex()
                        .id(ElementId::Name(format!("pi-opt-{rid}-{qid}-{oi}").into()))
                        .w_full()
                        .items_start()
                        .gap_2()
                        .px(px(10.))
                        .py(px(6.))
                        .rounded(px(8.))
                        .cursor_pointer();
                    if !responding {
                        row = row.hover(|h| h.bg(subtle)).on_click(cx.listener(
                            move |this, _ev, _w, cx| {
                                this.pi_toggle(&rid_c, &qid_c, &label_c, multi, cx);
                                // Single-select auto-advances (upstream
                                // autoAdvance after the pick).
                                if !multi {
                                    this.pi_advance(&rid_c, &qs_c, gen_c.clone(), cx);
                                }
                            },
                        ));
                    }
                    if oi < 9 {
                        row = row.child(
                            div()
                                .w(px(16.))
                                .h(px(16.))
                                .flex_none()
                                .rounded(px(4.))
                                .bg(theme.secondary)
                                .text_size(px(10.))
                                .text_color(theme.muted_foreground)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(format!("{}", oi + 1)),
                        );
                    }
                    let mut label_col = v_flex().flex_1().min_w_0().child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(div().text_size(px(13.)).child(label)),
                    );
                    if recommended {
                        label_col = label_col.child(
                            div()
                                .text_size(px(11.))
                                .text_color(theme.accent)
                                .child("Recommended"),
                        );
                    }
                    if !desc.is_empty() {
                        label_col = label_col.child(
                            div()
                                .text_size(px(12.))
                                .text_color(theme.muted_foreground)
                                .child(desc),
                        );
                    }
                    row = row.child(label_col);
                    row = row.child(div().w(px(16.)).flex_none().when(selected, |d| {
                        d.child(
                            svg_icon(icons::CHECK)
                                .size(px(13.))
                                .text_color(theme.foreground),
                        )
                    }));
                    optcol = optcol.child(row);
                }
                card = card.child(optcol);
            }

            // Footer: key hints + answered count + Cancel.
            let shortcut_count = options.len().min(9);
            let mut foot = h_flex()
                .items_center()
                .gap_3()
                .mt(px(4.))
                .child(
                    h_flex()
                        .items_center()
                        .gap_1()
                        .child(sc_chip(
                            if shortcut_count == 1 {
                                "1".to_string()
                            } else {
                                format!("1–{shortcut_count}")
                            },
                            &theme,
                        ))
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(theme.muted_foreground)
                                .child("to pick"),
                        ),
                )
                .child(
                    h_flex()
                        .items_center()
                        .gap_1()
                        .child(sc_chip("↵", &theme))
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(theme.muted_foreground)
                                .child(if is_last { "to submit" } else { "for next" }),
                        ),
                );
            if answered_count > 0 {
                foot = foot.child(
                    div()
                        .text_size(px(11.))
                        .text_color(theme.muted_foreground)
                        .child(format!("{answered_count}/{q_count} answered")),
                );
            }
            let rid_x = rid.clone();
            let gen_x = gen.clone();
            foot = foot.child(
                h_flex().flex_1().justify_end().child(
                    div()
                        .id(ElementId::Name(format!("pi-cancel-{rid}").into()))
                        .px(px(8.))
                        .py(px(4.))
                        .rounded(px(6.))
                        .cursor_pointer()
                        .text_size(px(12.))
                        .text_color(theme.muted_foreground)
                        .hover(|h| h.bg(subtle).text_color(theme.foreground))
                        .child("Cancel")
                        .on_click(cx.listener(move |this, _ev, _w, cx| {
                            // Upstream cancel submits an empty answers record.
                            this.pi_respond(rid_x.clone(), gen_x.clone(), json!({}), cx);
                        })),
                ),
            );
            card = card.child(foot);

            col = col.child(
                h_flex()
                    .w_full()
                    .justify_center()
                    .child(div().w_full().max_w(chat_mw).child(card)),
            );
        }

        let working = thread_actively_working(thread_v);
        let live_label = arr(thread_v, "activities").iter().rev().find_map(|a| {
            let kind = str_at(a, "kind");
            if !matches!(kind, "tool.started" | "tool.updated") {
                return None;
            }
            let summary = str_at(a, "summary");
            let label = if summary.is_empty() { kind } else { summary };
            (!label.is_empty()).then_some(label.to_string())
        });
        // Stopped-on-human state: a plain quiet transcript row, matching the
        // web client instead of borrowing another product's status treatment.
        let waiting_approval = arr(thread_v, "pendingInteractions").iter().any(|p| {
            str_at(p, "status") == "pending" && str_at(p, "interactionKind") != "userInput"
        }) || get(thread_v, "hasPendingApprovals").as_bool() == Some(true);
        let waiting_input = active_pending_input(thread_v).is_some()
            || get(thread_v, "hasPendingUserInput").as_bool() == Some(true);
        if waiting_approval || waiting_input {
            let (label, icon) = if waiting_approval {
                ("Waiting for approval", svg_icon(icons::SHIELD_ACCESS))
            } else {
                ("Waiting for your answer", svg_icon(icons::BUBBLE_TEXT))
            };
            col = col.child(
                h_flex().w_full().justify_center().child(
                    div().w_full().max_w(chat_mw).child(
                        h_flex()
                            .gap_1p5()
                            .items_center()
                            .py(px(2.))
                            .child(icon.size(px(12.)).text_color(theme.muted_foreground))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(theme.muted_foreground)
                                    .child(label),
                            ),
                    ),
                ),
            );
        } else if working {
            col = col.child(
                h_flex().w_full().justify_center().child(
                    div().w_full().max_w(chat_mw).child(
                        h_flex()
                            .gap_1p5()
                            .items_center()
                            .py(px(2.))
                            .child(animated_logo(px(14.), theme.primary))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(theme.muted_foreground)
                                    .child(live_label.unwrap_or_else(|| "Thinking".to_string())),
                            ),
                    ),
                ),
            );
        }

        col
    }

    /// Composer: send the draft as a turn on the selected thread, or create a
    /// thread in the first project then start the turn.
    pub(super) fn send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.composer.read(cx).value().to_string();
        let text = text.trim().to_string();
        // While a blocking userInput prompt is docked the composer holds the
        // active question's custom answer and Enter advances/submits the
        // form (upstream ComposerPendingUserInputPanel owns the composer).
        {
            let tv = self
                .detail
                .as_ref()
                .map(|d| get(d, "thread"))
                .cloned()
                .unwrap_or(Value::Null);
            if let Some((rid, gen, questions)) = active_pending_input(&tv) {
                let qidx = *self.pi_qidx.get(&rid).unwrap_or(&0);
                if let Some(q) = questions.get(qidx) {
                    if !text.is_empty() {
                        let qid = str_at(q, "id").to_string();
                        let draft = self
                            .pi_drafts
                            .entry(rid.clone())
                            .or_default()
                            .entry(qid)
                            .or_default();
                        draft.custom = text.clone();
                        draft.options.clear();
                    }
                    self.composer.update(cx, |s, cx| s.clean(window, cx));
                    self.pi_advance(&rid, &questions, gen, cx);
                }
                return;
            }
        }
        if text.is_empty() || self.sending {
            return;
        }
        // Expired side chats keep the composer read-only (upstream
        // submission.expired); a replacement starts from the notice bar.
        {
            let tv = self
                .detail
                .as_ref()
                .map(|d| get(d, "thread"))
                .cloned()
                .unwrap_or(Value::Null);
            if !get(&tv, "sidechatExpiredAt").is_null() {
                return;
            }
        }
        // Edit & resend (upstream thread.message.edit-and-resend): the pencil
        // on a user message reloads it into the composer; sending replaces
        // the original turn instead of appending a new one.
        if let Some(edit_id) = self.editing_message_id.take() {
            if self.selected_thread.is_some() {
                let tid = self.selected_thread.clone().unwrap_or_default();
                self.sending = true;
                self.composer.update(cx, |s, cx| s.clean(window, cx));
                let cell = self.client.clone();
                let mode = self.runtime_mode;
                let imode = self.interaction_mode;
                cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                    let cell2 = cell.clone();
                    let client = cx
                        .background_executor()
                        .spawn(async move { ensure_client(&cell2) })
                        .await;
                    let Some(client) = client else {
                        let _ = this.update(cx, |this: &mut Self, cx| {
                            this.sending = false;
                            cx.notify();
                        });
                        return;
                    };
                    let cmd = json!({
                        "type": "thread.message.edit-and-resend",
                        "commandId": uuid(),
                        "threadId": tid,
                        "messageId": edit_id,
                        "text": text,
                        "runtimeMode": mode,
                        "interactionMode": imode,
                        "createdAt": now_iso(),
                    });
                    let c = client.clone();
                    let _ = cx
                        .background_executor()
                        .spawn(async move { c.call("orchestration.dispatchCommand", cmd) })
                        .await;
                    let p = json!({"threadId": tid});
                    let c = client.clone();
                    let d = cx
                        .background_executor()
                        .spawn(async move { c.call("orchestration.getThreadDetailSnapshot", p) })
                        .await;
                    let _ = this.update(cx, |this: &mut Self, cx| {
                        this.sending = false;
                        if let Some(d) = d {
                            this.detail = Some(d);
                        }
                        cx.notify();
                    });
                })
                .detach();
                return;
            }
        }
        // While a turn is live, upstream enqueues the follow-up instead of
        // sending it (the queued pills render above the composer).
        if self.selected_thread.is_some()
            && matches!(
                session_phase(self.detail.as_ref()),
                "running" | "connecting"
            )
        {
            self.queued_turns.push(text);
            self.composer.update(cx, |s, cx| s.clean(window, cx));
            cx.notify();
            // `detail` may be a stale "running" left over after the stream
            // ended — refresh it and drain right away if the session has
            // actually settled, otherwise the pill waits for the next
            // stream event that may never come.
            let cell = self.client.clone();
            let tid = self.selected_thread.clone().unwrap_or_default();
            cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                let p = json!({"threadId": tid});
                let d = cx
                    .background_executor()
                    .spawn(async move {
                        let client = ensure_client(&cell)?;
                        client.call("orchestration.getThreadDetailSnapshot", p)
                    })
                    .await;
                let _ = this.update(cx, |this: &mut Self, cx| {
                    if let Some(d) = d {
                        this.detail = Some(d);
                    }
                    this.drain_queued(cx);
                    cx.notify();
                });
            })
            .detach();
            return;
        }
        self.sending = true;
        self.composer.update(cx, |s, cx| s.clean(window, cx));
        let selected = self.selected_thread.clone();
        let project_id = self.landing_project_id.clone().or_else(|| {
            arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects")
                .first()
                .and_then(|p| p.get("id"))
                .and_then(Value::as_str)
                .map(str::to_owned)
        });
        let client = self.client.clone();
        let mode = self.runtime_mode;
        let imode = self.interaction_mode;
        let composer_model = self.composer_model.clone();
        let this_effort = self.composer_effort_idx;
        // Attachments uploaded earlier keep the thread id they were staged
        // under (a fresh thread adopts it as its id).
        if self.composer_thread_id.is_none() && self.selected_thread.is_none() {
            self.composer_thread_id = Some(uuid());
        }
        let attachments = std::mem::take(&mut self.pending_attachments);
        let att_thread = if self.selected_thread.is_some() {
            self.selected_thread.clone()
        } else {
            self.composer_thread_id.clone()
        };
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
            // Upstream commits the trait row selection with the model: the
            // effort ladder position rides modelSelection.options.effort,
            // which the provider runners read back when spawning the CLI.
            let effort_names = ["minimal", "low", "medium", "high", "max"];
            let effort = effort_names[this_effort.min(4)];
            let model_selection = json!({
                "provider": composer_model.0,
                "instanceId": composer_model.0,
                "model": composer_model.1,
                "options": {"effort": effort},
            });
            let tid = selected
                .clone()
                .unwrap_or_else(|| att_thread.clone().unwrap_or_else(uuid));
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
                        "runtimeMode": mode,
                        "interactionMode": imode,
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
                    "attachments": attachments,
                },
                "runtimeMode": mode,
                "interactionMode": imode,
                "modelSelection": model_selection,
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
                // A new (or stream-less) thread needs open_thread to install
                // the live subscribeThread stream; a plain detail assignment
                // leaves the transcript frozen until re-click.
                if this.selected_thread.as_deref() != Some(tid.as_str())
                    || this.thread_stream_id.is_none()
                {
                    this.open_thread(tid.clone(), cx);
                }
                if let Some(d) = d {
                    this.detail = Some(d);
                    this.transcript_scroll.scroll_to_bottom();
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
}
