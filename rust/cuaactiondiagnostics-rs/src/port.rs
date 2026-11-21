//! The Rust port of `packages/shared/src/cuaActionDiagnostics.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value};

const DELIVERY_PATHS: [&str; 2] = ["ax", "keyboard"];
const ACTUATORS: [&str; 2] = ["ax_press", "ax_focus"];
const FOCUS_MUTATIONS: [&str; 1] = ["none"];
const RESTORE_STATUSES: [&str; 3] = ["not-needed", "failed", "restored"];
const ERROR_CODES: [&str; 1] = ["ax_dispatch_failed"];
const OBSERVATIONS: [&str; 1] = ["fresh-frame"];

fn one_of(v: Option<&Value>, allowed: &[&str]) -> Option<Value> {
    let s = v?.as_str()?;
    allowed.contains(&s).then(|| Value::String(s.to_string()))
}

fn int_in(v: Option<&Value>, lo: i64, hi: i64) -> Option<Value> {
    let n = v?.as_i64()?;
    (n >= lo && n <= hi).then(|| Value::Number(n.into()))
}

/// Port of `parseCuaActionDiagnostics` (`packages/shared/src/cuaActionDiagnostics.ts:59`).
pub fn parse_cua_action_diagnostics(
    value: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let Some(Value::Object(diag)) = value.get("diagnostics") else {
        return Ok(None);
    };
    let g = |k: &str| diag.get(k);
    let mut out = Map::new();
    let mut put = |k: &str, v: Option<Value>| {
        if let Some(v) = v {
            out.insert(k.to_string(), v);
        }
    };
    put("delivery_path", one_of(g("delivery_path"), &DELIVERY_PATHS));
    put("actuator", one_of(g("actuator"), &ACTUATORS));
    put(
        "focus_mutation",
        one_of(g("focus_mutation"), &FOCUS_MUTATIONS),
    );
    put(
        "restore_status",
        one_of(g("restore_status"), &RESTORE_STATUSES),
    );
    put("error_code", one_of(g("error_code"), &ERROR_CODES));
    put(
        "ax_error",
        int_in(g("ax_error"), i32::MIN as i64, i32::MAX as i64),
    );
    put("prior_pid", int_in(g("prior_pid"), 1, i32::MAX as i64));
    put(
        "prior_window_id",
        int_in(g("prior_window_id"), 1, u32::MAX as i64),
    );
    put("observation", one_of(g("observation"), &OBSERVATIONS));
    let scroll = g("scroll_delta_y")
        .and_then(|v| v.as_f64())
        .filter(|n| n.is_finite());
    if let Some(n) = scroll {
        out.insert("scroll_delta_y".to_string(), crate::js::number(n));
    }
    Ok((!out.is_empty()).then_some(Value::Object(out)))
}

/// Port of `cuaActionDiagnosticMessage` (`packages/shared/src/cuaActionDiagnostics.ts:87`).
pub fn cua_action_diagnostic_message(
    diagnostics: serde_json::Value,
) -> Result<Option<String>, String> {
    if diagnostics.is_null() {
        return Err("TypeError: Cannot read properties of null (reading 'error_code')".into());
    }
    if crate::js::is_undefined(&diagnostics) {
        return Err("TypeError: Cannot read properties of undefined (reading 'error_code')".into());
    }
    Ok(
        (diagnostics.get("error_code").and_then(|v| v.as_str()) == Some("ax_dispatch_failed"))
            .then(|| "The accessibility actuator reported a native error.".to_string()),
    )
}
