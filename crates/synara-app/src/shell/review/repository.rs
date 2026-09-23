//! Native repository tools over the existing host-aware, bounded Git API.
//! A panel keeps its original project/root even while the user navigates elsewhere.
mod model;
mod view;
use super::*;
use gpui::FocusHandle;
use model::{Action, Catalog as RepositoryCatalog, View};

pub(super) struct RepositoryPanel {
    target: WorkspaceTarget,
    workspace: WorkspaceService,
    runtime: Handle,
    catalog: RepositoryCatalog,
    view: View,
    worktrees_only: bool,
    query: Entity<TextEntry>,
    form: Option<Form>,
    busy: bool,
    error: Option<String>,
    notice: Option<String>,
    needs_focus: bool,
    focus: FocusHandle,
    _subscription: Subscription,
}
struct Form {
    action: Action,
    fields: Vec<Entity<TextEntry>>,
    execution: bool,
    credentials: bool,
    ssh: bool,
    untracked: bool,
}
impl RepositoryPanel {
    pub(super) fn show_worktrees(&mut self, cx: &mut Context<Self>) {
        if self.form.is_none() {
            self.view = View::Worktrees;
            self.worktrees_only = true;
            self.query.update(cx, |query, cx| query.clear(cx));
            cx.notify();
        }
    }
    pub(super) fn show_repository_tabs(&mut self, cx: &mut Context<Self>) {
        self.worktrees_only = false;
        cx.notify();
    }
    pub(super) fn new(
        target: WorkspaceTarget,
        workspace: WorkspaceService,
        runtime: Handle,
        cx: &mut Context<Self>,
    ) -> Self {
        let query = cx
            .new(|cx| TextEntry::new("Filter repository items...", EntryMode::SingleLine, 32., cx));
        let subscription = cx.subscribe(&query, |_, _, _, cx| cx.notify());
        let mut panel = Self {
            target,
            workspace,
            runtime,
            catalog: RepositoryCatalog::default(),
            view: View::Branches,
            worktrees_only: false,
            query,
            form: None,
            busy: false,
            error: None,
            notice: None,
            needs_focus: false,
            focus: cx.focus_handle(),
            _subscription: subscription,
        };
        panel.refresh(cx);
        panel
    }
    pub(super) fn pending(&self) -> bool {
        self.busy || self.form.is_some()
    }
    pub(super) fn busy(&self) -> bool {
        self.busy
    }
    fn refresh(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.busy = true;
        self.error = None;
        let target = self.target.clone();
        let workspace = self.workspace.clone();
        let (send, receive) = tokio::sync::oneshot::channel();
        self.runtime.spawn(async move {
            let _ = send.send(load(workspace, target).await);
        });
        cx.spawn(async move |panel, cx| {
            let result = receive
                .await
                .unwrap_or_else(|_| Err("Repository worker stopped. Refresh to retry.".into()));
            let _ = panel.update(cx, |this, cx| {
                this.busy = false;
                match result {
                    Ok(catalog) => {
                        this.catalog = catalog;
                        tracing::debug!(target: "synara_ui_layout", "repository-ready");
                    }
                    Err(error) => this.error = Some(error),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
    fn open(&mut self, action: Action, cx: &mut Context<Self>) {
        if self.busy || self.form.is_some() {
            return;
        }
        let fields = action
            .fields(&self.catalog)
            .into_iter()
            .map(|(label, initial)| {
                cx.new(|cx| {
                    let mut input = TextEntry::new(label, EntryMode::SingleLine, 34., cx);
                    input.set_text(initial, cx);
                    input
                })
            })
            .collect();
        self.form = Some(Form {
            action,
            fields,
            execution: false,
            credentials: false,
            ssh: false,
            untracked: false,
        });
        self.error = None;
        self.notice = None;
        self.needs_focus = true;
        cx.notify();
    }
    fn execute(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let Some(form) = &self.form else { return };
        let values: Vec<_> = form
            .fields
            .iter()
            .map(|field| field.read(cx).text().to_owned())
            .collect();
        let operation = match form.action.operation(&values, form.untracked) {
            Ok(operation) => operation,
            Err(error) => {
                self.error = Some(error.into());
                cx.notify();
                return;
            }
        };
        if form.action.executes_repository() && !form.execution {
            self.error = Some("Confirm repository execution before proceeding. Git filters and helpers can run code.".into());
            cx.notify();
            return;
        }
        let policy = GitOperationPolicy {
            allow_mutation: true,
            allow_repository_execution: form.execution,
            credentials: if form.credentials {
                GitCredentialPolicy::ConfiguredNoninteractive
            } else {
                GitCredentialPolicy::Disabled
            },
            network: if !form.action.network() {
                GitNetworkPolicy::Disabled
            } else if form.ssh {
                GitNetworkPolicy::HttpsAndSsh
            } else {
                GitNetworkPolicy::Https
            },
            ..Default::default()
        };
        let title = form.action.title().to_owned();
        self.busy = true;
        self.error = None;
        let workspace = self.workspace.clone();
        let target = self.target.clone();
        let (send, receive) = tokio::sync::oneshot::channel();
        self.runtime.spawn(async move {
            let result = async {
                let git = backend(workspace, target).await?;
                git.execute(
                    operation,
                    GitOperationOptions {
                        policy,
                        ..Default::default()
                    },
                    Default::default(),
                    None,
                )
                .await
                .map_err(|error| error.to_string())?;
                Ok::<_, String>(())
            }
            .await;
            let _ = send.send(result);
        });
        cx.spawn(async move |panel, cx| {
            let result = receive.await.unwrap_or_else(|_| Err("Repository worker stopped. Inspect Git before retrying. The operation may have changed the repository.".into()));
            let _ = panel.update(cx, |this, cx| {
                this.busy = false;
                match result {
                    Ok(()) => { this.form = None; this.notice = Some(format!("{title} completed. Refresh Changes to inspect the result.")); this.refresh(cx); }
                    Err(error) => this.error = Some(format!("{error}. No automatic rollback was attempted. Your form is retained.")),
                }
                cx.notify();
            });
        }).detach();
        cx.notify();
    }
}

async fn backend(
    workspace: WorkspaceService,
    target: WorkspaceTarget,
) -> Result<GitOperations, String> {
    match target {
        WorkspaceTarget::Local { root } => Ok(GitOperations::new(root)),
        WorkspaceTarget::Ssh {
            workspace: remote,
            root,
        } => {
            let profile = workspace
                .ssh_profile(remote.id)
                .await
                .map_err(|error| error.to_string())?
                .ok_or_else(|| "The remote workspace has no pinned SSH profile.".to_owned())?;
            let host = profile.host(&remote).map_err(|error| error.to_string())?;
            Ok(GitOperations::with_host(root, Arc::new(host)))
        }
    }
}
async fn load(
    workspace: WorkspaceService,
    target: WorkspaceTarget,
) -> Result<RepositoryCatalog, String> {
    let git = backend(workspace, target).await?;
    let mut output = Vec::new();
    for operation in [
        GitOperation::Branches,
        GitOperation::RemoteNames,
        GitOperation::Worktrees,
        GitOperation::Stashes,
    ] {
        let result = git
            .execute(
                operation,
                GitOperationOptions::default(),
                Default::default(),
                None,
            )
            .await
            .map_err(|error| error.to_string())?;
        output.push(String::from_utf8(result.stdout).map_err(|_| {
            "Git returned a non-UTF-8 name. Use Git directly for this repository.".to_owned()
        })?);
    }
    RepositoryCatalog::parse(&output)
}
