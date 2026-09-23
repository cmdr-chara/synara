use super::*;

#[derive(Clone, Copy, PartialEq)]
pub(super) enum View {
    Branches,
    Remotes,
    Worktrees,
    Stashes,
}
impl View {
    pub fn title(self) -> &'static str {
        match self {
            Self::Branches => "Branches",
            Self::Remotes => "Remotes",
            Self::Worktrees => "Worktrees",
            Self::Stashes => "Stashes",
        }
    }
}
#[derive(Default)]
pub(super) struct Catalog {
    pub branches: Vec<Branch>,
    pub remotes: Vec<String>,
    pub worktrees: Vec<Worktree>,
    pub stashes: Vec<Stash>,
}
pub(super) struct Branch {
    pub name: String,
    pub object: String,
    pub current: bool,
}
pub(super) struct Worktree {
    pub path: PathBuf,
    pub branch: String,
    pub locked: bool,
}
pub(super) struct Stash {
    pub object: String,
    pub subject: String,
}
impl Catalog {
    pub fn current(&self) -> String {
        self.branches
            .iter()
            .find(|b| b.current)
            .map(|b| b.name.clone())
            .unwrap_or_default()
    }
    pub fn parse(output: &[String]) -> Result<Self, String> {
        let mut result = Self::default();
        for line in output[0].lines().filter(|line| !line.is_empty()) {
            let fields: Vec<_> = line.split('\0').collect();
            if fields.len() < 3 || !fields[0].starts_with("refs/heads/") {
                return Err("Unexpected Git branch listing.".into());
            }
            result.branches.push(Branch {
                name: fields[0][11..].into(),
                object: fields[1].into(),
                current: fields[2] == "*",
            });
        }
        result.remotes = output[1]
            .lines()
            .filter(|line| !line.is_empty())
            .map(str::to_owned)
            .collect();
        for record in output[2].split("\0\0").filter(|record| !record.is_empty()) {
            let mut path = None;
            let mut branch = "Detached HEAD".to_owned();
            let mut locked = false;
            for field in record.split('\0') {
                if let Some(value) = field.strip_prefix("worktree ") {
                    path = Some(PathBuf::from(value));
                }
                if let Some(value) = field.strip_prefix("branch refs/heads/") {
                    branch = value.into();
                }
                if field == "bare" {
                    branch = "Bare repository".into();
                    locked = true;
                }
                if field.starts_with("locked") {
                    locked = true;
                }
            }
            if let Some(path) = path {
                result.worktrees.push(Worktree {
                    path,
                    branch,
                    locked,
                });
            }
        }
        let fields: Vec<_> = output[3].split_terminator('\0').collect();
        for pair in fields.chunks(2) {
            if pair.len() != 2 {
                return Err("Unexpected Git stash listing.".into());
            }
            result.stashes.push(Stash {
                object: pair[0].trim_start_matches('\n').into(),
                subject: pair[1].into(),
            });
        }
        Ok(result)
    }
}

#[derive(Clone)]
pub(super) enum Action {
    CreateBranch,
    Switch(String),
    Rename(String),
    Delete(String),
    AddRemote,
    EditRemote(String),
    RemoveRemote(String),
    Fetch(String),
    Pull(String),
    Push(String),
    AddWorktree,
    RemoveWorktree(PathBuf),
    SaveStash,
    ApplyStash(String),
}
impl Action {
    pub fn title(&self) -> &'static str {
        match self {
            Self::CreateBranch => "Create branch",
            Self::Switch(_) => "Switch branch",
            Self::Rename(_) => "Rename branch",
            Self::Delete(_) => "Delete merged branch",
            Self::AddRemote => "Add remote",
            Self::EditRemote(_) => "Change remote URL",
            Self::RemoveRemote(_) => "Remove remote",
            Self::Fetch(_) => "Fetch branch",
            Self::Pull(_) => "Pull fast-forward",
            Self::Push(_) => "Push branch",
            Self::AddWorktree => "Add worktree",
            Self::RemoveWorktree(_) => "Remove worktree",
            Self::SaveStash => "Save stash",
            Self::ApplyStash(_) => "Apply stash",
        }
    }
    pub fn network(&self) -> bool {
        matches!(self, Self::Fetch(_) | Self::Pull(_) | Self::Push(_))
    }
    pub fn executes_repository(&self) -> bool {
        self.network()
            || matches!(
                self,
                Self::Switch(_)
                    | Self::AddWorktree
                    | Self::RemoveWorktree(_)
                    | Self::SaveStash
                    | Self::ApplyStash(_)
            )
    }
    pub fn description(&self) -> String {
        match self {
            Self::CreateBranch => "Create a local branch without switching the current worktree.".into(),
            Self::Switch(name) => format!("Switch this workspace to {name}. Git will refuse to overwrite conflicting local changes."),
            Self::Rename(name) => format!("Rename local branch {name}. Remote branches are not renamed."),
            Self::Delete(name) => format!("Delete {name} only when Git considers it merged. Type its exact name to confirm. No force deletion."),
            Self::AddRemote => "Add a named remote using an HTTPS or ssh:// URL. Do not include credentials in the URL.".into(),
            Self::EditRemote(name) => format!("Replace the URL for {name}. No network operation is performed."),
            Self::RemoveRemote(name) => format!("Remove {name} and its local remote-tracking references. Type the exact remote name to confirm. The server is unchanged."),
            Self::Fetch(name) => format!("Fetch one branch from {name}. Other branches and tags are not fetched."),
            Self::Pull(name) => format!("Fast-forward the CURRENT local branch from one branch on {name}. No merge commit, rebase or automatic stash."),
            Self::Push(name) => format!("Publish the specified local branch to {name}. This writes to a remote repository. No force push or deletion."),
            Self::AddWorktree => "Check out an existing local branch into a new absolute directory on the selected host.".into(),
            Self::RemoveWorktree(path) => format!("Remove the worktree directory {}. Git refuses dirty or locked worktrees. Type the exact absolute path to confirm.", path.display()),
            Self::SaveStash => "Save tracked changes and remove them from the current worktree. Include untracked files only by explicit choice.".into(),
            Self::ApplyStash(id) => format!("Apply stash {} to the current worktree. The stash is retained, including if conflicts occur.", id.chars().take(12).collect::<String>()),
        }
    }
    pub fn fields(&self, catalog: &Catalog) -> Vec<(&'static str, String)> {
        match self {
            Self::CreateBranch => vec![
                ("New branch name", String::new()),
                ("Start: HEAD, full ref or object ID", "HEAD".into()),
            ],
            Self::Rename(_) => vec![("New branch name", String::new())],
            Self::Delete(_) | Self::RemoveRemote(_) => {
                vec![("Type the exact name to confirm", String::new())]
            }
            Self::AddRemote => vec![
                ("Remote name", String::new()),
                ("Remote URL (no credentials)", String::new()),
            ],
            Self::EditRemote(_) => vec![("New remote URL (no credentials)", String::new())],
            Self::Fetch(_) | Self::Pull(_) => vec![("Remote branch", catalog.current())],
            Self::Push(_) => vec![
                ("Local branch to publish", catalog.current()),
                ("Destination branch on remote", catalog.current()),
            ],
            Self::AddWorktree => vec![
                ("New absolute directory on selected host", String::new()),
                ("Existing local branch", String::new()),
            ],
            Self::RemoveWorktree(_) => {
                vec![("Type the exact absolute path to confirm", String::new())]
            }
            Self::SaveStash => vec![("Stash message", String::new())],
            _ => vec![],
        }
    }
    pub fn operation(
        &self,
        values: &[String],
        untracked: bool,
    ) -> Result<GitOperation, &'static str> {
        if values.iter().any(|s| s.trim().is_empty()) {
            return Err("Complete all fields before continuing.");
        }
        Ok(match self {
            Self::CreateBranch => GitOperation::CreateBranch {
                name: values[0].clone(),
                start: values[1].clone(),
            },
            Self::Switch(name) => GitOperation::SwitchBranch { name: name.clone() },
            Self::Rename(old) => GitOperation::RenameBranch {
                old: old.clone(),
                new: values[0].clone(),
            },
            Self::Delete(name) => {
                if values[0] != *name {
                    return Err("The confirmation must match the exact branch name.");
                }
                GitOperation::DeleteBranch { name: name.clone() }
            }
            Self::AddRemote => GitOperation::AddRemote {
                name: values[0].clone(),
                url: values[1].clone(),
            },
            Self::EditRemote(name) => GitOperation::SetRemoteUrl {
                name: name.clone(),
                url: values[0].clone(),
            },
            Self::RemoveRemote(name) => {
                if values[0] != *name {
                    return Err("The confirmation must match the exact remote name.");
                }
                GitOperation::RemoveRemote { name: name.clone() }
            }
            Self::Fetch(remote) => GitOperation::Fetch {
                remote: remote.clone(),
                branch: values[0].clone(),
            },
            Self::Pull(remote) => GitOperation::PullFastForward {
                remote: remote.clone(),
                branch: values[0].clone(),
            },
            Self::Push(remote) => GitOperation::Push {
                remote: remote.clone(),
                local_branch: values[0].clone(),
                remote_branch: values[1].clone(),
            },
            Self::AddWorktree => GitOperation::AddWorktree {
                path: values[0].clone().into(),
                branch: values[1].clone(),
            },
            Self::RemoveWorktree(path) => {
                if std::path::Path::new(&values[0]) != path.as_path() {
                    return Err("The confirmation must match the exact worktree path.");
                }
                GitOperation::RemoveWorktree { path: path.clone() }
            }
            Self::SaveStash => GitOperation::SaveStash {
                message: values[0].clone(),
                include_untracked: untracked,
            },
            Self::ApplyStash(object_id) => GitOperation::ApplyStash {
                object_id: object_id.clone(),
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_keeps_names_and_worktree_paths_without_whitespace_splitting() {
        let catalog = Catalog::parse(&[
            format!("refs/heads/main\0{}\0*\0\n", "a".repeat(40)),
            "origin\n".into(),
            "worktree /tmp/project with spaces\0HEAD abc\0branch refs/heads/main\0\0worktree /tmp/other\0branch refs/heads/topic\0locked reason\0\0".into(),
            format!("{}\0On main: keep this\0", "b".repeat(40)),
        ]).unwrap();
        assert_eq!(catalog.current(), "main");
        assert_eq!(
            catalog.worktrees[0].path,
            PathBuf::from("/tmp/project with spaces")
        );
        assert!(catalog.worktrees[1].locked);
        assert_eq!(catalog.stashes[0].subject, "On main: keep this");
    }
    #[test]
    fn destructive_actions_require_exact_confirmations() {
        assert!(
            Action::Delete("topic".into())
                .operation(&["wrong".into()], false)
                .is_err()
        );
        assert!(
            Action::RemoveRemote("origin".into())
                .operation(&["".into()], false)
                .is_err()
        );
        assert!(
            Action::RemoveWorktree("/tmp/topic".into())
                .operation(&["/tmp/topic-other".into()], false)
                .is_err()
        );
        assert!(matches!(
            Action::Delete("topic".into())
                .operation(&["topic".into()], false)
                .unwrap(),
            GitOperation::DeleteBranch { .. }
        ));
    }
    #[test]
    fn network_and_worktree_actions_require_additional_execution_consent() {
        assert!(Action::Push("origin".into()).network());
        assert!(Action::Pull("origin".into()).executes_repository());
        assert!(Action::SaveStash.executes_repository());
        assert!(Action::ApplyStash("a".repeat(40)).executes_repository());
        assert!(!Action::CreateBranch.executes_repository());
        assert!(!Action::AddRemote.network());
    }
}
