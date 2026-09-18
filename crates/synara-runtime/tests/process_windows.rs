#![cfg(windows)]
use std::{path::Path, process::Stdio, time::Duration};
use synara_runtime::{ExecutionHost, LaunchSpec, LocalHost};

#[test]
#[ignore = "isolated Windows process-tree fixture invoked only by the ownership test"]
fn windows_process_fixture() {
    let mode = std::env::var("SYNARA_WINDOWS_JOB_FIXTURE").expect("fixture mode required");
    let directory = std::path::PathBuf::from(
        std::env::var_os("SYNARA_WINDOWS_JOB_DIR").expect("fixture directory required"),
    );
    match mode.as_str() {
        "parent" => {
            let executable = std::env::current_exe().unwrap();
            let child = std::process::Command::new(executable)
                .args([
                    "--ignored",
                    "--exact",
                    "windows_process_fixture",
                    "--nocapture",
                ])
                .env("SYNARA_WINDOWS_JOB_FIXTURE", "leaf")
                .env("SYNARA_WINDOWS_JOB_DIR", &directory)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap();
            std::fs::write(directory.join("child.pid"), child.id().to_string()).unwrap();
            std::fs::write(directory.join("parent.pid"), std::process::id().to_string()).unwrap();
        }
        "leaf" => std::thread::sleep(Duration::from_secs(60)),
        _ => panic!("unknown fixture mode"),
    }
}

fn launch(root: &Path) -> LaunchSpec {
    let mut launch = LaunchSpec::new(std::env::current_exe().unwrap());
    launch.args = vec![
        "--ignored".into(),
        "--exact".into(),
        "windows_process_fixture".into(),
        "--nocapture".into(),
    ];
    launch
        .env
        .insert("SYNARA_WINDOWS_JOB_FIXTURE".into(), "parent".into());
    launch.env.insert(
        "SYNARA_WINDOWS_JOB_DIR".into(),
        root.to_string_lossy().into_owned(),
    );
    launch
}

async fn child_pid(root: &Path) -> u32 {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Ok(text) = std::fs::read_to_string(root.join("child.pid"))
                && let Ok(pid) = text.parse()
            {
                return pid;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("fixture did not publish its child PID")
}

fn running(pid: u32) -> bool {
    let output = std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .expect("tasklist must be available on native Windows");
    String::from_utf8_lossy(&output.stdout)
        .split(',')
        .nth(1)
        .and_then(|field| field.trim().trim_matches('"').parse::<u32>().ok())
        == Some(pid)
}

#[tokio::test]
async fn normal_parent_exit_releases_job_and_kills_owned_descendant() {
    let root = tempfile::tempdir().unwrap();
    let process = LocalHost.spawn(&launch(root.path()), root.path()).await.unwrap();
    let child = child_pid(root.path()).await;
    assert!(running(child), "fixture descendant never started");

    let exit = tokio::time::timeout(Duration::from_secs(5), process.handle.wait())
        .await
        .unwrap()
        .unwrap();
    assert!(exit.success(), "{exit:?}");

    tokio::time::timeout(Duration::from_secs(5), async {
        while running(child) {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("Job Object close did not terminate the descendant");
}
