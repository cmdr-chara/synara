//! Overlay surfaces extracted from native/mod.rs (wave 2 of the split):
//! context menus, hover cards, palette, model picker, dialogs and popovers.
#![allow(unused_imports)]

use super::*;
use gpui_kit::component::{
    button::{Button, ButtonCustomVariant, ButtonRounded, ButtonVariant, ButtonVariants as _},
    h_flex,
    input::{Editor, EditorState, Input, InputEvent, InputState},
    menu::{DropdownMenu as _, PopupMenuItem},
    slider::{Slider, SliderEvent, SliderState},
    scroll::ScrollableElement as _,
    spinner::Spinner,
    text::TextView,
    theme::Theme,
    v_flex,
    Icon,
    ActiveTheme as _, IconName, Sizable as _,
};
use gpui_kit::*;
use gpui_kit::component::WindowExt as _;
use gpui_kit::base::Disableable as _;
use gpui_kit::assets::IconName as AssetIcon;
use gpui::prelude::FluentBuilder as _;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

pub(super) struct NativeOverlays {
    pub(super) ctx_overlay: gpui::AnyElement,
    pub(super) hc_overlay: gpui::AnyElement,
    pub(super) pc_overlay: gpui::AnyElement,
    pub(super) mp_overlay: gpui::AnyElement,
    pub(super) bp_overlay: gpui::AnyElement,
    pub(super) ren_overlay: gpui::AnyElement,
    pub(super) palette_overlay: gpui::AnyElement,
    pub(super) sc_overlay: gpui::AnyElement,
    pub(super) fb_overlay: gpui::AnyElement,
    pub(super) scd_overlay: gpui::AnyElement,
    pub(super) snz_overlay: gpui::AnyElement,
    pub(super) tour_overlay: gpui::AnyElement,
    pub(super) git_overlay: gpui::AnyElement,
    pub(super) autom_overlay: gpui::AnyElement,
    pub(super) tp_overlay: gpui::AnyElement,
    pub(super) imp_overlay: gpui::AnyElement,
    pub(super) pr_overlay: gpui::AnyElement,
    pub(super) wn_pop_overlay: gpui::AnyElement,
    pub(super) wn_dialog_overlay: gpui::AnyElement,
}

impl Native {
    pub(super) fn build_overlays(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> NativeOverlays {
        let theme = cx.theme().clone();
        let dark_ui = match self.appearance_mode {
            2 => true,
            1 => false,
            _ => matches!(
                cx.window_appearance(),
                gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
            ),
        };
        let well_bg = if dark_ui { rgb(0x1d1d1d) } else { rgb(0xf7f7f7) };
        let hairline = if dark_ui { rgb(0x2e2e2e) } else { rgb(0xe5e5e5) };
        let subtle = if dark_ui { rgba(0xffffff14) } else { rgba(0x0000000d) };
        let chip_bg = if dark_ui { rgba(0xffffff12) } else { rgba(0x00000008) };
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

        // Thread context menu (right-click on a sidebar thread row): upstream
        // handleThreadContextMenu — rename, pin, handoff, fork/copy variant
        // submenus (contextMenuGroup), open-path-in-terminal, close tab,
        // archive, delete.
        let mut ctx_overlay = div().absolute().inset_0().size_full();
        if let Some((menu_tid, mx, my)) = self.ctx_menu.clone() {
            let menu_title = self
                .snapshot
                .as_ref()
                .and_then(|s| {
                    arr(s, "threads").iter().find_map(|t| {
                        if str_at(t, "id") == menu_tid {
                            Some(str_at(t, "title").to_string())
                        } else {
                            None
                        }
                    })
                })
                .unwrap_or_default();
            let is_pinned = self.pinned_threads.contains(&menu_tid);
            let ws_path = self.thread_workspace_cwd(&menu_tid);
            let row = |id: &'static str,
                       label: String,
                       cb: Box<dyn Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static>| {
                Button::new(id)
                    .ghost()
                    .w_full()
                    .h(px(30.))
                    .justify_start()
                    .px_2()
                    .label(label)
                    .on_click(cx.listener(move |this, _ev, w, cx| {
                        this.ctx_menu = None;
                        this.ctx_sub = None;
                        this.ctx_confirm_delete = None;
                        cb(this, w, cx);
                        cx.notify();
                    }))
                    .into_any_element()
            };
            // Submenu parent row (upstream contextMenuGroup): one compact line
            // with a chevron that opens a flyout aligned to this row's top.
            let prow = |id: &'static str, sub: &'static str, label: &'static str, top: f32| {
                h_flex()
                    .id(id)
                    .w_full()
                    .h(px(30.))
                    .px_2()
                    .items_center()
                    .justify_between()
                    .rounded_md()
                    .text_sm()
                    .cursor_pointer()
                    .hover(|d| d.bg(theme.secondary))
                    .child(label)
                    .child(div().text_color(theme.muted_foreground).child("›"))
                    .on_click(cx.listener(move |this, _ev, _w, cx| {
                        this.ctx_sub = if this.ctx_sub.as_ref().map(|s| s.0.as_str()) == Some(sub) {
                            None
                        } else {
                            Some((sub.to_string(), top))
                        };
                        cx.notify();
                    }))
                    .into_any_element()
            };
            let sep = || div().h(px(1.)).w_full().bg(theme.border).into_any_element();
            let tid_rename = menu_tid.clone();
            let title_rename = menu_title.clone();
            let tid_pin = menu_tid.clone();
            let tid_arch = menu_tid.clone();
            let tid_close = menu_tid.clone();
            let tid_hand = menu_tid.clone();
            let tid_del = menu_tid.clone();
            let tid_del_flag = menu_tid.clone();
            // Upstream "Clear notification" row: shown when the thread has a
            // dismissible status pill. Keyed statuses store the dismissalKey;
            // Completed/Reminder store a lastVisitedAt stamp instead.
            let clear: Option<(bool, String)> = self
                .snapshot
                .as_ref()
                .and_then(|s| {
                    arr(s, "threads")
                        .iter()
                        .find(|t| str_at(t, "id") == menu_tid)
                        .and_then(|t| {
                            inbox_needs_you_kind(
                                t,
                                &self.dismissed_status,
                                &self.visited_threads,
                            )
                            .map(|k| {
                                if matches!(k, "approval" | "input" | "plan") {
                                    (true, dismissal_key(needs_you_label(t, k), t))
                                } else {
                                    let a = str_at(get(t, "latestTurn"), "completedAt");
                                    let b = str_at(t, "snoozeReminderAt");
                                    (
                                        false,
                                        if a >= b { a.to_string() } else { b.to_string() },
                                    )
                                }
                            })
                        })
                });
            let menu_thread = self
                .snapshot
                .as_ref()
                .and_then(|s| {
                    arr(s, "threads")
                        .iter()
                        .find(|t| str_at(t, "id") == menu_tid)
                        .cloned()
                })
                .unwrap_or(Value::Null);
            let is_snoozed = !get(&menu_thread, "snoozedUntil").is_null();
            let unread_stamp = chrono::DateTime::parse_from_rfc3339(str_at(
                get(&menu_thread, "latestTurn"),
                "completedAt",
            ))
            .ok()
            .map(|d| {
                (d - chrono::Duration::milliseconds(1))
                    .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
            });
            let pin_label = if is_pinned { "Unpin thread" } else { "Pin thread" };
            let mut menu = v_flex()
                .absolute()
                .left(px(mx))
                .top(px(my))
                .w(px(180.))
                .p_1()
                .gap_y(px(1.))
                .bg(popover_bg)
                .border_1()
                .border_color(theme.border)
                .rounded_lg()
                .shadow_md()
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(|_this, _ev, _w, cx| cx.stop_propagation()),
                )
                .on_mouse_down(
                    gpui::MouseButton::Right,
                    cx.listener(|_this, _ev, _w, cx| cx.stop_propagation()),
                );
            if self.ctx_confirm_delete.is_some() {
                // Inline confirm like upstream confirmAndDeleteThread.
                menu = menu
                    .child(
                        div()
                            .px_2()
                            .pt_1()
                            .pb(px(2.))
                            .text_sm()
                            .child(format!("Delete thread \"{menu_title}\"?")),
                    )
                    .child(
                        div()
                            .px_2()
                            .pb_1()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(
                                "This permanently clears conversation history for this thread.",
                            ),
                    )
                    .child(
                        h_flex()
                            .id("ctx-del-go")
                            .w_full()
                            .h(px(30.))
                            .px_2()
                            .items_center()
                            .rounded_md()
                            .text_sm()
                            .text_color(theme.danger)
                            .cursor_pointer()
                            .hover(|d| d.bg(theme.secondary))
                            .child("Delete")
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                this.ctx_menu = None;
                                this.ctx_sub = None;
                                this.ctx_confirm_delete = None;
                                this.delete_thread(tid_del.clone(), cx);
                                cx.notify();
                            })),
                    )
                    .child(
                        h_flex()
                            .id("ctx-del-cancel")
                            .w_full()
                            .h(px(30.))
                            .px_2()
                            .items_center()
                            .rounded_md()
                            .text_sm()
                            .cursor_pointer()
                            .hover(|d| d.bg(theme.secondary))
                            .child("Cancel")
                            .on_click(cx.listener(|this, _ev, _w, cx| {
                                this.ctx_confirm_delete = None;
                                cx.notify();
                            })),
                    );
            } else {
                let mut y = my + 4.0;
                // Tab-menu extras (upstream OpenThreadTabStrip): scope rows
                // first, then a separator, then the usual thread actions.
                if self.ctx_menu_tab {
                    if let Some(idx) =
                        self.open_threads.iter().position(|t| t == &menu_tid)
                    {
                        let n = self.open_threads.len();
                        let mut extra = 0.0f32;
                        if idx > 0 {
                            let tid_l = menu_tid.clone();
                            menu = menu.child(row(
                                "ctx-cl-left",
                                "Close tabs to the left".to_string(),
                                Box::new(move |this, _w, cx| {
                                    this.close_tabs_scope(&tid_l, "left", cx);
                                }),
                            ));
                            extra += 30.0;
                        }
                        if idx + 1 < n {
                            let tid_r = menu_tid.clone();
                            menu = menu.child(row(
                                "ctx-cl-right",
                                "Close tabs to the right".to_string(),
                                Box::new(move |this, _w, cx| {
                                    this.close_tabs_scope(&tid_r, "right", cx);
                                }),
                            ));
                            extra += 30.0;
                        }
                        if n > 1 {
                            let tid_o = menu_tid.clone();
                            menu = menu.child(row(
                                "ctx-cl-others",
                                "Close other tabs".to_string(),
                                Box::new(move |this, _w, cx| {
                                    this.close_tabs_scope(&tid_o, "others", cx);
                                }),
                            ));
                            extra += 30.0;
                        }
                        if extra > 0.0 {
                            menu = menu.child(sep());
                            y += extra + 5.0;
                        }
                    }
                }
                menu = menu
                    .child(row("ctx-rename", "Rename thread".to_string(), Box::new(move |this, w, cx| {
                        this.open_thread(tid_rename.clone(), cx);
                        let input = cx.new(|cx| {
                            InputState::new(w, cx).placeholder("Thread title")
                        });
                        input.update(cx, |s, cx| {
                            s.set_value(title_rename.clone(), w, cx);
                        });
                        cx.subscribe_in(&input, w, |this: &mut Self, _s, event, _w, cx| {
                            if let InputEvent::PressEnter { .. } = event {
                                this.commit_rename(cx);
                            }
                        })
                        .detach();
                        let focus = input.read(cx).focus_handle(cx);
                        w.focus(&focus, cx);
                        this.rename_input = Some(input);
                        this.renaming = true;
                    })))
                    .child(row("ctx-pin", pin_label.to_string(), Box::new(move |this, _w, cx| {
                        this.toggle_pin(tid_pin.clone(), cx);
                    })))
                    .when_some(clear.clone(), |m, (is_key, val)| {
                        let tid_c = menu_tid.clone();
                        m.child(row("ctx-clearnotif", "Clear notification".to_string(), Box::new(
                            move |this, _w, cx| {
                                if is_key {
                                    this.dismissed_status.insert(tid_c.clone(), val.clone());
                                } else {
                                    this.visited_threads.insert(tid_c.clone(), val.clone());
                                }
                                this.persist_sidebar_ui(cx);
                            },
                        )))
                    })
                    .when_some(unread_stamp.clone(), |m, stamp| {
                        let tid_u = menu_tid.clone();
                        m.child(row("ctx-markunread", "Mark unread".to_string(), Box::new(
                            move |this, _w, cx| {
                                this.visited_threads.insert(tid_u.clone(), stamp.clone());
                                this.persist_sidebar_ui(cx);
                            },
                        )))
                    })
                    .when(is_snoozed, |m| {
                        let tid_sc = menu_tid.clone();
                        m.child(row("ctx-snooze-cancel", "Return now".to_string(), Box::new(
                            move |this, _w, cx| {
                                this.dispatch_cmd(
                                    json!({
                                        "type": "thread.meta.update",
                                        "commandId": uuid(),
                                        "threadId": tid_sc,
                                        "snoozedUntil": Value::Null,
                                    }),
                                    cx,
                                );
                            },
                        )))
                    })
                    .child(prow(
                        "ctx-snooze",
                        "snooze",
                        if is_snoozed { "Reschedule" } else { "Snooze" },
                        {
                            let mut yy = y;
                            yy += 90.0 + if clear.is_some() { 30.0 } else { 0.0 }
                                + if unread_stamp.is_some() { 30.0 } else { 0.0 }
                                + if is_snoozed { 30.0 } else { 0.0 };
                            yy
                        },
                    ))
                    .child(row("ctx-handoff", "Handoff".to_string(), Box::new(move |this, _w, cx| {
                        this.lineage(tid_hand.clone(), true, None, cx);
                    })));
                y += 93.0 + if clear.is_some() { 30.0 } else { 0.0 }
                    + if unread_stamp.is_some() { 30.0 } else { 0.0 }
                    + 30.0
                    + if is_snoozed { 30.0 } else { 0.0 };
                menu = menu.child(prow("ctx-fork", "fork", "Fork", y));
                y += 34.0;
                // Copy group has separatorBefore:true upstream.
                menu = menu.child(sep()).child(prow("ctx-copy", "copy", "Copy", y));
                if let Some(path) = ws_path.clone() {
                    menu = menu.child(
                        row("ctx-open-term", "Open Path in Terminal".to_string(), Box::new(
                            move |this, w, cx| this.open_terminal_at(path.clone(), w, cx),
                        )),
                    );
                }
                menu = menu
                    .child(row("ctx-close", "Close tab".to_string(), Box::new(move |this, _w, cx| {
                        this.close_tab(tid_close.clone(), cx);
                    })))
                    .child(sep())
                    .child(row("ctx-archive", "Archive".to_string(), Box::new(move |this, _w, cx| {
                        this.open_thread(tid_arch.clone(), cx);
                        this.archive_selected(cx);
                    })))
                    .child(
                        h_flex()
                            .id("ctx-del")
                            .w_full()
                            .h(px(30.))
                            .px_2()
                            .items_center()
                            .rounded_md()
                            .text_sm()
                            .text_color(theme.danger)
                            .cursor_pointer()
                            .hover(|d| d.bg(theme.secondary))
                            .child("Delete")
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                this.ctx_confirm_delete = Some(tid_del_flag.clone());
                                cx.notify();
                            })),
                    );
            }
            // Backdrop as ancestor + stop-propagation shield around the popup
            // (same pattern as the palette overlay): clicks inside the menu or
            // its flyout never reach the dismiss listener.
            // The shield lives on the menu/flyout elements themselves — a
            // full-size stop-propagation wrapper would eat the backdrop clicks
            // that dismiss the menu.
            let mut inner = div().child(menu);
            // Submenu flyout: renders next to the popup, aligned to the
            // parent row that opened it.
            if self.ctx_confirm_delete.is_none() {
                if let Some((sub, sub_y)) = self.ctx_sub.clone() {
                    let mut fly = v_flex()
                        .absolute()
                        .left(px(mx + 179.))
                        .top(px(sub_y))
                        .w(px(200.))
                        .p_1()
                        .gap_y(px(1.))
                        .bg(popover_bg)
                        .border_1()
                        .border_color(theme.border)
                        .rounded_lg()
                        .shadow_md()
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(|_this, _ev, _w, cx| cx.stop_propagation()),
                        )
                        .on_mouse_down(
                            gpui::MouseButton::Right,
                            cx.listener(|_this, _ev, _w, cx| cx.stop_propagation()),
                        );
                    match sub.as_str() {
                        "fork" => {
                            let t_local = menu_tid.clone();
                            let t_wt = menu_tid.clone();
                            fly = fly
                                .child(row("ctx-fork-local", "Fork Into Local".to_string(), Box::new(
                                    move |this, _w, cx| {
                                        this.lineage(t_local.clone(), false, Some("local"), cx);
                                    },
                                )))
                                .child(row("ctx-fork-wt", "Fork Into New Worktree".to_string(), Box::new(
                                    move |this, _w, cx| {
                                        this.lineage(t_wt.clone(), false, Some("worktree"), cx);
                                    },
                                )));
                        }
                        "snooze" => {
                            let mk = |id: &'static str, label: &'static str, mins: i64| {
                                let tid_s = menu_tid.clone();
                                row(id, label.to_string(), Box::new(move |this, _w, cx| {
                                    let until = (chrono::Utc::now()
                                        + chrono::Duration::minutes(mins))
                                    .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                                    this.dispatch_cmd(
                                        json!({
                                            "type": "thread.meta.update",
                                            "commandId": uuid(),
                                            "threadId": tid_s,
                                            "snoozedUntil": until,
                                        }),
                                        cx,
                                    );
                                }))
                            };
                            fly = fly
                                .child(mk("ctx-snooze-30", "For 30 minutes", 30))
                                .child(mk("ctx-snooze-60", "For 1 hour", 60))
                                .child(mk("ctx-snooze-120", "For 2 hours", 120))
                                .child({
                                    let tid_s = menu_tid.clone();
                                    row("ctx-snooze-tomorrow", "Until tomorrow at 9am".to_string(), Box::new(
                                        move |this, _w, cx| {
                                            let now = chrono::Local::now();
                                            let mut d = now + chrono::Duration::days(1);
                                            if let Some(nine) = d
                                                .date_naive()
                                                .and_hms_opt(9, 0, 0)
                                                .and_then(|t| {
                                                    t.and_local_timezone(chrono::Local).earliest()
                                                })
                                            {
                                                d = nine;
                                            }
                                            this.dispatch_cmd(
                                                json!({
                                                    "type": "thread.meta.update",
                                                    "commandId": uuid(),
                                                    "threadId": tid_s,
                                                    "snoozedUntil": d.to_rfc3339_opts(
                                                        chrono::SecondsFormat::Millis, true,
                                                    ),
                                                }),
                                                cx,
                                            );
                                        },
                                    ))
                                })
                                .child({
                                    let tid_s = menu_tid.clone();
                                    let cur = str_at(&menu_thread, "snoozedUntil")
                                        .to_string();
                                    row("ctx-snooze-custom", "Pick a date & time…".to_string(), Box::new(
                                        move |this, w, cx| {
                                            this.open_snooze_dialog(
                                                tid_s.clone(),
                                                (!cur.is_empty()).then(|| cur.clone()),
                                                w,
                                                cx,
                                            );
                                        },
                                    ))
                                });
                        }
                        "copy" => {
                            if let Some(path) = ws_path.clone() {
                                fly = fly.child(row("ctx-copy-path", "Path".to_string(), Box::new(
                                    move |_this, _w, cx| {
                                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                            path.clone(),
                                        ));
                                    },
                                )));
                            } else {
                                fly = fly.child(
                                    div()
                                        .w_full()
                                        .h(px(30.))
                                        .px_2()
                                        .flex()
                                        .items_center()
                                        .text_sm()
                                        .text_color(theme.muted_foreground)
                                        .child("Path"),
                                );
                            }
                            let t_id = menu_tid.clone();
                            fly = fly.child(row("ctx-copy-id", "Thread ID".to_string(), Box::new(
                                move |_this, _w, cx| {
                                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                        t_id.clone(),
                                    ));
                                },
                            )));
                        }
                        _ => {}
                    }
                    inner = inner.child(fly);
                }
            }
            ctx_overlay = ctx_overlay.child(
                v_flex()
                    .size_full()
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|this, _ev, _w, cx| {
                            this.ctx_menu = None;
                            this.ctx_sub = None;
                            this.ctx_confirm_delete = None;
                            cx.notify();
                        }),
                    )
                    .on_mouse_down(
                        gpui::MouseButton::Right,
                        cx.listener(|this, _ev, _w, cx| {
                            this.ctx_menu = None;
                            this.ctx_sub = None;
                            this.ctx_confirm_delete = None;
                            cx.notify();
                        }),
                    )
                    .child(inner),
            );
        }

        // ---- file-row context menu (upstream fileReferenceContextMenu):
        // Reference file in chat / Reveal in Finder / Copy path.
        if let Some((fpath, fx, fy)) = self.file_ctx_menu.clone() {
            let frow = |id: &'static str,
                        label: &'static str,
                        cb: Box<dyn Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static>| {
                Button::new(id)
                    .ghost()
                    .w_full()
                    .h(px(30.))
                    .justify_start()
                    .px_2()
                    .label(label)
                    .on_click(cx.listener(move |this, _ev, w, cx| {
                        this.file_ctx_menu = None;
                        cb(this, w, cx);
                        cx.notify();
                    }))
                    .into_any_element()
            };
            let p_ref = fpath.clone();
            let p_reveal = fpath.clone();
            let p_copy = fpath.clone();
            ctx_overlay = ctx_overlay.child(
                v_flex()
                    .size_full()
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|this, _ev, _w, cx| {
                            this.file_ctx_menu = None;
                            cx.notify();
                        }),
                    )
                    .on_mouse_down(
                        gpui::MouseButton::Right,
                        cx.listener(|this, _ev, _w, cx| {
                            this.file_ctx_menu = None;
                            cx.notify();
                        }),
                    )
                    .child(
                        v_flex()
                            .absolute()
                            .left(px(fx))
                            .top(px(fy))
                            .w(px(210.))
                            .p_1()
                            .gap_y(px(1.))
                            .bg(popover_bg)
                            .border_1()
                            .border_color(theme.border)
                            .rounded_lg()
                            .shadow_md()
                            .on_mouse_down(
                                gpui::MouseButton::Left,
                                cx.listener(|_this, _ev, _w, cx| cx.stop_propagation()),
                            )
                            .on_mouse_down(
                                gpui::MouseButton::Right,
                                cx.listener(|_this, _ev, _w, cx| cx.stop_propagation()),
                            )
                            .child(frow("fctx-ref", "Reference file in chat", Box::new(
                                move |this, w, cx| {
                                    let name = p_ref
                                        .rsplit('/')
                                        .next()
                                        .unwrap_or(p_ref.as_str())
                                        .to_string();
                                    let draft = this.composer.read(cx).value().to_string();
                                    let next = if draft.trim().is_empty() {
                                        format!("@{name} ")
                                    } else {
                                        format!("{draft} @{name} ")
                                    };
                                    this.composer.update(cx, |s, cx| {
                                        s.set_value(next, w, cx)
                                    });
                                },
                            )))
                            .child(frow("fctx-reveal", "Reveal in Finder", Box::new(
                                move |_this, _w, cx| {
                                    cx.reveal_path(std::path::Path::new(&p_reveal));
                                },
                            )))
                            .child(frow("fctx-copy", "Copy path", Box::new(
                                move |_this, _w, cx| {
                                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                        p_copy.clone(),
                                    ));
                                },
                            ))),
                    ),
            );
        }

        // ---- project context menu (upstream handleProjectContextMenu):
        // Finder / kanban / copy path, edit + pin, then thread cleanup and
        // the guarded project removal row.
        if let Some((menu_pid, mx, my)) = self.proj_ctx_menu.clone() {
            let mrow = |id: &'static str,
                        label: String,
                        cb: Box<dyn Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static>| {
                Button::new(id)
                    .ghost()
                    .w_full()
                    .h(px(30.))
                    .justify_start()
                    .px_2()
                    .label(label)
                    .on_click(cx.listener(move |this, _ev, w, cx| {
                        this.proj_ctx_menu = None;
                        this.proj_ctx_confirm = false;
                        this.ctx_sub = None;
                        cb(this, w, cx);
                        cx.notify();
                    }))
                    .into_any_element()
            };
            let msep = || div().h(px(1.)).w_full().bg(theme.border).into_any_element();
            let proj = arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects")
                .iter()
                .find(|p| str_at(p, "id") == menu_pid)
                .cloned()
                .unwrap_or(Value::Null);
            let proj_title = str_at(&proj, "title").to_string();
            let proj_root = {
                let r = str_at(&proj, "workspaceRoot");
                if r.is_empty() {
                    str_at(&proj, "workspace_root").to_string()
                } else {
                    r.to_string()
                }
            };
            let proj_pinned = get(&proj, "isPinned").as_bool().unwrap_or(false);
            let has_kids = arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "threads")
                .iter()
                .any(|t| {
                    str_at(t, "projectId") == menu_pid
                        && get(t, "deletedAt").is_null()
                        && get(t, "archivedAt").is_null()
                });
            let mut menu = v_flex()
                .absolute()
                .left(px(mx))
                .top(px(my))
                .w(px(200.))
                .p_1()
                .gap_y(px(1.))
                .bg(popover_bg)
                .border_1()
                .border_color(theme.border)
                .rounded_lg()
                .shadow_md()
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(|_this, _ev, _w, cx| cx.stop_propagation()),
                )
                .on_mouse_down(
                    gpui::MouseButton::Right,
                    cx.listener(|_this, _ev, _w, cx| cx.stop_propagation()),
                );
            if self.proj_ctx_confirm {
                // Inline confirm mirroring the thread delete branch: upstream
                // confirms before removeProject clears threads + folder.
                let pid_del = menu_pid.clone();
                menu = menu
                    .child(
                        div()
                            .px_2()
                            .pt_1()
                            .pb(px(2.))
                            .text_sm()
                            .child(format!("Remove project \"{proj_title}\"?")),
                    )
                    .child(
                        div()
                            .px_2()
                            .pb_1()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(if has_kids {
                                "Threads in this folder are deleted first."
                            } else {
                                "This removes the project from the sidebar."
                            }),
                    )
                    .child(
                        h_flex()
                            .id("pctx-del-go")
                            .w_full()
                            .h(px(30.))
                            .px_2()
                            .items_center()
                            .rounded_md()
                            .text_sm()
                            .text_color(theme.danger)
                            .cursor_pointer()
                            .hover(|d| d.bg(theme.secondary))
                            .child("Remove")
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                this.proj_ctx_menu = None;
                                this.proj_ctx_confirm = false;
                                this.delete_project(pid_del.clone(), cx);
                                cx.notify();
                            })),
                    )
                    .child(
                        h_flex()
                            .id("pctx-del-cancel")
                            .w_full()
                            .h(px(30.))
                            .px_2()
                            .items_center()
                            .rounded_md()
                            .text_sm()
                            .cursor_pointer()
                            .hover(|d| d.bg(theme.secondary))
                            .child("Cancel")
                            .on_click(cx.listener(|this, _ev, _w, cx| {
                                this.proj_ctx_confirm = false;
                                cx.notify();
                            })),
                    );
            } else {
                let p_reveal = proj_root.clone();
                let p_copy = proj_root.clone();
                let pid_pin = menu_pid.clone();
                let pid_edit = menu_pid.clone();
                let title_edit = proj_title.clone();
                let pid_arch = menu_pid.clone();
                let pid_del_t = menu_pid.clone();
                let pid_reloc = menu_pid.clone();
                let root_reloc = proj_root.clone();
                menu = menu
                    .child(mrow("pctx-finder", "Open in Finder".into(), Box::new(
                        move |_this, _w, cx| {
                            if !p_reveal.is_empty() {
                                cx.reveal_path(std::path::Path::new(&p_reveal));
                            }
                        },
                    )))
                    .child(mrow("pctx-kanban", "Open in kanban".into(), Box::new(
                        move |this, _w, cx| {
                            this.view_files = false;
                            this.view_dev = false;
                            this.view_inbox = false;
                            this.view_tasks = false;
                            this.view_pullreq = false;
                            this.view_autom = false;
                            this.view_studio = false;
                            this.view_spaces = false;
                            this.view_kanban = true;
                            cx.notify();
                        },
                    )))
                    .child(mrow("pctx-copy", "Copy path".into(), Box::new(
                        move |_this, _w, cx| {
                            cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                p_copy.clone(),
                            ));
                        },
                    )))
                    .child(msep());
                // Upstream start-dev|stop-dev row (Sidebar.tsx:8890-8930):
                // the start variant opens the run dialog — our equivalent
                // is the dev-servers pane where the command is typed.
                let dev_running = self
                    .dev_servers
                    .iter()
                    .any(|srv| str_at(srv, "projectId") == menu_pid);
                if dev_running {
                    let pid_stop = menu_pid.clone();
                    menu = menu.child(mrow("pctx-stop-dev", "Stop dev server".into(),
                        Box::new(move |this, _w, cx| {
                            this.dev_stop(pid_stop.clone(), cx);
                        })));
                } else {
                    menu = menu.child(mrow("pctx-start-dev", "Start dev server".into(),
                        Box::new(move |this, _w, cx| {
                            this.clear_views();
                            this.view_dev = true;
                            this.fetch_dev_servers(cx);
                        })));
                }
                // Upstream MoveProjectToSpaceMenu: parent row opens a submenu
                // listing every space plus "New space".
                menu = menu.child(
                    h_flex()
                        .id("pctx-space")
                        .w_full()
                        .h(px(30.))
                        .px_2()
                        .items_center()
                        .rounded_md()
                        .text_sm()
                        .cursor_pointer()
                        .hover(|d| d.bg(theme.secondary))
                        .child(div().flex_1().child("Move to space"))
                        .child(div().text_color(theme.muted_foreground).child("›"))
                        .on_click(cx.listener(move |this, _ev, _w, cx| {
                            this.ctx_sub =
                                Some(("pspace".to_string(), my + 130.));
                            cx.notify();
                        })),
                );
                menu = menu
                    .child(msep())
                    .child(mrow("pctx-edit", "Edit project".into(), Box::new(
                        move |this, w, cx| {
                            this.start_project_rename(pid_edit.clone(), title_edit.clone(), w, cx);
                        },
                    )))
                    .child(mrow("pctx-reloc", "Change project path".into(), Box::new(
                        move |this, w, cx| {
                            let pid = pid_reloc.clone();
                            this.proj_relocate = Some((pid.clone(), root_reloc.clone()));
                            this.start_project_relocate(pid, root_reloc.clone(), w, cx);
                        },
                    )))
                    .child(mrow("pctx-pin", if proj_pinned {
                        "Unpin project".to_string()
                    } else {
                        "Pin project".to_string()
                    }, Box::new(move |this, _w, cx| {
                        this.toggle_project_pin(pid_pin.clone(), proj_pinned, cx);
                    })));
                if has_kids {
                    menu = menu
                        .child(msep())
                        .child(mrow("pctx-arch", "Archive all threads".into(), Box::new(
                            move |this, _w, cx| {
                                this.project_threads_cmd(pid_arch.clone(), "thread.archive", cx);
                            },
                        )))
                        .child(mrow("pctx-delt", "Delete all threads".into(), Box::new(
                            move |this, _w, cx| {
                                this.project_threads_cmd(pid_del_t.clone(), "thread.delete", cx);
                            },
                        )));
                }
                menu = menu.child(msep()).child(
                    h_flex()
                        .id("pctx-remove")
                        .w_full()
                        .h(px(30.))
                        .px_2()
                        .items_center()
                        .rounded_md()
                        .text_sm()
                        .text_color(theme.danger)
                        .cursor_pointer()
                        .hover(|d| d.bg(theme.secondary))
                        .child("Remove")
                        .on_click(cx.listener(|this, _ev, _w, cx| {
                            this.proj_ctx_confirm = true;
                            cx.notify();
                        })),
                );
            }
            let mut pinner = div().child(menu);
            // Move-to-space flyout, mirroring the thread menu's submenu:
            // renders right of the popup, aligned to its parent row.
            if !self.proj_ctx_confirm {
                if let Some((sub, sub_y)) = self.ctx_sub.clone() {
                    if sub == "pspace" {
                        let mut fly = v_flex()
                            .absolute()
                            .left(px(mx + 204.))
                            .top(px(sub_y))
                            .w(px(200.))
                            .p_1()
                            .gap_y(px(1.))
                            .bg(popover_bg)
                            .border_1()
                            .border_color(theme.border)
                            .rounded_lg()
                            .shadow_md()
                            .on_mouse_down(
                                gpui::MouseButton::Left,
                                cx.listener(|_this, _ev, _w, cx| cx.stop_propagation()),
                            )
                            .on_mouse_down(
                                gpui::MouseButton::Right,
                                cx.listener(|_this, _ev, _w, cx| cx.stop_propagation()),
                            );
                        let frow = |id: SharedString, label: String, sel: bool,
                                    cb: Box<dyn Fn(&mut Self, &mut Context<Self>) + 'static>| {
                            h_flex()
                                .id(id)
                                .w_full()
                                .h(px(30.))
                                .px_2()
                                .items_center()
                                .rounded_md()
                                .text_sm()
                                .text_color(if sel { theme.accent } else { theme.foreground })
                                .cursor_pointer()
                                .hover(|d| d.bg(theme.secondary))
                                .child(label)
                                .on_click(cx.listener(move |this, _ev, _w, cx| {
                                    this.proj_ctx_menu = None;
                                    this.proj_ctx_confirm = false;
                                    this.ctx_sub = None;
                                    cb(this, cx);
                                    cx.notify();
                                }))
                                .into_any_element()
                        };
                        let cur_space = str_at(&proj, "spaceId").to_string();
                        for space in arr(
                            self.snapshot.as_ref().unwrap_or(&Value::Null),
                            "spaces",
                        ) {
                            let sid = str_at(space, "spaceId").to_string();
                            if sid.is_empty() {
                                continue;
                            }
                            let icon = str_at(space, "icon").to_string();
                            let name = str_at(space, "name").to_string();
                            let label = if icon.is_empty() {
                                name
                            } else {
                                format!("{icon} {name}")
                            };
                            let sel = sid == cur_space;
                            let pid_s = menu_pid.clone();
                            fly = fly.child(frow(
                                SharedString::from(format!("pctx-space-{sid}")),
                                label,
                                sel,
                                Box::new(move |this, cx| {
                                    this.dispatch_cmd(
                                        json!({
                                            "type": "project.meta.update",
                                            "commandId": uuid(),
                                            "projectId": pid_s,
                                            "spaceId": sid,
                                        }),
                                        cx,
                                    );
                                }),
                            ));
                        }
                        if !cur_space.is_empty() {
                            let pid_none = menu_pid.clone();
                            fly = fly.child(frow(
                                "pctx-space-none".into(),
                                "No space".into(),
                                false,
                                Box::new(move |this, cx| {
                                    this.dispatch_cmd(
                                        json!({
                                            "type": "project.meta.update",
                                            "commandId": uuid(),
                                            "projectId": pid_none,
                                            "spaceId": Value::Null,
                                        }),
                                        cx,
                                    );
                                }),
                            ));
                        }
                        fly = fly
                            .child(div().h(px(1.)).w_full().bg(theme.border))
                            .child(frow(
                                "pctx-space-new".into(),
                                "New space".into(),
                                false,
                                Box::new(move |this, cx| {
                                    this.clear_views();
                                    this.view_spaces = true;
                                    cx.notify();
                                }),
                            ));
                        pinner = pinner.child(fly);
                    }
                }
            }
            ctx_overlay = ctx_overlay.child(
                v_flex()
                    .size_full()
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|this, _ev, _w, cx| {
                            this.proj_ctx_menu = None;
                            this.proj_ctx_confirm = false;
                            this.ctx_sub = None;
                            cx.notify();
                        }),
                    )
                    .on_mouse_down(
                        gpui::MouseButton::Right,
                        cx.listener(|this, _ev, _w, cx| {
                            this.proj_ctx_menu = None;
                            this.proj_ctx_confirm = false;
                            this.ctx_sub = None;
                            cx.notify();
                        }),
                    )
                    .child(pinner),
            );
        }

        // ---- thread hover card (upstream ThreadHoverCardContent): title +
        // relative time header, then status / project / cwd / model rows.
        // Zero-size anchor: absolute children position from window origin
        // without the container itself intercepting pointer events.
        let mut hc_overlay = div().absolute().left_0().top_0().w_0().h_0();
        if let Some((hc_tid, hx, hy)) = self.hover_card.clone() {
            let threads_arr = arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "threads");
            if let Some(t) = threads_arr.iter().find(|t| str_at(t, "id") == hc_tid).cloned() {
                let title = str_at(&t, "title");
                let title = if title.is_empty() { "(untitled)".to_string() } else { title.to_string() };
                let rel = rel_time(str_at(&t, "updatedAt"));
                let status = inbox_needs_you_kind(&t, &self.dismissed_status, &self.visited_threads)
                    .map(|k| match k {
                        "approval" => "Needs approval",
                        "input" => "Awaiting input",
                        "plan" => "Plan ready",
                        "failed" => "Failed",
                        _ => "Unread",
                    })
                    .or_else(|| if thread_actively_working(&t) { Some("Working") } else { None })
                    .unwrap_or("Idle");
                let pid = str_at(&t, "projectId");
                let pname = arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects")
                    .iter()
                    .find(|p| str_at(p, "id") == pid)
                    .map(|p| str_at(p, "name").to_string())
                    .unwrap_or_default();
                let cwd = arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects")
                    .iter()
                    .find(|p| str_at(p, "id") == pid)
                    .map(|p| {
                        let r = str_at(p, "workspaceRoot");
                        if r.is_empty() { str_at(p, "root").to_string() } else { r.to_string() }
                    })
                    .unwrap_or_default();
                let model = {
                    let s = get(&t, "session");
                    let m = str_at(s, "modelName");
                    let m2 = str_at(s, "model");
                    let p = str_at(s, "providerName");
                    let pick = if !m.is_empty() { m } else { m2 };
                    if pick.is_empty() && p.is_empty() { String::new() } else { format!("{p} {pick}").trim().to_string() }
                };
                let mut card = v_flex()
                    .w(px(300.))
                    .p_2()
                    .gap_1()
                    .bg(popover_bg)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .shadow_md()
                    .child(
                        h_flex()
                            .items_baseline()
                            .justify_between()
                            .child(div().font_weight(FontWeight::SEMIBOLD).text_size(px(13.)).child(title))
                            .child(div().text_size(px(11.)).text_color(theme.muted_foreground).child(rel)),
                    )
                    .child(div().text_size(px(12.)).text_color(theme.accent).child(status));
                if !pname.is_empty() {
                    card = card.child(div().text_size(px(12.)).text_color(theme.foreground).child(pname));
                }
                if !cwd.is_empty() {
                    card = card.child(div().text_size(px(11.)).text_color(theme.muted_foreground).whitespace_nowrap().overflow_hidden().child(cwd));
                }
                if !model.is_empty() {
                    card = card.child(div().text_size(px(11.)).text_color(theme.muted_foreground).child(model));
                }
                let vs = window.viewport_size();
                // Mounted separately (small box only) — a full-size overlay
                // would swallow the pointer events the card depends on.
                hc_overlay = hc_overlay.child(
                    div()
                        .absolute()
                        .left(px((hx + 14.).min(f32::from(vs.width) - 320.)))
                        .top(px(hy.min(f32::from(vs.height) - 220.)))
                        .child(card),
                );
            }
        }

        // ---- workspace search palette (⌘K / ⌘P / ⇧⌘F): upstream
        // SidebarSearchPalette + WorkspaceSearchPalette — bare h-11 input,
        // max-w-lg popup anchored near the top, 28px result rows grouped by
        // section, arrows+enter keyboard navigation, backdrop/Esc dismiss.
        // ---- project hover card (upstream ProjectHoverCardContent): icon +
        // name + pin toggle header, chat count, path, "Edit project" row.
        let mut pc_overlay = div().absolute().left_0().top_0().w_0().h_0();
        if let Some((pc_pid, pcx, pcy)) = self.project_card.clone() {
            // Transparent backdrop: a click anywhere outside the card
            // dismisses it, and moving off both the row and the card
            // closes it like upstream's PreviewCard does. The overlay
            // wrapper is 0×0 so the backdrop needs explicit viewport size.
            let pc_vs = window.viewport_size();
            pc_overlay = pc_overlay.child(
                div()
                    .absolute()
                    .left_0()
                    .top_0()
                    .w(pc_vs.width)
                    .h(pc_vs.height)
                    .on_mouse_down(gpui::MouseButton::Left, cx.listener(
                        |this, _ev, _w, cx| {
                            this.project_card = None;
                            this.hover_project = None;
                            cx.notify();
                        },
                    ))
                    .on_mouse_down(gpui::MouseButton::Right, cx.listener(
                        |this, _ev, _w, cx| {
                            this.project_card = None;
                            this.hover_project = None;
                            cx.notify();
                        },
                    ))
                    .on_mouse_move(cx.listener(|this, _ev, _w, cx| {
                        if this.hover_project.is_none() && this.project_card.is_some() {
                            this.project_card = None;
                            cx.notify();
                        }
                    })),
            );
            let projects_arr = arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects");
            if let Some(p) = projects_arr
                .iter()
                .find(|p| str_at(p, "id") == pc_pid)
                .cloned()
            {
                let name = {
                    let t = str_at(&p, "title");
                    if t.is_empty() { "Untitled project".to_string() } else { t.to_string() }
                };
                let pinned = get(&p, "isPinned").as_bool().unwrap_or(false);
                let root = {
                    let r = str_at(&p, "workspaceRoot");
                    if r.is_empty() {
                        str_at(&p, "workspace_root").to_string()
                    } else {
                        r.to_string()
                    }
                };
                let n_chats = arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "threads")
                    .iter()
                    .filter(|t| {
                        str_at(t, "projectId") == pc_pid
                            && get(t, "deletedAt").is_null()
                            && get(t, "archivedAt").is_null()
                    })
                    .count();
                let pid_pin = pc_pid.clone();
                let pid_edit = pc_pid.clone();
                let name_edit = name.clone();
                let pin_icon = if pinned { icons::PIN_FILL } else { icons::PIN };
                let divider = |d: Div| d;
                let _ = divider;
                let card = v_flex()
                    .w(px(300.))
                    .p_1()
                    .bg(popover_bg)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .shadow_md()
                    .child(
                        h_flex()
                            .h(px(30.))
                            .px_2()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_size(px(13.))
                                    .flex_1()
                                    .child(name),
                            )
                            .child(
                                div()
                                    .id("pc-pin")
                                    .cursor_pointer()
                                    .rounded_sm()
                                    .p_1()
                                    .text_color(if pinned {
                                        theme.foreground
                                    } else {
                                        theme.muted_foreground
                                    })
                                    .hover(|d| d.text_color(theme.foreground))
                                    .child(svg_icon(pin_icon).size(px(12.)))
                                    .on_click(cx.listener(move |this, _ev, _w, cx| {
                                        this.project_card = None;
                                        this.toggle_project_pin(pid_pin.clone(), pinned, cx);
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        h_flex()
                            .h(px(28.))
                            .px_2()
                            .items_center()
                            .gap_2()
                            .text_size(px(12.))
                            .text_color(theme.muted_foreground)
                            .child(format!(
                                "{n_chats} {}",
                                if n_chats == 1 { "chat" } else { "chats" }
                            )),
                    )
                    .child(div().h(px(1.)).w_full().bg(theme.border));
                let mut card = card.child(
                    h_flex()
                        .h(px(28.))
                        .px_2()
                        .items_center()
                        .gap_2()
                        .text_size(px(12.))
                        .text_color(theme.muted_foreground)
                        .child(
                            div()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .child(if root.is_empty() {
                                    "No workspace folder".to_string()
                                } else {
                                    root
                                }),
                        ),
                );
                card = card
                    .child(div().h(px(1.)).w_full().bg(theme.border))
                    .child(
                        h_flex()
                            .id("pc-edit")
                            .h(px(30.))
                            .px_2()
                            .items_center()
                            .gap_2()
                            .rounded_md()
                            .text_size(px(12.))
                            .text_color(theme.foreground)
                            .cursor_pointer()
                            .hover(|d| d.bg(theme.secondary))
                            .child("Edit project")
                            .on_click(cx.listener(move |this, _ev, w, cx| {
                                this.project_card = None;
                                this.start_project_rename(
                                    pid_edit.clone(),
                                    name_edit.clone(),
                                    w,
                                    cx,
                                );
                            })),
                    );
                let vs = window.viewport_size();
                pc_overlay = pc_overlay.child(
                    div()
                        .absolute()
                        .left(px((pcx + 14.).min(f32::from(vs.width) - 320.)))
                        .top(px(pcy.min(f32::from(vs.height) - 180.)))
                        .child(card),
                );
            }
        }

        // ---- model picker panel (upstream ComposerModelPicker): anchored
        // bottom-right above the composer — tab row, search input, model rows
        // with ⌘N shortcut hints and star buttons, effort slider section.
        let mut mp_overlay = div().absolute().left_0().top_0().w_0().h_0();
        if self.model_picker_open {
            let mp_vs = window.viewport_size();
            mp_overlay = mp_overlay.child(
                div()
                    .absolute()
                    .left_0()
                    .top_0()
                    .w(mp_vs.width)
                    .h(mp_vs.height)
                    .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _ev, _w, cx| {
                        this.model_picker_open = false;
                        cx.notify();
                    })),
            );
            let dark_ui = theme.is_dark();
            let panel_bg: gpui::Hsla = if dark_ui {
                // Upstream frosts the dither texture through the picker glass
                // (blur(40px) + 55% popover). gpui has no blur-behind, and the
                // blur averages the orange texture into a warm haze — the
                // faithful approximation is the measured warm tone
                // rgb(26,21,18), not the neutral popover rgb(19,19,19).
                hsla(0.059, 0.18, 0.0865, 1.0)
            } else {
                gpui::hsla(0.0, 0.0, 1.0, 1.0)
            };
            let muted_fg = if dark_ui {
                hsla(0.0, 0.0, 0.62, 1.0)
            } else {
                hsla(0.0, 0.0, 0.42, 1.0)
            };
            let accent_orange = if dark_ui {
                hsla(0.048, 0.98, 0.63, 1.0)
            } else {
                hsla(0.048, 0.9, 0.45, 1.0)
            };
            // Rows: the open provider tab's catalog slice, or its seeded list.
            let active_prov = self.model_picker_provider.clone();
            let mut rows: Vec<(String, String, String)> = self
                .model_catalog
                .iter()
                .filter(|(_, _, prov)| *prov == active_prov)
                .cloned()
                .collect();
            if rows.is_empty() {
                rows = match active_prov.as_str() {
                    "devin" => vec![
                        ("adaptive".into(), "Adaptive".into(), "devin".into()),
                        ("swe-2".into(), "SWE 2".into(), "devin".into()),
                        ("swe-1.7".into(), "SWE 1.7".into(), "devin".into()),
                        ("devin-small".into(), "Devin Small".into(), "devin".into()),
                    ],
                    "claudeAgent" => vec![
                        ("claude-opus-5".into(), "Opus 5".into(), "claudeAgent".into()),
                        ("claude-sonnet-5".into(), "Sonnet 5".into(), "claudeAgent".into()),
                        ("claude-haiku-5".into(), "Haiku 5".into(), "claudeAgent".into()),
                    ],
                    _ => vec![
                        ("gpt-6-astra".into(), "6 Astra".into(), "codex".into()),
                        ("gpt-6-sol".into(), "6 Sol".into(), "codex".into()),
                        ("gpt-6-luna".into(), "6 Luna".into(), "codex".into()),
                        ("gpt-5.5".into(), "5.5".into(), "codex".into()),
                        ("gpt-5.4".into(), "5.4".into(), "codex".into()),
                        ("gpt-5.4-mini".into(), "5.4 Mini".into(), "codex".into()),
                        ("gpt-5.3-codex".into(), "5.3 Codex".into(), "codex".into()),
                    ],
                };
            }
            let query = self.model_picker_search.read(cx).value().to_lowercase();
            let cur_slug = self.composer_model.1.clone();
            let mut list = v_flex().w_full().py(px(4.));
            let mut idx = 0;
            for (slug, name, prov) in rows.iter() {
                if !query.is_empty() && !name.to_lowercase().contains(query.as_str()) {
                    continue;
                }
                idx += 1;
                let selected = *slug == cur_slug;
                let hint = if idx <= 9 { format!("\u{2318} {idx}") } else { String::new() };
                let slug_c = slug.clone();
                let name_c = name.clone();
                let prov_c = prov.clone();
                let star_rows = h_flex()
                    .gap_1()
                    .items_center()
                    .child(
                        // Upstream ShortcutKbd: rounded-full pill, fg 12% in
                        // dark, text-ui-2xs (~10px), h-4 min-w-4 px-2.
                        h_flex()
                            .h(px(16.))
                            .min_w(px(16.))
                            .px(px(6.))
                            .rounded(px(8.))
                            .bg(if dark_ui {
                                hsla(0.0, 0.0, 1.0, 0.12)
                            } else {
                                hsla(0.0, 0.0, 0.0, 0.05)
                            })
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(if dark_ui {
                                        hsla(0.0, 0.0, 0.8, 1.0)
                                    } else {
                                        hsla(0.0, 0.0, 0.3, 1.0)
                                    })
                                    .child(hint),
                            ),
                    )
                    .child(svg_icon(icons::STAR).size(px(14.)).text_color(muted_fg));
                let row = h_flex()
                    .id(ElementId::Name(format!("mp-{slug}").into()))
                    .w_full()
                    .px(px(6.))
                    .h(px(28.))
                    .items_center()
                    .gap_2()
                    .cursor_pointer()
                    .rounded(px(6.))
                    .when(selected, |d| d.bg(if dark_ui {
                        // Upstream bg-[--color-background-elevated-secondary]
                        // over the warm frost: barely-there warm lift
                        // (measured rgb(26,21,19) vs the rows' rgb(24,21,19)),
                        // not a gray slab.
                        hsla(0.055, 0.14, 0.105, 1.0)
                    } else {
                        hsla(0.0, 0.0, 0.94, 1.0)
                    }))
                    .hover(|d| d.bg(if dark_ui {
                        hsla(0.055, 0.13, 0.125, 1.0)
                    } else {
                        hsla(0.0, 0.0, 0.92, 1.0)
                    }))
                    .on_click(cx.listener(move |this, _ev, _w, cx| {
                        this.composer_model = (prov_c.clone(), slug_c.clone(), name_c.clone());
                        this.model_picker_open = false;
                        cx.notify();
                    }))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(12.))
                            .text_color(theme.foreground)
                            .child({
                                // Upstream composerModelDisplayName: the
                                // provider family prefix repeats the tab icon,
                                // so rows drop it — "GPT-6 Astra" reads
                                // "6 Astra", "Claude Opus 5.5" reads "Opus 5.5".
                                let mut n = name.clone();
                                for prefix in ["Claude ", "claude ", "GPT-", "gpt-"] {
                                    if let Some(rest) = n.strip_prefix(prefix) {
                                        n = rest.to_string();
                                        break;
                                    }
                                }
                                n
                            }),
                    )
                    .child(star_rows);
                list = list.child(row);
            }
            let search_input = Input::new(&self.model_picker_search)
                .appearance(false)
                .w_full()
                .text_size(px(12.));
            let _ = panel_bg;
            let panel = v_flex()
                .w(px(300.))
                .rounded(px(14.))
                .border_1()
                .border_color(theme.border.opacity(0.6))
                .bg(if dark_ui {
                    // Upstream frost: blur(40px) over the dither texture turns
                    // the dense orange behind the panel top into a warm wash
                    // (measured rgb(44,25,17)) cooling to the rows-zone tone
                    // rgb(26,21,18) further down. gpui has no blur-behind, so
                    // the haze is reproduced as this vertical gradient.
                    gpui::linear_gradient(
                        180.,
                        gpui::linear_color_stop(hsla(0.062, 0.44, 0.118, 1.0), 0.),
                        gpui::linear_color_stop(hsla(0.059, 0.18, 0.0865, 1.0), 0.45),
                    )
                } else {
                    gpui::linear_gradient(
                        180.,
                        gpui::linear_color_stop(gpui::hsla(0.0, 0.0, 1.0, 1.0), 0.),
                        gpui::linear_color_stop(gpui::hsla(0.0, 0.0, 1.0, 1.0), 1.),
                    )
                })
                .shadow_lg()
                .overflow_hidden()
                // tab row: starred tab + provider tab + add
                .child(
                    // Upstream SurfaceTabStrip: 40px strip, h-7 min-w-7
                    // rounded-lg tab buttons, active marker a 2px bar 4px
                    // below the button inset 6px from its sides.
                    h_flex()
                        .w_full()
                        .px(px(8.))
                        .h(px(40.))
                        .items_center()
                        .gap(px(4.))
                        .border_b_1()
                        .border_color(theme.border.opacity(0.5))
                        .child(
                            div()
                                .h(px(28.))
                                .min_w(px(28.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(8.))
                                .px(px(6.))
                                .child(svg_icon(icons::STAR_FILLED).size(px(16.)).text_color(theme.foreground)),
                        )
                        .children(
                            // One icon tab per provider account (upstream
                            // ComposerModelPickerTabs); the open tab carries
                            // the accent marker.
                            [("codex", icons::CODEX), ("devin", icons::P_DEVIN), ("claudeAgent", icons::P_CLAUDE)]
                                .into_iter()
                                .map(|(prov, icon)| {
                                    let is_active = prov == active_prov;
                                    let prov_c = prov.to_string();
                                    let mut tab = div()
                                        .id(ElementId::Name(format!("mp-tab-{prov}").into()))
                                        .h(px(28.))
                                        .min_w(px(28.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded(px(8.))
                                        .px(px(6.))
                                        .cursor_pointer()
                                        .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |this, _ev: &gpui::MouseDownEvent, _w, cx| {
                                            cx.stop_propagation();
                                            this.model_picker_provider = prov_c.clone();
                                            cx.notify();
                                        }))
                                        .child(svg_icon(icon).size(px(16.)).text_color(if is_active {
                                            theme.foreground
                                        } else {
                                            muted_fg
                                        }));
                                    if is_active {
                                        tab = tab.child(
                                            div()
                                                .absolute()
                                                .left(px(6.))
                                                .top(px(32.))
                                                .right(px(6.))
                                                .h(px(2.))
                                                .rounded(px(1.))
                                                .bg(accent_orange),
                                        );
                                    }
                                    tab
                                })
                                .collect::<Vec<_>>(),
                        )
                        .child(div().flex_1())
                        .child(svg_icon(icons::PLUS).size(px(16.)).text_color(muted_fg)),
                )
                // search row
                .child(
                    h_flex()
                        .w_full()
                        .px(px(10.))
                        .h(px(34.))
                        .items_center()
                        .gap_2()
                        .border_b_1()
                        .border_color(theme.border.opacity(0.5))
                        .child(svg_icon(icons::SEARCH).size(px(13.)).text_color(muted_fg))
                        .child(div().flex_1().child(search_input)),
                )
                .child(div().w_full().max_h(px(260.)).overflow_y_scrollbar().child(list))
                // effort section
                .child(
                    v_flex()
                        .w_full()
                        .px(px(10.))
                        .pt(px(8.))
                        .pb(px(12.))
                        .gap(px(6.))
                        .border_t_1()
                        .border_color(theme.border.opacity(0.5))
                        // Upstream grid-cols-[1.5rem_1fr_1.5rem]: fast/zap
                        // left, centered accent label, reset button right.
                        .child(
                            h_flex()
                                .w_full()
                                .items_center()
                                .child(
                                    div()
                                        .w(px(24.))
                                        .child(svg_icon(icons::ZAP).size(px(14.)).text_color(muted_fg)),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .text_center()
                                        .text_size(px(12.))
                                        .font_weight(gpui::FontWeight::MEDIUM)
                                        .text_color(accent_orange)
                                        .child({
                                            const EFFORTS: [&str; 5] = ["Minimal", "Low", "Medium", "High", "Max"];
                                            EFFORTS[self.composer_effort_idx.min(4)].to_string()
                                        }),
                                )
                                .child(
                                    div()
                                        .w(px(24.))
                                        .flex()
                                        .justify_end()
                                        .child(svg_icon(icons::UNDO).size(px(13.)).text_color(muted_fg)),
                                ),
                        )
                        // Upstream Slider size="large": track 24px rounded-full
                        // fg 14%, accent fill edge-aligned under a 28px white
                        // thumb, 4px step dots along the thumb-inset rail.
                        // Rail spans half-thumb insets: 14..266 of the 280px
                        // inner width; the thumb centre sits exactly on the
                        // active stop.
                        .child({
                            let idx = self.composer_effort_idx.min(4);
                            let frac = idx as f32 / 4.0;
                            let stop_x = 14.0 + frac * 252.0;
                            let mut track = div()
                                .id("effort-slider")
                                .w_full()
                                .h(px(28.))
                                .mt(px(4.))
                                .cursor_pointer()
                                .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, ev: &gpui::MouseDownEvent, _w, cx| {
                                    cx.stop_propagation();
                                    // Rail 14..266 over the 280px inner width:
                                    // map the click x to the nearest stop.
                                    let x: f32 = ev.position.x.into();
                                    // position is window-space; the listener
                                    // re-bases via the element bounds — use the
                                    // relative fraction gpui provides instead.
                                    let _ = x;
                                    let _ = this;
                                }));
                            let _ = &mut track;
                            track = track
                                .child(
                                    div()
                                        .absolute()
                                        .left_0()
                                        .top(px(2.))
                                        .w_full()
                                        .h(px(24.))
                                        .rounded(px(12.))
                                        .bg(if dark_ui {
                                            hsla(0.0, 0.0, 1.0, 0.14)
                                        } else {
                                            hsla(0.0, 0.0, 0.0, 0.14)
                                        }),
                                )
                                .child(
                                    div()
                                        .absolute()
                                        .left_0()
                                        .top(px(2.))
                                        .w(px(stop_x))
                                        .h(px(24.))
                                        .rounded(px(12.))
                                        .bg(accent_orange),
                                )
                                .children((0..5).map(|i| {
                                    let f = i as f32 / 4.0;
                                    let x = 14.0 + f * 252.0 - 2.0;
                                    let covered = x + 2.0 <= stop_x + 1.0;
                                    div()
                                        .absolute()
                                        .left(px(x))
                                        .top(px(12.))
                                        .w(px(4.))
                                        .h(px(4.))
                                        .rounded(px(2.))
                                        .bg(if covered {
                                            hsla(0.0, 0.0, 1.0, 0.85)
                                        } else if dark_ui {
                                            hsla(0.0, 0.0, 1.0, 0.35)
                                        } else {
                                            hsla(0.0, 0.0, 0.0, 0.3)
                                        })
                                }))
                                .child(
                                    div()
                                        .absolute()
                                        .left(px(stop_x - 14.0))
                                        .top_0()
                                        .w(px(28.))
                                        .h(px(28.))
                                        .rounded(px(14.))
                                        .bg(gpui::white())
                                        .shadow_sm(),
                                );
                            // stop click zones: five invisible slabs each
                            // covering one stop's segment — simpler and more
                            // reliable than coordinate math on the track.
                            for i in 0..5usize {
                                let f = i as f32 / 4.0;
                                let cx_pos = 14.0 + f * 252.0;
                                track = track.child(
                                    div()
                                        .id(ElementId::Name(format!("effort-stop-{i}").into()))
                                        .absolute()
                                        .left(px(cx_pos - 25.0))
                                        .top_0()
                                        .w(px(50.))
                                        .h(px(28.))
                                        .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |this, _ev: &gpui::MouseDownEvent, _w, cx| {
                                            cx.stop_propagation();
                                            this.composer_effort_idx = i;
                                            cx.notify();
                                        })),
                                );
                            }
                            track
                        }),
                );
            let panel = panel.on_mouse_down(gpui::MouseButton::Left, cx.listener(|_this, _ev: &gpui::MouseDownEvent, _w, cx| {
                cx.stop_propagation();
            }));
            // Anchor: bottom-right above the composer — composer bottom edge
            // sits ~10px over the window bottom padding, panel rises over it.
            mp_overlay = mp_overlay.child(
                div()
                    .absolute()
                    .left_0()
                    .top_0()
                    .w(mp_vs.width)
                    .h(mp_vs.height)
                    .flex()
                    .flex_col()
                    .items_end()
                    .justify_end()
                    .pr(px(16.))
                    .pb(px(148.))
                    .child(panel),
            );
        }

        // ---- shared RenameDialog (upstream RenameDialog): centered card with
        // a single field and Cancel/Save — covers the thread rename (mounted
        // for the first time here) and the project "Edit project" action.
        // Diff-line blame popover (upstream DiffLineBlamePopover):
        // summary + author + date card at the clicked diff line.
        let mut bp_overlay = div().absolute().inset_0().size_full();
        if let Some((bv, bx, by)) = self.blame_pop.clone() {
            bp_overlay = bp_overlay.child(
                div()
                    .size_full()
                    .on_mouse_down(gpui::MouseButton::Left, cx.listener(
                        |this, _ev, _w, cx| {
                            this.blame_pop = None;
                            cx.notify();
                        },
                    )),
            );
            let uncommitted = get(&bv, "uncommitted").as_bool().unwrap_or(false);
            let summary = {
                let t = str_at(&bv, "summary");
                if t.is_empty() && uncommitted { "Uncommitted changes".to_string() } else { t.to_string() }
            };
            let author = str_at(&bv, "author").to_string();
            let when = str_at(&bv, "authorTime").to_string();
            let sha = str_at(&bv, "shortSha").to_string();
            let vs = window.viewport_size();
            let card = v_flex()
                .w(px(264.))
                .gap_1()
                .p_2()
                .bg(popover_bg)
                .border_1()
                .border_color(theme.border)
                .rounded_lg()
                .shadow_md()
                .child(
                    div()
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .child(if uncommitted { "Uncommitted changes".to_string() } else { summary }),
                )
                .child(
                    h_flex()
                        .gap_2()
                        .text_size(px(12.))
                        .text_color(theme.muted_foreground)
                        .child(
                            div()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .flex_1()
                                .child(if uncommitted { "Local edit".to_string() } else { author }),
                        )
                        .child(if uncommitted { "".to_string() } else { when }),
                )
                .when(!sha.is_empty() && !uncommitted, |d| {
                    d.child(
                        div()
                            .text_size(px(11.))
                            .font_family(theme.mono_font_family.clone())
                            .text_color(theme.muted_foreground)
                            .child(sha),
                    )
                });
            bp_overlay = bp_overlay.child(
                div()
                    .absolute()
                    .left(px((bx + 8.).min(f32::from(vs.width) - 280.)))
                    .top(px((by + 8.).min(f32::from(vs.height) - 120.)))
                    .child(card),
            );
        }

        let mut ren_overlay = div().absolute().inset_0().size_full();
        if self.renaming || self.proj_rename.is_some() || self.proj_relocate.is_some() {
            let is_relocate = self.proj_relocate.is_some();
            let is_project = self.proj_rename.is_some();
            let dlg_title = if is_relocate {
                "Change project path"
            } else if is_project {
                "Edit project"
            } else {
                "Rename chat"
            };
            let input = if is_relocate {
                self.proj_relocate_input.clone()
            } else if is_project {
                self.proj_rename_input.clone()
            } else {
                self.rename_input.clone()
            };
            let can_save = input
                .as_ref()
                .map(|i| !i.read(cx).value().trim().is_empty())
                .unwrap_or(false);
            let mut dlg = v_flex()
                .w(px(448.))
                .bg(popover_bg)
                .border_1()
                .border_color(theme.border)
                .rounded(px(14.))
                .shadow_md()
                .overflow_hidden()
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(|_this, _ev, _w, cx| cx.stop_propagation()),
                )
                .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _w, cx| {
                    if ev.keystroke.key == "escape" {
                        this.renaming = false;
                        this.rename_input = None;
                        this.proj_rename = None;
                        this.proj_rename_input = None;
                        this.proj_relocate = None;
                        this.proj_relocate_input = None;
                        cx.stop_propagation();
                        cx.notify();
                    } else if ev.keystroke.key == "enter" {
                        if this.proj_relocate.is_some() {
                            this.commit_project_relocate(cx);
                        } else if this.proj_rename.is_some() {
                            this.commit_project_rename(cx);
                        } else {
                            this.commit_rename(cx);
                        }
                        cx.stop_propagation();
                    }
                }))
                .child(
                    div()
                        .px(px(20.))
                        .pt(px(20.))
                        .pb(px(8.))
                        .text_size(px(20.))
                        .font_weight(FontWeight::MEDIUM)
                        .child(dlg_title),
                );
            if let Some(inp) = input {
                let inp2 = inp.clone();
                dlg = dlg.child(
                    h_flex()
                        .mx(px(20.))
                        .my(px(8.))
                        .gap_2()
                        .items_center()
                        .child(div().flex_1().child(Input::new(&inp)))
                        .when(is_relocate, |d| {
                            d.child(
                                Button::new("reloc-browse")
                                    .ghost()
                                    .label("Browse\u{2026}")
                                    .on_click(cx.listener(move |_this, _ev, w, cx| {
                                        let rx = cx.prompt_for_paths(gpui::PathPromptOptions {
                                            files: false,
                                            directories: true,
                                            multiple: false,
                                            prompt: None,
                                        });
                                        let inp3 = inp2.clone();
                                        cx.spawn_in(w, async move |_this, cx| {
                                            if let Ok(Ok(Some(paths))) = rx.await {
                                                if let Some(dir) = paths.first() {
                                                    let p = dir.to_string_lossy().into_owned();
                                                    let _ = cx.update(|window, app| {
                                                        let _ = inp3.update(app, |s, cx| {
                                                            s.set_value(p, window, cx);
                                                        });
                                                    });
                                                }
                                            }
                                        })
                                        .detach();
                                    })),
                            )
                        }),
                );
            }
            dlg = dlg.child(
                h_flex()
                    .w_full()
                    .px(px(20.))
                    .py(px(16.))
                    .gap_2()
                    .justify_end()
                    .child(
                        Button::new("ren-cancel")
                            .ghost()
                            .label("Cancel")
                            .on_click(cx.listener(|this, _ev, _w, cx| {
                                this.renaming = false;
                                this.rename_input = None;
                                this.proj_rename = None;
                                this.proj_rename_input = None;
                                this.proj_relocate = None;
                                this.proj_relocate_input = None;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("ren-save")
                            .primary()
                            .label(if is_relocate { "Change path" } else { "Save" })
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                if !can_save {
                                    return;
                                }
                                if this.proj_relocate.is_some() {
                                    this.commit_project_relocate(cx);
                                } else if this.proj_rename.is_some() {
                                    this.commit_project_rename(cx);
                                } else {
                                    this.commit_rename(cx);
                                }
                            })),
                    ),
            );
            ren_overlay = ren_overlay.child(
                v_flex()
                    .size_full()
                    .items_center()
                    .justify_center()
                    .bg(gpui::black().opacity(0.3))
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|this, _ev, _w, cx| {
                            this.renaming = false;
                            this.rename_input = None;
                            this.proj_rename = None;
                            this.proj_rename_input = None;
                            this.proj_relocate = None;
                            this.proj_relocate_input = None;
                            cx.notify();
                        }),
                    )
                    .child(dlg),
            );
        }

        let mut palette_overlay = div().absolute().inset_0().size_full();
        if let Some(mode) = self.palette_mode {
            let query = self
                .palette_input
                .as_ref()
                .map(|i| i.read(cx).value().to_string())
                .unwrap_or_default();
            let items = self.palette_items(&query);
            let sel = self.palette_sel;
            let mut popup = v_flex()
                .w(px(512.))
                .mt(px(120.))
                .bg(popover_bg)
                .border_1()
                .border_color(theme.border)
                .rounded(px(12.))
                .shadow_md()
                .overflow_hidden()
                .on_key_down(cx.listener(|this, ev: &KeyDownEvent, window, cx| {
                    match ev.keystroke.key.as_str() {
                        "escape" => this.close_palette(window, cx),
                        "down" => {
                            let q = this
                                .palette_input
                                .as_ref()
                                .map(|i| i.read(cx).value().to_string())
                                .unwrap_or_default();
                            let len = this.palette_items(&q).len();
                            this.palette_sel = (this.palette_sel + 1).min(len.saturating_sub(1));
                            cx.notify();
                        }
                        "up" => {
                            this.palette_sel = this.palette_sel.saturating_sub(1);
                            cx.notify();
                        }
                        "backspace" => {
                            let empty = this
                                .palette_input
                                .as_ref()
                                .map(|i| i.read(cx).value().is_empty())
                                .unwrap_or(true);
                            if empty && !matches!(this.palette_mode, None | Some("search" | "files" | "snippets")) {
                                // Upstream: Backspace on an empty sub-page
                                // query goes back to the search page.
                                this.set_palette_mode("search", window, cx);
                            } else if let Some(input) = this.palette_input.clone() {
                                let mut q = input.read(cx).value().to_string();
                                if q.pop().is_some() {
                                    input.update(cx, |s, cx| {
                                        s.set_value(q.clone(), window, cx)
                                    });
                                    this.palette_query_changed(cx);
                                }
                            }
                        }
                        _ => {
                            // Printable characters: insert manually — the
                            // platform input handler is not reliably bound for
                            // late-mounted palette inputs, so key_char chars
                            // would bubble up unconsumed.
                            if let Some(ch) = ev.keystroke.key_char.as_ref() {
                                let m = ev.keystroke.modifiers;
                                if !m.control && !m.platform && !m.function {
                                    if let Some(input) = this.palette_input.clone() {
                                        let mut q =
                                            input.read(cx).value().to_string();
                                        q.push_str(ch);
                                        input.update(cx, |s, cx| {
                                            s.set_value(q, window, cx)
                                        });
                                        this.palette_query_changed(cx);
                                    }
                                }
                            }
                        }
                    }
                    cx.stop_propagation();
                }));
            if let Some(input) = self.palette_input.clone() {
                let mut input_row = h_flex()
                    .h(px(44.))
                    .px(px(14.))
                    .items_center()
                    .gap_2()
                    .border_b_1()
                    .border_color(theme.border);
                if matches!(
                    mode,
                    "import-sources" | "import-projs" | "import-thread"
                ) {
                    input_row = input_row.child(
                        div()
                            .id("palette-back")
                            .w(px(20.))
                            .h(px(20.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_md()
                            .cursor_pointer()
                            .hover(|d| d.bg(theme.secondary))
                            .child(
                                svg_icon(icons::CHEVRON_LEFT)
                                    .size(px(14.))
                                    .text_color(theme.muted_foreground),
                            )
                            .on_click(cx.listener(|this, _ev, window, cx| {
                                this.set_palette_mode("search", window, cx);
                            })),
                    );
                }
                popup = popup.child(
                    input_row.child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(Input::new(&input).appearance(false)),
                    ),
                );
            }
            let mut list = v_flex()
                .id("palette-list")
                .max_h(px(448.))
                .overflow_y_scroll()
                .p_1()
                .gap_y(px(1.));
            let mut last_group = "";
            let group_label = |kind: &str, query_empty: bool| -> &'static str {
                match kind {
                    "project" => "Projects",
                    "thread" => {
                        if query_empty {
                            "Recent chats"
                        } else {
                            "Threads"
                        }
                    }
                    "snippet" => "Matches",
                    "action" => "Quick actions",
                    "import-src" | "import-proj" => "Import projects",
                    _ => "Files",
                }
            };
            for (i, (kind, label, sub, payload)) in items.iter().enumerate() {
                let group = group_label(kind, query.trim().is_empty());
                if group != last_group {
                    last_group = group;
                    list = list.child(
                        div()
                            .px(px(10.))
                            .pt(px(6.))
                            .pb(px(4.))
                            .text_size(px(12.))
                            .text_color(theme.muted_foreground)
                            .child(group),
                    );
                }
                // Thread rows carry their provider's icon (upstream
                // SharedProviderIcon); generic "New thread" titles none.
                let thread_icon = if kind == "thread" {
                    arr(
                        self.snapshot.as_ref().unwrap_or(&Value::Null),
                        "threads",
                    )
                    .iter()
                    .find(|t| str_at(t, "id") == payload.as_str())
                    .map(|t| {
                        let generic = str_at(t, "title")
                            .trim()
                            .eq_ignore_ascii_case("new thread");
                        if generic {
                            &[][..]
                        } else {
                            provider_icon(str_at(t, "provider"))
                        }
                    })
                    .unwrap_or(icons::S_CHAT_BUBBLES)
                } else {
                    icons::FILE_TEXT
                };
                let icon_data: &[u8] = match kind.as_str() {
                    "directory" | "project" => icons::FOLDER,
                    "thread" => thread_icon,
                    "snippet" => icons::SEARCH,
                    "import-proj" => icons::FOLDER_OPEN,
                    "import-src" => match payload.as_str() {
                        "claudeAgent" => icons::P_CLAUDE,
                        "codex" => icons::CODEX,
                        _ => icons::ARROW_DOWN,
                    },
                    "action" => match payload.as_str() {
                        "new-chat" => icons::SIDECHAT,
                        "new-thread" => icons::NEW_THREAD,
                        "add-project" => icons::FOLDER_ADD,
                        "import-projects" => icons::DOWNLOAD,
                        "import-thread" => icons::IMPORT,
                        "feedback" => icons::BUG,
                        "settings" => icons::S_GEAR,
                        "usage-settings" => icons::S_GAUGE,
                        "new-space" => icons::PLUS,
                        "go-inbox" => icons::INBOX,
                        "go-kanban" => icons::KANBAN,
                        "go-studio" => icons::STUDIO,
                        "go-automations" => icons::AUTOMATIONS,
                        "new-automation" => icons::PLUS_MED,
                        _ => icons::ZAP,
                    },
                    _ => icons::FILE_TEXT,
                };
                let mut r = h_flex()
                    .id(ElementId::Name(format!("pal-{i}").into()))
                    .w_full()
                    .h(px(28.))
                    .px(px(10.))
                    .items_center()
                    .gap_2()
                    .rounded_lg()
                    .cursor_pointer()
                    .child(svg_icon(icon_data).size(px(14.)).text_color(theme.muted_foreground))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .text_size(px(13.))
                            .child(label.clone()),
                    );
                if !sub.is_empty() {
                    r = r.child(
                        div()
                            .max_w(px(220.))
                            .overflow_hidden()
                            .text_size(px(11.))
                            .text_color(theme.muted_foreground)
                            .child(sub.clone()),
                    );
                }
                // Rows that enter a palette page get the trailing ▸ (upstream
                // ChevronRightIcon — only "Import projects from…" has one).
                if kind == "action" && payload.as_str() == "import-projects" {
                    r = r.child(
                        svg_icon(icons::CHEVRON_RIGHT)
                            .size(px(12.))
                            .text_color(theme.muted_foreground),
                    );
                }
                if i == sel {
                    r = r.bg(theme.secondary);
                }
                r = r.on_click(cx.listener(move |this, _ev, window, cx| {
                    this.palette_sel = i;
                    this.palette_activate(window, cx);
                }));
                list = list.child(r);
            }
            if items.is_empty() {
                let msg = match mode {
                    "files" => {
                        if query.trim().is_empty() {
                            "Type to search for files"
                        } else {
                            "No matching files"
                        }
                    }
                    "snippets" => {
                        if query.trim().len() < 2 {
                            "Type at least 2 characters to search code"
                        } else {
                            "No matches"
                        }
                    }
                    "import-projs" => "No importable projects",
                    "import-thread" => "Paste a session id to import",
                    _ => "No results.",
                };
                list = list.child(
                    div()
                        .px(px(10.))
                        .py(px(8.))
                        .text_size(px(12.))
                        .text_color(theme.muted_foreground)
                        .child(msg),
                );
            }
            popup = popup.child(list);
            palette_overlay = palette_overlay.child(
                v_flex()
                    .size_full()
                    .items_center()
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|this, _ev, window, cx| this.close_palette(window, cx)),
                    )
                    .child(
                        div()
                            .on_mouse_down(
                                gpui::MouseButton::Left,
                                cx.listener(|_this, _ev, _w, cx| cx.stop_propagation()),
                            )
                            .child(popup),
                    ),
            );
        }

        // Shortcut recorder dialog (Settings > Keyboard shortcuts).
        let mut sc_overlay = div().absolute().inset_0().size_full();
        if let Some(rec) = self.sc_rec.as_ref().map(|r| ScRec {
            label: r.label.clone(),
            commands: r.commands.clone(),
            numbered: r.numbered,
            replacing: r.replacing.clone(),
            captured: r.captured.clone(),
        }) {
            let (problem, conflicts) = self.sc_evaluate(&rec);
            let captured_label = rec.captured.as_ref().map(sc_fmt_captured);
            let ready = rec.captured.is_some() && problem.is_none();
            let has_conflicts = !conflicts.is_empty();
            let can_save = ready;
            let status_text = if let Some(p) = &problem {
                p.clone()
            } else if ready {
                if has_conflicts {
                    let names: Vec<String> = conflicts
                        .iter()
                        .map(|(c, _)| Self::sc_command_label(c))
                        .collect();
                    format!(
                        "{} already runs \"{}\". Saving moves it here.",
                        captured_label.clone().unwrap_or_default(),
                        names.join("\" and \""),
                    )
                } else {
                    format!(
                        "{} is available.",
                        captured_label.clone().unwrap_or_default()
                    )
                }
            } else if rec.numbered {
                "Hold the modifiers then press any number key.".to_string()
            } else {
                "Use \u{2318}, \u{2325} or \u{2303} with any key, or an F-key on its own."
                    .to_string()
            };
            let status_color: Hsla = if problem.is_some() {
                gpui::hsla(0.0, 0.72, 0.51, 1.0)
            } else if has_conflicts {
                gpui::hsla(0.09, 0.85, 0.47, 1.0)
            } else {
                theme.muted_foreground
            };
            let desc = if rec.replacing.is_empty() {
                "Choose the keys that run it."
            } else {
                "Press new keys to change its shortcut."
            };
            sc_overlay = sc_overlay.child(
                v_flex()
                    .size_full()
                    .items_center()
                    .justify_center()
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|this, _ev, _w, cx| {
                            this.sc_rec = None;
                            cx.notify();
                        }),
                    )
                    .child(
                        div()
                            .on_mouse_down(
                                gpui::MouseButton::Left,
                                cx.listener(|_this, _ev, _w, cx| cx.stop_propagation()),
                            )
                            .child(
                                v_flex()
                                    .w(px(400.))
                                    .gap_3()
                                    .px_4()
                                    .pt_4()
                                    .pb_3()
                                    .rounded_xl()
                                    .bg(popover_bg)
                                    .border_1()
                                    .border_color(theme.border)
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap_3()
                                            .child(
                                                div()
                                                    .w(px(36.))
                                                    .h(px(36.))
                                                    .flex_shrink_0()
                                                    .items_center()
                                                    .justify_center()
                                                    .rounded(px(9.))
                                                    .border_1()
                                                    .border_color(theme.border)
                                                    .bg(theme.muted)
                                                    .child(
                                                        svg_icon(icons::SEARCH)
                                                            .size(px(16.))
                                                            .text_color(theme.foreground),
                                                    ),
                                            )
                                            .child(
                                                v_flex()
                                                    .min_w_0()
                                                    .gap(px(2.))
                                                    .child(
                                                        div()
                                                            .text_size(px(15.))
                                                            .font_weight(FontWeight::MEDIUM)
                                                            .child(rec.label.clone()),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(12.))
                                                            .text_color(theme.muted_foreground)
                                                            .child(desc),
                                                    ),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .w_full()
                                            .min_h(px(88.))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded_xl()
                                            .border_1()
                                            .border_color(theme.border)
                                            .bg(theme.muted)
                                            .px_3()
                                            .child(if let Some(l) = &captured_label {
                                                sc_chip(l.clone(), &theme).into_any_element()
                                            } else {
                                                div()
                                                    .text_size(px(13.))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(theme.muted_foreground)
                                                    .child(if rec.numbered {
                                                        "1\u{2013}9"
                                                    } else {
                                                        "Type a shortcut"
                                                    })
                                                    .into_any_element()
                                            }),
                                    )
                                    .child(
                                        div()
                                            .min_h(px(32.))
                                            .text_size(px(12.))
                                            .line_height(px(16.))
                                            .text_color(status_color)
                                            .child(status_text),
                                    )
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap_2()
                                            .child(div().flex_1())
                                            .child(
                                                h_flex()
                                                    .id("sc-cancel")
                                                    .cursor_pointer()
                                                    .on_click(cx.listener(
                                                        |this, _ev, _w, cx| {
                                                            this.sc_rec = None;
                                                            cx.notify();
                                                        },
                                                    ))
                                                    .child(
                                                        div()
                                                            .px_3()
                                                            .h(px(32.))
                                                            .items_center()
                                                            .rounded(px(8.))
                                                            .border_1()
                                                            .border_color(theme.border)
                                                            .text_size(px(13.))
                                                            .child("Cancel"),
                                                    ),
                                            )
                                            .child(
                                                h_flex()
                                                    .id("sc-save")
                                                    .cursor_pointer()
                                                    .on_click(cx.listener(
                                                        |this, _ev, _w, cx| {
                                                            this.sc_save(cx);
                                                        },
                                                    ))
                                                    .child(
                                                        div()
                                                            .px_3()
                                                            .h(px(32.))
                                                            .items_center()
                                                            .rounded(px(8.))
                                                            .bg(if can_save {
                                                                theme.foreground
                                                            } else {
                                                                theme.muted
                                                            })
                                                            .text_size(px(13.))
                                                            .text_color(theme.background)
                                                            .child("Save"),
                                                    ),
                                            ),
                                    ),
                            ),
                    ),
            );
        }

        // Feedback dialog (upstream FeedbackDialog): centered card over a
        // dimmed backdrop — category pills, details textarea, diagnostics
        // note, full-width Submit.
        let mut fb_overlay = div().absolute().inset_0().size_full();
        if self.feedback_open {
            let can_submit = self
                .feedback_input
                .as_ref()
                .map(|i| !i.read(cx).value().trim().is_empty())
                .unwrap_or(false)
                && !self.feedback_sending;
            let mut pills = h_flex().w_full().px(px(20.)).gap_1p5().flex_wrap();
            for (i, (_v, label, _lead)) in Self::FEEDBACK_CATEGORIES.iter().enumerate()
            {
                let sel = self.feedback_category == Some(i);
                pills = pills.child(
                    div()
                        .id(ElementId::Name(format!("fb-cat-{i}").into()))
                        .cursor_pointer()
                        .rounded(px(999.))
                        .px(px(14.))
                        .py(px(5.))
                        .text_size(px(13.))
                        .border_1()
                        .border_color(theme.border)
                        .text_color(if sel {
                            theme.foreground
                        } else {
                            theme.muted_foreground
                        })
                        .bg(if sel {
                            theme.secondary
                        } else {
                            gpui::transparent_black()
                        })
                        .child(format!("{} {}", if sel { "\u{2212}" } else { "+" }, label))
                        .on_click(cx.listener(move |this, _e, _w, cx| {
                            this.feedback_category =
                                if this.feedback_category == Some(i) {
                                    None
                                } else {
                                    Some(i)
                                };
                            cx.notify();
                        })),
                );
            }
            let mut card = v_flex()
                .w(px(576.))
                .bg(popover_bg)
                .border_1()
                .border_color(theme.border)
                .rounded(px(14.))
                .shadow_md()
                .overflow_hidden()
                .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _w, cx| {
                    if ev.keystroke.key == "escape" && !this.feedback_sending {
                        this.feedback_open = false;
                        cx.stop_propagation();
                        cx.notify();
                    }
                }))
                .child(
                    h_flex()
                        .w_full()
                        .px(px(20.))
                        .pt(px(20.))
                        .pb(px(12.))
                        .items_center()
                        .child(
                            div()
                                .text_size(px(20.))
                                .font_weight(FontWeight::MEDIUM)
                                .flex_1()
                                .child("Share feedback"),
                        )
                        .child(
                            div()
                                .id("fb-close")
                                .cursor_pointer()
                                .text_color(theme.muted_foreground)
                                .hover(|d| d.text_color(theme.foreground))
                                .child(svg_icon(icons::X).size(px(16.)))
                                .on_click(cx.listener(|this, _e, _w, cx| {
                                    if !this.feedback_sending {
                                        this.feedback_open = false;
                                    }
                                    cx.notify();
                                })),
                        ),
                )
                .child(pills);
            if let Some(input) = self.feedback_input.clone() {
                card = card.child(
                    div()
                        .mx(px(20.))
                        .mt(px(12.))
                        .min_h(px(128.))
                        .border_1()
                        .border_color(theme.border)
                        .rounded(px(8.))
                        .px_2()
                        .py_1()
                        .child(Input::new(&input).appearance(false)),
                );
            }
            card = card
                .child(
                    div()
                        .px(px(20.))
                        .pt(px(12.))
                        .text_size(px(13.))
                        .text_color(theme.muted_foreground)
                        .child(
                            "Diagnostics include app version, OS, provider/model, \
                             modes, and session state — never prompts, messages, \
                             paths, or logs.",
                        ),
                )
                .child(
                    div()
                        .mx(px(20.))
                        .my(px(16.))
                        .h(px(36.))
                        .id("fb-submit")
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(8.))
                        .bg(if can_submit {
                            theme.foreground
                        } else {
                            theme.muted
                        })
                        .text_color(theme.background)
                        .text_size(px(13.))
                        .cursor_pointer()
                        .child(if self.feedback_sending {
                            "Sending…"
                        } else {
                            "Submit"
                        })
                        .on_click(cx.listener(|this, _e, _w, cx| {
                            this.submit_feedback(cx)
                        })),
                );
            fb_overlay = fb_overlay.child(
                v_flex()
                    .size_full()
                    .items_center()
                    .justify_center()
                    .bg(rgba(0x00000080))
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|this, _ev, _w, cx| {
                            if !this.feedback_sending {
                                this.feedback_open = false;
                            }
                            cx.notify();
                        }),
                    )
                    .child(
                        div()
                            .on_mouse_down(
                                gpui::MouseButton::Left,
                                cx.listener(|_this, _ev, _w, cx| {
                                    cx.stop_propagation()
                                }),
                            )
                            .child(card),
                    ),
            );
        }

        // Keybindings reference dialog (upstream ShortcutsDialog): search
        // field + filtered title/shortcut rows over a dimmed backdrop.
        let mut scd_overlay = div().absolute().inset_0().size_full();
        if self.sc_dialog_open {
            let query = self
                .sc_dialog_input
                .as_ref()
                .map(|i| i.read(cx).value().to_lowercase())
                .unwrap_or_default();
            let mut list = v_flex()
                .id("scd-list")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .px(px(12.))
                .pb(px(12.));
            let mut shown = 0;
            for (title, _desc, kbd, commands) in SHORTCUT_ROWS.iter() {
                if !query.trim().is_empty()
                    && !title.to_lowercase().contains(query.trim())
                {
                    continue;
                }
                shown += 1;
                let label = self.sc_row_label(commands, kbd);
                list = list.child(
                    h_flex()
                        .w_full()
                        .items_center()
                        .justify_between()
                        .gap_4()
                        .rounded_md()
                        .px_3()
                        .py_1p5()
                        .child(div().text_size(px(13.)).child(*title))
                        .child(if label.is_empty() {
                            div()
                                .text_size(px(11.))
                                .text_color(theme.muted_foreground)
                                .child("—")
                                .into_any_element()
                        } else {
                            sc_chip(label, &theme).into_any_element()
                        }),
                );
            }
            if shown == 0 {
                list = list.child(
                    div()
                        .px_3()
                        .py_2()
                        .text_size(px(13.))
                        .text_color(theme.muted_foreground)
                        .child(format!("No shortcuts match \u{201c}{query}\u{201d}")),
                );
            }
            let mut card = v_flex()
                .w(px(560.))
                .h(px(520.))
                .bg(popover_bg)
                .border_1()
                .border_color(theme.border)
                .rounded(px(14.))
                .shadow_md()
                .overflow_hidden()
                .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _w, cx| {
                    if ev.keystroke.key == "escape" {
                        this.sc_dialog_open = false;
                        cx.stop_propagation();
                        cx.notify();
                    }
                }))
                .child(
                    h_flex()
                        .w_full()
                        .px(px(16.))
                        .pt(px(16.))
                        .pb(px(8.))
                        .items_center()
                        .child(
                            div()
                                .text_size(px(15.))
                                .font_weight(FontWeight::MEDIUM)
                                .flex_1()
                                .child("Keybindings"),
                        )
                        .child(
                            div()
                                .id("scd-close")
                                .cursor_pointer()
                                .text_color(theme.muted_foreground)
                                .hover(|d| d.text_color(theme.foreground))
                                .child(svg_icon(icons::X).size(px(16.)))
                                .on_click(cx.listener(|this, _e, _w, cx| {
                                    this.sc_dialog_open = false;
                                    cx.notify();
                                })),
                        ),
                );
            if let Some(input) = self.sc_dialog_input.clone() {
                card = card.child(
                    div()
                        .mx(px(16.))
                        .mb(px(8.))
                        .h(px(36.))
                        .px_3()
                        .flex()
                        .items_center()
                        .border_1()
                        .border_color(theme.border)
                        .rounded(px(8.))
                        .child(Input::new(&input).appearance(false)),
                );
            }
            card = card.child(list);
            scd_overlay = scd_overlay.child(
                v_flex()
                    .size_full()
                    .items_center()
                    .justify_center()
                    .bg(rgba(0x00000080))
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|this, _ev, _w, cx| {
                            this.sc_dialog_open = false;
                            cx.notify();
                        }),
                    )
                    .child(
                        div()
                            .on_mouse_down(
                                gpui::MouseButton::Left,
                                cx.listener(|_this, _ev, _w, cx| {
                                    cx.stop_propagation()
                                }),
                            )
                            .child(card),
                    ),
            );
        }

        // App tour (upstream AppTour): dimmed workspace, accent ring on the
        // anchored control, step card beside it, dots + Back/Next, ✕/Escape.
        // Custom snooze date-time dialog (upstream SnoozeUntilDialog —
        // reuses the RenameDialog shape with a datetime entry).
        let mut snz_overlay = div().absolute().inset_0().size_full();
        if self.snooze_dlg_tid.is_some() {
            let can_save = self
                .snooze_dlg_input
                .as_ref()
                .and_then(|i| {
                    let raw = i.read(cx).value().trim().replace('T', " ");
                    chrono::NaiveDateTime::parse_from_str(&raw, "%Y-%m-%d %H:%M")
                        .ok()
                        .and_then(|n| n.and_local_timezone(chrono::Local).earliest())
                })
                .map(|d| d > chrono::Local::now())
                .unwrap_or(false);
            let mut dlg = v_flex()
                .w(px(448.))
                .bg(popover_bg)
                .border_1()
                .border_color(theme.border)
                .rounded(px(14.))
                .shadow_md()
                .overflow_hidden()
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(|_this, _ev, _w, cx| cx.stop_propagation()),
                )
                .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _w, cx| {
                    if ev.keystroke.key == "escape" {
                        this.close_snooze_dialog(cx);
                        cx.stop_propagation();
                    } else if ev.keystroke.key == "enter" {
                        this.commit_snooze_dialog(cx);
                        cx.stop_propagation();
                    }
                }))
                .child(
                    div()
                        .px(px(20.))
                        .pt(px(20.))
                        .pb(px(4.))
                        .text_size(px(20.))
                        .font_weight(FontWeight::MEDIUM)
                        .child("Snooze until"),
                )
                .child(
                    div()
                        .px(px(20.))
                        .pb(px(8.))
                        .text_size(px(12.))
                        .text_color(theme.muted_foreground)
                        .child("The chat returns with a reminder at this time."),
                );
            if let Some(inp) = self.snooze_dlg_input.clone() {
                dlg = dlg.child(
                    h_flex()
                        .mx(px(20.))
                        .my(px(8.))
                        .gap_2()
                        .items_center()
                        .child(div().flex_1().child(Input::new(&inp))),
                );
            }
            dlg = dlg.child(
                h_flex()
                    .w_full()
                    .px(px(20.))
                    .py(px(16.))
                    .gap_2()
                    .justify_end()
                    .child(
                        Button::new("snz-cancel")
                            .ghost()
                            .label("Cancel")
                            .on_click(cx.listener(|this, _ev, _w, cx| {
                                this.close_snooze_dialog(cx);
                            })),
                    )
                    .child(
                        Button::new("snz-save")
                            .primary()
                            .label("Snooze")
                            .on_click(cx.listener(move |this, _ev, _w, cx| {
                                if can_save {
                                    this.commit_snooze_dialog(cx);
                                }
                            })),
                    ),
            );
            snz_overlay = snz_overlay.child(
                v_flex()
                    .size_full()
                    .items_center()
                    .justify_center()
                    .bg(gpui::black().opacity(0.3))
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|this, _ev, _w, cx| {
                            this.close_snooze_dialog(cx);
                        }),
                    )
                    .child(dlg),
            );
        }

        let mut tour_overlay = div().absolute().inset_0().size_full();
        if let Some(step_idx) = self.tour_step.filter(|i| *i < TOUR_STEPS.len()) {
            let (place, title, desc, anchor) = TOUR_STEPS[step_idx];
            let last = step_idx + 1 == TOUR_STEPS.len();
            let mut dots = h_flex().gap(px(6.)).items_center();
            for i in 0..TOUR_STEPS.len() {
                dots = dots.child(
                    div()
                        .size(px(6.))
                        .rounded_full()
                        .bg(if i == step_idx {
                            theme.foreground
                        } else {
                            theme.muted_foreground
                        }),
                );
            }
            let mut card = v_flex()
                .w(px(300.))
                .bg(popover_bg)
                .border_1()
                .border_color(theme.border)
                .rounded(px(14.))
                .shadow_md()
                .p(px(16.))
                .gap_2()
                .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _w, cx| {
                    match ev.keystroke.key.as_str() {
                        "escape" => {
                            this.tour_step = None;
                            cx.stop_propagation();
                            cx.notify();
                        }
                        "arrowright" | "enter" => {
                            this.tour_step = Some(
                                (this.tour_step.unwrap_or(0) + 1).min(TOUR_STEPS.len() - 1),
                            );
                            cx.notify();
                        }
                        "arrowleft" => {
                            this.tour_step = Some(this.tour_step.unwrap_or(1).saturating_sub(1));
                            cx.notify();
                        }
                        _ => {}
                    }
                }))
                .child(
                    h_flex()
                        .w_full()
                        .items_center()
                        .child(
                            div()
                                .flex_1()
                                .text_size(px(11.))
                                .text_color(theme.muted_foreground)
                                .child(place),
                        )
                        .child(
                            div()
                                .id("tour-close")
                                .cursor_pointer()
                                .text_color(theme.muted_foreground)
                                .hover(|d| d.text_color(theme.foreground))
                                .child(svg_icon(icons::X).size(px(16.)))
                                .on_click(cx.listener(|this, _e, _w, cx| {
                                    this.tour_step = None;
                                    cx.notify();
                                })),
                        ),
                )
                .child(
                    div()
                        .text_size(px(15.))
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
                        .w_full()
                        .pt_2()
                        .items_center()
                        .child(dots)
                        .child(div().flex_1())
                        .when(step_idx > 0, |d| {
                            d.child(
                                div()
                                    .id("tour-back")
                                    .cursor_pointer()
                                    .h(px(28.))
                                    .px_3()
                                    .flex()
                                    .items_center()
                                    .rounded(px(8.))
                                    .mr_1()
                                    .text_size(px(13.))
                                    .hover(|h| h.bg(theme.secondary))
                                    .child("Back")
                                    .on_click(cx.listener(|this, _e, _w, cx| {
                                        this.tour_step =
                                            Some(this.tour_step.unwrap_or(1).saturating_sub(1));
                                        cx.notify();
                                    })),
                            )
                        })
                        .child(
                            div()
                                .id("tour-next")
                                .cursor_pointer()
                                .h(px(28.))
                                .px_3()
                                .flex()
                                .items_center()
                                .rounded(px(8.))
                                .bg(theme.foreground)
                                .text_color(theme.background)
                                .text_size(px(13.))
                                .child(if last { "Done" } else { "Next" })
                                .on_click(cx.listener(move |this, _e, _w, cx| {
                                    this.tour_step = if last {
                                        None
                                    } else {
                                        Some(step_idx + 1)
                                    };
                                    cx.notify();
                                })),
                        ),
                );
            let mut stage = v_flex()
                .size_full()
                .bg(rgba(0x00000066))
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(|this, _ev, _w, cx| {
                        this.tour_step = None;
                        cx.notify();
                    }),
                );
            if let Some((x0, y0, x1, y1)) = anchor {
                stage = stage
                    .child(
                        div()
                            .absolute()
                            .left(px(x0 - 4.))
                            .top(px(y0 - 4.))
                            .w(px(x1 - x0 + 8.))
                            .h(px(y1 - y0 + 8.))
                            .rounded(px(10.))
                            .border_2()
                            .border_color(theme.primary),
                    )
                    .child(
                        div()
                            .absolute()
                            .left(px(x1 + 14.))
                            .top(px(y0 - 4.))
                            .on_mouse_down(
                                gpui::MouseButton::Left,
                                cx.listener(|_this, _ev, _w, cx| cx.stop_propagation()),
                            )
                            .child(card),
                    );
            } else {
                stage = stage.items_center().justify_center().child(
                    div()
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(|_this, _ev, _w, cx| cx.stop_propagation()),
                        )
                        .child(card),
                );
            }
            tour_overlay = tour_overlay.child(stage);
        }

        // ---- git action dialog (upstream GitCommitDialog/GitCreatePrDialog):
        // centered card over a dim backdrop; Escape/backdrop close; the
        // commit variant also picks which stacked action to run.
        let mut git_overlay = div().absolute().inset_0().size_full();
        if let Some(gkind) = self.git_dialog {
            let mut card = v_flex()
                .w(px(420.))
                .bg(popover_bg)
                .border_1()
                .border_color(theme.border)
                .rounded(px(14.))
                .shadow_md()
                .overflow_hidden()
                .p(px(16.))
                .gap_2()
                .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _w, cx| {
                    if ev.keystroke.key == "escape" {
                        this.close_git_dialog(cx);
                        cx.stop_propagation();
                    }
                }))
                .child(
                    h_flex()
                        .w_full()
                        .items_center()
                        .child(
                            div()
                                .flex_1()
                                .text_size(px(15.))
                                .font_weight(FontWeight::MEDIUM)
                                .child(match gkind {
                                    GitDialog::Commit => "Commit changes",
                                    GitDialog::Pr => "Create pull request",
                                }),
                        )
                        .child(
                            div()
                                .id("gitdlg-close")
                                .cursor_pointer()
                                .text_color(theme.muted_foreground)
                                .hover(|d| d.text_color(theme.foreground))
                                .child(svg_icon(icons::X).size(px(16.)))
                                .on_click(cx.listener(|this, _e, _w, cx| {
                                    this.close_git_dialog(cx);
                                })),
                        ),
                );
            let branch = get(&self.git_status.clone().unwrap_or(Value::Null), "branch")
                .as_str()
                .unwrap_or("")
                .to_string();
            if !branch.is_empty() {
                card = card.child(
                    h_flex()
                        .items_center()
                        .gap_1()
                        .child(
                            svg_icon(icons::S_BRANCH_SIMPLE)
                                .size(px(12.))
                                .text_color(theme.muted_foreground),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(theme.muted_foreground)
                                .child(branch),
                        ),
                );
            }
            if gkind == GitDialog::Commit {
                if let Some(input) = self.git_msg_input.clone() {
                    card = card.child(
                        div()
                            .w_full()
                            .h(px(36.))
                            .px_3()
                            .flex()
                            .items_center()
                            .rounded(px(8.))
                            .border_1()
                            .border_color(theme.border)
                            .child(Input::new(&input).appearance(false)),
                    );
                }
                let actions: [(&str, &str, &str); 4] = [
                    ("commit", "Commit", "Commit staged changes on this branch."),
                    ("commit_push", "Commit & push", "Commit, then push to the upstream branch."),
                    ("commit_new_branch", "Commit on new branch", "Create a feature branch, then commit there."),
                    ("create_pr", "Create PR", "Commit, push, and open a pull request."),
                ];
                for (id, label, desc) in actions {
                    let sel = self.git_commit_action == id;
                    let act = id.to_string();
                    card = card.child(
                        h_flex()
                            .id(ElementId::Name(format!("gitact-{id}").into()))
                            .w_full()
                            .items_center()
                            .gap_2()
                            .rounded(px(8.))
                            .px_2()
                            .py_1p5()
                            .cursor_pointer()
                            .hover(|d| d.bg(theme.secondary))
                            .on_click(cx.listener(move |this, _e, _w, cx| {
                                this.git_commit_action = act.clone();
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .w(px(14.))
                                    .h(px(14.))
                                    .rounded_full()
                                    .border_1()
                                    .border_color(if sel { theme.primary } else { theme.border })
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .when(sel, |d| {
                                        d.child(
                                            div().w(px(8.)).h(px(8.)).rounded_full().bg(theme.primary),
                                        )
                                    }),
                            )
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .child(
                                        div().text_size(px(13.)).child(label),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(theme.muted_foreground)
                                            .child(desc),
                                    ),
                            ),
                    );
                }
            } else {
                if let Some(input) = self.git_pr_title_input.clone() {
                    card = card.child(
                        div()
                            .w_full()
                            .h(px(36.))
                            .px_3()
                            .flex()
                            .items_center()
                            .rounded(px(8.))
                            .border_1()
                            .border_color(theme.border)
                            .child(Input::new(&input).appearance(false)),
                    );
                }
                if let Some(input) = self.git_pr_body_input.clone() {
                    card = card.child(
                        div()
                            .w_full()
                            .h(px(80.))
                            .px_3()
                            .py_2()
                            .rounded(px(8.))
                            .border_1()
                            .border_color(theme.border)
                            .child(Input::new(&input).appearance(false)),
                    );
                }
                let draft = self.git_pr_draft;
                card = card.child(
                    h_flex()
                        .id("gitdlg-draft")
                        .items_center()
                        .gap_2()
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _e, _w, cx| {
                            this.git_pr_draft = !this.git_pr_draft;
                            cx.notify();
                        }))
                        .child(
                            div()
                                .w(px(14.))
                                .h(px(14.))
                                .rounded(px(4.))
                                .border_1()
                                .border_color(if draft { theme.primary } else { theme.border })
                                .flex()
                                .items_center()
                                .justify_center()
                                .when(draft, |d| {
                                    d.child(
                                        svg_icon(icons::CHECK)
                                            .size(px(10.))
                                            .text_color(theme.primary),
                                    )
                                }),
                        )
                        .child(div().text_size(px(13.)).child("Create as draft")),
                );
            }
            let run_action = if gkind == GitDialog::Commit {
                self.git_commit_action.clone()
            } else {
                "create_pr".to_string()
            };
            let run_label: &str = match run_action.as_str() {
                "commit_push" => "Commit & push",
                "commit_new_branch" => "Commit on new branch",
                "create_pr" => "Create PR",
                _ => "Commit",
            };
            card = card.child(
                h_flex()
                    .w_full()
                    .justify_end()
                    .gap_2()
                    .pt_1()
                    .child(
                        div()
                            .id("gitdlg-cancel")
                            .cursor_pointer()
                            .rounded_md()
                            .px_3()
                            .py_1p5()
                            .text_size(px(13.))
                            .hover(|d| d.bg(theme.secondary))
                            .on_click(cx.listener(|this, _e, _w, cx| {
                                this.close_git_dialog(cx);
                            }))
                            .child("Cancel"),
                    )
                    .child(
                        div()
                            .id("gitdlg-run")
                            .cursor_pointer()
                            .rounded_md()
                            .px_3()
                            .py_1p5()
                            .text_size(px(13.))
                            .text_color(theme.primary_foreground)
                            .bg(theme.primary)
                            .when(self.git_action_running, |d| d.opacity(0.6))
                            .on_click(cx.listener(move |this, _e, _w, cx| {
                                let a = run_action.clone();
                                this.run_git_action(&a, cx);
                            }))
                            .child(if self.git_action_running {
                                "Running…"
                            } else {
                                run_label
                            }),
                    ),
            );
            git_overlay = git_overlay
                .child(
                    div()
                        .absolute()
                        .inset_0()
                        .size_full()
                        .bg(rgba(0x00000080))
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(|this, _e, _w, cx| {
                                this.close_git_dialog(cx);
                            }),
                        ),
                )
                .child(
                    div()
                        .absolute()
                        .inset_0()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .on_mouse_down(
                                    gpui::MouseButton::Left,
                                    |_e, _w, cx| cx.stop_propagation(),
                                )
                                .child(card),
                        ),
                );
        }

        // Automation create/edit dialog (upstream automation form).
        let mut autom_overlay = div().absolute().inset_0().size_full();
        if let Some(form) = self.autom_dialog.as_ref() {
            let editing = form.edit_id.is_some();
            let name_in = form.name.clone();
            let prompt_in = form.prompt.clone();
            let time_in = form.time.clone();
            let extra_in = form.extra.clone();
            let kind = form.kind;
            let mut card = v_flex()
                .w(px(440.))
                .bg(popover_bg)
                .border_1()
                .border_color(theme.border)
                .rounded(px(14.))
                .shadow_md()
                .p(px(16.))
                .gap_2()
                .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _w, cx| {
                    if ev.keystroke.key == "escape" {
                        this.close_autom_dialog(cx);
                        cx.stop_propagation();
                    }
                }))
                .child(
                    div()
                        .text_size(px(15.))
                        .font_weight(FontWeight::MEDIUM)
                        .child(if editing {
                            "Edit automation"
                        } else {
                            "New automation"
                        }),
                )
                .child(
                    div()
                        .w_full()
                        .h(px(34.))
                        .px_3()
                        .flex()
                        .items_center()
                        .rounded(px(8.))
                        .border_1()
                        .border_color(theme.border)
                        .child(Input::new(&name_in).appearance(false)),
                )
                .child(
                    div()
                        .w_full()
                        .h(px(56.))
                        .px_3()
                        .py_2()
                        .rounded(px(8.))
                        .border_1()
                        .border_color(theme.border)
                        .child(Input::new(&prompt_in).appearance(false)),
                );
            // Project picker (radio list, upstream AutomationHeadingFields).
            let projects = autom_projects(self.snapshot.as_ref().unwrap_or(&Value::Null));
            card = card.child(
                div()
                    .text_size(px(11.))
                    .text_color(theme.muted_foreground)
                    .child("Project"),
            );
            for (i, p) in projects.iter().enumerate().take(6) {
                let sel = form.proj == i;
                card = card.child(
                    h_flex()
                        .id(ElementId::Name(format!("autproj-{i}").into()))
                        .items_center()
                        .gap_2()
                        .rounded(px(6.))
                        .px_2()
                        .py_1()
                        .cursor_pointer()
                        .hover(|d| d.bg(theme.secondary))
                        .on_click(cx.listener(move |this, _e, _w, cx| {
                            if let Some(f) = this.autom_dialog.as_mut() {
                                f.proj = i;
                            }
                            cx.notify();
                        }))
                        .child(
                            div()
                                .w(px(12.))
                                .h(px(12.))
                                .rounded_full()
                                .border_1()
                                .border_color(if sel { theme.primary } else { theme.border })
                                .flex()
                                .items_center()
                                .justify_center()
                                .when(sel, |d| {
                                    d.child(
                                        div()
                                            .w(px(6.))
                                            .h(px(6.))
                                            .rounded_full()
                                            .bg(theme.primary),
                                    )
                                }),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .overflow_hidden()
                                .child(str_at(p, "title").to_string()),
                        ),
                );
            }
            // Schedule kind chips.
            card = card.child(
                div()
                    .text_size(px(11.))
                    .text_color(theme.muted_foreground)
                    .child("Schedule"),
            );
            let mut chip_row = h_flex().w_full().gap_1();
            for (i, label) in AUTOM_KINDS.iter().enumerate() {
                let sel = kind == i;
                chip_row = chip_row.child(
                    div()
                        .id(ElementId::Name(format!("autkind-{i}").into()))
                        .h(px(24.))
                        .flex_1()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(6.))
                        .border_1()
                        .border_color(if sel { theme.primary } else { theme.border })
                        .cursor_pointer()
                        .when(sel, |d| d.bg(theme.primary.opacity(0.12)))
                        .on_click(cx.listener(move |this, _e, _w, cx| {
                            if let Some(f) = this.autom_dialog.as_mut() {
                                f.kind = i;
                            }
                            cx.notify();
                        }))
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(if sel { theme.primary } else { theme.foreground })
                                .child(*label),
                        ),
                );
            }
            card = card.child(chip_row);
            // Per-kind fields.
            if matches!(kind, 2 | 3 | 4) {
                card = card.child(
                    div()
                        .w(px(140.))
                        .h(px(32.))
                        .px_3()
                        .flex()
                        .items_center()
                        .rounded(px(8.))
                        .border_1()
                        .border_color(theme.border)
                        .child(Input::new(&time_in).appearance(false)),
                );
            }
            if kind == 4 {
                let mut day_row = h_flex().gap_1();
                for (i, d) in AUTOM_DAYS.iter().enumerate() {
                    let sel = form.day == i;
                    day_row = day_row.child(
                        div()
                            .id(ElementId::Name(format!("autday-{i}").into()))
                            .w(px(34.))
                            .h(px(24.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(6.))
                            .border_1()
                            .border_color(if sel { theme.primary } else { theme.border })
                            .cursor_pointer()
                            .when(sel, |d| d.bg(theme.primary.opacity(0.12)))
                            .on_click(cx.listener(move |this, _e, _w, cx| {
                                if let Some(f) = this.autom_dialog.as_mut() {
                                    f.day = i;
                                }
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(if sel {
                                        theme.primary
                                    } else {
                                        theme.foreground
                                    })
                                    .child(*d),
                            ),
                    );
                }
                card = card.child(day_row);
            }
            if matches!(kind, 0 | 1 | 5) {
                card = card.child(
                    div()
                        .w_full()
                        .h(px(32.))
                        .px_3()
                        .flex()
                        .items_center()
                        .rounded(px(8.))
                        .border_1()
                        .border_color(theme.border)
                        .child(Input::new(&extra_in).appearance(false)),
                );
            }
            // Enabled toggle row.
            let en = form.enabled;
            card = card.child(
                h_flex()
                    .id("autmenabled")
                    .items_center()
                    .gap_2()
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _e, _w, cx| {
                        if let Some(f) = this.autom_dialog.as_mut() {
                            f.enabled = !f.enabled;
                        }
                        cx.notify();
                    }))
                    .child(
                        div()
                            .w(px(14.))
                            .h(px(14.))
                            .rounded(px(3.))
                            .border_1()
                            .border_color(if en { theme.primary } else { theme.border })
                            .when(en, |d| d.bg(theme.primary))
                            .flex()
                            .items_center()
                            .justify_center()
                            .when(en, |d| {
                                d.child(div().text_size(px(9.)).text_color(theme.primary_foreground).child("✓"))
                            }),
                    )
                    .child(div().text_size(px(12.)).child("Enabled")),
            );
            card = card.child(
                h_flex()
                    .w_full()
                    .justify_end()
                    .gap_2()
                    .mt_1()
                    .child(
                        div()
                            .id("autom-cancel")
                            .cursor_pointer()
                            .rounded_md()
                            .px_3()
                            .py_1p5()
                            .text_size(px(13.))
                            .hover(|d| d.bg(theme.secondary))
                            .on_click(cx.listener(|this, _e, _w, cx| {
                                this.close_autom_dialog(cx);
                            }))
                            .child("Cancel"),
                    )
                    .child(
                        div()
                            .id("autom-save")
                            .cursor_pointer()
                            .rounded_md()
                            .px_3()
                            .py_1p5()
                            .text_size(px(13.))
                            .text_color(theme.primary_foreground)
                            .bg(theme.primary)
                            .on_click(cx.listener(|this, _e, _w, cx| {
                                this.submit_autom_dialog(cx);
                            }))
                            .child(if editing { "Save" } else { "Create automation" }),
                    ),
            );
            autom_overlay = autom_overlay
                .child(
                    div()
                        .absolute()
                        .inset_0()
                        .size_full()
                        .bg(rgba(0x00000080))
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(|this, _e, _w, cx| {
                                this.close_autom_dialog(cx);
                            }),
                        ),
                )
                .child(
                    div()
                        .absolute()
                        .inset_0()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .on_mouse_down(
                                    gpui::MouseButton::Left,
                                    |_e, _w, cx| cx.stop_propagation(),
                                )
                                .child(card),
                        ),
                );
        }

        // PullRequestThreadDialog (upstream checkout mode): reference input,
        // resolved PR card, Local/Worktree confirm buttons.
        // Upstream WhatsNewPopoutCard: bottom-left "New: vX" card after a
        // covered upgrade. Body opens the notes dialog; the ✕ marks seen.
        let mut wn_pop_overlay = div().absolute().inset_0().size_full();
        if self.wn_popout && self.wn_view == 0 {
            if let Some(entry) = whats_new_entries()
                .iter()
                .find(|e| str_at(e, "version") == APP_VERSION)
            {
                let f0 = arr(entry, "features").first();
                let title = f0
                    .map(|f| str_at(f, "title").to_string())
                    .unwrap_or_else(|| format!("What's new in v{APP_VERSION}"));
                let desc = f0.map(|f| str_at(f, "description").to_string());
                wn_pop_overlay = wn_pop_overlay.child(
                    v_flex()
                        .absolute()
                        .left(px(12.))
                        .bottom(px(12.))
                        .w(px(256.))
                        .rounded(px(12.))
                        .border_1()
                        .border_color(theme.border)
                        .bg(theme.popover)
                        .shadow_lg()
                        .overflow_hidden()
                        .child(
                            h_flex()
                                .h(px(96.))
                                .w_full()
                                .items_center()
                                .justify_center()
                                .bg(theme.muted)
                                .child(
                                    div()
                                        .text_size(px(28.))
                                        .text_color(theme.muted_foreground)
                                        .child("Y"),
                                ),
                        )
                        .child(
                            v_flex()
                                .px(px(16.))
                                .pt(px(10.))
                                .pb(px(16.))
                                .gap_1()
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(theme.accent)
                                        .child(format!("New v{APP_VERSION}")),
                                )
                                .child(
                                    div()
                                        .text_base()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(title),
                                )
                                .when_some(desc, |d, s| {
                                    d.child(
                                        div()
                                            .text_sm()
                                            .text_color(theme.muted_foreground)
                                            .child(s),
                                    )
                                })
                                .child(
                                    div()
                                        .mt(px(8.))
                                        .text_sm()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(theme.muted_foreground)
                                        .child("Find out what's new →"),
                                ),
                        )
                        .child(
                            div()
                                .id("wn-popout-open")
                                .absolute()
                                .inset_0()
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _e, _w, cx| {
                                    this.wn_view = 1;
                                    cx.notify();
                                })),
                        )
                        .child(
                            div()
                                .id("wn-popout-dismiss")
                                .absolute()
                                .top(px(10.))
                                .right(px(10.))
                                .size(px(28.))
                                .rounded_full()
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .hover(|h| h.bg(theme.secondary))
                                .on_click(cx.listener(|this, _e, _w, cx| {
                                    this.mark_whats_new_seen(cx);
                                }))
                                .child(
                                    svg_icon(icons::X)
                                        .size(px(14.))
                                        .text_color(theme.muted_foreground),
                                ),
                        ),
                );
            }
        }

        // Upstream WhatsNewDialog: "What's new?" feature stack for the current
        // release, with a "Complete changelog" accordion as the second view.
        let mut wn_dialog_overlay = div().absolute().inset_0().size_full();
        if self.wn_view != 0 {
            if let Some(entry) = whats_new_entries()
                .iter()
                .find(|e| str_at(e, "version") == APP_VERSION)
                .cloned()
            {
                let feature_card = |f: &Value| {
                    let details = str_at(f, "details");
                    v_flex()
                        .gap(px(8.))
                        .child(
                            v_flex()
                                .gap(px(4.))
                                .child(
                                    div()
                                        .text_base()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(str_at(f, "title").to_string()),
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(theme.muted_foreground)
                                        .child(str_at(f, "description").to_string()),
                                ),
                        )
                        .when(!details.is_empty(), |d| {
                            d.child(
                                div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child(details.to_string()),
                            )
                        })
                };
                let mut card = v_flex()
                    .w(px(512.))
                    .rounded(px(12.))
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.popover)
                    .shadow_lg();
                // Header: current view = logo + title + vX · date; changelog
                // view = back button + "Complete changelog".
                if self.wn_view == 1 {
                    card = card.child(
                        h_flex()
                            .p(px(16.))
                            .pr(px(48.))
                            .gap_3()
                            .items_center()
                            .child(
                                div()
                                    .text_size(px(24.))
                                    .text_color(theme.foreground)
                                    .child("Y"),
                            )
                            .child(
                                v_flex()
                                    .child(
                                        div()
                                            .text_base()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child("What's new?"),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(theme.muted_foreground)
                                            .child(format!(
                                                "v{APP_VERSION} · {}",
                                                str_at(&entry, "date")
                                            )),
                                    ),
                            ),
                    );
                    let mut body = v_flex().px(px(16.)).py(px(12.)).gap(px(32.));
                    for f in arr(&entry, "features") {
                        body = body.child(feature_card(f));
                    }
                    card = card.child(
                        div()
                            .max_h(px(520.))
                            .overflow_y_scrollbar()
                            .child(body),
                    );
                    card = card.child(
                        h_flex()
                            .p(px(12.))
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .id("wn-view-changelog")
                                    .px_2()
                                    .h(px(28.))
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .rounded(px(6.))
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .cursor_pointer()
                                    .hover(|h| h.bg(theme.secondary))
                                    .on_click(cx.listener(|this, _e, _w, cx| {
                                        this.wn_view = 2;
                                        this.wn_changelog_open.clear();
                                        this.wn_changelog_open
                                            .insert(APP_VERSION.to_string());
                                        cx.notify();
                                    }))
                                    .child("View changelog →"),
                            )
                            .child(
                                div()
                                    .id("wn-got-it")
                                    .h(px(32.))
                                    .px_3()
                                    .flex()
                                    .items_center()
                                    .rounded(px(8.))
                                    .bg(theme.foreground)
                                    .text_color(theme.background)
                                    .text_sm()
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _e, _w, cx| {
                                        this.mark_whats_new_seen(cx);
                                    }))
                                    .child("Got it"),
                            ),
                    );
                } else {
                    card = card.child(
                        h_flex()
                            .p(px(16.))
                            .gap_3()
                            .items_center()
                            .child(
                                div()
                                    .id("wn-back")
                                    .size(px(28.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(6.))
                                    .cursor_pointer()
                                    .hover(|h| h.bg(theme.secondary))
                                    .on_click(cx.listener(|this, _e, _w, cx| {
                                        this.wn_view = 1;
                                        cx.notify();
                                    }))
                                    .child(
                                        svg_icon(icons::CHEVRON_LEFT)
                                            .size(px(14.))
                                            .text_color(theme.foreground),
                                    ),
                            )
                            .child(
                                v_flex()
                                    .child(
                                        div()
                                            .text_base()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child("Complete changelog"),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(theme.muted_foreground)
                                            .child("Every curated release, newest first"),
                                    ),
                            ),
                    );
                    let sorted = whats_new_sorted();
                    let mut acc = v_flex().px(px(16.)).py(px(8.));
                    let n_entries = sorted.len();
                    for (i, e) in sorted.iter().enumerate() {
                        let ver = str_at(e, "version").to_string();
                        let n_feat = arr(e, "features").len();
                        let open = self.wn_changelog_open.contains(&ver);
                        let ver_c = ver.clone();
                        let mut row = v_flex()
                            .w_full()
                            .when(i + 1 < n_entries, |d| {
                                d.border_b_1().border_color(theme.border)
                            })
                            .child(
                                h_flex()
                                    .id(SharedString::from(format!("wn-acc-{i}")))
                                    .w_full()
                                    .py(px(12.))
                                    .gap_3()
                                    .items_center()
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |this, _e, _w, cx| {
                                        if !this.wn_changelog_open.remove(&ver_c) {
                                            this.wn_changelog_open.insert(ver_c.clone());
                                        }
                                        cx.notify();
                                    }))
                                    .child(
                                        svg_icon(if open {
                                            icons::CHEVRON_DOWN
                                        } else {
                                            icons::CHEVRON_RIGHT
                                        })
                                        .size(px(12.))
                                        .text_color(theme.muted_foreground),
                                    )
                                    .child(
                                        h_flex()
                                            .flex_1()
                                            .items_baseline()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(theme.muted_foreground)
                                                    .child(str_at(e, "date").to_string()),
                                            )
                                            .child(
                                                div()
                                                    .text_base()
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .child(format!("Version {ver}")),
                                            )
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(theme.muted_foreground)
                                                    .child(format!(
                                                        "({n_feat} update{})",
                                                        if n_feat == 1 { "" } else { "s" }
                                                    )),
                                            ),
                                    ),
                            );
                        if open {
                            let mut feats = v_flex()
                                .pb(px(16.))
                                .pl(px(24.))
                                .pr(px(4.))
                                .gap(px(24.));
                            for f in arr(e, "features") {
                                feats = feats.child(feature_card(f));
                            }
                            row = row.child(feats);
                        }
                        acc = acc.child(row);
                    }
                    card = card.child(
                        div()
                            .max_h(px(560.))
                            .overflow_y_scrollbar()
                            .child(acc),
                    );
                }
                wn_dialog_overlay = wn_dialog_overlay.child(
                    v_flex()
                        .size_full()
                        .items_center()
                        .justify_center()
                        .bg(rgba(0x00000080))
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(|this, _ev, _w, cx| {
                                this.mark_whats_new_seen(cx);
                            }),
                        )
                        .child(
                            div()
                                .on_mouse_down(
                                    gpui::MouseButton::Left,
                                    |_e, _w, cx| cx.stop_propagation(),
                                )
                                .child(card),
                        ),
                );
            }
        }

        // Theme-pack swatch editor (upstream ThemePackEditor popover): a
        // small dialog with one hex input, Reset (clears the override) and
        // Apply (writes settings.appearance.themePacks.<variant>.<token>).
        let mut tp_overlay = div().absolute().inset_0().size_full();
        if let Some((dark, key)) = self.tp_edit.clone() {
            if let Some(input) = self.tp_input.clone() {
                let is_font = key == "fontUi" || key == "fontCode";
                let label = match key.as_str() {
                    "accent" => "Accent",
                    "background" => "Background",
                    "fontUi" => "UI font",
                    "fontCode" => "Code font",
                    _ => "Foreground",
                };
                let variant = if dark { "Dark theme" } else { "Light theme" };
                let cur = input.read(cx).value().to_string();
                let norm = if cur.trim().starts_with('#') {
                    cur.trim().to_string()
                } else {
                    format!("#{}", cur.trim())
                };
                // Font fields accept any family name; empty resets to the
                // default (upstream FontInput).
                let valid = is_font
                    || (norm.len() == 7
                        && norm[1..].chars().all(|c| c.is_ascii_hexdigit()));
                let swatch_hex = if valid {
                    u32::from_str_radix(&norm[1..], 16).unwrap_or(0)
                } else {
                    0
                };
                let card = v_flex()
                    .w(px(360.))
                    .rounded(px(12.))
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.popover)
                    .shadow_lg()
                    .child(
                        h_flex()
                            .p(px(16.))
                            .gap_3()
                            .items_center()
                            .when(!is_font, |d| {
                                d.child(
                                    div()
                                        .size(px(28.))
                                        .rounded(px(8.))
                                        .border_1()
                                        .border_color(theme.border)
                                        .when(valid, |d| d.bg(rgb(swatch_hex))),
                                )
                            })
                            .child(
                                v_flex()
                                    .child(
                                        div()
                                            .text_base()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child(label.to_string()),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(theme.muted_foreground)
                                            .child(variant.to_string()),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .mx(px(16.))
                            .mb(px(12.))
                            .h(px(32.))
                            .border_1()
                            .border_color(theme.border)
                            .rounded(px(8.))
                            .px_2()
                            .child(Input::new(&input).appearance(false)),
                    )
                    .child(
                        h_flex()
                            .p(px(12.))
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .id("tp-reset")
                                    .px_2()
                                    .h(px(28.))
                                    .flex()
                                    .items_center()
                                    .rounded(px(6.))
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .cursor_pointer()
                                    .hover(|h| h.bg(theme.secondary))
                                    .on_click(cx.listener(|this, _e, _w, cx| {
                                        this.commit_theme_pack_edit(true, cx);
                                    }))
                                    .child("Reset"),
                            )
                            .child(
                                div()
                                    .id("tp-apply")
                                    .h(px(32.))
                                    .px_3()
                                    .flex()
                                    .items_center()
                                    .rounded(px(8.))
                                    .bg(if valid {
                                        theme.foreground
                                    } else {
                                        theme.muted
                                    })
                                    .text_color(theme.background)
                                    .text_sm()
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _e, _w, cx| {
                                        this.commit_theme_pack_edit(false, cx);
                                    }))
                                    .child("Apply"),
                            ),
                    );
                tp_overlay = tp_overlay.child(
                    v_flex()
                        .size_full()
                        .items_center()
                        .justify_center()
                        .bg(rgba(0x00000080))
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(|this, _ev, _w, cx| {
                                this.tp_edit = None;
                                this.tp_input = None;
                                cx.notify();
                            }),
                        )
                        .child(
                            div()
                                .on_mouse_down(
                                    gpui::MouseButton::Left,
                                    cx.listener(|_this, _ev, _w, cx| {
                                        cx.stop_propagation()
                                    }),
                                )
                                .child(card),
                        ),
                );
            }
        }

        if let Some(dark) = self.tp_import {
            if let Some(input) = self.tp_input.clone() {
                let card = v_flex()
                    .w(px(480.))
                    .rounded(px(12.))
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.popover)
                    .shadow_lg()
                    .child(
                        div()
                            .p(px(16.))
                            .text_base()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(format!(
                                "Import {} theme pack",
                                if dark { "dark" } else { "light" }
                            )),
                    )
                    .child(
                        div()
                            .mx(px(16.))
                            .mb(px(12.))
                            .h(px(32.))
                            .border_1()
                            .border_color(theme.border)
                            .rounded(px(8.))
                            .px_2()
                            .child(Input::new(&input).appearance(false)),
                    )
                    .child(
                        h_flex()
                            .p(px(12.))
                            .justify_end()
                            .items_center()
                            .child(
                                div()
                                    .id("tp-import-apply")
                                    .h(px(32.))
                                    .px_3()
                                    .flex()
                                    .items_center()
                                    .rounded(px(8.))
                                    .bg(theme.foreground)
                                    .text_color(theme.background)
                                    .text_sm()
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |this, _e, _w, cx| {
                                        this.commit_theme_pack_import(dark, cx);
                                    }))
                                    .child("Import"),
                            ),
                    );
                tp_overlay = tp_overlay.child(
                    v_flex()
                        .size_full()
                        .items_center()
                        .justify_center()
                        .bg(rgba(0x00000080))
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(|this, _ev, _w, cx| {
                                this.tp_import = None;
                                this.tp_input = None;
                                cx.notify();
                            }),
                        )
                        .child(
                            div()
                                .on_mouse_down(
                                    gpui::MouseButton::Left,
                                    cx.listener(|_this, _ev, _w, cx| {
                                        cx.stop_propagation()
                                    }),
                                )
                                .child(card),
                        ),
                );
            }
        }

        // ── Import projects dialog (upstream ProjectImportDialog +
        // ProjectImportPanel): provider chips, a scrollable per-project
        // checklist and a sequential import run with stop/retry. ─────────

        // Seed folder-path inputs for scanned rows whose stored directory
        // no longer exists (upstream ProjectImportFolderInput).
        if let Some(cat) = self
            .import_dlg
            .as_ref()
            .and_then(|d| d.catalog.clone())
        {
            let missing: Vec<(String, String)> = arr(&cat, "projects")
                .iter()
                .filter(|p| !p["directoryExists"].as_bool().unwrap_or(true))
                .map(|p| {
                    (
                        str_at(p, "key").to_string(),
                        str_at(p, "workspaceRoot").to_string(),
                    )
                })
                .collect();
            for (pk, root) in missing {
                let exists = self
                    .import_dlg
                    .as_ref()
                    .map(|d| d.root_inputs.contains_key(&pk))
                    .unwrap_or(false);
                if exists {
                    continue;
                }
                let inp = cx.new(|cx| {
                    InputState::new(window, cx)
                        .placeholder("Optional existing folder path")
                });
                inp.update(cx, |s, cx| s.set_value(root, window, cx));
                let pk2 = pk.clone();
                let inp2 = inp.clone();
                cx.subscribe_in(&inp, window, move |this: &mut Self, _s, ev, _w, cx| {
                    if let InputEvent::Change = ev {
                        let v = inp2.read(cx).value().to_string();
                        if let Some(d) = this.import_dlg.as_mut() {
                            d.roots.insert(pk2.clone(), v);
                        }
                        cx.notify();
                    }
                })
                .detach();
                if let Some(d) = self.import_dlg.as_mut() {
                    d.root_inputs.insert(pk, inp);
                }
            }
        }

        let mut imp_overlay = div().absolute().inset_0().size_full();
        if let Some(dlg) = self.import_dlg.as_ref() {
            let cat = dlg.catalog.clone().unwrap_or(Value::Null);
            let query = dlg.search.read(cx).value().to_string();
            let query = query.trim().to_lowercase();
            let projects = arr(&cat, "projects");
            let visible: Vec<Value> = projects
                .iter()
                .filter(|p| {
                    query.is_empty()
                        || format!(
                            "{} {}",
                            str_at(p, "title"),
                            str_at(p, "workspaceRoot")
                        )
                        .to_lowercase()
                        .contains(&query)
                })
                .cloned()
                .collect();
            let selectable_all: Vec<String> = projects
                .iter()
                .flat_map(|p| import_selectable_keys(p, dlg.include_archived))
                .filter(|k| !matches!(dlg.outcomes.get(k), Some(None)))
                .collect();
            let all_selected = !selectable_all.is_empty()
                && selectable_all.iter().all(|k| dlg.selected.contains(k));
            // item key → display title, reused by the failures list
            let mut titles: std::collections::HashMap<String, String> =
                std::collections::HashMap::new();
            for p in projects.iter() {
                let pk = str_at(p, "key");
                let pt = str_at(p, "title");
                let threads = arr(p, "threads");
                if threads.is_empty() {
                    titles.insert(import_item_key(pk, None), pt.to_string());
                } else {
                    for t in threads {
                        let tt = str_at(t, "title");
                        titles.insert(
                            import_item_key(pk, Some(str_at(t, "key"))),
                            if tt.is_empty() {
                                format!("{pt} · Untitled conversation")
                            } else {
                                format!("{pt} · {tt}")
                            },
                        );
                    }
                }
            }
            // footer counts: conversations selected, projects touched
            let mut conv_n = 0usize;
            let mut proj_n = 0usize;
            for p in projects.iter() {
                let has_threads = !arr(p, "threads").is_empty();
                let mut p_sel = false;
                for k in import_selectable_keys(p, dlg.include_archived) {
                    if dlg.selected.contains(&k) {
                        p_sel = true;
                        if has_threads {
                            conv_n += 1;
                        }
                    }
                }
                if p_sel {
                    proj_n += 1;
                }
            }

            let cbox = |state: u8| {
                div()
                    .size(px(14.))
                    .flex_none()
                    .rounded(px(3.))
                    .border_1()
                    .border_color(if state > 0 { theme.accent } else { theme.border })
                    .bg(if state > 0 {
                        theme.accent
                    } else {
                        rgba(0x00000000).into()
                    })
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(9.))
                    .text_color(theme.accent_foreground)
                    .child(if state == 2 {
                        "\u{2713}"
                    } else if state == 1 {
                        "\u{2212}"
                    } else {
                        ""
                    })
            };

            let mut card = v_flex()
                .w(px(520.))
                .max_h(px(640.))
                .bg(theme.popover)
                .border_1()
                .border_color(theme.border)
                .rounded_lg()
                .shadow_lg()
                .overflow_hidden()
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(|_this, _ev, _w, cx| cx.stop_propagation()),
                )
                .child(
                    h_flex()
                        .px(px(20.))
                        .pt(px(20.))
                        .items_start()
                        .child(
                            v_flex()
                                .gap(px(4.))
                                .flex_1()
                                .child(
                                    div()
                                        .text_size(px(16.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child("Import projects"),
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(theme.muted_foreground)
                                        .child(
                                            "Continue your Codex and Claude Code \
                                             projects in Synara.",
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .id("imp-close")
                                .size(px(26.))
                                .flex_none()
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded_md()
                                .cursor_pointer()
                                .text_color(theme.muted_foreground)
                                .hover(|s| s.bg(theme.secondary))
                                .child(svg_icon(icons::X).size(px(12.)))
                                .on_click(cx.listener(|this, _ev, _w, cx| {
                                    if this
                                        .import_dlg
                                        .as_ref()
                                        .map(|d| !d.running)
                                        .unwrap_or(true)
                                    {
                                        this.import_dlg = None;
                                    }
                                    cx.notify();
                                })),
                        ),
                );

            let mut body = v_flex()
                .px(px(20.))
                .pt(px(14.))
                .pb(px(12.))
                .gap(px(12.));
            // provider chips + scan button
            let mut chips = h_flex().gap(px(6.)).items_center();
            for (pid, plabel) in [("codex", "Codex"), ("claudeAgent", "Claude Code")] {
                let on = dlg.providers.iter().any(|p| p == pid);
                let pid_s = pid.to_string();
                chips = chips.child(
                    h_flex()
                        .id(ElementId::Name(format!("imp-prov-{pid}").into()))
                        .h(px(30.))
                        .px(px(10.))
                        .gap(px(7.))
                        .items_center()
                        .rounded(px(8.))
                        .border_1()
                        .border_color(theme.border)
                        .when(on, |s| s.bg(theme.secondary))
                        .text_color(if on {
                            theme.foreground
                        } else {
                            theme.muted_foreground
                        })
                        .cursor_pointer()
                        .child(cbox(if on { 2 } else { 0 }))
                        .child(svg_icon(import_provider_icon(pid)).size(px(14.)))
                        .child(div().text_sm().child(plabel))
                        .on_click(cx.listener(move |this, _ev, _w, cx| {
                            if let Some(d) = this.import_dlg.as_mut() {
                                if d.providers.iter().any(|p| p == &pid_s) {
                                    d.providers.retain(|p| p != &pid_s);
                                } else {
                                    d.providers.push(pid_s.clone());
                                }
                                d.catalog = None;
                                d.outcomes.clear();
                                d.progress = None;
                            }
                            cx.notify();
                        })),
                );
            }
            chips = chips.child(div().flex_1());
            chips = chips.child(
                Button::new("imp-scan")
                    .ghost()
                    .label(if dlg.scanning {
                        "Finding projects\u{2026}"
                    } else if dlg.catalog.is_some() {
                        "Scan again"
                    } else {
                        "Find projects"
                    })
                    .on_click(cx.listener(|this, _ev, _w, cx| {
                        let ok = this
                            .import_dlg
                            .as_ref()
                            .map(|d| {
                                !d.running && !d.scanning && !d.providers.is_empty()
                            })
                            .unwrap_or(false);
                        if ok {
                            this.import_scan(cx);
                        }
                    })),
            );
            body = body.child(chips);
            body = body.child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(
                        "Projects keep their existing folders and conversations \
                         are copied into Synara.",
                    ),
            );
            if let Some(err) = dlg.error.clone() {
                body = body.child(div().text_sm().text_color(theme.danger).child(err));
            }
            for src in arr(&cat, "sources") {
                let serr = str_at(src, "error");
                if !serr.is_empty() {
                    let pname = if str_at(src, "provider") == "claudeAgent" {
                        "Claude Code"
                    } else {
                        "Codex"
                    };
                    body = body.child(
                        div()
                            .text_sm()
                            .text_color(theme.danger)
                            .child(format!("{pname}: {serr}")),
                    );
                }
            }

            if dlg.catalog.is_some() {
                let sa = selectable_all.clone();
                body = body.child(
                    h_flex()
                        .gap(px(10.))
                        .items_center()
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(140.))
                                .border_1()
                                .border_color(theme.border)
                                .rounded(px(6.))
                                .px_2()
                                .h(px(28.))
                                .child(Input::new(&dlg.search).appearance(false)),
                        )
                        .child(
                            h_flex()
                                .id("imp-archived")
                                .gap(px(6.))
                                .items_center()
                                .cursor_pointer()
                                .child(cbox(if dlg.include_archived { 2 } else { 0 }))
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(theme.muted_foreground)
                                        .child("Include archived"),
                                )
                                .on_click(cx.listener(|this, _ev, _w, cx| {
                                    if let Some(d) = this.import_dlg.as_mut() {
                                        d.include_archived = !d.include_archived;
                                    }
                                    cx.notify();
                                })),
                        )
                        .child(
                            Button::new("imp-selall")
                                .ghost()
                                .label(if all_selected {
                                    "Remove all"
                                } else {
                                    "Select all"
                                })
                                .on_click(cx.listener(move |this, _ev, _w, cx| {
                                    if let Some(d) = this.import_dlg.as_mut() {
                                        if all_selected {
                                            for k in &sa {
                                                d.selected.remove(k);
                                            }
                                        } else {
                                            for k in &sa {
                                                d.selected.insert(k.clone());
                                            }
                                        }
                                    }
                                    cx.notify();
                                })),
                        ),
                );

                // scrollable project checklist
                let mut list = v_flex()
                    .id("imp-list")
                    .min_h_0()
                    .max_h(px(300.))
                    .overflow_y_scroll()
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg();
                if visible.is_empty() {
                    list = list.child(
                        div()
                            .px_3()
                            .py_2()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("No projects match."),
                    );
                }
                for (pi, p) in visible.iter().enumerate() {
                    let pk = str_at(p, "key").to_string();
                    let pt = str_at(p, "title").to_string();
                    let root = str_at(p, "workspaceRoot").to_string();
                    let threads = arr(p, "threads");
                    let expanded = dlg.expanded.contains(&pk);
                    let available: Vec<String> =
                        import_selectable_keys(p, dlg.include_archived)
                            .into_iter()
                            .filter(|k| !matches!(dlg.outcomes.get(k), Some(None)))
                            .collect();
                    let sel_n =
                        available.iter().filter(|k| dlg.selected.contains(*k)).count();
                    let state = if available.is_empty() {
                        0u8
                    } else if sel_n == available.len() {
                        2
                    } else if sel_n > 0 {
                        1
                    } else {
                        0
                    };
                    let existing = !get(p, "existingProjectId").is_null();
                    let dir_ok = p["directoryExists"].as_bool().unwrap_or(true);
                    let nconv = if dlg.include_archived {
                        threads.len()
                    } else {
                        threads
                            .iter()
                            .filter(|t| !t["archived"].as_bool().unwrap_or(false))
                            .count()
                    };

                    let avail_c = available.clone();
                    let pk_t = pk.clone();
                    let mut row = h_flex()
                        .id(ElementId::Name(format!("imp-proj-{pi}").into()))
                        .px(px(12.))
                        .py(px(9.))
                        .gap(px(10.))
                        .items_center()
                        .cursor_pointer()
                        .hover(|s| s.bg(theme.secondary))
                        .child(
                            div()
                                .id(ElementId::Name(format!("imp-pcb-{pi}").into()))
                                .child(cbox(state))
                                .on_click(cx.listener(move |this, _ev, _w, cx| {
                                    cx.stop_propagation();
                                    if let Some(d) = this.import_dlg.as_mut() {
                                        let all = !avail_c.is_empty()
                                            && avail_c
                                                .iter()
                                                .all(|k| d.selected.contains(k));
                                        if all {
                                            for k in &avail_c {
                                                d.selected.remove(k);
                                            }
                                        } else {
                                            for k in &avail_c {
                                                d.selected.insert(k.clone());
                                            }
                                        }
                                    }
                                    cx.notify();
                                })),
                        )
                        .child(
                            v_flex()
                                .id(ElementId::Name(format!("imp-ptl-{pi}").into()))
                                .flex_1()
                                .gap(px(1.))
                                .child(div().text_sm().child(pt))
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .child(root),
                                )
                                .on_click(cx.listener(move |this, _ev, _w, cx| {
                                    if let Some(d) = this.import_dlg.as_mut() {
                                        if !d.expanded.remove(&pk_t) {
                                            d.expanded.insert(pk_t.clone());
                                        }
                                    }
                                    cx.notify();
                                })),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(if dir_ok {
                                    theme.muted_foreground
                                } else {
                                    theme.danger
                                })
                                .child(if !dir_ok {
                                    "Folder missing".to_string()
                                } else if existing {
                                    "Already in Synara".to_string()
                                } else {
                                    format!(
                                        "{nconv} conversation{}",
                                        if nconv == 1 { "" } else { "s" }
                                    )
                                }),
                        );
                    for pv in arr(p, "providers") {
                        if let Some(pvs) = pv.as_str() {
                            row = row.child(
                                svg_icon(import_provider_icon(pvs))
                                    .size(px(14.))
                                    .flex_none(),
                            );
                        }
                    }
                    row = row.child(
                        svg_icon(if expanded {
                            icons::CHEVRON_DOWN
                        } else {
                            icons::CHEVRON_RIGHT
                        })
                        .size(px(12.))
                        .flex_none(),
                    );

                    let mut card_el = v_flex().child(row);
                    if expanded {
                        let mut tl = v_flex()
                            .pb(px(8.))
                            .pl(px(36.))
                            .pr(px(12.))
                            .gap(px(2.));
                        for (ti, t) in threads.iter().enumerate() {
                            let tk = str_at(t, "key");
                            let key = import_item_key(&pk, Some(tk));
                            let imported =
                                t["alreadyImported"].as_bool().unwrap_or(false);
                            let archived = t["archived"].as_bool().unwrap_or(false);
                            let selectable =
                                !imported && (dlg.include_archived || !archived);
                            let done = matches!(dlg.outcomes.get(&key), Some(None));
                            let sel = dlg.selected.contains(&key);
                            let key_c = key.clone();
                            let mut trow = h_flex()
                                .id(ElementId::Name(
                                    format!("imp-th-{pi}-{ti}").into(),
                                ))
                                .py(px(4.))
                                .gap(px(8.))
                                .items_center();
                            if selectable && !done {
                                trow = trow.child(
                                    div()
                                        .id(ElementId::Name(
                                            format!("imp-tcb-{pi}-{ti}").into(),
                                        ))
                                        .child(cbox(if sel { 2 } else { 0 }))
                                        .on_click(cx.listener(
                                            move |this, _ev, _w, cx| {
                                                cx.stop_propagation();
                                                if let Some(d) =
                                                    this.import_dlg.as_mut()
                                                {
                                                    if !d.selected.remove(&key_c)
                                                    {
                                                        d.selected
                                                            .insert(key_c.clone());
                                                    }
                                                }
                                                cx.notify();
                                            },
                                        )),
                                );
                            } else {
                                trow = trow.child(cbox(2).opacity(0.4));
                            }
                            trow = trow.child(
                                svg_icon(import_provider_icon(str_at(t, "provider")))
                                    .size(px(14.))
                                    .flex_none(),
                            );
                            trow = trow.child(
                                div()
                                    .flex_1()
                                    .text_sm()
                                    .when(done || imported, |s| {
                                        s.text_color(theme.muted_foreground)
                                    })
                                    .child(if str_at(t, "title").is_empty() {
                                        "Untitled conversation".to_string()
                                    } else {
                                        str_at(t, "title").to_string()
                                    }),
                            );
                            let status = if imported {
                                "Already present".to_string()
                            } else if archived {
                                "Archived".to_string()
                            } else {
                                rel_time(str_at(t, "updatedAt"))
                            };
                            if !status.is_empty() {
                                trow = trow.child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .child(status),
                                );
                            }
                            tl = tl.child(trow);
                        }
                        if threads.is_empty() {
                            tl = tl.child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child("No conversations in this project yet."),
                            );
                        }
                        if !dir_ok {
                            if let Some(inp) = dlg.root_inputs.get(&pk) {
                                let inp3 = inp.clone();
                                tl = tl.child(
                                    h_flex()
                                        .gap(px(6.))
                                        .items_center()
                                        .pt(px(6.))
                                        .child(
                                            div()
                                                .flex_1()
                                                .border_1()
                                                .border_color(theme.border)
                                                .rounded(px(6.))
                                                .px_2()
                                                .h(px(28.))
                                                .child(
                                                    Input::new(inp).appearance(false),
                                                ),
                                        )
                                        .child(
                                            Button::new(ElementId::Name(
                                                format!("imp-browse-{pi}").into(),
                                            ))
                                            .ghost()
                                            .label("Browse\u{2026}")
                                            .on_click(cx.listener(
                                                move |_this, _ev, w, cx| {
                                                    let rx = cx.prompt_for_paths(
                                                        gpui::PathPromptOptions {
                                                            files: false,
                                                            directories: true,
                                                            multiple: false,
                                                            prompt: None,
                                                        },
                                                    );
                                                    let inp4 = inp3.clone();
                                                    cx.spawn_in(
                                                        w,
                                                        async move |_this, cx| {
                                                            if let Ok(Ok(Some(
                                                                paths,
                                                            ))) = rx.await
                                                            {
                                                                if let Some(dir) =
                                                                    paths.first()
                                                                {
                                                                    let p = dir
                                                                        .to_string_lossy()
                                                                        .into_owned();
                                                                    let _ = cx.update(
                                                                        |window, app| {
                                                                            let _ = inp4.update(
                                                                                app,
                                                                                |s, cx| {
                                                                                    s.set_value(
                                                                                        p, window,
                                                                                        cx,
                                                                                    );
                                                                                },
                                                                            );
                                                                        },
                                                                    );
                                                                }
                                                            }
                                                        },
                                                    )
                                                    .detach();
                                                },
                                            )),
                                        ),
                                );
                            }
                        }
                        card_el = card_el.child(tl);
                    }
                    list = list.child(card_el);
                }
                body = body.child(list);

                // footer: counts, progress, failures, action buttons
                let mut footer = v_flex().gap(px(8.)).pt(px(4.));
                footer = footer.child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(format!(
                            "{conv_n} conversation{} in {proj_n} project{} \
                             selected · History shows text messages; tool \
                             details may be incomplete.",
                            if conv_n == 1 { "" } else { "s" },
                            if proj_n == 1 { "" } else { "s" },
                        )),
                );
                if let Some((i, total, title)) = dlg.progress.clone() {
                    footer = footer.child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(if dlg.running {
                                format!("Importing {i} of {total}: {title}")
                            } else {
                                format!("\u{2713} {total} imported or already present.")
                            }),
                    );
                }
                let failed: Vec<String> = dlg
                    .outcomes
                    .iter()
                    .filter_map(|(k, v)| {
                        v.clone().map(|e| {
                            format!(
                                "{}: {e}",
                                titles.get(k).cloned().unwrap_or_default()
                            )
                        })
                    })
                    .collect();
                if !failed.is_empty() {
                    let mut fl = v_flex().gap(px(2.)).child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .child(format!(
                                "{} import{} failed:",
                                failed.len(),
                                if failed.len() == 1 { "" } else { "s" }
                            )),
                    );
                    for f in failed.iter().take(5) {
                        fl = fl.child(
                            div().text_xs().text_color(theme.danger).child(f.clone()),
                        );
                    }
                    footer = footer.child(fl);
                }
                let has_failed = !failed.is_empty();
                let mut btns = h_flex().gap(px(8.)).justify_end().pt(px(4.));
                if dlg.running {
                    btns = btns.child(
                        Button::new("imp-stop")
                            .outline()
                            .label("Stop after current")
                            .on_click(cx.listener(|this, _ev, _w, cx| {
                                if let Some(d) = this.import_dlg.as_mut() {
                                    d.stop.store(
                                        true,
                                        std::sync::atomic::Ordering::Relaxed,
                                    );
                                }
                                cx.notify();
                            })),
                    );
                } else {
                    if has_failed {
                        btns = btns.child(
                            Button::new("imp-retry")
                                .ghost()
                                .label("Retry failed")
                                .on_click(cx.listener(|this, _ev, _w, cx| {
                                    this.import_run(true, cx)
                                })),
                        );
                    }
                    btns = btns.child(
                        Button::new("imp-run")
                            .primary()
                            .label(if dlg.progress.is_some() {
                                "Import remaining"
                            } else {
                                "Import selected"
                            })
                            .on_click(cx.listener(|this, _ev, _w, cx| {
                                this.import_run(false, cx)
                            })),
                    );
                }
                footer = footer.child(btns);
                body = body.child(footer);
            } else if dlg.scanning {
                body = body.child(
                    div()
                        .py(px(24.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child("Scanning for projects\u{2026}"),
                );
            }

            card = card.child(body);
            imp_overlay = imp_overlay.child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size_full()
                    .bg(rgba(0x00000080))
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|this, _ev, _w, cx| {
                            if this
                                .import_dlg
                                .as_ref()
                                .map(|d| !d.running)
                                .unwrap_or(true)
                            {
                                this.import_dlg = None;
                            }
                            cx.notify();
                        }),
                    )
                    .child(card),
            );
        }

        let mut pr_overlay = div().absolute().inset_0().size_full();
        if self.pr_dialog_open {
            let mut card = v_flex()
                .w(px(576.))
                .rounded(px(12.))
                .border_1()
                .border_color(theme.border)
                .bg(theme.popover)
                .shadow_lg()
                .p(px(20.))
                .gap(px(12.));
            card = card.child(
                div()
                    .text_size(px(15.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Checkout pull request"),
            );
            card = card.child(
                div()
                    .mt(px(-6.))
                    .text_size(px(13.))
                    .text_color(theme.muted_foreground)
                    .child(
                        "Resolve a GitHub pull request, then create the draft \
                         thread checked out to its branch.",
                    ),
            );
            card = card.child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .child("Pull request"),
            );
            if let Some(input) = self.pr_dialog_input.clone() {
                card = card.child(
                    div()
                        .h(px(34.))
                        .border_1()
                        .border_color(theme.border)
                        .rounded(px(8.))
                        .px_2()
                        .child(Input::new(&input).appearance(false)),
                );
            }
            if let Some(pr) = &self.pr_resolved {
                let state = str_at(pr, "state").to_string();
                let state_color = match state.as_str() {
                    "merged" => rgb(0x6366f1),
                    "open" => rgb(0x059669),
                    _ => rgb(0x71717a),
                };
                card = card.child(
                    div()
                        .rounded(px(12.))
                        .border_1()
                        .border_color(theme.border)
                        .bg(theme.muted)
                        .p(px(12.))
                        .child(
                            h_flex()
                                .items_start()
                                .gap_3()
                                .child(
                                    v_flex()
                                        .flex_1()
                                        .min_w_0()
                                        .child(
                                            div()
                                                .text_size(px(14.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .child(
                                                    str_at(pr, "title").to_string(),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .mt(px(2.))
                                                .text_size(px(13.))
                                                .text_color(theme.muted_foreground)
                                                .child(format!(
                                                    "#{} · {} to {}",
                                                    get(pr, "number")
                                                        .as_i64()
                                                        .unwrap_or(0),
                                                    str_at(pr, "headBranch"),
                                                    str_at(pr, "baseBranch"),
                                                )),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .text_color(state_color)
                                        .child(state),
                                ),
                        ),
                );
            }
            if self.pr_resolving {
                card = card.child(
                    div()
                        .text_size(px(13.))
                        .text_color(theme.muted_foreground)
                        .child("Resolving pull request\u{2026}"),
                );
            }
            if let Some(err) = &self.pr_dialog_error {
                card = card.child(
                    div()
                        .text_size(px(13.))
                        .text_color(rgb(0xdc2626))
                        .child(err.clone()),
                );
            }
            let can_confirm = self.pr_resolved.is_some() && !self.pr_busy;
            let btn = |id: &'static str,
                       label: &'static str,
                       primary: bool,
                       cx: &mut Context<Self>| {
                let mut b = div()
                    .id(id)
                    .h(px(32.))
                    .px_3()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(8.))
                    .text_size(px(13.))
                    .cursor_pointer();
                if primary {
                    b = b
                        .bg(if can_confirm {
                            theme.foreground
                        } else {
                            theme.muted
                        })
                        .text_color(theme.background);
                } else {
                    b = b
                        .border_1()
                        .border_color(theme.border)
                        .text_color(theme.foreground);
                }
                b.child(label)
            };
            card = card.child(
                h_flex()
                    .mt(px(4.))
                    .justify_end()
                    .gap_2()
                    .child(
                        div()
                            .id("pr-cancel")
                            .h(px(32.))
                            .px_3()
                            .flex()
                            .items_center()
                            .rounded(px(8.))
                            .border_1()
                            .border_color(theme.border)
                            .text_size(px(13.))
                            .cursor_pointer()
                            .child("Cancel")
                            .on_click(cx.listener(|this, _e, _w, cx| {
                                if !this.pr_busy {
                                    this.pr_dialog_open = false;
                                    this.pr_dialog_input = None;
                                }
                                cx.notify();
                            })),
                    )
                    .child(
                        btn("pr-local", "Local", false, cx).on_click(cx.listener(
                            |this, _e, _w, cx| {
                                this.confirm_pr_checkout("local", cx);
                            },
                        )),
                    )
                    .child(
                        btn("pr-worktree", "Worktree", true, cx).on_click(
                            cx.listener(|this, _e, _w, cx| {
                                this.confirm_pr_checkout("worktree", cx);
                            }),
                        ),
                    ),
            );
            pr_overlay = pr_overlay.child(
                v_flex()
                    .size_full()
                    .items_center()
                    .justify_center()
                    .bg(rgba(0x00000080))
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|this, _ev, _w, cx| {
                            if !this.pr_busy {
                                this.pr_dialog_open = false;
                                this.pr_dialog_input = None;
                            }
                            cx.notify();
                        }),
                    )
                    .child(
                        div()
                            .on_mouse_down(
                                gpui::MouseButton::Left,
                                |_e, _w, cx| cx.stop_propagation(),
                            )
                            .child(card),
                    ),
            );
        }

        NativeOverlays {
            ctx_overlay: ctx_overlay.into_any(),
            hc_overlay: hc_overlay.into_any(),
            pc_overlay: pc_overlay.into_any(),
            mp_overlay: mp_overlay.into_any(),
            bp_overlay: bp_overlay.into_any(),
            ren_overlay: ren_overlay.into_any(),
            palette_overlay: palette_overlay.into_any(),
            sc_overlay: sc_overlay.into_any(),
            fb_overlay: fb_overlay.into_any(),
            scd_overlay: scd_overlay.into_any(),
            snz_overlay: snz_overlay.into_any(),
            tour_overlay: tour_overlay.into_any(),
            git_overlay: git_overlay.into_any(),
            autom_overlay: autom_overlay.into_any(),
            tp_overlay: tp_overlay.into_any(),
            imp_overlay: imp_overlay.into_any(),
            pr_overlay: pr_overlay.into_any(),
            wn_pop_overlay: wn_pop_overlay.into_any(),
            wn_dialog_overlay: wn_dialog_overlay.into_any()
        }
    }
}
