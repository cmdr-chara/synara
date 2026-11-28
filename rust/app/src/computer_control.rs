//! Per-thread computer control state, exact port of upstream
//! computer/ComputerControlState.ts. The generation is the security
//! primitive: it increments only when control is disabled, so consent a
//! client recorded against generation N stops matching the moment the owner
//! turns control off — stale queued consent cannot survive a disable. A
//! corrupt state file fails closed: control stays disabled everywhere while
//! ordinary server operation continues.

use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Debug)]
pub struct ThreadControlState {
    pub disabled: bool,
    pub generation: i64,
    pub chat_generation: Option<i64>,
}

struct ControlFile {
    threads: HashMap<String, ThreadControlState>,
    load_error: bool,
}

fn control_path() -> std::path::PathBuf {
    std::path::Path::new(&crate::todo::state_dir()).join("computer-control.json")
}

fn state() -> &'static Mutex<ControlFile> {
    static STATE: OnceLock<Mutex<ControlFile>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(load()))
}

fn load() -> ControlFile {
    let path = control_path();
    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(_) => {
            return ControlFile {
                threads: HashMap::new(),
                load_error: false,
            };
        }
    };
    let parsed: serde_json::Result<Value> = serde_json::from_str(&raw);
    let mut threads = HashMap::new();
    let valid = (|| -> bool {
        let Ok(Value::Object(map)) = parsed.as_ref().map_err(|_| ()).or(Err(())) else {
            return false;
        };
        for (thread, value) in map {
            let disabled = value.get("disabled").and_then(Value::as_bool);
            let generation = value.get("generation").and_then(Value::as_i64);
            let chat = value.get("chatGeneration").and_then(Value::as_i64);
            match (disabled, generation) {
                (Some(disabled), Some(generation)) if generation >= 0 => {
                    if chat.is_some_and(|c| c < 0) {
                        return false;
                    }
                    threads.insert(
                        thread.clone(),
                        ThreadControlState {
                            disabled,
                            generation,
                            chat_generation: chat,
                        },
                    );
                }
                _ => return false,
            }
        }
        true
    })();
    if valid {
        ControlFile {
            threads,
            load_error: false,
        }
    } else {
        // A broken consent file disables Computer, not ordinary server boot.
        ControlFile {
            threads: HashMap::new(),
            load_error: true,
        }
    }
}

pub fn get(thread_id: &str) -> ThreadControlState {
    let guard = state().lock().unwrap();
    if guard.load_error {
        return ThreadControlState {
            disabled: true,
            generation: 0,
            chat_generation: None,
        };
    }
    guard
        .threads
        .get(thread_id)
        .cloned()
        .unwrap_or(ThreadControlState {
            disabled: false,
            generation: 0,
            chat_generation: None,
        })
}

pub fn allows(thread_id: &str, generation: i64) -> bool {
    let state = get(thread_id);
    !state.disabled && generation == state.generation
}

fn persist(guard: &ControlFile) -> Result<(), String> {
    let path = control_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let map: serde_json::Map<String, Value> = guard
        .threads
        .iter()
        .map(|(thread, state)| {
            let mut entry = serde_json::json!({
                "disabled": state.disabled,
                "generation": state.generation,
            });
            if let Some(chat) = state.chat_generation {
                entry["chatGeneration"] = serde_json::json!(chat);
            }
            (thread.clone(), entry)
        })
        .collect();
    let body = serde_json::to_string(&Value::Object(map)).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, body).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
    }
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    Ok(())
}

/// Enable or disable control for a thread. Enabling when already enabled is a
/// no-op upstream — the generation must NOT move, or consent recorded moments
/// ago would silently freeze. Disabling always increments.
pub fn set(thread_id: &str, disabled: bool) -> Result<i64, String> {
    let mut guard = state().lock().unwrap();
    if guard.load_error {
        return Err(
            "Computer authorization state could not be loaded; control remains disabled."
                .to_string(),
        );
    }
    let current = guard
        .threads
        .get(thread_id)
        .cloned()
        .unwrap_or(ThreadControlState {
            disabled: false,
            generation: 0,
            chat_generation: None,
        });
    if !disabled && !current.disabled {
        return Ok(current.generation);
    }
    let generation = current.generation + if disabled { 1 } else { 0 };
    guard.threads.insert(
        thread_id.to_string(),
        ThreadControlState {
            disabled,
            generation,
            chat_generation: None,
        },
    );
    persist(&guard)?;
    Ok(generation)
}

/// Records which generation a chat-level enable intent was captured against.
/// The intent only sticks when the generation still matches — recording it
/// for a generation control has since moved past is meaningless.
pub fn record_chat_intent(thread_id: &str, enabled: bool, generation: i64) -> Result<(), String> {
    let mut guard = state().lock().unwrap();
    if guard.load_error {
        return if enabled {
            Err(
                "Computer authorization state could not be loaded; control remains disabled."
                    .to_string(),
            )
        } else {
            Ok(())
        };
    }
    let previous = guard
        .threads
        .get(thread_id)
        .cloned()
        .unwrap_or(ThreadControlState {
            disabled: false,
            generation: 0,
            chat_generation: None,
        });
    let chat_generation = if enabled && !previous.disabled && generation == previous.generation {
        Some(generation)
    } else {
        None
    };
    if previous.chat_generation == chat_generation {
        return Ok(());
    }
    guard.threads.insert(
        thread_id.to_string(),
        ThreadControlState {
            chat_generation,
            ..previous
        },
    );
    persist(&guard)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_thread() -> String {
        format!("test-{}", crate::orchestration_engine::uuid_v4())
    }

    #[test]
    fn generation_freezes_only_on_disable() {
        let thread = unique_thread();
        let base = get(&thread).generation;
        // Enabling when enabled is a no-op for the generation.
        let g1 = set(&thread, false).unwrap();
        assert_eq!(g1, base);
        // Disabling increments.
        let g2 = set(&thread, true).unwrap();
        assert_eq!(g2, base + 1);
        // Re-enabling does not increment further.
        let g3 = set(&thread, false).unwrap();
        assert_eq!(g3, base + 1);
        let _ = set(&thread, true);
    }

    #[test]
    fn allows_requires_matching_generation() {
        let thread = unique_thread();
        let generation = get(&thread).generation;
        assert!(allows(&thread, generation));
        assert!(!allows(&thread, generation + 1));
        let _ = set(&thread, true);
        assert!(!allows(&thread, generation + 1));
        let _ = set(&thread, false);
    }

    #[test]
    fn chat_intent_sticks_only_for_current_generation() {
        let thread = unique_thread();
        let generation = get(&thread).generation;
        record_chat_intent(&thread, true, generation).unwrap();
        assert_eq!(get(&thread).chat_generation, Some(generation));
        // A stale generation cannot record intent.
        record_chat_intent(&thread, true, generation + 5).unwrap();
        assert_eq!(get(&thread).chat_generation, None);
    }
}
