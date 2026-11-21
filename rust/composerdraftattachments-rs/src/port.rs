//! The Rust port of `apps/web/src/composerDraftAttachments.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::{Map, Value, json};

fn prop(value: &Value, key: &str) -> Value {
    value.get(key).cloned().unwrap_or_else(js::undefined)
}

fn prop_or_err(value: &Value, key: &str) -> Result<Value, String> {
    if value.is_null() {
        return Err(format!("Cannot read properties of null (reading '{key}')"));
    }
    if js::is_undefined(value) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        ));
    }
    Ok(prop(value, key))
}

fn truthy(value: &Value) -> bool {
    if let Some(n) = js::as_number(value) {
        return n != 0.0 && !n.is_nan();
    }
    if js::is_undefined(value) {
        return false;
    }
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::String(s) => !s.is_empty(),
        _ => true,
    }
}

fn js_strict_eq(a: &Value, b: &Value) -> bool {
    if let (Some(x), Some(y)) = (js::as_number(a), js::as_number(b)) {
        return x == y;
    }
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Object(_), Value::Object(_)) => js::is_undefined(a) && js::is_undefined(b),
        _ => false,
    }
}

fn svz_contains(set: &[Value], needle: &Value) -> bool {
    set.iter().any(|v| {
        if let (Some(x), Some(y)) = (js::as_number(v), js::as_number(needle)) {
            return x == y || (x.is_nan() && y.is_nan());
        }
        js_strict_eq(v, needle)
    })
}

fn js_to_string(value: &Value) -> String {
    if let Some(n) = js::as_number(value) {
        return js::number_to_string(n);
    }
    if js::is_undefined(value) {
        return "undefined".to_string();
    }
    if let Some(units) = js::string_units(value) {
        return String::from_utf16_lossy(&units);
    }
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .map(|item| {
                if item.is_null() || js::is_undefined(item) {
                    String::new()
                } else {
                    js_to_string(item)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

fn iterate<'a>(value: &'a Value, name: &str) -> Result<&'a [Value], String> {
    match value {
        Value::Array(items) => Ok(items),
        Value::Null => Err(format!("{name} is not iterable")),
        _ if js::is_undefined(value) => Err(format!("{name} is not iterable")),
        _ => Err(format!("{name} is not iterable")),
    }
}

fn image_dedup_key(image: &Value) -> Result<String, String> {
    let mime = prop_or_err(image, "mimeType")?;
    let size = prop_or_err(image, "sizeBytes")?;
    let name = prop_or_err(image, "name")?;
    Ok(format!(
        "{}\u{0}{}\u{0}{}",
        js_to_string(&mime),
        js_to_string(&size),
        js_to_string(&name)
    ))
}

/// Port of `pendingComposerAttachmentSyncGenerationCount` (`apps/web/src/composerDraftAttachments.ts:34`).
pub fn pending_composer_attachment_sync_generation_count() -> Result<f64, String> {
    Ok(0.0)
}

/// Port of `mergeComposerImages` (`apps/web/src/composerDraftAttachments.ts:72`).
pub fn merge_composer_images(
    existing_images: Vec<serde_json::Value>,
    incoming_images: serde_json::Value,
) -> Result<Option<Vec<serde_json::Value>>, String> {
    let incoming: Vec<Value> = match &incoming_images {
        Value::Array(items) => items.clone(),
        Value::String(s) => s.chars().map(|c| json!(c.to_string())).collect(),
        _ => return Err("incomingImages is not iterable".to_string()),
    };
    let mut existing_ids: Vec<Value> = Vec::new();
    let mut dedup_keys: Vec<String> = Vec::new();
    let mut preview_urls: Vec<Value> = Vec::new();
    for image in &existing_images {
        existing_ids.push(prop_or_err(image, "id")?);
        dedup_keys.push(image_dedup_key(image)?);
        preview_urls.push(prop_or_err(image, "previewUrl")?);
    }
    let mut accepted: Vec<Value> = Vec::new();
    for image in &incoming {
        let key = image_dedup_key(image)?;
        let id = prop_or_err(image, "id")?;
        if svz_contains(&existing_ids, &id) || dedup_keys.contains(&key) {
            let preview = prop_or_err(image, "previewUrl")?;
            if !svz_contains(&preview_urls, &preview) && !matches!(preview, Value::String(_)) {
                return Err(
                    "Cannot read properties of undefined (reading 'startsWith')".to_string()
                );
            }
            continue;
        }
        accepted.push(image.clone());
        existing_ids.push(id);
        dedup_keys.push(key);
        preview_urls.push(prop_or_err(image, "previewUrl")?);
    }
    if accepted.is_empty() {
        return Ok(None);
    }
    let mut out = existing_images;
    out.extend(accepted);
    Ok(Some(out))
}

/// Port of `composerFileDedupKey` (`apps/web/src/composerDraftAttachments.ts:96`).
pub fn composer_file_dedup_key(file: serde_json::Value) -> Result<String, String> {
    image_dedup_key(&file)
}

/// Port of `revokeObjectPreviewUrl` (`apps/web/src/composerDraftAttachments.ts:100`).
pub fn revoke_object_preview_url(preview_url: String) -> Result<(), String> {
    let _ = &preview_url;
    Ok(())
}

/// Port of `revokeQueuedTurnPreviewUrls` (`apps/web/src/composerDraftAttachments.ts:110`).
pub fn revoke_queued_turn_preview_urls(queued_turn: serde_json::Value) -> Result<(), String> {
    let kind = prop_or_err(&queued_turn, "kind")?;
    if !js_strict_eq(&kind, &json!("chat")) {
        return Ok(());
    }
    let images = prop(&queued_turn, "images");
    for image in iterate(&images, "queuedTurn.images")? {
        prop_or_err(image, "previewUrl")?;
    }
    Ok(())
}

fn revoke_saved_draft(saved_draft: &Value, name: &str) -> Result<(), String> {
    if !truthy(saved_draft) {
        return Ok(());
    }
    let images = prop(saved_draft, "images");
    for image in iterate(&images, name)? {
        prop_or_err(image, "previewUrl")?;
    }
    Ok(())
}

/// Port of `revokePromptHistorySavedDraftPreviewUrls` (`apps/web/src/composerDraftAttachments.ts:119`).
pub fn revoke_prompt_history_saved_draft_preview_urls(
    saved_draft: Option<serde_json::Value>,
) -> Result<(), String> {
    let saved_draft = saved_draft.unwrap_or_else(js::undefined);
    revoke_saved_draft(&saved_draft, "savedDraft.images")
}

/// Port of `revokeDraftPreviewUrls` (`apps/web/src/composerDraftAttachments.ts:130`).
pub fn revoke_draft_preview_urls(draft: Option<serde_json::Value>) -> Result<(), String> {
    let draft = draft.unwrap_or_else(js::undefined);
    if !truthy(&draft) {
        return Ok(());
    }
    let images = prop(&draft, "images");
    for image in iterate(&images, "draft.images")? {
        prop_or_err(image, "previewUrl")?;
    }
    let queued = prop(&draft, "queuedTurns");
    for turn in iterate(&queued, "draft.queuedTurns")? {
        revoke_queued_turn_preview_urls(turn.clone())?;
    }
    revoke_saved_draft(
        &prop(&draft, "promptHistorySavedDraft"),
        "draft.promptHistorySavedDraft.images",
    )
}

/// Port of `revokeDraftComposerImagePreviewUrls` (`apps/web/src/composerDraftAttachments.ts:143`).
pub fn revoke_draft_composer_image_preview_urls(
    draft: Option<serde_json::Value>,
) -> Result<(), String> {
    let draft = draft.unwrap_or_else(js::undefined);
    if !truthy(&draft) {
        return Ok(());
    }
    let images = prop(&draft, "images");
    for image in iterate(&images, "draft.images")? {
        prop_or_err(image, "previewUrl")?;
    }
    revoke_saved_draft(
        &prop(&draft, "promptHistorySavedDraft"),
        "draft.promptHistorySavedDraft.images",
    )
}

fn is_non_empty_string(value: &Value) -> bool {
    matches!(value, Value::String(s) if !s.is_empty())
}

fn is_object(value: &Value) -> bool {
    matches!(value, Value::Object(o) if o.len() != 1 || !o.keys().next().is_some_and(|k| k.starts_with('$')))
        || matches!(value, Value::Array(_))
}

fn is_png_data_url(value: &Value) -> bool {
    let Some(s) = value.as_str() else {
        return false;
    };
    if s.len() > 256_000 {
        return false;
    }
    let Some(rest) = s.strip_prefix("data:image/png;base64,") else {
        return false;
    };
    if rest.is_empty() {
        return false;
    }
    let (body, pad) = match rest.strip_suffix("==") {
        Some(b) => (b, 2),
        None => match rest.strip_suffix('=') {
            Some(b) => (b, 1),
            None => (rest, 0),
        },
    };
    if body.is_empty() || body.ends_with('=') {
        return false;
    }
    pad <= 2
        && rest.len() - pad == body.len()
        && body
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/')
}

fn normalize_image_source(value: &Value) -> Value {
    let Value::Object(_) = value else {
        return js::undefined();
    };
    if !is_object(value) {
        return js::undefined();
    }
    let kind = prop(value, "kind");
    if kind != "appsnap" && kind != "appshot" {
        return js::undefined();
    }
    let capture_id = prop(value, "captureId");
    if !is_non_empty_string(&capture_id) {
        return js::undefined();
    }
    let captured_at = prop(value, "capturedAt");
    if !matches!(captured_at, Value::String(_)) {
        return js::undefined();
    }
    let pick = |key: &str| -> Value {
        match prop(value, key) {
            Value::String(s) => Value::String(s),
            _ => Value::Null,
        }
    };
    let icon = prop(value, "appIconDataUrl");
    let icon = if is_png_data_url(&icon) {
        icon
    } else {
        Value::Null
    };
    let mut map = Map::new();
    map.insert("kind".to_string(), json!("appsnap"));
    map.insert("captureId".to_string(), capture_id);
    map.insert("capturedAt".to_string(), captured_at);
    map.insert("appName".to_string(), pick("appName"));
    map.insert("bundleIdentifier".to_string(), pick("bundleIdentifier"));
    map.insert("appIconDataUrl".to_string(), icon);
    map.insert("windowTitle".to_string(), pick("windowTitle"));
    Value::Object(map)
}

/// Port of `isComposerImageBlobReferenced` (`apps/web/src/composerDraftAttachments.ts:155`).
pub fn is_composer_image_blob_referenced(
    drafts_by_thread_id: serde_json::Value,
    blob_key: String,
) -> Result<bool, String> {
    if blob_key.is_empty() {
        return Ok(false);
    }
    let drafts: Vec<Value> = match &drafts_by_thread_id {
        Value::Object(o) => {
            if js::is_undefined(&drafts_by_thread_id) {
                return Err("Cannot convert undefined or null to object".to_string());
            }
            o.values().cloned().collect()
        }
        Value::Array(a) => a.clone(),
        Value::String(s) => s.chars().map(|c| json!(c.to_string())).collect(),
        Value::Null => return Err("Cannot convert undefined or null to object".to_string()),
        _ => Vec::new(),
    };
    for draft in &drafts {
        if !truthy(draft) {
            continue;
        }
        let attachments = prop(draft, "persistedAttachments");
        let Value::Array(items) = &attachments else {
            return Err(format!(
                "Cannot read properties of {} (reading 'some')",
                if js::is_undefined(&attachments) {
                    "undefined"
                } else {
                    "null"
                }
            ));
        };
        for attachment in items {
            let key = prop_or_err(attachment, "blobKey")?;
            if js_strict_eq(&key, &json!(blob_key)) {
                return Ok(true);
            }
        }
        let saved = prop(draft, "promptHistorySavedDraft");
        if saved.is_null() || js::is_undefined(&saved) {
            continue;
        }
        let saved_attachments = prop_or_err(&saved, "persistedAttachments")?;
        let Value::Array(items) = &saved_attachments else {
            return Err(format!(
                "Cannot read properties of {} (reading 'some')",
                if js::is_undefined(&saved_attachments) {
                    "undefined"
                } else {
                    "null"
                }
            ));
        };
        for attachment in items {
            let key = prop_or_err(attachment, "blobKey")?;
            if js_strict_eq(&key, &json!(blob_key)) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

/// Port of `findSupersededComposerImageBlobAttachments` (`apps/web/src/composerDraftAttachments.ts:176`).
pub fn find_superseded_composer_image_blob_attachments(
    previous_attachments: Vec<serde_json::Value>,
    next_attachments: Vec<serde_json::Value>,
) -> Result<Vec<serde_json::Value>, String> {
    let mut next_keys: Vec<Value> = Vec::new();
    for attachment in &next_attachments {
        let key = prop_or_err(attachment, "blobKey")?;
        if truthy(&key) {
            next_keys.push(key);
        }
    }
    let mut out = Vec::new();
    for attachment in previous_attachments {
        let key = prop_or_err(&attachment, "blobKey")?;
        if truthy(&key) && !svz_contains(&next_keys, &key) {
            out.push(attachment);
        }
    }
    Ok(out)
}

/// Port of `normalizePersistedAttachment` (`apps/web/src/composerDraftAttachments.ts:226`).
pub fn normalize_persisted_attachment(
    value: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    if !is_object(&value) {
        return Ok(None);
    }
    let id = prop(&value, "id");
    let name = prop(&value, "name");
    let mime_type = prop(&value, "mimeType");
    let size_bytes = prop(&value, "sizeBytes");
    let data_url = prop(&value, "dataUrl");
    let blob_key = prop(&value, "blobKey");
    let source = normalize_image_source(&prop(&value, "source"));
    if !is_non_empty_string(&id)
        || !matches!(name, Value::String(_))
        || !matches!(mime_type, Value::String(_))
        || js::as_number(&size_bytes).is_none_or(|n| !n.is_finite())
        || !(is_non_empty_string(&data_url) || is_non_empty_string(&blob_key))
    {
        return Ok(None);
    }
    let mut map = Map::new();
    map.insert("id".to_string(), id);
    map.insert("name".to_string(), name);
    map.insert("mimeType".to_string(), mime_type);
    map.insert("sizeBytes".to_string(), size_bytes);
    if is_non_empty_string(&data_url) {
        map.insert("dataUrl".to_string(), data_url);
    }
    if is_non_empty_string(&blob_key) {
        map.insert("blobKey".to_string(), blob_key);
    }
    if !js::is_undefined(&source) {
        map.insert("source".to_string(), source);
    }
    Ok(Some(Value::Object(map)))
}

/// Port of `toStorageSafePersistedAttachment` (`apps/web/src/composerDraftAttachments.ts:265`).
pub fn to_storage_safe_persisted_attachment(
    attachment: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if attachment.is_null() {
        return Err(
            "Cannot destructure property 'source' of 'attachment' as it is null.".to_string(),
        );
    }
    if js::is_undefined(&attachment) {
        return Err(
            "Cannot destructure property 'source' of 'attachment' as it is undefined.".to_string(),
        );
    }
    let source = normalize_image_source(&prop(&attachment, "source"));
    let mut map = Map::new();
    match &attachment {
        Value::Object(o) => {
            for (k, v) in o {
                if k != "source" {
                    map.insert(k.clone(), v.clone());
                }
            }
        }
        Value::Array(items) => {
            for (i, v) in items.iter().enumerate() {
                map.insert(i.to_string(), v.clone());
            }
        }
        Value::String(s) => {
            if let Some(units) = js::string_units(&attachment) {
                for (i, u) in units.iter().enumerate() {
                    map.insert(i.to_string(), js::utf16_string(&[*u]));
                }
            } else {
                for (i, ch) in s.chars().enumerate() {
                    map.insert(i.to_string(), Value::String(ch.to_string()));
                }
            }
        }
        _ => {}
    }
    if !js::is_undefined(&source) {
        let mut persisted = Map::new();
        if let Value::Object(o) = &source {
            for (k, v) in o {
                if k != "appIconDataUrl" {
                    persisted.insert(k.clone(), v.clone());
                }
            }
        }
        map.insert("source".to_string(), Value::Object(persisted));
    }
    Ok(Value::Object(map))
}

/// Port of `persistQueuedComposerImages` (`apps/web/src/composerDraftAttachments.ts:286`).
pub fn persist_queued_composer_images(
    images: Vec<serde_json::Value>,
) -> Result<Vec<serde_json::Value>, String> {
    let mut out = Vec::new();
    for image in &images {
        let preview = prop_or_err(image, "previewUrl")?;
        let Value::String(preview) = preview else {
            return Err("Cannot read properties of undefined (reading 'startsWith')".to_string());
        };
        if !preview.starts_with("data:") {
            continue;
        }
        let input = json!({
            "id": prop(image, "id"),
            "name": prop(image, "name"),
            "mimeType": prop(image, "mimeType"),
            "sizeBytes": prop(image, "sizeBytes"),
            "dataUrl": preview,
        });
        if let Some(normalized) = normalize_persisted_attachment(input)? {
            out.push(normalized);
        }
    }
    Ok(out)
}

/// Port of `hydrateImagesFromPersisted` (`apps/web/src/composerDraftAttachments.ts:592`).
pub fn hydrate_images_from_persisted(
    attachments: Vec<serde_json::Value>,
) -> Result<Vec<serde_json::Value>, String> {
    let mut out = Vec::new();
    for attachment in &attachments {
        let preview = prop_or_err(attachment, "dataUrl")?;
        if !truthy(&preview) {
            continue;
        }
        let Value::String(data_url) = &preview else {
            return Err("dataUrl.indexOf is not a function".to_string());
        };
        let comma = data_url.find(',');
        let (header, payload) = match comma {
            Some(i) => (&data_url[..i], &data_url[i + 1..]),
            None => (data_url.as_str(), ""),
        };
        if payload.is_empty() {
            continue;
        }
        let decoded = if header.contains(";base64") {
            base64_decode(payload)
        } else {
            percent_decode(payload)
        };
        if decoded.is_none() {
            continue;
        }
        let source = normalize_image_source(&prop(attachment, "source"));
        let mut map = Map::new();
        map.insert("type".to_string(), json!("image"));
        map.insert("id".to_string(), prop(attachment, "id"));
        map.insert("name".to_string(), prop(attachment, "name"));
        map.insert("mimeType".to_string(), prop(attachment, "mimeType"));
        map.insert("sizeBytes".to_string(), prop(attachment, "sizeBytes"));
        map.insert("previewUrl".to_string(), preview.clone());
        map.insert("file".to_string(), json!({}));
        if !js::is_undefined(&source) {
            map.insert("source".to_string(), source);
        }
        out.push(Value::Object(map));
    }
    Ok(out)
}

fn base64_decode(input: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut acc: u32 = 0;
    let mut bits = 0;
    for byte in input.bytes() {
        let v = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => break,
            b'\r' | b'\n' => continue,
            _ => return None,
        };
        acc = (acc << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

fn percent_decode(input: &str) -> Option<Vec<u8>> {
    let bytes = input.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    let hex = |b: u8| -> Option<u8> {
        match b {
            b'0'..=b'9' => Some(b - b'0'),
            b'a'..=b'f' => Some(b - b'a' + 10),
            b'A'..=b'F' => Some(b - b'A' + 10),
            _ => None,
        }
    };
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return None;
            }
            let hi = hex(bytes[i + 1])?;
            let lo = hex(bytes[i + 2])?;
            out.push(hi * 16 + lo);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    Some(out)
}
