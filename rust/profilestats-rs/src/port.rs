use serde_json::{Value, json};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use regex::Regex;

use crate::js::{as_number, is_undefined, number, number_to_string, sort as js_sort, string_units};

const PROVIDER_KINDS: &[&str] = &[
    "codex",
    "claudeAgent",
    "cursor",
    "antigravity",
    "grok",
    "droid",
    "opencode",
    "pi",
    "devin",
    "omp",
];

const BUILT_IN_COMPOSER_SLASH_COMMANDS: &[&str] = &[
    "clear",
    "compact",
    "model",
    "plan",
    "debug",
    "default",
    "review",
    "fork",
    "side",
    "status",
    "subagents",
    "computer-use",
    "fast",
    "export",
    "goal",
    "rename",
    "feedback",
    "automation",
];

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
}

fn is_js_space(c: char) -> bool {
    c.is_whitespace() || c == '\u{feff}'
}

fn as_js_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        _ => string_units(value).map(|units| String::from_utf16_lossy(&units)),
    }
}

fn non_empty_str(value: &Value) -> Option<String> {
    let s = as_js_string(value)?;
    let trimmed = js_trim(&s);
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn parse_number(s: &str) -> f64 {
    let t = js_trim(s);
    if t.is_empty() {
        return 0.0;
    }
    match t {
        "Infinity" | "+Infinity" | "infinity" | "+infinity" => return f64::INFINITY,
        "-Infinity" | "-infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    let (neg, body) = match t.strip_prefix('-') {
        Some(b) => (true, b),
        None => (false, t.strip_prefix('+').unwrap_or(t)),
    };
    let v = if let Some(h) = body.strip_prefix("0x").or_else(|| body.strip_prefix("0X")) {
        u128::from_str_radix(h, 16)
            .map(|v| v as f64)
            .unwrap_or(f64::NAN)
    } else if let Some(b) = body.strip_prefix("0b").or_else(|| body.strip_prefix("0B")) {
        u128::from_str_radix(b, 2)
            .map(|v| v as f64)
            .unwrap_or(f64::NAN)
    } else if let Some(o) = body.strip_prefix("0o").or_else(|| body.strip_prefix("0O")) {
        u128::from_str_radix(o, 8)
            .map(|v| v as f64)
            .unwrap_or(f64::NAN)
    } else {
        t.parse::<f64>().unwrap_or(f64::NAN)
    };
    if neg { -v } else { v }
}

fn to_number(value: &Value) -> f64 {
    match value {
        Value::Null => 0.0,
        Value::Bool(b) => u8::from(*b) as f64,
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => parse_number(s),
        Value::Array(a) => match a.as_slice() {
            [] => 0.0,
            [single] => to_number(single),
            _ => f64::NAN,
        },
        Value::Object(_) => {
            if is_undefined(value) {
                return f64::NAN;
            }
            if let Some(n) = as_number(value) {
                return n;
            }
            if let Some(s) = as_js_string(value) {
                return parse_number(&s);
            }
            f64::NAN
        }
    }
}

fn js_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64() != Some(0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) => true,
        Value::Object(_) => {
            if is_undefined(v) {
                false
            } else if let Some(x) = as_number(v) {
                x != 0.0 && !x.is_nan()
            } else if let Some(units) = string_units(v) {
                !units.is_empty()
            } else {
                true
            }
        }
    }
}

fn js_display(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::Null => "null".to_string(),
        Value::Array(_) => format!("{v:?}"),
        Value::Object(_) => {
            if let Some(x) = as_number(v) {
                return number_to_string(x);
            }
            if let Some(units) = string_units(v) {
                return String::from_utf16_lossy(&units);
            }
            format!("{v:?}")
        }
    }
}

// `String.prototype.padStart(2, "0")` — pads to a minimum length of 2.
fn pad2(s: &str) -> String {
    if s.chars().count() < 2 {
        format!("0{s}")
    } else {
        s.to_string()
    }
}

pub fn sqlite_modifier_from_utc_offset_minutes(offset_minutes: f64) -> Result<String, String> {
    let safe = if offset_minutes.is_finite() {
        offset_minutes.trunc()
    } else {
        0.0
    };
    let sign = if safe < 0.0 { "-" } else { "+" };
    let abs = safe.abs();
    let hh = pad2(&number_to_string((abs / 60.0).floor()));
    let mm = pad2(&number_to_string(abs % 60.0));
    Ok(format!("{sign}{hh}:{mm}"))
}

pub fn num(value: Value) -> Result<f64, String> {
    let parsed = match &value {
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        _ => to_number(&value),
    };
    Ok(if parsed.is_finite() { parsed } else { 0.0 })
}

pub fn non_empty_string(value: Value) -> Result<Option<String>, String> {
    Ok(non_empty_str(&value))
}

fn normalize_usage_name(value: &Value) -> Option<String> {
    let name = non_empty_str(value)?;
    normalize_usage_name_str(&name)
}

fn normalize_usage_name_str(name: &str) -> Option<String> {
    let rest = js_trim(name.trim_start_matches(['$', '/', '@']));
    (!rest.is_empty()).then(|| rest.to_string())
}

fn usage_key(kind: &str, name: &str) -> String {
    format!("{kind}\u{0}{}", name.to_lowercase())
}

fn usage_kind_sort_order(kind: &str) -> i64 {
    if kind == "skill" { 0 } else { 1 }
}

fn is_obvious_non_skill_dollar_token(name: &str) -> bool {
    match name.chars().next() {
        Some(c) if c.is_ascii_digit() => true,
        Some(c) if c == '_' || c.is_ascii_uppercase() => name
            .chars()
            .all(|c| c == '_' || c.is_ascii_uppercase() || c.is_ascii_digit()),
        _ => false,
    }
}

static TRAILING_BLOCK_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        "pasted_text",
        "file_comments",
        "terminal_context",
        "assistant_selection",
    ]
    .iter()
    .map(|tag| Regex::new(&format!(r"\n*<{tag}>\n[\s\S]*?\n</{tag}>\s*$")).unwrap())
    .collect()
});
static TRAILING_NEWLINES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\n+$").unwrap());

fn strip_profile_trailing_prompt_blocks(prompt: &str) -> String {
    let mut visible = prompt.to_string();
    loop {
        let mut stripped = false;
        for pattern in TRAILING_BLOCK_PATTERNS.iter() {
            let removed = pattern.replacen(&visible, 1, "");
            let next = TRAILING_NEWLINES.replace(&removed, "").into_owned();
            if next != visible {
                visible = next;
                stripped = true;
                break;
            }
        }
        if !stripped {
            return visible;
        }
    }
}

fn parse_reference_names(json: &Value) -> Vec<String> {
    let Some(value) = non_empty_str(json) else {
        return vec![];
    };
    let Ok(parsed) = serde_json::from_str::<Value>(&value) else {
        return vec![];
    };
    let Value::Array(entries) = parsed else {
        return vec![];
    };
    entries
        .iter()
        .filter_map(|entry| match entry {
            Value::Object(m) if m.contains_key("name") => normalize_usage_name(&m["name"]),
            _ => None,
        })
        .collect()
}

fn is_builtin_slash(name: &str) -> bool {
    let normalized = js_trim(name).trim_start_matches('/').to_lowercase();
    BUILT_IN_COMPOSER_SLASH_COMMANDS.contains(&normalized.as_str())
}

fn is_leading_boundary(c: char) -> bool {
    is_js_space(c) || matches!(c, '(' | '[' | '{' | '<')
}

fn is_trailing_boundary(c: char) -> bool {
    is_js_space(c) || matches!(c, '.' | ',' | '!' | '?' | ';' | ')' | ']' | '}' | '>')
}

// `[A-Za-z0-9](?:[A-Za-z0-9_-]*[A-Za-z0-9])?` — longest run prefix ending alphanumeric.
fn name_seg_end(chars: &[char], start: usize) -> Option<usize> {
    if start >= chars.len() || !chars[start].is_ascii_alphanumeric() {
        return None;
    }
    let mut end = start + 1;
    let mut k = start + 1;
    while k < chars.len()
        && (chars[k].is_ascii_alphanumeric() || chars[k] == '_' || chars[k] == '-')
    {
        if chars[k].is_ascii_alphanumeric() {
            end = k + 1;
        }
        k += 1;
    }
    Some(end)
}

// NAME = seg (?::seg)*
fn skill_name_end(chars: &[char], start: usize) -> Option<usize> {
    let mut end = name_seg_end(chars, start)?;
    while end < chars.len() && chars[end] == ':' {
        match name_seg_end(chars, end + 1) {
            Some(e) => end = e,
            None => break,
        }
    }
    Some(end)
}

fn extract_text_skill_names(text: &Value) -> Vec<String> {
    let Some(prompt) = non_empty_str(text) else {
        return vec![];
    };
    let visible = strip_profile_trailing_prompt_blocks(&prompt);
    if js_trim(&visible).is_empty() {
        return vec![];
    }
    let chars: Vec<char> = visible.chars().collect();
    let mut names = vec![];
    let mut i = 0usize;
    while i < chars.len() {
        let prefix = chars[i];
        if prefix != '$' && prefix != '/' {
            i += 1;
            continue;
        }
        if i > 0 && !is_leading_boundary(chars[i - 1]) {
            i += 1;
            continue;
        }
        let advanced = match skill_name_end(&chars, i + 1) {
            Some(end) if end >= chars.len() || is_trailing_boundary(chars[end]) => {
                let raw_name: String = chars[i + 1..end].iter().collect();
                let is_closing_tag = i > 0 && chars[i - 1] == '<' && prefix == '/';
                if !is_closing_tag && !(prefix == '/' && is_builtin_slash(&raw_name)) {
                    let lower = raw_name.to_lowercase();
                    let has_skill_prefix = lower.starts_with("skill:");
                    let normalized = if has_skill_prefix {
                        &raw_name[6..]
                    } else {
                        &raw_name
                    };
                    if let Some(name) = normalize_usage_name_str(normalized)
                        && !(prefix == '$'
                            && !has_skill_prefix
                            && is_obvious_non_skill_dollar_token(&name))
                    {
                        names.push(name);
                    }
                }
                end
            }
            _ => i + 1,
        };
        i = advanced;
    }
    names
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

// `for (const row of value)` — arrays iterate elements, strings iterate code points.
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

struct UsageCount {
    name: String,
    kind: &'static str,
    run_count: f64,
}

fn locale_cmp(a: &str, b: &str) -> Ordering {
    fn class(c: char) -> u8 {
        if c.is_ascii_digit() {
            1
        } else if c.is_ascii_alphabetic() {
            2
        } else {
            0
        }
    }
    let mut ai = a.chars();
    let mut bi = b.chars();
    loop {
        match (ai.next(), bi.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(ca), Some(cb)) => {
                let (cla, clb) = (class(ca), class(cb));
                if cla != clb {
                    return cla.cmp(&clb);
                }
                let (la, lb) = (ca.to_ascii_lowercase(), cb.to_ascii_lowercase());
                if la != lb {
                    return la.cmp(&lb);
                }
                if ca != cb {
                    return ca.cmp(&cb);
                }
            }
        }
    }
}

pub fn aggregate_profile_skill_usage_rows(
    rows: Value,
    archived_rows: Value,
) -> Result<Vec<Value>, String> {
    let mut counts: Vec<UsageCount> = vec![];
    let mut counts_index: HashMap<String, usize> = HashMap::new();

    for row in iter_values(&rows, "rows")? {
        let mut msg_skills: Vec<(String, f64, f64)> = vec![]; // (name, structured, text)
        let mut msg_skill_idx: HashMap<String, usize> = HashMap::new();
        let mut msg_agents: Vec<String> = vec![]; // names
        let mut msg_agent_set: HashSet<String> = HashSet::new();

        let add_skill = |raw_name: &str,
                         structured: bool,
                         msg_skills: &mut Vec<(String, f64, f64)>,
                         idx: &mut HashMap<String, usize>| {
            let Some(name) = normalize_usage_name_str(raw_name) else {
                return;
            };
            let key = usage_key("skill", &name);
            match idx.get(&key) {
                Some(&i) => {
                    if structured {
                        msg_skills[i].1 += 1.0;
                    } else {
                        msg_skills[i].2 += 1.0;
                    }
                }
                None => {
                    idx.insert(key, msg_skills.len());
                    msg_skills.push(if structured {
                        (name, 1.0, 0.0)
                    } else {
                        (name, 0.0, 1.0)
                    });
                }
            }
        };

        let skills_json = member(&row, "skillsJson")?.clone();
        for name in parse_reference_names(&skills_json) {
            add_skill(&name, true, &mut msg_skills, &mut msg_skill_idx);
        }
        let text = member(&row, "text")?.clone();
        for name in extract_text_skill_names(&text) {
            add_skill(&name, false, &mut msg_skills, &mut msg_skill_idx);
        }
        let mentions_json = member(&row, "mentionsJson")?.clone();
        for name in parse_reference_names(&mentions_json) {
            if let Some(name) = normalize_usage_name(&json!(name)) {
                let key = usage_key("agent", &name);
                if msg_agent_set.insert(key) {
                    msg_agents.push(name);
                }
            }
        }

        for (name, structured, text_count) in msg_skills {
            let increment = structured.max(text_count);
            if increment <= 0.0 {
                continue;
            }
            let key = usage_key("skill", &name);
            match counts_index.get(&key) {
                Some(&i) => counts[i].run_count += increment,
                None => {
                    counts_index.insert(key, counts.len());
                    counts.push(UsageCount {
                        name,
                        kind: "skill",
                        run_count: increment,
                    });
                }
            }
        }
        for name in msg_agents {
            let key = usage_key("agent", &name);
            match counts_index.get(&key) {
                Some(&i) => counts[i].run_count += 1.0,
                None => {
                    counts_index.insert(key, counts.len());
                    counts.push(UsageCount {
                        name,
                        kind: "agent",
                        run_count: 1.0,
                    });
                }
            }
        }
    }

    let archived = if is_undefined(&archived_rows) {
        vec![]
    } else {
        iter_values(&archived_rows, "archivedRows")?
    };
    for row in archived {
        let name = normalize_usage_name(member(&row, "name")?);
        let kind_v = member(&row, "kind")?;
        let kind: Option<&'static str> = match kind_v {
            Value::String(s) if s == "skill" => Some("skill"),
            Value::String(s) if s == "agent" => Some("agent"),
            _ => None,
        };
        let run_count = num(member(&row, "runCount")?.clone())?.trunc();
        let (Some(name), Some(kind)) = (name, kind) else {
            continue;
        };
        if run_count <= 0.0 {
            continue;
        }
        let key = usage_key(kind, &name);
        match counts_index.get(&key) {
            Some(&i) => counts[i].run_count += run_count,
            None => {
                counts_index.insert(key, counts.len());
                counts.push(UsageCount {
                    name,
                    kind,
                    run_count,
                });
            }
        }
    }

    js_sort(&mut counts, |l, r| {
        let d = r.run_count - l.run_count;
        if d != 0.0 && !d.is_nan() {
            d
        } else {
            let k = (usage_kind_sort_order(l.kind) - usage_kind_sort_order(r.kind)) as f64;
            if k != 0.0 {
                k
            } else {
                match locale_cmp(&l.name, &r.name) {
                    Ordering::Less => -1.0,
                    Ordering::Equal => 0.0,
                    Ordering::Greater => 1.0,
                }
            }
        }
    });

    Ok(counts
        .iter()
        .map(|u| {
            json!({
                "name": u.name,
                "displayName": format!("{}{}", if u.kind == "skill" { "$" } else { "@" }, u.name),
                "kind": u.kind,
                "runCount": number(u.run_count),
            })
        })
        .collect())
}

const HEATMAP_LEVELS: f64 = 4.0;

pub fn heatmap_intensity(count: f64, sorted_active_counts: Vec<f64>) -> Result<f64, String> {
    if count <= 0.0 || sorted_active_counts.is_empty() {
        return Ok(0.0);
    }
    let mut low = 0usize;
    let mut high = sorted_active_counts.len();
    while low < high {
        let mid = (low + high) / 2;
        if sorted_active_counts[mid] <= count {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    let level = (low as f64 * HEATMAP_LEVELS / sorted_active_counts.len() as f64).ceil();
    Ok(level.clamp(1.0, HEATMAP_LEVELS))
}

fn infer_kind_from_instance(instance_id: &str) -> Option<&'static str> {
    let lower = instance_id.to_lowercase();
    if lower.starts_with("claude") {
        Some("claudeAgent")
    } else if lower.starts_with("codex") {
        Some("codex")
    } else if lower.starts_with("cursor") {
        Some("cursor")
    } else if lower.starts_with("antigravity")
        || lower.starts_with("gemini")
        || lower.starts_with("agy")
    {
        Some("antigravity")
    } else if lower.starts_with("grok") {
        Some("grok")
    } else if lower.starts_with("droid") {
        Some("droid")
    } else if lower.starts_with("opencode")
        || lower.starts_with("open_code")
        || lower.starts_with("kilo")
    {
        Some("opencode")
    } else if lower.starts_with("pi") {
        Some("pi")
    } else if lower.starts_with("devin") {
        Some("devin")
    } else {
        None
    }
}

fn infer_kind_from_model(model: &str) -> &'static str {
    let lower = model.to_lowercase();
    if lower.contains("claude")
        || lower.contains("sonnet")
        || lower.contains("opus")
        || lower.contains("haiku")
    {
        "claudeAgent"
    } else if lower.contains("gemini") || lower.contains("antigravity") {
        "antigravity"
    } else if lower.contains("grok") {
        "grok"
    } else if lower.contains("droid") {
        "droid"
    } else if lower.contains("devin") {
        "devin"
    } else if lower.contains("opencode") || lower.contains("open_code") || lower.contains("kilo") {
        "opencode"
    } else if lower.contains("cursor") {
        "cursor"
    } else if lower.starts_with("pi/") || lower.contains("/pi/") {
        "pi"
    } else {
        "codex"
    }
}

pub fn normalize_provider_kind(value: Value, model: Option<Value>) -> Result<Value, String> {
    if let Some(provider) = non_empty_str(&value) {
        if PROVIDER_KINDS.contains(&provider.as_str()) {
            return Ok(json!(provider));
        }
        if let Some(inferred) = infer_kind_from_instance(&provider) {
            return Ok(json!(inferred));
        }
    }
    let model_value = model.as_ref().and_then(non_empty_str);
    Ok(json!(match model_value {
        Some(m) => infer_kind_from_model(&m),
        None => "unknown",
    }))
}

fn sql_literal(sql: &Value) -> Result<Value, String> {
    if sql.is_null() {
        return Err("Cannot read properties of null (reading 'literal')".to_string());
    }
    if is_undefined(sql) {
        return Err("Cannot read properties of undefined (reading 'literal')".to_string());
    }
    Err("sql.literal is not a function".to_string())
}

fn has_in(scope: &Value, key: &str) -> Result<bool, String> {
    match scope {
        Value::Object(m) => Ok(m.contains_key(key)),
        Value::Array(_) => Ok(false),
        _ => Err(format!(
            "Cannot use 'in' operator to search for '{key}' in {}",
            js_display(scope)
        )),
    }
}

pub fn turn_model_selection_cte(sql: Value, scope: Option<Value>) -> Result<Value, String> {
    let scope = scope.unwrap_or_else(crate::js::undefined);
    if js_truthy(&scope) && has_in(&scope, "threadId")? {
        // `sql\`${scope.threadId}\`` — tagged-template call on a non-function.
        return Err("sql is not a function".to_string());
    }
    sql_literal(&sql)?;
    if !js_truthy(&scope) {
        sql_literal(&sql)?;
    } else {
        // scope truthy here means has_in already evaluated false at the first check.
        return Err("sql is not a function".to_string());
    }
    Err("sql is not a function".to_string())
}

pub fn user_prompt_events_query(sql: Value, range: Option<Value>) -> Result<Value, String> {
    let range = range.unwrap_or_else(crate::js::undefined);
    if js_truthy(&range) {
        // `sql\`AND m.created_at >= ...\`` — tagged-template call on a non-function.
        return Err("sql is not a function".to_string());
    }
    sql_literal(&sql)?;
    sql_literal(&sql)?;
    Err("sql is not a function".to_string())
}

pub fn token_delta_ctes(sql: Value, _scope: Option<Value>) -> Result<Value, String> {
    sql_literal(&sql)
}
