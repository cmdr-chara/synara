//! JS bindings for `apps/server/src/localServerMonitor.ts`: generated from the signatures in `port.rs` by rust-anything.
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

#[allow(dead_code)]
async fn run_async<T>(f: impl std::future::Future<Output = Result<T, String>>) -> napi::Result<T> {
    use futures::FutureExt as _;
    match std::panic::AssertUnwindSafe(f).catch_unwind().await {
        Ok(result) => result.map_err(napi::Error::from_reason),
        Err(payload) => Err(napi::Error::from_reason(format!(
            "Rust panic: {}",
            panic_message(&*payload)
        ))),
    }
}

/// `parseLsofTcpListenOutput`.
#[napi(js_name = "parseLsofTcpListenOutput")]
pub fn parse_lsof_tcp_listen_output(output: String) -> napi::Result<String> {
    run(move || {
        let out = port::parse_lsof_tcp_listen_output(output)?;
        crate::js::to_json(&out)
    })
}

/// `parseLsofCwdOutput`.
#[napi(js_name = "parseLsofCwdOutput")]
pub fn parse_lsof_cwd_output(output: String) -> napi::Result<String> {
    run(move || {
        let out = port::parse_lsof_cwd_output(output)?;
        crate::js::to_json(&out)
    })
}

/// `parseProcessInfo`.
#[napi(js_name = "parseProcessInfo")]
pub fn parse_process_info(output: String) -> napi::Result<String> {
    run(move || {
        let out = port::parse_process_info(output)?;
        crate::js::to_json(&out)
    })
}

/// `isIgnoredLocalServerProcess`.
#[napi(js_name = "isIgnoredLocalServerProcess")]
pub fn is_ignored_local_server_process(input: String) -> napi::Result<bool> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::is_ignored_local_server_process(input)?;
        Ok(out)
    })
}

/// `detectDevServerKindFromText`.
#[napi(js_name = "detectDevServerKindFromText")]
pub fn detect_dev_server_kind_from_text(
    input: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::detect_dev_server_kind_from_text(input)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `extractLocalServerPageTitle`.
#[napi(js_name = "extractLocalServerPageTitle")]
pub fn extract_local_server_page_title(
    html: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::extract_local_server_page_title(html)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `buildLocalServerProcesses`.
#[napi(js_name = "buildLocalServerProcesses")]
pub fn build_local_server_processes(
    listener_items: String,
    process_info_by_pid: String,
    cwd_by_pid: String,
) -> napi::Result<String> {
    run(move || {
        let listener_items = crate::js::from_json(&listener_items)?;
        let process_info_by_pid = crate::js::from_json(&process_info_by_pid)?;
        let cwd_by_pid = crate::js::from_json(&cwd_by_pid)?;
        let out =
            port::build_local_server_processes(listener_items, process_info_by_pid, cwd_by_pid)?;
        crate::js::to_json(&out)
    })
}

/// `listLocalServers`.
#[napi(js_name = "listLocalServers")]
pub async fn list_local_servers() -> napi::Result<String> {
    run_async(async move {
        let out = port::list_local_servers().await?;
        crate::js::to_json(&out)
    })
    .await
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

static S_PARSE_LSOF_TCP_LISTEN_OUTPUT_OUT: wire::Schema =
    wire::Schema::List(&wire::Schema::Obj(&[
        ("pid", wire::Schema::Num),
        ("command", wire::Schema::Str16),
        ("protocol", wire::Schema::Str16),
        ("host", wire::Schema::Str16),
        ("port", wire::Schema::Num),
        ("family", wire::Schema::Dyn),
    ]));

/// `parseLsofTcpListenOutput` over the binary wire.
#[napi(js_name = "parseLsofTcpListenOutputWire")]
pub fn parse_lsof_tcp_listen_output_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let output: String = r.string16()?;
        let out = &port::parse_lsof_tcp_listen_output(output)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_PARSE_LSOF_TCP_LISTEN_OUTPUT_OUT.inner())?;
        }
        Ok(())
    })
}

static S_PARSE_LSOF_CWD_OUTPUT_OUT: wire::Schema = wire::Schema::Dyn;

/// `parseLsofCwdOutput` over the binary wire.
#[napi(js_name = "parseLsofCwdOutputWire")]
pub fn parse_lsof_cwd_output_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let output: String = r.string16()?;
        let out = &port::parse_lsof_cwd_output(output)?;
        wire::write_value(w, out, &S_PARSE_LSOF_CWD_OUTPUT_OUT)?;
        Ok(())
    })
}

static S_PARSE_PROCESS_INFO_OUT: wire::Schema = wire::Schema::Dyn;

/// `parseProcessInfo` over the binary wire.
#[napi(js_name = "parseProcessInfoWire")]
pub fn parse_process_info_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let output: String = r.string16()?;
        let out = &port::parse_process_info(output)?;
        wire::write_value(w, out, &S_PARSE_PROCESS_INFO_OUT)?;
        Ok(())
    })
}

static S_IS_IGNORED_LOCAL_SERVER_PROCESS_0: wire::Schema = wire::Schema::Dyn;

/// `isIgnoredLocalServerProcess` over the binary wire.
#[napi(js_name = "isIgnoredLocalServerProcessWire")]
pub fn is_ignored_local_server_process_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_IS_IGNORED_LOCAL_SERVER_PROCESS_0)?;
        let out = &port::is_ignored_local_server_process(input)?;
        w.bool(*out);
        Ok(())
    })
}

static S_DETECT_DEV_SERVER_KIND_FROM_TEXT_0: wire::Schema = wire::Schema::Dyn;

/// `detectDevServerKindFromText` over the binary wire.
#[napi(js_name = "detectDevServerKindFromTextWire")]
pub fn detect_dev_server_kind_from_text_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_DETECT_DEV_SERVER_KIND_FROM_TEXT_0)?;
        let out = &port::detect_dev_server_kind_from_text(input)?;
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

/// `extractLocalServerPageTitle` over the binary wire.
#[napi(js_name = "extractLocalServerPageTitleWire")]
pub fn extract_local_server_page_title_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let html: String = r.string16()?;
        let out = &port::extract_local_server_page_title(html)?;
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

static S_BUILD_LOCAL_SERVER_PROCESSES_0: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[
    ("pid", wire::Schema::Num),
    ("command", wire::Schema::Str16),
    ("protocol", wire::Schema::Str16),
    ("host", wire::Schema::Str16),
    ("port", wire::Schema::Num),
    ("family", wire::Schema::Dyn),
]));
static S_BUILD_LOCAL_SERVER_PROCESSES_1: wire::Schema = wire::Schema::Dyn;
static S_BUILD_LOCAL_SERVER_PROCESSES_2: wire::Schema = wire::Schema::Dyn;
static S_BUILD_LOCAL_SERVER_PROCESSES_OUT: wire::Schema = wire::Schema::Dyn;

/// `buildLocalServerProcesses` over the binary wire.
#[napi(js_name = "buildLocalServerProcessesWire")]
pub fn build_local_server_processes_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let listener_items: Vec<serde_json::Value> =
            r.list(|r| wire::read_value(r, S_BUILD_LOCAL_SERVER_PROCESSES_0.inner()))?;
        let process_info_by_pid: serde_json::Value =
            wire::read_value(r, &S_BUILD_LOCAL_SERVER_PROCESSES_1)?;
        let cwd_by_pid: serde_json::Value = wire::read_value(r, &S_BUILD_LOCAL_SERVER_PROCESSES_2)?;
        let out =
            &port::build_local_server_processes(listener_items, process_info_by_pid, cwd_by_pid)?;
        wire::write_value(w, out, &S_BUILD_LOCAL_SERVER_PROCESSES_OUT)?;
        Ok(())
    })
}
