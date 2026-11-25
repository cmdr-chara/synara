mod account;
mod agent_gateway_mcp;
mod acp_runner;
mod claude_runner;
mod claude_monitor;
mod claude_workflow_script;
mod claude_workflow_runtime;
mod claude_tasks;
mod provider_env;
mod provider_instances;
mod claude_environment;
mod codex_runner;
mod device;
mod editor_intelligence;
mod editor;
mod hosts;
mod automation_executor;
mod automation_schedule;
mod automation;
mod browser_automation;
mod browser_cdp;
mod fs_browse;
mod git_domain;
mod harness_policy;
mod git_handoff;
mod git_action_registry;
mod git_pr;
mod github_inbox;
mod list_worktrees;
mod native_ws;
mod orchestration_reads;
mod orchestration_snapshot;
mod original;
mod mcp_connectors;
mod provider_usage;
mod textgen;
mod provider_attachments;
mod provider_cache;
mod skills_catalog;
mod computer_audit;
mod computer_native;
mod opencode_runner;
mod pi_runner;
mod orchestration_engine;
mod usage_limit;
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
    // The sweep owns running turns only when the native engine serves
    // orchestration. With the upstream TS server active, its turns may be
    // legitimately in flight in the shared state and must not be touched.
    if std::env::var("SYNARA_NO_UPSTREAM").is_ok() {
        let orphans = orchestration_engine::recover_orphaned_turns();
        if !orphans.is_empty() {
            let crash_resumed = orchestration_engine::resume_orphaned_turns(&orphans);
            eprintln!(
                "startup recovery: settled {} orphaned thread(s), resumed {crash_resumed}",
                orphans.len()
            );
        }
        let (fenced, recovered) = git_handoff::recover();
        if fenced + recovered > 0 {
            eprintln!("startup recovery: fenced {fenced} interrupted git handoff(s), replayed {recovered}");
        }
        automation_executor::start();
        let resumed = orchestration_engine::consume_quit_resume_record();
        if resumed > 0 {
            eprintln!("startup recovery: resumed {resumed} thread(s) from the quit-resume record");
        }
    }
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
    if std::env::var("SYNARA_NO_UPSTREAM").is_ok() {
        std::thread::spawn(|| loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
            let fired = orchestration_engine::fire_due_usage_limit_recoveries();
            if fired > 0 {
                eprintln!("usage-limit recovery: resumed {fired} thread(s) after reset");
            }
        });
        claude_runner::spawn_credential_keepalive();
    }
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
    crate::acp_runner::drop_all_resources();
    crate::codex_runner::drop_all_resources();
    crate::claude_runner::drop_all_resources();
    crate::opencode_runner::drop_all_resources();
    crate::pi_runner::drop_all_resources();
    crate::browser_automation::shutdown_browser_automation().await;
}
