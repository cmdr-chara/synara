//! The Rust port of `apps/desktop/src/backendStartupBlock.ts`.
//!
//! Reconstructed clean-room from cases.jsonl (220+ recorded cases) and black-box
//! probing of the original class. `BackendStartupBlockDetector` accumulates
//! backend output chunks into one chronological log (the `source` argument only
//! feeds `end`'s decoder lookup); `read()` scans lines for the
//! `SYNARA_MIGRATION_DIVERGENCE_CONSENT_REQUIRED=` and
//! `SYNARA_MIGRATION_SCHEMA_TOO_NEW=` markers and reports the detected startup
//! block.

#![forbid(unsafe_code)]

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

const DIVERGENCE_PREFIX: &str = "SYNARA_MIGRATION_DIVERGENCE_CONSENT_REQUIRED=";
const SCHEMA_TOO_NEW_PREFIX: &str = "SYNARA_MIGRATION_SCHEMA_TOO_NEW=";

/// Fields hashed into the consent token — `backupDirectory` and `consentToken`
/// are deliberately excluded.
const TOKEN_FIELDS: [&str; 9] = [
    "version",
    "databasePath",
    "sourceVersion",
    "targetVersion",
    "firstDivergedId",
    "expectedName",
    "recordedName",
    "highWaterMark",
    "lineageFingerprint",
];

/// Port of `BackendStartupBlockDetector` (`apps/desktop/src/backendStartupBlock.ts`).
pub struct BackendStartupBlockDetector {
    log: String,
    frozen: bool,
}

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

/// Object property read; `None` = JS `undefined`.
fn field<'a>(v: &'a Value, name: &str) -> Option<&'a Value> {
    match v {
        Value::Object(m) if !is_undefined(v) => m.get(name).filter(|v| !is_undefined(v)),
        _ => None,
    }
}

/// JS `String(v)` on a bridge value.
fn js_to_string(v: &Value, out: &mut String) {
    if is_undefined(v) {
        out.push_str("undefined");
        return;
    }
    if let Some(x) = crate::js::as_number(v) {
        out.push_str(&crate::js::number_to_string(x));
        return;
    }
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&n.to_string()),
        Value::String(s) => out.push_str(s),
        // `String(array)` is `array.join(",")`; null/undefined elements join as "".
        Value::Array(items) => {
            for (i, x) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                if !x.is_null() {
                    js_to_string(x, out);
                }
            }
        }
        Value::Object(_) => out.push_str("[object Object]"),
    }
}

fn sha256_hex(text: &str) -> String {
    let mut h = Sha256::new();
    h.update(text.as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// `JSON.stringify` of a bridge value (undefined object properties drop,
/// non-finite numbers become `null`).
fn js_json(v: &Value, out: &mut String) {
    if is_undefined(v) {
        out.push_str("null");
        return;
    }
    if let Some(x) = crate::js::as_number(v) {
        if x.is_finite() {
            out.push_str(&crate::js::number_to_string(x));
        } else {
            out.push_str("null");
        }
        return;
    }
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&n.to_string()),
        Value::String(s) => {
            out.push_str(&serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into()))
        }
        Value::Array(items) => {
            out.push('[');
            for (i, x) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                js_json(x, out);
            }
            out.push(']');
        }
        Value::Object(m) => {
            out.push('{');
            let mut first = true;
            for (k, x) in m {
                if is_undefined(x) {
                    continue;
                }
                if !first {
                    out.push(',');
                }
                first = false;
                out.push_str(&serde_json::to_string(k).unwrap_or_else(|_| "\"\"".into()));
                out.push(':');
                js_json(x, out);
            }
            out.push('}');
        }
    }
}

fn js_stringify(v: &Value) -> String {
    let mut s = String::new();
    js_json(v, &mut s);
    s
}

/// The divergence consent token over the canonical field list.
fn consent_token(candidate: &Value) -> String {
    let mut obj = Map::with_capacity(TOKEN_FIELDS.len());
    for name in TOKEN_FIELDS {
        if let Some(v) = field(candidate, name) {
            obj.insert(name.to_string(), v.clone());
        }
    }
    sha256_hex(&js_stringify(&Value::Object(obj)))
}

/// Whether a `SYNARA_MIGRATION_DIVERGENCE_CONSENT_REQUIRED=` payload parses and
/// carries a consent token that recomputes from the challenge fields.
fn valid_divergence(text: &str) -> Option<Value> {
    let candidate: Value = serde_json::from_str(text).ok()?;
    let Value::Object(_) = &candidate else {
        return None;
    };
    let token = field(&candidate, "consentToken")?;
    (token.as_str() == Some(consent_token(&candidate).as_str())).then_some(candidate)
}

/// Whether `v` is a non-empty string.
fn non_empty_str(v: Option<&Value>) -> bool {
    v.and_then(Value::as_str).is_some_and(|s| !s.is_empty())
}

fn is_number(v: Option<&Value>) -> bool {
    v.and_then(crate::js::as_number).is_some()
}

/// Whether a `SYNARA_MIGRATION_SCHEMA_TOO_NEW=` payload parses to a valid block.
fn valid_schema_too_new(text: &str) -> Option<Value> {
    let candidate: Value = serde_json::from_str(text).ok()?;
    let Value::Object(_) = &candidate else {
        return None;
    };
    let valid = is_number(field(&candidate, "version"))
        && non_empty_str(field(&candidate, "databasePath"))
        && is_number(field(&candidate, "databaseMigrationId"))
        && is_number(field(&candidate, "latestSupportedMigrationId"))
        && match field(&candidate, "recovery") {
            Some(recovery) => match field(recovery, "kind").and_then(Value::as_str) {
                Some("restore-available") => {
                    non_empty_str(field(recovery, "backupPath"))
                        && non_empty_str(field(recovery, "provenancePath"))
                        && is_number(field(recovery, "backupMigrationId"))
                }
                Some("restore-unavailable") => non_empty_str(field(recovery, "reason")),
                _ => false,
            },
            None => false,
        };
    valid.then_some(candidate)
}

impl BackendStartupBlockDetector {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            log: String::new(),
            frozen: false,
        })
    }

    /// Append `String(chunk)` to the merged output log; a no-op once frozen.
    pub fn push(
        &mut self,
        chunk: serde_json::Value,
        _source: Option<String>,
    ) -> Result<(), String> {
        if self.frozen {
            return Ok(());
        }
        js_to_string(&chunk, &mut self.log);
        Ok(())
    }

    /// Flush a source's decoder; unknown sources throw like
    /// `decoders[source].end()` on `undefined` — unless already frozen.
    pub fn end(&mut self, source: String) -> Result<(), String> {
        if self.frozen {
            return Ok(());
        }
        match source.as_str() {
            "stdout" | "stderr" => Ok(()),
            _ => Err("Cannot read properties of undefined (reading 'end')".into()),
        }
    }

    /// Stop accumulation: later `push` and `end` calls are no-ops.
    pub fn freeze(&mut self) -> Result<(), String> {
        self.frozen = true;
        Ok(())
    }

    /// Scan the log for marker lines; a divergence consent beats a schema block,
    /// and any marker line at all beats `null` when nothing parses.
    pub fn read(&mut self) -> Result<Option<serde_json::Value>, String> {
        let mut divergence = None;
        let mut schema_too_new = None;
        let mut marker_seen = false;
        for line in self.log.split('\n') {
            if let Some(rest) = line.strip_prefix(DIVERGENCE_PREFIX) {
                marker_seen = true;
                if let Some(c) = valid_divergence(rest) {
                    divergence = Some(c);
                }
            } else if let Some(rest) = line.strip_prefix(SCHEMA_TOO_NEW_PREFIX) {
                marker_seen = true;
                if let Some(b) = valid_schema_too_new(rest) {
                    schema_too_new = Some(b);
                }
            }
        }
        let block = if let Some(challenge) = divergence {
            Some(serde_json::json!({
                "kind": "migration-divergence-consent-required",
                "challenge": challenge,
            }))
        } else if let Some(block) = schema_too_new {
            Some(serde_json::json!({
                "kind": "migration-schema-too-new",
                "block": block,
            }))
        } else {
            marker_seen.then(|| {
                serde_json::json!({
                    "kind": "migration-startup-block-invalid",
                })
            })
        };
        Ok(block)
    }
}
