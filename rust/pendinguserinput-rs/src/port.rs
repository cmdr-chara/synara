//! The Rust port of `apps/web/src/pendingUserInput.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value};

fn js_ws(c: char) -> bool {
    (c.is_whitespace() && c != '\u{85}') || c == '\u{FEFF}'
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(js_ws)
}

fn trim_units(units: &[u16]) -> &[u16] {
    let is_ws = |u: u16| {
        matches!(
            u,
            0x9 | 0xa | 0xb | 0xc | 0xd | 0x20 | 0xa0 | 0x1680 | 0x2000
                ..=0x200a | 0x2028 | 0x2029 | 0x202f | 0x205f | 0x3000 | 0xfeff
        )
    };
    let start = units.iter().position(|u| !is_ws(*u)).unwrap_or(units.len());
    let end = units
        .iter()
        .rposition(|u| !is_ws(*u))
        .map_or(start, |i| i + 1);
    &units[start..end]
}

/// `v.key` — throws on null/undefined receivers like JS property
/// access; primitives yield undefined for non-existent members.
fn prop(v: &Value, key: &str) -> Result<Value, String> {
    match v {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        _ if crate::js::is_undefined(v) => Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        )),
        Value::Object(m) => Ok(m.get(key).cloned().unwrap_or_else(crate::js::undefined)),
        _ => Ok(crate::js::undefined()),
    }
}

/// `v?.key` — undefined on null/undefined, plain lookup otherwise.
fn opt_prop(v: &Value, key: &str) -> Value {
    match v {
        Value::Object(m) => m.get(key).cloned().unwrap_or_else(crate::js::undefined),
        _ => crate::js::undefined(),
    }
}

fn truthy(v: &Value) -> bool {
    match v {
        _ if crate::js::is_undefined(v) => false,
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::String(s) => !s.is_empty(),
        _ => match crate::js::as_number(v) {
            Some(n) => n != 0.0 && !n.is_nan(),
            None => true,
        },
    }
}

/// JS `ToString` for values used as object keys / indices.
fn js_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Array(items) => items
            .iter()
            .map(|x| {
                if x.is_null() || crate::js::is_undefined(x) {
                    String::new()
                } else {
                    js_to_string(x)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        _ if crate::js::is_undefined(v) => "undefined".to_string(),
        _ => {
            if let Some(n) = crate::js::as_number(v) {
                crate::js::number_to_string(n)
            } else {
                "[object Object]".to_string()
            }
        }
    }
}

/// Canonical array-index check: `arr["0"]` indexes, `arr["foo"]` does not.
fn array_index(key: &str) -> Option<usize> {
    if key.is_empty() || !key.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let n: usize = key.parse().ok()?;
    (n.to_string() == key).then_some(n)
}

/// `obj[key]` with a stringified key, incl. array/string indexing.
fn index(v: &Value, key: &str) -> Result<Value, String> {
    match v {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        _ if crate::js::is_undefined(v) => Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        )),
        Value::Object(m) => Ok(m.get(key).cloned().unwrap_or_else(crate::js::undefined)),
        Value::Array(items) => Ok(array_index(key)
            .and_then(|i| items.get(i).cloned())
            .unwrap_or_else(crate::js::undefined)),
        Value::String(s) => Ok(array_index(key)
            .and_then(|i| {
                let units: Vec<u16> = s.encode_utf16().collect();
                units.get(i).map(|u| crate::js::utf16_string(&[*u]))
            })
            .unwrap_or_else(crate::js::undefined)),
        _ => Ok(crate::js::undefined()),
    }
}

/// The trimmed `entry` of a string wire value (`{"$utf16"}` included).
fn trimmed_string(v: &Value) -> Option<Value> {
    match v {
        Value::String(s) => Some(Value::String(js_trim(s).to_string())),
        _ => crate::js::string_units(v).map(|u| crate::js::utf16_string(trim_units(&u))),
    }
}

fn string_value(v: &Value) -> Option<Value> {
    match v {
        Value::String(_) => Some(v.clone()),
        _ if crate::js::string_units(v).is_some() => Some(v.clone()),
        _ => None,
    }
}

fn is_empty_string(v: &Value) -> bool {
    match v {
        Value::String(s) => s.is_empty(),
        _ => crate::js::string_units(v).is_some_and(|u| u.is_empty()),
    }
}

/// `normalizeDraftAnswer`: strings trim to null-or-trimmed; anything
/// else is null.
fn sanitize_draft_answer(v: &Value) -> Option<Value> {
    let trimmed = trimmed_string(v)?;
    if is_empty_string(&trimmed) {
        None
    } else {
        Some(trimmed)
    }
}

/// `normalizeSelectedOptionLabels`: string entries, trimmed,
/// non-empty, first-occurrence deduped.
fn normalize_selected_labels(v: &Value) -> Vec<Value> {
    let Value::Array(items) = v else {
        return Vec::new();
    };
    let mut out: Vec<Value> = Vec::new();
    for item in items {
        let Some(s) = string_value(item).and_then(|x| trimmed_string(&x)) else {
            continue;
        };
        if is_empty_string(&s) || out.contains(&s) {
            continue;
        }
        out.push(s);
    }
    out
}

/// Port of `resolvePendingUserInputAnswer` (`apps/web/src/pendingUserInput.ts:50`).
pub fn resolve_pending_user_input_answer(
    question: serde_json::Value,
    draft: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if let Some(custom) = sanitize_draft_answer(&opt_prop(&draft, "customAnswer")) {
        return Ok(custom);
    }
    let selected = normalize_selected_labels(&opt_prop(&draft, "selectedOptionLabels"));
    if truthy(&prop(&question, "multiSelect")?) {
        return Ok(if selected.is_empty() {
            Value::Null
        } else {
            Value::Array(selected)
        });
    }
    Ok(selected.into_iter().next().unwrap_or(Value::Null))
}

/// Port of `setPendingUserInputCustomAnswer` (`apps/web/src/pendingUserInput.ts:67`).
pub fn set_pending_user_input_custom_answer(
    draft: serde_json::Value,
    custom_answer: String,
) -> Result<serde_json::Value, String> {
    let mut out = Map::new();
    out.insert(
        "customAnswer".to_string(),
        Value::String(custom_answer.clone()),
    );
    if js_trim(&custom_answer).is_empty() {
        let selected = normalize_selected_labels(&opt_prop(&draft, "selectedOptionLabels"));
        if !selected.is_empty() {
            out.insert("selectedOptionLabels".to_string(), Value::Array(selected));
        }
    }
    Ok(Value::Object(out))
}

/// Port of `togglePendingUserInputOptionSelection` (`apps/web/src/pendingUserInput.ts:83`).
pub fn toggle_pending_user_input_option_selection(
    question: serde_json::Value,
    draft: serde_json::Value,
    option_label: String,
) -> Result<serde_json::Value, String> {
    let mut out = Map::new();
    out.insert("customAnswer".to_string(), Value::String(String::new()));
    if truthy(&prop(&question, "multiSelect")?) {
        let selected = normalize_selected_labels(&opt_prop(&draft, "selectedOptionLabels"));
        let mut next: Vec<Value> = if selected
            .iter()
            .any(|s| matches!(s, Value::String(l) if *l == option_label))
        {
            selected
                .into_iter()
                .filter(|s| !matches!(s, Value::String(l) if *l == option_label))
                .collect()
        } else {
            let mut s = selected;
            s.push(Value::String(option_label));
            s
        };
        if !next.is_empty() {
            out.insert(
                "selectedOptionLabels".to_string(),
                Value::Array(std::mem::take(&mut next)),
            );
        }
    } else {
        out.insert(
            "selectedOptionLabels".to_string(),
            Value::Array(vec![Value::String(option_label)]),
        );
    }
    Ok(Value::Object(out))
}

/// `for (const q of questions)` — strings iterate as chars, arrays as
/// elements; anything else is not iterable.
fn iterate(questions: &Value) -> Result<Vec<Value>, String> {
    match questions {
        Value::Array(items) => Ok(items.clone()),
        Value::String(s) => Ok(s.chars().map(|c| Value::String(c.to_string())).collect()),
        _ => Err("questions is not iterable".to_string()),
    }
}

/// Port of `buildPendingUserInputAnswers` (`apps/web/src/pendingUserInput.ts:108`).
pub fn build_pending_user_input_answers(
    questions: serde_json::Value,
    draft_answers: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let mut answers = Map::new();
    for question in iterate(&questions)? {
        let id = prop(&question, "id")?;
        let key = js_to_string(&id);
        let draft = index(&draft_answers, &key)?;
        let answer = resolve_pending_user_input_answer(question, draft)?;
        if !truthy(&answer) {
            return Ok(None);
        }
        answers.insert(key, answer);
    }
    Ok(Some(Value::Object(answers)))
}

/// `Object.entries` equivalents over recordable values.
fn object_entries(v: &Value) -> Result<Vec<(String, Value)>, String> {
    match v {
        Value::Null => Err("Cannot convert undefined or null to object".to_string()),
        _ if crate::js::is_undefined(v) => {
            Err("Cannot convert undefined or null to object".to_string())
        }
        Value::Object(m) => Ok(m.iter().map(|(k, x)| (k.clone(), x.clone())).collect()),
        Value::Array(items) => Ok(items
            .iter()
            .enumerate()
            .map(|(i, x)| (i.to_string(), x.clone()))
            .collect()),
        Value::String(s) => Ok(s
            .chars()
            .enumerate()
            .map(|(i, c)| (i.to_string(), Value::String(c.to_string())))
            .collect()),
        _ => Ok(Vec::new()),
    }
}

/// Port of `hasCompletePendingUserInputAnswers` (`apps/web/src/pendingUserInput.ts:125`).
pub fn has_complete_pending_user_input_answers(answers: serde_json::Value) -> Result<bool, String> {
    let entries = object_entries(&answers)?;
    if entries.is_empty() {
        return Ok(false);
    }
    Ok(entries.iter().all(|(_, answer)| match answer {
        Value::String(s) => !js_trim(s).is_empty(),
        Value::Array(items) => items
            .iter()
            .any(|entry| trimmed_string(entry).is_some_and(|s| !is_empty_string(&s))),
        _ => false,
    }))
}

/// Port of `omitNullPendingUserInputAnswers` (`apps/web/src/pendingUserInput.ts:144`).
pub fn omit_null_pending_user_input_answers(
    answers: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let mut out = Map::new();
    for (key, value) in object_entries(&answers)? {
        if !value.is_null() && !crate::js::is_undefined(&value) {
            out.insert(key, value);
        }
    }
    Ok(Value::Object(out))
}

/// `questions[Math.max(0, Math.min(i, len-1))]` — JS dynamic indexing
/// on arrays (integer only) and strings (UTF-16 units).
fn dynamic_index(questions: &Value, i: f64) -> Value {
    if !i.is_finite() || i < 0.0 || i.fract() != 0.0 {
        return crate::js::undefined();
    }
    let idx = i as usize;
    match questions {
        Value::Array(items) => items.get(idx).cloned().unwrap_or_else(crate::js::undefined),
        Value::String(s) => {
            let units: Vec<u16> = s.encode_utf16().collect();
            units
                .get(idx)
                .map(|u| crate::js::utf16_string(&[*u]))
                .unwrap_or_else(crate::js::undefined)
        }
        _ => crate::js::undefined(),
    }
}

/// Port of `derivePendingUserInputProgress` (`apps/web/src/pendingUserInput.ts:161`).
pub fn derive_pending_user_input_progress(
    questions: serde_json::Value,
    draft_answers: serde_json::Value,
    question_index: f64,
) -> Result<serde_json::Value, String> {
    let len = match &questions {
        Value::Array(items) => items.len() as f64,
        Value::String(s) => s.encode_utf16().count() as f64,
        _ => crate::js::as_number(&prop(&questions, "length")?).unwrap_or(f64::NAN),
    };
    let normalized = if len == 0.0 {
        0.0
    } else {
        // JS Math.min/Math.max: NaN in any operand yields NaN.
        let inner = if question_index.is_nan() || (len - 1.0).is_nan() {
            f64::NAN
        } else {
            question_index.min(len - 1.0)
        };
        if inner.is_nan() {
            f64::NAN
        } else {
            inner.max(0.0)
        }
    };
    let active_question = dynamic_index(&questions, normalized);
    let active_question = if crate::js::is_undefined(&active_question) {
        Value::Null
    } else {
        active_question
    };
    let active_draft = if truthy(&active_question) {
        index(
            &draft_answers,
            &js_to_string(&prop(&active_question, "id")?),
        )?
    } else {
        crate::js::undefined()
    };
    let resolved_answer = if truthy(&active_question) {
        resolve_pending_user_input_answer(active_question.clone(), active_draft.clone())?
    } else {
        Value::Null
    };
    let custom_answer = {
        let c = opt_prop(&active_draft, "customAnswer");
        if c.is_null() || crate::js::is_undefined(&c) {
            Value::String(String::new())
        } else {
            c
        }
    };
    // `questions.reduce(...)` — only arrays (and strings via reduce?
    // no) have reduce; anything else throws here.
    let list = match &questions {
        Value::Array(items) => items.clone(),
        _ => return Err("questions.reduce is not a function".to_string()),
    };
    let mut answered_count = 0usize;
    for question in &list {
        let id = prop(question, "id")?;
        let draft = index(&draft_answers, &js_to_string(&id))?;
        if truthy(&resolve_pending_user_input_answer(question.clone(), draft)?) {
            answered_count += 1;
        }
    }
    let is_last = len == 0.0 || normalized >= len - 1.0;
    let using_custom = match &custom_answer {
        Value::String(s) => !js_trim(s).is_empty(),
        _ => return Err("customAnswer.trim is not a function".to_string()),
    };
    let is_complete = build_pending_user_input_answers(questions, draft_answers)?.is_some();
    let mut out = Map::new();
    out.insert("questionIndex".to_string(), crate::js::number(normalized));
    out.insert("activeQuestion".to_string(), active_question);
    out.insert("activeDraft".to_string(), active_draft.clone());
    out.insert(
        "selectedOptionLabels".to_string(),
        Value::Array(normalize_selected_labels(&opt_prop(
            &active_draft,
            "selectedOptionLabels",
        ))),
    );
    out.insert("customAnswer".to_string(), custom_answer);
    out.insert("resolvedAnswer".to_string(), resolved_answer.clone());
    out.insert("usingCustomAnswer".to_string(), Value::Bool(using_custom));
    out.insert(
        "answeredQuestionCount".to_string(),
        crate::js::number(answered_count as f64),
    );
    out.insert("isLastQuestion".to_string(), Value::Bool(is_last));
    out.insert("isComplete".to_string(), Value::Bool(is_complete));
    out.insert(
        "canAdvance".to_string(),
        Value::Bool(truthy(&resolved_answer)),
    );
    Ok(Value::Object(out))
}

/// Port of `splitRecommendedOptionLabel` (`apps/web/src/pendingUserInput.ts:197`).
///
/// Splits a trailing `\s*(recommended)\s*$` (case-insensitive) off the
/// label; empty results keep the original label.
pub fn split_recommended_option_label(label: String) -> Result<serde_json::Value, String> {
    let units: Vec<u16> = label.encode_utf16().collect();
    // Find the longest suffix matching \s* ( recommended ) \s*
    let is_ws = |u: u16| {
        matches!(
            u,
            0x9 | 0xa | 0xb | 0xc | 0xd | 0x20 | 0xa0 | 0x1680 | 0x2000
                ..=0x200a | 0x2028 | 0x2029 | 0x202f | 0x205f | 0x3000 | 0xfeff
        )
    };
    let mut end = units.len();
    while end > 0 && is_ws(units[end - 1]) {
        end -= 1;
    }
    const WORD: &[u8] = b"(recommended)";
    let is_ci = |u: u16, w: u8| u < 128 && (u as u8 == w || u as u8 == w.to_ascii_uppercase());
    let mut stripped_end = None;
    if end >= WORD.len() && units[end - 1] == b')' as u16 {
        let start = end - WORD.len();
        let tail = &units[start..end];
        if tail.iter().enumerate().all(|(i, u)| {
            if i == 0 || i == WORD.len() - 1 {
                *u == WORD[i] as u16
            } else {
                is_ci(*u, WORD[i])
            }
        }) {
            // `\s*` before the word
            let mut s = start;
            while s > 0 && is_ws(units[s - 1]) {
                s -= 1;
            }
            stripped_end = Some(s);
        }
    }
    let mut out = Map::new();
    match stripped_end {
        Some(0) | None => {
            out.insert("label".to_string(), Value::String(label));
            out.insert("recommended".to_string(), Value::Bool(false));
        }
        Some(n) => {
            let stripped = String::from_utf16_lossy(&units[..n]);
            out.insert("label".to_string(), Value::String(stripped));
            out.insert("recommended".to_string(), Value::Bool(true));
        }
    }
    Ok(Value::Object(out))
}
