mod account;
mod agent_gateway_mcp;
mod acp_runner;
mod claude_runner;
mod codex_runner;
mod device;
mod editor;
mod hosts;
mod automation;
mod browser_automation;
mod fs_browse;
mod git_domain;
mod harness_policy;
mod git_handoff;
mod git_pr;
mod github_inbox;
mod list_worktrees;
mod native_ws;
mod orchestration_reads;
mod orchestration_snapshot;
mod original;
mod mcp_connectors;
mod provider_cache;
mod skills_catalog;
mod computer_audit;
mod computer_native;
mod opencode_runner;
mod orchestration_engine;
mod project_agent;
mod project_imports;
mod projects_provision;
mod projects_devserver;
mod projects_fileops;
mod projects_fs;
mod projects_refs;
mod projects_search;
mod pull_requests;
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
mod studio;
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
        .merge(agent_gateway_mcp::router())
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
