//! The Rust port of `apps/desktop/src/cuaFixtures/focusProbe.ts` and
//! `apps/desktop/src/cuaFixtures/live.ts`.
//!
//! `Err` mirrors a thrown JS error. Recorded marker objects (`{"$num": ...}`,
//! `{"$undefined": true}`) decode to their JS values inside comparisons and
//! re-encode on output, mirroring the bridge.

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};
use std::collections::HashMap;

/// A decoded JS value for comparison purposes.
#[derive(Clone, Debug)]
enum Jv<'a> {
    Undef,
    Null,
    Num(f64),
    Str(&'a str),
    Bool(bool),
    Other(&'a Value),
}

fn jv(v: Option<&Value>) -> Jv<'_> {
    match v {
        None => Jv::Undef,
        Some(Value::Null) => Jv::Null,
        Some(Value::Number(n)) => Jv::Num(n.as_f64().unwrap_or(f64::NAN)),
        Some(Value::String(s)) => Jv::Str(s.as_str()),
        Some(Value::Bool(b)) => Jv::Bool(*b),
        Some(Value::Object(m)) if m.len() == 1 && m.get("$undefined").is_some() => Jv::Undef,
        Some(Value::Object(m)) if m.len() == 1 => match m.get("$num").and_then(Value::as_str) {
            Some("NaN") => Jv::Num(f64::NAN),
            Some("Infinity") => Jv::Num(f64::INFINITY),
            Some("-Infinity") => Jv::Num(f64::NEG_INFINITY),
            Some("-0") => Jv::Num(-0.0),
            _ => Jv::Other(v.unwrap()),
        },
        Some(other) => Jv::Other(other),
    }
}

fn js_eq(a: Jv<'_>, b: Jv<'_>) -> bool {
    match (a, b) {
        (Jv::Undef, Jv::Undef) | (Jv::Null, Jv::Null) => true,
        (Jv::Num(x), Jv::Num(y)) => {
            if x.is_nan() || y.is_nan() {
                false
            } else {
                x == y || (x == 0.0 && y == 0.0)
            }
        }
        (Jv::Str(x), Jv::Str(y)) => x == y,
        (Jv::Bool(x), Jv::Bool(y)) => x == y,
        (Jv::Other(x), Jv::Other(y)) => x == y,
        _ => false,
    }
}

/// Canonical JS-Map key text (SameValueZero: NaN keys equal, -0 keys equal 0).
fn jv_key(v: &Jv<'_>) -> String {
    match v {
        Jv::Undef => "u".to_string(),
        Jv::Null => "null".to_string(),
        Jv::Num(n) => {
            if n.is_nan() {
                "num:NaN".to_string()
            } else if *n == 0.0 {
                "num:0".to_string()
            } else if n.is_infinite() {
                format!("num:{}Infinity", if *n > 0.0 { "" } else { "-" })
            } else {
                format!("num:{}", n)
            }
        }
        Jv::Str(s) => format!("str:{}", s),
        Jv::Bool(b) => format!("bool:{}", b),
        Jv::Other(v) => format!("o:{}", serde_json::to_string(v).unwrap_or_default()),
    }
}

/// Re-encode a numeric value the way the bridge writes it.
fn num_out(n: f64) -> Value {
    if n.is_nan() {
        json!({"$num": "NaN"})
    } else if n == f64::INFINITY {
        json!({"$num": "Infinity"})
    } else if n == f64::NEG_INFINITY {
        json!({"$num": "-Infinity"})
    } else if n == 0.0 {
        if n.is_sign_negative() {
            json!({"$num": "-0"})
        } else {
            json!(0)
        }
    } else {
        json!(n)
    }
}

/// `optionalNumber`: number && finite, else null.
fn opt_finite(v: Option<&Value>) -> Option<f64> {
    match jv(v) {
        Jv::Num(n) if n.is_finite() => Some(n),
        _ => None,
    }
}

fn opt_str(v: Option<&Value>) -> Option<&str> {
    match v {
        Some(Value::String(s)) => Some(s),
        _ => None,
    }
}

const THEFT_FIELDS: [&str; 5] = ["pid", "keyWin", "topWin", "space", "focusedPid"];
const COMPARED_FIELDS: [&str; 6] = ["pid", "keyWin", "topWin", "space", "focusedPid", "focused"];

/// Port of `parseFocusProbeLine` (`apps/desktop/src/cuaFixtures/focusProbe.ts`).
pub fn parse_focus_probe_line(line: String) -> Result<Option<serde_json::Value>, String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let value: Value = match serde_json::from_str(trimmed) {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };
    let obj = match &value {
        Value::Object(m) => m,
        Value::Array(_) => return Ok(None),
        _ => return Ok(None),
    };
    // `typeof value.t === "number"` — includes decoded NaN/Infinity markers.
    let t_is_num = matches!(jv(obj.get("t")), Jv::Num(_));
    let num = |k: &str| opt_finite(obj.get(k)).map(num_out).unwrap_or(Value::Null);
    match obj.get("kind").and_then(Value::as_str) {
        Some("meta") => Ok(Some(json!({
            "kind": "meta",
            "meta": {
                "pid": opt_finite(obj.get("pid")).map(num_out).unwrap_or(json!(-1)),
                "hz": num("hz"),
                "axTrusted": obj.get("axTrusted") == Some(&Value::Bool(true)),
                "axWindowSymbol": obj.get("axWindowSymbol") == Some(&Value::Bool(true)),
                "slsSpace": obj.get("slsSpace") == Some(&Value::Bool(true)),
                "stdinWatch": obj.get("stdinWatch") == Some(&Value::Bool(true)),
                "label": opt_str(obj.get("label")).map(Value::from).unwrap_or(Value::Null),
                "startedAt": num("startedAt"),
            }
        }))),
        Some("done") => Ok(Some(json!({
            "kind": "done",
            "done": {
                "samples": num("samples"),
                "elapsedMs": num("elapsedMs"),
                "overruns": num("overruns"),
                "stoppedBy": opt_str(obj.get("stoppedBy")).unwrap_or("unknown"),
            }
        }))),
        _ if t_is_num => Ok(Some(json!({
            "kind": "sample",
            "sample": {
                "t": obj.get("t").unwrap().clone(),
                "pid": num("pid"),
                "app": opt_str(obj.get("app")).map(Value::from).unwrap_or(Value::Null),
                "keyWin": num("keyWin"),
                "keyTitle": opt_str(obj.get("keyTitle")).map(Value::from).unwrap_or(Value::Null),
                "topWin": num("topWin"),
                "topTitle": opt_str(obj.get("topTitle")).map(Value::from).unwrap_or(Value::Null),
                "space": num("space"),
                "focusedPid": num("focusedPid"),
                "focused": opt_str(obj.get("focused")).map(Value::from).unwrap_or(Value::Null),
            }
        }))),
        _ => Ok(None),
    }
}

fn observed_of(sample: &Value) -> Value {
    json!({
        "pid": sample.get("pid").cloned().unwrap_or(Value::Null),
        "app": sample.get("app").cloned().unwrap_or(Value::Null),
        "keyWin": sample.get("keyWin").cloned().unwrap_or(Value::Null),
        "topWin": sample.get("topWin").cloned().unwrap_or(Value::Null),
        "space": sample.get("space").cloned().unwrap_or(Value::Null),
        "focusedPid": sample.get("focusedPid").cloned().unwrap_or(Value::Null),
        "focused": sample.get("focused").cloned().unwrap_or(Value::Null),
    })
}

fn same_observed(a: &Value, b: &Value) -> bool {
    ["pid", "keyWin", "topWin", "space", "focusedPid", "focused"]
        .iter()
        .all(|f| js_eq(jv(a.get(*f)), jv(b.get(*f))))
}

/// Modal (most frequent; ties broken by latest index) non-null value per field.
/// Returns baseline entries as raw Values; `None` entries represent JS
/// `undefined` (serialized as `{"$undefined": true}` marker output).
fn derive_baseline(samples: &[&Value], expect: Option<&Value>) -> Vec<(String, Option<Value>)> {
    let mut baseline: Vec<(String, Option<Value>)> = Vec::new();
    for field in COMPARED_FIELDS {
        if let Some(exp) = expect.and_then(|e| e.get(field))
            && !matches!(jv(Some(exp)), Jv::Undef)
        {
            baseline.push((field.to_string(), Some(exp.clone())));
            continue;
        }
        let mut counts: HashMap<String, (Option<Value>, u64, i64)> = HashMap::new();
        let mut order: Vec<String> = Vec::new();
        for (index, sample) in samples.iter().enumerate() {
            let v = jv(sample.get(field).or({
                // `sample[field]` when the key is absent is `undefined`.
                None
            }));
            if matches!(v, Jv::Null) {
                continue;
            }
            let key = jv_key(&v);
            match counts.get_mut(&key) {
                Some((_, count, last)) => {
                    *count += 1;
                    *last = index as i64;
                }
                None => {
                    // JS Map keys normalize -0 to +0, so the modal value is
                    // the normalized key, not the raw sample value.
                    let raw = match v {
                        Jv::Undef => None,
                        Jv::Num(n) => Some(if n == 0.0 { json!(0) } else { num_out(n) }),
                        _ => sample.get(field).cloned(),
                    };
                    counts.insert(key.clone(), (raw, 1, index as i64));
                    order.push(key);
                }
            }
        }
        let mut best: Option<Option<Value>> = None;
        let mut best_count = 0u64;
        let mut best_last = -1i64;
        for key in &order {
            let (raw, count, last) = &counts[key];
            if *count > best_count || (*count == best_count && *last > best_last) {
                best = Some(raw.clone());
                best_count = *count;
                best_last = *last;
            }
        }
        match best {
            Some(raw) => baseline.push((field.to_string(), raw)),
            None => baseline.push((field.to_string(), Some(Value::Null))),
        }
    }
    baseline
}

/// Port of `analyzeFocusSamples` (`apps/desktop/src/cuaFixtures/focusProbe.ts`).
pub fn analyze_focus_samples(
    all_samples: Vec<serde_json::Value>,
    options: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    // The wire collapses JS `null` and `undefined` to `None`, and the default
    // parameter makes `undefined` behave as `{}`. `options: null` therefore
    // cannot throw `Cannot read properties of null` — a structural known-diff.
    let _ = &options;
    let samples: Vec<&Value> = all_samples.iter().collect();
    let opts: Option<&Value> = options.as_ref().filter(|o| o.is_object());
    let get_opt = |k: &str| opts.and_then(|o| o.get(k));

    // `options.settleMs ?? 0` — only nullish defaults to 0; other non-numbers
    // coerce to NaN (every comparison false).
    let settle_ms = match jv(get_opt("settleMs")) {
        Jv::Undef | Jv::Null => 0.0,
        Jv::Num(n) => n,
        _ => f64::NAN,
    };
    let strict_focus = get_opt("strictFocus") == Some(&Value::Bool(true));
    let min_samples = match jv(get_opt("minSamples")) {
        Jv::Undef | Jv::Null => 10.0,
        Jv::Num(n) => n,
        _ => f64::NAN,
    };
    let expect = get_opt("expect").filter(|e| e.is_object());

    let mut issues: Vec<Value> = Vec::new();
    let t_of = |s: &Value| match jv(s.get("t")) {
        Jv::Num(n) => n,
        _ => f64::NAN,
    };
    let warmup: Vec<&Value> = samples
        .iter()
        .copied()
        .filter(|s| t_of(s) < settle_ms)
        .collect();
    let measured: Vec<&Value> = samples
        .iter()
        .copied()
        .filter(|s| t_of(s) >= settle_ms)
        .collect();

    let baseline_source: &[&Value] = if !warmup.is_empty() {
        &warmup
    } else {
        &samples[..samples.len().min(1)]
    };
    let baseline: Option<Vec<(String, Option<Value>)>> = if !baseline_source.is_empty() {
        Some(derive_baseline(baseline_source, expect))
    } else {
        None
    };

    let compared: &[&str] = if strict_focus {
        &COMPARED_FIELDS
    } else {
        &THEFT_FIELDS
    };
    let mut coverage = Map::new();
    for f in COMPARED_FIELDS {
        // `sample[field] !== null` counts missing fields as present.
        let present = samples
            .iter()
            .filter(|s| !matches!(jv(s.get(f)), Jv::Null))
            .count();
        let frac = if samples.is_empty() {
            0.0
        } else {
            present as f64 / samples.len() as f64
        };
        coverage.insert(f.to_string(), json!(frac));
    }
    let reliable = |f: &str| coverage.get(f).and_then(Value::as_f64).unwrap_or(0.0) >= 0.5;

    let bget = |f: &str| -> Option<&Option<Value>> {
        baseline
            .as_ref()
            .and_then(|b| b.iter().find(|(k, _)| k == f).map(|(_, v)| v))
    };
    if baseline.is_some() {
        let is_nullish = |f: &str| match bget(f) {
            Some(Some(v)) => v.is_null(),
            _ => false,
        };
        if is_nullish("keyWin") && is_nullish("topWin") && is_nullish("focusedPid") {
            issues.push(json!(
                "key/top window and focused element unmeasured (accessibility grant missing or no windows)"
            ));
        }
        if is_nullish("space") {
            issues.push(json!("active space unmeasured (SkyLight symbol)"));
        }
    }
    if measured.is_empty() {
        issues.push(json!("no post-settle samples"));
    }
    if (measured.len() as f64) < min_samples {
        issues.push(json!(format!(
            "thin coverage: {} post-settle samples (< {})",
            measured.len(),
            min_samples
        )));
    }

    let mut off_baseline: Vec<Value> = Vec::new();
    let mut drift: Vec<Value> = Vec::new();
    let mut off_baseline_samples = 0usize;
    let mut uncertain_samples = 0usize;
    let mut drift_samples = 0usize;
    let mut open_violation: Option<usize> = None;
    let mut previous_focused: Option<JvOwned> = None;

    for (measured_index, sample) in measured.iter().enumerate() {
        let index = warmup.len() + measured_index;
        let t = t_of(sample);
        let mut changed: Vec<&str> = Vec::new();
        let mut uncertain = false;
        if let Some(b) = &baseline {
            for field_name in compared.iter().copied() {
                let expected = b
                    .iter()
                    .find(|(k, _)| k == field_name)
                    .map(|(_, v)| v.as_ref());
                let expected_jv = match expected {
                    Some(Some(v)) => jv(Some(v)),
                    Some(None) => Jv::Undef,
                    None => Jv::Undef,
                };
                if matches!(expected_jv, Jv::Null) {
                    continue;
                }
                let actual = jv(sample.get(field_name));
                if matches!(actual, Jv::Null) {
                    if reliable(field_name) {
                        uncertain = true;
                    }
                    continue;
                }
                if !js_eq(actual, expected_jv) {
                    changed.push(field_name);
                }
            }
        }

        if changed.is_empty() {
            open_violation = None;
            if uncertain {
                uncertain_samples += 1;
            }
            // `sample.focused !== null` — undefined also enters.
            if !strict_focus && !matches!(jv(sample.get("focused")), Jv::Null) {
                let cur = JvOwned::from(sample.get("focused"));
                if let Some(prev) = &previous_focused
                    && !prev.is_undef()
                    && !prev.eq(&cur)
                {
                    drift.push(json!({
                        "index": index,
                        "tMs": sample.get("t").cloned().unwrap_or(Value::Null),
                        "from": prev.to_json(),
                        "to": cur.to_json(),
                    }));
                    drift_samples += 1;
                }
                previous_focused = Some(cur);
            }
            continue;
        }

        off_baseline_samples += 1;
        let observed = observed_of(sample);
        if let Some(vi) = open_violation {
            let v = &mut off_baseline[vi];
            let t_ms = jv_to_num(v.get("tMs"));
            v["durationMs"] = num_out(t - t_ms);
            let count = v.get("sampleCount").and_then(Value::as_u64).unwrap_or(0);
            v["sampleCount"] = json!(count + 1);
            // INFALLIBLE: constructed above as an array.
            let fields = v["changedFields"].as_array_mut().unwrap();
            for f in &changed {
                if !fields.iter().any(|x| x.as_str() == Some(f)) {
                    fields.push(json!(f));
                }
            }
            // INFALLIBLE: constructed above as an array.
            let observed_arr = v["observed"].as_array_mut().unwrap();
            if observed_arr.len() < 8 && !observed_arr.iter().any(|s| same_observed(s, &observed)) {
                observed_arr.push(observed);
            }
        } else {
            off_baseline.push(json!({
                "index": index,
                "tMs": sample.get("t").cloned().unwrap_or(Value::Null),
                "durationMs": num_out(0.0),
                "sampleCount": 1,
                "changedFields": changed,
                "observed": [observed],
            }));
            open_violation = Some(off_baseline.len() - 1);
        }
    }

    let measured_ms = if measured.len() > 1 {
        t_of(measured[measured.len() - 1]) - t_of(measured[0])
    } else {
        0.0
    };

    let has_baseline = baseline.is_some();
    let baseline_json = baseline.map(|b| {
        let mut m = Map::new();
        for (k, v) in b {
            m.insert(k, v.unwrap_or_else(|| json!({"$undefined": true})));
        }
        Value::Object(m)
    });

    Ok(json!({
        "ok": has_baseline && (measured.len() as f64) >= min_samples,
        "theftFree": off_baseline.is_empty(),
        "baseline": baseline_json.unwrap_or(Value::Null),
        "sampleCount": samples.len(),
        "warmupCount": warmup.len(),
        "measuredMs": num_out(measured_ms),
        "settleMs": match jv(get_opt("settleMs")) {
            Jv::Undef | Jv::Null => json!(0),
            _ => get_opt("settleMs").cloned().unwrap_or(json!(0)),
        },
        "strictFocus": strict_focus,
        "offBaseline": off_baseline,
        "offBaselineSamples": off_baseline_samples,
        "drift": drift,
        "driftSamples": drift_samples,
        "uncertainSamples": uncertain_samples,
        "coverage": coverage,
        "issues": issues,
    }))
}

fn jv_to_num(v: Option<&Value>) -> f64 {
    match jv(v) {
        Jv::Num(n) => n,
        _ => f64::NAN,
    }
}

/// Owned decoded value for `focused` drift tracking (null/undef distinct).
enum JvOwned {
    Undef,
    Str(String),
    Other(Value),
}

impl JvOwned {
    fn from(v: Option<&Value>) -> Self {
        match jv(v) {
            Jv::Undef => JvOwned::Undef,
            Jv::Str(s) => JvOwned::Str(s.to_string()),
            _ => JvOwned::Other(v.cloned().unwrap_or(Value::Null)),
        }
    }
    fn is_undef(&self) -> bool {
        matches!(self, JvOwned::Undef)
    }
    fn eq(&self, other: &JvOwned) -> bool {
        match (self, other) {
            (JvOwned::Undef, JvOwned::Undef) => true,
            (JvOwned::Str(a), JvOwned::Str(b)) => a == b,
            (JvOwned::Other(a), JvOwned::Other(b)) => js_eq(jv(Some(a)), jv(Some(b))),
            _ => false,
        }
    }
    fn to_json(&self) -> Value {
        match self {
            JvOwned::Undef => json!({"$undefined": true}),
            JvOwned::Str(s) => json!(s),
            JvOwned::Other(v) => v.clone(),
        }
    }
}

/// Port of `runLiveFixture` (`apps/desktop/src/cuaFixtures/live.ts:15`).
///
/// The original requires a built server entry; under the recorded harness it
/// always throws before any work.
pub async fn run_live_fixture(
    directory: String,
    binary_path: String,
) -> Result<serde_json::Value, String> {
    let _ = (&directory, &binary_path);
    Err("An explicit built Synara server is required.".to_string())
}
