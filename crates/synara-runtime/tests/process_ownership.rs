#![cfg(target_os = "linux")]
use std::{path::Path, time::Duration};
use synara_runtime::{ExecutionHost, LaunchSpec, LocalHost};
use tokio::io::AsyncReadExt;

fn running(pid: i32) -> bool {
    std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .ok()
        .is_some_and(|stat| {
            stat.rsplit_once(") ")
                .is_some_and(|(_, rest)| !rest.starts_with(['Z', 'X']))
        })
}
async fn pid_file(root: &Path, name: &str) -> i32 {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Ok(text) = std::fs::read_to_string(root.join(name))
                && let Ok(pid) = text.parse()
            {
                return pid;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("process did not publish its identity")
}
fn script(source: &str) -> LaunchSpec {
    let mut launch = LaunchSpec::new("python3");
    launch.args = vec!["-c".into(), source.into()];
    launch
}
const DESCENDANT: &str = r#"
import os, signal, time
from pathlib import Path
child = os.fork()
if child == 0:
    signal.signal(signal.SIGTERM, signal.SIG_IGN)
    Path('child').write_text(str(os.getpid()))
    while True: time.sleep(1)
while not Path('child').exists(): time.sleep(.001)
Path('parent').write_text(str(os.getpid()))
"#;

#[tokio::test]
async fn exit_before_supervision_releases_descendant_pipe_and_reports_original_exit() {
    for _ in 0..8 {
        let root = tempfile::tempdir().unwrap();
        let mut process = LocalHost
            .spawn(
                &script(&format!(
                    "{DESCENDANT}\nos.write(1,b'final-output'); os._exit(7)"
                )),
                root.path(),
            )
            .await
            .unwrap();
        let child = pid_file(root.path(), "child").await;
        let result = tokio::time::timeout(Duration::from_secs(5), process.handle.wait())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(result.code, Some(7));
        assert_eq!(result.error, None);
        assert!(
            !running(child),
            "wait completed before owned descendant stopped"
        );
        let mut bytes = Vec::new();
        tokio::time::timeout(
            Duration::from_secs(1),
            process.stdout.read_to_end(&mut bytes),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(bytes, b"final-output");
        process.handle.request_stop();
        process.handle.request_stop();
        assert_eq!(process.handle.shutdown().await.unwrap(), result);
    }
}

#[tokio::test]
async fn cancellation_cleans_ignoring_children_but_not_an_unrelated_group() {
    let root = tempfile::tempdir().unwrap();
    let unrelated = LocalHost
        .spawn(&script("import time; time.sleep(300)"), root.path())
        .await
        .unwrap();
    let process = LocalHost
        .spawn(
            &script(&format!("{DESCENDANT}\nwhile True: time.sleep(1)")),
            root.path(),
        )
        .await
        .unwrap();
    let child = pid_file(root.path(), "child").await;
    let parent = pid_file(root.path(), "parent").await;
    process.handle.request_stop();
    process.handle.request_stop();
    assert!(process.handle.shutdown().await.unwrap().error.is_none());
    assert!(!running(child));
    assert!(!running(parent));
    assert!(running(unrelated.handle.pid() as i32));
    assert!(unrelated.handle.shutdown().await.unwrap().error.is_none());
}

#[tokio::test]
async fn dropping_last_owner_requests_cleanup_without_blocking() {
    let root = tempfile::tempdir().unwrap();
    let process = LocalHost
        .spawn(
            &script(&format!("{DESCENDANT}\nwhile True: time.sleep(1)")),
            root.path(),
        )
        .await
        .unwrap();
    let child = pid_file(root.path(), "child").await;
    let parent = pid_file(root.path(), "parent").await;
    let start = std::time::Instant::now();
    drop(process);
    assert!(start.elapsed() < Duration::from_millis(100));
    tokio::time::timeout(Duration::from_secs(5), async {
        while running(child) || running(parent) {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
}

#[test]
fn aggregate_launch_arguments_and_environment_are_bounded() {
    let mut launch = LaunchSpec::new("program");
    launch.args = vec!["x".repeat(256 * 1024); 8];
    assert!(launch.validate().is_err());
    launch.args.clear();
    for n in 0..8 {
        launch
            .env
            .insert(format!("VALUE{n}"), "x".repeat(256 * 1024));
    }
    assert!(launch.validate().is_err());
    launch.env.clear();
    launch.args = vec!["--literal".into(), "雪 ' $HOME".into()];
    assert!(launch.validate().is_ok());
}
