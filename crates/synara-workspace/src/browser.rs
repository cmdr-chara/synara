//! Browser ownership is independent from conversation state. Only the native UI
//! receives the trusted session. Agents receive a revocable task-bound client.
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Instant,
};
use synara_browser::{
    session::{NativePort, Session},
    *,
};
use tokio_util::sync::CancellationToken;
pub(crate) mod mcp;
pub use synara_browser as browser_domain;
#[derive(Clone)]
pub struct BrowserService(Arc<Inner>);
struct Inner {
    session: Mutex<Session>,
    clock: Instant,
    clients: Mutex<HashMap<u128, (u128, CancellationToken)>>,
}
impl Default for BrowserService {
    fn default() -> Self {
        Self::new(Box::new(session::UnavailablePort))
    }
}
impl BrowserService {
    pub fn new(port: Box<dyn NativePort>) -> Self {
        Self(Arc::new(Inner {
            session: Mutex::new(Session::new(port)),
            clock: Instant::now(),
            clients: Mutex::new(HashMap::new()),
        }))
    }
    pub fn now(&self) -> u64 {
        self.0.clock.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
    }
    /// Trusted host/UI only. Not included in the MCP interface.
    pub fn with<T>(&self, f: impl FnOnce(&mut Session, u64) -> Result<T>) -> Result<T> {
        let mut session = self
            .0
            .session
            .lock()
            .map_err(|_| BrowserError::Unavailable)?;
        let now = self.now();
        session.tick(now);
        f(&mut session, now)
    }
    pub(crate) fn bind(&self, task: u128) -> Result<Client> {
        let mut clients = self
            .0
            .clients
            .lock()
            .map_err(|_| BrowserError::Unavailable)?;
        if clients.len() >= 128 && !clients.contains_key(&task) {
            return Err(BrowserError::Limit);
        }
        if let Some((_, old)) = clients.remove(&task) {
            old.cancel();
        }
        self.with(|s, _| {
            s.shutdown_task(task);
            s.open(BrowserProfile::AgentTask { task })
        })?;
        let cancel = CancellationToken::new();
        let generation = uuid::Uuid::new_v4().as_u128();
        clients.insert(task, (generation, cancel.clone()));
        Ok(Client {
            owner: self.clone(),
            task,
            cancel,
            generation,
        })
    }
    pub fn revoke(&self, task: u128) {
        if let Ok(mut clients) = self.0.clients.lock() {
            if let Some((_, c)) = clients.remove(&task) {
                c.cancel();
            }
            let _ = self.with(|s, _| {
                s.shutdown_task(task);
                Ok(())
            });
        }
    }
    fn revoke_generation(&self, task: u128, generation: u128) {
        if let Ok(mut clients) = self.0.clients.lock() {
            if clients.get(&task).is_some_and(|(g, _)| *g == generation) {
                if let Some((_, c)) = clients.remove(&task) {
                    c.cancel();
                }
                let _ = self.with(|s, _| {
                    s.shutdown_task(task);
                    Ok(())
                });
            }
        }
    }
    pub fn shutdown(&self) {
        if let Ok(mut clients) = self.0.clients.lock() {
            for (_, (_, c)) in clients.drain() {
                c.cancel();
            }
        }
        let _ = self.with(|s, _| {
            for t in s.tabs() {
                let _ = s.close(t.id);
            }
            Ok(())
        });
    }
}
#[derive(Clone)]
pub(crate) struct Client {
    owner: BrowserService,
    task: u128,
    cancel: CancellationToken,
    generation: u128,
}
impl Client {
    fn with<T>(&self, f: impl FnOnce(&mut Session, u64) -> Result<T>) -> Result<T> {
        self.owner.with(|s, now| {
            if self.cancel.is_cancelled() {
                return Err(BrowserError::WrongContext);
            }
            f(s, now)
        })
    }
    pub fn tabs(&self) -> Result<Vec<HostTabId>> {
        self.with(|s, _| Ok(s.task_tabs(self.task)))
    }
    pub fn request(&self, tab: HostTabId, op: BrowserOperation) -> Result<HostRequestId> {
        self.with(|s, n| s.request(self.task, tab, op, n))
    }
    pub fn result(&self, id: HostRequestId) -> Result<session::RequestView> {
        self.with(|s, _| s.result(self.task, id))
    }
    pub fn cancel(&self, id: HostRequestId) -> Result<()> {
        self.with(|s, n| s.cancel(self.task, id, n))
    }
    pub fn forget(&self, id: HostRequestId) -> Result<()> {
        self.with(|s, _| s.forget(self.task, id))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn revocation_is_permanent_for_old_client_even_after_reenrollment() {
        let service = BrowserService::default();
        let a = service.bind(7).unwrap();
        let b = service.bind(8).unwrap();
        assert_eq!(a.tabs().unwrap().len(), 1);
        service.revoke(7);
        assert!(a.tabs().is_err());
        assert_eq!(b.tabs().unwrap().len(), 1);
        let c = service.bind(7).unwrap();
        assert!(a.tabs().is_err());
        assert_eq!(c.tabs().unwrap().len(), 1);
        service.shutdown();
        assert!(b.tabs().is_err());
        assert!(c.tabs().is_err());
    }
}
