//! JS bindings for `apps/server/src/transcriptUsageParsers.ts`: generated from the signatures in `port.rs` by rust-anything.
//! Regenerate with `rust-anything refresh` after changing them.

use napi_derive::napi;

pub mod js;
pub mod port;
pub mod wire;

#[allow(unused_imports)]
use serde_json::Value;

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".into())
}

#[allow(dead_code)]
fn run<T>(f: impl FnOnce() -> Result<T, String>) -> napi::Result<T> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(result) => result.map_err(napi::Error::from_reason),
        Err(payload) => Err(napi::Error::from_reason(format!(
            "Rust panic: {}",
            panic_message(&*payload)
        ))),
    }
}

/// `transcriptTokenTotal`.
#[napi(js_name = "transcriptTokenTotal")]
pub fn transcript_token_total(tokens: String) -> napi::Result<f64> {
    run(move || {
        let tokens = crate::js::from_json(&tokens)?;
        let out = port::transcript_token_total(tokens)?;
        Ok(out)
    })
}

/// `transcriptPromptTokens`.
#[napi(js_name = "transcriptPromptTokens")]
pub fn transcript_prompt_tokens(tokens: String) -> napi::Result<f64> {
    run(move || {
        let tokens = crate::js::from_json(&tokens)?;
        let out = port::transcript_prompt_tokens(tokens)?;
        Ok(out)
    })
}

/// `claudeLineMightMatter`.
#[napi(js_name = "claudeLineMightMatter")]
pub fn claude_line_might_matter(line: String) -> napi::Result<bool> {
    run(move || {
        let line = crate::js::from_json(&line)?;
        let out = port::claude_line_might_matter(line)?;
        Ok(out)
    })
}

/// `parseClaudeTranscriptLine`.
#[napi(js_name = "parseClaudeTranscriptLine")]
pub fn parse_claude_transcript_line(line: String) -> napi::Result<String> {
    run(move || {
        let out = port::parse_claude_transcript_line(line)?;
        crate::js::to_json(&out)
    })
}

/// `initialCodexTranscriptState`.
#[napi(js_name = "initialCodexTranscriptState")]
pub fn initial_codex_transcript_state() -> napi::Result<String> {
    run(move || {
        let out = port::initial_codex_transcript_state()?;
        crate::js::to_json(&out)
    })
}

/// `codexLineMightMatter`.
#[napi(js_name = "codexLineMightMatter")]
pub fn codex_line_might_matter(line: String) -> napi::Result<bool> {
    run(move || {
        let line = crate::js::from_json(&line)?;
        let out = port::codex_line_might_matter(line)?;
        Ok(out)
    })
}

/// `parseCodexTranscriptLine`.
#[napi(js_name = "parseCodexTranscriptLine")]
pub fn parse_codex_transcript_line(line: String, state: String) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let out = port::parse_codex_transcript_line(line, state)?;
        crate::js::to_json(&out)
    })
}

// ---- the binary wire ------------------------------------------------------------------------

type WireBuffers = (
    napi::bindgen_prelude::Uint8Array,
    napi::bindgen_prelude::Uint8Array,
    napi::bindgen_prelude::Uint32Array,
);

thread_local! {
    static WIRE: std::cell::RefCell<Option<WireBuffers>> = const { std::cell::RefCell::new(None) };
    static PENDING: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) };
    static SCRATCH: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Registers the argument, result and argument-length buffers `index.cjs` uses.
#[napi(js_name = "rustAnythingWireInit")]
pub fn rust_anything_wire_init(
    input: napi::bindgen_prelude::Uint8Array,
    output: napi::bindgen_prelude::Uint8Array,
    len: napi::bindgen_prelude::Uint32Array,
) {
    WIRE.with(|w| *w.borrow_mut() = Some((input, output, len)));
}

/// Copies a result that did not fit into the (now larger) result buffer.
#[napi(js_name = "rustAnythingWireFlush")]
pub fn rust_anything_wire_flush() -> f64 {
    let bytes = PENDING.with(|p| std::mem::take(&mut *p.borrow_mut()));
    WIRE.with(|w| {
        let mut w = w.borrow_mut();
        let Some((_, output, _)) = w.as_mut() else {
            return -1.0;
        };
        // SAFETY: the call is synchronous; JS does not touch `output` until it returns.
        let out = unsafe { output.as_mut() };
        let n = bytes.len().min(out.len());
        out[..n].copy_from_slice(&bytes[..n]);
        n as f64
    })
}

/// Reads the argument bytes (`wLen[0]` of them), runs `f`, and writes its result; a negative
/// return is the size of a result that did not fit, kept for `rustAnythingWireFlush`.
fn wire_io(
    f: impl FnOnce(&mut wire::Reader, &mut wire::Writer) -> Result<(), String>,
) -> napi::Result<f64> {
    WIRE.with(|cell| {
        let mut guard = cell.borrow_mut();
        let Some((input, output, len)) = guard.as_mut() else {
            return Err(napi::Error::from_reason(
                "rust-anything: the wire buffers are not registered",
            ));
        };
        let len = (len.first().copied().unwrap_or(0) as usize).min(input.len());
        let mut w = wire::Writer(SCRATCH.with(|s| std::mem::take(&mut *s.borrow_mut())));
        w.0.clear();
        let result = run(|| f(&mut wire::Reader::new(&input[..len]), &mut w));
        let keep = |w: wire::Writer| SCRATCH.with(|s| *s.borrow_mut() = w.0);
        match result {
            // A result of another shape than the TS type: JS runs the call through JSON.
            Err(e) if e.reason.starts_with(wire::MISMATCH) => {
                keep(w);
                return Ok(f64::NAN);
            }
            Err(e) => {
                keep(w);
                let mut m = wire::Writer::default();
                m.string16(&e.reason);
                // SAFETY: the call is synchronous; JS does not touch `output` until it returns.
                let out = unsafe { output.as_mut() };
                if m.0.len() > out.len() {
                    return Err(e);
                }
                out[..m.0.len()].copy_from_slice(&m.0);
                return Ok(f64::INFINITY);
            }
            Ok(()) => {}
        }
        // SAFETY: the call is synchronous; JS does not touch `output` until it returns.
        let out = unsafe { output.as_mut() };
        let n = w.0.len();
        if n > out.len() {
            PENDING.with(|p| *p.borrow_mut() = w.0);
            return Ok(-(n as f64));
        }
        out[..n].copy_from_slice(&w.0);
        keep(w);
        Ok(n as f64)
    })
}

static S_TRANSCRIPT_TOKEN_TOTAL_0: wire::Schema = wire::Schema::Dyn;

/// `transcriptTokenTotal` over the binary wire.
#[napi(js_name = "transcriptTokenTotalWire")]
pub fn transcript_token_total_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let tokens: serde_json::Value = wire::read_value(r, &S_TRANSCRIPT_TOKEN_TOTAL_0)?;
        let out = &port::transcript_token_total(tokens)?;
        w.f64(*out);
        Ok(())
    })
}

static S_TRANSCRIPT_PROMPT_TOKENS_0: wire::Schema = wire::Schema::Dyn;

/// `transcriptPromptTokens` over the binary wire.
#[napi(js_name = "transcriptPromptTokensWire")]
pub fn transcript_prompt_tokens_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let tokens: serde_json::Value = wire::read_value(r, &S_TRANSCRIPT_PROMPT_TOKENS_0)?;
        let out = &port::transcript_prompt_tokens(tokens)?;
        w.f64(*out);
        Ok(())
    })
}

static S_CLAUDE_LINE_MIGHT_MATTER_0: wire::Schema = wire::Schema::Dyn;

/// `claudeLineMightMatter` over the binary wire.
#[napi(js_name = "claudeLineMightMatterWire")]
pub fn claude_line_might_matter_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let line: serde_json::Value = wire::read_value(r, &S_CLAUDE_LINE_MIGHT_MATTER_0)?;
        let out = &port::claude_line_might_matter(line)?;
        w.bool(*out);
        Ok(())
    })
}

/// `parseClaudeTranscriptLine` over the binary wire.
#[napi(js_name = "parseClaudeTranscriptLineWire")]
pub fn parse_claude_transcript_line_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let line: String = r.string16()?;
        let out = &port::parse_claude_transcript_line(line)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_INITIAL_CODEX_TRANSCRIPT_STATE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("model", wire::Schema::Str16),
    ("cwd", wire::Schema::Str16),
    (
        "lastUsageSignature",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    ("sawSessionMeta", wire::Schema::Bool),
    ("suppressingForkCopies", wire::Schema::Bool),
    ("forkCopyAnchorMs", wire::Schema::Num),
]);

/// `initialCodexTranscriptState` over the binary wire.
#[napi(js_name = "initialCodexTranscriptStateWire")]
pub fn initial_codex_transcript_state_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::initial_codex_transcript_state()?;
        wire::write_value(w, out, &S_INITIAL_CODEX_TRANSCRIPT_STATE_OUT)?;
        Ok(())
    })
}

static S_CODEX_LINE_MIGHT_MATTER_0: wire::Schema = wire::Schema::Dyn;

/// `codexLineMightMatter` over the binary wire.
#[napi(js_name = "codexLineMightMatterWire")]
pub fn codex_line_might_matter_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let line: serde_json::Value = wire::read_value(r, &S_CODEX_LINE_MIGHT_MATTER_0)?;
        let out = &port::codex_line_might_matter(line)?;
        w.bool(*out);
        Ok(())
    })
}

static S_PARSE_CODEX_TRANSCRIPT_LINE_1: wire::Schema = wire::Schema::Dyn;

/// `parseCodexTranscriptLine` over the binary wire.
#[napi(js_name = "parseCodexTranscriptLineWire")]
pub fn parse_codex_transcript_line_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let line: String = r.string16()?;
        let state: serde_json::Value = wire::read_value(r, &S_PARSE_CODEX_TRANSCRIPT_LINE_1)?;
        let out = &port::parse_codex_transcript_line(line, state)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}
