//! The Rust port of `apps/server/src/backoff.ts`.

#![forbid(unsafe_code)]

// JS Math.min/Math.max return NaN when either operand is NaN; f64::min/max
// return the non-NaN operand. js_min/js_max keep the JS contract.
fn js_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.min(b)
    }
}

fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}

/// Port of `exponentialBackoffMs` (`apps/server/src/backoff.ts:7`).
pub fn exponential_backoff_ms(failures: f64, base_ms: f64, max_ms: f64) -> Result<f64, String> {
    let exponent = js_min(js_max(0.0, failures - 1.0), 30.0);
    Ok(js_min(max_ms, base_ms * 2f64.powf(exponent)))
}
