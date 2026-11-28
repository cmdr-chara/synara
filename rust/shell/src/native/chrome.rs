//! Overlay surfaces extracted from native/mod.rs (wave 2 of the split):
//! context menus, hover cards, palette, model picker, dialogs and popovers.
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

pub(super) struct NativeChrome {
    pub(super) changes_pane: gpui::AnyElement,
}

impl Native {
    pub(super) fn build_chrome(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> NativeChrome {
        let theme = cx.theme().clone();
        let dark_ui = match self.appearance_mode {
            2 => true,
            1 => false,
            _ => matches!(
                cx.window_appearance(),
                gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
            ),
        };
        let well_bg = if dark_ui {
            rgb(0x1d1d1d)
        } else {
            rgb(0xf7f7f7)
        };
        let hairline = if dark_ui {
            rgb(0x2e2e2e)
        } else {
            rgb(0xe5e5e5)
        };
        let subtle = if dark_ui {
            rgba(0xffffff14)
        } else {
            rgba(0x0000000d)
        };
        let chip_bg = if dark_ui {
            rgba(0xffffff12)
        } else {
            rgba(0x00000008)
        };
        let soft_bg: gpui::Hsla = if dark_ui {
            rgba(0xffffff0a).into()
        } else {
            rgb(0xf7f7f7).into()
        };
        let sel_fill: gpui::Hsla = if dark_ui {
            rgba(0xffffff0a).into()
        } else {
            theme.list_active
        };
        let content_bg: gpui::Hsla = if dark_ui {
            rgb(0x111111).into()
        } else {
            theme.background
        };
        let sidebar_bg: gpui::Hsla = if dark_ui {
            rgb(0x111111).into()
        } else {
            theme.background
        };
        let popover_bg: gpui::Hsla = if dark_ui {
            rgb(0x161616).into()
        } else {
            theme.popover
        };

        // ---- changes dock (right pane): upstream GitPanel ----
        // Header "Source control" + refresh + close; sections Staged/Changes
        // with file rows (icon, dir muted + name, +ins/-del, stage action);
        // bottom diff viewport over a border-t.
        let mut changes_pane = v_flex().w(px(300.)).h_full().flex_col();
        changes_pane = changes_pane.child(
            h_flex()
                .items_center()
                .w_full()
                .px_4()
                .pt(px(9.))
                .pb_1()
                .gap(px(6.))
                // gpui scroll offsets go negative toward the end of content.
                .when(self.dock_tabs_scroll.offset().x < px(0.), |row| {
                    // Overflow chevron (upstream SurfaceContentTabs
                    // affordance) — scrolls the tab strip back to start.
                    row.child(
                        div()
                            .id("dock-tabs-left")
                            .px(px(1.))
                            .cursor_pointer()
                            .text_color(theme.muted_foreground)
                            .on_click(cx.listener(|this, _ev, _w, cx| {
                                this.dock_tabs_scroll
                                    .set_offset(gpui::point(px(0.), px(0.)));
                                cx.notify();
                            }))
                            .child(Icon::new(IconName::ChevronLeft).size(px(14.))),
                    )
                })
                .child(
                    // Upstream SurfaceContentTabs: icon + label per pane,
                    // the strip scrolls horizontally past its width floor.
                    div()
                        .id("dock-tabs")
                        .flex_1()
                        .min_w_0()
                        .track_scroll(&self.dock_tabs_scroll)
                        .overflow_x_scroll()
                        .child(
                            // Fixed natural width so Taffy reports real
                            // overflow for the scroll range (tabs ~430px).
                            h_flex().gap_1().w(px(430.)).children(
                                [
                                    (0usize, "dock-tab-git", "Git", icons::GIT_COMMIT),
                                    (1usize, "dock-tab-sub", "Subagents", icons::ROBOT),
                                    (
                                        2usize,
                                        "dock-tab-tasks",
                                        "Background tasks",
                                        icons::DOWN_WALL,
                                    ),
                                    (3usize, "dock-tab-side", "Side chats", icons::SIDECHAT),
                                ]
                                .into_iter()
                                .map(|(tab, id, label, icon)| {
                                    div()
                                        .id(id)
                                        .px_2()
                                        .py(px(2.))
                                        .rounded(px(6.))
                                        .cursor_pointer()
                                        .flex()
                                        .items_center()
                                        .gap(px(6.))
                                        .flex_none()
                                        .whitespace_nowrap()
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .when(self.dock_tab == tab, |d| d.bg(subtle))
                                        .when(self.dock_tab != tab, |d| {
                                            d.text_color(theme.muted_foreground)
                                        })
                                        .child(
                                            svg_icon(icon)
                                                .size(px(14.))
                                                .text_color(theme.muted_foreground),
                                        )
                                        .child(label)
                                        .on_click(cx.listener(move |this, _ev, _w, cx| {
                                            this.dock_tab = tab;
                                            cx.notify();
                                        }))
                                }),
                            ),
                        ),
                )
                .when(
                    self.dock_tabs_scroll.offset().x > -self.dock_tabs_scroll.max_offset().x,
                    |row| {
                        row.child(
                            div()
                                .id("dock-tabs-right")
                                .px(px(1.))
                                .cursor_pointer()
                                .text_color(theme.muted_foreground)
                                .on_click(cx.listener(|this, _ev, _w, cx| {
                                    let max = this.dock_tabs_scroll.max_offset();
                                    this.dock_tabs_scroll
                                        .set_offset(gpui::point(-max.x, px(0.)));
                                    cx.notify();
                                }))
                                .child(svg_icon(icons::CHEVRON_RIGHT).size(px(14.))),
                        )
                    },
                )
                .child(
                    Button::new("changes-refresh")
                        .ghost()
                        .compact()
                        .icon(IconName::RefreshCw)
                        .on_click(cx.listener(|this, _ev, _w, cx| {
                            this.changes_diff = None;
                            this.changes_diff_state = None;
                            this.fetch_changes(cx);
                        })),
                )
                .child(
                    Button::new("changes-close")
                        .ghost()
                        .compact()
                        .icon(
                            svg_icon(icons::X)
                                .size(px(14.))
                                .text_color(theme.muted_foreground),
                        )
                        .on_click(cx.listener(|this, _ev, _w, cx| {
                            this.view_changes = false;
                            this.blame_pop = None;
                            cx.notify();
                        })),
                ),
        );
        // -- sections (top, capped height, scrollable) --
        let selected_path = self.changes_diff.as_ref().map(|(p, _)| p.clone());
        if self.dock_tab == 1 {
            // SubagentsDockPane: threads created as children of another thread
            // (parentThreadId set) — nickname/role when present.
            let mut sub_area = v_flex()
                .w_full()
                .flex_1()
                .min_h_0()
                .id("subagents-scroll")
                .overflow_y_scrollbar()
                .px(px(6.))
                .py_2()
                .gap_1();
            let subs: Vec<&Value> = arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "threads")
                .iter()
                .filter(|t| !get(t, "parentThreadId").is_null() && get(t, "deletedAt").is_null())
                .collect();
            if subs.is_empty() {
                sub_area = sub_area.child(
                    div()
                        .px(px(8.))
                        .py_2()
                        .text_size(px(13.))
                        .text_color(theme.muted_foreground)
                        .child("No subagents yet"),
                );
            } else {
                for (si, t) in subs.iter().enumerate() {
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
                    let role = str_at(t, "subagentRole").to_string();
                    sub_area = sub_area.child(
                        div()
                            .id(SharedString::from(format!("subag-{si}")))
                            .w_full()
                            .px_2()
                            .py(px(5.))
                            .rounded(px(6.))
                            .cursor_pointer()
                            .hover(|d| d.bg(subtle))
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                this.open_thread(tid.clone(), cx)
                            }))
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .flex_1()
                                            .min_w_0()
                                            .overflow_hidden()
                                            .whitespace_nowrap()
                                            .child(name),
                                    )
                                    .when(!role.is_empty(), |d| {
                                        d.child(
                                            div()
                                                .text_size(px(11.))
                                                .text_color(theme.muted_foreground)
                                                .child(role.clone()),
                                        )
                                    }),
                            ),
                    );
                }
            }
            changes_pane = changes_pane.child(sub_area);
        } else if self.dock_tab == 2 {
            // BackgroundTasksDockPane: the thread's background tasks, derived
            // from its activities like upstream's workLog. Running rows show a
            // Stop action dispatching thread.task.stop.
            let thread_v = get(self.detail.as_ref().unwrap_or(&Value::Null), "thread").clone();
            let tasks = derive_bg_tasks(&thread_v);
            let mut task_area = v_flex()
                .w_full()
                .flex_1()
                .min_h_0()
                .id("tasks-scroll")
                .overflow_y_scrollbar()
                .px(px(6.))
                .py_2()
                .gap_1();
            if tasks.is_empty() {
                task_area = task_area.child(
                    div()
                        .px(px(8.))
                        .py_2()
                        .text_size(px(13.))
                        .text_color(theme.muted_foreground)
                        .child("No background tasks in this chat yet"),
                );
            } else {
                let running: Vec<&BgTask> = tasks
                    .iter()
                    .filter(|t| t.status == "running")
                    .rev()
                    .collect();
                let finished: Vec<&BgTask> = tasks
                    .iter()
                    .filter(|t| t.status != "running")
                    .rev()
                    .collect();
                let row = |t: &BgTask, on_stop: bool, this_tid: &str| {
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
                    let when = rel_time(t.completed_at.as_deref().unwrap_or(t.started_at.as_str()));
                    let stop_tid = t.task_id.clone();
                    let stop_thread = this_tid.to_string();
                    div()
                        .id(SharedString::from(format!("bgtask-{}", t.task_id)))
                        .w_full()
                        .px_2()
                        .py(px(5.))
                        .rounded(px(6.))
                        .child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(
                                    svg_icon(icons::TERMINAL)
                                        .size(px(14.))
                                        .text_color(if t.status == "running" {
                                            theme.foreground
                                        } else {
                                            theme.muted_foreground
                                        }),
                                )
                                .child(
                                    v_flex()
                                        .flex_1()
                                        .min_w_0()
                                        .child(
                                            div()
                                                .text_size(px(13.))
                                                .overflow_hidden()
                                                .whitespace_nowrap()
                                                .child(t.title.clone()),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(11.))
                                                .text_color(if t.status == "failed" {
                                                    theme.danger
                                                } else {
                                                    theme.muted_foreground
                                                })
                                                .child(status_label),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(theme.muted_foreground)
                                        .child(when),
                                )
                                .when(on_stop, |d| {
                                    d.child(
                                        div()
                                            .id(SharedString::from(format!(
                                                "bgtask-stop-{}",
                                                t.task_id
                                            )))
                                            .px_1()
                                            .rounded(px(4.))
                                            .cursor_pointer()
                                            .hover(|h| h.bg(theme.border))
                                            .child(
                                                svg_icon(icons::X)
                                                    .size(px(12.))
                                                    .text_color(theme.muted_foreground),
                                            )
                                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                                this.dispatch_cmd(
                                                    json!({
                                                        "type": "thread.task.stop",
                                                        "commandId": uuid(),
                                                        "threadId": stop_thread,
                                                        "taskId": stop_tid,
                                                        "createdAt": chrono::Utc::now()
                                                            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                                                    }),
                                                    cx,
                                                );
                                            })),
                                    )
                                }),
                        )
                        .into_any_element()
                };
                let tid = self.selected_thread.clone().unwrap_or_default();
                if !running.is_empty() {
                    task_area = task_area.child(
                        div()
                            .px(px(8.))
                            .pt(px(4.))
                            .pb(px(2.))
                            .text_size(px(11.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.muted_foreground)
                            .child(format!("Active · {}", running.len())),
                    );
                    for t in running {
                        task_area = task_area.child(row(t, true, &tid));
                    }
                }
                if !finished.is_empty() {
                    task_area = task_area.child(
                        div()
                            .px(px(8.))
                            .pt(px(4.))
                            .pb(px(2.))
                            .text_size(px(11.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.muted_foreground)
                            .child(format!("Done · {}", finished.len())),
                    );
                    for t in finished.into_iter().take(10) {
                        task_area = task_area.child(row(t, false, &tid));
                    }
                }
            }
            changes_pane = changes_pane.child(task_area);
        } else if self.dock_tab == 3 {
            // SidechatDockPane: side chats forked from this thread
            // (sidechatSourceThreadId), plus a "New side chat" action that
            // dispatches thread.fork.create like upstream's sidechatCreation.
            let cur_tid = self.selected_thread.clone().unwrap_or_default();
            let mut side_area = v_flex()
                .w_full()
                .flex_1()
                .min_h_0()
                .id("sidechat-scroll")
                .overflow_y_scrollbar()
                .px(px(6.))
                .py_2()
                .gap_1();
            side_area = side_area.child(
                div()
                    .id("sidechat-new")
                    .w_full()
                    .px_2()
                    .py(px(5.))
                    .rounded(px(6.))
                    .cursor_pointer()
                    .hover(|d| d.bg(subtle))
                    .on_click(cx.listener(move |this, _ev, _w, cx| {
                        let detail = this.detail.clone().unwrap_or(Value::Null);
                        let tv = get(&detail, "thread");
                        let Some(src) = this.selected_thread.clone() else {
                            return;
                        };
                        let title_seed = str_at(tv, "title");
                        let imported: Vec<Value> = arr(tv, "messages")
                            .iter()
                            .filter(|m| {
                                !str_at(m, "text").is_empty()
                                    && get(m, "streaming").as_bool() != Some(true)
                            })
                            .map(|m| {
                                json!({
                                    "messageId": uuid(),
                                    "role": str_at(m, "role"),
                                    "text": str_at(m, "text"),
                                    "createdAt": str_at(m, "createdAt"),
                                    "updatedAt": str_at(m, "updatedAt"),
                                })
                            })
                            .collect();
                        this.dispatch_cmd(
                            json!({
                                "type": "thread.fork.create",
                                "commandId": uuid(),
                                "threadId": uuid(),
                                "sourceThreadId": src,
                                "sidechatSourceThreadId": src,
                                "projectId": str_at(tv, "projectId"),
                                "title": format!("Sidechat: {title_seed}"),
                                "modelSelection": get(tv, "modelSelection"),
                                "runtimeMode": str_at(tv, "runtimeMode"),
                                "interactionMode": "default",
                                "envMode": str_at(tv, "envMode"),
                                "branch": get(tv, "branch"),
                                "worktreePath": get(tv, "worktreePath"),
                                "workingDirectory": get(tv, "workingDirectory"),
                                "associatedWorktreePath": get(tv, "associatedWorktreePath"),
                                "associatedWorktreeBranch": get(tv, "associatedWorktreeBranch"),
                                "associatedWorktreeRef": get(tv, "associatedWorktreeRef"),
                                "importedMessages": imported,
                                "createdAt": chrono::Utc::now()
                                    .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                            }),
                            cx,
                        );
                    }))
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                svg_icon(icons::PLUS)
                                    .size(px(13.))
                                    .text_color(theme.muted_foreground),
                            )
                            .child(div().text_size(px(13.)).child("New side chat")),
                    ),
            );
            let sides: Vec<&Value> = arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "threads")
                .iter()
                .filter(|t| {
                    str_at(t, "sidechatSourceThreadId") == cur_tid && get(t, "deletedAt").is_null()
                })
                .collect();
            if sides.is_empty() {
                side_area = side_area.child(
                    div()
                        .px(px(8.))
                        .py_2()
                        .text_size(px(13.))
                        .text_color(theme.muted_foreground)
                        .child("No side chats yet"),
                );
            } else {
                for (si, t) in sides.iter().enumerate() {
                    let tid = str_at(t, "id").to_string();
                    let title = {
                        let s = str_at(t, "title");
                        if s.is_empty() {
                            "Side chat".to_string()
                        } else {
                            s.to_string()
                        }
                    };
                    let when = rel_time(str_at(t, "updatedAt"));
                    side_area = side_area.child(
                        div()
                            .id(SharedString::from(format!("sidechat-{si}")))
                            .w_full()
                            .px_2()
                            .py(px(5.))
                            .rounded(px(6.))
                            .cursor_pointer()
                            .hover(|d| d.bg(subtle))
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                this.open_thread(tid.clone(), cx)
                            }))
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .flex_1()
                                            .min_w_0()
                                            .overflow_hidden()
                                            .whitespace_nowrap()
                                            .child(title),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(theme.muted_foreground)
                                            .child(when),
                                    ),
                            ),
                    );
                }
            }
            changes_pane = changes_pane.child(side_area);
        } else {
            let mut files_area = v_flex()
                .w_full()
                .flex_none()
                .max_h(px(430.))
                .id("changes-files-scroll")
                .overflow_y_scrollbar()
                .px(px(6.))
                .py_2()
                .gap_2();
            if self.changes_loading {
                files_area = files_area.child(
                    div()
                        .px(px(6.))
                        .py_1()
                        .text_size(px(13.))
                        .text_color(theme.muted_foreground.opacity(0.7))
                        .child("Loading changes..."),
                );
            } else if self.changes_unavailable {
                files_area = files_area.child(
                    div()
                        .w_full()
                        .py_2()
                        .flex()
                        .justify_center()
                        .text_size(px(14.))
                        .text_color(theme.muted_foreground.opacity(0.7))
                        .child("Source control is unavailable for this thread."),
                );
            } else if self.staged_files.is_empty() && self.unstaged_files.is_empty() {
                files_area = files_area.child(
                    div()
                        .w_full()
                        .py_2()
                        .flex()
                        .justify_center()
                        .text_size(px(14.))
                        .text_color(theme.muted_foreground.opacity(0.7))
                        .child("No changes in the working tree."),
                );
            } else {
                for (sec_title, sec_files, empty_label, unstage, action_all) in [
                    (
                        "Staged",
                        self.staged_files.clone(),
                        "No staged changes.",
                        true,
                        "Unstage all",
                    ),
                    (
                        "Changes",
                        self.unstaged_files.clone(),
                        "No unstaged changes.",
                        false,
                        "Stage all",
                    ),
                ] {
                    let adds: i64 = sec_files
                        .iter()
                        .map(|f| get(f, "insertions").as_i64().unwrap_or(0))
                        .sum();
                    let dels: i64 = sec_files
                        .iter()
                        .map(|f| get(f, "deletions").as_i64().unwrap_or(0))
                        .sum();
                    let all_paths: Vec<String> = sec_files
                        .iter()
                        .map(|f| str_at(f, "path").to_string())
                        .collect();
                    let mut header = h_flex()
                        .items_center()
                        .gap_2()
                        .px(px(6.))
                        .py_1()
                        .child(
                            div()
                                .text_size(px(13.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(theme.muted_foreground)
                                .child(sec_title),
                        )
                        .child(
                            div()
                                .rounded_full()
                                .bg(theme.muted)
                                .px(px(6.))
                                .text_size(px(11.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(theme.muted_foreground)
                                .child(sec_files.len().to_string()),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(theme.success)
                                .child(format!("+{adds}")),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(theme.danger)
                                .child(format!("-{dels}")),
                        )
                        .child(div().flex_1());
                    if !sec_files.is_empty() {
                        header = header.child(
                            Button::new(ElementId::Name(format!("chg-{sec_title}-all").into()))
                                .ghost()
                                .compact()
                                .label(action_all)
                                .on_click(cx.listener(move |this, _ev, _w, cx| {
                                    this.stage_paths(all_paths.clone(), unstage, cx);
                                })),
                        );
                    }
                    let mut section = v_flex().w_full().child(header);
                    if sec_files.is_empty() {
                        section = section.child(
                            div()
                                .px(px(6.))
                                .py_1()
                                .text_size(px(13.))
                                .text_color(theme.muted_foreground)
                                .child(empty_label),
                        );
                    } else {
                        let mut rows = v_flex().gap(px(2.));
                        for f in sec_files {
                            let path = str_at(&f, "path").to_string();
                            let ins = get(&f, "insertions").as_i64().unwrap_or(0);
                            let del = get(&f, "deletions").as_i64().unwrap_or(0);
                            let (dir, name) = match path.rfind('/') {
                                Some(i) => (format!("{}/", &path[..i]), path[i + 1..].to_string()),
                                None => (String::new(), path.clone()),
                            };
                            let p = path.clone();
                            let is_sel = selected_path.as_deref() == Some(path.as_str());
                            let mut row = h_flex()
                                .id(ElementId::Name(format!("chg-{path}").into()))
                                .w_full()
                                .gap(px(6.))
                                .items_center()
                                .rounded(px(6.))
                                .px(px(6.))
                                .py(px(4.))
                                .cursor_pointer();
                            if is_sel {
                                row = row.bg(theme.sidebar_accent);
                            } else {
                                row = row.hover(|s| s.bg(theme.sidebar_accent.opacity(0.6)));
                            }
                            row = row
                                .child(
                                    svg_icon(icons::FILE_TEXT)
                                        .size(px(16.))
                                        .text_color(theme.muted_foreground),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .text_size(px(14.))
                                        .overflow_hidden()
                                        .text_ellipsis()
                                        .whitespace_nowrap()
                                        .child(
                                            h_flex()
                                                .min_w_0()
                                                .child(
                                                    div()
                                                        .text_color(
                                                            theme.muted_foreground.opacity(0.7),
                                                        )
                                                        .child(dir),
                                                )
                                                .child(div().child(name)),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .text_color(theme.success)
                                        .child(format!("+{ins}")),
                                )
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .text_color(theme.danger)
                                        .child(format!("-{del}")),
                                );
                            let p2 = path.clone();
                            let action_icon = if unstage {
                                icons::UNSTAGE
                            } else {
                                icons::STAGE
                            };
                            row = row
                                .child(
                                    Button::new(ElementId::Name(format!("chg-act-{path}").into()))
                                        .ghost()
                                        .compact()
                                        .icon(
                                            svg_icon(action_icon)
                                                .size(px(14.))
                                                .text_color(theme.muted_foreground),
                                        )
                                        .on_click(cx.listener(move |this, _ev, _w, cx| {
                                            this.stage_paths(vec![p.clone()], unstage, cx);
                                        })),
                                )
                                .on_click(cx.listener(move |this, _ev, _w, cx| {
                                    this.open_diff(p2.clone(), cx);
                                }));
                            rows = rows.child(row);
                        }
                        section = section.child(rows);
                    }
                    files_area = files_area.child(section);
                }
            }
            changes_pane = changes_pane.child(files_area);
            // -- diff viewport over a border-t --
            let mut viewport = v_flex()
                .flex_1()
                .min_h_0()
                .w_full()
                .border_t_1()
                .border_color(theme.border.opacity(0.7));
            if let Some((dpath, _patch)) = self.changes_diff.clone() {
                if let Some(dstate) = self.changes_diff_state.clone() {
                    let _ = dpath;
                    viewport = viewport.child(
                        gpui_kit::component::diff::Diff::new(&dstate)
                            .line_number(true)
                            .syntax_highlight(true)
                            .header_visible(false)
                            .hunk_separator(gpui_kit::component::diff::DiffHunkSeparator::Metadata)
                            .hover_highlight(gpui_kit::component::diff::DiffHoverHighlight::Line)
                            .on_line_click({
                                let ent = cx.entity();
                                move |pos: &gpui_kit::component::diff::DiffLinePosition,
                                      ev: &gpui::ClickEvent,
                                      _w,
                                      cx| {
                                    let p = pos.path().to_string();
                                    let ln = pos.line() as u32;
                                    let base = matches!(
                                        pos.side(),
                                        gpui_kit::component::diff::DiffSide::Original
                                    );
                                    let (px_, py_) =
                                        (ev.position().x.as_f32(), ev.position().y.as_f32());
                                    let _ = ent.update(cx, |this, cx| {
                                        this.fetch_blame(p, ln, base, px_, py_, cx);
                                    });
                                }
                            }),
                    );
                }
            } else {
                viewport = viewport.child(
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .justify_center()
                        .px_4()
                        .text_size(px(14.))
                        .text_color(theme.muted_foreground.opacity(0.7))
                        .child("Select a file to view its diff."),
                );
            }
            changes_pane = changes_pane.child(viewport);
        }

        NativeChrome {
            changes_pane: changes_pane
                .border_l_1()
                .border_color(theme.border)
                .into_any_element(),
        }
    }
}
