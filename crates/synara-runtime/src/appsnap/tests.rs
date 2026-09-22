use super::*;
const INFO: &str =
    "xwininfo: Window id: 0x123 \"Test\"\n  Width: 80\n  Height: 60\n  Map State: IsViewable\n";
const PROPS: &str = "_NET_WM_PID(CARDINAL) = 17\nWM_CLASS(STRING) = \"fixture\", \"Fixture\"\n_NET_WM_NAME(UTF8_STRING) = \"Owned window\"\n";
#[test]
fn discovery_is_deterministic_complete_and_never_accepts_arbitrary_address() {
    assert_eq!(
        window_ids(
            b"_NET_CLIENT_LIST_STACKING(WINDOW): window id # 0x124, 0x123",
            b""
        )
        .unwrap(),
        [0x123, 0x124]
    );
    assert_eq!(
        window_ids(
            b"_NET_CLIENT_LIST_STACKING: not found.",
            b"xwininfo: Window id: 0x1 (the root window)\n    0x123 \"Test\": (...)"
        )
        .unwrap(),
        [0x123]
    );
    for token in ["root", "0x0", "-window", "0x123;command", "0x123456789"] {
        assert!(xid(token).is_err());
    }
    let list = (1..=65)
        .map(|v| format!("0x{v:x}"))
        .collect::<Vec<_>>()
        .join(", ");
    assert!(matches!(
        window_ids(
            format!("_NET_CLIENT_LIST_STACKING(WINDOW): window id # {list}").as_bytes(),
            b""
        ),
        Err(RuntimeError::Limit)
    ));
}
#[test]
fn only_visible_process_identified_non_root_bounded_windows_are_selectable() {
    let window = parse_window(0x123, 1, INFO.as_bytes(), PROPS.as_bytes()).unwrap();
    assert_eq!(window.pid, 17);
    for bad in [
        INFO.replace("IsViewable", "IsUnMapped"),
        INFO.replace("Width: 80", "Width: 9000"),
        INFO.replace("0x123", "0x456"),
        INFO.replace("\"Test\"", "(the root window)"),
    ] {
        assert!(parse_window(0x123, 1, bad.as_bytes(), PROPS.as_bytes()).is_err());
    }
    assert!(parse_window(0x123, 0x123, INFO.as_bytes(), PROPS.as_bytes()).is_err());
    assert!(
        parse_window(
            0x123,
            1,
            INFO.as_bytes(),
            PROPS.replace(" = 17", " = 0").as_bytes()
        )
        .is_err()
    );
    assert!(parse_window(0x123, 1, INFO.as_bytes(), b"WM_NAME(STRING) = \"No PID\"").is_err());
}
#[test]
fn captured_frame_dimensions_must_match_reviewed_window() {
    let w = parse_window(0x123, 1, INFO.as_bytes(), PROPS.as_bytes()).unwrap();
    let mut png = vec![0; 33];
    png[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
    png[12..16].copy_from_slice(b"IHDR");
    png[16..20].copy_from_slice(&80u32.to_be_bytes());
    png[20..24].copy_from_slice(&60u32.to_be_bytes());
    assert!(check_png(&png, &w).is_ok());
    png[19] = 81;
    assert!(matches!(check_png(&png, &w), Err(RuntimeError::Conflict)));
    assert!(check_png(b"not a picture", &w).is_err());
}
#[cfg(unix)]
fn helper(dir: &Path, name: &str, body: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join(name);
    std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    path
}
#[cfg(unix)]
#[tokio::test]
async fn stale_target_and_permission_failure_cannot_start_capture() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("captured");
    let tools = SnapTools {
        info: helper(dir.path(), "info", &format!("printf '%s' '{}'", INFO)),
        prop: helper(
            dir.path(),
            "prop",
            &format!("printf '%s' '{}'", PROPS.replace(" = 17", " = 18")),
        ),
        capture: helper(
            dir.path(),
            "capture",
            &format!("touch '{}'", marker.display()),
        ),
    };
    let w = parse_window(0x123, 1, INFO.as_bytes(), PROPS.as_bytes()).unwrap();
    let cancel = CancellationToken::new();
    assert!(matches!(
        tools.capture(&w, &cancel).await,
        Err(RuntimeError::Conflict)
    ));
    assert!(!marker.exists());
    let denied = SnapTools {
        info: helper(dir.path(), "denied", "exit 17"),
        ..tools.clone()
    };
    assert!(matches!(
        denied.capture(&w, &cancel).await,
        Err(RuntimeError::Denied(_))
    ));
    assert!(!marker.exists());
    cancel.cancel();
    assert!(matches!(
        tools.capture(&w, &cancel).await,
        Err(RuntimeError::Closed)
    ));
    assert!(!marker.exists());
}
#[cfg(unix)]
#[tokio::test]
async fn live_capture_stop_uses_existing_process_tree_owner() {
    let dir = tempfile::tempdir().unwrap();
    let tools = SnapTools {
        info: helper(dir.path(), "info", &format!("printf '%s' '{}'", INFO)),
        prop: helper(dir.path(), "prop", &format!("printf '%s' '{}'", PROPS)),
        capture: helper(dir.path(), "capture", "sleep 60"),
    };
    let w = parse_window(0x123, 1, INFO.as_bytes(), PROPS.as_bytes()).unwrap();
    let cancel = CancellationToken::new();
    let stop = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(150)).await;
        stop.cancel();
    });
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(5), tools.capture(&w, &cancel))
            .await
            .unwrap(),
        Err(RuntimeError::Closed)
    ));
}

#[test]
fn desktop_and_dock_are_never_application_targets() {
    for kind in ["DESKTOP", "DOCK"] {
        let props = format!("{PROPS}_NET_WM_WINDOW_TYPE(ATOM) = _NET_WM_WINDOW_TYPE_{kind}\n");
        assert!(matches!(
            parse_window(0x123, 1, INFO.as_bytes(), props.as_bytes()),
            Err(RuntimeError::Denied(_))
        ));
    }
}
