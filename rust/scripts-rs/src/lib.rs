//! JS bindings for `apps/marketing/scripts/`: generated from the signatures in `port.rs` by rust-anything.
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

/// `parseFrontmatter`.
#[napi(js_name = "parseFrontmatter")]
pub fn parse_frontmatter(source: String) -> napi::Result<String> {
    run(move || {
        let source = crate::js::from_json(&source)?;
        let out = port::parse_frontmatter(source)?;
        crate::js::to_json(&out)
    })
}

/// `findUnbalancedCodeFences`.
#[napi(js_name = "findUnbalancedCodeFences")]
pub fn find_unbalanced_code_fences(source: String) -> napi::Result<String> {
    run(move || {
        let source = crate::js::from_json(&source)?;
        let out = port::find_unbalanced_code_fences(source)?;
        crate::js::to_json(&out)
    })
}

/// `findUnbalancedMdxComponents`.
#[napi(js_name = "findUnbalancedMdxComponents")]
pub fn find_unbalanced_mdx_components(source: String) -> napi::Result<String> {
    run(move || {
        let source = crate::js::from_json(&source)?;
        let out = port::find_unbalanced_mdx_components(source)?;
        crate::js::to_json(&out)
    })
}

/// `extractInternalLinks`.
#[napi(js_name = "extractInternalLinks")]
pub fn extract_internal_links(source: String) -> napi::Result<String> {
    run(move || {
        let source = crate::js::from_json(&source)?;
        let out = port::extract_internal_links(source)?;
        crate::js::to_json(&out)
    })
}

/// `main`.
#[napi(js_name = "main")]
pub fn main() -> napi::Result<String> {
    run(move || {
        let out = port::main()?;
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

static S_PARSE_FRONTMATTER_0: wire::Schema = wire::Schema::Dyn;
static S_PARSE_FRONTMATTER_OUT: wire::Schema = wire::Schema::Dyn;

/// `parseFrontmatter` over the binary wire.
#[napi(js_name = "parseFrontmatterWire")]
pub fn parse_frontmatter_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let source: serde_json::Value = wire::read_value(r, &S_PARSE_FRONTMATTER_0)?;
        let out = &port::parse_frontmatter(source)?;
        wire::write_value(w, out, &S_PARSE_FRONTMATTER_OUT)?;
        Ok(())
    })
}

static S_FIND_UNBALANCED_CODE_FENCES_0: wire::Schema = wire::Schema::Dyn;
static S_FIND_UNBALANCED_CODE_FENCES_OUT: wire::Schema = wire::Schema::Dyn;

/// `findUnbalancedCodeFences` over the binary wire.
#[napi(js_name = "findUnbalancedCodeFencesWire")]
pub fn find_unbalanced_code_fences_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let source: serde_json::Value = wire::read_value(r, &S_FIND_UNBALANCED_CODE_FENCES_0)?;
        let out = &port::find_unbalanced_code_fences(source)?;
        wire::write_value(w, out, &S_FIND_UNBALANCED_CODE_FENCES_OUT)?;
        Ok(())
    })
}

static S_FIND_UNBALANCED_MDX_COMPONENTS_0: wire::Schema = wire::Schema::Dyn;
static S_FIND_UNBALANCED_MDX_COMPONENTS_OUT: wire::Schema = wire::Schema::Dyn;

/// `findUnbalancedMdxComponents` over the binary wire.
#[napi(js_name = "findUnbalancedMdxComponentsWire")]
pub fn find_unbalanced_mdx_components_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let source: serde_json::Value = wire::read_value(r, &S_FIND_UNBALANCED_MDX_COMPONENTS_0)?;
        let out = &port::find_unbalanced_mdx_components(source)?;
        wire::write_value(w, out, &S_FIND_UNBALANCED_MDX_COMPONENTS_OUT)?;
        Ok(())
    })
}

static S_EXTRACT_INTERNAL_LINKS_0: wire::Schema = wire::Schema::Dyn;
static S_EXTRACT_INTERNAL_LINKS_OUT: wire::Schema = wire::Schema::Dyn;

/// `extractInternalLinks` over the binary wire.
#[napi(js_name = "extractInternalLinksWire")]
pub fn extract_internal_links_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let source: serde_json::Value = wire::read_value(r, &S_EXTRACT_INTERNAL_LINKS_0)?;
        let out = &port::extract_internal_links(source)?;
        wire::write_value(w, out, &S_EXTRACT_INTERNAL_LINKS_OUT)?;
        Ok(())
    })
}

static S_MAIN_OUT: wire::Schema = wire::Schema::Dyn;

/// `main` over the binary wire.
#[napi(js_name = "mainWire")]
pub fn main_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::main()?;
        wire::write_value(w, out, &S_MAIN_OUT)?;
        Ok(())
    })
}
