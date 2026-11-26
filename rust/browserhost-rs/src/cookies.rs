use serde_json::{json, Value};
use std::time::Duration;

const SUPPORTED: [(&str, &str); 3] = [("chrome", "Chrome"), ("safari", "Safari"), ("edge", "Microsoft Edge")];
const MAX_COOKIES: usize = 10_000;

fn normalize_domain(input: &str) -> Result<String, String> {
    let raw = input.trim().trim_start_matches('.').to_lowercase();
    if raw.is_empty()
        || raw.len() > 253
        || raw.chars().any(char::is_whitespace)
        || raw.contains(['/', ':', '@'])
    {
        return Err("cookie import domains must be valid hostnames".into());
    }
    let ascii = url::Url::parse(&format!("https://{raw}"))
        .ok()
        .and_then(|url| url.host_str().map(str::to_string))
        .ok_or("cookie import domains must be valid hostnames")?;
    if ascii.starts_with('.') || ascii.ends_with('.') || ascii.contains("..") {
        return Err("cookie import domains must be valid hostnames".into());
    }
    Ok(ascii)
}

fn domain_selected(cookie_domain: &str, filters: &[String]) -> bool {
    if filters.is_empty() {
        return true;
    }
    let host = cookie_domain.trim_start_matches('.');
    filters.iter().any(|domain| {
        host == domain || host.ends_with(&format!(".{domain}")) || domain.ends_with(&format!(".{host}"))
    })
}

pub fn sources() -> Vec<Value> {
    SUPPORTED
        .iter()
        .filter_map(|(id, name)| {
            let installed = rookie_cookies::profiles(id).map(|profiles| !profiles.is_empty()).unwrap_or(false);
            installed.then(|| json!({"id": id, "name": name}))
        })
        .collect()
}

pub fn profiles(browser: &str) -> Result<Vec<Value>, String> {
    if !SUPPORTED.iter().any(|(id, _)| *id == browser) {
        return Err("unsupported cookie source".into());
    }
    rookie_cookies::profiles(browser)
        .map(|profiles| {
            profiles
                .iter()
                .map(|descriptor| {
                    json!({
                        "id": descriptor.profile.profile_id.as_str(),
                        "name": descriptor.profile.display_name,
                    })
                })
                .collect()
        })
        .map_err(|error| format!("cookie profiles unavailable: {}", error.code()))
}

fn same_site(value: i64) -> Option<&'static str> {
    match value {
        0 => Some("None"),
        1 => Some("Lax"),
        2 => Some("Strict"),
        _ => None,
    }
}

/// Convert one lossless rookie-cookies row into a CDP cookie. Cookie values
/// are sensitive and are never included in errors or returned metadata.
pub fn cdp_cookie(detailed: &rookie_cookies::common::enums::DetailedCookie) -> Result<Value, String> {
    let cookie = detailed.cookie();
    if cookie.name.is_empty()
        || cookie.name.len() > 1024
        || cookie.value.len() > 4096
        || cookie.domain.len() > 253
        || cookie.path.len() > 1024
        || cookie.name.bytes().any(|byte| byte <= 0x1f || byte == 0x7f || byte == b';')
    {
        return Err("malformed".into());
    }
    let mut out = json!({
        "name": cookie.name,
        "value": cookie.value,
        "domain": cookie.domain,
        "path": if cookie.path.is_empty() { "/" } else { cookie.path.as_str() },
        "secure": cookie.secure,
        "httpOnly": cookie.http_only,
    });
    if let Some(expires) = cookie.expires {
        out["expires"] = json!(expires);
    }
    if let Some(same_site) = same_site(cookie.same_site) {
        out["sameSite"] = json!(same_site);
    }
    if let Some(site) = detailed.context.top_frame_site_key.as_deref() {
        if url::Url::parse(site).is_ok() {
            out["partitionKey"] = json!({
                "topLevelSite": site,
                "hasCrossSiteAncestor": detailed.context.has_cross_site_ancestor.unwrap_or(false),
            });
        }
    }
    Ok(out)
}

pub struct CookieImport {
    pub cookies: Vec<Value>,
    pub domains: Vec<String>,
    pub warnings: Vec<Value>,
    pub skipped: usize,
}

pub fn read_import(
    browser: &str,
    profile: Option<&str>,
    domain_filters: &[String],
) -> Result<CookieImport, String> {
    if !SUPPORTED.iter().any(|(id, _)| *id == browser) {
        return Err("unsupported cookie source".into());
    }
    let filters = domain_filters
        .iter()
        .map(|domain| normalize_domain(domain))
        .collect::<Result<Vec<_>, _>>()?;
    let mut request = rookie_cookies::ReadRequest::browser(browser)
        .include_session()
        .timeout(Duration::from_secs(30))
        .app_bound(rookie_cookies::AppBoundPolicy::Disabled);
    if let Some(profile) = profile {
        request = request.profile(profile);
    }
    let snapshot = rookie_cookies::read(request)
        .map_err(|error| format!("cookie read failed: {}", error.code()))?;
    let mut cookies = Vec::new();
    let mut skipped = 0usize;
    for detailed in snapshot.detailed_cookies() {
        if cookies.len() >= MAX_COOKIES {
            skipped += 1;
            continue;
        }
        if !domain_selected(&detailed.cookie().domain, &filters) {
            continue;
        }
        match cdp_cookie(detailed) {
            Ok(cookie) => cookies.push(cookie),
            Err(_) => skipped += 1,
        }
    }
    let mut domains: Vec<String> = cookies
        .iter()
        .filter_map(|cookie| cookie.get("domain").and_then(Value::as_str))
        .map(|domain| domain.trim_start_matches('.').to_string())
        .collect();
    domains.sort();
    domains.dedup();
    Ok(CookieImport {
        cookies,
        domains,
        warnings: snapshot
            .warnings()
            .iter()
            .map(|warning| json!({"code": warning.code(), "count": warning.count()}))
            .collect(),
        skipped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rookie_cookies::common::enums::DetailedCookie;

    #[test]
    fn cdp_cookie_preserves_browser_fields_without_exposing_value_in_errors() {
        let detailed: DetailedCookie = serde_json::from_value(json!({
            "cookie": {
                "domain": ".example.com",
                "path": "/app",
                "secure": true,
                "expires": 1_900_000_000u64,
                "name": "session",
                "value": "secret-value",
                "http_only": true,
                "same_site": 2,
            },
            "context": {},
        }))
        .unwrap();
        let converted = cdp_cookie(&detailed).unwrap();
        assert_eq!(converted.get("domain").and_then(Value::as_str), Some(".example.com"));
        assert_eq!(converted.get("sameSite").and_then(Value::as_str), Some("Strict"));
        assert_eq!(converted.get("httpOnly").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn domain_selection_includes_registrable_children_without_cross_domain_matches() {
        let filters = vec!["example.com".to_string()];
        assert!(domain_selected(".example.com", &filters));
        assert!(domain_selected("app.example.com", &filters));
        assert!(!domain_selected("notexample.com", &filters));
        assert!(!domain_selected("example.com.evil.test", &filters));
    }
}
