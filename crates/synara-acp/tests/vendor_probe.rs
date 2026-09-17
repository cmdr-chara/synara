//! Opt-in interoperability probe for reviewed vendor releases, without provider credentials.
#![cfg(target_os = "linux")]

use async_trait::async_trait;
use serde_json::{Value, json};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use synara_acp::{AcpBackend, AcpTimeouts};
use synara_agent::{
    AgentBackend, AgentError, AgentResult, ConnectionContext, DenyInteractions, EventSink,
    SessionOptions,
};
use synara_core::{ConnectionState, ThreadEvent, ThreadId};
use synara_registry::{AgentEntry, HttpsDownloader, Platform, Registry, RegistryStore};
use synara_runtime::LocalHost;

#[derive(Default)]
struct CountEvents(AtomicUsize);

#[async_trait]
impl EventSink for CountEvents {
    async fn emit(&self, _: ThreadId, _: ThreadEvent) -> AgentResult<()> {
        self.0.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

fn classify(error: &AgentError) -> Value {
    // Do not export vendor error text, authentication URLs, session IDs or payloads.
    match error {
        AgentError::Remote { code, .. } => json!({"kind": "remote_error", "code": code}),
        AgentError::AuthenticationRequired => json!({"kind": "authentication_required"}),
        AgentError::Timeout => json!({"kind": "timeout"}),
        AgentError::Disconnected(_) => json!({"kind": "disconnected"}),
        AgentError::Runtime(_) => json!({"kind": "runtime_error"}),
        AgentError::Unsupported(_) => json!({"kind": "unsupported"}),
        AgentError::Invalid(_) => json!({"kind": "invalid"}),
        AgentError::Cancelled => json!({"kind": "cancelled"}),
        AgentError::Busy => json!({"kind": "busy"}),
        AgentError::Limit => json!({"kind": "resource_limit"}),
        AgentError::EventDelivery => json!({"kind": "event_delivery"}),
    }
}

fn private_directory(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    fs::create_dir(path).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
}

async fn probe(entry: &AgentEntry, root: &Path) -> Value {
    let mut report = json!({
        "id": entry.id,
        "release_version": entry.version,
        "repository": entry.repository,
        "installation": "pending",
        "initialization": "not_attempted",
        "session": "not_attempted",
        "credentials_supplied": false,
        "authentication_attempted": false,
        "prompt_sent": false,
        "passed": false,
    });
    let store = RegistryStore::open(root.join("agents")).unwrap();
    let plan = entry.plan(Platform::current().unwrap()).unwrap();
    let installed = match store.install(&plan, &HttpsDownloader::default()) {
        Ok(installed) => installed,
        Err(error) => {
            report["installation"] = json!("failed");
            report["installation_error"] = json!(error.to_string());
            return report;
        }
    };
    let mut spec = match installed.reference.agent_spec() {
        Ok(spec) => spec,
        Err(error) => {
            report["installation"] = json!("revalidation_failed");
            report["installation_error"] = json!(error.to_string());
            return report;
        }
    };
    report["installation"] = json!("checksum_and_receipt_verified");
    for (key, directory) in [
        ("HOME", "home"),
        ("XDG_CONFIG_HOME", "config"),
        ("XDG_DATA_HOME", "data"),
        ("XDG_CACHE_HOME", "cache"),
        ("XDG_STATE_HOME", "state"),
        ("XDG_RUNTIME_DIR", "runtime"),
    ] {
        let path = root.join(directory);
        private_directory(&path);
        spec.launch
            .env
            .insert(key.into(), path.to_str().unwrap().into());
    }
    for key in [
        "SSH_AUTH_SOCK",
        "DBUS_SESSION_BUS_ADDRESS",
        "DISPLAY",
        "WAYLAND_DISPLAY",
    ] {
        spec.launch.env.insert(key.into(), String::new());
    }
    spec.launch.env.insert("CI".into(), "true".into());
    spec.launch.env.insert("NO_COLOR".into(), "1".into());
    spec.launch.env.insert("TERM".into(), "dumb".into());
    let cwd = root.join("empty project");
    private_directory(&cwd);
    let events = Arc::new(CountEvents::default());
    let backend = AcpBackend {
        timeouts: AcpTimeouts {
            initialize: Duration::from_secs(45),
            operation: Duration::from_secs(45),
            authentication: Duration::from_secs(5),
            prompt: Duration::from_secs(5),
            cancellation: Duration::from_secs(3),
        },
    };
    let connection = match backend
        .connect(
            &spec,
            ConnectionContext {
                host: Arc::new(LocalHost),
                cwd: cwd.clone(),
                events: events.clone(),
                interactions: Arc::new(DenyInteractions),
            },
        )
        .await
    {
        Ok(connection) => connection,
        Err(error) => {
            report["initialization"] = json!("failed");
            report["error"] = classify(&error);
            return report;
        }
    };
    let info = connection.info();
    report["initialization"] = json!("passed");
    report["agent_identity"] = json!(info.identity);
    report["capabilities"] = json!(info.capabilities);
    report["authentication_method_count"] = json!(info.authentication.len());
    let session_ok = match connection
        .new_session(SessionOptions::new(ThreadId::new(), cwd))
        .await
    {
        Ok(session) => {
            report["session"] = json!("created");
            report["configuration_option_count"] = json!(session.configuration().options.len());
            true
        }
        Err(AgentError::AuthenticationRequired) => {
            report["session"] = json!("authentication_required");
            true
        }
        Err(error) => {
            report["session"] = json!("failed");
            report["error"] = classify(&error);
            false
        }
    };
    let trace = connection.trace();
    let methods: Vec<_> = trace.iter().filter_map(|event| event.method.as_deref()).collect();
    let no_auth_or_prompt = !methods
        .iter()
        .any(|method| matches!(*method, "authenticate" | "session/prompt"));
    report["observed_methods"] = json!(methods);
    report["event_count"] = json!(events.0.load(Ordering::Relaxed));
    let disconnected = match connection.disconnect().await {
        Ok(()) => connection.info().state == ConnectionState::Disconnected,
        Err(error) => {
            report["disconnect_error"] = classify(&error);
            false
        }
    };
    report["disconnected"] = json!(disconnected);
    report["passed"] = json!(session_ok && no_auth_or_prompt && disconnected);
    report
}

#[tokio::test]
#[ignore = "downloads and launches reviewed vendor releases, requires explicit opt-in"]
async fn reviewed_releases_initialize_without_credentials() {
    assert_eq!(
        std::env::var("SYNARA_VENDOR_PROBE").as_deref(),
        Ok("approved-release-check"),
        "Use the opt-in vendor verification workflow or documented isolated procedure"
    );
    assert_eq!(Platform::current().unwrap(), Platform::LinuxIntel);
    let output = std::env::var_os("SYNARA_VENDOR_REPORT").expect("missing report output path");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .unwrap();
    let registry = Registry::parse(include_bytes!("fixtures/vendor_releases.json")).unwrap();
    let mut results = Vec::new();
    for entry in &registry.agents {
        let root = tempfile::Builder::new()
            .prefix("synara-vendor-probe-")
            .tempdir()
            .unwrap();
        results.push(probe(entry, root.path()).await);
        let document = json!({
            "format": 1,
            "scope": "release installation and no-credentials ACP initialization/session probe",
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "agents": results,
        });
        file.set_len(0).unwrap();
        use std::io::{Seek, SeekFrom};
        file.seek(SeekFrom::Start(0)).unwrap();
        file.write_all(&serde_json::to_vec_pretty(&document).unwrap()).unwrap();
        file.sync_all().unwrap();
    }
    println!("{}", serde_json::to_string_pretty(&results).unwrap());
    assert!(
        results.iter().all(|result| result["passed"] == true),
        "At least one reviewed release did not pass the bounded protocol probe"
    );
}

#[test]
fn diagnostic_errors_do_not_export_remote_text() {
    let error = AgentError::Remote {
        code: -32000,
        message: "secret-canary https://example.test/login?token=private".into(),
    };
    let report = classify(&error).to_string();
    assert!(!report.contains("secret-canary"));
    assert!(!report.contains("private"));
    assert!(report.contains("-32000"));
}

#[test]
fn reviewed_release_manifest_is_valid_and_requires_checksums() {
    let registry = Registry::parse(include_bytes!("fixtures/vendor_releases.json")).unwrap();
    assert_eq!(registry.agents.len(), 2);
    for entry in registry.agents {
        assert!(entry.plan(Platform::LinuxIntel).is_ok());
        assert!(entry.distribution.npx.is_none());
        assert!(entry.distribution.uvx.is_none());
    }
}
