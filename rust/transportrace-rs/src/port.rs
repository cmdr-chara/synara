//! The Rust port of `packages/shared/src/transportRace.ts`.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::Value;

const TRANSPORT_PREFERENCE: [&str; 6] =
    ["loopback", "lan", "tailscale", "ssh", "cloudflare", "relay"];

/// Port of `transportPreferenceRank` (`packages/shared/src/transportRace.ts:44`).
pub fn transport_preference_rank(kind: String) -> Result<f64, String> {
    Ok(TRANSPORT_PREFERENCE
        .iter()
        .position(|k| *k == kind)
        .map(|i| i as f64)
        .unwrap_or(TRANSPORT_PREFERENCE.len() as f64))
}

fn rank_of(candidate: &Value) -> Result<f64, String> {
    let kind = match candidate {
        Value::Null => return Err("Cannot read properties of null (reading 'kind')".into()),
        _ if js::is_undefined(candidate) => {
            return Err("Cannot read properties of undefined (reading 'kind')".into());
        }
        Value::Object(o) => o.get("kind"),
        _ => None,
    };
    Ok(match kind {
        Some(Value::String(s)) => TRANSPORT_PREFERENCE
            .iter()
            .position(|k| *k == *s)
            .map(|i| i as f64)
            .unwrap_or(TRANSPORT_PREFERENCE.len() as f64),
        _ => TRANSPORT_PREFERENCE.len() as f64,
    })
}

/// Port of `sortByTransportPreference` (`packages/shared/src/transportRace.ts:53`).
pub fn sort_by_transport_preference(
    candidates: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    let items = match &candidates {
        Value::Array(a) => a,
        Value::Null => return Err("Cannot read properties of null (reading 'map')".into()),
        _ if js::is_undefined(&candidates) => {
            return Err("Cannot read properties of undefined (reading 'map')".into());
        }
        _ => return Err("candidates.map is not a function".into()),
    };
    let mut keyed: Vec<(usize, Value, f64)> = Vec::with_capacity(items.len());
    for (index, candidate) in items.iter().enumerate() {
        keyed.push((index, candidate.clone(), rank_of(candidate)?));
    }
    keyed.sort_by(|a, b| a.2.total_cmp(&b.2).then(a.0.cmp(&b.0)));
    Ok(keyed.into_iter().map(|(_, c, _)| c).collect())
}
