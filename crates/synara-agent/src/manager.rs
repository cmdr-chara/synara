use crate::{AgentBackend, AgentConnection, AgentError, AgentResult, AgentSpec, ConnectionContext};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, sync::Arc};
use synara_core::ConnectionState;
use tokio::sync::Mutex;

type Slot = Arc<Mutex<Option<Arc<dyn AgentConnection>>>>;

/// Owns shared connections. A prompt never implicitly creates another process.
#[derive(Default)]
pub struct ConnectionManager {
    slots: Mutex<HashMap<String, Slot>>,
}
impl ConnectionManager {
    pub async fn connection(
        &self,
        backend: &dyn AgentBackend,
        spec: &AgentSpec,
        context: ConnectionContext,
    ) -> AgentResult<Arc<dyn AgentConnection>> {
        spec.validate()?;
        let slot = self.slot(spec, &context).await?;
        let mut owned = slot.lock().await;
        if let Some(connection) = &*owned {
            if matches!(
                connection.info().state,
                ConnectionState::Connected | ConnectionState::Authenticating
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
}
