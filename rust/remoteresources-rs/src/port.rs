//! The Rust port of `packages/shared/src/remoteResources.ts` (clean-room port).

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};

#[rustfmt::skip]
fn prop<'a>(v: &'a Value, k: &str) -> Option<&'a Value> { v.as_object()?.get(k) }

#[rustfmt::skip]
fn js_trim(s: &str) -> &str { s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}') }

fn truthy(v: &Value) -> bool {
    if crate::js::is_undefined(v) {
        return false;
    }
    match v {
        Value::Null | Value::Bool(false) => false,
        Value::Bool(true) | Value::Array(_) => true,
        Value::String(s) => !s.is_empty(),
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::Object(o) => match o.get("$num").and_then(Value::as_str) {
            Some(m) => !matches!(m, "0" | "NaN"),
            None => true,
        },
    }
}

// JS `String(value)` for route params.
fn js_str(v: &Value) -> String {
    if crate::js::is_undefined(v) {
        return "undefined".into();
    }
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => js_num(n.as_f64().unwrap_or(0.0)),
        Value::String(s) => s.clone(),
        Value::Array(a) => a.iter().map(js_str).collect::<Vec<_>>().join(","),
        Value::Object(o) => o
            .get("$num")
            .and_then(Value::as_str)
            .map_or("[object Object]".into(), |m| m.into()),
    }
}

// JS `String(number)`.
fn js_num(x: f64) -> String {
    if x.is_nan() {
        return "NaN".into();
    }
    if x.is_infinite() {
        return if x < 0.0 { "-Infinity" } else { "Infinity" }.into();
    }
    if x == 0.0 || (1e-6..1e21).contains(&x.abs()) {
        return format!("{x}");
    }
    let s = format!("{x:e}");
    let (m, e) = s.split_once('e').unwrap();
    format!(
        "{}e{}{e}",
        m.trim_end_matches('0').trim_end_matches('.'),
        if e.starts_with('-') { "" } else { "+" }
    )
}

fn pct(s: &str, unres: &[u8], plus: bool) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        if b.is_ascii_alphanumeric() || unres.contains(&b) {
            out.push(b as char)
        } else if b == b' ' && plus {
            out.push('+')
        } else {
            out.push_str(&format!("%{b:02X}"))
        }
    }
    out
}

const FE: &[u8] = b"*-._"; // form-encode unreserved
const UE: &[u8] = b"-_.!~*'()"; // encodeURIComponent unreserved

#[rustfmt::skip]
pub fn remote_resource_route(resource: Value) -> Result<Value, String> {
    if resource.is_null() {
        return Err("Cannot read properties of null (reading 'kind')".into());
    }
    if crate::js::is_undefined(&resource) {
        return Err("Cannot read properties of undefined (reading 'kind')".into());
    }
    let p = |k: &str| pct(&prop(&resource, k).map_or("undefined".to_string(), js_str), FE, true);
    let route = |path: String, method: &str, max: f64| json!({"path": path, "method": method, "maxBodyBytes": max});
    let opt = |k: &str, path: &mut String| {
        if let Some(v) = prop(&resource, k).filter(|v| truthy(v)) {
            path.push_str(&format!("&{k}={}", pct(&js_str(v), FE, true)));
        }
    };
    Ok(match prop(&resource, "kind").and_then(Value::as_str) {
        Some("attachment") => route(format!("/attachments/{}", p("attachmentId")), "GET", 0.0),
        Some("attachment-upload") => route(
            format!("/api/attachments/upload?threadId={}&type={}&name={}&mimeType={}", p("threadId"), p("type"), p("name"), p("mimeType")),
            "POST",
            if prop(&resource, "type").and_then(Value::as_str) == Some("image") { 10_485_760.0 } else { 26_214_400.0 },
        ),
        Some("attachment-cancel") => route("/api/attachments/cancel".into(), "POST", 4096.0),
        Some("workspace-preview") => {
            let mut path = format!("/api/local-image?path={}", p("path"));
            opt("cwd", &mut path);
            route(path, "GET", 0.0)
        }
        Some("thread-export") => route(format!("/api/thread-export?threadId={}", p("threadId")), "GET", 0.0),
        Some("project-favicon") => route(format!("/api/project-favicon?cwd={}&fallback=none", p("cwd")), "GET", 0.0),
        Some("site-favicon") => route(format!("/api/site-favicon?domain={}", p("domain")), "GET", 0.0),
        Some("editor-icon") => route(format!("/api/editor-icon?id={}", p("editorId")), "GET", 0.0),
        Some("voice-upload") => {
            let mut path = format!("/api/voice/transcribe?provider={}&cwd={}&mimeType={}&sampleRateHz={}&durationMs={}", p("provider"), p("cwd"), p("mimeType"), p("sampleRateHz"), p("durationMs"));
            opt("threadId", &mut path);
            route(path, "POST", 10_485_760.0)
        }
        _ => route("undefined".into(), "GET", 0.0),
    })
}

const U: &str = "{ readonly \"kind\": \"attachment\", ... } | { readonly \"kind\": \"attachment-upload\", ... } | { readonly \"kind\": \"attachment-cancel\", ... } | { readonly \"kind\": \"workspace-preview\", ... } | { readonly \"kind\": \"thread-export\", ... } | { readonly \"kind\": \"project-favicon\", ... } | { readonly \"kind\": \"site-favicon\", ... } | { readonly \"kind\": \"editor-icon\", ... } | { readonly \"kind\": \"voice-upload\", ... }";

#[rustfmt::skip]
fn jc(v: &Value) -> String { serde_json::to_string(v).unwrap_or_default() }

#[rustfmt::skip]
fn at(f: &str) -> String { format!("\n  at [\"resource\"][\"{f}\"]") }

#[rustfmt::skip]
fn fval<'a>(o: &'a Map<String, Value>, f: &str) -> Result<&'a Value, String> {
    o.get(f).ok_or_else(|| format!("Missing key{}", at(f)))
}

#[rustfmt::skip]
fn fstr<'a>(o: &'a Map<String, Value>, f: &str) -> Result<&'a str, String> {
    fval(o, f)?.as_str().ok_or_else(|| format!("Expected string, got {}{}", jc(&o[f]), at(f)))
}

#[rustfmt::skip]
fn min1(s: &str, f: &str) -> Result<String, String> {
    let t = js_trim(s);
    if t.is_empty() { Err(format!("Expected a value with a length of at least 1, got \"{t}\"{}", at(f))) } else { Ok(t.into()) }
}

#[rustfmt::skip]
fn m1(o: &Map<String, Value>, f: &str) -> Result<String, String> { min1(fstr(o, f)?, f) }

#[rustfmt::skip]
fn opt_min1(o: &Map<String, Value>, f: &str) -> Result<Option<String>, String> {
    match o.get(f) {
        None => Ok(None),
        Some(v) => Ok(Some(min1(v.as_str().ok_or_else(|| format!("Expected string, got {}{}", jc(v), at(f)))?, f)?)),
    }
}

#[rustfmt::skip]
fn r1(o: &Map<String, Value>, f: &str, pat: &str, extra: &[u8], max: usize) -> Result<String, String> {
    let s = fstr(o, f)?;
    if !s.is_empty() && s.len() <= max && s.bytes().all(|b| b.is_ascii_alphanumeric() || extra.contains(&b)) {
        Ok(s.into())
    } else {
        Err(format!("Expected a string matching the RegExp {pat}, got {}{}", jc(&o[f]), at(f)))
    }
}

#[rustfmt::skip]
fn fnum(o: &Map<String, Value>, f: &str) -> Result<f64, String> {
    fval(o, f)?.as_f64().ok_or_else(|| format!("Expected number, got {}{}", jc(&o[f]), at(f)))
}

#[rustfmt::skip]
fn rng(x: f64, lo: f64, hi: f64, f: &str) -> Result<(), String> {
    if (lo..=hi).contains(&x) { Ok(()) } else {
        Err(format!("Expected a value between {} and {}, got {}{}", js_num(lo), js_num(hi), js_num(x), at(f)))
    }
}

#[rustfmt::skip]
fn set(out: &mut Map<String, Value>, f: &str, v: impl Into<Value>) { out.insert(f.into(), v.into()); }

const RX256: &str = "^[A-Za-z0-9_-]{1,256}$";

#[rustfmt::skip]
fn validate_resource(res: &Value) -> Result<Value, String> {
    let Some(o) = res.as_object() else {
        return Err(format!("Expected {U}, got {}\n  at [\"resource\"]", jc(res)));
    };
    let Some(kind) = o.get("kind").and_then(Value::as_str).filter(|k| {
        matches!(*k, "attachment" | "attachment-upload" | "attachment-cancel" | "workspace-preview" | "thread-export" | "project-favicon" | "site-favicon" | "editor-icon" | "voice-upload")
    }) else {
        return Err(format!("Expected {U}, got {}\n  at [\"resource\"]", jc(res)));
    };
    let mut out = Map::new();
    set(&mut out, "kind", kind);
    match kind {
        "attachment" => set(&mut out, "attachmentId", r1(o, "attachmentId", RX256, b"_-", 256)?),
        "attachment-upload" => {
            let t = m1(o, "threadId")?;
            let ty = fstr(o, "type")?;
            if !matches!(ty, "image" | "file") {
                return Err(format!("Expected \"image\" | \"file\", got {}{}", jc(&o["type"]), at("type")));
            }
            set(&mut out, "threadId", t);
            set(&mut out, "type", ty);
            set(&mut out, "name", m1(o, "name")?);
            set(&mut out, "mimeType", m1(o, "mimeType")?);
        }
        "attachment-cancel" => {}
        "workspace-preview" => {
            if let Some(c) = opt_min1(o, "cwd")? {
                set(&mut out, "cwd", c);
            }
            set(&mut out, "path", m1(o, "path")?);
        }
        "thread-export" => set(&mut out, "threadId", m1(o, "threadId")?),
        "project-favicon" => set(&mut out, "cwd", m1(o, "cwd")?),
        "site-favicon" => set(&mut out, "domain", r1(o, "domain", "^[A-Za-z0-9.-]{1,253}$", b".-", 253)?),
        "editor-icon" => set(&mut out, "editorId", r1(o, "editorId", RX256, b"_-", 256)?),
        _ => {
            let pr = r1(o, "provider", RX256, b"_-", 256)?;
            let cw = m1(o, "cwd")?;
            let th = opt_min1(o, "threadId")?;
            let mt = m1(o, "mimeType")?;
            let sr = fnum(o, "sampleRateHz")?;
            if sr.fract() != 0.0 {
                return Err(format!("Expected an integer, got {}{}", js_num(sr), at("sampleRateHz")));
            }
            rng(sr, 8000.0, 192000.0, "sampleRateHz")?;
            rng(fnum(o, "durationMs")?, 1.0, 3_600_000.0, "durationMs")?;
            set(&mut out, "provider", pr);
            set(&mut out, "cwd", cw);
            if let Some(t) = th {
                set(&mut out, "threadId", t);
            }
            set(&mut out, "mimeType", mt);
            out.insert("sampleRateHz".into(), o["sampleRateHz"].clone());
            out.insert("durationMs".into(), o["durationMs"].clone());
        }
    }
    Ok(Value::Object(out))
}

pub fn decode_remote_resource_reference(value: String) -> Result<Value, String> {
    let parsed: Value = match serde_json::from_str(&value) {
        Ok(v) => v,
        Err(_) => return Err(v8::v8_err(&value)),
    };
    let Some(o) = parsed.as_object() else {
        return Err(format!("Expected object, got {}", jc(&parsed)));
    };
    let env = match o.get("environmentId") {
        None => return Err("Missing key\n  at [\"environmentId\"]".into()),
        Some(v) => match v.as_str() {
            None => {
                return Err(format!(
                    "Expected string, got {}\n  at [\"environmentId\"]",
                    jc(v)
                ));
            }
            Some(s) => {
                let t = js_trim(s);
                if t.is_empty() {
                    return Err("Expected a value with a length of at least 1, got \"\"\n  at [\"environmentId\"]".into());
                }
                t.to_string()
            }
        },
    };
    let Some(res) = o.get("resource") else {
        return Err("Missing key\n  at [\"resource\"]".into());
    };
    Ok(json!({"environmentId": env, "resource": validate_resource(res)?}))
}

pub fn remote_resource_presentation_url(
    environment_id: String,
    encoded_reference: String,
) -> Result<String, String> {
    Ok(format!(
        "synara://remote/{}/v2/resource/{}",
        pct(&environment_id, UE, false),
        pct(&encoded_reference, UE, false)
    ))
}

mod v8;
