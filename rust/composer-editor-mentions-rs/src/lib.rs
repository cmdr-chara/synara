//! JS bindings for `apps/web/src/composer-editor-mentions.ts`: generated from the signatures in `port.rs` by rust-anything.
//! Regenerate with `rust-anything refresh` after changing them.

use napi_derive::napi;

pub mod js;
pub mod port;
pub mod wire;

#[allow(unused_imports)]
use serde_json::Value;
pub mod utf16;

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

/// `matchComposerLinkToken`.
#[napi(js_name = "matchComposerLinkToken")]
pub fn match_composer_link_token(text: String, options: String) -> napi::Result<String> {
    run(move || {
        let options = crate::js::from_json(&options)?;
        let out = port::match_composer_link_token(text, options)?;
        crate::js::to_json(&out)
    })
}

/// `matchComposerSlashCommandChipToken`.
#[napi(js_name = "matchComposerSlashCommandChipToken")]
pub fn match_composer_slash_command_chip_token(text: String) -> napi::Result<String> {
    run(move || {
        let out = port::match_composer_slash_command_chip_token(text)?;
        crate::js::to_json(&out)
    })
}

/// `splitPromptIntoDisplaySegments`.
#[napi(js_name = "splitPromptIntoDisplaySegments")]
pub fn split_prompt_into_display_segments(
    prompt: String,
    mention_references: String,
) -> napi::Result<String> {
    run(move || {
        let mention_references = crate::js::from_json(&mention_references)?;
        let out = port::split_prompt_into_display_segments(prompt, mention_references)?;
        crate::js::to_json(&out)
    })
}

/// `splitPromptIntoComposerSegments`.
#[napi(js_name = "splitPromptIntoComposerSegments")]
pub fn split_prompt_into_composer_segments(
    prompt: String,
    terminal_contexts: String,
    mention_references: String,
) -> napi::Result<String> {
    run(move || {
        let terminal_contexts = crate::js::from_json(&terminal_contexts)?;
        let mention_references = crate::js::from_json(&mention_references)?;
        let out = port::split_prompt_into_composer_segments(
            prompt,
            terminal_contexts,
            mention_references,
        )?;
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

static S_MATCH_COMPOSER_LINK_TOKEN_1: wire::Schema = wire::Schema::Dyn;
static S_MATCH_COMPOSER_LINK_TOKEN_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    ("url", wire::Schema::Str16),
    ("start", wire::Schema::Num),
    ("end", wire::Schema::Num),
]));

/// `matchComposerLinkToken` over the binary wire.
#[napi(js_name = "matchComposerLinkTokenWire")]
pub fn match_composer_link_token_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: String = r.string16()?;
        let options: serde_json::Value = wire::read_value(r, &S_MATCH_COMPOSER_LINK_TOKEN_1)?;
        let out = &port::match_composer_link_token(text, options)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_MATCH_COMPOSER_LINK_TOKEN_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_MATCH_COMPOSER_SLASH_COMMAND_CHIP_TOKEN_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Obj(&[
        ("command", wire::Schema::Dyn),
        ("start", wire::Schema::Num),
        ("end", wire::Schema::Num),
    ]));

/// `matchComposerSlashCommandChipToken` over the binary wire.
#[napi(js_name = "matchComposerSlashCommandChipTokenWire")]
pub fn match_composer_slash_command_chip_token_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: String = r.string16()?;
        let out = &port::match_composer_slash_command_chip_token(text)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_MATCH_COMPOSER_SLASH_COMMAND_CHIP_TOKEN_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_SPLIT_PROMPT_INTO_DISPLAY_SEGMENTS_1: wire::Schema = wire::Schema::Dyn;
static S_SPLIT_PROMPT_INTO_DISPLAY_SEGMENTS_OUT: wire::Schema =
    wire::Schema::List(&wire::Schema::Dyn);

/// `splitPromptIntoDisplaySegments` over the binary wire.
#[napi(js_name = "splitPromptIntoDisplaySegmentsWire")]
pub fn split_prompt_into_display_segments_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let prompt: String = r.string16()?;
        let mention_references: serde_json::Value =
            wire::read_value(r, &S_SPLIT_PROMPT_INTO_DISPLAY_SEGMENTS_1)?;
        let out = &port::split_prompt_into_display_segments(prompt, mention_references)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_SPLIT_PROMPT_INTO_DISPLAY_SEGMENTS_OUT.inner())?;
        }
        Ok(())
    })
}

static S_SPLIT_PROMPT_INTO_COMPOSER_SEGMENTS_1: wire::Schema = wire::Schema::Dyn;
static S_SPLIT_PROMPT_INTO_COMPOSER_SEGMENTS_2: wire::Schema = wire::Schema::Dyn;
static S_SPLIT_PROMPT_INTO_COMPOSER_SEGMENTS_OUT: wire::Schema =
    wire::Schema::List(&wire::Schema::Dyn);

/// `splitPromptIntoComposerSegments` over the binary wire.
#[napi(js_name = "splitPromptIntoComposerSegmentsWire")]
pub fn split_prompt_into_composer_segments_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let prompt: String = r.string16()?;
        let terminal_contexts: serde_json::Value =
            wire::read_value(r, &S_SPLIT_PROMPT_INTO_COMPOSER_SEGMENTS_1)?;
        let mention_references: serde_json::Value =
            wire::read_value(r, &S_SPLIT_PROMPT_INTO_COMPOSER_SEGMENTS_2)?;
        let out = &port::split_prompt_into_composer_segments(
            prompt,
            terminal_contexts,
            mention_references,
        )?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_SPLIT_PROMPT_INTO_COMPOSER_SEGMENTS_OUT.inner())?;
        }
        Ok(())
    })
}
