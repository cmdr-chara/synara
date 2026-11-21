//! JS bindings for `apps/server/src/codexProcessEnv.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `buildCodexAppServerArgs`.
#[napi(js_name = "buildCodexAppServerArgs")]
pub fn build_codex_app_server_args(source_home_path: String) -> napi::Result<Vec<String>> {
    run(move || {
        let out = port::build_codex_app_server_args(source_home_path)?;
        Ok(out)
    })
}

/// `readEffectiveCodexAuthCredentialsStoreMode`.
#[napi(js_name = "readEffectiveCodexAuthCredentialsStoreMode")]
pub fn read_effective_codex_auth_credentials_store_mode(config: String) -> napi::Result<String> {
    run(move || {
        let out = port::read_effective_codex_auth_credentials_store_mode(config)?;
        Ok(out)
    })
}

/// `readCodexPreparedHomeFileSnapshot`.
#[napi(js_name = "readCodexPreparedHomeFileSnapshot")]
pub fn read_codex_prepared_home_file_snapshot(
    source: String,
    file_name: String,
) -> napi::Result<String> {
    run(move || {
        let source = crate::js::from_json(&source)?;
        let out = port::read_codex_prepared_home_file_snapshot(source, file_name)?;
        crate::js::to_json(&out)
    })
}

/// `readCodexPreparedAuthTrackingFingerprint`.
#[napi(js_name = "readCodexPreparedAuthTrackingFingerprint")]
pub fn read_codex_prepared_auth_tracking_fingerprint(tracking: String) -> napi::Result<String> {
    run(move || {
        let tracking = crate::js::from_json(&tracking)?;
        let out = port::read_codex_prepared_auth_tracking_fingerprint(tracking)?;
        Ok(out)
    })
}

/// `readSynaraConfigSuppressions`.
#[napi(js_name = "readSynaraConfigSuppressions")]
pub async fn read_synara_config_suppressions(marker_path: String) -> napi::Result<Vec<String>> {
    run_async(async move {
        let out = port::read_synara_config_suppressions(marker_path).await?;
        Ok(out)
    })
    .await
}

/// `disableCodexConfigSections`.
#[napi(js_name = "disableCodexConfigSections")]
pub fn disable_codex_config_sections(
    config: String,
    section_headers: Vec<String>,
    append_missing: Option<bool>,
) -> napi::Result<String> {
    run(move || {
        let out = port::disable_codex_config_sections(config, section_headers, append_missing)?;
        Ok(out)
    })
}

/// `disableCompetingCodexBrowserPluginsInConfig`.
#[napi(js_name = "disableCompetingCodexBrowserPluginsInConfig")]
pub fn disable_competing_codex_browser_plugins_in_config(config: String) -> napi::Result<String> {
    run(move || {
        let out = port::disable_competing_codex_browser_plugins_in_config(config)?;
        Ok(out)
    })
}

/// `linkOrCopyCodexOverlayEntry`.
#[napi(js_name = "linkOrCopyCodexOverlayEntry")]
pub async fn link_or_copy_codex_overlay_entry(input: String, linker: String) -> napi::Result<()> {
    run_async(async move {
        let input = crate::js::from_json(&input)?;
        let linker = crate::js::from_json(&linker)?;
        port::link_or_copy_codex_overlay_entry(input, linker).await?;
        Ok(())
    })
    .await
}

/// `prioritizeCodexOverlayEntries`.
#[napi(js_name = "prioritizeCodexOverlayEntries")]
pub fn prioritize_codex_overlay_entries(entries: Vec<String>) -> napi::Result<Vec<String>> {
    run(move || {
        let out = port::prioritize_codex_overlay_entries(entries)?;
        Ok(out)
    })
}

/// `extractManagedCodexConfigSection`.
#[napi(js_name = "extractManagedCodexConfigSection")]
pub fn extract_managed_codex_config_section(config: String) -> napi::Result<String> {
    run(move || {
        let out = port::extract_managed_codex_config_section(config)?;
        crate::js::to_json(&out)
    })
}

/// `configHasTomlTableHeader`.
#[napi(js_name = "configHasTomlTableHeader")]
pub fn config_has_toml_table_header(config: String, header: String) -> napi::Result<bool> {
    run(move || {
        let out = port::config_has_toml_table_header(config, header)?;
        Ok(out)
    })
}

/// `mergeShellEnvPolicyExclude`.
#[napi(js_name = "mergeShellEnvPolicyExclude")]
pub fn merge_shell_env_policy_exclude(
    config: String,
    env_var_name: String,
) -> napi::Result<String> {
    run(move || {
        let out = port::merge_shell_env_policy_exclude(config, env_var_name)?;
        Ok(out)
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

/// `buildCodexAppServerArgs` over the binary wire.
#[napi(js_name = "buildCodexAppServerArgsWire")]
pub fn build_codex_app_server_args_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let source_home_path: String = r.string16()?;
        let out = &port::build_codex_app_server_args(source_home_path)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}

/// `readEffectiveCodexAuthCredentialsStoreMode` over the binary wire.
#[napi(js_name = "readEffectiveCodexAuthCredentialsStoreModeWire")]
pub fn read_effective_codex_auth_credentials_store_mode_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let config: String = r.string16()?;
        let out = &port::read_effective_codex_auth_credentials_store_mode(config)?;
        w.string16(out);
        Ok(())
    })
}

static S_READ_CODEX_PREPARED_HOME_FILE_SNAPSHOT_0: wire::Schema = wire::Schema::Dyn;
static S_READ_CODEX_PREPARED_HOME_FILE_SNAPSHOT_OUT: wire::Schema = wire::Schema::Dyn;

/// `readCodexPreparedHomeFileSnapshot` over the binary wire.
#[napi(js_name = "readCodexPreparedHomeFileSnapshotWire")]
pub fn read_codex_prepared_home_file_snapshot_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let source: serde_json::Value =
            wire::read_value(r, &S_READ_CODEX_PREPARED_HOME_FILE_SNAPSHOT_0)?;
        let file_name: String = r.string16()?;
        let out = &port::read_codex_prepared_home_file_snapshot(source, file_name)?;
        wire::write_value(w, out, &S_READ_CODEX_PREPARED_HOME_FILE_SNAPSHOT_OUT)?;
        Ok(())
    })
}

static S_READ_CODEX_PREPARED_AUTH_TRACKING_FINGERPRINT_0: wire::Schema = wire::Schema::Dyn;

/// `readCodexPreparedAuthTrackingFingerprint` over the binary wire.
#[napi(js_name = "readCodexPreparedAuthTrackingFingerprintWire")]
pub fn read_codex_prepared_auth_tracking_fingerprint_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let tracking: serde_json::Value =
            wire::read_value(r, &S_READ_CODEX_PREPARED_AUTH_TRACKING_FINGERPRINT_0)?;
        let out = &port::read_codex_prepared_auth_tracking_fingerprint(tracking)?;
        w.string16(out);
        Ok(())
    })
}

/// `disableCodexConfigSections` over the binary wire.
#[napi(js_name = "disableCodexConfigSectionsWire")]
pub fn disable_codex_config_sections_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let config: String = r.string16()?;
        let section_headers: Vec<String> = r.list(|r| r.string16())?;
        let append_missing: Option<bool> = r.opt_js(|r| r.bool())?;
        let out = &port::disable_codex_config_sections(config, section_headers, append_missing)?;
        w.string16(out);
        Ok(())
    })
}

/// `disableCompetingCodexBrowserPluginsInConfig` over the binary wire.
#[napi(js_name = "disableCompetingCodexBrowserPluginsInConfigWire")]
pub fn disable_competing_codex_browser_plugins_in_config_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let config: String = r.string16()?;
        let out = &port::disable_competing_codex_browser_plugins_in_config(config)?;
        w.string16(out);
        Ok(())
    })
}

/// `prioritizeCodexOverlayEntries` over the binary wire.
#[napi(js_name = "prioritizeCodexOverlayEntriesWire")]
pub fn prioritize_codex_overlay_entries_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let entries: Vec<String> = r.list(|r| r.string16())?;
        let out = &port::prioritize_codex_overlay_entries(entries)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}

static S_EXTRACT_MANAGED_CODEX_CONFIG_SECTION_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Str16);

/// `extractManagedCodexConfigSection` over the binary wire.
#[napi(js_name = "extractManagedCodexConfigSectionWire")]
pub fn extract_managed_codex_config_section_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let config: String = r.string16()?;
        let out = &port::extract_managed_codex_config_section(config)?;
        wire::write_value(w, out, &S_EXTRACT_MANAGED_CODEX_CONFIG_SECTION_OUT)?;
        Ok(())
    })
}

/// `configHasTomlTableHeader` over the binary wire.
#[napi(js_name = "configHasTomlTableHeaderWire")]
pub fn config_has_toml_table_header_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let config: String = r.string16()?;
        let header: String = r.string16()?;
        let out = &port::config_has_toml_table_header(config, header)?;
        w.bool(*out);
        Ok(())
    })
}

/// `mergeShellEnvPolicyExclude` over the binary wire.
#[napi(js_name = "mergeShellEnvPolicyExcludeWire")]
pub fn merge_shell_env_policy_exclude_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let config: String = r.string16()?;
        let env_var_name: String = r.string16()?;
        let out = &port::merge_shell_env_policy_exclude(config, env_var_name)?;
        w.string16(out);
        Ok(())
    })
}
