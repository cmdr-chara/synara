//! The Rust port of `packages/shared/src/computerInvocation.ts`.

#![forbid(unsafe_code)]

use serde_json::json;

fn js_ws(c: char) -> bool {
    (c.is_whitespace() || c == '\u{feff}') && c != '\u{85}'
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(js_ws)
}

/// `/^ {0,3}\/computer-use(?=\s|$)([\s\S]*)$/i` — up to 3 leading spaces, command
/// at the very start, then whitespace or end of input.
pub fn parse_computer_invocation(
    text: Option<String>,
) -> Result<Option<serde_json::Value>, String> {
    let Some(text) = text else { return Ok(None) };
    let spaces = text.len() - text.trim_start_matches(' ').len();
    if spaces > 3 {
        return Ok(None);
    }
    let rest = &text[spaces..];
    let cmd = rest
        .as_bytes()
        .get(.."/computer-use".len())
        .unwrap_or_default();
    if !cmd.eq_ignore_ascii_case(b"/computer-use") {
        return Ok(None);
    }
    let tail = &rest["/computer-use".len()..];
    if tail.chars().next().is_some_and(|c| !js_ws(c)) {
        return Ok(None);
    }
    Ok(Some(json!({ "prompt": js_trim(tail) })))
}

/// Port of `resolveComputerInvocationMode` (`packages/shared/src/computerInvocation.ts:15`).
pub fn resolve_computer_invocation_mode(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if input.is_null() {
        return Err(
            "TypeError: Cannot read properties of null (reading 'computerControlMode')".into(),
        );
    }
    if crate::js::is_undefined(&input) {
        return Err(
            "TypeError: undefined is not an object (evaluating 'input.computerControlMode')".into(),
        );
    }
    let get = |key: &str| input.get(key).filter(|v| !crate::js::is_undefined(v));
    if let Some(mode) = get("computerControlMode") {
        return Ok(mode.clone());
    }
    if input.get("enableComputerControl").and_then(|v| v.as_bool()) == Some(true) {
        return Ok(json!("chat"));
    }
    if get("dispatchOrigin")
        .and_then(|v| v.as_str())
        .is_some_and(|o| o == "agent" || o == "automation")
    {
        return Ok(json!("off"));
    }
    let text = input
        .get("messageText")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    Ok(json!(if parse_computer_invocation(text)?.is_some() {
        "request"
    } else {
        "off"
    }))
}
