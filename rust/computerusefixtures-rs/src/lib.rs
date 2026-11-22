//! JS bindings for `scripts/computer-use-fixtures/`: generated from the signatures in `port.rs` by rust-anything.
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

/// `canaryOpenArgs`.
#[napi(js_name = "canaryOpenArgs")]
pub fn canary_open_args(app: String, env_assignment: String) -> napi::Result<String> {
    run(move || {
        let app = crate::js::from_json(&app)?;
        let env_assignment = crate::js::from_json(&env_assignment)?;
        let out = port::canary_open_args(app, env_assignment)?;
        crate::js::to_json(&out)
    })
}

/// `parseBrowserBenchmarkRunCount`.
#[napi(js_name = "parseBrowserBenchmarkRunCount")]
pub fn parse_browser_benchmark_run_count(value: Option<String>) -> napi::Result<String> {
    run(move || {
        let out = port::parse_browser_benchmark_run_count(value)?;
        crate::js::to_json(&out)
    })
}

/// `assessBrowserBenchmarkRuns`.
#[napi(js_name = "assessBrowserBenchmarkRuns")]
pub fn assess_browser_benchmark_runs(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::assess_browser_benchmark_runs(input)?;
        crate::js::to_json(&out)
    })
}

/// `parseNeweggPrice`.
#[napi(js_name = "parseNeweggPrice")]
pub fn parse_newegg_price(text: String, currency_hint: Option<String>) -> napi::Result<String> {
    run(move || {
        let out = port::parse_newegg_price(text, currency_hint)?;
        crate::js::to_json(&out)
    })
}

/// `parseNeweggPageEvidence`.
#[napi(js_name = "parseNeweggPageEvidence")]
pub fn parse_newegg_page_evidence(value: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::parse_newegg_page_evidence(value)?;
        crate::js::to_json(&out)
    })
}

/// `observedNeweggCategory`.
#[napi(js_name = "observedNeweggCategory")]
pub fn observed_newegg_category(product: String) -> napi::Result<String> {
    run(move || {
        let product = crate::js::from_json(&product)?;
        let out = port::observed_newegg_category(product)?;
        crate::js::to_json(&out)
    })
}

/// `assessNeweggCompletion`.
#[napi(js_name = "assessNeweggCompletion")]
pub fn assess_newegg_completion(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::assess_newegg_completion(input)?;
        crate::js::to_json(&out)
    })
}

/// `parseBrowserWitness`.
#[napi(js_name = "parseBrowserWitness")]
pub fn parse_browser_witness(value: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::parse_browser_witness(value)?;
        crate::js::to_json(&out)
    })
}

/// `parsePullRequestTitles`.
#[napi(js_name = "parsePullRequestTitles")]
pub fn parse_pull_request_titles(value: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::parse_pull_request_titles(value)?;
        crate::js::to_json(&out)
    })
}

/// `assessGitHubCompletion`.
#[napi(js_name = "assessGitHubCompletion")]
pub fn assess_git_hub_completion(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::assess_git_hub_completion(input)?;
        crate::js::to_json(&out)
    })
}

/// `browserBenchmarkPrompt`.
#[napi(js_name = "browserBenchmarkPrompt")]
pub fn browser_benchmark_prompt(
    task: String,
    profile_name: String,
    repo: Option<String>,
) -> napi::Result<String> {
    run(move || {
        let out = port::browser_benchmark_prompt(task, profile_name, repo)?;
        crate::js::to_json(&out)
    })
}

/// `neweggItemIdFromUrl`.
#[napi(js_name = "neweggItemIdFromUrl")]
pub fn newegg_item_id_from_url(
    value: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::newegg_item_id_from_url(value)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `parseObserverDescriptor`.
#[napi(js_name = "parseObserverDescriptor")]
pub fn parse_observer_descriptor(value: String, profile_name: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::parse_observer_descriptor(value, profile_name)?;
        crate::js::to_json(&out)
    })
}

/// `readGitHubReference`.
#[napi(js_name = "readGitHubReference")]
pub async fn read_git_hub_reference(repo: String) -> napi::Result<String> {
    run_async(async move {
        let out = port::read_git_hub_reference(repo).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `createBrowserObserver`.
#[napi(js_name = "createBrowserObserver")]
pub fn create_browser_observer(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::create_browser_observer(input)?;
        crate::js::to_json(&out)
    })
}

/// `assertPackagedAppInstallation`.
#[napi(js_name = "assertPackagedAppInstallation")]
pub fn assert_packaged_app_installation(
    bundle: String,
    registered_bundle: Option<String>,
) -> napi::Result<String> {
    run(move || {
        let out = port::assert_packaged_app_installation(bundle, registered_bundle)?;
        crate::js::to_json(&out)
    })
}

/// `artifactIdentity`.
#[napi(js_name = "artifactIdentity")]
pub async fn artifact_identity(bundle: String) -> napi::Result<String> {
    run_async(async move {
        let out = port::artifact_identity(bundle).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `assertLoopbackUrl`.
#[napi(js_name = "assertLoopbackUrl")]
pub fn assert_loopback_url(value: String, protocol: String) -> napi::Result<String> {
    run(move || {
        let out = port::assert_loopback_url(value, protocol)?;
        crate::js::to_json(&out)
    })
}

/// `connectPackagedOwner`.
#[napi(js_name = "connectPackagedOwner")]
pub async fn connect_packaged_owner(cdp_port: f64) -> napi::Result<String> {
    run_async(async move {
        let out = port::connect_packaged_owner(cdp_port).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `connectOwnerUrl`.
#[napi(js_name = "connectOwnerUrl")]
pub async fn connect_owner_url(raw_url: String) -> napi::Result<String> {
    run_async(async move {
        let out = port::connect_owner_url(raw_url).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `assertPassiveComputerReady`.
#[napi(js_name = "assertPassiveComputerReady")]
pub fn assert_passive_computer_ready(status: String) -> napi::Result<()> {
    run(move || {
        let status = crate::js::from_json(&status)?;
        port::assert_passive_computer_ready(status)?;
        Ok(())
    })
}

/// `resolveFixtureComputerWindow`.
#[napi(js_name = "resolveFixtureComputerWindow")]
pub fn resolve_fixture_computer_window(windows: String, fixture: String) -> napi::Result<String> {
    run(move || {
        let windows = crate::js::from_json(&windows)?;
        let fixture = crate::js::from_json(&fixture)?;
        let out = port::resolve_fixture_computer_window(windows, fixture)?;
        crate::js::to_json(&out)
    })
}

/// `assessFixtureReportCoverage`.
#[napi(js_name = "assessFixtureReportCoverage")]
pub fn assess_fixture_report_coverage(
    reports: String,
    expected_completed_runs: f64,
) -> napi::Result<String> {
    run(move || {
        let reports = crate::js::from_json(&reports)?;
        let out = port::assess_fixture_report_coverage(reports, expected_completed_runs)?;
        crate::js::to_json(&out)
    })
}

/// `parseFixtureState`.
#[napi(js_name = "parseFixtureState")]
pub fn parse_fixture_state(value: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::parse_fixture_state(value)?;
        crate::js::to_json(&out)
    })
}

/// `verifyFixtureClick`.
#[napi(js_name = "verifyFixtureClick")]
pub fn verify_fixture_click(before: String, after: String) -> napi::Result<String> {
    run(move || {
        let before = crate::js::from_json(&before)?;
        let after = crate::js::from_json(&after)?;
        let out = port::verify_fixture_click(before, after)?;
        crate::js::to_json(&out)
    })
}

/// `fixtureUnchanged`.
#[napi(js_name = "fixtureUnchanged")]
pub fn fixture_unchanged(before: String, after: String) -> napi::Result<bool> {
    run(move || {
        let before = crate::js::from_json(&before)?;
        let after = crate::js::from_json(&after)?;
        let out = port::fixture_unchanged(before, after)?;
        Ok(out)
    })
}

/// `fixtureReceiptTiming`.
#[napi(js_name = "fixtureReceiptTiming")]
pub fn fixture_receipt_timing(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::fixture_receipt_timing(input)?;
        crate::js::to_json(&out)
    })
}

/// `assessContinuousFocus`.
#[napi(js_name = "assessContinuousFocus")]
pub fn assess_continuous_focus(
    result: String,
    expected: String,
    interval: String,
) -> napi::Result<String> {
    run(move || {
        let result = crate::js::from_json(&result)?;
        let expected = crate::js::from_json(&expected)?;
        let interval = crate::js::from_json(&interval)?;
        let out = port::assess_continuous_focus(result, expected, interval)?;
        crate::js::to_json(&out)
    })
}

/// `delay`.
#[napi(js_name = "delay")]
pub fn delay(ms: f64) -> napi::Result<String> {
    run(move || {
        let out = port::delay(ms)?;
        crate::js::to_json(&out)
    })
}

/// `startNativeTarget`.
#[napi(js_name = "startNativeTarget")]
pub async fn start_native_target(binary: String) -> napi::Result<String> {
    run_async(async move {
        let out = port::start_native_target(binary).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `parseLifetimeCpuSeconds`.
#[napi(js_name = "parseLifetimeCpuSeconds")]
pub fn parse_lifetime_cpu_seconds(value: String) -> napi::Result<f64> {
    run(move || {
        let out = port::parse_lifetime_cpu_seconds(value)?;
        Ok(out)
    })
}

/// `parseResourceSnapshot`.
#[napi(js_name = "parseResourceSnapshot")]
pub fn parse_resource_snapshot(output: String) -> napi::Result<String> {
    run(move || {
        let out = port::parse_resource_snapshot(output)?;
        crate::js::to_json(&out)
    })
}

/// `resourceIdentity`.
#[napi(js_name = "resourceIdentity")]
pub fn resource_identity(process: String) -> napi::Result<String> {
    run(move || {
        let process = crate::js::from_json(&process)?;
        let out = port::resource_identity(process)?;
        Ok(out)
    })
}

/// `makeResourceTracker`.
#[napi(js_name = "makeResourceTracker")]
pub fn make_resource_tracker(root: String) -> napi::Result<String> {
    run(move || {
        let root = crate::js::from_json(&root)?;
        let out = port::make_resource_tracker(root)?;
        crate::js::to_json(&out)
    })
}

/// `resourceSamplingOptions`.
#[napi(js_name = "resourceSamplingOptions")]
pub fn resource_sampling_options(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::resource_sampling_options(input)?;
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

static S_CANARY_OPEN_ARGS_0: wire::Schema = wire::Schema::Dyn;
static S_CANARY_OPEN_ARGS_1: wire::Schema = wire::Schema::Dyn;
static S_CANARY_OPEN_ARGS_OUT: wire::Schema = wire::Schema::Dyn;

/// `canaryOpenArgs` over the binary wire.
#[napi(js_name = "canaryOpenArgsWire")]
pub fn canary_open_args_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let app: serde_json::Value = wire::read_value(r, &S_CANARY_OPEN_ARGS_0)?;
        let env_assignment: serde_json::Value = wire::read_value(r, &S_CANARY_OPEN_ARGS_1)?;
        let out = &port::canary_open_args(app, env_assignment)?;
        wire::write_value(w, out, &S_CANARY_OPEN_ARGS_OUT)?;
        Ok(())
    })
}

static S_PARSE_BROWSER_BENCHMARK_RUN_COUNT_OUT: wire::Schema = wire::Schema::Num;

/// `parseBrowserBenchmarkRunCount` over the binary wire.
#[napi(js_name = "parseBrowserBenchmarkRunCountWire")]
pub fn parse_browser_benchmark_run_count_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::parse_browser_benchmark_run_count(value)?;
        wire::write_value(w, out, &S_PARSE_BROWSER_BENCHMARK_RUN_COUNT_OUT)?;
        Ok(())
    })
}

static S_ASSESS_BROWSER_BENCHMARK_RUNS_0: wire::Schema = wire::Schema::Dyn;
static S_ASSESS_BROWSER_BENCHMARK_RUNS_OUT: wire::Schema = wire::Schema::Dyn;

/// `assessBrowserBenchmarkRuns` over the binary wire.
#[napi(js_name = "assessBrowserBenchmarkRunsWire")]
pub fn assess_browser_benchmark_runs_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_ASSESS_BROWSER_BENCHMARK_RUNS_0)?;
        let out = &port::assess_browser_benchmark_runs(input)?;
        wire::write_value(w, out, &S_ASSESS_BROWSER_BENCHMARK_RUNS_OUT)?;
        Ok(())
    })
}

static S_PARSE_NEWEGG_PRICE_OUT: wire::Schema = wire::Schema::Dyn;

/// `parseNeweggPrice` over the binary wire.
#[napi(js_name = "parseNeweggPriceWire")]
pub fn parse_newegg_price_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: String = r.string16()?;
        let currency_hint: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::parse_newegg_price(text, currency_hint)?;
        wire::write_value(w, out, &S_PARSE_NEWEGG_PRICE_OUT)?;
        Ok(())
    })
}

static S_PARSE_NEWEGG_PAGE_EVIDENCE_0: wire::Schema = wire::Schema::Dyn;
static S_PARSE_NEWEGG_PAGE_EVIDENCE_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    (
        "product",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            ("itemId", wire::Schema::Str16),
            ("title", wire::Schema::Str16),
            ("breadcrumbs", wire::Schema::List(&wire::Schema::Str16)),
            (
                "facts",
                wire::Schema::List(&wire::Schema::Obj(&[
                    ("label", wire::Schema::Dyn),
                    ("value", wire::Schema::Dyn),
                ])),
            ),
        ])),
    ),
    (
        "cart",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            ("items", wire::Schema::List(&wire::Schema::Dyn)),
            ("itemCount", wire::Schema::OptJs(&wire::Schema::Num)),
            ("subtotalMinor", wire::Schema::OptJs(&wire::Schema::Num)),
            ("currency", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("rowCoverageComplete", wire::Schema::Bool),
        ])),
    ),
]));

/// `parseNeweggPageEvidence` over the binary wire.
#[napi(js_name = "parseNeweggPageEvidenceWire")]
pub fn parse_newegg_page_evidence_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value = wire::read_value(r, &S_PARSE_NEWEGG_PAGE_EVIDENCE_0)?;
        let out = &port::parse_newegg_page_evidence(value)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_PARSE_NEWEGG_PAGE_EVIDENCE_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_OBSERVED_NEWEGG_CATEGORY_0: wire::Schema = wire::Schema::Dyn;

/// `observedNeweggCategory` over the binary wire.
#[napi(js_name = "observedNeweggCategoryWire")]
pub fn observed_newegg_category_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let product: serde_json::Value = wire::read_value(r, &S_OBSERVED_NEWEGG_CATEGORY_0)?;
        let out = &port::observed_newegg_category(product)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_ASSESS_NEWEGG_COMPLETION_0: wire::Schema = wire::Schema::Dyn;
static S_ASSESS_NEWEGG_COMPLETION_OUT: wire::Schema = wire::Schema::Dyn;

/// `assessNeweggCompletion` over the binary wire.
#[napi(js_name = "assessNeweggCompletionWire")]
pub fn assess_newegg_completion_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_ASSESS_NEWEGG_COMPLETION_0)?;
        let out = &port::assess_newegg_completion(input)?;
        wire::write_value(w, out, &S_ASSESS_NEWEGG_COMPLETION_OUT)?;
        Ok(())
    })
}

static S_PARSE_BROWSER_WITNESS_0: wire::Schema = wire::Schema::Dyn;
static S_PARSE_BROWSER_WITNESS_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    ("page", wire::Schema::Num),
    ("path", wire::Schema::Str16),
    (
        "prs",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("number", wire::Schema::Num),
            ("title", wire::Schema::Str16),
        ])),
    ),
    ("cartVisible", wire::Schema::Bool),
    ("productPaths", wire::Schema::List(&wire::Schema::Str16)),
    ("checkoutVisible", wire::Schema::Bool),
    (
        "newegg",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            (
                "product",
                wire::Schema::OptJs(&wire::Schema::Obj(&[
                    ("itemId", wire::Schema::Dyn),
                    ("title", wire::Schema::Dyn),
                    ("breadcrumbs", wire::Schema::Dyn),
                    ("facts", wire::Schema::Dyn),
                ])),
            ),
            (
                "cart",
                wire::Schema::OptJs(&wire::Schema::Obj(&[
                    ("items", wire::Schema::Dyn),
                    ("itemCount", wire::Schema::Dyn),
                    ("subtotalMinor", wire::Schema::Dyn),
                    ("currency", wire::Schema::Dyn),
                    ("rowCoverageComplete", wire::Schema::Dyn),
                ])),
            ),
        ])),
    ),
]));

/// `parseBrowserWitness` over the binary wire.
#[napi(js_name = "parseBrowserWitnessWire")]
pub fn parse_browser_witness_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value = wire::read_value(r, &S_PARSE_BROWSER_WITNESS_0)?;
        let out = &port::parse_browser_witness(value)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_PARSE_BROWSER_WITNESS_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_PARSE_PULL_REQUEST_TITLES_0: wire::Schema = wire::Schema::Dyn;
static S_PARSE_PULL_REQUEST_TITLES_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Obj(&[
        ("number", wire::Schema::Num),
        ("title", wire::Schema::Str16),
    ])));

/// `parsePullRequestTitles` over the binary wire.
#[napi(js_name = "parsePullRequestTitlesWire")]
pub fn parse_pull_request_titles_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value = wire::read_value(r, &S_PARSE_PULL_REQUEST_TITLES_0)?;
        let out = &port::parse_pull_request_titles(value)?;
        match out {
            Some(x0) => {
                w.tag(1);
                w.tag(1);
                w.count(x0.len());
                for x1 in x0.iter() {
                    wire::write_value(w, x1, S_PARSE_PULL_REQUEST_TITLES_OUT.inner().inner())?;
                }
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_ASSESS_GIT_HUB_COMPLETION_0: wire::Schema = wire::Schema::Dyn;
static S_ASSESS_GIT_HUB_COMPLETION_OUT: wire::Schema = wire::Schema::Dyn;

/// `assessGitHubCompletion` over the binary wire.
#[napi(js_name = "assessGitHubCompletionWire")]
pub fn assess_git_hub_completion_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_ASSESS_GIT_HUB_COMPLETION_0)?;
        let out = &port::assess_git_hub_completion(input)?;
        wire::write_value(w, out, &S_ASSESS_GIT_HUB_COMPLETION_OUT)?;
        Ok(())
    })
}

static S_BROWSER_BENCHMARK_PROMPT_OUT: wire::Schema = wire::Schema::Dyn;

/// `browserBenchmarkPrompt` over the binary wire.
#[napi(js_name = "browserBenchmarkPromptWire")]
pub fn browser_benchmark_prompt_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let task: String = r.string16()?;
        let profile_name: String = r.string16()?;
        let repo: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::browser_benchmark_prompt(task, profile_name, repo)?;
        wire::write_value(w, out, &S_BROWSER_BENCHMARK_PROMPT_OUT)?;
        Ok(())
    })
}

/// `neweggItemIdFromUrl` over the binary wire.
#[napi(js_name = "neweggItemIdFromUrlWire")]
pub fn newegg_item_id_from_url_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: String = r.string16()?;
        let out = &port::newegg_item_id_from_url(value)?;
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

static S_PARSE_OBSERVER_DESCRIPTOR_0: wire::Schema = wire::Schema::Dyn;
static S_PARSE_OBSERVER_DESCRIPTOR_OUT: wire::Schema = wire::Schema::Obj(&[
    ("profileName", wire::Schema::Str16),
    ("profileDirectory", wire::Schema::Str16),
    ("browserPid", wire::Schema::Num),
    ("endpoint", wire::Schema::Str16),
]);

/// `parseObserverDescriptor` over the binary wire.
#[napi(js_name = "parseObserverDescriptorWire")]
pub fn parse_observer_descriptor_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value = wire::read_value(r, &S_PARSE_OBSERVER_DESCRIPTOR_0)?;
        let profile_name: String = r.string16()?;
        let out = &port::parse_observer_descriptor(value, profile_name)?;
        wire::write_value(w, out, &S_PARSE_OBSERVER_DESCRIPTOR_OUT)?;
        Ok(())
    })
}

static S_CREATE_BROWSER_OBSERVER_0: wire::Schema = wire::Schema::Dyn;
static S_CREATE_BROWSER_OBSERVER_OUT: wire::Schema = wire::Schema::Dyn;

/// `createBrowserObserver` over the binary wire.
#[napi(js_name = "createBrowserObserverWire")]
pub fn create_browser_observer_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_CREATE_BROWSER_OBSERVER_0)?;
        let out = &port::create_browser_observer(input)?;
        wire::write_value(w, out, &S_CREATE_BROWSER_OBSERVER_OUT)?;
        Ok(())
    })
}

static S_ASSERT_PACKAGED_APP_INSTALLATION_OUT: wire::Schema = wire::Schema::Dyn;

/// `assertPackagedAppInstallation` over the binary wire.
#[napi(js_name = "assertPackagedAppInstallationWire")]
pub fn assert_packaged_app_installation_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let bundle: String = r.string16()?;
        let registered_bundle: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::assert_packaged_app_installation(bundle, registered_bundle)?;
        wire::write_value(w, out, &S_ASSERT_PACKAGED_APP_INSTALLATION_OUT)?;
        Ok(())
    })
}

static S_ASSERT_LOOPBACK_URL_OUT: wire::Schema = wire::Schema::Dyn;

/// `assertLoopbackUrl` over the binary wire.
#[napi(js_name = "assertLoopbackUrlWire")]
pub fn assert_loopback_url_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: String = r.string16()?;
        let protocol: String = r.string16()?;
        let out = &port::assert_loopback_url(value, protocol)?;
        wire::write_value(w, out, &S_ASSERT_LOOPBACK_URL_OUT)?;
        Ok(())
    })
}

static S_ASSERT_PASSIVE_COMPUTER_READY_0: wire::Schema = wire::Schema::Dyn;

/// `assertPassiveComputerReady` over the binary wire.
#[napi(js_name = "assertPassiveComputerReadyWire")]
pub fn assert_passive_computer_ready_wire() -> napi::Result<f64> {
    wire_io(|r, _w| {
        let status: serde_json::Value = wire::read_value(r, &S_ASSERT_PASSIVE_COMPUTER_READY_0)?;
        let () = port::assert_passive_computer_ready(status)?;
        Ok(())
    })
}

static S_RESOLVE_FIXTURE_COMPUTER_WINDOW_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_FIXTURE_COMPUTER_WINDOW_1: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_FIXTURE_COMPUTER_WINDOW_OUT: wire::Schema = wire::Schema::Dyn;

/// `resolveFixtureComputerWindow` over the binary wire.
#[napi(js_name = "resolveFixtureComputerWindowWire")]
pub fn resolve_fixture_computer_window_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let windows: serde_json::Value = wire::read_value(r, &S_RESOLVE_FIXTURE_COMPUTER_WINDOW_0)?;
        let fixture: serde_json::Value = wire::read_value(r, &S_RESOLVE_FIXTURE_COMPUTER_WINDOW_1)?;
        let out = &port::resolve_fixture_computer_window(windows, fixture)?;
        wire::write_value(w, out, &S_RESOLVE_FIXTURE_COMPUTER_WINDOW_OUT)?;
        Ok(())
    })
}

static S_ASSESS_FIXTURE_REPORT_COVERAGE_0: wire::Schema = wire::Schema::Dyn;
static S_ASSESS_FIXTURE_REPORT_COVERAGE_OUT: wire::Schema = wire::Schema::Dyn;

/// `assessFixtureReportCoverage` over the binary wire.
#[napi(js_name = "assessFixtureReportCoverageWire")]
pub fn assess_fixture_report_coverage_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let reports: serde_json::Value = wire::read_value(r, &S_ASSESS_FIXTURE_REPORT_COVERAGE_0)?;
        let expected_completed_runs: f64 = r.f64()?;
        let out = &port::assess_fixture_report_coverage(reports, expected_completed_runs)?;
        wire::write_value(w, out, &S_ASSESS_FIXTURE_REPORT_COVERAGE_OUT)?;
        Ok(())
    })
}

static S_PARSE_FIXTURE_STATE_0: wire::Schema = wire::Schema::Dyn;
static S_PARSE_FIXTURE_STATE_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    ("pid", wire::Schema::Num),
    ("windowId", wire::Schema::Num),
    ("title", wire::Schema::Str16),
    ("label", wire::Schema::Str16),
    ("clicks", wire::Schema::Num),
    ("edits", wire::Schema::Num),
    ("text", wire::Schema::Str16),
]));

/// `parseFixtureState` over the binary wire.
#[napi(js_name = "parseFixtureStateWire")]
pub fn parse_fixture_state_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value = wire::read_value(r, &S_PARSE_FIXTURE_STATE_0)?;
        let out = &port::parse_fixture_state(value)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_PARSE_FIXTURE_STATE_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_VERIFY_FIXTURE_CLICK_0: wire::Schema = wire::Schema::Dyn;
static S_VERIFY_FIXTURE_CLICK_1: wire::Schema = wire::Schema::Dyn;
static S_VERIFY_FIXTURE_CLICK_OUT: wire::Schema = wire::Schema::Dyn;

/// `verifyFixtureClick` over the binary wire.
#[napi(js_name = "verifyFixtureClickWire")]
pub fn verify_fixture_click_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let before: serde_json::Value = wire::read_value(r, &S_VERIFY_FIXTURE_CLICK_0)?;
        let after: serde_json::Value = wire::read_value(r, &S_VERIFY_FIXTURE_CLICK_1)?;
        let out = &port::verify_fixture_click(before, after)?;
        wire::write_value(w, out, &S_VERIFY_FIXTURE_CLICK_OUT)?;
        Ok(())
    })
}

static S_FIXTURE_UNCHANGED_0: wire::Schema = wire::Schema::Dyn;
static S_FIXTURE_UNCHANGED_1: wire::Schema = wire::Schema::Dyn;

/// `fixtureUnchanged` over the binary wire.
#[napi(js_name = "fixtureUnchangedWire")]
pub fn fixture_unchanged_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let before: serde_json::Value = wire::read_value(r, &S_FIXTURE_UNCHANGED_0)?;
        let after: serde_json::Value = wire::read_value(r, &S_FIXTURE_UNCHANGED_1)?;
        let out = &port::fixture_unchanged(before, after)?;
        w.bool(*out);
        Ok(())
    })
}

static S_FIXTURE_RECEIPT_TIMING_0: wire::Schema = wire::Schema::Dyn;
static S_FIXTURE_RECEIPT_TIMING_OUT: wire::Schema = wire::Schema::Dyn;

/// `fixtureReceiptTiming` over the binary wire.
#[napi(js_name = "fixtureReceiptTimingWire")]
pub fn fixture_receipt_timing_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_FIXTURE_RECEIPT_TIMING_0)?;
        let out = &port::fixture_receipt_timing(input)?;
        wire::write_value(w, out, &S_FIXTURE_RECEIPT_TIMING_OUT)?;
        Ok(())
    })
}

static S_ASSESS_CONTINUOUS_FOCUS_0: wire::Schema = wire::Schema::Dyn;
static S_ASSESS_CONTINUOUS_FOCUS_1: wire::Schema = wire::Schema::Dyn;
static S_ASSESS_CONTINUOUS_FOCUS_2: wire::Schema = wire::Schema::Dyn;
static S_ASSESS_CONTINUOUS_FOCUS_OUT: wire::Schema = wire::Schema::Dyn;

/// `assessContinuousFocus` over the binary wire.
#[napi(js_name = "assessContinuousFocusWire")]
pub fn assess_continuous_focus_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let result: serde_json::Value = wire::read_value(r, &S_ASSESS_CONTINUOUS_FOCUS_0)?;
        let expected: serde_json::Value = wire::read_value(r, &S_ASSESS_CONTINUOUS_FOCUS_1)?;
        let interval: serde_json::Value = wire::read_value(r, &S_ASSESS_CONTINUOUS_FOCUS_2)?;
        let out = &port::assess_continuous_focus(result, expected, interval)?;
        wire::write_value(w, out, &S_ASSESS_CONTINUOUS_FOCUS_OUT)?;
        Ok(())
    })
}

static S_DELAY_OUT: wire::Schema = wire::Schema::Dyn;

/// `delay` over the binary wire.
#[napi(js_name = "delayWire")]
pub fn delay_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let ms: f64 = r.f64()?;
        let out = &port::delay(ms)?;
        wire::write_value(w, out, &S_DELAY_OUT)?;
        Ok(())
    })
}

/// `parseLifetimeCpuSeconds` over the binary wire.
#[napi(js_name = "parseLifetimeCpuSecondsWire")]
pub fn parse_lifetime_cpu_seconds_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: String = r.string16()?;
        let out = &port::parse_lifetime_cpu_seconds(value)?;
        w.f64(*out);
        Ok(())
    })
}

static S_PARSE_RESOURCE_SNAPSHOT_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[
    ("pid", wire::Schema::Num),
    ("parentPid", wire::Schema::Num),
    ("startedAt", wire::Schema::Str16),
    ("executable", wire::Schema::Str16),
    ("rssKiB", wire::Schema::Num),
    ("cpuPercent", wire::Schema::Num),
    ("lifetimeCpuSeconds", wire::Schema::Num),
]));

/// `parseResourceSnapshot` over the binary wire.
#[napi(js_name = "parseResourceSnapshotWire")]
pub fn parse_resource_snapshot_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let output: String = r.string16()?;
        let out = &port::parse_resource_snapshot(output)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_PARSE_RESOURCE_SNAPSHOT_OUT.inner())?;
        }
        Ok(())
    })
}

static S_RESOURCE_IDENTITY_0: wire::Schema = wire::Schema::Dyn;

/// `resourceIdentity` over the binary wire.
#[napi(js_name = "resourceIdentityWire")]
pub fn resource_identity_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let process: serde_json::Value = wire::read_value(r, &S_RESOURCE_IDENTITY_0)?;
        let out = &port::resource_identity(process)?;
        w.string16(out);
        Ok(())
    })
}

static S_MAKE_RESOURCE_TRACKER_0: wire::Schema = wire::Schema::Dyn;
static S_MAKE_RESOURCE_TRACKER_OUT: wire::Schema = wire::Schema::Dyn;

/// `makeResourceTracker` over the binary wire.
#[napi(js_name = "makeResourceTrackerWire")]
pub fn make_resource_tracker_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let root: serde_json::Value = wire::read_value(r, &S_MAKE_RESOURCE_TRACKER_0)?;
        let out = &port::make_resource_tracker(root)?;
        wire::write_value(w, out, &S_MAKE_RESOURCE_TRACKER_OUT)?;
        Ok(())
    })
}

static S_RESOURCE_SAMPLING_OPTIONS_0: wire::Schema = wire::Schema::Dyn;
static S_RESOURCE_SAMPLING_OPTIONS_OUT: wire::Schema = wire::Schema::Obj(&[
    ("rootPid", wire::Schema::Num),
    ("durationMs", wire::Schema::Num),
    ("intervalMs", wire::Schema::Num),
]);

/// `resourceSamplingOptions` over the binary wire.
#[napi(js_name = "resourceSamplingOptionsWire")]
pub fn resource_sampling_options_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_RESOURCE_SAMPLING_OPTIONS_0)?;
        let out = &port::resource_sampling_options(input)?;
        wire::write_value(w, out, &S_RESOURCE_SAMPLING_OPTIONS_OUT)?;
        Ok(())
    })
}
