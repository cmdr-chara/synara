use crate::{AgentBackend, AgentConnection, AgentError, AgentResult, AgentSpec, ConnectionContext};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, sync::Arc};
use synara_core::ConnectionState;
use tokio::sync::{Mutex, RwLock};

type Slot = Arc<Mutex<Option<Arc<dyn AgentConnection>>>>;

/// Owns shared connections. A prompt never implicitly creates another process.
#[derive(Default)]
pub struct ConnectionManager {
    slots: Mutex<HashMap<String, Slot>>,
    lifecycle: RwLock<()>,
}
impl ConnectionManager {
    pub async fn connection(
        &self,
        backend: &dyn AgentBackend,
        spec: &AgentSpec,
        context: ConnectionContext,
    ) -> AgentResult<Arc<dyn AgentConnection>> {
        spec.validate()?;
        let _lifetime = self.lifecycle.read().await;
        let slot = self.slot(spec, &context).await?;
        let mut owned = slot.lock().await;
        if let Some(connection) = &*owned {
            if matches!(
                connection.info().state,
                ConnectionState::Connected
                    | ConnectionState::AuthenticationRequired
                    | ConnectionState::Authenticating
            ) {
                return Ok(connection.clone());
            }
            connection.disconnect().await?;
        }
        *owned = None;
        let connection = backend.connect(spec, context).await?;
        *owned = Some(connection.clone());
        Ok(connection)
    }

    pub async fn restart(
        &self,
        backend: &dyn AgentBackend,
        spec: &AgentSpec,
        context: ConnectionContext,
    ) -> AgentResult<Arc<dyn AgentConnection>> {
        spec.validate()?;
        let _lifetime = self.lifecycle.read().await;
        let slot = self.slot(spec, &context).await?;
        let mut owned = slot.lock().await;
        if let Some(connection) = owned.take() {
            connection.disconnect().await?;
        }
        let connection = backend.connect(spec, context).await?;
        *owned = Some(connection.clone());
        Ok(connection)
    }

    async fn slot(&self, spec: &AgentSpec, context: &ConnectionContext) -> AgentResult<Slot> {
        let key = connection_key(spec, context);
        let mut slots = self.slots.lock().await;
        if !slots.contains_key(&key) && slots.len() >= 64 {
            return Err(AgentError::Limit);
        }
        Ok(slots
            .entry(key)
            .or_insert_with(|| Arc::new(Mutex::new(None)))
            .clone())
    }

    pub async fn disconnect_all(&self) -> AgentResult<()> {
        // Do not detach a slot while a connect/restart operation is still filling it.
        // New acquisitions begin only after this shutdown boundary has completed.
        let _lifetime = self.lifecycle.write().await;
        let slots = std::mem::take(&mut *self.slots.lock().await);
        let mut failure = None;
        for slot in slots.into_values() {
            if let Some(connection) = slot.lock().await.take()
                && let Err(error) = connection.disconnect().await
            {
                failure = Some(error);
            }
        }
        failure.map_or(Ok(()), Err)
    }
}

fn connection_key(spec: &AgentSpec, context: &ConnectionContext) -> String {
    let mut hash = Sha256::new();
    let mut part = |bytes: &[u8]| {
        hash.update(bytes.len().to_le_bytes());
        hash.update(bytes);
    };
    part(spec.id.as_bytes());
    part(spec.launch.command.as_os_str().as_encoded_bytes());
    part(&(spec.launch.args.len() as u64).to_le_bytes());
    for value in &spec.launch.args {
        part(value.as_bytes());
    }
    part(&(spec.launch.env.len() as u64).to_le_bytes());
    for (key, value) in &spec.launch.env {
        part(key.as_bytes());
        part(value.as_bytes());
    }
    part(&[u8::from(spec.launch_directory.is_some())]);
    if let Some(directory) = &spec.launch_directory {
        part(directory.as_os_str().as_encoded_bytes());
    }
    part(context.cwd.as_os_str().as_encoded_bytes());
    part(context.host.label().as_bytes());
    hex::encode(hash.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentSession, ChannelEvents, ConnectionInfo, DenyInteractions, SessionOptions};
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use synara_core::{AgentCapabilities, ConnectionId};
    use synara_runtime::{LaunchSpec, LocalHost};
    use tokio::sync::{mpsc, watch};

    struct FakeBackend(AtomicUsize);
    struct FakeConnection(watch::Sender<ConnectionInfo>);
    #[async_trait]
    impl AgentBackend for FakeBackend {
        async fn connect(
            &self,
            _spec: &AgentSpec,
            context: ConnectionContext,
        ) -> AgentResult<Arc<dyn AgentConnection>> {
            self.0.fetch_add(1, Ordering::SeqCst);
            tokio::task::yield_now().await;
            let (state, _) = watch::channel(ConnectionInfo {
                id: ConnectionId::new(),
                state: ConnectionState::Connected,
                identity: None,
                capabilities: AgentCapabilities::default(),
                authentication: vec![],
                host: context.host.label(),
                error: None,
            });
            Ok(Arc::new(FakeConnection(state)))
        }
    }
    #[async_trait]
    impl AgentConnection for FakeConnection {
        fn info(&self) -> ConnectionInfo {
            self.0.borrow().clone()
        }
        fn observe(&self) -> watch::Receiver<ConnectionInfo> {
            self.0.subscribe()
        }
        async fn new_session(
            &self,
            _options: SessionOptions,
        ) -> AgentResult<Arc<dyn AgentSession>> {
            Err(AgentError::Unsupported("fixture".into()))
        }
        async fn disconnect(&self) -> AgentResult<()> {
            self.0
                .send_modify(|state| state.state = ConnectionState::Disconnected);
            Ok(())
        }
    }

    #[tokio::test]
    async fn concurrent_callers_share_one_connection_and_restart_replaces_it() {
        let manager = ConnectionManager::default();
        let backend = FakeBackend(AtomicUsize::new(0));
        let (events, _receiver) = mpsc::channel(32);
        let context = ConnectionContext {
            host: Arc::new(LocalHost),
            cwd: std::env::temp_dir(),
            events: Arc::new(ChannelEvents(events)),
            interactions: Arc::new(DenyInteractions),
        };
        let spec = AgentSpec {
            launch_directory: None,
            id: "fixture".into(),
            name: "Fixture".into(),
            origin: "test".into(),
            launch: LaunchSpec::new("fixture"),
        };
        let (one, two) = tokio::join!(
            manager.connection(&backend, &spec, context.clone()),
            manager.connection(&backend, &spec, context.clone())
        );
        assert_eq!(one.unwrap().info().id, two.unwrap().info().id);
        assert_eq!(backend.0.load(Ordering::SeqCst), 1);
        manager.restart(&backend, &spec, context).await.unwrap();
        assert_eq!(backend.0.load(Ordering::SeqCst), 2);
        manager.disconnect_all().await.unwrap();
    }

    fn fixture() -> (AgentSpec, ConnectionContext) {
        let (events, _receiver) = mpsc::channel(32);
        (
            AgentSpec {
                launch_directory: None,
                id: "fixture".into(),
                name: "Fixture".into(),
                origin: "test".into(),
                launch: LaunchSpec::new("fixture"),
            },
            ConnectionContext {
                host: Arc::new(LocalHost),
                cwd: std::env::temp_dir(),
                events: Arc::new(ChannelEvents(events)),
                interactions: Arc::new(DenyInteractions),
            },
        )
    }

    struct HeldBackend {
        backend: FakeBackend,
        entered: tokio::sync::Notify,
        release: tokio::sync::Notify,
    }
    #[async_trait]
    impl AgentBackend for HeldBackend {
        async fn connect(
            &self,
            spec: &AgentSpec,
            context: ConnectionContext,
        ) -> AgentResult<Arc<dyn AgentConnection>> {
            let connection = self.backend.connect(spec, context).await?;
            self.entered.notify_one();
            self.release.notified().await;
            Ok(connection)
        }
    }

    #[tokio::test]
    async fn shutdown_waits_for_a_slot_being_connected_then_disconnects_it() {
        let manager = Arc::new(ConnectionManager::default());
        let backend = Arc::new(HeldBackend {
            backend: FakeBackend(AtomicUsize::new(0)),
            entered: tokio::sync::Notify::new(),
            release: tokio::sync::Notify::new(),
        });
        let (spec, context) = fixture();
        let connect = {
            let manager = manager.clone();
            let backend = backend.clone();
            let spec = spec.clone();
            let context = context.clone();
            tokio::spawn(async move { manager.connection(backend.as_ref(), &spec, context).await })
        };
        backend.entered.notified().await;
        let shutdown = manager.disconnect_all();
        tokio::pin!(shutdown);
        tokio::select! {
            biased;
            _ = &mut shutdown => panic!("shutdown detached a live connection acquisition"),
            () = tokio::task::yield_now() => {}
        }
        backend.release.notify_one();
        tokio::time::timeout(std::time::Duration::from_secs(1), &mut shutdown)
            .await
            .unwrap()
            .unwrap();
        let connection = connect.await.unwrap().unwrap();
        assert_eq!(connection.info().state, ConnectionState::Disconnected);
        assert!(manager.slots.lock().await.is_empty());
        let replacement = manager
            .connection(&backend.backend, &spec, context)
            .await
            .unwrap();
        assert_ne!(connection.info().id, replacement.info().id);
        assert_eq!(backend.backend.0.load(Ordering::SeqCst), 2);
        manager.disconnect_all().await.unwrap();
    }

    #[tokio::test]
    async fn invalid_restart_does_not_disconnect_the_owned_connection() {
        let manager = ConnectionManager::default();
        let backend = FakeBackend(AtomicUsize::new(0));
        let (mut spec, context) = fixture();
        let connection = manager
            .connection(&backend, &spec, context.clone())
            .await
            .unwrap();
        spec.name.clear();
        assert!(matches!(
            manager.restart(&backend, &spec, context).await,
            Err(AgentError::Invalid(_))
        ));
        assert_eq!(connection.info().state, ConnectionState::Connected);
        assert_eq!(backend.0.load(Ordering::SeqCst), 1);
        manager.disconnect_all().await.unwrap();
    }

    #[tokio::test]
    async fn authentication_required_connection_is_reused_without_a_second_process() {
        let manager = ConnectionManager::default();
        let backend = FakeBackend(AtomicUsize::new(0));
        let (spec, context) = fixture();
        let mut info = backend
            .connect(&spec, context.clone())
            .await
            .unwrap()
            .info();
        info.state = ConnectionState::AuthenticationRequired;
        let (state, _) = watch::channel(info);
        let connection = Arc::new(FakeConnection(state));
        *manager.slot(&spec, &context).await.unwrap().lock().await = Some(connection.clone());
        for expected in [
            ConnectionState::AuthenticationRequired,
            ConnectionState::Authenticating,
        ] {
            connection.0.send_modify(|info| info.state = expected);
            let reused = manager
                .connection(&backend, &spec, context.clone())
                .await
                .unwrap();
            assert_eq!(reused.info().id, connection.info().id);
            assert_eq!(backend.0.load(Ordering::SeqCst), 1);
        }
        manager.disconnect_all().await.unwrap();
    }
}
