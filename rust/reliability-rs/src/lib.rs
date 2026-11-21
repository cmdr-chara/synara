//! JS bindings for `apps/analytics/reliability.ts`: generated from the signatures in `port.rs` by rust-anything.
//! Regenerate with `rust-anything refresh` after changing them.

use wasm_bindgen::prelude::*;

pub mod js;
pub mod port;
pub mod utf16;

/// `storeReliabilityRecords`.
async fn store_reliability_records_inner(
    db: String,
    records: String,
    received_day: String,
) -> Result<Vec<String>, String> {
    let db = crate::js::from_json(&db)?;
    let records = crate::js::from_json(&records)?;
    let out = port::store_reliability_records(db, records, received_day).await?;
    Ok(out)
}

#[wasm_bindgen(js_name = "storeReliabilityRecords")]
pub async fn store_reliability_records(
    db: String,
    records: String,
    received_day: String,
) -> Result<Vec<String>, JsError> {
    store_reliability_records_inner(db, records, received_day)
        .await
        .map_err(|e| JsError::new(&e))
}

/// `readReliabilityTotals`.
async fn read_reliability_totals_inner(db: String, url: String) -> Result<String, String> {
    let db = crate::js::from_json(&db)?;
    let url = crate::js::from_json(&url)?;
    let out = port::read_reliability_totals(db, url).await?;
    crate::js::to_json(&out)
}

#[wasm_bindgen(js_name = "readReliabilityTotals")]
pub async fn read_reliability_totals(db: String, url: String) -> Result<String, JsError> {
    read_reliability_totals_inner(db, url)
        .await
        .map_err(|e| JsError::new(&e))
}
