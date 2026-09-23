//! Bounded, provider-owned stacks. No local Git commands or checkout ownership.
//! A reviewed prefix is merged root-first using merge commits only. Child base
//! retargeting is part of the explicit review, never an incidental cleanup step.
use super::*;
use std::collections::{BTreeMap, BTreeSet};

const MAX_OPEN: usize = 100;
const MAX_STACK: usize = 16;

#[derive(Clone, Debug, Eq, PartialEq)]
struct Identity {
    number: u64,
    head_repo: String,
    head_ref: String,
    head: String,
    base_repo: String,
    base_ref: String,
    base_commit: String,
}
#[derive(Clone, Debug)]
pub struct StackRow {
    identity: Identity,
    pub title: String,
    pub parent: Option<u64>,
    pub depth: usize,
    /// None means the reported checks and mergeability currently permit review.
    /// GitHub still makes the authoritative merge-policy decision on every write.
    pub blocked: Option<String>,
}
impl StackRow {
    pub fn number(&self) -> u64 {
        self.identity.number
    }
    pub fn head(&self) -> &str {
        &self.identity.head
    }
    pub fn branch(&self) -> &str {
        &self.identity.head_ref
    }
    pub fn base(&self) -> &str {
        &self.identity.base_ref
    }
}
/// Not deserializable. Only a complete, scoped provider read constructs a review.
#[derive(Clone, Debug)]
pub struct StackReview {
    pub repository: GithubRepository,
    rows: Vec<StackRow>,
    selected: u64,
}
impl StackReview {
    pub fn rows(&self) -> &[StackRow] {
        &self.rows
    }
    pub fn selected(&self) -> u64 {
        self.selected
    }
    pub fn select(&mut self, number: u64) -> Result<()> {
        if !self.rows.iter().any(|r| r.number() == number) {
            return Err("PR is not in this reviewed stack".into());
        }
        self.selected = number;
        Ok(())
    }
    pub fn prefix(&self) -> Result<Vec<&StackRow>> {
        let mut out = Vec::new();
        let mut next = Some(self.selected);
        while let Some(number) = next {
            if out.len() >= MAX_STACK || out.iter().any(|r: &&StackRow| r.number() == number) {
                return Err("Invalid or cyclic stack".into());
            }
            let row = self
                .rows
                .iter()
                .find(|r| r.number() == number)
                .ok_or("Incomplete stack")?;
            out.push(row);
            next = row.parent;
        }
        out.reverse();
        if out.is_empty() {
            return Err("No selected prefix".into());
        }
        Ok(out)
    }
    pub fn ready(&self) -> bool {
        self.prefix()
            .is_ok_and(|rows| rows.iter().all(|r| r.blocked.is_none()))
    }
}
#[derive(Clone, Debug, Default)]
pub struct StackProgress {
    pub merged: Vec<u64>,
    pub retargeted: Vec<u64>,
    pub stopped: Option<String>,
}
impl StackProgress {
    pub fn summary(&self) -> String {
        format!(
            "Confirmed merges: {:?}. Confirmed base changes: {:?}. {}",
            self.merged,
            self.retargeted,
            self.stopped
                .as_deref()
                .unwrap_or("Selected prefix completed. Refresh provider state.")
        )
    }
}
impl PullRequests {
    pub async fn review_stack(
        &self,
        repo: &GithubRepository,
        selected: u64,
        cancel: PullRequestCancellation,
    ) -> Result<StackReview> {
        positive(selected)?;
        let root = repo.path()?;
        let work = async {
            let value = self
                .api(
                    "GET",
                    &format!(
                        "{root}/pulls?state=open&sort=created&direction=asc&per_page=100&page=1"
                    ),
                    None,
                    cancel.clone(),
                )
                .await?;
            let open: Vec<Value> = decode_list(value, MAX_OPEN)?;
            // Never infer a parent from an incomplete first page.
            if open.len() == MAX_OPEN {
                let tail = self
                    .api(
                        "GET",
                        &format!(
                            "{root}/pulls?state=open&sort=created&direction=asc&per_page=100&page=2"
                        ),
                        None,
                        cancel.clone(),
                    )
                    .await?;
                if !decode_list::<Value>(tail, MAX_OPEN)?.is_empty() {
                    return Err("Stack discovery supports at most 100 open PRs. No truncated stack was inferred.".into());
                }
            }
            let mut review = graph(repo, selected, &open)?;
            for row in &mut review.rows {
                let (fresh, blocked) = self.stack_state(repo, row.number(), cancel.clone()).await?;
                if fresh != row.identity {
                    return Err("Stack changed during discovery. Refresh and review again.".into());
                }
                row.blocked = blocked;
            }
            if cancel.is_cancelled() {
                return Err("Cancelled".into());
            }
            Ok(review)
        };
        tokio::time::timeout(Duration::from_secs(120), work)
            .await
            .map_err(|_| "Stack discovery timed out".to_owned())?
    }
    /// Serialize with all other PR writes. A stop is a report, not a fictitious
    /// transaction rollback. Previously confirmed writes remain visible to users.
    pub async fn merge_stack_prefix(
        &self,
        repo: &GithubRepository,
        review: StackReview,
        confirmed: bool,
        cancel: PullRequestCancellation,
    ) -> Result<StackProgress> {
        if !confirmed {
            return Err("Explicit selected-prefix confirmation is required".into());
        }
        if *repo != review.repository {
            return Err("Reviewed repository scope changed".into());
        }
        repo.path()?;
        let prefix: Vec<StackRow> = review.prefix()?.into_iter().cloned().collect();
        if !review.ready() {
            return Err("Selected prefix has blocked or unknown readiness".into());
        }
        let _gate = tokio::select! { biased;
            _ = cancel.cancelled() => return Err("Cancelled before any stack write".into()),
            gate = self.serial.lock() => gate,
        };
        // Re-read the full relationship graph and every selected head before the
        // first write. Approval is not transferable to a later graph or SHA.
        let fresh = self
            .review_stack(repo, review.selected, cancel.clone())
            .await?;
        let current = fresh.prefix()?;
        if current.len() != prefix.len()
            || current
                .iter()
                .zip(&prefix)
                .any(|(a, b)| a.identity != b.identity || a.parent != b.parent)
        {
            return Err(
                "Stale stack head, base or parent. Nothing was merged. Review again.".into(),
            );
        }
        if !fresh.ready() {
            return Err("Stack checks or mergeability changed. Nothing was merged.".into());
        }
        let root_base = prefix[0].identity.base_ref.clone();
        let mut progress = StackProgress::default();
        for (position, row) in prefix.iter().enumerate() {
            let step = async {
                if cancel.is_cancelled() { return Err("Cancelled before the next write".into()); }
                let (mut identity, mut blocked) = self.stack_state(repo, row.number(), cancel.clone()).await?;
                if identity != row.identity {
                    return Err("Stale stack head or base. Remaining steps were not executed.".into());
                }
                if let Some(reason) = blocked.take() { return Err(format!("PR #{} is not ready: {reason}",row.number())); }
                if position > 0 && identity.base_ref != root_base {
                    // Merging the parent with a merge commit keeps the reviewed
                    // descendant head ancestry intact. Do not squash or rebase.
                    let path = format!("{}/pulls/{}", repo.path()?, row.number());
                    let value = self.api("PATCH", &path, Some(json!({"base":root_base})), cancel.clone()).await
                        .map_err(ambiguous_write)?;
                    let updated = parse_identity(repo, &value).map_err(ambiguous_write)?;
                    if !same_head(&updated, &row.identity) || updated.base_ref != root_base {
                        return Err(ambiguous_write("Provider did not confirm the reviewed base change".into()));
                    }
                    progress.retargeted.push(row.number());
                    // Retargeting can invalidate approvals or trigger new checks.
                    // Never bypass that state or poll/retry a write automatically.
                    (identity, blocked) = self.stack_state(repo, row.number(), cancel.clone()).await?;
                    if !same_head(&identity, &row.identity) || identity.base_ref != root_base {
                        return Err("Head or target changed after retargeting. Review again.".into());
                    }
                    if let Some(reason) = blocked { return Err(format!("Retargeted PR #{} requires a new review/check run: {reason}",row.number())); }
                }
                let action = PrAction::Merge { number: row.number(), sha: identity.head, method: MergeMethod::Merge };
                let (method, path, body) = action.plan(repo)?;
                let value = self.api(method, &path, Some(body), cancel.clone()).await.map_err(ambiguous_write)?;
                if value["merged"].as_bool() != Some(true) {
                    return Err("GitHub did not confirm this merge. Remaining steps stopped. Refresh before another action.".into());
                }
                progress.merged.push(row.number());
                Ok::<(),String>(())
            }.await;
            if let Err(error) = step {
                progress.stopped = Some(error);
                break;
            }
        }
        Ok(progress)
    }
    async fn stack_state(
        &self,
        repo: &GithubRepository,
        number: u64,
        cancel: PullRequestCancellation,
    ) -> Result<(Identity, Option<String>)> {
        let root = repo.path()?;
        let value = self
            .api(
                "GET",
                &format!("{root}/pulls/{}", positive(number)?),
                None,
                cancel.clone(),
            )
            .await?;
        let identity = parse_identity(repo, &value)?;
        if identity.number != number {
            return Err("Provider PR identity mismatch".into());
        }
        let mut blocked = if identity.head_repo != repo.slug().to_ascii_lowercase() {
            Some("Fork heads can be inspected, but selected-prefix merge supports same-repository stacks only.".into())
        } else if value["draft"].as_bool() != Some(false) {
            Some("Draft or unknown draft state".into())
        } else if value["mergeable"].as_bool() != Some(true)
            || value["mergeable_state"].as_str() != Some("clean")
        {
            Some("GitHub mergeability is blocked or not yet clean".into())
        } else {
            None
        };
        let checks = self
            .api(
                "GET",
                &format!("{root}/commits/{}/check-runs?per_page=100", identity.head),
                None,
                cancel.clone(),
            )
            .await?;
        let status = self
            .api(
                "GET",
                &format!("{root}/commits/{}/status?per_page=100", identity.head),
                None,
                cancel,
            )
            .await?;
        if let Err(reason) = checks_ready(&checks, &status) {
            blocked = Some(reason);
        }
        Ok((identity, blocked))
    }
}
fn ambiguous_write(error: String) -> String {
    format!(
        "{error}. The last write may have reached GitHub. Refresh before retrying. No automatic retry or rollback."
    )
}
fn same_head(a: &Identity, b: &Identity) -> bool {
    a.number == b.number
        && a.head_repo == b.head_repo
        && a.head_ref == b.head_ref
        && a.head == b.head
        && a.base_repo == b.base_repo
}
fn field(value: &Value, key: &str, max: usize) -> Result<String> {
    let s = value[key]
        .as_str()
        .ok_or_else(|| format!("Missing stack field: {key}"))?;
    if s.is_empty() || s.len() > max || s.chars().any(char::is_control) {
        return Err("Invalid or oversized stack identity".into());
    }
    Ok(s.into())
}
fn parse_identity(repo: &GithubRepository, value: &Value) -> Result<Identity> {
    let number = positive(value["number"].as_u64().ok_or("Missing stack PR number")?)?;
    if value["state"].as_str() != Some("open") || value["merged"].as_bool() == Some(true) {
        return Err("Stack PR is no longer open".into());
    }
    let head = &value["head"];
    let base = &value["base"];
    let slug = |v: &Value| -> Result<String> {
        let s = field(&v["repo"], "full_name", 201)?;
        Ok(GithubRepository::parse(&format!("https://github.com/{s}"))?
            .slug()
            .to_ascii_lowercase())
    };
    let out = Identity {
        number,
        head_repo: slug(head)?,
        head_ref: field(head, "ref", 256)?,
        head: field(head, "sha", 40)?,
        base_repo: slug(base)?,
        base_ref: field(base, "ref", 256)?,
        base_commit: field(base, "sha", 40)?,
    };
    if out.base_repo != repo.slug().to_ascii_lowercase()
        || !valid_sha(&out.head)
        || !valid_sha(&out.base_commit)
    {
        return Err("Stack repository or commit identity is invalid".into());
    }
    Ok(out)
}
fn graph(repo: &GithubRepository, selected: u64, open: &[Value]) -> Result<StackReview> {
    if open.len() > MAX_OPEN {
        return Err("Too many open PRs".into());
    }
    let mut identities = BTreeMap::new();
    let mut heads = BTreeMap::new();
    let mut titles = BTreeMap::new();
    for value in open {
        let identity = parse_identity(repo, value)?;
        if heads
            .insert(
                (identity.head_repo.clone(), identity.head_ref.clone()),
                identity.number,
            )
            .is_some()
            || identities.contains_key(&identity.number)
        {
            return Err("Duplicate PR or ambiguous stack head branch".into());
        }
        titles.insert(identity.number, field(value, "title", 1024)?);
        identities.insert(identity.number, identity);
    }
    if !identities.contains_key(&selected) {
        return Err("Selected PR is not in the complete open-PR snapshot".into());
    }
    let parents: BTreeMap<u64, Option<u64>> = identities
        .values()
        .map(|id| {
            (
                id.number,
                heads
                    .get(&(id.base_repo.clone(), id.base_ref.clone()))
                    .copied(),
            )
        })
        .collect();
    // Reject all cycles, not only the selected node, before inferring roots.
    for number in identities.keys() {
        let mut visited = BTreeSet::new();
        let mut next = Some(*number);
        while let Some(n) = next {
            if !visited.insert(n) {
                return Err("Cyclic stack relationships".into());
            }
            next = *parents.get(&n).ok_or("Incomplete stack graph")?;
        }
    }
    let mut root = selected;
    while let Some(parent) = parents[&root] {
        root = parent;
    }
    let mut pending = vec![(root, 0)];
    let mut rows = Vec::new();
    while let Some((number, depth)) = pending.pop() {
        if rows.len() >= MAX_STACK {
            return Err("This stack exceeds 16 PRs. No truncated stack was inferred.".into());
        }
        rows.push(StackRow {
            identity: identities[&number].clone(),
            title: titles[&number].clone(),
            parent: parents[&number],
            depth,
            blocked: Some("Readiness has not been loaded".into()),
        });
        // A BTreeMap gives a stable numeric sibling order independent of API order.
        for (&child, &parent) in parents.iter().rev() {
            if parent == Some(number) {
                pending.push((child, depth + 1));
            }
        }
    }
    Ok(StackReview {
        repository: repo.clone(),
        rows,
        selected,
    })
}
fn checks_ready(checks: &Value, status: &Value) -> Result<()> {
    for (value, items) in [(checks, "check_runs"), (status, "statuses")] {
        let list = value[items]
            .as_array()
            .ok_or("Unknown checks or status collection")?;
        if list.len() > 100 || value["total_count"].as_u64() != Some(list.len() as u64) {
            return Err("Checks/statuses are incomplete or exceed 100 entries".into());
        }
    }
    if checks["check_runs"].as_array().unwrap().iter().any(|c| {
        c["status"].as_str() != Some("completed")
            || !matches!(
                c["conclusion"].as_str(),
                Some("success" | "neutral" | "skipped")
            )
    }) {
        return Err("Checks are pending, failed or unknown".into());
    }
    if status["total_count"].as_u64() != Some(0)
        && (status["state"].as_str() != Some("success")
            || status["statuses"]
                .as_array()
                .unwrap()
                .iter()
                .any(|s| s["state"].as_str() != Some("success")))
    {
        return Err("Commit statuses are pending, failed or unknown".into());
    }
    Ok(())
}

#[cfg(test)]
#[path = "stack_tests.rs"]
mod tests;
