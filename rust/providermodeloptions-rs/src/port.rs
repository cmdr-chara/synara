//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the JavaScript function would throw. Use `crate::js` helpers for JS coercions
//! (utf16 strings, numbers, undefined) so every exported signature stays
//! (safe JSON inputs) -> Result<safe JSON output, String>.

#[path = "data.rs"]
mod data;

use serde_json::{Map, Value, json};

const OMP_ROLE_ISSUE: &str =
    "This saved OMP role is no longer supported. Choose an OMP model before sending.";
const COLLAPSIBLE_MODEL_GROUP_THRESHOLD: f64 = 3.0;

fn js_string(v: &Value) -> Option<String> {
    crate::js::string_units(v).map(|u| String::from_utf16_lossy(&u))
}

/// `v?.trim()` — `None` for nullish, `Err` for non-strings.
fn trim_opt(v: Option<&Value>) -> Result<Option<String>, String> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(s) => match js_string(s) {
            Some(t) => Ok(Some(crate::js::js_trim(&t).to_string())),
            None => Err("value.trim is not a function".into()),
        },
    }
}

/// `v.trim()` — `Err` for nullish and non-strings.
fn trim_req(v: Option<&Value>, name: &str) -> Result<String, String> {
    match v {
        None => Err("Cannot read properties of undefined (reading 'trim')".into()),
        Some(Value::Null) => Err("Cannot read properties of null (reading 'trim')".into()),
        Some(s) => match js_string(s) {
            Some(t) => Ok(crate::js::js_trim(&t).to_string()),
            None => Err(format!("{name}.trim is not a function")),
        },
    }
}

// ── shared/model helpers ────────────────────────────────────────────────

fn strip_bracket_suffix(s: &str) -> &str {
    // `\[([^\]]+)\]$`
    if s.ends_with(']')
        && let Some(open) = s[..s.len() - 1].rfind('[')
    {
        return &s[..open];
    }
    s
}

fn bracket_suffix(s: &str) -> Option<String> {
    // `\[([^\]]+)\]$`
    if s.ends_with(']')
        && let Some(open) = s[..s.len() - 1].rfind('[')
    {
        return Some(s[open + 1..s.len() - 1].to_lowercase());
    }
    None
}

fn strip_claude_context_window_suffix(s: &str) -> &str {
    strip_bracket_suffix(s)
}

fn get_claude_context_window_suffix(s: &str) -> Option<String> {
    bracket_suffix(s)
}

fn is_all_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

fn join_model_version_tokens(tokens: Vec<String>) -> Vec<String> {
    let mut merged: Vec<String> = Vec::new();
    for token in tokens {
        let previous = merged.last();
        if is_all_digits(&token)
            && (token == "0" || !token.starts_with('0'))
            && token.len() != 8
            && previous.is_some_and(|p| p.ends_with(|c: char| c.is_ascii_digit()))
        {
            let merged_token = format!("{}.{}", merged.pop().unwrap_or_default(), token);
            merged.push(merged_token);
        } else {
            merged.push(token);
        }
    }
    merged
}

fn token_display_name(lower: &str) -> Option<&'static str> {
    Some(match lower {
        "deepseek" => "DeepSeek",
        "glm" => "GLM",
        "gpt" => "GPT",
        "minimax" => "MiniMax",
        "openai" => "OpenAI",
        "opencode" => "OpenCode",
        "swe" => "SWE",
        "xai" => "xAI",
        "xhigh" => "XHigh",
        _ => return None,
    })
}

fn is_model_family_token(lower: &str) -> bool {
    token_display_name(lower).is_some()
        || matches!(
            lower,
            "adaptive"
                | "auto"
                | "claude"
                | "codex"
                | "composer"
                | "cursor"
                | "devin"
                | "gemini"
                | "grok"
                | "inkling"
                | "kimi"
                | "nemotron"
        )
}

/// `token.charAt(0).toUpperCase() + token.slice(1)` — astral first code points
/// come back unchanged because the surrogate halves rejoin.
fn humanize_model_token(token: &str) -> String {
    if let Some(d) = token_display_name(&token.to_lowercase()) {
        return d.to_string();
    }
    let mut chars = token.chars();
    match chars.next() {
        Some(c) if c.len_utf16() == 1 => {
            format!("{}{}", c.to_uppercase(), chars.as_str())
        }
        _ => token.to_string(),
    }
}

/// `\bGPT (\d)` → `GPT-$1`
fn restore_model_name_separators(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let bytes = name.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i..].starts_with(b"GPT ")
            && (i == 0 || !matches!(bytes[i - 1], b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_'))
            && bytes.get(i + 4).is_some_and(|b| b.is_ascii_digit())
        {
            out.push_str("GPT-");
            i += 4;
        } else {
            out.push_str(&name[i..i + 1]);
            i += 1;
        }
    }
    out
}

fn humanize_model_slug(slug: &str) -> String {
    if slug.contains('/') {
        return slug.to_string();
    }
    let tokens: Vec<String> = slug
        .split(['-', '_'])
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .collect();
    let merged = join_model_version_tokens(tokens)
        .iter()
        .map(|t| humanize_model_token(t))
        .collect::<Vec<_>>()
        .join(" ");
    restore_model_name_separators(&merged)
}

fn normalize_model_display_name(name: &str) -> String {
    let trimmed = crate::js::js_trim(name);
    let paren = trimmed.find('(');
    let head = paren
        .map(|i| crate::js::js_trim_end(&trimmed[..i]))
        .unwrap_or(trimmed);
    let tail = paren.map(|i| &trimmed[i..]).unwrap_or("");
    let tokens: Vec<String> = head
        .split(|c: char| c == '-' || c == '_' || c.is_whitespace())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .collect();
    let Some(first) = tokens.first() else {
        return trimmed.to_string();
    };
    if !is_model_family_token(&first.to_lowercase()) {
        return trimmed.to_string();
    }
    let normalized = join_model_version_tokens(tokens)
        .iter()
        .map(|t| {
            token_display_name(&t.to_lowercase())
                .unwrap_or(t)
                .to_string()
        })
        .collect::<Vec<_>>()
        .join(" ");
    let head_out = restore_model_name_separators(&normalized);
    if tail.is_empty() {
        head_out
    } else {
        format!("{head_out} {tail}")
    }
}

/// `formatModelDisplayName` — `None` = JS undefined.
fn format_model_display_name(model: &str) -> Option<String> {
    let trimmed = crate::js::js_trim(model);
    if trimmed.is_empty() {
        return None;
    }
    Some(
        data::model_name_by_slug(&trimmed.to_lowercase())
            .map(|s| s.to_string())
            .unwrap_or_else(|| humanize_model_slug(trimmed)),
    )
}

/// `normalizeModelSlug(model, provider)` — `None` = JS null.
fn normalize_model_slug(model: &Value, provider: &str) -> Option<String> {
    let trimmed = crate::js::js_trim(&js_string(model)?).to_string();
    if trimmed.is_empty() {
        return None;
    }
    let scoped = if provider == "claudeAgent" {
        strip_claude_context_window_suffix(&trimmed).to_string()
    } else if provider == "devin"
        && trimmed == trimmed.to_lowercase()
        && trimmed.ends_with("-medium")
    {
        trimmed[..trimmed.len() - "-medium".len()].to_string()
    } else {
        trimmed.clone()
    };
    let normalized = data::slug_alias(provider, &scoped.to_lowercase())
        .map(|s| s.to_string())
        .unwrap_or(scoped);
    if provider == "claudeAgent"
        && get_claude_context_window_suffix(&trimmed).as_deref() == Some("1m")
    {
        Some(format!("{normalized}[1m]"))
    } else {
        Some(normalized)
    }
}

struct ClaudeVersion {
    family: String,
    major: i64,
    minor: i64,
}

/// `^claude-(fable|opus|sonnet|haiku)-(\d{1,2})(?:-(\d{1,2}))?(?:-\d{8})?$`
fn parse_claude_family_version(slug: &str) -> Option<ClaudeVersion> {
    let s = crate::js::js_trim(slug).to_lowercase();
    let rest = s.strip_prefix("claude-")?;
    let (family, rest) = rest.split_once('-')?;
    if !matches!(family, "fable" | "opus" | "sonnet" | "haiku") {
        return None;
    }
    let parts: Vec<&str> = rest.split('-').collect();
    let (major_s, minor_s, extra) = match parts.as_slice() {
        [m] => (*m, "0", None),
        [m, n] => (*m, *n, None),
        [m, n, d] => (*m, *n, Some(*d)),
        _ => return None,
    };
    if major_s.len() > 2 || !is_all_digits(major_s) {
        return None;
    }
    if minor_s.len() > 2 || !is_all_digits(minor_s) {
        return None;
    }
    if let Some(d) = extra
        && (d.len() != 8 || !is_all_digits(d))
    {
        return None;
    }
    Some(ClaudeVersion {
        family: family.to_string(),
        major: major_s.parse().ok()?,
        minor: minor_s.parse().ok()?,
    })
}

/// `resolveNewestKnownClaudeFamilyModel` — `None` = JS null.
fn resolve_newest_known_claude_family_model(model: &Value) -> Option<String> {
    let normalized = normalize_model_slug(model, "claudeAgent")?;
    let slug = strip_claude_context_window_suffix(&normalized);
    if data::is_claude_catalog_slug(slug) {
        return None;
    }
    let version = parse_claude_family_version(slug)?;
    let newest = data::newest_claude_catalog_by_family(&version.family)?;
    let newest_v = parse_claude_family_version(newest)?;
    if (version.major, version.minor) > (newest_v.major, newest_v.minor) {
        Some(newest.to_string())
    } else {
        None
    }
}

/// `normalizeCursorModelVariantBaseId` — `None` = JS null.
fn normalize_cursor_model_variant_base_id(model: &Value) -> Option<String> {
    let trimmed = crate::js::js_trim(&js_string(model)?).to_string();
    if trimmed.is_empty() {
        return None;
    }
    let mut base = strip_bracket_suffix(&trimmed).to_string();
    for suffix in ["-fast", "", "-thinking", "-fast", ""] {
        if suffix == "-fast" {
            if let Some(b) = base.strip_suffix("-fast") {
                base = b.to_string();
            }
        } else if suffix == "-thinking" {
            if let Some(b) = base.strip_suffix("-thinking") {
                base = b.to_string();
            }
        } else {
            for e in ["-extra-high", "-none", "-low", "-medium", "-high", "-xhigh"] {
                if let Some(b) = base.strip_suffix(e) {
                    base = b.to_string();
                    break;
                }
            }
        }
    }
    if base.ends_with("-max") && !base.contains("codex-max") {
        base = base[..base.len() - "-max".len()].to_string();
    }
    // `^claude-(\d+(?:\.\d+)?)-([a-z]+)-max$` → `claude-$1-$2`
    if let Some((v, f)) = claude_reorder(&base, true) {
        base = format!("claude-{v}-{f}");
    }
    if let Some(b) = base.strip_suffix("-preview") {
        base = b.to_string();
    }
    // `^claude-(\d+(?:\.\d+)?)-([a-z]+)$` → `claude-${family}-${version.replace(".","-")}`
    if let Some((v, f)) = claude_reorder(&base, false) {
        return Some(format!("claude-{f}-{}", v.replace('.', "-")));
    }
    Some(base)
}

/// Parse `^claude-(\d+(?:\.\d+)?)-([a-z]+)(-max)?$` → (version, family).
fn claude_reorder(base: &str, with_max: bool) -> Option<(String, String)> {
    let s = if with_max {
        base.strip_suffix("-max")?
    } else {
        base
    };
    let rest = s.strip_prefix("claude-")?;
    let (version, family) = rest.rsplit_once('-')?;
    if family.is_empty() || !family.bytes().all(|b| b.is_ascii_lowercase()) {
        return None;
    }
    let is_num = |x: &str| !x.is_empty() && x.bytes().all(|b| b.is_ascii_digit());
    let mut it = version.split('.');
    let a = it.next()?;
    let b = it.next();
    if it.next().is_some() || !is_num(a) || b.is_some_and(|x| !is_num(x)) {
        return None;
    }
    Some((version.to_string(), family.to_string()))
}

/// `normalizeDynamicModelSlug(provider, slug)` on a raw wire value.
fn normalize_dynamic_model_slug(provider: &Value, slug: &Value) -> Result<String, String> {
    if crate::js::eq_str(provider, "claudeAgent") {
        let s = match js_string(slug) {
            Some(s) => s,
            None => {
                return Err(if slug.is_null() {
                    "Cannot read properties of null (reading 'replace')".into()
                } else if crate::js::is_undefined(slug) {
                    "Cannot read properties of undefined (reading 'replace')".into()
                } else {
                    "slug.replace is not a function".into()
                });
            }
        };
        let without = strip_claude_context_window_suffix(&s).to_string();
        return Ok(
            normalize_model_slug(&Value::String(without.clone()), "claudeAgent").unwrap_or(without),
        );
    }
    if crate::js::eq_str(provider, "grok") {
        return match js_string(slug) {
            Some(s) => Ok(crate::js::js_trim(&s).to_string()),
            None => Err(if slug.is_null() {
                "Cannot read properties of null (reading 'trim')".into()
            } else if crate::js::is_undefined(slug) {
                "Cannot read properties of undefined (reading 'trim')".into()
            } else {
                "slug.trim is not a function".into()
            }),
        };
    }
    if crate::js::eq_str(provider, "cursor") {
        if let Some(base) = normalize_cursor_model_variant_base_id(slug) {
            return Ok(base);
        }
        return match js_string(slug) {
            Some(s) => Ok(crate::js::js_trim(&s).to_string()),
            None => Err(if slug.is_null() {
                "Cannot read properties of null (reading 'trim')".into()
            } else if crate::js::is_undefined(slug) {
                "Cannot read properties of undefined (reading 'trim')".into()
            } else {
                "slug.trim is not a function".into()
            }),
        };
    }
    Ok(
        normalize_model_slug(slug, provider_str(provider).as_deref().unwrap_or("")).unwrap_or_else(
            || js_string(slug).unwrap_or_else(|| crate::js::to_json(slug).unwrap_or_default()),
        ),
    )
}

fn provider_str(provider: &Value) -> Option<String> {
    js_string(provider)
}

// ── exports ─────────────────────────────────────────────────────────────

pub fn get_omp_model_selection_issue(model: String, options: Value) -> Result<Value, String> {
    let selector = crate::js::js_trim(&model).to_string();
    if !selector.starts_with("role:") {
        return Ok(Value::Null);
    }
    let options = crate_js_iterate(&options, "options")?;
    for option in &options {
        if option.is_null() {
            return Err("Cannot read properties of null (reading 'slug')".into());
        }
        let slug = option.as_object().and_then(|o| o.get("slug"));
        let hit = slug.is_some_and(|s| crate::js::eq_str(s, &selector));
        if hit {
            let is_custom = option
                .as_object()
                .and_then(|o| o.get("isCustom"))
                .is_some_and(|v| matches!(v, Value::Bool(true)));
            if !is_custom {
                return Ok(Value::String(OMP_ROLE_ISSUE.into()));
            }
        }
    }
    Ok(Value::Null)
}

pub fn provider_model_option_provenance_label(input: Value) -> Result<String, String> {
    let option = crate::js::prop(&input, "option")?;
    let option = match option {
        Some(v) => v,
        None => {
            return Err(
                "Cannot read properties of undefined (reading 'upstreamProviderName')".into(),
            );
        }
    };
    if let Some(name) = trim_opt(crate::js::prop(option, "upstreamProviderName")?)?
        && !name.is_empty()
    {
        return Ok(name);
    }
    if let Some(id) = trim_opt(crate::js::prop(option, "upstreamProviderId")?)?
        && !id.is_empty()
    {
        return Ok(humanize_model_slug(&id));
    }
    // `input.option.slug.split("/", 1)[0]?.trim()` then `slug.includes("/")`.
    let slug = crate::js::prop(option, "slug")?;
    let slug_str = match slug {
        None => return Err("Cannot read properties of undefined (reading 'split')".into()),
        Some(Value::Null) => return Err("Cannot read properties of null (reading 'split')".into()),
        Some(v) => match js_string(v) {
            Some(s) => s,
            None => return Err("input.option.slug.split is not a function".into()),
        },
    };
    let slug_provider =
        crate::js::js_trim(slug_str.split('/').next().unwrap_or_default()).to_string();
    if slug_str.contains('/') && !slug_provider.is_empty() {
        return Ok(humanize_model_slug(&slug_provider));
    }
    let provider = crate::js::prop(&input, "provider")?;
    if let Some(p) = provider
        && let Some(name) = js_string(p).and_then(|s| data::provider_display_name(&s))
    {
        return Ok(name.to_string());
    }
    Ok(crate::js::to_json(&crate::js::undefined()).unwrap_or_default())
}

pub fn format_provider_model_option_name(input: Value) -> Result<String, String> {
    let provider = crate::js::prop(&input, "provider")?;
    let slug = crate::js::prop(&input, "slug")?;
    let raw_slug = trim_req(slug, "input.slug")?;
    let trimmed_slug = if provider.is_some_and(|p| crate::js::eq_str(p, "cursor")) {
        strip_bracket_suffix(&raw_slug).to_string()
    } else {
        raw_slug
    };
    if trimmed_slug.is_empty() {
        return Ok(trimmed_slug);
    }
    let scoped = provider.is_some_and(|p| {
        crate::js::eq_str(p, "opencode")
            || crate::js::eq_str(p, "pi")
            || crate::js::eq_str(p, "omp")
    });
    if scoped {
        let model_identifier = trimmed_slug
            .rsplit_once('/')
            .map(|(_, tail)| tail)
            .unwrap_or(&trimmed_slug);
        return Ok(format_model_display_name(model_identifier)
            .unwrap_or_else(|| humanize_model_slug(model_identifier)));
    }
    Ok(format_model_display_name(&trimmed_slug).unwrap_or(trimmed_slug))
}

pub fn normalize_claude_model_option_slug(model: Value) -> Result<String, String> {
    let resolved = crate::js::prop(&model, "resolvedModel")?;
    let resolved_slug = match resolved {
        Some(v) if crate::js::truthy(v) => Some(normalize_dynamic_model_slug(
            &Value::String("claudeAgent".into()),
            v,
        )?),
        _ => None,
    };
    let newest = resolved_slug
        .as_ref()
        .and_then(|s| resolve_newest_known_claude_family_model(&Value::String(s.clone())));
    if resolved_slug.is_some() && newest.is_some() {
        return Ok(resolved_slug.unwrap_or_default());
    }
    let slug = crate::js::prop(&model, "slug")?;
    normalize_dynamic_model_slug(
        &Value::String("claudeAgent".into()),
        slug.unwrap_or(&Value::Null),
    )
}

pub fn merge_dynamic_model_options(input: Value) -> Result<Vec<Value>, String> {
    let provider = crate::js::prop(&input, "provider")?.cloned();
    let static_options = crate::js::prop(&input, "staticOptions")?;
    let dynamic_models = crate::js::prop(&input, "dynamicModels")?;

    // `input.staticOptions.filter(m => !m.isCustom).map(m => [m.slug, m.name])`
    let static_opts = match static_options {
        None => return Err("Cannot read properties of undefined (reading 'filter')".into()),
        Some(Value::Null) => return Err("Cannot read properties of null (reading 'filter')".into()),
        Some(v) => match v.as_array() {
            Some(a) => a.clone(),
            None => return Err("input.staticOptions.filter is not a function".into()),
        },
    };
    let mut static_name_by_slug: Vec<(Value, Value)> = Vec::new();
    for model in &static_opts {
        let is_custom = match model {
            Value::Null => return Err("Cannot read properties of null (reading 'isCustom')".into()),
            v => crate::js::truthy(
                v.as_object()
                    .and_then(|o| o.get("isCustom"))
                    .unwrap_or(&Value::Null),
            ),
        };
        if !is_custom {
            let slug = model
                .as_object()
                .and_then(|o| o.get("slug"))
                .cloned()
                .unwrap_or_else(crate::js::undefined);
            let name = model
                .as_object()
                .and_then(|o| o.get("name"))
                .cloned()
                .unwrap_or_else(crate::js::undefined);
            if !static_name_by_slug.iter().any(|(s, _)| svz_eq(s, &slug)) {
                static_name_by_slug.push((slug, name));
            }
        }
    }

    let is_claude = provider
        .as_ref()
        .is_some_and(|p| crate::js::eq_str(p, "claudeAgent"));
    let is_scoped = provider.as_ref().is_some_and(|p| {
        crate::js::eq_str(p, "omp")
            || crate::js::eq_str(p, "pi")
            || crate::js::eq_str(p, "opencode")
    });

    let mut dynamic_normalized_slugs: Vec<String> = Vec::new();
    let mut normalized_dynamic_options: Vec<Value> = Vec::new();

    let models = match dynamic_models {
        None => return Err("input.dynamicModels is not iterable".into()),
        Some(v) => crate_js_iterate(v, "input.dynamicModels")?,
    };
    for dynamic_model in &models {
        let raw_name = trim_opt(crate::js::prop(dynamic_model, "name")?)?.unwrap_or_default();
        let mut claude_default_alias = false;
        if is_claude {
            let lower = raw_name.to_lowercase();
            let slug_is_default =
                if lower == "default (recommended)" || lower == "default recommended" {
                    true
                } else {
                    trim_req(crate::js::prop(dynamic_model, "slug")?, "dynamicModel.slug")?
                        .to_lowercase()
                        == "default"
                };
            claude_default_alias = slug_is_default;
        }
        if claude_default_alias {
            continue;
        }

        let model_slug = crate::js::prop(dynamic_model, "slug")?;
        let normalized_slug = if is_claude {
            normalize_claude_model_option_slug(dynamic_model.clone())?
        } else {
            normalize_dynamic_model_slug(
                provider.as_ref().unwrap_or(&Value::Null),
                model_slug.unwrap_or(&Value::Null),
            )?
        };
        let model_identifier = normalized_slug
            .rsplit_once('/')
            .map(|(_, tail)| tail)
            .unwrap_or(&normalized_slug)
            .to_string();
        let display_name_fallback = format_provider_model_option_name(json!({
            "provider": provider.clone().unwrap_or(Value::Null),
            "slug": normalized_slug,
        }))?;
        if dynamic_normalized_slugs.contains(&normalized_slug) {
            continue;
        }
        dynamic_normalized_slugs.push(normalized_slug.clone());

        let static_name = static_name_by_slug
            .iter()
            .find(|(s, _)| crate::js::eq_str(s, &normalized_slug))
            .map(|(_, n)| n.clone());
        let uncatalogued_claude = is_claude
            && resolve_newest_known_claude_family_model(&Value::String(normalized_slug.clone()))
                .is_some();
        let slug_trim = trim_opt(model_slug)?.unwrap_or_default();
        let picked_name = static_name
            .or_else(|| {
                if uncatalogued_claude {
                    Some(Value::String(display_name_fallback.clone()))
                } else {
                    None
                }
            })
            .or_else(|| {
                if !raw_name.is_empty()
                    && raw_name != slug_trim
                    && raw_name != normalized_slug
                    && raw_name != model_identifier
                {
                    Some(Value::String(normalize_model_display_name(&raw_name)))
                } else {
                    Some(Value::String(display_name_fallback.clone()))
                }
            })
            .unwrap_or_else(crate::js::undefined);

        let mut option = Map::new();
        option.insert("slug".into(), Value::String(normalized_slug));
        option.insert("name".into(), picked_name);
        if let Some(d) = trim_opt(crate::js::prop(dynamic_model, "description")?)?
            && !d.is_empty()
        {
            option.insert("description".into(), Value::String(d));
        }
        if let Some(u) = trim_opt(crate::js::prop(dynamic_model, "upstreamProviderId")?)?
            && !u.is_empty()
        {
            option.insert("upstreamProviderId".into(), Value::String(u));
        }
        if let Some(u) = trim_opt(crate::js::prop(dynamic_model, "upstreamProviderName")?)?
            && !u.is_empty()
        {
            option.insert("upstreamProviderName".into(), Value::String(u));
        }
        normalized_dynamic_options.push(Value::Object(option));
    }

    // id-part counts for scoped providers
    let mut id_part_counts: Vec<(String, usize)> = Vec::new();
    if is_scoped {
        for option in &normalized_dynamic_options {
            let slug = option
                .as_object()
                .and_then(|o| o.get("slug"))
                .and_then(js_string)
                .unwrap_or_default();
            let part = slug
                .rsplit_once('/')
                .map(|(_, tail)| tail.to_string())
                .unwrap_or(slug);
            if let Some((_, c)) = id_part_counts.iter_mut().find(|(p, _)| *p == part) {
                *c += 1;
            } else {
                id_part_counts.push((part, 1));
            }
        }
    }

    let is_droid = provider
        .as_ref()
        .is_some_and(|p| crate::js::eq_str(p, "droid"));
    let custom_only: Vec<Value> = if is_droid {
        Vec::new()
    } else {
        let mut out = Vec::new();
        for model in &static_opts {
            // `!("isCustom" in model) || !model.isCustom`
            if !model.is_object() {
                return Err("Cannot use 'in' operator to search for 'isCustom' in value".into());
            }
            let is_custom = crate::js::truthy(
                model
                    .as_object()
                    .and_then(|o| o.get("isCustom"))
                    .unwrap_or(&Value::Null),
            );
            if !is_custom {
                continue;
            }
            let slug = model
                .as_object()
                .and_then(|o| o.get("slug"))
                .cloned()
                .unwrap_or_else(crate::js::undefined);
            let normalized_custom =
                normalize_dynamic_model_slug(provider.as_ref().unwrap_or(&Value::Null), &slug)?;
            if dynamic_normalized_slugs.contains(&normalized_custom) {
                continue;
            }
            if is_scoped
                && !normalized_custom.contains('/')
                && id_part_counts
                    .iter()
                    .find(|(p, _)| *p == normalized_custom)
                    .is_some_and(|(_, c)| *c == 1)
            {
                continue;
            }
            out.push(model.clone());
        }
        out
    };

    let static_built_ins: Vec<&Value> = static_opts
        .iter()
        .filter(|model| {
            if !model.is_object() {
                return false;
            }
            !crate::js::truthy(
                model
                    .as_object()
                    .and_then(|o| o.get("isCustom"))
                    .unwrap_or(&Value::Null),
            )
        })
        .collect();

    let authoritative = provider
        .as_ref()
        .is_some_and(|p| crate::js::eq_str(p, "codex"))
        || (!normalized_dynamic_options.is_empty()
            && provider.as_ref().is_some_and(|p| {
                crate::js::eq_str(p, "antigravity")
                    || crate::js::eq_str(p, "opencode")
                    || crate::js::eq_str(p, "cursor")
                    || crate::js::eq_str(p, "droid")
                    || crate::js::eq_str(p, "grok")
                    || crate::js::eq_str(p, "devin")
            }));
    let missing_built_ins: Vec<Value> = if authoritative {
        Vec::new()
    } else {
        static_built_ins
            .iter()
            .filter(|model| {
                let slug = model.as_object().and_then(|o| o.get("slug"));
                !slug.is_some_and(|s| {
                    js_string(s).is_some_and(|ss| dynamic_normalized_slugs.contains(&ss))
                })
            })
            .map(|m| (*m).clone())
            .collect()
    };

    if is_claude {
        let mut combined = normalized_dynamic_options;
        combined.extend(missing_built_ins);
        combined.sort_by(|l, r| {
            claude_model_rank(l)
                .partial_cmp(&claude_model_rank(r))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        combined.extend(custom_only);
        return Ok(combined);
    }

    let mut out = normalized_dynamic_options;
    out.extend(missing_built_ins);
    out.extend(custom_only);
    Ok(out)
}

/// `claudeModelRank(option.slug)` — `.slug` on null options throws.
fn claude_model_rank(option: &Value) -> f64 {
    let slug = option
        .as_object()
        .and_then(|o| o.get("slug"))
        .and_then(js_string)
        .unwrap_or_default();
    if let Some(rank) = data::claude_catalog_rank(&slug) {
        return rank;
    }
    let family_rank = resolve_newest_known_claude_family_model(&Value::String(slug))
        .and_then(|n| data::claude_catalog_rank(&n));
    match family_rank {
        Some(r) => r - 0.5,
        None => -1.0,
    }
}

fn svz_eq(a: &Value, b: &Value) -> bool {
    if crate::js::is_undefined(a) || crate::js::is_undefined(b) {
        return crate::js::is_undefined(a) && crate::js::is_undefined(b);
    }
    if let (Some(x), Some(y)) = (crate::js::string_units(a), crate::js::string_units(b)) {
        return x == y;
    }
    if let (Some(x), Some(y)) = (crate::js::as_number(a), crate::js::as_number(b)) {
        return x == y || (x.is_nan() && y.is_nan());
    }
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        _ => false,
    }
}

fn crate_js_iterate(v: &Value, name: &str) -> Result<Vec<Value>, String> {
    match v {
        Value::Array(a) => Ok(a.clone()),
        Value::Null => Err(format!("{name} is not iterable")),
        _ if crate::js::is_undefined(v) => Err(format!("{name} is not iterable")),
        _ if crate::js::string_units(v).is_some() => Ok(String::from_utf16_lossy(
            &crate::js::string_units(v).unwrap_or_default(),
        )
        .chars()
        .map(|c| Value::String(c.to_string()))
        .collect()),
        _ => Err(format!("{name} is not iterable")),
    }
}

pub fn provider_model_cost_multiplier_label(
    description: Option<String>,
) -> Result<Option<String>, String> {
    // `description?.trim().match(...)` — the whole chain short-circuits.
    let Some(description) = description else {
        return Ok(None);
    };
    let d = crate::js::js_trim(&description).to_lowercase();
    // `^(\d+(?:\.\d+)?)x(?:\s|$)`
    let bytes = d.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i > 0 && bytes.get(i) == Some(&b'.') {
        let mut j = i + 1;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
        }
        if j > i + 1 {
            i = j;
        }
    }
    if i > 0 && bytes.get(i) == Some(&b'x') {
        let after = i + 1;
        if after >= bytes.len() || bytes[after].is_ascii_whitespace() {
            return Ok(Some(format!("{}×", &crate::js::js_trim(&description)[..i])));
        }
    }
    Ok(None)
}

pub fn group_provider_model_options(options: Value) -> Result<Vec<Value>, String> {
    let options = crate_js_iterate(&options, "options")?;
    let mut grouped: Vec<Value> = Vec::new();
    let mut index_by_key: Vec<(String, usize)> = Vec::new();
    for option in &options {
        let id = trim_opt(crate::js::prop(option, "upstreamProviderId")?)?;
        let name = trim_opt(crate::js::prop(option, "upstreamProviderName")?)?;
        let group_label = name
            .filter(|n| !n.is_empty())
            .or_else(|| id.clone().filter(|i| !i.is_empty()));
        let group_key = match &group_label {
            Some(_) => id
                .unwrap_or_else(|| group_label.clone().unwrap_or_default())
                .to_lowercase(),
            None => "__ungrouped__".to_string(),
        };
        if let Some((_, idx)) = index_by_key.iter().find(|(k, _)| *k == group_key) {
            if let Some(Value::Object(g)) = grouped.get_mut(*idx)
                && let Some(Value::Array(opts)) = g.get_mut("options")
            {
                opts.push(option.clone());
            }
            continue;
        }
        index_by_key.push((group_key.clone(), grouped.len()));
        grouped.push(json!({
            "key": group_key,
            "label": group_label,
            "options": [option.clone()],
        }));
    }
    Ok(grouped)
}

pub fn group_provider_model_options_with_favorites(input: Value) -> Result<Vec<Value>, String> {
    let favorites = crate::js::prop(&input, "favoriteSlugs")?;
    // `input.favoriteSlugs.size === 0`
    let size_zero = match favorites {
        None => return Err("Cannot read properties of undefined (reading 'size')".into()),
        Some(Value::Null) => return Err("Cannot read properties of null (reading 'size')".into()),
        Some(v) => {
            matches!(v.as_object().and_then(|o| o.get("size")), Some(n) if crate::js::as_number(n) == Some(0.0))
        }
    };
    let options = match crate::js::prop(&input, "options")? {
        None => return Err("Cannot read properties of undefined (reading 'filter')".into()),
        Some(Value::Null) => return Err("Cannot read properties of null (reading 'filter')".into()),
        Some(v) => match v.as_array() {
            Some(a) => a.clone(),
            None => return Err("input.options.filter is not a function".into()),
        },
    };
    let favorites = favorites.unwrap_or(&Value::Null);
    if size_zero {
        return group_provider_model_options(Value::Array(options));
    }
    let fav_has = |_slug: &Value| -> Result<bool, String> {
        if favorites.is_null() || crate::js::is_undefined(favorites) {
            return Err("Cannot read properties of null (reading 'has')".into());
        }
        Err("input.favoriteSlugs.has is not a function".into())
    };
    let mut favorite_options = Vec::new();
    let mut rest = Vec::new();
    for option in &options {
        let slug = option
            .as_object()
            .and_then(|o| o.get("slug"))
            .cloned()
            .unwrap_or_else(crate::js::undefined);
        if fav_has(&slug)? {
            favorite_options.push(option.clone());
        } else {
            rest.push(option.clone());
        }
    }
    if favorite_options.is_empty() {
        return group_provider_model_options(Value::Array(options));
    }
    let mut grouped = group_provider_model_options(Value::Array(rest))?;
    let label = crate::js::prop(&input, "favoriteLabel")?
        .cloned()
        .unwrap_or_else(|| Value::String("Favourites".into()));
    grouped.insert(
        0,
        json!({
            "key": "__favorites__",
            "label": label,
            "options": favorite_options,
        }),
    );
    Ok(grouped)
}

pub fn should_use_collapsible_model_groups(
    group_count: f64,
    is_searching: bool,
) -> Result<bool, String> {
    Ok(group_count >= COLLAPSIBLE_MODEL_GROUP_THRESHOLD && !is_searching)
}

pub fn resolve_model_group_default_open(input: Value) -> Result<bool, String> {
    let group_count = crate::js::prop(&input, "groupCount")?
        .and_then(crate::js::as_number)
        .unwrap_or(f64::NAN);
    if group_count < COLLAPSIBLE_MODEL_GROUP_THRESHOLD {
        return Ok(true);
    }
    if crate::js::prop(&input, "groupKey")?.is_some_and(|k| crate::js::eq_str(k, "__favorites__")) {
        return Ok(true);
    }
    let options = match crate::js::prop(&input, "options")? {
        None => return Err("Cannot read properties of undefined (reading 'some')".into()),
        Some(Value::Null) => return Err("Cannot read properties of null (reading 'some')".into()),
        Some(v) => match v.as_array() {
            Some(a) => a.clone(),
            None => return Err("input.options.some is not a function".into()),
        },
    };
    let active = crate::js::prop(&input, "activeModel")?
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    for option in &options {
        if option.is_null() {
            return Err("Cannot read properties of null (reading 'slug')".into());
        }
        let slug = option
            .as_object()
            .and_then(|o| o.get("slug"))
            .cloned()
            .unwrap_or_else(crate::js::undefined);
        if svz_eq(&slug, &active) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// `{...a, ...b}` — strings spread per code point into index keys.
fn spread_into(dst: &mut Map<String, Value>, v: &Value) {
    match v {
        Value::Object(o) => {
            for (k, val) in o {
                dst.insert(k.clone(), val.clone());
            }
        }
        serde_json::Value::Array(a) => {
            for (i, val) in a.iter().enumerate() {
                dst.insert(i.to_string(), val.clone());
            }
        }
        _ if crate::js::string_units(v).is_some() => {
            for (i, c) in String::from_utf16_lossy(&crate::js::string_units(v).unwrap_or_default())
                .chars()
                .enumerate()
            {
                dst.insert(i.to_string(), Value::String(c.to_string()));
            }
        }
        _ => {}
    }
}

pub fn build_next_provider_options(
    provider: Value,
    model_options: Option<Value>,
    patch: Value,
) -> Result<Value, String> {
    if crate::js::eq_str(&provider, "codex") {
        let mut next = Map::new();
        if let Some(mo) = &model_options {
            spread_into(&mut next, mo);
        }
        spread_into(&mut next, &patch);
        // `"fastMode" in patch`
        match &patch {
            Value::Object(o) if o.contains_key("fastMode") => {
                next.shift_remove("serviceTier");
                return Ok(Value::Object(next));
            }
            Value::Object(_) => {}
            _ => return Err("Cannot use 'in' operator to search for 'fastMode' in patch".into()),
        }
        if crate::js::eq_str(next.get("serviceTier").unwrap_or(&Value::Null), "ultrafast") {
            next.shift_remove("fastMode");
        }
        return Ok(Value::Object(next));
    }
    let mut next = Map::new();
    if let Some(mo) = &model_options {
        spread_into(&mut next, mo);
    }
    spread_into(&mut next, &patch);
    Ok(Value::Object(next))
}

pub fn build_provider_option_patch(
    provider: Value,
    option_id: String,
    value: Value,
) -> Result<Value, String> {
    let _ = &provider;
    let mut out = Map::new();
    out.insert(option_id, value);
    Ok(Value::Object(out))
}

pub fn build_model_selection(
    provider: Value,
    model: String,
    options: Option<Value>,
    supports_auto_mode_or_metadata: Value,
    explicit_metadata: Value,
) -> Result<Value, String> {
    // `typeof supportsAutoModeOrMetadata === "boolean"` / `=== "object"`
    let supports_auto_mode = match &supports_auto_mode_or_metadata {
        Value::Bool(b) => Some(*b),
        _ => None,
    };
    let is_object = matches!(
        &supports_auto_mode_or_metadata,
        Value::Object(_) | Value::Array(_) | Value::Null
    );
    let metadata = if is_object {
        supports_auto_mode_or_metadata.clone()
    } else {
        explicit_metadata
    };

    let attach = |selection: &mut Map<String, Value>| -> Result<(), String> {
        let instance_id = if metadata.is_null() || crate::js::is_undefined(&metadata) {
            None
        } else {
            trim_opt(crate::js::prop(&metadata, "instanceId")?)?
        };
        if let Some(id) = instance_id
            && !id.is_empty()
        {
            selection.insert("instanceId".into(), Value::String(id));
        }
        Ok(())
    };

    let providers_with_options_key = [
        "antigravity",
        "codex",
        "cursor",
        "devin",
        "grok",
        "droid",
        "opencode",
        "pi",
        "omp",
    ];
    let provider_matches = |names: &[&str]| names.iter().any(|n| crate::js::eq_str(&provider, n));

    let mut selection = Map::new();
    if !provider_matches(&providers_with_options_key)
        && !crate::js::eq_str(&provider, "claudeAgent")
    {
        // switch with no default → undefined
        return Ok(crate::js::undefined());
    }
    selection.insert("provider".into(), provider.clone());
    selection.insert("model".into(), Value::String(model));
    if options.as_ref().is_some_and(crate::js::truthy) {
        selection.insert("options".into(), options.clone().unwrap_or(Value::Null));
    }
    if crate::js::eq_str(&provider, "claudeAgent")
        && let Some(b) = supports_auto_mode
    {
        selection.insert("supportsAutoMode".into(), Value::Bool(b));
    }
    attach(&mut selection)?;
    Ok(Value::Object(selection))
}
