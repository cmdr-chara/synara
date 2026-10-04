use super::*;
fn target() -> ToolDevice {
    ToolDevice {
        descriptor: DeviceDescriptor {
            id: DeviceId::new("00000000-0000-0000-0000-000000000001").unwrap(),
            name: "iPhone".into(),
            platform: "iOS Simulator".into(),
            kind: DeviceKind::Simulator,
            state: DeviceState::Discovered,
        },
        availability: DeviceAvailability::Ready,
        runtime: None,
    }
}
#[test]
fn install_and_terminate_keep_exact_target_and_literal_arguments() {
    assert_eq!(
        apple::install_args(
            target().descriptor.id.as_str().into(),
            "/tmp/日本語 $HOME.app".into()
        ),
        [
            "simctl",
            "install",
            "00000000-0000-0000-0000-000000000001",
            "/tmp/日本語 $HOME.app"
        ]
    );
    assert_eq!(
        apple::terminate_args(
            target().descriptor.id.as_str().into(),
            "com.example.App".into()
        ),
        [
            "simctl",
            "terminate",
            "00000000-0000-0000-0000-000000000001",
            "com.example.App"
        ]
    );
    let tools = DeviceTools {
        backend: DeviceBackend::AppleSimulator,
        executable: PathBuf::from("/not-installed"),
        apple_helper: None,
    };
    for value in [
        "relative.app",
        "/tmp/archive.ipa",
        "/tmp/../App.app",
        "/tmp/Bad\n.app",
    ] {
        assert!(
            tools
                .validate_install_app(&target(), Path::new(value))
                .is_err()
        );
    }
    let mut stopped = target();
    stopped.availability = DeviceAvailability::Stopped;
    assert!(
        tools
            .validate_install_app(&stopped, Path::new("/tmp/App.app"))
            .is_err()
    );
    let android = DeviceTools {
        backend: DeviceBackend::Android,
        executable: PathBuf::from("/not-installed"),
        apple_helper: None,
    };
    assert!(
        android
            .validate_install_app(&target(), Path::new("/tmp/App.app"))
            .is_err()
    );
}
#[cfg(unix)]
#[tokio::test]
async fn install_bundle_uses_owned_helper_and_rejects_symlinks_or_missing_plist() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let dir = tempfile::tempdir().unwrap();
    let helper = dir.path().join("xcrun-fixture");
    std::fs::write(&helper, "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$0.args\"\n").unwrap();
    std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o700)).unwrap();
    let tools = DeviceTools {
        backend: DeviceBackend::AppleSimulator,
        executable: helper.clone(),
        apple_helper: None,
    };
    let bundle = dir.path().join("日本語 $HOME.app");
    std::fs::create_dir(&bundle).unwrap();
    let cancel = CancellationToken::new();
    assert!(
        tools
            .install_app(&target(), &bundle, &cancel)
            .await
            .is_err()
    );
    assert!(!helper.with_extension("args").exists());
    std::fs::write(bundle.join("Info.plist"), "<plist/>").unwrap();
    let link = dir.path().join("Linked.app");
    symlink(&bundle, &link).unwrap();
    assert!(tools.install_app(&target(), &link, &cancel).await.is_err());
    tools
        .install_app(&target(), &bundle, &cancel)
        .await
        .unwrap();
    let args = std::fs::read_to_string(helper.with_extension("args")).unwrap();
    let canonical_bundle = bundle.canonicalize().unwrap();
    assert_eq!(
        args.lines().collect::<Vec<_>>(),
        [
            "simctl",
            "install",
            target().descriptor.id.as_str(),
            canonical_bundle.to_str().unwrap()
        ]
    );
    tools
        .terminate_app(&target(), "com.example.App", &cancel)
        .await
        .unwrap();
    assert!(
        std::fs::read_to_string(helper.with_extension("args"))
            .unwrap()
            .contains("\nterminate\n")
    );
    std::fs::remove_file(helper.with_extension("args")).unwrap();
    cancel.cancel();
    assert!(
        tools
            .install_app(&target(), &bundle, &cancel)
            .await
            .is_err()
    );
    assert!(
        !helper.with_extension("args").exists(),
        "cancelled install must never spawn"
    );
}
