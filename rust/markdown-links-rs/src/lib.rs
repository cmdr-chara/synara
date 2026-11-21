//! JS bindings for `apps/web/src/markdown-links.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `markdownFilePathHref`.
#[napi(js_name = "markdownFilePathHref")]
pub fn markdown_file_path_href(path: String) -> napi::Result<String> {
    run(move || {
        let out = port::markdown_file_path_href(path)?;
        Ok(out)
    })
}

/// `rewriteMarkdownFileUriHref`.
#[napi(js_name = "rewriteMarkdownFileUriHref")]
pub fn rewrite_markdown_file_uri_href(
    href: Option<String>,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::rewrite_markdown_file_uri_href(href)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `extractAbsoluteFilesystemPaths`.
#[napi(js_name = "extractAbsoluteFilesystemPaths")]
pub fn extract_absolute_filesystem_paths(text: String) -> napi::Result<Vec<String>> {
    run(move || {
        let out = port::extract_absolute_filesystem_paths(text)?;
        Ok(out)
    })
}

/// `resolveUniqueAbsoluteSuffixTarget`.
#[napi(js_name = "resolveUniqueAbsoluteSuffixTarget")]
pub fn resolve_unique_absolute_suffix_target(
    reference: String,
    known_absolute_paths: Vec<String>,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::resolve_unique_absolute_suffix_target(reference, known_absolute_paths)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `resolveChatFileChipTarget`.
#[napi(js_name = "resolveChatFileChipTarget")]
pub fn resolve_chat_file_chip_target(
    reference: Option<String>,
    cwd: Option<String>,
    known_absolute_paths: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let known_absolute_paths = crate::js::from_json(&known_absolute_paths)?;
        let out = port::resolve_chat_file_chip_target(reference, cwd, known_absolute_paths)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `resolveMarkdownFileLinkTarget`.
#[napi(js_name = "resolveMarkdownFileLinkTarget")]
pub fn resolve_markdown_file_link_target(
    href: Option<String>,
    cwd: Option<String>,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::resolve_markdown_file_link_target(href, cwd)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
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

/// `markdownFilePathHref` over the binary wire.
#[napi(js_name = "markdownFilePathHrefWire")]
pub fn markdown_file_path_href_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let path: String = r.string16()?;
        let out = &port::markdown_file_path_href(path)?;
        w.string16(out);
        Ok(())
    })
}

/// `rewriteMarkdownFileUriHref` over the binary wire.
#[napi(js_name = "rewriteMarkdownFileUriHrefWire")]
pub fn rewrite_markdown_file_uri_href_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let href: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::rewrite_markdown_file_uri_href(href)?;
        match out {
            Some(x0) => {
                w.tag(1);
                w.string16(x0);
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

/// `extractAbsoluteFilesystemPaths` over the binary wire.
#[napi(js_name = "extractAbsoluteFilesystemPathsWire")]
pub fn extract_absolute_filesystem_paths_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: String = r.string16()?;
        let out = &port::extract_absolute_filesystem_paths(text)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}

/// `resolveUniqueAbsoluteSuffixTarget` over the binary wire.
#[napi(js_name = "resolveUniqueAbsoluteSuffixTargetWire")]
pub fn resolve_unique_absolute_suffix_target_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let reference: String = r.string16()?;
        let known_absolute_paths: Vec<String> = r.list(|r| r.string16())?;
        let out = &port::resolve_unique_absolute_suffix_target(reference, known_absolute_paths)?;
        match out {
            Some(x0) => {
                w.tag(1);
                w.string16(x0);
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

/// `resolveChatFileChipTarget` over the binary wire.
#[napi(js_name = "resolveChatFileChipTargetWire")]
pub fn resolve_chat_file_chip_target_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let reference: Option<String> = r.opt_js(|r| r.string16())?;
        let cwd: Option<String> = r.opt_js(|r| r.string16())?;
        let known_absolute_paths: Option<Vec<String>> = r.opt_js(|r| r.list(|r| r.string16()))?;
        let out = &port::resolve_chat_file_chip_target(reference, cwd, known_absolute_paths)?;
        match out {
            Some(x0) => {
                w.tag(1);
                w.string16(x0);
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

/// `resolveMarkdownFileLinkTarget` over the binary wire.
#[napi(js_name = "resolveMarkdownFileLinkTargetWire")]
pub fn resolve_markdown_file_link_target_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let href: Option<String> = r.opt_js(|r| r.string16())?;
        let cwd: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::resolve_markdown_file_link_target(href, cwd)?;
        match out {
            Some(x0) => {
                w.tag(1);
                w.string16(x0);
            }
            None => w.tag(2),
        }
        Ok(())
    })
}
