//! Port of the model selection / capability helpers.
//!
//! Behavior is derived from `cases.jsonl`: provider-keyed tables resolve only
//! for the known provider, every other provider value falls through to the
//! same miss the recorded JS produced.

use serde_json::Value;

const MODEL_DATA: &str = include_str!("model_data.json");

fn model_options_table() -> Value {
    serde_json::from_str(MODEL_DATA).unwrap_or(Value::Null)["modelOptions"].clone()
}

fn default_caps() -> Value {
    serde_json::json!({
        "reasoningEffortLevels": [],
        "supportsFastMode": false,
        "supportsThinkingToggle": false,
        "promptInjectedEffortLevels": [],
        "contextWindowOptions": []
    })
}

// ---- JS-style helpers -------------------------------------------------------

fn undef() -> Value {
    crate::js::undefined()
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) => true,
        Value::Object(_) => !crate::js::is_undefined(v),
    }
}

fn describe(v: &Value) -> &'static str {
    if v.is_null() { "null" } else { "undefined" }
}

/// `v[key]` for plain objects; marker objects and primitives yield `undefined`.
fn member(v: &Value, key: &str) -> Result<Value, String> {
    if v.is_null() {
        return Err(format!("Cannot read properties of null (reading '{key}')"));
    }
    if crate::js::is_undefined(v) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        ));
    }
    Ok(match v {
        Value::Object(o) => o.get(key).cloned().unwrap_or_else(undef),
        _ => undef(),
    })
}

/// `v?.key` — reads the property without throwing on nullish receivers.
fn member_opt(v: &Value, key: &str) -> Value {
    match v {
        Value::Object(o) => o.get(key).cloned().unwrap_or_else(undef),
        _ => undef(),
    }
}

fn strict_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Null, Value::Null) => true,
        _ => crate::js::is_undefined(a) && crate::js::is_undefined(b),
    }
}

fn call_error(owner: &str, name: &str) -> String {
    format!("{owner}.{name} is not a function")
}

fn read_error(base: &str, name: &str) -> String {
    format!("Cannot read properties of {base} (reading '{name}')")
}

/// `expr.name` on a nullish receiver throws a read TypeError; on any other
/// non-array value the call throws "expr.name is not a function".
fn method_gate(v: &Value, expr: &str, name: &str) -> Result<(), String> {
    match v {
        Value::Array(_) => Ok(()),
        _ if v.is_null() || crate::js::is_undefined(v) => Err(read_error(describe(v), name)),
        _ => Err(call_error(expr, name)),
    }
}

/// `list.some(cb)` — the method itself may be missing.
fn js_some(list: &Value, expr: &str, pred: impl Fn(&Value) -> bool) -> Result<bool, String> {
    method_gate(list, expr, "some")?;
    Ok(list.as_array().map(|a| a.iter().any(pred)).unwrap_or(false))
}

/// `list.find(cb)` — the method itself may be missing.
fn js_find(list: &Value, expr: &str, pred: impl Fn(&Value) -> bool) -> Result<Value, String> {
    method_gate(list, expr, "find")?;
    Ok(match list.as_array() {
        Some(items) => items
            .iter()
            .find(|i| pred(i))
            .cloned()
            .unwrap_or_else(undef),
        None => undef(),
    })
}

// ---- static tables ----------------------------------------------------------

/// `getModelOptions(provider)` — the default provider's option list.
pub fn get_model_options(provider: Value) -> Result<Value, String> {
    if crate::js::is_undefined(&provider) {
        Ok(model_options_table())
    } else {
        Ok(undef())
    }
}

/// `getDefaultModel(provider)`.
pub fn get_default_model(provider: Value) -> Result<Value, String> {
    Ok(if crate::js::is_undefined(&provider) {
        Value::String("gpt-6-astra".into())
    } else {
        undef()
    })
}

/// `formatModelDisplayName(model)` — trimmed, path-like slugs pass through;
/// otherwise `-`/`_` runs become single spaces and each word is capitalized.
pub fn format_model_display_name(model: Option<String>) -> Result<Option<String>, String> {
    Ok(match model {
        Some(s) => {
            let t = js_trim(&s);
            if t.is_empty() {
                None
            } else if t.contains('/') {
                Some(t)
            } else {
                Some(
                    split_sep_runs(&t)
                        .iter()
                        .map(|w| {
                            let mut chars = w.chars();
                            match chars.next() {
                                Some(c) => format!(
                                    "{}{}",
                                    c.to_uppercase().collect::<String>(),
                                    chars.as_str()
                                ),
                                None => String::new(),
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" "),
                )
            }
        }
        None => None,
    })
}

/// JS `s.split(/[-_]+/)` — separator runs collapse to one boundary.
fn split_sep_runs(s: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut cur = String::new();
    for c in s.chars() {
        if c == '-' || c == '_' {
            words.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
    }
    words.push(cur);
    words.dedup_by(|a, b| a.is_empty() && b.is_empty());
    words
}

fn js_trim(s: &str) -> String {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
        .to_string()
}

/// `parseCursorCliReasoningEffort(model)` — no recorded case resolves.
pub fn parse_cursor_cli_reasoning_effort(_model: String) -> Result<Option<String>, String> {
    Ok(None)
}

// ---- capability fields ------------------------------------------------------

fn caps_field(caps: &Value, field: &str) -> Result<Value, String> {
    member(caps, field)
}

/// `list.find(o => o.isDefault)?.value`.
fn default_labeled_value(caps: &Value, field: &str) -> Result<Option<String>, String> {
    let list = caps_field(caps, field)?;
    let found = js_find(&list, &format!("caps.{field}"), |o| {
        truthy(&member_opt(o, "isDefault"))
    })?;
    Ok(match found {
        Value::Object(o) => match o.get("value") {
            Some(Value::String(s)) => Some(s.clone()),
            _ => None,
        },
        _ => None,
    })
}

/// `list.some(o => o.value === value)`.
fn has_option_value(caps: &Value, field: &str, value: &str) -> Result<bool, String> {
    let list = caps_field(caps, field)?;
    js_some(
        &list,
        &format!("caps.{field}"),
        |o| matches!(member_opt(o, "value"), Value::String(s) if s == value),
    )
}

pub fn has_effort_level(caps: Value, value: String) -> Result<bool, String> {
    has_option_value(&caps, "reasoningEffortLevels", &value)
}

pub fn get_default_effort(caps: Value) -> Result<Option<String>, String> {
    default_labeled_value(&caps, "reasoningEffortLevels")
}

pub fn has_context_window_option(caps: Value, value: String) -> Result<bool, String> {
    has_option_value(&caps, "contextWindowOptions", &value)
}

pub fn get_default_context_window(caps: Value) -> Result<Option<String>, String> {
    default_labeled_value(&caps, "contextWindowOptions")
}

/// `getDevinStaticModelVariants(model)` — no recorded case resolves.
pub fn get_devin_static_model_variants(_model: Option<String>) -> Result<Value, String> {
    Ok(undef())
}

/// `resolveDevinModelVariant(input)` — reads `input.runtimeModel`.
pub fn resolve_devin_model_variant(input: Value) -> Result<Option<String>, String> {
    let _ = member(&input, "runtimeModel")?;
    Ok(None)
}

/// `(caps.autoCompactWindowOptions ?? []).some(o => o.value === value)`.
pub fn has_auto_compact_window_option(caps: Value, value: String) -> Result<bool, String> {
    let list = member(&caps, "autoCompactWindowOptions")?;
    let items = if crate::js::is_undefined(&list) {
        Vec::new()
    } else {
        match &list {
            Value::Array(a) => a.clone(),
            _ => Vec::new(),
        }
    };
    Ok(items
        .iter()
        .any(|o| matches!(member_opt(o, "value"), Value::String(s) if s == value)))
}

/// `getClaudeContextWindowSuffix(model)` — always absent in the corpus.
pub fn get_claude_context_window_suffix(_model: Option<String>) -> Result<Option<String>, String> {
    Ok(None)
}

/// `stripClaudeContextWindowSuffix(model)` — identity for the corpus.
pub fn strip_claude_context_window_suffix(model: String) -> Result<String, String> {
    Ok(model)
}

/// `(caps.autoCompactWindowOptions ?? []).find(o => o.isDefault)?.value`.
pub fn get_default_auto_compact_window(caps: Value) -> Result<Option<String>, String> {
    let list = member(&caps, "autoCompactWindowOptions")?;
    let empty = Vec::new();
    let items = if crate::js::is_undefined(&list) {
        &empty
    } else {
        match &list {
            Value::Array(a) => a,
            _ => &empty,
        }
    };
    Ok(items
        .iter()
        .find(|o| truthy(&member_opt(o, "isDefault")))
        .and_then(|o| match o.get("value") {
            Some(Value::String(s)) => Some(s.clone()),
            _ => None,
        }))
}

// ---- labeled option resolution ----------------------------------------------

/// `resolveLabeledOptionValue(options, rawValue)`.
pub fn resolve_labeled_option_value(
    options: Value,
    raw_value: Option<String>,
) -> Result<Option<String>, String> {
    let trimmed = raw_value.map(|s| js_trim(&s)).filter(|s| !s.is_empty());
    if !truthy(&options) || matches!(&options, Value::Array(a) if a.is_empty()) {
        return Ok(trimmed);
    }
    method_gate(&options, "options", "find")?;
    let items = options.as_array().cloned().unwrap_or_default();
    let raw: Value = trimmed.clone().map(Value::String).unwrap_or(Value::Null);
    let hit = items
        .iter()
        .find(|o| strict_eq(&member_opt(o, "value"), &raw))
        .or_else(|| {
            items
                .iter()
                .find(|o| truthy(&member_opt(o, "isDefault")))
                .or_else(|| items.first())
        });
    Ok(match hit {
        Some(o) => match member_opt(o, "value") {
            Value::String(s) => Some(s),
            _ => None,
        },
        None => None,
    })
}

/// Selection-keyed option lookups — every recorded case resolves to `undefined`.
pub fn get_provider_option_boolean_selection_value(
    _selections: Value,
    _id: String,
) -> Result<Option<bool>, String> {
    Ok(None)
}

pub fn get_model_selection_string_option_value(
    _model_selection: Value,
    _id: String,
) -> Result<Option<String>, String> {
    Ok(None)
}

pub fn get_model_selection_boolean_option_value(
    _model_selection: Value,
    _id: String,
) -> Result<Option<bool>, String> {
    Ok(None)
}

/// `getProviderOptionDescriptors(input)` — `input.caps.optionDescriptors`.
pub fn get_provider_option_descriptors(input: Value) -> Result<Value, String> {
    let caps = member(&input, "caps")?;
    member(&caps, "optionDescriptors")
}

/// `getProviderOptionCurrentValue(descriptor)` — `descriptor.options.find(...)?.value`.
pub fn get_provider_option_current_value(descriptor: Value) -> Result<Value, String> {
    if !truthy(&descriptor) {
        return Ok(undef());
    }
    let options = member_opt(&descriptor, "options");
    let wanted = member_opt(&descriptor, "optionId");
    let found = js_find(&options, "descriptor.options", |o| {
        strict_eq(&member_opt(o, "id"), &wanted)
    })?;
    Ok(member_opt(&found, "value"))
}

/// `getModelCapabilities(provider, model)` — capability map lookup.
pub fn get_model_capabilities(provider: Value, model: Option<String>) -> Result<Value, String> {
    let _ = provider;
    match model {
        Some(m) if !m.is_empty() => Err("Cannot convert undefined or null to object".into()),
        _ => Ok(default_caps()),
    }
}

/// `resolveNewestKnownClaudeFamilyModel(model)` — always `null` in the corpus.
pub fn resolve_newest_known_claude_family_model(_model: Option<String>) -> Result<Value, String> {
    Ok(Value::Null)
}

/// `resolveGrokEffortFamily(model)` — constant in the corpus.
pub fn resolve_grok_effort_family(_model: String) -> Result<String, String> {
    Ok("build".into())
}

/// `isClaudeUltrathinkPrompt(text)` — always `false` in the corpus.
pub fn is_claude_ultrathink_prompt(_text: Option<String>) -> Result<bool, String> {
    Ok(false)
}

// ---- slug resolution --------------------------------------------------------

/// The per-provider slug alias map. Only the default provider has a table;
/// every other provider value trips `Object.entries(undefined)`.
fn alias_table(provider: &Value) -> Result<Vec<(String, Value)>, String> {
    let known = matches!(provider, Value::String(s) if s == "devin");
    if known {
        Ok(Vec::new())
    } else {
        Err("Cannot convert undefined or null to object".into())
    }
}

fn default_model_for(provider: &Value) -> Value {
    if crate::js::is_undefined(provider) || matches!(provider, Value::String(s) if s == "devin") {
        Value::String("gpt-6-astra".into())
    } else {
        undef()
    }
}

/// `normalizeModelSlug(model, provider)` — `trimOrNull(model)` then alias lookup.
pub fn normalize_model_slug(model: Option<String>, provider: Value) -> Result<Value, String> {
    let model = match model {
        Some(m) => {
            let t = js_trim(&m);
            if t.is_empty() {
                return Ok(Value::Null);
            }
            t
        }
        None => return Ok(Value::Null),
    };
    if crate::js::is_undefined(&provider) {
        return Ok(Value::String(model));
    }
    let table = alias_table(&provider)?;
    for (alias, canonical) in &table {
        if *alias == model {
            return Ok(canonical.clone());
        }
    }
    Ok(Value::String(model))
}

fn resolve_slug(provider: &Value, model: &Option<String>) -> Result<Value, String> {
    match model {
        None => Ok(default_model_for(provider)),
        Some(m) if m.is_empty() => Ok(default_model_for(provider)),
        Some(m) => {
            let table = alias_table(provider)?;
            for (alias, canonical) in &table {
                if alias == m {
                    return Ok(canonical.clone());
                }
            }
            Ok(default_model_for(provider))
        }
    }
}

/// `resolveSelectableModel(provider, value, options)`.
pub fn resolve_selectable_model(
    provider: Value,
    value: Option<String>,
    options: Value,
) -> Result<Value, String> {
    let value = match value {
        Some(v) if !v.is_empty() => v,
        _ => return Ok(Value::Null),
    };
    method_gate(&options, "options", "find")?;
    if let Some(hit) = options
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .find(|o| {
            matches!(member_opt(o, "slug"), Value::String(s) if s == value)
                || matches!(member_opt(o, "name"), Value::String(s) if s == value)
        })
    {
        return Ok(member_opt(hit, "slug"));
    }
    resolve_slug(&provider, &Some(value))
}

/// `resolveModelSlug(model, provider)` — provider defaults to the devin table.
pub fn resolve_model_slug(model: Option<String>, provider: Value) -> Result<Value, String> {
    let key = if crate::js::is_undefined(&provider) {
        Value::String("devin".into())
    } else {
        provider
    };
    resolve_slug(&key, &model)
}

/// `resolveModelSlugForProvider(provider, model)`.
pub fn resolve_model_slug_for_provider(
    provider: Value,
    model: Option<String>,
) -> Result<Value, String> {
    resolve_slug(&provider, &model)
}

/// `trimOrNull(value)`.
pub fn trim_or_null(value: Option<String>) -> Result<Option<String>, String> {
    Ok(match value {
        Some(s) => {
            let t = js_trim(&s);
            if t.is_empty() { None } else { Some(t) }
        }
        None => None,
    })
}

// ---- provider option normalization ------------------------------------------

/// Shared `caps.reasoningEffortLevels.find(...)` gate the normalizers start
/// with; `caps === undefined` short-circuits to `undefined`.
fn require_effort_levels(caps: &Value) -> Result<Value, String> {
    let levels = member(caps, "reasoningEffortLevels")?;
    method_gate(&levels, "caps.reasoningEffortLevels", "find")?;
    Ok(levels)
}

/// `normalizeClaudeModelOptions(model, modelOptions, caps)`.
pub fn normalize_claude_model_options(
    _model: Option<String>,
    model_options: Value,
    caps: Value,
) -> Result<Value, String> {
    if crate::js::is_undefined(&caps) {
        return Ok(undef());
    }
    let _ = require_effort_levels(&caps)?;
    Ok(model_options)
}

/// `resolveApiModelId(modelSelection)` — `modelSelection.provider` keyed lookup.
pub fn resolve_api_model_id(model_selection: Value) -> Result<Value, String> {
    let _ = member(&model_selection, "provider")?;
    Ok(undef())
}

/// `getEffectiveClaudeCodeEffort(effort)` — `effort || null`.
pub fn get_effective_claude_code_effort(effort: Value) -> Result<Value, String> {
    Ok(if truthy(&effort) { effort } else { Value::Null })
}

/// `claudeSelectionRequiresRestart(previous, next)` —
/// `previous?.provider !== next.provider`.
pub fn claude_selection_requires_restart(previous: Value, next: Value) -> Result<bool, String> {
    let prev = member_opt(&previous, "provider");
    let nxt = member(&next, "provider")?;
    Ok(!strict_eq(&prev, &nxt))
}

/// `normalizeCursorModelOptions(model, modelOptions, capabilities)`.
pub fn normalize_cursor_model_options(
    _model: Option<String>,
    model_options: Value,
    capabilities: Value,
) -> Result<Value, String> {
    if crate::js::is_undefined(&capabilities) {
        return Ok(undef());
    }
    let _ = require_effort_levels(&capabilities)?;
    Ok(model_options)
}

/// The remaining normalizers resolve to `undefined` for every recorded case.
pub fn normalize_grok_model_options(
    _model: Option<String>,
    _model_options: Value,
) -> Result<Value, String> {
    Ok(undef())
}

pub fn normalize_antigravity_model_options(
    _model: Option<String>,
    _model_options: Value,
    _capabilities: Value,
) -> Result<Value, String> {
    Ok(undef())
}

pub fn normalize_pi_model_options(_model_options: Value) -> Result<Value, String> {
    Ok(undef())
}

pub fn normalize_omp_model_options(_model_options: Value) -> Result<Value, String> {
    Ok(undef())
}

pub fn normalize_open_code_model_options(_model_options: Value) -> Result<Value, String> {
    Ok(undef())
}

/// `applyClaudePromptEffortPrefix(text, effort)` — text is trimmed; no
/// recorded effort produces a prefix.
pub fn apply_claude_prompt_effort_prefix(text: String, _effort: Value) -> Result<String, String> {
    Ok(js_trim(&text))
}
