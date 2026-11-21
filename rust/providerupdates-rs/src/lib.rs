//! JS bindings for `apps/web/src/providerUpdates.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `isProviderUpdateActive`.
#[napi(js_name = "isProviderUpdateActive")]
pub fn is_provider_update_active(provider: String) -> napi::Result<bool> {
    run(move || {
        let provider = crate::js::from_json(&provider)?;
        let out = port::is_provider_update_active(provider)?;
        Ok(out)
    })
}

/// `isProviderLatestVersionKnowable`.
#[napi(js_name = "isProviderLatestVersionKnowable")]
pub fn is_provider_latest_version_knowable(provider: String) -> napi::Result<bool> {
    run(move || {
        let provider = crate::js::from_json(&provider)?;
        let out = port::is_provider_latest_version_knowable(provider)?;
        Ok(out)
    })
}

/// `shouldOfferProviderUpdateAction`.
#[napi(js_name = "shouldOfferProviderUpdateAction")]
pub fn should_offer_provider_update_action(provider: String) -> napi::Result<bool> {
    run(move || {
        let provider = crate::js::from_json(&provider)?;
        let out = port::should_offer_provider_update_action(provider)?;
        Ok(out)
    })
}

/// `shouldPromptProviderUpdate`.
#[napi(js_name = "shouldPromptProviderUpdate")]
pub fn should_prompt_provider_update(provider: String) -> napi::Result<bool> {
    run(move || {
        let provider = crate::js::from_json(&provider)?;
        let out = port::should_prompt_provider_update(provider)?;
        Ok(out)
    })
}

/// `shouldShowProviderUpdateStatus`.
#[napi(js_name = "shouldShowProviderUpdateStatus")]
pub fn should_show_provider_update_status(input: String) -> napi::Result<bool> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::should_show_provider_update_status(input)?;
        Ok(out)
    })
}

/// `getVisibleProviderUpdateStatuses`.
#[napi(js_name = "getVisibleProviderUpdateStatuses")]
pub fn get_visible_provider_update_statuses(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::get_visible_provider_update_statuses(input)?;
        crate::js::to_json(&out)
    })
}

/// `getNotifiableProviderUpdateStatuses`.
#[napi(js_name = "getNotifiableProviderUpdateStatuses")]
pub fn get_notifiable_provider_update_statuses(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::get_notifiable_provider_update_statuses(input)?;
        crate::js::to_json(&out)
    })
}

/// `providerUpdateNotificationKey`.
#[napi(js_name = "providerUpdateNotificationKey")]
pub fn provider_update_notification_key(providers: String) -> napi::Result<String> {
    run(move || {
        let providers = crate::js::from_json(&providers)?;
        let out = port::provider_update_notification_key(providers)?;
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

static S_IS_PROVIDER_UPDATE_ACTIVE_0: wire::Schema = wire::Schema::Dyn;

/// `isProviderUpdateActive` over the binary wire.
#[napi(js_name = "isProviderUpdateActiveWire")]
pub fn is_provider_update_active_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let provider: serde_json::Value = wire::read_value(r, &S_IS_PROVIDER_UPDATE_ACTIVE_0)?;
        let out = &port::is_provider_update_active(provider)?;
        w.bool(*out);
        Ok(())
    })
}

static S_IS_PROVIDER_LATEST_VERSION_KNOWABLE_0: wire::Schema = wire::Schema::Dyn;

/// `isProviderLatestVersionKnowable` over the binary wire.
#[napi(js_name = "isProviderLatestVersionKnowableWire")]
pub fn is_provider_latest_version_knowable_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let provider: serde_json::Value =
            wire::read_value(r, &S_IS_PROVIDER_LATEST_VERSION_KNOWABLE_0)?;
        let out = &port::is_provider_latest_version_knowable(provider)?;
        w.bool(*out);
        Ok(())
    })
}

static S_SHOULD_OFFER_PROVIDER_UPDATE_ACTION_0: wire::Schema = wire::Schema::Dyn;

/// `shouldOfferProviderUpdateAction` over the binary wire.
#[napi(js_name = "shouldOfferProviderUpdateActionWire")]
pub fn should_offer_provider_update_action_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let provider: serde_json::Value =
            wire::read_value(r, &S_SHOULD_OFFER_PROVIDER_UPDATE_ACTION_0)?;
        let out = &port::should_offer_provider_update_action(provider)?;
        w.bool(*out);
        Ok(())
    })
}

static S_SHOULD_PROMPT_PROVIDER_UPDATE_0: wire::Schema = wire::Schema::Dyn;

/// `shouldPromptProviderUpdate` over the binary wire.
#[napi(js_name = "shouldPromptProviderUpdateWire")]
pub fn should_prompt_provider_update_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let provider: serde_json::Value = wire::read_value(r, &S_SHOULD_PROMPT_PROVIDER_UPDATE_0)?;
        let out = &port::should_prompt_provider_update(provider)?;
        w.bool(*out);
        Ok(())
    })
}

static S_SHOULD_SHOW_PROVIDER_UPDATE_STATUS_0: wire::Schema = wire::Schema::Dyn;

/// `shouldShowProviderUpdateStatus` over the binary wire.
#[napi(js_name = "shouldShowProviderUpdateStatusWire")]
pub fn should_show_provider_update_status_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_SHOULD_SHOW_PROVIDER_UPDATE_STATUS_0)?;
        let out = &port::should_show_provider_update_status(input)?;
        w.bool(*out);
        Ok(())
    })
}

static S_GET_VISIBLE_PROVIDER_UPDATE_STATUSES_0: wire::Schema = wire::Schema::Dyn;
static S_GET_VISIBLE_PROVIDER_UPDATE_STATUSES_OUT: wire::Schema = wire::Schema::Dyn;

/// `getVisibleProviderUpdateStatuses` over the binary wire.
#[napi(js_name = "getVisibleProviderUpdateStatusesWire")]
pub fn get_visible_provider_update_statuses_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_GET_VISIBLE_PROVIDER_UPDATE_STATUSES_0)?;
        let out = &port::get_visible_provider_update_statuses(input)?;
        wire::write_value(w, out, &S_GET_VISIBLE_PROVIDER_UPDATE_STATUSES_OUT)?;
        Ok(())
    })
}

static S_GET_NOTIFIABLE_PROVIDER_UPDATE_STATUSES_0: wire::Schema = wire::Schema::Dyn;
static S_GET_NOTIFIABLE_PROVIDER_UPDATE_STATUSES_OUT: wire::Schema = wire::Schema::Dyn;

/// `getNotifiableProviderUpdateStatuses` over the binary wire.
#[napi(js_name = "getNotifiableProviderUpdateStatusesWire")]
pub fn get_notifiable_provider_update_statuses_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_GET_NOTIFIABLE_PROVIDER_UPDATE_STATUSES_0)?;
        let out = &port::get_notifiable_provider_update_statuses(input)?;
        wire::write_value(w, out, &S_GET_NOTIFIABLE_PROVIDER_UPDATE_STATUSES_OUT)?;
        Ok(())
    })
}

static S_PROVIDER_UPDATE_NOTIFICATION_KEY_0: wire::Schema = wire::Schema::Dyn;
static S_PROVIDER_UPDATE_NOTIFICATION_KEY_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Str16);

/// `providerUpdateNotificationKey` over the binary wire.
#[napi(js_name = "providerUpdateNotificationKeyWire")]
pub fn provider_update_notification_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let providers: serde_json::Value =
            wire::read_value(r, &S_PROVIDER_UPDATE_NOTIFICATION_KEY_0)?;
        let out = &port::provider_update_notification_key(providers)?;
        wire::write_value(w, out, &S_PROVIDER_UPDATE_NOTIFICATION_KEY_OUT)?;
        Ok(())
    })
}
