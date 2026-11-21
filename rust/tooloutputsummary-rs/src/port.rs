//! The Rust port of `packages/shared/src/toolOutputSummary.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

/// Port of `stripTrailingToolExitCode` (`packages/shared/src/toolOutputSummary.ts:18`).
pub fn strip_trailing_tool_exit_code(value: String) -> Result<serde_json::Value, String> {
    let _ = &value;
    unimplemented!("port `stripTrailingToolExitCode` from packages/shared/src/toolOutputSummary.ts")
}

/// Port of `countTextLines` (`packages/shared/src/toolOutputSummary.ts:38`).
pub fn count_text_lines(content: String) -> Result<f64, String> {
    let _ = &content;
    unimplemented!("port `countTextLines` from packages/shared/src/toolOutputSummary.ts")
}

/// Port of `summarizeToolRawOutput` (`packages/shared/src/toolOutputSummary.ts:49`).
pub fn summarize_tool_raw_output(raw_output: serde_json::Value) -> Result<Option<String>, String> {
    let _ = &raw_output;
    unimplemented!("port `summarizeToolRawOutput` from packages/shared/src/toolOutputSummary.ts")
}
