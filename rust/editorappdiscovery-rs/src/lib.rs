//! JS bindings for `apps/server/src/editorAppDiscovery.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `getEditorMacApplications`.
#[napi(js_name = "getEditorMacApplications")]
pub fn get_editor_mac_applications(editor: String) -> napi::Result<String> {
    run(move || {
        let editor = crate::js::from_json(&editor)?;
        let out = port::get_editor_mac_applications(editor)?;
        crate::js::to_json(&out)
    })
}

/// `getEditorWindowsUriScheme`.
#[napi(js_name = "getEditorWindowsUriScheme")]
pub fn get_editor_windows_uri_scheme(editor: String) -> napi::Result<String> {
    run(move || {
        let editor = crate::js::from_json(&editor)?;
        let out = port::get_editor_windows_uri_scheme(editor)?;
        crate::js::to_json(&out)
    })
}

/// `getEditorWindowsStorePackages`.
#[napi(js_name = "getEditorWindowsStorePackages")]
pub fn get_editor_windows_store_packages(editor: String) -> napi::Result<String> {
    run(move || {
        let editor = crate::js::from_json(&editor)?;
        let out = port::get_editor_windows_store_packages(editor)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeMacApplicationBundleName`.
#[napi(js_name = "normalizeMacApplicationBundleName")]
pub fn normalize_mac_application_bundle_name(app_name: String) -> napi::Result<String> {
    run(move || {
        let out = port::normalize_mac_application_bundle_name(app_name)?;
        Ok(out)
    })
}

/// `resolveMacApplicationSearchPaths`.
#[napi(js_name = "resolveMacApplicationSearchPaths")]
pub fn resolve_mac_application_search_paths(
    app_name: String,
    env: String,
) -> napi::Result<Vec<String>> {
    run(move || {
        let env = crate::js::from_json(&env)?;
        let out = port::resolve_mac_application_search_paths(app_name, env)?;
        Ok(out)
    })
}

/// `resolveMacApplicationBundlePath`.
#[napi(js_name = "resolveMacApplicationBundlePath")]
pub fn resolve_mac_application_bundle_path(
    app_names: String,
    platform: String,
    env: String,
) -> napi::Result<String> {
    run(move || {
        let app_names = crate::js::from_json(&app_names)?;
        let platform = crate::js::from_json(&platform)?;
        let env = crate::js::from_json(&env)?;
        let out = port::resolve_mac_application_bundle_path(app_names, platform, env)?;
        crate::js::to_json(&out)
    })
}

/// `resolveAvailableMacApplication`.
#[napi(js_name = "resolveAvailableMacApplication")]
pub fn resolve_available_mac_application(
    app_names: String,
    platform: String,
    env: String,
) -> napi::Result<String> {
    run(move || {
        let app_names = crate::js::from_json(&app_names)?;
        let platform = crate::js::from_json(&platform)?;
        let env = crate::js::from_json(&env)?;
        let out = port::resolve_available_mac_application(app_names, platform, env)?;
        crate::js::to_json(&out)
    })
}

/// `clearWindowsStorePackageDiscoveryCache`.
#[napi(js_name = "clearWindowsStorePackageDiscoveryCache")]
pub fn clear_windows_store_package_discovery_cache() -> napi::Result<String> {
    run(move || {
        let out = port::clear_windows_store_package_discovery_cache()?;
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

static S_GET_EDITOR_MAC_APPLICATIONS_0: wire::Schema = wire::Schema::Dyn;
static S_GET_EDITOR_MAC_APPLICATIONS_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Str16));

/// `getEditorMacApplications` over the binary wire.
#[napi(js_name = "getEditorMacApplicationsWire")]
pub fn get_editor_mac_applications_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let editor: serde_json::Value = wire::read_value(r, &S_GET_EDITOR_MAC_APPLICATIONS_0)?;
        let out = &port::get_editor_mac_applications(editor)?;
        wire::write_value(w, out, &S_GET_EDITOR_MAC_APPLICATIONS_OUT)?;
        Ok(())
    })
}

static S_GET_EDITOR_WINDOWS_URI_SCHEME_0: wire::Schema = wire::Schema::Dyn;
static S_GET_EDITOR_WINDOWS_URI_SCHEME_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Str16);

/// `getEditorWindowsUriScheme` over the binary wire.
#[napi(js_name = "getEditorWindowsUriSchemeWire")]
pub fn get_editor_windows_uri_scheme_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let editor: serde_json::Value = wire::read_value(r, &S_GET_EDITOR_WINDOWS_URI_SCHEME_0)?;
        let out = &port::get_editor_windows_uri_scheme(editor)?;
        wire::write_value(w, out, &S_GET_EDITOR_WINDOWS_URI_SCHEME_OUT)?;
        Ok(())
    })
}

static S_GET_EDITOR_WINDOWS_STORE_PACKAGES_0: wire::Schema = wire::Schema::Dyn;
static S_GET_EDITOR_WINDOWS_STORE_PACKAGES_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Obj(&[
        ("packageName", wire::Schema::Str16),
        ("publisherId", wire::Schema::Str16),
    ])));

/// `getEditorWindowsStorePackages` over the binary wire.
#[napi(js_name = "getEditorWindowsStorePackagesWire")]
pub fn get_editor_windows_store_packages_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let editor: serde_json::Value =
            wire::read_value(r, &S_GET_EDITOR_WINDOWS_STORE_PACKAGES_0)?;
        let out = &port::get_editor_windows_store_packages(editor)?;
        wire::write_value(w, out, &S_GET_EDITOR_WINDOWS_STORE_PACKAGES_OUT)?;
        Ok(())
    })
}

/// `normalizeMacApplicationBundleName` over the binary wire.
#[napi(js_name = "normalizeMacApplicationBundleNameWire")]
pub fn normalize_mac_application_bundle_name_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let app_name: String = r.string16()?;
        let out = &port::normalize_mac_application_bundle_name(app_name)?;
        w.string16(out);
        Ok(())
    })
}

static S_RESOLVE_MAC_APPLICATION_SEARCH_PATHS_1: wire::Schema = wire::Schema::Dyn;

/// `resolveMacApplicationSearchPaths` over the binary wire.
#[napi(js_name = "resolveMacApplicationSearchPathsWire")]
pub fn resolve_mac_application_search_paths_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let app_name: String = r.string16()?;
        let env: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_MAC_APPLICATION_SEARCH_PATHS_1)?;
        let out = &port::resolve_mac_application_search_paths(app_name, env)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}

static S_RESOLVE_MAC_APPLICATION_BUNDLE_PATH_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_MAC_APPLICATION_BUNDLE_PATH_1: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_MAC_APPLICATION_BUNDLE_PATH_2: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_MAC_APPLICATION_BUNDLE_PATH_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Str16);

/// `resolveMacApplicationBundlePath` over the binary wire.
#[napi(js_name = "resolveMacApplicationBundlePathWire")]
pub fn resolve_mac_application_bundle_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let app_names: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_MAC_APPLICATION_BUNDLE_PATH_0)?;
        let platform: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_MAC_APPLICATION_BUNDLE_PATH_1)?;
        let env: serde_json::Value = wire::read_value(r, &S_RESOLVE_MAC_APPLICATION_BUNDLE_PATH_2)?;
        let out = &port::resolve_mac_application_bundle_path(app_names, platform, env)?;
        wire::write_value(w, out, &S_RESOLVE_MAC_APPLICATION_BUNDLE_PATH_OUT)?;
        Ok(())
    })
}

static S_RESOLVE_AVAILABLE_MAC_APPLICATION_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_AVAILABLE_MAC_APPLICATION_1: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_AVAILABLE_MAC_APPLICATION_2: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_AVAILABLE_MAC_APPLICATION_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Str16);

/// `resolveAvailableMacApplication` over the binary wire.
#[napi(js_name = "resolveAvailableMacApplicationWire")]
pub fn resolve_available_mac_application_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let app_names: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_AVAILABLE_MAC_APPLICATION_0)?;
        let platform: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_AVAILABLE_MAC_APPLICATION_1)?;
        let env: serde_json::Value = wire::read_value(r, &S_RESOLVE_AVAILABLE_MAC_APPLICATION_2)?;
        let out = &port::resolve_available_mac_application(app_names, platform, env)?;
        wire::write_value(w, out, &S_RESOLVE_AVAILABLE_MAC_APPLICATION_OUT)?;
        Ok(())
    })
}

static S_CLEAR_WINDOWS_STORE_PACKAGE_DISCOVERY_CACHE_OUT: wire::Schema = wire::Schema::Dyn;

/// `clearWindowsStorePackageDiscoveryCache` over the binary wire.
#[napi(js_name = "clearWindowsStorePackageDiscoveryCacheWire")]
pub fn clear_windows_store_package_discovery_cache_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::clear_windows_store_package_discovery_cache()?;
        wire::write_value(w, out, &S_CLEAR_WINDOWS_STORE_PACKAGE_DISCOVERY_CACHE_OUT)?;
        Ok(())
    })
}
