#![cfg(windows)]
//! Real ConPTY tests using only this disposable test executable, never a user shell.
use std::{io::Write, time::Duration};
use synara_runtime::{LaunchSpec, NativeTerminal, RuntimeError};

#[test]
#[ignore = "isolated ConPTY fixture entry, invoked only with an explicit test mode"]
fn conpty_fixture_entry() {
    let Ok(mode) = std::env::var("SYNARA_CONPTY_FIXTURE") else {
        return;
    };
    let mut stdout = std::io::stdout().lock();
    match mode.as_str() {
        "output" => {
            write!(stdout, "\x1b[31mterminal-ok 日本語 æ\x1b[0m\r\n").unwrap();
            stdout.flush().unwrap();
            std::thread::sleep(Duration::from_millis(100));
        }
        "blocked" => {
            writeln!(stdout, "blocked-ready").unwrap();
            stdout.flush().unwrap();
            std::thread::sleep(Duration::from_secs(20));
        }
        "flood" => loop {
            stdout.write_all(&[b'x'; 8192]).unwrap();
            stdout.flush().unwrap();
        },
        _ => panic!("unrecognized isolated fixture mode"),
    }
}

fn launch(mode: &str) -> LaunchSpec {
    let mut launch = LaunchSpec::new(std::env::current_exe().unwrap().to_string_lossy().into_owned());
    launch.args = vec![
        "--ignored".into(),
        "--exact".into(),
        "conpty_fixture_entry".into(),
        "--nocapture".into(),
    ];
    launch
        .env
        .insert("SYNARA_CONPTY_FIXTURE".into(), mode.into());
    launch
}

#[tokio::test]
async fn conpty_output_is_retained_and_renderable_after_shutdown() {
    let root = tempfile::tempdir().unwrap();
    let terminal = NativeTerminal::spawn(&launch("output"), root.path(), 24, 100).unwrap();
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(5), terminal.wait())
            .await
            .unwrap()
            .unwrap(),
        0
    );
    let snapshot = terminal.snapshot().unwrap();
    assert!(snapshot.text.contains("terminal-ok 日本語 æ"));
    assert!(snapshot.error.is_none(), "{:?}", snapshot.error);
    let grid = terminal.render_snapshot().unwrap().grid;
    assert_eq!(grid.rows, 24);
    assert_eq!(grid.columns, 100);
    assert_eq!(grid.cells.len(), 2400);
    assert!(grid.revision > 0);
    assert!(matches!(terminal.text("late input"), Err(RuntimeError::Closed)));
    terminal.scrollback(100).unwrap();
    terminal.kill().unwrap();
    terminal.kill().unwrap();
}

#[tokio::test]
async fn conpty_input_backpressure_does_not_block_cancellation_or_rendering() {
    let root = tempfile::tempdir().unwrap();
    let terminal = NativeTerminal::spawn(&launch("blocked"), root.path(), 24, 80).unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        while !terminal.snapshot().unwrap().text.contains("blocked-ready") {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let bytes = vec![b'x'; 64 * 1024];
    let start = std::time::Instant::now();
    let mut refused = false;
    for _ in 0..256 {
        match terminal.input(&bytes) {
            Ok(()) => {}
            Err(RuntimeError::Limit) => {
                refused = true;
                break;
            }
            Err(error) => panic!("unexpected input failure: {error}"),
        }
    }
    assert!(refused, "blocked input did not enforce its byte budget");
    assert!(start.elapsed() < Duration::from_secs(1));
    terminal.render_snapshot().unwrap();
    terminal.shutdown().await.unwrap();
    assert!(terminal.snapshot().unwrap().error.is_none());
}

#[tokio::test]
async fn conpty_flood_is_bounded_and_resize_remains_responsive() {
    let root = tempfile::tempdir().unwrap();
    let terminal = NativeTerminal::spawn(&launch("flood"), root.path(), 24, 80).unwrap();
    tokio::time::timeout(Duration::from_secs(10), async {
        while !terminal.snapshot().unwrap().truncated {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    terminal.resize(30, 120).unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        while terminal.render_snapshot().unwrap().grid.columns != 120 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(terminal.snapshot().unwrap().raw_tail.len() <= 1024 * 1024);
    terminal.shutdown().await.unwrap();
    assert!(terminal.snapshot().unwrap().error.is_none());
}

#[test]
fn conpty_partial_start_and_invalid_dimensions_fail_without_a_child() {
    let root = tempfile::tempdir().unwrap();
    let missing = LaunchSpec::new(root.path().join("does-not-exist.exe").to_string_lossy());
    assert!(NativeTerminal::spawn(&missing, root.path(), 24, 80).is_err());
    assert!(NativeTerminal::spawn(&launch("blocked"), root.path(), 0, 80).is_err());
    assert!(NativeTerminal::spawn(&launch("blocked"), root.path(), 200, 500).is_err());
}
