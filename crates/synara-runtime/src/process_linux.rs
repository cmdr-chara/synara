//! Linux child ownership. Never reap the leader before signaling its group.
use crate::{ProcessExit, RuntimeError, process::ProcessTreeGuard};
use nix::{
    sys::wait::{Id, WaitPidFlag, WaitStatus, waitid},
    unistd::{Pid, getpgid},
};
use rustix::process::{PidfdFlags, Signal, pidfd_open, pidfd_send_signal};
use std::time::{Duration, Instant};
use tokio::{process::Child, runtime::Handle, sync::watch};
use tokio_util::sync::CancellationToken;

struct OwnedChild {
    child: Child,
    guard: ProcessTreeGuard,
}
impl Drop for OwnedChild {
    fn drop(&mut self) {
        // This runs before Child's kill-on-drop/reaping machinery. It cannot
        // target a recycled group after the leader has been reaped.
        self.guard.kill_and_disarm();
    }
}

pub(crate) fn supervise(
    child: Child,
    guard: ProcessTreeGuard,
    stop: CancellationToken,
    exit: watch::Sender<Option<ProcessExit>>,
) {
    let handle = Handle::current();
    let mut owned = OwnedChild { child, guard };
    // A blocking supervisor is intentional: runtime shutdown tracks it even if
    // a caller drops its future. No process/network wait happens on the GUI.
    tokio::task::spawn_blocking(move || {
        let pid = owned.guard.pid;
        let mut stop_deadline = None;
        let mut error = None;
        loop {
            match exited(pid) {
                Ok(true) => break,
                Ok(false) => {}
                Err(e) => {
                    error = Some(e.to_string());
                    break;
                }
            }
            if stop.is_cancelled() {
                if stop_deadline.is_none() {
                    super::process::signal_tree(pid, false);
                    stop_deadline = Some(Instant::now() + Duration::from_secs(2));
                }
                if stop_deadline.is_some_and(|deadline| Instant::now() >= deadline) {
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        if let Err(e) = clean_group(pid) {
            error = Some(format!("process group cleanup incomplete: {e}"));
        }
        // The group has been signaled while the child PID was reserved. Never
        // send a numerical group signal from a post-wait Drop implementation.
        owned.guard.kill_and_disarm();
        let _ = owned.child.start_kill();
        let mut result = match handle.block_on(owned.child.wait()) {
            Ok(status) => {
                use std::os::unix::process::ExitStatusExt;
                ProcessExit {
                    code: status.code(),
                    signal: status.signal(),
                    error: None,
                }
            }
            Err(e) => ProcessExit {
                error: Some(e.to_string()),
                ..Default::default()
            },
        };
        if error.is_some() {
            result.error = error;
        }
        drop(owned);
        let _ = exit.send(Some(result));
    });
}

fn exited(pid: u32) -> Result<bool, RuntimeError> {
    let raw = i32::try_from(pid).map_err(|_| RuntimeError::Invalid("invalid child PID".into()))?;
    match waitid(
        Id::Pid(Pid::from_raw(raw)),
        WaitPidFlag::WEXITED | WaitPidFlag::WNOWAIT | WaitPidFlag::WNOHANG,
    ) {
        Ok(WaitStatus::StillAlive) => Ok(false),
        Ok(WaitStatus::Exited(..) | WaitStatus::Signaled(..)) => Ok(true),
        Ok(_) | Err(nix::errno::Errno::EINTR) => Ok(false),
        Err(error) => Err(std::io::Error::from_raw_os_error(error as i32).into()),
    }
}

fn clean_group(pid: u32) -> Result<(), RuntimeError> {
    let raw = i32::try_from(pid).map_err(|_| RuntimeError::Invalid("invalid child PID".into()))?;
    let group = Pid::from_raw(raw);
    if raw <= 1 || getpgid(Some(group)).ok() != Some(group) {
        return Err(RuntimeError::Denied(
            "process is not its owned group leader".into(),
        ));
    }
    super::process::signal_tree(pid, true);
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        let mut alive = false;
        for (count, entry) in std::fs::read_dir("/proc")?.enumerate() {
            if count >= 131_072 {
                return Err(RuntimeError::Limit);
            }
            let entry = entry?;
            let Some(id) = entry
                .file_name()
                .to_str()
                .and_then(|name| name.parse::<i32>().ok())
            else {
                continue;
            };
            if id <= 1 || id == raw || getpgid(Some(Pid::from_raw(id))).ok() != Some(group) {
                continue;
            }
            let fd = match pidfd_open(
                rustix::process::Pid::from_raw(id).ok_or(RuntimeError::Closed)?,
                PidfdFlags::empty(),
            ) {
                Ok(fd) => fd,
                Err(rustix::io::Errno::SRCH) => continue,
                Err(e) => return Err(std::io::Error::from(e).into()),
            };
            if getpgid(Some(Pid::from_raw(id))).ok() != Some(group) {
                continue;
            }
            let mut pollfd = [rustix::event::PollFd::new(
                &fd,
                rustix::event::PollFlags::IN,
            )];
            if rustix::event::poll(
                &mut pollfd,
                Some(&rustix::event::Timespec {
                    tv_sec: 0,
                    tv_nsec: 0,
                }),
            )
            .map_err(std::io::Error::from)?
                != 0
            {
                continue;
            }
            alive = true;
            match pidfd_send_signal(&fd, Signal::KILL) {
                Ok(()) | Err(rustix::io::Errno::SRCH) => {}
                Err(e) => return Err(std::io::Error::from(e).into()),
            }
        }
        if !alive {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(RuntimeError::Timeout);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}
