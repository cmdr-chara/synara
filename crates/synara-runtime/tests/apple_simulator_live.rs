#![cfg(target_os = "macos")]

use std::time::Duration;

use synara_runtime::{
    DeviceAvailability, DeviceBackend, DeviceCancellation, DeviceTools, ToolDevice,
};

async fn ready_device(
    tools: &DeviceTools,
    id: &str,
    cancel: &DeviceCancellation,
) -> Result<ToolDevice, String> {
    for _ in 0..60 {
        let devices = tools
            .discover(cancel)
            .await
            .map_err(|error| error.to_string())?;
        if let Some(device) = devices.into_iter().find(|device| {
            device.descriptor.id.as_str() == id && device.availability == DeviceAvailability::Ready
        }) {
            return Ok(device);
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    Err("iOS Simulator did not become ready within 60 seconds".into())
}

#[tokio::test]
async fn live_apple_simulator_boot_capture_and_open_url() {
    let tools = DeviceTools::new(DeviceBackend::AppleSimulator, None)
        .expect("Apple Simulator helper must be available on the macOS acceptance runner");
    let cancel = DeviceCancellation::new();
    let devices = tools
        .discover(&cancel)
        .await
        .expect("simctl discovery must succeed");
    let initial = devices
        .into_iter()
        .find(|device| {
            device.descriptor.platform == "iOS Simulator"
                && matches!(
                    device.availability,
                    DeviceAvailability::Ready | DeviceAvailability::Stopped
                )
        })
        .expect("macOS acceptance requires at least one available iOS Simulator runtime");

    let id = initial.descriptor.id.as_str().to_owned();
    let booted_by_test = initial.availability == DeviceAvailability::Stopped;
    if booted_by_test {
        tools
            .set_running(&initial, true, &cancel)
            .await
            .expect("Synara must boot the selected iOS Simulator");
    }

    let result = async {
        let ready = ready_device(&tools, &id, &cancel).await?;

        let mut capture_error = "unknown error".to_owned();
        let mut png = None;
        for _ in 0..30 {
            match tools.capture(&ready, &cancel).await {
                Ok(bytes) => {
                    png = Some(bytes);
                    break;
                }
                Err(error) => {
                    capture_error = error.to_string();
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
        let png = png.ok_or_else(|| {
            format!("Simulator capture did not become available: {capture_error}")
        })?;
        if !png.starts_with(b"\x89PNG\r\n\x1a\n") {
            return Err("Synara Simulator capture did not return PNG data".into());
        }

        tools
            .open_url(&ready, "https://example.com/", &cancel)
            .await
            .map_err(|error| format!("Synara Simulator open-URL failed: {error}"))?;
        Ok::<(), String>(())
    }
    .await;

    if booted_by_test && let Ok(current) = ready_device(&tools, &id, &cancel).await {
        tools
            .set_running(&current, false, &cancel)
            .await
            .expect("acceptance must restore a Simulator it booted");
    }

    result.expect("live Apple Simulator acceptance failed");
}
