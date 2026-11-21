//! The Rust port of `packages/shared/src/migrationRecovery.ts`.
//!
//! Reconstructed clean-room from cases.jsonl (2216 cases, 98% line coverage), the
//! exported signatures and the serialized marker shapes
//! (`SYNARA_MIGRATION_DIVERGENCE_CONSENT_REQUIRED=<json>`,
//! `SYNARA_MIGRATION_SCHEMA_TOO_NEW=<json>`).

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

fn sha256_hex(text: &str) -> String {
    let mut h = Sha256::new();
    h.update(text.as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// Append `JSON.stringify` of a bridge value: `undefined`/non-finite numbers
/// inside containers become `null`; object properties that are `undefined` drop.
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

/// `JSON.stringify(v)` for a top-level bridge value (`undefined` → `undefined`).
fn js_stringify(v: &Value) -> String {
    if is_undefined(v) {
        return "undefined".into();
    }
    let mut s = String::new();
    js_json(v, &mut s);
    s
}

/// Scan `output` for `PREFIX=<json>` markers; return each parsed line payload.
fn scan_markers<'a>(output: &'a str, prefix: &'a str) -> impl Iterator<Item = Value> + 'a {
    let mut rest = output;
    std::iter::from_fn(move || {
        loop {
            let at = rest.find(prefix)? + prefix.len();
            let tail = &rest[at..];
            let line = tail.split('\n').next().unwrap_or(tail);
            rest = &tail[line.len().min(tail.len())..];
            if let Ok(v) = serde_json::from_str::<Value>(line) {
                return Some(v);
            }
        }
    })
}

/// Port of `migrationRecoveryMarkerPath` (`packages/shared/src/migrationRecovery.ts:4`).
pub fn migration_recovery_marker_path(db_path: String) -> Result<String, String> {
    Ok(format!("{db_path}.migration-recovery.json"))
}

/// Port of `migrationBackupDirectory` (`packages/shared/src/migrationRecovery.ts:8`).
pub fn migration_backup_directory(db_path: String) -> Result<String, String> {
    Ok(format!("{db_path}.backups"))
}

/// Port of `migrationBackupProvenancePath` (`packages/shared/src/migrationRecovery.ts:12`).
pub fn migration_backup_provenance_path(db_path: String) -> Result<String, String> {
    Ok(format!("{db_path}.migration-backup.json"))
}

/// Port of `parseMigrationRecoveryResumeState` (`packages/shared/src/migrationRecovery.ts:47`).
pub fn parse_migration_recovery_resume_state(
    marker_text: String,
) -> Result<Option<serde_json::Value>, String> {
    let Ok(parsed) = serde_json::from_str::<Value>(&marker_text) else {
        return Ok(None);
    };
    if !matches!(parsed, Value::Object(_)) {
        return Ok(None);
    }
    let attempts = match field(&parsed, "resumeAttempts") {
        None => 0.0,
        Some(v) => match crate::js::as_number(v) {
            Some(n) if n.fract() == 0.0 && n >= 0.0 && n.is_finite() => n,
            _ => return Ok(None),
        },
    };
    Ok(Some(serde_json::json!({
        "attempts": attempts,
        "exhausted": attempts >= 2.0
    })))
}

/// Port of `createMigrationDivergenceConsentToken` (`packages/shared/src/migrationRecovery.ts:93`).
pub fn create_migration_divergence_consent_token(
    challenge: serde_json::Value,
) -> Result<String, String> {
    if challenge.is_null() {
        return Err("Cannot read properties of null (reading 'version')".into());
    }
    if is_undefined(&challenge) {
        return Err("Cannot read properties of undefined (reading 'version')".into());
    }
    let mut obj = Map::with_capacity(TOKEN_FIELDS.len());
    for name in TOKEN_FIELDS {
        if let Some(v) = field(&challenge, name) {
            obj.insert(name.to_string(), v.clone());
        }
    }
    Ok(sha256_hex(&js_stringify(&Value::Object(obj))))
}

/// Port of `migrationRuntimeSourceDigest` (`packages/shared/src/migrationRecovery.ts:140`).
pub fn migration_runtime_source_digest(source: String) -> Result<String, String> {
    Ok(sha256_hex(&source))
}

/// Port of `findMigrationRuntimeIdentityMismatch` (`packages/shared/src/migrationRecovery.ts:144`).
pub fn find_migration_runtime_identity_mismatch(
    input: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'launcherDigest')".into());
    }
    if is_undefined(&input) {
        return Err("Cannot read properties of undefined (reading 'launcherDigest')".into());
    }
    let embedded = field(&input, "embeddedDigest").cloned();
    let mismatch = |kind: &str, expected: Value| {
        let mut m = Map::with_capacity(3);
        m.insert("kind".into(), Value::String(kind.into()));
        m.insert("expectedDigest".into(), expected);
        m.insert(
            "actualDigest".into(),
            embedded.clone().unwrap_or_else(crate::js::undefined),
        );
        Value::Object(m)
    };
    let not_eq =
        |a: &Value, b: &Value| serde_json::to_string(a).ok() != serde_json::to_string(b).ok();

    if let Some(launcher) = field(&input, "launcherDigest")
        && embedded.as_ref().is_none_or(|e| not_eq(e, launcher))
    {
        return Ok(Some(mismatch("launcher-bundle", launcher.clone())));
    }
    if let Some(source) = field(&input, "sourceText") {
        let expected = Value::String(sha256_hex(source.as_str().unwrap_or_default()));
        if embedded.as_ref().is_none_or(|e| not_eq(e, &expected)) {
            return Ok(Some(mismatch("source-bundle", expected)));
        }
    }
    Ok(None)
}

/// Port of `serializeMigrationDivergenceConsentChallenge` (`packages/shared/src/migrationRecovery.ts:169`).
pub fn serialize_migration_divergence_consent_challenge(
    challenge: serde_json::Value,
) -> Result<String, String> {
    Ok(format!("{DIVERGENCE_PREFIX}{}", js_stringify(&challenge)))
}

/// Port of `parseMigrationDivergenceConsentChallenge` (`packages/shared/src/migrationRecovery.ts:175`).
pub fn parse_migration_divergence_consent_challenge(
    output: String,
) -> Result<Option<serde_json::Value>, String> {
    let mut found = None;
    for candidate in scan_markers(&output, DIVERGENCE_PREFIX) {
        let Value::Object(_) = &candidate else {
            continue;
        };
        let Some(token) = field(&candidate, "consentToken") else {
            continue;
        };
        let Ok(computed) = create_migration_divergence_consent_token(candidate.clone()) else {
            continue;
        };
        if token.as_str() == Some(computed.as_str()) {
            found = Some(candidate);
        }
    }
    Ok(found)
}

/// Port of `serializeMigrationSchemaTooNewStartupBlock` (`packages/shared/src/migrationRecovery.ts:187`).
pub fn serialize_migration_schema_too_new_startup_block(
    block: serde_json::Value,
) -> Result<String, String> {
    Ok(format!("{SCHEMA_TOO_NEW_PREFIX}{}", js_stringify(&block)))
}

/// Whether `v` is a non-empty string.
fn non_empty_str(v: Option<&Value>) -> bool {
    v.and_then(Value::as_str).is_some_and(|s| !s.is_empty())
}

/// Port of `parseMigrationSchemaTooNewStartupBlock` (`packages/shared/src/migrationRecovery.ts:193`).
pub fn parse_migration_schema_too_new_startup_block(
    output: String,
) -> Result<Option<serde_json::Value>, String> {
    let mut found = None;
    for candidate in scan_markers(&output, SCHEMA_TOO_NEW_PREFIX) {
        let Value::Object(_) = &candidate else {
            continue;
        };
        let valid = crate::js::as_number(field(&candidate, "version").unwrap_or(&Value::Null))
            .is_some()
            && non_empty_str(field(&candidate, "databasePath"))
            && crate::js::as_number(
                field(&candidate, "databaseMigrationId").unwrap_or(&Value::Null),
            )
            .is_some()
            && crate::js::as_number(
                field(&candidate, "latestSupportedMigrationId").unwrap_or(&Value::Null),
            )
            .is_some()
            && match field(&candidate, "recovery") {
                Some(recovery) => match field(recovery, "kind").and_then(Value::as_str) {
                    Some("restore-available") => {
                        non_empty_str(field(recovery, "backupPath"))
                            && non_empty_str(field(recovery, "provenancePath"))
                            && crate::js::as_number(
                                field(recovery, "backupMigrationId").unwrap_or(&Value::Null),
                            )
                            .is_some()
                    }
                    Some("restore-unavailable") => non_empty_str(field(recovery, "reason")),
                    _ => false,
                },
                None => false,
            };
        if valid {
            found = Some(candidate);
        }
    }
    Ok(found)
}
