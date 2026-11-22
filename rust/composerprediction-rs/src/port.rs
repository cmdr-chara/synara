//! The Rust port of `apps/server/src/composerPrediction/composerPrediction.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};

use crate::js;

const OMITTED_MARKER: &str = "[...]";
const USER_LABEL: &str = "User: ";
const ASSISTANT_LABEL: &str = "Assistant: ";
const MESSAGE_SEPARATOR: &str = "\n\n";
const MAX_EXCHANGES: usize = 4;
const MAX_CHARS: usize = 12_000;
const MAX_LATEST_REQUEST_CHARS: usize = 4_000;

const MIN_HEDGE_DELAY_MS: f64 = 6_000.0;
const DEFAULT_HEDGE_DELAY_MS: f64 = 10_000.0;

const EFFORT_ORDER: &[&str] = &[
    "none", "off", "minimal", "low", "medium", "high", "xhigh", "max", "ultra",
];

const GIT_TEXT_GENERATION_PROVIDERS: &[&str] =
    &["codex", "claudeAgent", "cursor", "opencode", "droid"];

const BUILT_IN_PROVIDER_KINDS: &[&str] = &[
    "codex",
    "claudeAgent",
    "cursor",
    "devin",
    "antigravity",
    "grok",
    "droid",
    "opencode",
    "pi",
    "omp",
];

const CATALOG: &str = include_str!("catalog.json");

fn catalog() -> &'static Value {
    static C: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    C.get_or_init(|| serde_json::from_str(CATALOG).unwrap_or(Value::Null))
}

fn js_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn prop<'a>(v: &'a Value, key: &str) -> &'a Value {
    v.get(key).unwrap_or(&Value::Null)
}

fn present(v: &Value) -> bool {
    !v.is_null() && !js::is_undefined(v)
}

fn is_missing(v: &Value) -> bool {
    v.is_null() || js::is_undefined(v)
}

/// UTF-16 code-unit length, matching JS `string.length`.
fn u16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

/// Slice by UTF-16 code-unit indices, matching JS `string.slice`.
fn u16_slice(s: &str, start: usize) -> String {
    s.encode_utf16()
        .skip(start)
        .map(|u| char::from_u32(u as u32).unwrap_or('\u{FFFD}'))
        .collect()
}

fn u16_head(s: &str, count: usize) -> String {
    s.encode_utf16()
        .take(count)
        .map(|u| char::from_u32(u as u32).unwrap_or('\u{FFFD}'))
        .collect()
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}' || c == '\u{00A0}')
}

fn as_str_units(v: &Value) -> Option<String> {
    js::string_units(v).map(|units| {
        units
            .iter()
            .map(|&u| char::from_u32(u as u32).unwrap_or('\u{FFFD}'))
            .collect()
    })
}

/// Port of `resolveComposerPredictionTurnBlocker`.
pub fn resolve_composer_prediction_turn_blocker(
    thread: Option<Value>,
    expected_turn_id: Value,
) -> Result<Option<String>, String> {
    let thread = match thread {
        Some(t) if js_truthy(&t) => t,
        _ => return Ok(Some("thread-missing".into())),
    };
    if present(prop(&thread, "archivedAt")) {
        return Ok(Some("thread-archived".into()));
    }
    let turn = prop(&thread, "latestTurn");
    if !js_truthy(turn) || !js_strict_eq(prop(turn, "turnId"), &expected_turn_id) {
        return Ok(Some("turn-not-latest".into()));
    }
    if prop(turn, "state").as_str() != Some("completed")
        || prop(turn, "completedAt").is_null()
        || prop(turn, "assistantMessageId").is_null()
    {
        return Ok(Some("turn-incomplete".into()));
    }
    let session_status = prop(prop(&thread, "session"), "status");
    if session_status.as_str() == Some("running") || session_status.as_str() == Some("starting") {
        return Ok(Some("session-busy".into()));
    }
    if prop(&thread, "hasPendingApprovals") == &Value::Bool(true) {
        return Ok(Some("pending-approval".into()));
    }
    if prop(&thread, "hasPendingUserInput") == &Value::Bool(true) {
        return Ok(Some("pending-user-input".into()));
    }
    Ok(None)
}

fn js_strict_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        _ => a == b,
    }
}

/// Port of `isComposerPredictionTurnCurrent`.
pub fn is_composer_prediction_turn_current(
    thread: Option<Value>,
    expected_turn_id: Value,
) -> Result<bool, String> {
    Ok(resolve_composer_prediction_turn_blocker(thread, expected_turn_id)?.is_none())
}

fn keep_tail(text: &str, max_chars: usize) -> String {
    if u16_len(text) <= max_chars {
        return text.to_string();
    }
    let tail = max_chars.saturating_sub(u16_len(OMITTED_MARKER) + 1);
    let skip = u16_len(text) - tail;
    format!("{OMITTED_MARKER}\n{}", u16_slice(text, skip))
}

fn keep_head_and_tail(text: &str, max_chars: usize) -> String {
    if u16_len(text) <= max_chars {
        return text.to_string();
    }
    let available = max_chars.saturating_sub(u16_len(OMITTED_MARKER) + 2);
    let head = available.div_ceil(2);
    let tail = available - head;
    let skip = u16_len(text) - tail;
    format!(
        "{}\n{OMITTED_MARKER}\n{}",
        u16_head(text, head),
        u16_slice(text, skip)
    )
}

fn joined_length(lines: &[String]) -> usize {
    lines
        .iter()
        .enumerate()
        .map(|(i, l)| u16_len(l) + if i > 0 { MESSAGE_SEPARATOR.len() } else { 0 })
        .sum()
}

/// Port of `buildComposerPredictionConversation`.
pub fn build_composer_prediction_conversation(input: Value) -> Result<Option<String>, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'messages')".into());
    }
    let messages = prop(&input, "messages");
    let Value::Array(messages) = messages else {
        return Err("Cannot read properties of undefined (reading 'findIndex')".into());
    };
    let final_id = prop(&input, "finalReplyMessageId");
    let Some(final_index) = messages
        .iter()
        .position(|m| js_strict_eq(prop(m, "id"), final_id))
    else {
        return Ok(None);
    };
    let final_reply = &messages[final_index];
    let reply_text = as_str_units(prop(final_reply, "text")).unwrap_or_default();
    if prop(final_reply, "role").as_str() != Some("assistant")
        || js_truthy(prop(final_reply, "streaming"))
        || js_trim(&reply_text).is_empty()
    {
        return Ok(None);
    }
    if messages[final_index + 1..]
        .iter()
        .any(|m| prop(m, "role").as_str() == Some("user"))
    {
        return Ok(None);
    }

    struct Exchange {
        requests: Vec<String>,
        reply: Option<String>,
    }
    let mut exchanges: Vec<Exchange> = Vec::new();
    for message in &messages[..=final_index] {
        let text = js_trim(&as_str_units(prop(message, "text")).unwrap_or_default()).to_string();
        if js_truthy(prop(message, "streaming")) || text.is_empty() {
            continue;
        }
        let role = prop(message, "role").as_str();
        if role == Some("user") {
            match exchanges.last_mut() {
                Some(cur) if cur.reply.is_none() => cur.requests.push(text),
                _ => exchanges.push(Exchange {
                    requests: vec![text],
                    reply: None,
                }),
            }
        } else if role == Some("assistant")
            && let Some(cur) = exchanges.last_mut()
        {
            cur.reply = Some(text);
        }
    }
    let latest = match exchanges.last() {
        Some(e) if e.reply.is_some() => e,
        _ => return Ok(None),
    };

    let mut request_lines: Vec<String> = Vec::new();
    for request in latest.requests.iter().rev() {
        let line = if request_lines.is_empty() {
            format!(
                "{USER_LABEL}{}",
                keep_head_and_tail(
                    request,
                    MAX_LATEST_REQUEST_CHARS.saturating_sub(u16_len(USER_LABEL))
                )
            )
        } else {
            format!("{USER_LABEL}{request}")
        };
        let mut probe = vec![line.clone()];
        probe.extend(request_lines.iter().cloned());
        if joined_length(&probe) > MAX_LATEST_REQUEST_CHARS {
            break;
        }
        request_lines.insert(0, line);
    }
    let reply_budget = MAX_CHARS
        .saturating_sub(joined_length(&request_lines))
        .saturating_sub(MESSAGE_SEPARATOR.len())
        .saturating_sub(u16_len(ASSISTANT_LABEL));
    let mut latest_lines = request_lines;
    latest_lines.push(format!(
        "{ASSISTANT_LABEL}{}",
        keep_tail(latest.reply.as_deref().unwrap_or_default(), reply_budget)
    ));

    let mut older_blocks: Vec<Vec<String>> = Vec::new();
    let mut used = joined_length(&latest_lines);
    let older_start = exchanges.len().saturating_sub(MAX_EXCHANGES);
    for exchange in exchanges[older_start..exchanges.len() - 1].iter().rev() {
        let mut lines: Vec<String> = exchange
            .requests
            .iter()
            .map(|r| format!("{USER_LABEL}{r}"))
            .collect();
        if let Some(reply) = &exchange.reply {
            lines.push(format!("{ASSISTANT_LABEL}{reply}"));
        }
        let next_used = used + joined_length(&lines) + MESSAGE_SEPARATOR.len();
        if next_used > MAX_CHARS {
            break;
        }
        older_blocks.insert(0, lines);
        used = next_used;
    }
    let mut out: Vec<String> = older_blocks.into_iter().flatten().collect();
    out.extend(latest_lines);
    Ok(Some(out.join(MESSAGE_SEPARATOR)))
}

fn normalize_model_slug(model: &str, provider: &str) -> Option<String> {
    let trimmed = js_trim(model);
    if trimmed.is_empty() {
        return None;
    }
    let scoped = if provider == "claudeAgent" {
        strip_claude_suffix(trimmed).to_string()
    } else if provider == "devin"
        && trimmed == trimmed.to_lowercase()
        && trimmed.ends_with("-medium")
    {
        trimmed[..trimmed.len() - "-medium".len()].to_string()
    } else {
        trimmed.to_string()
    };
    let aliases = &catalog()["aliases"][provider];
    if !aliases.is_object() {
        // `Object.prototype.hasOwnProperty.call(undefined, key)` throws.
        return Some("__THROW_CANNOT_CONVERT__".into());
    }
    let key = scoped.to_lowercase();
    let normalized = aliases
        .get(&key)
        .and_then(Value::as_str)
        .unwrap_or(&scoped)
        .to_string();
    Some(
        if provider == "claudeAgent" && claude_suffix(trimmed) == Some("1m".into()) {
            format!("{normalized}[1m]")
        } else {
            normalized
        },
    )
}

fn claude_suffix(model: &str) -> Option<String> {
    let end = model.rfind('[')?;
    if !model.ends_with(']') {
        return None;
    }
    Some(model[end + 1..model.len() - 1].to_lowercase())
}

fn strip_claude_suffix(model: &str) -> &str {
    if model.ends_with(']')
        && let Some(start) = model.rfind('[')
    {
        return &model[..start];
    }
    model
}

fn parse_claude_family_version(slug: &str) -> Option<(&'static str, u64, u64)> {
    // /^claude-(fable|opus|sonnet|haiku)-(\d{1,2})(?:-(\d{1,2}))?(?:-\d{8})?$/
    let rest = slug.trim().to_lowercase();
    let rest = rest.strip_prefix("claude-")?;
    let (family, rest) = rest.split_once('-')?;
    let family = match family {
        "fable" | "opus" | "sonnet" | "haiku" => family,
        _ => return None,
    };
    let mut parts = rest.split('-');
    let major: u64 = parts.next()?.parse().ok()?;
    if major > 99 {
        return None;
    }
    let mut minor = 0u64;
    if let Some(p) = parts.next() {
        if p.len() == 8 && p.chars().all(|c| c.is_ascii_digit()) && parts.next().is_none() {
            return Some((
                match family {
                    "fable" => "fable",
                    "opus" => "opus",
                    "sonnet" => "sonnet",
                    _ => "haiku",
                },
                major,
                minor,
            ));
        }
        minor = p.parse().ok()?;
        if minor > 99 {
            return None;
        }
        if let Some(d) = parts.next()
            && !(d.len() == 8 && d.chars().all(|c| c.is_ascii_digit()) && parts.next().is_none())
        {
            return None;
        }
    }
    Some((
        match family {
            "fable" => "fable",
            "opus" => "opus",
            "sonnet" => "sonnet",
            _ => "haiku",
        },
        major,
        minor,
    ))
}

fn resolve_grok_family(model: &str) -> &'static str {
    let slug = model.trim().to_lowercase();
    let mut major = None;
    let mut minor = None;
    if let Some(rest) = slug.strip_prefix("grok-") {
        let mut it = rest.split('.');
        if let (Ok(a), Ok(b)) = (
            it.next()
                .unwrap_or("")
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse::<u64>(),
            it.next()
                .unwrap_or("")
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse::<u64>(),
        ) && !rest.is_empty()
            && rest.chars().next().unwrap().is_ascii_digit()
        {
            major = Some(a);
            minor = Some(b);
        }
    }
    let versioned = matches!((major, minor), (Some(a), Some(b)) if a > 4 || (a == 4 && b >= 5));
    if !versioned
        && (slug.contains("build")
            || slug.contains("code-fast")
            || slug == "grok-4"
            || slug == "grok-4.3"
            || slug.starts_with("grok-4.3-"))
    {
        return "build";
    }
    let (Some(major), Some(minor)) = (major, minor) else {
        return "build";
    };
    if major < 4 || (major == 4 && minor <= 3) {
        "build"
    } else if major == 4 && minor == 5 {
        "4.5"
    } else {
        "4.6"
    }
}

fn model_capabilities(provider: &str, model: &str) -> Result<Value, String> {
    let normalized = normalize_model_slug(model, provider);
    let Some(normalized) = normalized else {
        return Ok(
            json!({"reasoningEffortLevels": [], "supportsFastMode": false, "supportsThinkingToggle": false, "promptInjectedEffortLevels": [], "contextWindowOptions": []}),
        );
    };
    if normalized == "__THROW_CANNOT_CONVERT__" {
        return Err("Cannot convert undefined or null to object".into());
    }
    let slug = if provider == "claudeAgent" {
        strip_claude_suffix(&normalized).to_string()
    } else {
        normalized
    };
    if let Some(caps) = catalog()["index"][provider].get(&slug) {
        return Ok(caps.clone());
    }
    if provider == "grok" {
        let grok = &catalog()["index"]["grok"];
        let caps = match resolve_grok_family(&slug) {
            "build" => grok.get("grok-build").cloned().unwrap_or(json!({"reasoningEffortLevels":[],"supportsFastMode":false,"supportsThinkingToggle":false,"promptInjectedEffortLevels":[],"contextWindowOptions":[]})),
            "4.5" => grok
                .get("grok-4.5")
                .or_else(|| grok.get("grok-4.6"))
                .cloned()
                .unwrap_or(json!({"reasoningEffortLevels":[],"supportsFastMode":false,"supportsThinkingToggle":false,"promptInjectedEffortLevels":[],"contextWindowOptions":[]})),
            _ => grok.get("grok-4.6").cloned().unwrap_or(json!({"reasoningEffortLevels":[],"supportsFastMode":false,"supportsThinkingToggle":false,"promptInjectedEffortLevels":[],"contextWindowOptions":[]})),
        };
        return Ok(caps);
    }
    if provider == "claudeAgent"
        && let Some(newest) = newest_claude_family_model(&slug)
        && let Some(caps) = catalog()["index"]["claudeAgent"].get(&newest)
    {
        return Ok(caps.clone());
    }
    Ok(
        json!({"reasoningEffortLevels":[],"supportsFastMode":false,"supportsThinkingToggle":false,"promptInjectedEffortLevels":[],"contextWindowOptions":[]}),
    )
}

fn newest_claude_family_model(slug: &str) -> Option<String> {
    let normalized = normalize_model_slug(slug, "claudeAgent")?;
    if normalized == "__THROW_CANNOT_CONVERT__" {
        return None;
    }
    let slug = strip_claude_suffix(&normalized).to_string();
    let slugs = catalog()["claudeSlugs"].as_array()?;
    if slugs.iter().any(|s| s.as_str() == Some(slug.as_str())) {
        return None;
    }
    let (family, major, minor) = parse_claude_family_version(&slug)?;
    let mut best: Option<(u64, u64, &str)> = None;
    for s in slugs {
        let Some(s) = s.as_str() else { continue };
        let Some((f, a, b)) = parse_claude_family_version(s) else {
            continue;
        };
        if f == family && best.is_none_or(|(ba, bb, _)| a > ba || (a == ba && b > bb)) {
            best = Some((a, b, s));
        }
    }
    let (ba, bb, s) = best?;
    if major > ba || (major == ba && minor > bb) {
        Some(s.to_string())
    } else {
        None
    }
}

/// Port of `lowestSupportedEffort`.
pub fn lowest_supported_effort(provider: Value, model: String) -> Result<Option<String>, String> {
    let provider_key = match &provider {
        Value::String(s) => s.clone(),
        other => js_key_coerce(other),
    };
    if js_trim(&model).is_empty() {
        // normalizeModelSlug returns null → EMPTY capabilities → no levels.
        return Ok(None);
    }
    let caps = model_capabilities(&provider_key, &model)?;
    let raw_levels: Vec<Value> = if provider_key == "opencode" {
        caps["variants"].as_array().cloned().unwrap_or_default()
    } else {
        caps["levels"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|l| {
                let cs = &l["controlSource"];
                cs.is_null() || cs.as_str() == Some("api-effort")
            })
            .collect()
    };
    let mut best: Option<(i64, String)> = None;
    for (index, level) in raw_levels.iter().enumerate() {
        let Some(value) = level["value"].as_str() else {
            continue;
        };
        let rank = EFFORT_ORDER
            .iter()
            .position(|e| *e == value)
            .map(|r| r as i64)
            .unwrap_or(EFFORT_ORDER.len() as i64 + index as i64);
        if best.as_ref().is_none_or(|(br, _)| rank < *br) {
            best = Some((rank, value.to_string()));
        }
    }
    Ok(best.map(|(_, v)| v))
}

fn js_key_coerce(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .map(|i| match i {
                Value::Null => String::new(),
                Value::String(s) => s.clone(),
                other => js_key_coerce(other),
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".into(),
    }
}

fn effort_option_key(provider: &str) -> &'static str {
    match provider {
        "claudeAgent" => "effort",
        "opencode" => "variant",
        _ => "reasoningEffort",
    }
}

fn with_lowest_effort(selection: &Value, provider: &str) -> Value {
    let model = prop(selection, "model").as_str().unwrap_or("").to_string();
    let Ok(Some(lowest)) = lowest_supported_effort(json!(provider), model) else {
        return selection.clone();
    };
    let key = effort_option_key(provider);
    let options = &selection["options"];
    let mut sel = selection.clone();
    if let Value::Array(items) = options {
        let mut next: Vec<Value> = items
            .iter()
            .filter(|o| prop(o, "id") != &json!(key))
            .cloned()
            .collect();
        next.push(json!({ "id": key, "value": lowest }));
        sel["options"] = json!(next);
    } else {
        let mut map = match options {
            Value::Object(m) => m.clone(),
            _ => Map::new(),
        };
        map.insert(key.to_string(), json!(lowest));
        sel["options"] = json!(map);
    }
    sel
}

/// Port of `resolveComposerPredictionHedgeDelayMs`.
pub fn resolve_composer_prediction_hedge_delay_ms(input: Value) -> Result<Option<f64>, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'recentDurationsMs')".into());
    }
    let durations = &input["recentDurationsMs"];
    let Value::Array(items) = durations else {
        return Err("Cannot read properties of undefined (reading 'length')".into());
    };
    if items.is_empty() {
        return Ok(Some(DEFAULT_HEDGE_DELAY_MS));
    }
    let mut sorted: Vec<f64> = items
        .iter()
        .map(|v| js::as_number(v).unwrap_or(f64::NAN))
        .collect();
    // V8 numeric sort: NaN comparator results are treated as 0 (no reorder).
    js::sort(&mut sorted, |a, b| a - b);
    let median = sorted[sorted.len() / 2];
    let delay_ms = if (median * 2.0).is_nan() {
        f64::NAN
    } else {
        f64::max(MIN_HEDGE_DELAY_MS, median * 2.0)
    };
    let timeout = js::as_number(&input["timeoutMs"]).unwrap_or(f64::NAN);
    if delay_ms + median <= timeout {
        Ok(Some(delay_ms))
    } else {
        Ok(None)
    }
}

const COMPOSER_PREDICTION_MODEL: &[(&str, &str)] = &[
    ("claudeAgent", "claude-sonnet-5-5"),
    ("codex", "gpt-6-luna"),
];

const UNRESOLVED_AUTOMATION_INSTANCE_PREFIX: &str = "synara_unresolved_automation_";

fn is_unresolved_automation_instance_id(id: &str) -> bool {
    id.starts_with(UNRESOLVED_AUTOMATION_INSTANCE_PREFIX)
}

fn is_provider_kind(v: &str) -> bool {
    BUILT_IN_PROVIDER_KINDS.contains(&v)
}

fn resolve_model_selection_instance_id(selection: &Value) -> String {
    prop(selection, "instanceId")
        .as_str()
        .or_else(|| prop(selection, "provider").as_str())
        .unwrap_or("codex")
        .to_string()
}

/// Bounded port of `deriveProviderInstances`: built-in defaults plus explicit
/// `settings.providerInstances` entries (legacy-account derivation is out of the
/// recorded surface).
fn derive_provider_instances(settings: &Value) -> Vec<Value> {
    let mut instances: Vec<Value> = Vec::new();
    for kind in BUILT_IN_PROVIDER_KINDS {
        if !is_unresolved_automation_instance_id(kind) {
            instances.push(json!({
                "instanceId": kind,
                "driver": kind,
                "enabled": true,
                "isDefault": true,
            }));
        }
    }
    if let Value::Object(map) = &settings["providerInstances"] {
        for (instance_id, raw) in map {
            if is_unresolved_automation_instance_id(instance_id) {
                continue;
            }
            let Some(driver) = prop(raw, "driver").as_str() else {
                continue;
            };
            if !is_provider_kind(driver) {
                continue;
            }
            if BUILT_IN_PROVIDER_KINDS.contains(&instance_id.as_str()) && instance_id != driver {
                continue;
            }
            let config = &raw["config"];
            let enabled = prop(raw, "enabled").as_bool() != Some(false)
                && prop(config, "enabled").as_bool() != Some(false);
            // Explicit entries merge over derived built-ins.
            if let Some(existing) = instances
                .iter_mut()
                .find(|i| prop(i, "instanceId").as_str() == Some(instance_id.as_str()))
            {
                existing["enabled"] = json!(enabled);
                existing["isDefault"] = json!(false);
            } else {
                instances.push(json!({
                    "instanceId": instance_id,
                    "driver": driver,
                    "enabled": enabled,
                    "isDefault": instance_id == driver,
                }));
            }
        }
    }
    instances
}

fn resolve_provider_instance(settings: &Value, instance_id: Option<&str>) -> Option<Value> {
    if let Some(id) = instance_id
        && is_unresolved_automation_instance_id(id)
    {
        return None;
    }
    let instances = derive_provider_instances(settings);
    if let Some(id) = instance_id {
        return instances
            .into_iter()
            .find(|i| prop(i, "instanceId").as_str() == Some(id));
    }
    instances
        .iter()
        .find(|i| prop(i, "instanceId").as_str() == Some("codex"))
        .cloned()
        .or_else(|| {
            instances
                .iter()
                .find(|i| prop(i, "isDefault") == &Value::Bool(true))
                .cloned()
        })
        .or_else(|| instances.into_iter().next())
}

/// Port of `resolveComposerPredictionRouting`.
pub fn resolve_composer_prediction_routing(input: Value) -> Result<Option<Value>, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'thread')".into());
    }
    let thread = &input["thread"];
    if js::is_undefined(thread) {
        return Err("Cannot read properties of undefined (reading 'modelSelection')".into());
    }
    if thread.is_null() {
        return Err("Cannot read properties of null (reading 'modelSelection')".into());
    }
    let settings = &input["settings"];
    let chat_selection = prop(thread, "modelSelection");
    let chat_instance_id = resolve_model_selection_instance_id(chat_selection);
    let session = prop(thread, "session");
    let session_instance_id = prop(session, "providerInstanceId");
    if !is_missing(session_instance_id)
        && session_instance_id.as_str() != Some(chat_instance_id.as_str())
    {
        return Ok(None);
    }
    let instance = match resolve_provider_instance(settings, Some(&chat_instance_id)) {
        Some(i) => i,
        None => return Ok(None),
    };
    if prop(&instance, "enabled") != &Value::Bool(true) {
        return Ok(None);
    }
    let driver = prop(&instance, "driver").as_str().unwrap_or("").to_string();
    if !GIT_TEXT_GENERATION_PROVIDERS.contains(&driver.as_str())
        || Some(driver.as_str()) != prop(chat_selection, "provider").as_str()
    {
        return Ok(None);
    }
    let configured = prop(settings, "textGenerationModelSelection");
    let use_configured = prop(configured, "provider").as_str() == Some(driver.as_str())
        && resolve_model_selection_instance_id(configured) == chat_instance_id;
    let prediction_model = COMPOSER_PREDICTION_MODEL
        .iter()
        .find(|(p, _)| *p == driver)
        .map(|(_, m)| *m);
    let source = if use_configured {
        "git-writing-model"
    } else if prediction_model.is_some() {
        "prediction-model"
    } else {
        "chat-model"
    };
    let base = if use_configured {
        configured.clone()
    } else if let Some(model) = prediction_model {
        json!({ "provider": driver, "model": model })
    } else {
        chat_selection.clone()
    };
    let selection = with_lowest_effort(&base, &driver);
    let mut routed_selection = selection.clone();
    routed_selection["provider"] = json!(driver);
    routed_selection["instanceId"] = prop(&instance, "instanceId").clone();
    if !GIT_TEXT_GENERATION_PROVIDERS
        .contains(&prop(&routed_selection, "provider").as_str().unwrap_or(""))
    {
        return Ok(None);
    }
    Ok(Some(json!({
        "modelSelection": routed_selection,
        "source": source,
    })))
}
