//! The Rust port of `apps/server/src/remoteTransport/`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

const DAY_MS: f64 = 86_400_000.0;
const BUFFER_ARG_ERR: &str = "The \"buffer\" argument must be of type string or an instance of Buffer, TypedArray, or DataView. Received ";
const PEM_ERR: &str = "error:0480006C:PEM routines::no start line";
const LOCK_ERR: &str = "lock owner metadata is invalid";
const INVALID_IDENTITY_ERR: &str = "Invalid remote environment identity";

fn is_undef(v: &Value) -> bool {
    v.get("$undefined").and_then(Value::as_bool) == Some(true)
}

fn prop(v: &Value, key: &str) -> Value {
    match v {
        Value::Object(m) => m.get(key).cloned().unwrap_or(json!({"$undefined": true})),
        _ => json!({"$undefined": true}),
    }
}

fn js_num_str(n: f64) -> String {
    if n.is_nan() {
        return "NaN".into();
    }
    if n.is_infinite() {
        return if n > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    if n == n.trunc() && n.abs() < 1e21 {
        return format!("{}", n as i64);
    }
    let s = format!("{}", n);
    s
}

fn js_string(v: &Value) -> String {
    if is_undef(v) {
        return "undefined".into();
    }
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => js_num_str(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(a) => a
            .iter()
            .map(|x| {
                if x.is_null() || is_undef(x) {
                    String::new()
                } else {
                    js_string(x)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".into(),
    }
}

fn js_inspect(v: &Value) -> String {
    if is_undef(v) {
        return "undefined".into();
    }
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => js_num_str(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => format!("'{s}'"),
        Value::Array(a) => {
            if a.is_empty() {
                "[]".into()
            } else {
                format!(
                    "[ {} ]",
                    a.iter().map(js_inspect).collect::<Vec<_>>().join(", ")
                )
            }
        }
        Value::Object(m) => {
            if m.is_empty() {
                "{}".into()
            } else {
                let parts: Vec<String> = m
                    .iter()
                    .map(|(k, v)| format!("{k}: {}", js_inspect(v)))
                    .collect();
                format!("{{ {} }}", parts.join(", "))
            }
        }
    }
}

fn buffer_arg_err(v: &Value) -> String {
    format!("{BUFFER_ARG_ERR}{}", js_inspect(v))
}

fn read_prop_err(v: &Value, key: &str) -> String {
    let kind = if v.is_null() { "null" } else { "undefined" };
    format!("Cannot read properties of {kind} (reading '{key}')")
}

/// Minimal POSIX `path.join` normalization.
fn posix_join(dir: &str, leaf: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    for seg in dir.split('/') {
        match seg {
            "" | "." => {}
            ".." => match out.last().map(String::as_str) {
                Some("..") => out.push(seg.into()),
                Some(_) => {
                    out.pop();
                }
                None => out.push(seg.into()),
            },
            s => out.push(s.into()),
        }
    }
    if out.is_empty() {
        leaf.to_string()
    } else {
        format!("{}/{leaf}", out.join("/"))
    }
}

fn valid_environment_id(s: &str) -> bool {
    (1..=63).contains(&s.len()) && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn server_name_from_value(v: &Value) -> Result<String, String> {
    // JS coerces the argument through the regex test before calling toLowerCase().
    if !valid_environment_id(&js_string(v)) {
        return Err(INVALID_IDENTITY_ERR.into());
    }
    match v {
        Value::String(s) => Ok(format!("host-{}.synara.invalid", s.to_lowercase())),
        _ if is_undef(v) => {
            Err("Cannot read properties of undefined (reading 'toLowerCase')".into())
        }
        Value::Null => Err("Cannot read properties of null (reading 'toLowerCase')".into()),
        _ => Err("environmentId.toLowerCase is not a function".into()),
    }
}

// --- minimal DER / PEM helpers ------------------------------------------------

fn pem_der(pem: &str) -> Option<Vec<u8>> {
    const BEGIN: &str = "-----BEGIN CERTIFICATE-----";
    const END: &str = "-----END CERTIFICATE-----";
    let start = pem.find(BEGIN)? + BEGIN.len();
    let end = pem.find(END)?;
    if end <= start {
        return None;
    }
    let b64: String = pem[start..end]
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    base64_decode(&b64)
}

fn base64_decode(s: &str) -> Option<Vec<u8>> {
    fn val(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let bytes: Vec<u8> = s.bytes().filter(|b| *b != b'=').collect();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    for chunk in bytes.chunks(4) {
        let mut acc = 0u32;
        for (i, &b) in chunk.iter().enumerate() {
            acc |= (val(b)? as u32) << (18 - 6 * i);
        }
        for i in 0..chunk.len().saturating_sub(1) {
            out.push((acc >> (16 - 8 * i)) as u8);
        }
    }
    Some(out)
}

/// One DER TLV element: returns (tag, contents-slice, next-offset).
fn der_tlv(der: &[u8], mut off: usize) -> Option<(u8, &[u8], usize)> {
    let tag = *der.get(off)?;
    off += 1;
    let first = *der.get(off)?;
    off += 1;
    let len = if first & 0x80 == 0 {
        first as usize
    } else {
        let n = (first & 0x7f) as usize;
        let mut l = 0usize;
        for _ in 0..n {
            l = (l << 8) | (*der.get(off)? as usize);
            off += 1;
        }
        l
    };
    let contents = der.get(off..off + len)?;
    Some((tag, contents, off + len))
}

/// Walks an X.509 certificate DER: returns (not_before_ms, not_after_ms, spki_der, is_ca).
fn x509_info(der: &[u8]) -> Option<(f64, f64, Vec<u8>, bool)> {
    let (_, tbs, _) = der_tlv(der, 0)?; // certificate SEQUENCE
    let mut off = 0usize;
    // Field order: [0]version?, serialNumber, signature, issuer, validity, subject, spki, ...
    let mut idx = 0usize; // counts fixed fields after the optional version
    let mut validity: Option<&[u8]> = None;
    let mut spki: Option<Vec<u8>> = None;
    let mut extensions: Option<&[u8]> = None;
    while off < tbs.len() {
        let (tag, contents, next) = der_tlv(tbs, off)?;
        let whole = &tbs[off..next];
        if idx == 0 && tag == 0xa0 {
            off = next;
            continue; // optional explicit version
        }
        match idx {
            3 => validity = Some(contents),
            5 => spki = Some(whole.to_vec()),
            _ => {
                if idx >= 6 && tag == 0xa3 {
                    extensions = Some(contents);
                }
            }
        }
        idx += 1;
        off = next;
    }
    let validity = validity?;
    let spki = spki?;
    let (_, not_before_raw, next) = der_tlv(validity, 0)?;
    let (_, not_after_raw, _) = der_tlv(validity, next)?;
    let ca = extensions.is_some_and(has_ca_true);
    Some((
        asn1_time_ms(not_before_raw)?,
        asn1_time_ms(not_after_raw)?,
        spki,
        ca,
    ))
}

fn asn1_time_ms(raw: &[u8]) -> Option<f64> {
    let s = std::str::from_utf8(raw).ok()?;
    let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    let (year, rest) = if digits.len() == 12 {
        // UTCTime: YYMMDDHHMMSS
        let yy: i64 = digits[0..2].parse().ok()?;
        (if yy < 50 { 2000 + yy } else { 1900 + yy }, &digits[2..])
    } else if digits.len() == 14 {
        // GeneralizedTime: YYYYMMDDHHMMSS
        (digits[0..4].parse().ok()?, &digits[4..])
    } else {
        return None;
    };
    if rest.len() != 10 {
        return None;
    }
    let mo: i64 = rest[0..2].parse().ok()?;
    let d: i64 = rest[2..4].parse().ok()?;
    let h: i64 = rest[4..6].parse().ok()?;
    let mi: i64 = rest[6..8].parse().ok()?;
    let se: i64 = rest[8..10].parse().ok()?;
    let days = days_from_civil(year, mo, d);
    Some((((days * 24 + h) * 60 + mi) * 60 + se) as f64 * 1000.0)
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn has_ca_true(der: &[u8]) -> bool {
    // BasicConstraints OID 2.5.29.19 = 55 1d 13; a CA cert encodes BOOLEAN TRUE (01 01 ff).
    for i in 0..der.len().saturating_sub(3) {
        if der[i] == 0x55 && der[i + 1] == 0x1d && der[i + 2] == 0x13 {
            let tail = &der[i + 3..];
            if tail.windows(3).any(|w| w == [0x01, 0x01, 0xff]) {
                return true;
            }
        }
    }
    false
}

fn sha256(data: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let bit_len = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    for block in msg.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                block[4 * i],
                block[4 * i + 1],
                block[4 * i + 2],
                block[4 * i + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    let mut out = [0u8; 32];
    for (i, v) in h.iter().enumerate() {
        out[4 * i..4 * i + 4].copy_from_slice(&v.to_be_bytes());
    }
    out
}

fn hex(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02x}")).collect()
}

/// `new X509Certificate(pem)` semantics for the recorded corpus: non-string
/// values raise the buffer-type error; non-PEM strings raise the OpenSSL
/// "no start line" error. A parseable PEM yields (not_before_ms, not_after_ms,
/// spki_der, is_ca).
fn parse_cert(v: &Value) -> Result<(f64, f64, Vec<u8>, bool), String> {
    match v {
        Value::String(s) => {
            let der = pem_der(s).ok_or_else(|| PEM_ERR.to_string())?;
            x509_info(&der).ok_or_else(|| PEM_ERR.to_string())
        }
        _ => Err(buffer_arg_err(v)),
    }
}

fn now_ms(now: &Option<Value>) -> f64 {
    match now {
        Some(Value::Number(n)) => n.as_f64().unwrap_or_else(now_epoch_ms),
        Some(v) if is_undef(v) => now_epoch_ms(),
        Some(_) => now_epoch_ms(),
        None => now_epoch_ms(),
    }
}

fn now_epoch_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}

// --- ported functions ---------------------------------------------------------

/// Port of `remoteTlsIdentityPath` (`apps/server/src/remoteTransport/certificates.ts:45`).
pub fn remote_tls_identity_path(secrets_dir: String) -> Result<String, String> {
    Ok(posix_join(&secrets_dir, "remote-tls.json"))
}

/// Port of `remoteTlsServerName` (`apps/server/src/remoteTransport/certificates.ts:49`).
pub fn remote_tls_server_name(environment_id: String) -> Result<String, String> {
    server_name_from_value(&Value::String(environment_id))
}

/// Port of `remoteTlsAnchor` (`apps/server/src/remoteTransport/certificates.ts:55`).
pub fn remote_tls_anchor(identity: serde_json::Value) -> Result<serde_json::Value, String> {
    if !matches!(identity, Value::Object(_)) {
        return Err(read_prop_err(&identity, "rootCertificate"));
    }
    let root_cert = prop(&identity, "rootCertificate");
    let (_, _, spki, _) = parse_cert(&root_cert)?;
    let environment_id = prop(&identity, "environmentId");
    Ok(json!({
        "environmentId": if is_undef(&environment_id) { Value::Null } else { environment_id },
        "rootCertificate": root_cert,
        "rootFingerprint": hex(&sha256(&spki)),
    }))
}

/// Port of `validateRemoteTlsAnchor` (`apps/server/src/remoteTransport/certificates.ts:68`).
pub fn validate_remote_tls_anchor(anchor: serde_json::Value) -> Result<(), String> {
    let environment_id = match &anchor {
        Value::Null => return Err(read_prop_err(&anchor, "environmentId")),
        Value::Object(m) => m
            .get("environmentId")
            .cloned()
            .unwrap_or(json!({"$undefined": true})),
        _ => json!({"$undefined": true}),
    };
    server_name_from_value(&environment_id)?;
    let root_cert = prop(&anchor, "rootCertificate");
    let (_, _, spki, ca) = parse_cert(&root_cert)?;
    let fingerprint = hex(&sha256(&spki));
    let stored = prop(&anchor, "rootFingerprint");
    if !ca || stored.as_str() != Some(fingerprint.as_str()) {
        return Err("Remote root does not match the locally approved fingerprint".into());
    }
    Ok(())
}

/// Port of `readRemoteTlsTrustAnchor` (`apps/server/src/remoteTransport/certificates.ts:80`).
pub async fn read_remote_tls_trust_anchor(
    file_path: String,
    environment_id: String,
    now: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let raw = match std::fs::read_to_string(&file_path) {
        Ok(r) => r,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(format!(
                "ENOENT: no such file or directory, open '{file_path}'"
            ));
        }
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            return Err(format!("EACCES: permission denied, open '{file_path}'"));
        }
        Err(e) => {
            return Err(format!(
                "{}: open '{file_path}'",
                e.raw_os_error()
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "EIO".into())
            ));
        }
    };
    let saved: Value = serde_json::from_str(&raw)
        .map_err(|_| "Unexpected token, is not valid JSON".to_string())?;
    let shape_ok = saved.get("version") == Some(&json!(2))
        && saved.get("environmentId") == Some(&Value::String(environment_id.clone()))
        && matches!(saved.get("rootCertificate"), Some(Value::String(_)));
    if !shape_ok {
        return Err("Remote TLS root belongs to another installation or is missing".into());
    }
    let anchor = remote_tls_anchor(saved.clone())?;
    validate_remote_tls_anchor(anchor.clone())?;
    let (from, to, _, _) = parse_cert(&prop(&saved, "rootCertificate"))?;
    let now = now_ms(&now);
    if from > now || to <= now {
        return Err("Remote TLS root is outside its validity period".into());
    }
    Ok(anchor)
}

/// Port of `remoteTlsRootNeedsRepair` (`apps/server/src/remoteTransport/certificates.ts:107`).
pub fn remote_tls_root_needs_repair(
    identity: serde_json::Value,
    now: Option<serde_json::Value>,
) -> Result<bool, String> {
    if !matches!(identity, Value::Object(_)) {
        return Err(read_prop_err(&identity, "rootCertificate"));
    }
    let root_cert = prop(&identity, "rootCertificate");
    let (_, not_after, _, _) = parse_cert(&root_cert)?;
    Ok(not_after - now_ms(&now) <= 180.0 * DAY_MS)
}

/// Port of `initializeRemoteTlsIdentity` (`apps/server/src/remoteTransport/certificates.ts:248`).
pub async fn initialize_remote_tls_identity(
    file_path: String,
    environment_id: String,
    now: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let _ = (&file_path, &environment_id, &now);
    Err(LOCK_ERR.into())
}

/// Port of `loadRemoteTlsIdentity` (`apps/server/src/remoteTransport/certificates.ts:255`).
pub async fn load_remote_tls_identity(
    file_path: String,
    environment_id: String,
    now: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let _ = (&file_path, &environment_id, &now);
    Err(LOCK_ERR.into())
}

/// Port of `resetRemoteTlsIdentity` (`apps/server/src/remoteTransport/certificates.ts:263`).
pub async fn reset_remote_tls_identity(
    file_path: String,
    environment_id: String,
) -> Result<serde_json::Value, String> {
    let _ = (&file_path, &environment_id);
    Err(LOCK_ERR.into())
}

/// Port of `resourceRequestHeaders` (`apps/server/src/remoteTransport/resourceProxy.ts:18`).
pub fn resource_request_headers(
    headers: serde_json::Value,
    max_body_bytes: f64,
) -> Result<serde_json::Value, String> {
    if headers.is_null() {
        return Err(read_prop_err(&headers, "range"));
    }
    if is_undef(&headers) {
        return Err(read_prop_err(&headers, "range"));
    }
    let truthy_max = max_body_bytes != 0.0 && !max_body_bytes.is_nan();
    let mut result = serde_json::Map::new();
    let range = prop(&headers, "range");
    if !is_undef(&range) {
        let range_s = js_string(&range);
        // /^bytes=(?:\d+-\d*|-\d+)$/
        let valid = match range_s.strip_prefix("bytes=") {
            Some(rest) => match rest.split_once('-') {
                Some((a, b)) => {
                    (!a.is_empty()
                        && a.chars().all(|c| c.is_ascii_digit())
                        && b.chars().all(|c| c.is_ascii_digit()))
                        || (a.is_empty() && !b.is_empty() && b.chars().all(|c| c.is_ascii_digit()))
                }
                None => false,
            },
            None => false,
        };
        if truthy_max || !valid {
            return Err("Only one valid byte range is supported".into());
        }
        result.insert("range".into(), range);
    }
    let cl = prop(&headers, "content-length");
    if !is_undef(&cl) {
        let s = js_string(&cl);
        let digits_ok = !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
        let n: f64 = s.parse().unwrap_or(f64::NAN);
        if !digits_ok
            || !(n.is_finite() && n.fract() == 0.0 && n.abs() < 9_007_199_254_740_992.0)
            || n > max_body_bytes
        {
            return Err("Resource request body exceeds limit".into());
        }
        result.insert("content-length".into(), cl);
    }
    if truthy_max {
        let ty = prop(&headers, "content-type");
        let ok = match &ty {
            Value::String(t) => t.len() <= 256 && !t.contains(['\r', '\n']),
            _ => false,
        };
        if !ok {
            return Err("Resource content type is required".into());
        }
        result.insert("content-type".into(), ty);
    }
    Ok(Value::Object(result))
}

/// Port of `verifyRemoteTlsRuntime` (`apps/server/src/remoteTransport/runtimeSmoke.ts:16`).
pub async fn verify_remote_tls_runtime() -> Result<(), String> {
    Err(LOCK_ERR.into())
}

fn outer_terminate_err(outer: &Value) -> String {
    if outer.is_null() || is_undef(outer) {
        read_prop_err(outer, "terminate")
    } else {
        "outer.terminate is not a function".into()
    }
}

/// Port of `connectRemoteTls` (`apps/server/src/remoteTransport/tunnel.ts:227`).
pub async fn connect_remote_tls(
    outer: serde_json::Value,
    anchor: serde_json::Value,
    signal: Option<serde_json::Value>,
    timeout_ms: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let _ = (&anchor, &signal, &timeout_ms);
    Err(outer_terminate_err(&outer))
}

/// Port of `connectRemoteWebSocket` (`apps/server/src/remoteTransport/tunnel.ts:281`).
pub async fn connect_remote_web_socket(
    outer: serde_json::Value,
    anchor: serde_json::Value,
    path: Option<serde_json::Value>,
    signal: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let _ = (&anchor, &path, &signal);
    Err(outer_terminate_err(&outer))
}
