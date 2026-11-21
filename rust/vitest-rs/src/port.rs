//! The Rust port of `vitest.shards.ts`.
//!
//! Clean-room port: the source returns a class; the wire keeps it as the recorded
//! `{"$function":""}` marker — the sequencer's `shard` logic stays on the JS side.

#![forbid(unsafe_code)]

/// Port of `createPinnedShardSequencer` (`vitest.shards.ts:4`).
pub fn create_pinned_shard_sequencer(
    pins_by_count: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let _ = &pins_by_count;
    Ok(serde_json::json!({ "$function": "" }))
}
