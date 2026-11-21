//! The Rust port of `packages/shared/src/model.ts`.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::{Map, Value, json};
use std::sync::OnceLock;

// ── Contract data (dumped from `packages/contracts/src/model.ts`) ────────────

fn data() -> &'static Value {
    static DATA: OnceLock<Value> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("model_data.json")).unwrap_or(Value::Null)
    })
}

fn table(name: &str) -> &'static Map<String, Value> {
    static EMPTY: OnceLock<Map<String, Value>> = OnceLock::new();
    data()
        .get(name)
        .and_then(Value::as_object)
        .unwrap_or_else(|| EMPTY.get_or_init(Map::new))
}

fn str_set(name: &str) -> &'static Vec<Value> {
    static EMPTY: Vec<Value> = Vec::new();
    data().get(name).and_then(Value::as_array).unwrap_or(&EMPTY)
}

fn provider_key(provider: &Value) -> &str {
    provider.as_str().unwrap_or("")
}

fn member(v: &Value, key: &str) -> Value {
    v.as_object()
        .and_then(|o| o.get(key))
        .cloned()
        .unwrap_or_else(js::undefined)
}

fn null_err(key: &str) -> String {
    format!("Cannot read properties of null (reading '{key}')")
}

fn undef_err(key: &str) -> String {
    format!("Cannot read properties of undefined (reading '{key}')")
}

fn is_undef(v: &Value) -> bool {
    js::is_undefined(v)
}

/// JS `String.prototype.trim`: Unicode whitespace plus U+FEFF.
fn jtrim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) => true,
        Value::Object(_) => !is_undef(v),
    }
}

fn js_string(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(_) => js::number_to_string(js::as_number(v).unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(a) => a.iter().map(js_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => {
            if is_undef(v) {
                "undefined".into()
            } else {
                "[object Object]".into()
            }
        }
    }
}

fn trim_or_null_str(value: Option<&Value>) -> Option<String> {
    let s = value?.as_str()?;
    let trimmed = jtrim(s);
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

// ── Simple exports ───────────────────────────────────────────────────────────

/// Port of `getModelOptions`.
pub fn get_model_options(provider: serde_json::Value) -> Result<serde_json::Value, String> {
    let key = if is_undef(&provider) {
        "codex"
    } else {
        provider_key(&provider)
    };
    Ok(table("MODEL_OPTIONS_BY_PROVIDER")
        .get(key)
        .cloned()
        .unwrap_or_else(js::undefined))
}

/// Port of `getDefaultModel`.
pub fn get_default_model(provider: serde_json::Value) -> Result<serde_json::Value, String> {
    let key = if is_undef(&provider) {
        "codex"
    } else {
        provider_key(&provider)
    };
    if key == "pi" || key == "omp" {
        return Ok(Value::Null);
    }
    Ok(table("DEFAULT_MODEL_BY_PROVIDER")
        .get(key)
        .cloned()
        .unwrap_or_else(js::undefined))
}

/// Port of `formatModelDisplayName`.
pub fn format_model_display_name(model: Option<String>) -> Result<Option<String>, String> {
    let Some(normalized) = model.as_deref().and_then(|m| {
        let t = jtrim(m);
        if t.is_empty() {
            None
        } else {
            Some(t.to_string())
        }
    }) else {
        return Ok(None);
    };
    if let Some(name) = model_name_by_slug()
        .get(&normalized.to_lowercase())
        .and_then(Value::as_str)
    {
        return Ok(Some(name.to_string()));
    }
    Ok(Some(humanize_model_slug(&normalized)))
}

fn model_name_by_slug() -> &'static Map<String, Value> {
    static NAMES: OnceLock<Map<String, Value>> = OnceLock::new();
    NAMES.get_or_init(|| {
        let mut map = Map::new();
        for (_provider, models) in table("MODEL_OPTIONS_BY_PROVIDER") {
            if let Some(list) = models.as_array() {
                for option in list {
                    if let (Some(slug), Some(name)) = (
                        option.get("slug").and_then(Value::as_str),
                        option.get("name").and_then(Value::as_str),
                    ) {
                        map.insert(slug.to_lowercase(), json!(name));
                    }
                }
            }
        }
        map
    })
}

/// Port of `parseCursorCliReasoningEffort`.
pub fn parse_cursor_cli_reasoning_effort(model: String) -> Result<Option<String>, String> {
    let lowered = jtrim(&model).to_lowercase();
    let tokens: Vec<&str> = lowered.split('-').collect();
    for index in (0..tokens.len()).rev() {
        let token = tokens[index];
        if token.is_empty() {
            continue;
        }
        if token == "xhigh" {
            return Ok(Some("xhigh".into()));
        }
        if token == "high" && index > 0 && tokens[index - 1] == "extra" {
            return Ok(Some("xhigh".into()));
        }
        if matches!(token, "max" | "none" | "low" | "medium" | "high") {
            return Ok(Some(token.into()));
        }
    }
    Ok(None)
}

/// Port of `hasEffortLevel`.
pub fn has_effort_level(caps: serde_json::Value, value: String) -> Result<bool, String> {
    if caps.is_null() {
        return Err(null_err("reasoningEffortLevels"));
    }
    if is_undef(&caps) {
        return Err(undef_err("reasoningEffortLevels"));
    }
    match member(&caps, "reasoningEffortLevels") {
        Value::Array(list) => Ok(list
            .iter()
            .any(|l| l.get("value").and_then(Value::as_str) == Some(value.as_str()))),
        v if is_undef(&v) => Err(undef_err("find")),
        _ => Err("caps.reasoningEffortLevels.some is not a function".into()),
    }
}

/// Port of `getDefaultEffort`.
pub fn get_default_effort(caps: serde_json::Value) -> Result<Option<String>, String> {
    caps_find_default(&caps, "reasoningEffortLevels")
}

/// Port of `hasContextWindowOption`.
pub fn has_context_window_option(caps: serde_json::Value, value: String) -> Result<bool, String> {
    if caps.is_null() {
        return Err(null_err("contextWindowOptions"));
    }
    if is_undef(&caps) {
        return Err(undef_err("contextWindowOptions"));
    }
    match member(&caps, "contextWindowOptions") {
        Value::Array(list) => Ok(list
            .iter()
            .any(|o| o.get("value").and_then(Value::as_str) == Some(value.as_str()))),
        v if is_undef(&v) => Err(undef_err("find")),
        _ => Err("caps.contextWindowOptions.some is not a function".into()),
    }
}

/// Port of `getDefaultContextWindow`.
pub fn get_default_context_window(caps: serde_json::Value) -> Result<Option<String>, String> {
    if caps.is_null() {
        return Err(null_err("contextWindowOptions"));
    }
    if is_undef(&caps) {
        return Err(undef_err("contextWindowOptions"));
    }
    match member(&caps, "contextWindowOptions") {
        Value::Array(list) => Ok(list
            .iter()
            .find(|o| truthy(o.get("isDefault").unwrap_or(&Value::Null)))
            .and_then(|o| o.get("value").and_then(Value::as_str).map(String::from))),
        v if is_undef(&v) => Err(undef_err("find")),
        _ => Err("caps.contextWindowOptions.find is not a function".into()),
    }
}

// ── Claude context-window suffix ─────────────────────────────────────────────

fn claude_suffix(model: &str) -> Option<String> {
    // /\[([^\]]+)\]$/u
    if !model.ends_with(']') {
        return None;
    }
    let open = model[..model.len() - 1].rfind('[')?;
    Some(model[open + 1..model.len() - 1].to_lowercase())
}

/// Port of `getClaudeContextWindowSuffix`.
pub fn get_claude_context_window_suffix(model: Option<String>) -> Result<Option<String>, String> {
    Ok(model.and_then(|m| claude_suffix(&m)))
}

/// Port of `stripClaudeContextWindowSuffix`.
pub fn strip_claude_context_window_suffix(model: String) -> Result<String, String> {
    if !model.ends_with(']') {
        return Ok(model);
    }
    if let Some(open) = model[..model.len() - 1].rfind('[') {
        return Ok(model[..open].to_string());
    }
    Ok(model)
}

// ── Devin variants ───────────────────────────────────────────────────────────

fn devin_static_variants(model: &str) -> Value {
    match model {
        "swe-1-6" => json!([
            {"model": "swe-1-6", "fastMode": false},
            {"model": "swe-1-6-fast", "fastMode": true}
        ]),
        "swe-1-7" => json!([
            {"model": "swe-1-7", "fastMode": false},
            {"model": "swe-1-7-lightning", "fastMode": true}
        ]),
        _ => js::undefined(),
    }
}

/// Port of `getDevinStaticModelVariants`.
pub fn get_devin_static_model_variants(model: Option<String>) -> Result<serde_json::Value, String> {
    let normalized = normalize_slug_internal(model.as_deref(), &json!("devin"))?;
    Ok(match normalized {
        Some(Value::String(slug)) => devin_static_variants(&slug),
        _ => js::undefined(),
    })
}

/// Port of `resolveDevinModelVariant`.
pub fn resolve_devin_model_variant(input: serde_json::Value) -> Result<Option<String>, String> {
    if input.is_null() {
        return Err(null_err("runtimeModel"));
    }
    if is_undef(&input) {
        return Err(undef_err("runtimeModel"));
    }
    let runtime_model = member(&input, "runtimeModel");
    let variants = match member(&runtime_model, "modelVariants") {
        Value::Array(list) if !list.is_empty() => Value::Array(list),
        _ => {
            let normalized =
                normalize_slug_internal(member(&input, "model").as_str(), &json!("devin"))?;
            match normalized {
                Some(Value::String(slug)) => devin_static_variants(&slug),
                _ => js::undefined(),
            }
        }
    };
    let explicit_variant = trim_or_null_str(Some(&member(&input, "modelVariant")));
    let list = match &variants {
        Value::Array(list) if !list.is_empty() => list.clone(),
        Value::Array(_) => return Ok(explicit_variant),
        v if is_undef(v) || v.is_null() => return Ok(explicit_variant),
        _ => return Err("variants.some is not a function".into()),
    };
    let reasoning_effort = trim_or_null_str(Some(&member(&input, "reasoningEffort")));
    let context_window = trim_or_null_str(Some(&member(&input, "contextWindow")));
    let fast_mode = member(&input, "fastMode");
    let thinking = member(&input, "thinking");
    let maps_effort = reasoning_effort.is_some()
        && list
            .iter()
            .any(|v| !is_undef(&member(v, "reasoningEffort")));
    let maps_fast = !is_undef(&fast_mode) && list.iter().any(|v| !is_undef(&member(v, "fastMode")));
    let maps_thinking = !thinking.is_null()
        && !is_undef(&thinking)
        && list.iter().any(|v| !is_undef(&member(v, "thinking")));
    let maps_window =
        context_window.is_some() && list.iter().any(|v| !is_undef(&member(v, "contextWindow")));
    if !maps_effort && !maps_fast && !maps_thinking && !maps_window {
        return Ok(explicit_variant);
    }
    let effective_effort = reasoning_effort
        .or_else(|| trim_or_null_str(Some(&member(&runtime_model, "defaultReasoningEffort"))));
    let effective_window = context_window
        .clone()
        .or_else(|| trim_or_null_str(Some(&member(&runtime_model, "defaultContextWindow"))));
    let effective_thinking = if !thinking.is_null() && !is_undef(&thinking) {
        thinking.clone()
    } else if member(&runtime_model, "supportsThinkingToggle") == Value::Bool(true) {
        Value::Bool(true)
    } else {
        js::undefined()
    };
    let fast_true = fast_mode == Value::Bool(true);
    let preferred: Vec<&Value> = list
        .iter()
        .filter(|variant| {
            if let Some(e) = &effective_effort
                && member(variant, "reasoningEffort").as_str() != Some(e.as_str())
            {
                return false;
            }
            if let Some(w) = &effective_window
                && member(variant, "contextWindow").as_str() != Some(w.as_str())
            {
                return false;
            }
            if fast_true && member(variant, "fastMode") != Value::Bool(true) {
                return false;
            }
            if !fast_true && member(variant, "fastMode") == Value::Bool(true) {
                return false;
            }
            if !effective_thinking.is_null()
                && !is_undef(&effective_thinking)
                && !is_undef(&member(variant, "thinking"))
            {
                return member(variant, "thinking") == effective_thinking;
            }
            true
        })
        .collect();
    let with_default: Vec<&Value> = if context_window.is_none() && effective_window.is_some() {
        let w = effective_window.as_deref().unwrap_or_default();
        preferred
            .iter()
            .copied()
            .filter(|v| member(v, "contextWindow").as_str() == Some(w))
            .collect()
    } else {
        preferred.clone()
    };
    Ok(with_default
        .first()
        .or_else(|| preferred.first())
        .and_then(|v| v.get("model").and_then(Value::as_str).map(String::from)))
}

// ── Auto-compact windows ─────────────────────────────────────────────────────

/// Port of `hasAutoCompactWindowOption`.
pub fn has_auto_compact_window_option(
    caps: serde_json::Value,
    value: String,
) -> Result<bool, String> {
    if caps.is_null() {
        return Err(null_err("autoCompactWindowOptions"));
    }
    if is_undef(&caps) {
        return Err(undef_err("autoCompactWindowOptions"));
    }
    match member(&caps, "autoCompactWindowOptions") {
        Value::Array(list) => Ok(list
            .iter()
            .any(|o| o.get("value").and_then(Value::as_str) == Some(value.as_str()))),
        v if is_undef(&v) || v.is_null() => Ok(false),
        _ => Err("caps.autoCompactWindowOptions.some is not a function".into()),
    }
}

/// Port of `getDefaultAutoCompactWindow`.
pub fn get_default_auto_compact_window(caps: serde_json::Value) -> Result<Option<String>, String> {
    if caps.is_null() {
        return Err(null_err("autoCompactWindowOptions"));
    }
    if is_undef(&caps) {
        return Err(undef_err("autoCompactWindowOptions"));
    }
    match member(&caps, "autoCompactWindowOptions") {
        Value::Array(list) => Ok(list
            .iter()
            .find(|o| truthy(o.get("isDefault").unwrap_or(&Value::Null)))
            .and_then(|o| o.get("value").and_then(Value::as_str).map(String::from))),
        v if is_undef(&v) || v.is_null() => Ok(None),
        _ => Err("caps.autoCompactWindowOptions.find is not a function".into()),
    }
}

/// Port of `resolveLabeledOptionValue`.
pub fn resolve_labeled_option_value(
    options: serde_json::Value,
    raw_value: Option<String>,
) -> Result<Option<String>, String> {
    let trimmed = raw_value.and_then(|v| {
        let t = jtrim(&v);
        if t.is_empty() {
            None
        } else {
            Some(t.to_string())
        }
    });
    if !truthy(&options) {
        return Ok(trimmed);
    }
    let Value::Array(list) = &options else {
        return Err(if trimmed.is_some() {
            "options.some is not a function".into()
        } else {
            "options.find is not a function".into()
        });
    };
    if list.is_empty() {
        return Ok(trimmed);
    }
    if let Some(t) = &trimmed
        && list
            .iter()
            .any(|o| o.get("value").and_then(Value::as_str) == Some(t.as_str()))
    {
        return Ok(Some(t.clone()));
    }
    Ok(list
        .iter()
        .find(|o| truthy(o.get("isDefault").unwrap_or(&Value::Null)))
        .or_else(|| list.first())
        .and_then(|o| o.get("value").and_then(Value::as_str).map(String::from)))
}

// ── Provider option selections ───────────────────────────────────────────────

fn selection_value(selections: &Value, id: &str) -> Value {
    if !truthy(selections) {
        return js::undefined();
    }
    if let Value::Array(list) = selections {
        return list
            .iter()
            .find(|s| member(s, "id").as_str() == Some(id))
            .map(|s| member(s, "value"))
            .unwrap_or_else(js::undefined);
    }
    let value = member(selections, id);
    if let Some(n) = js::as_number(&value)
        && n.is_finite()
    {
        return json!(js::number_to_string(n));
    }
    value
}

/// Port of `getProviderOptionBooleanSelectionValue`.
pub fn get_provider_option_boolean_selection_value(
    selections: serde_json::Value,
    id: String,
) -> Result<Option<bool>, String> {
    Ok(selection_value(&selections, &id).as_bool())
}

/// Port of `getModelSelectionStringOptionValue`.
pub fn get_model_selection_string_option_value(
    model_selection: serde_json::Value,
    id: String,
) -> Result<Option<String>, String> {
    let options = member(&model_selection, "options");
    Ok(selection_value(&options, &id).as_str().map(String::from))
}

/// Port of `getModelSelectionBooleanOptionValue`.
pub fn get_model_selection_boolean_option_value(
    model_selection: serde_json::Value,
    id: String,
) -> Result<Option<bool>, String> {
    let options = member(&model_selection, "options");
    Ok(selection_value(&options, &id).as_bool())
}

// ── Provider option descriptors ──────────────────────────────────────────────

fn clone_descriptor(d: &Value) -> Value {
    let Some(obj) = d.as_object() else {
        return d.clone();
    };
    let mut out = obj.clone();
    if obj.get("type").and_then(Value::as_str) == Some("select") {
        if let Some(opts) = obj.get("options").and_then(Value::as_array) {
            out.insert("options".into(), Value::Array(opts.clone()));
        }
        if let Some(p) = obj.get("promptInjectedValues").and_then(Value::as_array) {
            out.insert("promptInjectedValues".into(), Value::Array(p.clone()));
        }
    }
    Value::Object(out)
}

fn resolve_choice(descriptor: &Value, raw: Option<String>) -> Result<Option<String>, String> {
    let trimmed = raw.and_then(|v| {
        let t = jtrim(&v);
        if t.is_empty() {
            None
        } else {
            Some(t.to_string())
        }
    });
    let options = member(descriptor, "options");
    let Value::Array(list) = &options else {
        return Err("descriptor.options.some is not a function".into());
    };
    if let Some(t) = &trimmed
        && list
            .iter()
            .any(|o| o.get("id").and_then(Value::as_str) == Some(t.as_str()))
    {
        return Ok(Some(t.clone()));
    }
    let current = member(descriptor, "currentValue");
    if !is_undef(&current) && !current.is_null() {
        return Ok(current.as_str().map(String::from));
    }
    Ok(list
        .iter()
        .find(|o| truthy(o.get("isDefault").unwrap_or(&Value::Null)))
        .and_then(|o| o.get("id").and_then(Value::as_str).map(String::from)))
}

fn with_current_value(descriptor: Value, raw: &Value) -> Result<Value, String> {
    if member(&descriptor, "type").as_str() == Some("boolean") {
        if let Some(b) = raw.as_bool() {
            let mut obj = descriptor.as_object().cloned().unwrap_or_default();
            obj.insert("currentValue".into(), Value::Bool(b));
            return Ok(Value::Object(obj));
        }
        return Ok(descriptor);
    }
    let current = if let Some(s) = raw.as_str() {
        resolve_choice(&descriptor, Some(s.to_string()))?
    } else {
        let cur = member(&descriptor, "currentValue");
        resolve_choice(&descriptor, cur.as_str().map(String::from))?
    };
    let mut obj = descriptor.as_object().cloned().unwrap_or_default();
    match current {
        Some(c) => {
            obj.insert("currentValue".into(), json!(c));
        }
        None => {
            obj.remove("currentValue");
        }
    }
    Ok(Value::Object(obj))
}

fn reasoning_descriptor_id(provider: &str) -> &'static str {
    match provider {
        "claudeAgent" => "effort",
        "opencode" => "variant",
        "pi" | "omp" => "thinkingLevel",
        _ => "reasoningEffort",
    }
}

fn legacy_descriptors(provider: &str, caps: &Value) -> Result<Vec<Value>, String> {
    let primary = if provider == "opencode" {
        let v = member(caps, "variantOptions");
        if is_undef(&v) || v.is_null() {
            Vec::new()
        } else {
            v.as_array().cloned().unwrap_or_default()
        }
    } else {
        match member(caps, "reasoningEffortLevels") {
            Value::Array(list) => list,
            v if is_undef(&v) => return Err(undef_err("length")),
            v if v.is_null() => return Err(null_err("length")),
            _ => Vec::new(),
        }
    };
    let mut descriptors: Vec<Value> = Vec::new();
    if !primary.is_empty() {
        let default_primary = primary
            .iter()
            .find(|o| truthy(o.get("isDefault").unwrap_or(&Value::Null)));
        let options: Vec<Value> = primary
            .iter()
            .map(|o| {
                let mut d = Map::new();
                d.insert("id".into(), member(o, "value"));
                d.insert("label".into(), member(o, "label"));
                if let Some(desc) = o.get("description")
                    && !is_undef(desc)
                    && truthy(desc)
                {
                    d.insert("description".into(), desc.clone());
                }
                if truthy(o.get("isDefault").unwrap_or(&Value::Null)) {
                    d.insert("isDefault".into(), json!(true));
                }
                Value::Object(d)
            })
            .collect();
        let mut d = Map::new();
        d.insert("id".into(), json!(reasoning_descriptor_id(provider)));
        d.insert(
            "label".into(),
            json!(if provider == "opencode" {
                "Variant"
            } else {
                "Reasoning"
            }),
        );
        d.insert("type".into(), json!("select"));
        d.insert("options".into(), Value::Array(options));
        if let Some(def) = default_primary {
            d.insert("currentValue".into(), member(def, "value"));
        }
        match member(caps, "promptInjectedEffortLevels") {
            Value::Array(p) if !p.is_empty() => {
                d.insert("promptInjectedValues".into(), Value::Array(p));
            }
            v if is_undef(&v) => return Err(undef_err("length")),
            _ => {}
        }
        descriptors.push(Value::Object(d));
    }
    let context_opts = match member(caps, "contextWindowOptions") {
        Value::Array(list) => list,
        v if is_undef(&v) => return Err(undef_err("length")),
        v if v.is_null() => return Err(null_err("length")),
        _ => Vec::new(),
    };
    if !context_opts.is_empty() {
        let default_opt = context_opts
            .iter()
            .find(|o| truthy(o.get("isDefault").unwrap_or(&Value::Null)));
        let options: Vec<Value> = context_opts
            .iter()
            .map(|o| {
                let mut d = Map::new();
                d.insert("id".into(), member(o, "value"));
                d.insert("label".into(), member(o, "label"));
                if truthy(o.get("isDefault").unwrap_or(&Value::Null)) {
                    d.insert("isDefault".into(), json!(true));
                }
                Value::Object(d)
            })
            .collect();
        let mut d = Map::new();
        d.insert("id".into(), json!("contextWindow"));
        d.insert("label".into(), json!("Context Window"));
        d.insert("type".into(), json!("select"));
        d.insert("options".into(), Value::Array(options));
        if let Some(def) = default_opt {
            d.insert("currentValue".into(), member(def, "value"));
        }
        descriptors.push(Value::Object(d));
    }
    let auto_opts = member(caps, "autoCompactWindowOptions");
    if truthy(&auto_opts) {
        match &auto_opts {
            Value::Array(list) if !list.is_empty() => {
                let default_opt = list
                    .iter()
                    .find(|o| truthy(o.get("isDefault").unwrap_or(&Value::Null)));
                let options: Vec<Value> = list
                    .iter()
                    .map(|o| {
                        let mut d = Map::new();
                        d.insert("id".into(), member(o, "value"));
                        d.insert("label".into(), member(o, "label"));
                        if truthy(o.get("isDefault").unwrap_or(&Value::Null)) {
                            d.insert("isDefault".into(), json!(true));
                        }
                        Value::Object(d)
                    })
                    .collect();
                let mut d = Map::new();
                d.insert("id".into(), json!("autoCompactWindow"));
                d.insert("label".into(), json!("Auto-compact"));
                d.insert("type".into(), json!("select"));
                d.insert("options".into(), Value::Array(options));
                if let Some(def) = default_opt {
                    d.insert("currentValue".into(), member(def, "value"));
                }
                descriptors.push(Value::Object(d));
            }
            Value::Array(_) => {}
            _ => {
                return Err("caps.autoCompactWindowOptions.find is not a function".into());
            }
        }
    }
    if truthy(&member(caps, "supportsFastMode")) {
        descriptors.push(json!({"id": "fastMode", "label": "Fast Mode", "type": "boolean"}));
    }
    if truthy(&member(caps, "supportsThinkingToggle")) {
        descriptors.push(
            json!({"id": "thinking", "label": "Thinking", "type": "boolean", "currentValue": true}),
        );
    }
    Ok(descriptors)
}

/// Port of `getProviderOptionDescriptors`.
pub fn get_provider_option_descriptors(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if input.is_null() {
        return Err(null_err("caps"));
    }
    if is_undef(&input) {
        return Err(undef_err("caps"));
    }
    let caps = member(&input, "caps");
    if caps.is_null() {
        return Err(null_err("optionDescriptors"));
    }
    if is_undef(&caps) {
        return Err(undef_err("optionDescriptors"));
    }
    let descriptors = match member(&caps, "optionDescriptors") {
        Value::Array(list) => list.iter().map(clone_descriptor).collect::<Vec<_>>(),
        v if is_undef(&v) || v.is_null() => legacy_descriptors(
            input.get("provider").and_then(Value::as_str).unwrap_or(""),
            &caps,
        )?,
        _ => return Err("caps.optionDescriptors.map is not a function".into()),
    };
    let provider = input.get("provider").and_then(Value::as_str).unwrap_or("");
    let selections = member(&input, "selections");
    let mut out = Vec::new();
    for descriptor in descriptors {
        let mut resolved = descriptor;
        if provider == "claudeAgent"
            && member(&resolved, "type").as_str() == Some("select")
            && member(&resolved, "id").as_str() == Some("effort")
        {
            let options = member(&resolved, "options");
            let Value::Array(list) = &options else {
                return Err("descriptor.options.filter is not a function".into());
            };
            let mut obj = resolved.as_object().cloned().unwrap_or_default();
            let filtered: Vec<Value> = list
                .iter()
                .filter(|o| o.get("id").and_then(Value::as_str) != Some("default"))
                .cloned()
                .collect();
            obj.insert("options".into(), Value::Array(filtered));
            match obj.get("currentValue") {
                Some(c) if c.as_str() != Some("default") => {}
                _ => {
                    obj.remove("currentValue");
                }
            }
            resolved = Value::Object(obj);
        }
        let raw = selection_value(&selections, member(&resolved, "id").as_str().unwrap_or(""));
        out.push(with_current_value(resolved, &raw)?);
    }
    Ok(Value::Array(out))
}

/// Port of `getProviderOptionCurrentValue`.
pub fn get_provider_option_current_value(
    descriptor: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if !truthy(&descriptor) {
        return Ok(js::undefined());
    }
    if member(&descriptor, "type").as_str() == Some("boolean") {
        return Ok(member(&descriptor, "currentValue"));
    }
    let current = member(&descriptor, "currentValue");
    if !is_undef(&current) && !current.is_null() {
        return Ok(current);
    }
    let options = member(&descriptor, "options");
    let Value::Array(list) = &options else {
        return Err(if is_undef(&options) {
            undef_err("find")
        } else if options.is_null() {
            null_err("find")
        } else {
            "descriptor.options.find is not a function".into()
        });
    };
    Ok(list
        .iter()
        .find(|o| truthy(o.get("isDefault").unwrap_or(&Value::Null)))
        .map(|o| member(o, "id"))
        .unwrap_or_else(js::undefined))
}

// ── Capabilities resolver ────────────────────────────────────────────────────

fn empty_capabilities() -> Value {
    json!({
        "reasoningEffortLevels": [],
        "supportsFastMode": false,
        "supportsThinkingToggle": false,
        "promptInjectedEffortLevels": [],
        "contextWindowOptions": []
    })
}

fn normalize_slug_internal(model: Option<&str>, provider: &Value) -> Result<Option<Value>, String> {
    let Some(model) = model else {
        return Ok(None);
    };
    let trimmed = jtrim(model);
    if trimmed.is_empty() {
        return Ok(None);
    }
    let provider = if is_undef(provider) {
        "codex"
    } else {
        provider_key(provider)
    };
    let scoped: String = if provider == "claudeAgent" {
        strip_claude_context_window_suffix(trimmed.to_string())?
    } else if provider == "devin"
        && trimmed == trimmed.to_lowercase()
        && trimmed.ends_with("-medium")
    {
        trimmed[..trimmed.len() - "-medium".len()].to_string()
    } else {
        trimmed.to_string()
    };
    let Some(aliases) = table("MODEL_SLUG_ALIASES_BY_PROVIDER").get(provider) else {
        return Err("Cannot convert undefined or null to object".into());
    };
    let alias_key = scoped.to_lowercase();
    let normalized = aliases
        .as_object()
        .and_then(|a| a.get(&alias_key))
        .and_then(Value::as_str)
        .unwrap_or(&scoped)
        .to_string();
    if provider == "claudeAgent" && claude_suffix(trimmed).as_deref() == Some("1m") {
        return Ok(Some(json!(format!("{normalized}[1m]"))));
    }
    Ok(Some(json!(normalized)))
}

fn claude_family_version(slug: &str) -> Option<(String, f64, f64)> {
    // /^claude-(fable|opus|sonnet|haiku)-(\d{1,2})(?:-(\d{1,2}))?(?:-\d{8})?$/
    let s = jtrim(slug).to_lowercase();
    let rest = s.strip_prefix("claude-")?;
    let (family, rest) = rest.split_once('-')?;
    if !matches!(family, "fable" | "opus" | "sonnet" | "haiku") {
        return None;
    }
    let segs: Vec<&str> = rest.split('-').collect();
    if segs.is_empty() || segs.len() > 3 {
        return None;
    }
    let major: f64 =
        if !segs[0].is_empty() && segs[0].len() <= 2 && segs[0].chars().all(|c| c.is_ascii_digit())
        {
            segs[0].parse().ok()?
        } else {
            return None;
        };
    let mut minor = 0.0;
    let mut index = 1;
    if index < segs.len() {
        let seg = segs[index];
        if !seg.is_empty() && seg.len() <= 2 && seg.chars().all(|c| c.is_ascii_digit()) {
            minor = seg.parse().ok()?;
            index += 1;
        }
    }
    if index < segs.len() {
        let seg = segs[index];
        if !(seg.len() == 8 && seg.chars().all(|c| c.is_ascii_digit())) {
            return None;
        }
        index += 1;
    }
    if index != segs.len() {
        return None;
    }
    Some((family.to_string(), major, minor))
}

fn newest_claude_catalog() -> &'static Map<String, Value> {
    static NEWEST: OnceLock<Map<String, Value>> = OnceLock::new();
    NEWEST.get_or_init(|| {
        let mut newest: Map<String, Value> = Map::new();
        if let Some(models) = table("MODEL_OPTIONS_BY_PROVIDER")
            .get("claudeAgent")
            .and_then(Value::as_array)
        {
            for model in models {
                if let Some(slug) = model.get("slug").and_then(Value::as_str)
                    && let Some((family, major, minor)) = claude_family_version(slug)
                {
                    let better = newest
                        .get(&family)
                        .and_then(|e| e.get("version"))
                        .and_then(|v| Some((v.get("major")?.as_f64()?, v.get("minor")?.as_f64()?)))
                        .is_none_or(|(ma, mi)| major - ma + (minor - mi) > 0.0 || major > ma);
                    if better {
                        newest.insert(
                            family,
                            json!({"slug": slug, "version": {"major": major, "minor": minor}}),
                        );
                    }
                }
            }
        }
        newest
    })
}

/// Port of `resolveNewestKnownClaudeFamilyModel`.
pub fn resolve_newest_known_claude_family_model(
    model: Option<String>,
) -> Result<serde_json::Value, String> {
    let normalized = normalize_slug_internal(model.as_deref(), &json!("claudeAgent"))?;
    let Some(Value::String(slug)) = normalized else {
        return Ok(Value::Null);
    };
    let slug = strip_claude_context_window_suffix(slug)?;
    let known: std::collections::HashSet<String> = table("MODEL_OPTIONS_BY_PROVIDER")
        .get("claudeAgent")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|o| o.get("slug").and_then(Value::as_str).map(String::from))
                .collect()
        })
        .unwrap_or_default();
    if known.contains(&slug) {
        return Ok(Value::Null);
    }
    let Some((family, major, minor)) = claude_family_version(&slug) else {
        return Ok(Value::Null);
    };
    let Some(entry) = newest_claude_catalog().get(&family) else {
        return Ok(Value::Null);
    };
    let newest_major = entry
        .get("version")
        .and_then(|v| v.get("major"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let newest_minor = entry
        .get("version")
        .and_then(|v| v.get("minor"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    if major - newest_major != 0.0 {
        if major > newest_major {
            return Ok(entry.get("slug").cloned().unwrap_or(Value::Null));
        }
        return Ok(Value::Null);
    }
    if minor > newest_minor {
        return Ok(entry.get("slug").cloned().unwrap_or(Value::Null));
    }
    Ok(Value::Null)
}

/// Port of `resolveGrokEffortFamily`.
pub fn resolve_grok_effort_family(model: String) -> Result<String, String> {
    let slug = jtrim(&model).to_lowercase();
    // /grok-(\d+)\.(\d+)/u
    let mut major = None;
    let mut minor = None;
    let mut idx = 0;
    while let Some(pos) = slug[idx..].find("grok-") {
        let rest = &slug[idx + pos + 5..];
        let m_end = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        if m_end > 0 && rest[m_end..].starts_with('.') {
            let nrest = &rest[m_end + 1..];
            let n_end = nrest
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(nrest.len());
            if n_end > 0 {
                major = rest[..m_end].parse::<f64>().ok();
                minor = nrest[..n_end].parse::<f64>().ok();
                break;
            }
        }
        idx += pos + 1;
    }
    let versioned =
        matches!((major, minor), (Some(ma), Some(mi)) if ma > 4.0 || (ma == 4.0 && mi >= 5.0));
    if !versioned
        && (slug.contains("build")
            || slug.contains("code-fast")
            || slug == "grok-4"
            || slug == "grok-4.3"
            || slug.starts_with("grok-4.3-"))
    {
        return Ok("build".into());
    }
    let (Some(ma), Some(mi)) = (major, minor) else {
        return Ok("build".into());
    };
    if ma < 4.0 || (ma == 4.0 && mi <= 3.0) {
        return Ok("build".into());
    }
    if ma == 4.0 && mi == 5.0 {
        return Ok("4.5".into());
    }
    Ok("4.6".into())
}

fn grok_caps_for_family(family: &str) -> Value {
    let grok = table("MODEL_CAPABILITIES_INDEX").get("grok");
    let pick = |slug: &str| grok.and_then(|g| g.get(slug)).cloned();
    match family {
        "build" => pick("grok-build").unwrap_or_else(empty_capabilities),
        "4.5" => pick("grok-4.5")
            .or_else(|| pick("grok-4.6"))
            .unwrap_or_else(empty_capabilities),
        _ => pick("grok-4.6").unwrap_or_else(empty_capabilities),
    }
}

/// Port of `isClaudeUltrathinkPrompt`.
pub fn is_claude_ultrathink_prompt(text: Option<String>) -> Result<bool, String> {
    let Some(text) = text else {
        return Ok(false);
    };
    let lower = text.to_lowercase();
    let needle = "ultrathink";
    for (i, _) in lower.match_indices(needle) {
        let before_ok = i == 0
            || !lower[..i]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
        let after_ok = i + needle.len() >= lower.len()
            || !lower[i + needle.len()..]
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
        if before_ok && after_ok {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Port of `normalizeModelSlug`.
pub fn normalize_model_slug(
    model: Option<String>,
    provider: serde_json::Value,
) -> Result<serde_json::Value, String> {
    Ok(normalize_slug_internal(model.as_deref(), &provider)?.unwrap_or(Value::Null))
}

/// Port of `resolveSelectableModel`.
pub fn resolve_selectable_model(
    provider: serde_json::Value,
    value: Option<String>,
    options: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let Some(value) = value else {
        return Ok(Value::Null);
    };
    let trimmed = jtrim(&value);
    if trimmed.is_empty() {
        return Ok(Value::Null);
    }
    let Value::Array(list) = &options else {
        return Err("options.find is not a function".into());
    };
    if let Some(direct) = list
        .iter()
        .find(|o| o.get("slug").and_then(Value::as_str) == Some(trimmed))
    {
        return Ok(member(direct, "slug"));
    }
    if let Some(by_name) = list.iter().find(|o| {
        o.get("name")
            .and_then(Value::as_str)
            .is_some_and(|n| n.to_lowercase() == trimmed.to_lowercase())
    }) {
        return Ok(member(by_name, "slug"));
    }
    let normalized = normalize_slug_internal(Some(trimmed), &provider)?;
    let Some(Value::String(norm)) = normalized else {
        return Ok(Value::Null);
    };
    if let Some(resolved) = list
        .iter()
        .find(|o| o.get("slug").and_then(Value::as_str) == Some(norm.as_str()))
    {
        return Ok(member(resolved, "slug"));
    }
    let pkey = provider_key(&provider);
    if matches!(pkey, "omp" | "pi" | "opencode") && !norm.contains('/') {
        let scoped: Vec<&Value> = list
            .iter()
            .filter(|o| {
                o.get("slug")
                    .and_then(Value::as_str)
                    .is_some_and(|s| s.ends_with(&format!("/{norm}")))
            })
            .collect();
        if scoped.len() == 1 {
            return Ok(member(scoped[0], "slug"));
        }
    }
    Ok(Value::Null)
}

/// Port of `resolveModelSlug`.
pub fn resolve_model_slug(
    model: Option<String>,
    provider: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let normalized = normalize_slug_internal(model.as_deref(), &provider)?;
    let pkey = if is_undef(&provider) {
        "codex"
    } else {
        provider_key(&provider)
    };
    let normalized = if pkey == "claudeAgent" {
        normalized
            .and_then(|v| v.as_str().map(|s| json!(strip(s))).map(Some))
            .flatten()
    } else {
        normalized
    };
    if matches!(pkey, "devin" | "pi" | "omp") {
        return Ok(normalized.unwrap_or(Value::Null));
    }
    let Some(Value::String(norm)) = normalized else {
        return Ok(table("DEFAULT_MODEL_BY_PROVIDER")
            .get(pkey)
            .cloned()
            .unwrap_or_else(js::undefined));
    };
    let in_set = table("MODEL_OPTIONS_BY_PROVIDER")
        .get(pkey)
        .and_then(Value::as_array)
        .is_some_and(|list| {
            list.iter()
                .any(|o| o.get("slug").and_then(Value::as_str) == Some(norm.as_str()))
        });
    if !table("MODEL_OPTIONS_BY_PROVIDER").contains_key(pkey) && !in_set {
        return Err(undef_err("has"));
    }
    Ok(if in_set {
        json!(norm)
    } else {
        table("DEFAULT_MODEL_BY_PROVIDER")
            .get(pkey)
            .cloned()
            .unwrap_or_else(js::undefined)
    })
}

fn strip(s: &str) -> String {
    if !s.ends_with(']') {
        return s.to_string();
    }
    if let Some(open) = s[..s.len() - 1].rfind('[') {
        return s[..open].to_string();
    }
    s.to_string()
}

/// Port of `resolveModelSlugForProvider`.
pub fn resolve_model_slug_for_provider(
    provider: serde_json::Value,
    model: Option<String>,
) -> Result<serde_json::Value, String> {
    resolve_model_slug(model, provider)
}

/// Port of `trimOrNull`.
pub fn trim_or_null(value: Option<String>) -> Result<Option<String>, String> {
    Ok(value.and_then(|v| {
        let t = jtrim(&v);
        if t.is_empty() {
            None
        } else {
            Some(t.to_string())
        }
    }))
}

/// Port of `getModelCapabilities`.
pub fn get_model_capabilities(
    provider: serde_json::Value,
    model: Option<String>,
) -> Result<serde_json::Value, String> {
    let normalized = normalize_slug_internal(model.as_deref(), &provider)?;
    let pkey = if is_undef(&provider) {
        "codex"
    } else {
        provider_key(&provider)
    };
    let slug = if pkey == "claudeAgent" {
        normalized
            .as_ref()
            .and_then(Value::as_str)
            .map(|s| strip(s).to_string())
    } else {
        normalized
            .as_ref()
            .and_then(Value::as_str)
            .map(String::from)
    };
    if let Some(slug) = &slug
        && let Some(caps) = table("MODEL_CAPABILITIES_INDEX")
            .get(pkey)
            .and_then(|p| p.get(slug))
    {
        return Ok(caps.clone());
    }
    if pkey == "grok" && slug.is_some() {
        return Ok(grok_caps_for_family(&resolve_grok_effort_family(
            slug.unwrap_or_default(),
        )?));
    }
    if pkey == "claudeAgent" && slug.is_some() {
        let newest =
            resolve_newest_known_claude_family_model(slug.clone().map(Some).unwrap_or(None))?;
        if let Value::String(n) = newest {
            return Ok(table("MODEL_CAPABILITIES_INDEX")
                .get("claudeAgent")
                .and_then(|p| p.get(&n))
                .cloned()
                .unwrap_or_else(empty_capabilities));
        }
    }
    Ok(empty_capabilities())
}

// ── Option normalization ─────────────────────────────────────────────────────

/// Port of `normalizeClaudeModelOptions`.
pub fn normalize_claude_model_options(
    model: Option<String>,
    model_options: serde_json::Value,
    caps: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let caps = if is_undef(&caps) {
        get_model_capabilities(json!("claudeAgent"), model.clone())?
    } else {
        caps
    };
    let default_effort = caps_find_default(&caps, "reasoningEffortLevels")?;
    let default_window = caps_find_default_optional(&caps, "autoCompactWindowOptions")?;
    let resolved_effort = trim_or_null_str(Some(&member(&model_options, "effort")));
    let resolved_window = trim_or_null_str(Some(&member(&model_options, "autoCompactWindow")))
        .or_else(|| trim_or_null_str(Some(&member(&model_options, "contextWindow"))));
    let injected = match member(&caps, "promptInjectedEffortLevels") {
        Value::Array(list) => list
            .iter()
            .any(|l| l.as_str() == Some(resolved_effort.as_deref().unwrap_or(""))),
        v if is_undef(&v) => return Err(undef_err("includes")),
        v if v.is_null() => return Err(null_err("includes")),
        _ => return Err("caps.promptInjectedEffortLevels.includes is not a function".into()),
    };
    let effort = match &resolved_effort {
        Some(e)
            if !injected
                && caps_list_has(&caps, "reasoningEffortLevels", e)?
                && Some(e.clone()) != default_effort =>
        {
            Some(e.clone())
        }
        _ => None,
    };
    let window = match &resolved_window {
        Some(w)
            if caps_list_has(&caps, "autoCompactWindowOptions", w)?
                && Some(w.clone()) != default_window =>
        {
            Some(w.clone())
        }
        _ => None,
    };
    let thinking = if truthy(&member(&caps, "supportsThinkingToggle"))
        && member(&model_options, "thinking") == Value::Bool(false)
    {
        Some(false)
    } else {
        None
    };
    let fast = if truthy(&member(&caps, "supportsFastMode"))
        && member(&model_options, "fastMode") == Value::Bool(true)
    {
        Some(true)
    } else {
        None
    };
    let mut out = Map::new();
    if let Some(t) = thinking {
        out.insert("thinking".into(), json!(t));
    }
    if let Some(e) = effort {
        out.insert("effort".into(), json!(e));
    }
    if let Some(f) = fast {
        out.insert("fastMode".into(), json!(f));
    }
    if let Some(w) = window {
        out.insert("autoCompactWindow".into(), json!(w));
    }
    if out.is_empty() {
        Ok(js::undefined())
    } else {
        Ok(Value::Object(out))
    }
}

fn caps_find_default(caps: &Value, key: &str) -> Result<Option<String>, String> {
    if caps.is_null() {
        return Err(null_err(key));
    }
    if is_undef(caps) {
        return Err(undef_err(key));
    }
    match member(caps, key) {
        Value::Array(list) => Ok(list
            .iter()
            .find(|o| truthy(o.get("isDefault").unwrap_or(&Value::Null)))
            .and_then(|o| o.get("value").and_then(Value::as_str).map(String::from))),
        v if is_undef(&v) => Err(undef_err("find")),
        v if v.is_null() => Err(null_err("find")),
        _ => Err(format!("caps.{key}.find is not a function")),
    }
}

fn caps_find_default_optional(caps: &Value, key: &str) -> Result<Option<String>, String> {
    match member(caps, key) {
        Value::Array(list) => Ok(list
            .iter()
            .find(|o| truthy(o.get("isDefault").unwrap_or(&Value::Null)))
            .and_then(|o| o.get("value").and_then(Value::as_str).map(String::from))),
        v if is_undef(&v) || v.is_null() => Ok(None),
        _ => Err(format!("caps.{key}.find is not a function")),
    }
}

fn caps_list_has(caps: &Value, key: &str, value: &str) -> Result<bool, String> {
    match member(caps, key) {
        Value::Array(list) => Ok(list
            .iter()
            .any(|o| o.get("value").and_then(Value::as_str) == Some(value))),
        v if is_undef(&v) => Err(undef_err("some")),
        v if v.is_null() => Err(null_err("some")),
        _ => Err(format!("caps.{key}.some is not a function")),
    }
}

/// Port of `resolveApiModelId`.
pub fn resolve_api_model_id(
    model_selection: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if model_selection.is_null() {
        return Err(null_err("provider"));
    }
    if is_undef(&model_selection) {
        return Err(undef_err("provider"));
    }
    let model = member(&model_selection, "model");
    if member(&model_selection, "provider").as_str() == Some("claudeAgent")
        && normalize_claude_model_options(
            model.as_str().map(String::from),
            member(&model_selection, "options"),
            js::undefined(),
        )?
        .get("autoCompactWindow")
        .and_then(Value::as_str)
            == Some("1m")
        && claude_suffix(model.as_str().unwrap_or_default()).is_none()
    {
        return Ok(json!(format!("{}[1m]", js_string(&model))));
    }
    Ok(model)
}

/// Port of `getEffectiveClaudeCodeEffort`.
pub fn get_effective_claude_code_effort(
    effort: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if !truthy(&effort) || effort.as_str() == Some("ultrathink") {
        return Ok(Value::Null);
    }
    if effort.as_str() == Some("ultracode") {
        return Ok(json!("xhigh"));
    }
    Ok(effort)
}

fn claude_effort_set() -> &'static Vec<Value> {
    str_set("CLAUDE_CODE_EFFORT_OPTIONS")
}

fn claude_spawn_profile(selection: &Value) -> Result<(bool, Value), String> {
    let options = member(selection, "options");
    let effort_raw = trim_or_null_str(Some(&member(&options, "effort")));
    let effort = effort_raw.filter(|e| {
        claude_effort_set()
            .iter()
            .any(|o| o.as_str() == Some(e.as_str()))
    });
    let max_effort =
        get_effective_claude_code_effort(effort.map(Value::String).unwrap_or(Value::Null))?
            .as_str()
            == Some("max");
    let window = normalize_claude_model_options(
        member(selection, "model").as_str().map(String::from),
        options,
        js::undefined(),
    )?
    .get("autoCompactWindow")
    .cloned()
    .unwrap_or_else(js::undefined);
    Ok((max_effort, window))
}

/// Port of `claudeSelectionRequiresRestart`.
pub fn claude_selection_requires_restart(
    previous: serde_json::Value,
    next: serde_json::Value,
) -> Result<bool, String> {
    if next.is_null() {
        return Err(null_err("provider"));
    }
    if is_undef(&next) {
        return Err(undef_err("provider"));
    }
    if member(&next, "provider").as_str() != Some("claudeAgent") {
        return Ok(false);
    }
    if is_undef(&previous) {
        return Ok(false);
    }
    if previous.is_null() {
        return Err(null_err("provider"));
    }
    if member(&previous, "provider").as_str() != Some("claudeAgent") {
        return Ok(true);
    }
    let prev = claude_spawn_profile(&previous)?;
    let desired = claude_spawn_profile(&next)?;
    Ok(prev.0 != desired.0 || prev.1 != desired.1)
}

/// Port of `normalizeCursorModelOptions`.
pub fn normalize_cursor_model_options(
    model: Option<String>,
    model_options: serde_json::Value,
    capabilities: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let caps = if is_undef(&capabilities) {
        get_model_capabilities(json!("cursor"), model)?
    } else {
        capabilities
    };
    let default_effort = caps_find_default(&caps, "reasoningEffortLevels")?;
    let raw_effort = trim_or_null_str(Some(&member(&model_options, "reasoningEffort")));
    let effort = match &raw_effort {
        Some(e) if caps_list_has(&caps, "reasoningEffortLevels", e)? => Some(e.clone()),
        _ => match &default_effort {
            Some(d) if caps_list_has(&caps, "reasoningEffortLevels", d)? => Some(d.clone()),
            _ => None,
        },
    };
    let raw_window = trim_or_null_str(Some(&member(&model_options, "contextWindow")));
    let default_window = caps_find_default_optional(&caps, "contextWindowOptions")?;
    let window = match &raw_window {
        Some(w)
            if caps_list_has(&caps, "contextWindowOptions", w)?
                && Some(w.clone()) != default_window =>
        {
            Some(w.clone())
        }
        _ => None,
    };
    let fast = if truthy(&member(&caps, "supportsFastMode")) {
        Some(member(&model_options, "fastMode") == Value::Bool(true))
    } else {
        None
    };
    let thinking_raw = member(&model_options, "thinking");
    let thinking = if truthy(&member(&caps, "supportsThinkingToggle")) && !is_undef(&thinking_raw) {
        Some(thinking_raw)
    } else {
        None
    };
    let mut out = Map::new();
    if let Some(e) = effort {
        out.insert("reasoningEffort".into(), json!(e));
    }
    if let Some(f) = fast {
        out.insert("fastMode".into(), json!(f));
    }
    if let Some(t) = thinking {
        out.insert("thinking".into(), t);
    }
    if let Some(w) = window {
        out.insert("contextWindow".into(), json!(w));
    }
    if out.is_empty() {
        Ok(js::undefined())
    } else {
        Ok(Value::Object(out))
    }
}

/// Port of `normalizeGrokModelOptions`.
pub fn normalize_grok_model_options(
    model: Option<String>,
    model_options: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let caps = get_model_capabilities(json!("grok"), model)?;
    let reasoning = trim_or_null_str(Some(&member(&model_options, "reasoningEffort")));
    let Some(effort) = reasoning else {
        return Ok(js::undefined());
    };
    if !caps_list_has(&caps, "reasoningEffortLevels", &effort)? {
        return Ok(js::undefined());
    }
    if Some(effort.clone()) == caps_find_default(&caps, "reasoningEffortLevels")? {
        return Ok(js::undefined());
    }
    Ok(json!({"reasoningEffort": effort}))
}

/// Port of `normalizeAntigravityModelOptions`.
pub fn normalize_antigravity_model_options(
    model: Option<String>,
    model_options: serde_json::Value,
    capabilities: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let caps = if is_undef(&capabilities) {
        get_model_capabilities(json!("antigravity"), model)?
    } else {
        capabilities
    };
    let Some(effort) = trim_or_null_str(Some(&member(&model_options, "reasoningEffort"))) else {
        return Ok(js::undefined());
    };
    if !caps_list_has(&caps, "reasoningEffortLevels", &effort)? {
        return Ok(js::undefined());
    }
    if Some(effort.clone()) == caps_find_default(&caps, "reasoningEffortLevels")? {
        return Ok(js::undefined());
    }
    Ok(json!({"reasoningEffort": effort}))
}

/// Port of `normalizePiModelOptions`.
pub fn normalize_pi_model_options(
    model_options: serde_json::Value,
) -> Result<serde_json::Value, String> {
    const LEVELS: [&str; 7] = ["off", "minimal", "low", "medium", "high", "xhigh", "max"];
    match trim_or_null_str(Some(&member(&model_options, "thinkingLevel"))) {
        Some(level) if LEVELS.contains(&level.as_str()) => Ok(json!({"thinkingLevel": level})),
        _ => Ok(js::undefined()),
    }
}

/// Port of `normalizeOmpModelOptions`.
pub fn normalize_omp_model_options(
    model_options: serde_json::Value,
) -> Result<serde_json::Value, String> {
    match trim_or_null_str(Some(&member(&model_options, "thinkingLevel"))) {
        Some(level)
            if str_set("OMP_THINKING_LEVEL_OPTIONS")
                .iter()
                .any(|o| o.as_str() == Some(level.as_str())) =>
        {
            Ok(json!({"thinkingLevel": level}))
        }
        _ => Ok(js::undefined()),
    }
}

/// Port of `normalizeOpenCodeModelOptions`.
pub fn normalize_open_code_model_options(
    model_options: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let mut out = Map::new();
    if let Some(v) = trim_or_null_str(Some(&member(&model_options, "variant"))) {
        out.insert("variant".into(), json!(v));
    }
    if let Some(a) = trim_or_null_str(Some(&member(&model_options, "agent"))) {
        out.insert("agent".into(), json!(a));
    }
    if out.is_empty() {
        Ok(js::undefined())
    } else {
        Ok(Value::Object(out))
    }
}

/// Port of `applyClaudePromptEffortPrefix`.
pub fn apply_claude_prompt_effort_prefix(
    text: String,
    effort: serde_json::Value,
) -> Result<String, String> {
    let trimmed = jtrim(&text).to_string();
    if trimmed.is_empty() {
        return Ok(trimmed);
    }
    if effort.as_str() != Some("ultrathink") {
        return Ok(trimmed);
    }
    if trimmed.starts_with("Ultrathink:") {
        return Ok(trimmed);
    }
    Ok(format!("Ultrathink:\n{trimmed}"))
}

// ── modelName.ts helpers ─────────────────────────────────────────────────────

fn token_display_name(token: &str) -> Option<&'static str> {
    Some(match token {
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

fn join_version_tokens(tokens: &[String]) -> Vec<String> {
    let mut merged: Vec<String> = Vec::new();
    for token in tokens {
        let is_digits = !token.is_empty() && token.chars().all(|c| c.is_ascii_digit());
        let mergeable = is_digits
            && (token == "0" || !token.starts_with('0'))
            && token.len() != 8
            && merged
                .last()
                .is_some_and(|p| p.chars().last().is_some_and(|c| c.is_ascii_digit()));
        if mergeable {
            let prev = merged.pop().unwrap_or_default();
            merged.push(format!("{prev}.{token}"));
        } else {
            merged.push(token.clone());
        }
    }
    merged
}

fn restore_separators(name: &str) -> String {
    // /\bGPT (\d)/g → "GPT-$1"
    let mut out = String::new();
    let bytes: Vec<char> = name.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        if i + 3 < bytes.len()
            && bytes[i] == 'G'
            && bytes[i + 1] == 'P'
            && bytes[i + 2] == 'T'
            && bytes[i + 3] == ' '
            && i + 4 < bytes.len()
            && bytes[i + 4].is_ascii_digit()
            && (i == 0 || !bytes[i - 1].is_ascii_alphanumeric() && bytes[i - 1] != '_')
        {
            out.push_str("GPT-");
            i += 4;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    out
}

fn split_tokens(s: &str, on_space: bool) -> Vec<String> {
    s.split(|c: char| c == '-' || c == '_' || (on_space && c.is_whitespace()))
        .filter(|t| !t.is_empty())
        .map(String::from)
        .collect()
}

fn humanize_model_slug(slug: &str) -> String {
    if slug.contains('/') {
        return slug.to_string();
    }
    let tokens = join_version_tokens(&split_tokens(slug, false));
    let humanized: Vec<String> = tokens
        .iter()
        .map(|t| {
            token_display_name(&t.to_lowercase())
                .map(String::from)
                .unwrap_or_else(|| {
                    let mut chars = t.chars();
                    match chars.next() {
                        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                        None => String::new(),
                    }
                })
        })
        .collect();
    restore_separators(&humanized.join(" "))
}
