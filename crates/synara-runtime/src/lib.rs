//! Execution resources and filesystem capabilities, independent of the desktop UI.
mod bounded;
mod device;
mod filesystem;
mod host;
mod platform;
mod process;
#[cfg(target_os = "linux")]
mod process_linux;
mod remote_fs;
#[cfg(unix)]
mod remote_terminal;
mod secrets;
mod ssh;
mod terminal;
mod terminal_input;
mod terminal_screen;

pub use bounded::*;
pub use device::*;
pub use filesystem::*;
pub use host::*;
pub use platform::*;
pub use process::*;
pub use remote_fs::*;
#[cfg(unix)]
pub use remote_terminal::*;
pub use secrets::*;
pub use ssh::*;
pub use terminal::*;
pub use terminal_input::*;
pub use terminal_screen::*;

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("I/O operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid input: {0}")]
    Invalid(String),
    #[error("access denied: {0}")]
    Denied(String),
    #[error("file changed outside this editor")]
    Conflict,
    #[error("resource exceeds the configured limit")]
    Limit,
    #[error("operation is not supported: {0}")]
    Unsupported(String),
    #[error("execution timed out")]
    Timeout,
    #[error("resource is no longer available")]
    Closed,
    #[error("a remote guarded write may have completed but its acknowledgement was lost")]
    WriteOutcomeUnknown,
}
