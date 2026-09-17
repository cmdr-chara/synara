//! Linux PTY session cleanup. The leader MUST remain unreaped until this returns.
//!
//! A terminal owns its controlling-terminal session, not arbitrary processes
//! that deliberately detach with setsid. pidfds prevent recycled PID signaling.
use crate::RuntimeError;
use nix::{
    sys::wait::{Id, WaitPidFlag, WaitStatus, waitid},
    unistd::{Pid, getsid},
};
use rustix::{
    event::{PollFd, PollFlags, Timespec, poll},
    process::{PidfdFlags, Signal, pidfd_open, pidfd_send_signal},
};
use std::{
    os::fd::OwnedFd,
    time::{Duration, Instant},
};

pub(super) fn exited_without_reaping(pid: u32) -> Result<bool, RuntimeError> {
    let raw = i32::try_from(pid).map_err(|_| RuntimeError::Invalid("invalid child PID".into()))?;
    if raw <= 1 {
        return Err(RuntimeError::Invalid("invalid child PID".into()));
    }
    match waitid(
        Id::Pid(Pid::from_raw(raw)),
        WaitPidFlag::WEXITED | WaitPidFlag::WNOWAIT | WaitPidFlag::WNOHANG,
    ) {
        Ok(WaitStatus::StillAlive) => Ok(false),
        Ok(WaitStatus::Exited(..) | WaitStatus::Signaled(..)) => Ok(true),
        Ok(_) => Ok(false),
        Err(nix::errno::Errno::EINTR) => Ok(false),
        Err(error) => Err(std::io::Error::from_raw_os_error(error as i32).into()),
    }
}

fn live(fd: &OwnedFd) -> Result<bool, RuntimeError> {
    let mut fds = [PollFd::new(fd, PollFlags::IN)];
    let ready = poll(
        &mut fds,
        Some(&Timespec {
            tv_sec: 0,
            tv_nsec: 0,
        }),
    )
    .map_err(std::io::Error::from)?;
    Ok(ready == 0)
}

pub(super) fn cleanup_session(pid: u32) -> Result<(), RuntimeError> {
    let raw =
        i32::try_from(pid).map_err(|_| RuntimeError::Invalid("invalid session PID".into()))?;
    if raw <= 1 {
        return Err(RuntimeError::Invalid("invalid session PID".into()));
    }
    let sid = Pid::from_raw(raw);
    if getsid(Some(sid)).ok() != Some(sid) {
        return Err(RuntimeError::Denied(
            "PTY child is not its owned session leader".into(),
        ));
    }
    // The unreaped leader reserves this process-group/session ID. Freeze further
    // shell spawning before collecting foreground and background job groups.
    crate::process::signal_tree(pid, true);
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        let mut found_live = false;
        for (count, entry) in std::fs::read_dir("/proc")?.enumerate() {
            if count >= 131_072 {
                return Err(RuntimeError::Limit);
            }
            let entry = entry?;
            let Some(candidate) = entry
                .file_name()
                .to_str()
                .and_then(|s| s.parse::<i32>().ok())
            else {
                continue;
            };
            if candidate <= 1 || candidate == raw {
                continue;
            }
            let id = Pid::from_raw(candidate);
            if getsid(Some(id)).ok() != Some(sid) {
                continue;
            }
            let id = rustix::process::Pid::from_raw(candidate).ok_or(RuntimeError::Closed)?;
            let fd = match pidfd_open(id, PidfdFlags::empty()) {
                Ok(fd) => fd,
                Err(rustix::io::Errno::SRCH) => continue,
                Err(error) => return Err(std::io::Error::from(error).into()),
            };
            // Revalidate after pinning the process. A PID recycled before open
            // must never make us signal a different session.
            if getsid(Some(Pid::from_raw(candidate))).ok() != Some(sid) || !live(&fd)? {
                continue;
            }
            found_live = true;
            match pidfd_send_signal(&fd, Signal::KILL) {
                Ok(()) | Err(rustix::io::Errno::SRCH) => {}
                Err(error) => return Err(std::io::Error::from(error).into()),
            }
        }
        if !found_live {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(RuntimeError::Timeout);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}
