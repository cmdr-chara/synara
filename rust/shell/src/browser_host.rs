//! Shell-side browser host: the upstream desktop role. Publishes a unix-socket
//! JSON-RPC endpoint (getInfo/executeTool) that the Synara server delegates
//! visible browser automation to, and drives the browser over Chrome CDP.

use serde_json::Value;
use std::sync::Arc;

struct CdpExecutor;

impl browserhost_rs::server::HostExecutor for CdpExecutor {
    fn execute(
        &self,
        _session_id: &str,
        thread_id: Option<&str>,
        name: &str,
        arguments: Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value, String>> + Send>> {
        let name = name.to_string();
        let thread = thread_id.map(str::to_string);
        Box::pin(async move {
            browserhost_rs::cdp::call_cdp_tool(&name, &arguments, thread.as_deref()).await
        })
    }
}

fn random_capability() -> String {
    use base64::Engine;
    let mut bytes = [0u8; 32];
    if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
        use std::io::Read;
        let _ = f.read_exact(&mut bytes);
    } else {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = (nanos.wrapping_add(i as u128) & 0xff) as u8;
        }
    }
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// Start the host on a dedicated thread with its own tokio runtime and publish
/// discovery for the backend. No-op when Chrome is absent (nothing the host
/// could drive anyway).
pub fn start_browser_host() {
    if !browserhost_rs::cdp::cdp_available() {
        eprintln!("browser host: chrome not found, not starting");
        return;
    }
    std::thread::spawn(|| {
        let runtime = match tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(error) => {
                eprintln!("browser host: runtime: {error}");
                return;
            }
        };
        runtime.block_on(async {
            let pipe_path = browserhost_rs::discovery::default_pipe_path();
            let capability = random_capability();
            let server = browserhost_rs::server::HostServer {
                pipe_path: pipe_path.to_string_lossy().to_string(),
                capability: capability.clone(),
            };
            if let Err(error) = browserhost_rs::discovery::publish(
                &server.pipe_path,
                &server.capability,
            ) {
                eprintln!("browser host: publish discovery: {error}");
                return;
            }
            eprintln!("browser host: serving on {}", server.pipe_path);
            let _ = server.serve(Arc::new(CdpExecutor)).await;
        });
    });
}
