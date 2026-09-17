//! Generic ACP proof over a real, pinned loopback SSH transport.
#![cfg(unix)]

use async_trait::async_trait;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use synara_acp::{AcpBackend, AcpTimeouts};
use synara_agent::*;
use synara_core::*;
use synara_runtime::{LaunchSpec, PinnedSshHost, SshTarget};

#[derive(Default)]
struct Events(Mutex<Vec<ThreadEvent>>);

#[async_trait]
impl EventSink for Events {
    async fn emit(&self, _: ThreadId, event: ThreadEvent) -> AgentResult<()> {
        self.0.lock().unwrap().push(event);
        Ok(())
    }
}

#[tokio::test]
#[ignore = "requires the isolated server from scripts/ssh_smoke.py"]
async fn two_agent_profiles_use_the_same_acp_backend_over_ssh() {
    let root = PathBuf::from(std::env::var_os("SYNARA_SSH_SMOKE_ROOT").expect("use ssh_smoke.py"));
    assert!(root.is_absolute());
    assert_eq!(
        std::fs::read_to_string(root.join("fixture.marker")).unwrap(),
        "Synara isolated SSH fixture v1\n"
    );
    let port = std::env::var("SYNARA_SSH_SMOKE_PORT")
        .unwrap()
        .parse::<u16>()
        .unwrap();
    assert!(port > 1024);
    let host = Arc::new(
        PinnedSshHost::new(
            SshTarget {
                host: "127.0.0.1".into(),
                port,
                user: Some(std::env::var("SYNARA_SSH_SMOKE_USER").unwrap()),
            },
            root.join("known hosts"),
            root.join("identity"),
        )
        .unwrap(),
    );
    let backend = AcpBackend {
        timeouts: AcpTimeouts {
            initialize: Duration::from_secs(10),
            operation: Duration::from_secs(10),
            authentication: Duration::from_secs(10),
            prompt: Duration::from_secs(10),
            cancellation: Duration::from_secs(3),
        },
    };
    for profile in ["alpha", "beta"] {
        let events = Arc::new(Events::default());
        let cwd = root.join("project with ' quote");
        let spec = AgentSpec {
            launch_directory: None,
            id: format!("ssh-{profile}"),
            name: format!("SSH fixture {profile}"),
            origin: "isolated integration fixture".into(),
            launch: LaunchSpec {
                command: env!("CARGO_BIN_EXE_synara-acp-fixture").into(),
                args: vec!["--integration-fixture".into(), profile.into()],
                env: Default::default(),
            },
        };
        let connection = backend
            .connect(
                &spec,
                ConnectionContext {
                    cwd: cwd.clone(),
                    host: host.clone(),
                    events: events.clone(),
                    interactions: Arc::new(DenyInteractions),
                },
            )
            .await
            .unwrap();
        assert_eq!(
            connection.info().identity.unwrap().name,
            format!("fixture-{profile}")
        );
        assert!(connection.info().host.starts_with("SSH "));
        assert_eq!(
            connection.info().capabilities.load_session,
            profile == "alpha"
        );
        let session = connection
            .new_session(SessionOptions::new(ThreadId::new(), cwd))
            .await
            .unwrap();
        assert_eq!(session.configuration().options[0].choices[0].value, profile);
        assert_eq!(
            session.prompt(Prompt::text("hello")).await.unwrap(),
            "end_turn"
        );
        let text: String = events
            .0
            .lock()
            .unwrap()
            .iter()
            .filter_map(|event| match event {
                ThreadEvent::TextDelta {
                    role: Role::Assistant,
                    text,
                    ..
                } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(text, format!("Hello from {profile}"));
        assert!(
            connection
                .trace()
                .iter()
                .any(|entry| entry.method.as_deref() == Some("initialize"))
        );
        connection.disconnect().await.unwrap();
        assert_eq!(connection.info().state, ConnectionState::Disconnected);
    }
}
