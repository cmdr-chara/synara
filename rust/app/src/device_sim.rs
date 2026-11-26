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

pub fn handles(tag: &str) -> bool {
    matches!(
        tag,
        "device.list" | "device.boot" | "device.shutdown" | "device.screenshot"
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
