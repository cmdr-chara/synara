//! Real macOS computer backend. OS calls go through a small Swift helper
//! (`computer_helper.swift`, compiled once into the state dir) because the
//! JXA bridge cannot reach CoreGraphics C functions. Permissions are
//! probed through `osascript -l JavaScript` (ApplicationServices/Quartz
//! predicates do bridge). Only engaged under SYNARA_NO_UPSTREAM on macOS;
//! every handled method returns upstream wire shapes.

use serde_json::{json, Value};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

pub const COMPUTER_ID: &str = "desktop";

fn jxa(script: &str) -> Result<String, String> {
    let out = Command::new("osascript")
        .args(["-l", "JavaScript", "-e", script])
        .output()
        .map_err(|e| format!("Failed to run osascript: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn ax_trusted() -> bool {
    jxa("ObjC.import('ApplicationServices'); $.AXIsProcessTrusted();").ok().as_deref() == Some("true")
}

fn screen_capture_granted() -> bool {
    jxa("ObjC.import('Quartz'); $.CGPreflightScreenCaptureAccess();").ok().as_deref() == Some("true")
}

fn helper_bin() -> Result<PathBuf, String> {
    let dir = std::path::Path::new(&crate::todo::state_dir()).join("computer-helper");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let bin = dir.join("computer-helper");
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("computer_helper.swift");
    let stale = match (std::fs::metadata(&bin), std::fs::metadata(&src)) {
        (Ok(b), Ok(s)) => b.modified().ok() < s.modified().ok(),
        _ => true,
    };
    if stale {
        let out = Command::new("swiftc")
            .args(["-O", "-o"])
            .arg(&bin)
            .arg(&src)
            .output()
            .map_err(|e| format!("Failed to run swiftc: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "computer-helper build failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
    }
    Ok(bin)
}

fn helper(cmd: &str, arg: &Value) -> Result<Value, String> {
    let bin = helper_bin()?;
    let out = Command::new(&bin)
        .arg(cmd)
        .arg(serde_json::to_string(arg).unwrap_or_else(|_| "{}".into()))
        .output()
        .map_err(|e| format!("Failed to run computer-helper {cmd}: {e}"))?;
    if !out.status.success() {
        let e = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if e.is_empty() { format!("computer-helper {cmd} failed") } else { e });
    }
    serde_json::from_slice(&out.stdout).map_err(|e| format!("computer-helper {cmd} invalid output: {e}"))
}

fn missing_permissions() -> Vec<&'static str> {
    let mut m = Vec::new();
    if !ax_trusted() {
        m.push("accessibility");
    }
    if !screen_capture_granted() {
        m.push("screenRecording");
    }
    m
}

fn availability() -> Value {
    let missing = missing_permissions();
    if missing.is_empty() {
        json!({"kind": "available", "backend": "cua"})
    } else {
        let list = missing.join(" and ");
        json!({
            "kind": "permission-required",
            "missing": missing,
            "message": format!("Synara needs {list} permission to control this Mac. Grant it in System Settings > Privacy & Security, then try again."),
            "buildSignature": "unknown"
        })
    }
}

fn capabilities() -> Value {
    json!({
        "windows": true, "windowBounds": true, "stacking": true, "capture": true,
        "input": true, "clipboard": true, "focus": true, "raise": true,
        "ghostCursor": false, "visibleDesktop": true
    })
}

fn health() -> Value {
    json!({
        "status": "connected", "consecutiveFailures": 0, "reconnects": 0,
        "captureAvailable": screen_capture_granted()
    })
}

fn screen_size() -> Result<Value, String> {
    let v = helper("screen-size", &json!({}))?;
    let w = v["width"].as_i64().unwrap_or(0);
    let h = v["height"].as_i64().unwrap_or(0);
    if w <= 0 || h <= 0 {
        return Err("Cua returned no primary display geometry.".into());
    }
    let mut out = json!({"width": w, "height": h});
    if let Some(s) = v["scale"].as_f64().filter(|s| *s > 0.0) {
        out["scale"] = json!(s);
    }
    Ok(out)
}

fn list_windows() -> Result<Vec<Value>, String> {
    let v = helper("windows", &json!({}))?;
    Ok(v["windows"].as_array().cloned().unwrap_or_default())
}

/// Window list as a JSON value, for the subscribeEvents poller.
pub fn windows_value() -> Result<Value, String> {
    Ok(Value::Array(list_windows()?))
}

fn action_channel() -> &'static tokio::sync::broadcast::Sender<Value> {
    static CH: std::sync::OnceLock<tokio::sync::broadcast::Sender<Value>> =
        std::sync::OnceLock::new();
    CH.get_or_init(|| tokio::sync::broadcast::channel(64).0)
}

/// Live `computer.action` events from this server's own dispatches.
pub fn action_events() -> tokio::sync::broadcast::Receiver<Value> {
    action_channel().subscribe()
}

fn base64(bytes: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::new();
    for c in bytes.chunks(3) {
        let n = ((c[0] as u32) << 16) | ((*c.get(1).unwrap_or(&0) as u32) << 8) | (*c.get(2).unwrap_or(&0) as u32);
        for i in 0..4 {
            s.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
        }
        if c.len() < 3 {
            s.truncate(s.len() - (3 - c.len()));
            for _ in 0..(3 - c.len()) {
                s.push('=');
            }
        }
    }
    s
}

fn png_dims(bytes: &[u8]) -> (i64, i64) {
    if bytes.len() < 24 || &bytes[0..8] != b"\x89PNG\r\n\x1a\n" {
        return (0, 0);
    }
    (
        i32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]) as i64,
        i32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]) as i64,
    )
}

fn capture_screenshot() -> Result<Value, String> {
    let dir = crate::todo::state_dir();
    let path = std::path::Path::new(&dir).join("computer-capture.png");
    let out = Command::new("screencapture")
        .args(["-x", "-t", "png"])
        .arg(&path)
        .output()
        .map_err(|e| format!("Failed to run screencapture: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "screencapture failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let bytes = std::fs::read(&path).map_err(|e| format!("Failed to read capture: {e}"))?;
    let _ = std::fs::remove_file(&path);
    let (w, h) = png_dims(&bytes);
    Ok(json!({
        "mimeType": "image/png", "width": w, "height": h,
        "sizeBytes": bytes.len(), "bytesBase64": base64(&bytes),
        "capturedAt": iso_now()
    }))
}

fn iso_now() -> String {
    let out = Command::new("date").args(["-u", "+%Y-%m-%dT%H:%M:%S.000Z"]).output();
    out.ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "1970-01-01T00:00:00.000Z".to_string())
}

fn flag_mask(mods: &[String]) -> u64 {
    const CMD: u64 = 0x100000; // kCGEventFlagMaskCommand
    const ALT: u64 = 0x080000;
    const CTRL: u64 = 0x040000;
    const SHIFT: u64 = 0x020000;
    let mut m = 0u64;
    for f in mods {
        m |= match f.as_str() {
            "meta" | "cmd" | "command" => CMD,
            "ctrl" | "control" => CTRL,
            "alt" | "option" => ALT,
            "shift" => SHIFT,
            _ => 0,
        };
    }
    m
}

fn key_code(name: &str) -> Option<i64> {
    Some(match name.to_ascii_lowercase().as_str() {
        "enter" | "return" => 36, "escape" | "esc" => 53, "space" => 49, "tab" => 48,
        "backspace" | "delete" => 51, "forwarddelete" => 117,
        "arrowleft" | "left" => 123, "arrowright" | "right" => 124,
        "arrowup" | "up" => 126, "arrowdown" | "down" => 125,
        "home" => 115, "end" => 119, "pageup" => 116, "pagedown" => 121,
        "f1" => 122, "f2" => 120, "f3" => 99, "f4" => 118, "f5" => 96, "f6" => 97,
        "f7" => 98, "f8" => 100, "f9" => 101, "f10" => 109, "f11" => 103, "f12" => 111,
        "a" => 0, "s" => 1, "d" => 2, "f" => 3, "h" => 4, "g" => 5, "z" => 6, "x" => 7,
        "c" => 8, "v" => 9, "b" => 11, "q" => 12, "w" => 13, "e" => 14, "r" => 15,
        "y" => 16, "t" => 17, "o" => 31, "u" => 32, "i" => 34, "p" => 35, "l" => 37,
        "j" => 38, "k" => 40, "n" => 45, "m" => 46,
        "1" => 18, "2" => 19, "3" => 20, "4" => 21, "5" => 23, "6" => 22, "7" => 26,
        "8" => 28, "9" => 25, "0" => 29,
        "-" => 27, "=" => 24, "[" => 33, "]" => 30, "\\" => 42, ";" => 41, "'" => 39,
        "," => 43, "." => 47, "/" => 44, "`" => 50,
        _ => return None,
    })
}

fn point_of(payload: &Value) -> Option<(f64, f64)> {
    Some((payload.get("x")?.as_f64()?, payload.get("y")?.as_f64()?))
}

fn require_ax() -> Result<(), String> {
    if ax_trusted() {
        Ok(())
    } else {
        Err("Synara needs accessibility permission to control this Mac. Grant it in System Settings > Privacy & Security, then try again.".into())
    }
}

fn publish_action(action: &str, ok: bool, result: &Value, window_id: Option<&str>) {
    let mut ev = json!({"type": "computer.action", "action": action, "ok": ok});
    if let Some(d) = result.get("delivery") {
        ev["delivery"] = d.clone();
    }
    if let Some(w) = window_id {
        ev["windowId"] = json!(w);
    }
    let _ = action_channel().send(ev);
}

fn action_result(action: &str, point: Option<(f64, f64)>) -> Value {
    let mut v = json!({"computerId": COMPUTER_ID, "action": action});
    if let Some((x, y)) = point {
        v["point"] = json!({"x": x, "y": y});
    }
    v["delivery"] = json!({"path": "cgevent-hidtap", "verified": "unverifiable"});
    v
}

fn numeric_window_id(id: &str) -> Option<(u32, u32)> {
    let mut parts = id.split(':');
    if parts.next()? != "cua" {
        return None;
    }
    Some((parts.next()?.parse().ok()?, parts.next()?.parse().ok()?))
}

fn masked_activation_opted_in(bundle_id: Option<&str>) -> bool {
    let enabled = std::env::var("SYNARA_CUA_MASKED_ACTIVATION")
        .ok()
        .is_some_and(|value| matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"));
    if !enabled {
        return false;
    }
    let Some(bundle_id) = bundle_id.map(str::trim).filter(|value| !value.is_empty()) else {
        return false;
    };
    std::env::var("SYNARA_CUA_MASKED_APPS")
        .unwrap_or_default()
        .split([',', ';', ' ', '\t', '\n', '\r'])
        .any(|entry| entry.trim().eq_ignore_ascii_case(bundle_id))
}

fn activate_one(window_id: &str) -> Result<Value, String> {
    let result = helper("activate", &json!({"windowId": window_id}))?;
    if result.get("ok").and_then(Value::as_bool) == Some(true) {
        Ok(result)
    } else {
        Err(result
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("The window could not be activated.")
            .to_string())
    }
}

/// Foreground-with-restore primitive behind the agent-gateway visible-use
/// gate. Records the frontmost window, optionally masks the target, raises and
/// focuses it, then restores the prior window with explicit restore status.
fn activate_window_with_restore(payload: &Value) -> Result<Value, String> {
    require_ax()?;
    let window_id = payload
        .get("windowId")
        .and_then(Value::as_str)
        .filter(|value| numeric_window_id(value).is_some())
        .ok_or("windowId is required")?;
    let thread_id = payload.get("threadId").and_then(Value::as_str).filter(|value| !value.is_empty());
    if let Some(thread) = thread_id {
        crate::computer_manager::assert_control_authority(Some(thread))?;
        crate::computer_manager::assert_input_not_paused(Some(thread))?;
        crate::computer_manager::assert_foreground_allowed(
            Some(thread),
            payload.get("foregroundApproved").and_then(Value::as_bool).unwrap_or(false),
        )?;
    }

    let windows = list_windows()?;
    let Some(target) = windows.iter().find(|window| window.get("id").and_then(Value::as_str) == Some(window_id)).cloned() else {
        return Err(format!("Window {window_id} was not found."));
    };
    let app_name = target.get("appName").and_then(Value::as_str);
    let bundle_id = target.get("bundleId").and_then(Value::as_str);
    crate::computer_manager::assert_not_denied(app_name, bundle_id)?;
    let previous_id = windows
        .iter()
        .find(|window| {
            window.get("visible").and_then(Value::as_bool).unwrap_or(true)
                && !window.get("minimized").and_then(Value::as_bool).unwrap_or(false)
        })
        .and_then(|window| window.get("id").and_then(Value::as_str))
        .map(str::to_string);

    let mut shield_id = None;
    if masked_activation_opted_in(bundle_id) {
        let (pid, wid) = numeric_window_id(window_id).ok_or("Invalid window id.")?;
        let bounds = target.get("bounds").cloned().unwrap_or_else(|| json!({}));
        let host = crate::computer_shield::shared_host()
            .ok_or("Masked activation is opted in for this app, but the activation shield is unavailable.")?;
        let id = format!("shield-{}", crate::orchestration_engine::uuid_v4());
        let task = thread_id.map(|thread| crate::computer_shield::ShieldTask {
            thread_id: thread.to_string(),
            turn_id: payload.get("turnId").and_then(Value::as_str).map(str::to_string),
        });
        host.engage(
            crate::computer_shield::ShieldEngagement {
                shield_id: id.clone(),
                frame: crate::computer_shield::ShieldFrame {
                    x: bounds.get("x").and_then(Value::as_f64).unwrap_or(0.0),
                    y: bounds.get("y").and_then(Value::as_f64).unwrap_or(0.0),
                    width: bounds.get("width").and_then(Value::as_f64).unwrap_or(1.0),
                    height: bounds.get("height").and_then(Value::as_f64).unwrap_or(1.0),
                },
                window_id: wid,
                pid,
                label: app_name.map(str::to_string),
            },
            task.as_ref(),
        )?;
        shield_id = Some(id);
    }

    let activation_result = activate_one(window_id);
    let (restore_status, restored_window_id, note) = if activation_result.is_ok() {
        match previous_id.as_deref() {
            None => (
                "frontmost-unobservable",
                None,
                Some("No frontmost window was observable before activation, so nothing was restored.".to_string()),
            ),
            Some(previous) if previous == window_id => ("already-frontmost", None, None),
            Some(previous) => match activate_one(previous) {
                Ok(_) => ("restored", Some(previous.to_string()), None),
                Err(_) => (
                    "restore-missed",
                    Some(previous.to_string()),
                    Some(format!(
                        "Activated window {window_id:?} but could not restore the previously frontmost window {previous:?}; the desktop was left with {window_id:?} raised."
                    )),
                ),
            },
        }
    } else {
        ("not-attempted", None, None)
    };
    if let Some(id) = &shield_id
        && let Some(host) = crate::computer_shield::shared_host()
    {
        host.release(id);
    }
    activation_result?;

    let mut result = json!({
        "computerId": COMPUTER_ID,
        "action": "computer_activate_window",
        "windowId": window_id,
        "restoreStatus": restore_status,
        "masked": shield_id.is_some(),
    });
    if let Some(restored) = restored_window_id {
        result["restoredWindowId"] = json!(restored);
    }
    if let Some(note) = &note {
        result["note"] = json!(note.clone());
    }
    let mut event = json!({
        "type": "computer.action",
        "action": "computer_activate_window",
        "ok": true,
        "windowId": window_id,
        "restoreStatus": restore_status,
    });
    if let Some(note) = note {
        event["note"] = json!(note);
    }
    if shield_id.is_some() {
        event["masked"] = json!(true);
    }
    let _ = action_channel().send(event);
    Ok(result)
}

fn window_target(payload: &Value) -> Result<Value, String> {
    require_ax()?;
    let window_id = payload
        .get("windowId")
        .and_then(Value::as_str)
        .filter(|value| numeric_window_id(value).is_some())
        .ok_or("windowId is required")?;
    let windows = list_windows()?;
    let target = windows
        .iter()
        .find(|window| window.get("id").and_then(Value::as_str) == Some(window_id))
        .cloned()
        .ok_or_else(|| format!("Window {window_id} was not found."))?;
    crate::computer_manager::assert_not_denied(
        target.get("appName").and_then(Value::as_str),
        target.get("bundleId").and_then(Value::as_str),
    )?;
    Ok(target)
}

fn checked_window_helper(command: &str, payload: Value) -> Result<Value, String> {
    let result = helper(command, &payload)?;
    if result.get("ok").and_then(Value::as_bool) == Some(true) {
        Ok(result)
    } else {
        Err(result
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("The window operation was refused.")
            .to_string())
    }
}

fn finite_frame(payload: &Value) -> Result<Value, String> {
    let x = payload.get("x").and_then(Value::as_f64);
    let y = payload.get("y").and_then(Value::as_f64);
    let width = payload.get("width").and_then(Value::as_f64);
    let height = payload.get("height").and_then(Value::as_f64);
    let (Some(x), Some(y), Some(width), Some(height)) = (x, y, width, height) else {
        return Err("x/y/width/height must be finite, with positive size.".into());
    };
    if !(x.is_finite() && y.is_finite() && width.is_finite() && height.is_finite()) || width <= 0.0 || height <= 0.0 {
        return Err("x/y/width/height must be finite, with positive size.".into());
    }
    Ok(json!({"x":x,"y":y,"width":width,"height":height}))
}

fn set_window_minimized(payload: &Value) -> Result<Value, String> {
    let target = window_target(payload)?;
    let window_id = target.get("id").and_then(Value::as_str).ok_or("Invalid window id.")?;
    let minimized = payload.get("minimized").and_then(Value::as_bool).ok_or("minimized must be a boolean")?;
    let result = checked_window_helper("windowattr", json!({"windowId":window_id,"minimized":minimized}))?;
    Ok(json!({
        "computerId": COMPUTER_ID,
        "action": "computer_set_window_minimized",
        "windowId": window_id,
        "minimized": result.get("minimized").cloned().unwrap_or(json!(minimized)),
    }))
}

fn set_window_frame(payload: &Value) -> Result<Value, String> {
    let target = window_target(payload)?;
    let window_id = target.get("id").and_then(Value::as_str).ok_or("Invalid window id.")?;
    let frame = finite_frame(payload)?;
    let result = checked_window_helper("windowattr", json!({"windowId":window_id,"frame":frame}))?;
    Ok(json!({
        "computerId": COMPUTER_ID,
        "action": "computer_set_window_frame",
        "windowId": window_id,
        "frame": result.get("frame").cloned().unwrap_or(Value::Null),
    }))
}

fn kill_window_app(payload: &Value) -> Result<Value, String> {
    let target = window_target(payload)?;
    let window_id = target.get("id").and_then(Value::as_str).ok_or("Invalid window id.")?;
    let pid = target.get("pid").and_then(Value::as_i64).ok_or("The target window has no owning process.")?;
    let result = checked_window_helper("killapp", json!({"pid":pid}))?;
    Ok(json!({
        "computerId": COMPUTER_ID,
        "action": "computer_kill_app",
        "windowId": window_id,
        "pid": pid,
        "terminated": result.get("ok").cloned().unwrap_or(json!(true)),
    }))
}

fn list_apps() -> Result<Value, String> {
    let result = helper("apps", &json!({}))?;
    Ok(json!({
        "computerId": COMPUTER_ID,
        "apps": result.get("apps").cloned().unwrap_or(json!([])),
    }))
}

fn set_app_visibility(payload: &Value) -> Result<Value, String> {
    let pid = payload.get("pid").and_then(Value::as_i64).filter(|pid| *pid > 0).ok_or("pid is required")?;
    let hidden = payload.get("hidden").and_then(Value::as_bool).ok_or("hidden must be a boolean")?;
    let apps = helper("apps", &json!({}))?;
    let target = apps.get("apps").and_then(Value::as_array).and_then(|apps| {
        apps.iter().find(|app| app.get("pid").and_then(Value::as_i64) == Some(pid)).cloned()
    }).ok_or("The target application is not running.")?;
    crate::computer_manager::assert_not_denied(
        target.get("appName").and_then(Value::as_str),
        target.get("bundleId").and_then(Value::as_str),
    )?;
    let result = checked_window_helper("appvisibility", json!({"pid":pid,"hidden":hidden}))?;
    Ok(json!({
        "computerId": COMPUTER_ID,
        "action": "computer_set_app_visibility",
        "pid": pid,
        "hidden": result.get("hidden").cloned().unwrap_or(json!(hidden)),
    }))
}

fn read_clipboard() -> Result<Value, String> {
    let result = helper("clipboard", &json!({"op":"read"}))?;
    Ok(json!({
        "computerId": COMPUTER_ID,
        "text": result.get("text").cloned().unwrap_or(Value::Null),
    }))
}

fn write_clipboard(payload: &Value) -> Result<Value, String> {
    let text = payload.get("text").and_then(Value::as_str).ok_or("text is required")?;
    let result = helper("clipboard", &json!({"op":"write","text":text}))?;
    if result.get("ok").and_then(Value::as_bool) != Some(true) {
        return Err("The clipboard write was refused.".into());
    }
    Ok(json!({"computerId": COMPUTER_ID, "action": "computer_write_clipboard", "bytes": text.len()}))
}

fn paste_clipboard_text(payload: &Value) -> Result<Value, String> {
    require_ax()?;
    let text = payload.get("text").and_then(Value::as_str).ok_or("text is required")?;
    let previous = helper("clipboard", &json!({"op":"read"}))
        .ok()
        .and_then(|value| value.get("text").and_then(Value::as_str).map(str::to_string));
    let write = helper("clipboard", &json!({"op":"write","text":text}))?;
    if write.get("ok").and_then(Value::as_bool) != Some(true) {
        return Err("The clipboard write was refused.".into());
    }
    let paste_result = match key_code("v") {
        Some(code) => helper("press", &json!({"code": code, "flags": flag_mask(&["cmd".to_string()])})).map(|_| ()),
        None => Err("Paste key is unavailable".to_string()),
    };
    // Give the paste key event a short dispatch window before restoring the
    // user's prior textual clipboard. Non-text pasteboard content is outside
    // this compact native path and is documented in the result.
    std::thread::sleep(std::time::Duration::from_millis(120));
    let restored = if let Some(previous) = previous {
        helper("clipboard", &json!({"op":"write","text":previous}))
            .ok()
            .and_then(|value| value.get("ok").and_then(Value::as_bool))
            .unwrap_or(false)
    } else {
        false
    };
    paste_result?;
    Ok(json!({
        "computerId": COMPUTER_ID,
        "action": "computer_paste",
        "bytes": text.len(),
        "clipboardRestored": restored,
        "clipboardRestoreScope": if restored { "text" } else { "unavailable" },
    }))
}

fn menu_path(payload: &Value) -> Result<Vec<String>, String> {
    let path: Vec<String> = payload
        .get("path")
        .and_then(Value::as_array)
        .map(|items| {
            items.iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    if path.is_empty() || path.len() > 6 {
        return Err("path must contain one to six menu titles".into());
    }
    Ok(path)
}

fn invoke_menu(payload: &Value) -> Result<Value, String> {
    require_ax()?;
    let thread_id = payload.get("threadId").and_then(Value::as_str).filter(|value| !value.is_empty());
    if let Some(thread) = thread_id {
        crate::computer_manager::assert_control_authority(Some(thread))?;
        crate::computer_manager::assert_input_not_paused(Some(thread))?;
        crate::computer_manager::assert_foreground_allowed(
            Some(thread),
            payload.get("foregroundApproved").and_then(Value::as_bool).unwrap_or(false),
        )?;
    }
    let path = menu_path(payload)?;
    let mut request = json!({"path": path});
    let mut app_name = None;
    let mut bundle_id = None;
    if let Some(window_id) = payload.get("windowId").and_then(Value::as_str).filter(|value| numeric_window_id(value).is_some()) {
        let target = window_target(&json!({"windowId": window_id}))?;
        app_name = target.get("appName").and_then(Value::as_str).map(str::to_string);
        bundle_id = target.get("bundleId").and_then(Value::as_str).map(str::to_string);
        request["windowId"] = json!(window_id);
    } else if let Some(pid) = payload.get("pid").and_then(Value::as_i64).filter(|pid| *pid > 0) {
        let apps = helper("apps", &json!({}))?;
        let target = apps.get("apps").and_then(Value::as_array).and_then(|apps| {
            apps.iter().find(|app| app.get("pid").and_then(Value::as_i64) == Some(pid)).cloned()
        }).ok_or("The target application is not running.")?;
        app_name = target.get("appName").and_then(Value::as_str).map(str::to_string);
        bundle_id = target.get("bundleId").and_then(Value::as_str).map(str::to_string);
        request["pid"] = json!(pid);
    } else if let Some(app) = payload.get("app").and_then(Value::as_str).filter(|value| !value.trim().is_empty()) {
        app_name = Some(app.to_string());
        request["app"] = json!(app);
    } else {
        return Err("windowId, pid, or app is required".into());
    }
    crate::computer_manager::assert_not_denied(app_name.as_deref(), bundle_id.as_deref())?;
    let result = checked_window_helper("menu", request)?;
    Ok(json!({
        "computerId": COMPUTER_ID,
        "action": "computer_invoke_menu",
        "pid": result.get("pid").cloned().unwrap_or(Value::Null),
        "path": result.get("path").cloned().unwrap_or(json!(path)),
    }))
}

fn desktop_state_json() -> Value {
    let (epoch, interruptions, pauses) = crate::computer_manager::desktop_interruption_state();
    json!({
        "epoch": epoch,
        "interruptions": interruptions,
        "pauses": pauses,
    })
}

fn status_json() -> Value {
    json!({
        "provisionable": true,
        "computerId": COMPUTER_ID,
        "availability": availability(),
        "health": health(),
        "capabilities": capabilities(),
        "inputMonitor": crate::input_monitor::state_json(),
        "desktop": desktop_state_json()
    })
}

fn control_path() -> PathBuf {
    std::path::Path::new(&crate::todo::state_dir()).join("computer-control.json")
}

fn set_control_enabled(payload: &Value) -> Result<Value, String> {
    let tid = payload["threadId"].as_str().unwrap_or_default();
    let enabled = payload["enabled"].as_bool().unwrap_or(false);
    if tid.is_empty() {
        return Err("threadId is required".into());
    }
    let generation = crate::computer_control::set(tid, !enabled)?;
    if enabled {
        crate::input_monitor::control_armed(tid);
    } else {
        crate::input_monitor::control_disarmed(tid);
        if let Some(host) = crate::computer_shield::shared_host() {
            host.end_task(&crate::computer_shield::ShieldTask { thread_id: tid.to_string(), turn_id: None });
        }
    }
    Ok(json!({"enabled": enabled, "generation": generation}))
}

fn handle_shield(payload: &Value) -> Result<Value, String> {
    let args = crate::computer_shield::parse_shield_args(&payload["args"])
        .ok_or("Invalid computer shield request.")?;
    let host = crate::computer_shield::shared_host()
        .ok_or("The activation shield is not available in this build.")?;
    let thread_id = payload["threadId"].as_str().unwrap_or_default();
    let turn_id = payload["turnId"].as_str().filter(|value| !value.is_empty());
    let task = (!thread_id.is_empty()).then(|| crate::computer_shield::ShieldTask {
        thread_id: thread_id.to_string(),
        turn_id: turn_id.map(str::to_string),
    });
    match args {
        crate::computer_shield::ShieldArgs::Engage(engagement) => {
            if !thread_id.is_empty() {
                crate::computer_manager::assert_control_authority(Some(thread_id))?;
                crate::computer_manager::assert_input_not_paused(Some(thread_id))?;
            }
            host.engage(engagement.clone(), task.as_ref())?;
            Ok(json!({"engaged": true, "shield_id": engagement.shield_id}))
        }
        crate::computer_shield::ShieldArgs::Release { shield_id } => {
            host.release(&shield_id);
            Ok(json!({"released": true}))
        }
        crate::computer_shield::ShieldArgs::ReleaseAll => {
            Ok(json!({"released": host.release_all()}))
        }
    }
}

/// Resolve a semantic target (windowId/nodePath/ref/role/label) to a live AX
/// element and apply `op` through the helper. Returns the element's
/// activation point plus its role/label for the action result.
fn ax_action(payload: &Value, op: &str) -> Result<Value, String> {
    require_ax()?;
    let mut a = json!({"op": op});
    for k in ["windowId", "role", "label"] {
        if let Some(s) = payload[k].as_str() {
            a[k] = json!(s);
        }
    }
    if let Some(np) = payload.get("nodePath") {
        a["nodePath"] = np.clone();
    }
    for k in ["ref", "refOrdinal"] {
        if let Some(n) = payload[k].as_i64() {
            a[k] = json!(n);
        }
    }
    match op {
        "setvalue" => a["value"] = payload["value"].clone(),
        "perform" => a["action"] = payload["action"].clone(),
        "selecttext" => {
            a["start"] = payload["start"].clone();
            a["length"] = payload["length"].clone();
        }
        _ => {}
    }
    // Semantic targeting needs a window or a semantic selector; a bare
    // x/y target resolves to the frontmost window's tree.
    let r = helper("axaction", &a)?;
    if r["ok"].as_bool() != Some(true) {
        return Err(r["error"].as_str().unwrap_or("The target did not resolve.").to_string());
    }
    Ok(r)
}

fn ax_result(action: &str, resolved: &Value) -> Value {
    let mut v = json!({"computerId": COMPUTER_ID, "action": action});
    if let Some(p) = resolved.get("point") {
        v["point"] = p.clone();
    }
    v["delivery"] = json!({"path": "ax-write", "verified": "unverifiable"});
    v
}

/// Returns Some(result) when this module handles `tag` natively on macOS.
#[cfg(target_os = "macos")]
pub fn handle(tag: &str, payload: &Value) -> Option<Result<Value, String>> {
    const MISS: &str = "\u{1}native-miss";
    // Control-plane guards gate every mutating action before dispatch, the
    // manager semantics upstream asserts at the same boundary: the owning
    // conversation must still hold control, input must not be paused, and
    // denylisted targets never execute. Reading the desktop stays open.
    let owner_opt = payload["threadId"].as_str().map(str::to_string);
    let owner = owner_opt.as_deref().unwrap_or("");
    let mutating = matches!(
        tag,
        "computer.click" | "computer.doubleClick" | "computer.rightClick" | "computer.input.click"
            | "computer.moveCursor" | "computer.drag" | "computer.scroll" | "computer.input.scroll"
            | "computer.typeText" | "computer.input.typeText" | "computer.input.key" | "computer.pressKey" | "computer.hotkey"
            | "computer.selectText" | "computer.setValue" | "computer.performAction" | "computer.launchApp" | "computer.activateWindow"
            | "computer.setWindowMinimized" | "computer.setWindowFrame" | "computer.killApp" | "computer.invokeMenu"
            | "computer.writeClipboard" | "computer.paste" | "computer.setAppVisibility"
    );
    if mutating && !owner.is_empty() {
        if let Err(e) = crate::computer_manager::assert_control_authority(Some(owner)) {
            return Some(Err(e));
        }
        if let Err(e) = crate::computer_manager::assert_input_not_paused(Some(owner)) {
            return Some(Err(e));
        }
    }
    if tag == "computer.launchApp" {
        let app = payload["app"].as_str().unwrap_or("");
        if let Err(e) = crate::computer_manager::assert_not_denied(Some(app), None) {
            return Some(Err(e));
        }
    }
    let user_input = matches!(
        tag,
        "computer.input.click" | "computer.input.scroll" | "computer.input.key" | "computer.input.typeText"
    );
    let agent_mutating = mutating && !user_input;
    if agent_mutating {
        if owner.is_empty() {
            return Some(Err("Computer input requires an owning conversation; no input was dispatched.".into()));
        }
        if let Err(e) = crate::input_monitor::ensure_input_ready(owner) {
            return Some(Err(e));
        }
        crate::computer_manager::input_started(owner);
    }
    let desktop_baseline = agent_mutating.then(crate::computer_manager::desktop_epoch);
    let r = (|| -> Result<Value, String> {
        match tag {
            "computer.getStatus" => Ok(status_json()),
            "computer.getCursorPosition" => {
                let cursor = helper("cursor", &json!({}))?;
                Ok(json!({
                    "computerId": COMPUTER_ID,
                    "x": cursor.get("x").cloned().unwrap_or(Value::Null),
                    "y": cursor.get("y").cloned().unwrap_or(Value::Null),
                }))
            }
            "computer.getScreenSize" => Ok(json!({
                "computerId": COMPUTER_ID,
                "screenSize": screen_size()?,
                "availability": availability()
            })),
            "computer.screenshot" => {
                let screenshot = capture_screenshot()?;
                Ok(json!({
                    "computerId": COMPUTER_ID,
                    "screenshot": screenshot,
                    "capturedAt": iso_now(),
                }))
            }
            "computer.listWindows" => Ok(json!({
                "computerId": COMPUTER_ID,
                "windows": list_windows()?,
                "availability": availability()
            })),
            "computer.getThreadState" => {
                let mut v = json!({
                    "computerId": COMPUTER_ID,
                    "threadId": payload["threadId"].as_str().unwrap_or(""),
                    "version": 0,
                    "windows": list_windows()?,
                    "screenSize": screen_size()?,
                    "agentActive": false,
                    "controlledByOtherThread": false,
                    "availability": availability(),
                    "health": health(),
                    "capabilities": capabilities(),
                    "lastError": null,
                    "desktop": desktop_state_json()
                });
                if let Ok(c) = helper("cursor", &json!({})) {
                    v["cursor"] = json!({"x": c["x"], "y": c["y"]});
                }
                Ok(v)
            }
            "computer.getState" => {
                let mut v = json!({
                    "computerId": COMPUTER_ID,
                    "windows": list_windows()?,
                    "screenSize": screen_size()?,
                    "availability": availability(),
                    "capturedAt": iso_now()
                });
                if payload["includeScreenshot"].as_bool() == Some(true) {
                    v["screenshot"] = capture_screenshot()?;
                }
                // Accessibility tree of the target window (or the frontmost
                // app's focused window). A failed walk is a partial state,
                // never a hard error — the window list still stands.
                let mut ta = json!({});
                if let Some(w) = payload["windowId"].as_str() {
                    ta["windowId"] = json!(w);
                }
                match helper("tree", &ta) {
                    Ok(t) if t["ok"].as_bool() == Some(true) => {
                        v["root"] = t["root"].clone();
                        if payload["includeText"].as_bool() == Some(true) {
                            let mut lines = Vec::new();
                            collect_text(&t["root"], &mut lines);
                            if let Some(needle) = payload["labelContains"].as_str() {
                                let n = needle.to_lowercase();
                                lines.retain(|l| l.to_lowercase().contains(&n));
                            }
                            v["text"] = json!(lines.join("\n"));
                        }
                    }
                    _ => {
                        v["accessibility"] = json!({
                            "status": "unavailable",
                            "unavailableWindowIds": []
                        });
                    }
                }
                if payload["modelObservation"].as_bool() == Some(true)
                    && (v.get("screenshot").is_some() || v.get("root").is_some())
                {
                    if let Some(thread_id) = payload["threadId"].as_str().filter(|value| !value.is_empty()) {
                        crate::computer_manager::clear_input_pause(thread_id);
                    }
                }
                Ok(v)
            }
            "computer.provision" => Ok(json!({
                "summary": "Computer permissions are ready. Send a message to continue; no action is retried automatically.",
                "status": status_json()
            })),
            "computer.setValue" => {
                let r = ax_action(payload, "setvalue")?;
                let mut v = ax_result("computer_set_value", &r);
                if let Some(val) = r.get("value") {
                    v["value"] = val.clone();
                }
                Ok(v)
            }
            "computer.performAction" => {
                let r = ax_action(payload, "perform")?;
                Ok(ax_result("computer_perform_action", &r))
            }
            "computer.selectText" => {
                let r = ax_action(payload, "selecttext")?;
                Ok(ax_result("computer_select_text", &r))
            }
            "computer.listApps" => list_apps(),
            "computer.setAppVisibility" => set_app_visibility(payload),
            "computer.readClipboard" => read_clipboard(),
            "computer.writeClipboard" => write_clipboard(payload),
            "computer.paste" => paste_clipboard_text(payload),
            "computer.noteUserInput" => {
                crate::computer_manager::note_user_desktop_input();
                let thread_id = payload["threadId"].as_str().unwrap_or("");
                if !thread_id.is_empty() {
                    crate::computer_manager::record_input_pause(
                        thread_id,
                        json!({
                            "kind": "human-takeover",
                            "message": "You used the keyboard or mouse while the agent was controlling the computer. Observe the desktop again before continuing; no further input was dispatched."
                        }),
                    );
                }
                Ok(json!({"accepted": true, "computerId": COMPUTER_ID}))
            }
            "computer.setControlEnabled" => set_control_enabled(payload),
            "computer.activateWindow" => activate_window_with_restore(payload),
            "computer.invokeMenu" => invoke_menu(payload),
            "computer.setWindowMinimized" => set_window_minimized(payload),
            "computer.setWindowFrame" => set_window_frame(payload),
            "computer.killApp" => kill_window_app(payload),
            "computer.shield" => handle_shield(payload),
            "computer.launchApp" => {
                let app = payload["app"].as_str().unwrap_or("");
                if app.is_empty() {
                    return Err("app is required".into());
                }
                let mut cmd = Command::new("open");
                cmd.args(["-n", "-a", app]);
                if let Some(args) = payload["arguments"].as_array() {
                    cmd.arg("--args");
                    for a in args {
                        if let Some(s) = a.as_str() {
                            cmd.arg(s);
                        }
                    }
                }
                let out = cmd.output().map_err(|e| format!("Failed to launch {app}: {e}"))?;
                if !out.status.success() {
                    return Err(format!(
                        "Failed to launch {app}: {}",
                        String::from_utf8_lossy(&out.stderr).trim()
                    ));
                }
                Ok(json!({
                    "computerId": COMPUTER_ID, "app": app,
                    "windowStatus": "not_checked", "window": null
                }))
            }
            "computer.click" | "computer.doubleClick" | "computer.rightClick" | "computer.input.click" => {
                require_ax()?;
                let (x, y) = point_of(payload).ok_or("A coordinate point is required.")?;
                let (button, count) = match tag {
                    "computer.rightClick" => ("right", 1),
                    "computer.doubleClick" => ("left", 2),
                    "computer.input.click" => {
                        let b = payload["button"].as_str().unwrap_or("left");
                        let c = payload["clickCount"].as_u64().unwrap_or(1) as u32;
                        (if b == "right" { "right" } else { "left" }, c)
                    }
                    _ => ("left", 1),
                };
                helper("click", &json!({"x": x, "y": y, "button": button, "count": count}))?;
                Ok(action_result(tag.strip_prefix("computer.").unwrap_or(tag), Some((x, y))))
            }
            "computer.moveCursor" => {
                require_ax()?;
                let (x, y) = point_of(payload).ok_or("A coordinate point is required.")?;
                helper("move", &json!({"x": x, "y": y}))?;
                Ok(action_result("moveCursor", Some((x, y))))
            }
            "computer.drag" => {
                require_ax()?;
                let (fx, fy) = point_of(&payload["from"]).ok_or("A from point is required.")?;
                let (tx, ty) = point_of(&payload["to"]).ok_or("A to point is required.")?;
                helper("drag", &json!({"fx": fx, "fy": fy, "tx": tx, "ty": ty}))?;
                Ok(action_result("drag", Some((tx, ty))))
            }
            "computer.scroll" | "computer.input.scroll" => {
                require_ax()?;
                let (x, y) = point_of(payload).ok_or("A coordinate point is required.")?;
                let dx = payload["deltaX"].as_f64().unwrap_or(0.0);
                let dy = payload["deltaY"].as_f64().unwrap_or(0.0);
                helper("scroll", &json!({"x": x, "y": y, "dx": dx, "dy": dy}))?;
                let mut v = action_result(tag.strip_prefix("computer.").unwrap_or(tag), Some((x, y)));
                v["scroll"] = json!({
                    "requested": {"deltaX": dx, "deltaY": dy},
                    "injected": {"deltaX": dx, "deltaY": dy},
                    "routes": ["wheel"]
                });
                Ok(v)
            }
            "computer.typeText" | "computer.input.typeText" => {
                require_ax()?;
                let text = payload["text"].as_str().unwrap_or("");
                helper("type", &json!({"text": text}))?;
                let mut v = action_result("typeText", None);
                v["value"] = json!(text);
                Ok(v)
            }
            "computer.pressKey" | "computer.input.key" => {
                require_ax()?;
                let key = payload["key"].as_str().unwrap_or("");
                let mods: Vec<String> = payload["modifiers"]
                    .as_array()
                    .map(|a| a.iter().filter_map(|m| m.as_str().map(String::from)).collect())
                    .unwrap_or_default();
                if let Some(code) = key_code(key) {
                    helper("press", &json!({"code": code, "flags": flag_mask(&mods)}))?;
                } else if key.chars().count() == 1 {
                    helper("type", &json!({"text": key}))?;
                } else {
                    return Err(format!("Unknown key: {key}"));
                }
                let mut v = action_result(tag.strip_prefix("computer.").unwrap_or(tag), None);
                v["value"] = json!(key);
                Ok(v)
            }
            "computer.hotkey" => {
                require_ax()?;
                let keys: Vec<String> = payload["keys"]
                    .as_array()
                    .map(|a| a.iter().filter_map(|k| k.as_str().map(String::from)).collect())
                    .unwrap_or_default();
                if keys.is_empty() {
                    return Err("keys is required".into());
                }
                let (mods, key) = keys.split_at(keys.len() - 1);
                let Some(code) = key_code(&key[0]) else {
                    return Err(format!("Unknown key: {}", key[0]));
                };
                helper("press", &json!({"code": code, "flags": flag_mask(&mods.to_vec())}))?;
                let mut v = action_result("hotkey", None);
                v["value"] = json!(keys.join("+"));
                Ok(v)
            }
            _ => return Err(MISS.into()),
        }
    })();
    if agent_mutating {
        crate::computer_manager::input_finished(owner);
    }
    let r = match (r, desktop_baseline) {
        (Ok(value), Some(baseline)) if crate::computer_manager::desktop_epoch() != baseline => {
            let _ = value;
            Err("The desktop changed while this operation was in flight. Observe again before continuing; do not replay an uncertain action.".into())
        }
        (result, _) => result,
    };
    let outcome = match r {
        Err(m) if m == MISS => None,
        Ok(v) => {
            if let Some(name) = tag
                .strip_prefix("computer.")
                .filter(|n| ACTION_TAGS.contains(n))
            {
                publish_action(name, true, &v, payload["windowId"].as_str());
            }
            Some(Ok(v))
        }
        _ => Some(r),
    };
    // Every handled action leaves an audit record with its effect — the
    // sensitive payload never crosses into it, the log summarizes.
    if outcome.is_some() {
        let effect = match &outcome {
            Some(Ok(_)) => "performed",
            Some(Err(e)) if e.contains("denylist") || e.contains("security") => "refused",
            Some(Err(_)) => "error",
            None => "performed",
        };
        let mut entry = json!({
            "tool": tag.strip_prefix("computer.").map(|n| format!("computer_{}", n.replace('.', "_"))).unwrap_or_else(|| tag.to_string()),
            "effect": effect,
            "layer": "server-manager",
        });
        if let Some(t) = owner_opt {
            entry["threadId"] = json!(t);
        }
        if let Some(args) = payload.get("args").cloned().or_else(|| {
            let mut shallow = serde_json::Map::new();
            for key in ["x", "y", "text", "value", "key", "keys", "button", "app", "direction", "amount"] {
                if let Some(v) = payload.get(key) {
                    shallow.insert(key.to_string(), v.clone());
                }
            }
            if shallow.is_empty() { None } else { Some(Value::Object(shallow)) }
        }) {
            entry["args"] = args;
        }
        if let Some(Err(e)) = &outcome {
            entry["code"] = json!(e);
        }
        crate::computer_audit::record(entry);
    }
    outcome
}

const ACTION_TAGS: &[&str] = &[
    "click", "doubleClick", "rightClick", "moveCursor", "drag", "scroll",
    "typeText", "pressKey", "hotkey", "setValue", "performAction",
    "selectText", "input.click", "input.scroll", "input.key", "invokeMenu",
];

fn collect_text(node: &Value, out: &mut Vec<String>) {
    if let Some(l) = node["label"].as_str().filter(|s| !s.is_empty()) {
        out.push(l.to_string());
    } else if let Some(val) = node["value"].as_str().filter(|s| !s.is_empty()) {
        out.push(val.to_string());
    }
    if let Some(kids) = node["children"].as_array() {
        for k in kids {
            collect_text(k, out);
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn handle(_tag: &str, _payload: &Value) -> Option<Result<Value, String>> {
    None
}
