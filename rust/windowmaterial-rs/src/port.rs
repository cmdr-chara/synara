//! The Rust port of `apps/desktop/src/windowMaterial.ts`.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::{Value, json};

const DESKTOP_WINDOW_BLUR_RADIUS_MAX: f64 = 64.0;
const DESKTOP_WINDOW_BLUR_RADIUS_MIN: f64 = 1.0;

/// Port of `parseHyprlandBlurEnabled` (`apps/desktop/src/windowMaterial.ts:45`).
pub fn parse_hyprland_blur_enabled(stdout: String) -> Result<Value, String> {
    let option: Value = match serde_json::from_str(&stdout) {
        Ok(v) => v,
        Err(_) => return Ok(Value::Null),
    };
    if option.is_null() || js::is_undefined(&option) || !option.is_object() {
        return Ok(Value::Null);
    }
    let bool_v = option.get("bool");
    if let Some(Value::Bool(b)) = bool_v {
        return Ok(json!(b));
    }
    if let Some(i) = option.get("int").and_then(|v| v.as_f64()) {
        return Ok(json!(i != 0.0));
    }
    Ok(Value::Null)
}

/// Port of `parseDesktopWindowMaterial` (`apps/desktop/src/windowMaterial.ts:66`).
pub fn parse_desktop_window_material(raw: Value) -> Result<Value, String> {
    if raw.is_null() || js::is_undefined(&raw) || !(raw.is_object() || raw.is_array()) {
        return Ok(Value::Null);
    }
    let material = raw.get("material").and_then(Value::as_str);
    if material != Some("opaque") && material != Some("translucent") {
        return Ok(Value::Null);
    }
    let blur = match raw.get("blurRadius").and_then(js::as_number) {
        Some(b) if b.is_finite() => b,
        _ => return Ok(Value::Null),
    };
    let min = if material == Some("translucent") {
        DESKTOP_WINDOW_BLUR_RADIUS_MIN
    } else {
        0.0
    };
    let radius = js::math_round(DESKTOP_WINDOW_BLUR_RADIUS_MAX.min(blur.max(min)));
    Ok(json!({
        "material": material,
        "blurRadius": js::number(radius),
    }))
}
