//! JS bindings for `benchmarks/desktop-perf/lib/`: generated from the signatures in `port.rs` by rust-anything.
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

/// `useTemplateSet`.
#[napi(js_name = "useTemplateSet")]
pub fn use_template_set(name: String) -> napi::Result<String> {
    run(move || {
        let name = crate::js::from_json(&name)?;
        let out = port::use_template_set(name)?;
        crate::js::to_json(&out)
    })
}

/// `currentTemplateSet`.
#[napi(js_name = "currentTemplateSet")]
pub fn current_template_set() -> napi::Result<String> {
    run(move || {
        let out = port::current_template_set()?;
        crate::js::to_json(&out)
    })
}

/// `buildName`.
#[napi(js_name = "buildName")]
pub fn build_name(r#ref: String) -> napi::Result<String> {
    run(move || {
        let r#ref = crate::js::from_json(&r#ref)?;
        let out = port::build_name(r#ref)?;
        crate::js::to_json(&out)
    })
}

/// `buildInfo`.
#[napi(js_name = "buildInfo")]
pub fn build_info(name: String) -> napi::Result<String> {
    run(move || {
        let name = crate::js::from_json(&name)?;
        let out = port::build_info(name)?;
        crate::js::to_json(&out)
    })
}

/// `prepareBuild`.
#[napi(js_name = "prepareBuild")]
pub fn prepare_build(r#ref: String, name: String, arg2: String) -> napi::Result<String> {
    run(move || {
        let r#ref = crate::js::from_json(&r#ref)?;
        let name = crate::js::from_json(&name)?;
        let arg2 = crate::js::from_json(&arg2)?;
        let out = port::prepare_build(r#ref, name, arg2)?;
        crate::js::to_json(&out)
    })
}

/// `templatePaths`.
#[napi(js_name = "templatePaths")]
pub fn template_paths(kind: String) -> napi::Result<String> {
    run(move || {
        let kind = crate::js::from_json(&kind)?;
        let out = port::template_paths(kind)?;
        crate::js::to_json(&out)
    })
}

/// `templateInfo`.
#[napi(js_name = "templateInfo")]
pub fn template_info(kind: String) -> napi::Result<String> {
    run(move || {
        let kind = crate::js::from_json(&kind)?;
        let out = port::template_info(kind)?;
        crate::js::to_json(&out)
    })
}

/// `freshHomeFromTemplate`.
#[napi(js_name = "freshHomeFromTemplate")]
pub fn fresh_home_from_template(kind: String) -> napi::Result<String> {
    run(move || {
        let kind = crate::js::from_json(&kind)?;
        let out = port::fresh_home_from_template(kind)?;
        crate::js::to_json(&out)
    })
}

/// `removeHome`.
#[napi(js_name = "removeHome")]
pub fn remove_home(home: String) -> napi::Result<String> {
    run(move || {
        let home = crate::js::from_json(&home)?;
        let out = port::remove_home(home)?;
        crate::js::to_json(&out)
    })
}

/// `clearChromiumCaches`.
#[napi(js_name = "clearChromiumCaches")]
pub fn clear_chromium_caches(profile: String) -> napi::Result<String> {
    run(move || {
        let profile = crate::js::from_json(&profile)?;
        let out = port::clear_chromium_caches(profile)?;
        crate::js::to_json(&out)
    })
}

/// `seed`.
#[napi(js_name = "seed")]
pub fn seed(build: String, home: String, shape: String, arg3: String) -> napi::Result<String> {
    run(move || {
        let build = crate::js::from_json(&build)?;
        let home = crate::js::from_json(&home)?;
        let shape = crate::js::from_json(&shape)?;
        let arg3 = crate::js::from_json(&arg3)?;
        let out = port::seed(build, home, shape, arg3)?;
        crate::js::to_json(&out)
    })
}

/// `createTemplates`.
#[napi(js_name = "createTemplates")]
pub async fn create_templates(build: String, arg1: String) -> napi::Result<String> {
    run_async(async move {
        let build = crate::js::from_json(&build)?;
        let arg1 = crate::js::from_json(&arg1)?;
        let out = port::create_templates(build, arg1).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `fetchJson`.
#[napi(js_name = "fetchJson")]
pub async fn fetch_json(url: String, timeout_ms: Option<f64>) -> napi::Result<String> {
    run_async(async move {
        let url = crate::js::from_json(&url)?;
        let out = port::fetch_json(url, timeout_ms).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `listTargets`.
#[napi(js_name = "listTargets")]
pub async fn list_targets(port: String) -> napi::Result<String> {
    run_async(async move {
        let port = crate::js::from_json(&port)?;
        let out = port::list_targets(port).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `waitFor`.
#[napi(js_name = "waitFor")]
pub async fn wait_for(check: String, arg1: String) -> napi::Result<String> {
    run_async(async move {
        let check = crate::js::from_json(&check)?;
        let arg1 = crate::js::from_json(&arg1)?;
        let out = port::wait_for(check, arg1).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `sleep`.
#[napi(js_name = "sleep")]
pub fn sleep(ms: String) -> napi::Result<String> {
    run(move || {
        let ms = crate::js::from_json(&ms)?;
        let out = port::sleep(ms)?;
        crate::js::to_json(&out)
    })
}

/// `createProfileSummarizer`.
#[napi(js_name = "createProfileSummarizer")]
pub fn create_profile_summarizer(build_dir: String) -> napi::Result<String> {
    run(move || {
        let build_dir = crate::js::from_json(&build_dir)?;
        let out = port::create_profile_summarizer(build_dir)?;
        crate::js::to_json(&out)
    })
}

/// `elementCenter`.
#[napi(js_name = "elementCenter")]
pub async fn element_center(session: String, selector: String) -> napi::Result<String> {
    run_async(async move {
        let session = crate::js::from_json(&session)?;
        let selector = crate::js::from_json(&selector)?;
        let out = port::element_center(session, selector).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `clickAt`.
#[napi(js_name = "clickAt")]
pub async fn click_at(session: String, arg1: String) -> napi::Result<String> {
    run_async(async move {
        let session = crate::js::from_json(&session)?;
        let arg1 = crate::js::from_json(&arg1)?;
        let out = port::click_at(session, arg1).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `clickSelector`.
#[napi(js_name = "clickSelector")]
pub async fn click_selector(
    session: String,
    selector: String,
    arg2: String,
) -> napi::Result<String> {
    run_async(async move {
        let session = crate::js::from_json(&session)?;
        let selector = crate::js::from_json(&selector)?;
        let arg2 = crate::js::from_json(&arg2)?;
        let out = port::click_selector(session, selector, arg2).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `clickLabel`.
#[napi(js_name = "clickLabel")]
pub async fn click_label(session: String, labels: String) -> napi::Result<String> {
    run_async(async move {
        let session = crate::js::from_json(&session)?;
        let labels = crate::js::from_json(&labels)?;
        let out = port::click_label(session, labels).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `dismissAnnouncements`.
#[napi(js_name = "dismissAnnouncements")]
pub async fn dismiss_announcements(session: String, arg1: String) -> napi::Result<String> {
    run_async(async move {
        let session = crate::js::from_json(&session)?;
        let arg1 = crate::js::from_json(&arg1)?;
        let out = port::dismiss_announcements(session, arg1).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `focusEmulation`.
#[napi(js_name = "focusEmulation")]
pub async fn focus_emulation(session: String, enabled: Option<bool>) -> napi::Result<String> {
    run_async(async move {
        let session = crate::js::from_json(&session)?;
        let out = port::focus_emulation(session, enabled).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `pressKey`.
#[napi(js_name = "pressKey")]
pub async fn press_key(
    session: String,
    key: String,
    modifiers: Option<f64>,
) -> napi::Result<String> {
    run_async(async move {
        let session = crate::js::from_json(&session)?;
        let key = crate::js::from_json(&key)?;
        let out = port::press_key(session, key, modifiers).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `typeCharacter`.
#[napi(js_name = "typeCharacter")]
pub async fn type_character(session: String, character: String) -> napi::Result<String> {
    run_async(async move {
        let session = crate::js::from_json(&session)?;
        let character = crate::js::from_json(&character)?;
        let out = port::type_character(session, character).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `wheel`.
#[napi(js_name = "wheel")]
pub async fn wheel(session: String, arg1: String) -> napi::Result<String> {
    run_async(async move {
        let session = crate::js::from_json(&session)?;
        let arg1 = crate::js::from_json(&arg1)?;
        let out = port::wheel(session, arg1).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `composerText`.
#[napi(js_name = "composerText")]
pub async fn composer_text(session: String) -> napi::Result<String> {
    run_async(async move {
        let session = crate::js::from_json(&session)?;
        let out = port::composer_text(session).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `screenshot`.
#[napi(js_name = "screenshot")]
pub async fn screenshot(session: String, path: String) -> napi::Result<String> {
    run_async(async move {
        let session = crate::js::from_json(&session)?;
        let path = crate::js::from_json(&path)?;
        let out = port::screenshot(session, path).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `openThreadFromSidebar`.
#[napi(js_name = "openThreadFromSidebar")]
pub async fn open_thread_from_sidebar(
    session: String,
    thread_id: String,
    arg2: String,
) -> napi::Result<String> {
    run_async(async move {
        let session = crate::js::from_json(&session)?;
        let thread_id = crate::js::from_json(&thread_id)?;
        let arg2 = crate::js::from_json(&arg2)?;
        let out = port::open_thread_from_sidebar(session, thread_id, arg2).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `bootHeadlessBackend`.
#[napi(js_name = "bootHeadlessBackend")]
pub async fn boot_headless_backend(
    build: String,
    home: String,
    arg2: String,
) -> napi::Result<String> {
    run_async(async move {
        let build = crate::js::from_json(&build)?;
        let home = crate::js::from_json(&home)?;
        let arg2 = crate::js::from_json(&arg2)?;
        let out = port::boot_headless_backend(build, home, arg2).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `assertIsolatedHome`.
#[napi(js_name = "assertIsolatedHome")]
pub fn assert_isolated_home(home: String) -> napi::Result<String> {
    run(move || {
        let home = crate::js::from_json(&home)?;
        let out = port::assert_isolated_home(home)?;
        crate::js::to_json(&out)
    })
}

/// `resolveElectronBinary`.
#[napi(js_name = "resolveElectronBinary")]
pub fn resolve_electron_binary(build_dir: String) -> napi::Result<String> {
    run(move || {
        let build_dir = crate::js::from_json(&build_dir)?;
        let out = port::resolve_electron_binary(build_dir)?;
        crate::js::to_json(&out)
    })
}

/// `freePort`.
#[napi(js_name = "freePort")]
pub async fn free_port() -> napi::Result<String> {
    run_async(async move {
        let out = port::free_port().await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `isolatedEnvironment`.
#[napi(js_name = "isolatedEnvironment")]
pub fn isolated_environment(arg0: String) -> napi::Result<String> {
    run(move || {
        let arg0 = crate::js::from_json(&arg0)?;
        let out = port::isolated_environment(arg0)?;
        crate::js::to_json(&out)
    })
}

/// `processTree`.
#[napi(js_name = "processTree")]
pub fn process_tree(root_pid: String) -> napi::Result<String> {
    run(move || {
        let root_pid = crate::js::from_json(&root_pid)?;
        let out = port::process_tree(root_pid)?;
        crate::js::to_json(&out)
    })
}

/// `classifyProcess`.
#[napi(js_name = "classifyProcess")]
pub fn classify_process(row: String, arg1: String) -> napi::Result<String> {
    run(move || {
        let row = crate::js::from_json(&row)?;
        let arg1 = crate::js::from_json(&arg1)?;
        let out = port::classify_process(row, arg1)?;
        crate::js::to_json(&out)
    })
}

/// `launchDesktop`.
#[napi(js_name = "launchDesktop")]
pub fn launch_desktop(arg0: String) -> napi::Result<String> {
    run(move || {
        let arg0 = crate::js::from_json(&arg0)?;
        let out = port::launch_desktop(arg0)?;
        crate::js::to_json(&out)
    })
}

/// `attachMainPage`.
#[napi(js_name = "attachMainPage")]
pub async fn attach_main_page(app: String, arg1: String) -> napi::Result<String> {
    run_async(async move {
        let app = crate::js::from_json(&app)?;
        let arg1 = crate::js::from_json(&arg1)?;
        let out = port::attach_main_page(app, arg1).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `quitDesktop`.
#[napi(js_name = "quitDesktop")]
pub async fn quit_desktop(app: String, arg1: String) -> napi::Result<String> {
    run_async(async move {
        let app = crate::js::from_json(&app)?;
        let arg1 = crate::js::from_json(&arg1)?;
        let out = port::quit_desktop(app, arg1).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `isFrontmost`.
#[napi(js_name = "isFrontmost")]
pub fn is_frontmost(app: String) -> napi::Result<String> {
    run(move || {
        let app = crate::js::from_json(&app)?;
        let out = port::is_frontmost(app)?;
        crate::js::to_json(&out)
    })
}

/// `secondsSinceUserInput`.
#[napi(js_name = "secondsSinceUserInput")]
pub fn seconds_since_user_input() -> napi::Result<String> {
    run(move || {
        let out = port::seconds_since_user_input()?;
        crate::js::to_json(&out)
    })
}

/// `waitForQuietMac`.
#[napi(js_name = "waitForQuietMac")]
pub async fn wait_for_quiet_mac(arg0: String) -> napi::Result<String> {
    run_async(async move {
        let arg0 = crate::js::from_json(&arg0)?;
        let out = port::wait_for_quiet_mac(arg0).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `screenLocked`.
#[napi(js_name = "screenLocked")]
pub fn screen_locked() -> napi::Result<String> {
    run(move || {
        let out = port::screen_locked()?;
        crate::js::to_json(&out)
    })
}

/// `frontmostProblem`.
#[napi(js_name = "frontmostProblem")]
pub fn frontmost_problem(app: String, when: String) -> napi::Result<String> {
    run(move || {
        let app = crate::js::from_json(&app)?;
        let when = crate::js::from_json(&when)?;
        let out = port::frontmost_problem(app, when)?;
        crate::js::to_json(&out)
    })
}

/// `footprintMb`.
#[napi(js_name = "footprintMb")]
pub fn footprint_mb(pids: String) -> napi::Result<String> {
    run(move || {
        let pids = crate::js::from_json(&pids)?;
        let out = port::footprint_mb(pids)?;
        crate::js::to_json(&out)
    })
}

/// `memorySnapshot`.
#[napi(js_name = "memorySnapshot")]
pub fn memory_snapshot(app: String) -> napi::Result<String> {
    run(move || {
        let app = crate::js::from_json(&app)?;
        let out = port::memory_snapshot(app)?;
        crate::js::to_json(&out)
    })
}

/// `rssTotalMb`.
#[napi(js_name = "rssTotalMb")]
pub fn rss_total_mb(app: String) -> napi::Result<String> {
    run(move || {
        let app = crate::js::from_json(&app)?;
        let out = port::rss_total_mb(app)?;
        crate::js::to_json(&out)
    })
}

/// `cpuOverWindow`.
#[napi(js_name = "cpuOverWindow")]
pub async fn cpu_over_window(app: String, seconds: String) -> napi::Result<String> {
    run_async(async move {
        let app = crate::js::from_json(&app)?;
        let seconds = crate::js::from_json(&seconds)?;
        let out = port::cpu_over_window(app, seconds).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `wakeupsOverWindow`.
#[napi(js_name = "wakeupsOverWindow")]
pub fn wakeups_over_window(app: String, seconds: String) -> napi::Result<String> {
    run(move || {
        let app = crate::js::from_json(&app)?;
        let seconds = crate::js::from_json(&seconds)?;
        let out = port::wakeups_over_window(app, seconds)?;
        crate::js::to_json(&out)
    })
}

/// `gpuSystemSample`.
#[napi(js_name = "gpuSystemSample")]
pub fn gpu_system_sample() -> napi::Result<String> {
    run(move || {
        let out = port::gpu_system_sample()?;
        crate::js::to_json(&out)
    })
}

/// `gpuSystemOverWindow`.
#[napi(js_name = "gpuSystemOverWindow")]
pub async fn gpu_system_over_window(seconds: String) -> napi::Result<String> {
    run_async(async move {
        let seconds = crate::js::from_json(&seconds)?;
        let out = port::gpu_system_over_window(seconds).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `machineState`.
#[napi(js_name = "machineState")]
pub fn machine_state() -> napi::Result<String> {
    run(move || {
        let out = port::machine_state()?;
        crate::js::to_json(&out)
    })
}

/// `machineIdentity`.
#[napi(js_name = "machineIdentity")]
pub fn machine_identity() -> napi::Result<String> {
    run(move || {
        let out = port::machine_identity()?;
        crate::js::to_json(&out)
    })
}

/// `median`.
#[napi(js_name = "median")]
pub fn median(values: String) -> napi::Result<String> {
    run(move || {
        let values = crate::js::from_json(&values)?;
        let out = port::median(values)?;
        crate::js::to_json(&out)
    })
}

/// `percentile`.
#[napi(js_name = "percentile")]
pub fn percentile(values: String, fraction: String) -> napi::Result<String> {
    run(move || {
        let values = crate::js::from_json(&values)?;
        let fraction = crate::js::from_json(&fraction)?;
        let out = port::percentile(values, fraction)?;
        crate::js::to_json(&out)
    })
}

/// `summarize`.
#[napi(js_name = "summarize")]
pub fn summarize(samples: String, unit: String) -> napi::Result<String> {
    run(move || {
        let samples = crate::js::from_json(&samples)?;
        let unit = crate::js::from_json(&unit)?;
        let out = port::summarize(samples, unit)?;
        crate::js::to_json(&out)
    })
}

/// `gap`.
#[napi(js_name = "gap")]
pub fn gap(result: String, cause: String) -> napi::Result<String> {
    run(move || {
        let result = crate::js::from_json(&result)?;
        let cause = crate::js::from_json(&cause)?;
        let out = port::gap(result, cause)?;
        crate::js::to_json(&out)
    })
}

/// `startApp`.
#[napi(js_name = "startApp")]
pub async fn start_app(arg0: String) -> napi::Result<String> {
    run_async(async move {
        let arg0 = crate::js::from_json(&arg0)?;
        let out = port::start_app(arg0).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `windowBounds`.
#[napi(js_name = "windowBounds")]
pub async fn window_bounds(session: String) -> napi::Result<String> {
    run_async(async move {
        let session = crate::js::from_json(&session)?;
        let out = port::window_bounds(session).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `measureIdle`.
#[napi(js_name = "measureIdle")]
pub async fn measure_idle(arg0: String) -> napi::Result<String> {
    run_async(async move {
        let arg0 = crate::js::from_json(&arg0)?;
        let out = port::measure_idle(arg0).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `measureSwitching`.
#[napi(js_name = "measureSwitching")]
pub async fn measure_switching(arg0: String) -> napi::Result<String> {
    run_async(async move {
        let arg0 = crate::js::from_json(&arg0)?;
        let out = port::measure_switching(arg0).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `measureLongChat`.
#[napi(js_name = "measureLongChat")]
pub async fn measure_long_chat(arg0: String) -> napi::Result<String> {
    run_async(async move {
        let arg0 = crate::js::from_json(&arg0)?;
        let out = port::measure_long_chat(arg0).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `measureLoad`.
#[napi(js_name = "measureLoad")]
pub async fn measure_load(arg0: String) -> napi::Result<String> {
    run_async(async move {
        let arg0 = crate::js::from_json(&arg0)?;
        let out = port::measure_load(arg0).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `measureLaunch`.
#[napi(js_name = "measureLaunch")]
pub async fn measure_launch(arg0: String) -> napi::Result<String> {
    run_async(async move {
        let arg0 = crate::js::from_json(&arg0)?;
        let out = port::measure_launch(arg0).await?;
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

static S_USE_TEMPLATE_SET_0: wire::Schema = wire::Schema::Dyn;
static S_USE_TEMPLATE_SET_OUT: wire::Schema = wire::Schema::Dyn;

/// `useTemplateSet` over the binary wire.
#[napi(js_name = "useTemplateSetWire")]
pub fn use_template_set_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let name: Value = wire::read_value(r, &S_USE_TEMPLATE_SET_0)?;
        let out = &port::use_template_set(name)?;
        wire::write_value(w, out, &S_USE_TEMPLATE_SET_OUT)?;
        Ok(())
    })
}

static S_CURRENT_TEMPLATE_SET_OUT: wire::Schema = wire::Schema::Dyn;

/// `currentTemplateSet` over the binary wire.
#[napi(js_name = "currentTemplateSetWire")]
pub fn current_template_set_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::current_template_set()?;
        wire::write_value(w, out, &S_CURRENT_TEMPLATE_SET_OUT)?;
        Ok(())
    })
}

static S_BUILD_NAME_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_NAME_OUT: wire::Schema = wire::Schema::Dyn;

/// `buildName` over the binary wire.
#[napi(js_name = "buildNameWire")]
pub fn build_name_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let r#ref: Value = wire::read_value(r, &S_BUILD_NAME_0)?;
        let out = &port::build_name(r#ref)?;
        wire::write_value(w, out, &S_BUILD_NAME_OUT)?;
        Ok(())
    })
}

static S_BUILD_INFO_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_INFO_OUT: wire::Schema = wire::Schema::Dyn;

/// `buildInfo` over the binary wire.
#[napi(js_name = "buildInfoWire")]
pub fn build_info_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let name: Value = wire::read_value(r, &S_BUILD_INFO_0)?;
        let out = &port::build_info(name)?;
        wire::write_value(w, out, &S_BUILD_INFO_OUT)?;
        Ok(())
    })
}

static S_PREPARE_BUILD_0: wire::Schema = wire::Schema::Dyn;
static S_PREPARE_BUILD_1: wire::Schema = wire::Schema::Dyn;
static S_PREPARE_BUILD_2: wire::Schema = wire::Schema::Dyn;
static S_PREPARE_BUILD_OUT: wire::Schema = wire::Schema::Dyn;

/// `prepareBuild` over the binary wire.
#[napi(js_name = "prepareBuildWire")]
pub fn prepare_build_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let r#ref: Value = wire::read_value(r, &S_PREPARE_BUILD_0)?;
        let name: Value = wire::read_value(r, &S_PREPARE_BUILD_1)?;
        let arg2: Value = wire::read_value(r, &S_PREPARE_BUILD_2)?;
        let out = &port::prepare_build(r#ref, name, arg2)?;
        wire::write_value(w, out, &S_PREPARE_BUILD_OUT)?;
        Ok(())
    })
}

static S_TEMPLATE_PATHS_0: wire::Schema = wire::Schema::Dyn;
static S_TEMPLATE_PATHS_OUT: wire::Schema = wire::Schema::Dyn;

/// `templatePaths` over the binary wire.
#[napi(js_name = "templatePathsWire")]
pub fn template_paths_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let kind: Value = wire::read_value(r, &S_TEMPLATE_PATHS_0)?;
        let out = &port::template_paths(kind)?;
        wire::write_value(w, out, &S_TEMPLATE_PATHS_OUT)?;
        Ok(())
    })
}

static S_TEMPLATE_INFO_0: wire::Schema = wire::Schema::Dyn;
static S_TEMPLATE_INFO_OUT: wire::Schema = wire::Schema::Dyn;

/// `templateInfo` over the binary wire.
#[napi(js_name = "templateInfoWire")]
pub fn template_info_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let kind: Value = wire::read_value(r, &S_TEMPLATE_INFO_0)?;
        let out = &port::template_info(kind)?;
        wire::write_value(w, out, &S_TEMPLATE_INFO_OUT)?;
        Ok(())
    })
}

static S_FRESH_HOME_FROM_TEMPLATE_0: wire::Schema = wire::Schema::Dyn;
static S_FRESH_HOME_FROM_TEMPLATE_OUT: wire::Schema = wire::Schema::Dyn;

/// `freshHomeFromTemplate` over the binary wire.
#[napi(js_name = "freshHomeFromTemplateWire")]
pub fn fresh_home_from_template_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let kind: Value = wire::read_value(r, &S_FRESH_HOME_FROM_TEMPLATE_0)?;
        let out = &port::fresh_home_from_template(kind)?;
        wire::write_value(w, out, &S_FRESH_HOME_FROM_TEMPLATE_OUT)?;
        Ok(())
    })
}

static S_REMOVE_HOME_0: wire::Schema = wire::Schema::Dyn;
static S_REMOVE_HOME_OUT: wire::Schema = wire::Schema::Dyn;

/// `removeHome` over the binary wire.
#[napi(js_name = "removeHomeWire")]
pub fn remove_home_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let home: Value = wire::read_value(r, &S_REMOVE_HOME_0)?;
        let out = &port::remove_home(home)?;
        wire::write_value(w, out, &S_REMOVE_HOME_OUT)?;
        Ok(())
    })
}

static S_CLEAR_CHROMIUM_CACHES_0: wire::Schema = wire::Schema::Dyn;
static S_CLEAR_CHROMIUM_CACHES_OUT: wire::Schema = wire::Schema::Dyn;

/// `clearChromiumCaches` over the binary wire.
#[napi(js_name = "clearChromiumCachesWire")]
pub fn clear_chromium_caches_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let profile: Value = wire::read_value(r, &S_CLEAR_CHROMIUM_CACHES_0)?;
        let out = &port::clear_chromium_caches(profile)?;
        wire::write_value(w, out, &S_CLEAR_CHROMIUM_CACHES_OUT)?;
        Ok(())
    })
}

static S_SEED_0: wire::Schema = wire::Schema::Dyn;
static S_SEED_1: wire::Schema = wire::Schema::Dyn;
static S_SEED_2: wire::Schema = wire::Schema::Dyn;
static S_SEED_3: wire::Schema = wire::Schema::Dyn;
static S_SEED_OUT: wire::Schema = wire::Schema::Dyn;

/// `seed` over the binary wire.
#[napi(js_name = "seedWire")]
pub fn seed_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let build: Value = wire::read_value(r, &S_SEED_0)?;
        let home: Value = wire::read_value(r, &S_SEED_1)?;
        let shape: Value = wire::read_value(r, &S_SEED_2)?;
        let arg3: Value = wire::read_value(r, &S_SEED_3)?;
        let out = &port::seed(build, home, shape, arg3)?;
        wire::write_value(w, out, &S_SEED_OUT)?;
        Ok(())
    })
}

static S_SLEEP_0: wire::Schema = wire::Schema::Dyn;
static S_SLEEP_OUT: wire::Schema = wire::Schema::Dyn;

/// `sleep` over the binary wire.
#[napi(js_name = "sleepWire")]
pub fn sleep_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let ms: Value = wire::read_value(r, &S_SLEEP_0)?;
        let out = &port::sleep(ms)?;
        wire::write_value(w, out, &S_SLEEP_OUT)?;
        Ok(())
    })
}

static S_CREATE_PROFILE_SUMMARIZER_0: wire::Schema = wire::Schema::Dyn;
static S_CREATE_PROFILE_SUMMARIZER_OUT: wire::Schema = wire::Schema::Dyn;

/// `createProfileSummarizer` over the binary wire.
#[napi(js_name = "createProfileSummarizerWire")]
pub fn create_profile_summarizer_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let build_dir: Value = wire::read_value(r, &S_CREATE_PROFILE_SUMMARIZER_0)?;
        let out = &port::create_profile_summarizer(build_dir)?;
        wire::write_value(w, out, &S_CREATE_PROFILE_SUMMARIZER_OUT)?;
        Ok(())
    })
}

static S_ASSERT_ISOLATED_HOME_0: wire::Schema = wire::Schema::Dyn;
static S_ASSERT_ISOLATED_HOME_OUT: wire::Schema = wire::Schema::Dyn;

/// `assertIsolatedHome` over the binary wire.
#[napi(js_name = "assertIsolatedHomeWire")]
pub fn assert_isolated_home_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let home: Value = wire::read_value(r, &S_ASSERT_ISOLATED_HOME_0)?;
        let out = &port::assert_isolated_home(home)?;
        wire::write_value(w, out, &S_ASSERT_ISOLATED_HOME_OUT)?;
        Ok(())
    })
}

static S_RESOLVE_ELECTRON_BINARY_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_ELECTRON_BINARY_OUT: wire::Schema = wire::Schema::Dyn;

/// `resolveElectronBinary` over the binary wire.
#[napi(js_name = "resolveElectronBinaryWire")]
pub fn resolve_electron_binary_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let build_dir: Value = wire::read_value(r, &S_RESOLVE_ELECTRON_BINARY_0)?;
        let out = &port::resolve_electron_binary(build_dir)?;
        wire::write_value(w, out, &S_RESOLVE_ELECTRON_BINARY_OUT)?;
        Ok(())
    })
}

static S_ISOLATED_ENVIRONMENT_0: wire::Schema = wire::Schema::Dyn;
static S_ISOLATED_ENVIRONMENT_OUT: wire::Schema = wire::Schema::Dyn;

/// `isolatedEnvironment` over the binary wire.
#[napi(js_name = "isolatedEnvironmentWire")]
pub fn isolated_environment_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let arg0: Value = wire::read_value(r, &S_ISOLATED_ENVIRONMENT_0)?;
        let out = &port::isolated_environment(arg0)?;
        wire::write_value(w, out, &S_ISOLATED_ENVIRONMENT_OUT)?;
        Ok(())
    })
}

static S_PROCESS_TREE_0: wire::Schema = wire::Schema::Dyn;
static S_PROCESS_TREE_OUT: wire::Schema = wire::Schema::Dyn;

/// `processTree` over the binary wire.
#[napi(js_name = "processTreeWire")]
pub fn process_tree_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let root_pid: Value = wire::read_value(r, &S_PROCESS_TREE_0)?;
        let out = &port::process_tree(root_pid)?;
        wire::write_value(w, out, &S_PROCESS_TREE_OUT)?;
        Ok(())
    })
}

static S_CLASSIFY_PROCESS_0: wire::Schema = wire::Schema::Dyn;
static S_CLASSIFY_PROCESS_1: wire::Schema = wire::Schema::Dyn;
static S_CLASSIFY_PROCESS_OUT: wire::Schema = wire::Schema::Dyn;

/// `classifyProcess` over the binary wire.
#[napi(js_name = "classifyProcessWire")]
pub fn classify_process_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let row: Value = wire::read_value(r, &S_CLASSIFY_PROCESS_0)?;
        let arg1: Value = wire::read_value(r, &S_CLASSIFY_PROCESS_1)?;
        let out = &port::classify_process(row, arg1)?;
        wire::write_value(w, out, &S_CLASSIFY_PROCESS_OUT)?;
        Ok(())
    })
}

static S_LAUNCH_DESKTOP_0: wire::Schema = wire::Schema::Dyn;
static S_LAUNCH_DESKTOP_OUT: wire::Schema = wire::Schema::Dyn;

/// `launchDesktop` over the binary wire.
#[napi(js_name = "launchDesktopWire")]
pub fn launch_desktop_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let arg0: Value = wire::read_value(r, &S_LAUNCH_DESKTOP_0)?;
        let out = &port::launch_desktop(arg0)?;
        wire::write_value(w, out, &S_LAUNCH_DESKTOP_OUT)?;
        Ok(())
    })
}

static S_IS_FRONTMOST_0: wire::Schema = wire::Schema::Dyn;
static S_IS_FRONTMOST_OUT: wire::Schema = wire::Schema::Dyn;

/// `isFrontmost` over the binary wire.
#[napi(js_name = "isFrontmostWire")]
pub fn is_frontmost_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let app: Value = wire::read_value(r, &S_IS_FRONTMOST_0)?;
        let out = &port::is_frontmost(app)?;
        wire::write_value(w, out, &S_IS_FRONTMOST_OUT)?;
        Ok(())
    })
}

static S_SECONDS_SINCE_USER_INPUT_OUT: wire::Schema = wire::Schema::Dyn;

/// `secondsSinceUserInput` over the binary wire.
#[napi(js_name = "secondsSinceUserInputWire")]
pub fn seconds_since_user_input_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::seconds_since_user_input()?;
        wire::write_value(w, out, &S_SECONDS_SINCE_USER_INPUT_OUT)?;
        Ok(())
    })
}

static S_SCREEN_LOCKED_OUT: wire::Schema = wire::Schema::Dyn;

/// `screenLocked` over the binary wire.
#[napi(js_name = "screenLockedWire")]
pub fn screen_locked_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::screen_locked()?;
        wire::write_value(w, out, &S_SCREEN_LOCKED_OUT)?;
        Ok(())
    })
}

static S_FRONTMOST_PROBLEM_0: wire::Schema = wire::Schema::Dyn;
static S_FRONTMOST_PROBLEM_1: wire::Schema = wire::Schema::Dyn;
static S_FRONTMOST_PROBLEM_OUT: wire::Schema = wire::Schema::Dyn;

/// `frontmostProblem` over the binary wire.
#[napi(js_name = "frontmostProblemWire")]
pub fn frontmost_problem_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let app: Value = wire::read_value(r, &S_FRONTMOST_PROBLEM_0)?;
        let when: Value = wire::read_value(r, &S_FRONTMOST_PROBLEM_1)?;
        let out = &port::frontmost_problem(app, when)?;
        wire::write_value(w, out, &S_FRONTMOST_PROBLEM_OUT)?;
        Ok(())
    })
}

static S_FOOTPRINT_MB_0: wire::Schema = wire::Schema::Dyn;
static S_FOOTPRINT_MB_OUT: wire::Schema = wire::Schema::Dyn;

/// `footprintMb` over the binary wire.
#[napi(js_name = "footprintMbWire")]
pub fn footprint_mb_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let pids: Value = wire::read_value(r, &S_FOOTPRINT_MB_0)?;
        let out = &port::footprint_mb(pids)?;
        wire::write_value(w, out, &S_FOOTPRINT_MB_OUT)?;
        Ok(())
    })
}

static S_MEMORY_SNAPSHOT_0: wire::Schema = wire::Schema::Dyn;
static S_MEMORY_SNAPSHOT_OUT: wire::Schema = wire::Schema::Dyn;

/// `memorySnapshot` over the binary wire.
#[napi(js_name = "memorySnapshotWire")]
pub fn memory_snapshot_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let app: Value = wire::read_value(r, &S_MEMORY_SNAPSHOT_0)?;
        let out = &port::memory_snapshot(app)?;
        wire::write_value(w, out, &S_MEMORY_SNAPSHOT_OUT)?;
        Ok(())
    })
}

static S_RSS_TOTAL_MB_0: wire::Schema = wire::Schema::Dyn;
static S_RSS_TOTAL_MB_OUT: wire::Schema = wire::Schema::Dyn;

/// `rssTotalMb` over the binary wire.
#[napi(js_name = "rssTotalMbWire")]
pub fn rss_total_mb_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let app: Value = wire::read_value(r, &S_RSS_TOTAL_MB_0)?;
        let out = &port::rss_total_mb(app)?;
        wire::write_value(w, out, &S_RSS_TOTAL_MB_OUT)?;
        Ok(())
    })
}

static S_WAKEUPS_OVER_WINDOW_0: wire::Schema = wire::Schema::Dyn;
static S_WAKEUPS_OVER_WINDOW_1: wire::Schema = wire::Schema::Dyn;
static S_WAKEUPS_OVER_WINDOW_OUT: wire::Schema = wire::Schema::Dyn;

/// `wakeupsOverWindow` over the binary wire.
#[napi(js_name = "wakeupsOverWindowWire")]
pub fn wakeups_over_window_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let app: Value = wire::read_value(r, &S_WAKEUPS_OVER_WINDOW_0)?;
        let seconds: Value = wire::read_value(r, &S_WAKEUPS_OVER_WINDOW_1)?;
        let out = &port::wakeups_over_window(app, seconds)?;
        wire::write_value(w, out, &S_WAKEUPS_OVER_WINDOW_OUT)?;
        Ok(())
    })
}

static S_GPU_SYSTEM_SAMPLE_OUT: wire::Schema = wire::Schema::Dyn;

/// `gpuSystemSample` over the binary wire.
#[napi(js_name = "gpuSystemSampleWire")]
pub fn gpu_system_sample_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::gpu_system_sample()?;
        wire::write_value(w, out, &S_GPU_SYSTEM_SAMPLE_OUT)?;
        Ok(())
    })
}

static S_MACHINE_STATE_OUT: wire::Schema = wire::Schema::Dyn;

/// `machineState` over the binary wire.
#[napi(js_name = "machineStateWire")]
pub fn machine_state_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::machine_state()?;
        wire::write_value(w, out, &S_MACHINE_STATE_OUT)?;
        Ok(())
    })
}

static S_MACHINE_IDENTITY_OUT: wire::Schema = wire::Schema::Dyn;

/// `machineIdentity` over the binary wire.
#[napi(js_name = "machineIdentityWire")]
pub fn machine_identity_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::machine_identity()?;
        wire::write_value(w, out, &S_MACHINE_IDENTITY_OUT)?;
        Ok(())
    })
}

static S_MEDIAN_0: wire::Schema = wire::Schema::Dyn;
static S_MEDIAN_OUT: wire::Schema = wire::Schema::Dyn;

/// `median` over the binary wire.
#[napi(js_name = "medianWire")]
pub fn median_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let values: Value = wire::read_value(r, &S_MEDIAN_0)?;
        let out = &port::median(values)?;
        wire::write_value(w, out, &S_MEDIAN_OUT)?;
        Ok(())
    })
}

static S_PERCENTILE_0: wire::Schema = wire::Schema::Dyn;
static S_PERCENTILE_1: wire::Schema = wire::Schema::Dyn;
static S_PERCENTILE_OUT: wire::Schema = wire::Schema::Dyn;

/// `percentile` over the binary wire.
#[napi(js_name = "percentileWire")]
pub fn percentile_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let values: Value = wire::read_value(r, &S_PERCENTILE_0)?;
        let fraction: Value = wire::read_value(r, &S_PERCENTILE_1)?;
        let out = &port::percentile(values, fraction)?;
        wire::write_value(w, out, &S_PERCENTILE_OUT)?;
        Ok(())
    })
}

static S_SUMMARIZE_0: wire::Schema = wire::Schema::Dyn;
static S_SUMMARIZE_1: wire::Schema = wire::Schema::Dyn;
static S_SUMMARIZE_OUT: wire::Schema = wire::Schema::Dyn;

/// `summarize` over the binary wire.
#[napi(js_name = "summarizeWire")]
pub fn summarize_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let samples: Value = wire::read_value(r, &S_SUMMARIZE_0)?;
        let unit: Value = wire::read_value(r, &S_SUMMARIZE_1)?;
        let out = &port::summarize(samples, unit)?;
        wire::write_value(w, out, &S_SUMMARIZE_OUT)?;
        Ok(())
    })
}

static S_GAP_0: wire::Schema = wire::Schema::Dyn;
static S_GAP_1: wire::Schema = wire::Schema::Dyn;
static S_GAP_OUT: wire::Schema = wire::Schema::Dyn;

/// `gap` over the binary wire.
#[napi(js_name = "gapWire")]
pub fn gap_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let result: Value = wire::read_value(r, &S_GAP_0)?;
        let cause: Value = wire::read_value(r, &S_GAP_1)?;
        let out = &port::gap(result, cause)?;
        wire::write_value(w, out, &S_GAP_OUT)?;
        Ok(())
    })
}
