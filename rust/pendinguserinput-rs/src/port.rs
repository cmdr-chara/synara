//! The Rust port of `apps/web/src/pendingUserInput.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value};

fn blank(u: u16) -> bool {
    matches!(
        u,
        0x9 | 0xa | 0xb | 0xc | 0xd | 0x20 | 0xa0 | 0x1680 | 0x2000
            ..=0x200a | 0x2028 | 0x2029 | 0x202f | 0x205f | 0x3000 | 0xfeff
    )
}

/// JS `String.prototype.trim` on UTF-16 code units.
fn clip_units(units: &[u16]) -> &[u16] {
    let first = units.iter().position(|u| !blank(*u)).unwrap_or(units.len());
    let last = units
        .iter()
        .rposition(|u| !blank(*u))
        .map_or(first, |i| i + 1);
    &units[first..last]
}

/// A JS string value as UTF-16 code units, plain or `$utf16`-marked.
fn text_units(v: &Value) -> Option<Vec<u16>> {
    crate::js::string_units(v)
}

fn nullish(v: &Value) -> bool {
    v.is_null() || crate::js::is_undefined(v)
}

/// JS truthiness over the structured-value encoding.
fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0),
        Value::String(s) => !s.is_empty(),
        _ => {
            if crate::js::is_undefined(v) {
                return false;
            }
            if let Some(x) = crate::js::as_number(v) {
                return x != 0.0 && !x.is_nan();
            }
            if let Some(units) = text_units(v) {
                return !units.is_empty();
            }
            true
        }
    }
}

/// `v.name` — a member read that throws on null/undefined receivers like JS.
fn field(v: &Value, name: &str) -> Result<Value, String> {
    if v.is_null() {
        return Err(format!("Cannot read properties of null (reading '{name}')"));
    }
    if crate::js::is_undefined(v) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{name}')"
        ));
    }
    Ok(plain_field(v, name))
}

/// Member read on a non-nullish receiver: objects give their own property,
/// strings and arrays answer `length`, anything else is undefined.
fn plain_field(v: &Value, name: &str) -> Value {
    match v {
        Value::Object(m) => m.get(name).cloned().unwrap_or_else(crate::js::undefined),
        Value::Array(a) if name == "length" => crate::js::number(a.len() as f64),
        Value::String(s) if name == "length" => crate::js::number(s.encode_utf16().count() as f64),
        _ => crate::js::undefined(),
    }
}

/// `v?.name` — optional chaining: nullish receivers yield undefined, never throw.
fn field_opt(v: &Value, name: &str) -> Value {
    if nullish(v) {
        crate::js::undefined()
    } else {
        plain_field(v, name)
    }
}

/// The property key JS builds for `container[id]`.
fn key_of(id: &Value) -> String {
    match id {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        _ => {
            if crate::js::is_undefined(id) {
                "undefined".to_string()
            } else if let Some(x) = crate::js::as_number(id) {
                crate::js::number_to_string(x)
            } else if let Some(units) = text_units(id) {
                String::from_utf16_lossy(&units)
            } else if let Value::Array(a) = id {
                a.iter().map(key_of).collect::<Vec<_>>().join(",")
            } else {
                "[object Object]".to_string()
            }
        }
    }
}

/// `questions[i]` — numeric slot read; out of range or non-indexable is undefined.
fn slot(v: &Value, i: f64) -> Value {
    let in_range = |len: usize| i >= 0.0 && i.fract() == 0.0 && (i as usize) < len;
    match v {
        Value::Array(a) => {
            if in_range(a.len()) {
                a[i as usize].clone()
            } else {
                crate::js::undefined()
            }
        }
        Value::String(s) => {
            let units: Vec<u16> = s.encode_utf16().collect();
            if in_range(units.len()) {
                crate::js::utf16_string(&units[i as usize..=i as usize])
            } else {
                crate::js::undefined()
            }
        }
        _ => crate::js::undefined(),
    }
}

/// `for (const x of questions)` — arrays yield elements, strings code points,
/// everything else throws.
fn sequence(v: &Value) -> Result<Vec<Value>, String> {
    match v {
        Value::Array(a) => Ok(a.clone()),
        Value::String(s) => Ok(s.chars().map(|c| Value::String(c.to_string())).collect()),
        _ => Err("questions is not iterable".to_string()),
    }
}

/// `Object.entries(v)` — strings expose UTF-16 unit indices, arrays their
/// indices, primitives nothing; null/undefined throw.
fn pairs(v: &Value) -> Result<Vec<(String, Value)>, String> {
    if v.is_null() || crate::js::is_undefined(v) {
        return Err("Cannot convert undefined or null to object".to_string());
    }
    if crate::js::as_number(v).is_some() {
        return Ok(vec![]);
    }
    if v.is_object()
        && let Some(units) = text_units(v)
    {
        return Ok(units
            .iter()
            .enumerate()
            .map(|(i, u)| (i.to_string(), crate::js::utf16_string(&[*u])))
            .collect());
    }
    match v {
        Value::Object(m) => Ok(m.iter().map(|(k, x)| (k.clone(), x.clone())).collect()),
        Value::Array(a) => Ok(a
            .iter()
            .enumerate()
            .map(|(i, x)| (i.to_string(), x.clone()))
            .collect()),
        Value::String(s) => Ok(s
            .encode_utf16()
            .enumerate()
            .map(|(i, u)| (i.to_string(), crate::js::utf16_string(&[u])))
            .collect()),
        _ => Ok(vec![]),
    }
}

/// A draft string field becomes an answer only when trimmed content survives.
fn clean_text(v: &Value) -> Option<Vec<u16>> {
    let units = text_units(v)?;
    let kept = clip_units(&units);
    (!kept.is_empty()).then(|| kept.to_vec())
}

/// The draft's usable option labels: each kept only when trimmed content survives.
fn label_list(draft: &Value) -> Vec<Vec<u16>> {
    match field_opt(draft, "selectedOptionLabels") {
        Value::Array(items) => items.iter().filter_map(clean_text).collect(),
        _ => vec![],
    }
}

fn picked_answer(question: &Value, draft: &Value) -> Result<Value, String> {
    if let Some(custom) = clean_text(&field_opt(draft, "customAnswer")) {
        return Ok(crate::js::utf16_string(&custom));
    }
    let picks = label_list(draft);
    if truthy(&field(question, "multiSelect")?) {
        return Ok(if picks.is_empty() {
            Value::Null
        } else {
            Value::Array(picks.iter().map(|u| crate::js::utf16_string(u)).collect())
        });
    }
    Ok(picks
        .first()
        .map(|u| crate::js::utf16_string(u))
        .unwrap_or(Value::Null))
}

/// Port of `resolvePendingUserInputAnswer` (`apps/web/src/pendingUserInput.ts:50`).
pub fn resolve_pending_user_input_answer(
    question: serde_json::Value,
    draft: serde_json::Value,
) -> Result<serde_json::Value, String> {
    picked_answer(&question, &draft)
}

/// Port of `setPendingUserInputCustomAnswer` (`apps/web/src/pendingUserInput.ts:67`).
pub fn set_pending_user_input_custom_answer(
    draft: serde_json::Value,
    custom_answer: String,
) -> Result<serde_json::Value, String> {
    let units: Vec<u16> = custom_answer.encode_utf16().collect();
    let mut out = Map::new();
    out.insert("customAnswer".to_string(), crate::js::utf16_string(&units));
    if clip_units(&units).is_empty() {
        let labels = field_opt(&draft, "selectedOptionLabels");
        if !nullish(&labels) {
            out.insert("selectedOptionLabels".to_string(), labels);
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
    let multi = field(&question, "multiSelect")?;
    let mut out = Map::new();
    out.insert("customAnswer".to_string(), Value::String(String::new()));
    let is_label = |x: &Value| matches!(x, Value::String(s) if *s == option_label);
    let chosen = if truthy(&multi) {
        match field_opt(&draft, "selectedOptionLabels") {
            Value::Array(items) if items.iter().any(&is_label) => {
                items.into_iter().filter(|x| !is_label(x)).collect()
            }
            Value::Array(mut items) => {
                items.push(Value::String(option_label));
                items
            }
            _ => vec![Value::String(option_label)],
        }
    } else {
        vec![Value::String(option_label)]
    };
    out.insert("selectedOptionLabels".to_string(), Value::Array(chosen));
    Ok(Value::Object(out))
}

/// Port of `buildPendingUserInputAnswers` (`apps/web/src/pendingUserInput.ts:108`).
pub fn build_pending_user_input_answers(
    questions: serde_json::Value,
    draft_answers: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let mut out = Map::new();
    for question in sequence(&questions)? {
        let key = key_of(&field(&question, "id")?);
        let draft = field(&draft_answers, &key)?;
        let answer = picked_answer(&question, &draft)?;
        if !truthy(&answer) {
            return Ok(None);
        }
        out.insert(key, answer);
    }
    Ok(Some(Value::Object(out)))
}

/// Port of `hasCompletePendingUserInputAnswers` (`apps/web/src/pendingUserInput.ts:125`).
pub fn has_complete_pending_user_input_answers(answers: serde_json::Value) -> Result<bool, String> {
    let entries = pairs(&answers)?;
    if entries.is_empty() {
        return Ok(false);
    }
    Ok(entries.iter().all(|(_, answer)| match answer {
        Value::String(s) => !clip_units(&s.encode_utf16().collect::<Vec<_>>()).is_empty(),
        Value::Array(items) => items.iter().any(|x| clean_text(x).is_some()),
        _ => false,
    }))
}

/// Port of `omitNullPendingUserInputAnswers` (`apps/web/src/pendingUserInput.ts:144`).
pub fn omit_null_pending_user_input_answers(
    answers: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let mut out = Map::new();
    for (key, value) in pairs(&answers)? {
        if !nullish(&value) {
            out.insert(key, value);
        }
    }
    Ok(Value::Object(out))
}

/// JS `Math.max`: NaN wins, and `Math.max(0, -0)` is `+0`.
fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == b {
        if a == 0.0 { 0.0 } else { a }
    } else {
        f64::max(a, b)
    }
}

/// JS `Math.min`: NaN wins, and `Math.min(-0, 0)` is `-0`.
fn js_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == b {
        if a == 0.0 {
            if a.is_sign_negative() && b.is_sign_negative() {
                -0.0
            } else {
                0.0
            }
        } else {
            a
        }
    } else {
        f64::min(a, b)
    }
}

/// Port of `derivePendingUserInputProgress` (`apps/web/src/pendingUserInput.ts:161`).
pub fn derive_pending_user_input_progress(
    questions: serde_json::Value,
    draft_answers: serde_json::Value,
    question_index: f64,
) -> Result<serde_json::Value, String> {
    let len = crate::js::as_number(&field(&questions, "length")?).unwrap_or(f64::NAN);
    let index = if len == 0.0 {
        0.0
    } else {
        js_min(js_max(0.0, question_index), len - 1.0)
    };

    let active = slot(&questions, index);
    let active_present = !nullish(&active);
    let active_question = if active_present {
        active.clone()
    } else {
        Value::Null
    };

    let draft = if active_present {
        let key = key_of(&field(&active, "id")?);
        field(&draft_answers, &key)?
    } else {
        crate::js::undefined()
    };

    let raw_labels = field_opt(&draft, "selectedOptionLabels");
    let selected_labels = if nullish(&raw_labels) {
        Value::Array(vec![])
    } else {
        raw_labels
    };
    let raw_custom = field_opt(&draft, "customAnswer");
    let custom_text = if nullish(&raw_custom) {
        Value::String(String::new())
    } else {
        raw_custom
    };

    let resolved = if active_present {
        picked_answer(&active, &draft)?
    } else {
        Value::Null
    };
    let using_custom = matches!(&resolved, Value::String(_))
        && clean_text(&custom_text).is_some_and(|u| crate::js::utf16_string(&u) == resolved);

    let answered_count = match &questions {
        Value::Array(items) => {
            let mut count = 0usize;
            for q in items {
                let key = key_of(&field(q, "id")?);
                let draft = field(&draft_answers, &key)?;
                if truthy(&picked_answer(q, &draft)?) {
                    count += 1;
                }
            }
            count
        }
        _ => return Err("questions.reduce is not a function".to_string()),
    };

    let mut out = Map::new();
    out.insert("questionIndex".to_string(), crate::js::number(index));
    out.insert("activeQuestion".to_string(), active_question);
    out.insert("activeDraft".to_string(), draft);
    out.insert("selectedOptionLabels".to_string(), selected_labels);
    out.insert("customAnswer".to_string(), custom_text);
    out.insert("resolvedAnswer".to_string(), resolved.clone());
    out.insert("usingCustomAnswer".to_string(), Value::Bool(using_custom));
    out.insert(
        "answeredQuestionCount".to_string(),
        crate::js::number(answered_count as f64),
    );
    out.insert(
        "isLastQuestion".to_string(),
        Value::Bool(index >= len - 1.0),
    );
    out.insert(
        "isComplete".to_string(),
        Value::Bool(answered_count as f64 == len),
    );
    out.insert("canAdvance".to_string(), Value::Bool(!resolved.is_null()));
    Ok(Value::Object(out))
}

/// Port of `splitRecommendedOptionLabel` (`apps/web/src/pendingUserInput.ts:197`).
pub fn split_recommended_option_label(label: String) -> Result<serde_json::Value, String> {
    let units: Vec<u16> = label.encode_utf16().collect();
    const WORD: [u16; 11] = [
        b'r' as u16,
        b'e' as u16,
        b'c' as u16,
        b'o' as u16,
        b'm' as u16,
        b'm' as u16,
        b'e' as u16,
        b'n' as u16,
        b'd' as u16,
        b'e' as u16,
        b'd' as u16,
    ];
    let strip_back = |units: &[u16], mut at: usize| {
        while at > 0 && blank(units[at - 1]) {
            at -= 1;
        }
        at
    };
    let mut end = units.len();
    let mut scan = strip_back(&units, end);
    if scan > 0 && units[scan - 1] == b')' as u16 {
        scan = strip_back(&units, scan - 1);
        if scan >= WORD.len()
            && units[scan - WORD.len()..scan]
                .iter()
                .zip(WORD.iter())
                .all(|(a, b)| {
                    let folded = if *a < 0x80 {
                        (*a as u8).to_ascii_lowercase() as u16
                    } else {
                        *a
                    };
                    folded == *b
                })
        {
            scan = strip_back(&units, scan - WORD.len());
            if scan > 0 && units[scan - 1] == b'(' as u16 {
                end = strip_back(&units, scan - 1);
            }
        }
    }
    let candidate = &units[..end];
    let label_out = if clip_units(candidate).is_empty() {
        crate::js::utf16_string(&units)
    } else {
        crate::js::utf16_string(candidate)
    };
    let mut out = Map::new();
    out.insert("label".to_string(), label_out);
    out.insert("recommended".to_string(), Value::Bool(end != units.len()));
    Ok(Value::Object(out))
}
