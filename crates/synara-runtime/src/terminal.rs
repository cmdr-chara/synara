//! Native PTY ownership. The rendering snapshot never owns platform resources.
#[cfg(not(unix))]
mod fallback;
#[cfg(unix)]
mod posix;
#[cfg(target_os = "linux")]
mod session;

#[cfg(not(unix))]
pub use fallback::NativeTerminal;
#[cfg(unix)]
pub use posix::NativeTerminal;

/// Stable callback-facing snapshot. Raw bytes are for protocol adapters only,
/// never for GUI selection or clipboard copy.
#[derive(Clone, Debug)]
pub struct TerminalSnapshot {
    pub text: String,
    pub raw_tail: Vec<u8>,
    pub truncated: bool,
    pub exit_code: Option<u32>,
    pub error: Option<String>,
    pub revision: u64,
}

#[derive(Clone, Debug)]
pub struct TerminalRenderSnapshot {
    pub grid: crate::TerminalGrid,
    pub exit_code: Option<u32>,
    pub error: Option<String>,
}
