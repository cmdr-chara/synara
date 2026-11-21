//! The Rust port of the pure helpers in `apps/desktop/src/audioLevelMonitor.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

use crate::js;

const MAX_MICROPHONE_ID_LENGTH: usize = 512;

fn utf16_len(s: &str) -> usize {
    s.chars().map(|c| c.len_utf16()).sum()
}

/// `typeof x === "object" && x !== null` plus plain JSON.parse results.
fn is_object(v: &Value) -> bool {
    matches!(v, Value::Object(_) | Value::Array(_))
}

fn member(v: &Value, key: &str) -> Value {
    match v {
        Value::Object(o) => o.get(key).cloned().unwrap_or_else(js::undefined),
        _ => js::undefined(),
    }
}

fn strict_eq(a: &Value, b: &Value) -> bool {
    if js::is_undefined(a) && js::is_undefined(b) {
        return true;
    }
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        _ => false,
    }
}

fn is_string(v: &Value) -> bool {
    matches!(v, Value::String(_))
}

/// `for (const x of iterable)` items: arrays, string chars, `$set`, `$map` entries.
fn iterate(v: &Value, name: &str) -> Result<Vec<Value>, String> {
    match v {
        Value::Array(a) => Ok(a.clone()),
        Value::String(s) => Ok(s.chars().map(|c| json!(c.to_string())).collect()),
        Value::Object(o) => {
            if let Some(Value::Array(items)) = o.get("$set") {
                return Ok(items.clone());
            }
            if let Some(Value::Array(entries)) = o.get("$map") {
                return Ok(entries.clone());
            }
            Err(format!("{name} is not iterable"))
        }
        _ => Err(format!("{name} is not iterable")),
    }
}

/// Port of `parseAudioLevelMessage` (`apps/desktop/src/audioLevelMonitor.ts:27`).
pub fn parse_audio_level_message(line: String) -> Result<Option<serde_json::Value>, String> {
    let parsed: Value = match serde_json::from_str(&line) {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };
    if !is_object(&parsed) || parsed.is_null() {
        return Ok(None);
    }
    let ty = member(&parsed, "type");
    if strict_eq(&ty, &json!("ready")) {
        return Ok(Some(json!({ "type": "ready" })));
    }
    if strict_eq(&ty, &json!("audio-level")) {
        let level = member(&parsed, "level");
        if let Some(n) = js::as_number(&level)
            && n.is_finite()
        {
            return Ok(Some(
                json!({ "type": "audio-level", "level": n.clamp(0.0, 1.0) }),
            ));
        }
        return Ok(None);
    }
    if strict_eq(&ty, &json!("error")) {
        let code = member(&parsed, "code");
        if is_string(&code) {
            let message = member(&parsed, "message");
            let message = if is_string(&message) {
                message
            } else {
                code.clone()
            };
            return Ok(Some(
                json!({ "type": "error", "code": code, "message": message }),
            ));
        }
        return Ok(None);
    }
    Ok(None)
}

/// Port of `combineAudioLevelSources` (`apps/desktop/src/audioLevelMonitor.ts:62`).
pub fn combine_audio_level_sources(sources: serde_json::Value) -> Result<Option<String>, String> {
    let mut system = false;
    let mut microphone = false;
    for source in iterate(&sources, "sources")? {
        if !strict_eq(&source, &json!("microphone")) {
            system = true;
        }
        if !strict_eq(&source, &json!("system")) {
            microphone = true;
        }
    }
    Ok(if system && microphone {
        Some("both".to_string())
    } else if system {
        Some("system".to_string())
    } else if microphone {
        Some("microphone".to_string())
    } else {
        None
    })
}

/// Port of `combineAudioLevelSubscriptions` (`apps/desktop/src/audioLevelMonitor.ts:77`).
pub fn combine_audio_level_subscriptions(
    subscriptions: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let all = iterate(&subscriptions, "subscriptions")?;
    let mut system = false;
    let mut microphone = false;
    for sub in &all {
        if sub.is_null() {
            return Err("Cannot read properties of null (reading 'source')".to_string());
        }
        if js::is_undefined(sub) {
            return Err("Cannot read properties of undefined (reading 'source')".to_string());
        }
        let source = member(sub, "source");
        if !strict_eq(&source, &json!("microphone")) {
            system = true;
        }
        if !strict_eq(&source, &json!("system")) {
            microphone = true;
        }
    }
    let source = if system && microphone {
        "both"
    } else if system {
        "system"
    } else if microphone {
        "microphone"
    } else {
        return Ok(None);
    };
    let mut microphone_id = js::undefined();
    if source != "system" {
        for sub in &all {
            if !strict_eq(&member(sub, "source"), &json!("system")) {
                microphone_id = member(sub, "microphoneId");
            }
        }
    }
    Ok(Some(
        json!({ "source": source, "microphoneId": microphone_id }),
    ))
}

/// Port of `parseAudioInputsMessage` (`apps/desktop/src/audioLevelMonitor.ts:110`).
pub fn parse_audio_inputs_message(output: String) -> Result<Vec<serde_json::Value>, String> {
    for line in output.split('\n') {
        let parsed: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if !is_object(&parsed) || parsed.is_null() {
            continue;
        }
        if !strict_eq(&member(&parsed, "type"), &json!("audio-inputs")) {
            continue;
        }
        let Value::Array(raw_devices) = member(&parsed, "devices") else {
            continue;
        };
        let mut devices = Vec::new();
        for raw in &raw_devices {
            if !is_object(raw) {
                continue;
            }
            let id = member(raw, "id");
            let Value::String(id) = &id else {
                continue;
            };
            if utf16_len(id) == 0 || utf16_len(id) > MAX_MICROPHONE_ID_LENGTH {
                continue;
            }
            let name = member(raw, "name");
            let name = if is_string(&name) && utf16_len(name.as_str().unwrap_or_default()) > 0 {
                name
            } else {
                Value::String(id.clone())
            };
            devices.push(json!({
                "id": id,
                "name": name,
                "bluetooth": strict_eq(&member(raw, "bluetooth"), &json!(true)),
                "default": strict_eq(&member(raw, "default"), &json!(true)),
            }));
        }
        return Ok(devices);
    }
    Ok(Vec::new())
}

/// Port of `listAudioInputDevices` (`apps/desktop/src/audioLevelMonitor.ts:141`).
/// A non-function `execFile`, an empty helper path, or any spawn/output failure
/// resolves to `[]` (or throws where the original does).
pub async fn list_audio_input_devices(
    helper_path: String,
    exec_file: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    if !js::is_undefined(&exec_file) {
        return Err("execFile is not a function".to_string());
    }
    if helper_path.is_empty() {
        return Err("The argument 'file' cannot be empty. Received ''".to_string());
    }
    let mut child = match std::process::Command::new(&helper_path)
        .arg("--list-audio-inputs")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .stdin(std::process::Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return Ok(Vec::new()),
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(5_000);
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break Some(s),
            Ok(None) => {
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    break None;
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Err(_) => break None,
        }
    };
    let Some(_status) = status else {
        return Ok(Vec::new());
    };
    let mut stdout = String::new();
    if let Some(mut out) = child.stdout.take() {
        use std::io::Read;
        if out.read_to_string(&mut stdout).is_err() {
            return Ok(Vec::new());
        }
    }
    parse_audio_inputs_message(stdout)
}
