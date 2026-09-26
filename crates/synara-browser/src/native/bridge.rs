//! Bounded, revocable handoff. Native widgets never cross the UI thread.
use super::*;
use std::sync::{Mutex, mpsc};

pub(super) struct Shared {
    pub ready: AtomicBool,
    state: Mutex<State>,
}
#[derive(Default)]
struct State {
    epochs: BTreeMap<HostTabId, u64>,
    requests: BTreeMap<HostRequestId, Arc<AtomicBool>>,
}
pub(super) struct Delivery {
    pub command: Command,
    pub epoch: u64,
    pub cancelled: Arc<AtomicBool>,
}
pub struct Port {
    sender: mpsc::SyncSender<Delivery>,
    shared: Arc<Shared>,
}
impl NativePort for Port {
    fn capabilities(&self) -> Capabilities {
        let ready = self.shared.ready.load(Ordering::Acquire);
        Capabilities {
            navigation: ready,
            document: ready,
            input: ready,
            capture: false,
            downloads: false,
        }
    }
    fn send(&mut self, command: Command) -> Result<()> {
        let mut state = self
            .shared
            .state
            .lock()
            .map_err(|_| BrowserError::Unavailable)?;
        let cancelled = Arc::new(AtomicBool::new(false));
        let mut epoch = 0;
        match &command {
            Command::Open { tab, .. } => {
                if state.epochs.len() >= MAX_TABS {
                    return Err(BrowserError::Limit);
                }
                state.epochs.insert(*tab, 0);
            }
            Command::Close { tab } => {
                state.epochs.remove(tab);
            }
            Command::DismissPopup { tab } => {
                epoch = *state.epochs.get(tab).ok_or(BrowserError::MissingTab)?;
            }
            Command::Navigate { tab, .. } | Command::Stop { tab } => {
                let value = state.epochs.get_mut(tab).ok_or(BrowserError::MissingTab)?;
                *value = value.checked_add(1).ok_or(BrowserError::Limit)?;
                epoch = *value;
            }
            Command::Operation {
                request, command, ..
            } => {
                if state.requests.len() >= MAX_PENDING {
                    return Err(BrowserError::Limit);
                }
                epoch = *state
                    .epochs
                    .get(&command.tab)
                    .ok_or(BrowserError::MissingTab)?;
                state.requests.insert(*request, cancelled.clone());
            }
            Command::Cancel { request } => {
                if let Some(flag) = state.requests.get(request) {
                    flag.store(true, Ordering::Release);
                }
                // Cancellation does not depend on finding room in the command queue.
                return Ok(());
            }
        }
        // Stop/close epoch changes are visible even if the queue is full.
        let stop = matches!(command, Command::Close { .. } | Command::Stop { .. });
        let opened = match &command {
            Command::Open { tab, .. } => Some(*tab),
            _ => None,
        };
        let request = match &command {
            Command::Operation { request, .. } => Some(*request),
            _ => None,
        };
        match self.sender.try_send(Delivery {
            command,
            epoch,
            cancelled,
        }) {
            Ok(()) => Ok(()),
            Err(_) if stop => Ok(()),
            Err(_) => {
                if let Some(tab) = opened {
                    state.epochs.remove(&tab);
                }
                if let Some(id) = request {
                    state.requests.remove(&id);
                }
                Err(BrowserError::Limit)
            }
        }
    }
}
impl Shared {
    pub fn epoch(&self, tab: HostTabId) -> Option<u64> {
        self.state.lock().ok()?.epochs.get(&tab).copied()
    }
    pub fn finish(&self, request: HostRequestId) {
        if let Ok(mut state) = self.state.lock() {
            state.requests.remove(&request);
        }
    }
}
pub(super) fn channel() -> (Port, mpsc::Receiver<Delivery>, Arc<Shared>) {
    let (sender, receiver) = mpsc::sync_channel(128);
    let shared = Arc::new(Shared {
        ready: AtomicBool::new(false),
        state: Mutex::new(State::default()),
    });
    (
        Port {
            sender,
            shared: shared.clone(),
        },
        receiver,
        shared,
    )
}
