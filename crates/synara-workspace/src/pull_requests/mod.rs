//! Provider operations use the existing Git and process owners. No checkout,
//! index write, fetch, push or merge of the local repository occurs here.
use crate::{GitOperation, GitOperationOptions, GitOperations};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc, time::Duration};
use synara_runtime::{ExecutionHost, LaunchSpec, LocalHost};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
pub use tokio_util::sync::CancellationToken as PullRequestCancellation;
mod actions;
pub use actions::{PrAction, ReviewKind, MergeMethod};
pub type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GithubRepository { pub owner: String, pub name: String }
impl GithubRepository {
    pub fn parse(remote: &str) -> Result<Self> {
        let path = if let Some(path) = remote.strip_prefix("git@github.com:") { path.to_owned() } else {
            let url = url::Url::parse(remote).map_err(|_| "Unsupported repository URL")?;
            if url.host_str() != Some("github.com") || !matches!(url.scheme(), "https" | "ssh")
                || url.password().is_some() || url.query().is_some() || url.fragment().is_some()
                || url.port().is_some() || (!url.username().is_empty() && !(url.scheme() == "ssh" && url.username() == "git")) {
                return Err("Only canonical GitHub.com HTTPS/SSH remotes are supported. No provider was guessed.".into());
            }
            url.path().trim_start_matches('/').to_owned()
        };
        let path = path.strip_suffix(".git").unwrap_or(&path);
        let (owner, name) = path.split_once('/').ok_or("Repository must contain owner/name")?;
        let valid = |s: &str| !s.is_empty() && s.len() <= 100 && s != "." && s != ".." && !s.starts_with('-')
            && s.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b));
        if !valid(owner) || !valid(name) { return Err("Invalid repository identity".into()); }
        Ok(Self { owner: owner.into(), name: name.into() })
    }
    pub fn slug(&self) -> String { format!("{}/{}", self.owner, self.name) }
    fn path(&self) -> Result<String> {
        let canonical = Self::parse(&format!("https://github.com/{}", self.slug()))?;
        if &canonical != self { return Err("Invalid repository identity".into()); }
        Ok(format!("repos/{}", self.slug()))
    }
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PrFilter { #[default] Open, Closed, Draft, All }
#[derive(Clone, Debug, Deserialize)]
pub struct PullRequest {
    pub number: u64, pub title: String, pub state: String,
    #[serde(default)] pub draft: Option<bool>,
    #[serde(default)] pub body: Option<String>,
    pub user: Value,
    #[serde(default)] pub base: Value,
    #[serde(default)] pub head: Value,
    #[serde(default)] pub node_id: String,
    #[serde(default)] pub html_url: String,
    #[serde(default)] pub merged: Option<bool>,
}
#[derive(Clone, Debug, Deserialize)]
pub struct PrFile { pub filename: String, pub status: String, pub additions: u64, pub deletions: u64,
    pub patch: Option<String> }
impl PrFile {
    pub fn editor_path(&self) -> Result<PathBuf> {
        if self.filename.is_empty() || self.filename.len() > 4096 || self.filename.contains(['\\', ':'])
            || self.filename.chars().any(char::is_control) || self.filename.starts_with('/')
            || self.filename.split('/').any(|p| matches!(p, "" | "." | "..")) {
            return Err("Provider returned an unsafe file path".into());
        }
        Ok(PathBuf::from(&self.filename))
    }
}
#[derive(Clone, Debug)]
pub struct PrDetail { pub pr: PullRequest, pub files: Vec<PrFile>, pub commits: Vec<Value>, pub activity: Vec<Value>,
    pub reviews: Vec<Value>, pub checks: Vec<Value>, pub statuses: Vec<Value>, pub notices: Vec<String> }
#[derive(Clone)]
pub struct PullRequests { root: PathBuf, host: Arc<dyn ExecutionHost>, serial: Arc<tokio::sync::Mutex<()>> }
impl PullRequests {
    pub fn new(root: PathBuf) -> Self { Self::with_host(root, Arc::new(LocalHost)) }
    pub fn with_host(root: PathBuf, host: Arc<dyn ExecutionHost>) -> Self { Self { root, host, serial: Arc::new(tokio::sync::Mutex::new(())) } }
    pub async fn discover(&self, cancel: PullRequestCancellation) -> Result<Vec<(String, GithubRepository)>> {
        let git = GitOperations::with_host(self.root.clone(), self.host.clone());
        let options = GitOperationOptions { timeout: Duration::from_secs(10), max_stdout_bytes: 16 * 1024, ..Default::default() };
        let names = git.execute(GitOperation::RemoteNames, options.clone(), cancel.clone(), None).await.map_err(|e| e.to_string())?;
        let names = String::from_utf8(names.stdout).map_err(|_| "Non-UTF-8 remote name")?;
        if names.lines().count() > 16 { return Err("More than 16 remotes. Select a smaller repository configuration explicitly.".into()); }
        let mut repos = vec![];
        for name in names.lines() {
            let output = git.execute(GitOperation::RemoteUrl { name: name.into(), push: false }, options.clone(), cancel.clone(), None)
                .await.map_err(|e| e.to_string())?;
            let url = String::from_utf8(output.stdout).map_err(|_| "Non-UTF-8 remote URL")?;
            if let Ok(repo) = GithubRepository::parse(url.trim()) { repos.push((name.to_owned(), repo)); }
        }
        if repos.is_empty() { return Err("No supported GitHub.com remote found. Other providers are not implemented.".into()); }
        Ok(repos)
    }
    pub async fn list(&self, repo: &GithubRepository, filter: PrFilter, text: &str, page: u32, cancel: PullRequestCancellation) -> Result<Vec<PullRequest>> {
        repo.path()?;
        if text.len() > 256 || text.chars().any(char::is_control) || !(1..=20).contains(&page) { return Err("Search is limited to 256 characters and 20 pages.".into()); }
        let state = match filter { PrFilter::Open => "is:open", PrFilter::Closed => "is:closed", PrFilter::Draft => "is:open draft:true", PrFilter::All => "" };
        // User text is literal title/body text, not a provider query that can escape repository scope.
        let literal = text.replace(['"', '\\'], " ");
        let query = format!("repo:{} is:pr {state} {}", repo.slug(), if literal.trim().is_empty() { String::new() } else { format!("\"{literal}\"") });
        let encoded = url::form_urlencoded::Serializer::new(String::new()).append_pair("q", &query).finish();
        let value = self.api("GET", &format!("search/issues?{encoded}&sort=updated&order=desc&per_page=50&page={page}"), None, cancel).await?;
        if value["incomplete_results"].as_bool() == Some(true) { return Err("GitHub returned incomplete search results. Narrow the search and refresh.".into()); }
        decode_list(value.get("items").cloned().ok_or("Missing search results")?, 50)
    }
    pub async fn detail(&self, repo: &GithubRepository, number: u64, cancel: PullRequestCancellation) -> Result<PrDetail> {
        let path = format!("{}/pulls/{}", repo.path()?, positive(number)?);
        let work = async {
            let pr: PullRequest = serde_json::from_value(self.api("GET", &path, None, cancel.clone()).await?).map_err(|_| "Invalid PR detail")?;
            let mut out = PrDetail { pr, files: vec![], commits: vec![], activity: vec![], reviews: vec![], checks: vec![], statuses: vec![],
                notices: vec!["Each detail collection is bounded to its first 100 items. Omitted patches may be binary or provider-truncated.".into()] };
            for (name, endpoint) in [("files", format!("{path}/files?per_page=100")), ("commits", format!("{path}/commits?per_page=100")),
                ("activity", format!("{}/issues/{number}/timeline?per_page=100", repo.path()?)), ("reviews", format!("{path}/reviews?per_page=100"))] {
                let result = self.api("GET", &endpoint, None, cancel.clone()).await;
                match result {
                    Ok(value) => match name {
                        "files" => match decode_list(value, 100) { Ok(v) => out.files = v, Err(e) => out.notices.push(format!("Files: {e}")) },
                        _ => match decode_list(value, 100) { Ok(v) => match name { "commits" => out.commits = v, "activity" => out.activity = v, _ => out.reviews = v }, Err(e) => out.notices.push(format!("{name}: {e}")) }
                    },
                    Err(e) => out.notices.push(format!("{name}: {e}")),
                }
            }
            if let Some(sha) = out.pr.head["sha"].as_str().filter(|s| valid_sha(s)) {
                for (name, endpoint, field) in [("checks", format!("{}/commits/{sha}/check-runs?per_page=100", repo.path()?), "check_runs"),
                    ("statuses", format!("{}/commits/{sha}/status?per_page=100", repo.path()?), "statuses")] {
                    match self.api("GET", &endpoint, None, cancel.clone()).await.and_then(|v| decode_list(v[field].clone(), 100)) {
                        Ok(v) => if name == "checks" { out.checks = v } else { out.statuses = v }, Err(e) => out.notices.push(format!("{name}: {e}")),
                    }
                }
            }
            if cancel.is_cancelled() { return Err("Cancelled".into()); }
            Ok(out)
        };
        tokio::time::timeout(Duration::from_secs(60), work).await.map_err(|_| "Detail request timed out".to_owned())?
    }
    pub async fn perform(&self, repo: &GithubRepository, action: PrAction, confirmed: bool, cancel: PullRequestCancellation) -> Result<Value> {
        if !confirmed { return Err("Explicit user confirmation is required".into()); }
        let _gate = tokio::select! { biased; _ = cancel.cancelled() => return Err("Cancelled".into()), gate = self.serial.lock() => gate };
        let (method, path, body) = action.plan(repo)?;
        let result = self.api(method, &path, Some(body), cancel).await;
        match result {
            Ok(value) if value.get("errors").is_some() => Err("GitHub rejected this GraphQL action. Refresh the PR before another action.".into()),
            Ok(value) if matches!(action, PrAction::Merge { .. }) && value["merged"].as_bool() != Some(true) => Err("GitHub did not confirm a merge. Refresh checks and the PR before retrying.".into()),
            value => value.map_err(|e| format!("{e} The write may have reached GitHub. Refresh before retrying; Synara never retries writes automatically.")),
        }
    }
    async fn api(&self, method: &str, endpoint: &str, body: Option<Value>, cancel: PullRequestCancellation) -> Result<Value> {
        let (launch, body) = request_plan(method, endpoint, body)?;
        if cancel.is_cancelled() { return Err("Cancelled".into()); }
        let operation = async {
            let process = self.host.spawn(&launch, &self.root).await.map_err(|_| "GitHub CLI unavailable on the selected host. Install/authenticate gh explicitly there.".to_owned())?;
            let handle = process.handle;
            let read = async {
                let writer = async { let mut stdin = process.stdin; if let Some(body) = body { stdin.write_all(&body).await.map_err(|_| "Cannot write request".to_owned())?; } stdin.shutdown().await.map_err(|_| "Cannot close request".to_owned()) };
                let output = read_bounded(process.stdout, 2 * 1024 * 1024);
                let error = read_bounded(process.stderr, 64 * 1024);
                let (_, out, err) = tokio::try_join!(writer, output, error)?;
                let exit = handle.wait().await.map_err(|_| "GitHub process failed")?;
                if !exit.success() {
                    let diagnostics = String::from_utf8_lossy(&err).to_lowercase();
                    return Err(if diagnostics.contains("401") || diagnostics.contains("gh auth login") || diagnostics.contains("not logged") { "Authentication required on selected host. Use gh auth login explicitly." }
                        else if diagnostics.contains("403") { "GitHub denied access or rate-limited this action." } else if diagnostics.contains("404") { "Repository/PR unavailable, or current credentials lack access." }
                        else if diagnostics.contains("409") || diagnostics.contains("405") || diagnostics.contains("422") { "GitHub rejected the action or its precondition. Refresh provider state." } else { "GitHub CLI request failed. Raw diagnostics are not exposed because they may contain credentials." }.to_owned());
                }
                serde_json::from_slice(&out).map_err(|_| "Invalid or oversized GitHub JSON response".to_owned())
            };
            let result = tokio::select! { biased; _ = cancel.cancelled() => Err("Cancelled".into()), result = read => result };
            if result.is_err() { handle.request_stop(); }
            result
        };
        tokio::time::timeout(Duration::from_secs(20), operation).await.map_err(|_| "GitHub request timed out".to_owned())?
    }
}
async fn read_bounded(reader: synara_runtime::ProcessReader, max: usize) -> Result<Vec<u8>> {
    let mut output = Vec::new(); reader.take(max as u64 + 1).read_to_end(&mut output).await.map_err(|_| "Cannot read GitHub process")?;
    if output.len() > max { return Err("GitHub response exceeded its byte limit".into()); } Ok(output)
}
fn request_plan(method: &str, endpoint: &str, body: Option<Value>) -> Result<(LaunchSpec, Option<Vec<u8>>)> {
    if !matches!(method, "GET" | "POST" | "PATCH" | "PUT") || endpoint.len() > 2048 || endpoint.chars().any(char::is_control)
        || !matches!(endpoint.split('/').next(), Some("repos" | "search") ) && endpoint != "graphql" {
        return Err("Invalid GitHub request".into());
    }
    let body = body.map(|v| serde_json::to_vec(&v).map_err(|_| "Invalid JSON")).transpose()?;
    if body.as_ref().is_some_and(|v| v.len() > 128 * 1024) { return Err("Request exceeds 128 KiB".into()); }
    let mut spec = LaunchSpec::new("gh");
    spec.args = vec!["api".into(), "--hostname".into(), "github.com".into(), "--method".into(), method.into(),
        "-H".into(), "Accept: application/vnd.github+json".into(), "-H".into(), "X-GitHub-Api-Version: 2022-11-28".into()];
    if body.is_some() { spec.args.extend(["--input".into(), "-".into()]); }
    spec.args.push(endpoint.into());
    for (k,v) in [("GH_PROMPT_DISABLED","1"), ("GH_NO_UPDATE_NOTIFIER","1"), ("GH_PAGER","cat"), ("PAGER","cat")] { spec.env.insert(k.into(), v.into()); }
    Ok((spec, body))
}
fn decode_list<T: for<'de> Deserialize<'de>>(value: Value, max: usize) -> Result<Vec<T>> {
    if value.as_array().is_none_or(|v| v.len() > max) { return Err("Invalid or oversized collection".into()); }
    serde_json::from_value(value).map_err(|_| "Invalid provider collection".into())
}
fn positive(number: u64) -> Result<u64> { if number == 0 { Err("Invalid PR number".into()) } else { Ok(number) } }
fn valid_sha(s: &str) -> bool { s.len() == 40 && s.bytes().all(|b| b.is_ascii_hexdigit()) }
#[cfg(test)] mod tests;
