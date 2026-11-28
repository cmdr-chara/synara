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

pub(super) struct NativeWorkspace {
    pub(super) sidebar: gpui::AnyElement,
    pub(super) pane: gpui::AnyElement,
    pub(super) inbox_pane: gpui::AnyElement,
    pub(super) dev_pane: gpui::AnyElement,
    pub(super) files_pane: gpui::AnyElement,
    pub(super) update_toast: gpui::Div,
    pub(super) composer_bar: gpui::AnyElement,
    pub(super) split_bar: gpui::AnyElement,
    pub(super) split_pane: gpui::AnyElement,
    pub(super) provider_banner: Option<gpui::AnyElement>,
    pub(super) thread_error_banner: Option<gpui::AnyElement>,
    pub(super) query: String,
    pub(super) toast_shown: bool,
    pub(super) has_thread: bool,
    pub(super) show_landing: bool,
    pub(super) thread_v: Value,
}

impl Native {
    pub(super) fn build_workspace(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> NativeWorkspace {
        let theme = cx.theme().clone();
        let dark_ui = match self.appearance_mode {
            2 => true,
            1 => false,
            _ => matches!(
                cx.window_appearance(),
                gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
            ),
        };
        let _well_bg = if dark_ui {
            rgb(0x1d1d1d)
        } else {
            rgb(0xf7f7f7)
        };
        let _hairline = if dark_ui {
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
        let _content_bg: gpui::Hsla = if dark_ui {
            rgb(0x111111).into()
        } else {
            theme.background
        };
        let _sidebar_bg: gpui::Hsla = if dark_ui {
            rgb(0x111111).into()
        } else {
            theme.background
        };
        let popover_bg: gpui::Hsla = if dark_ui {
            rgb(0x161616).into()
        } else {
            theme.popover
        };

        // ---- sidebar ----
        let mut sidebar = v_flex()
            .w_full()
            .h_full()
            .gap_0p5()
            .px_2()
            .pb_2()
            // Card interior below the top strip — the first row lands ~4px
            // under the card's top edge like the upstream panel.
            .pt(px(4.))
            .overflow_y_scrollbar();
        let query = self.search.read(cx).value().to_string().to_lowercase();
        // Header: bold wordmark + chevron (workspace switcher), search +
        // new-thread icon buttons on the right — mirrors the web sidebar.
        sidebar =
            sidebar.child(
                h_flex()
                    .items_center()
                    .gap_1()
                    // Measured against the web reference: upstream leaves ~8px
                    // below the workspace row before the first action row.
                    .mb_2()
                    .child(
                        // Upstream wordmark: font-display (Cal Sans) 17px, normal
                        // weight (the face is already semibold-looking).
                        div()
                            .px_1()
                            .text_size(px(17.))
                            .font_family("Cal Sans")
                            .child("Synara"),
                    )
                    .child(
                        // Upstream surface picker: active surface ("Synara") plus
                        // "Hubs", each with its subtitle. Selecting Hubs opens the
                        // hubs rail section.
                        Button::new("app-menu")
                            .ghost()
                            .compact()
                            // Upstream keeps the chevron 6px from the wordmark.
                            .ml(px(-12.))
                            .icon(
                                Icon::new(AssetIcon::ChevronDown)
                                    .size(px(14.))
                                    .text_color(theme.foreground.opacity(0.41)),
                            )
                            .dropdown_menu({
                                let weak = cx.weak_entity();
                                move |menu, _, _cx| {
                                    let w2 = weak.clone();
                                    menu.min_w(px(200.))
                                        .item(
                                            PopupMenuItem::element(|_, _| {
                                                v_flex()
                                                    .gap_0p5()
                                                    .child(div().text_size(px(13.)).child("Synara"))
                                                    .child(
                                                        div()
                                                            .text_size(px(11.))
                                                            .text_color(rgba(0x777777aa))
                                                            .child("Build, debug, and ship"),
                                                    )
                                            })
                                            .checked(true)
                                            .on_click(|_, _, _| {}),
                                        )
                                        .item(
                                            PopupMenuItem::element(|_, _| {
                                                v_flex()
                                                    .gap_0p5()
                                                    .child(div().text_size(px(13.)).child("Hubs"))
                                                    .child(
                                                        div()
                                                            .text_size(px(11.))
                                                            .text_color(rgba(0x777777aa))
                                                            .child("Coordinated work across repos"),
                                                    )
                                            })
                                            .on_click(move |_, _w, cx| {
                                                let _ = w2.update(cx, |this, cx| {
                                                    this.clear_views();
                                                    this.view_studio = true;
                                                    cx.notify();
                                                });
                                            }),
                                        )
                                }
                            }),
                    )
                    .child(div().flex_1())
                    .child(
                        icon_btn_pad(
                            "sidebar-search",
                            icons::SEARCH,
                            15.,
                            9.,
                            theme.secondary,
                            theme.foreground.opacity(0.35),
                        )
                        .on_click(cx.listener(|this, _ev, _w, cx| {
                            this.search_open = !this.search_open;
                            cx.notify();
                        })),
                    )
                    .child(
                        icon_btn_pad(
                            "sidebar-new",
                            icons::NEW_THREAD,
                            15.,
                            9.,
                            theme.secondary,
                            theme.foreground.opacity(0.44),
                        )
                        .on_click(cx.listener(|this, _ev, _w, cx| this.new_thread(cx))),
                    ),
            );
        if self.search_open {
            sidebar = sidebar.child(
                h_flex()
                    .gap_2()
                    .child(div().flex_1().child(Input::new(&self.search))),
            );
        }
        sidebar = sidebar.child(
            h_flex()
                .id("new-thread")
                .w_full()
                .h(px(28.))
                .px_2()
                .gap_2()
                .items_center()
                .rounded(px(8.))
                .cursor_pointer()
                .hover(|d| d.bg(theme.secondary))
                .child(
                    svg_icon(icons::NEW_THREAD)
                        .size(px(16.))
                        .text_color(theme.foreground),
                )
                .child(div().text_size(px(13.)).child("New thread"))
                .on_click(cx.listener(|this, _ev, _w, cx| this.new_thread(cx))),
        );
        let all_threads: Vec<Value> =
            arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "threads").to_vec();
        let live_thread = |t: &Value| {
            // Subagent threads stay out of the sidebar lists upstream —
            // they surface via the parent's SubagentRunCard and the dock.
            get(t, "deletedAt").is_null()
                && get(t, "archivedAt").is_null()
                && get(t, "parentThreadId").is_null()
                // Upstream hides snoozed threads from the sidebar until the
                // snooze fires (snoozeReminderAt) and the engine clears it.
                && get(t, "snoozedUntil").is_null()
        };
        let matches_query = |t: &Value| {
            query.is_empty()
                || str_at(t, "title").to_lowercase().contains(&query)
                || str_at(t, "latestReplyPreview")
                    .to_lowercase()
                    .contains(&query)
        };
        let pinned_ids = self.pinned_threads.clone();
        let selected_tid = self.selected_thread.clone();
        let thread_row = |thread: &Value,
                          theme: &Theme,
                          pinned: &[String],
                          selected: Option<&str>,
                          indent: f32,
                          cx: &mut Context<Self>|
         -> Stateful<Div> {
            let tid = str_at(thread, "id").to_string();
            let title = str_at(thread, "title");
            let is_pinned = pinned.contains(&tid);
            let is_sel = selected == Some(tid.as_str());
            // Upstream: the row's leading glyph is the thread's provider icon
            // (session.providerName), not a uniform marker.
            let provider_icon = provider_icon(str_at(get(thread, "session"), "providerName"));
            let mut el = h_flex()
                .id(ElementId::Name(format!("thread-{tid}").into()))
                .w_full()
                .h(px(28. * self.density_scale()))
                .pl(px(indent))
                .pr_2()
                .gap(px(8.))
                .items_center()
                .cursor_pointer()
                .rounded(px(6.))
                .child(
                    svg_icon(provider_icon)
                        .size(px(14.))
                        .text_color(theme.foreground.opacity(0.95)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .text_size(px(13.))
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .text_color(if is_sel {
                            theme.foreground
                        } else {
                            // Upstream SIDEBAR_ROW_IDLE_TEXT_CLASS_NAME is
                            // text-foreground/89 — resting rows read almost full
                            // foreground, not muted.
                            theme.foreground.opacity(0.89)
                        })
                        .child(if title.is_empty() {
                            "(untitled)".to_string()
                        } else {
                            title.to_string()
                        }),
                );
            // Upstream ThreadRowQuickActions: hovering a row reveals
            // pin / archive / done icon buttons at its trailing edge.
            let hovered = self.hover_thread.as_deref() == Some(tid.as_str());
            let settled = !str_at(thread, "settledAt").is_empty();
            if hovered {
                let mut qa = h_flex().items_center().gap_1();
                for (aid, icon, act) in [
                    (
                        "qa-pin",
                        if is_pinned {
                            icons::PIN_FILL
                        } else {
                            icons::PIN
                        },
                        "pin",
                    ),
                    ("qa-archive", icons::S_ARCHIVE, "archive"),
                    (
                        "qa-done",
                        if settled { icons::UNDO } else { icons::CHECK },
                        "done",
                    ),
                ] {
                    let tid_q = tid.clone();
                    qa = qa.child(
                        div()
                            .id(ElementId::Name(format!("{aid}-{tid_q}").into()))
                            .size(px(18.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_sm()
                            .cursor_pointer()
                            .hover(|d| d.bg(theme.muted.opacity(0.35)))
                            .child(
                                svg_icon(icon)
                                    .size(px(11.))
                                    .text_color(theme.muted_foreground),
                            )
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                cx.stop_propagation();
                                match act {
                                    "pin" => this.toggle_pin(tid_q.clone(), cx),
                                    "archive" => {
                                        let c = tid_q.clone();
                                        this.dispatch_cmd(
                                            json!({
                                                "type": "thread.archive",
                                                "commandId": uuid(),
                                                "threadId": c,
                                            }),
                                            cx,
                                        );
                                    }
                                    _ => {
                                        let c = tid_q.clone();
                                        this.dispatch_cmd(
                                            json!({
                                                "type": "thread.meta.update",
                                                "commandId": uuid(),
                                                "threadId": c,
                                                "isSettled": !settled,
                                            }),
                                            cx,
                                        );
                                    }
                                }
                            })),
                    );
                }
                el = el.child(qa);
            } else {
                // Upstream trailing slot: status glyph (SidebarStatusTrailingGlyph)
                // then the pin affordance. Completed -> accent dot, Reminder ->
                // clock, approval/input/plan -> colored status dot.
                if let Some(k) =
                    inbox_needs_you_kind(thread, &self.dismissed_status, &self.visited_threads)
                {
                    let label = needs_you_label(thread, k);
                    el = el.child(if label == "Reminder" {
                        svg_icon(icons::CLOCK)
                            .size(px(11.))
                            .text_color(rgb(0x38bdf8))
                            .into_any_element()
                    } else {
                        div()
                            .size(px(7.))
                            .rounded_full()
                            .bg(match label {
                                "Pending Approval" => theme.warning,
                                "Awaiting Input" => rgb(0x6366f1).into(),
                                "Plan Ready" => rgb(0x8b5cf6).into(),
                                _ => theme.accent,
                            })
                            .into_any_element()
                    });
                }
                if is_pinned {
                    el = el.child(
                        svg_icon(icons::PIN_FILL)
                            .size(px(11.))
                            .text_color(theme.muted_foreground),
                    );
                }
            }
            if is_sel {
                el = el.bg(sel_fill);
            } else {
                el = el.hover(|d| d.bg(theme.secondary));
            }
            let tid_ctx = tid.clone();
            let tid_drag = tid.clone();
            let title_drag = if title.is_empty() {
                "(untitled)".to_string()
            } else {
                title.to_string()
            };
            let tid_click = tid.clone();
            el = el
                .on_click(cx.listener(move |this, _ev, _w, cx| {
                    this.ctx_menu = None;
                    this.ctx_sub = None;
                    this.ctx_confirm_delete = None;
                    this.open_thread(tid_click.clone(), cx)
                }))
                .on_mouse_down(
                    gpui::MouseButton::Right,
                    cx.listener(move |this, ev: &gpui::MouseDownEvent, _w, cx| {
                        this.ctx_menu = Some((
                            tid_ctx.clone(),
                            ev.position.x.as_f32(),
                            ev.position.y.as_f32(),
                        ));
                        this.ctx_menu_tab = false;
                        this.ctx_sub = None;
                        this.ctx_confirm_delete = None;
                        this.hover_card = None;
                        this.hover_thread = None;
                        this.hover_card_seq += 1;
                        cx.notify();
                        cx.stop_propagation();
                    }),
                );
            // Upstream hover: quick-action icons appear and a delayed hover
            // card shows title/status/project/model.
            let tid_hover = tid.clone();
            let tid_hover2 = tid.clone();
            el = el.on_hover(cx.listener(move |this, hovered: &bool, w, cx| {
                if this.ctx_menu.is_some() {
                    return;
                }
                if *hovered {
                    if this.hover_thread.as_deref() != Some(tid_hover.as_str()) {
                        this.hover_thread = Some(tid_hover.clone());
                        cx.notify();
                    }
                    // Upstream hover card: short delay, then info popup.
                    this.hover_card_seq += 1;
                    let seq = this.hover_card_seq;
                    let pos = w.mouse_position();
                    let (px_, py_) = (f32::from(pos.x), f32::from(pos.y));
                    let tidc = tid_hover2.clone();
                    cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(450))
                            .await;
                        let _ = this.update(cx, |this: &mut Self, cx| {
                            if seq == this.hover_card_seq
                                && this.hover_thread.as_deref() == Some(tidc.as_str())
                            {
                                this.hover_card = Some((tidc.clone(), px_, py_));
                                cx.notify();
                            }
                        });
                    })
                    .detach();
                } else if this.hover_thread.as_deref() == Some(tid_hover.as_str()) {
                    this.hover_thread = None;
                    this.hover_card_seq += 1;
                    if this.hover_card.as_ref().map(|c| c.0.as_str()) == Some(tid_hover.as_str()) {
                        this.hover_card = None;
                    }
                    cx.notify();
                }
            }));
            // Upstream beginThreadDrag: the row drags onto the composer
            // as an @mention (and onto chat panes as a split).
            el.on_drag(ThreadDrag(tid_drag.clone()), move |_d, _pos, _w, cx| {
                cx.new(|_| DragGhost(title_drag.clone()))
            })
        };
        // Upstream "Pinned" section: pinned threads listed under their own
        // header above the project groups (they still appear in Chats too).
        let pinned_rows: Vec<Value> = all_threads
            .iter()
            .filter(|t| {
                live_thread(t)
                    && matches_query(t)
                    && pinned_ids.contains(&str_at(t, "id").to_string())
            })
            .cloned()
            .collect();
        if !pinned_rows.is_empty() {
            sidebar = sidebar.child(
                div()
                    .mt(px(15.))
                    .text_size(px(13.))
                    .text_color(theme.muted_foreground)
                    .child("Pinned"),
            );
            for t in &pinned_rows {
                sidebar = sidebar.child(thread_row(
                    t,
                    &theme,
                    &pinned_ids,
                    selected_tid.as_deref(),
                    8.,
                    cx,
                ));
            }
        }
        sidebar = sidebar.child(
            div()
                .mt(px(20.))
                .text_size(px(13.))
                .text_color(theme.muted_foreground)
                .child(if self.view_spaces {
                    // spaces — upstream navigates INTO the space: same sidebar
                    // chrome, the section label is the space name.
                    arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "spaces")
                        .first()
                        .map(|s| str_at(s, "name").to_string())
                        .filter(|n: &String| !n.is_empty())
                        .unwrap_or_else(|| "Void".to_string())
                } else {
                    "Projects".to_string()
                }),
        );
        if arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects")
            .iter()
            .filter(|p| str_at(p, "kind") != "chat")
            .count()
            == 0
        {
            sidebar = sidebar.child(
                div()
                    .w_full()
                    .pt(px(16.))
                    .px(px(8.))
                    .text_size(px(13.))
                    .text_color(theme.muted_foreground)
                    .child(div().text_center().child("No projects yet")),
            );
        }
        // Projects: only real (non-chat) projects appear here — chat-kind
        // projects (Home) surface only via the "Chats" section. Pinned first,
        // then creation order, matching the web sidebar.
        let mut projects_list: Vec<Value> =
            arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects")
                .iter()
                .filter(|p| str_at(p, "kind") != "chat")
                .cloned()
                .collect();
        projects_list.sort_by_key(|p| {
            (
                if get(p, "isPinned").as_bool().unwrap_or(false) {
                    0
                } else {
                    1
                },
                str_at(p, "createdAt").to_string(),
            )
        });
        for (project_index, project) in projects_list.iter().enumerate() {
            let pid = str_at(project, "id").to_string();
            let pid_key = pid.clone();
            // Expanded when it has visible threads nested below.
            let mut kids: Vec<Value> = all_threads
                .iter()
                .filter(|t| str_at(t, "projectId") == pid && live_thread(t) && matches_query(t))
                .cloned()
                .collect();
            // Newest threads first — the web sidebar lists latest activity on top.
            kids.sort_by_key(|t| {
                (
                    if pinned_ids.contains(&str_at(t, "id").to_string()) {
                        0
                    } else {
                        1
                    },
                    std::cmp::Reverse(str_at(t, "createdAt").to_string()),
                )
            });
            let expanded = !kids.is_empty() && !self.collapsed_projects.contains(&pid);
            let pinned_proj = get(project, "isPinned").as_bool().unwrap_or(false);
            // A project whose workspace carries a favicon (here: the synara
            // checkout) shows its logo in a hairline frame; the pin glyph wins
            // for pinned projects; otherwise the folder opens/closes.
            let has_logo = str_at(project, "workspaceRoot").ends_with("synara");
            let glyph = if pinned_proj {
                icons::PIN_FILL
            } else if has_logo {
                icons::LOGO
            } else if expanded {
                icons::FOLDER_OPEN
            } else {
                icons::FOLDER
            };
            let glyph_el = if has_logo && !pinned_proj {
                div()
                    .size(px(19.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(5.))
                    .border_1()
                    .border_color(theme.border)
                    .child(svg_icon(glyph).size(px(12.)).text_color(theme.foreground))
                    .into_any_element()
            } else {
                svg_icon(glyph)
                    .size(px(16.))
                    .text_color(theme.foreground.opacity(0.95))
                    .into_any_element()
            };
            // The disclosure chevron only appears beside the title when the
            // leading glyph cannot convey open/closed (pin or logo).
            let needs_chevron = pinned_proj || has_logo;
            let is_active_project = has_logo;
            let mut row_head = h_flex()
                .gap_1p5()
                .items_center()
                .h(px(28.))
                .mb(px(4.))
                .child(glyph_el)
                .child(
                    div()
                        .min_w_0()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .text_size(px(13.))
                        .child(str_at(project, "title").to_string()),
                );
            if needs_chevron {
                row_head = row_head.child(
                    div()
                        .text_size(px(10.))
                        .text_color(theme.muted_foreground)
                        .child(if expanded { "\u{2228}" } else { "\u{203a}" }),
                );
            }
            row_head = row_head.child(div().flex_1());
            if is_active_project {
                row_head = row_head.child(
                    div()
                        .size(px(6.))
                        .rounded_full()
                        .bg(gpui_kit::rgb(0x22c55e)),
                );
            }
            let pid_ctx = pid.clone();
            let pid_hover = pid.clone();
            let pid_hover2 = pid.clone();
            let proj_head = v_flex()
                .gap_0()
                .mt(if project_index == 0 { px(8.) } else { px(14.) })
                .id(ElementId::Name(
                    format!("project-{}", str_at(project, "id")).into(),
                ))
                .cursor_pointer()
                .rounded_md()
                .px_2()
                .child(row_head)
                .on_click(cx.listener(move |this, _ev, _w, cx| {
                    this.proj_ctx_menu = None;
                    this.proj_ctx_confirm = false;
                    // Upstream toggleProject: the row click expands/collapses
                    // the folder's thread list.
                    if this.collapsed_projects.contains(&pid) {
                        this.collapsed_projects.remove(&pid);
                    } else {
                        this.collapsed_projects.insert(pid.clone());
                    }
                    this.view_files = false;
                    cx.notify();
                }))
                .on_mouse_down(
                    gpui::MouseButton::Right,
                    cx.listener(move |this, ev: &gpui::MouseDownEvent, _w, cx| {
                        this.proj_ctx_menu = Some((
                            pid_ctx.clone(),
                            ev.position.x.as_f32(),
                            ev.position.y.as_f32(),
                        ));
                        this.proj_ctx_confirm = false;
                        this.ctx_sub = None;
                        this.fetch_dev_servers(cx);
                        // The menu replaces the hover card (upstream's
                        // PreviewCard closes as the menu opens).
                        this.hover_project = None;
                        this.project_card = None;
                        this.project_card_seq += 1;
                        cx.notify();
                        cx.stop_propagation();
                    }),
                )
                .on_hover(cx.listener(move |this, hovered: &bool, w, cx| {
                    if *hovered {
                        if this.hover_project.as_deref() != Some(pid_hover.as_str()) {
                            this.hover_project = Some(pid_hover.clone());
                            cx.notify();
                        }
                        // Upstream PreviewCard: short delay, then info popup.
                        this.project_card_seq += 1;
                        let seq = this.project_card_seq;
                        let pos = w.mouse_position();
                        let (px_, py_) = (f32::from(pos.x), f32::from(pos.y));
                        let pidc = pid_hover2.clone();
                        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                            cx.background_executor()
                                .timer(std::time::Duration::from_millis(450))
                                .await;
                            let _ = this.update(cx, |this: &mut Self, cx| {
                                if seq == this.project_card_seq
                                    && this.hover_project.as_deref() == Some(pidc.as_str())
                                {
                                    this.project_card = Some((pidc.clone(), px_, py_));
                                    cx.notify();
                                }
                            });
                        })
                        .detach();
                    } else if this.hover_project.as_deref() == Some(pid_hover.as_str()) {
                        this.hover_project = None;
                        this.project_card_seq += 1;
                        if this.project_card.as_ref().map(|c| c.0.as_str())
                            == Some(pid_hover.as_str())
                        {
                            this.project_card = None;
                        }
                        cx.notify();
                    }
                }));
            sidebar = sidebar.child(proj_head);
            if !expanded {
                continue;
            }
            let pages = *self.preview_pages.get(&pid_key).unwrap_or(&0) as usize;
            let visible = 5 + pages * 5;
            for thread in kids.iter().take(visible) {
                sidebar = sidebar.child(thread_row(
                    thread,
                    &theme,
                    &pinned_ids,
                    selected_tid.as_deref(),
                    34.,
                    cx,
                ));
            }
            if kids.len() > 5 {
                let pid_more = pid_key.clone();
                let pid_less = pid_key.clone();
                let mut page_row = h_flex()
                    .id(ElementId::Name(format!("pg-{pid_key}").into()))
                    .h(px(28.))
                    .mb(px(4.))
                    .items_center()
                    .pl(px(34.))
                    .gap_2();
                if kids.len() > visible {
                    page_row = page_row.child(
                        div()
                            .id(ElementId::Name(format!("pgm-{pid_key}").into()))
                            .cursor_pointer()
                            .text_size(px(13.))
                            .text_color(theme.muted_foreground)
                            .hover(|d| d.text_color(theme.foreground))
                            .child("Show more")
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                *this.preview_pages.entry(pid_more.clone()).or_insert(0) += 1;
                                cx.notify();
                            })),
                    );
                }
                if pages > 0 {
                    page_row = page_row.child(
                        div()
                            .id(ElementId::Name(format!("pgl-{pid_key}").into()))
                            .cursor_pointer()
                            .text_size(px(13.))
                            .text_color(theme.muted_foreground)
                            .hover(|d| d.text_color(theme.foreground))
                            .child("Show less")
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                if let Some(n) = this.preview_pages.get_mut(&pid_less) {
                                    *n = n.saturating_sub(1);
                                }
                                cx.notify();
                            })),
                    );
                }
                sidebar = sidebar.child(page_row);
            }
        }
        sidebar = sidebar.child(
            div()
                .id("chats-toggle")
                .mt_2()
                .cursor_pointer()
                .text_size(px(13.))
                .text_color(theme.muted_foreground)
                .child(if self.chats_collapsed {
                    "Chats \u{203a}"
                } else {
                    "Chats \u{2228}"
                })
                .on_click(cx.listener(|this, _ev, _w, cx| {
                    this.chats_collapsed = !this.chats_collapsed;
                    cx.notify();
                })),
        );
        // "Chats" = threads filed under chat-kind projects (Home), matching
        // the web sidebar's home chats section.
        let chat_project_ids: Vec<String> =
            arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects")
                .iter()
                .filter(|p| str_at(p, "kind") == "chat")
                .map(|p| str_at(p, "id").to_string())
                .collect();
        let mut chat_threads: Vec<Value> = if self.chats_collapsed {
            Vec::new()
        } else {
            all_threads
                .iter()
                .filter(|t| {
                    chat_project_ids
                        .iter()
                        .any(|id| id == str_at(t, "projectId"))
                        && get(t, "parentThreadId").is_null()
                        && live_thread(t)
                        && matches_query(t)
                })
                .cloned()
                .collect()
        };
        chat_threads.sort_by_key(|t| {
            if pinned_ids.contains(&str_at(t, "id").to_string()) {
                0
            } else {
                1
            }
        });
        {
            let pages = *self.preview_pages.get("chats").unwrap_or(&0) as usize;
            let visible = 5 + pages * 5;
            for thread in chat_threads.iter().take(visible) {
                sidebar = sidebar.child(thread_row(
                    thread,
                    &theme,
                    &pinned_ids,
                    selected_tid.as_deref(),
                    8.,
                    cx,
                ));
            }
            if chat_threads.is_empty() && !self.chats_collapsed {
                sidebar = sidebar.child(
                    div()
                        .pl(px(6.))
                        .py(px(6.))
                        .text_size(px(13.))
                        .text_color(theme.muted_foreground.opacity(0.48))
                        .child("No chats yet"),
                );
            }
            if chat_threads.len() > 5 {
                let mut page_row = h_flex()
                    .id("pg-chats")
                    .h(px(28.))
                    .mb(px(4.))
                    .items_center()
                    .pl(px(8.))
                    .gap_2();
                if chat_threads.len() > visible {
                    page_row = page_row.child(
                        div()
                            .id("pgm-chats")
                            .cursor_pointer()
                            .text_size(px(13.))
                            .text_color(theme.muted_foreground)
                            .hover(|d| d.text_color(theme.foreground))
                            .child("Show more")
                            .on_click(cx.listener(|this, _ev, _w, cx| {
                                *this.preview_pages.entry("chats".to_string()).or_insert(0) += 1;
                                cx.notify();
                            })),
                    );
                }
                if pages > 0 {
                    page_row = page_row.child(
                        div()
                            .id("pgl-chats")
                            .cursor_pointer()
                            .text_size(px(13.))
                            .text_color(theme.muted_foreground)
                            .hover(|d| d.text_color(theme.foreground))
                            .child("Show less")
                            .on_click(cx.listener(|this, _ev, _w, cx| {
                                if let Some(n) = this.preview_pages.get_mut("chats") {
                                    *n = n.saturating_sub(1);
                                }
                                cx.notify();
                            })),
                    );
                }
                sidebar = sidebar.child(page_row);
            }
        }
        // Server-side content matches (searchThreads).
        if !self.search_hits.is_empty() {
            sidebar = sidebar.child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.muted_foreground)
                    .child("CONTENT MATCHES"),
            );
            for hit in self.search_hits.clone() {
                let tid = str_at(&hit, "threadId").to_string();
                let count = get(&hit, "matchCount").as_i64().unwrap_or(0);
                let excerpt = str_at(&hit, "excerpt");
                // Show the thread's title when we know it.
                let known = arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "threads")
                    .iter()
                    .find(|t| str_at(t, "id") == tid)
                    .map(|t| str_at(t, "title").to_string())
                    .filter(|t| !t.is_empty())
                    .unwrap_or_else(|| tid.chars().take(8).collect());
                sidebar = sidebar.child(row(
                    ElementId::Name(format!("hit-{tid}-{count}").into()),
                    format!("{known} (x{count})"),
                    excerpt.chars().take(80).collect(),
                    false,
                    cx,
                    move |this, _ev, _w, cx| this.open_thread(tid.clone(), cx),
                ));
            }
        }
        // ---- SidebarFooter: DebugFeatureFlagsMenu (upstream dev tool) ----
        // The flyout opens upward above the footer row, like upstream's
        // ComposerPickerMenuPopup side="top".
        {
            const FLAG_ROWS: [(&'static str, &'static str); 3] = [
                (
                    "Keep debug error toasts open",
                    "Disable auto-dismiss for locally triggered error toasts.",
                ),
                (
                    "Pin git progress toast",
                    "Keep a looping git progress toast visible for styling.",
                ),
                (
                    "Show debug task banner",
                    "Render a local sample active task banner for UI testing.",
                ),
            ];
            let mut footer = v_flex().mt_auto();
            if self.flags_menu_open {
                let mut menu = v_flex()
                    .mx(px(4.))
                    .mb(px(4.))
                    .rounded(px(10.))
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.popover)
                    .shadow_lg()
                    .py(px(6.));
                menu = menu.child(
                    div()
                        .px(px(10.))
                        .pt(px(4.))
                        .pb(px(6.))
                        .text_size(px(11.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.muted_foreground)
                        .child("Local feature flags"),
                );
                // Action row — fires stacked Git-failure toasts.
                menu = menu.child(
                    div()
                        .id("ff-trigger-toasts")
                        .mx(px(4.))
                        .px(px(8.))
                        .py(px(8.))
                        .rounded(px(6.))
                        .cursor_pointer()
                        .hover(|d| d.bg(theme.secondary))
                        .child(
                            div()
                                .text_size(px(13.))
                                .text_color(theme.foreground)
                                .child("Trigger action failed toasts"),
                        )
                        .child(
                            div()
                                .mt(px(1.))
                                .text_size(px(11.))
                                .text_color(theme.muted_foreground)
                                .child(
                                    "Show stacked Git action failure toasts for local UI testing.",
                                ),
                        )
                        .on_click(cx.listener(|this, _ev, _w, cx| {
                            this.trigger_debug_toasts(cx);
                        })),
                );
                for (i, (label, desc)) in FLAG_ROWS.iter().enumerate() {
                    let on = self.feature_flags[i];
                    let track = switch_track(
                        on,
                        text_accent(matches!(theme.mode, gpui_kit::component::ThemeMode::Dark)),
                        &theme,
                    );
                    menu = menu.child(
                        div()
                            .id(ElementId::Name(format!("ff-row-{i}").into()))
                            .mx(px(4.))
                            .px(px(8.))
                            .py(px(8.))
                            .rounded(px(6.))
                            .cursor_pointer()
                            .hover(|d| d.bg(theme.secondary))
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .child(
                                                div()
                                                    .text_size(px(13.))
                                                    .text_color(theme.foreground)
                                                    .child(*label),
                                            )
                                            .child(
                                                div()
                                                    .mt(px(1.))
                                                    .text_size(px(11.))
                                                    .text_color(theme.muted_foreground)
                                                    .child(*desc),
                                            ),
                                    )
                                    .child(track),
                            )
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                this.toggle_feature_flag(i, cx);
                            })),
                    );
                }
                menu = menu.child(
                    div()
                        .mt(px(4.))
                        .border_t_1()
                        .border_color(theme.border)
                        .px(px(10.))
                        .pt(px(6.))
                        .pb(px(4.))
                        .text_size(px(11.))
                        .text_color(theme.muted_foreground)
                        .child("Stored only in this browser profile."),
                );
                footer = footer.child(menu);
            }
            if std::env::var("SYNARA_DEV_FLAGS").is_ok() {
                footer = footer.child(
                    h_flex()
                        .id("ff-footer")
                        .mx(px(4.))
                        .px(px(8.))
                        .h(px(32.))
                        .gap(px(10.))
                        .items_center()
                        .rounded(px(8.))
                        .cursor_pointer()
                        .hover(|d| d.bg(theme.secondary))
                        .child(
                            svg_icon(icons::FLAG)
                                .size(px(15.))
                                .text_color(theme.muted_foreground),
                        )
                        .child(
                            div()
                                .text_size(px(13.))
                                .text_color(theme.muted_foreground)
                                .child("Feature flags"),
                        )
                        .on_click(cx.listener(|this, _ev, _w, cx| {
                            this.flags_menu_open = !this.flags_menu_open;
                            cx.notify();
                        })),
                );
            }
            sidebar = sidebar.child(footer);
        }
        // ---- transcript ----
        let mut pane = v_flex()
            .flex_1()
            .h_full()
            .gap_2()
            .on_drag_move::<ThreadDrag>(cx.listener(
                |this, _ev: &gpui::DragMoveEvent<ThreadDrag>, _w, cx| {
                    if this.composer_thread_over {
                        this.composer_thread_over = false;
                        cx.notify();
                    }
                },
            ));
        // Upstream shows the new-chat landing inside the thread pane when the
        // thread has no messages yet (same logo + heading + watermark + banner).
        let thread_msgs_empty = self.selected_thread.is_some()
            && arr(
                get(self.detail.as_ref().unwrap_or(&Value::Null), "thread"),
                "messages",
            )
            .is_empty();
        let landing_title: Option<String> = if self.selected_thread.is_none() {
            // Home scope: upstream global copy; project drafts use the
            // folder-specific variant below.
            Some("What should we work on?".to_string())
        } else if thread_msgs_empty {
            let tid = self.selected_thread.as_deref().unwrap_or("");
            let snap = self.snapshot.as_ref().unwrap_or(&Value::Null);
            let pid = arr(snap, "threads")
                .iter()
                .find(|t| str_at(t, "id") == tid)
                .map(|t| str_at(t, "projectId").to_string())
                .unwrap_or_default();
            let name = arr(snap, "projects")
                .iter()
                .find(|p| str_at(p, "id") == pid)
                .map(|p| str_at(p, "title").to_string())
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| "this folder".to_string());
            Some(format!("What should we do in {name}?"))
        } else {
            None
        };
        let show_landing = landing_title.is_some();
        // Landing unmounted: the next landing gets a fresh field (upstream
        // remounts AsciiWaveBackground each time).
        if !show_landing {
            self.ascii_fresh = true;
        }
        if self.selected_thread.is_some() && !show_landing {
            pane = pane.p_4();
        }
        if show_landing {
            // AsciiWaveBackground parity: animated glyph field — three
            // cycling noise patterns with crossfade, per-cell scramble,
            // center fade, and accent ripples on click. Ink alpha is bucketed
            // into five layers (upstream draws a continuous 0..0.18 alpha).
            let pane_ox = 51. + if self.sidebar_collapsed { 0. } else { 257. };
            let avail_w = (window.viewport_size().width.as_f32() - pane_ox).max(60.);
            let avail_h = (window.viewport_size().height.as_f32() - 47.).max(60.);
            if self.ascii_cell_w <= 0. {
                let measured = window
                    .text_system()
                    .shape_line(
                        "M".into(),
                        px(10.),
                        &[TextRun {
                            len: 1,
                            font: gpui::font(theme.font_family.clone()),
                            color: theme.foreground,
                            background_color: None,
                            underline: None,
                            strikethrough: None,
                        }],
                        None,
                    )
                    .width();
                self.ascii_cell_w = (f32::from(measured) + 1.).round().max(1.);
            }
            let cw = self.ascii_cell_w;
            // Size the grid to the whole viewport, not the estimated pane
            // offsets: the canvas clips the overflow, so glyphs always reach
            // the field's real edges like upstream's cols*cellW tiling.
            let cols = (window.viewport_size().width.as_f32() / cw).ceil() as usize + 1;
            let rows = (window.viewport_size().height.as_f32() / ASCII_CELL_H).ceil() as usize + 1;
            let _ = (avail_w, avail_h);
            let n_cells = cols * rows;
            if self.ascii_fresh || self.ascii_cells.len() != n_cells {
                self.ascii_t0 = std::time::Instant::now();
                self.ascii_seed = {
                    let n = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.subsec_nanos() as u64 ^ d.as_secs())
                        .unwrap_or(0);
                    (n % 1000) as f32
                };
                let mut cells = vec![0u8; n_cells];
                let mut rng = (self.ascii_seed as u32) ^ 0x9e37_79b9;
                for cell in cells.iter_mut() {
                    rng ^= rng << 13;
                    rng ^= rng >> 17;
                    rng ^= rng << 5;
                    *cell = (rng as usize % ASCII_GLYPHS.len()) as u8;
                }
                self.ascii_cells = cells;
                self.ascii_ripples.clear();
                self.ascii_fresh = false;
            }
            let now = std::time::Instant::now();
            let elapsed_ms = now.saturating_duration_since(self.ascii_t0).as_millis() as u64;
            let t = elapsed_ms as f32 / 1000.0 + self.ascii_seed;
            let reveal = smoothstep(0., ASCII_REVEAL_MS as f32, elapsed_ms as f32);
            let cycle = (elapsed_ms / ASCII_PATTERN_MS) as usize;
            let mix = smoothstep(
                (ASCII_PATTERN_MS - ASCII_CROSSFADE_MS) as f32,
                ASCII_PATTERN_MS as f32,
                (elapsed_ms % ASCII_PATTERN_MS) as f32,
            );
            let pat = &ASCII_PATTERNS[cycle % ASCII_PATTERNS.len()];
            let next_pat = if mix > 0. {
                Some(&ASCII_PATTERNS[(cycle + 1) % ASCII_PATTERNS.len()])
            } else {
                None
            };
            let fw = cols as f32 * cw;
            let fh = rows as f32 * ASCII_CELL_H;
            let (fcx, fcy) = (fw / 2., fh / 2.);
            self.ascii_ripples.retain(|r| {
                now.saturating_duration_since(r.4).as_millis() < ASCII_RIPPLE_LIFE_MS as u128
            });
            let ripples = self.ascii_ripples.clone();
            // (col, row, glyph, alpha-key): ink alpha quantized to 2/255
            // steps like the upstream 8-bit mask; 255 marks an accent cell.
            let mut draws: Vec<(u16, u16, u8, u8)> = Vec::new();
            let mut rng = (elapsed_ms as u32)
                .wrapping_mul(747_796_405)
                .wrapping_add(2_891_336_453);
            // Upstream samples the noise on a FIELD_STEP=2 lattice and
            // bilinearly interpolates per cell, giving smoother blotch edges.
            const FIELD_STEP: usize = 2;
            let lat_cols = cols / FIELD_STEP + 1;
            let lat_rows = rows / FIELD_STEP + 1;
            let mut patch_lat = vec![0f32; lat_cols * lat_rows];
            let mut wave_lat = vec![0f32; lat_cols * lat_rows];
            for ly in 0..lat_rows {
                let y = ((ly * FIELD_STEP) as f32 + 0.5) * ASCII_CELL_H;
                for lx in 0..lat_cols {
                    let x = ((lx * FIELD_STEP) as f32 + 0.5) * cw;
                    let (mut patch, mut wave) = ascii_sample(pat, x, y, fcx, fcy, t);
                    if let Some(np) = next_pat {
                        let (np_patch, np_wave) = ascii_sample(np, x, y, fcx, fcy, t);
                        patch += (np_patch - patch) * mix;
                        wave += (np_wave - wave) * mix;
                    }
                    patch_lat[ly * lat_cols + lx] = patch;
                    wave_lat[ly * lat_cols + lx] = wave;
                }
            }
            for r in 0..rows {
                for c in 0..cols {
                    let idx = r * cols + c;
                    let x = (c as f32 + 0.5) * cw;
                    let y = (r as f32 + 0.5) * ASCII_CELL_H;
                    // Bilinear lookup into the subsampled lattice.
                    let lx = (c / FIELD_STEP).min(lat_cols - 2);
                    let ly = (r / FIELD_STEP).min(lat_rows - 2);
                    let fx = (c % FIELD_STEP) as f32 / FIELD_STEP as f32;
                    let fy = (r % FIELD_STEP) as f32 / FIELD_STEP as f32;
                    let top = ly * lat_cols + lx;
                    let bot = top + lat_cols;
                    let lerp = |a: f32, b: f32, f: f32| a + (b - a) * f;
                    let patch = lerp(
                        lerp(patch_lat[top], patch_lat[top + 1], fx),
                        lerp(patch_lat[bot], patch_lat[bot + 1], fx),
                        fy,
                    );
                    let wave = lerp(
                        lerp(wave_lat[top], wave_lat[top + 1], fx),
                        lerp(wave_lat[bot], wave_lat[bot + 1], fx),
                        fy,
                    );
                    let dx = (x - fcx) / (fw * 0.3);
                    let dy = (y - fcy) / (fh * 0.3);
                    let center_fade = (0.45 + 0.55 * (dx * dx + dy * dy).sqrt()).min(1.);
                    let ink_alpha =
                        0.18 * patch * patch * (0.4 + 0.6 * wave) * center_fade * reveal;
                    let mut strength = 0.0f32;
                    if !ripples.is_empty() {
                        let warp = (value_noise3(x / 60., y / 60., t * 0.6) - 0.5) * 34.;
                        for (rx, ry, pa, pb, start) in &ripples {
                            let age = now.saturating_duration_since(*start).as_millis() as f32;
                            let ddx = x - rx;
                            let ddy = y - ry;
                            let dist = (ddx * ddx + ddy * ddy).sqrt();
                            let radius = age * 0.55;
                            if (dist - radius).abs() > 2. * 30. + 34. + 0.1 * radius {
                                continue;
                            }
                            let ang = ddy.atan2(ddx);
                            let lobes = radius
                                * (0.06 * (3. * ang + pa + t).sin()
                                    + 0.04 * (5. * ang + pb - t * 1.3).sin());
                            let off = (dist - radius - lobes - warp) / 30.;
                            if off > -2. && off < 2. {
                                let fade = 1. - age / ASCII_RIPPLE_LIFE_MS as f32;
                                strength += (-off * off).exp() * fade * fade;
                            }
                        }
                        strength = strength.min(1.);
                    }
                    let use_accent = strength > 0.08;
                    // Scramble: visible cells re-roll their glyph.
                    rng ^= rng << 13;
                    rng ^= rng >> 17;
                    rng ^= rng << 5;
                    let roll = rng as f32 / 4294967296.0;
                    if (ink_alpha >= 0.01 || use_accent) && roll < 0.025 + 0.35 * strength {
                        self.ascii_cells[idx] = (rng as usize % ASCII_GLYPHS.len()) as u8;
                    }
                    let glyph = self.ascii_cells[idx];
                    if use_accent {
                        draws.push((c as u16, r as u16, glyph, 255));
                    } else if ink_alpha >= 0.004 {
                        let aq = ((ink_alpha * 255. / 2.).round() as u8).clamp(1, 127);
                        draws.push((c as u16, r as u16, glyph, aq));
                    }
                }
            }
            // EmptyChatBackdrop (upstream bfb3bd3551): the dithered temple
            // scene replaced the ascii wave — same toggle, same click-ripple
            // surface below. Static port: no ripples/gradient style yet.
            let wm = if self.show_ascii_bg {
                // Upstream floating-sidebar layout: the landing canvas spans
                // the FULL window and the transparent sidebar floats over it —
                // the temple columns visibly continue into the sidebar region
                // in the Electron capture. The bitmap is therefore built in
                // window coordinates and mounted at the root behind everything;
                // quiet zones are offset by the rail+sidebar+strip geometry.
                let vpw = window.viewport_size().width.as_f32().max(200.0);
                let vph = window.viewport_size().height.as_f32().max(200.0);
                // Upstream EmptyChatBackdrop geometry: the landing canvas is
                // inset-0 of the CONTENT PANE (right of rail+sidebar 308,
                // below the top strip 44) — never full-window. What reads
                // through the transparent chrome is only the separate dim
                // shell atmosphere, so the scene scale and position follow
                // the pane size, not the window.
                let bw = (vpw - 308.0).max(200.0);
                let bh = (vph - 44.0).max(200.0);
                let dark = theme.is_dark();
                let key = (bw as u32, bh as u32, dark);
                if self.backdrop_key != key || self.backdrop_img.is_none() {
                    // Quiet zones approximated from the landing layout: the
                    // centered heading band and the composer card (upstream
                    // measures the DOM rects), in pane coordinates.
                    let col_w = 736.0f32.min(bw * 0.9);
                    let cx0 = (bw - col_w) / 2.0;
                    let quiet = vec![
                        (cx0, bh * 0.30, col_w, bh * 0.22),
                        (cx0, bh - 210.0, col_w, 180.0),
                    ];
                    let spec = crate::backdrop::BackdropSpec {
                        width: bw,
                        height: bh,
                        dark,
                        accent_rgb: if dark {
                            [0xf2, 0x61, 0x2d]
                        } else {
                            [0xc7, 0x46, 0x14]
                        },
                        quiet,
                        scale: window.scale_factor(),
                        alpha_scale: 1.0,
                        chrome_clip: None,
                    };
                    self.backdrop_field = crate::backdrop::compute_backdrop_field(&spec);
                    self.backdrop_img = self
                        .backdrop_field
                        .as_ref()
                        .and_then(|f| crate::backdrop::rasterize_backdrop(&spec, f, &[]));
                    self.backdrop_spec = Some(spec);
                    self.backdrop_key = key;
                }
                // Upstream: the empty-chat pane is opaque only in normal
                // window mode. With `atmosphereFullWindow` on, the content
                // surfaces go transparent (upstream index.css
                // [data-atmosphere-full]) and the shell atmosphere double-
                // prints with the landing canvas — that composite density
                // is the reference look, not a bug.
                // The landing canvas is window-sized and mounted at the root
                // behind rail+sidebar+strip, so the pane stays fully clear here
                // (the sidebar floats over the same continuous scene, matching
                // upstream's floating-sidebar layout).
                let field = div().size_full().overflow_hidden().relative();
                field
            } else {
                div().size_full()
            };
            // Clicks on empty landing space spawn accent ripples
            // (upstream AsciiWaveBackground pointer handler; interactive
            // children consume their own clicks first).
            let mut home = v_flex()
                .id("home-landing")
                .relative()
                .flex_1()
                .h_full()
                .items_center()
                .justify_center()
                .gap_5()
                .on_click(cx.listener(move |this, ev: &ClickEvent, _w, cx| {
                    let fx = ev.position().x.as_f32() - pane_ox;
                    let fy = ev.position().y.as_f32() - 47.;
                    if fx < 0. || fy < 0. {
                        return;
                    }
                    let mut rng =
                        ev.position().x.as_f32() as u32 ^ (ev.position().y.as_f32() as u32) << 16;
                    rng ^= rng << 13;
                    rng ^= rng >> 17;
                    rng ^= rng << 5;
                    let phase_a = (rng as f32 / 4294967296.0) * 6.2831855;
                    rng ^= rng << 13;
                    rng ^= rng >> 17;
                    rng ^= rng << 5;
                    let phase_b = (rng as f32 / 4294967296.0) * 6.2831855;
                    this.ascii_ripples
                        .push((fx, fy, phase_a, phase_b, std::time::Instant::now()));
                    if this.ascii_ripples.len() > ASCII_RIPPLE_MAX {
                        this.ascii_ripples.remove(0);
                    }
                    cx.notify();
                }));
            // The wm build above populates self.backdrop_img; the field itself
            // is mounted at the chat-column level (over the composer too),
            // because the scrollable pane cannot span the composer tray.
            let _ = wm;
            if !self.sponsor_dismissed {
                // Upstream SponsorLandingBanner: centered, max-w-520, borderless
                // row — star + title + subtitle; dismiss X appears only on hover.
                home = home.child(
                    div().absolute().top(px(15.)).left_0().w_full().child(
                        h_flex().w_full().justify_center().px_6().child(
                            h_flex()
                                .relative()
                                .id("sponsor-banner")
                                .group("sponsor-banner")
                                .cursor_pointer()
                                .w(px(520.))
                                .rounded(px(16.))
                                .px_4()
                                .py(px(12.))
                                .gap_4()
                                .items_center()
                                .hover(|d| d.bg(rgba(0x0000000a)))
                                .on_click(cx.listener(|_this, _ev, _w, cx| {
                                    cx.open_url("https://www.trysynara.com/sponsor");
                                }))
                                .child(
                                    div()
                                        .size(px(40.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(
                                            svg_icon(icons::STAR)
                                                .size(px(20.))
                                                .text_color(theme.primary),
                                        ),
                                )
                                .child(
                                    v_flex()
                                        .flex_1()
                                        .min_w_0()
                                        .gap_0p5()
                                        .child(
                                            div()
                                                .text_size(px(14.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .whitespace_nowrap()
                                                .child("Support Synara"),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(14.))
                                                .text_color(theme.muted_foreground)
                                                .whitespace_nowrap()
                                                .child("Explore sponsorships and help fund development"),
                                        ),
                                )
                                .child(
                                    div()
                                        .id("sponsor-dismiss")
                                        .absolute()
                                        .top(px(-6.))
                                        .right(px(-6.))
                                        .size(px(22.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded_full()
                                        .border_1()
                                        .border_color(theme.border)
                                        .bg(theme.background)
                                        .cursor_pointer()
                                        .invisible()
                                        .group_hover("sponsor-banner", |d| d.visible())
                                        .child(
                                            Icon::new(AssetIcon::X)
                                                .size(px(12.))
                                                .text_color(theme.muted_foreground),
                                        )
                                        .on_click(cx.listener(|this, _ev, _w, cx| {
                                            this.sponsor_dismissed = true;
                                            cx.notify();
                                        })),
                                ),
                        ),
                    ),
                );
            }
            pane = pane.child(
                home.child(svg_icon(icons::LOGO).size(px(40.))).child(
                    div()
                        // Upstream empty-landing h2: text-[30px] (sm:) /
                        // font-normal / leading 1.15 / foreground 95%.
                        .text_size(px(30.))
                        .line_height(px(34.5))
                        .font_weight(FontWeight::NORMAL)
                        .text_color({
                            let mut c = theme.foreground;
                            c.a *= 0.95;
                            c
                        })
                        .child(landing_title.clone().unwrap_or_default()),
                ),
            );
        }
        let thread_detail = self.detail.clone().unwrap_or(Value::Null);
        let thread_v = get(&thread_detail, "thread");
        let has_thread = self.selected_thread.is_some();
        pane = pane.child(self.transcript_body(&thread_detail, has_thread, show_landing, cx));
        // ---- files browser / viewer ----
        let mut files_pane = v_flex().flex_1().h_full().p_4().gap_1();
        if self.view_files {
            if let Some((path, text)) = self.open_file.clone() {
                if self.file_editor.is_none() {
                    let ed = cx.new(|cx| EditorState::new(window, cx));
                    cx.subscribe_in(&ed, window, |this: &mut Self, _state, event, _w, cx| {
                        if let InputEvent::Change = event {
                            this.file_dirty = true;
                            cx.notify();
                        }
                    })
                    .detach();
                    self.file_editor = Some(ed);
                }
                let editor = self.file_editor.clone().unwrap();
                let sync_key = format!(
                    "{}:{}",
                    path,
                    self.open_file_meta
                        .as_ref()
                        .map(|m| m.0.as_str())
                        .unwrap_or("")
                );
                if !self.editing_file && self.editor_synced.as_deref() != Some(&sync_key) {
                    let lang = Self::editor_language(&path);
                    let t = text.clone();
                    editor.update(cx, |s, cx| {
                        s.set_highlighter(lang, cx);
                        s.set_value(t, window, cx);
                    });
                    self.editor_synced = Some(sync_key);
                }
                let p = path.clone();
                let editing = self.editing_file;
                let mut header = h_flex()
                    .gap_2()
                    .child(
                        Button::new("close-file")
                            .ghost()
                            .label("x")
                            .on_click(cx.listener(|this, _ev, _w, cx| this.close_file(cx))),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(p),
                    );
                if editing {
                    if self.file_dirty {
                        header = header.child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child("Unsaved"),
                        );
                    }
                    header = header
                        .child(
                            Button::new("save-file")
                                .primary()
                                .label("Save")
                                .on_click(cx.listener(|this, _ev, _w, cx| this.save_file(cx))),
                        )
                        .child(Button::new("cancel-edit").ghost().label("Cancel").on_click(
                            cx.listener(|this, _ev, _w, cx| {
                                this.editing_file = false;
                                this.editor_synced = None;
                                cx.notify();
                            }),
                        ));
                } else {
                    header = header.child(
                        Button::new("open-in-editor")
                            .ghost()
                            .label("Open in\u{2026}")
                            .on_click(cx.listener(|this, _ev, _w, cx| {
                                this.open_in_menu = !this.open_in_menu;
                                cx.notify();
                            })),
                    );
                    header = header.child(Button::new("edit-file").ghost().label("Edit").on_click(
                        cx.listener(|this, _ev, _w, cx| {
                            this.editing_file = true;
                            cx.notify();
                        }),
                    ));
                }
                files_pane = files_pane.child(header).child(
                    div()
                        .flex_1()
                        .h_full()
                        .child(Editor::new(&editor).readonly(!editing)),
                );
                if self.open_in_menu {
                    // Upstream "Open in editor" (EditorActionPicker): launches
                    // the file in an installed external editor app.
                    const EDITOR_APPS: &[(&str, &str)] = &[
                        ("Cursor", "Cursor"),
                        ("VS Code", "Visual Studio Code"),
                        ("VSCodium", "VSCodium"),
                        ("Zed", "Zed"),
                        ("Windsurf", "Windsurf"),
                        ("Sublime Text", "Sublime Text"),
                    ];
                    let file_path = path.clone();
                    let mut menu = v_flex().gap_0p5().py_1();
                    let mut any_app = false;
                    for (label, app) in EDITOR_APPS {
                        if !std::path::Path::new(&format!("/Applications/{app}.app")).exists() {
                            continue;
                        }
                        any_app = true;
                        let app_name = (*app).to_string();
                        let p = file_path.clone();
                        menu = menu.child(
                            div()
                                .id(ElementId::Name(format!("open-in-{label}").into()))
                                .cursor_pointer()
                                .px_3()
                                .py(px(6.))
                                .rounded_md()
                                .text_size(px(13.))
                                .text_color(theme.foreground)
                                .child((*label).to_string())
                                .on_click(cx.listener(move |this, _ev, _w, cx| {
                                    this.open_in_menu = false;
                                    let app = app_name.clone();
                                    let path = p.clone();
                                    std::thread::spawn(move || {
                                        let _ = std::process::Command::new("open")
                                            .args(["-a", &app, &path])
                                            .spawn();
                                    });
                                    cx.notify();
                                })),
                        );
                    }
                    if !any_app {
                        menu = menu.child(
                            div()
                                .px_3()
                                .py(px(6.))
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child("No external editors found"),
                        );
                    }
                    files_pane = files_pane.child(
                        div()
                            .mx_2()
                            .mb_2()
                            .rounded_lg()
                            .border_1()
                            .border_color(theme.border)
                            .bg(popover_bg)
                            .child(menu),
                    );
                }
            } else {
                let parent = self
                    .files_dir
                    .rsplit_once('/')
                    .map(|(d, _)| d.to_string())
                    .filter(|d| !d.is_empty());
                let mut header = h_flex().gap_2().child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.muted_foreground)
                        .child(self.files_dir.clone()),
                );
                if let Some(parent) = parent {
                    header = header.child(
                        div()
                            .id("files-up")
                            .cursor_pointer()
                            .text_xs()
                            .text_color(theme.primary)
                            .child("..")
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                this.load_dir(parent.clone(), cx)
                            })),
                    );
                }
                files_pane = files_pane.child(header);
                // Upstream WorkspaceSearchInputHeader: fuzzy file search over
                // the workspace; empty query shows the tree, non-empty the
                // flat result rows.
                files_pane = files_pane.child(
                    h_flex()
                        .items_center()
                        .gap_1p5()
                        .px_2()
                        .h(px(30.))
                        .rounded_md()
                        .bg(subtle)
                        .child(
                            svg_icon(icons::SEARCH)
                                .size(px(13.))
                                .text_color(theme.muted_foreground),
                        )
                        .child(
                            div()
                                .flex_1()
                                .child(Input::new(&self.file_search).appearance(false)),
                        ),
                );
                let fs_query = self.file_search.read(cx).value().trim().to_string();
                if !fs_query.is_empty() {
                    if self.file_search_results.is_empty() {
                        files_pane = files_pane.child(
                            div()
                                .text_size(px(13.))
                                .text_color(theme.muted_foreground)
                                .child("loading\u{2026}"),
                        );
                    }
                    let mut srows = v_flex().gap_0p5();
                    for entry in self.file_search_results.clone() {
                        let rel = str_at(&entry, "path").to_string();
                        let name = rel.rsplit('/').next().unwrap_or(rel.as_str()).to_string();
                        let dir_part = rel
                            .rsplit_once('/')
                            .map(|(d, _)| d.to_string())
                            .unwrap_or_default();
                        let dir = self.files_dir.clone();
                        let abs = if rel.starts_with('/') {
                            rel.clone()
                        } else {
                            format!("{dir}/{rel}")
                        };
                        let mut e2 = entry.clone();
                        if let Some(m) = e2.as_object_mut() {
                            m.insert("path".into(), json!(abs));
                        }
                        srows = srows.child(
                            h_flex()
                                .id(ElementId::Name(format!("fs-{rel}").into()))
                                .w_full()
                                .h(px(32.))
                                .items_center()
                                .gap_1p5()
                                .px_2()
                                .rounded_md()
                                .cursor_pointer()
                                .hover(|d| d.bg(subtle))
                                .child(
                                    svg_icon(icons::FILE_TEXT)
                                        .size(px(14.))
                                        .text_color(theme.muted_foreground),
                                )
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .whitespace_nowrap()
                                        .child(name),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .overflow_hidden()
                                        .whitespace_nowrap()
                                        .text_size(px(12.))
                                        .text_color(theme.muted_foreground)
                                        .child(dir_part),
                                )
                                .on_click(cx.listener(move |this, _ev, _w, cx| {
                                    this.file_search.update(cx, |s, cx| s.set_value("", _w, cx));
                                    this.file_search_results.clear();
                                    this.open_entry(e2.clone(), cx);
                                }))
                                .on_mouse_down(
                                    gpui::MouseButton::Right,
                                    cx.listener(move |this, ev: &MouseDownEvent, _w, cx| {
                                        this.ctx_menu = None;
                                        this.file_ctx_menu = Some((
                                            abs.clone(),
                                            ev.position.x.into(),
                                            ev.position.y.into(),
                                        ));
                                        cx.stop_propagation();
                                        cx.notify();
                                    }),
                                ),
                        );
                    }
                    files_pane = files_pane.child(srows);
                } else {
                    if self.files_loading {
                        files_pane = files_pane.child(
                            div()
                                .text_size(px(13.))
                                .text_color(theme.muted_foreground)
                                .child("loading\u{2026}"),
                        );
                    }
                    // Explorer tree (upstream WorkspaceFilesTreeBody): lazy
                    // expand/collapse directories under the workspace root.
                    let mut rows: Vec<gpui::AnyElement> = Vec::new();
                    let root_entries = self.files.clone();
                    self.file_tree_rows(&root_entries, 0, &theme, subtle.into(), &mut rows, cx);
                    files_pane = files_pane.child(v_flex().gap_0p5().children(rows));
                }
            }
        }

        // ---- dev servers pane ----
        let mut dev_pane = v_flex().flex_1().h_full().p_4().gap_2();
        if self.view_dev {
            dev_pane = dev_pane.child(
                h_flex()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.muted_foreground)
                            .child("DEV SERVERS"),
                    )
                    .child(
                        Button::new("dev-refresh")
                            .ghost()
                            .label("Refresh")
                            .on_click(cx.listener(|this, _ev, _w, cx| this.fetch_dev_servers(cx))),
                    ),
            );
            dev_pane = dev_pane.child(
                h_flex()
                    .gap_2()
                    .child(div().flex_1().child(Input::new(&self.dev_cmd)))
                    .child(
                        Button::new("dev-run")
                            .primary()
                            .label("Run")
                            .on_click(cx.listener(|this, _ev, w, cx| this.dev_run(w, cx))),
                    ),
            );
            if self.dev_servers.is_empty() {
                dev_pane = dev_pane.child(
                    div()
                        .text_size(px(13.))
                        .text_color(theme.muted_foreground)
                        .child("No dev servers running"),
                );
            }
            for s in self.dev_servers.clone() {
                let pid_key = str_at(&s, "projectId").to_string();
                let status = str_at(&s, "status");
                let color = match status {
                    "running" => theme.primary,
                    "failed" | "exited" | "stopped" => theme.danger,
                    _ => theme.muted_foreground,
                };
                dev_pane = dev_pane.child(
                    h_flex()
                        .gap_2()
                        .child(
                            div()
                                .flex_1()
                                .text_xs()
                                .font_family("monospace")
                                .child(str_at(&s, "command").to_string()),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(color)
                                .child(format!("{status} (pid {})", get(&s, "pid"))),
                        )
                        .child(
                            Button::new(ElementId::Name(format!("dev-stop-{pid_key}").into()))
                                .ghost()
                                .label("Stop")
                                .on_click(cx.listener(move |this, _ev, _w, cx| {
                                    this.dev_stop(pid_key.clone(), cx)
                                })),
                        ),
                );
            }
        }

        // ---- inbox pane (upstream /inbox recap layout) ----
        // Upstream inbox column: p-10 (40px) and gap-7 (28px) at desktop width.
        // measured vs upstream web capture: content block sat +24x/+21y with uniform p(40)
        let mut inbox_pane = v_flex()
            .flex_1()
            .h_full()
            .pl(px(16.))
            .pr(px(40.))
            .pt(px(19.))
            .pb(px(40.))
            .gap(px(28.));
        if self.view_inbox {
            use chrono::Timelike as _;
            let now = chrono::Local::now();
            let slot = match now.hour() {
                5..=11 => "Good morning",
                12..=17 => "Good afternoon",
                _ => "Good evening",
            };
            let greeting = format!("{}. {}.", slot, now.format("%A, %B %-d"));
            // Upstream InboxTaskList groups over the activity threads —
            // computed here so the greeting sub and the list share the result.
            let activity_threads: Vec<Value> =
                arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "threads").to_vec();
            let (needs_you, working, finished, failed) = inbox_groups(
                &activity_threads,
                &self.dismissed_status,
                &self.visited_threads,
            );
            let needs_count = needs_you.len();
            let sub = if needs_count == 0 {
                "You're all caught up.".to_string()
            } else {
                let n = needs_count;
                format!(
                    "{n} thing{} need{} you.",
                    if n == 1 { "" } else { "s" },
                    if n == 1 { "s" } else { "" }
                )
            };
            // Upstream inbox sections: rounded-2xl (18px), bg-muted, which is
            // foreground at ~0.8% in dark mode (nearly flush with the page).
            let tile = |children: gpui::AnyElement| {
                let fill = if dark_ui {
                    theme.foreground.opacity(0.008)
                } else {
                    soft_bg
                };
                div().rounded(px(18.)).bg(fill).child(children)
            };

            let unread_ids: Vec<String> = finished
                .iter()
                .map(|t| str_at(t, "id").to_string())
                .collect();
            let header_right = if unread_ids.is_empty() {
                None
            } else {
                Some(
                    Button::new("inbox-mark-all-read")
                        .outline()
                        .label("Mark all read")
                        .on_click(cx.listener(move |this, _ev, _w, cx| {
                            let now = now_iso();
                            for id in &unread_ids {
                                let prev =
                                    this.visited_threads.get(id).cloned().unwrap_or_default();
                                if now > prev {
                                    this.visited_threads.insert(id.clone(), now.clone());
                                }
                            }
                            this.persist_sidebar_ui(cx);
                            cx.notify();
                        })),
                )
            };
            let header = h_flex()
                .items_baseline()
                .gap_2()
                .child(
                    // Upstream h1: text-3xl (30px/36px) medium, tracking-tight.
                    div()
                        .text_size(px(30.))
                        .line_height(px(36.))
                        .font_weight(FontWeight::MEDIUM)
                        .child("Inbox"),
                )
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(theme.muted_foreground)
                        .child(format!("{greeting} {sub}")),
                );
            let header = h_flex()
                .w_full()
                .items_end()
                .justify_between()
                .child(header)
                .children(header_right);

            // Tasks card (left column, 4/12 of the upstream @5xl grid)
            let tasks_card = v_flex()
                .w_full()
                .gap_2()
                .p_4()
                .child(
                    h_flex()
                        .justify_between()
                        .items_center()
                        .child(
                            div()
                                .text_size(px(13.))
                                .child(format!("Today's tasks {}", self.inbox_tasks.len())),
                        )
                        .child(
                            div()
                                .text_size(px(13.))
                                .text_color(theme.muted_foreground)
                                .child("All tasks"),
                        ),
                )
                .child(
                    h_flex()
                        .items_center()
                        .gap_2()
                        .child(
                            Icon::new(AssetIcon::CircleDot)
                                .size(px(14.))
                                .text_color(theme.muted_foreground),
                        )
                        .child(
                            div()
                                .text_size(px(13.))
                                .whitespace_nowrap()
                                .text_color(theme.muted_foreground)
                                .child("Add a task for today, or press Tab to hand it to an agent"),
                        ),
                );
            // Task rows (upstream InboxTasks TaskListItem): check toggle,
            // title, due-day chip; rows linked to a thread open it on click.
            let today = now.format("%Y-%m-%d").to_string();
            let mut rows = v_flex().w_full().gap_1();
            for task in self.inbox_tasks.clone() {
                let title = str_at(&task, "title").to_string();
                if title.is_empty() {
                    continue;
                }
                let id = str_at(&task, "id").to_string().replace("null", "");
                let done = !str_at(&task, "completedAt").is_empty()
                    && str_at(&task, "completedAt") != "null";
                let due = str_at(&task, "dueDate").to_string();
                let due_label = if due.is_empty() || due == "null" {
                    String::new()
                } else if due < today {
                    "Overdue".to_string()
                } else if due == today {
                    "Today".to_string()
                } else {
                    due[..due.len().min(10)].to_string()
                };
                let tid = str_at(&task, "threadId").to_string();
                let mut row = h_flex()
                    .id(ElementId::Name(format!("inbox-task-{id}").into()))
                    .w_full()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .py(px(6.))
                    .rounded_md()
                    .child(
                        div()
                            .id(ElementId::Name(format!("inbox-task-check-{id}").into()))
                            .cursor_pointer()
                            .child(
                                Icon::new(if done {
                                    AssetIcon::CircleCheck
                                } else {
                                    AssetIcon::CircleDot
                                })
                                .size(px(14.))
                                .text_color(if done {
                                    theme.success
                                } else {
                                    theme.muted_foreground
                                }),
                            )
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                this.toggle_todo(id.clone(), !done, cx);
                            })),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(13.))
                            .text_color(if done {
                                theme.muted_foreground
                            } else {
                                theme.foreground
                            })
                            .child(title),
                    );
                if !due_label.is_empty() {
                    row = row.child(
                        div()
                            .rounded_full()
                            .px(px(6.))
                            .py(px(1.))
                            .bg(theme.muted)
                            .text_size(px(11.))
                            .text_color(if due_label == "Overdue" {
                                theme.danger
                            } else {
                                theme.muted_foreground
                            })
                            .child(due_label),
                    );
                }
                if !tid.is_empty() && tid != "null" {
                    row = row.cursor_pointer().on_click(
                        cx.listener(move |this, _ev, _w, cx| this.open_thread(tid.clone(), cx)),
                    );
                }
                rows = rows.child(row);
            }
            let tasks_card = tasks_card.child(rows);

            // Right column: upstream RecapDigest — DigestCard (part-styled
            // sentences + hourly slot columns) and the compact tile grid.
            let mut right_col = v_flex().flex_1().gap_3();
            let recap = self.inbox_recap.clone().unwrap_or(Value::Null);
            let prev = self.inbox_recap_prev.clone().unwrap_or(Value::Null);
            let now_ms = chrono::Local::now().timestamp_millis() as f64;
            let hour_ms = 3_600_000.0;
            let (day_from, day_to, _b) = inbox_day_range();
            let (pfrom, pto, _pb) = inbox_day_range_at(day_from - 1.0);
            let as_of_ms = chrono::DateTime::parse_from_rfc3339(str_at(&recap, "generatedAt"))
                .map(|d| d.timestamp_millis() as f64)
                .unwrap_or(now_ms);
            let updated_label = if str_at(&recap, "generatedAt").is_empty() {
                String::new()
            } else {
                format!("Updated {}", clock_time(str_at(&recap, "generatedAt")))
            };
            // Hourly buckets keyed by start ms (upstream recapBucketsByStart).
            let hours_of = |rc: &Value| -> Vec<(f64, Value)> {
                arr(rc, "slots")
                    .iter()
                    .filter_map(|b| {
                        chrono::DateTime::parse_from_rfc3339(str_at(b, "from"))
                            .ok()
                            .map(|d| (d.timestamp_millis() as f64, b.clone()))
                    })
                    .collect()
            };
            let buckets = hours_of(&recap);
            let prev_buckets = hours_of(&prev);
            fn bucket_at<'b>(list: &'b [(f64, Value)], from: f64) -> Option<&'b Value> {
                list.iter()
                    .find(|(f, _)| (*f - from).abs() < 1.0)
                    .map(|(_, b)| b)
            }
            // sumRecapBefore: yesterday's totals up to "this time" pro rata.
            let cutoff = as_of_ms - 24.0 * hour_ms;
            let mut yest = (0.0f64, 0.0f64, 0.0f64, 0.0f64); // prompts, turns, tokens, work
            {
                let mut h = pfrom;
                while h < pto {
                    let hto = (h + hour_ms).min(pto);
                    if let Some(b) = bucket_at(&prev_buckets[..], h) {
                        if h < cutoff {
                            let share = ((cutoff - h) / (hto - h)).min(1.0);
                            yest.0 += num(b.get("prompts")) * share;
                            yest.1 += num(b.get("turns")) * share;
                            yest.2 += recap_tokens(b) * share;
                            yest.3 += num(b.get("agentWorkMs")) * share;
                        }
                    }
                    h = hto;
                }
            }
            // summarizeInboxSlots: started slots only; idle past slots hidden.
            let slot_ranges = [
                ("Morning", day_from, day_from + 8.0 * hour_ms),
                (
                    "Afternoon",
                    day_from + 8.0 * hour_ms,
                    day_from + 14.0 * hour_ms,
                ),
                ("Evening", day_from + 14.0 * hour_ms, day_to),
            ];
            // (label, is_now, prompts, agentWorkMs, hours[(tokens, future)])
            let mut slot_sums: Vec<(&'static str, bool, f64, f64, Vec<(f64, bool)>)> = Vec::new();
            for (label, from, to) in slot_ranges {
                if from > now_ms {
                    continue;
                }
                let mut hours: Vec<(f64, bool)> = Vec::new();
                let mut prompts = 0.0;
                let mut turns = 0.0;
                let mut tokens = 0.0;
                let mut work = 0.0;
                let mut h = from;
                while h < to && h < day_to {
                    if let Some(b) = bucket_at(&buckets[..], h) {
                        prompts += num(b.get("prompts"));
                        turns += num(b.get("turns"));
                        tokens += recap_tokens(b);
                        work += num(b.get("agentWorkMs"));
                        hours.push((recap_tokens(b), h > now_ms));
                    }
                    h += hour_ms;
                }
                let is_now = now_ms < to;
                let idle = prompts == 0.0 && turns == 0.0 && tokens == 0.0 && work == 0.0;
                if is_now || !idle {
                    slot_sums.push((label, is_now, prompts, work, hours));
                }
            }
            let totals = get(&recap, "totals").clone();
            let prev_totals = get(&prev, "totals").clone();
            let sent_prompts = num(totals.get("prompts"));
            let chats = num(totals.get("chats"));
            let work_ms = num(totals.get("agentWorkMs"));
            let turns = num(totals.get("turns"));
            let failed_turns = num(totals.get("failedTurns"));
            let tokens = recap_tokens(&totals);
            let cache_read = num(totals.get("cacheReadTokens")).min(tokens);
            let prev_prompts = num(prev_totals.get("prompts"));
            let prev_turns = num(prev_totals.get("turns"));
            let prev_tokens = recap_tokens(&prev_totals);
            let models = arr(&recap, "models").to_vec();
            let projects = arr(&recap, "projects").to_vec();
            let prev_models = arr(&prev, "models").to_vec();
            let busiest = slot_sums
                .iter()
                .filter(|_s| slot_sums.len() >= 2)
                .max_by(|a, b| a.3.partial_cmp(&b.3).unwrap_or(std::cmp::Ordering::Equal))
                .map(|s| s.0);
            // changeAmount: "17% more" / "20% fewer" vs yesterday (tone good/bad).
            let change_amount =
                |today: f64, yesterday: f64, fewer: &str| -> Option<(String, &'static str)> {
                    if yesterday < 1.0 {
                        return None;
                    }
                    let change = ((today - yesterday) / yesterday * 100.0).round();
                    if change == 0.0 {
                        return None;
                    }
                    if change > 0.0 {
                        Some((format!("{change:.0}% more"), "good"))
                    } else {
                        Some((format!("{:.0}% {}", change.abs(), fewer), "bad"))
                    }
                };
            // Digest parts: (text, strong, tone, provider-icon-key).
            type Part = (String, bool, &'static str, Option<&'static str>);
            let txt = |t: String| -> Part { (t, false, "", None) };
            let stg = |t: String| -> Part { (t, true, "", None) };
            let stg_t = |t: String, tone: &'static str| -> Part { (t, true, tone, None) };
            let stg_i = |t: String, p: &'static str| -> Part { (t, true, "", Some(p)) };
            let mut sentences: Vec<Vec<Part>> = Vec::new();
            if sent_prompts > 0.0 {
                let mut parts = vec![
                    txt("You sent ".into()),
                    stg(plural(sent_prompts, "prompt", "prompts")),
                    txt(" across ".into()),
                    stg(plural(chats, "chat", "chats")),
                ];
                if let Some((words, tone)) = change_amount(sent_prompts, yest.0, "fewer") {
                    parts.push(txt(", ".into()));
                    parts.push(stg_t(words, tone));
                    parts.push(txt(" than this time yesterday.".into()));
                } else {
                    parts.push(txt(".".into()));
                }
                sentences.push(parts);
            } else if turns > 0.0 {
                sentences.push(vec![
                    txt("Your automations and agents ran ".into()),
                    stg(plural(turns, "turn", "turns")),
                    txt(" so far today.".into()),
                ]);
            }
            let top_project = projects.first();
            let best_model = models.first();
            let next_model = models.get(1);
            if top_project.is_some() || best_model.is_some() {
                let mut parts: Vec<Part> = Vec::new();
                if let Some(pj) = top_project {
                    parts.push(txt("Most of it went into ".into()));
                    parts.push(stg(str_at(pj, "title").to_string()));
                }
                if let Some(bm) = best_model {
                    let bprov = match str_at(bm, "provider") {
                        "pi" => "pi",
                        "claude" => "claude",
                        "devin" => "devin",
                        "cursor" => "cursor",
                        "opencode" => "opencode",
                        "grok" => "grok",
                        _ => "codex",
                    };
                    let m = stg_i(str_at(bm, "model").to_string(), bprov);
                    if top_project.is_some() {
                        parts.push(txt(", with ".into()));
                    }
                    parts.push(m);
                    let share = if num(bm.get("tokens")) > 0.0 && tokens > 0.0 {
                        Some((num(bm.get("tokens")) / tokens * 100.0).round() as i64)
                    } else {
                        None
                    };
                    if let Some(share) = share.filter(|s| *s < 100) {
                        parts.push(txt(" carrying ".into()));
                        parts.push(stg(format!("{share}%")));
                        parts.push(txt(" of the tokens".into()));
                        if let Some(nm) = next_model {
                            let only_two = models.len() == 2;
                            parts.push(txt(if only_two {
                                " and ".into()
                            } else {
                                ", ahead of ".into()
                            }));
                            parts.push(stg(str_at(nm, "model").to_string()));
                            if only_two {
                                parts.push(txt(" the rest".into()));
                            }
                        }
                    } else if next_model.is_some() {
                        parts.push(txt(if top_project.is_some() {
                            " doing most of the work".into()
                        } else {
                            " did most of the work".into()
                        }));
                    } else {
                        parts.push(txt(if top_project.is_some() {
                            " doing the work".into()
                        } else {
                            " did all the work".into()
                        }));
                    }
                }
                parts.push(txt(".".into()));
                sentences.push(parts);
            }
            if work_ms >= 1_000.0 {
                let mut parts = vec![txt("Your agents ran for ".into()), stg(fmt_dur_ms(work_ms))];
                if let Some(b) = busiest {
                    parts.push(txt(format!(", mostly in the {}", b.to_lowercase())));
                }
                if failed_turns > 0.0 {
                    parts.push(txt(", and ".into()));
                    parts.push(stg_t(
                        format!("{} failed", plural(failed_turns, "turn", "turns")),
                        "bad",
                    ));
                }
                parts.push(txt(".".into()));
                sentences.push(parts);
            }
            if sentences.is_empty() && (prev_prompts > 0.0 || prev_tokens > 0.0) {
                let mut parts: Vec<Part> = Vec::new();
                if prev_prompts > 0.0 {
                    parts.push(txt("Nothing ran yet today. Yesterday you sent ".into()));
                    parts.push(stg(plural(prev_prompts, "prompt", "prompts")));
                    parts.push(txt(" and used ".into()));
                } else {
                    parts.push(txt(
                        "Nothing ran yet today. Yesterday your agents used ".into()
                    ));
                }
                parts.push(stg(format!("{} tokens", fmt_compact(prev_tokens))));
                if let Some(bm) = prev_models.first() {
                    parts.push(txt(", mostly with ".into()));
                    parts.push(stg(str_at(bm, "model").to_string()));
                }
                parts.push(txt(".".into()));
                sentences.push(parts);
            }
            // Task digest sentence (upstream buildTaskDigest).
            {
                let today_str = chrono::Local::now().format("%Y-%m-%d").to_string();
                let mut open = 0.0;
                let mut done = 0.0;
                let mut overdue = 0.0;
                for t in &self.inbox_tasks {
                    let ca = str_at(t, "completedAt");
                    if !ca.is_empty() && ca != "null" {
                        done += 1.0;
                    } else {
                        open += 1.0;
                        let due = str_at(t, "dueDate");
                        if !due.is_empty() && due != "null" && due < today_str.as_str() {
                            overdue += 1.0;
                        }
                    }
                }
                if open > 0.0 || done > 0.0 {
                    let mut parts: Vec<Part> = Vec::new();
                    if open == 0.0 {
                        parts.push(txt(if done == 1.0 {
                            "You finished your ".into()
                        } else {
                            "You finished all ".into()
                        }));
                        parts.push(stg(plural(done, "task", "tasks")));
                        parts.push(txt(" for today.".into()));
                    } else {
                        if done > 0.0 {
                            parts.push(txt("You finished ".into()));
                            parts.push(stg(plural(done, "task", "tasks")));
                            parts.push(txt(" today, ".into()));
                            parts.push(stg(format!("{} to go", open as i64)));
                        } else {
                            parts.push(txt("You have ".into()));
                            parts.push(stg(plural(open, "task", "tasks")));
                            parts.push(txt(" to do today".into()));
                        }
                        if overdue > 0.0 {
                            parts.push(txt(", ".into()));
                            parts.push(stg_t(format!("{} overdue", overdue as i64), "bad"));
                        }
                        parts.push(txt(".".into()));
                    }
                    sentences.push(parts);
                }
            }

            if sentences.is_empty() {
                right_col = right_col.child(tile(
                    div()
                        .p_5()
                        .text_size(px(15.))
                        .text_color(theme.muted_foreground)
                        .child("Nothing ran yet today. This fills in as your agents work.")
                        .into_any_element(),
                ));
            } else {
                // One flowing paragraph: muted ui-xl text, facts in medium fg.
                let mut para = h_flex().w_full().flex_wrap().gap_y(px(2.));
                for (i, sentence) in sentences.iter().enumerate() {
                    if i > 0 {
                        para = para.child(div().child(" "));
                    }
                    for (text, strong, tone, icon) in sentence {
                        let mut el = h_flex().items_baseline();
                        if let Some(p) = icon {
                            el = el.child(
                                svg_icon(provider_icon(p))
                                    .size(px(19.))
                                    .text_color(theme.foreground),
                            );
                        }
                        el = el.child(
                            div()
                                .text_size(px(19.))
                                .when(*strong, |d| d.font_weight(FontWeight::MEDIUM))
                                .text_color(match *tone {
                                    "good" => theme.success,
                                    "bad" => theme.danger,
                                    _ => {
                                        if *strong {
                                            theme.foreground
                                        } else {
                                            theme.muted_foreground
                                        }
                                    }
                                })
                                .child(text.clone()),
                        );
                        para = para.child(el);
                    }
                }
                // Slot columns with hourly bars (upstream SlotColumns).
                let mut strip = h_flex().w_full().gap_4();
                let max_tokens = slot_sums
                    .iter()
                    .flat_map(|s| s.4.iter().map(|h| h.0))
                    .fold(1.0f64, f64::max);
                for (label, is_now, prompts, _work, hours) in &slot_sums {
                    let mut bars = h_flex()
                        .w_full()
                        .h(px(28.))
                        .items_end()
                        .justify_between()
                        .gap(px(3.));
                    for (tok, future) in hours {
                        let hgt = ((*tok / max_tokens) * 28.0).max(3.0);
                        bars = bars.child(
                            div()
                                .w_full()
                                .max_w(px(8.))
                                .h(px(hgt as f32))
                                .rounded(px(2.))
                                .bg(if *future || *tok == 0.0 {
                                    theme.muted
                                } else if *is_now {
                                    theme.foreground
                                } else {
                                    theme.muted_foreground
                                }),
                        );
                    }
                    strip = strip.child(
                        v_flex()
                            .flex_1()
                            .gap_2()
                            .child(
                                h_flex()
                                    .items_baseline()
                                    .justify_between()
                                    .child(
                                        h_flex()
                                            .child(
                                                div()
                                                    .text_size(px(13.))
                                                    .text_color(theme.muted_foreground)
                                                    .child(label.to_string()),
                                            )
                                            .when(*is_now, |e| {
                                                e.child(
                                                    div()
                                                        .text_size(px(13.))
                                                        .text_color(theme.foreground)
                                                        .child(", now"),
                                                )
                                            }),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .text_color(if *is_now {
                                                theme.foreground
                                            } else {
                                                theme.muted_foreground
                                            })
                                            .child(plural(*prompts, "prompt", "prompts")),
                                    ),
                            )
                            .child(bars),
                    );
                }
                right_col = right_col.child(tile(
                    v_flex()
                        .p_5()
                        .gap_4()
                        .child(
                            h_flex()
                                .justify_between()
                                .items_baseline()
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .text_color(theme.muted_foreground)
                                        .child("Your day so far"),
                                )
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .text_color(theme.muted_foreground)
                                        .child(updated_label),
                                ),
                        )
                        .child(para)
                        .when(!slot_sums.is_empty(), |c| {
                            c.child(
                                v_flex()
                                    .pt_4()
                                    .border_t_1()
                                    .border_color(theme.border)
                                    .child(strip),
                            )
                        })
                        .into_any_element(),
                ));
            }

            // Compact tiles (upstream buildInboxTiles order).
            let mut stat_tiles: Vec<gpui::Div> = Vec::new();
            if let Some(bm) = best_model {
                let share = if num(bm.get("tokens")) > 0.0 && tokens > 0.0 {
                    Some((num(bm.get("tokens")) / tokens * 100.0).round() as i64)
                } else {
                    None
                };
                let detail = if let Some(s) = share {
                    format!(
                        "{s}% of tokens, {}",
                        plural(num(bm.get("turns")), "turn", "turns")
                    )
                } else {
                    format!(
                        "{}{}",
                        plural(num(bm.get("turns")), "turn", "turns"),
                        if str_at(bm, "provider") == "unknown" || str_at(bm, "provider").is_empty()
                        {
                            String::new()
                        } else {
                            format!(" through {}", str_at(bm, "provider"))
                        }
                    )
                };
                stat_tiles.push(
                    tile(
                        v_flex()
                            .min_h(px(104.))
                            .justify_between()
                            .gap_2()
                            .px(px(14.))
                            .py(px(12.))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(theme.muted_foreground)
                                    .child("Your best model was"),
                            )
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        svg_icon(provider_icon(str_at(bm, "provider")))
                                            .size(px(18.))
                                            .text_color(theme.foreground),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(19.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .child(str_at(bm, "model").to_string()),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(theme.muted_foreground)
                                    .child(detail),
                            )
                            .into_any_element(),
                    )
                    .flex_1(),
                );
            }
            if let Some(pj) = top_project {
                let np = projects.get(1);
                let detail = format!(
                    "{}{}",
                    plural(num(pj.get("prompts")), "prompt", "prompts"),
                    if let Some(np) = np {
                        format!(", then {}", str_at(np, "title"))
                    } else {
                        format!(", {} tokens", fmt_compact(num(pj.get("tokens"))))
                    }
                );
                stat_tiles.push(
                    tile(
                        v_flex()
                            .min_h(px(104.))
                            .justify_between()
                            .gap_2()
                            .px(px(14.))
                            .py(px(12.))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(theme.muted_foreground)
                                    .child("You worked most on"),
                            )
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        svg_icon(icons::FOLDER)
                                            .size(px(18.))
                                            .text_color(theme.foreground),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(19.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .child(str_at(pj, "title").to_string()),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(theme.muted_foreground)
                                    .child(detail),
                            )
                            .into_any_element(),
                    )
                    .flex_1(),
                );
            }
            if tokens > 0.0 {
                let (detail, tone) = if cache_read > 0.0 {
                    (
                        format!(
                            "{} other \u{00b7} {} from cache",
                            fmt_compact(tokens - cache_read),
                            fmt_compact(cache_read)
                        ),
                        "",
                    )
                } else if let Some((w, t)) = change_amount(tokens, yest.2, "less") {
                    (format!("{w} than this time yesterday"), t)
                } else {
                    ("Cache reads included".to_string(), "")
                };
                stat_tiles.push(
                    tile(
                        v_flex()
                            .min_h(px(104.))
                            .justify_between()
                            .gap_2()
                            .px(px(14.))
                            .py(px(12.))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(theme.muted_foreground)
                                    .child("You used"),
                            )
                            .child(
                                div()
                                    .text_size(px(19.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(format!("{} tokens", fmt_compact(tokens))),
                            )
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(if tone == "good" {
                                        theme.success
                                    } else {
                                        theme.muted_foreground
                                    })
                                    .child(detail),
                            )
                            .into_any_element(),
                    )
                    .flex_1(),
                );
            }
            if work_ms >= 1_000.0 {
                let (detail, tone) = if failed_turns > 0.0 {
                    (format!("{} failed", failed_turns as i64), "bad")
                } else if let Some(b) = busiest {
                    (format!("Mostly {}", b.to_lowercase()), "")
                } else {
                    ("Nothing failed".to_string(), "")
                };
                let detail = if failed_turns > 0.0 && busiest.is_some() {
                    format!(
                        "{}, {}",
                        busiest.unwrap().to_lowercase().replacen("m", "M", 1),
                        detail
                    )
                } else {
                    detail
                };
                stat_tiles.push(
                    tile(
                        v_flex()
                            .min_h(px(104.))
                            .justify_between()
                            .gap_2()
                            .px(px(14.))
                            .py(px(12.))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(theme.muted_foreground)
                                    .child("Your agents worked for"),
                            )
                            .child(
                                div()
                                    .text_size(px(19.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(fmt_dur_ms(work_ms)),
                            )
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(if tone == "bad" {
                                        theme.danger
                                    } else {
                                        theme.muted_foreground
                                    })
                                    .child(detail),
                            )
                            .into_any_element(),
                    )
                    .flex_1(),
                );
            }
            if prev_tokens > 0.0 || prev_prompts > 0.0 {
                let activity = if prev_prompts > 0.0 {
                    plural(prev_prompts, "prompt", "prompts")
                } else if prev_turns > 0.0 {
                    plural(prev_turns, "turn", "turns")
                } else {
                    String::new()
                };
                let pm = prev_models.first();
                let detail = match (activity.is_empty(), pm) {
                    (false, Some(m)) => format!("{}, mostly {}", activity, str_at(m, "model")),
                    (false, None) => activity,
                    (true, Some(m)) => format!("Mostly {}", str_at(m, "model")),
                    (true, None) => "Cache reads included".to_string(),
                };
                stat_tiles.push(
                    tile(
                        v_flex()
                            .min_h(px(104.))
                            .justify_between()
                            .gap_2()
                            .px(px(14.))
                            .py(px(12.))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(theme.muted_foreground)
                                    .child("Yesterday you used"),
                            )
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .when_some(
                                        pm.map(|m| str_at(m, "provider").to_string())
                                            .filter(|s| !s.is_empty()),
                                        |e, p| {
                                            e.child(
                                                svg_icon(provider_icon(&p))
                                                    .size(px(18.))
                                                    .text_color(theme.foreground),
                                            )
                                        },
                                    )
                                    .child(
                                        div()
                                            .text_size(px(19.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .child(format!("{} tokens", fmt_compact(prev_tokens))),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(theme.muted_foreground)
                                    .child(detail),
                            )
                            .into_any_element(),
                    )
                    .flex_1(),
                );
            }
            // Quota: tightest window across snapshots (upstream sorts by
            // remainingPercent and keeps the first).
            let tightest = self
                .inbox_usage
                .iter()
                .min_by(|a, b| {
                    let ra = get(a, "remainingPct").as_f64().unwrap_or(100.0);
                    let rb = get(b, "remainingPct").as_f64().unwrap_or(100.0);
                    ra.partial_cmp(&rb).unwrap_or(std::cmp::Ordering::Equal)
                })
                .cloned();
            if let Some(u) = tightest {
                let provider = str_at(&u, "provider").to_string();
                let window = str_at(&u, "window").to_string();
                let window = if window.is_empty() {
                    "weekly".to_string()
                } else {
                    window.to_lowercase()
                };
                let pct = get(&u, "remainingPct")
                    .as_f64()
                    .or_else(|| get(&u, "usedPct").as_f64().map(|p| 100.0 - p))
                    .unwrap_or(93.0)
                    .round();
                let resets = str_at(&u, "resetsIn").to_string();
                stat_tiles.push(
                    tile(
                        v_flex()
                            .min_h(px(104.))
                            .justify_between()
                            .gap_2()
                            .px(px(14.))
                            .py(px(12.))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(theme.muted_foreground)
                                    .child(format!("Left on {provider}, {window}")),
                            )
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        svg_icon(provider_icon(&provider))
                                            .size(px(18.))
                                            .text_color(theme.foreground),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(19.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .child(format!("{pct:.0}%")),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .w(px(56.))
                                            .h(px(4.))
                                            .rounded(px(2.))
                                            .bg(theme.muted)
                                            .child(
                                                div()
                                                    .w(gpui::relative((pct / 100.0) as f32))
                                                    .h_full()
                                                    .rounded(px(2.))
                                                    .bg(theme.success),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .text_color(theme.muted_foreground)
                                            .child(if resets.is_empty() {
                                                "Your tightest limit".to_string()
                                            } else {
                                                format!("Resets in {resets}")
                                            }),
                                    ),
                            )
                            .into_any_element(),
                    )
                    .flex_1(),
                );
            }
            // tileSpanClassName: 4 tiles pack 2-per-row, everything else 3.
            let per_row = if stat_tiles.len() == 4 { 2 } else { 3 };
            let mut tile_rows = v_flex().w_full().gap_3();
            let mut it = stat_tiles.into_iter();
            loop {
                let mut r = h_flex().w_full().gap_3();
                let mut c = 0;
                for _ in 0..per_row {
                    if let Some(t) = it.next() {
                        r = r.child(t);
                        c += 1;
                    }
                }
                if c == 0 {
                    break;
                }
                for _ in c..per_row {
                    r = r.child(div().flex_1());
                }
                tile_rows = tile_rows.child(r);
                if c < per_row {
                    break;
                }
            }
            right_col = right_col.child(tile_rows);

            // Upstream InboxTaskList groups (computed above for the greeting):
            // the left column stacks this card above the day's tasks.
            let has_inbox_groups = !(needs_you.is_empty()
                && working.is_empty()
                && finished.is_empty()
                && failed.is_empty());
            let groups_tile = if !(needs_you.is_empty()
                && working.is_empty()
                && finished.is_empty()
                && failed.is_empty())
            {
                let reason = |k: &str| match k {
                    "approval" => "approve a command",
                    "input" => "has a question",
                    _ => "plan ready",
                };
                let mut specs: Vec<(&str, AssetIcon, Hsla, Vec<(Value, String)>)> = Vec::new();
                if !needs_you.is_empty() {
                    specs.push((
                        "Needs you",
                        AssetIcon::TriangleAlert,
                        theme.warning,
                        needs_you
                            .iter()
                            .map(|(k, t)| (t.clone(), reason(k).to_string()))
                            .collect(),
                    ));
                }
                if !working.is_empty() {
                    specs.push((
                        "In progress",
                        AssetIcon::CircleDot,
                        theme.accent,
                        working
                            .iter()
                            .map(|t| (t.clone(), "working".to_string()))
                            .collect(),
                    ));
                }
                if !finished.is_empty() {
                    specs.push((
                        "Ready for review",
                        AssetIcon::CircleCheck,
                        theme.success,
                        finished
                            .iter()
                            .map(|t| (t.clone(), "finished".to_string()))
                            .collect(),
                    ));
                }
                if !failed.is_empty() {
                    specs.push((
                        "Failed",
                        AssetIcon::X,
                        theme.danger,
                        failed
                            .iter()
                            .map(|t| (t.clone(), "failed".to_string()))
                            .collect(),
                    ));
                }
                let mut tcard = v_flex().w_full().p_4().gap_3();
                for (label, ic, color, group_rows) in specs {
                    let mut sec = v_flex().w_full().gap_1().child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.muted_foreground)
                            .child(format!("{label} {}", group_rows.len())),
                    );
                    for (t, note) in group_rows {
                        let tid = str_at(&t, "id").to_string();
                        let title = str_at(&t, "title").to_string();
                        sec = sec.child(
                            h_flex()
                                .id(ElementId::Name(format!("inbox-thr-{tid}").into()))
                                .w_full()
                                .items_center()
                                .gap_2()
                                .px_2()
                                .py(px(6.))
                                .rounded_md()
                                .cursor_pointer()
                                .hover(|d| d.bg(theme.muted))
                                .child(Icon::new(ic).size(px(14.)).text_color(color))
                                .child(
                                    div()
                                        .flex_1()
                                        .overflow_hidden()
                                        .whitespace_nowrap()
                                        .text_size(px(13.))
                                        .child(title),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(theme.muted_foreground)
                                        .child(note),
                                )
                                .on_click(cx.listener(move |this, _ev, _w, cx| {
                                    this.open_thread(tid.clone(), cx);
                                })),
                        );
                    }
                    tcard = tcard.child(sec);
                }
                tile(tcard.into_any_element()).into_any_element()
            } else {
                div().into_any_element()
            };

            let mut left_col = v_flex().w(gpui::relative(4.0 / 12.0)).gap(px(16.));
            if has_inbox_groups {
                left_col = left_col.child(groups_tile);
            }
            left_col = left_col.child(tile(tasks_card.into_any_element()).into_any_element());
            let grid = h_flex()
                .gap(px(16.))
                .items_start()
                .child(left_col)
                .child(right_col);

            // Upstream page: p-10 under the top bar, gap-7 between sections
            // (title lands 40px below the 44px bar, like the web Inbox).
            let mut page = v_flex()
                .flex_1()
                .px(px(24.))
                .pt(px(20.))
                .pb(px(32.))
                .gap(px(28.))
                .child(header)
                .child(grid);

            if self.inbox_signed_in != Some(true) {
                page = page.child(
                    h_flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_size(px(13.))
                                .text_color(theme.muted_foreground)
                                .child(
                                "Sign in to automatically save private recaps across your devices.",
                            ),
                        )
                        .child(Button::new("inbox-signin").outline().label("Sign in")),
                );
            }
            inbox_pane = inbox_pane.child(page);
        }

        // Provider-update toast (upstream toastManager warning card): global
        // overlay over the main column, centered, does not shift content.
        let mut update_toast = div()
            .absolute()
            .top(px(-12.))
            .w_full()
            .flex()
            .justify_center();
        let mut toast_shown = false;
        if !self.inbox_updates.is_empty() {
            toast_shown = true;
            let n = self.inbox_updates.len();
            let pname = {
                let id = str_at(&self.inbox_updates[0], "provider");
                match id {
                    "codex" => "Codex",
                    "claudeAgent" | "claude" => "Claude",
                    "cursor" => "Cursor",
                    "antigravity" => "Antigravity",
                    "grok" => "Grok",
                    "droid" => "Droid",
                    "devin" => "Devin",
                    "opencode" => "OpenCode",
                    "pi" => "Pi",
                    "omp" => "Oh My Pi",
                    other => other,
                }
            };
            let (title, desc) = if n == 1 {
                (
                    format!("{pname} update available"),
                    format!("{pname} has a newer version available."),
                )
            } else {
                (
                    format!("{n} provider updates available"),
                    format!(
                        "{pname} and {} more provider{} have newer versions available.",
                        n - 1,
                        if n == 2 { "" } else { "s" }
                    ),
                )
            };
            update_toast = update_toast.child(
                h_flex()
                    .w(px(364.))
                    .gap_2()
                    .items_start()
                    .rounded(px(12.))
                    .bg(popover_bg)
                    .border_1()
                    .border_color(theme.border)
                    .p(px(14.))
                    .child(
                        Icon::new(AssetIcon::TriangleAlert)
                            .size(px(16.))
                            .text_color(rgb(0xc24904)),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .gap(px(6.))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(title),
                            )
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(theme.muted_foreground)
                                    .child(desc),
                            )
                            .child(
                                h_flex()
                                    .mt(px(6.))
                                    .gap(px(16.))
                                    .child(div().text_size(px(13.)).child("Review updates"))
                                    .child(div().text_size(px(13.)).child("Update all")),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(theme.muted_foreground)
                            .child("\u{00d7}"),
                    ),
            );
        }

        // ---- composer ----
        // chat-composer-shell squircle: 19px radius card, hairline border,
        // soft surface shadow (approximated by the border + card tone).
        // Upstream approval-mode chip tint: light = theme accent orange,
        // dark = the brightened warning orange measured in the web client
        // (rgb(254,133,73) on the "Full access" menu item).
        let mode_accent = if dark_ui {
            rgb(0xfe8549)
        } else {
            rgb(0xc74614)
        };
        let draft_text = self.composer.read(cx).value().to_string();
        let draft_empty = draft_text.trim().is_empty();
        // Upstream ComposerPromptEditor: font-system-ui text-chat (13px)
        // leading-relaxed (1.625 -> ~21.1px/line), min-height 2lh (~42px),
        // padding x12/top12/bottom8; placeholder sits at the text origin.
        let attachment_row = if !self.pending_attachments.is_empty() || self.uploading {
            let mut chips = h_flex().gap_1().flex_wrap().pb_1();
            if self.uploading {
                chips = chips.child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child("uploading\u{2026}"),
                );
            }
            for (i, a) in self.pending_attachments.iter().enumerate() {
                let label = str_at(a, "name");
                let label = if label.is_empty() {
                    str_at(a, "fileName").to_string()
                } else {
                    label.to_string()
                };
                let thumb = self.attachment_thumbs.get(str_at(a, "id")).cloned();
                if let Some(render) = thumb {
                    // Upstream ComposerImageAttachmentChip (plain image): size-16
                    // rounded-xl cover thumbnail, circular solid remove button
                    // overlapping the top-right corner.
                    chips = chips.child(
                        div()
                            .id(ElementId::Name(format!("att-{i}").into()))
                            .flex_none()
                            .relative()
                            .child(
                                div()
                                    .size(px(64.))
                                    .overflow_hidden()
                                    .rounded(px(12.))
                                    .border_1()
                                    .border_color(theme.border)
                                    .child(
                                        gpui::img(gpui::ImageSource::Render(render))
                                            .object_fit(gpui::ObjectFit::Cover)
                                            .size_full(),
                                    ),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .top(px(4.))
                                    .right(px(4.))
                                    .id(ElementId::Name(format!("att-x-{i}").into()))
                                    .cursor_pointer()
                                    .size(px(20.))
                                    .rounded_full()
                                    .bg(theme.foreground.opacity(0.8))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_color(theme.background)
                                    .text_size(px(11.))
                                    .on_click(cx.listener(move |this, _ev, _w, cx| {
                                        if i < this.pending_attachments.len() {
                                            let removed = this.pending_attachments.remove(i);
                                            this.attachment_thumbs.remove(str_at(&removed, "id"));
                                        }
                                        cx.notify();
                                    }))
                                    .child("\u{00d7}"),
                            ),
                    );
                } else {
                    chips = chips.child(
                        h_flex()
                            .id(ElementId::Name(format!("att-{i}").into()))
                            .cursor_pointer()
                            .gap_1()
                            .items_center()
                            .border_1()
                            .border_color(theme.border)
                            .rounded_md()
                            .px_2()
                            .py_0p5()
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                if i < this.pending_attachments.len() {
                                    this.pending_attachments.remove(i);
                                }
                                cx.notify();
                            }))
                            .child(div().text_xs().child(format!("{label} \u{00d7}"))),
                    );
                }
            }
            Some(chips)
        } else {
            None
        };
        let mut input_zone = div()
            .relative()
            .w_full()
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _e, window, cx| {
                    // Guarantee click-to-focus regardless of how the inner
                    // editor hitbox resolves in the flex chain.
                    let input = this.composer.clone();
                    input.update(cx, |s, cx| s.focus_handle(cx).focus(window, cx));
                }),
            )
            .children(attachment_row)
            .child(
                div()
                    .w_full()
                    .min_h(px(if self.selected_thread.is_some() {
                        24.
                    } else {
                        42.25
                    }))
                    .child({
                        // Upstream: pasting an image into the composer attaches it
                        // to the draft instead of inserting text.
                        let weak = cx.weak_entity();
                        Input::new(&self.composer)
                            .appearance(false)
                            .w_full()
                            .on_paste(move |item, _window, cx| {
                                let mut handled = false;
                                for entry in &item.entries {
                                    if let gpui::ClipboardEntry::Image(image) = entry {
                                        let (mime, ext) = match image.format {
                                            gpui::ImageFormat::Png => ("image/png", "png"),
                                            gpui::ImageFormat::Jpeg => ("image/jpeg", "jpg"),
                                            gpui::ImageFormat::Gif => ("image/gif", "gif"),
                                            gpui::ImageFormat::Webp => ("image/webp", "webp"),
                                            gpui::ImageFormat::Bmp => ("image/bmp", "bmp"),
                                            gpui::ImageFormat::Svg => ("image/svg+xml", "svg"),
                                            _ => ("image/png", "png"),
                                        };
                                        let bytes = image.bytes.clone();
                                        let mime = mime.to_string();
                                        let ext = ext.to_string();
                                        let _ = weak.update(cx, |this: &mut Self, cx| {
                                            this.attach_clipboard_image(bytes, mime, ext, cx);
                                        });
                                        handled = true;
                                    }
                                }
                                handled
                            })
                    }),
            );
        if draft_empty {
            // Placeholder rendered on top: the web uses muted-foreground/40,
            // much lighter than the editor's own muted foreground tone.
            input_zone = input_zone.child(
                div()
                    .absolute()
                    .top(px(if self.selected_thread.is_some() {
                        1.
                    } else {
                        12.
                    } + if !self.pending_attachments.is_empty() || self.uploading {
                        // Attachments render inside the input zone above the
                        // editor; the absolutely-positioned placeholder must
                        // clear the 64px chip row plus the 8px upstream mb-2.
                        65.
                    } else {
                        0.
                    }))
                    .left(px(12.))
                    .text_size(px(13.))
                    .line_height(px(21.125))
                    .text_color(rgba(0x67676766))
                    .child("Do anything in Synara"),
            );
        }
        let in_thread = self.selected_thread.is_some();
        // "+" extras trigger: 28px filled circle, black/3%; opens the upstream
        // "Add" extras menu (files, attach window, goal, plan/debug/fast).
        let extras_weak = cx.weak_entity();
        let extras_cell = self.client.clone();
        let imode = self.interaction_mode;
        let fast_on = self.fast_mode;
        // Upstream ComposerExtrasTrigger: a size-7 (28px) circle with a
        // size-3.5 (14px) plus icon — not the 36/20 the shell started with.
        let plus_btn = Button::new("composer-attach")
            .compact()
            .icon(svg_icon(icons::PLUS).size(px(14.)))
            .rounded(ButtonRounded::Size(px(14.)))
            .with_variant(ButtonVariant::Custom(
                ButtonCustomVariant::new(cx)
                    .color(chip_bg.into())
                    .foreground(theme.foreground)
                    .hover(subtle.into()),
            ))
            .w(px(28.))
            .h(px(28.))
            .dropdown_menu(move |menu, window, cx| {
                let wins: Vec<(String, String, String)> = ensure_client(&extras_cell)
                    .and_then(|c| c.call("computer.listWindows", json!({})))
                    .and_then(|v| v.get("windows").cloned())
                    .and_then(|v| v.as_array().cloned())
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|w| {
                        let id = w.get("id").and_then(|s| s.as_str())?.to_string();
                        let title = w
                            .get("title")
                            .and_then(|s| s.as_str())
                            .unwrap_or("")
                            .to_string();
                        let app = w
                            .get("appName")
                            .and_then(|s| s.as_str())
                            .unwrap_or("")
                            .to_string();
                        Some((id, title, app))
                    })
                    .take(12)
                    .collect();
                let weak = extras_weak.clone();
                let mut m = menu.min_w(px(280.)).item(PopupMenuItem::label("Add")).item(
                    PopupMenuItem::new("Files and folders")
                        .icon(svg_icon(icons::PAPERCLIP).size(px(14.)))
                        .on_click(move |_, _w, cx| {
                            let _ = weak.update(cx, |this, cx| this.attach(cx));
                        }),
                );
                let weak = extras_weak.clone();
                m = m.submenu_with_icon(
                    Some(svg_icon(icons::WINDOW).size(px(14.))),
                    "Attach window",
                    window,
                    cx,
                    move |sm, _, _| {
                        let mut sm = sm.min_w(px(260.));
                        if wins.is_empty() {
                            sm = sm.item(PopupMenuItem::new("No windows").disabled(true));
                        } else {
                            for (id, title, app) in &wins {
                                let label = if title.is_empty() {
                                    app.clone()
                                } else {
                                    format!("{app} — {title}")
                                };
                                let id = id.clone();
                                let title = title.clone();
                                let weak = weak.clone();
                                sm = sm.item(
                                    PopupMenuItem::new(label)
                                        .icon(svg_icon(icons::WINDOW).size(px(14.)))
                                        .on_click(move |_, _w, cx| {
                                            let _ = weak.update(cx, |this, cx| {
                                                this.capture_window(id.clone(), title.clone(), cx);
                                            });
                                        }),
                                );
                            }
                        }
                        sm
                    },
                );
                let weak = extras_weak.clone();
                m = m.item(
                    PopupMenuItem::new("Goal")
                        .icon(svg_icon(icons::GOAL).size(px(14.)))
                        .on_click(move |_, w, cx| {
                            let _ = weak.update(cx, |this, cx| this.insert_goal(w, cx));
                        }),
                );
                for (label, icon, mode) in [
                    ("Plan Mode", icons::TODOS, "plan"),
                    ("Debug Mode", icons::BUG, "debug"),
                ] {
                    let weak = extras_weak.clone();
                    m = m.item(
                        PopupMenuItem::new(label)
                            .icon(svg_icon(icon).size(px(14.)))
                            .checked(imode == mode)
                            .on_click(move |_, _w, cx| {
                                let _ = weak.update(cx, |this, cx| {
                                    this.interaction_mode = if this.interaction_mode == mode {
                                        "default"
                                    } else {
                                        mode
                                    };
                                    cx.notify();
                                });
                            }),
                    );
                }
                let weak = extras_weak.clone();
                m.item(
                    PopupMenuItem::new("Fast Mode")
                        .icon(svg_icon(icons::ZAP).size(px(14.)))
                        .checked(fast_on)
                        .on_click(move |_, _w, cx| {
                            let _ = weak.update(cx, |this, cx| {
                                this.fast_mode = !this.fast_mode;
                                cx.notify();
                            });
                        }),
                )
            });
        // Upstream runtime-mode chip opens a picker menu: label + description
        // rows ("Ask for approval" / "Approve for me" / "Full access") with a
        // check on the active mode.
        let mode_weak = cx.weak_entity();
        let cur_mode = self.runtime_mode;
        let build_mode_chip = move |cx: &mut Context<Self>| {
            let weak = mode_weak.clone();
            let active = cur_mode;
            // Upstream composer chips: h-7 (28px) pill with text-ui-sm (12px)
            // label, size-3.5 icon, gap-1.5. gpui-kit XSmall gives the 12px
            // text; the height is overridden back to the upstream 28.
            Button::new("composer-mode")
                .compact()
                .with_size(gpui_kit::component::Size::XSmall)
                .h(px(28.))
                .icon(
                    svg_icon(match active {
                        "auto" => icons::SHIELD_CODE,
                        _ => icons::SHIELD_ACCESS,
                    })
                    .size(px(14.)),
                )
                .label(match active {
                    "approval-required" => "Ask for approval",
                    "auto" => "Approve for me",
                    _ => "Full access",
                })
                .dropdown_caret(true)
                .with_variant(ButtonVariant::Custom(
                    ButtonCustomVariant::new(cx)
                        .color(rgba(0x00000000).into())
                        .foreground(mode_accent.into())
                        .hover(rgba(0xc2490414).into()),
                ))
                .dropdown_menu(move |menu, _w, _cx| {
                    const MODES: [(&str, &str, &str); 3] = [
                        (
                            "approval-required",
                            "Ask for approval",
                            "Always ask to edit external files and use the internet",
                        ),
                        (
                            "auto",
                            "Approve for me",
                            "Only ask for actions detected as potentially unsafe",
                        ),
                        (
                            "full-access",
                            "Full access",
                            "Unrestricted access to the internet and any file on your computer",
                        ),
                    ];
                    let mut m = menu.min_w(px(300.));
                    for (id, label, desc) in MODES {
                        let checked = active == id;
                        let weak2 = weak.clone();
                        m = m.item(
                            PopupMenuItem::element(move |_, _| {
                                v_flex()
                                    .gap_0p5()
                                    .child(div().text_size(px(13.)).child(label))
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(rgba(0x777777aa))
                                            .child(desc),
                                    )
                            })
                            .checked(checked)
                            .on_click(move |_, _w, cx| {
                                let _ = weak2.update(cx, |this, cx| {
                                    this.runtime_mode = id;
                                    cx.notify();
                                });
                            }),
                        );
                    }
                    m
                })
        };
        // The thread composer labels the chip with the thread's own model.
        let thread_model = str_at(get(thread_v, "modelSelection"), "label").to_string();
        let thread_model = if thread_model.is_empty() {
            str_at(get(thread_v, "modelSelection"), "model").to_string()
        } else {
            thread_model
        };
        let model_label = if in_thread {
            if thread_model.is_empty() {
                // Upstream seeds temporary chat threads with Devin Small.
                "Devin Small".to_string()
            } else {
                // Upstream shows the model display name in the chip, not the
                // raw slug ("GPT-6 Astra", not "gpt-6-astra").
                // Upstream composerModelDisplayName: the provider's family
                // prefix repeats the icon, so the chip drops it — "GPT-6
                // Astra" reads "6 Astra", "Claude Opus 5" reads "Opus 5".
                match thread_model.as_str() {
                    "gpt-6-astra" | "GPT-6 Astra" => "6 Astra".to_string(),
                    "adaptive" => "Adaptive".to_string(),
                    "swe-1-7" => "SWE 1.7".to_string(),
                    "swe-2" => "SWE 2".to_string(),
                    other => other.to_string(),
                }
            }
        } else {
            // Same composerModelDisplayName stripping for the draft chip.
            let lbl = self.composer_model.2.clone();
            if self.composer_model.0 == "codex" {
                lbl.replacen("GPT-", "", 1)
            } else if self.composer_model.0 == "claudeAgent" {
                lbl.replacen("Claude ", "", 1)
            } else {
                lbl
            }
        };
        let model_weak = cx.weak_entity();
        let model_cur = self.composer_model.clone();
        // Chip icon follows the thread's own provider inside threads.
        let chip_provider = if in_thread {
            let p = str_at(get(thread_v, "modelSelection"), "provider");
            if p.is_empty() {
                self.composer_model.0.clone()
            } else {
                p.to_string()
            }
        } else {
            self.composer_model.0.clone()
        };
        let model_icon = if chip_provider == "devin" {
            icons::P_DEVIN
        } else {
            icons::CODEX
        };
        // Upstream deriveEffectiveComposerModelState: inside a thread the chip
        // reflects that thread's persisted modelSelection — including the
        // effort in its options — not the global picker slider. The global
        // slider only seeds new drafts.
        // Upstream: a thread chip shows the effort only when the thread's
        // modelSelection pins one in its options; drafts always reflect the
        // picker slider.
        let chip_effort: Option<usize> = if in_thread {
            let eff = str_at(get(get(thread_v, "modelSelection"), "options"), "effort");
            match eff {
                "minimal" => Some(0),
                "low" => Some(1),
                "medium" => Some(2),
                "high" => Some(3),
                "max" => Some(4),
                _ => None,
            }
        } else {
            Some(self.composer_effort_idx.min(4))
        };
        let theme_chip = theme.clone();
        let build_model_chip =
            move |cx: &mut Context<Self>| {
                let _cur = model_cur.clone();
                let _weak = model_weak.clone();
                {
                    let eff_muted = theme_chip.muted_foreground;
                    let fg = theme_chip.foreground;
                    div()
                        .id("composer-model")
                        .h(px(28.))
                        .px(px(8.))
                        .gap(px(6.))
                        .flex()
                        .items_center()
                        .rounded(px(6.))
                        .cursor_pointer()
                        .hover(|s| s.bg(theme_chip.muted.opacity(0.6)))
                        .child(svg_icon(model_icon).size(px(14.)))
                        .child(
                            div()
                                .text_size(px(13.))
                                .text_color(fg)
                                .child(model_label.clone()),
                        )
                        .children(chip_effort.map(|idx| {
                            div().text_size(px(13.)).text_color(eff_muted).child({
                                const EFFORTS: [&str; 5] =
                                    ["Minimal", "Low", "Medium", "High", "Max"];
                                EFFORTS[idx].to_string()
                            })
                        }))
                        .child(
                            svg_icon(icons::CHEVRON_DOWN)
                                .size(px(12.))
                                .text_color(eff_muted),
                        )
                        .on_click(cx.listener(move |this, _ev, window, cx| {
                            this.model_picker_open = !this.model_picker_open;
                            if this.model_picker_open {
                                // Open on the tab of the current model's provider.
                                this.model_picker_provider = this.composer_model.0.clone();
                            }
                            if this.model_picker_open && this.model_catalog.is_empty() {
                                // Lazy-fill the catalog from the backend once per
                                // provider; rows render when they land.
                                if let Some(client) = ensure_client(&this.client) {
                                    let mut out: Vec<(String, String, String)> = Vec::new();
                                    for prov in ["codex", "devin", "claudeAgent"] {
                                        if let Some(v) = client.call(
                                "provider.listModels",
                                json!({"provider": prov, "instanceId": prov, "cwd": null}),
                            ) {
                                let rows = v
                                    .get("models")
                                    .or_else(|| v.get("values"))
                                    .and_then(Value::as_array)
                                    .cloned()
                                    .unwrap_or_default();
                                for m in rows {
                                    let slug = str_at(&m, "slug");
                                    let name = {
                                        let n = str_at(&m, "name");
                                        if n.is_empty() { slug.to_string() } else { n.to_string() }
                                    };
                                    if !slug.is_empty() {
                                        out.push((slug.to_string(), name, prov.to_string()));
                                    }
                                }
                            }
                                    }
                                    if !out.is_empty() {
                                        this.model_catalog = out;
                                    }
                                }
                            }
                            cx.notify();
                            let _ = window;
                        }))
                }
            };
        // While the selected thread's turn is live, upstream swaps the send
        // button for a prominent dark stop square (thread.turn.interrupt).
        let turn_live = in_thread
            && matches!(
                session_phase(self.detail.as_ref()),
                "running" | "connecting"
            );
        // Expired side chats disable the send control (upstream
        // submission.expired → muted circle like an empty draft).
        let composer_expired = in_thread && !get(thread_v, "sidechatExpiredAt").is_null();
        // Composer mic (upstream voice notes): idle starts the capture, the
        // red recording state stops and transcribes into the draft. Muted and
        // inert while a clip is with the transcription backend. Built through
        // a closure because AnyElement is not Clone and the two composer
        // layouts are mutually exclusive per render.
        // Upstream deriveComposerVoiceState: the mic renders only when the
        // active provider is enabled and authenticated (voice input itself is
        // always available in the native shell). Signed-out providers hide it.
        let active_provider = if in_thread {
            str_at(thread_v, "providerName").to_string()
        } else {
            self.composer_model.0.clone()
        };
        let voice_visible = self
            .provider_statuses
            .iter()
            .find(|p| {
                let pid = str_at(p, "provider");
                let pid = if pid.is_empty() {
                    str_at(p, "driver")
                } else {
                    pid
                };
                pid == active_provider
            })
            .map(|p| {
                get(p, "enabled").as_bool().unwrap_or(true)
                    && get(p, "available").as_bool().unwrap_or(false)
                    && str_at(p, "authStatus") != "unauthenticated"
            })
            // Upstream: available is `=== true` strict — a provider missing
            // from the status list (CLI absent) renders no mic.
            .unwrap_or(false);
        let make_mic_btn = |cx: &mut Context<Self>| {
            let recording = self.voice_capture.is_some();
            let busy = self.voice_transcribing;
            let icon_color: gpui::Hsla = if recording {
                rgb(0xef4444).into()
            } else if busy {
                theme.muted_foreground.opacity(0.4)
            } else {
                theme.muted_foreground
            };
            let base = div()
                .id("mic")
                .size(px(28.))
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .rounded_full()
                .cursor_pointer()
                .on_click(cx.listener(|this, _ev, w, cx| {
                    if this.voice_capture.is_some() {
                        this.stop_voice(w, cx);
                    } else {
                        this.start_voice(cx);
                    }
                }));
            let icon = svg_icon(icons::MIC).size(px(15.)).text_color(icon_color);
            if recording {
                base.child(icon.with_animation(
                    "voice-rec-pulse",
                    Animation::new(std::time::Duration::from_millis(1200)).repeat_synced(),
                    |this, delta| {
                        let tri = 1.0 - (delta * 2.0 - 1.0).abs();
                        this.opacity(0.45 + 0.55 * tri)
                    },
                ))
                .into_any_element()
            } else {
                base.child(icon).into_any_element()
            }
        };
        let send_btn = if turn_live {
            div()
                .id("send")
                .size(px(28.))
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .rounded_full()
                .bg(theme.foreground)
                .cursor_pointer()
                .child(div().size(px(10.)).rounded(px(2.)).bg(theme.background))
                .on_click(cx.listener(|this, _ev, _w, cx| this.stop(cx)))
                .into_any_element()
        } else {
            // Empty draft renders the muted (disabled) circle like
            // the web composer; filled primary once there's text.
            div()
                .id("send")
                .size(px(28.))
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .rounded_full()
                .bg(if draft_empty || composer_expired {
                    theme.foreground.opacity(0.2)
                } else {
                    theme.foreground
                })
                .cursor_pointer()
                .child(
                    svg_icon(icons::SEND)
                        .size(px(20.))
                        .text_color(theme.background),
                )
                .on_click(cx.listener(|this, _ev, w, cx| this.send(w, cx)))
                .into_any_element()
        };
        // Upstream: started threads use the single-row composer while the draft
        // is one plain line — the card holds [+ | input | send] and the pickers
        // move to a row under the card. Multi-line drafts and the home composer
        // keep the block layout with controls inside.
        let row_composer = in_thread && !draft_text.contains('\n');
        let composer_border = if self.composer_thread_over {
            theme.accent
        } else {
            theme.border
        };
        // Light: white 55% over the #F9F9F9 tray reads 252 — matches the
        // Electron measurement exactly. Dark: the whole composer interior
        // measures a uniform rgb 23 in the real Electron capture (input zone
        // included) — the visible structure comes from the border and shadow,
        // not from a second fill. Before this fix the shell used the flat
        // sidebar black (17), so the composer read as a black hole against
        // the window background.
        let composer_card_bg = if theme.is_dark() {
            gpui::hsla(0.0, 0.0, 0.0902, 1.0)
        } else {
            // Opaque 252: the measured Electron input surface. Upstream
            // reaches it through white/0.55 over the opaque tray; gpui's
            // compositing chain does not reproduce that exactly, so the
            // resolved tone is hardcoded — the opaque tray beneath makes the
            // shortcut invisible.
            gpui::hsla(0.0, 0.0, 0.988, 1.0)
        };
        // Upstream in-thread composer: the input row is a fully-rounded white
        // pill inside the grey shell; permission/model chips sit on the grey
        // tray below, outside the pill.
        let composer_card: Div = if row_composer {
            // Upstream COMPOSER_SINGLE_ROW_SURFACE_CLASS_NAME: one grid row,
            // p-1.5 (6px) all around, gap-x-1 (4px) between the cells.
            div()
                .w_full()
                .border_1()
                .border_color(composer_border)
                .rounded(px(9999.))
                .bg(composer_card_bg)
                .shadow_md()
                .p(px(6.))
                .child(
                    h_flex()
                        .gap_1()
                        .items_center()
                        .child(plus_btn)
                        .child(div().flex_1().min_w_0().child(input_zone))
                        .children(if voice_visible {
                            Some(make_mic_btn(cx)).into_iter()
                        } else {
                            None.into_iter()
                        })
                        .child(send_btn),
                )
        } else {
            v_flex()
                .w_full()
                .max_w(px(755.))
                .border_1()
                .border_color(composer_border)
                .rounded(px(19.2))
                .bg(composer_card_bg)
                .shadow_md()
                .px_2()
                .pt(px(4.))
                .pb(px(8.))
                .gap(px(8.))
                .child(input_zone)
                .child(
                    h_flex()
                        .gap_3()
                        .items_center()
                        .child(plus_btn)
                        .child(build_mode_chip(cx))
                        .child(div().flex_1())
                        .children(if in_thread {
                            context_window_usage(thread_v)
                                .map(|(pct, used, max_t)| {
                                    let _label = if let Some(m) = max_t {
                                        format!(
                                            "{:.0}% — {} of {} tokens used",
                                            pct, used as u64, m as u64
                                        )
                                    } else {
                                        format!("{:.0}% of context used", pct)
                                    };
                                    div()
                                        .flex_none()
                                        .size(px(16.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .text_color(theme.muted_foreground)
                                        .child(
                                            gpui::svg()
                                                .data(meter_ring_data(pct).as_bytes())
                                                .size(px(16.))
                                                .text_color(theme.muted_foreground),
                                        )
                                        .into_any_element()
                                })
                                .into_iter()
                        } else {
                            None.into_iter()
                        })
                        .child(build_model_chip(cx))
                        .children(if voice_visible {
                            Some(make_mic_btn(cx)).into_iter()
                        } else {
                            None.into_iter()
                        })
                        .child(send_btn),
                )
        };
        let chat_mw = self.chat_max_w();
        let mut landing_controls: Option<Div> = None;
        let mut composer_col = v_flex()
            .w_full()
            .max_w(chat_mw)
            .gap_0()
            // Thread rows drop here as an @mention (upstream
            // useComposerThreadMentionDrop); the current thread can't
            // mention itself.
            .can_drop(|v, _w, _cx| v.is::<ThreadDrag>() || v.is::<FileDrag>())
            .on_drag_move::<ThreadDrag>(cx.listener(
                |this, _ev: &gpui::DragMoveEvent<ThreadDrag>, _w, cx| {
                    if !this.composer_thread_over {
                        this.composer_thread_over = true;
                        cx.notify();
                    }
                },
            ))
            .on_drag_move::<FileDrag>(cx.listener(
                |this, _ev: &gpui::DragMoveEvent<FileDrag>, _w, cx| {
                    if !this.composer_thread_over {
                        this.composer_thread_over = true;
                        cx.notify();
                    }
                },
            ))
            .on_drop::<FileDrag>(cx.listener(|this, drag: &FileDrag, w, cx| {
                this.composer_thread_over = false;
                let name = drag
                    .0
                    .rsplit('/')
                    .next()
                    .unwrap_or(drag.0.as_str())
                    .to_string();
                let draft = this.composer.read(cx).value().to_string();
                let sep = if draft.is_empty() || draft.ends_with(' ') {
                    ""
                } else {
                    " "
                };
                this.set_draft(&format!("{draft}{sep}@{name} "), w, cx);
                cx.notify();
            }))
            .on_drop::<ThreadDrag>(cx.listener(|this, drag: &ThreadDrag, w, cx| {
                this.composer_thread_over = false;
                if this.selected_thread.as_deref() == Some(drag.0.as_str()) {
                    cx.notify();
                    return;
                }
                let title = arr(this.snapshot.as_ref().unwrap_or(&Value::Null), "threads")
                    .iter()
                    .find(|t| str_at(t, "id") == drag.0)
                    .map(|t| str_at(t, "title").to_string())
                    .filter(|t| !t.is_empty())
                    .unwrap_or_else(|| drag.0.chars().take(8).collect());
                let draft = this.composer.read(cx).value().to_string();
                let sep = if draft.is_empty() || draft.ends_with(' ') {
                    ""
                } else {
                    " "
                };
                this.set_draft(&format!("{draft}{sep}@{title} "), w, cx);
                cx.notify();
            }));
        // Upstream floats context chips above the card only for threads bound
        // to a worktree (envMode != "local"): project chip left, env chip right.
        let wt_path = str_at(thread_v, "worktreePath").to_string();
        let env_mode = str_at(thread_v, "envMode").to_string();
        let has_env_chips =
            in_thread && (!wt_path.is_empty() || (env_mode != "local" && !env_mode.is_empty()));
        if has_env_chips {
            let thread_pid = str_at(thread_v, "projectId").to_string();
            let proj_name = arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects")
                .iter()
                .find(|p| str_at(p, "id") == thread_pid)
                .map(|p| str_at(p, "title").to_string())
                .filter(|n: &String| !n.is_empty())
                .unwrap_or_else(|| thread_pid.clone());
            let chip = |text: String| {
                div()
                    .px(px(8.))
                    .py(px(2.))
                    .rounded(px(6.))
                    .bg(popover_bg)
                    .border_1()
                    .border_color(theme.border)
                    .text_size(px(11.))
                    .text_color(theme.muted_foreground)
                    .child(text)
            };
            composer_col = composer_col.child(
                h_flex()
                    .w_full()
                    .items_center()
                    .pb(px(4.))
                    .child(chip(format!("\u{29c9} {proj_name}")))
                    .child(div().flex_1())
                    .child(chip(format!(
                        "\u{25cc} {}",
                        if wt_path.is_empty() {
                            "Temporary"
                        } else {
                            "Worktree"
                        }
                    ))),
            );
        }
        if draft_text.starts_with('/') && !draft_text.contains(' ') {
            // Upstream ComposerCommandMenu: popup card above the composer
            // listing built-in slash commands filtered by the "/query".
            const SLASH_COMMANDS: [(&str, &str, &str); 18] = [
                (
                    "clear",
                    "Clear",
                    "Start a fresh thread and clear the current conversation context",
                ),
                (
                    "compact",
                    "Compact Context",
                    "Compact the current thread context to free space",
                ),
                ("model", "Model", "Switch response model for this thread"),
                ("plan", "Plan Mode", "Switch this thread into plan mode"),
                (
                    "debug",
                    "Debug Mode",
                    "Switch this thread into evidence-first debug mode",
                ),
                (
                    "default",
                    "Default Mode",
                    "Switch this thread back to normal chat mode",
                ),
                (
                    "review",
                    "Code Review",
                    "Start a code review for current changes",
                ),
                (
                    "fork",
                    "Fork",
                    "Fork this thread into local or a new worktree",
                ),
                (
                    "side",
                    "Sidechat",
                    "Open a guarded Side from this thread, optionally on another provider",
                ),
                (
                    "status",
                    "Status",
                    "Show context usage and rate-limit status",
                ),
                (
                    "subagents",
                    "Subagents",
                    "Insert a prompt that asks the assistant to delegate work",
                ),
                (
                    "computer-use",
                    "Computer Use",
                    "Use Synara Computer for this request only",
                ),
                (
                    "fast",
                    "Fast Mode",
                    "Turn fast mode on or off for this thread",
                ),
                (
                    "export",
                    "Export",
                    "Download this thread as a ZIP archive (thread.json + transcript.md)",
                ),
                (
                    "goal",
                    "Goal",
                    "Set, edit, pause, resume, or clear this thread's persistent goal",
                ),
                (
                    "rename",
                    "Rename",
                    "Regenerate this thread title, or set an exact title",
                ),
                (
                    "feedback",
                    "Feedback on Synara",
                    "Send feedback to the Synara team",
                ),
                (
                    "automation",
                    "Automation",
                    "Create an automation from this thread",
                ),
            ];
            let query = draft_text[1..].to_lowercase();
            let mut menu_card = v_flex()
                .w_full()
                .max_h(px(300.))
                .overflow_y_scrollbar()
                .border_1()
                .border_color(theme.border)
                .rounded(px(12.))
                .bg(popover_bg)
                .shadow_md()
                .py_1()
                .mb(px(6.));
            let mut shown = 0;
            for (cmd, title, desc) in SLASH_COMMANDS {
                if !query.is_empty() && !cmd.contains(query.as_str()) {
                    continue;
                }
                shown += 1;
                menu_card = menu_card.child(
                    h_flex()
                        .id(ElementId::Name(format!("slash-{cmd}").into()))
                        .w_full()
                        .px_3()
                        .py(px(6.))
                        .items_center()
                        .cursor_pointer()
                        .rounded_md()
                        .hover(|d| d.bg(theme.secondary))
                        .on_click(cx.listener(move |this, _ev, w, cx| {
                            // Commands the shell executes natively; the rest
                            // (provider/session-bound) insert the literal
                            // "/cmd " into the draft, like upstream does for
                            // provider-native commands.
                            match cmd {
                                "clear" => this.new_thread(cx),
                                "plan" | "debug" | "default" => {
                                    this.interaction_mode = cmd;
                                    this.set_draft("", w, cx);
                                    if let Some(tid) = this.selected_thread.clone() {
                                        this.dispatch_cmd(
                                            json!({
                                                "type": "thread.interaction-mode.set",
                                                "commandId": uuid(),
                                                "threadId": tid,
                                                "interactionMode": cmd,
                                                "createdAt": now_iso(),
                                            }),
                                            cx,
                                        );
                                    }
                                }
                                "fast" => {
                                    this.fast_mode = !this.fast_mode;
                                    this.set_draft("", w, cx);
                                }
                                "goal" => this.insert_goal(w, cx),
                                "rename" => this.start_rename(w, cx),
                                "feedback" => {
                                    this.set_draft("", w, cx);
                                    this.open_feedback(w, cx);
                                }
                                _ => this.set_draft(&format!("/{cmd} "), w, cx),
                            }
                            cx.notify();
                        }))
                        .child(
                            v_flex()
                                .min_w_0()
                                .flex_1()
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(title),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(theme.muted_foreground)
                                        .child(desc),
                                ),
                        )
                        .child(
                            div()
                                .ml_2()
                                .flex_none()
                                .text_size(px(12.))
                                .text_color(theme.muted_foreground)
                                .child(format!("/{cmd}")),
                        ),
                );
            }
            if shown == 0 {
                menu_card = menu_card.child(
                    div()
                        .px_3()
                        .py_2()
                        .text_size(px(13.))
                        .text_color(theme.muted_foreground)
                        .child("No matching command."),
                );
            }
            composer_col = composer_col.child(menu_card);
        } else if let Some(query) = mention_query(&draft_text) {
            // Upstream local-folder mention: "@" browses the filesystem and
            // inserts a @path token. Path-like queries list their dirname
            // and filter on the basename; plain queries list $HOME and
            // filter by substring. Directories descend (append "/").
            let q_lower = query.rsplit('/').next().unwrap_or("").to_lowercase();
            let mut menu_card = v_flex()
                .w_full()
                .max_h(px(300.))
                .overflow_y_scrollbar()
                .border_1()
                .border_color(theme.border)
                .rounded(px(12.))
                .bg(popover_bg)
                .shadow_md()
                .py_1()
                .mb(px(6.));
            let mut shown = 0;
            for entry in self.mention_entries.iter() {
                let name = str_at(entry, "name").to_string();
                if name.starts_with('.')
                    || (!q_lower.is_empty() && !name.to_lowercase().contains(&q_lower))
                {
                    continue;
                }
                shown += 1;
                let is_dir = str_at(entry, "kind") == "directory";
                let path = str_at(entry, "path").to_string();
                let entry = entry.clone();
                menu_card = menu_card.child(
                    h_flex()
                        .id(ElementId::Name(format!("mention-{path}").into()))
                        .w_full()
                        .px_3()
                        .py(px(6.))
                        .items_center()
                        .gap_1p5()
                        .cursor_pointer()
                        .rounded_md()
                        .hover(|d| d.bg(theme.secondary))
                        .on_click(cx.listener(move |this, _ev, w, cx| {
                            let draft = this.composer.read(cx).value().to_string();
                            let at = draft.rfind('@').unwrap_or(draft.len());
                            let prefix = draft[..at].to_string();
                            if str_at(&entry, "kind") == "directory" {
                                let p = str_at(&entry, "path").to_string();
                                this.set_draft(&format!("{prefix}@{p}/"), w, cx);
                                this.load_mention_dir(p, cx);
                            } else {
                                let p = str_at(&entry, "path").to_string();
                                this.set_draft(&format!("{prefix}@{p} "), w, cx);
                            }
                            cx.notify();
                        }))
                        .child(
                            svg_icon(if is_dir {
                                icons::FOLDER
                            } else {
                                icons::FILE_TEXT
                            })
                            .size(px(14.))
                            .text_color(theme.muted_foreground),
                        )
                        .child(
                            div()
                                .text_size(px(13.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(theme.foreground)
                                .child(name),
                        ),
                );
                if shown >= 50 {
                    break;
                }
            }
            if shown == 0 {
                menu_card = menu_card.child(
                    div()
                        .px_3()
                        .py_2()
                        .text_size(px(13.))
                        .text_color(theme.muted_foreground)
                        .child("No matching files."),
                );
            }
            composer_col = composer_col.child(menu_card);
        }
        if !self.queued_turns.is_empty() {
            // Upstream ComposerQueuedHeader: stacked panel above the composer,
            // one row per queued follow-up (steer icon + preview + actions).
            let mut panel = v_flex()
                .w_full()
                .border_1()
                .border_color(theme.border)
                .rounded(px(12.))
                .bg(popover_bg)
                .mb(px(6.))
                .py_1();
            if self.queue_paused {
                panel = panel.child(
                    h_flex()
                        .w_full()
                        .px_3()
                        .py(px(6.))
                        .items_center()
                        .child(
                            div()
                                .flex_1()
                                .text_size(px(12.))
                                .text_color(theme.muted_foreground)
                                .child("Queue paused — queued follow-ups are held."),
                        )
                        .child(
                            div()
                                .id("queue-resume")
                                .cursor_pointer()
                                .px_2()
                                .py(px(2.))
                                .rounded_md()
                                .text_size(px(12.))
                                .hover(|d| d.bg(theme.secondary))
                                .child("Resume")
                                .on_click(cx.listener(|this, _ev, _w, cx| {
                                    this.queue_paused = false;
                                    cx.notify();
                                })),
                        ),
                );
            }
            for (i, qtext) in self.queued_turns.iter().enumerate() {
                let preview: String = qtext.chars().take(120).collect();
                panel = panel.child(
                    h_flex()
                        .id(ElementId::Name(format!("queued-{i}").into()))
                        .w_full()
                        .px_3()
                        .py(px(6.))
                        .items_center()
                        .gap_2()
                        .cursor_pointer()
                        .hover(|d| d.bg(theme.secondary))
                        .on_click(cx.listener(move |this, _ev, w, cx| {
                            // Upstream Edit: move the queued text back into the
                            // composer draft.
                            if i < this.queued_turns.len() {
                                let text = this.queued_turns.remove(i);
                                this.set_draft(&text, w, cx);
                            }
                        }))
                        .child(
                            svg_icon(icons::SEND)
                                .size(px(14.))
                                .text_color(theme.muted_foreground),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_size(px(13.))
                                .overflow_hidden()
                                .child(preview),
                        )
                        .child(
                            div()
                                .id(ElementId::Name(format!("queued-rm-{i}").into()))
                                .flex_none()
                                .cursor_pointer()
                                .px_1()
                                .text_size(px(13.))
                                .text_color(theme.muted_foreground)
                                .child("\u{00d7}")
                                .on_click(cx.listener(move |this, _ev, _w, cx| {
                                    cx.stop_propagation();
                                    if i < this.queued_turns.len() {
                                        this.queued_turns.remove(i);
                                    }
                                    cx.notify();
                                })),
                        ),
                );
            }
            composer_col = composer_col.child(panel);
        }
        // Upstream empty-landing controls tray: on the home landing a row of
        // unfilled chips floats directly above the composer, led by the
        // ProjectPicker trigger ("Work in a project" / the chosen folder).
        if !in_thread {
            if self.project_picker_open {
                if self.project_picker_input.is_none() {
                    let input =
                        cx.new(|cx| InputState::new(window, cx).placeholder("Search projects"));
                    self.project_picker_input = Some(input);
                }
                let query = self
                    .project_picker_input
                    .as_ref()
                    .map(|i| i.read(cx).value().to_string().to_lowercase())
                    .unwrap_or_default();
                let mut card = v_flex()
                    .w(px(280.))
                    .max_h(px(280.))
                    .overflow_y_scrollbar()
                    .border_1()
                    .border_color(theme.border)
                    .rounded(px(12.))
                    .bg(popover_bg)
                    .shadow_md()
                    .py_1()
                    .mb(px(4.));
                if let Some(input) = self.project_picker_input.clone() {
                    card = card.child(
                        div()
                            .px_2()
                            .py_1()
                            .child(Input::new(&input).appearance(false)),
                    );
                }
                if self.landing_project_id.is_some() {
                    card = card.child(
                        div()
                            .id("project-picker-reset")
                            .mx_1()
                            .px_2()
                            .py(px(6.))
                            .rounded(px(6.))
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.muted))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _e, _w, cx| {
                                    this.landing_project_id = None;
                                    this.project_picker_open = false;
                                    cx.notify();
                                }),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(theme.foreground)
                                    .child("Don't work in a project"),
                            ),
                    );
                }
                for p in arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects") {
                    // Upstream activeFolderOptions: ordinary projects only.
                    if str_at(p, "kind") != "project" {
                        continue;
                    }
                    let pid = str_at(p, "id").to_string();
                    if pid.is_empty() {
                        continue;
                    }
                    let cwd = str_at(p, "cwd").to_string();
                    let folder = cwd
                        .rsplit('/')
                        .find(|s| !s.is_empty())
                        .map(str::to_owned)
                        .or_else(|| {
                            let t = str_at(p, "folderName").to_string();
                            if t.is_empty() {
                                None
                            } else {
                                Some(t)
                            }
                        })
                        .unwrap_or_else(|| str_at(p, "title").to_string());
                    let local = str_at(p, "localName").to_string();
                    let primary = if local.is_empty() {
                        folder.clone()
                    } else {
                        local
                    };
                    if !query.is_empty()
                        && !primary.to_lowercase().contains(query.as_str())
                        && !folder.to_lowercase().contains(query.as_str())
                    {
                        continue;
                    }
                    let selected = self.landing_project_id.as_deref() == Some(pid.as_str());
                    card = card.child(
                        div()
                            .id(SharedString::from(format!("pp-{pid}")))
                            .mx_1()
                            .px_2()
                            .py(px(6.))
                            .rounded(px(6.))
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.muted))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _e, _w, cx| {
                                    this.landing_project_id = Some(pid.clone());
                                    this.project_picker_open = false;
                                    cx.notify();
                                }),
                            )
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(theme.muted_foreground)
                                            .child("\u{1f4c1}"),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .text_size(px(12.))
                                            .text_color(theme.foreground)
                                            .child(primary),
                                    )
                                    .when(selected, |d| {
                                        d.child(
                                            div()
                                                .text_size(px(12.))
                                                .text_color(theme.accent)
                                                .child("\u{2713}"),
                                        )
                                    }),
                            ),
                    );
                }
                composer_col = composer_col.child(card);
            }
            // In an empty project thread the composer is bound to the
            // thread's project (upstream): the chip shows that folder even
            // though the home picker state is empty.
            let thread_project_id: Option<String> = if self.selected_thread.is_some()
                && arr(
                    get(self.detail.as_ref().unwrap_or(&Value::Null), "thread"),
                    "messages",
                )
                .is_empty()
            {
                let tid = self.selected_thread.as_deref().unwrap_or("");
                arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "threads")
                    .iter()
                    .find(|t| str_at(t, "id") == tid)
                    .map(|t| str_at(t, "projectId").to_string())
                    .filter(|p| !p.is_empty())
            } else {
                None
            };
            let effective_project_id = self.landing_project_id.clone().or(thread_project_id);
            let chip_label = effective_project_id
                .as_ref()
                .and_then(|pid| {
                    arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects")
                        .iter()
                        .find(|p| str_at(p, "id") == pid.as_str())
                        .map(|p| {
                            let cwd = str_at(p, "cwd").to_string();
                            let local = str_at(p, "localName").to_string();
                            if !local.is_empty() {
                                local
                            } else {
                                cwd.rsplit('/')
                                    .find(|s| !s.is_empty())
                                    .map(str::to_owned)
                                    .unwrap_or_else(|| str_at(p, "title").to_string())
                            }
                        })
                })
                .unwrap_or_else(|| "Work in a project".to_string());
            // Upstream empty-landing controls tray: the workspace folder
            // chip hugs the left edge and the "Temporary" toggle the right.
            let mut controls = h_flex().w_full().items_center().pb(px(4.)).child(
                div()
                    .id("landing-project-picker")
                    .px_2()
                    .py(px(4.))
                    .rounded(px(999.))
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.muted))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _e, _w, cx| {
                            this.project_picker_open = !this.project_picker_open;
                            cx.notify();
                        }),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .items_center()
                            .child(
                                svg_icon(icons::FOLDER)
                                    .size(px(12.))
                                    .text_color(theme.muted_foreground),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(theme.muted_foreground)
                                    .child(chip_label),
                            ),
                    ),
            );
            if effective_project_id.is_some() {
                let temp_active = self.landing_temporary;
                controls = controls.child(div().flex_1()).child(
                    div()
                        .id("landing-temporary")
                        .px_2()
                        .py(px(4.))
                        .rounded(px(999.))
                        .cursor_pointer()
                        .hover(|s| s.bg(theme.muted))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _e, _w, cx| {
                                this.landing_temporary = !this.landing_temporary;
                                cx.notify();
                            }),
                        )
                        .child(
                            h_flex()
                                .gap(px(6.))
                                .items_center()
                                .child(svg_icon(icons::TEMPORARY).size(px(12.)).text_color(
                                    if temp_active {
                                        theme.accent
                                    } else {
                                        theme.muted_foreground
                                    },
                                ))
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(if temp_active {
                                            theme.accent
                                        } else {
                                            theme.muted_foreground
                                        })
                                        .child("Temporary"),
                                ),
                        ),
                );
            }
            landing_controls = Some(controls);
        }
        // Upstream status banners sit at the top of the chat column: the
        // provider-health alert above everything and the thread error first
        // inside the transcript pane (ChatColumnBannerFrame +
        // ChatTranscriptPane), not above the composer.
        let mut thread_error_banner: Option<gpui::AnyElement> = None;
        let mut provider_banner: Option<gpui::AnyElement> = None;
        // Upstream ThreadErrorBanner: a red alert carries a quarantined/failed
        // thread's session error, with a details toggle, copy, an Unblock
        // action for provider-delivery quarantines, and a session-local
        // dismiss.
        if in_thread {
            let session_err = str_at(get(thread_v, "session"), "lastError").to_string();
            let tid_s = self.selected_thread.clone().unwrap_or_default();
            if !session_err.is_empty() && !self.dismissed_thread_errors.contains(&tid_s) {
                if self.error_details_for.as_deref() != Some(session_err.as_str()) {
                    self.error_details_for = Some(session_err.clone());
                    self.error_details_open = false;
                }
                let expanded = self.error_details_open;
                let can_unblock =
                    session_err.starts_with("Thread is blocked by an earlier provider failure");
                let clamped = session_err.len() > 160 || session_err.contains('\n');
                let shown: String = if expanded || !clamped {
                    session_err.clone()
                } else {
                    let mut cut: String = session_err.chars().take(160).collect();
                    cut.push_str("\u{2026}");
                    cut
                };
                let err_for_copy = session_err.clone();
                let unblocking = self.unblocking_thread;
                thread_error_banner = Some(
                    h_flex().w_full().justify_center().child(
                        div()
                            .w_full()
                            .max_w(px(576.))
                            .mb(px(6.))
                            .border_1()
                            .border_color(theme.danger.opacity(0.5))
                            .rounded(px(10.))
                            .bg(theme.danger.opacity(0.08))
                            .px_3()
                            .py_2()
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_start()
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .text_color(theme.danger)
                                            .child("\u{26a0}"),
                                    )
                                    .child(
                                        v_flex().flex_1().min_w_0().gap_1().child(
                                            div()
                                                .text_size(px(12.))
                                                .text_color(theme.danger)
                                                .child(shown),
                                        )
                                        .child(
                                            h_flex()
                                                .gap_2()
                                                .items_center()
                                                .when(clamped, |d| {
                                                    d.child(
                                                        div()
                                                            .id("err-details")
                                                            .text_size(px(11.))
                                                            .text_color(theme.muted_foreground)
                                                            .cursor_pointer()
                                                            .hover(|s| s.text_color(theme.foreground))
                                                            .on_mouse_down(
                                                                MouseButton::Left,
                                                                cx.listener(|this, _e, _w, cx| {
                                                                    this.error_details_open =
                                                                        !this.error_details_open;
                                                                    cx.notify();
                                                                }),
                                                            )
                                                            .child(if expanded {
                                                                "\u{25be} Hide details"
                                                            } else {
                                                                "\u{25b8} Show details"
                                                            }),
                                                    )
                                                })
                                                .child(
                                                    div()
                                                        .id("err-copy")
                                                        .text_size(px(11.))
                                                        .text_color(theme.muted_foreground)
                                                        .cursor_pointer()
                                                        .hover(|s| s.text_color(theme.foreground))
                                                        .on_mouse_down(
                                                            MouseButton::Left,
                                                            cx.listener(move |_this, _e, _w, cx| {
                                                                cx.write_to_clipboard(
                                                                    ClipboardItem::new_string(
                                                                        err_for_copy.clone(),
                                                                    ),
                                                                );
                                                            }),
                                                        )
                                                        .child("Copy error"),
                                                )
                                                .when(can_unblock, |d| {
                                                    d.child(
                                                        div()
                                                            .id("err-unblock")
                                                            .text_size(px(11.))
                                                            .text_color(theme.danger)
                                                            .cursor_pointer()
                                                            .when(!unblocking, |b| {
                                                                b.hover(|s| {
                                                                    s.text_color(theme.foreground)
                                                                })
                                                            })
                                                            .on_mouse_down(
                                                                MouseButton::Left,
                                                                cx.listener(|this, _e, _w, cx| {
                                                                    if this.unblocking_thread {
                                                                        return;
                                                                    }
                                                                    let tid = this
                                                                        .selected_thread
                                                                        .clone()
                                                                        .unwrap_or_default();
                                                                    let cell = this.client.clone();
                                                                    this.unblocking_thread = true;
                                                                    cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                                                                        let client = cx
                                                                            .background_executor()
                                                                            .spawn(async move {
                                                                                ensure_client(&cell)
                                                                            })
                                                                            .await;
                                                                        let Some(client) = client else {
                                                                            let _ = this.update(cx, |t: &mut Self, cx| {
                                                                                t.unblocking_thread = false;
                                                                                cx.notify();
                                                                            });
                                                                            return;
                                                                        };
                                                                        let blockers = cx
                                                                            .background_executor()
                                                                            .spawn({
                                                                                let c = client.clone();
                                                                                let tid = tid.clone();
                                                                                async move {
                                                                                    c.call(
                                                                                        "orchestration.listProviderDeliveryBlockers",
                                                                                        json!({"threadId": tid, "limit": 50}),
                                                                                    )
                                                                                }
                                                                            })
                                                                            .await;
                                                                        for b in blockers
                                                                            .and_then(|v| v.as_array().cloned())
                                                                            .unwrap_or_default()
                                                                        {
                                                                            let seq = b
                                                                                .get("eventSequence")
                                                                                .and_then(Value::as_i64)
                                                                                .unwrap_or(0);
                                                                            let st = str_at(&b, "state")
                                                                                .to_string();
                                                                            let c = client.clone();
                                                                            let tid = tid.clone();
                                                                            let _ = cx
                                                                                .background_executor()
                                                                                .spawn(async move {
                                                                                    c.call(
                                                                                        "orchestration.reconcileProviderDelivery",
                                                                                        json!({
                                                                                            "eventSequence": seq,
                                                                                            "threadId": tid,
                                                                                            "expectedState": st,
                                                                                            "outcome": "abandon",
                                                                                            "note": "Unblocked from the native shell",
                                                                                        }),
                                                                                    )
                                                                                })
                                                                                .await;
                                                                        }
                                                                        let p = json!({"threadId": tid});
                                                                        let detail = cx
                                                                            .background_executor()
                                                                            .spawn({
                                                                                let c = client.clone();
                                                                                async move {
                                                                                    c.call(
                                                                                        "orchestration.getThreadDetailSnapshot",
                                                                                        p,
                                                                                    )
                                                                                }
                                                                            })
                                                                            .await;
                                                                        let snap = cx
                                                                            .background_executor()
                                                                            .spawn(async move {
                                                                                client.call(
                                                                                    "orchestration.getShellSnapshot",
                                                                                    json!({}),
                                                                                )
                                                                            })
                                                                            .await;
                                                                        let _ = this.update(cx, |this: &mut Self, cx| {
                                                                            this.unblocking_thread = false;
                                                                            if detail.is_some() {
                                                                                this.detail = detail;
                                                                            }
                                                                            if let Some(s) = snap {
                                                                                this.snapshot = Some(s);
                                                                            }
                                                                            cx.notify();
                                                                        });
                                                                    })
                                                                    .detach();
                                                                }),
                                                            )
                                                            .child(if unblocking {
                                                                "Unblocking\u{2026}"
                                                            } else {
                                                                "Unblock thread"
                                                            }),
                                                    )
                                                }),
                                        ),
                                    )
                                    .child(
                                        div()
                                            .id("err-dismiss")
                                            .text_size(px(12.))
                                            .text_color(theme.muted_foreground)
                                            .cursor_pointer()
                                            .hover(|s| s.text_color(theme.foreground))
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(|this, _e, _w, cx| {
                                                    if let Some(t) = this.selected_thread.clone() {
                                                        this.dismissed_thread_errors.insert(t);
                                                    }
                                                    cx.notify();
                                                }),
                                            )
                                            .child("\u{2715}"),
                                    ),
                            ),
                    )
                    .into_any_element(),
                );
            }
        }
        // Upstream ProviderHealthBanner: the selected thread's provider
        // status surfaces as a warning/error alert above the chat column
        // until dismissed (ready providers render nothing). Dismissal keys
        // carry every field that would make the next banner a new one.
        if in_thread {
            let sess = get(thread_v, "session");
            let prov = str_at(sess, "providerName").to_string();
            let pinst = str_at(sess, "providerInstanceId").to_string();
            let status_v = self
                .provider_statuses
                .iter()
                .find(|st| {
                    let drv = {
                        let d = str_at(st, "driver").to_string();
                        if d.is_empty() {
                            str_at(st, "provider").to_string()
                        } else {
                            d
                        }
                    };
                    let matches_provider = drv.eq_ignore_ascii_case(&prov)
                        || str_at(st, "displayName").eq_ignore_ascii_case(&prov)
                        || str_at(st, "provider").eq_ignore_ascii_case(&prov);
                    let matches_instance = pinst.is_empty()
                        || str_at(st, "instanceId") == pinst
                        || str_at(st, "provider") == pinst;
                    matches_provider && matches_instance
                })
                .cloned();
            if let Some(st) = status_v {
                let s_state = str_at(&st, "status").to_string();
                if s_state != "ready" {
                    let label = {
                        let d = str_at(&st, "displayName").to_string();
                        if d.is_empty() {
                            str_at(&st, "provider").to_string()
                        } else {
                            d
                        }
                    };
                    let dkey = [
                        str_at(&st, "provider").to_string(),
                        {
                            let i = str_at(&st, "instanceId").to_string();
                            if i.is_empty() {
                                str_at(&st, "provider").to_string()
                            } else {
                                i
                            }
                        },
                        s_state.clone(),
                        if st
                            .get("available")
                            .and_then(Value::as_bool)
                            .unwrap_or(false)
                        {
                            "available".to_string()
                        } else {
                            "unavailable".to_string()
                        },
                        str_at(&st, "authStatus").to_string(),
                        str_at(&st, "message").to_string(),
                    ]
                    .join("\u{1f}");
                    if !self.dismissed_provider_banner_keys.contains(&dkey) {
                        let is_err = s_state == "error";
                        let accent = if is_err { theme.danger } else { theme.warning };
                        let msg = {
                            let m = str_at(&st, "message").to_string();
                            if m.is_empty() {
                                if is_err {
                                    format!("{label} provider is unavailable.")
                                } else {
                                    format!("{label} provider has limited availability.")
                                }
                            } else {
                                m
                            }
                        };
                        provider_banner = Some(
                            h_flex()
                                .w_full()
                                .justify_center()
                                .pt(px(12.))
                                .child(
                                    div()
                                        .w_full()
                                        .max_w(px(576.))
                                        .mb(px(6.))
                                        .relative()
                                        .border_1()
                                        .border_color(accent.opacity(0.5))
                                        .rounded(px(10.))
                                        .bg(accent.opacity(0.08))
                                        .px_3()
                                        .py_2()
                                        .child(
                                            h_flex()
                                                .gap_2()
                                                .items_start()
                                                .child(
                                                    div()
                                                        .text_size(px(13.))
                                                        .text_color(accent)
                                                        .child(if is_err {
                                                            "\u{2298}"
                                                        } else {
                                                            "\u{26a0}"
                                                        }),
                                                )
                                                .child(
                                                    v_flex()
                                                        .flex_1()
                                                        .min_w_0()
                                                        .child(
                                                            div()
                                                                .text_size(px(12.))
                                                                .text_color(accent)
                                                                .child(format!(
                                                                    "{label} provider status"
                                                                )),
                                                        )
                                                        .child(
                                                            div()
                                                                .text_size(px(11.))
                                                                .text_color(accent.opacity(0.72))
                                                                .child(msg),
                                                        ),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .id("provider-banner-dismiss")
                                                .absolute()
                                                .top(px(6.))
                                                .right(px(8.))
                                                .size(px(20.))
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .rounded_full()
                                                .text_size(px(11.))
                                                .text_color(accent.opacity(0.65))
                                                .cursor_pointer()
                                                .hover(|s| s.text_color(accent))
                                                .on_mouse_down(
                                                    MouseButton::Left,
                                                    cx.listener(move |this, _e, _w, cx| {
                                                        this.dismissed_provider_banner_keys
                                                            .insert(dkey.clone());
                                                        cx.notify();
                                                    }),
                                                )
                                                .child("\u{2715}"),
                                        ),
                                )
                                .into_any_element(),
                        );
                    }
                }
            }
        }
        // Upstream ExpiredSidechatNotice: an expired side chat keeps its
        // transcript but the composer is gated behind a start-new bar that
        // creates the replacement side chat under the same context.
        if in_thread && !get(thread_v, "sidechatExpiredAt").is_null() {
            composer_col = composer_col.child(
                h_flex().w_full().justify_center().child(
                    h_flex()
                        .w_full()
                        .max_w(px(576.))
                        .mb(px(8.))
                        .items_center()
                        .justify_between()
                        .gap_3()
                        .border_1()
                        .border_color(theme.border)
                        .rounded(px(12.))
                        .bg(theme.popover)
                        .px_3()
                        .py_2()
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_size(px(12.))
                                .text_color(theme.muted_foreground)
                                .child("This side chat expired after a period of inactivity. Start a new side chat."),
                        )
                        .child(
                            div()
                                .id("sidechat-start-new")
                                .flex_none()
                                .text_size(px(11.))
                                .text_color(theme.foreground)
                                .border_1()
                                .border_color(theme.border)
                                .rounded(px(6.))
                                .px_2()
                                .py(px(3.))
                                .cursor_pointer()
                                .hover(|s| s.bg(theme.muted))
                                .when(self.starting_sidechat, |b| {
                                    b.text_color(theme.muted_foreground)
                                })
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|this, _e, _w, cx| {
                                        if this.starting_sidechat {
                                            return;
                                        }
                                        let tv = this
                                            .detail
                                            .as_ref()
                                            .map(|d| get(d, "thread"))
                                            .cloned()
                                            .unwrap_or(Value::Null);
                                        let cell = this.client.clone();
                                        this.starting_sidechat = true;
                                        cx.notify();
                                        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                                            let client = cx
                                                .background_executor()
                                                .spawn(async move { ensure_client(&cell) })
                                                .await;
                                            let Some(client) = client else {
                                                let _ = this.update(cx, |t: &mut Self, cx| {
                                                    t.starting_sidechat = false;
                                                    cx.notify();
                                                });
                                                return;
                                            };
                                            let new_id = uuid();
                                            let src = str_at(&tv, "sidechatSourceThreadId");
                                            let cmd = json!({
                                                "type": "thread.create",
                                                "commandId": uuid(),
                                                "threadId": new_id,
                                                "projectId": str_at(&tv, "projectId"),
                                                "title": str_at(&tv, "title"),
                                                "sidechatContext": get(&tv, "sidechatContext").clone(),
                                                "sidechatSourceThreadId": if src.is_empty() { Value::Null } else { json!(src) },
                                                "createdAt": now_iso(),
                                            });
                                            let c = client.clone();
                                            let _ = cx
                                                .background_executor()
                                                .spawn(async move {
                                                    c.call("orchestration.dispatchCommand", cmd)
                                                })
                                                .await;
                                            let snap = cx
                                                .background_executor()
                                                .spawn(async move {
                                                    client.call("orchestration.getShellSnapshot", json!({}))
                                                })
                                                .await;
                                            let _ = this.update(cx, |t: &mut Self, cx| {
                                                t.starting_sidechat = false;
                                                if let Some(s) = snap {
                                                    t.snapshot = Some(s);
                                                }
                                                t.open_thread(new_id, cx);
                                                cx.notify();
                                            });
                                        })
                                        .detach();
                                    }),
                                )
                                .child(if self.starting_sidechat {
                                    "Starting\u{2026}"
                                } else {
                                    "Start new"
                                }),
                        ),
                ),
            );
        }
        // Upstream ComposerActiveTaskListCard: a compact capsule
        // ("1/3 tasks") above the composer card that opens the task
        // list panel on click. The "Show debug task banner" flag injects
        // upstream's sample list; otherwise the list is derived from the
        // thread's `turn.tasks.updated` activities while a turn is live.
        let task_list: Option<Vec<(String, u8)>> = if self.feature_flags[2] && has_thread {
            Some(vec![
                (
                    "Inspect banner layout without overlapping transcript text".to_string(),
                    0,
                ),
                ("Confirm compact task banner width".to_string(), 1),
                ("Verify sidebar task controls".to_string(), 2),
            ])
        } else if has_thread
            && !is_latest_turn_settled(get(&thread_v, "latestTurn"), get(&thread_v, "session"))
        {
            derive_active_task_list(&thread_v)
        } else {
            None
        };
        if let Some(task_list) = task_list {
            let completed = task_list.iter().filter(|(_, s)| *s == 2).count();
            if self.task_card_open {
                // Popover card (upstream ActiveTaskListCard): header row with
                // progress + the per-task status list.
                let mut panel = v_flex()
                    .w_full()
                    .mb(px(6.))
                    .rounded(px(12.))
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.popover)
                    .shadow_md()
                    .py(px(6.));
                panel = panel.child(
                    h_flex()
                        .w_full()
                        .px_3()
                        .py(px(4.))
                        .items_center()
                        .gap_2()
                        .child(
                            svg_icon(icons::TODOS)
                                .size(px(12.))
                                .text_color(theme.muted_foreground),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(theme.muted_foreground)
                                .child(format!(
                                    "{completed} out of {total} tasks completed",
                                    total = task_list.len()
                                )),
                        ),
                );
                for (task, status) in task_list.iter() {
                    let (glyph, color) = match status {
                        0 => ("●", theme.accent),           // inProgress
                        1 => ("○", theme.muted_foreground), // pending
                        _ => ("✓", theme.muted_foreground), // completed
                    };
                    panel = panel.child(
                        h_flex()
                            .px_3()
                            .py(px(3.))
                            .gap_2()
                            .items_center()
                            .child(div().text_size(px(11.)).text_color(color).child(glyph))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(if *status == 2 {
                                        theme.muted_foreground
                                    } else {
                                        theme.foreground
                                    })
                                    .child(task.clone()),
                            ),
                    );
                }
                composer_col = composer_col.child(panel);
            }
            composer_col = composer_col.child(
                h_flex().w_full().mb(px(6.)).child(
                    h_flex()
                        .id("active-task-list-capsule")
                        .gap_2()
                        .items_center()
                        .rounded(px(12.))
                        .border_1()
                        .border_color(theme.border)
                        .bg(theme.popover)
                        .px_3()
                        .py(px(6.))
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _ev, _w, cx| {
                            this.task_card_open = !this.task_card_open;
                            cx.notify();
                        }))
                        .child(
                            svg_icon(icons::TODOS)
                                .size(px(12.))
                                .text_color(theme.muted_foreground),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(theme.foreground)
                                .child(format!("{completed}/{} tasks", task_list.len())),
                        ),
                ),
            );
        }
        // Upstream .chat-composer-shell: ONE opaque tray surface holding the
        // project/branch chips strip, a hairline, the input zone and the
        // controls row — not a floating chip above a separate card. The
        // surface keeps upstream's translucent white/0.55 over an opaque
        // near-white shell so the texture does not wash it out.
        // Upstream tray tone measured in the Electron capture: #F9F9F9
        // (foreground 2% over the white background) with a #F1F1F1 hairline.
        // Upstream tray tone: light is foreground 2% over background (measured
        // #F9F9F9); dark is foreground 4% over background (measured rgb 23 —
        // NOT the flat sidebar black, which is what made the composer read as
        // a black slab).
        let composer_shell_bg: gpui::Hsla = if theme.is_dark() {
            gpui::hsla(0.0, 0.0, 0.0902, 1.0)
        } else {
            gpui::hsla(0.0, 0.0, 0.976, 1.0)
        };
        let composer_shelled = if let Some(controls) = landing_controls {
            // Canonical upstream tray (#2306): a soft opaque GREY squircle —
            // foreground 2% over background in light — with 4px padding; the
            // project strip sits on the grey, and the composer inside keeps
            // its own white surface, border, shadow and radius.
            let tray_bg: gpui::Hsla = if theme.is_dark() {
                gpui::hsla(0.0, 0.0, 0.0902, 1.0)
            } else {
                gpui::hsla(0.0, 0.0, 0.975, 1.0)
            };
            div().flex().justify_center().w_full().child(
                v_flex()
                    .w_full()
                    .max_w(px(755.))
                    .rounded(px(24.2))
                    .border_1()
                    .border_color(theme.border.opacity(0.5))
                    .bg(tray_bg)
                    .p(px(4.))
                    .gap(px(4.))
                    .child(div().px(px(6.)).child(controls))
                    .child(composer_card),
            )
        } else {
            // In-thread tray (upstream chat-composer-shell): opaque grey
            // squircle spanning the chat column; the white pill sits inside,
            // and the permission/model chips row rides on the grey below it.
            let tray_inner = if row_composer {
                // Upstream .chat-composer-shell[data-composer-tray]: the tray
                // keeps squircle corners at --composer-tray-radius (1.2rem +
                // 0.25rem + 1px = 24.2px) in every layout — only the input
                // surface inside the tray becomes a pill in row mode. The
                // stadium shape here is what made the thread composer read
                // as a blob.
                v_flex()
                    .w_full()
                    .max_w(chat_mw)
                    .rounded(px(24.2))
                    .border_1()
                    .border_color(theme.border.opacity(0.5))
                    .bg(composer_shell_bg)
                    .p(px(4.))
                    .gap(px(4.))
                    .child(composer_card)
                    .child(
                        // Upstream COMPOSER_FOOTER_ROW_CLASS_NAME: pl 6px,
                        // pr 8px, pb 6px inside the tray's own 4px padding.
                        h_flex()
                            .w_full()
                            .pl(px(6.))
                            .pr(px(8.))
                            .pb(px(2.))
                            .items_center()
                            .child(build_mode_chip(cx))
                            .child(div().flex_1())
                            .child(build_model_chip(cx)),
                    )
            } else {
                v_flex()
                    .w_full()
                    .max_w(chat_mw)
                    .rounded(px(24.2))
                    .border_1()
                    .border_color(theme.border.opacity(0.5))
                    .bg(composer_shell_bg)
                    .p(px(4.))
                    .child(composer_card)
            };
            div().flex().justify_center().w_full().child(tray_inner)
        };
        composer_col = composer_col.child(composer_shelled);
        let composer_bar = div().w_full().pb(px(10.)).child(
            h_flex()
                .w_full()
                .justify_center()
                .pr(px(4.))
                .child(composer_col),
        );

        // ---- split pane (chat.split, cmd-\): right column with its own
        // header, transcript surface, thread picker and composer ----
        let mut split_pane = v_flex().flex_1().h_full().gap_2().p_4();
        let mut split_bar = v_flex().w_full();
        if self.split_open {
            let st_detail = self.split_detail.clone().unwrap_or(Value::Null);
            let pane_focused = self.focused_pane == 1;
            let mut header = h_flex()
                .w_full()
                .h(px(32.))
                .px_2()
                .items_center()
                .gap_2()
                .border_b_1()
                .border_color(if pane_focused {
                    theme.primary
                } else {
                    theme.border
                });
            if self.split_thread.is_some() {
                let title = str_at(get(&st_detail, "thread"), "title").to_string();
                let provider = str_at(get(&st_detail, "session"), "providerName").to_string();
                let focused = self.focused_pane == 1;
                header = header
                    .child(
                        svg_icon(provider_icon(&provider))
                            .size(px(13.))
                            .text_color(theme.muted_foreground),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(if focused {
                                theme.foreground
                            } else {
                                theme.muted_foreground
                            })
                            .child(title),
                    )
                    .child(
                        div()
                            .id("split-close")
                            .cursor_pointer()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child("\u{2715}")
                            .on_click(cx.listener(|this, _ev, _w, cx| {
                                this.split_thread = None;
                                this.split_detail = None;
                                cx.notify();
                            })),
                    );
            }
            split_pane = split_pane.child(header);
            if self.split_thread.is_some() {
                split_pane = split_pane.child(self.transcript_body(&st_detail, true, false, cx));
            } else {
                // Empty leaf: upstream SplitPaneEmptyState — centered label
                // plus a scrollable list of threads not already shown.
                split_pane = split_pane.child(
                    v_flex()
                        .w_full()
                        .pt(px(64.))
                        .items_center()
                        .gap_1()
                        .child(
                            div()
                                .text_size(px(14.))
                                .text_color(theme.foreground)
                                .child("Select a chat"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child("Pick a thread to open in this pane"),
                        ),
                );
                let mut list = v_flex().w_full().max_w(px(384.)).gap_y(px(1.));
                for t in arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "threads") {
                    let tid = str_at(t, "id").to_string();
                    if tid.is_empty() {
                        continue;
                    }
                    let in_use = self.open_threads.contains(&tid)
                        || self.split_thread.as_deref() == Some(tid.as_str());
                    let title = str_at(t, "title").to_string();
                    let provider = str_at(get(t, "session"), "providerName").to_string();
                    let mut row = h_flex()
                        .id(SharedString::from(format!("splitpick-{tid}")))
                        .w_full()
                        .px_2()
                        .py_1p5()
                        .rounded_md()
                        .gap_2()
                        .items_center()
                        .child(
                            svg_icon(provider_icon(&provider))
                                .size(px(13.))
                                .text_color(theme.muted_foreground),
                        )
                        .child(div().flex_1().text_xs().child(title));
                    if in_use {
                        row = row.opacity(0.35);
                    } else {
                        row = row
                            .cursor_pointer()
                            .hover(|s| s.bg(subtle))
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                this.open_split_thread(tid.clone(), cx);
                            }));
                    }
                    list = list.child(row);
                }
                split_pane = split_pane.child(
                    div()
                        .flex_1()
                        .overflow_y_scrollbar()
                        .child(h_flex().w_full().justify_center().child(list)),
                );
            }
            // Dropping a dragged sidebar thread assigns it to this pane
            // (upstream createFromDrop).
            split_pane = split_pane
                .can_drop(|v, _w, _cx| v.is::<ThreadDrag>() || v.is::<TabDrag>())
                .on_drop::<ThreadDrag>(cx.listener(|this, drag: &ThreadDrag, _w, cx| {
                    this.open_split_thread(drag.0.clone(), cx);
                }))
                .on_drop::<TabDrag>(cx.listener(|this, drag: &TabDrag, _w, cx| {
                    this.open_split_thread(drag.0.clone(), cx);
                }));
            if let Some(input) = self.split_composer.clone() {
                let card = div()
                    .w_full()
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .px_3()
                    .py_2()
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(Input::new(&input).appearance(false).w_full())
                            .child(
                                div()
                                    .id("split-send")
                                    .cursor_pointer()
                                    .child(
                                        svg_icon(icons::SEND)
                                            .size(px(14.))
                                            .text_color(theme.muted_foreground),
                                    )
                                    .on_click(cx.listener(|this, _ev, w, cx| {
                                        this.send_split(w, cx);
                                    })),
                            ),
                    );
                let chat_mw = self.chat_max_w();
                split_bar = split_bar.child(
                    div().w_full().pb(px(18.)).child(
                        h_flex()
                            .w_full()
                            .justify_center()
                            .pr(px(4.))
                            .child(v_flex().w_full().max_w(chat_mw).gap_0().child(card)),
                    ),
                );
            }
        }

        NativeWorkspace {
            sidebar: sidebar.w_full().into_any_element(),
            pane: pane
                .id("transcript-scroll")
                .track_scroll(&self.transcript_scroll)
                .overflow_y_scroll()
                .vertical_scrollbar(&self.transcript_scroll)
                .into_any_element(),
            inbox_pane: inbox_pane.overflow_y_scrollbar().into_any_element(),
            dev_pane: dev_pane.overflow_y_scrollbar().into_any_element(),
            files_pane: files_pane.overflow_y_scrollbar().into_any_element(),
            update_toast,
            composer_bar: composer_bar.into_any_element(),
            split_bar: split_bar.into_any_element(),
            split_pane: split_pane.overflow_y_scrollbar().into_any_element(),
            provider_banner: provider_banner.map(|el| el.into_any_element()),
            thread_error_banner: thread_error_banner.map(|el| el.into_any_element()),
            query,
            toast_shown,
            has_thread,
            show_landing,
            thread_v: thread_v.clone(),
        }
    }
}
