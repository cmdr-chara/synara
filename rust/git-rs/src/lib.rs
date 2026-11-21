//! JS bindings for `packages/shared/src/git.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `sanitizeBranchFragment`.
#[napi(js_name = "sanitizeBranchFragment")]
pub fn sanitize_branch_fragment(raw: String) -> napi::Result<String> {
    run(move || {
        let out = port::sanitize_branch_fragment(raw)?;
        Ok(out)
    })
}

/// `sanitizeFeatureBranchName`.
#[napi(js_name = "sanitizeFeatureBranchName")]
pub fn sanitize_feature_branch_name(raw: String) -> napi::Result<String> {
    run(move || {
        let out = port::sanitize_feature_branch_name(raw)?;
        Ok(out)
    })
}

/// `resolveAutoFeatureBranchName`.
#[napi(js_name = "resolveAutoFeatureBranchName")]
pub fn resolve_auto_feature_branch_name(
    existing_branch_names: Vec<String>,
    preferred_branch: Option<String>,
) -> napi::Result<String> {
    run(move || {
        let out = port::resolve_auto_feature_branch_name(existing_branch_names, preferred_branch)?;
        Ok(out)
    })
}

/// `buildSynaraBranchName`.
#[napi(js_name = "buildSynaraBranchName")]
pub fn build_synara_branch_name(preferred_branch: Option<String>) -> napi::Result<String> {
    run(move || {
        let out = port::build_synara_branch_name(preferred_branch)?;
        Ok(out)
    })
}

/// `resolveUniqueSynaraBranchName`.
#[napi(js_name = "resolveUniqueSynaraBranchName")]
pub fn resolve_unique_synara_branch_name(
    existing_branch_names: Vec<String>,
    preferred_branch: Option<String>,
) -> napi::Result<String> {
    run(move || {
        let out = port::resolve_unique_synara_branch_name(existing_branch_names, preferred_branch)?;
        Ok(out)
    })
}

/// `isTemporaryWorktreeBranch`.
#[napi(js_name = "isTemporaryWorktreeBranch")]
pub fn is_temporary_worktree_branch(branch: String) -> napi::Result<bool> {
    run(move || {
        let out = port::is_temporary_worktree_branch(branch)?;
        Ok(out)
    })
}

/// `resolveThreadBranchRegressionGuard`.
#[napi(js_name = "resolveThreadBranchRegressionGuard")]
pub fn resolve_thread_branch_regression_guard(
    input: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::resolve_thread_branch_regression_guard(input)?;
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

/// `sanitizeBranchFragment` over the binary wire.
#[napi(js_name = "sanitizeBranchFragmentWire")]
pub fn sanitize_branch_fragment_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw: String = r.string16()?;
        let out = &port::sanitize_branch_fragment(raw)?;
        w.string16(out);
        Ok(())
    })
}

/// `sanitizeFeatureBranchName` over the binary wire.
#[napi(js_name = "sanitizeFeatureBranchNameWire")]
pub fn sanitize_feature_branch_name_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw: String = r.string16()?;
        let out = &port::sanitize_feature_branch_name(raw)?;
        w.string16(out);
        Ok(())
    })
}

/// `resolveAutoFeatureBranchName` over the binary wire.
#[napi(js_name = "resolveAutoFeatureBranchNameWire")]
pub fn resolve_auto_feature_branch_name_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let existing_branch_names: Vec<String> = r.list(|r| r.string16())?;
        let preferred_branch: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::resolve_auto_feature_branch_name(existing_branch_names, preferred_branch)?;
        w.string16(out);
        Ok(())
    })
}

/// `buildSynaraBranchName` over the binary wire.
#[napi(js_name = "buildSynaraBranchNameWire")]
pub fn build_synara_branch_name_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let preferred_branch: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::build_synara_branch_name(preferred_branch)?;
        w.string16(out);
        Ok(())
    })
}

/// `resolveUniqueSynaraBranchName` over the binary wire.
#[napi(js_name = "resolveUniqueSynaraBranchNameWire")]
pub fn resolve_unique_synara_branch_name_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let existing_branch_names: Vec<String> = r.list(|r| r.string16())?;
        let preferred_branch: Option<String> = r.opt_js(|r| r.string16())?;
        let out =
            &port::resolve_unique_synara_branch_name(existing_branch_names, preferred_branch)?;
        w.string16(out);
        Ok(())
    })
}

/// `isTemporaryWorktreeBranch` over the binary wire.
#[napi(js_name = "isTemporaryWorktreeBranchWire")]
pub fn is_temporary_worktree_branch_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let branch: String = r.string16()?;
        let out = &port::is_temporary_worktree_branch(branch)?;
        w.bool(*out);
        Ok(())
    })
}

static S_RESOLVE_THREAD_BRANCH_REGRESSION_GUARD_0: wire::Schema = wire::Schema::Dyn;

/// `resolveThreadBranchRegressionGuard` over the binary wire.
#[napi(js_name = "resolveThreadBranchRegressionGuardWire")]
pub fn resolve_thread_branch_regression_guard_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_THREAD_BRANCH_REGRESSION_GUARD_0)?;
        let out = &port::resolve_thread_branch_regression_guard(input)?;
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
