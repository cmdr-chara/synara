//! The Rust port of `packages/shared/src/formatBytes.ts`.
//!
//! Clean-room implementation (ADR-001): written from the recorded behavior in
//! `cases.jsonl` and the exported signature, not from the source body.

#![forbid(unsafe_code)]

/// Port of `formatBytes`: non-finite or sub-byte inputs print `0 B`; whole
/// bytes floor; KB rounds to one decimal without a trailing `.0`; MB and GB
/// keep `toFixed(1)` output.
pub fn format_bytes(bytes: f64) -> Result<String, String> {
    if !bytes.is_finite() || bytes < 1.0 {
        return Ok("0 B".to_string());
    }
    if bytes < 1024.0 {
        return Ok(format!("{} B", bytes.floor() as i64));
    }
    let mut v = bytes / 1024.0;
    if v < 1024.0 {
        let r = (v * 10.0).round() / 10.0;
        return Ok(if r.fract() == 0.0 {
            format!("{} KB", r as i64)
        } else {
            format!("{r:.1} KB")
        });
    }
    v /= 1024.0;
    if v < 1024.0 {
        return Ok(format!("{} MB", fmt1(v)));
    }
    v /= 1024.0;
    Ok(format!("{} GB", fmt1(v)))
}

/// `v.toFixed(1)`, except JS switches `toFixed` out at `1e21` where the
/// source prints `String(v)`'s exponential form instead.
fn fmt1(v: f64) -> String {
    if v.abs() < 1e21 {
        return format!("{v:.1}");
    }
    let s = serde_json::to_string(&v).unwrap_or_default();
    match s.find('e') {
        Some(i) => {
            let (m, e) = s.split_at(i);
            match e[1..].strip_prefix('-') {
                Some(d) => format!("{m}e-{d}"),
                None => format!("{m}e+{}", e[1..].trim_start_matches('+')),
            }
        }
        None => s,
    }
}
