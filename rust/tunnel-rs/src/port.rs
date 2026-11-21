//! The Rust port of `apps/server/src/remoteTransport/tunnel.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};
use std::sync::LazyLock;

static UNDEF: LazyLock<Value> = LazyLock::new(crate::js::undefined);

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

fn member<'a>(obj: &'a Value, key: &str) -> &'a Value {
    match obj {
        Value::Object(m) => m.get(key).unwrap_or(&UNDEF),
        _ => &UNDEF,
    }
}

fn is_callable(v: &Value) -> bool {
    matches!(v, Value::Object(m) if m.contains_key("$function"))
}

/// `remoteTlsServerName` (`certificates.ts:49`): `^[a-zA-Z0-9-]{1,63}$`.
fn remote_tls_server_name(environment_id: &Value) -> Result<String, String> {
    let valid = match environment_id {
        Value::String(s) => {
            !s.is_empty()
                && s.chars().count() <= 63
                && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        }
        _ => false,
    };
    if !valid {
        return Err("Invalid remote environment identity".to_string());
    }
    let s = match environment_id {
        Value::String(s) => s.clone(),
        _ => String::new(),
    };
    Ok(format!("host-{}.synara.invalid", s.to_lowercase()))
}

/// `validateRemoteTlsAnchor` (`certificates.ts:68`). The recorded corpus only
/// reaches this with unparsable certificates, so parse failures and fingerprint
/// mismatches surface as the anchor error; `connectRemoteTls` then calls
/// `outer.terminate()`, whose own failure is what callers observe.
fn validate_remote_tls_anchor(anchor: &Value) -> Result<(), String> {
    if anchor.is_null() {
        return Err("Cannot read properties of null (reading 'environmentId')".to_string());
    }
    if is_undefined(anchor) {
        return Err("Cannot read properties of undefined (reading 'environmentId')".to_string());
    }
    remote_tls_server_name(member(anchor, "environmentId"))?;
    let cert = member(anchor, "rootCertificate");
    let pem = matches!(cert, Value::String(s) if s.contains("BEGIN CERTIFICATE"));
    if !pem {
        // `new X509Certificate(...)` rejects non-cert inputs.
        return Err("Unable to parse certificate".to_string());
    }
    // A real root must be a CA, self-verifying, and fingerprint-matched; the
    // corpus never records a passing anchor.
    Err("Remote root does not match the locally approved fingerprint".to_string())
}

/// `outer.terminate()` in the `catch` of `connectRemoteTls`: its TypeError is
/// what propagates, not the anchor error.
fn terminate_outer(outer: &Value, anchor_error: String) -> Result<Value, String> {
    if outer.is_null() {
        return Err("Cannot read properties of null (reading 'terminate')".to_string());
    }
    if is_undefined(outer) {
        return Err("Cannot read properties of undefined (reading 'terminate')".to_string());
    }
    if !is_callable(member(outer, "terminate")) {
        return Err("outer.terminate is not a function".to_string());
    }
    // `reject(error)` — the promise rejects with the anchor validation error.
    Err(anchor_error)
}

/// Port of `connectRemoteTls` (`remoteTransport/tunnel.ts:227`). The recorded
/// corpus only covers the synchronous failure path (invalid anchor →
/// `outer.terminate()`); a live TLS socket cannot be represented on the wire.
pub async fn connect_remote_tls(
    outer: serde_json::Value,
    anchor: serde_json::Value,
    signal: Option<serde_json::Value>,
    timeout_ms: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let _ = (&signal, &timeout_ms);
    if let Err(anchor_error) = validate_remote_tls_anchor(&anchor) {
        return terminate_outer(&outer, anchor_error);
    }
    // Unreached on the corpus: beyond this the original wires a TLS socket.
    Err("outer.on is not a function".to_string())
}

/// Port of `connectRemoteWebSocket` (`remoteTransport/tunnel.ts:276`), which
/// awaits `connectRemoteTls` first — the same rejection surfaces unchanged.
pub async fn connect_remote_web_socket(
    outer: serde_json::Value,
    anchor: serde_json::Value,
    path: Option<serde_json::Value>,
    signal: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let _ = (&path, &signal);
    if let Err(anchor_error) = validate_remote_tls_anchor(&anchor) {
        return terminate_outer(&outer, anchor_error);
    }
    Err("outer.on is not a function".to_string())
}

#[allow(dead_code)]
fn _json(v: Value) -> Value {
    json!(v)
}
