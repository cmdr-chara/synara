//! The Rust port of `apps/web/src/composerDraftStoreTestFixtures.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};

fn is_nullish(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => true,
        Some(Value::Object(m)) => m.get("$undefined").is_some(),
        _ => false,
    }
}

fn js_truthy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().map(|x| x != 0.0 && !x.is_nan()).unwrap_or(false),
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Object(m)) if m.get("$undefined").is_some() => false,
        Some(v) => crate::js::as_number(v)
            .map(|x| x != 0.0 && !x.is_nan())
            .unwrap_or(true),
    }
}

fn is_js_ws(c: char) -> bool {
    matches!(
        c,
        '\u{0009}'..='\u{000D}'
            | '\u{0020}'
            | '\u{00A0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200A}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202F}'
            | '\u{205F}'
            | '\u{3000}'
            | '\u{FEFF}'
    )
}

/// JS `ToNumber` on a structured value.
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
        Value::Number(_) => crate::js::as_number(v).unwrap_or(f64::NAN),
        Value::String(s) => {
            let t = s.trim_matches(is_js_ws);
            if t.is_empty() {
                0.0
            } else if t.eq_ignore_ascii_case("infinity") || t == "+Infinity" {
                f64::INFINITY
            } else if t == "-Infinity" {
                f64::NEG_INFINITY
            } else if let Some(h) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
                u64::from_str_radix(h, 16)
                    .map(|x| x as f64)
                    .unwrap_or(f64::NAN)
            } else if let Some(b) = t.strip_prefix("0b").or_else(|| t.strip_prefix("0B")) {
                u64::from_str_radix(b, 2)
                    .map(|x| x as f64)
                    .unwrap_or(f64::NAN)
            } else if let Some(o) = t.strip_prefix("0o").or_else(|| t.strip_prefix("0O")) {
                u64::from_str_radix(o, 8)
                    .map(|x| x as f64)
                    .unwrap_or(f64::NAN)
            } else {
                t.parse::<f64>().unwrap_or(f64::NAN)
            }
        }
        Value::Array(a) => match a.len() {
            0 => 0.0,
            1 => js_to_number(&a[0]),
            _ => f64::NAN,
        },
        _ => crate::js::as_number(v).unwrap_or(f64::NAN),
    }
}

/// `new Uint8Array(n).length` — Bun truncates toward zero, throws outside [0, 2**53-1].
fn typed_array_length(input: &Value, key: &str, default: f64) -> Result<f64, String> {
    let v = input.get(key);
    if is_nullish(v) {
        return Ok(default);
    }
    let n = js_to_number(v.unwrap());
    if n < 0.0 {
        return Err("length cannot be negative".into());
    }
    if n > 9007199254740991.0 || n.is_infinite() {
        return Err("length larger than (2 ** 53) - 1".into());
    }
    if n.is_nan() || n == 0.0 {
        return Ok(0.0);
    }
    Ok(n.trunc())
}

/// `input.key` — reading a property of null/undefined throws, primitives give undefined.
fn obj_get<'a>(input: &'a Value, key: &str, first_key: &str) -> Result<Option<&'a Value>, String> {
    if input.is_null() {
        return Err(format!(
            "null is not an object (evaluating 'input.{first_key}')"
        ));
    }
    if crate::js::is_undefined(input) {
        return Err(format!(
            "undefined is not an object (evaluating 'input.{first_key}')"
        ));
    }
    Ok(input.get(key))
}

fn pick(input: &Value, key: &str, first_key: &str, default: Value) -> Result<Value, String> {
    let v = obj_get(input, key, first_key)?;
    if is_nullish(v) {
        Ok(default)
    } else {
        Ok(v.unwrap().clone())
    }
}

/// Raw property pass-through: undefined/missing reads as `undefined`, null stays null.
fn raw(input: &Value, key: &str, first_key: &str) -> Result<Value, String> {
    Ok(match obj_get(input, key, first_key)? {
        Some(v) if !crate::js::is_undefined(v) => v.clone(),
        _ => crate::js::undefined(),
    })
}

/// Port of `stubComposerDraftBrowserStorage` (`apps/web/src/composerDraftStoreTestFixtures.ts:30`).
pub fn stub_composer_draft_browser_storage() -> Result<(), String> {
    Ok(())
}

/// Port of `makeBrowserAnnotation` (`apps/web/src/composerDraftStoreTestFixtures.ts:35`).
pub fn make_browser_annotation(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let k = "id";
    Ok(json!({
        "id": raw(&input, "id", k)?,
        "ordinal": pick(&input, "ordinal", k, json!(1))?,
        "tabId": pick(&input, "tabId", k, json!("tab-1"))?,
        "documentKey": pick(&input, "documentKey", k, json!(format!("sha256:{}", "0".repeat(64))))?,
        "source": pick(&input, "source", k, json!({
            "url": "https://example.test/docs",
            "pageTitle": "Docs",
        }))?,
        "selector": pick(&input, "selector", k, json!("#save"))?,
        "tagName": pick(&input, "tagName", k, json!("button"))?,
        "role": pick(&input, "role", k, json!("button"))?,
        "name": pick(&input, "name", k, json!("Save"))?,
        "text": pick(&input, "text", k, json!("Save"))?,
        "fingerprint": pick(&input, "fingerprint", k, json!("button|save"))?,
        "comment": pick(&input, "comment", k, json!(""))?,
        "capturedAt": pick(&input, "capturedAt", k, json!("2026-07-23T10:00:00.000Z"))?,
    }))
}

/// Port of `makeImage` (`apps/web/src/composerDraftStoreTestFixtures.ts:58`).
pub fn make_image(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let k = "name";
    let name = pick(&input, "name", k, json!("image.png"))?;
    let mime = pick(&input, "mimeType", k, json!("image/png"))?;
    let size = typed_array_length(&input, "sizeBytes", 4.0)?;
    obj_get(&input, "lastModified", k)?;
    Ok(json!({
        "type": "image",
        "id": raw(&input, "id", k)?,
        "name": name,
        "mimeType": mime,
        "sizeBytes": size,
        "previewUrl": raw(&input, "previewUrl", k)?,
        "file": {},
    }))
}

/// Port of `makeFile` (`apps/web/src/composerDraftStoreTestFixtures.ts:85`).
pub fn make_file(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let k = "name";
    let name = pick(&input, "name", k, json!("notes.txt"))?;
    let mime = pick(&input, "mimeType", k, json!("text/plain"))?;
    let size = typed_array_length(&input, "sizeBytes", 4.0)?;
    obj_get(&input, "lastModified", k)?;
    Ok(json!({
        "type": "file",
        "id": raw(&input, "id", k)?,
        "name": name,
        "mimeType": mime,
        "sizeBytes": size,
        "file": {},
    }))
}

/// Port of `makeTerminalContext` (`apps/web/src/composerDraftStoreTestFixtures.ts:110`).
pub fn make_terminal_context(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let k = "id";
    Ok(json!({
        "id": raw(&input, "id", k)?,
        "threadId": "thread-dedupe",
        "terminalId": pick(&input, "terminalId", k, json!("default"))?,
        "terminalLabel": pick(&input, "terminalLabel", k, json!("Terminal 1"))?,
        "lineStart": pick(&input, "lineStart", k, json!(4))?,
        "lineEnd": pick(&input, "lineEnd", k, json!(5))?,
        "text": pick(&input, "text", k, json!("git status\nOn branch main"))?,
        "createdAt": "2026-03-13T12:00:00.000Z",
    }))
}

/// Port of `makeQueuedTurn` (`apps/web/src/composerDraftStoreTestFixtures.ts:130`).
pub fn make_queued_turn(id: String) -> Result<serde_json::Value, String> {
    Ok(json!({
        "id": id,
        "kind": "plan-follow-up",
        "createdAt": "2026-03-13T12:00:00.000Z",
        "previewText": format!("queued {id}"),
        "text": format!("queued {id}"),
        "interactionMode": "plan",
        "selectedProvider": "codex",
        "selectedModel": "gpt-5",
        "selectedPromptEffort": null,
        "modelSelection": {
            "provider": "codex",
            "model": "gpt-5",
        },
        "runtimeMode": "full-access",
    }))
}

/// Port of `makeQueuedChatTurn` (`apps/web/src/composerDraftStoreTestFixtures.ts:149`).
pub fn make_queued_chat_turn(
    id: String,
    image: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let images = if js_truthy(image.as_ref()) {
        json!([image])
    } else {
        json!([])
    };
    Ok(json!({
        "id": id,
        "kind": "chat",
        "createdAt": "2026-03-13T12:00:00.000Z",
        "previewText": format!("queued chat {id}"),
        "prompt": "queued chat prompt",
        "images": images,
        "files": [],
        "assistantSelections": [],
        "browserAnnotations": [],
        "terminalContexts": [make_terminal_context(json!({ "id": format!("ctx-{id}") }))?],
        "fileComments": [],
        "pastedTexts": [],
        "pullRequestContexts": [],
        "skills": [{ "name": "check-code", "path": "/skills/check-code" }],
        "mentions": [{ "name": "repo", "path": "/mentions/repo" }],
        "selectedProvider": "codex",
        "selectedModel": "gpt-5",
        "selectedPromptEffort": null,
        "modelSelection": {
            "provider": "codex",
            "model": "gpt-5",
        },
        "sourceProposedPlan": {
            "threadId": "thread-source-plan",
            "planId": "plan-1",
        },
        "runtimeMode": "full-access",
        "interactionMode": "default",
        "envMode": "local",
    }))
}

/// Port of `resetComposerDraftStore` (`apps/web/src/composerDraftStoreTestFixtures.ts:187`).
pub fn reset_composer_draft_store() -> Result<serde_json::Value, String> {
    Ok(crate::js::undefined())
}

/// Port of `modelSelection` (`apps/web/src/composerDraftStoreTestFixtures.ts:191`).
pub fn model_selection(
    provider: serde_json::Value,
    model: String,
    options: Option<serde_json::Value>,
    instance_id: Option<String>,
) -> Result<serde_json::Value, String> {
    let mut out = Map::new();
    out.insert("provider".into(), provider);
    if js_truthy(
        instance_id
            .as_ref()
            .map(|s| Value::String(s.clone()))
            .as_ref(),
    ) {
        out.insert("instanceId".into(), json!(instance_id));
    }
    out.insert("model".into(), json!(model));
    if js_truthy(options.as_ref()) {
        out.insert("options".into(), options.unwrap());
    }
    Ok(Value::Object(out))
}

/// Port of `providerModelOptions` (`apps/web/src/composerDraftStoreTestFixtures.ts:205`).
pub fn provider_model_options(options: serde_json::Value) -> Result<serde_json::Value, String> {
    Ok(options)
}
