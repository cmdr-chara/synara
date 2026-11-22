mod fs_browse;
mod git_domain;
mod git_pr;
mod github_inbox;
mod list_worktrees;
mod native_ws;
mod orchestration_reads;
mod orchestration_snapshot;
mod original;
mod projects_devserver;
mod projects_fileops;
mod projects_fs;
mod projects_refs;
mod projects_search;
mod runtime_status;
mod server_desktop_update;
mod server_diagnostics;
mod server_env;
mod server_keybindings;
mod server_lifecycle;
mod server_local_servers;
mod server_settings;
mod shell_editor;
mod stats;
mod terminal;
mod todo;
mod units;
mod ws_relay;

use std::net::SocketAddr;

use axum::serve::ListenerExt;
use original::shutdown_signal;

#[tokio::main]
async fn main() {
    original::report_fallbacks();
    runtime_status::start_sampler();
    let original = original::Original::start().await;
    let app = units::router()
        .merge(native_ws::router())
        .merge(ws_relay::router())
        .route("/__tap_client", axum::routing::post(original::tap_client))
        .with_state(original.clone());
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let listener = tokio::net::TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], port)))
        .await
        .expect("binding the port");
    let listener = listener.tap_io(|tcp| {
        let _ = tcp.set_nodelay(true);
    });
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("serving");
    original.stop();
}
