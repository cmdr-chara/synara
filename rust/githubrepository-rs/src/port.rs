//! The Rust port of `packages/shared/src/githubRepository.ts`.
//!
//! Clean-room implementation (ADR-001): written from the recorded behavior in
//! `cases.jsonl` and the exported signatures, not from the source body.

#![forbid(unsafe_code)]

fn is_valid_owner(owner: &str) -> bool {
    // GitHub user/org names: ASCII alphanumeric segments joined by single hyphens,
    // 1-39 characters, no leading or trailing hyphen.
    !owner.is_empty()
        && owner.len() <= 39
        && owner
            .split('-')
            .all(|seg| !seg.is_empty() && seg.bytes().all(|b| b.is_ascii_alphanumeric()))
}

fn is_valid_repo_name(repo: &str) -> bool {
    // GitHub repo names: 1-100 chars of ASCII alnum plus `.`, `-`, `_`;
    // a name made only of dots is not allowed.
    !repo.is_empty()
        && repo.len() <= 100
        && !repo.bytes().all(|b| b == b'.')
        && repo
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
}

fn strip_ci_prefix<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    let head = s.as_bytes().get(..prefix.len())?;
    if head.eq_ignore_ascii_case(prefix.as_bytes()) {
        s.get(prefix.len()..)
    } else {
        None
    }
}

fn split_owner_repo(s: &str) -> Option<(&str, &str)> {
    let (owner, repo) = s.split_once('/')?;
    if repo.contains('/') || !is_valid_owner(owner) || !is_valid_repo_name(repo) {
        return None;
    }
    Some((owner, repo))
}

fn strip_git_suffix(path: &str) -> &str {
    let path = path.strip_suffix('/').unwrap_or(path);
    let bytes = path.as_bytes();
    if bytes.len() > 4 && bytes[bytes.len() - 4..].eq_ignore_ascii_case(b".git") {
        &path[..path.len() - 4]
    } else {
        path
    }
}

fn parse_remote_path(url: &str) -> Option<String> {
    let url = url.trim();
    let path = if let Some(rest) = strip_ci_prefix(url, "git@github.com:") {
        rest
    } else {
        let (scheme, rest) = url.split_once("://")?;
        if !matches!(
            scheme.to_ascii_lowercase().as_str(),
            "http" | "https" | "git" | "ssh"
        ) {
            return None;
        }
        // `git@` userinfo only for ssh/git transports; http(s) must name the host directly.
        let after_user = if matches!(scheme.to_ascii_lowercase().as_str(), "git" | "ssh") {
            rest.strip_prefix("git@").unwrap_or(rest)
        } else {
            rest
        };
        let (host, path) = after_user.split_once('/')?;
        if !host.eq_ignore_ascii_case("github.com") {
            return None;
        }
        path
    };
    let (owner, repo) = split_owner_repo(strip_git_suffix(path))?;
    Some(format!("{}/{}", owner, repo))
}

/// Returns (owner, repo, digits) for `https?://github.com/owner/repo/pull/N[/files][?q][#f]`.
fn parse_pull_request_parts(url: &str) -> Option<(&str, &str, &str)> {
    let url = url.trim();
    let (scheme, rest) = url.split_once("://")?;
    if !matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https") {
        return None;
    }
    let (host, path) = rest.split_once('/')?;
    if !host.eq_ignore_ascii_case("github.com") {
        return None;
    }
    let path = path.split(['?', '#']).next().unwrap_or(path);
    let mut segs = path.split('/');
    let owner = segs.next()?;
    let repo = segs.next()?;
    if segs.next()? != "pull" {
        return None;
    }
    let digits = segs.next()?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    match segs.next() {
        None => {}
        Some("files") if segs.next().is_none() => {}
        _ => return None,
    }
    split_owner_repo(&format!("{}/{}", owner, repo))?;
    Some((owner, repo, digits))
}

/// Port of `isValidGitHubRepositoryNameWithOwner` (`packages/shared/src/githubRepository.ts:2`).
pub fn is_valid_git_hub_repository_name_with_owner(repository: String) -> Result<bool, String> {
    Ok(split_owner_repo(&repository).is_some())
}

/// Port of `parseGitHubRepositoryInput` (`packages/shared/src/githubRepository.ts:20`).
pub fn parse_git_hub_repository_input(input: Option<String>) -> Result<Option<String>, String> {
    let trimmed = match input.as_deref().map(str::trim) {
        Some(t) if !t.is_empty() => t,
        _ => return Ok(None),
    };
    if split_owner_repo(trimmed).is_some() {
        return Ok(Some(trimmed.to_string()));
    }
    // URL form: `https://github.com/owner/repo[.git][/]` only — recorded inputs
    // reject scp-like, ssh:// and git:// transports here.
    if let Some(rest) = strip_ci_prefix(trimmed, "https://github.com/")
        && let Some((owner, repo)) = split_owner_repo(strip_git_suffix(rest))
    {
        return Ok(Some(format!("{}/{}", owner, repo)));
    }
    Ok(None)
}

/// Port of `parseGitHubRepositoryNameWithOwnerFromRemoteUrl` (`packages/shared/src/githubRepository.ts:32`).
pub fn parse_git_hub_repository_name_with_owner_from_remote_url(
    url: Option<String>,
) -> Result<Option<String>, String> {
    Ok(url.as_deref().and_then(parse_remote_path))
}

/// Port of `parseGitHubRepositoryNameWithOwnerFromPullRequestUrl` (`packages/shared/src/githubRepository.ts:49`).
pub fn parse_git_hub_repository_name_with_owner_from_pull_request_url(
    url: Option<String>,
) -> Result<Option<String>, String> {
    Ok(url
        .as_deref()
        .and_then(parse_pull_request_parts)
        .map(|(owner, repo, _)| format!("{}/{}", owner, repo)))
}

/// Port of `normalizeGitHubPullRequestUrl` (`packages/shared/src/githubRepository.ts:62`).
pub fn normalize_git_hub_pull_request_url(input: Option<String>) -> Result<Option<String>, String> {
    const MAX_SAFE_INTEGER: u128 = 9_007_199_254_740_991; // Number.MAX_SAFE_INTEGER
    let Some((owner, repo, digits)) = input.as_deref().and_then(parse_pull_request_parts) else {
        return Ok(None);
    };
    let n: u128 = match digits.parse() {
        Ok(n) => n,
        Err(_) => return Ok(None),
    };
    if n == 0 || n > MAX_SAFE_INTEGER {
        return Ok(None);
    }
    Ok(Some(format!(
        "https://github.com/{}/{}/pull/{}",
        owner.to_lowercase(),
        repo.to_lowercase(),
        n
    )))
}
