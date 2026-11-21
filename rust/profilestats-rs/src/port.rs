//! The Rust port of `apps/server/src/profileStats.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

const HEATMAP_LEVELS: f64 = 4.0;

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

const BUILT_IN_SLASH_COMMANDS: &[&str] = &[
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

fn js_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

fn is_fn(v: &Value) -> bool {
    v.get("$function").is_some()
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
}

fn non_empty_str(v: &Value) -> Option<String> {
    v.as_str()
        .map(js_trim)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
}

fn js_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        _ => !js_undefined(v),
    }
}

/// JS `Number(value)` — object/array/null/string coercion.
fn js_to_number(v: &Value) -> f64 {
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => js_string_to_number(s),
        Value::Array(a) => {
            if a.is_empty() {
                0.0
            } else if a.len() == 1 {
                js_to_number(&a[0])
            } else {
                f64::NAN
            }
        }
        Value::Object(_) => f64::NAN,
    }
}

fn js_string_to_number(s: &str) -> f64 {
    let t = js_trim(s);
    if t.is_empty() {
        return 0.0;
    }
    match t {
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        return u64::from_str_radix(hex, 16)
            .map(|u| u as f64)
            .unwrap_or(f64::NAN);
    }
    t.parse::<f64>().unwrap_or(f64::NAN)
}

fn prop(v: &Value, key: &str) -> Value {
    match v {
        Value::Object(o) => o.get(key).cloned().unwrap_or_else(crate::js::undefined),
        _ => crate::js::undefined(),
    }
}

fn normalize_usage_name(v: &Value) -> Option<String> {
    let name = non_empty_str(v)?;
    let stripped = name.trim_start_matches(['$', '/', '@']);
    let t = js_trim(stripped);
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

fn usage_key(kind: &str, name: &str) -> String {
    format!("{kind}\u{0}{}", name.to_lowercase())
}

/// `parseReferenceNames` — JSON array of objects with a `name` prop.
fn parse_reference_names(json: &Value) -> Vec<String> {
    let Some(text) = non_empty_str(json) else {
        return vec![];
    };
    let Ok(parsed) = serde_json::from_str::<Value>(&text) else {
        return vec![];
    };
    let Value::Array(entries) = parsed else {
        return vec![];
    };
    entries
        .iter()
        .filter_map(|e| {
            if e.is_object() {
                normalize_usage_name(&prop(e, "name"))
            } else {
                None
            }
        })
        .collect()
}

fn is_builtin_slash_command(raw_name: &str) -> bool {
    let normalized = js_trim(raw_name).trim_start_matches('/').to_lowercase();
    BUILT_IN_SLASH_COMMANDS.iter().any(|c| *c == normalized)
}

fn is_obvious_non_skill_dollar_token(name: &str) -> bool {
    let mut chars = name.chars();
    if chars.next().is_some_and(|c| c.is_ascii_digit()) {
        return true;
    }
    // /^[A-Z_][A-Z0-9_]*$/
    let mut it = name.chars();
    match it.next() {
        Some(c) if c.is_ascii_uppercase() || c == '_' => {}
        _ => return false,
    }
    it.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// `(^|[\s([{<])([$/])(NAME)(?=$|[\s.,!?;)\]}>])` — implemented by manual scan
/// so capture groups keep the JS semantics.
fn extract_text_skill_names(text: &Value) -> Vec<String> {
    let Some(prompt) = non_empty_str(text) else {
        return vec![];
    };
    let visible = strip_trailing_prompt_blocks(&prompt);
    if js_trim(&visible).is_empty() {
        return vec![];
    }
    let mut names = Vec::new();
    let chars: Vec<char> = visible.chars().collect();
    let n = chars.len();
    let is_boundary_before = |i: usize| -> bool {
        if i == 0 {
            return true;
        }
        let c = chars[i - 1];
        c.is_whitespace() || matches!(c, '(' | '[' | '{' | '<')
    };
    let is_boundary_after = |i: usize| -> bool {
        if i >= n {
            return true;
        }
        let c = chars[i];
        c.is_whitespace() || matches!(c, '.' | ',' | '!' | '?' | ';' | ')' | ']' | '}' | '>')
    };
    let mut i = 0;
    while i < n {
        let c = chars[i];
        if (c == '$' || c == '/') && is_boundary_before(i) {
            let mut j = i + 1;
            let mut raw = String::new();
            // NAME = seg ( ':' seg )* ; seg = alnum (alnum|-|_)* ending on alnum.
            while j < n && chars[j].is_ascii_alphanumeric() {
                let seg_start = j;
                while j < n
                    && (chars[j].is_ascii_alphanumeric() || chars[j] == '-' || chars[j] == '_')
                {
                    j += 1;
                }
                let mut seg_end = j;
                while seg_end > seg_start + 1 && matches!(chars[seg_end - 1], '-' | '_') {
                    seg_end -= 1;
                }
                j = seg_end;
                raw.push_str(&chars[seg_start..seg_end].iter().collect::<String>());
                if j < n && chars[j] == ':' && j + 1 < n && chars[j + 1].is_ascii_alphanumeric() {
                    raw.push(':');
                    j += 1;
                    continue;
                }
                break;
            }
            let ok = !raw.is_empty();
            if ok && is_boundary_after(j) {
                let leading = if i == 0 { '\0' } else { chars[i - 1] };
                if !(leading == '<' && c == '/') && (c != '/' || !is_builtin_slash_command(&raw)) {
                    let has_skill_prefix = raw.to_lowercase().starts_with("skill:");
                    let normalized = if has_skill_prefix {
                        raw["skill:".len()..].to_string()
                    } else {
                        raw.clone()
                    };
                    if let Some(name) = normalize_usage_name(&Value::String(normalized))
                        && !(c == '$'
                            && !has_skill_prefix
                            && is_obvious_non_skill_dollar_token(&name))
                    {
                        names.push(name);
                    }
                }
            }
        }
        i += 1;
    }
    names
}

/// `stripProfileTrailingPromptBlocks` — repeatedly remove trailing
/// `<pasted_text>`/`<file_comments>`/`<terminal_context>`/`<assistant_selection>` blocks.
fn strip_trailing_prompt_blocks(prompt: &str) -> String {
    const TAGS: &[&str] = &[
        "pasted_text",
        "file_comments",
        "terminal_context",
        "assistant_selection",
    ];
    let mut out = prompt.to_string();
    loop {
        let mut stripped = false;
        for tag in TAGS {
            let open = format!("<{tag}>\n");
            let close = format!("\n</{tag}>");
            let Some(open_idx) = find_trailing_block(&out, &open, &close) else {
                continue;
            };
            let mut next = out[..open_idx].to_string();
            // `.replace(pattern,"")` drops `\n*` before the tag too, then `\n+$` trailing.
            let trimmed = next.trim_end_matches('\n');
            next = trimmed.to_string();
            if next != out {
                out = next;
                stripped = true;
                break;
            }
        }
        if !stripped {
            return out;
        }
    }
}

/// Locate the last `...<tag>\n[anything]\n</tag>\s*$` block in `s`.
fn find_trailing_block(s: &str, open: &str, close: &str) -> Option<usize> {
    // Close tag must appear, followed only by whitespace, at the very end.
    let trimmed_end = s.len()
        - s.chars()
            .rev()
            .take_while(|c| c.is_whitespace())
            .map(char::len_utf8)
            .sum::<usize>();
    let close_idx = s[..trimmed_end].rfind(close)?;
    if close_idx + close.len() != trimmed_end {
        return None;
    }
    // Open tag: the last `<tag>\n` before the close tag; `\n*` before it are part of the match.
    let open_idx = s[..close_idx].rfind(open)?;
    let mut start = open_idx;
    let bytes = s.as_bytes();
    while start > 0 && bytes[start - 1] == b'\n' {
        start -= 1;
    }
    Some(start)
}

// ── Ported fns ──────────────────────────────────────────────────────────

/// Port of `sqliteModifierFromUtcOffsetMinutes` (`apps/server/src/profileStats.ts:116`).
pub fn sqlite_modifier_from_utc_offset_minutes(offset_minutes: f64) -> Result<String, String> {
    let safe = if offset_minutes.is_finite() {
        offset_minutes.trunc()
    } else {
        0.0
    };
    let sign = if safe < 0.0 { "-" } else { "+" };
    let abs = safe.abs();
    let hh = crate::js::number_to_string((abs / 60.0).floor());
    let mm = crate::js::number_to_string(abs % 60.0);
    Ok(format!("{sign}{hh:0>2}:{mm:0>2}"))
}

/// Port of `num` (`apps/server/src/profileStats.ts:129`).
pub fn num(value: serde_json::Value) -> Result<f64, String> {
    let n = match &value {
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        other => js_to_number(other),
    };
    Ok(if n.is_finite() { n } else { 0.0 })
}

/// Port of `nonEmptyString` (`apps/server/src/profileStats.ts:140`).
pub fn non_empty_string(value: serde_json::Value) -> Result<Option<String>, String> {
    Ok(non_empty_str(&value))
}

/// Port of `aggregateProfileSkillUsageRows` (`apps/server/src/profileStats.ts:263`).
pub fn aggregate_profile_skill_usage_rows(
    rows: serde_json::Value,
    archived_rows: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    fn iterable(v: &Value) -> Vec<Value> {
        match v {
            Value::Array(a) => a.clone(),
            Value::String(s) => s
                .encode_utf16()
                .map(|u| {
                    Value::String(char::from_u32(u as u32).map_or(String::new(), |c| c.to_string()))
                })
                .collect(),
            _ => vec![],
        }
    }
    if !matches!(rows, Value::Array(_) | Value::String(_)) {
        return Err("rows is not iterable".to_string());
    }
    if !js_undefined(&archived_rows) && !matches!(archived_rows, Value::Array(_) | Value::String(_))
    {
        return Err("archivedRows is not iterable".to_string());
    }

    #[derive(Clone)]
    struct Usage {
        name: String,
        kind: &'static str,
        run_count: f64,
    }
    let mut order: Vec<String> = Vec::new();
    let mut counts = std::collections::HashMap::<String, Usage>::new();

    for row in iterable(&rows) {
        // messageId unused beyond identity; text, skillsJson, mentionsJson drive it.
        let text = prop(&row, "text");
        let skills_json = prop(&row, "skillsJson");
        let mentions_json = prop(&row, "mentionsJson");

        let mut skill_counts = std::collections::HashMap::<String, (String, f64, f64)>::new();
        let mut skill_order: Vec<String> = Vec::new();
        let mut agent_usages = std::collections::HashMap::<String, String>::new();
        let mut agent_order: Vec<String> = Vec::new();

        let mut add_skill = |raw: &Value, structured: bool| {
            if let Some(name) = normalize_usage_name(raw) {
                let key = usage_key("skill", &name);
                let entry = skill_counts.entry(key.clone()).or_insert_with(|| {
                    skill_order.push(key.clone());
                    (name.clone(), 0.0, 0.0)
                });
                if structured {
                    entry.1 += 1.0;
                } else {
                    entry.2 += 1.0;
                }
            }
        };
        for name in parse_reference_names(&skills_json) {
            add_skill(&Value::String(name), true);
        }
        for name in extract_text_skill_names(&text) {
            add_skill(&Value::String(name), false);
        }
        for name in parse_reference_names(&mentions_json) {
            if let Some(name) = normalize_usage_name(&Value::String(name)) {
                let key = usage_key("agent", &name);
                if agent_usages.insert(key.clone(), name).is_none() {
                    agent_order.push(key);
                }
            }
        }
        for key in skill_order {
            let (name, structured, text) = &skill_counts[&key];
            let increment = structured.max(*text);
            if increment <= 0.0 {
                continue;
            }
            let entry = counts.entry(key.clone()).or_insert_with(|| {
                order.push(key.clone());
                Usage {
                    name: name.clone(),
                    kind: "skill",
                    run_count: 0.0,
                }
            });
            entry.run_count += increment;
        }
        for key in agent_order {
            let name = &agent_usages[&key];
            let entry = counts.entry(key.clone()).or_insert_with(|| {
                order.push(key.clone());
                Usage {
                    name: name.clone(),
                    kind: "agent",
                    run_count: 0.0,
                }
            });
            entry.run_count += 1.0;
        }
    }

    for row in iterable(&archived_rows) {
        let name = normalize_usage_name(&prop(&row, "name"));
        let kind_v = prop(&row, "kind");
        let kind = match kind_v.as_str() {
            Some("skill") => Some("skill"),
            Some("agent") => Some("agent"),
            _ => None,
        };
        let run_count = num(prop(&row, "runCount"))?.trunc();
        let (Some(name), Some(kind)) = (name, kind) else {
            continue;
        };
        if run_count <= 0.0 {
            continue;
        }
        let key = usage_key(kind, &name);
        let entry = counts.entry(key.clone()).or_insert_with(|| {
            order.push(key.clone());
            Usage {
                name: name.clone(),
                kind,
                run_count: 0.0,
            }
        });
        entry.run_count += run_count;
    }

    let mut usages: Vec<Usage> = order.iter().map(|k| counts[k].clone()).collect();
    usages.sort_by(|a, b| {
        b.run_count
            .total_cmp(&a.run_count)
            .then((a.kind == "agent").cmp(&(b.kind == "agent")))
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(usages
        .into_iter()
        .map(|u| {
            serde_json::json!({
                "name": u.name,
                "displayName": format!("{}{}", if u.kind == "skill" { "$" } else { "@" }, u.name),
                "kind": u.kind,
                "runCount": u.run_count,
            })
        })
        .collect())
}

/// Port of `heatmapIntensity` (`apps/server/src/profileStats.ts:391`).
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
    let level = ((low as f64 * HEATMAP_LEVELS) / sorted_active_counts.len() as f64).ceil();
    Ok(level.clamp(1.0, HEATMAP_LEVELS))
}

/// `inferLegacyProviderKindFromInstanceId` (packages/shared/providerInstances.ts).
fn infer_kind_from_instance_id(instance_id: &str) -> Option<&'static str> {
    if PROVIDER_KINDS.contains(&instance_id) {
        return Some(match instance_id {
            "codex" => "codex",
            "claudeAgent" => "claudeAgent",
            "cursor" => "cursor",
            "antigravity" => "antigravity",
            "grok" => "grok",
            "droid" => "droid",
            "opencode" => "opencode",
            "pi" => "pi",
            "devin" => "devin",
            "omp" => "omp",
            _ => unreachable!(),
        });
    }
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

/// `inferLegacyProviderKindFromModel` (packages/shared/providerInstances.ts).
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

/// Port of `normalizeProviderKind` (`apps/server/src/profileStats.ts:451`).
pub fn normalize_provider_kind(
    value: serde_json::Value,
    model: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    if let Some(provider) = non_empty_str(&value) {
        if PROVIDER_KINDS.contains(&provider.as_str()) {
            return Ok(Value::String(provider));
        }
        if let Some(kind) = infer_kind_from_instance_id(&provider) {
            return Ok(Value::String(kind.to_string()));
        }
    }
    let model_value = model.as_ref().and_then(non_empty_str);
    Ok(Value::String(
        model_value
            .map(|m| infer_kind_from_model(&m).to_string())
            .unwrap_or_else(|| "unknown".to_string()),
    ))
}

fn sql_display(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        Value::Array(a) => a.iter().map(sql_display).collect::<Vec<_>>().join(","),
        _ => "".to_string(),
    }
}

/// `sql.literal(x)` — sql must be an object/function with a callable `literal`.
fn check_sql_literal(sql: &Value) -> Result<(), String> {
    if sql.is_null() {
        return Err("Cannot read properties of null (reading 'literal')".to_string());
    }
    if js_undefined(sql) {
        return Err("Cannot read properties of undefined (reading 'literal')".to_string());
    }
    if !is_fn(&prop(sql, "literal")) {
        return Err("sql.literal is not a function".to_string());
    }
    Ok(())
}

fn check_sql_fn(sql: &Value) -> Result<(), String> {
    if !is_fn(sql) {
        return Err("sql is not a function".to_string());
    }
    Ok(())
}

/// `"threadId" in scope` (evaluated only when scope is truthy).
fn thread_id_in_scope(scope: &Value) -> Result<bool, String> {
    match scope {
        Value::Object(o) => Ok(o.contains_key("threadId")),
        Value::Array(_) => Ok(false),
        _ => Err(format!(
            "Cannot use 'in' operator to search for 'threadId' in {}",
            sql_display(scope)
        )),
    }
}

/// `turnModelSelectionCte` shared error path.
fn turn_model_selection(sql: &Value, scope: &Value) -> Result<serde_json::Value, String> {
    let has_thread = js_truthy(scope) && !js_undefined(scope);
    if has_thread {
        let has = thread_id_in_scope(scope)?;
        if has {
            check_sql_fn(sql)?;
        } else {
            check_sql_literal(sql)?;
        }
    } else {
        check_sql_literal(sql)?;
    }
    // eventThreadScope
    if !has_thread {
        check_sql_literal(sql)?;
    } else {
        let _ = thread_id_in_scope(scope)?;
        check_sql_fn(sql)?;
    }
    check_sql_fn(sql)?;
    Ok(crate::js::undefined())
}

/// Port of `turnModelSelectionCte` (`apps/server/src/profileStats.ts:620`).
pub fn turn_model_selection_cte(
    sql: serde_json::Value,
    scope: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let scope = scope.unwrap_or_else(crate::js::undefined);
    turn_model_selection(&sql, &scope)
}

/// Port of `userPromptEventsQuery` (`apps/server/src/profileStats.ts:666`).
pub fn user_prompt_events_query(
    sql: serde_json::Value,
    range: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let range = range.unwrap_or_else(crate::js::undefined);
    let has_range = js_truthy(&range) && !js_undefined(&range);
    if has_range {
        check_sql_fn(&sql)?; // liveRange: sql`AND m.created_at ...`
    } else {
        check_sql_literal(&sql)?; // liveRange: sql.literal("")
    }
    if has_range {
        check_sql_fn(&sql)?; // archivedRange
    } else {
        check_sql_literal(&sql)?;
    }
    check_sql_fn(&sql)?; // final template
    Ok(crate::js::undefined())
}

/// Port of `tokenDeltaCtes` (`apps/server/src/profileStats.ts:722`).
pub fn token_delta_ctes(
    sql: serde_json::Value,
    scope: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let scope = scope.unwrap_or_else(crate::js::undefined);
    check_sql_literal(&sql)?; // deltaOrder = sql.literal(...)
    turn_model_selection(&sql, &scope)?;
    check_sql_fn(&sql)?; // final template
    Ok(crate::js::undefined())
}
