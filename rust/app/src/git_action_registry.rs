//! Registry for streaming git actions, ported from upstream
//! `git/gitActionRunner.ts`. Actions are keyed by actionId; a client that
//! reconnects and reissues the same request (resume) attaches to the
//! in-flight or completed action instead of duplicating the mutation.
//! Mutations run detached: client disconnect never cancels the git work.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use serde_json::Value;

const COMPLETED_ACTIONS_TO_KEEP: usize = 256;

pub struct ActionEntry {
    pub fingerprint: String,
    // Full ordered event history. Replaying it converges to the same client
    // state as upstream's structural compaction because events are ordered.
    pub events: Vec<Value>,
    // Some(Ok(())) on success, Some(Err(message)) on failure.
    pub outcome: Option<Result<(), String>>,
    pub subscribers: Vec<tokio::sync::mpsc::UnboundedSender<Value>>,
}

pub struct Registry {
    actions: HashMap<String, ActionEntry>,
    completed_order: Vec<String>,
}

fn registry() -> &'static Mutex<Registry> {
    static R: OnceLock<Mutex<Registry>> = OnceLock::new();
    R.get_or_init(|| {
        Mutex::new(Registry {
            actions: HashMap::new(),
            completed_order: Vec::new(),
        })
    })
}

/// sha256 of the stable (key-sorted) JSON of the input minus the transport
/// flags, matching upstream's fingerprint contract.
pub fn fingerprint(payload: &Value) -> String {
    let mut command = payload.clone();
    if let Some(obj) = command.as_object_mut() {
        obj.remove("resume");
        obj.remove("recoverable");
    }
    let stable = stable_json(&command);
    let digest = md5_like_sha256(stable.as_bytes());
    digest
}

fn stable_json(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let inner: Vec<String> = keys
                .into_iter()
                .map(|k| {
                    format!(
                        "{}:{}",
                        serde_json::to_string(k).unwrap(),
                        stable_json(&map[k])
                    )
                })
                .collect();
            format!("{{{}}}", inner.join(","))
        }
        Value::Array(items) => {
            let inner: Vec<String> = items.iter().map(stable_json).collect();
            format!("[{}]", inner.join(","))
        }
        other => serde_json::to_string(other).unwrap_or_else(|_| "null".to_string()),
    }
}

// Minimal sha256 implementation (no new dependencies for one hash).
fn md5_like_sha256(data: &[u8]) -> String {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    sha2::Digest::update(&mut hasher, data);
    hex::encode_sha2_digest(hasher.finalize())
}

mod hex {
    pub fn encode_sha2_digest(bytes: impl AsRef<[u8]>) -> String {
        bytes.as_ref().iter().map(|b| format!("{b:02x}")).collect()
    }
}

pub enum Attach {
    // A brand-new action: caller spawns the work. The subscriber is
    // registered atomically so no event published after this point is missed.
    New {
        rx: tokio::sync::mpsc::UnboundedReceiver<Value>,
    },
    // Attach to an existing action: replay these events, then either finish
    // with the outcome or forward live events through the subscriber channel.
    Existing {
        events: Vec<Value>,
        outcome: Option<Result<(), String>>,
        subscriber: Option<tokio::sync::mpsc::UnboundedReceiver<Value>>,
    },
    Mismatch,
    ResumeMissing,
}

pub fn attach(action_id: &str, fp: &str, resume: bool) -> Attach {
    let mut reg = registry().lock().expect("registry");
    match reg.actions.get_mut(action_id) {
        None => {
            if resume {
                return Attach::ResumeMissing;
            }
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            reg.actions.insert(
                action_id.to_string(),
                ActionEntry {
                    fingerprint: fp.to_string(),
                    events: Vec::new(),
                    outcome: None,
                    subscribers: vec![tx],
                },
            );
            Attach::New { rx }
        }
        Some(entry) => {
            if entry.fingerprint != fp {
                return Attach::Mismatch;
            }
            let events = entry.events.clone();
            if let Some(outcome) = &entry.outcome {
                return Attach::Existing {
                    events,
                    outcome: Some(outcome.clone()),
                    subscriber: None,
                };
            }
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            entry.subscribers.push(tx);
            Attach::Existing {
                events,
                outcome: None,
                subscriber: Some(rx),
            }
        }
    }
}

/// Publish one progress event: append to history and fan out live.
pub fn publish(action_id: &str, event: &Value) {
    let mut reg = registry().lock().expect("registry");
    if let Some(entry) = reg.actions.get_mut(action_id) {
        entry.events.push(event.clone());
        entry
            .subscribers
            .retain(|tx| tx.send(event.clone()).is_ok());
    }
}

/// Terminal outcome: wakes remaining subscribers with nothing further; the
/// outcome itself is replayed to late attachers.
pub fn finish(action_id: &str, outcome: Result<(), String>) {
    let mut reg = registry().lock().expect("registry");
    if let Some(entry) = reg.actions.get_mut(action_id) {
        entry.outcome = Some(outcome);
        entry.subscribers.clear();
        reg.completed_order.push(action_id.to_string());
        while reg.completed_order.len() > COMPLETED_ACTIONS_TO_KEEP {
            let oldest = reg.completed_order.remove(0);
            reg.actions.remove(&oldest);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_ignores_transport_flags() {
        let a = serde_json::json!({"actionId":"x","cwd":"/r","kind":"commit","resume":true});
        let b = serde_json::json!({"actionId":"x","cwd":"/r","kind":"commit"});
        assert_eq!(fingerprint(&a), fingerprint(&b));
    }

    #[test]
    fn fingerprint_stable_key_order() {
        let a = serde_json::json!({"a":1,"b":{"y":2,"x":3}});
        let b = serde_json::json!({"b":{"x":3,"y":2},"a":1});
        assert_eq!(fingerprint(&a), fingerprint(&b));
    }

    #[test]
    fn attach_resume_and_mismatch() {
        let id = format!("test-{}", std::process::id());
        match attach(&id, "fp1", false) {
            Attach::New { .. } => {}
            _ => panic!("first attach must be New"),
        }
        match attach(&id, "fp2", false) {
            Attach::Mismatch => {}
            _ => panic!("different fingerprint must mismatch"),
        }
        match attach(&id, "fp1", true) {
            Attach::Existing { .. } => {}
            _ => panic!("resume with same fingerprint must attach"),
        }
        publish(
            &id,
            &serde_json::json!({"kind":"phase_started","phase":"commit"}),
        );
        match attach(&id, "fp1", true) {
            Attach::Existing {
                events, outcome, ..
            } => {
                assert_eq!(events.len(), 1);
                assert!(outcome.is_none());
            }
            _ => panic!("expected existing"),
        }
        finish(&id, Ok(()));
        match attach(&id, "fp1", true) {
            Attach::Existing { outcome, .. } => assert!(matches!(outcome, Some(Ok(())))),
            _ => panic!("expected completed outcome"),
        }
        let missing = format!("missing-{}", std::process::id());
        assert!(matches!(
            attach(&missing, "fp", true),
            Attach::ResumeMissing
        ));
    }
}
