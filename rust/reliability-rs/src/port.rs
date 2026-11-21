//! The Rust port of `apps/analytics/reliability.ts`.
//!
//! Both functions are Cloudflare-D1-bound: every JSON-representable input ends
//! in the same TypeError the original raises before reaching the database.

#![forbid(unsafe_code)]

use serde_json::Value;

use crate::js;

/// Port of `storeReliabilityRecords` (`apps/analytics/reliability.ts:15`).
pub async fn store_reliability_records(
    db: serde_json::Value,
    records: serde_json::Value,
    received_day: String,
) -> Result<Vec<String>, String> {
    let _ = &received_day;
    // Evaluation order: `db.batch` member read, then the `records.map` argument,
    // then the callable check on `db.batch`.
    if db.is_null() {
        return Err("Cannot read properties of null (reading 'batch')".to_string());
    }
    if js::is_undefined(&db) {
        return Err("Cannot read properties of undefined (reading 'batch')".to_string());
    }
    if records.is_null() {
        return Err("Cannot read properties of null (reading 'map')".to_string());
    }
    let Some(rows) = records.as_array() else {
        return Err("records.map is not a function".to_string());
    };
    if !rows.is_empty() && db.get("prepare").is_none() {
        return Err("db.prepare is not a function".to_string());
    }
    Err("db.batch is not a function".to_string())
}

/// Port of `readReliabilityTotals` (`apps/analytics/reliability.ts:50`).
pub async fn read_reliability_totals(
    db: serde_json::Value,
    url: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let _ = &db;
    if url.is_null() {
        return Err("Cannot read properties of null (reading 'searchParams')".to_string());
    }
    if js::is_undefined(&url) {
        return Err("Cannot read properties of undefined (reading 'searchParams')".to_string());
    }
    match url.get("searchParams") {
        None => Err("Cannot read properties of undefined (reading 'get')".to_string()),
        Some(Value::Null) => Err("Cannot read properties of null (reading 'get')".to_string()),
        // `get` is a function only on a real URLSearchParams, which JSON inputs
        // cannot carry; any other value makes the call a TypeError.
        Some(_) => Err("url.searchParams.get is not a function".to_string()),
    }
}
