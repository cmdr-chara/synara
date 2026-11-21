//! The Rust port of `apps/web/src/cursorModelVariants.ts` —
//! `normalizeCursorModelVariantBaseId` and `collapseCursorModelVariants`, plus the
//! `parseCursorCliReasoningEffort` helper they call (`packages/shared/src/model.ts:108`).

#![forbid(unsafe_code)]

use serde_json::{Value, json};

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

fn js_undefined() -> Value {
    json!({"$undefined": true})
}

fn is_falsy(v: &Value) -> bool {
    match v {
        Value::Null | Value::Bool(false) => true,
        Value::String(s) => s.is_empty(),
        Value::Number(n) => n.as_f64().is_some_and(|x| x == 0.0 || x.is_nan()),
        Value::Object(o) => {
            o.contains_key("$undefined")
                || (o.contains_key("$num") || o.contains_key("$bigint"))
                    && crate::js::as_number(v).is_some_and(|x| x == 0.0 || x.is_nan())
                || o.contains_key("$utf16")
                    && crate::js::string_units(v).is_some_and(|u| u.is_empty())
        }
        _ => false,
    }
}

/// `value[key]` with JS semantics: null/undefined receivers throw TypeError.
fn prop<'v>(v: &'v Value, k: &str) -> Result<Option<&'v Value>, String> {
    if v.is_null() || is_undefined(v) {
        return Err("Cannot read properties of null or undefined".to_string());
    }
    match v {
        Value::Object(o) => Ok(o.get(k).filter(|x| !is_undefined(x))),
        _ => Ok(None),
    }
}

fn str_value(v: &Value) -> Option<String> {
    crate::js::string_units(v).and_then(|u| String::from_utf16(&u).ok())
}

fn is_js_whitespace(c: char) -> bool {
    c.is_whitespace() || c == '\u{feff}'
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_whitespace)
}

/// `slug.endsWith("-fast")` — non-string or absent slug throws TypeError.
fn js_ends_with(v: Option<&Value>, suffix: &str) -> Result<bool, String> {
    match v {
        None => Err("Cannot read properties of undefined (reading 'endsWith')".to_string()),
        Some(x) if x.is_null() => {
            Err("Cannot read properties of null (reading 'endsWith')".to_string())
        }
        Some(x) => match str_value(x) {
            Some(s) => Ok(s.ends_with(suffix)),
            None => Err("slug.endsWith is not a function".to_string()),
        },
    }
}

/// `a === b` under the wire codec (primitives only; distinct objects never ===).
fn js_strict_eq(a: &Value, b: &Value) -> bool {
    if a.is_null() && b.is_null() {
        return true;
    }
    if is_undefined(a) || is_undefined(b) {
        return is_undefined(a) && is_undefined(b);
    }
    if let (Some(x), Some(y)) = (crate::js::as_number(a), crate::js::as_number(b)) {
        return x == y;
    }
    if let (Some(x), Some(y)) = (crate::js::string_units(a), crate::js::string_units(b)) {
        return x == y;
    }
    matches!((a, b), (Value::Bool(x), Value::Bool(y)) if x == y)
}

// --- normalizeCursorModelVariantBaseId ---------------------------------------

/// `value.trim().replace(/\[[^\]]*\]$/u, "")`.
fn strip_parameterized_suffix(s: &str) -> String {
    if s.ends_with(']')
        && let Some(open) = s.rfind('[')
        && !s[open + 1..s.len() - 1].contains(']')
    {
        return s[..open].to_string();
    }
    s.to_string()
}

const EFFORT_SUFFIXES: [&str; 6] = ["-extra-high", "-none", "-low", "-medium", "-high", "-xhigh"];

fn strip_effort_suffix(base: &mut String) {
    for suf in EFFORT_SUFFIXES {
        if let Some(b) = base.strip_suffix(suf) {
            base.truncate(b.len());
            break;
        }
    }
}

fn normalize_str(trimmed: &str) -> String {
    let mut base = strip_parameterized_suffix(trimmed);
    for pattern in ["-fast", "", "-thinking", "-fast", ""] {
        if pattern == "-fast" || pattern == "-thinking" {
            if let Some(b) = base.strip_suffix(pattern) {
                base.truncate(b.len());
            }
        } else {
            strip_effort_suffix(&mut base);
        }
    }
    if base.ends_with("-max") && !base.contains("codex-max") {
        base.truncate(base.len() - "-max".len());
    }
    // /^claude-(\d+(?:\.\d+)?)-([a-z]+)-max$/ → "claude-$1-$2"
    if let Some(rest) = base
        .strip_prefix("claude-")
        .and_then(|r| r.strip_suffix("-max"))
        && let Some((version, family)) = rest.split_once('-')
        && is_claude_version(version)
        && is_lower_alpha(family)
    {
        base = format!("claude-{version}-{family}");
    }
    if let Some(b) = base.strip_suffix("-preview") {
        base.truncate(b.len());
    }
    // /^claude-(\d+(?:\.\d+)?)-([a-z]+)$/ → `claude-${family}-${version.replace(".", "-")}`
    if let Some(rest) = base.strip_prefix("claude-")
        && let Some((version, family)) = rest.split_once('-')
        && is_claude_version(version)
        && is_lower_alpha(family)
    {
        return format!("claude-{family}-{}", version.replacen('.', "-", 1));
    }
    base
}

fn is_claude_version(v: &str) -> bool {
    let dots = v.chars().filter(|c| *c == '.').count();
    dots <= 1
        && !v.is_empty()
        && v.split('.')
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

fn is_lower_alpha(f: &str) -> bool {
    !f.is_empty() && f.chars().all(|c| c.is_ascii_lowercase())
}

/// Shared inner: `model?.trim()` — non-string non-nullish throws TypeError.
fn normalize_inner(model: Option<&Value>) -> Result<Option<String>, String> {
    let Some(model) = model else {
        return Ok(None);
    };
    if model.is_null() || is_undefined(model) {
        return Ok(None);
    }
    let Some(s) = str_value(model) else {
        return Err("model.trim is not a function".to_string());
    };
    let trimmed = js_trim(&s);
    if trimmed.is_empty() {
        return Ok(None);
    }
    Ok(Some(normalize_str(trimmed)))
}

/// Port of `normalizeCursorModelVariantBaseId`.
pub fn normalize_cursor_model_variant_base_id(
    model: Option<String>,
) -> Result<Option<String>, String> {
    match model.as_deref().map(js_trim) {
        None | Some("") => Ok(None),
        Some(t) => Ok(Some(normalize_str(t))),
    }
}

// --- parseCursorCliReasoningEffort ---------------------------------------------

/// `model.trim().toLowerCase().split("-")` scanned back to front.
fn parse_cli_reasoning_effort(model: &str) -> Option<&'static str> {
    let tokens: Vec<String> = js_trim(model)
        .to_lowercase()
        .split('-')
        .map(String::from)
        .collect();
    for index in (0..tokens.len()).rev() {
        let token = tokens[index].as_str();
        if token.is_empty() {
            continue;
        }
        if token == "xhigh" {
            return Some("xhigh");
        }
        if token == "high" && index > 0 && tokens[index - 1] == "extra" {
            return Some("xhigh");
        }
        if matches!(token, "max" | "none" | "low" | "medium" | "high") {
            return Some(match token {
                "max" => "max",
                "none" => "none",
                "low" => "low",
                "medium" => "medium",
                _ => "high",
            });
        }
    }
    None
}

fn cursor_reasoning_label(value: &str) -> String {
    match value {
        "xhigh" => "Extra High".to_string(),
        "max" => "Max".to_string(),
        _ => {
            let mut chars = value.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        }
    }
}

// --- collapseCursorModelVariants ----------------------------------------------

/// `/\s+<word>$/iu` — trailing whitespace + case-insensitive word.
fn strip_ci_word_suffix(s: &mut String, word: &str) {
    let n = s.len();
    let w = word.len();
    if n > w
        && s[n - w..].eq_ignore_ascii_case(word)
        && s[..n - w].chars().next_back().is_some_and(is_js_whitespace)
    {
        s.truncate(n - w);
    }
}

/// `name.replace(/\s+Fast$/iu, "")` chain (order matters).
fn remove_variant_name_suffix(name: &str) -> String {
    let mut out = name.to_string();
    strip_ci_word_suffix(&mut out, "Fast");
    strip_ci_word_suffix(&mut out, "Thinking");
    strip_ci_word_suffix(&mut out, "Fast");
    for pattern in ["None", "Low", "Medium", "High", "Extra High"] {
        let n = out.len();
        let w = pattern.len();
        if n > w
            && out[n - w..].eq_ignore_ascii_case(pattern)
            && out[..n - w]
                .chars()
                .next_back()
                .is_some_and(is_js_whitespace)
        {
            out.truncate(n - w);
            break;
        }
    }
    // /\s+1M$/u — case-sensitive in the source.
    let n = out.len();
    if n >= 3
        && out.ends_with("1M")
        && out[..n - 2]
            .chars()
            .next_back()
            .is_some_and(is_js_whitespace)
    {
        out.truncate(n - 2);
    }
    js_trim(&out).to_string()
}

fn unique_by_value(items: Vec<Value>) -> Result<Vec<Value>, String> {
    let mut seen: Vec<Value> = Vec::new();
    let mut out = Vec::new();
    for item in items {
        let key = prop(&item, "value")?.cloned().unwrap_or_else(js_undefined);
        if seen.iter().any(|k| js_strict_eq(k, &key)) {
            continue;
        }
        seen.push(key);
        out.push(item);
    }
    Ok(out)
}

fn is_cursor_one_million_variant(model: &Value) -> Result<bool, String> {
    if prop(model, "defaultContextWindow")?
        .and_then(str_value)
        .as_deref()
        == Some("1m")
    {
        return Ok(true);
    }
    match prop(model, "contextWindowOptions")? {
        None => {}
        Some(Value::Array(options)) => {
            for option in options {
                if prop(option, "value")?.and_then(str_value).as_deref() == Some("1m")
                    && prop(option, "isDefault")? == Some(&Value::Bool(true))
                {
                    return Ok(true);
                }
            }
        }
        Some(_) => return Err("contextWindowOptions.some is not a function".to_string()),
    }
    let name = match prop(model, "name")? {
        Some(v) if !v.is_null() => js_to_string(v),
        _ => String::new(),
    };
    Ok(word_boundary_1m(&name))
}

/// `String(v)` for `\b1M\b` testing (rare non-string names).
fn js_to_string(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(items) => items.iter().map(js_to_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => {
            if let Some(units) = crate::js::string_units(v) {
                String::from_utf16_lossy(&units)
            } else if let Some(n) = crate::js::as_number(v) {
                crate::js::number_to_string(n)
            } else {
                "[object Object]".to_string()
            }
        }
    }
}

/// `/\b1M\b/` — ASCII \w boundary on both sides.
fn word_boundary_1m(s: &str) -> bool {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i + 2 <= bytes.len() {
        if &bytes[i..i + 2] == b"1M" {
            let before_ok = i == 0 || !is_word_byte(bytes[i - 1]);
            let after_ok = i + 2 == bytes.len() || !is_word_byte(bytes[i + 2]);
            if before_ok && after_ok {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn fallback_context_window_options(base_slug: &str, has_1m: bool) -> Vec<Value> {
    if !has_1m {
        return Vec::new();
    }
    let (value, label) = match base_slug {
        "gpt-5.5" | "gpt-5.4" => ("272k", "272K"),
        "claude-fable-5-1" | "claude-fable-5" | "claude-sonnet-5" | "claude-opus-4-8"
        | "claude-opus-4-7" => ("300k", "300K"),
        "claude-opus-4-6" | "claude-sonnet-4-6" => ("200k", "200K"),
        _ => return Vec::new(),
    };
    vec![
        json!({"value": value, "label": label, "isDefault": true}),
        json!({"value": "1m", "label": "1M"}),
    ]
}

fn default_effort_for_group(base_slug: &str, efforts: &[String]) -> Option<String> {
    if efforts.is_empty() {
        return None;
    }
    if base_slug.contains("gpt") || base_slug.contains("codex") {
        return Some(
            if efforts.iter().any(|e| e == "medium") {
                "medium"
            } else {
                efforts[0].as_str()
            }
            .to_string(),
        );
    }
    if efforts.iter().any(|e| e == "high") {
        return Some("high".to_string());
    }
    Some(efforts[0].clone())
}

/// `variant.supportedReasoningEfforts ?? []` spread — non-array throws.
fn spread_array(v: Option<&Value>) -> Result<Vec<Value>, String> {
    match v {
        None => Ok(Vec::new()),
        Some(Value::Array(a)) => Ok(a.clone()),
        Some(_) => Err("value is not iterable".to_string()),
    }
}

/// Port of `collapseCursorModelVariants`.
pub fn collapse_cursor_model_variants(
    models: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let Value::Array(models) = &models else {
        return Err("models is not iterable".to_string());
    };
    let mut groups: Vec<(Value, Vec<Value>)> = Vec::new();
    for model in models {
        let slug = prop(model, "slug")?;
        let base_slug = match normalize_inner(slug)? {
            Some(b) => Value::String(b),
            None => slug.cloned().unwrap_or_else(js_undefined),
        };
        if let Some((_, group)) = groups.iter_mut().find(|(k, _)| js_strict_eq(k, &base_slug)) {
            group.push(model.clone());
        } else {
            groups.push((base_slug, vec![model.clone()]));
        }
    }

    let mut output = Vec::new();
    for (base_slug, variants) in groups {
        let mut preferred_name: Option<Value> = None;
        for variant in &variants {
            let slug = prop(variant, "slug")?.cloned().unwrap_or_else(js_undefined);
            if js_strict_eq(&slug, &base_slug) {
                preferred_name = prop(variant, "name")?.cloned();
                break;
            }
        }
        if preferred_name.is_none() {
            for variant in &variants {
                if !js_ends_with(prop(variant, "slug")?, "-fast")? {
                    preferred_name = prop(variant, "name")?.cloned();
                    break;
                }
            }
        }
        if preferred_name.is_none() {
            preferred_name = prop(&variants[0], "name")?.cloned();
        }
        let name_source = preferred_name.unwrap_or_else(|| base_slug.clone());
        let Some(name_str) = str_value(&name_source) else {
            return Err("name.replace is not a function".to_string());
        };
        let name = remove_variant_name_suffix(&name_str);

        let mut raw_efforts: Vec<Value> = Vec::new();
        for variant in &variants {
            raw_efforts.extend(spread_array(prop(variant, "supportedReasoningEfforts")?)?);
            let slug = prop(variant, "slug")?;
            if let Some(s) = slug.and_then(str_value)
                && let Some(effort) = parse_cli_reasoning_effort(&s)
            {
                raw_efforts.push(json!({"value": effort, "label": cursor_reasoning_label(effort)}));
            }
        }
        let efforts = unique_by_value(raw_efforts)?;
        let effort_values: Vec<String> = efforts
            .iter()
            .filter_map(|e| prop(e, "value").ok().flatten().and_then(str_value))
            .collect();

        let mut default_effort: Option<Value> = None;
        for variant in &variants {
            let slug = prop(variant, "slug")?.cloned().unwrap_or_else(js_undefined);
            let norm = normalize_inner(prop(variant, "slug")?)?
                .map(Value::String)
                .unwrap_or(Value::Null);
            if js_strict_eq(&norm, &slug) {
                default_effort = prop(variant, "defaultReasoningEffort")?.cloned();
                break;
            }
        }
        if default_effort.is_none() {
            if !effort_values.is_empty() && str_value(&base_slug).is_none() {
                return Err("baseSlug.includes is not a function".to_string());
            }
            default_effort = default_effort_for_group(
                &str_value(&base_slug).unwrap_or_default(),
                &effort_values,
            )
            .map(Value::String);
        }

        let mut has_1m = false;
        for variant in &variants {
            if is_cursor_one_million_variant(variant)? {
                has_1m = true;
                break;
            }
        }
        let mut all_ctx =
            fallback_context_window_options(&str_value(&base_slug).unwrap_or_default(), has_1m);
        for variant in &variants {
            all_ctx.extend(spread_array(prop(variant, "contextWindowOptions")?)?);
        }
        let context_window_options = unique_by_value(all_ctx)?;

        let mut out = serde_json::Map::new();
        out.insert("slug".into(), base_slug.clone());
        out.insert("name".into(), json!(name));
        if let Some(v) = prop(&variants[0], "upstreamProviderId")?
            && !is_falsy(v)
        {
            out.insert("upstreamProviderId".into(), v.clone());
        }
        if let Some(v) = prop(&variants[0], "upstreamProviderName")?
            && !is_falsy(v)
        {
            out.insert("upstreamProviderName".into(), v.clone());
        }
        if !efforts.is_empty() {
            let mut list = Vec::new();
            for effort in &efforts {
                let mut e = serde_json::Map::new();
                if let Some(v) = prop(effort, "value")? {
                    e.insert("value".into(), v.clone());
                }
                if let Some(l) = prop(effort, "label")? {
                    e.insert("label".into(), l.clone());
                }
                if let (Some(v), Some(d)) = (prop(effort, "value")?, default_effort.as_ref())
                    && js_strict_eq(v, d)
                {
                    e.insert("isDefault".into(), Value::Bool(true));
                }
                list.push(Value::Object(e));
            }
            out.insert("supportedReasoningEfforts".into(), Value::Array(list));
            if let Some(d) = default_effort {
                out.insert("defaultReasoningEffort".into(), d);
            }
        }
        let mut fast = false;
        let mut thinking = false;
        for variant in &variants {
            if prop(variant, "supportsFastMode")? == Some(&Value::Bool(true)) {
                fast = true;
            }
            if prop(variant, "supportsThinkingToggle")? == Some(&Value::Bool(true)) {
                thinking = true;
            }
        }
        if fast {
            out.insert("supportsFastMode".into(), Value::Bool(true));
        }
        if thinking {
            out.insert("supportsThinkingToggle".into(), Value::Bool(true));
        }
        if !context_window_options.is_empty() {
            let default_ctx = context_window_options
                .iter()
                .find(|o| prop(o, "isDefault").ok().flatten() == Some(&Value::Bool(true)))
                .and_then(|o| prop(o, "value").ok().flatten().cloned())
                .or_else(|| {
                    prop(&context_window_options[0], "value")
                        .ok()
                        .flatten()
                        .cloned()
                });
            out.insert(
                "contextWindowOptions".into(),
                Value::Array(context_window_options),
            );
            if let Some(d) = default_ctx {
                out.insert("defaultContextWindow".into(), d);
            }
        }
        output.push(Value::Object(out));
    }
    Ok(Value::Array(output))
}
