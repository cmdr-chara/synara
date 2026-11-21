//! The Rust port of `packages/shared/src/computerGrants.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

const PERMISSIONS: [&str; 3] = ["accessibility", "screenRecording", "inputMonitoring"];
const STATE_KEYS: [&str; 3] = [
    "accessibilityPermission",
    "screenRecordingPermission",
    "inputMonitoringPermission",
];

fn label(p: &str) -> &'static str {
    match p {
        "accessibility" => "Accessibility",
        "screenRecording" => "Screen Recording",
        _ => "Input Monitoring",
    }
}

fn tcc_service(p: &str) -> &'static str {
    match p {
        "accessibility" => "Accessibility",
        "screenRecording" => "ScreenCapture",
        _ => "ListenEvent",
    }
}

/// `permissions.includes(p)`: arrays do `===`, strings do substring; anything else throws.
fn includes(permissions: &Value, p: &str) -> Result<bool, String> {
    match permissions {
        Value::Array(a) => Ok(a.iter().any(|x| x.as_str() == Some(p))),
        Value::String(s) => Ok(s.contains(p)),
        Value::Null => Err("TypeError: Cannot read properties of null (reading 'includes')".into()),
        v if crate::js::is_undefined(v) => {
            Err("TypeError: Cannot read properties of undefined (reading 'includes')".into())
        }
        _ => Err("TypeError: permissions.includes is not a function".into()),
    }
}

/// The canonical permission order filtered to the ones `permissions` includes.
fn sorted_known(permissions: &Value) -> Result<Vec<&'static str>, String> {
    let mut out = Vec::new();
    for p in PERMISSIONS {
        if includes(permissions, p)? {
            out.push(p);
        }
    }
    Ok(out)
}

/// "A and B" / "A, B and C" for the labels of `known`.
fn join_labels(known: &[&str]) -> String {
    let labels: Vec<&str> = known.iter().map(|p| label(p)).collect();
    match labels.split_last() {
        None => String::new(),
        Some((last, [])) => last.to_string(),
        Some((last, rest)) => format!("{} and {}", rest.join(", "), last),
    }
}

/// Port of `missingComputerAppSnapPermissions` (`packages/shared/src/computerGrants.ts:43`).
pub fn missing_computer_app_snap_permissions(
    state: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    if state.is_null() {
        return Err(
            "TypeError: Cannot read properties of null (reading 'accessibilityPermission')".into(),
        );
    }
    if crate::js::is_undefined(&state) {
        return Err(
            "TypeError: Cannot read properties of undefined (reading 'accessibilityPermission')"
                .into(),
        );
    }
    Ok(PERMISSIONS
        .iter()
        .zip(STATE_KEYS)
        .filter(|(_, k)| state.get(*k).and_then(|v| v.as_str()) != Some("granted"))
        .map(|(p, _)| json!(p))
        .collect())
}

/// Port of `computerGrantsBlockControl` (`packages/shared/src/computerGrants.ts:75`).
pub fn computer_grants_block_control(permissions: serde_json::Value) -> Result<bool, String> {
    Ok(includes(&permissions, "accessibility")? || includes(&permissions, "inputMonitoring")?)
}

/// Port of `sortComputerPermissions` (`packages/shared/src/computerGrants.ts:103`).
pub fn sort_computer_permissions(
    permissions: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    Ok(sorted_known(&permissions)?
        .iter()
        .map(|p| json!(p))
        .collect())
}

/// Port of `listComputerPermissions` (`packages/shared/src/computerGrants.ts:110`).
pub fn list_computer_permissions(permissions: serde_json::Value) -> Result<String, String> {
    Ok(join_labels(&sorted_known(&permissions)?))
}

/// `bundleId?.trim()` per the source: nullish skips, non-string throws.
fn trimmed_bundle_id(bundle_id: &Value) -> Result<Option<String>, String> {
    match bundle_id {
        v if v.is_null() || crate::js::is_undefined(v) => Ok(None),
        Value::String(s) => Ok(Some(s.trim().to_string())),
        _ => Err("TypeError: bundleId?.trim is not a function".into()),
    }
}

const STALE_BASE: &str = "This is a locally built copy of Synara, so macOS may already list it with the switch on from an earlier build. Remove this app from the permission list, add the current build again, then fully quit and reopen it.";

/// Port of `computerStaleGrantAdvice` (`packages/shared/src/computerGrants.ts:145`).
pub fn computer_stale_grant_advice(
    permissions: serde_json::Value,
    build_signature: serde_json::Value,
    bundle_id: serde_json::Value,
) -> Result<Option<String>, String> {
    if build_signature.as_str() != Some("adhoc") {
        return Ok(None);
    }
    let known = sorted_known(&permissions)?;
    if known.is_empty() {
        return Ok(None);
    }
    let mut advice = STALE_BASE.to_string();
    if let Some(b) = trimmed_bundle_id(&bundle_id)?.filter(|s| !s.is_empty()) {
        let cmds: Vec<String> = known
            .iter()
            .map(|p| format!("tccutil reset {} {}", tcc_service(p), b))
            .collect();
        advice.push_str(&format!(
            " To clear only this app's old grant, run `{}` in Terminal and grant access again.",
            cmds.join(", then ")
        ));
    }
    Ok(Some(advice))
}

/// Port of `computerPermissionSetupMessage` (`packages/shared/src/computerGrants.ts:169`).
pub fn computer_permission_setup_message(
    permissions: serde_json::Value,
    build_signature: serde_json::Value,
    bundle_id: serde_json::Value,
) -> Result<String, String> {
    let known = sorted_known(&permissions)?;
    let mut message = if known.is_empty() {
        "Synara needs a macOS privacy permission to control this Mac. Grant it in System Settings \u{203a} Privacy & Security, then try again.".to_string()
    } else {
        let names = join_labels(&known);
        format!(
            "Synara needs {names} to control this Mac. Turn Synara on in System Settings \u{203a} Privacy & Security \u{203a} {names}, then try again."
        )
    };
    if let Some(advice) = computer_stale_grant_advice(permissions, build_signature, bundle_id)? {
        message.push(' ');
        message.push_str(&advice);
    }
    Ok(message)
}
