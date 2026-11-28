mod account;
mod account_usage;
mod acp_runner;
mod agent_gateway_mcp;
mod agent_gateway_synara;
mod attachments_http;
mod auth_connections;
mod auth_http;
mod auth_sessions;
mod automation;
mod automation_executor;
mod automation_schedule;
mod beta_import;
mod beta_operational_issue;
mod browser_automation;
mod browser_frame;
mod checkpointing;
mod claude_environment;
mod claude_monitor;
mod claude_runner;
mod claude_tasks;
mod claude_workflow_runtime;
mod claude_workflow_script;
mod codex_runner;
mod codex_usage;
mod computer_approval;
mod computer_audit;
mod computer_control;
mod computer_denylist;
mod computer_frames;
mod computer_manager;
mod computer_native;
mod computer_shield;
mod database_lifecycle_lock;
mod device;
mod device_helper;
mod device_identity;
mod device_sim;
mod device_stream;
mod editor;
mod editor_debug;
mod editor_icon;
mod editor_intelligence;
mod factory_import;
mod favicons;
mod frame_transport;
mod fs_browse;
mod git_action_registry;
mod git_domain;
mod git_handoff;
mod git_pr;
mod github_inbox;
mod harness_policy;
mod host_auth_verify;
mod host_grant_verify;
mod host_identity;
mod host_secrets;
mod host_usage_snapshot;
mod hosts;
mod hub_outputs;
mod hub_work;
mod input_monitor;
mod list_worktrees;
mod local_image;
mod mcp_connectors;
mod mcp_oauth;
mod migrations;
mod native_ws;
mod omp_import;
mod opencode_runner;
mod orchestration_engine;
mod orchestration_reads;
mod orchestration_snapshot;
mod pairing_gateway;
mod pi_opencode_catalog;
mod pi_runner;
mod pr_command_detection;
mod project_agent;
mod project_imports;
mod projects_devserver;
mod projects_fileops;
mod projects_fs;
mod projects_provision;
mod projects_refs;
mod projects_search;
mod provider_attachments;
mod provider_cache;
mod provider_env;
mod provider_instances;
mod provider_resume;
mod provider_usage;
mod pull_requests;
mod remote_client;
mod remote_connections;
mod remote_host_trust;
mod remote_pairing;
mod remote_resource;
mod remote_resource_client;
mod remote_sessions;
mod remote_tls;
mod remote_tunnel;
mod runtime_status;
mod server_beta_usage;
mod server_desktop_update;
mod server_diagnostics;
mod server_env;
mod server_keybindings;
mod server_lifecycle;
mod server_local_servers;
mod server_secrets;
mod server_settings;
mod shell_editor;
mod sidechat_expiry;
mod skills_catalog;
mod state;
mod stats;
mod studio;
mod terminal;
mod textgen;
mod thread_export;
mod thread_git_metadata;
mod todo;
mod turn_format;
mod units;
mod usage_limit;
mod voice_transcription;
mod web_static;
mod ws_relay;

use std::net::SocketAddr;

use axum::serve::ListenerExt;
use state::shutdown_signal;

static DATABASE_LIFECYCLE_LOCK: std::sync::OnceLock<
    std::sync::Mutex<Option<database_lifecycle_lock::DatabaseLifecycleLock>>,
> = std::sync::OnceLock::new();

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let restore_executable = args
        .first()
        .and_then(|arg| std::path::Path::new(arg).file_name())
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.trim_end_matches(".exe") == "synara-restore-migration-backup");
    if restore_executable
        || args
            .get(1)
            .is_some_and(|arg| arg == "--restore-migration-backup")
    {
        let index = if restore_executable { 1 } else { 2 };
        let Some(db) = args.get(index) else {
            eprintln!("Usage: synara-restore-migration-backup <absolute-database-path>");
            std::process::exit(2);
        };
        let path = std::path::PathBuf::from(db);
        if !path.is_absolute() {
            eprintln!("Database path must be absolute: {db}");
            std::process::exit(2);
        }
        eprintln!("WARNING: Stop every Synara process before restoring a migration backup.");
        let lifecycle_lock = match database_lifecycle_lock::acquire_database_lifecycle_lock(&path) {
            Ok(lock) => lock,
            Err(error) => {
                eprintln!(
                    "Failed to restore migration backup for {}: {error}",
                    path.display()
                );
                std::process::exit(1);
            }
        };
        let restore_result = migrations::restore_marked_migration_backup(&path);
        let release_result = lifecycle_lock.release();
        if let Err(error) = release_result {
            eprintln!("database lifecycle lock release failed after restore: {error}");
        }
        match restore_result {
            Ok(()) => {
                println!("Restored migration backup for {}", path.display());
                std::process::exit(0);
            }
            Err(error) => {
                eprintln!(
                    "Failed to restore migration backup for {}: {error}",
                    path.display()
                );
                std::process::exit(1);
            }
        }
    }
    {
        let state_dir = std::path::PathBuf::from(todo::state_dir());
        let home_dir = state_dir
            .parent()
            .map(std::path::Path::to_path_buf)
            .unwrap_or(state_dir);
        synara_shellenv::fix_path(&home_dir);
    }
    runtime_status::start_sampler();
    server_lifecycle::start_keep_awake();
    let app_state = state::AppState::open();
    // The sweep owns running turns only when the native engine serves
    // orchestration. With the upstream TS server active, its turns may be
    // legitimately in flight in the shared state and must not be touched.
    if true {
        // Stable→Beta handoff consumption must happen before migrations open
        // state.sqlite: the imported snapshot can carry a different schema.
        let lifecycle_lock = match database_lifecycle_lock::acquire_database_lifecycle_lock(
            std::path::Path::new(&todo::state_dir())
                .join("state.sqlite")
                .as_path(),
        ) {
            Ok(lock) => lock,
            Err(error) => {
                beta_operational_issue::report("startup.database-locked", None, None);
                eprintln!("{error}");
                std::process::exit(1);
            }
        };
        DATABASE_LIFECYCLE_LOCK
            .get_or_init(|| std::sync::Mutex::new(None))
            .lock()
            .expect("database lifecycle lock slot")
            .replace(lifecycle_lock);
        let beta_import = beta_import::run_beta_import_if_requested();
        if beta_import.consumed {
            if beta_import.ok {
                eprintln!("beta import: stable data copied");
            } else {
                eprintln!(
                    "beta import failed: {}",
                    beta_import.error.as_deref().unwrap_or("unknown error")
                );
            }
        }
        // Schema ownership: the native backend runs the full migration ledger
        // once at startup so fresh homes get the complete schema and existing
        // databases validate against the canonical lineage before any reader
        // touches state.sqlite.
        match migrations::run_state_migrations() {
            Ok(applied) => {
                if !applied.is_empty() {
                    eprintln!(
                        "migrations: applied {} migration(s) through {}",
                        applied.len(),
                        applied.last().copied().unwrap_or(0)
                    );
                }
                server_beta_usage::start_beta_usage_snapshot_job();
                host_usage_snapshot::start_host_usage_snapshot_job();
                account_usage::start_account_usage_reporter_job();
            }
            Err(e) => {
                beta_operational_issue::report_startup_migration_error(&e.0);
                eprintln!(
                    "migrations failed: {} — refusing to serve orchestration against an unvalidated schema",
                    e.0
                );
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
            eprintln!(
                "startup recovery: fenced {fenced} interrupted git handoff(s), replayed {recovered}"
            );
        }
        auth_http::seed_bootstrap_credential();
        hosts::restore_desired_connections();
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
        .route(
            "/api/local-image",
            axum::routing::get(local_image::local_image),
        )
        .route(
            "/api/attachments/upload",
            axum::routing::post(attachments_http::upload),
        )
        .route(
            "/api/attachments/cancel",
            axum::routing::post(attachments_http::cancel),
        )
        .route(
            "/attachments/{id}",
            axum::routing::get(attachments_http::serve),
        )
        .route(
            "/api/thread-export",
            axum::routing::get(thread_export::thread_export),
        )
        .route(
            "/oauth/mcp/callback",
            axum::routing::get(oauth_mcp_callback),
        )
        .route(
            "/api/project-favicon",
            axum::routing::get(favicons::project_favicon),
        )
        .route(
            "/api/browser/frame",
            axum::routing::get(browser_frame::browser_frame),
        )
        .route(
            "/api/remote/resource/{host_id}",
            axum::routing::get(remote_resource_client::handle_remote_resource)
                .post(remote_resource_client::handle_remote_resource),
        )
        .route(
            "/api/site-favicon",
            axum::routing::get(favicons::site_favicon),
        )
        .route(
            "/api/editor-icon",
            axum::routing::get(editor_icon::editor_icon),
        )
        .route("/health", axum::routing::get(health))
        .route(
            "/api/auth/bootstrap",
            axum::routing::post(auth_http::bootstrap),
        )
        .route(
            "/api/auth/session",
            axum::routing::get(auth_http::session_state),
        )
        .route(
            "/api/auth/ws-token",
            axum::routing::post(auth_http::ws_token),
        )
        .route(
            "/api/auth/pairing-token",
            axum::routing::post(auth_http::pairing_token),
        )
        .route("/api/auth/logout", axum::routing::post(auth_http::logout))
        .route("/api/auth/clients", axum::routing::get(auth_http::clients))
        .route(
            "/api/auth/clients/revoke",
            axum::routing::post(auth_http::clients_revoke),
        )
        .route(
            "/api/auth/bootstrap/bearer",
            axum::routing::post(auth_http::bootstrap_bearer),
        )
        .route(
            "/api/auth/pairing-links",
            axum::routing::get(auth_http::pairing_links),
        )
        .route(
            "/api/auth/pairing-links/revoke",
            axum::routing::post(auth_http::pairing_links_revoke),
        )
        .route(
            "/api/auth/clients/revoke-others",
            axum::routing::post(auth_http::clients_revoke_others),
        )
        .fallback(axum::routing::get(web_static::serve))
        .with_state(app_state.clone());
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    if true {
        std::thread::spawn(|| {
            loop {
                std::thread::sleep(std::time::Duration::from_secs(60));
                let fired = orchestration_engine::fire_due_usage_limit_recoveries();
                if fired > 0 {
                    eprintln!("usage-limit recovery: resumed {fired} thread(s) after reset");
                }
                let woken = orchestration_engine::fire_due_thread_snoozes();
                if woken > 0 {
                    eprintln!("thread snooze: woke {woken} thread(s) past deadline");
                }
            }
        });
        // Retention sweep: upstream archives inactive threads daily after a
        // five-minute startup delay; one pass an hour converges to the same
        // steady state with no extra scheduler.
        std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_secs(300));
            loop {
                let archived = orchestration_engine::archive_inactive_threads();
                if archived > 0 {
                    eprintln!("thread retention: archived {archived} inactive thread(s)");
                }
                let purged = orchestration_engine::purge_deleted_threads();
                if purged > 0 {
                    eprintln!("thread retention: purged {purged} deleted thread(s)");
                }
                std::thread::sleep(std::time::Duration::from_secs(3600));
            }
        });
        claude_runner::spawn_credential_keepalive();
        crate::pull_requests::spawn_auto_fix_watcher();
        crate::sidechat_expiry::spawn_expiry_watcher();
        crate::hub_work::spawn_admission_loop();
        std::thread::spawn(|| {
            loop {
                crate::remote_connections::retry_reconnecting_once();
                std::thread::sleep(std::time::Duration::from_secs(5));
            }
        });
    }
    // Loopback by default, matching upstream: remote exposure requires an
    // explicit SYNARA_HOST override, and ws_relay then enforces authentication
    // for non-loopback binds. Binding wildcard unconditionally would publish
    // the HTTP API and every socket on the LAN without notice.
    let bind_host = crate::native_ws::config()
        .host()
        .and_then(|value| value.parse::<std::net::IpAddr>().ok())
        .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST));
    let listener = tokio::net::TcpListener::bind(SocketAddr::from((bind_host, port)))
        .await
        .expect("binding the port");
    let listener = listener.tap_io(|tcp| {
        let _ = tcp.set_nodelay(true);
    });
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("serving");
    crate::acp_runner::drop_all_resources();
    crate::codex_runner::drop_all_resources();
    crate::claude_runner::drop_all_resources();
    crate::opencode_runner::drop_all_resources();
    crate::pi_runner::drop_all_resources();
    crate::browser_automation::shutdown_browser_automation().await;
    crate::computer_shield::stop_shared_host();
    if let Some(lock) = DATABASE_LIFECYCLE_LOCK
        .get()
        .and_then(|slot| slot.lock().ok().and_then(|mut guard| guard.take()))
    {
        if let Err(error) = lock.release() {
            eprintln!("database lifecycle lock release failed: {error}");
        }
    }
}

/// GET /oauth/mcp/callback — completes an MCP connector OAuth sign-in: the
/// token exchange runs here, tokens persist with the connector row in the
/// connectors document, and the user gets a closable HTML result page.
async fn oauth_mcp_callback(
    axum::extract::RawQuery(query): axum::extract::RawQuery,
) -> axum::response::Response {
    let query = query.unwrap_or_default();
    let result = tokio::task::spawn_blocking(move || mcp_oauth::complete_sign_in(&query)).await;
    let html = match result {
        Ok(Ok((connector_input, tokens))) => {
            match persist_oauth_connector(&connector_input, &tokens) {
                Ok(()) => "<html><body><h2>Connector connected</h2><p>You can close this tab and return to Synara.</p></body></html>".to_string(),
                Err(e) => format!("<html><body><h2>Sign-in failed</h2><p>{}</p></body></html>", html_escape(&e)),
            }
        }
        Ok(Err(e)) => format!(
            "<html><body><h2>Sign-in failed</h2><p>{}</p></body></html>",
            html_escape(&e.0)
        ),
        Err(e) => format!(
            "<html><body><h2>Sign-in failed</h2><p>{}</p></body></html>",
            html_escape(&e.to_string())
        ),
    };
    axum::response::Response::builder()
        .header("content-type", "text/html; charset=utf-8")
        .body(axum::body::Body::from(html))
        .unwrap()
}

async fn health() -> axum::Json<serde_json::Value> {
    let high_water_sequence = tokio::task::spawn_blocking(orchestration_high_water_sequence)
        .await
        .unwrap_or(0);
    axum::Json(serde_json::json!({
        "status": "ok",
        // Tokio has no direct equivalent of Node's monitorEventLoopDelay, so
        // this surface reports the sampler's measured scheduler-lag analog.
        "eventLoop": runtime_status::get_runtime_status(),
        // Axum only routes this after migrations/recovery and router construction
        // have completed, so the lifecycle gates upstream separates are already
        // satisfied by the time a client can observe this response.
        "startupReady": true,
        "pushBusReady": true,
        "keybindingsReady": true,
        "terminalSubscriptionsReady": true,
        "orchestrationSubscriptionsReady": true,
        "projection": {
            "state": "healthy",
            "inFlight": false,
            "retryAttempts": 0,
            "hasFailure": false,
            "highWaterSequence": high_water_sequence,
            "lagByProjector": {},
            "missingProjectors": [],
        },
    }))
}

fn orchestration_high_water_sequence() -> i64 {
    let db = std::path::PathBuf::from(todo::state_dir()).join("state.sqlite");
    let Ok(conn) =
        rusqlite::Connection::open_with_flags(db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
    else {
        return 0;
    };
    conn.query_row(
        "SELECT COALESCE(MAX(sequence), 0) FROM orchestration_events",
        [],
        |row| row.get(0),
    )
    .unwrap_or(0)
}

fn html_escape(v: &str) -> String {
    v.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Persist the OAuth-completed HTTP connector: unique server name, tokens in
/// the same secrets-backed document the stdio rows use (0600 by construction),
/// deduped on url. Mirrors the stdio insertion in mcp_connectors.
fn persist_oauth_connector(
    input: &serde_json::Value,
    tokens: &serde_json::Value,
) -> Result<(), String> {
    let url = input
        .get("url")
        .and_then(serde_json::Value::as_str)
        .ok_or("missing url")?;
    let name = input
        .get("name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("Connector");
    mcp_connectors::insert_http_connector(url, name, input, tokens)
        .map_err(|e| e.0)
        .map(|_| ())
}
