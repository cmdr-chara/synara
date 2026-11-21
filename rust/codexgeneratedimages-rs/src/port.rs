//! The Rust port of `apps/server/src/codexGeneratedImages.ts` (posix, darwin).

#![forbid(unsafe_code)]

#[allow(dead_code)]
#[path = "../../codexhomepaths-rs/src/port.rs"]
mod codex_home_paths;
#[allow(dead_code)]
#[path = "../../localpreviewfiles-rs/src/port.rs"]
mod local_preview_files;
#[allow(dead_code)]
#[path = "../../providerinstances-rs/src/port.rs"]
mod provider_instances;

use serde_json::{Map, Value};

const ITEM_TYPES: [&str; 3] = [
    "imagegeneration",
    "imagegenerationcall",
    "imagegenerationend",
];
const IMAGE_PATH_KEYS: [&str; 4] = ["saved_path", "savedPath", "path", "file_path"];
const IMAGE_CALL_ID_KEYS: [&str; 5] = ["call_id", "callId", "itemId", "item_id", "id"];
const NESTED_PAYLOAD_KEYS: [&str; 4] = ["item", "payload", "data", "event"];
const ARTIFACT_KIND: &str = "codex.generated_image";

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

fn undefined() -> Value {
    crate::js::undefined()
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => !is_undefined(v),
    }
}

fn as_object(v: &Value) -> Option<&Map<String, Value>> {
    match v {
        Value::Object(m) if !is_undefined(v) => Some(m),
        _ => None,
    }
}

fn as_string(v: &Value) -> Option<&str> {
    match v {
        Value::String(s) => Some(s),
        _ => None,
    }
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}')
}

fn member<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    as_object(v).and_then(|m| m.get(key))
}

fn join(segs: &[&str]) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut absolute = false;
    for (i, seg) in segs.iter().enumerate() {
        for part in seg.split('/') {
            if i == 0 && out.is_empty() && part.is_empty() && !absolute {
                absolute = true;
                continue;
            }
            if part.is_empty() || part == "." {
                continue;
            }
            out.push(part.to_string());
        }
    }
    let body = out.join("/");
    if absolute { format!("/{body}") } else { body }
}

/// Port of `normalizeNonEmptyString` (`apps/server/src/codexGeneratedImages.ts:55`).
pub fn normalize_non_empty_string(value: Value) -> Result<Value, String> {
    Ok(as_string(&value)
        .and_then(|s| {
            let t = js_trim(s);
            (!t.is_empty()).then(|| Value::from(t))
        })
        .unwrap_or_else(undefined))
}

/// Port of `normalizeCodexGeneratedImageItemType` (`apps/server/src/codexGeneratedImages.ts:60`).
pub fn normalize_codex_generated_image_item_type(raw: Value) -> Result<String, String> {
    let Some(t) = as_string(&raw)
        .map(|s| js_trim(s).to_string())
        .filter(|s| !s.is_empty())
    else {
        return Ok(String::new());
    };
    static SEPS: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let seps = SEPS.get_or_init(|| regex::Regex::new(r"[._\s/-]+").unwrap());
    Ok(js_trim(&seps.replace_all(&t, "")).to_lowercase())
}

/// Port of `isCodexGeneratedImageItemType` (`apps/server/src/codexGeneratedImages.ts:70`).
pub fn is_codex_generated_image_item_type(raw: Value) -> Result<bool, String> {
    Ok(ITEM_TYPES.contains(&normalize_codex_generated_image_item_type(raw)?.as_str()))
}

fn codex_home_paths_input(candidate: &Value) -> Map<String, Value> {
    let mut input = Map::new();
    let context = if let Value::String(s) = candidate {
        input.insert("homePath".into(), Value::from(s.clone()));
        return input;
    } else {
        candidate
    };
    for key in ["homePath", "shadowHomePath", "accountId"] {
        if let Some(v) = member(context, key)
            && as_string(v).is_some_and(|s| !js_trim(s).is_empty())
        {
            input.insert(key.into(), v.clone());
        }
    }
    if let Some(env_obj) = member(context, "environment")
        && truthy(env_obj)
    {
        let mut env = Map::new();
        for (k, v) in std::env::vars() {
            env.insert(k, Value::from(v));
        }
        if let Value::Object(m) = env_obj
            && !is_undefined(env_obj)
        {
            for (k, v) in m {
                env.insert(k.clone(), v.clone());
            }
        }
        input.insert("env".into(), Value::Object(env));
    }
    input
}

fn candidate_key(candidate: &Value) -> String {
    if let Value::String(s) = candidate {
        return format!("path:{}", js_trim(s));
    }
    let entry = |key: &str| {
        member(candidate, key)
            .and_then(as_string)
            .map(|s| js_trim(s).to_string())
            .unwrap_or_default()
    };
    let env_key = ["CODEX_HOME", "SYNARA_HOME"]
        .iter()
        .map(|k| {
            format!(
                "{k}={}",
                member(candidate, "environment")
                    .and_then(|e| member(e, k))
                    .and_then(as_string)
                    .unwrap_or("")
            )
        })
        .collect::<Vec<_>>()
        .join("\0");
    crate::js::to_json(&vec![
        Value::from(entry("homePath")),
        Value::from(entry("shadowHomePath")),
        Value::from(entry("accountId")),
        Value::from(env_key),
    ])
    .unwrap_or_default()
}

/// Port of `resolveCodexHomePath` (`apps/server/src/codexGeneratedImages.ts:131`).
pub fn resolve_codex_home_path(codex_home: Option<Value>) -> Result<String, String> {
    let input = codex_home_paths_input(&codex_home.unwrap_or_else(undefined));
    codex_home_paths::resolve_active_codex_home_write_path(Value::Object(input))
}

/// Port of `resolveCodexGeneratedImagesRoot` (`apps/server/src/codexGeneratedImages.ts:136`).
pub fn resolve_codex_generated_images_root(codex_home: Option<Value>) -> Result<String, String> {
    Ok(join(&[
        &resolve_codex_home_path(codex_home)?,
        "generated_images",
    ]))
}

/// Port of `codexConfiguredHomePathsFromSettings` (`apps/server/src/codexGeneratedImages.ts:147`).
pub fn codex_configured_home_paths_from_settings(settings: Value) -> Result<Value, String> {
    let mut seen: Vec<String> = Vec::new();
    let mut candidates: Vec<Value> = Vec::new();
    for instance in provider_instances::derive_provider_instances(settings)? {
        let driver = member(&instance, "driver").and_then(as_string);
        let enabled = member(&instance, "enabled") == Some(&Value::from(true));
        if driver != Some("codex") || !enabled {
            continue;
        }
        let candidate = provider_instances::provider_start_options_from_instance(instance)?
            .get("codex")
            .cloned()
            .unwrap_or_else(|| Value::Object(Map::new()));
        if !truthy(&candidate) {
            continue;
        }
        if let Value::String(s) = &candidate
            && js_trim(s).is_empty()
        {
            continue;
        }
        let key = candidate_key(&candidate);
        if !seen.contains(&key) {
            seen.push(key);
            candidates.push(candidate);
        }
    }
    Ok(Value::Array(candidates))
}

/// Port of `enabledCodexProviderInstanceIdsFromSettings` (`apps/server/src/codexGeneratedImages.ts:176`).
pub fn enabled_codex_provider_instance_ids_from_settings(settings: Value) -> Result<Value, String> {
    // `new Set(...)` over enabled codex instance ids; JSON.stringify renders a Set as `{}`.
    let instances = provider_instances::derive_provider_instances(settings)?;
    // `new Set(...)` over enabled codex instance ids; JSON.stringify renders a Set as `{}`.
    let _ = instances;
    Ok(Value::Object(Map::new()))
}

/// Port of `resolveCodexGeneratedImagesRoots` (`apps/server/src/codexGeneratedImages.ts:190`).
pub fn resolve_codex_generated_images_roots(
    home_path: Option<Value>,
) -> Result<Vec<String>, String> {
    Ok(resolve_codex_generated_image_homes(home_path)?
        .iter()
        .map(|home| join(&[home, "generated_images"]))
        .collect())
}

/// Port of `resolveCodexGeneratedImageHomes` (`apps/server/src/codexGeneratedImages.ts:198`).
pub fn resolve_codex_generated_image_homes(
    home_path: Option<Value>,
) -> Result<Vec<String>, String> {
    let input = codex_home_paths_input(&home_path.unwrap_or_else(undefined));
    codex_home_paths::resolve_codex_home_allowlist_candidates(Value::Object(input))
}

/// Port of `firstStringValue` (`apps/server/src/codexGeneratedImages.ts:204`).
pub fn first_string_value(record: Option<Value>, keys: Vec<String>) -> Result<Value, String> {
    let Some(record) = record.as_ref().and_then(as_object) else {
        return Ok(undefined());
    };
    for key in keys {
        if let Some(v) =
            normalize_non_empty_string(record.get(&key).cloned().unwrap_or_else(undefined))?
                .as_str()
                .map(str::to_string)
        {
            return Ok(Value::from(v));
        }
    }
    Ok(undefined())
}

/// Port of `extractCodexGeneratedImagePath` (`apps/server/src/codexGeneratedImages.ts:216`).
pub fn extract_codex_generated_image_path(record: Option<Value>) -> Result<Value, String> {
    first_string_value(
        record,
        IMAGE_PATH_KEYS.iter().map(|s| s.to_string()).collect(),
    )
}

/// Port of `extractCodexGeneratedImageCallId` (`apps/server/src/codexGeneratedImages.ts:222`).
pub fn extract_codex_generated_image_call_id(record: Option<Value>) -> Result<Value, String> {
    first_string_value(
        record,
        IMAGE_CALL_ID_KEYS.iter().map(|s| s.to_string()).collect(),
    )
}

/// Port of `predictedCodexGeneratedImagePath` (`apps/server/src/codexGeneratedImages.ts:228`).
pub fn predicted_codex_generated_image_path(input: Value) -> Result<Value, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'threadId')".into());
    }
    let thread_id = normalize_non_empty_string(
        member(&input, "threadId")
            .cloned()
            .unwrap_or_else(undefined),
    )?;
    let call_id = extract_codex_generated_image_call_id(member(&input, "item").cloned())?;
    let (Some(t), Some(c)) = (thread_id.as_str(), call_id.as_str()) else {
        return Ok(undefined());
    };
    let home = member(&input, "codexHome")
        .cloned()
        .unwrap_or_else(undefined);
    let root = resolve_codex_generated_images_root(if is_undefined(&home) {
        None
    } else {
        Some(home)
    })?;
    Ok(Value::from(join(&[&root, t, &format!("{c}.png")])))
}

/// Port of `annotateCodexGeneratedImagePayload` (`apps/server/src/codexGeneratedImages.ts:242`).
pub fn annotate_codex_generated_image_payload(input: Value) -> Result<Value, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'value')".into());
    }
    let value = member(&input, "value").cloned().unwrap_or_else(undefined);
    let Some(item) = as_object(&value).cloned() else {
        return Ok(value);
    };
    let type_or_kind = match item.get("type") {
        Some(t) if !t.is_null() && !is_undefined(t) => t.clone(),
        _ => item.get("kind").cloned().unwrap_or_else(undefined),
    };
    if !is_codex_generated_image_item_type(type_or_kind)? {
        return Ok(value);
    }
    let existing = extract_codex_generated_image_path(Some(value.clone()))?;
    let generated = if existing.as_str().is_some() {
        existing.clone()
    } else {
        predicted_codex_generated_image_path(input.clone())?
    };
    let mut next = item;
    let mut did_change = false;
    if generated.as_str().is_some() && existing.as_str().is_none() {
        next.insert("saved_path".into(), generated);
        did_change = true;
    }
    if next
        .get("result")
        .and_then(as_string)
        .is_some_and(|s| !s.is_empty())
    {
        next.shift_remove("result");
        next.insert("result_elided_for_relay".into(), Value::from(true));
        did_change = true;
    }
    Ok(if did_change {
        Value::Object(next)
    } else {
        value
    })
}

/// Port of `sanitizeNestedCodexGeneratedImagePayloads` (`apps/server/src/codexGeneratedImages.ts:277`).
pub fn sanitize_nested_codex_generated_image_payloads(input: Value) -> Result<Value, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'value')".into());
    }
    let value = member(&input, "value").cloned().unwrap_or_else(undefined);
    let annotated = annotate_codex_generated_image_payload(input.clone())?;
    let Some(record) = as_object(&annotated).cloned() else {
        return Ok(annotated);
    };
    let mut merged = record.clone();
    let mut has_overrides = false;
    for key in NESTED_PAYLOAD_KEYS {
        let Some(nested) = record.get(key) else {
            continue;
        };
        if as_object(nested).is_none() {
            continue;
        }
        let mut child_input = Map::new();
        child_input.insert("value".into(), nested.clone());
        if let Some(t) = member(&input, "threadId") {
            child_input.insert("threadId".into(), t.clone());
        }
        if let Some(h) = member(&input, "codexHome")
            && truthy(h)
        {
            child_input.insert("codexHome".into(), h.clone());
        }
        let sanitized = sanitize_nested_codex_generated_image_payloads(Value::Object(child_input))?;
        if &sanitized != nested {
            merged.insert(key.into(), sanitized);
            has_overrides = true;
        }
    }
    Ok(if has_overrides || annotated != value {
        Value::Object(merged)
    } else {
        value
    })
}

/// Port of `extractCodexGeneratedImageReference` (`apps/server/src/codexGeneratedImages.ts:316`).
pub fn extract_codex_generated_image_reference(input: Value) -> Result<Value, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'value')".into());
    }
    let value = member(&input, "value").cloned().unwrap_or_else(undefined);
    let Some(item) = as_object(&value).cloned() else {
        return Ok(undefined());
    };
    let type_or_kind = match item.get("type") {
        Some(t) if !t.is_null() && !is_undefined(t) => t.clone(),
        _ => item.get("kind").cloned().unwrap_or_else(undefined),
    };
    if !is_codex_generated_image_item_type(type_or_kind)? {
        return Ok(undefined());
    }
    let existing = extract_codex_generated_image_path(Some(value.clone()))?;
    let image_path = if existing.as_str().is_some() {
        existing
    } else {
        predicted_codex_generated_image_path(input)?
    };
    if !local_preview_files::is_supported_local_image_path(image_path.clone())? {
        return Ok(undefined());
    }
    let mut out = Map::new();
    out.insert("path".into(), image_path);
    let call_id = extract_codex_generated_image_call_id(Some(value))?;
    if truthy(&call_id) {
        out.insert("callId".into(), call_id);
    }
    Ok(Value::Object(out))
}

/// Port of `codexGeneratedImageArtifact` (`apps/server/src/codexGeneratedImages.ts:342`).
pub fn codex_generated_image_artifact(reference: Value) -> Result<Value, String> {
    if reference.is_null() {
        return Err("Cannot read properties of null (reading 'path')".into());
    }
    let mut out = Map::new();
    out.insert("kind".into(), Value::from(ARTIFACT_KIND));
    out.insert(
        "path".into(),
        member(&reference, "path")
            .cloned()
            .unwrap_or_else(undefined),
    );
    if let Some(c) = member(&reference, "callId")
        && truthy(c)
    {
        out.insert("callId".into(), c.clone());
    }
    Ok(Value::Object(out))
}

/// Port of `isCodexGeneratedImageArtifact` (`apps/server/src/codexGeneratedImages.ts:352`).
pub fn is_codex_generated_image_artifact(value: Value) -> Result<bool, String> {
    let Some(record) = as_object(&value) else {
        return Ok(false);
    };
    Ok(record.get("kind") == Some(&Value::from(ARTIFACT_KIND))
        && record
            .get("path")
            .and_then(as_string)
            .is_some_and(|s| !js_trim(s).is_empty()))
}

/// Port of `markdownImagePath` (`apps/server/src/codexGeneratedImages.ts:363`).
pub fn markdown_image_path(file_path: String) -> Result<String, String> {
    let trimmed = js_trim(&file_path);
    if trimmed.contains(')') || trimmed.contains(' ') || trimmed.contains('%') {
        let escaped = trimmed
            .replace('%', "%25")
            .replace('>', "%3E")
            .replace(')', "%29");
        Ok(format!("<{escaped}>"))
    } else {
        Ok(trimmed.to_string())
    }
}

/// Port of `generatedImageMarkdown` (`apps/server/src/codexGeneratedImages.ts:369`).
pub fn generated_image_markdown(file_path: String) -> Result<String, String> {
    Ok(format!(
        "![Generated image]({})",
        markdown_image_path(file_path)?
    ))
}

/// Port of `generatedImagePathFromRuntimeEvent` (`apps/server/src/codexGeneratedImages.ts:377`).
pub fn generated_image_path_from_runtime_event(event: Value) -> Result<Value, String> {
    if event.is_null() {
        return Err("Cannot read properties of null (reading 'type')".into());
    }
    if member(&event, "type").and_then(as_string) != Some("item.completed")
        || member(&event, "payload")
            .and_then(|p| member(p, "itemType"))
            .and_then(as_string)
            != Some("image_generation")
    {
        return Ok(undefined());
    }
    let data = member(&event, "payload")
        .and_then(|p| member(p, "data"))
        .cloned()
        .unwrap_or_else(undefined);
    if is_codex_generated_image_artifact(data.clone())? {
        Ok(member(&data, "path").cloned().unwrap_or_else(undefined))
    } else {
        Ok(undefined())
    }
}
