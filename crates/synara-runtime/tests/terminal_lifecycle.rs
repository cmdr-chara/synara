#![cfg(target_os = "linux")]
//! Real PTY regression tests. Every launched process is confined to a temporary
//! test directory and its own controlling-terminal session.
use std::{
    path::Path,
    time::{Duration, Instant},
};
use synara_runtime::{
    LaunchSpec, NativeTerminal, PasteDecision, PreparedPaste, RuntimeError, TerminalKey,
    TerminalModifiers,
};

static SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn python(root: &Path, source: &str) -> NativeTerminal {
    let mut launch = LaunchSpec::new("python3");
    launch.args = vec!["-c".into(), source.into()];
    NativeTerminal::spawn(&launch, root, 24, 80).unwrap()
}
async fn wait_file(root: &Path, name: &str) -> String {
    let path = root.join(name);
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Ok(text) = std::fs::read_to_string(&path)
            && !text.is_empty()
        {
            return text;
        }
        assert!(Instant::now() < deadline, "fixture did not produce {name}");
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}
async fn finished(terminal: &NativeTerminal) -> u32 {
    let code = tokio::time::timeout(Duration::from_secs(5), terminal.wait())
        .await
        .expect("PTY worker did not finish")
        .unwrap();
    assert!(
        terminal.snapshot().unwrap().error.is_none(),
        "{:?}",
        terminal.snapshot().unwrap()
    );
    code
}
fn running(pid: i32) -> bool {
    std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .ok()
        .is_some_and(|stat| {
            stat.rsplit_once(") ")
                .is_some_and(|(_, rest)| !rest.starts_with(['Z', 'X']))
        })
}

#[tokio::test]
async fn normal_exit_retains_final_output_and_stop_is_idempotent() {
    let _serial = SERIAL.lock().await;
    let root = tempfile::tempdir().unwrap();
    for _ in 0..12 {
        let terminal = python(
            root.path(),
            "import os; os.write(1, 'retained-雪'.encode()); raise SystemExit(7)",
        );
        assert_eq!(finished(&terminal).await, 7);
        assert_eq!(terminal.snapshot().unwrap().text, "retained-雪");
        assert!(
            terminal
                .render_snapshot()
                .unwrap()
                .grid
                .plain_text()
                .contains("retained-雪")
        );
        terminal.kill().unwrap();
        terminal.kill().unwrap();
        assert_eq!(terminal.shutdown().await.unwrap(), 7);
        assert!(matches!(terminal.input(b"late"), Err(RuntimeError::Closed)));
    }
}

#[tokio::test]
async fn typing_and_paste_use_modes_from_output_not_stale_gui_state() {
    let _serial = SERIAL.lock().await;
    let root = tempfile::tempdir().unwrap();
    let paste = PreparedPaste::new("safe\x1b[201~\ntext").unwrap();
    let expected_paste = paste.encode(PasteDecision::Approved, true).unwrap();
    let expected = [
        b"\x1bOA".as_slice(),
        "雪".as_bytes(),
        expected_paste.as_slice(),
    ]
    .concat();
    let terminal = python(
        root.path(),
        &format!(
            r#"
import os, tty
from pathlib import Path
tty.setraw(0)
os.write(1, b'\x1b[?1h\x1b[?2004h')
Path('ready').write_text('ready')
data = b''
while len(data) < {}:
    data += os.read(0, {} - len(data))
Path('received').write_text(data.hex())
"#,
            expected.len(),
            expected.len()
        ),
    );
    wait_file(root.path(), "ready").await;
    terminal
        .key(TerminalKey::Up, TerminalModifiers::default())
        .unwrap();
    terminal.text("雪").unwrap();
    assert!(
        terminal
            .paste(paste.clone(), PasteDecision::Unreviewed)
            .is_err()
    );
    terminal.paste(paste, PasteDecision::Approved).unwrap();
    assert_eq!(finished(&terminal).await, 0);
    assert_eq!(
        std::fs::read_to_string(root.path().join("received")).unwrap(),
        hex::encode(expected)
    );
}

#[tokio::test]
async fn resize_reaches_the_kernel_pty_before_subsequent_input() {
    let _serial = SERIAL.lock().await;
    let root = tempfile::tempdir().unwrap();
    let terminal = python(
        root.path(),
        r#"
import os, tty, fcntl, termios, struct
from pathlib import Path
tty.setraw(0)
Path('ready').write_text('ready')
os.read(0, 1)
rows, cols, _, _ = struct.unpack('HHHH', fcntl.ioctl(0, termios.TIOCGWINSZ, bytes(8)))
Path('size').write_text(f'{rows}x{cols}')
"#,
    );
    wait_file(root.path(), "ready").await;
    terminal.resize(30, 90).unwrap();
    terminal.resize(31, 99).unwrap();
    terminal.input(b"R").unwrap();
    assert_eq!(finished(&terminal).await, 0);
    assert_eq!(
        std::fs::read_to_string(root.path().join("size")).unwrap(),
        "31x99"
    );
    let grid = terminal.render_snapshot().unwrap().grid;
    assert_eq!((grid.rows, grid.columns), (31, 99));
}

const JOB: &str = r#"
import os, signal, time
from pathlib import Path
child = os.fork()
if child == 0:
    os.setpgrp()
    signal.signal(signal.SIGHUP, signal.SIG_IGN)
    signal.signal(signal.SIGTERM, signal.SIG_IGN)
    Path('job').write_text(str(os.getpid()))
    while True: time.sleep(1)
while not Path('job').exists(): time.sleep(.001)
os.tcsetpgrp(0, child)
Path('ready').write_text(str(os.getpid()))
"#;

#[tokio::test]
async fn stop_removes_foreground_job_in_a_different_process_group() {
    let _serial = SERIAL.lock().await;
    let root = tempfile::tempdir().unwrap();
    let terminal = python(root.path(), &format!("{JOB}\nwhile True: time.sleep(1)\n"));
    let leader: i32 = wait_file(root.path(), "ready").await.parse().unwrap();
    let job: i32 = wait_file(root.path(), "job").await.parse().unwrap();
    assert_ne!(leader, job);
    assert_eq!(
        nix::unistd::getpgid(Some(nix::unistd::Pid::from_raw(job)))
            .unwrap()
            .as_raw(),
        job
    );
    assert!(running(job));
    terminal.kill().unwrap();
    finished(&terminal).await;
    assert!(!running(job), "foreground process {job} remained alive");
    assert!(!running(leader));
}

#[tokio::test]
async fn exit_before_stop_cleans_foreground_job_while_pid_is_reserved() {
    let _serial = SERIAL.lock().await;
    let root = tempfile::tempdir().unwrap();
    let terminal = python(root.path(), &format!("{JOB}\nos._exit(9)\n"));
    let job: i32 = wait_file(root.path(), "job").await.parse().unwrap();
    assert_eq!(finished(&terminal).await, 9);
    assert!(!running(job));
    terminal.kill().unwrap();
    assert_eq!(terminal.wait().await.unwrap(), 9);
}

#[tokio::test]
async fn interrupt_is_direct_pty_control_input() {
    let _serial = SERIAL.lock().await;
    let root = tempfile::tempdir().unwrap();
    let terminal = python(
        root.path(),
        r#"
import os, signal, time
from pathlib import Path
def interrupted(signum, frame):
    os.write(1, b'interrupted')
    raise SystemExit(8)
signal.signal(signal.SIGINT, interrupted)
Path('ready').write_text('ready')
while True: time.sleep(1)
"#,
    );
    wait_file(root.path(), "ready").await;
    terminal
        .key(
            TerminalKey::Character('c'),
            TerminalModifiers {
                control: true,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(finished(&terminal).await, 8);
    assert!(terminal.snapshot().unwrap().text.contains("interrupted"));
}

#[tokio::test]
async fn full_input_queue_cannot_block_stop_or_render_thread() {
    let _serial = SERIAL.lock().await;
    let root = tempfile::tempdir().unwrap();
    let terminal = python(
        root.path(),
        "import tty,time; from pathlib import Path; tty.setraw(0); Path('ready').write_text('ready'); time.sleep(300)",
    );
    wait_file(root.path(), "ready").await;
    let data = vec![b'x'; 256 * 1024];
    let start = Instant::now();
    let mut bounded = false;
    for _ in 0..100 {
        if matches!(terminal.input(&data), Err(RuntimeError::Limit)) {
            bounded = true;
            break;
        }
    }
    assert!(bounded, "input queue was not bounded");
    terminal.kill().unwrap();
    terminal.kill().unwrap();
    assert!(
        start.elapsed() < Duration::from_millis(250),
        "control call blocked on the child"
    );
    finished(&terminal).await;
}

#[tokio::test]
async fn repeated_start_close_and_partial_start_leave_no_pty_handles() {
    let _serial = SERIAL.lock().await;
    let root = tempfile::tempdir().unwrap();
    let count = || {
        std::fs::read_dir("/proc/self/fd")
            .unwrap()
            .filter_map(Result::ok)
            .filter_map(|entry| std::fs::read_link(entry.path()).ok())
            .filter(|path| path.starts_with("/dev/pts"))
            .count()
    };
    let before = count();
    for n in 0..10 {
        let mut launch = LaunchSpec::new("/synara-test/definitely-missing-command");
        assert!(NativeTerminal::spawn(&launch, root.path(), 24, 80).is_err());
        launch.command = "sh".into();
        launch.args = vec!["-c".into(), "printf cycle; sleep 300".into()];
        let terminal = NativeTerminal::spawn(&launch, root.path(), 24, 80).unwrap();
        terminal.shutdown().await.unwrap();
        drop(terminal);
        assert_eq!(count(), before, "PTY descriptors leaked in cycle {n}");
    }
    let terminal = python(root.path(), &format!("{JOB}\nwhile True: time.sleep(1)\n"));
    wait_file(root.path(), "ready").await;
    let job: i32 = wait_file(root.path(), "job").await.parse().unwrap();
    drop(terminal);
    let deadline = Instant::now() + Duration::from_secs(3);
    while running(job) || count() != before {
        assert!(
            Instant::now() < deadline,
            "drop did not clean child/reader handles"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}
