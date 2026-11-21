//! The Rust port of `apps/web/src/appSnap.logic.ts` —
//! `didAppSnapHydrationInputsChange`, `hasPersistedAppSnapCapture`,
//! `hasHydratedAppSnapCapture`, `persistedAppSnapCaptureBlobKeys`.

#![forbid(unsafe_code)]

use crate::js::{eq_str, idx, is_undefined, iterate, js_eq, keys, prop, truthy};
use serde_json::Value;

/// `isComposerAppSnapCaptureSource(value, captureId)` — falsy/non-object value
/// or empty captureId gives false; kind must be "appsnap"/"appshot" and
/// `captureId === captureId` strictly.
fn is_capture_source(v: &Value, capture_id: &str) -> bool {
    if capture_id.is_empty() || !v.is_object() {
        return false;
    }
    let Some(o) = v.as_object() else { return false };
    (eq_str(o.get("kind"), "appsnap") || eq_str(o.get("kind"), "appshot"))
        && eq_str(o.get("captureId"), capture_id)
}

/// `isCaptureEntry` — reading `entry.source` throws TypeError on a nullish entry.
fn is_capture_entry(entry: &Value, capture_id: &str) -> Result<bool, String> {
    Ok(is_capture_source(
        &prop(entry, "source")?.cloned().unwrap_or(Value::Null),
        capture_id,
    ))
}

/// `entries.some(e => isCaptureEntry(e, captureId))` — `.some` on a nullish
/// receiver throws "Cannot read properties of ...", on a non-array truthy
/// receiver "entries.some is not a function".
fn entries_contain_capture(entries: Option<&Value>, capture_id: &str) -> Result<bool, String> {
    let Some(entries) = entries else {
        return Err("Cannot read properties of undefined (reading 'some')".into());
    };
    if entries.is_null() || is_undefined(entries) {
        return Err("Cannot read properties of null (reading 'some')".into());
    }
    let Value::Array(list) = entries else {
        return Err("entries.some is not a function".into());
    };
    for entry in list {
        if is_capture_entry(entry, capture_id)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// `?? []` — nullish input becomes an empty list; other values pass through.
fn or_empty(v: Option<&Value>) -> Value {
    v.filter(|x| !x.is_null())
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()))
}

/// `Object.keys` union of both maps, first-seen order (Set preserves it).
fn merged_keys(current: &Value, previous: &Value) -> Result<Vec<String>, String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for k in keys(current)?.into_iter().chain(keys(previous)?) {
        if seen.insert(k.clone()) {
            out.push(k);
        }
    }
    Ok(out)
}

/// `a !== b` — strict inequality: equal primitives compare equal, but two
/// separately-decoded objects/arrays are never `===`.
fn neq(a: Option<Value>, b: Option<Value>) -> bool {
    !js_eq(a.as_ref(), b.as_ref())
}

/// Port of `didAppSnapHydrationInputsChange` (`apps/web/src/appSnap.logic.ts:76`).
pub fn did_app_snap_hydration_inputs_change(
    current: serde_json::Value,
    previous: serde_json::Value,
) -> Result<bool, String> {
    for thread_id in merged_keys(&current, &previous)? {
        let cur = idx(Some(&current), &thread_id);
        let prev = idx(Some(&previous), &thread_id);
        if !truthy(cur.as_ref()) || !truthy(prev.as_ref()) {
            return Ok(true);
        }
        let cur = cur.unwrap_or(Value::Null);
        let prev = prev.unwrap_or(Value::Null);
        if neq(
            prop(&cur, "images")?.cloned(),
            prop(&prev, "images")?.cloned(),
        ) || neq(
            prop(&cur, "persistedAttachments")?.cloned(),
            prop(&prev, "persistedAttachments")?.cloned(),
        ) {
            return Ok(true);
        }
        let cur_saved = prop(&cur, "promptHistorySavedDraft")?.cloned();
        let prev_saved = prop(&prev, "promptHistorySavedDraft")?.cloned();
        if truthy(cur_saved.as_ref()) != truthy(prev_saved.as_ref()) {
            return Ok(true);
        }
        if let (Some(cs), Some(ps)) = (&cur_saved, &prev_saved)
            && truthy(Some(cs))
            && truthy(Some(ps))
            && (neq(prop(cs, "images")?.cloned(), prop(ps, "images")?.cloned())
                || neq(
                    prop(cs, "persistedAttachments")?.cloned(),
                    prop(ps, "persistedAttachments")?.cloned(),
                ))
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Port of `hasPersistedAppSnapCapture` (`apps/web/src/appSnap.logic.ts:117`).
pub fn has_persisted_app_snap_capture(
    drafts: serde_json::Value,
    capture_id: String,
) -> Result<bool, String> {
    if capture_id.is_empty() {
        return Ok(false);
    }
    for draft in iterate(&drafts, "drafts")? {
        if !truthy(Some(&draft)) {
            continue;
        }
        if entries_contain_capture(prop(&draft, "persistedAttachments")?, &capture_id)? {
            return Ok(true);
        }
        if let Some(saved) = prop(&draft, "promptHistorySavedDraft")?
            && !saved.is_null()
            && entries_contain_capture(prop(saved, "persistedAttachments")?, &capture_id)?
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Port of `hasHydratedAppSnapCapture` (`apps/web/src/appSnap.logic.ts:137`).
pub fn has_hydrated_app_snap_capture(
    drafts: serde_json::Value,
    capture_id: String,
) -> Result<bool, String> {
    if capture_id.is_empty() {
        return Ok(false);
    }
    for draft in iterate(&drafts, "drafts")? {
        if !truthy(Some(&draft)) {
            continue;
        }
        // `draft.images ?? []` — nullish reads as an empty list, never throws.
        let images = or_empty(prop(&draft, "images")?);
        if entries_contain_capture(Some(&images), &capture_id)? {
            return Ok(true);
        }
        // `draft.promptHistorySavedDraft?.images ?? []`.
        let saved_images = match prop(&draft, "promptHistorySavedDraft")? {
            Some(s) if !s.is_null() => or_empty(prop(s, "images")?),
            _ => Value::Array(Vec::new()),
        };
        if entries_contain_capture(Some(&saved_images), &capture_id)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Port of `persistedAppSnapCaptureBlobKeys` (`apps/web/src/appSnap.logic.ts:155`).
pub fn persisted_app_snap_capture_blob_keys(
    drafts: serde_json::Value,
    capture_id: String,
) -> Result<Vec<String>, String> {
    if capture_id.is_empty() {
        return Ok(Vec::new());
    }
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for draft in iterate(&drafts, "drafts")? {
        if !truthy(Some(&draft)) {
            continue;
        }
        // `[...draft.persistedAttachments, ...]` — nullish or non-iterable throws.
        let mut entries = match prop(&draft, "persistedAttachments")? {
            Some(v) => iterate(v, "draft.persistedAttachments")?,
            None => return Err("draft.persistedAttachments is not iterable".into()),
        };
        // `...(draft.promptHistorySavedDraft?.persistedAttachments ?? [])`.
        if let Some(s) = prop(&draft, "promptHistorySavedDraft")?
            && !s.is_null()
            && let Some(v) = prop(s, "persistedAttachments")?.filter(|v| !v.is_null())
        {
            entries.extend(iterate(v, "(intermediate value)")?);
        }
        for entry in &entries {
            if !is_capture_entry(entry, &capture_id)? {
                continue;
            }
            // `typeof entry.blobKey === "string" && entry.blobKey.length > 0`.
            if let Some(v) = prop(entry, "blobKey")?
                && let Some(units) = crate::js::string_units(v)
            {
                let blob_key = String::from_utf16_lossy(&units);
                if !units.is_empty() && seen.insert(blob_key.clone()) {
                    out.push(blob_key);
                }
            }
        }
    }
    Ok(out)
}
