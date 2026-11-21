use serde_json::{Value, json};
use std::collections::HashMap;

use crate::js::{as_number, is_undefined, number, number_to_string, string_units};

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
}

fn as_js_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        _ => string_units(value).map(|units| String::from_utf16_lossy(&units)),
    }
}

fn is_nullish(v: &Value) -> bool {
    v.is_null() || is_undefined(v)
}

// `readString` — string with non-blank trim → the ORIGINAL (untrimmed) string.
fn read_string_opt(v: Option<&Value>) -> Option<String> {
    let s = as_js_string(v?)?;
    if js_trim(&s).is_empty() {
        None
    } else {
        Some(s)
    }
}

fn is_marker_primitive(v: &Value) -> bool {
    is_undefined(v) || as_number(v).is_some() || string_units(v).is_some()
}

static MISSING: Value = Value::Null;

fn member<'a>(row: &'a Value, key: &str) -> Result<&'a Value, String> {
    match row {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        Value::Object(m) => Ok(m.get(key).unwrap_or(&MISSING)),
        _ => {
            if is_undefined(row) {
                Err(format!(
                    "Cannot read properties of undefined (reading '{key}')"
                ))
            } else {
                Ok(&MISSING)
            }
        }
    }
}

// `for (const x of value)` — arrays iterate elements, strings iterate code points.
fn iter_values(v: &Value, name: &str) -> Result<Vec<Value>, String> {
    match v {
        Value::Array(a) => Ok(a.clone()),
        Value::String(s) => Ok(s.chars().map(|c| Value::String(c.to_string())).collect()),
        Value::Object(_) => {
            if let Some(units) = string_units(v) {
                Ok(String::from_utf16_lossy(&units)
                    .chars()
                    .map(|c| Value::String(c.to_string()))
                    .collect())
            } else {
                Err(format!("{name} is not iterable"))
            }
        }
        _ => Err(format!("{name} is not iterable")),
    }
}

// JS `${value}` — String() coercion inside template literals / key joins.
fn js_template_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        Value::Array(a) => a
            .iter()
            .map(|e| {
                if is_nullish(e) {
                    String::new()
                } else {
                    js_template_string(e)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => {
            if is_undefined(v) {
                "undefined".to_string()
            } else if let Some(x) = as_number(v) {
                number_to_string(x)
            } else if let Some(s) = as_js_string(v) {
                s
            } else {
                "[object Object]".to_string()
            }
        }
    }
}

#[derive(Clone, Default)]
struct ModelSelection {
    provider: Option<String>,
    instance_id: Option<String>,
    model: Option<String>,
    reasoning: Option<String>,
}

fn read_reasoning_option(options: Option<&Value>) -> Option<String> {
    let options = options?;
    match options {
        Value::Array(items) => {
            for id in ["reasoningEffort", "effort"] {
                let found = items.iter().find(|e| {
                    matches!(e, Value::Object(m) if !is_marker_primitive(e) && m.get("id") == Some(&json!(id)))
                });
                let value = found.and_then(|e| read_string_opt(e.get("value")));
                if value.is_some() {
                    return value;
                }
            }
            None
        }
        Value::Object(m) => {
            if is_marker_primitive(options) {
                return None;
            }
            read_string_opt(m.get("reasoningEffort")).or_else(|| read_string_opt(m.get("effort")))
        }
        _ => None,
    }
}

fn parse_model_selection(value: &Value) -> Option<ModelSelection> {
    let Value::Object(m) = value else {
        return None;
    };
    if is_marker_primitive(value) {
        return None;
    }
    let provider = read_string_opt(m.get("provider"));
    Some(ModelSelection {
        instance_id: read_string_opt(m.get("instanceId")).or_else(|| provider.clone()),
        provider,
        model: read_string_opt(m.get("model")),
        reasoning: read_reasoning_option(m.get("options")),
    })
}

fn parse_model_selection_json(json: &Value) -> Result<Option<ModelSelection>, String> {
    if json.is_null() {
        return Ok(None);
    }
    if is_undefined(json) {
        return Err("Cannot read properties of undefined (reading 'trim')".to_string());
    }
    // `json.trim()` — only strings carry trim.
    let Some(s) = as_js_string(json) else {
        return Err("json.trim is not a function".to_string());
    };
    if js_trim(&s).is_empty() {
        return Ok(None);
    }
    let Ok(parsed) = serde_json::from_str::<Value>(&s) else {
        return Ok(None);
    };
    Ok(parse_model_selection(&parsed))
}

struct TurnRow {
    provider: Option<String>,
    instance_id: Option<String>,
    model: Option<String>,
    reasoning: Option<String>,
    turn_count: f64,
}

// `JSON.parse(arg)` — the argument is coerced to a string first.
fn json_parse_coerced(v: &Value) -> Option<Value> {
    let s = match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        Value::Array(a) => a
            .iter()
            .map(|e| {
                if is_nullish(e) {
                    String::new()
                } else {
                    js_template_string(e)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => {
            if let Some(x) = as_number(v) {
                number_to_string(x)
            } else if let Some(s) = as_js_string(v) {
                s
            } else if is_undefined(v) {
                return None; // JSON.parse(undefined) throws
            } else {
                "[object Object]".to_string()
            }
        }
    };
    serde_json::from_str(&s).ok()
}

pub fn aggregate_thread_turn_snapshot_rows(
    events: Value,
    thread_model_selection_json: Value,
) -> Result<Vec<Value>, String> {
    let thread_selection = parse_model_selection_json(&thread_model_selection_json)?;
    let mut counts: Vec<TurnRow> = vec![];
    let mut counts_index: HashMap<String, usize> = HashMap::new();

    for event in iter_values(&events, "events")? {
        let payload_json = member(&event, "payloadJson")?;
        let mut event_selection: Option<ModelSelection> = None;
        if !payload_json.is_null()
            && let Some(payload) = json_parse_coerced(payload_json)
            && let Value::Object(m) = &payload
            && !is_marker_primitive(&payload)
        {
            event_selection = parse_model_selection(m.get("modelSelection").unwrap_or(&MISSING));
        }
        let selection = event_selection.as_ref().or(thread_selection.as_ref());
        let provider = selection.and_then(|s| s.provider.clone());
        let instance_id = selection
            .and_then(|s| s.instance_id.clone())
            .or_else(|| provider.clone());
        let model = selection.and_then(|s| s.model.clone());
        let reasoning = selection.and_then(|s| s.reasoning.clone());
        let key = format!(
            "{}\u{0}{}\u{0}{}\u{0}{}",
            provider.as_deref().unwrap_or(""),
            instance_id.as_deref().unwrap_or(""),
            model.as_deref().unwrap_or(""),
            reasoning.as_deref().unwrap_or("")
        );
        match counts_index.get(&key) {
            Some(&i) => counts[i].turn_count += 1.0,
            None => {
                counts_index.insert(key, counts.len());
                counts.push(TurnRow {
                    provider,
                    instance_id,
                    model,
                    reasoning,
                    turn_count: 1.0,
                });
            }
        }
    }

    Ok(counts
        .iter()
        .map(|row| {
            json!({
                "provider": row.provider,
                "instanceId": row.instance_id,
                "model": row.model,
                "reasoning": row.reasoning,
                "turnCount": number(row.turn_count),
            })
        })
        .collect())
}

// `tokenCounterValue` — real finite numbers only (bigint unreachable on the wire).
fn token_counter_value(v: &Value) -> Option<f64> {
    let total = match v {
        Value::Number(n) => n.as_f64(),
        Value::Object(_) => as_number(v),
        _ => None,
    }?;
    if total.is_finite() { Some(total) } else { None }
}

struct PmSel {
    provider: Option<String>,
    instance_id: Option<String>,
    model: Option<String>,
}

fn fb_str(fallback: &Value, key: &str) -> Option<String> {
    if let Value::Object(m) = fallback
        && !is_marker_primitive(fallback)
    {
        return read_string_opt(m.get(key));
    }
    None
}

fn resolve_token_provider_model(row: &Value, fallback: &Value) -> Result<PmSel, String> {
    let stamped = read_string_opt(Some(member(row, "provider")?));
    let fb_provider = fb_str(fallback, "provider");
    let provider = stamped.clone().or_else(|| fb_provider.clone());
    let instance_id = read_string_opt(Some(member(row, "instanceId")?))
        .or_else(|| fb_str(fallback, "instanceId"))
        .or_else(|| provider.clone());
    let model = read_string_opt(Some(member(row, "model")?)).or_else(|| {
        if stamped.is_none() || stamped.as_deref() == fb_provider.as_deref() {
            fb_str(fallback, "model")
        } else {
            None
        }
    });
    Ok(PmSel {
        provider,
        instance_id,
        model,
    })
}

fn token_provider_model_key(sel: &PmSel) -> String {
    format!(
        "{}\u{0}{}\u{0}{}",
        sel.provider.as_deref().unwrap_or(""),
        sel.instance_id.as_deref().unwrap_or(""),
        sel.model.as_deref().unwrap_or("")
    )
}

fn usage_session_key(row: &Value) -> Result<String, String> {
    let v = member(row, "usageSessionId")?;
    Ok(if is_nullish(v) {
        String::new()
    } else {
        js_template_string(v)
    })
}

fn token_counter_key(counter_provider: Option<&str>, usage_session_key: &str) -> String {
    format!("{}\u{0}{usage_session_key}", counter_provider.unwrap_or(""))
}

fn counter_provider_of(row: &Value, fallback: &Value) -> Result<Option<String>, String> {
    let stamped = read_string_opt(Some(member(row, "counterProvider")?));
    Ok(match stamped {
        Some(s) => Some(s),
        None => resolve_token_provider_model(row, fallback)?.provider,
    })
}

fn dispatch_origin_blocks(row: &Value) -> Result<bool, String> {
    let origin = member(row, "dispatchOrigin")?;
    Ok(!is_nullish(origin) && !matches!(origin, Value::String(s) if s == "user"))
}

struct TokenRow {
    created_at: Value,
    sel: PmSel,
    tokens: f64,
}

fn add_token_snapshot_row(
    rows: &mut Vec<TokenRow>,
    index: &mut HashMap<String, usize>,
    row: TokenRow,
) {
    let key = format!(
        "{}\u{0}{}",
        js_template_string(&row.created_at),
        token_provider_model_key(&row.sel)
    );
    match index.get(&key) {
        Some(&i) => rows[i].tokens += row.tokens,
        None => {
            index.insert(key, rows.len());
            rows.push(row);
        }
    }
}

pub fn aggregate_thread_token_rows(
    rows: Value,
    fallback_selection: Value,
) -> Result<Vec<Value>, String> {
    // `rows.filter` — only arrays carry it.
    let row_items: &[Value] = match &rows {
        Value::Array(a) => a,
        Value::Null => {
            return Err("Cannot read properties of null (reading 'filter')".to_string());
        }
        Value::Object(_) if is_undefined(&rows) => {
            return Err("Cannot read properties of undefined (reading 'filter')".to_string());
        }
        _ => return Err("rows.filter is not a function".to_string()),
    };

    let mut non_claude: Vec<&Value> = vec![];
    for row in row_items {
        if resolve_token_provider_model(row, &fallback_selection)?
            .provider
            .as_deref()
            != Some("claudeAgent")
        {
            non_claude.push(row);
        }
    }

    let mut cumulative_provider_models: std::collections::HashSet<String> =
        std::collections::HashSet::new();
    for row in &non_claude {
        if token_counter_value(member(row, "totalProcessedTokens")?).is_none() {
            continue;
        }
        let sel = resolve_token_provider_model(row, &fallback_selection)?;
        let usid = usage_session_key(row)?;
        cumulative_provider_models.insert(format!("{}\u{0}{usid}", token_provider_model_key(&sel)));
    }

    let mut tokens_by_key: Vec<TokenRow> = vec![];
    let mut tokens_index: HashMap<String, usize> = HashMap::new();

    let mut previous_cumulative: HashMap<String, f64> = HashMap::new();
    for row in &non_claude {
        let Some(total) = token_counter_value(member(row, "totalProcessedTokens")?) else {
            continue;
        };
        let counter_provider = counter_provider_of(row, &fallback_selection)?;
        let counter_key = token_counter_key(counter_provider.as_deref(), &usage_session_key(row)?);
        let previous = previous_cumulative.get(&counter_key).copied();
        let delta = match previous {
            None => total,
            Some(prev) if total < prev => total,
            Some(prev) => (total - prev).max(0.0),
        };
        previous_cumulative.insert(counter_key, total);
        let created_at = member(row, "createdAt")?;
        if delta <= 0.0 || created_at.is_null() || dispatch_origin_blocks(row)? {
            continue;
        }
        let sel = resolve_token_provider_model(row, &fallback_selection)?;
        add_token_snapshot_row(
            &mut tokens_by_key,
            &mut tokens_index,
            TokenRow {
                created_at: created_at.clone(),
                sel,
                tokens: delta,
            },
        );
    }

    let mut previous_used: HashMap<String, (f64, String)> = HashMap::new();
    for row in &non_claude {
        let sel = resolve_token_provider_model(row, &fallback_selection)?;
        let provider_model_key = token_provider_model_key(&sel);
        let usid = usage_session_key(row)?;
        if cumulative_provider_models.contains(&format!("{provider_model_key}\u{0}{usid}")) {
            continue;
        }
        let Some(total) = token_counter_value(member(row, "usedTokens")?) else {
            continue;
        };
        let counter_provider = counter_provider_of(row, &fallback_selection)?;
        let counter_key = token_counter_key(counter_provider.as_deref(), &usid);
        let previous = previous_used.get(&counter_key);
        let delta = match previous {
            None => total,
            Some((prev_total, prev_pm_key))
                if total < *prev_total && provider_model_key != *prev_pm_key =>
            {
                total
            }
            Some((prev_total, _)) => (total - prev_total).max(0.0),
        };
        previous_used.insert(counter_key, (total, provider_model_key));
        let created_at = member(row, "createdAt")?;
        if delta <= 0.0 || created_at.is_null() || dispatch_origin_blocks(row)? {
            continue;
        }
        add_token_snapshot_row(
            &mut tokens_by_key,
            &mut tokens_index,
            TokenRow {
                created_at: created_at.clone(),
                sel,
                tokens: delta,
            },
        );
    }

    Ok(tokens_by_key
        .iter()
        .map(|row| {
            let mut obj = serde_json::Map::new();
            if !is_undefined(&row.created_at) {
                obj.insert("createdAt".to_string(), row.created_at.clone());
            }
            obj.insert(
                "provider".to_string(),
                row.sel
                    .provider
                    .clone()
                    .map(Value::String)
                    .unwrap_or(Value::Null),
            );
            obj.insert(
                "instanceId".to_string(),
                row.sel
                    .instance_id
                    .clone()
                    .map(Value::String)
                    .unwrap_or(Value::Null),
            );
            obj.insert(
                "model".to_string(),
                row.sel
                    .model
                    .clone()
                    .map(Value::String)
                    .unwrap_or(Value::Null),
            );
            obj.insert("tokens".to_string(), number(row.tokens));
            Value::Object(obj)
        })
        .collect())
}
