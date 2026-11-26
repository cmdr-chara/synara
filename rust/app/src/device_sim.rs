//! iOS simulator device backend over `xcrun simctl` — slice 1 of the device
//! control plane port: discovery, boot/shutdown and screenshots. The upstream
//! backend hides input injection behind a helper process; discovery, lifecycle
//! and capture are pure simctl and ported here directly.

use serde_json::{json, Value};
use std::process::Command;

fn simctl(args: &[&str]) -> Result<String, String> {
    let out = Command::new("xcrun")
        .arg("simctl")
        .args(args)
        .output()
        .map_err(|e| format!("failed to run xcrun simctl: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "simctl {} failed: {}",
            args.first().copied().unwrap_or(""),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

fn map_state(raw: &str) -> &'static str {
    match raw {
        "Booted" => "booted",
        "Booting" => "booting",
        "Shutting Down" => "shutting-down",
        _ => "shutdown",
    }
}

fn format_runtime(identifier: &str) -> String {
    // com.apple.CoreSimulator.SimRuntime.iOS-17-0 -> iOS 17.0
    let tail = identifier.rsplit('.').next().unwrap_or(identifier);
    let mut parts = tail.split('-');
    let os = parts.next().unwrap_or(tail);
    let version: Vec<&str> = parts.collect();
    if version.is_empty() {
        os.to_string()
    } else {
        format!("{os} {}", version.join("."))
    }
}

/// `simctl list devices --json` flattened into contract-shaped descriptors.
pub fn list_devices(include_shutdown: bool) -> Result<Value, String> {
    let raw = simctl(&["list", "devices", "--json"])?;
    let parsed: Value = serde_json::from_str(&raw)
        .map_err(|_| "could not parse simctl device list".to_string())?;
    let mut devices = Vec::new();
    if let Some(by_runtime) = parsed.get("devices").and_then(|d| d.as_object()) {
        for (runtime_id, list) in by_runtime {
            let runtime = format_runtime(runtime_id);
            if let Some(arr) = list.as_array() {
                for dev in arr {
                    if dev.get("isAvailable").and_then(|v| v.as_bool()) == Some(false) {
                        continue;
                    }
                    let udid = dev.get("udid").and_then(|v| v.as_str()).unwrap_or("");
                    let name = dev.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    if udid.is_empty() || name.is_empty() {
                        continue;
                    }
                    let state = map_state(dev.get("state").and_then(|v| v.as_str()).unwrap_or(""));
                    if !include_shutdown && state == "shutdown" {
                        continue;
                    }
                    devices.push(json!({
                        "platform": "ios-simulator",
                        "udid": udid,
                        "name": name,
                        "runtime": runtime,
                        "state": state,
                        "bootSource": "user",
                    }));
                }
            }
        }
    }
    Ok(json!({ "devices": devices }))
}

pub fn boot_device(udid: &str) -> Result<Value, String> {
    match simctl(&["boot", udid]) {
        Ok(_) => {}
        Err(e) => {
            // "Unable to boot device in current state: Booted" is success-ish.
            if !e.contains("Booted") {
                return Err(e);
            }
        }
    }
    simctl(&["bootstatus", udid]).map_err(|e| format!("bootstatus: {e}"))?;
    Ok(json!({ "udid": udid, "state": "booted" }))
}

pub fn shutdown_device(udid: &str) -> Result<Value, String> {
    simctl(&["shutdown", udid])?;
    Ok(json!({ "udid": udid, "state": "shutdown" }))
}

/// Capture a PNG screenshot of a booted device; returns base64 plus the file
/// path the capture landed in.
pub fn screenshot(udid: &str) -> Result<Value, String> {
    let dir = std::path::Path::new(&crate::todo::state_dir()).join("device-shots");
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {e}"))?;
    let file = dir.join(format!("{udid}-{}.png", uuid_v4_short()));
    let path = file.to_string_lossy().to_string();
    simctl(&["io", udid, "screenshot", &path])?;
    let bytes = std::fs::read(&path).map_err(|e| format!("read capture: {e}"))?;
    use base64::Engine;
    Ok(json!({
        "udid": udid,
        "path": path,
        "base64": base64::engine::general_purpose::STANDARD.encode(&bytes),
        "byteCount": bytes.len(),
    }))
}

fn uuid_v4_short() -> String {
    crate::orchestration_engine::uuid_v4().replace('-', "")[..12].to_string()
}


// ---- input injection through the native helper -----------------------------

/// Attach + normalize + forward one input method. Geometry comes back from
/// the helper's `attach`; coordinates arrive in device points and the helper
/// protocol wants 0..1 normalized, matching upstream's `helper.normalize`.
fn with_attachment<F>(udid: &str, f: F) -> Result<Value, String>
where
    F: FnOnce(f64, f64) -> Result<Value, String>,
{
    let geometry = crate::device_helper::attach(udid)?;
    let w = geometry
        .get("pointWidth")
        .and_then(|v| v.as_f64())
        .ok_or("attach returned no pointWidth")?;
    let h = geometry
        .get("pointHeight")
        .and_then(|v| v.as_f64())
        .ok_or("attach returned no pointHeight")?;
    f(w, h)
}

fn norm(x: f64, extent: f64) -> f64 {
    if extent <= 0.0 {
        0.0
    } else {
        (x / extent).clamp(0.0, 1.0)
    }
}

fn device_tap(udid: &str, payload: &Value) -> Result<Value, String> {
    let x = payload.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let y = payload.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
    with_attachment(udid, |w, h| {
        let mut params = json!({ "x": norm(x, w), "y": norm(y, h) });
        if let Some(hold) = payload.get("holdMs").and_then(|v| v.as_u64()) {
            params["holdMs"] = json!(hold);
        }
        crate::device_helper::request("tap", params)?;
        Ok(json!({ "udid": udid, "performed": true }))
    })
}

fn device_swipe(udid: &str, payload: &Value) -> Result<Value, String> {
    with_attachment(udid, |w, h| {
        let fx = payload.get("fromX").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let fy = payload.get("fromY").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let tx = payload.get("toX").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let ty = payload.get("toY").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let mut params = json!({
            "fromX": norm(fx, w), "fromY": norm(fy, h),
            "toX": norm(tx, w), "toY": norm(ty, h),
        });
        if let Some(d) = payload.get("durationMs").and_then(|v| v.as_u64()) {
            params["durationMs"] = json!(d);
        }
        if let Some(edge) = payload.get("edge").and_then(|v| v.as_str()) {
            params["edge"] = json!(edge);
        }
        crate::device_helper::request("swipe", params)?;
        Ok(json!({ "udid": udid, "performed": true }))
    })
}

fn device_type_text(udid: &str, payload: &Value) -> Result<Value, String> {
    let text = payload.get("text").and_then(|v| v.as_str()).unwrap_or("");
    crate::device_helper::attach(udid)?;
    crate::device_helper::request("text", json!({ "text": text }))?;
    Ok(json!({ "udid": udid, "performed": true }))
}

fn device_key(udid: &str, payload: &Value) -> Result<Value, String> {
    let key = payload.get("key").and_then(|v| v.as_str()).unwrap_or("");
    crate::device_helper::attach(udid)?;
    crate::device_helper::request("key", json!({ "key": key }))?;
    Ok(json!({ "udid": udid, "performed": true }))
}

fn device_button(udid: &str, payload: &Value) -> Result<Value, String> {
    let button = payload.get("button").and_then(|v| v.as_str()).unwrap_or("");
    crate::device_helper::attach(udid)?;
    crate::device_helper::request("button", json!({ "button": button }))?;
    Ok(json!({ "udid": udid, "performed": true }))
}

fn device_describe_ui(udid: &str) -> Result<Value, String> {
    crate::device_helper::attach(udid)?;
    let tree = crate::device_helper::request("describe-ui", json!({}))?;
    Ok(json!({ "udid": udid, "tree": tree }))
}


// ---- video capture -----------------------------------------------------------

static RECORDINGS: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<String, std::process::Child>>,
> = std::sync::OnceLock::new();

fn recordings() -> &'static std::sync::Mutex<
    std::collections::HashMap<String, std::process::Child>,
> {
    RECORDINGS.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

fn start_recording(udid: &str) -> Result<Value, String> {
    {
        let map = recordings().lock().map_err(|_| "recordings lock")?;
        if map.contains_key(udid) {
            return Err(format!("Device {udid} is already recording"));
        }
    }
    let dir = std::path::Path::new(&crate::todo::state_dir()).join("device-recordings");
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {e}"))?;
    let file = dir.join(format!("{udid}-{}.mp4", uuid_v4_short()));
    let path = file.to_string_lossy().to_string();
    let child = std::process::Command::new("xcrun")
        .args(["simctl", "io", udid, "recordVideo", &path])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("recordVideo spawn: {e}"))?;
    recordings()
        .lock()
        .map_err(|_| "recordings lock")?
        .insert(udid.to_string(), child);
    Ok(json!({ "udid": udid, "recording": true, "path": path }))
}

fn stop_recording(udid: &str) -> Result<Value, String> {
    let mut child = recordings()
        .lock()
        .map_err(|_| "recordings lock")?
        .remove(udid)
        .ok_or_else(|| format!("Device {udid} is not recording"))?;
    // SIGINT so simctl finalizes the mp4 instead of truncating it.
    unsafe {
        libc::kill(child.id() as i32, libc::SIGINT);
    }
    let _ = child.wait();
    Ok(json!({ "udid": udid, "recording": false }))
}


// ---- thread-scoped attachment (pragmatic manager slice) ----------------------
//
// Upstream's DeviceManager versions one ThreadDeviceState per thread with
// boot ownership, stream lifecycle and idle teardown. This slice implements
// the observable contract — attach boots the device when needed, attaches the
// input helper and records the attachment per thread; detach releases it —
// while the stream-lifecycle machinery stays upstream-shaped future work.

static ATTACHMENTS: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<String, Value>>,
> = std::sync::OnceLock::new();

fn attachments() -> &'static std::sync::Mutex<std::collections::HashMap<String, Value>> {
    ATTACHMENTS.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

fn thread_key(payload: &Value) -> String {
    payload
        .get("threadId")
        .and_then(|v| v.as_str())
        .unwrap_or("shell")
        .to_string()
}

fn device_attach(payload: &Value) -> Result<Value, String> {
    let udid = payload.get("udid").and_then(|v| v.as_str()).unwrap_or("");
    if udid.is_empty() {
        return Err("udid required".into());
    }
    let key = thread_key(payload);
    // Boot when shutdown — attach on a shutdown simulator never works.
    let devices = list_devices(true)?;
    let state = devices
        .get("devices")
        .and_then(|d| d.as_array())
        .and_then(|arr| {
            arr.iter()
                .find(|d| d.get("udid").and_then(|v| v.as_str()) == Some(udid))
                .and_then(|d| d.get("state").and_then(|v| v.as_str()))
                .map(str::to_string)
        })
        .ok_or_else(|| format!("no such device {udid}"))?;
    if state == "shutdown" {
        boot_device(udid)?;
    }
    let geometry = crate::device_helper::attach(udid)?;
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let value = json!({
        "threadId": key,
        "attachedDeviceUdid": udid,
        "attachmentSource": payload.get("source").and_then(|v| v.as_str()).unwrap_or("user"),
        "attachedAt": now,
        "geometry": geometry,
        "version": 1,
    });
    attachments()
        .lock()
        .map_err(|_| "attachments lock")?
        .insert(key, value.clone());
    Ok(value)
}

fn device_detach(payload: &Value) -> Result<Value, String> {
    let key = thread_key(payload);
    let existed = attachments()
        .lock()
        .map_err(|_| "attachments lock")?
        .remove(&key)
        .is_some();
    Ok(json!({ "threadId": key, "detached": existed }))
}

fn device_thread_state(payload: &Value) -> Result<Value, String> {
    let key = thread_key(payload);
    let map = attachments().lock().map_err(|_| "attachments lock")?;
    match map.get(&key) {
        Some(state) => Ok(state.clone()),
        None => Ok(json!({
            "threadId": key,
            "attachedDeviceUdid": Value::Null,
            "version": 0,
        })),
    }
}

pub fn handles(tag: &str) -> bool {
    matches!(
        tag,
        "device.list"
            | "device.boot"
            | "device.shutdown"
            | "device.screenshot"
            | "device.installApp"
            | "device.launchApp"
            | "device.openUrl"
            | "device.tap"
            | "device.swipe"
            | "device.typeText"
            | "device.keyEvent"
            | "device.pressButton"
            | "device.describeUi"
            | "device.startRecording"
            | "device.stopRecording"
            | "device.attach"
            | "device.detach"
            | "device.getThreadState"
    )
}

pub fn handle(tag: &str, payload: &Value) -> Result<Value, String> {
    let udid = payload.get("udid").and_then(|v| v.as_str()).unwrap_or("");
    match tag {
        "device.list" => list_devices(
            payload
                .get("includeShutdown")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
        ),
        "device.boot" => {
            if udid.is_empty() {
                Err("udid required".into())
            } else {
                boot_device(udid)
            }
        }
        "device.shutdown" => {
            if udid.is_empty() {
                Err("udid required".into())
            } else {
                shutdown_device(udid)
            }
        }
        "device.screenshot" => {
            if udid.is_empty() {
                Err("udid required".into())
            } else {
                screenshot(udid)
            }
        }
        "device.installApp" => {
            let app_path = payload.get("appPath").and_then(|v| v.as_str()).unwrap_or("");
            if udid.is_empty() || app_path.is_empty() {
                Err("udid and appPath required".into())
            } else {
                simctl(&["install", udid, app_path])?;
                Ok(json!({ "udid": udid, "installed": true }))
            }
        }
        "device.launchApp" => {
            let bundle = payload
                .get("bundleId")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if udid.is_empty() || bundle.is_empty() {
                Err("udid and bundleId required".into())
            } else {
                if payload.get("terminateExisting").and_then(|v| v.as_bool()) == Some(true) {
                    let _ = simctl(&["terminate", udid, bundle]);
                }
                // `simctl launch` prints `<bundleId>: <pid>`.
                let out = simctl(&["launch", udid, bundle])?;
                let pid = out
                    .trim()
                    .rsplit(':')
                    .next()
                    .and_then(|p| p.trim().parse::<u64>().ok());
                Ok(json!({ "udid": udid, "bundleId": bundle, "pid": pid }))
            }
        }
        "device.tap" => device_tap(udid, payload),
        "device.swipe" => device_swipe(udid, payload),
        "device.typeText" => device_type_text(udid, payload),
        "device.keyEvent" => device_key(udid, payload),
        "device.pressButton" => device_button(udid, payload),
        "device.describeUi" => device_describe_ui(udid),
        "device.attach" => device_attach(payload),
        "device.detach" => device_detach(payload),
        "device.getThreadState" => device_thread_state(payload),
        "device.startRecording" => {
            if udid.is_empty() { Err("udid required".into()) } else { start_recording(udid) }
        }
        "device.stopRecording" => {
            if udid.is_empty() { Err("udid required".into()) } else { stop_recording(udid) }
        }
        "device.openUrl" => {
            let url = payload.get("url").and_then(|v| v.as_str()).unwrap_or("");
            if udid.is_empty() || url.is_empty() {
                Err("udid and url required".into())
            } else {
                simctl(&["openurl", udid, url])?;
                Ok(json!({ "udid": udid, "opened": true }))
            }
        }
        _ => Err(format!("unknown device tag {tag}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_formatting_matches_upstream() {
        assert_eq!(
            format_runtime("com.apple.CoreSimulator.SimRuntime.iOS-17-0"),
            "iOS 17.0"
        );
        assert_eq!(
            format_runtime("com.apple.CoreSimulator.SimRuntime.iOS-26-0"),
            "iOS 26.0"
        );
    }

    #[test]
    fn state_mapping() {
        assert_eq!(map_state("Booted"), "booted");
        assert_eq!(map_state("Shutdown"), "shutdown");
        assert_eq!(map_state("Shutting Down"), "shutting-down");
    }
}
