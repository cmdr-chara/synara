mod account;
mod migrations;
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
mod host_auth_verify;
mod host_identity;
mod pairing_gateway;
mod remote_pairing;
mod remote_resource;
mod remote_sessions;
mod remote_tls;
mod remote_tunnel;
mod hosts;
mod automation_executor;
mod automation_schedule;
mod automation;
mod beta_import;
mod browser_automation;
mod browser_frame;
mod fs_browse;
mod git_domain;
mod harness_policy;
mod git_handoff;
mod git_action_registry;
mod git_pr;
mod github_inbox;
mod attachments_http;
mod favicons;
mod auth_http;
mod editor_icon;
mod auth_sessions;
mod server_secrets;
mod thread_export;
mod local_image;
mod list_worktrees;
mod native_ws;
mod orchestration_reads;
mod orchestration_snapshot;
mod original;
mod mcp_connectors;
mod mcp_oauth;
mod editor_debug;
mod provider_usage;
mod textgen;
mod provider_attachments;
mod provider_cache;
mod skills_catalog;
mod computer_audit;
mod computer_frames;
mod frame_transport;
mod computer_approval;
mod computer_control;
mod computer_denylist;
mod computer_manager;
mod device_helper;
mod web_static;
mod device_sim;
mod device_stream;
mod computer_native;
mod opencode_runner;
mod pi_runner;
mod pi_opencode_catalog;
mod codex_usage;
mod voice_transcription;
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
    server_lifecycle::start_keep_awake();
    let original = original::Original::start().await;
    // The sweep owns running turns only when the native engine serves
    // orchestration. With the upstream TS server active, its turns may be
    // legitimately in flight in the shared state and must not be touched.
    if std::env::var("SYNARA_NO_UPSTREAM").is_ok() {
        // Stable→Beta handoff consumption must happen before migrations open
        // state.sqlite: the imported snapshot can carry a different schema.
        let beta_import = beta_import::run_beta_import_if_requested();
        if beta_import.consumed {
            if beta_import.ok {
                eprintln!("beta import: stable data copied");
            } else {
                eprintln!("beta import failed: {}", beta_import.error.as_deref().unwrap_or("unknown error"));
            }
        }
        // Schema ownership: the native backend runs the full migration ledger
        // once at startup so fresh homes get the complete schema and existing
        // databases validate against the canonical lineage before any reader
        // touches state.sqlite.
        match migrations::run_state_migrations() {
            Ok(applied) if !applied.is_empty() => {
                eprintln!("migrations: applied {} migration(s) through {}", applied.len(), applied.last().copied().unwrap_or(0));
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("migrations failed: {} — refusing to serve orchestration against an unvalidated schema", e.0);
                std::process::exit(1);
            }
        }
        // Host identity is established once at startup like upstream: the
        // pairing scope, credential minting and tunnel admission all derive
        // from it, so a fresh home must have it before any remote surface is
        // touched. Existing identities load unchanged.
        {
            let state_dir = todo::state_dir();
            let secrets_dir = format!("{state_dir}/secrets");
            let _ = std::fs::create_dir_all(&secrets_dir);
            // The environment id is created once on first boot exactly like
            // upstream's accountAuth — a bare UUID in a file every remote
            // surface and server.getEnvironment reads back.
            let env_id_path = std::path::Path::new(&state_dir).join("environment-id");
            if server_env::environment_id(&state_dir).is_none() {
                let id = orchestration_engine::uuid_v4();
                if std::fs::write(&env_id_path, format!("{id}\n")).is_ok() {
                    eprintln!("environment id: generated");
                }
            }
            let identity_path = host_identity::host_identity_path(&secrets_dir);
            match host_identity::read_host_identity(&identity_path) {
                Ok(Some(_)) => {}
                Ok(None) => {
                    if host_identity::generate_and_persist_host_identity(&identity_path).is_ok() {
                        eprintln!("host identity: generated");
                    }
                }
                Err(e) => eprintln!("host identity unreadable: {e}"),
            }
        }
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
        auth_http::seed_bootstrap_credential();
    attachments_http::start_cleanup_worker();
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
        .route("/api/local-image", axum::routing::get(local_image::local_image))
        .route("/api/attachments/upload", axum::routing::post(attachments_http::upload))
        .route("/attachments/{id}", axum::routing::get(attachments_http::serve))
        .route("/api/thread-export", axum::routing::get(thread_export::thread_export))
        .route("/oauth/mcp/callback", axum::routing::get(oauth_mcp_callback))
        .route("/api/project-favicon", axum::routing::get(favicons::project_favicon))
        .route("/api/browser/frame", axum::routing::get(browser_frame::browser_frame))
        .route("/api/site-favicon", axum::routing::get(favicons::site_favicon))
        .route("/api/editor-icon", axum::routing::get(editor_icon::editor_icon))
        .route("/health", axum::routing::get(|| async { "ok" }))
        .route("/api/auth/bootstrap", axum::routing::post(auth_http::bootstrap))
        .route("/api/auth/session", axum::routing::get(auth_http::session_state))
        .route("/api/auth/ws-token", axum::routing::post(auth_http::ws_token))
        .route("/api/auth/pairing-token", axum::routing::post(auth_http::pairing_token))
        .route("/api/auth/logout", axum::routing::post(auth_http::logout))
        .route("/api/auth/clients", axum::routing::get(auth_http::clients))
        .route("/api/auth/clients/revoke", axum::routing::post(auth_http::clients_revoke))
        .route("/api/auth/bootstrap/bearer", axum::routing::post(auth_http::bootstrap_bearer))
        .route("/api/auth/pairing-links", axum::routing::get(auth_http::pairing_links))
        .route("/api/auth/pairing-links/revoke", axum::routing::post(auth_http::pairing_links_revoke))
        .route("/api/auth/clients/revoke-others", axum::routing::post(auth_http::clients_revoke_others))
        .fallback(axum::routing::get(web_static::serve))
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

/// GET /oauth/mcp/callback — completes an MCP connector OAuth sign-in: the
/// token exchange runs here, tokens persist with the connector row in the
/// connectors document, and the user gets a closable HTML result page.
async fn oauth_mcp_callback(axum::extract::RawQuery(query): axum::extract::RawQuery) -> axum::response::Response {
    let query = query.unwrap_or_default();
    let result = tokio::task::spawn_blocking(move || {
        mcp_oauth::complete_sign_in(&query)
    })
    .await;
    let html = match result {
        Ok(Ok((connector_input, tokens))) => {
            match persist_oauth_connector(&connector_input, &tokens) {
                Ok(()) => "<html><body><h2>Connector connected</h2><p>You can close this tab and return to Synara.</p></body></html>".to_string(),
                Err(e) => format!("<html><body><h2>Sign-in failed</h2><p>{}</p></body></html>", html_escape(&e)),
            }
        }
        Ok(Err(e)) => format!("<html><body><h2>Sign-in failed</h2><p>{}</p></body></html>", html_escape(&e.0)),
        Err(e) => format!("<html><body><h2>Sign-in failed</h2><p>{}</p></body></html>", html_escape(&e.to_string())),
    };
    axum::response::Response::builder()
        .header("content-type", "text/html; charset=utf-8")
        .body(axum::body::Body::from(html))
        .unwrap()
}

fn html_escape(v: &str) -> String {
    v.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Persist the OAuth-completed HTTP connector: unique server name, tokens in
/// the same secrets-backed document the stdio rows use (0600 by construction),
/// deduped on url. Mirrors the stdio insertion in mcp_connectors.
fn persist_oauth_connector(input: &serde_json::Value, tokens: &serde_json::Value) -> Result<(), String> {
    use serde_json::json;
    let url = input.get("url").and_then(serde_json::Value::as_str).ok_or("missing url")?;
    let name = input.get("name").and_then(serde_json::Value::as_str).unwrap_or("Connector");
    mcp_connectors::insert_http_connector(url, name, input, tokens).map_err(|e| e.0)
        .map(|_| ())
}
