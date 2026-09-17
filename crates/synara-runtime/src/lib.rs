//! Execution resources and filesystem capabilities, independent of the desktop UI.
mod bounded;
mod filesystem;
mod host;
mod process;
mod ssh;
mod terminal;

pub use bounded::*;
pub use filesystem::*;
pub use host::*;
pub use process::*;
pub use ssh::*;
pub use terminal::*;

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
}
