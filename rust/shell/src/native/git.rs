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
    pub(super) fn toggle_changes(&mut self, cx: &mut Context<Self>) {
        self.view_changes = !self.view_changes;
        if self.view_changes {
            self.fetch_changes(cx);
        }
        cx.notify();
    }

    /// End-of-turn files card: open the precomputed checkpoint diff for one
    /// turn in the diff panel (upstream Review on the Edited-files card).
    pub(super) fn open_turn_diff(
        &mut self,
        thread_id: String,
        from_turn: i64,
        to_turn: i64,
        cx: &mut Context<Self>,
    ) {
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            let res = cx
                .background_executor()
                .spawn(async move {
                    client.call(
                        "orchestration.getTurnDiff",
                        json!({"threadId": thread_id, "fromTurnCount": from_turn, "toTurnCount": to_turn}),
                    )
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if let Some(v) = res {
                    let patch = str_at(&v, "diff").to_string();
                    if patch.is_empty() {
                        return;
                    }
                    this.blame_pop = None;
                    let files =
                        gpui_kit::component::diff::DiffFile::parse(&patch).unwrap_or_default();
                    this.changes_diff_state = Some(cx.new(|cx| {
                        gpui_kit::component::diff::DiffState::new(files, cx)
                            .with_mode(gpui_kit::component::diff::DiffMode::Unified)
                    }));
                    this.changes_diff = Some((format!("turn {to_turn}"), patch));
                    this.view_changes = true;
                    this.dock_tab = 0;
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn stage_paths(
        &mut self,
        paths: Vec<String>,
        unstage: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(cwd) = self.files_root() else {
            return;
        };
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            let method = if unstage {
                "git.unstageFiles"
            } else {
                "git.stageFiles"
            };
            let res = cx
                .background_executor()
                .spawn(async move { client.call(method, json!({"cwd": cwd, "paths": paths})) })
                .await;
            let _ = res;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.fetch_changes(cx);
            });
        })
        .detach();
    }

    pub(super) fn open_diff(&mut self, path: String, cx: &mut Context<Self>) {
        let Some(cwd) = self.files_root() else {
            return;
        };
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            let p = path.clone();
            let res = cx
                .background_executor()
                .spawn(async move {
                    client.call(
                        "git.readWorkingTreeDiff",
                        json!({"cwd": cwd, "filePath": p}),
                    )
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if let Some(v) = res {
                    this.blame_pop = None;
                    let patch = str_at(&v, "patch").to_string();
                    let files =
                        gpui_kit::component::diff::DiffFile::parse(&patch).unwrap_or_default();
                    this.changes_diff_state = Some(cx.new(|cx| {
                        gpui_kit::component::diff::DiffState::new(files, cx)
                            .with_mode(gpui_kit::component::diff::DiffMode::Unified)
                    }));
                    this.changes_diff = Some((path.clone(), patch));
                    this.view_changes = true;
                    this.dock_tab = 0;
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Pin/unpin one inbox row in every project context it carries (upstream
    /// pullRequestPinToggleInputs), then refresh the list superset.
    pub(super) fn toggle_github_pin(&mut self, item: Value, cx: &mut Context<Self>) {
        let pinned = get(&item, "isPinned").as_bool().unwrap_or(false);
        let repository = str_at(&item, "repository").to_string();
        let number = get(&item, "number").as_i64().unwrap_or(0);
        if repository.is_empty() || number <= 0 {
            return;
        }
        let mut project_ids: Vec<String> = arr(&item, "projectContexts")
            .iter()
            .filter(|c| !pinned || get(c, "isPinned").as_bool() == Some(true))
            .map(|c| str_at(c, "projectId").to_string())
            .filter(|id| !id.is_empty())
            .collect();
        if project_ids.is_empty() {
            let pid = str_at(&item, "projectId").to_string();
            if !pid.is_empty() {
                project_ids.push(pid);
            }
        }
        project_ids.sort();
        project_ids.dedup();
        if project_ids.is_empty() {
            return;
        }
        let next = !pinned;
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            let repository2 = repository.clone();
            let res = cx
                .background_executor()
                .spawn(async move {
                    let mut ok = true;
                    for project_id in project_ids {
                        ok &= client
                            .call(
                                "pullRequests.setPinned",
                                json!({
                                    "projectId": project_id,
                                    "repository": repository2,
                                    "number": number,
                                    "isPinned": next,
                                }),
                            )
                            .is_some();
                    }
                    ok
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if res {
                    this.fetch_github_inbox(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn select_github_item(&mut self, item: Value, cx: &mut Context<Self>) {
        let project_id = str_at(&item, "projectId").to_string();
        let repository = str_at(&item, "repository").to_string();
        let number = get(&item, "number").as_u64().unwrap_or(0);
        if project_id.is_empty() || repository.is_empty() || number == 0 {
            return;
        }
        if str_at(&item, "kind") == "issue" {
            self.pr_sel = None;
            self.pr_detail = None;
            self.issue_sel = Some((project_id, repository, number));
            self.issue_detail = None;
            self.issue_detail_tab = 0;
            self.issue_comment_input = None;
            self.fetch_issue_detail(cx);
        } else {
            self.issue_sel = None;
            self.issue_detail = None;
            self.select_pr(project_id, repository, number, cx);
        }
    }

    pub(super) fn fetch_issue_detail(&mut self, cx: &mut Context<Self>) {
        let Some((pid, repo, num)) = self.issue_sel.clone() else {
            return;
        };
        self.issue_detail_loading = true;
        self.issue_detail_epoch += 1;
        let epoch = self.issue_detail_epoch;
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            let res = cx
                .background_executor()
                .spawn(async move {
                    client.call(
                        "githubInbox.issueDetail",
                        json!({"projectId": pid, "repository": repo, "number": num}),
                    )
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if this.issue_detail_epoch != epoch {
                    return;
                }
                this.issue_detail_loading = false;
                if let Some(v) = res {
                    this.issue_detail = Some(v);
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn post_issue_comment(&mut self, cx: &mut Context<Self>) {
        let Some((pid, repo, num)) = self.issue_sel.clone() else {
            return;
        };
        let body = self
            .issue_comment_input
            .as_ref()
            .map(|i| i.read(cx).value().to_string())
            .unwrap_or_default()
            .trim()
            .to_string();
        if body.is_empty() || self.issue_comment_posting {
            return;
        }
        self.issue_comment_posting = true;
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            let res = cx
                .background_executor()
                .spawn(async move {
                    client.call(
                        "githubInbox.issueComment",
                        json!({"projectId": pid, "repository": repo, "number": num, "body": body}),
                    )
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.issue_comment_posting = false;
                if res.is_some() {
                    this.issue_comment_input = None;
                    this.fetch_issue_detail(cx);
                    this.fetch_github_inbox(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Code review page: select a PR row and load `pullRequests.detail`
    /// (upstream pullRequestDetailQueryOptions feeding PullRequestDetailPanel).
    pub(super) fn select_pr(
        &mut self,
        project_id: String,
        repository: String,
        number: u64,
        cx: &mut Context<Self>,
    ) {
        self.pr_sel = Some((project_id, repository, number));
        self.pr_detail = None;
        self.pr_detail_tab = 0;
        self.pr_disc = [true, true, true];
        self.pr_diff_state = None;
        self.pr_diff_loading = false;
        self.pr_diff_error = None;
        self.pr_diff_truncated = false;
        self.pr_diff_files = Vec::new();
        self.pr_actions_menu = None;
        self.pr_stack_menu = None;
        self.pr_confirm = None;
        self.pr_merge_method = "merge".to_string();
        self.pr_comments_closed.clear();
        self.pr_comment_input = None;
        self.fetch_pr_detail(cx);
    }

    pub(super) fn fetch_pr_detail(&mut self, cx: &mut Context<Self>) {
        let Some((pid, repo, num)) = self.pr_sel.clone() else {
            return;
        };
        self.pr_detail_loading = true;
        self.pr_detail_epoch += 1;
        let epoch = self.pr_detail_epoch;
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            let res = cx
                .background_executor()
                .spawn(async move {
                    client.call(
                        "pullRequests.detail",
                        json!({"projectId": pid, "repository": repo, "number": num}),
                    )
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if this.pr_detail_epoch != epoch {
                    return;
                }
                this.pr_detail_loading = false;
                if let Some(v) = res {
                    this.pr_detail = Some(v);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Code tab: `pullRequests.diff` → DiffState in split mode (upstream
    /// PullRequestCodeTab → DiffPanelPatchViewport diffRenderMode="split").
    pub(super) fn fetch_pr_diff(&mut self, cx: &mut Context<Self>) {
        let Some((pid, repo, num)) = self.pr_sel.clone() else {
            return;
        };
        self.pr_diff_loading = true;
        self.pr_diff_error = None;
        let epoch = self.pr_detail_epoch;
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            let res = cx
                .background_executor()
                .spawn(async move {
                    client.call(
                        "pullRequests.diff",
                        json!({"projectId": pid, "repository": repo, "number": num}),
                    )
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if this.pr_detail_epoch != epoch {
                    return;
                }
                this.pr_diff_loading = false;
                match res {
                    Some(v) => {
                        this.pr_diff_truncated = get(&v, "truncated").as_bool().unwrap_or(false);
                        let patch = str_at(&v, "patch").to_string();
                        if patch.is_empty() {
                            this.pr_diff_error =
                                Some("The pull request diff is unavailable.".to_string());
                            this.pr_diff_state = None;
                        } else {
                            let files = gpui_kit::component::diff::DiffFile::parse(&patch)
                                .unwrap_or_default();
                            this.pr_diff_files = parse_patch_files(&patch);
                            this.pr_diff_state = Some(cx.new(|cx| {
                                gpui_kit::component::diff::DiffState::new(files, cx)
                                    .with_mode(gpui_kit::component::diff::DiffMode::Split)
                            }));
                        }
                    }
                    None => {
                        this.pr_diff_error = Some("Could not load diff.".to_string());
                        this.pr_diff_state = None;
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Comment composer → `pullRequests.comment`, then the detail refetches
    /// so the new comment lands in Summary and Timeline (upstream
    /// pullRequestCommentMutationOptions).
    pub(super) fn post_pr_comment(&mut self, cx: &mut Context<Self>) {
        let Some((pid, repo, num)) = self.pr_sel.clone() else {
            return;
        };
        let body = self
            .pr_comment_input
            .as_ref()
            .map(|i| i.read(cx).value().to_string())
            .unwrap_or_default();
        let body = body.trim().to_string();
        if body.is_empty() || self.pr_comment_posting {
            return;
        }
        self.pr_comment_posting = true;
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            let res = cx
                .background_executor()
                .spawn(async move {
                    client.call(
                        "pullRequests.comment",
                        json!({"projectId": pid, "repository": repo, "number": num, "body": body}),
                    )
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.pr_comment_posting = false;
                if res.is_some() {
                    // Recreated empty on the next render (set_value needs a Window).
                    this.pr_comment_input = None;
                    this.fetch_pr_detail(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Merge/ready/draft/close/reopen → `pullRequests.action`, then the
    /// detail refetches (upstream PullRequestActions).
    pub(super) fn pr_action(&mut self, action: &'static str, cx: &mut Context<Self>) {
        let Some((pid, repo, num)) = self.pr_sel.clone() else {
            return;
        };
        let method = self.pr_merge_method.clone();
        let client = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cell = client.clone();
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            let res = cx
                .background_executor()
                .spawn(async move {
                    client.call(
                        "pullRequests.action",
                        json!({"projectId": pid, "repository": repo, "number": num,
                               "action": action, "mergeMethod": method}),
                    )
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                if res.is_some() {
                    this.fetch_pr_detail(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// PullRequestCheckStatusIcon: one glyph per check status.
    pub(super) fn pr_check_icon(&self, status: &str, theme: &Theme) -> AnyElement {
        match status {
            "pending" => div()
                .child(
                    svg_icon(icons::LOADER)
                        .size(px(14.))
                        .text_color(theme.warning),
                )
                .into_any_element(),
            "success" => div()
                .child(
                    svg_icon(icons::CHECK_CIRCLE)
                        .size(px(14.))
                        .text_color(theme.success),
                )
                .into_any_element(),
            "failure" | "cancelled" => div()
                .child(
                    svg_icon(icons::X_CIRCLE)
                        .size(px(14.))
                        .text_color(theme.danger),
                )
                .into_any_element(),
            // skipped | neutral: empty circle outline at half opacity.
            _ => div()
                .size(px(14.))
                .rounded_full()
                .border_1()
                .border_color(theme.muted_foreground)
                .opacity(0.5)
                .into_any_element(),
        }
    }

    /// PullRequestChecksRing: 14px segmented progress ring (success →
    /// failure → pending → neutral order, 2-unit gaps, arcs from 12 o'clock).
    pub(super) fn pr_checks_ring(&self, checks: &[Value], theme: &Theme) -> AnyElement {
        let buckets: Vec<(usize, gpui::Hsla)> = [
            (
                checks
                    .iter()
                    .filter(|c| str_at(c, "status") == "success")
                    .count(),
                theme.success,
            ),
            (
                checks
                    .iter()
                    .filter(|c| matches!(str_at(c, "status"), "failure" | "cancelled"))
                    .count(),
                theme.danger,
            ),
            (
                checks
                    .iter()
                    .filter(|c| str_at(c, "status") == "pending")
                    .count(),
                theme.warning,
            ),
            (
                checks
                    .iter()
                    .filter(|c| matches!(str_at(c, "status"), "skipped" | "neutral"))
                    .count(),
                theme.muted_foreground.opacity(0.5),
            ),
        ]
        .into_iter()
        .filter(|(n, _)| *n > 0)
        .collect();
        let total: usize = buckets.iter().map(|(n, _)| n).sum();
        let segments: Vec<(f32, f32, gpui::Hsla)> = {
            // (start_deg, sweep_deg, color)
            let n_b = buckets.len().max(1) as f32;
            let gap_deg = if buckets.len() > 1 {
                // upstream gap: 2 units of the 6.25r circumference
                2.0 / (2.0 * std::f32::consts::PI * 6.25) * 360.0
            } else {
                0.0
            };
            let mut start = -90.0_f32;
            let mut out = Vec::new();
            for (n, color) in buckets {
                let mut sweep = n as f32 / total as f32 * 360.0;
                if n_b > 1.0 {
                    sweep = (sweep - gap_deg).max(4.0);
                    out.push((start + gap_deg / 2.0, sweep, color));
                } else {
                    out.push((start, sweep, color));
                }
                start += n as f32 / total as f32 * 360.0;
            }
            out
        };
        gpui::canvas(
            move |_bounds, _window, _cx| segments.clone(),
            move |_bounds, segs, window, _cx| {
                let size = 14.0_f32;
                let r = size / 2.0 - 1.05; // stroke 2.1 inside a 14px box
                let c = size / 2.0;
                for (start_deg, sweep_deg, color) in segs.iter() {
                    let a0 = start_deg.to_radians();
                    let a1 = (start_deg + sweep_deg).to_radians();
                    let p0 = gpui::point(px(c + r * a0.cos()), px(c + r * a0.sin()));
                    let p1 = gpui::point(px(c + r * a1.cos()), px(c + r * a1.sin()));
                    let mut b = gpui::PathBuilder::stroke(px(2.1));
                    b.move_to(p0);
                    b.arc_to(
                        gpui::point(px(r), px(r)),
                        px(0.),
                        *sweep_deg > 180.0,
                        true,
                        p1,
                    );
                    if let Ok(path) = b.build() {
                        window.paint_path(path, *color);
                    }
                }
            },
        )
        .w(px(14.))
        .h(px(14.))
        .into_any_element()
    }

    /// PullRequestDetailPanel dock layout in the `/pull-requests` right
    /// pane: header (tab strip + stack/link/menu/primary/close) then the
    /// Summary / Timeline / Code body.
    /// GitHubInbox issue detail: summary/timeline over
    /// `githubInbox.issueDetail`, with external open, pin and the issue
    /// comment composer (upstream GitHubIssueDetailPanel, plain-text body
    /// rendering in the first native pass).
    pub(super) fn issue_detail_pane(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme().clone();
        let detail = self.issue_detail.clone().unwrap_or(Value::Null);
        let muted = theme.muted_foreground;
        let state = str_at(&detail, "state").to_string();
        let number = get(&detail, "number")
            .as_u64()
            .unwrap_or_else(|| self.issue_sel.as_ref().map(|(_, _, n)| *n).unwrap_or(0));
        let repo = str_at(&detail, "repository").to_string();

        let mut tabs = h_flex().gap_1().items_center().min_w_0();
        for (i, name) in ["Summary", "Timeline"].iter().enumerate() {
            let active = self.issue_detail_tab == i;
            tabs = tabs.child(
                div()
                    .id(ElementId::Name(format!("issue-tab-{i}").into()))
                    .h(px(28.))
                    .px(px(10.))
                    .flex()
                    .items_center()
                    .rounded(px(6.))
                    .cursor_pointer()
                    .text_size(px(13.))
                    .when(active, |d| {
                        d.bg(theme.secondary).font_weight(FontWeight::MEDIUM)
                    })
                    .when(!active, |d| {
                        d.text_color(muted)
                            .hover(|d| d.bg(theme.secondary.opacity(0.6)))
                    })
                    .on_click(cx.listener(move |this, _e, _w, cx| {
                        this.issue_detail_tab = i;
                        cx.notify();
                    }))
                    .child(*name),
            );
        }

        let mut right = h_flex().items_center().gap_1().flex_shrink_0();
        if let Some(item) = self
            .gh_items
            .iter()
            .find(|i| {
                str_at(i, "kind") == "issue"
                    && str_at(i, "repository") == repo
                    && get(i, "number").as_u64() == Some(number)
            })
            .cloned()
        {
            let pinned = get(&item, "isPinned").as_bool() == Some(true);
            right = right.child(
                div()
                    .id("issue-pin")
                    .size(px(28.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(6.))
                    .cursor_pointer()
                    .hover(|d| d.bg(theme.secondary))
                    .on_click(cx.listener(move |this, _e, _w, cx| {
                        this.toggle_github_pin(item.clone(), cx);
                    }))
                    .child(
                        svg_icon(if pinned { icons::PIN_FILL } else { icons::PIN })
                            .size(px(14.))
                            .text_color(if pinned { theme.foreground } else { muted }),
                    ),
            );
        }
        let url = str_at(&detail, "url").to_string();
        if !url.is_empty() {
            right = right.child(
                div()
                    .id("issue-open-external")
                    .size(px(28.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(6.))
                    .cursor_pointer()
                    .hover(|d| d.bg(theme.secondary))
                    .on_click(cx.listener(move |_, _e, _w, cx| {
                        cx.open_url(&url);
                    }))
                    .child(
                        svg_icon(icons::EXTERNAL_LINK)
                            .size(px(14.))
                            .text_color(muted),
                    ),
            );
        }

        let mut content = v_flex().w_full().gap_4().px_5().py_4();
        if self.issue_detail_loading && self.issue_detail.is_none() {
            content = content.child(
                div()
                    .py(px(40.))
                    .w_full()
                    .text_center()
                    .text_size(px(13.))
                    .text_color(muted)
                    .child("Loading issue…"),
            );
        } else if self.issue_detail.is_none() {
            content = content.child(
                div()
                    .py(px(40.))
                    .w_full()
                    .text_center()
                    .text_size(px(13.))
                    .text_color(muted)
                    .child("The issue details are unavailable."),
            );
        } else {
            let author = str_at(get(&detail, "author"), "login");
            let created = gh_relative_time(str_at(&detail, "createdAt"));
            let status_color = if state == "open" {
                theme.success
            } else {
                theme.accent
            };
            content = content.child(
                v_flex()
                    .gap_2()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Icon::new(if state == "open" {
                                    AssetIcon::CircleDot
                                } else {
                                    AssetIcon::CircleCheck
                                })
                                .size(px(17.))
                                .text_color(status_color),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_size(px(17.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(str_at(&detail, "title").to_string()),
                            ),
                    )
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .text_size(px(12.))
                            .text_color(muted)
                            .child(format!("#{number}"))
                            .child(repo.clone())
                            .child(author.to_string())
                            .child(format!("opened {created}")),
                    ),
            );

            if self.issue_detail_tab == 0 {
                let body = str_at(&detail, "body").to_string();
                content = content.child(
                    v_flex()
                        .gap_2()
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(muted)
                                .child("Description"),
                        )
                        .child(if body.trim().is_empty() {
                            div()
                                .text_size(px(13.))
                                .text_color(muted)
                                .italic()
                                .child("No description provided.")
                                .into_any_element()
                        } else {
                            TextView::markdown(ElementId::Name("issue-desc-md".into()), body)
                                .style({
                                    let mut s = gpui_kit::component::text::TextViewStyle::default();
                                    s.highlight_theme =
                                        gpui_kit::component::highlighter::HighlightTheme::default_dark();
                                    s
                                })
                                .into_any_element()
                        }),
                );
            } else {
                let comments = arr(&detail, "comments").to_vec();
                if comments.is_empty() {
                    content = content.child(
                        div()
                            .py(px(24.))
                            .text_size(px(13.))
                            .text_color(muted)
                            .child("No comments yet."),
                    );
                }
                for (ci, c) in comments.iter().enumerate() {
                    let who = str_at(get(c, "author"), "login").to_string();
                    let when = gh_relative_time(str_at(c, "createdAt"));
                    let body = str_at(c, "body").to_string();
                    content = content.child(
                        v_flex()
                            .id(ElementId::Name(format!("issue-comment-{ci}").into()))
                            .w_full()
                            .gap_2()
                            .rounded(px(8.))
                            .border_1()
                            .border_color(theme.border.opacity(0.55))
                            .px_3()
                            .py_2()
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .child(if who.is_empty() { "unknown".to_string() } else { who }),
                                    )
                                    .child(div().text_size(px(11.)).text_color(muted).child(when)),
                            )
                            .child(if body.trim().is_empty() {
                                div()
                                    .text_size(px(13.))
                                    .text_color(muted)
                                    .italic()
                                    .child("No comment body.")
                                    .into_any_element()
                            } else {
                                TextView::markdown(
                                    ElementId::Name(format!("issue-comment-md-{ci}").into()),
                                    body,
                                )
                                .style({
                                    let mut s = gpui_kit::component::text::TextViewStyle::default();
                                    s.highlight_theme =
                                        gpui_kit::component::highlighter::HighlightTheme::default_dark();
                                    s
                                })
                                .into_any_element()
                            }),
                    );
                }
            }

            // Composer stays visible in both tabs, like the upstream issue panel.
            if self.issue_comment_input.is_none() {
                let st = cx.new(|cx| {
                    gpui_kit::component::input::TextareaState::new(window, cx)
                        .placeholder("Comment on this issue…")
                        .auto_grow(1, 6)
                        .submit_on_enter(true)
                });
                cx.subscribe_in(&st, window, |this, _s, event, _w, cx| {
                    if let gpui_kit::component::input::InputEvent::PressEnter { shift, .. } = event
                    {
                        if !shift {
                            this.post_issue_comment(cx);
                        }
                    }
                })
                .detach();
                self.issue_comment_input = Some(st);
            }
            let input = self.issue_comment_input.clone().unwrap();
            content = content.child(
                h_flex()
                    .mt_2()
                    .w_full()
                    .items_end()
                    .gap_1p5()
                    .rounded(px(24.))
                    .border_1()
                    .border_color(theme.border.opacity(0.6))
                    .py_1()
                    .pl_3()
                    .pr_1p5()
                    .child(
                        div()
                            .size(px(20.))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_full()
                            .bg(theme.secondary)
                            .mb(px(4.))
                            .child(
                                Icon::new(AssetIcon::CircleDot)
                                    .size(px(12.))
                                    .text_color(muted),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(gpui_kit::component::input::Textarea::new(&input)),
                    )
                    .child(
                        div()
                            .id("issue-comment-send")
                            .size(px(28.))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_full()
                            .cursor_pointer()
                            .bg(theme.accent)
                            .when(self.issue_comment_posting, |d| d.opacity(0.35))
                            .on_click(cx.listener(|this, _e, _w, cx| {
                                this.post_issue_comment(cx);
                            }))
                            .child(
                                svg_icon(icons::SEND)
                                    .size(px(14.))
                                    .text_color(theme.accent_foreground),
                            ),
                    ),
            );
        }

        v_flex()
            .flex_1()
            .h_full()
            .min_w_0()
            .child(
                h_flex()
                    .h(px(44.))
                    .px_3()
                    .items_center()
                    .gap_2()
                    .border_b_1()
                    .border_color(theme.border)
                    .child(tabs)
                    .child(div().flex_1())
                    .child(right),
            )
            .child(
                div()
                    .id("issue-detail-scroll")
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .track_scroll(&self.issue_scroll)
                    .overflow_y_scroll()
                    .child(content),
            )
            .into_any_element()
    }

    pub(super) fn pr_detail_pane(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme().clone();
        let detail_opt = self.pr_detail.clone();
        let detail = detail_opt.clone().unwrap_or(Value::Null);
        let state = str_at(&detail, "state").to_string();
        let is_draft = get(&detail, "isDraft").as_bool().unwrap_or(false);
        let muted = theme.muted_foreground;

        // ---------- tab strip ----------
        let mut tabs = h_flex().gap_1().items_center().min_w_0();
        for (i, name) in ["Summary", "Timeline", "Code"].iter().enumerate() {
            let active = self.pr_detail_tab == i;
            tabs = tabs.child(
                div()
                    .id(ElementId::Name(format!("pr-tab-{i}").into()))
                    .h(px(28.))
                    .px(px(10.))
                    .flex()
                    .items_center()
                    .rounded(px(6.))
                    .cursor_pointer()
                    .text_size(px(13.))
                    .when(active, |d| {
                        d.bg(theme.secondary).font_weight(FontWeight::MEDIUM)
                    })
                    .when(!active, |d| {
                        d.text_color(muted)
                            .hover(|d| d.bg(theme.secondary.opacity(0.6)))
                    })
                    .on_click(cx.listener(move |this, _e, _w, cx| {
                        this.pr_detail_tab = i;
                        this.pr_actions_menu = None;
                        this.pr_stack_menu = None;
                        if i == 2 {
                            this.fetch_pr_diff(cx);
                        }
                        cx.notify();
                    }))
                    .child(*name),
            );
        }

        // ---------- header right cluster ----------
        let mut right = h_flex().items_center().gap_1().flex_shrink_0();
        let stack = get(&detail, "stack");
        if !stack.is_null() {
            let size = get(stack, "size").as_i64().unwrap_or(0);
            let pos = get(stack, "position").as_i64().unwrap_or(0);
            let (label, tone, _, _, _) = pr_stack_assessment(stack);
            let tone_color = match tone {
                "ready" | "complete" => theme.success,
                "warning" => theme.warning,
                "blocked" => theme.danger,
                _ => muted,
            };
            right = right.child(
                h_flex()
                    .id("pr-stack-chip")
                    .h(px(28.))
                    .px_2()
                    .gap_1p5()
                    .items_center()
                    .rounded(px(6.))
                    .cursor_pointer()
                    .hover(|d| d.bg(theme.secondary))
                    .on_click(cx.listener(|this, e: &gpui::ClickEvent, _w, cx| {
                        this.pr_actions_menu = None;
                        this.pr_stack_menu = if this.pr_stack_menu.is_some() {
                            None
                        } else {
                            Some((e.position().x.as_f32(), e.position().y.as_f32() + 6.0))
                        };
                        cx.notify();
                    }))
                    .child(div().size(px(7.)).rounded_full().bg(tone_color))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .child(format!("Stack {pos} of {size}")),
                    )
                    .child(div().text_size(px(11.)).text_color(muted).child(label))
                    .child(
                        svg_icon(icons::CHEVRON_DOWN)
                            .size(px(11.))
                            .text_color(muted),
                    ),
            );
        }
        let url = str_at(&detail, "url").to_string();
        if !url.is_empty() {
            right = right.child(
                div()
                    .id("pr-open-external")
                    .size(px(28.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(6.))
                    .cursor_pointer()
                    .hover(|d| d.bg(theme.secondary))
                    .on_click(cx.listener(move |_, _e, _w, cx| {
                        cx.open_url(&url);
                    }))
                    .child(
                        svg_icon(icons::EXTERNAL_LINK)
                            .size(px(14.))
                            .text_color(muted),
                    ),
            );
        }
        right = right.child(
            div()
                .id("pr-actions-menu-btn")
                .size(px(28.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(6.))
                .cursor_pointer()
                .hover(|d| d.bg(theme.secondary))
                .on_click(cx.listener(|this, e: &gpui::ClickEvent, _w, cx| {
                    this.pr_stack_menu = None;
                    this.pr_actions_menu = if this.pr_actions_menu.is_some() {
                        None
                    } else {
                        Some((e.position().x.as_f32(), e.position().y.as_f32() + 6.0))
                    };
                    cx.notify();
                }))
                .child(svg_icon(icons::MORE).size(px(14.)).text_color(muted)),
        );
        // primary action pill
        if detail_opt.is_some() {
            if let Some((kind, blocker, stack_count)) = pr_primary_action(&detail) {
                match kind {
                    "ready" => {
                        right = right.child(
                            Button::new("pr-primary-ready")
                                .primary()
                                .compact()
                                .label("Ready for review")
                                .on_click(cx.listener(|this, _e, _w, cx| {
                                    this.pr_action("ready", cx);
                                })),
                        );
                    }
                    _ => {
                        let label = match stack_count {
                            Some(n) if n > 1 => format!("Merge {n}"),
                            _ => "Merge".to_string(),
                        };
                        if let Some(_reason) = blocker {
                            right = right.child(
                                div()
                                    .h(px(28.))
                                    .px_3()
                                    .flex()
                                    .items_center()
                                    .rounded(px(6.))
                                    .bg(theme.secondary)
                                    .text_size(px(13.))
                                    .text_color(muted)
                                    .child(label),
                            );
                        } else {
                            right = right.child(
                                Button::new("pr-primary-merge")
                                    .primary()
                                    .compact()
                                    .label(label)
                                    .on_click(cx.listener(|this, _e, _w, cx| {
                                        this.pr_confirm = Some("merge");
                                        cx.notify();
                                    })),
                            );
                        }
                    }
                }
            }
        }
        right = right.child(
            div()
                .id("pr-close")
                .size(px(28.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(6.))
                .cursor_pointer()
                .hover(|d| d.bg(theme.secondary))
                .on_click(cx.listener(|this, _e, _w, cx| {
                    this.pr_sel = None;
                    this.pr_detail = None;
                    this.pr_actions_menu = None;
                    this.pr_stack_menu = None;
                    this.pr_confirm = None;
                    cx.notify();
                }))
                .child(svg_icon(icons::X).size(px(14.)).text_color(muted)),
        );

        let header = h_flex()
            .min_h(px(48.))
            .flex_shrink_0()
            .items_center()
            .gap_2()
            .px_2()
            .w_full()
            .child(tabs)
            .child(div().flex_1())
            .child(right);

        // ---------- body ----------
        let body: AnyElement = if detail_opt.is_none() {
            // upstream GitHubItemPageLayout skeleton
            let sk = |h: f32, w: f32| {
                div()
                    .h(px(h))
                    .w(relative(w))
                    .rounded(px(6.))
                    .bg(theme.secondary)
            };
            v_flex()
                .flex_1()
                .min_h_0()
                .p_5()
                .gap_4()
                .child(sk(28., 0.8))
                .child(sk(16., 0.4))
                .child(sk(112., 1.0))
                .child(sk(160., 1.0))
                .into_any_element()
        } else {
            match self.pr_detail_tab {
                1 => self.pr_timeline_body(&detail, &theme, cx),
                2 => self.pr_code_body(&detail, &theme, cx),
                _ => self.pr_summary_body(&detail, &theme, window, cx),
            }
        };

        // ---------- overlays ----------
        let mut pane = v_flex()
            .h_full()
            .min_h_0()
            .w_full()
            .child(header)
            .child(body);

        // stack popover
        if let (Some((mx, my)), false) = (self.pr_stack_menu, stack.is_null()) {
            let entries = arr(stack, "entries");
            let pos = get(stack, "position").as_i64().unwrap_or(0);
            let mut list = v_flex().gap_px().p_1();
            for e in entries {
                let n = get(e, "number").as_i64().unwrap_or(0);
                let title = str_at(e, "title").to_string();
                let epos = get(e, "position").as_i64().unwrap_or(0);
                let estate = str_at(e, "state").to_string();
                let edraft = get(e, "isDraft").as_bool().unwrap_or(false);
                let (icon, color) = match (estate.as_str(), edraft) {
                    ("merged", _) => (icons::MERGED_SIMPLE, theme.accent),
                    ("closed", _) => (icons::PR_CLOSED_SIMPLE, theme.danger),
                    (_, true) => (icons::PR_SIMPLE, muted),
                    _ => (icons::PR_SIMPLE, theme.success),
                };
                let url = str_at(e, "url").to_string();
                list = list.child(
                    h_flex()
                        .id(ElementId::Name(format!("pr-stack-e{n}").into()))
                        .h(px(30.))
                        .px_2()
                        .gap_2()
                        .items_center()
                        .rounded(px(5.))
                        .cursor_pointer()
                        .hover(|d| d.bg(theme.secondary))
                        .on_click(cx.listener(move |this, _e, _w, cx| {
                            this.pr_stack_menu = None;
                            if !url.is_empty() {
                                cx.open_url(&url);
                            }
                            cx.notify();
                        }))
                        .child(svg_icon(icon).size(px(13.)).text_color(color))
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(muted)
                                .child(format!("#{n}")),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .text_ellipsis()
                                .whitespace_nowrap()
                                .text_size(px(13.))
                                .when(epos == pos, |d| d.font_weight(FontWeight::MEDIUM))
                                .child(title),
                        ),
                );
            }
            let _ = (mx, my);
            pane = pane.child(deferred(
                div()
                    .absolute()
                    .top(px(52.))
                    .right(px(160.))
                    .w(px(280.))
                    .max_h(px(320.))
                    .overflow_y_hidden()
                    .rounded(px(8.))
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.background)
                    .shadow_lg()
                    .child(
                        v_flex()
                            .child(
                                div()
                                    .px_3()
                                    .pt_2()
                                    .pb_1()
                                    .text_size(px(11.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(muted)
                                    .child("Stack position"),
                            )
                            .child(list),
                    )
                    .on_mouse_down_out(cx.listener(move |this, _e, _w, cx| {
                        this.pr_stack_menu = None;
                        cx.notify();
                    }))
                    .into_any_element(),
            ));
        }

        // actions menu
        if let Some((mx, my)) = self.pr_actions_menu {
            let mut menu = v_flex().gap_px().p_1();
            let mrow = |id: &'static str, label: &'static str| -> Button {
                Button::new(id)
                    .ghost()
                    .w_full()
                    .h(px(30.))
                    .justify_start()
                    .px_2()
                    .label(label)
            };
            if state == "open" {
                // draft radio group
                for (val, lab) in [(true, "Draft"), (false, "Ready for review")] {
                    let sel = is_draft == val;
                    let act: &'static str = if val { "draft" } else { "ready" };
                    menu = menu.child(
                        h_flex()
                            .id(ElementId::Name(format!("pr-draft-{val}").into()))
                            .h(px(30.))
                            .px_2()
                            .gap_2()
                            .items_center()
                            .rounded(px(5.))
                            .cursor_pointer()
                            .hover(|d| d.bg(theme.secondary))
                            .on_click(cx.listener(move |this, _e, _w, cx| {
                                this.pr_actions_menu = None;
                                if !sel {
                                    this.pr_action(act, cx);
                                }
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .size(px(12.))
                                    .rounded_full()
                                    .border_1()
                                    .border_color(if sel {
                                        theme.accent
                                    } else {
                                        muted.opacity(0.6)
                                    })
                                    .when(sel, |d| {
                                        d.child(
                                            div()
                                                .size(px(6.))
                                                .m(px(2.))
                                                .rounded_full()
                                                .bg(theme.accent),
                                        )
                                    }),
                            )
                            .child(div().text_size(px(13.)).child(lab)),
                    );
                }
                menu = menu.child(div().h(px(1.)).w_full().my_1().bg(theme.border));
                // merge-method radio group when >1 allowed
                let caps = get(&detail, "mergeCapabilities");
                let methods: Vec<(&'static str, &'static str)> = [
                    ("merge", "Merge commit"),
                    ("squash", "Squash and merge"),
                    ("rebase", "Rebase and merge"),
                ]
                .into_iter()
                .filter(|(m, _)| get(caps, m).as_bool().unwrap_or(false))
                .collect();
                if methods.len() > 1 {
                    menu = menu.child(
                        div()
                            .px_2()
                            .py_1()
                            .text_size(px(11.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(muted)
                            .child("Merge method"),
                    );
                    for (val, lab) in methods {
                        let sel = self.pr_merge_method == val;
                        menu = menu.child(
                            h_flex()
                                .id(ElementId::Name(format!("pr-method-{val}").into()))
                                .h(px(30.))
                                .px_2()
                                .gap_2()
                                .items_center()
                                .rounded(px(5.))
                                .cursor_pointer()
                                .hover(|d| d.bg(theme.secondary))
                                .on_click(cx.listener(move |this, _e, _w, cx| {
                                    this.pr_merge_method = val.to_string();
                                    this.pr_actions_menu = None;
                                    cx.notify();
                                }))
                                .child(
                                    div()
                                        .size(px(12.))
                                        .rounded_full()
                                        .border_1()
                                        .border_color(if sel {
                                            theme.accent
                                        } else {
                                            muted.opacity(0.6)
                                        })
                                        .when(sel, |d| {
                                            d.child(
                                                div()
                                                    .size(px(6.))
                                                    .m(px(2.))
                                                    .rounded_full()
                                                    .bg(theme.accent),
                                            )
                                        }),
                                )
                                .child(div().text_size(px(13.)).child(lab)),
                        );
                    }
                    menu = menu.child(div().h(px(1.)).w_full().my_1().bg(theme.border));
                }
            }
            // copy link
            let url = str_at(&detail, "url").to_string();
            if !url.is_empty() {
                menu = menu.child(
                    mrow("pr-copy-link", "Copy link")
                        .icon(svg_icon(icons::LINK).size(px(14.)))
                        .on_click(cx.listener(move |this, _e, _w, cx| {
                            this.pr_actions_menu = None;
                            cx.write_to_clipboard(ClipboardItem::new_string(url.clone()));
                            cx.notify();
                        })),
                );
            }
            if state == "open" {
                menu = menu.child(div().h(px(1.)).w_full().my_1().bg(theme.border));
                menu = menu.child(
                    Button::new("pr-close-pr")
                        .ghost()
                        .w_full()
                        .h(px(30.))
                        .justify_start()
                        .px_2()
                        .label("Close pull request")
                        .on_click(cx.listener(|this, _e, _w, cx| {
                            this.pr_actions_menu = None;
                            this.pr_confirm = Some("close");
                            cx.notify();
                        })),
                );
            } else if state == "closed" {
                menu = menu.child(mrow("pr-reopen-pr", "Reopen pull request").on_click(
                    cx.listener(|this, _e, _w, cx| {
                        this.pr_actions_menu = None;
                        this.pr_action("reopen", cx);
                    }),
                ));
            }
            let _ = (mx, my);
            pane = pane.child(deferred(
                div()
                    .absolute()
                    .top(px(52.))
                    .right(px(130.))
                    .w(px(220.))
                    .rounded(px(8.))
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.background)
                    .shadow_lg()
                    .child(menu)
                    .on_mouse_down_out(cx.listener(|this, _e, _w, cx| {
                        this.pr_actions_menu = None;
                        cx.notify();
                    }))
                    .into_any_element(),
            ));
        }

        // confirm dialog
        if let Some(kind) = self.pr_confirm {
            let num = get(&detail, "number").as_i64().unwrap_or(0);
            let base = str_at(&detail, "baseBranch").to_string();
            let (title, desc, confirm_label) = if kind == "close" {
                (
                    "Close pull request".to_string(),
                    format!("This will close #{num} without merging it."),
                    "Close pull request",
                )
            } else {
                let stack_n = pr_stack_assessment(get(&detail, "stack")).2;
                let title = if stack_n > 1 {
                    format!("Merge {stack_n} pull requests")
                } else {
                    "Merge pull request".to_string()
                };
                let mut desc = format!("This will merge #{num}");
                if !base.is_empty() {
                    desc.push_str(&format!(" into {base}"));
                }
                desc.push_str(&format!(" using {}.", self.pr_merge_method));
                (title, desc, "Merge")
            };
            pane = pane.child(deferred(
                div()
                    .absolute()
                    .inset_0()
                    .bg(gpui::black().opacity(0.45))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        v_flex()
                            .w(px(400.))
                            .rounded(px(10.))
                            .border_1()
                            .border_color(theme.border)
                            .bg(theme.background)
                            .shadow_lg()
                            .p_4()
                            .gap_3()
                            .child(
                                div()
                                    .text_base()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(title),
                            )
                            .child(div().text_size(px(13.)).text_color(muted).child(desc))
                            .child(
                                h_flex()
                                    .w_full()
                                    .justify_end()
                                    .gap_2()
                                    .child(
                                        Button::new("pr-confirm-cancel")
                                            .ghost()
                                            .label("Cancel")
                                            .on_click(cx.listener(|this, _e, _w, cx| {
                                                this.pr_confirm = None;
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        Button::new("pr-confirm-ok")
                                            .label(confirm_label)
                                            .when(kind == "close", |b| b.danger())
                                            .when(kind == "merge", |b| b.primary())
                                            .on_click(cx.listener(move |this, _e, _w, cx| {
                                                this.pr_confirm = None;
                                                this.pr_action(kind, cx);
                                            })),
                                    ),
                            ),
                    )
                    .into_any_element(),
            ));
        }

        pane.into_any_element()
    }

    /// PullRequestSummaryTab: title, meta line, meta rows, then the
    /// Description / Checks / Comments disclosures.
    pub(super) fn pr_summary_body(
        &mut self,
        detail: &Value,
        theme: &Theme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let muted = theme.muted_foreground;
        let is_draft = get(detail, "isDraft").as_bool().unwrap_or(false);
        let state = str_at(&detail, "state").to_string();
        let checks = arr(detail, "checks");
        let comments = arr(detail, "comments");

        let mut section = v_flex()
            .px_5()
            .py_5()
            .gap_4()
            .child(
                div()
                    .text_size(px(18.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(str_at(detail, "title").to_string()),
            )
            .child(
                h_flex()
                    .mt_1p5()
                    .gap_2()
                    .flex_wrap()
                    .text_size(px(13.))
                    .text_color(muted)
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.foreground)
                            .child(gh_actor_name(get(detail, "author"))),
                    )
                    .child("·")
                    .child(format!("updated {}", rel_time(str_at(detail, "updatedAt"))))
                    .child("·")
                    .child(pr_state_label(&state, is_draft)),
            );

        // ---- meta rows ----
        let meta_row = |icon: &'static [u8], label: &'static str, content: AnyElement| {
            h_flex()
                .items_start()
                .gap_3()
                .py_1p5()
                .child(
                    h_flex()
                        .h(px(24.))
                        .w(px(96.))
                        .flex_shrink_0()
                        .items_center()
                        .gap_2()
                        .text_color(muted)
                        .child(svg_icon(icon).size(px(13.)).text_color(muted))
                        .child(div().text_size(px(13.)).child(label)),
                )
                .child(
                    h_flex()
                        .min_h(px(24.))
                        .min_w_0()
                        .flex_1()
                        .flex_wrap()
                        .items_center()
                        .gap_2()
                        .child(content),
                )
        };

        // Branches + diffstat
        let adds = get(detail, "additions").as_i64().unwrap_or(0);
        let dels = get(detail, "deletions").as_i64().unwrap_or(0);
        let head = str_at(detail, "headBranch").to_string();
        let base = str_at(detail, "baseBranch").to_string();
        section = section.child(meta_row(
            icons::BRANCH,
            "Branch",
            h_flex()
                .items_center()
                .gap_2()
                .flex_wrap()
                .child(
                    div()
                        .px_1p5()
                        .py_px()
                        .rounded(px(4.))
                        .bg(theme.secondary)
                        .text_size(px(12.))
                        .child(head),
                )
                .child(div().text_color(muted).child("›"))
                .child(
                    div()
                        .px_1p5()
                        .py_px()
                        .rounded(px(4.))
                        .bg(theme.secondary)
                        .text_size(px(12.))
                        .child(base.clone()),
                )
                .when(adds > 0, |d| {
                    d.child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.success)
                            .child(format!("+{adds}")),
                    )
                })
                .when(dels > 0, |d| {
                    d.child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.danger)
                            .child(format!("-{dels}")),
                    )
                })
                .into_any_element(),
        ));

        // Merge conflicts row
        if state == "open" && str_at(detail, "mergeability") == "conflicting" {
            section = section.child(meta_row(
                icons::MERGE_CONFLICT,
                "Merge",
                div()
                    .text_size(px(13.))
                    .text_color(theme.danger)
                    .child(format!("Conflicts with {base}"))
                    .into_any_element(),
            ));
        }

        // Reviewers
        let reviewers = arr(detail, "reviewers");
        let rev_content = if reviewers.is_empty() {
            div().text_size(px(13.)).text_color(muted).child("None")
        } else {
            let names: Vec<String> = reviewers.iter().map(|r| gh_actor_name(r)).collect();
            div().text_size(px(13.)).child(names.join(", "))
        };
        section = section.child(meta_row(
            icons::USER_GROUP,
            "Reviewers",
            rev_content.into_any_element(),
        ));

        // Comments count
        let truncated = get(detail, "commentsTruncated").as_bool().unwrap_or(false)
            || get(detail, "commentsIncomplete").as_bool().unwrap_or(false);
        section = section.child(meta_row(
            icons::BUBBLE_TEXT,
            "Comments",
            div()
                .text_size(px(13.))
                .child(pr_comments_summary(comments.len(), truncated))
                .into_any_element(),
        ));

        // Checks summary with ring
        let (check_summary, check_tone) = pr_checks_summary(&checks);
        let tone_color = match check_tone {
            "success" => theme.success,
            "failure" => theme.danger,
            "warning" => theme.warning,
            _ => muted,
        };
        section = section.child(meta_row(
            icons::CHECK_CIRCLE,
            "Checks",
            h_flex()
                .items_center()
                .gap_2()
                .child(self.pr_checks_ring(&checks, theme))
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(tone_color)
                        .child(check_summary),
                )
                .into_any_element(),
        ));

        // Description
        let body_text = str_at(detail, "body").to_string();
        let desc_el: AnyElement = div()
            .px_5()
            .child(if body_text.trim().is_empty() {
                div()
                    .text_size(px(13.))
                    .text_color(muted)
                    .italic()
                    .child("No description provided.")
                    .into_any_element()
            } else {
                TextView::markdown(ElementId::Name("pr-desc-md".into()), body_text)
                    .style(markdown_style(theme.is_dark()))
                    .into_any_element()
            })
            .into_any_element();
        section = section.child(self.pr_disclosure(
            0,
            "Description".to_string(),
            None,
            desc_el,
            theme,
            cx,
        ));

        // Checks rows
        let mut checks_list = v_flex().px_3().gap_px();
        if checks.is_empty() {
            checks_list = checks_list.child(
                div()
                    .px_2()
                    .py_2()
                    .text_size(px(13.))
                    .text_color(muted)
                    .child("No checks reported"),
            );
        } else {
            for (ci, c) in checks.iter().enumerate() {
                let cname = str_at(c, "name").to_string();
                let cstatus = str_at(c, "status").to_string();
                let clabel = pr_check_status_label(&cstatus);
                let curl = str_at(c, "url").to_string();
                checks_list = checks_list.child(
                    h_flex()
                        .id(ElementId::Name(format!("pr-check-{ci}").into()))
                        .items_center()
                        .gap_2()
                        .px_2()
                        .py_1p5()
                        .rounded(px(6.))
                        .cursor_pointer()
                        .hover(|d| d.bg(theme.secondary))
                        .on_click(cx.listener(move |_, _e, _w, cx| {
                            if !curl.is_empty() {
                                cx.open_url(&curl);
                            }
                        }))
                        .child(self.pr_check_icon(&cstatus, theme))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .text_ellipsis()
                                .whitespace_nowrap()
                                .text_size(px(13.))
                                .child(cname),
                        )
                        .child(div().text_size(px(12.)).text_color(muted).child(clabel)),
                );
            }
        }
        section = section.child(self.pr_disclosure(
            1,
            "Checks".to_string(),
            Some(checks.len()),
            checks_list.into_any_element(),
            theme,
            cx,
        ));

        // Comments: cards (expandable) + composer
        let mut comments_list = v_flex().px_5().gap_px();
        let incomplete = get(detail, "commentsIncomplete").as_bool().unwrap_or(false);
        if incomplete || truncated {
            comments_list = comments_list.child(
                div()
                    .py_1()
                    .text_size(px(12.))
                    .text_color(theme.warning)
                    .child(if incomplete {
                        "Some comments could not be loaded."
                    } else {
                        "Older comments are truncated."
                    }),
            );
        }
        if comments.is_empty() {
            comments_list = comments_list.child(
                div()
                    .py_4()
                    .w_full()
                    .flex()
                    .justify_center()
                    .text_size(px(13.))
                    .text_color(muted)
                    .child("No comments"),
            );
        } else {
            let n = comments.len();
            for (ci, c) in comments.iter().enumerate() {
                let cid = str_at(c, "id").to_string();
                let default_open = n <= 2 || ci >= n.saturating_sub(2);
                let open = default_open ^ self.pr_comments_closed.contains(&cid);
                let author = gh_actor_name(get(c, "author"));
                let when = rel_time(str_at(c, "createdAt"));
                let curl = str_at(c, "url").to_string();
                let path = str_at(c, "path").to_string();
                let body = str_at(c, "body").to_string();
                let cid2 = cid.clone();
                let mut card = v_flex()
                    .border_t_1()
                    .when(ci == 0, |d| d.border_t_0())
                    .border_color(theme.border.opacity(0.5));
                card = card.child(
                    h_flex()
                        .id(ElementId::Name(format!("pr-comment-{ci}").into()))
                        .w_full()
                        .items_center()
                        .gap_2()
                        .py_2p5()
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _e, _w, cx| {
                            if this.pr_comments_closed.contains(&cid2) {
                                this.pr_comments_closed.remove(&cid2);
                            } else {
                                this.pr_comments_closed.insert(cid2.clone());
                            }
                            cx.notify();
                        }))
                        .child(
                            div()
                                .text_size(px(13.))
                                .font_weight(FontWeight::MEDIUM)
                                .child(author),
                        )
                        .child(div().text_size(px(11.)).text_color(muted).child(when))
                        .child(div().flex_1())
                        .child(
                            svg_icon(if open {
                                icons::CHEVRON_DOWN
                            } else {
                                icons::CHEVRON_RIGHT
                            })
                            .size(px(12.))
                            .text_color(muted),
                        ),
                );
                if open {
                    let mut panel = v_flex().pb_3();
                    if !path.is_empty() {
                        panel = panel.child(
                            div()
                                .text_size(px(11.))
                                .text_color(muted)
                                .overflow_hidden()
                                .text_ellipsis()
                                .whitespace_nowrap()
                                .child(path),
                        );
                    }
                    panel = panel.child(if body.trim().is_empty() {
                        div()
                            .text_size(px(13.))
                            .text_color(muted)
                            .italic()
                            .child("No review body.")
                            .into_any_element()
                    } else {
                        TextView::markdown(ElementId::Name(format!("pr-cmd-md-{ci}").into()), body)
                            .style({
                                let mut s = gpui_kit::component::text::TextViewStyle::default();
                                s.highlight_theme =
                                    gpui_kit::component::highlighter::HighlightTheme::default_dark(
                                    );
                                s
                            })
                            .into_any_element()
                    });
                    if !curl.is_empty() {
                        panel = panel.child(
                            h_flex().w_full().justify_end().child(
                                Button::new(ElementId::Name(format!("pr-creply-{ci}").into()))
                                    .ghost()
                                    .compact()
                                    .label("Reply")
                                    .on_click(cx.listener(move |_, _e, _w, cx| {
                                        cx.open_url(&curl);
                                    })),
                            ),
                        );
                    }
                    card = card.child(panel);
                }
                comments_list = comments_list.child(card);
            }
        }

        // composer pill (upstream PullRequestCommentComposer)
        if self.pr_comment_input.is_none() {
            let st = cx.new(|cx| {
                gpui_kit::component::input::TextareaState::new(window, cx)
                    .placeholder("Comment on this pull request…")
                    .auto_grow(1, 6)
                    .submit_on_enter(true)
            });
            cx.subscribe_in(&st, window, |this, _s, event, _w, cx| {
                if let gpui_kit::component::input::InputEvent::PressEnter { shift, .. } = event {
                    if !shift {
                        this.post_pr_comment(cx);
                    }
                }
            })
            .detach();
            self.pr_comment_input = Some(st);
        }
        let input = self.pr_comment_input.clone().unwrap();
        let composer = h_flex()
            .mt_2()
            .w_full()
            .items_end()
            .gap_1p5()
            .rounded(px(24.))
            .border_1()
            .border_color(theme.border.opacity(0.6))
            .py_1()
            .pl_3()
            .pr_1p5()
            .child(
                div()
                    .size(px(20.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .bg(theme.secondary)
                    .mb(px(4.))
                    .child(
                        svg_icon(icons::PULL_REQUESTS)
                            .size(px(12.))
                            .text_color(muted),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(gpui_kit::component::input::Textarea::new(&input)),
            )
            .child(
                div()
                    .id("pr-comment-send")
                    .size(px(28.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .cursor_pointer()
                    .bg(theme.accent)
                    .when(self.pr_comment_posting, |d| d.opacity(0.35))
                    .on_click(cx.listener(|this, _e, _w, cx| {
                        this.post_pr_comment(cx);
                    }))
                    .child(
                        svg_icon(icons::SEND)
                            .size(px(14.))
                            .text_color(theme.accent_foreground),
                    ),
            );
        comments_list = comments_list.child(composer);

        section = section.child(self.pr_disclosure(
            2,
            "Comments".to_string(),
            Some(comments.len()),
            comments_list.into_any_element(),
            theme,
            cx,
        ));

        div()
            .id("pr-summary-scroll")
            .flex_1()
            .min_h_0()
            .w_full()
            .track_scroll(&self.pr_summary_scroll)
            .overflow_y_scroll()
            .child(section)
            .into_any_element()
    }

    /// PullRequestDisclosureSection: border-t header (label + chevron +
    /// count) toggling a padded body.
    pub(super) fn pr_disclosure(
        &self,
        i: usize,
        label: String,
        count: Option<usize>,
        body: AnyElement,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let open = self.pr_disc[i];
        let mut head = h_flex()
            .id(ElementId::Name(format!("pr-disc-{i}").into()))
            .w_full()
            .items_center()
            .gap_1p5()
            .border_t_1()
            .border_color(theme.border.opacity(0.6))
            .py_3()
            .px_5()
            .font_weight(FontWeight::MEDIUM)
            .cursor_pointer()
            .text_size(px(13.))
            .child(label)
            .child(
                svg_icon(if open {
                    icons::CHEVRON_DOWN
                } else {
                    icons::CHEVRON_RIGHT
                })
                .size(px(12.))
                .text_color(theme.muted_foreground),
            )
            .on_click(cx.listener(move |this, _e, _w, cx| {
                this.pr_disc[i] = !this.pr_disc[i];
                cx.notify();
            }));
        if let Some(c) = count {
            head = head.child(
                div()
                    .text_size(px(12.))
                    .text_color(theme.muted_foreground)
                    .child(format!("{c}")),
            );
        }
        let mut sec = v_flex().child(head);
        if open {
            sec = sec.child(div().pb_4().child(body));
        }
        sec.into_any_element()
    }

    /// PullRequestTimelineTab: left rail with dots, event title + relative
    /// time + 3-line body preview.
    pub(super) fn pr_timeline_body(
        &mut self,
        detail: &Value,
        theme: &Theme,
        _cx: &mut Context<Self>,
    ) -> AnyElement {
        let muted = theme.muted_foreground;
        let events = pr_timeline_events(detail);
        let mut rail = v_flex()
            .ml_2()
            .border_l_1()
            .border_color(theme.border.opacity(0.7))
            .pl_5();
        if events.is_empty() {
            rail = rail.child(
                div()
                    .py_4()
                    .text_size(px(13.))
                    .text_color(muted)
                    .child("No activity yet"),
            );
        }
        for (i, (at, title, body)) in events.iter().enumerate() {
            let mut ev = v_flex()
                .relative()
                .pb_5()
                .child(
                    div()
                        .absolute()
                        .left(px(-24.5))
                        .top(px(4.))
                        .size(px(8.))
                        .rounded_full()
                        .border_1()
                        .border_color(theme.border)
                        .bg(theme.background),
                )
                .child(
                    h_flex()
                        .items_baseline()
                        .gap_2()
                        .child(
                            div()
                                .text_size(px(13.))
                                .font_weight(FontWeight::MEDIUM)
                                .child(title.clone()),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(muted)
                                .child(rel_time(at)),
                        ),
                );
            if let Some(b) = body {
                ev = ev.child(
                    div()
                        .mt_1()
                        .text_size(px(12.))
                        .text_color(muted)
                        .line_height(relative(1.4))
                        .max_h(px(50.))
                        .overflow_hidden()
                        .child(b.clone()),
                );
            }
            let _ = i;
            rail = rail.child(ev);
        }
        div()
            .id("pr-timeline-scroll")
            .flex_1()
            .min_h_0()
            .w_full()
            .track_scroll(&self.pr_timeline_scroll)
            .overflow_y_scroll()
            .child(div().px_5().py_5().child(rail))
            .into_any_element()
    }

    /// PullRequestCodeTab: stat line ("N files" + diffstat) then the split
    /// diff viewport; truncation banner when the patch was cut at 8 MiB.
    pub(super) fn pr_code_body(
        &mut self,
        detail: &Value,
        theme: &Theme,
        _cx: &mut Context<Self>,
    ) -> AnyElement {
        let muted = theme.muted_foreground;
        let mut pane = v_flex().flex_1().min_h_0().w_full();
        if self.pr_diff_truncated {
            pane = pane.child(
                div()
                    .px_3()
                    .py_2()
                    .text_size(px(12.))
                    .text_color(theme.warning)
                    .child("The diff exceeded 8 MiB and was truncated."),
            );
        }
        let files = &self.pr_diff_files;
        if !files.is_empty() {
            let n = files.len();
            let ins: i64 = files
                .iter()
                .map(|f| get(f, "insertions").as_i64().unwrap_or(0))
                .sum();
            let del: i64 = files
                .iter()
                .map(|f| get(f, "deletions").as_i64().unwrap_or(0))
                .sum();
            pane = pane.child(
                h_flex()
                    .border_b_1()
                    .border_color(theme.border)
                    .px_3()
                    .py_2()
                    .gap_2()
                    .text_size(px(12.))
                    .text_color(muted)
                    .child(plural(n as f64, "file", "files"))
                    .child(div().text_color(theme.success).child(format!("+{ins}")))
                    .child(div().text_color(theme.danger).child(format!("-{del}"))),
            );
        }
        let content: AnyElement = if self.pr_diff_loading {
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(13.))
                .text_color(muted)
                .child("Loading pull request diff")
                .into_any_element()
        } else if let Some(err) = self.pr_diff_error.clone() {
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(13.))
                .text_color(muted)
                .child(err)
                .into_any_element()
        } else if let Some(dstate) = self.pr_diff_state.clone() {
            gpui_kit::component::diff::Diff::new(&dstate)
                .line_number(true)
                .syntax_highlight(true)
                .header_visible(false)
                .hunk_separator(gpui_kit::component::diff::DiffHunkSeparator::Metadata)
                .hover_highlight(gpui_kit::component::diff::DiffHoverHighlight::Line)
                .into_any_element()
        } else {
            let changed = get(detail, "changedFiles").as_i64().unwrap_or(0);
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(13.))
                .text_color(muted)
                .child(if changed == 0 {
                    "This pull request has no file changes."
                } else {
                    "The pull request diff is unavailable."
                })
                .into_any_element()
        };
        pane = pane.child(div().flex_1().min_h_0().w_full().child(content));
        pane.into_any_element()
    }

    pub(super) fn git_cwd(&self) -> Option<String> {
        self.thread_project_root().or_else(|| self.files_root())
    }

    pub(super) fn open_git_dialog(
        &mut self,
        kind: GitDialog,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let ph = match kind {
            GitDialog::Commit => "Commit message",
            GitDialog::Pr => "Pull request title",
        };
        let input = cx.new(|cx| InputState::new(window, cx).placeholder(ph));
        cx.subscribe_in(&input, window, |_this, _s, event, _w, cx| {
            if let InputEvent::Change = event {
                cx.notify();
            }
        })
        .detach();
        match kind {
            GitDialog::Commit => {
                self.git_msg_input = Some(input.clone());
                self.git_commit_action = "commit".to_string();
            }
            GitDialog::Pr => {
                self.git_pr_title_input = Some(input.clone());
                let body =
                    cx.new(|cx| InputState::new(window, cx).placeholder("Pull request body"));
                cx.subscribe_in(&body, window, |_this, _s, event, _w, cx| {
                    if let InputEvent::Change = event {
                        cx.notify();
                    }
                })
                .detach();
                self.git_pr_body_input = Some(body);
                self.git_pr_draft = false;
            }
        }
        self.git_dialog = Some(kind);
        let focus = input.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        cx.notify();
    }

    pub(super) fn close_git_dialog(&mut self, cx: &mut Context<Self>) {
        self.git_dialog = None;
        cx.notify();
    }

    /// git.runStackedAction: action in commit|commit_push|push|create_pr;
    /// dialog "commit_new_branch" maps to action=commit + featureBranch=true.
    pub(super) fn run_git_action(&mut self, action: &str, cx: &mut Context<Self>) {
        if self.git_action_running {
            return;
        }
        let Some(cwd) = self.git_cwd() else {
            self.feedback_toast = Some(("No repository for this thread.".to_string(), true));
            cx.notify();
            return;
        };
        let mut payload = json!({"cwd": cwd, "action": action});
        let mut effective = action.to_string();
        if self.git_dialog == Some(GitDialog::Commit) {
            let msg = self
                .git_msg_input
                .as_ref()
                .map(|i| i.read(cx).value().to_string())
                .unwrap_or_default();
            if !msg.trim().is_empty() {
                payload["commitMessage"] = json!(msg.trim());
            }
            if action == "commit_new_branch" {
                effective = "commit".to_string();
                payload["action"] = json!("commit");
                payload["featureBranch"] = json!(true);
            }
        } else if self.git_dialog == Some(GitDialog::Pr) {
            let title = self
                .git_pr_title_input
                .as_ref()
                .map(|i| i.read(cx).value().to_string())
                .unwrap_or_default();
            let body = self
                .git_pr_body_input
                .as_ref()
                .map(|i| i.read(cx).value().to_string())
                .unwrap_or_default();
            if !title.trim().is_empty() {
                payload["prTitle"] = json!(title.trim());
            }
            if !body.trim().is_empty() {
                payload["prBody"] = json!(body);
            }
            if self.git_pr_draft {
                payload["prDraft"] = json!(true);
            }
            payload["action"] = json!("create_pr");
            effective = "create_pr".to_string();
        }
        self.git_dialog = None;
        self.git_action_running = true;
        let cell = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let v = cx
                .background_executor()
                .spawn(async move {
                    ensure_client(&cell).and_then(|c| c.call("git.runStackedAction", payload))
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.git_action_running = false;
                match &v {
                    Some(r) => {
                        let c = get(r, "commit");
                        let p = get(r, "push");
                        let pr = get(r, "pr");
                        let msg = if str_at(&pr, "status") == "created"
                            || str_at(&pr, "status") == "opened_existing"
                        {
                            let n = pr
                                .get("number")
                                .and_then(Value::as_u64)
                                .map(|n| format!(" #{n}"))
                                .unwrap_or_default();
                            format!("Created PR{n}")
                        } else if str_at(&c, "status") == "created" {
                            let sha = str_at(&c, "commitSha").chars().take(7).collect::<String>();
                            if str_at(&p, "status") == "pushed" {
                                format!("Committed {sha} and pushed")
                            } else {
                                format!("Committed {sha}")
                            }
                        } else if str_at(&p, "status") == "pushed" {
                            "Pushed".to_string()
                        } else if str_at(&c, "status") == "skipped_no_changes" {
                            "No changes to commit.".to_string()
                        } else {
                            "Done".to_string()
                        };
                        this.feedback_toast = Some((msg, false));
                        this.fetch_git_status(cx);
                        this.fetch_changes(cx);
                    }
                    None => {
                        this.feedback_toast = Some(("Git action failed.".to_string(), true));
                    }
                }
                cx.notify();
            });
            let _ = effective;
        })
        .detach();
    }

    /// Upstream PullRequestThreadDialog (checkout mode) — opened from the
    /// Code review panel's "Search or paste a PR link" field.
    pub(super) fn open_pr_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Paste a GitHub pull request URL or enter #123")
        });
        cx.subscribe_in(
            &input,
            window,
            |this: &mut Self, _s, event, _w, cx| match event {
                InputEvent::Change => this.pr_input_changed(cx),
                InputEvent::PressEnter { .. } => this.confirm_pr_checkout("local", cx),
                _ => {}
            },
        )
        .detach();
        let focus = input.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        self.pr_dialog_input = Some(input);
        self.pr_resolved = None;
        self.pr_resolving = false;
        self.pr_dialog_error = None;
        self.pr_busy = false;
        self.pr_dialog_open = true;
        cx.notify();
    }

    /// Upstream parsePullRequestReference: `#123`, `123`, or a
    /// github.com/<owner>/<repo>/pull/<n> URL.
    pub(super) fn pr_ref_number(&self, cx: &App) -> Option<String> {
        let raw = self
            .pr_dialog_input
            .as_ref()?
            .read(cx)
            .value()
            .trim()
            .to_string();
        if raw.is_empty() {
            return None;
        }
        if let Some(rest) = raw.strip_prefix('#') {
            return if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()) {
                Some(rest.to_string())
            } else {
                None
            };
        }
        if raw.chars().all(|c| c.is_ascii_digit()) {
            return Some(raw);
        }
        if raw.contains("github.com") && raw.contains("/pull/") {
            let tail = raw.rsplit("/pull/").next().unwrap_or("");
            let digits: String = tail.chars().take_while(|c| c.is_ascii_digit()).collect();
            return if digits.is_empty() {
                None
            } else {
                Some(digits)
            };
        }
        None
    }

    /// Debounced resolve — upstream waits 450ms then calls
    /// git.resolvePullRequest.
    pub(super) fn pr_input_changed(&mut self, cx: &mut Context<Self>) {
        self.pr_resolved = None;
        self.pr_dialog_error = None;
        self.pr_resolve_epoch += 1;
        let epoch = self.pr_resolve_epoch;
        let Some(n) = self.pr_ref_number(cx) else {
            self.pr_resolving = false;
            cx.notify();
            return;
        };
        self.pr_resolving = true;
        cx.notify();
        let cell = self.client.clone();
        let cwd = self.git_cwd();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(450))
                .await;
            let Some(client) = ensure_client(&cell) else {
                return;
            };
            let Some(cwd) = cwd else { return };
            let res = client.call(
                "git.resolvePullRequest",
                json!({"cwd": cwd, "reference": n}),
            );
            let _ = this.update(cx, |this: &mut Self, cx| {
                if this.pr_resolve_epoch != epoch {
                    return;
                }
                this.pr_resolving = false;
                match res {
                    Some(v) if !get(&v, "pullRequest").is_null() => {
                        this.pr_resolved = Some(get(&v, "pullRequest").clone());
                    }
                    Some(_) => {
                        this.pr_dialog_error =
                            Some("Could not resolve that pull request.".to_string());
                    }
                    None => {
                        this.pr_dialog_error =
                            Some("Could not resolve that pull request.".to_string());
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// git.preparePullRequestThread then a draft thread bound to the checked
    /// out branch/worktree — upstream's onPrepared flow.
    pub(super) fn confirm_pr_checkout(&mut self, mode: &'static str, cx: &mut Context<Self>) {
        if self.pr_busy || self.pr_resolving {
            return;
        }
        let Some(pr) = self.pr_resolved.clone() else {
            return;
        };
        let Some(n) = self.pr_ref_number(cx) else {
            return;
        };
        let Some(cwd) = self.git_cwd() else {
            self.pr_dialog_error = Some("No repository for this thread.".to_string());
            cx.notify();
            return;
        };
        self.pr_busy = true;
        cx.notify();
        let cell = self.client.clone();
        let pid = arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects")
            .iter()
            .find(|p| {
                let root = str_at(p, "repoPath");
                !root.is_empty() && cwd.starts_with(root)
            })
            .or_else(|| arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "projects").first())
            .map(|p| str_at(p, "id").to_string())
            .unwrap_or_default();
        let title = format!(
            "PR #{}: {}",
            get(&pr, "number").as_i64().unwrap_or(0),
            str_at(&pr, "title")
        );
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let Some(client) = ensure_client(&cell) else {
                let _ = this.update(cx, |this: &mut Self, cx| {
                    this.pr_busy = false;
                    this.pr_dialog_error = Some("Not connected.".to_string());
                    cx.notify();
                });
                return;
            };
            let res = client.call(
                "git.preparePullRequestThread",
                json!({"cwd": cwd, "reference": n, "mode": mode}),
            );
            let Some(v) = res else {
                let _ = this.update(cx, |this: &mut Self, cx| {
                    this.pr_busy = false;
                    this.pr_dialog_error =
                        Some("Failed to prepare pull request thread.".to_string());
                    cx.notify();
                });
                return;
            };
            let branch = str_at(&v, "branch").to_string();
            let worktree = get(&v, "worktreePath").clone();
            let tid = uuid();
            let cmd = json!({
                "type": "thread.create",
                "commandId": uuid(),
                "threadId": tid,
                "projectId": pid,
                "title": title,
                "modelSelection": {
                    "provider": "codex",
                    "instanceId": "codex",
                    "model": "gpt-6-astra",
                },
                "runtimeMode": "fullAccess",
                "interactionMode": "chat",
                "envMode": "local",
                "branch": if branch.is_empty() { Value::Null } else { json!(branch) },
                "worktreePath": worktree,
                "createdAt": now_iso(),
            });
            let c = client.clone();
            cx.background_executor()
                .spawn(async move { c.call("orchestration.dispatchCommand", cmd) })
                .await;
            let s = client.call("orchestration.getShellSnapshot", json!({}));
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.pr_busy = false;
                this.pr_dialog_open = false;
                this.pr_dialog_input = None;
                if let Some(s) = s {
                    this.snapshot = Some(s);
                }
                this.open_thread(tid, cx);
                cx.notify();
            });
        })
        .detach();
    }
}
