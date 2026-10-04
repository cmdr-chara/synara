use super::{
    GitCredentialPolicy, GitHookPolicy, GitNetworkPolicy, GitOperation, GitOperationErrorKind,
    GitOperationPolicy, GitSigningPolicy,
};
use std::path::PathBuf;

type Result<T> = std::result::Result<T, GitOperationErrorKind>;

pub(super) struct Plan {
    pub args: Vec<String>,
    pub mutation: bool,
    pub push_porcelain: bool,
}

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

pub(super) fn branch(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 1024
        || value.starts_with('-')
        || value.starts_with('/')
        || value.ends_with('/')
        || value.ends_with('.')
        || value == "@"
        || value.contains("..")
        || value.contains("@{")
        || value.contains("//")
        || value
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || "~^:?*[\\".contains(c))
        || value
            .split('/')
            .any(|part| part.starts_with('.') || part.ends_with(".lock"))
    {
        return Err(GitOperationErrorKind::InvalidInput);
    }
    Ok(())
}

fn remote(value: &str) -> Result<()> {
    branch(value)?;
    if value.len() > 128 || value.contains('/') || !value.is_ascii() {
        return Err(GitOperationErrorKind::InvalidInput);
    }
    Ok(())
}

fn message(value: &str, max: usize) -> Result<()> {
    if value.trim().is_empty() || value.len() > max || value.contains('\0') {
        return Err(GitOperationErrorKind::InvalidInput);
    }
    Ok(())
}

fn object_id(value: &str) -> Result<()> {
    if !matches!(value.len(), 40 | 64) || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(GitOperationErrorKind::InvalidInput);
    }
    Ok(())
}

fn start(value: &str) -> Result<()> {
    if value == "HEAD" || object_id(value).is_ok() {
        return Ok(());
    }
    if let Some(name) = value.strip_prefix("refs/heads/") {
        return branch(name);
    }
    if let Some(name) = value.strip_prefix("refs/remotes/") {
        return branch(name);
    }
    Err(GitOperationErrorKind::InvalidInput)
}

fn path(value: PathBuf, local: bool) -> Result<String> {
    let value = value
        .into_os_string()
        .into_string()
        .map_err(|_| GitOperationErrorKind::InvalidInput)?;
    if value.is_empty()
        || value.len() > 4096
        || value.chars().any(char::is_control)
        || (local && !std::path::Path::new(&value).is_absolute())
        || (!local && !value.starts_with('/'))
        || value
            .replace('\\', "/")
            .split('/')
            .any(|p| matches!(p, "." | ".."))
    {
        return Err(GitOperationErrorKind::InvalidInput);
    }
    Ok(value)
}

fn url(value: &str) -> Result<()> {
    if value.is_empty() || value.len() > 4096 || value.chars().any(char::is_control) {
        return Err(GitOperationErrorKind::InvalidInput);
    }
    // Explicit filesystem remotes can contain spaces. Execution still needs a
    // LocalFilesystem grant, and happens on the selected host only.
    if value.starts_with('/')
        || (value.len() >= 3
            && value.as_bytes()[0].is_ascii_alphabetic()
            && value.as_bytes()[1] == b':'
            && matches!(value.as_bytes()[2], b'/' | b'\\'))
    {
        return Ok(());
    }
    let (rest, ssh) = if let Some(rest) = value.strip_prefix("https://") {
        (rest, false)
    } else if let Some(rest) = value.strip_prefix("ssh://") {
        (rest, true)
    } else {
        return Err(GitOperationErrorKind::InvalidInput);
    };
    if value.chars().any(char::is_whitespace) || value.contains(['?', '#']) {
        return Err(GitOperationErrorKind::InvalidInput);
    }
    let authority = rest.split('/').next().unwrap_or_default();
    let host = if ssh {
        if let Some((user, host)) = authority.split_once('@') {
            if user.is_empty()
                || !user
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
            {
                return Err(GitOperationErrorKind::InvalidInput);
            }
            host
        } else {
            authority
        }
    } else {
        authority
    };
    if host.is_empty()
        || host.starts_with('-')
        || !host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".:-[]".contains(&b))
    {
        return Err(GitOperationErrorKind::InvalidInput);
    }
    Ok(())
}

pub(super) fn build(
    operation: GitOperation,
    policy: &GitOperationPolicy,
    local: bool,
) -> Result<Plan> {
    use GitOperation::*;
    let updating_remote = matches!(&operation, SetRemoteUrl { .. });
    let push_porcelain = matches!(&operation, Push { .. });
    let (command, mutation, executes_repository, network) = match operation {
        Branches => (
            args(&[
                "for-each-ref",
                "--format=%(refname)%00%(objectname)%00%(HEAD)%00",
                "refs/heads/",
            ]),
            false,
            false,
            false,
        ),
        RemoteNames => (args(&["remote"]), false, false, false),
        RemoteUrl { name, push } => {
            remote(&name)?;
            let mut command = args(&["remote", "get-url", "--all"]);
            if push {
                command.push("--push".into());
            }
            command.extend(args(&["--", &name]));
            (command, false, false, false)
        }
        Worktrees => (
            args(&["worktree", "list", "--porcelain", "-z"]),
            false,
            false,
            false,
        ),
        Stashes => (
            args(&["stash", "list", "-z", "--format=%H%x00%gs"]),
            false,
            false,
            false,
        ),
        CreateBranch { name, start: at } => {
            branch(&name)?;
            start(&at)?;
            (
                args(&["branch", "--no-track", "--", &name, &at]),
                true,
                false,
                false,
            )
        }
        RenameBranch { old, new } => {
            branch(&old)?;
            branch(&new)?;
            (
                args(&["branch", "-m", "--", &old, &new]),
                true,
                false,
                false,
            )
        }
        DeleteBranch { name } => {
            branch(&name)?;
            (args(&["branch", "-d", "--", &name]), true, false, false)
        }
        SwitchBranch { name } => {
            branch(&name)?;
            (
                args(&[
                    "switch",
                    "--no-guess",
                    "--no-recurse-submodules",
                    "--",
                    &name,
                ]),
                true,
                true,
                false,
            )
        }
        AddRemote {
            name,
            url: endpoint,
        }
        | SetRemoteUrl {
            name,
            url: endpoint,
        } => {
            remote(&name)?;
            url(&endpoint)?;
            let verb = if updating_remote { "set-url" } else { "add" };
            (
                args(&["remote", verb, "--", &name, &endpoint]),
                true,
                false,
                false,
            )
        }
        RemoveRemote { name } => {
            remote(&name)?;
            (args(&["remote", "remove", "--", &name]), true, false, false)
        }
        Fetch {
            remote: name,
            branch: selected,
        } => {
            remote(&name)?;
            branch(&selected)?;
            let refspec = format!("refs/heads/{selected}:refs/remotes/{name}/{selected}");
            (
                args(&[
                    "fetch",
                    "--no-tags",
                    "--no-recurse-submodules",
                    "--no-write-fetch-head",
                    "--no-auto-maintenance",
                    "--refmap=",
                    "--",
                    &name,
                    &refspec,
                ]),
                true,
                true,
                true,
            )
        }
        PullFastForward {
            remote: name,
            branch: selected,
        } => {
            remote(&name)?;
            branch(&selected)?;
            let source = format!("refs/heads/{selected}");
            (
                args(&[
                    "-c",
                    "merge.autoStash=false",
                    "pull",
                    "--ff-only",
                    "--no-rebase",
                    "--no-autostash",
                    "--no-recurse-submodules",
                    "--no-tags",
                    "--no-edit",
                    "--",
                    &name,
                    &source,
                ]),
                true,
                true,
                true,
            )
        }
        Push {
            remote: name,
            local_branch,
            remote_branch,
        } => {
            remote(&name)?;
            branch(&local_branch)?;
            branch(&remote_branch)?;
            let mirror = format!("remote.{name}.mirror=false");
            let refspec = format!("refs/heads/{local_branch}:refs/heads/{remote_branch}");
            (
                args(&[
                    "-c",
                    &mirror,
                    "push",
                    "--porcelain",
                    "--no-follow-tags",
                    "--recurse-submodules=no",
                    "--",
                    &name,
                    &refspec,
                ]),
                true,
                true,
                true,
            )
        }
        AddWorktree {
            path: destination,
            branch: selected,
        } => {
            let destination = path(destination, local)?;
            branch(&selected)?;
            (
                args(&["worktree", "add", "--", &destination, &selected]),
                true,
                true,
                false,
            )
        }
        AddNewWorktree {
            path: destination,
            branch: selected,
            head,
        } => {
            let destination = path(destination, local)?;
            branch(&selected)?;
            object_id(&head)?;
            (
                args(&[
                    "worktree",
                    "add",
                    "-b",
                    &selected,
                    "--",
                    &destination,
                    &head,
                ]),
                true,
                true,
                false,
            )
        }
        RemoveWorktree { path: destination } => {
            let destination = path(destination, local)?;
            (
                args(&["worktree", "remove", "--", &destination]),
                true,
                true,
                false,
            )
        }
        PruneWorktrees => (args(&["worktree", "prune"]), true, true, false),
        SaveStash {
            message: text,
            include_untracked,
        } => {
            message(&text, 4096)?;
            let mut command = args(&["stash", "push", "-m", &text]);
            if include_untracked {
                command.push("--include-untracked".into());
            }
            command.push("--".into());
            (command, true, true, false)
        }
        ApplyStash {
            object_id: identity,
        } => {
            object_id(&identity)?;
            (
                args(&["stash", "apply", "--", &identity]),
                true,
                true,
                false,
            )
        }
        Commit { message: text } => {
            message(&text, 64 * 1024)?;
            (args(&["commit", "-m", &text]), true, false, false)
        }
    };
    if (mutation && !policy.allow_mutation)
        || (network && policy.network == GitNetworkPolicy::Disabled)
        || ((executes_repository
            || policy.hooks == GitHookPolicy::Configured
            || policy.signing == GitSigningPolicy::Configured
            || policy.credentials == GitCredentialPolicy::ConfiguredNoninteractive)
            && !policy.allow_repository_execution)
    {
        return Err(GitOperationErrorKind::ConsentRequired);
    }
    Ok(Plan {
        args: command,
        mutation,
        push_porcelain,
    })
}
