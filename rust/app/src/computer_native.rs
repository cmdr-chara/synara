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

const COMPUTER_ID: &str = "desktop";

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

fn status_json() -> Value {
    json!({
        "provisionable": true,
        "computerId": COMPUTER_ID,
        "availability": availability(),
        "health": health(),
        "capabilities": capabilities()
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
    let path = control_path();
    let mut state: Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_else(|| json!({}));
    let prev = state[tid]["generation"].as_i64().unwrap_or(-1);
    let generation = if enabled { prev.max(0) } else { prev.max(0) + 1 };
    state[tid] = json!({"disabled": !enabled, "generation": generation});
    let tmp = path.with_extension("json.tmp");
    let mut f = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
    f.write_all(serde_json::to_string(&state).unwrap().as_bytes())
        .map_err(|e| e.to_string())?;
    drop(f);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
    }
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    Ok(json!({"enabled": enabled, "generation": generation}))
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
    let r = (|| -> Result<Value, String> {
        match tag {
            "computer.getStatus" => Ok(status_json()),
            "computer.getScreenSize" => Ok(json!({
                "computerId": COMPUTER_ID,
                "screenSize": screen_size()?,
                "availability": availability()
            })),
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
                    "lastError": null
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
            "computer.setControlEnabled" => set_control_enabled(payload),
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
            "computer.typeText" => {
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
    match r {
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
    }
}

const ACTION_TAGS: &[&str] = &[
    "click", "doubleClick", "rightClick", "moveCursor", "drag", "scroll",
    "typeText", "pressKey", "hotkey", "setValue", "performAction",
    "selectText", "input.click", "input.scroll", "input.key",
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
