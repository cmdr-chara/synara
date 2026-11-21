//! The Rust port of `apps/desktop/src/desktopWsBridge.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

use crate::js;

fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

/// Port of `normalizeDesktopWsUrl` (`apps/desktop/src/desktopWsBridge.ts:7`).
pub fn normalize_desktop_ws_url(value: serde_json::Value) -> Result<Option<String>, String> {
    let Value::String(s) = &value else {
        return Ok(None);
    };
    let trimmed = s.trim_matches(is_js_space);
    Ok(if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    })
}

/// Port of `resolveDesktopWsUrlFromEnv` (`apps/desktop/src/desktopWsBridge.ts:15`).
pub fn resolve_desktop_ws_url_from_env(env: serde_json::Value) -> Result<Option<String>, String> {
    if env.is_null() {
        return Err("Cannot read properties of null (reading 'SYNARA_DESKTOP_WS_URL')".to_string());
    }
    if js::is_undefined(&env) {
        return Err(
            "Cannot read properties of undefined (reading 'SYNARA_DESKTOP_WS_URL')".to_string(),
        );
    }
    let value = env
        .get("SYNARA_DESKTOP_WS_URL")
        .cloned()
        .unwrap_or(Value::Null);
    normalize_desktop_ws_url(value)
}
