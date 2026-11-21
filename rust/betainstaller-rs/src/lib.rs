//! JS bindings for `apps/desktop/src/betaInstaller.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `parseBetaMacManifest`.
#[napi(js_name = "parseBetaMacManifest")]
pub fn parse_beta_mac_manifest(text: String) -> napi::Result<String> {
    run(move || {
        let out = port::parse_beta_mac_manifest(text)?;
        crate::js::to_json(&out)
    })
}

/// `selectBetaDownloadFile`.
#[napi(js_name = "selectBetaDownloadFile")]
pub fn select_beta_download_file(files: String, arch: String) -> napi::Result<String> {
    run(move || {
        let files = crate::js::from_json(&files)?;
        let out = port::select_beta_download_file(files, arch)?;
        crate::js::to_json(&out)
    })
}

/// `httpsFetchText`.
#[napi(js_name = "httpsFetchText")]
pub fn https_fetch_text(url: String) -> napi::Result<String> {
    run(move || {
        let url = crate::js::from_json(&url)?;
        let out = port::https_fetch_text(url)?;
        crate::js::to_json(&out)
    })
}

/// `sha512FileBase64`.
#[napi(js_name = "sha512FileBase64")]
pub fn sha512_file_base64(path: String) -> napi::Result<String> {
    run(move || {
        let out = port::sha512_file_base64(path)?;
        Ok(out)
    })
}

/// `httpsDownloadFile`.
#[napi(js_name = "httpsDownloadFile")]
pub fn https_download_file(
    url: String,
    destination_path: String,
    on_progress: String,
) -> napi::Result<String> {
    run(move || {
        let url = crate::js::from_json(&url)?;
        let destination_path = crate::js::from_json(&destination_path)?;
        let on_progress = crate::js::from_json(&on_progress)?;
        let out = port::https_download_file(url, destination_path, on_progress)?;
        crate::js::to_json(&out)
    })
}

/// `verifyBetaAppBundle`.
#[napi(js_name = "verifyBetaAppBundle")]
pub fn verify_beta_app_bundle(app_path: String) -> napi::Result<()> {
    run(move || {
        port::verify_beta_app_bundle(app_path)?;
        Ok(())
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

static S_PARSE_BETA_MAC_MANIFEST_OUT: wire::Schema = wire::Schema::Obj(&[
    ("version", wire::Schema::Str16),
    (
        "files",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("url", wire::Schema::Str16),
            ("sha512", wire::Schema::Str16),
            ("size", wire::Schema::OptJs(&wire::Schema::Num)),
        ])),
    ),
]);

/// `parseBetaMacManifest` over the binary wire.
#[napi(js_name = "parseBetaMacManifestWire")]
pub fn parse_beta_mac_manifest_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: String = r.string16()?;
        let out = &port::parse_beta_mac_manifest(text)?;
        wire::write_value(w, out, &S_PARSE_BETA_MAC_MANIFEST_OUT)?;
        Ok(())
    })
}

static S_SELECT_BETA_DOWNLOAD_FILE_0: wire::Schema = wire::Schema::Dyn;
static S_SELECT_BETA_DOWNLOAD_FILE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("url", wire::Schema::Str16),
    ("sha512", wire::Schema::Str16),
    ("size", wire::Schema::OptJs(&wire::Schema::Num)),
]);

/// `selectBetaDownloadFile` over the binary wire.
#[napi(js_name = "selectBetaDownloadFileWire")]
pub fn select_beta_download_file_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let files: serde_json::Value = wire::read_value(r, &S_SELECT_BETA_DOWNLOAD_FILE_0)?;
        let arch: String = r.string16()?;
        let out = &port::select_beta_download_file(files, arch)?;
        wire::write_value(w, out, &S_SELECT_BETA_DOWNLOAD_FILE_OUT)?;
        Ok(())
    })
}

static S_HTTPS_FETCH_TEXT_0: wire::Schema = wire::Schema::Dyn;
static S_HTTPS_FETCH_TEXT_OUT: wire::Schema = wire::Schema::Dyn;

/// `httpsFetchText` over the binary wire.
#[napi(js_name = "httpsFetchTextWire")]
pub fn https_fetch_text_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let url: serde_json::Value = wire::read_value(r, &S_HTTPS_FETCH_TEXT_0)?;
        let out = &port::https_fetch_text(url)?;
        wire::write_value(w, out, &S_HTTPS_FETCH_TEXT_OUT)?;
        Ok(())
    })
}

/// `sha512FileBase64` over the binary wire.
#[napi(js_name = "sha512FileBase64Wire")]
pub fn sha512_file_base64_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let path: String = r.string16()?;
        let out = &port::sha512_file_base64(path)?;
        w.string16(out);
        Ok(())
    })
}

static S_HTTPS_DOWNLOAD_FILE_0: wire::Schema = wire::Schema::Dyn;
static S_HTTPS_DOWNLOAD_FILE_1: wire::Schema = wire::Schema::Dyn;
static S_HTTPS_DOWNLOAD_FILE_2: wire::Schema = wire::Schema::Dyn;
static S_HTTPS_DOWNLOAD_FILE_OUT: wire::Schema = wire::Schema::Dyn;

/// `httpsDownloadFile` over the binary wire.
#[napi(js_name = "httpsDownloadFileWire")]
pub fn https_download_file_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let url: serde_json::Value = wire::read_value(r, &S_HTTPS_DOWNLOAD_FILE_0)?;
        let destination_path: serde_json::Value = wire::read_value(r, &S_HTTPS_DOWNLOAD_FILE_1)?;
        let on_progress: serde_json::Value = wire::read_value(r, &S_HTTPS_DOWNLOAD_FILE_2)?;
        let out = &port::https_download_file(url, destination_path, on_progress)?;
        wire::write_value(w, out, &S_HTTPS_DOWNLOAD_FILE_OUT)?;
        Ok(())
    })
}

/// `verifyBetaAppBundle` over the binary wire.
#[napi(js_name = "verifyBetaAppBundleWire")]
pub fn verify_beta_app_bundle_wire() -> napi::Result<f64> {
    wire_io(|r, _w| {
        let app_path: String = r.string16()?;
        let () = port::verify_beta_app_bundle(app_path)?;
        Ok(())
    })
}
