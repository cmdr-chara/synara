//! Execution resources and filesystem capabilities, independent of the desktop UI.
mod bounded;
mod filesystem;
mod host;
mod process;
mod platform;
mod secrets;
#[cfg(target_os = "linux")]
mod process_linux;
mod remote_fs;
#[cfg(unix)]
mod remote_terminal;
mod ssh;
mod terminal;
mod terminal_input;
mod terminal_screen;

pub use bounded::*;
pub use filesystem::*;
pub use host::*;
pub use process::*;
pub use platform::*;
pub use remote_fs::*;
pub use secrets::*;
#[cfg(unix)]
pub use remote_terminal::*;
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
