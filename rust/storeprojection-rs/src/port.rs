//! The Rust port of `apps/web/src/storeProjection.ts`.
#![forbid(unsafe_code)]

mod jv;
mod norm;
mod proj;

use jv::{decode, encode};
use serde_json::Value;

fn d(v: Value) -> jv::JV {
    decode(&v)
}
fn e(v: jv::JV) -> Value {
    encode(&v)
}
fn opt_to_jv(v: Option<Value>) -> jv::JV {
    match v {
        Some(x) => decode(&x),
        None => jv::JV::U,
    }
}

/// Port of `upsertProject` (`apps/web/src/storeProjection.ts:249`).
pub fn upsert_project(
    state: serde_json::Value,
    incoming: serde_json::Value,
    match_policy: String,
) -> Result<serde_json::Value, String> {
    let out = proj::upsert_project_jv(&d(state), &d(incoming), &jv::s(&match_policy))?;
    Ok(e(out))
}

/// Port of `upsertSpace` (`apps/web/src/storeProjection.ts:284`).
pub fn upsert_space(
    state: serde_json::Value,
    incoming: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let out = proj::upsert_space_jv(&d(state), &d(incoming))?;
    Ok(e(out))
}

/// Port of `removeSpace` (`apps/web/src/storeProjection.ts:302`).
pub fn remove_space(
    state: serde_json::Value,
    space_id: serde_json::Value,
    assignment_updated_at: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let out = proj::remove_space_jv(&d(state), &d(space_id), &opt_to_jv(assignment_updated_at))?;
    Ok(e(out))
}

/// Port of `applySpaceOrder` (`apps/web/src/storeProjection.ts:329`).
pub fn apply_space_order(
    state: serde_json::Value,
    ordered_space_ids: serde_json::Value,
    updated_at: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let out = proj::apply_space_order_jv(&d(state), &d(ordered_space_ids), &opt_to_jv(updated_at))?;
    Ok(e(out))
}

/// Port of `markThreadDetailSyncFailedInClientState` (`apps/web/src/storeProjection.ts:721`).
pub fn mark_thread_detail_sync_failed_in_client_state(
    state: serde_json::Value,
    thread_id: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let out = proj::mark_thread_detail_sync_failed(&d(state), &d(thread_id))?;
    Ok(e(out))
}

/// Port of `clearThreadDetailSyncFailureInClientState` (`apps/web/src/storeProjection.ts:733`).
pub fn clear_thread_detail_sync_failure_in_client_state(
    state: serde_json::Value,
    thread_id: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let out = proj::clear_thread_detail_sync_failure(&d(state), &d(thread_id))?;
    Ok(e(out))
}

/// Port of `evictThreadDetailFromClientState` (`apps/web/src/storeProjection.ts:966`).
pub fn evict_thread_detail_from_client_state(
    state: serde_json::Value,
    thread_id: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let out = proj::evict_thread_detail(&d(state), &d(thread_id))?;
    Ok(e(out))
}

/// Port of `removeDeletedThreadFromClientState` (`apps/web/src/storeProjection.ts:1118`).
pub fn remove_deleted_thread_from_client_state(
    state: serde_json::Value,
    thread_id: serde_json::Value,
    deleted_at_sequence: Option<f64>,
) -> Result<serde_json::Value, String> {
    let seq = deleted_at_sequence.map(jv::n).unwrap_or(jv::JV::U);
    let out = proj::remove_deleted_thread(&d(state), &d(thread_id), &seq)?;
    Ok(e(out))
}

/// Port of `removeDeletedProjectFromClientState` (`apps/web/src/storeProjection.ts:1177`).
pub fn remove_deleted_project_from_client_state(
    state: serde_json::Value,
    project_id: serde_json::Value,
    deleted_at_sequence: Option<f64>,
) -> Result<serde_json::Value, String> {
    let seq = deleted_at_sequence.map(jv::n).unwrap_or(jv::JV::U);
    let out = proj::remove_deleted_project(&d(state), &d(project_id), &seq)?;
    Ok(e(out))
}

/// Port of `syncServerShellSnapshot` (`apps/web/src/storeProjection.ts:1319`).
pub fn sync_server_shell_snapshot(
    state: serde_json::Value,
    snapshot: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let out = proj::sync_server_shell_snapshot_jv(&d(state), &d(snapshot))?;
    Ok(e(out))
}

/// Port of `syncServerThreadDetail` (`apps/web/src/storeProjection.ts:1437`).
pub fn sync_server_thread_detail(
    state: serde_json::Value,
    thread: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let out = proj::sync_server_thread_detail_jv(&d(state), &d(thread))?;
    Ok(e(out))
}

/// Port of `syncServerThreadDetailHotPath` (`apps/web/src/storeProjection.ts:1447`).
pub fn sync_server_thread_detail_hot_path(
    state: serde_json::Value,
    thread: serde_json::Value,
    snapshot_sequence: Option<f64>,
    history_revision_sequence: Option<f64>,
) -> Result<serde_json::Value, String> {
    let out = proj::sync_server_thread_detail_hot_path_jv(
        &d(state),
        &d(thread),
        &snapshot_sequence.map(jv::n).unwrap_or(jv::JV::U),
        &history_revision_sequence.map(jv::n).unwrap_or(jv::JV::U),
    )?;
    Ok(e(out))
}

/// Port of `applyShellEvent` (`apps/web/src/storeProjection.ts:1484`).
pub fn apply_shell_event(
    state: serde_json::Value,
    event: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let out = proj::apply_shell_event_jv(&d(state), &d(event))?;
    Ok(e(out))
}

/// Port of `syncServerReadModel` (`apps/web/src/storeProjection.ts:1520`).
pub fn sync_server_read_model(
    state: serde_json::Value,
    read_model: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let out = proj::sync_server_read_model_jv(&d(state), &d(read_model))?;
    Ok(e(out))
}
