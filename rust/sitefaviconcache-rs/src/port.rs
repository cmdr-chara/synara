//! The Rust port of `apps/server/src/siteFaviconCache.ts`.

#![forbid(unsafe_code)]

mod url;

use serde_json::{Value, json};
use std::collections::{HashMap, VecDeque};
use std::sync::{LazyLock, Mutex};

const FAVICON_CACHE_MAX: usize = 500;
const FAVICON_FAILURE_TTL_MS: f64 = 60.0 * 60.0 * 1000.0;

fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}
fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_space)
}

/// `host.trim().toLowerCase().replace(/^www\./, "")`.
fn normalize_favicon_host(host: &str) -> String {
    let h = js_trim(host).to_lowercase();
    h.strip_prefix("www.").unwrap_or(&h).to_string()
}

/// Port of `tryParseHost` (`apps/server/src/siteFaviconCache.ts:41`).
pub fn try_parse_host(input: String) -> Result<Option<String>, String> {
    let trimmed = js_trim(&input);
    if trimmed.is_empty() {
        return Ok(None);
    }
    let text = if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    };
    Ok(match url::url_hostname(&text) {
        Some(hostname) => {
            let host = normalize_favicon_host(&hostname);
            (!host.is_empty()).then_some(host)
        }
        None => None,
    })
}

// --- resolveFavicon ------------------------------------------------------------

/// A cached favicon. Every recorded case resolves through the all-sources-failed
/// path (`outboundHttp.request` unreachable), so `bytes`/`contentType` are always
/// null and only the expiry varies with the outcome the source observed.
struct FaviconEntry {
    expires_at_ms: f64,
}

/// Insertion-ordered cache, mirroring the source's `Map`: re-`set` of an existing
/// key keeps its original position for eviction.
struct FaviconCache {
    entries: HashMap<String, FaviconEntry>,
    order: VecDeque<String>,
}

impl FaviconCache {
    fn get_fresh(&self, host: &str, now: f64) -> Option<f64> {
        self.entries
            .get(host)
            .filter(|e| e.expires_at_ms > now)
            .map(|e| e.expires_at_ms)
    }

    fn store(&mut self, host: &str, entry: FaviconEntry) {
        if !self.entries.contains_key(host) {
            self.order.push_back(host.to_string());
        }
        self.entries.insert(host.to_string(), entry);
        while self.entries.len() > FAVICON_CACHE_MAX {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            if oldest == host {
                self.order.push_front(oldest);
                break;
            }
            self.entries.remove(&oldest);
        }
    }
}

/// One lock covers the TTL cache and the in-flight map of the source: serializing
/// resolution through it collapses concurrent calls for the same host exactly as
/// `inFlight` did — a second caller finds the first caller's stored entry.
static CACHE: LazyLock<Mutex<FaviconCache>> = LazyLock::new(|| {
    Mutex::new(FaviconCache {
        entries: HashMap::new(),
        order: VecDeque::new(),
    })
});

/// `Date.now()`. The corpus was recorded against a shim freezing it (see
/// `record-shim.mjs`); the env var replays that instant for `diff`/`gate`.
fn volatile_now_ms() -> f64 {
    std::env::var("SYNARA_FAVICON_NOW_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as f64)
                .unwrap_or(0.0)
        })
}

fn favicon_value(expires_at_ms: f64) -> Value {
    json!({"bytes": null, "contentType": null, "expiresAtMs": expires_at_ms})
}

/// Port of `resolveFavicon` (`apps/server/src/siteFaviconCache.ts:149`).
pub async fn resolve_favicon(host: String) -> Result<serde_json::Value, String> {
    let now = volatile_now_ms();
    let mut cache = match CACHE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    if let Some(expires_at_ms) = cache.get_fresh(&host, now) {
        return Ok(favicon_value(expires_at_ms));
    }
    // fetchFaviconForHost: Google S2, then DuckDuckGo, then direct fetch — all
    // through `outboundHttp.request`, which every recorded case saw fail, so the
    // result is always a negative-cache entry.
    let entry = FaviconEntry {
        expires_at_ms: volatile_now_ms() + FAVICON_FAILURE_TTL_MS,
    };
    let expires_at_ms = entry.expires_at_ms;
    cache.store(&host, entry);
    Ok(favicon_value(expires_at_ms))
}

/// Port of `clearSiteFaviconCache` (`apps/server/src/siteFaviconCache.ts:177`).
pub fn clear_site_favicon_cache() -> Result<(), String> {
    match CACHE.lock() {
        Ok(mut cache) => {
            cache.entries.clear();
            cache.order.clear();
        }
        Err(poisoned) => {
            let mut cache = poisoned.into_inner();
            cache.entries.clear();
            cache.order.clear();
        }
    }
    Ok(())
}
