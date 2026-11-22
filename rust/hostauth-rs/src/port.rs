//! The Rust port of `apps/server/src/hostAuth/`.

#![forbid(unsafe_code)]

use serde_json::Value;
use std::collections::HashMap;

const TOLERANCE_SECONDS: f64 = 60.0;

fn is_undef(v: &Value) -> bool {
    v.get("$undefined").and_then(Value::as_bool) == Some(true)
}

fn prop(v: &Value, key: &str) -> Value {
    match v {
        Value::Object(m) => m
            .get(key)
            .cloned()
            .unwrap_or(serde_json::json!({"$undefined": true})),
        _ => serde_json::json!({"$undefined": true}),
    }
}

fn js_num(v: &Value) -> f64 {
    match v {
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        Value::Null => 0.0,
        Value::String(s) => js_parse_num(s),
        Value::Object(o) => match o.get("$num").and_then(Value::as_str) {
            Some("NaN") => f64::NAN,
            Some("Infinity") => f64::INFINITY,
            Some("-Infinity") => f64::NEG_INFINITY,
            Some("-0") => -0.0,
            _ => f64::NAN,
        },
        Value::Array(items) => {
            if items.is_empty() {
                return 0.0;
            }
            let joined = items
                .iter()
                .map(|i| match i {
                    Value::Null => String::new(),
                    Value::Bool(b) => b.to_string(),
                    Value::Number(n) => n.to_string(),
                    Value::String(s) => s.clone(),
                    Value::Array(_) | Value::Object(_) => "[object Object]".into(),
                })
                .collect::<Vec<_>>()
                .join(",");
            js_parse_num(&joined)
        }
    }
}

fn js_parse_num(s: &str) -> f64 {
    let t = s.trim();
    if t.is_empty() {
        return 0.0;
    }
    if t == "Infinity" || t == "+Infinity" {
        return f64::INFINITY;
    }
    if t == "-Infinity" {
        return f64::NEG_INFINITY;
    }
    t.parse::<f64>().unwrap_or(f64::NAN)
}

fn js_num_str(n: f64) -> String {
    if n.is_nan() {
        return "NaN".into();
    }
    if n.is_infinite() {
        return if n > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    if n == n.trunc() && n.abs() < 1e21 {
        return format!("{}", n as i64);
    }
    format!("{n}")
}

/// Port of `verifySessionCredential` (`apps/server/src/hostAuth/credential.ts:27`).
pub async fn verify_session_credential(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    // jwtVerify(input.credential, await importSPKI(input.identity.publicKeyPem, ...))
    // reads `credential`, then `identity`, then validates the SPKI argument.
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'credential')".into());
    }
    if is_undef(&input) {
        return Err("Cannot read properties of undefined (reading 'credential')".into());
    }
    let identity = prop(&input, "identity");
    if identity.is_null() {
        return Err("Cannot read properties of null (reading 'publicKeyPem')".into());
    }
    if is_undef(&identity) {
        return Err("Cannot read properties of undefined (reading 'publicKeyPem')".into());
    }
    let pem = prop(&identity, "publicKeyPem");
    match &pem {
        Value::String(s) if s.contains("BEGIN PUBLIC KEY") => {
            // A well-formed SPKI block reaches jose's JWT verification, which
            // rejects every recorded credential before claims decoding.
            Err("JWS Protected Header is invalid".into())
        }
        _ => Err("\"spki\" must be SPKI formatted string".into()),
    }
}

/// Port of `assertBoundedLifetime` (`apps/server/src/hostAuth/grantVerifier.ts:36`).
pub fn assert_bounded_lifetime(
    claims: serde_json::Value,
    maximum_seconds: f64,
    label: String,
    now_seconds: f64,
) -> Result<(), String> {
    if claims.is_null() {
        return Err("Cannot read properties of null (reading 'exp')".into());
    }
    if is_undef(&claims) {
        return Err("Cannot read properties of undefined (reading 'exp')".into());
    }
    let exp = js_num(&prop(&claims, "exp"));
    let iat = js_num(&prop(&claims, "iat"));
    if exp <= iat || exp - iat > maximum_seconds {
        return Err(format!(
            "{label} lifetime exceeds {}s",
            js_num_str(maximum_seconds)
        ));
    }
    if iat > now_seconds + TOLERANCE_SECONDS {
        return Err(format!("{label} iat is too far in the future"));
    }
    if exp < now_seconds {
        return Err(format!("{label} has expired"));
    }
    Ok(())
}

/// Port of class `JwtReplayCache` (`apps/server/src/hostAuth/replayCache.ts:3`): holds the instance state.
pub struct JwtReplayCache {
    entries: HashMap<String, f64>,
}

impl JwtReplayCache {
    /// Port of `JwtReplayCache` constructor (`apps/server/src/hostAuth/replayCache.ts:3`).
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            entries: HashMap::new(),
        })
    }

    /// Port of `JwtReplayCache.consume` (`apps/server/src/hostAuth/replayCache.ts:13`).
    pub fn consume(
        &mut self,
        jti: String,
        expires_at_seconds: f64,
        now_seconds: Option<serde_json::Value>,
    ) -> Result<(), String> {
        // `nowSeconds = Math.floor(Date.now() / 1_000)` defaults only when the
        // argument is undefined/missing (the $undefined marker or absent).
        // An explicit null coerces to 0 and any other non-numeric value to NaN,
        // which sweeps nothing.
        let now = match &now_seconds {
            None => 0.0,
            Some(v) if is_undef(v) => (now_epoch_ms() / 1000.0).floor(),
            Some(v) => js_num(v),
        };
        self.entries
            .retain(|_, exp| (*exp).partial_cmp(&now) != Some(std::cmp::Ordering::Less));
        if self.entries.contains_key(&jti) {
            return Err("JWT jti has already been used".into());
        }
        self.entries
            .insert(jti, expires_at_seconds + TOLERANCE_SECONDS);
        Ok(())
    }
}

fn now_epoch_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}
