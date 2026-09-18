use super::*;
use std::{path::Path, sync::Mutex};
use synara_runtime::{ProcessHandle, SpawnedProcess};
use tokio::sync::watch;

struct FixtureHost {
    script: &'static str,
    handles: Mutex<Vec<ProcessHandle>>,
    spawned: watch::Sender<bool>,
}
#[async_trait::async_trait]
impl ExecutionHost for FixtureHost {
    fn label(&self) -> String {
        "isolated Git lifecycle fixture".into()
    }
    fn is_local(&self) -> bool {
        true
    }
    async fn spawn(&self, _: &LaunchSpec, cwd: &Path) -> Result<SpawnedProcess, RuntimeError> {
        // Re-execute only the ignored child fixture in this native test binary.
        // No shell, platform-specific command, external service, or user data.
        let executable = std::env::current_exe().unwrap();
        let mut launch = LaunchSpec::new(executable.to_str().unwrap());
        launch.args = vec![
            "--exact".into(),
            "tools::git_lifecycle_tests::owned_child_fixture".into(),
            "--ignored".into(),
            "--nocapture".into(),
        ];
        launch
            .env
            .insert("SYNARA_GIT_TEST_CHILD".into(), self.script.into());
        let process = LocalHost.spawn(&launch, cwd).await?;
        self.handles.lock().unwrap().push(process.handle.clone());
        self.spawned.send_replace(true);
        Ok(process)
    }
}
fn fixture(
    script: &'static str,
) -> (
    tempfile::TempDir,
    GitService,
    Arc<FixtureHost>,
    watch::Receiver<bool>,
) {
    let root = tempfile::tempdir().unwrap();
    let (spawned, receiver) = watch::channel(false);
    let host = Arc::new(FixtureHost {
        script,
        handles: Mutex::new(vec![]),
        spawned,
    });
    let service = GitService::with_host(root.path().into(), host.clone());
    (root, service, host, receiver)
}

#[tokio::test]
async fn aborting_git_service_stops_process_despite_other_owners() {
    let (_root, service, host, mut spawned) = fixture("sleep");
    let task = tokio::spawn(async move { service.status().await });
    tokio::time::timeout(Duration::from_secs(5), spawned.wait_for(|ready| *ready))
        .await
        .unwrap()
        .unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    let handle = host.handles.lock().unwrap()[0].clone();
    let observed = tokio::time::timeout(Duration::from_secs(3), handle.wait()).await;
    // Always clean the isolated fixture even when the regression assertion fails.
    let _ = handle.shutdown().await;
    assert!(
        observed.is_ok(),
        "dropping GitService future left its process alive"
    );
}

#[tokio::test]
async fn failed_git_service_diagnostics_do_not_export_agent_controlled_stderr() {
    let (_root, service, _, _) = fixture("canary_failure");
    let error = service.diff(false, None).await.unwrap_err();
    let display = format!("{error} {error:?}");
    assert!(
        !display.contains("SECRET_CANARY"),
        "raw credentials escaped into diagnostics"
    );
    assert!(display.contains("authentication"));
    assert!(!display.contains("example.invalid"));
}

#[tokio::test]
async fn output_limits_and_deadline_stop_the_process_before_returning() {
    for (script, stdout_limit, timeout, is_timeout) in [
        ("stdout_flood", 3, Duration::from_secs(5), false),
        ("stderr_flood", 8192, Duration::from_secs(5), false),
        ("sleep", 8192, Duration::from_secs(1), true),
    ] {
        let (_root, service, host, _) = fixture(script);
        let error = service
            .run_until(
                vec!["status".into()],
                stdout_limit,
                Instant::now() + timeout,
            )
            .await
            .unwrap_err();
        assert!(!format!("{error:?} {error}").contains("SECRET_CANARY"));
        if is_timeout {
            assert!(matches!(
                error,
                WorkspaceError::Runtime(RuntimeError::Timeout)
            ));
        } else {
            assert!(matches!(
                error,
                WorkspaceError::Runtime(RuntimeError::Limit)
            ));
        }
        let handle = host.handles.lock().unwrap()[0].clone();
        assert!(
            handle.exit().is_some(),
            "service returned without bounded cleanup"
        );
    }
}

struct FailingHost {
    missing: bool,
}
#[async_trait::async_trait]
impl ExecutionHost for FailingHost {
    fn label(&self) -> String {
        "SECRET_CANARY".into()
    }
    fn is_local(&self) -> bool {
        false
    }
    async fn spawn(&self, _: &LaunchSpec, _: &Path) -> Result<SpawnedProcess, RuntimeError> {
        Err(if self.missing {
            RuntimeError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "SECRET_CANARY",
            ))
        } else {
            RuntimeError::Invalid("SECRET_CANARY".into())
        })
    }
}
#[tokio::test]
async fn failed_host_has_safe_diagnostics_and_never_uses_the_local_repository() {
    let root = tempfile::tempdir().unwrap();
    let status = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(root.path())
        .status()
        .unwrap();
    assert!(status.success());
    std::fs::write(root.path().join("private.txt"), "retain me").unwrap();
    for missing in [false, true] {
        let service = GitService::with_host(root.path().into(), Arc::new(FailingHost { missing }));
        let error = service.status().await.unwrap_err();
        let diagnostic = format!("{error} {error:?}");
        assert!(!diagnostic.contains("SECRET_CANARY"));
        assert_eq!(diagnostic.contains("not available"), missing);
    }
    assert_eq!(
        std::fs::read(root.path().join("private.txt")).unwrap(),
        b"retain me"
    );
}

struct PendingHost;
#[async_trait::async_trait]
impl ExecutionHost for PendingHost {
    fn label(&self) -> String {
        "pending fixture".into()
    }
    fn is_local(&self) -> bool {
        false
    }
    async fn spawn(&self, _: &LaunchSpec, _: &Path) -> Result<SpawnedProcess, RuntimeError> {
        std::future::pending().await
    }
}
#[tokio::test]
async fn deadline_includes_host_startup_not_only_process_output() {
    let root = tempfile::tempdir().unwrap();
    let service = GitService::with_host(root.path().into(), Arc::new(PendingHost));
    let result = tokio::time::timeout(
        Duration::from_secs(3),
        service.run_until(
            vec!["status".into()],
            8192,
            Instant::now() + Duration::from_millis(100),
        ),
    )
    .await
    .unwrap();
    assert!(matches!(
        result,
        Err(WorkspaceError::Runtime(RuntimeError::Timeout))
    ));
}

#[tokio::test]
async fn successful_output_is_retained_and_stdin_is_closed() {
    let (_root, service, host, _) = fixture("stdin_eof");
    let result = service
        .run_until(
            vec!["status".into()],
            8192,
            Instant::now() + Duration::from_secs(3),
        )
        .await
        .unwrap();
    assert!(String::from_utf8(result).unwrap().contains("final-output"));
    assert!(host.handles.lock().unwrap()[0].exit().unwrap().success());
}

#[test]
#[ignore = "child process fixture invoked only by GitService lifecycle tests"]
fn owned_child_fixture() {
    use std::io::{Read, Write};
    let mode = std::env::var("SYNARA_GIT_TEST_CHILD").expect("isolated child mode required");
    match mode.as_str() {
        "sleep" => std::thread::sleep(Duration::from_secs(60)),
        "canary_failure" => {
            std::io::stdout()
                .write_all(b"private stdout SECRET_CANARY")
                .unwrap();
            std::io::stderr()
                .write_all(
                    b"fatal: authentication failed for https://user:SECRET_CANARY@example.invalid/",
                )
                .unwrap();
            std::process::exit(1);
        }
        "stdout_flood" => loop {
            if std::io::stdout().write_all(&[b'x'; 4096]).is_err() {
                std::process::exit(1);
            }
        },
        "stderr_flood" => loop {
            if std::io::stderr().write_all(&[b'y'; 4096]).is_err() {
                std::process::exit(1);
            }
        },
        "stdin_eof" => {
            let mut input = [0u8; 1];
            assert_eq!(std::io::stdin().read(&mut input).unwrap(), 0);
            std::io::stdout().write_all(b"final-output").unwrap();
        }
        _ => panic!("unknown isolated child mode"),
    }
}
