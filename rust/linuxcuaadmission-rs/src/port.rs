//! The Rust port of `apps/desktop/src/linuxCuaAdmission.ts`.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::{Value, json};

const UNSUPPORTED_NATIVE_TOOLS: &[&str] =
    &["select_text", "set_window_minimized", "set_app_visibility"];
const UNCANCELLABLE_WINDOW_TOOLS: &[&str] = &["launch_app", "bring_to_front", "set_window_frame"];
const CUA_ACTION_TOOLS: &[&str] = &[
    "click",
    "move_cursor",
    "drag",
    "scroll",
    "type_text",
    "press_key",
    "hotkey",
    "set_value",
    "select_text",
    "clipboard_read",
    "clipboard_write",
    "launch_app",
    "bring_to_front",
    "invoke_menu",
    "set_window_frame",
    "set_window_minimized",
    "set_app_visibility",
    "kill_app",
];
const CUA_BROWSER_TOOLS: &[&str] = &[
    "get_browser_state",
    "browser_prepare",
    "browser_navigate",
    "browser_click",
    "browser_type",
    "browser_dialog",
    "browser_set_input_files",
    "browser_download",
    "browser_pointer",
];

fn as_args(input: &Value) -> &Value {
    static EMPTY: Value = Value::Null;
    match input {
        Value::Object(_) if !js::is_undefined(input) => input,
        _ => &EMPTY,
    }
}

fn arg<'a>(args: &'a Value, key: &str) -> Option<&'a Value> {
    args.as_object().and_then(|o| o.get(key))
}

/// `value === undefined` (absent key or the undefined marker).
fn is_undef(v: Option<&Value>) -> bool {
    v.is_none() || v.is_some_and(js::is_undefined)
}

fn refusal(code: &str, message: &str) -> Value {
    json!({
        "ok": true,
        "effect": "not-dispatched",
        "result": {
            "isError": true,
            "content": [{ "type": "text", "text": message }],
            "structuredContent": { "effect": "refused", "code": code, "message": message },
        },
    })
}

/// Port of `linuxBrowserCallIsReadOnly` (`apps/desktop/src/linuxCuaAdmission.ts:20`).
pub fn linux_browser_call_is_read_only(name: String, input: Value) -> Result<bool, String> {
    if name == "get_browser_state" {
        return Ok(true);
    }
    let args = as_args(&input);
    if name == "browser_dialog" {
        return Ok(arg(args, "action").and_then(Value::as_str) == Some("inspect"));
    }
    Ok(name == "browser_prepare"
        && (is_undef(arg(args, "allow_launch"))
            || arg(args, "allow_launch") == Some(&Value::Bool(false)))
        && (is_undef(arg(args, "strategy")) || arg(args, "strategy") == Some(&Value::Null)))
}

/// Port of `linuxCuaAdmissionRefusal` (`apps/desktop/src/linuxCuaAdmission.ts:44`).
pub fn linux_cua_admission_refusal(
    name: String,
    input: Value,
    delivery_mode: Value,
    browser_input_control_verified: Option<bool>,
) -> Result<Option<Value>, String> {
    let verified = browser_input_control_verified.unwrap_or(false);
    let args = as_args(&input);

    if CUA_BROWSER_TOOLS.contains(&name.as_str()) {
        let read_only = linux_browser_call_is_read_only(name.clone(), args.clone())?;
        if read_only && (name != "browser_prepare" || !verified) {
            return Ok(None);
        }
        if name == "browser_prepare"
            && !verified
            && arg(args, "allow_launch") == Some(&Value::Bool(true))
            && arg(args, "windowed") != Some(&Value::Bool(true))
        {
            return Ok(Some(refusal(
                "linux_headless_browser_unavailable",
                "This Linux driver cannot launch a headless browser, and browser launches are unavailable until cancellation cleanup is supported. Only observation of an existing debuggable browser is available.",
            )));
        }
        if verified {
            if name == "browser_prepare" && arg(args, "windowed") == Some(&Value::Bool(true)) {
                return Ok(Some(refusal(
                    "linux_windowed_browser_unavailable",
                    "The Linux browser runtime supports isolated headless browsers only. Visible browser launches are unavailable; keep windowed false.",
                )));
            }
            if name == "browser_prepare"
                && !(is_undef(arg(args, "strategy")) || arg(args, "strategy") == Some(&Value::Null))
            {
                return Ok(Some(refusal(
                    "linux_browser_profile_unavailable",
                    "The Linux browser runtime cannot change or attach a personal browser profile for automation. Use an isolated_new or isolated_named headless profile; passive endpoint detection remains available.",
                )));
            }
            if name == "browser_dialog"
                && arg(args, "delivery_mode").and_then(Value::as_str) == Some("foreground")
                && delivery_mode.as_str() != Some("foreground")
            {
                return Ok(Some(refusal(
                    "linux_background_unavailable",
                    "A model-supplied foreground dialog mode cannot authorize visible computer use. Keep the headless background route.",
                )));
            }
            return Ok(None);
        }
        return Ok(Some(refusal(
            "linux_browser_cleanup_unavailable",
            "This Linux driver cannot stop an in-flight browser mutation or launch reliably. Browser actions are unavailable until native cancellation cleanup is supported, even with foreground consent. Only browser observation, endpoint detection, and dialog inspection are available; nothing was changed.",
        )));
    }

    if !CUA_ACTION_TOOLS.contains(&name.as_str()) {
        return Ok(None);
    }
    if UNSUPPORTED_NATIVE_TOOLS.contains(&name.as_str()) {
        return Ok(Some(refusal(
            "unsupported_linux_operation",
            "This native operation is not implemented by the Linux driver. Observe the current window and use an available route.",
        )));
    }
    if name == "set_value"
        || arg(args, "semantic_only") == Some(&Value::Bool(true))
        || !is_undef(arg(args, "element_token"))
        || !is_undef(arg(args, "element_index"))
    {
        return Ok(Some(refusal(
            "linux_semantic_target_unproven",
            "This Linux route does not prove the observed element's exact window/object identity. Native pointer and keyboard input are also unavailable until cancellation cleanup is supported. Use observation or an existing debuggable browser when appropriate.",
        )));
    }
    if name == "clipboard_read"
        || name == "clipboard_write"
        || name == "kill_app"
        || (name == "move_cursor"
            && (is_undef(arg(args, "scope"))
                || arg(args, "scope").and_then(Value::as_str) == Some("window")))
    {
        return Ok(None);
    }
    if UNCANCELLABLE_WINDOW_TOOLS.contains(&name.as_str()) {
        return Ok(Some(refusal(
            "linux_input_cleanup_unavailable",
            "This Linux driver cannot stop an in-flight native app or window operation reliably. Launching apps, invoking menus, activating windows, and changing window frames are unavailable until native cancellation checkpoints are supported, even with foreground consent. No action was dispatched.",
        )));
    }
    if delivery_mode.as_str() == Some("foreground") {
        return Ok(Some(refusal(
            "linux_input_cleanup_unavailable",
            "This Linux driver cannot acknowledge release of native input when Computer is stopped. Native pointer and keyboard input are unavailable until cancellation cleanup is supported; no input was sent.",
        )));
    }
    Ok(Some(refusal(
        "linux_background_unavailable",
        "This Linux native route cannot guarantee background input without moving desktop focus or the human pointer. Foreground input is also unavailable until cancellation cleanup is supported. No input was sent; use observation or an existing debuggable browser when appropriate.",
    )))
}
