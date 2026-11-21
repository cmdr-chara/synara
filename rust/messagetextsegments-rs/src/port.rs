//! The Rust port of `apps/web/src/messageTextSegments.ts` —
//! `textSegmentsCoverText` and `advanceMessageTextSegments`.

#![forbid(unsafe_code)]

use crate::js::{
    at_minus_one, iterate, js_eq, js_length, js_to_string, opt_prop, prop, slice_minus_one,
    spread_value, truthy,
};
use serde_json::Value;

/// `v[k]` coerced with `+` semantics — absent reads as "undefined".
fn text_of(v: &Value, k: &str) -> Result<String, String> {
    Ok(prop(v, k)?
        .map(js_to_string)
        .unwrap_or_else(|| "undefined".into()))
}

/// Concatenated `segment.text` of every iterated segment.
fn collate(segments: &Value) -> Result<String, String> {
    let mut out = String::new();
    for segment in iterate(segments)? {
        out += &text_of(&segment, "text")?;
    }
    Ok(out)
}

/// `collated === next`; iteration (and its throws) precedes the comparison.
fn covers(segments: &Value, next: &Value) -> Result<bool, String> {
    let collated = collate(segments)?;
    Ok(crate::js::string_units(next).is_some_and(|u| collated == String::from_utf16_lossy(&u)))
}

/// `[...previous.slice(0, -1), { ...tail, ...patch }]`.
fn merge_tail(previous: &Value, tail: &Value, patch: Vec<(&str, Value)>) -> Result<Value, String> {
    let mut merged = spread_value(tail);
    for (k, v) in patch {
        merged.insert(k.into(), v);
    }
    let mut out = slice_minus_one(previous)?;
    out.push(Value::Object(merged));
    Ok(Value::Array(out))
}

/// Port of `textSegmentsCoverText` (`apps/web/src/messageTextSegments.ts:14`).
pub fn text_segments_cover_text(segments: serde_json::Value, text: String) -> Result<bool, String> {
    covers(&segments, &Value::String(text))
}

/// Port of `advanceMessageTextSegments` (`apps/web/src/messageTextSegments.ts:34`).
/// `previous` takes `Value`: recorded cases pass strings/numbers/objects/null.
pub fn advance_message_text_segments(
    previous: serde_json::Value,
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if crate::js::is_undefined(&previous) {
        return Ok(crate::js::undefined());
    }
    if previous.is_null() {
        return Err("Cannot read properties of null (reading 'length')".into());
    }
    if js_length(&previous) == Some(0.0) {
        return Ok(crate::js::undefined());
    }
    if !truthy(prop(&input, "streaming")?) {
        if js_length(&previous).is_some_and(|l| l < 2.0)
            || !covers(&previous, &opt_prop(&input, "nextText")?)?
        {
            return Ok(crate::js::undefined());
        }
        let tail = at_minus_one(&previous)?.unwrap_or(Value::Null);
        return merge_tail(
            &previous,
            &tail,
            vec![("endedAt", opt_prop(&input, "updatedAt")?)],
        );
    }
    if let Some(started_at) = prop(&input, "segmentStartedAt")? {
        let Value::Array(prev) = &previous else {
            return Err("previous.findIndex is not a function".into());
        };
        let sequence = prop(&input, "segmentSequence")?;
        let opened = Value::Object(serde_json::Map::from_iter([
            ("sequence".into(), opt_prop(&input, "segmentSequence")?),
            ("startedAt".into(), started_at.clone()),
            ("endedAt".into(), opt_prop(&input, "updatedAt")?),
            ("text".into(), opt_prop(&input, "deltaText")?),
        ]));
        let mut out = prev.clone();
        match prev
            .iter()
            .position(|s| js_eq(prop(s, "sequence").ok().flatten(), sequence))
        {
            Some(i) => out[i] = opened,
            None => out.push(opened),
        }
        return Ok(Value::Array(out));
    }
    let tail = at_minus_one(&previous)?.unwrap_or(Value::Null);
    let text = format!(
        "{}{}",
        text_of(&tail, "text")?,
        text_of(&input, "deltaText")?
    );
    merge_tail(
        &previous,
        &tail,
        vec![
            ("text", Value::String(text)),
            ("endedAt", opt_prop(&input, "updatedAt")?),
        ],
    )
}
