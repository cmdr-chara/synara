//! JS bindings for `apps/web/src/lib/projectCatalog/`: generated from the signatures in `port.rs` by rust-anything.
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

/// `emptyCatalog`.
#[napi(js_name = "emptyCatalog")]
pub fn empty_catalog() -> napi::Result<String> {
    run(move || {
        let out = port::empty_catalog()?;
        crate::js::to_json(&out)
    })
}

/// `checkoutKey`.
#[napi(js_name = "checkoutKey")]
pub fn checkout_key(r#ref: String) -> napi::Result<String> {
    run(move || {
        let r#ref = crate::js::from_json(&r#ref)?;
        let out = port::checkout_key(r#ref)?;
        Ok(out)
    })
}

/// `repositoryIdentity`.
#[napi(js_name = "repositoryIdentity")]
pub fn repository_identity(raw: String) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::repository_identity(raw)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `observeCheckouts`.
#[napi(js_name = "observeCheckouts")]
pub fn observe_checkouts(
    catalog: String,
    environment_id: String,
    incoming: String,
) -> napi::Result<String> {
    run(move || {
        let catalog = crate::js::from_json(&catalog)?;
        let incoming = crate::js::from_json(&incoming)?;
        let out = port::observe_checkouts(catalog, environment_id, incoming)?;
        crate::js::to_json(&out)
    })
}

/// `catalogSuggestions`.
#[napi(js_name = "catalogSuggestions")]
pub fn catalog_suggestions(catalog: String) -> napi::Result<String> {
    run(move || {
        let catalog = crate::js::from_json(&catalog)?;
        let out = port::catalog_suggestions(catalog)?;
        crate::js::to_json(&out)
    })
}

/// `excludeSuggestion`.
#[napi(js_name = "excludeSuggestion")]
pub fn exclude_suggestion(catalog: String, a: String, b: String) -> napi::Result<String> {
    run(move || {
        let catalog = crate::js::from_json(&catalog)?;
        let a = crate::js::from_json(&a)?;
        let b = crate::js::from_json(&b)?;
        let out = port::exclude_suggestion(catalog, a, b)?;
        crate::js::to_json(&out)
    })
}

/// `linkCheckouts`.
#[napi(js_name = "linkCheckouts")]
pub fn link_checkouts(
    catalog: String,
    refs: String,
    name: String,
    id: String,
) -> napi::Result<String> {
    run(move || {
        let catalog = crate::js::from_json(&catalog)?;
        let refs = crate::js::from_json(&refs)?;
        let out = port::link_checkouts(catalog, refs, name, id)?;
        crate::js::to_json(&out)
    })
}

/// `unlinkCheckout`.
#[napi(js_name = "unlinkCheckout")]
pub fn unlink_checkout(catalog: String, group_id: String, r#ref: String) -> napi::Result<String> {
    run(move || {
        let catalog = crate::js::from_json(&catalog)?;
        let r#ref = crate::js::from_json(&r#ref)?;
        let out = port::unlink_checkout(catalog, group_id, r#ref)?;
        crate::js::to_json(&out)
    })
}

/// `splitGroup`.
#[napi(js_name = "splitGroup")]
pub fn split_group(catalog: String, group_id: String) -> napi::Result<String> {
    run(move || {
        let catalog = crate::js::from_json(&catalog)?;
        let out = port::split_group(catalog, group_id)?;
        crate::js::to_json(&out)
    })
}

/// `openCatalogCheckout`.
#[napi(js_name = "openCatalogCheckout")]
pub async fn open_catalog_checkout(checkout: String) -> napi::Result<()> {
    run_async(async move {
        let checkout = crate::js::from_json(&checkout)?;
        port::open_catalog_checkout(checkout).await?;
        Ok(())
    })
    .await
}

/// `projectCatalogKey`.
#[napi(js_name = "projectCatalogKey")]
pub fn project_catalog_key() -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::project_catalog_key()?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `readProjectCatalog`.
#[napi(js_name = "readProjectCatalog")]
pub fn read_project_catalog(key: String) -> napi::Result<String> {
    run(move || {
        let out = port::read_project_catalog(key)?;
        crate::js::to_json(&out)
    })
}

/// `saveProjectCatalog`.
#[napi(js_name = "saveProjectCatalog")]
pub fn save_project_catalog(key: String, value: String) -> napi::Result<()> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        port::save_project_catalog(key, value)?;
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

static S_EMPTY_CATALOG_OUT: wire::Schema = wire::Schema::Obj(&[
    ("version", wire::Schema::Num),
    (
        "checkouts",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("name", wire::Schema::Str16),
            ("cwd", wire::Schema::Str16),
            ("hostName", wire::Schema::Str16),
            ("channel", wire::Schema::Str16),
            ("hostId", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("observedAt", wire::Schema::Str16),
            ("repositoryUrls", wire::Schema::List(&wire::Schema::Str16)),
            ("missing", wire::Schema::OptJs(&wire::Schema::Bool)),
        ])),
    ),
    (
        "groups",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("id", wire::Schema::Str16),
            ("name", wire::Schema::Str16),
            ("appearance", wire::Schema::Dyn),
            ("members", wire::Schema::List(&wire::Schema::Dyn)),
            (
                "preferredCheckout",
                wire::Schema::OptJs(&wire::Schema::Obj(&[
                    ("environmentId", wire::Schema::Dyn),
                    ("projectId", wire::Schema::Dyn),
                ])),
            ),
        ])),
    ),
    ("excludedPairs", wire::Schema::List(&wire::Schema::Str16)),
]);

/// `emptyCatalog` over the binary wire.
#[napi(js_name = "emptyCatalogWire")]
pub fn empty_catalog_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::empty_catalog()?;
        wire::write_value(w, out, &S_EMPTY_CATALOG_OUT)?;
        Ok(())
    })
}

static S_CHECKOUT_KEY_0: wire::Schema = wire::Schema::Dyn;

/// `checkoutKey` over the binary wire.
#[napi(js_name = "checkoutKeyWire")]
pub fn checkout_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let r#ref: serde_json::Value = wire::read_value(r, &S_CHECKOUT_KEY_0)?;
        let out = &port::checkout_key(r#ref)?;
        w.string16(out);
        Ok(())
    })
}

/// `repositoryIdentity` over the binary wire.
#[napi(js_name = "repositoryIdentityWire")]
pub fn repository_identity_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw: String = r.string16()?;
        let out = &port::repository_identity(raw)?;
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

static S_OBSERVE_CHECKOUTS_0: wire::Schema = wire::Schema::Dyn;
static S_OBSERVE_CHECKOUTS_2: wire::Schema = wire::Schema::Dyn;
static S_OBSERVE_CHECKOUTS_OUT: wire::Schema = wire::Schema::Obj(&[
    ("version", wire::Schema::Num),
    (
        "checkouts",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("name", wire::Schema::Str16),
            ("cwd", wire::Schema::Str16),
            ("hostName", wire::Schema::Str16),
            ("channel", wire::Schema::Str16),
            ("hostId", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("observedAt", wire::Schema::Str16),
            ("repositoryUrls", wire::Schema::List(&wire::Schema::Str16)),
            ("missing", wire::Schema::OptJs(&wire::Schema::Bool)),
        ])),
    ),
    (
        "groups",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("id", wire::Schema::Str16),
            ("name", wire::Schema::Str16),
            ("appearance", wire::Schema::Dyn),
            ("members", wire::Schema::List(&wire::Schema::Dyn)),
            (
                "preferredCheckout",
                wire::Schema::OptJs(&wire::Schema::Obj(&[
                    ("environmentId", wire::Schema::Dyn),
                    ("projectId", wire::Schema::Dyn),
                ])),
            ),
        ])),
    ),
    ("excludedPairs", wire::Schema::List(&wire::Schema::Str16)),
]);

/// `observeCheckouts` over the binary wire.
#[napi(js_name = "observeCheckoutsWire")]
pub fn observe_checkouts_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let catalog: serde_json::Value = wire::read_value(r, &S_OBSERVE_CHECKOUTS_0)?;
        let environment_id: String = r.string16()?;
        let incoming: serde_json::Value = wire::read_value(r, &S_OBSERVE_CHECKOUTS_2)?;
        let out = &port::observe_checkouts(catalog, environment_id, incoming)?;
        wire::write_value(w, out, &S_OBSERVE_CHECKOUTS_OUT)?;
        Ok(())
    })
}

static S_CATALOG_SUGGESTIONS_0: wire::Schema = wire::Schema::Dyn;
static S_CATALOG_SUGGESTIONS_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Dyn);

/// `catalogSuggestions` over the binary wire.
#[napi(js_name = "catalogSuggestionsWire")]
pub fn catalog_suggestions_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let catalog: serde_json::Value = wire::read_value(r, &S_CATALOG_SUGGESTIONS_0)?;
        let out = &port::catalog_suggestions(catalog)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_CATALOG_SUGGESTIONS_OUT.inner())?;
        }
        Ok(())
    })
}

static S_EXCLUDE_SUGGESTION_0: wire::Schema = wire::Schema::Dyn;
static S_EXCLUDE_SUGGESTION_1: wire::Schema = wire::Schema::Dyn;
static S_EXCLUDE_SUGGESTION_2: wire::Schema = wire::Schema::Dyn;
static S_EXCLUDE_SUGGESTION_OUT: wire::Schema = wire::Schema::Obj(&[
    ("version", wire::Schema::Num),
    (
        "checkouts",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("name", wire::Schema::Str16),
            ("cwd", wire::Schema::Str16),
            ("hostName", wire::Schema::Str16),
            ("channel", wire::Schema::Str16),
            ("hostId", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("observedAt", wire::Schema::Str16),
            ("repositoryUrls", wire::Schema::List(&wire::Schema::Str16)),
            ("missing", wire::Schema::OptJs(&wire::Schema::Bool)),
        ])),
    ),
    (
        "groups",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("id", wire::Schema::Str16),
            ("name", wire::Schema::Str16),
            ("appearance", wire::Schema::Dyn),
            ("members", wire::Schema::List(&wire::Schema::Dyn)),
            (
                "preferredCheckout",
                wire::Schema::OptJs(&wire::Schema::Obj(&[
                    ("environmentId", wire::Schema::Dyn),
                    ("projectId", wire::Schema::Dyn),
                ])),
            ),
        ])),
    ),
    ("excludedPairs", wire::Schema::List(&wire::Schema::Str16)),
]);

/// `excludeSuggestion` over the binary wire.
#[napi(js_name = "excludeSuggestionWire")]
pub fn exclude_suggestion_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let catalog: serde_json::Value = wire::read_value(r, &S_EXCLUDE_SUGGESTION_0)?;
        let a: serde_json::Value = wire::read_value(r, &S_EXCLUDE_SUGGESTION_1)?;
        let b: serde_json::Value = wire::read_value(r, &S_EXCLUDE_SUGGESTION_2)?;
        let out = &port::exclude_suggestion(catalog, a, b)?;
        wire::write_value(w, out, &S_EXCLUDE_SUGGESTION_OUT)?;
        Ok(())
    })
}

static S_LINK_CHECKOUTS_0: wire::Schema = wire::Schema::Dyn;
static S_LINK_CHECKOUTS_1: wire::Schema = wire::Schema::Dyn;
static S_LINK_CHECKOUTS_OUT: wire::Schema = wire::Schema::Obj(&[
    ("version", wire::Schema::Num),
    (
        "checkouts",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("name", wire::Schema::Str16),
            ("cwd", wire::Schema::Str16),
            ("hostName", wire::Schema::Str16),
            ("channel", wire::Schema::Str16),
            ("hostId", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("observedAt", wire::Schema::Str16),
            ("repositoryUrls", wire::Schema::List(&wire::Schema::Str16)),
            ("missing", wire::Schema::OptJs(&wire::Schema::Bool)),
        ])),
    ),
    (
        "groups",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("id", wire::Schema::Str16),
            ("name", wire::Schema::Str16),
            ("appearance", wire::Schema::Dyn),
            ("members", wire::Schema::List(&wire::Schema::Dyn)),
            (
                "preferredCheckout",
                wire::Schema::OptJs(&wire::Schema::Obj(&[
                    ("environmentId", wire::Schema::Dyn),
                    ("projectId", wire::Schema::Dyn),
                ])),
            ),
        ])),
    ),
    ("excludedPairs", wire::Schema::List(&wire::Schema::Str16)),
]);

/// `linkCheckouts` over the binary wire.
#[napi(js_name = "linkCheckoutsWire")]
pub fn link_checkouts_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let catalog: serde_json::Value = wire::read_value(r, &S_LINK_CHECKOUTS_0)?;
        let refs: serde_json::Value = wire::read_value(r, &S_LINK_CHECKOUTS_1)?;
        let name: String = r.string16()?;
        let id: String = r.string16()?;
        let out = &port::link_checkouts(catalog, refs, name, id)?;
        wire::write_value(w, out, &S_LINK_CHECKOUTS_OUT)?;
        Ok(())
    })
}

static S_UNLINK_CHECKOUT_0: wire::Schema = wire::Schema::Dyn;
static S_UNLINK_CHECKOUT_2: wire::Schema = wire::Schema::Dyn;
static S_UNLINK_CHECKOUT_OUT: wire::Schema = wire::Schema::Obj(&[
    ("version", wire::Schema::Num),
    (
        "checkouts",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("name", wire::Schema::Str16),
            ("cwd", wire::Schema::Str16),
            ("hostName", wire::Schema::Str16),
            ("channel", wire::Schema::Str16),
            ("hostId", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("observedAt", wire::Schema::Str16),
            ("repositoryUrls", wire::Schema::List(&wire::Schema::Str16)),
            ("missing", wire::Schema::OptJs(&wire::Schema::Bool)),
        ])),
    ),
    (
        "groups",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("id", wire::Schema::Str16),
            ("name", wire::Schema::Str16),
            ("appearance", wire::Schema::Dyn),
            ("members", wire::Schema::List(&wire::Schema::Dyn)),
            (
                "preferredCheckout",
                wire::Schema::OptJs(&wire::Schema::Obj(&[
                    ("environmentId", wire::Schema::Dyn),
                    ("projectId", wire::Schema::Dyn),
                ])),
            ),
        ])),
    ),
    ("excludedPairs", wire::Schema::List(&wire::Schema::Str16)),
]);

/// `unlinkCheckout` over the binary wire.
#[napi(js_name = "unlinkCheckoutWire")]
pub fn unlink_checkout_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let catalog: serde_json::Value = wire::read_value(r, &S_UNLINK_CHECKOUT_0)?;
        let group_id: String = r.string16()?;
        let r#ref: serde_json::Value = wire::read_value(r, &S_UNLINK_CHECKOUT_2)?;
        let out = &port::unlink_checkout(catalog, group_id, r#ref)?;
        wire::write_value(w, out, &S_UNLINK_CHECKOUT_OUT)?;
        Ok(())
    })
}

static S_SPLIT_GROUP_0: wire::Schema = wire::Schema::Dyn;
static S_SPLIT_GROUP_OUT: wire::Schema = wire::Schema::Obj(&[
    ("version", wire::Schema::Num),
    (
        "checkouts",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("name", wire::Schema::Str16),
            ("cwd", wire::Schema::Str16),
            ("hostName", wire::Schema::Str16),
            ("channel", wire::Schema::Str16),
            ("hostId", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("observedAt", wire::Schema::Str16),
            ("repositoryUrls", wire::Schema::List(&wire::Schema::Str16)),
            ("missing", wire::Schema::OptJs(&wire::Schema::Bool)),
        ])),
    ),
    (
        "groups",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("id", wire::Schema::Str16),
            ("name", wire::Schema::Str16),
            ("appearance", wire::Schema::Dyn),
            ("members", wire::Schema::List(&wire::Schema::Dyn)),
            (
                "preferredCheckout",
                wire::Schema::OptJs(&wire::Schema::Obj(&[
                    ("environmentId", wire::Schema::Dyn),
                    ("projectId", wire::Schema::Dyn),
                ])),
            ),
        ])),
    ),
    ("excludedPairs", wire::Schema::List(&wire::Schema::Str16)),
]);

/// `splitGroup` over the binary wire.
#[napi(js_name = "splitGroupWire")]
pub fn split_group_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let catalog: serde_json::Value = wire::read_value(r, &S_SPLIT_GROUP_0)?;
        let group_id: String = r.string16()?;
        let out = &port::split_group(catalog, group_id)?;
        wire::write_value(w, out, &S_SPLIT_GROUP_OUT)?;
        Ok(())
    })
}

/// `projectCatalogKey` over the binary wire.
#[napi(js_name = "projectCatalogKeyWire")]
pub fn project_catalog_key_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::project_catalog_key()?;
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

static S_READ_PROJECT_CATALOG_OUT: wire::Schema = wire::Schema::Obj(&[
    ("version", wire::Schema::Num),
    (
        "checkouts",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("name", wire::Schema::Str16),
            ("cwd", wire::Schema::Str16),
            ("hostName", wire::Schema::Str16),
            ("channel", wire::Schema::Str16),
            ("hostId", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("observedAt", wire::Schema::Str16),
            ("repositoryUrls", wire::Schema::List(&wire::Schema::Str16)),
            ("missing", wire::Schema::OptJs(&wire::Schema::Bool)),
        ])),
    ),
    (
        "groups",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("id", wire::Schema::Str16),
            ("name", wire::Schema::Str16),
            ("appearance", wire::Schema::Dyn),
            ("members", wire::Schema::List(&wire::Schema::Dyn)),
            (
                "preferredCheckout",
                wire::Schema::OptJs(&wire::Schema::Obj(&[
                    ("environmentId", wire::Schema::Dyn),
                    ("projectId", wire::Schema::Dyn),
                ])),
            ),
        ])),
    ),
    ("excludedPairs", wire::Schema::List(&wire::Schema::Str16)),
]);

/// `readProjectCatalog` over the binary wire.
#[napi(js_name = "readProjectCatalogWire")]
pub fn read_project_catalog_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let key: String = r.string16()?;
        let out = &port::read_project_catalog(key)?;
        wire::write_value(w, out, &S_READ_PROJECT_CATALOG_OUT)?;
        Ok(())
    })
}

static S_SAVE_PROJECT_CATALOG_1: wire::Schema = wire::Schema::Dyn;

/// `saveProjectCatalog` over the binary wire.
#[napi(js_name = "saveProjectCatalogWire")]
pub fn save_project_catalog_wire() -> napi::Result<f64> {
    wire_io(|r, _w| {
        let key: String = r.string16()?;
        let value: serde_json::Value = wire::read_value(r, &S_SAVE_PROJECT_CATALOG_1)?;
        let () = port::save_project_catalog(key, value)?;
        Ok(())
    })
}
