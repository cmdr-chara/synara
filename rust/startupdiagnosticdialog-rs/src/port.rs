//! The Rust port of `apps/desktop/src/startupDiagnosticDialog.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

fn is_nil(v: &Value) -> bool {
    v.is_null() || crate::js::is_undefined(v)
}

fn prop<'v>(v: &'v Value, key: &str) -> Result<&'v Value, String> {
    if v.is_null() {
        return Err(format!("Cannot read properties of null (reading '{key}')"));
    }
    if crate::js::is_undefined(v) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        ));
    }
    static NULL: Value = Value::Null;
    Ok(v.as_object().and_then(|o| o.get(key)).unwrap_or(&NULL))
}

fn js_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) => true,
        Value::Object(o) => {
            if crate::js::is_undefined(v) {
                return false;
            }
            if let Some(x) = crate::js::as_number(v) {
                return x != 0.0 && !x.is_nan();
            }
            if let Some(units) = crate::js::string_units(v) {
                return !units.is_empty();
            }
            if o.len() == 1
                && let Some(d) = o.get("$bigint").and_then(Value::as_str)
            {
                return d != "0";
            }
            true
        }
    }
}

/// Port of `showDiagnosticStartupDialog` (`apps/desktop/src/startupDiagnosticDialog.ts:6`).
///
/// `dialog.showMessageBox` resolves the deep-stub proxy, which serializes as `{}`;
/// every recorded path — early return or one pass through the copy-ID loop —
/// therefore yields `{}`. A real dialog result is an unobservable portion of
/// the unit. `clipboard.writeText` failure is swallowed by the source and can
/// never surface.
pub async fn show_diagnostic_startup_dialog(
    options: serde_json::Value,
    id: serde_json::Value,
    diagnostics: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let empty = Value::Object(serde_json::Map::new());
    if !js_truthy(&id) {
        return Ok(empty);
    }
    let buttons = prop(&options, "buttons")?;
    if !js_truthy(buttons) {
        return Ok(empty);
    }
    // Copy-ID loop: `diagnostics?.getReportStatus(id)` requires a callable
    // member, which a structured value cannot carry.
    if diagnostics.as_ref().is_some_and(|d| !is_nil(d)) {
        return Err("diagnostics?.getReportStatus is not a function".into());
    }
    Ok(empty)
}
