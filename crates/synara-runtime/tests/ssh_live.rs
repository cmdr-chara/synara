//! Real loopback SSH tests. Invoked explicitly by scripts/ssh_smoke.py.
#![cfg(unix)]

use std::{path::PathBuf, time::Duration};
use synara_runtime::{ExecutionHost, LaunchSpec, PinnedSshHost, ProcessExit, SshHost, SshTarget};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn fixture() -> (PathBuf, SshTarget) {
    let root = PathBuf::from(std::env::var_os("SYNARA_SSH_SMOKE_ROOT").expect("use ssh_smoke.py"));
    assert!(root.is_absolute());
    assert_eq!(
        std::fs::read_to_string(root.join("fixture.marker")).unwrap(),
        "Synara isolated SSH fixture v1\n"
    );
    let target = SshTarget {
        host: "127.0.0.1".into(),
        port: std::env::var("SYNARA_SSH_SMOKE_PORT")
            .unwrap()
            .parse()
            .unwrap(),
        user: Some(std::env::var("SYNARA_SSH_SMOKE_USER").unwrap()),
    };
    assert!(target.port > 1024);
    (root, target)
}

async fn execute(
    host: &PinnedSshHost,
    launch: LaunchSpec,
    cwd: &std::path::Path,
    input: &[u8],
) -> (Vec<u8>, Vec<u8>, ProcessExit) {
    tokio::time::timeout(Duration::from_secs(20), async {
        let process = host.spawn(&launch, cwd).await.unwrap();
        let mut stdin = process.stdin;
        stdin.write_all(input).await.unwrap();
        stdin.shutdown().await.unwrap();
        drop(stdin);
        let mut stdout = process.stdout.take(8192);
        let mut stderr = process.stderr.take(8192);
        let mut output = Vec::new();
        let mut diagnostic = Vec::new();
        let (out, err, exit) = tokio::join!(
            stdout.read_to_end(&mut output),
            stderr.read_to_end(&mut diagnostic),
            process.handle.wait()
        );
        out.unwrap();
        err.unwrap();
        (output, diagnostic, exit.unwrap())
    })
    .await
    .expect("SSH fixture operation timed out")
}

#[tokio::test]
#[ignore = "requires the isolated server from scripts/ssh_smoke.py"]
async fn ssh_preserves_remote_cwd_and_literal_arguments() {
    let (root, target) = fixture();
    let host = PinnedSshHost::new(target, root.join("known hosts"), root.join("identity")).unwrap();
    let cwd = root.join("project with ' quote");
    let literals = [
        "",
        "sp ace",
        "single'quote",
        "\"double\"",
        "$HOME",
        "$(touch injected)",
        "`touch injected`",
        "; touch injected;",
        "line\nbreak",
        "-flag",
        "雪",
    ];
    let mut launch = LaunchSpec::new("/bin/sh");
    launch.args = vec![
        "-c".into(),
        "printf '%s\\0' \"$PWD\" \"$@\"".into(),
        "fixture".into(),
    ];
    launch.args.extend(literals.iter().map(|s| (*s).into()));
    let (output, diagnostic, exit) = execute(&host, launch, &cwd, b"").await;
    assert!(exit.success(), "{}", String::from_utf8_lossy(&diagnostic));
    let mut expected = Vec::new();
    for value in std::iter::once(cwd.to_str().unwrap()).chain(literals) {
        expected.extend_from_slice(value.as_bytes());
        expected.push(0);
    }
    assert_eq!(output, expected);
    assert!(!cwd.join("injected").exists());
}

#[tokio::test]
#[ignore = "requires the isolated server from scripts/ssh_smoke.py"]
async fn ssh_stdio_remains_binary_transparent() {
    let (root, target) = fixture();
    let host = PinnedSshHost::new(target, root.join("known hosts"), root.join("identity")).unwrap();
    let bytes = b"first\0second\nthird\xff\r\n";
    let (output, diagnostic, exit) =
        execute(&host, LaunchSpec::new("/bin/cat"), &root, bytes).await;
    assert!(exit.success(), "{}", String::from_utf8_lossy(&diagnostic));
    assert_eq!(output, bytes);
}

#[tokio::test]
#[ignore = "requires the isolated server from scripts/ssh_smoke.py"]
async fn ssh_keeps_stderr_separate_and_reports_remote_exit() {
    let (root, target) = fixture();
    let host = PinnedSshHost::new(target, root.join("known hosts"), root.join("identity")).unwrap();
    let mut launch = LaunchSpec::new("/bin/sh");
    launch.args = vec![
        "-c".into(),
        "printf output; printf diagnostic >&2; exit 7".into(),
    ];
    let (output, diagnostic, exit) = execute(&host, launch, &root, b"").await;
    assert_eq!(output, b"output");
    assert!(String::from_utf8_lossy(&diagnostic).contains("diagnostic"));
    assert_eq!(exit.code, Some(7));
}

#[tokio::test]
#[ignore = "requires the isolated server from scripts/ssh_smoke.py"]
async fn ssh_rejects_unknown_and_changed_host_keys_without_execution() {
    let (root, target) = fixture();
    for trust in ["unknown hosts", "changed hosts"] {
        let path = root.join(trust);
        let before = std::fs::read(&path).unwrap();
        let host = PinnedSshHost::new(target.clone(), &path, root.join("identity")).unwrap();
        let mut launch = LaunchSpec::new("/bin/sh");
        launch.args = vec!["-c".into(), "touch forbidden-execution".into()];
        let (output, diagnostic, exit) = execute(&host, launch, &root, b"").await;
        assert_eq!(exit.code, Some(255));
        assert!(output.is_empty());
        let text = String::from_utf8_lossy(&diagnostic);
        assert!(text.contains("Host key verification failed"), "{text}");
        assert_eq!(std::fs::read(path).unwrap(), before);
        assert!(!root.join("forbidden-execution").exists());
    }
}

#[tokio::test]
#[ignore = "requires the isolated server from scripts/ssh_smoke.py"]
async fn ssh_rejects_an_unapproved_identity() {
    let (root, target) = fixture();
    let host = PinnedSshHost::new(
        target,
        root.join("known hosts"),
        root.join("wrong identity"),
    )
    .unwrap();
    let (output, diagnostic, exit) = execute(&host, LaunchSpec::new("/bin/true"), &root, b"").await;
    assert_eq!(exit.code, Some(255));
    assert!(output.is_empty());
    assert!(String::from_utf8_lossy(&diagnostic).contains("Permission denied"));
}

#[test]
#[ignore = "requires OpenSSH from scripts/ssh_smoke.py"]
fn ssh_effective_configuration_overrides_ambient_forwarding() {
    let (root, target) = fixture();
    let host = SshHost { target };
    let spec = host.command(&LaunchSpec::new("agent"), &root).unwrap();
    let result = std::process::Command::new(&spec.command)
        .arg("-G")
        .arg("-F")
        .arg(root.join("ambient config"))
        .args(spec.args)
        .output()
        .unwrap();
    assert!(result.status.success());
    let config = String::from_utf8(result.stdout).unwrap();
    for expected in [
        "forwardagent no",
        "forwardx11 no",
        "clearallforwardings yes",
        "permitlocalcommand no",
        "controlmaster false",
        "requesttty false",
        "forkafterauthentication no",
    ] {
        assert!(
            config.lines().any(|line| line == expected),
            "{expected}\n{config}"
        );
    }
    assert!(!config.lines().any(|line| line.starts_with("localforward ")));
    assert!(
        !config
            .lines()
            .any(|line| line.starts_with("remoteforward "))
    );
    assert!(!config.lines().any(|line| line.starts_with("controlpath ")));
}
