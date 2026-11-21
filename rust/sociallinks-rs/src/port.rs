//! The Rust port of `packages/shared/src/socialLinks.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

fn is_alnum(c: char) -> bool {
    c.is_ascii_alphanumeric()
}

fn x_pattern(s: &str) -> bool {
    (1..=15).contains(&s.len()) && s.chars().all(|c| is_alnum(c) || c == '_')
}

fn linkedin_pattern(s: &str) -> bool {
    (3..=100).contains(&s.len())
        && s.chars().next().is_some_and(is_alnum)
        && s[1..].chars().all(|c| is_alnum(c) || c == '-')
}

fn github_pattern(s: &str) -> bool {
    (1..=39).contains(&s.len())
        && s.chars().next().is_some_and(is_alnum)
        && s.chars().last().is_some_and(is_alnum)
        && !s.contains("--")
        && s.chars().all(|c| is_alnum(c) || c == '-')
}

fn threads_pattern(s: &str) -> bool {
    (1..=30).contains(&s.len()) && s.chars().all(|c| is_alnum(c) || c == '.' || c == '_')
}

fn youtube_pattern(s: &str) -> bool {
    (3..=30).contains(&s.len())
        && s.chars()
            .all(|c| is_alnum(c) || c == '.' || c == '_' || c == '-')
}

struct Platform {
    label: &'static str,
    url_prefix: &'static str,
    hosts: &'static [&'static str],
    matches_pattern: fn(&str) -> bool,
}

const X: Platform = Platform {
    label: "X",
    url_prefix: "https://x.com/",
    hosts: &["x.com", "www.x.com", "twitter.com", "www.twitter.com"],
    matches_pattern: x_pattern,
};
const LINKEDIN: Platform = Platform {
    label: "LinkedIn",
    url_prefix: "https://www.linkedin.com/in/",
    hosts: &["linkedin.com", "www.linkedin.com"],
    matches_pattern: linkedin_pattern,
};
const GITHUB: Platform = Platform {
    label: "GitHub",
    url_prefix: "https://github.com/",
    hosts: &["github.com", "www.github.com"],
    matches_pattern: github_pattern,
};
const THREADS: Platform = Platform {
    label: "Threads",
    url_prefix: "https://www.threads.com/@",
    hosts: &[
        "threads.com",
        "www.threads.com",
        "threads.net",
        "www.threads.net",
    ],
    matches_pattern: threads_pattern,
};
const YOUTUBE: Platform = Platform {
    label: "YouTube",
    url_prefix: "https://www.youtube.com/@",
    hosts: &["youtube.com", "www.youtube.com", "m.youtube.com"],
    matches_pattern: youtube_pattern,
};

fn find_platform(key: &str) -> Option<&'static Platform> {
    match key {
        "x" => Some(&X),
        "linkedin" => Some(&LINKEDIN),
        "github" => Some(&GITHUB),
        "threads" => Some(&THREADS),
        "youtube" => Some(&YOUTUBE),
        _ => None,
    }
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
}

/// Splits `host/path` or `scheme://host/path` (protocol-relative allowed) into
/// (lowercased host, path without query/fragment).
fn parse_url_like(s: &str) -> Option<(String, String)> {
    let rest = match s.find("://") {
        Some(i) => &s[i + 3..],
        None => s.strip_prefix("//").unwrap_or(s),
    };
    let slash = rest.find('/')?;
    let authority = &rest[..slash];
    let after_userinfo = authority.rsplit('@').next()?;
    let host = after_userinfo.split(':').next()?.to_lowercase();
    let path = rest[slash..].split(['?', '#']).next()?.to_string();
    Some((host, path))
}

/// Extracts the username segment from a platform URL path.
fn extract_segment<'a>(key: &str, path: &'a str) -> Option<&'a str> {
    let trimmed = path.strip_suffix('/').unwrap_or(path);
    let rest = trimmed.strip_prefix('/')?;
    match key {
        "x" | "github" => {
            if rest.is_empty() || rest.contains('/') || rest.starts_with('@') {
                None
            } else {
                Some(rest)
            }
        }
        "linkedin" => rest
            .strip_prefix("in/")
            .filter(|s| !s.is_empty() && !s.contains('/')),
        "threads" | "youtube" => rest
            .strip_prefix('@')
            .filter(|s| !s.is_empty() && !s.contains('/')),
        _ => None,
    }
}

fn normalize(key: &str, username: &str) -> Result<Option<String>, String> {
    let s = js_trim(username);
    if s.contains('/') {
        let (host, path) = match parse_url_like(s) {
            Some(v) => v,
            None => return Ok(None),
        };
        let platform = match find_platform(key) {
            Some(p) if p.hosts.contains(&host.as_str()) => p,
            _ => return Ok(None),
        };
        return Ok(extract_segment(key, &path)
            .filter(|seg| (platform.matches_pattern)(seg))
            .map(str::to_string));
    }
    let platform = match find_platform(key) {
        Some(p) => p,
        None => return Err("Cannot read properties of undefined (reading 'pattern')".to_string()),
    };
    let candidate = s.strip_prefix('@').unwrap_or(s);
    Ok(if (platform.matches_pattern)(candidate) {
        Some(candidate.to_string())
    } else {
        None
    })
}

/// Port of `normalizeSocialUsername` (`packages/shared/src/socialLinks.ts:46`).
pub fn normalize_social_username(
    platform: serde_json::Value,
    input: String,
) -> Result<Option<String>, String> {
    let key = match platform.as_str() {
        Some(k) => k,
        None => {
            return if input.contains('/') {
                Ok(None)
            } else {
                Err("Cannot read properties of undefined (reading 'pattern')".to_string())
            };
        }
    };
    normalize(key, &input)
}

/// Port of `socialProfileUrl` (`packages/shared/src/socialLinks.ts:74`).
pub fn social_profile_url(platform: serde_json::Value, username: String) -> Result<String, String> {
    match platform.as_str().and_then(find_platform) {
        Some(p) => Ok(format!("{}{}", p.url_prefix, username)),
        None => Err("Cannot read properties of undefined (reading 'url')".to_string()),
    }
}

/// Port of `socialLinkLabel` (`packages/shared/src/socialLinks.ts:78`).
pub fn social_link_label(platform: serde_json::Value) -> Result<String, String> {
    match platform.as_str().and_then(find_platform) {
        Some(p) => Ok(p.label.to_string()),
        None => Err("Cannot read properties of undefined (reading 'label')".to_string()),
    }
}

/// Port of `sanitizeSocialLinks` (`packages/shared/src/socialLinks.ts:83`).
pub fn sanitize_social_links(value: serde_json::Value) -> Result<serde_json::Value, String> {
    let mut out = serde_json::Map::new();
    if let Value::Object(map) = &value {
        for (key, v) in map {
            if let (Some(p), Value::String(u)) = (find_platform(key), v) {
                if (p.matches_pattern)(u) {
                    out.insert(key.clone(), v.clone());
                }
            }
        }
    }
    Ok(Value::Object(out))
}
