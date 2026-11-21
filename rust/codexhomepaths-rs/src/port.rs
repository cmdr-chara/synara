//! The Rust port of `apps/server/src/codexHomePaths.ts` (posix, darwin).

#![forbid(unsafe_code)]

#[path = "../../provideraccounthomepath-rs/src/port.rs"]
mod provider_home;

use provider_home::expand_provider_account_home_path;
use serde_json::Value;
use sha2::{Digest, Sha256};

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

fn member(v: &Value, key: &str) -> Result<Value, String> {
    match v {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        _ if is_undefined(v) => Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        )),
        Value::Object(m) => Ok(m.get(key).cloned().unwrap_or_else(crate::js::undefined)),
        _ => Ok(crate::js::undefined()),
    }
}

fn truthy(v: &Value) -> bool {
    if let Some(n) = crate::js::as_number(v) {
        return n != 0.0 && !n.is_nan();
    }
    match v {
        Value::Null | Value::Bool(false) => false,
        Value::Bool(true) => true,
        Value::String(s) => !s.is_empty(),
        Value::Object(_) => !is_undefined(v),
        Value::Array(_) => true,
        Value::Number(_) => false,
    }
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
}

/// `v?.trim()`: None on undefined/null, trimmed string, TypeError otherwise.
fn opt_trim(v: &Value, expr: &str) -> Result<Option<String>, String> {
    if is_undefined(v) || v.is_null() {
        return Ok(None);
    }
    match v {
        Value::String(s) => Ok(Some(js_trim(s).to_string())),
        _ => Err(format!("{expr}?.trim is not a function")),
    }
}

fn homedir() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/".to_string())
}

// --- posix path helpers (darwin host) ---

fn posix_normalize(p: &str) -> String {
    let is_abs = p.starts_with('/');
    let trailing = p.len() > 1 && p.ends_with('/');
    let mut out: Vec<&str> = Vec::new();
    for seg in p.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if out.last() == Some(&"..") || (out.is_empty() && !is_abs) {
                    out.push("..");
                } else {
                    out.pop();
                }
            }
            s => out.push(s),
        }
    }
    let mut r = if is_abs {
        format!("/{}", out.join("/"))
    } else {
        out.join("/")
    };
    if r.is_empty() {
        r = if is_abs {
            "/".to_string()
        } else {
            ".".to_string()
        };
    }
    if trailing && r != "/" {
        r.push('/');
    }
    r
}

fn posix_resolve(p: &str) -> String {
    if p.starts_with('/') {
        posix_normalize(p)
    } else {
        let cwd = std::env::current_dir()
            .map(|d| d.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "/".to_string());
        posix_normalize(&format!("{cwd}/{p}"))
    }
}

fn posix_dirname(p: &str) -> String {
    let s = p.trim_end_matches('/');
    let s = if s.is_empty() {
        if p.is_empty() {
            return ".".to_string();
        }
        "/"
    } else {
        s
    };
    match s.rfind('/') {
        None => ".".to_string(),
        Some(0) => "/".to_string(),
        Some(i) => s[..i].to_string(),
    }
}

fn posix_join(parts: &[&str]) -> String {
    posix_normalize(
        &parts
            .iter()
            .filter(|s| !s.is_empty())
            .copied()
            .collect::<Vec<_>>()
            .join("/"),
    )
}

// --- env view: a provided env object, or process.env when `??` substitutes ---

enum Env<'a> {
    Val(&'a Value),
    Proc,
}

impl Env<'_> {
    fn get(&self, key: &str) -> Result<Value, String> {
        match self {
            Env::Val(v) => member(v, key),
            Env::Proc => Ok(std::env::var(key)
                .map(Value::String)
                .unwrap_or_else(|_| crate::js::undefined())),
        }
    }
}

fn env_view(v: &Value) -> Env<'_> {
    if is_undefined(v) || v.is_null() {
        Env::Proc
    } else {
        Env::Val(v)
    }
}

// --- ports ---

fn base_home(env: &Env, explicit: &Value) -> Result<String, String> {
    // `explicitHomePath?.trim() || env.CODEX_HOME?.trim() || join(homedir(), ".codex")`
    // — `||` short-circuits, so CODEX_HOME is only read when the explicit trim is falsy.
    if let Some(t) = opt_trim(explicit, "explicitHomePath")?
        && !t.is_empty()
    {
        return expand_provider_account_home_path(t, None);
    }
    let codex_home = env.get("CODEX_HOME")?;
    if let Some(t) = opt_trim(&codex_home, "env.CODEX_HOME")?
        && !t.is_empty()
    {
        return expand_provider_account_home_path(t, None);
    }
    expand_provider_account_home_path(posix_join(&[&homedir(), ".codex"]), None)
}

fn overlay_home(env: &Env, source: &str, segment: Option<&str>) -> Result<String, String> {
    let synara_home = env.get("SYNARA_HOME")?;
    let root = match opt_trim(&synara_home, "env.SYNARA_HOME")? {
        Some(t) if !t.is_empty() => t,
        _ => posix_join(&[&posix_dirname(source), ".synara", "runtime"]),
    };
    let home = posix_join(&[&root, "codex-home-overlay"]);
    Ok(match segment {
        Some(seg) if !seg.is_empty() => posix_join(&[&home, "accounts", seg]),
        _ => home,
    })
}

fn account_segment(input: &Value) -> Result<Option<String>, String> {
    let account_id = opt_trim(&member(input, "accountId")?, "input.accountId")?;
    let shadow = opt_trim(&member(input, "shadowHomePath")?, "input.shadowHomePath")?;
    if (account_id.is_none() || account_id.as_deref() == Some("default")) && shadow.is_none() {
        return Ok(None);
    }
    // label: (accountId || "shadow").replace(/[^A-Za-z0-9_-]+/g, "-").slice(0,32) || "codex"
    let raw = account_id.as_deref().unwrap_or("shadow");
    let mut label = String::with_capacity(raw.len());
    let mut in_bad = false;
    for c in raw.chars() {
        if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
            label.push(c);
            in_bad = false;
        } else if !in_bad {
            label.push('-');
            in_bad = true;
        }
    }
    let label = if label.is_empty() {
        "codex".to_string()
    } else {
        label.chars().take(32).collect()
    };
    // sha256(accountId ?? "" + \0 + homePath ?? "" + \0 + shadowHomePath ?? "")
    let home = member(input, "homePath")?;
    let home_str = match &home {
        _ if is_undefined(&home) || home.is_null() => String::new(),
        Value::String(s) => s.clone(),
        _ => return Err("The \"data\" argument must be of type string".to_string()),
    };
    let mut h = Sha256::new();
    h.update(account_id.as_deref().unwrap_or("").as_bytes());
    h.update([0u8]);
    h.update(home_str.as_bytes());
    h.update([0u8]);
    h.update(shadow.as_deref().unwrap_or("").as_bytes());
    let digest: String = h
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .take(6)
        .collect();
    Ok(Some(format!("{label}-{digest}")))
}

fn truthy_string(v: &Value) -> bool {
    truthy(v)
}

/// Port of `resolveBaseCodexHomePath` (`apps/server/src/codexHomePaths.ts:26`).
pub fn resolve_base_codex_home_path(
    env: serde_json::Value,
    explicit_home_path: Option<String>,
) -> Result<String, String> {
    let explicit = explicit_home_path
        .map(Value::String)
        .unwrap_or_else(crate::js::undefined);
    base_home(&Env::Val(&env), &explicit)
}

/// Port of `resolveSynaraCodexHomeOverlayPath` (`apps/server/src/codexHomePaths.ts:35`).
pub fn resolve_synara_codex_home_overlay_path(
    env: serde_json::Value,
    source_home_path: String,
    account_segment: Option<String>,
) -> Result<String, String> {
    overlay_home(
        &Env::Val(&env),
        &source_home_path,
        account_segment.as_deref(),
    )
}

/// Port of `resolveCodexHomeOverlayAccountSegment` (`apps/server/src/codexHomePaths.ts:48`).
pub fn resolve_codex_home_overlay_account_segment(
    input: serde_json::Value,
) -> Result<Option<String>, String> {
    account_segment(&input)
}

/// Port of `resolveActiveCodexHomeWritePath` (`apps/server/src/codexHomePaths.ts:74`).
pub fn resolve_active_codex_home_write_path(input: serde_json::Value) -> Result<String, String> {
    let input = if is_undefined(&input) {
        Value::Object(serde_json::Map::new())
    } else {
        input
    };
    let env_member = member(&input, "env")?;
    let env = env_view(&env_member);
    let home_member = member(&input, "homePath")?;
    let source = base_home(&env, &home_member)?;
    // `{homePath: source, ...(accountId? {...}), ...(shadowHomePath ? {resolved} : {})}`
    let mut seg_input = serde_json::Map::new();
    seg_input.insert("homePath".to_string(), Value::String(source.clone()));
    let account_id = member(&input, "accountId")?;
    if truthy_string(&account_id) {
        seg_input.insert("accountId".to_string(), account_id);
    }
    let shadow_raw = member(&input, "shadowHomePath")?;
    if truthy_string(&shadow_raw) {
        seg_input.insert(
            "shadowHomePath".to_string(),
            Value::String(base_home(&env, &shadow_raw)?),
        );
    }
    let seg = account_segment(&Value::Object(seg_input))?;
    let overlay = overlay_home(&env, &source, seg.as_deref())?;
    Ok(if posix_resolve(&source) == posix_resolve(&overlay) {
        source
    } else {
        overlay
    })
}

/// Port of `resolveCodexHomeAllowlistCandidates` (`apps/server/src/codexHomePaths.ts:99`).
pub fn resolve_codex_home_allowlist_candidates(
    input: serde_json::Value,
) -> Result<Vec<String>, String> {
    let input = if is_undefined(&input) {
        Value::Object(serde_json::Map::new())
    } else {
        input
    };
    let env_member = member(&input, "env")?;
    let env = env_view(&env_member);
    let home_member = member(&input, "homePath")?;
    let source = base_home(&env, &home_member)?;
    let shadow_raw = member(&input, "shadowHomePath")?;
    let shadow = if truthy_string(&shadow_raw) {
        Some(base_home(&env, &shadow_raw)?)
    } else {
        None
    };
    let mut seg_input = serde_json::Map::new();
    seg_input.insert("homePath".to_string(), Value::String(source.clone()));
    let account_id = member(&input, "accountId")?;
    if truthy_string(&account_id) {
        seg_input.insert("accountId".to_string(), account_id);
    }
    if let Some(s) = &shadow {
        seg_input.insert("shadowHomePath".to_string(), Value::String(s.clone()));
    }
    let seg = account_segment(&Value::Object(seg_input))?;
    let overlay = overlay_home(&env, &source, seg.as_deref())?;
    let legacy = overlay_home(&env, &source, None)?;
    let mut candidates = if posix_resolve(&source) == posix_resolve(&overlay) {
        vec![source]
    } else {
        vec![source, overlay.clone()]
    };
    if posix_resolve(&legacy) != posix_resolve(&overlay)
        && !candidates
            .iter()
            .any(|c| posix_resolve(c) == posix_resolve(&legacy))
    {
        candidates.push(legacy);
    }
    if let Some(shadow) = shadow
        && !candidates
            .iter()
            .any(|c| posix_resolve(c) == posix_resolve(&shadow))
    {
        candidates.push(shadow);
    }
    Ok(candidates)
}
