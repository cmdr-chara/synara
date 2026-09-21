use super::*;
use crate::ui::{self, Glyph, palette};

impl RepositoryPanel {
    fn action(&self, id: (&'static str, usize), label: &str, action: Action, cx: &mut Context<Self>) -> gpui::AnyElement {
        ui::action(id, label.to_owned(), None, false, cx.listener(move |this, _: &(), _, cx| this.open(action.clone(), cx)))
            .text_size(px(12.)).relative().child(ui::layout_probe_slot(id.0, id.1)).when(self.busy, |el| el.opacity(0.45)).into_any_element()
    }
    fn row(&self, label: String, detail: String) -> gpui::Div {
        div().min_w_0().p_2().border_b_1().border_color(rgb(palette().border)).flex().flex_col().gap_1()
            .child(div().min_w_0().text_ellipsis().text_size(px(13.)).child(label))
            .child(div().min_w_0().text_ellipsis().text_size(px(11.)).text_color(rgb(palette().muted)).child(detail))
    }
    fn list(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let query = self.query.read(cx).text().to_lowercase();
        let mut rows = Vec::new();
        match self.view {
            View::Branches => for (i, branch) in self.catalog.branches.iter().filter(|b| b.name.to_lowercase().contains(&query)).take(400).enumerate() {
                let name = branch.name.clone();
                let copy = name.clone();
                let mut actions = div().flex().flex_wrap().gap_1();
                if !branch.current { actions = actions.child(self.action(("repo-switch", i), "Switch", Action::Switch(name.clone()), cx)); }
                actions = actions.child(self.action(("repo-rename", i), "Rename", Action::Rename(name.clone()), cx));
                if !branch.current { actions = actions.child(self.action(("repo-delete", i), "Delete", Action::Delete(name.clone()), cx)); }
                actions = actions.child(ui::action(("repo-copy-branch", i), "Copy", Some(Glyph::Copy), false, move |_: &(), _, cx| cx.write_to_clipboard(gpui::ClipboardItem::new_string(copy.clone()))).text_size(px(12.)));
                rows.push(self.row(name, format!("{}{}", if branch.current { "Current branch · " } else { "" }, branch.object.chars().take(12).collect::<String>())).child(actions).into_any_element());
            },
            View::Remotes => for (i, name) in self.catalog.remotes.iter().filter(|n| n.to_lowercase().contains(&query)).take(400).enumerate() {
                rows.push(self.row(name.clone(), "Credentials stay on the selected host. No URL is logged or copied automatically.".into())
                    .child(div().flex().flex_wrap().gap_1()
                        .child(self.action(("repo-fetch", i), "Fetch", Action::Fetch(name.clone()), cx))
                        .child(self.action(("repo-pull", i), "Pull FF", Action::Pull(name.clone()), cx))
                        .child(self.action(("repo-push", i), "Push", Action::Push(name.clone()), cx))
                        .child(self.action(("repo-url", i), "Edit URL", Action::EditRemote(name.clone()), cx))
                        .child(self.action(("repo-remove-remote", i), "Remove", Action::RemoveRemote(name.clone()), cx)))
                    .into_any_element());
            },
            View::Worktrees => for (i, item) in self.catalog.worktrees.iter().enumerate().filter(|(_, w)| format!("{} {}", w.path.display(), w.branch).to_lowercase().contains(&query)).take(400) {
                let path = item.path.display().to_string();
                let copy = path.clone();
                let mut actions = div().flex().flex_wrap().gap_1()
                    .child(ui::action(("repo-copy-worktree", i), "Copy path", Some(Glyph::Copy), false, move |_: &(), _, cx| cx.write_to_clipboard(gpui::ClipboardItem::new_string(copy.clone()))).text_size(px(12.)));
                // Git does not permit removing its primary or locked worktree.
                if i > 0 && !item.locked && item.path != *self.target.root() {
                    actions = actions.child(self.action(("repo-remove-worktree", i), "Remove", Action::RemoveWorktree(item.path.clone()), cx));
                }
                rows.push(self.row(path, format!("{}{}", item.branch, if item.locked { " · locked/bare" } else { "" })).child(actions).into_any_element());
            },
            View::Stashes => for (i, item) in self.catalog.stashes.iter().filter(|s| s.subject.to_lowercase().contains(&query)).take(400).enumerate() {
                let copy = item.object.clone();
                rows.push(self.row(item.subject.clone(), item.object.chars().take(12).collect())
                    .child(div().flex().flex_wrap().gap_1()
                        .child(self.action(("repo-apply-stash", i), "Apply and keep", Action::ApplyStash(item.object.clone()), cx))
                        .child(ui::action(("repo-copy-stash", i), "Copy ID", Some(Glyph::Copy), false, move |_: &(), _, cx| cx.write_to_clipboard(gpui::ClipboardItem::new_string(copy.clone()))).text_size(px(12.))))
                    .into_any_element());
            },
        }
        let empty = rows.is_empty();
        div().id("repository-items").min_h_0().min_w_0().flex_1().overflow_y_scroll().flex().flex_col()
            .children(rows)
            .children((empty && !self.busy).then(|| div().p_4().text_size(px(12.)).text_color(rgb(palette().muted)).child(if self.error.is_some() { "The repository could not be fully loaded. Refresh to retry." } else { "No matching items. Create one or change the filter." })))
            .into_any_element()
    }
    fn form_view(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let form = self.form.as_ref().unwrap();
        let labels = form.action.fields(&self.catalog);
        div().id("repository-form").relative().child(ui::layout_probe("repository-form"))
            .min_h_0().min_w_0().flex_1().overflow_y_scroll().flex().flex_col().p_3().gap_3()
            .child(div().text_size(px(16.)).child(form.action.title()))
            .child(div().text_size(px(12.)).text_color(rgb(palette().muted)).child(form.action.description()))
            .child(div().text_size(px(12.)).child(format!("Current branch: {}", self.catalog.current())))
            .children(form.fields.iter().enumerate().map(|(index, input)| {
                div().min_w_0().flex().flex_col().gap_1().text_size(px(11.))
                    .child(labels[index].0).relative().child(ui::layout_probe_slot("repo-field", index))
                    .child(if self.busy { div().child(input.read(cx).text().to_owned()).into_any_element() } else { input.clone().into_any_element() })
            }))
            .when(matches!(form.action, Action::SaveStash), |el| el.child(ui::action("repo-untracked", "Include untracked files", Some(if form.untracked { Glyph::Check } else { Glyph::Plus }), form.untracked,
                cx.listener(|this, _: &(), _, cx| { if !this.busy && let Some(form) = this.form.as_mut() { form.untracked = !form.untracked; cx.notify(); } })).text_size(px(12.))))
            .when(form.action.executes_repository(), |el| el.child(ui::action("repo-execution-consent", "Allow repository filters/helpers for this operation", Some(if form.execution { Glyph::Check } else { Glyph::Shield }), form.execution,
                cx.listener(|this, _: &(), _, cx| { if !this.busy && let Some(form) = this.form.as_mut() { form.execution = !form.execution; cx.notify(); } })).text_size(px(12.)).relative().child(ui::layout_probe("repo-execution-consent")))
                .child(div().text_size(px(11.)).text_color(rgb(palette().muted)).child("Git can run repository-configured filters and helpers. This approval is only for this action, not for the agent. Hooks and signing remain disabled.")))
            .when(form.action.network(), |el| el
                .child(ui::action("repo-credentials", "Use configured noninteractive credentials", Some(if form.credentials { Glyph::Check } else { Glyph::Shield }), form.credentials,
                    cx.listener(|this, _: &(), _, cx| { if !this.busy && let Some(form) = this.form.as_mut() { form.credentials = !form.credentials; cx.notify(); } })).text_size(px(12.)))
                .child(ui::action("repo-ssh", "Allow SSH transport as well as HTTPS", Some(if form.ssh { Glyph::Check } else { Glyph::Shield }), form.ssh,
                    cx.listener(|this, _: &(), _, cx| { if !this.busy && let Some(form) = this.form.as_mut() { form.ssh = !form.ssh; cx.notify(); } })).text_size(px(12.)))
                .child(div().text_size(px(11.)).text_color(rgb(palette().muted)).child("Default: HTTPS with credential helpers disabled. No credential prompt or token storage. Configure authentication on the selected host.")))
            .child(div().flex().flex_wrap().gap_2()
                .child(ui::action("repo-confirm", if self.busy { "Running..." } else { "Confirm operation" }, Some(Glyph::Check), false,
                    cx.listener(|this, _: &(), _, cx| this.execute(cx))).relative().child(ui::layout_probe("repo-confirm")))
                .child(ui::action("repo-cancel", "Cancel", None, false,
                    cx.listener(|this, _: &(), window, cx| { if !this.busy { this.form = None; this.error = None; window.focus(&this.focus, cx); cx.notify(); } }))))
            .into_any_element()
    }
}
impl Render for RepositoryPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.needs_focus {
            self.needs_focus = false;
            let focus = self.form.as_ref().and_then(|form| form.fields.first()).map(|field| field.read(cx).focus_handle(cx)).unwrap_or_else(|| self.focus.clone());
            window.focus(&focus, cx);
        }
        let total = match self.view { View::Branches => self.catalog.branches.len(), View::Remotes => self.catalog.remotes.len(), View::Worktrees => self.catalog.worktrees.len(), View::Stashes => self.catalog.stashes.len() };
        let add = match self.view { View::Branches => Action::CreateBranch, View::Remotes => Action::AddRemote, View::Worktrees => Action::AddWorktree, View::Stashes => Action::SaveStash };
        div().id("repository-panel").track_focus(&self.focus).relative().child(ui::layout_probe("repository-panel"))
            .flex().flex_col().flex_1().min_h_0().min_w_0().bg(gpui::rgba(0))
            .child(div().px_3().py_2().flex().flex_col().gap_1()
                .child(div().text_size(px(15.)).child("Repository"))
                .child(div().text_size(px(11.)).min_w_0().text_ellipsis().text_color(rgb(palette().muted)).child(format!("{} · {}", if matches!(self.target, WorkspaceTarget::Local { .. }) { "Local" } else { "SSH" }, self.target.root().display()))))
            .child(div().px_2().flex().flex_wrap().gap_1().children([View::Branches, View::Remotes, View::Worktrees, View::Stashes].into_iter().enumerate().map(|(index, view)| {
                ui::action(("repository-view", index), view.title(), None, self.view == view,
                    cx.listener(move |this, _: &(), _, cx| { if this.form.is_none() { this.view = view; this.query.update(cx, |q, cx| q.clear(cx)); cx.notify(); } })).text_size(px(12.)).relative().child(ui::layout_probe_slot("repository-view", index))
            })))
            .when(self.form.is_none(), |el| el.child(div().px_2().py_2().flex().flex_wrap().gap_1()
                .child(div().flex_1().min_w(px(100.)).child(self.query.clone()))
                .child(self.action(("repo-new", 0), add.title(), add.clone(), cx))
                .child(ui::chrome_button("repo-refresh", "Refresh repository", Glyph::Restore, self.busy, cx.listener(|this, _: &(), _, cx| this.refresh(cx))))))
            .children(self.error.as_ref().map(|error| div().p_3().text_size(px(12.)).text_color(rgb(palette().error)).child(error.clone())))
            .children(self.notice.as_ref().map(|notice| div().px_3().py_2().text_size(px(12.)).text_color(rgb(palette().muted)).child(notice.clone())))
            .children(self.busy.then(|| div().px_3().py_2().text_size(px(12.)).child("Git is running on the selected host. No automatic retries.")))
            .child(if self.form.is_some() { self.form_view(cx) } else { self.list(cx) })
            .child(div().px_3().py_2().text_size(px(11.)).text_color(rgb(palette().muted)).child(format!("{total} items · Showing up to 400 filtered results. No force operations.")))
    }
}
