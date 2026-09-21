//! All Apple command knowledge stays here. No private native APIs or pretend
//! physical-device support. xcrun must be supplied by the user's Xcode install.
use super::*;
use serde::Deserialize;
use std::collections::BTreeMap;

pub(super) fn executable() -> Result<PathBuf, RuntimeError> {
    if cfg!(target_os = "macos") {
        Ok(PathBuf::from("/usr/bin/xcrun"))
    } else {
        Err(RuntimeError::Unsupported(
            "Apple Simulator requires macOS and an installed Xcode simulator runtime".into(),
        ))
    }
}
pub(super) fn discovery_args() -> Vec<String> {
    ["simctl", "list", "devices", "--json"]
        .map(String::from)
        .to_vec()
}
pub(super) fn capture_args(id: String) -> Vec<String> {
    vec![
        "simctl".into(),
        "io".into(),
        id,
        "screenshot".into(),
        "--type=png".into(),
        "-".into(),
    ]
}
pub(super) fn lifecycle_args(id: String, running: bool) -> Vec<String> {
    vec![
        "simctl".into(),
        if running { "boot" } else { "shutdown" }.into(),
        id,
    ]
}
pub(super) fn valid_id(id: &str) -> bool {
    id.len() == 36
        && id.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}
#[derive(Deserialize)]
struct List {
    devices: BTreeMap<String, Vec<Simulator>>,
}
#[derive(Deserialize)]
struct Simulator {
    udid: String,
    name: String,
    state: String,
    #[serde(rename = "isAvailable")]
    available: bool,
}
pub(super) fn parse(bytes: &[u8]) -> Result<Vec<ToolDevice>, RuntimeError> {
    if bytes.len() > DISCOVERY_LIMIT {
        return Err(RuntimeError::Limit);
    }
    let list: List = serde_json::from_slice(bytes)
        .map_err(|_| RuntimeError::Invalid("Invalid simctl discovery response".into()))?;
    let mut devices = Vec::new();
    for (runtime, simulators) in list.devices {
        if runtime.len() > 512 || runtime.chars().any(char::is_control) {
            return Err(RuntimeError::Limit);
        }
        for simulator in simulators {
            if !valid_id(&simulator.udid) {
                return Err(RuntimeError::Invalid("Invalid simulator identifier".into()));
            }
            let supported = runtime.starts_with("com.apple.CoreSimulator.SimRuntime.iOS-")
                && simulator.available;
            let availability = if !supported {
                DeviceAvailability::Unsupported
            } else {
                match simulator.state.as_str() {
                    "Booted" => DeviceAvailability::Ready,
                    "Shutdown" => DeviceAvailability::Stopped,
                    _ => DeviceAvailability::Disconnected,
                }
            };
            devices.push(ToolDevice {
                descriptor: DeviceDescriptor {
                    id: DeviceId::new(simulator.udid)?,
                    name: simulator.name,
                    platform: if supported {
                        "iOS Simulator"
                    } else {
                        "Unsupported Apple runtime"
                    }
                    .into(),
                    kind: DeviceKind::Simulator,
                    state: if supported {
                        DeviceState::Discovered
                    } else {
                        DeviceState::Disconnected
                    },
                },
                availability,
                runtime: Some(runtime.clone()),
            });
            if devices.len() > 256 {
                return Err(RuntimeError::Limit);
            }
        }
    }
    validate_discovery(
        &devices
            .iter()
            .map(|d| d.descriptor.clone())
            .collect::<Vec<_>>(),
    )?;
    Ok(devices)
}
