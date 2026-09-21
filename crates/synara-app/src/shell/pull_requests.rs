//! Native PR review, scoped to the explicitly loaded project and provider.
use super::*;
use crate::ui::{self, Glyph, palette};
use gpui::AnyElement;
use serde_json::Value;
use synara_workspace::pull_requests::*;
pub(super) struct PrView {
    search: Entity<TextEntry>,
    title: Entity<TextEntry>,
    body: Entity<TextEntry>,
    base: Entity<TextEntry>,
    head: Entity<TextEntry>,
    project: Option<ProjectId>,
    target: Option<WorkspaceTarget>,
    client: Option<PullRequests>,
    repos: Vec<(String, GithubRepository)>,
    repo: Option<GithubRepository>,
    list: Vec<PullRequest>,
    detail: Option<PrDetail>,
    file: Option<usize>,
    tab: usize,
    filter: PrFilter,
    page: u32,
    generation: u64,
    busy: bool,
    error: Option<String>,
    cancel: PullRequestCancellation,
    confirmation: Option<PrAction>,
    create: bool,
    draft: bool,
}
pub(super) struct Reply {
    generation: u64,
    result: std::result::Result<Outcome, String>,
}
pub(super) enum Outcome {
    Repositories(PullRequests, Vec<(String, GithubRepository)>),
    List(Vec<PullRequest>),
    Detail(Box<PrDetail>),
    Written,
}
impl PrView {
    pub fn new(cx: &mut Context<Shell>) -> Self {
        let field = |cx: &mut Context<Shell>, label: &'static str, mode| {
            cx.new(|cx| TextEntry::new(label, mode, 34., cx))
        };
        Self {
            search: field(cx, "Search title/body", EntryMode::SingleLine),
            title: field(cx, "PR title", EntryMode::SingleLine),
            body: field(
                cx,
                "Description / comment / review text",
                EntryMode::Composer,
            ),
            base: field(cx, "Base branch", EntryMode::SingleLine),
            head: field(cx, "Head branch or owner:branch", EntryMode::SingleLine),
            project: None,
            target: None,
            client: None,
            repos: vec![],
            repo: None,
            list: vec![],
            detail: None,
            file: None,
            tab: 0,
            filter: PrFilter::Open,
            page: 1,
            generation: 0,
            busy: false,
            error: None,
            cancel: Default::default(),
            confirmation: None,
            create: false,
            draft: true,
        }
    }
    pub(super) fn retire(&mut self) {
        self.cancel.cancel();
        self.generation = self.generation.wrapping_add(1);
        self.busy = false;
        self.confirmation = None;
    }
    fn begin(&mut self) -> (u64, PullRequestCancellation) {
        self.cancel.cancel();
        self.cancel = Default::default();
        self.generation = self.generation.wrapping_add(1);
        self.busy = true;
        self.error = None;
        self.confirmation = None;
        (self.generation, self.cancel.clone())
    }
}
impl Drop for PrView {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}
fn same_target(left: &WorkspaceTarget, right: &WorkspaceTarget) -> bool {
    match (left, right) {
        (WorkspaceTarget::Local { root: a }, WorkspaceTarget::Local { root: b }) => a == b,
        (
            WorkspaceTarget::Ssh {
                workspace: a,
                root: ar,
            },
            WorkspaceTarget::Ssh {
                workspace: b,
                root: br,
            },
        ) => a.id == b.id && ar == br && a.location == b.location,
        _ => false,
    }
}
impl Shell {
    fn pr_scope_current(&self) -> bool {
        self.project == self.pull_requests.project
            && self
                .pull_requests
                .target
                .as_ref()
                .zip(self.workspace_target().as_ref())
                .is_some_and(|(loaded, current)| same_target(loaded, current))
    }
    fn pr_require_scope(&mut self, cx: &mut Context<Self>) -> bool {
        if self.pr_scope_current() {
            return true;
        }
        self.pull_requests.retire();
        self.pull_requests.detail = None;
        self.pull_requests.error = Some("The loaded project or workspace changed. Load the selected project again before reading or writing provider state.".into());
        cx.notify();
        false
    }
    pub(super) fn pr_reply(&mut self, reply: Reply, cx: &mut Context<Self>) {
        if !self.pr_require_scope(cx) || reply.generation != self.pull_requests.generation {
            return;
        }
        let view = &mut self.pull_requests;
        view.busy = false;
        match reply.result {
            Err(e) => view.error = Some(e),
            Ok(Outcome::Repositories(client, repos)) => {
                view.client = Some(client);
                view.repos = repos;
                view.repo = None;
            }
            Ok(Outcome::List(list)) => {
                view.list = list;
                view.detail = None;
                view.file = None;
            }
            Ok(Outcome::Detail(detail)) => {
                view.detail = Some(*detail);
                view.file = None;
                view.tab = 0;
            }
            Ok(Outcome::Written) => {
                view.detail = None;
                view.list.clear();
                view.error = Some(
                    "GitHub confirmed the action. Refresh to read the resulting provider state."
                        .into(),
                );
            }
        }
        cx.notify();
    }
    fn pr_discover(&mut self, cx: &mut Context<Self>) {
        if self.pull_requests.busy {
            return;
        }
        let Some(target) = self.workspace_target() else {
            self.pull_requests.error = Some("Select a project first.".into());
            cx.notify();
            return;
        };
        let view = &mut self.pull_requests;
        view.project = self.project;
        view.target = Some(target.clone());
        view.repos.clear();
        view.repo = None;
        view.list.clear();
        view.detail = None;
        let (generation, cancel) = view.begin();
        let workspace = self.controller.workspace.clone();
        let sender = self.sender.clone();
        self.runtime.spawn(async move {
            let result = async {
                let client = match target {
                    WorkspaceTarget::Local { root } => PullRequests::new(root),
                    WorkspaceTarget::Ssh {
                        workspace: remote,
                        root,
                    } => {
                        let profile = workspace
                            .ssh_profile(remote.id)
                            .await
                            .map_err(|e| e.to_string())?
                            .ok_or("No pinned SSH profile")?;
                        PullRequests::with_host(
                            root,
                            Arc::new(profile.host(&remote).map_err(|e| e.to_string())?),
                        )
                    }
                };
                let repos = client.discover(cancel).await?;
                Ok(Outcome::Repositories(client, repos))
            }
            .await;
            let _ = sender
                .send(Update::PullRequests(Box::new(Reply { generation, result })))
                .await;
        });
        cx.notify();
    }
    fn pr_load(&mut self, number: Option<u64>, cx: &mut Context<Self>) {
        if !self.pr_require_scope(cx) {
            return;
        }
        let view = &mut self.pull_requests;
        if view.busy {
            return;
        }
        let (Some(client), Some(repo)) = (view.client.clone(), view.repo.clone()) else {
            view.error = Some("Discover and select a repository first.".into());
            cx.notify();
            return;
        };
        let text = view.search.read(cx).text().to_owned();
        let filter = view.filter;
        let page = view.page;
        let (generation, cancel) = view.begin();
        let sender = self.sender.clone();
        self.runtime.spawn(async move {
            let result = match number {
                Some(n) => client
                    .detail(&repo, n, cancel)
                    .await
                    .map(|v| Outcome::Detail(Box::new(v))),
                None => client
                    .list(&repo, filter, &text, page, cancel)
                    .await
                    .map(Outcome::List),
            };
            let _ = sender
                .send(Update::PullRequests(Box::new(Reply { generation, result })))
                .await;
        });
        cx.notify();
    }
    fn pr_confirm(&mut self, cx: &mut Context<Self>) {
        if !self.pr_require_scope(cx) {
            return;
        }
        let view = &mut self.pull_requests;
        if view.busy {
            return;
        }
        let (Some(client), Some(repo), Some(action)) = (
            view.client.clone(),
            view.repo.clone(),
            view.confirmation.take(),
        ) else {
            return;
        };
        let (generation, cancel) = view.begin();
        let sender = self.sender.clone();
        self.runtime.spawn(async move {
            let result = client
                .perform(&repo, action, true, cancel)
                .await
                .map(|_| Outcome::Written);
            let _ = sender
                .send(Update::PullRequests(Box::new(Reply { generation, result })))
                .await;
        });
        cx.notify();
    }
    pub(super) fn pull_requests_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let v = &self.pull_requests;
        let mut pane = div().id("pull-requests").size_full().flex().flex_col().p_3().gap_2().min_h_0()
            .child(div().flex().items_center().gap_2().child("Pull requests")
                .child(ui::action("pr-discover", "Load selected project", Some(Glyph::Folder), false, cx.listener(|this, _: &(), _, cx| this.pr_discover(cx))))
                .child(ui::action("pr-cancel", "Cancel request", Some(Glyph::Stop), false, cx.listener(|this, _: &(), _, cx| { this.pull_requests.retire(); this.pull_requests.error = Some("Request cancelled. A submitted write may have reached GitHub. Refresh before retrying.".into()); cx.notify(); }))));
        if let Some(target) = &v.target {
            pane = pane.child(div().text_xs().child(format!(
                "Loaded root: {} | Authentication: selected host's GitHub CLI",
                target.root().display()
            )));
        }
        let mut repos = div().flex().gap_2().flex_wrap();
        for (remote, repo) in &v.repos {
            let selected = v.repo.as_ref() == Some(repo);
            let repo = repo.clone();
            let label = format!("{remote}: {}", repo.slug());
            repos = repos.child(ui::action(
                format!("pr-remote-{remote}"),
                label,
                None,
                selected,
                cx.listener(move |this, _: &(), _, cx| {
                    if this.pull_requests.busy {
                        return;
                    }
                    this.pull_requests.repo = Some(repo.clone());
                    this.pull_requests.page = 1;
                    this.pull_requests.detail = None;
                    this.pull_requests.list.clear();
                    this.pr_load(None, cx);
                }),
            ));
        }
        pane = pane.child(repos);
        let mut filters = div()
            .flex()
            .gap_1()
            .items_center()
            .child(div().flex_1().child(v.search.clone()));
        for (label, filter) in [
            ("Open", PrFilter::Open),
            ("Closed", PrFilter::Closed),
            ("Draft", PrFilter::Draft),
            ("All", PrFilter::All),
        ] {
            filters = filters.child(ui::action(
                format!("pr-filter-{label}"),
                label,
                None,
                v.filter == filter,
                cx.listener(move |this, _: &(), _, cx| {
                    if this.pull_requests.busy {
                        return;
                    }
                    this.pull_requests.filter = filter;
                    this.pull_requests.page = 1;
                    this.pr_load(None, cx);
                }),
            ));
        }
        filters = filters
            .child(ui::action(
                "pr-search",
                "Search / refresh",
                None,
                false,
                cx.listener(|this, _: &(), _, cx| this.pr_load(None, cx)),
            ))
            .child(ui::action(
                "pr-create",
                "Create PR...",
                Some(Glyph::Plus),
                false,
                cx.listener(|this, _: &(), _, cx| {
                    this.pull_requests.create = !this.pull_requests.create;
                    this.pull_requests.confirmation = None;
                    cx.notify();
                }),
            ));
        pane = pane.child(filters).child(div().text_xs().child(format!(
            "Page {} | 50 results per page, maximum 20 pages. No local Git mutation.",
            v.page
        )));
        let mut pager = div().flex().gap_2();
        for (label, delta) in [("Previous", -1i32), ("Next", 1)] {
            pager = pager.child(ui::action(
                format!("pr-page-{label}"),
                label,
                None,
                false,
                cx.listener(move |this, _: &(), _, cx| {
                    if this.pull_requests.busy {
                        return;
                    }
                    this.pull_requests.page =
                        (this.pull_requests.page as i32 + delta).clamp(1, 20) as u32;
                    this.pr_load(None, cx);
                }),
            ));
        }
        pane = pane.child(pager);
        if v.busy {
            pane = pane.child("Loading / executing explicit action...");
        }
        if let Some(error) = &v.error {
            pane = pane.child(div().text_color(rgb(palette().error)).child(error.clone()));
        }
        if v.create {
            pane = pane
                .child(v.title.clone())
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(v.base.clone())
                        .child(v.head.clone()),
                )
                .child(v.body.clone())
                .child(ui::action(
                    "pr-create-draft",
                    if v.draft {
                        "Create as draft: yes"
                    } else {
                        "Create as draft: no"
                    },
                    None,
                    v.draft,
                    cx.listener(|this, _: &(), _, cx| {
                        if !this.pull_requests.busy {
                            this.pull_requests.draft = !this.pull_requests.draft;
                            this.pull_requests.confirmation = None;
                        }
                        cx.notify();
                    }),
                ))
                .child(ui::action(
                    "pr-prepare-create",
                    "Review creation...",
                    None,
                    false,
                    cx.listener(|this, _: &(), _, cx| {
                        let v = &mut this.pull_requests;
                        if v.busy {
                            return;
                        }
                        v.confirmation = Some(PrAction::Create {
                            title: v.title.read(cx).text().into(),
                            body: v.body.read(cx).text().into(),
                            base: v.base.read(cx).text().into(),
                            head: v.head.read(cx).text().into(),
                            draft: v.draft,
                        });
                        cx.notify();
                    }),
                ));
        }
        if let (Some(action), Some(repo)) = (&v.confirmation, &v.repo) {
            pane = pane
                .child(
                    div()
                        .id("pr-confirmation")
                        .max_h(px(220.))
                        .overflow_y_scroll()
                        .border_y_1()
                        .border_color(rgb(palette().border))
                        .p_2()
                        .child(format!("Confirm on {}: {action:?}", repo.slug())),
                )
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(ui::action(
                            "pr-confirm",
                            "Confirm this action",
                            Some(Glyph::Shield),
                            false,
                            cx.listener(|this, _: &(), _, cx| this.pr_confirm(cx)),
                        ))
                        .child(ui::action(
                            "pr-dismiss",
                            "Cancel",
                            None,
                            false,
                            cx.listener(|this, _: &(), _, cx| {
                                this.pull_requests.confirmation = None;
                                cx.notify();
                            }),
                        )),
                );
        }
        let mut list = div()
            .id("pr-list")
            .w(px(280.))
            .min_w(px(180.))
            .overflow_y_scroll()
            .border_r_1()
            .border_color(rgb(palette().border));
        for pr in &v.list {
            let number = pr.number;
            let label = format!(
                "#{} {}\n{} | {}{}",
                number,
                pr.title,
                text(&pr.user, "login"),
                pr.state,
                if pr.draft == Some(true) {
                    " | draft"
                } else {
                    ""
                }
            );
            list = list.child(ui::action(
                format!("pr-{number}"),
                label,
                Some(Glyph::PullRequest),
                v.detail.as_ref().is_some_and(|d| d.pr.number == number),
                cx.listener(move |this, _: &(), _, cx| this.pr_load(Some(number), cx)),
            ));
        }
        pane.child(
            div()
                .flex()
                .flex_1()
                .min_h_0()
                .gap_3()
                .child(list)
                .child(self.pr_detail(cx)),
        )
        .into_any_element()
    }
    fn pr_detail(&self, cx: &mut Context<Self>) -> AnyElement {
        let v = &self.pull_requests;
        let mut pane = div()
            .id("pr-detail")
            .flex_1()
            .min_w_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_2();
        let Some(detail) = &v.detail else {
            return pane
                .child("Select a pull request to review its provider state.")
                .into_any_element();
        };
        let pr = &detail.pr;
        let number = pr.number;
        pane = pane
            .child(div().text_lg().child(format!("#{number} {}", pr.title)))
            .child(format!(
                "{} | {} | draft {:?} | merged {:?}",
                pr.state,
                text(&pr.user, "login"),
                pr.draft,
                pr.merged
            ))
            .child(format!(
                "{} <- {} | head {}",
                text(&pr.base, "ref"),
                text(&pr.head, "ref"),
                text(&pr.head, "sha")
            ));
        for notice in &detail.notices {
            pane = pane.child(
                div()
                    .text_xs()
                    .text_color(rgb(palette().muted))
                    .child(notice.clone()),
            );
        }
        let mut tabs = div().flex().gap_1().flex_wrap();
        for (at, label) in [
            "Description",
            "Files / diff",
            "Commits",
            "Activity",
            "Checks",
            "Reviews",
        ]
        .iter()
        .enumerate()
        {
            tabs = tabs.child(ui::action(
                format!("pr-detail-{at}"),
                *label,
                None,
                v.tab == at,
                cx.listener(move |this, _: &(), _, cx| {
                    this.pull_requests.tab = at;
                    cx.notify();
                }),
            ));
        }
        pane = pane.child(tabs);
        match v.tab {
            0 => {
                pane = pane.child(
                    pr.body
                        .clone()
                        .unwrap_or_else(|| "No description supplied".into()),
                )
            }
            1 => {
                for (at, file) in detail.files.iter().enumerate() {
                    pane = pane.child(ui::action(
                        format!("pr-file-{at}"),
                        format!(
                            "{} | {} +{} -{}",
                            file.filename, file.status, file.additions, file.deletions
                        ),
                        Some(Glyph::Files),
                        v.file == Some(at),
                        cx.listener(move |this, _: &(), _, cx| {
                            this.pull_requests.file = Some(at);
                            cx.notify();
                        }),
                    ));
                }
                if let Some(file) = v.file.and_then(|at| detail.files.get(at)) {
                    if let Ok(path) = file.editor_path() {
                        pane = pane.child(ui::action(
                            "pr-open-local",
                            "Open current checkout file (not PR revision)",
                            Some(Glyph::Files),
                            false,
                            cx.listener(move |this, _: &(), _, cx| {
                                if !this.pr_require_scope(cx) {
                                    return;
                                }
                                this.set_panel(Panel::Files, cx);
                                this.open_file(path.clone(), cx);
                            }),
                        ));
                    }
                    if let Some(raw) = &file.patch {
                        let diff = review::diff::Diff::parse(raw);
                        for line in diff.lines.iter().take(1000) {
                            let color = match line.kind {
                                review::diff::Kind::Added => palette().focus,
                                review::diff::Kind::Removed => palette().error,
                                _ => palette().text,
                            };
                            pane = pane.child(
                                div()
                                    .text_xs()
                                    .font_family("monospace")
                                    .text_color(rgb(color))
                                    .child(format!(
                                        "{:>5} {:>5} {}",
                                        line.old.map(|n| n.to_string()).unwrap_or_default(),
                                        line.new.map(|n| n.to_string()).unwrap_or_default(),
                                        line.text
                                    )),
                            );
                        }
                        if diff.limited || diff.lines.len() > 1000 {
                            pane=pane.child("Diff display limited to 1000 lines. Provider patches may also be truncated.");
                        }
                    } else {
                        pane=pane.child("No textual patch supplied by provider (binary, oversized or unavailable).");
                    }
                }
            }
            index => {
                let rows = match index {
                    2 => &detail.commits,
                    3 => &detail.activity,
                    4 => &detail.checks,
                    _ => &detail.reviews,
                };
                for row in rows {
                    let summary = match index {
                        2 => format!(
                            "{} | {}\n{}",
                            text(row, "sha"),
                            text(&row["commit"]["author"], "name"),
                            text(&row["commit"], "message")
                        ),
                        3 => format!(
                            "{} | {} | {}\n{}",
                            text(row, "created_at"),
                            text(&row["actor"], "login"),
                            text(row, "event"),
                            text(row, "body")
                        ),
                        4 => format!(
                            "{} | {} | {}",
                            text(row, "name"),
                            text(row, "status"),
                            text(row, "conclusion")
                        ),
                        _ => format!(
                            "{} | {} | {}\n{}",
                            text(&row["user"], "login"),
                            text(row, "state"),
                            text(row, "submitted_at"),
                            text(row, "body")
                        ),
                    };
                    pane = pane.child(
                        div()
                            .py_2()
                            .border_b_1()
                            .border_color(rgb(palette().border))
                            .child(summary),
                    );
                }
                if index == 4 {
                    for row in &detail.statuses {
                        pane = pane.child(format!(
                            "{} | {} | {}",
                            text(row, "context"),
                            text(row, "state"),
                            text(row, "description")
                        ));
                    }
                }
            }
        }
        pane = pane.child(
            div()
                .border_t_1()
                .border_color(rgb(palette().border))
                .pt_2()
                .child(v.body.clone()),
        );
        let mut actions = div().flex().flex_wrap().gap_1();
        let sha = text(&pr.head, "sha");
        let node = pr.node_id.clone();
        let draft = pr.draft;
        let is_draft = pr.draft == Some(true);
        let is_open = pr.state == "open";
        for (label, kind) in [
            ("Comment...", 0),
            ("Close/reopen...", 1),
            ("Draft/ready...", 2),
            ("Approve...", 3),
            ("Request changes...", 4),
            ("Merge (squash)...", 5),
            ("Submit review...", 6),
        ] {
            let sha = sha.clone();
            let node = node.clone();
            actions = actions.child(ui::action(
                format!("pr-action-{kind}"),
                label,
                None,
                false,
                cx.listener(move |this, _: &(), _, cx| {
                    if !this.pr_require_scope(cx) {
                        return;
                    }
                    let v = &mut this.pull_requests;
                    if v.busy {
                        return;
                    }
                    if kind == 2 && draft.is_none() {
                        v.error = Some(
                            "Provider draft state is unknown. Refresh before changing it.".into(),
                        );
                        cx.notify();
                        return;
                    }
                    let body = v.body.read(cx).text().to_owned();
                    v.confirmation = Some(match kind {
                        0 => PrAction::Comment { number, body },
                        1 => PrAction::State {
                            number,
                            open: !is_open,
                        },
                        2 => PrAction::Draft {
                            node_id: node.clone(),
                            draft: !is_draft,
                        },
                        3 | 4 | 6 => PrAction::Review {
                            number,
                            sha: sha.clone(),
                            body,
                            kind: match kind {
                                3 => ReviewKind::Approve,
                                4 => ReviewKind::RequestChanges,
                                _ => ReviewKind::Comment,
                            },
                        },
                        _ => PrAction::Merge {
                            number,
                            sha: sha.clone(),
                            method: MergeMethod::Squash,
                        },
                    });
                    cx.notify();
                }),
            ));
        }
        pane.child(actions).into_any_element()
    }
}
fn text(value: &Value, key: &str) -> String {
    value[key]
        .as_str()
        .unwrap_or("unavailable")
        .chars()
        .take(8192)
        .collect()
}
