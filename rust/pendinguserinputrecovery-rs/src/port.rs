//! The Rust port of `apps/web/src/pendingUserInputRecovery.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value};

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}')
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

fn canonical_index(k: &str) -> Option<u64> {
    if k.is_empty() || k.len() > 10 {
        return None;
    }
    if !k.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let n: u64 = k.parse().ok()?;
    if n >= 4_294_967_295 || n.to_string() != k {
        return None;
    }
    Some(n)
}

fn js_key_order(map: &Map<String, Value>) -> Vec<String> {
    let mut ints: Vec<u64> = map.keys().filter_map(|k| canonical_index(k)).collect();
    ints.sort_unstable();
    let mut keys: Vec<String> = ints.iter().map(|n| n.to_string()).collect();
    for k in map.keys() {
        if canonical_index(k).is_none() {
            keys.push(k.clone());
        }
    }
    keys
}

fn is_trimmed_non_empty(v: &Value) -> bool {
    match v {
        Value::String(s) => !s.is_empty() && js_trim(s) == s,
        _ => false,
    }
}

fn is_question(v: &Value) -> bool {
    let Value::Object(_) = v else { return false };
    for key in ["id", "header", "question"] {
        match member(v, key) {
            Ok(val) if is_trimmed_non_empty(&val) => {}
            _ => return false,
        }
    }
    let options_ok = match member(v, "options") {
        Ok(Value::Array(opts)) => opts.iter().all(|o| {
            member(o, "label")
                .map(|v| is_trimmed_non_empty(&v))
                .unwrap_or(false)
                && member(o, "description")
                    .map(|v| is_trimmed_non_empty(&v))
                    .unwrap_or(false)
        }),
        _ => false,
    };
    options_ok
        && match member(v, "multiSelect") {
            Ok(Value::Bool(_)) | Err(_) => true,
            Ok(v) => is_undefined(&v),
        }
}

fn is_answer(v: &Value) -> bool {
    let Value::Object(_) = v else { return false };
    match member(v, "customAnswer") {
        Ok(Value::String(_)) | Err(_) => {}
        Ok(v) if is_undefined(&v) => {}
        _ => return false,
    }
    match member(v, "selectedOptionLabels") {
        Ok(Value::Array(a)) => a.iter().all(|e| matches!(e, Value::String(_))),
        Ok(v) if is_undefined(&v) => true,
        Err(_) => true,
        _ => false,
    }
}

fn instance_key(request_id: &str, generation: &Value) -> String {
    let g = if let Value::String(s) = generation {
        s.as_str()
    } else {
        "legacy"
    };
    format!("{request_id}\u{0}{g}")
}

fn stored_draft(draft: &Value) -> bool {
    let Value::Object(_) = draft else {
        return false;
    };
    let request = match member(draft, "request") {
        Ok(Value::Object(_)) => member(draft, "request").unwrap_or(Value::Null),
        _ => return false,
    };
    if !match member(&request, "requestId") {
        Ok(v) => is_trimmed_non_empty(&v),
        Err(_) => false,
    } {
        return false;
    }
    if !match member(&request, "lifecycleGeneration") {
        Ok(Value::String(_)) | Err(_) => true,
        Ok(v) => is_undefined(&v),
    } {
        return false;
    }
    if !matches!(member(&request, "createdAt"), Ok(Value::String(_))) {
        return false;
    }
    if !match member(&request, "questions") {
        Ok(Value::Array(qs)) => qs.iter().all(is_question),
        _ => false,
    } {
        return false;
    }
    match member(draft, "answers") {
        Ok(Value::Object(answers)) => answers.values().all(is_answer),
        _ => false,
    }
}

/// Port of `normalizePendingUserInputDrafts`
/// (`apps/web/src/pendingUserInputRecovery.ts:39`).
pub fn normalize_pending_user_input_drafts(
    value: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let Value::Object(map) = &value else {
        return Ok(Value::Object(Map::new()));
    };
    let mut out = Map::new();
    for key in js_key_order(map) {
        let draft = &map[&key];
        if !stored_draft(draft) {
            continue;
        }
        let request = member(draft, "request")?;
        let request_id = member(&request, "requestId")?;
        let generation = member(&request, "lifecycleGeneration")?;
        let id_str = request_id.as_str().unwrap_or("");
        if key != instance_key(id_str, &generation) {
            continue;
        }
        let mut answers = Map::new();
        if let Ok(Value::Object(stored_answers)) = member(draft, "answers") {
            for id in js_key_order(&stored_answers) {
                let answer = &stored_answers[&id];
                let mut a = Map::new();
                if let Ok(Value::String(s)) = member(answer, "customAnswer") {
                    a.insert("customAnswer".to_string(), Value::String(s));
                }
                if let Ok(Value::Array(labels)) = member(answer, "selectedOptionLabels") {
                    a.insert("selectedOptionLabels".to_string(), Value::Array(labels));
                }
                answers.insert(id, Value::Object(a));
            }
        }
        let mut entry = Map::new();
        entry.insert("request".to_string(), request);
        entry.insert("answers".to_string(), Value::Object(answers));
        out.insert(key, Value::Object(entry));
    }
    Ok(Value::Object(out))
}

fn is_stale_detail(detail: &str) -> bool {
    let d = detail.to_lowercase();
    [
        "stale pending approval request",
        "stale pending user-input request",
        "unknown pending approval request",
        "unknown pending permission request",
        "unknown pending user-input request",
        "stale pending user input request",
        "unknown pending user input request",
    ]
    .iter()
    .any(|s| d.contains(s))
}

/// Port of `expiredUserInputDrafts` (`apps/web/src/pendingUserInputRecovery.ts:66`).
pub fn expired_user_input_drafts(
    drafts: serde_json::Value,
    activities: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let activity_items: Vec<Value> = match &activities {
        Value::Array(a) => a.clone(),
        Value::String(s) => s.chars().map(|c| Value::String(c.to_string())).collect(),
        _ => return Err("activities is not iterable".to_string()),
    };
    let mut stale_at: Vec<(String, String)> = Vec::new();
    for activity in &activity_items {
        let kind = match member(activity, "kind") {
            Ok(Value::String(k)) if k == "provider.approval.respond.failed" => "approval",
            Ok(Value::String(k)) if k == "provider.user-input.respond.failed" => "userInput",
            _ => continue,
        };
        let payload = match member(activity, "payload") {
            Ok(Value::Object(p)) => p,
            _ => continue,
        };
        let request_id = match payload.get("requestId") {
            Some(Value::String(s)) => s.clone(),
            _ => continue,
        };
        let detail_ok = match payload.get("detail") {
            Some(Value::String(d)) => is_stale_detail(d),
            _ => false,
        };
        if !detail_ok {
            continue;
        }
        let generation = match payload.get("lifecycleGeneration") {
            Some(Value::String(s)) if !s.is_empty() => s.clone(),
            _ => "legacy".to_string(),
        };
        let key = format!("{kind}\u{0}{request_id}\u{0}{generation}");
        let created = member(activity, "createdAt")?;
        let created = match created {
            Value::String(s) => s,
            _ => continue,
        };
        match stale_at.iter_mut().find(|(k, _)| *k == key) {
            Some((_, prev)) => {
                if created > *prev {
                    *prev = created;
                }
            }
            None => stale_at.push((key, created)),
        }
    }
    let stale_get =
        |key: &str| -> Option<&String> { stale_at.iter().find(|(k, _)| k == key).map(|(_, v)| v) };
    let entries: Vec<(String, Value)> = match &drafts {
        Value::Object(m) => js_key_order(m)
            .iter()
            .map(|k| (k.clone(), m[k].clone()))
            .collect(),
        Value::Null => return Err("Cannot convert undefined or null to object".to_string()),
        v if is_undefined(v) => {
            return Err("Cannot convert undefined or null to object".to_string());
        }
        Value::Array(a) => a
            .iter()
            .enumerate()
            .map(|(i, v)| (i.to_string(), v.clone()))
            .collect(),
        Value::String(s) => s
            .chars()
            .enumerate()
            .map(|(i, c)| (i.to_string(), Value::String(c.to_string())))
            .collect(),
        _ => Vec::new(),
    };
    let mut out = Vec::new();
    for (key, draft) in entries {
        let request = member(&draft, "request").unwrap_or(Value::Null);
        let request_id = match &request {
            Value::Object(m) => m.get("requestId").cloned().unwrap_or(Value::Null),
            _ => Value::Null,
        };
        let created_at = match &request {
            Value::Object(m) => m.get("createdAt").cloned().unwrap_or(Value::Null),
            _ => Value::Null,
        };
        let generation = match &request {
            Value::Object(m) => m.get("lifecycleGeneration").cloned().unwrap_or(Value::Null),
            _ => Value::Null,
        };
        let rid_str = js_key_string(&request_id);
        let gen_str = match &generation {
            Value::String(s) => s.clone(),
            Value::Null => "legacy".to_string(),
            v if is_undefined(v) => "legacy".to_string(),
            _ => js_key_string(&generation),
        };
        let stale = if !generation.is_null() && !is_undefined(&generation) {
            stale_get(&format!("userInput\u{0}{rid_str}\u{0}{gen_str}")).is_some()
        } else {
            match stale_get(&format!("userInput\u{0}{rid_str}\u{0}legacy")) {
                Some(at) => match &created_at {
                    Value::String(c) => at >= c,
                    _ => at.as_str() >= js_key_string(&created_at).as_str(),
                },
                None => false,
            }
        };
        if stale {
            out.push(Value::Array(vec![Value::String(key), draft]));
        }
    }
    Ok(Value::Array(out))
}

fn js_key_string(v: &Value) -> String {
    if let Some(n) = crate::js::as_number(v) {
        return crate::js::number_to_string(n);
    }
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Array(a) => a
            .iter()
            .map(|e| match e {
                Value::Null => String::new(),
                x if is_undefined(x) => String::new(),
                x => js_key_string(x),
            })
            .collect::<Vec<_>>()
            .join(","),
        v if is_undefined(v) => "undefined".to_string(),
        _ => "[object Object]".to_string(),
    }
}

fn normalize_selected(v: &Value) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    if let Value::Array(a) = v {
        for e in a {
            if let Value::String(s) = e {
                let t = js_trim(s);
                if !t.is_empty() && !seen.iter().any(|x| x == t) {
                    seen.push(t.to_string());
                }
            }
        }
    }
    seen
}

/// Port of `restoreUserInputDraft` (`apps/web/src/pendingUserInputRecovery.ts:76`).
pub fn restore_user_input_draft(
    prompt: String,
    draft: serde_json::Value,
) -> Result<String, String> {
    let request = member(&draft, "request")?;
    let questions = member(&request, "questions")?;
    let answers = member(&draft, "answers")?;
    let Value::Array(questions) = questions else {
        return Err("draft.request.questions.flatMap is not a function".to_string());
    };
    let mut pairs: Vec<String> = Vec::new();
    for question in &questions {
        let qid = member(question, "id")?;
        let answer_value = match &answers {
            Value::Null => {
                return Err(format!(
                    "Cannot read properties of null (reading '{}')",
                    js_key_string(&qid)
                ));
            }
            v if is_undefined(v) => {
                return Err(format!(
                    "Cannot read properties of undefined (reading '{}')",
                    js_key_string(&qid)
                ));
            }
            Value::Object(m) => m
                .get(&js_key_string(&qid))
                .cloned()
                .unwrap_or_else(crate::js::undefined),
            _ => crate::js::undefined(),
        };
        let custom = match member(&answer_value, "customAnswer") {
            Ok(Value::String(s)) => {
                let t = js_trim(&s);
                if t.is_empty() {
                    None
                } else {
                    Some(t.to_string())
                }
            }
            _ => None,
        };
        let pair_text = if let Some(c) = custom {
            c
        } else {
            let labels = normalize_selected(
                &member(&answer_value, "selectedOptionLabels")
                    .unwrap_or_else(|_| crate::js::undefined()),
            );
            let multi = member(question, "multiSelect").unwrap_or_else(|_| crate::js::undefined());
            if truthy(&multi) {
                if labels.is_empty() {
                    continue;
                }
                labels.join(", ")
            } else {
                match labels.first() {
                    Some(l) => l.clone(),
                    None => continue,
                }
            }
        };
        let q_text = match member(question, "question") {
            Ok(Value::String(s)) => s,
            _ => String::new(),
        };
        pairs.push(format!("{q_text}\n{pair_text}"));
    }
    let joined = pairs.join("\n\n");
    let parts: Vec<&str> = [prompt.as_str(), joined.as_str()]
        .into_iter()
        .filter(|p| !p.is_empty())
        .collect();
    Ok(parts.join("\n\n"))
}
