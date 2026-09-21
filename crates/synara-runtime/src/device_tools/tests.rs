#[cfg(test)]
mod tests {
    use super::super::*;
    #[test]
    fn distinguishes_real_simulator_unknown_unauthorized_and_stale() {
        let devices = parse_adb(b"List of devices attached\nemulator-5554 device model:Pixel\nUSB123 device usb:1-2 model:My_Phone\n192.0.2.1:5555 device\nABC unauthorized\nDEF offline\n").unwrap();
        assert_eq!(devices[0].descriptor.kind, DeviceKind::Simulator);
        assert_eq!(devices[1].descriptor.kind, DeviceKind::Physical);
        assert_eq!(devices[1].descriptor.name, "My Phone");
        assert_eq!(devices[2].descriptor.kind, DeviceKind::Unknown);
        assert_eq!(devices[3].availability, DeviceAvailability::Unauthorized);
        assert_eq!(devices[4].availability, DeviceAvailability::Disconnected);
        let stale = reconcile_devices(&devices, vec![]);
        assert_eq!(stale.len(), 5);
        assert!(stale.iter().all(|d| d.availability == DeviceAvailability::Disconnected));
    }
    #[test]
    fn discovery_fails_closed_for_duplicate_missing_header_and_shell_serials() {
        for bytes in [b"error: failed".as_slice(), b"List of devices attached\nabc device\nabc device\n", b"List of devices attached\na;rm device\n", b"List of devices attached\n--all device\n"] {
            assert!(parse_adb(bytes).is_err());
        }
        assert!(parse_adb(b"List of devices attached\n").unwrap().is_empty());
    }
    #[test]
    fn input_is_numeric_allowlisted_and_inside_actual_frame() {
        assert_eq!(android_input_args(&DeviceInput::Tap { x: 20, y: 30 }, 100, 200).unwrap(), ["tap", "20", "30"]);
        assert!(android_input_args(&DeviceInput::Tap { x: 100, y: 0 }, 100, 200).is_err());
        assert!(android_input_args(&DeviceInput::Text { text: "hello".into() }, 100, 200).is_err());
        for key in [";reboot", "POWER", "3", "home;ls"] {
            assert!(android_input_args(&DeviceInput::Key { key: key.into() }, 100, 200).is_err());
        }
        assert_eq!(android_input_args(&DeviceInput::Key { key: "back".into() }, 100, 200).unwrap(), ["keyevent", "4"]);
    }
    #[test]
    fn apple_metadata_does_not_claim_other_platforms() {
        let devices = apple::parse(br#"{"devices":{"com.apple.CoreSimulator.SimRuntime.iOS-18-0":[{"udid":"00000000-0000-0000-0000-000000000001","name":"iPhone","state":"Shutdown","isAvailable":true}],"com.apple.CoreSimulator.SimRuntime.tvOS-18-0":[{"udid":"00000000-0000-0000-0000-000000000002","name":"TV","state":"Booted","isAvailable":true}]}}"#).unwrap();
        assert_eq!(devices[0].availability, DeviceAvailability::Stopped);
        assert_eq!(devices[1].availability, DeviceAvailability::Unsupported);
        assert!(!apple::valid_id("booted"));
        assert!(!apple::valid_id("--help"));
    }
    #[test]
    fn malformed_deserialized_identity_is_rejected() {
        let device: DeviceDescriptor = serde_json::from_value(serde_json::json!({"id":"\n", "name":"Phone", "platform":"Android", "kind":"physical", "state":"discovered"})).unwrap();
        assert!(device.validate().is_err());
    }
    #[tokio::test]
    async fn cancelled_command_never_spawns() {
        let cancel = CancellationToken::new();
        cancel.cancel();
        assert!(matches!(command::run(Path::new("/not-installed"), vec![], 16, &cancel).await, Err(RuntimeError::Closed)));
    }
}
