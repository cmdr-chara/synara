//! The Rust port of `packages/shared/src/desktopChrome.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

/// Port of `getMacTrafficLightPosition` (`packages/shared/src/desktopChrome.ts:11`).
pub fn get_mac_traffic_light_position() -> Result<serde_json::Value, String> {
    Ok(json!({ "x": 16, "y": 15 }))
}

/// JS `Number(v)`: null→0, bool→0/1, string trimmed (''→0, hex/Infinity/f64),
/// []→0, [x]→Number(x), anything else NaN.
fn js_number(v: &Value) -> f64 {
    if let Some(n) = crate::js::as_number(v) {
        return n;
    }
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => f64::from(*b as u8),
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => {
            let t =
                s.trim_matches(|c: char| (c.is_whitespace() || c == '\u{feff}') && c != '\u{85}');
            if t.is_empty() {
                0.0
            } else if let Some(h) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
                u64::from_str_radix(h, 16).map_or(f64::NAN, |v| v as f64)
            } else if t == "Infinity" || t == "+Infinity" {
                f64::INFINITY
            } else if t == "-Infinity" {
                f64::NEG_INFINITY
            } else {
                t.parse::<f64>().unwrap_or(f64::NAN)
            }
        }
        Value::Array(a) if a.is_empty() => 0.0,
        Value::Array(a) if a.len() == 1 => js_number(&a[0]),
        _ => f64::NAN,
    }
}

fn norm_zoom(z: f64) -> f64 {
    if z.is_finite() && z > 0.0 { z } else { 1.0 }
}

/// Port of `normalizeDesktopZoomFactor` (`packages/shared/src/desktopChrome.ts:16`).
pub fn normalize_desktop_zoom_factor(zoom_factor: serde_json::Value) -> Result<f64, String> {
    Ok(norm_zoom(
        crate::js::as_number(&zoom_factor).unwrap_or(f64::NAN),
    ))
}

/// Port of `resolveMacDesktopTopBarTrafficLightGutterCssPx` (`packages/shared/src/desktopChrome.ts:76`).
pub fn resolve_mac_desktop_top_bar_traffic_light_gutter_css_px(
    zoom_factor: f64,
) -> Result<f64, String> {
    Ok(crate::js::math_round(90.0 / norm_zoom(zoom_factor)))
}

/// Port of `resolveDesktopDipRectFromCssRect` (`packages/shared/src/desktopChrome.ts:101`).
pub fn resolve_desktop_dip_rect_from_css_rect(
    rect: serde_json::Value,
    zoom_factor: f64,
) -> Result<serde_json::Value, String> {
    let zoom = norm_zoom(zoom_factor);
    if zoom == 1.0 {
        return Ok(rect);
    }
    if rect.is_null() {
        return Err("TypeError: Cannot read properties of null (reading 'x')".into());
    }
    let get = |k: &str| rect.get(k).cloned().unwrap_or_else(crate::js::undefined);
    Ok(json!({
        "x": crate::js::number(js_number(&get("x")) * zoom),
        "y": crate::js::number(js_number(&get("y")) * zoom),
        "width": crate::js::number(js_number(&get("width")) * zoom),
        "height": crate::js::number(js_number(&get("height")) * zoom),
    }))
}
