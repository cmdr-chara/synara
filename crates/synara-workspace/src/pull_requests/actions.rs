use super::*;
#[derive(Clone, Copy, Debug)]
pub enum ReviewKind {
    Comment,
    Approve,
    RequestChanges,
}
#[derive(Clone, Copy, Debug)]
pub enum MergeMethod {
    Merge,
    Squash,
    Rebase,
}
/// Native confirmation owns construction. Not deserializable by agents.
#[derive(Clone, Debug)]
pub enum PrAction {
    Create {
        title: String,
        body: String,
        base: String,
        head: String,
        draft: bool,
    },
    Comment {
        number: u64,
        body: String,
    },
    State {
        number: u64,
        open: bool,
    },
    Draft {
        node_id: String,
        draft: bool,
    },
    Review {
        number: u64,
        sha: String,
        body: String,
        kind: ReviewKind,
    },
    Merge {
        number: u64,
        sha: String,
        method: MergeMethod,
    },
}
impl PrAction {
    pub(super) fn plan(&self, repo: &GithubRepository) -> Result<(&'static str, String, Value)> {
        let repo = repo.path()?;
        let text = |s: &str, max: usize, required: bool| -> Result<()> {
            if s.len() > max || s.contains('\0') || (required && s.trim().is_empty()) {
                Err("Invalid or oversized action text".into())
            } else {
                Ok(())
            }
        };
        let sha = |s: &str| {
            if valid_sha(s) {
                Ok(())
            } else {
                Err("A reviewed 40-character head SHA is required".to_owned())
            }
        };
        match self {
            Self::Create {
                title,
                body,
                base,
                head,
                draft,
            } => {
                text(title, 256, true)?;
                text(body, 64 * 1024, false)?;
                for branch in [base, head] {
                    text(branch, 256, true)?;
                    if branch.starts_with('-') || branch.contains(['\n', '\r']) {
                        return Err("Invalid branch".into());
                    }
                }
                Ok((
                    "POST",
                    format!("{repo}/pulls"),
                    json!({"title":title,"body":body,"base":base,"head":head,"draft":draft}),
                ))
            }
            Self::Comment { number, body } => {
                positive(*number)?;
                text(body, 64 * 1024, true)?;
                Ok((
                    "POST",
                    format!("{repo}/issues/{number}/comments"),
                    json!({"body":body}),
                ))
            }
            Self::State { number, open } => {
                positive(*number)?;
                Ok((
                    "PATCH",
                    format!("{repo}/pulls/{number}"),
                    json!({"state":if *open {"open"} else {"closed"}}),
                ))
            }
            Self::Draft { node_id, draft } => {
                text(node_id, 256, true)?;
                if !node_id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_-=".contains(&b))
                {
                    return Err("Invalid provider node identity".into());
                }
                let name = if *draft {
                    "convertPullRequestToDraft"
                } else {
                    "markPullRequestReadyForReview"
                };
                Ok((
                    "POST",
                    "graphql".into(),
                    json!({"query":format!("mutation($id:ID!){{{name}(input:{{pullRequestId:$id}}){{pullRequest{{id isDraft}}}}}}"),"variables":{"id":node_id}}),
                ))
            }
            Self::Review {
                number,
                sha: head,
                body,
                kind,
            } => {
                positive(*number)?;
                sha(head)?;
                text(body, 64 * 1024, !matches!(kind, ReviewKind::Approve))?;
                Ok((
                    "POST",
                    format!("{repo}/pulls/{number}/reviews"),
                    json!({"commit_id":head,"body":body,"event":match kind { ReviewKind::Comment => "COMMENT", ReviewKind::Approve => "APPROVE", ReviewKind::RequestChanges => "REQUEST_CHANGES" }}),
                ))
            }
            Self::Merge {
                number,
                sha: head,
                method,
            } => {
                positive(*number)?;
                sha(head)?;
                Ok((
                    "PUT",
                    format!("{repo}/pulls/{number}/merge"),
                    json!({"sha":head,"merge_method":match method { MergeMethod::Merge => "merge", MergeMethod::Squash => "squash", MergeMethod::Rebase => "rebase" }}),
                ))
            }
        }
    }
}
