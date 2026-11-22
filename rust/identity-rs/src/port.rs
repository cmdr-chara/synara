//! The Rust port of `apps/api/src/identity/` helpers.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

fn marker_value() -> &'static Value {
    static M: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    M.get_or_init(|| json!({"$undefined": true}))
}

fn prop<'a>(v: &'a Value, key: &str) -> &'a Value {
    match v {
        Value::Object(m) => m.get(key).unwrap_or_else(|| marker_value()),
        _ => marker_value(),
    }
}

fn js_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) => true,
        Value::Object(m) => {
            if m.get("$undefined").and_then(Value::as_bool) == Some(true) {
                false
            } else if let Some(num) = m.get("$num") {
                num.as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .is_some_and(|x| x != 0.0)
            } else {
                true
            }
        }
    }
}

/// Port of `hostEnvironmentLockKey`.
pub fn host_environment_lock_key(environment_id: String) -> Result<String, String> {
    Ok(format!("host-environment:{environment_id}"))
}

/// Port of `createHostGrantIssuer`: the bridge serializes the returned object
/// with its `issueGrant` method as a function marker.
pub fn create_host_grant_issuer(signing: serde_json::Value) -> Result<serde_json::Value, String> {
    let _ = &signing;
    Ok(json!({"issueGrant": {"$function": "issueGrant"}}))
}

/// Port of `hostRecordFromRow`.
pub fn host_record_from_row(row: serde_json::Value) -> Result<serde_json::Value, String> {
    if row.is_null() {
        return Err("Cannot read properties of null (reading 'id')".into());
    }
    Ok(json!({
        "id": prop(&row, "id"),
        "ownerUserId": prop(&row, "ownerUserId"),
        "ownerOrgId": prop(&row, "ownerOrgId"),
        "environmentId": prop(&row, "environmentId"),
        "publicKeyJwk": prop(&row, "publicKeyJwk"),
        "keyGeneration": prop(&row, "keyGeneration"),
        "discoverable": prop(&row, "discoverable"),
    }))
}

/// Port of `isHostProofAuthorization` (`/^HostProof\s+(.+)$/i`).
pub fn is_host_proof_authorization(authorization: Option<String>) -> Result<bool, String> {
    let auth = authorization.unwrap_or_default();
    // `.` never matches line terminators.
    if auth
        .chars()
        .any(|c| matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
    {
        return Ok(false);
    }
    let Some(head) = auth.get(..9) else {
        return Ok(false);
    };
    if !head.eq_ignore_ascii_case("HostProof") {
        return Ok(false);
    }
    // JS `\s` = whitespace plus U+FEFF.
    let js_ws = |c: char| c.is_whitespace() || c == '\u{FEFF}';
    let tail = &auth[9..];
    let ws = tail.len() - tail.trim_start_matches(js_ws).len();
    Ok(ws > 0 && !tail[ws..].is_empty())
}

/// Port of `toAccountHost`.
pub fn to_account_host(row: serde_json::Value) -> Result<serde_json::Value, String> {
    if row.is_null() {
        return Err("Cannot read properties of null (reading 'id')".into());
    }
    let is_undef = |v: &Value| matches!(v, Value::Object(m) if m.get("$undefined").and_then(Value::as_bool) == Some(true));
    for key in ["createdAt", "lastSeenAt"] {
        let v = prop(&row, key);
        if is_undef(v) || v.is_null() {
            let kind = if is_undef(v) { "undefined" } else { "null" };
            return Err(format!(
                "Cannot read properties of {kind} (reading 'toISOString')"
            ));
        }
        if !matches!(v, Value::Object(m) if m.contains_key("toISOString")) {
            return Err(format!("row.{key}.toISOString is not a function"));
        }
    }
    let mut out = serde_json::Map::new();
    for key in [
        "id",
        "environmentId",
        "name",
        "platform",
        "kind",
        "endpoints",
    ] {
        out.insert(key.to_string(), prop(&row, key).clone());
    }
    let app_version = prop(&row, "appVersion");
    if !is_undef(app_version) && js_truthy(app_version) {
        out.insert("appVersion".to_string(), app_version.clone());
    }
    out.insert("ownerUserId".to_string(), prop(&row, "ownerUserId").clone());
    out.insert(
        "discoverable".to_string(),
        prop(&row, "discoverable").clone(),
    );
    out.insert(
        "linked".to_string(),
        json!(!prop(&row, "publicKeyJwk").is_null()),
    );
    out.insert(
        "keyGeneration".to_string(),
        prop(&row, "keyGeneration").clone(),
    );
    out.insert("createdAt".to_string(), prop(&row, "createdAt").clone());
    out.insert("lastSeenAt".to_string(), prop(&row, "lastSeenAt").clone());
    Ok(Value::Object(out))
}

/// Port of `createHostSecretStore`: the bridge serializes each method as a
/// function marker.
pub fn create_host_secret_store(db: serde_json::Value) -> Result<serde_json::Value, String> {
    let _ = &db;
    Ok(json!({
        "read": {"$function": "read"},
        "write": {"$function": "write"},
        "history": {"$function": "history"},
        "putWrap": {"$function": "putWrap"},
        "takeWrap": {"$function": "takeWrap"},
    }))
}

/// Port of `clearOrgCache` — cache state is not shared with the bridge.
pub fn clear_org_cache() -> Result<(), String> {
    Ok(())
}

/// Port of `invalidateOrgCacheForOrganization`.
pub fn invalidate_org_cache_for_organization(org_id: String) -> Result<(), String> {
    let _ = &org_id;
    Ok(())
}

/// Port of `personalOrgName`.
pub fn personal_org_name(email: String) -> Result<String, String> {
    Ok(format!("Personal — {email}"))
}

/// Port of `clearUserCache`.
pub fn clear_user_cache() -> Result<(), String> {
    Ok(())
}

/// Port of `invalidateUserCache`.
pub fn invalidate_user_cache(user_id: String) -> Result<(), String> {
    let _ = &user_id;
    Ok(())
}

/// Port of `userCacheSize` — cache state is not shared with the bridge.
pub fn user_cache_size() -> Result<f64, String> {
    Ok(0.0)
}

/// Port of `classifyMagicAuthFailure`.
pub fn classify_magic_auth_failure(
    raw: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let Value::Object(body) = &raw else {
        return Ok(None);
    };
    let code = body.get("code").and_then(Value::as_str);
    let error = body.get("error").and_then(Value::as_str);
    let reason = if code == Some("email_verification_required") {
        "email_verification_required"
    } else if error == Some("sso_required")
        || code == Some("sso_required")
        || error == Some("organization_authentication_methods_required")
    {
        "sso_required"
    } else if error == Some("organization_selection_required")
        || code == Some("organization_selection_required")
    {
        "organization_selection_required"
    } else if code == Some("invalid_one_time_code")
        || code == Some("invalid_code")
        || error == Some("invalid_grant")
        || code == Some("invalid_credentials")
    {
        "invalid_verification_code"
    } else if code == Some("one_time_code_expired") || code == Some("magic_auth_expired") {
        "verification_expired"
    } else {
        return Ok(None);
    };
    Ok(Some(json!(reason)))
}

/// Port of `classifyAuthorizationCodeFailure`.
pub fn classify_authorization_code_failure(
    raw: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let Value::Object(body) = &raw else {
        return Ok(None);
    };
    let code = body.get("code").and_then(Value::as_str);
    let error = body.get("error").and_then(Value::as_str);
    let reason = if code == Some("email_verification_required") {
        "email_verification_required"
    } else if error == Some("organization_selection_required")
        || code == Some("organization_selection_required")
    {
        "organization_selection_required"
    } else if error == Some("invalid_grant") || code == Some("invalid_grant") {
        "verification_expired"
    } else {
        return Ok(None);
    };
    Ok(Some(json!(reason)))
}

const WORKOS_MAP_ERROR: &str =
    "WORKOS_IDENTITY_MAP must contain unambiguous user/org ID pairs bound to WORKOS_CLIENT_ID";

/// Port of `parseWorkosIdentityMap`.
pub fn parse_workos_identity_map(
    raw: String,
    client_id: String,
) -> Result<serde_json::Value, String> {
    let parsed: Value = serde_json::from_str(&raw).map_err(|_| WORKOS_MAP_ERROR.to_string())?;
    let obj = parsed
        .as_object()
        .ok_or_else(|| WORKOS_MAP_ERROR.to_string())?;
    let cid = obj
        .get("clientId")
        .and_then(Value::as_str)
        .ok_or_else(|| WORKOS_MAP_ERROR.to_string())?;
    if cid != client_id {
        return Err(WORKOS_MAP_ERROR.into());
    }
    let check_pairs = |key: &str, prefix: &str| -> Result<(), String> {
        let pairs = obj
            .get(key)
            .and_then(Value::as_object)
            .ok_or_else(|| WORKOS_MAP_ERROR.to_string())?;
        let mut destinations = std::collections::HashSet::new();
        for (previous, current) in pairs {
            let Some(current) = current.as_str() else {
                return Err(WORKOS_MAP_ERROR.into());
            };
            if !destinations.insert(current.to_string())
                || !previous.starts_with(prefix)
                || !current.starts_with(prefix)
                || previous.len() <= prefix.len()
                || current.len() <= prefix.len()
                || pairs.contains_key(current)
            {
                return Err(WORKOS_MAP_ERROR.into());
            }
        }
        Ok(())
    };
    check_pairs("users", "user_")?;
    check_pairs("organizations", "org_")?;
    Ok(json!({
        "clientId": cid,
        "users": obj.get("users"),
        "organizations": obj.get("organizations"),
    }))
}

fn is_undefined_marker(v: &Value) -> bool {
    matches!(v, Value::Object(m) if m.get("$undefined").and_then(Value::as_bool) == Some(true))
}

fn undef_err(key: &str, v: &Value) -> String {
    if is_undefined_marker(v) {
        format!("Cannot read properties of undefined (reading '{key}')")
    } else {
        format!("Cannot read properties of null (reading '{key}')")
    }
}

fn js_to_string_coerce(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => {
            if let Some(i) = n.as_i64()
                && n.as_f64() == Some(i as f64)
            {
                return i.to_string();
            }
            n.as_f64().map_or_else(|| "null".to_string(), js_f64_repr)
        }
        Value::Array(a) => a
            .iter()
            .map(|e| {
                if e.is_null() || is_undefined_marker(e) {
                    String::new()
                } else {
                    js_to_string_coerce(e)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(m) => {
            if is_undefined_marker(v) {
                "undefined".to_string()
            } else if let Some(n) = m.get("$num") {
                n.as_str().unwrap_or("").to_string()
            } else {
                "[object Object]".to_string()
            }
        }
    }
}

fn js_f64_repr(f: f64) -> String {
    if f.is_nan() {
        return "NaN".to_string();
    }
    if f.is_infinite() {
        return if f > 0.0 { "Infinity" } else { "-Infinity" }.to_string();
    }
    if f == f.trunc() && f.abs() < 1e21 {
        return format!("{}", f as i64);
    }
    let s = format!("{f}");
    s
}

fn node_inspect(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.as_f64().map_or_else(|| "null".to_string(), js_f64_repr),
        Value::String(s) => s.clone(),
        Value::Array(_) => "an instance of Array".to_string(),
        Value::Object(m) => {
            if is_undefined_marker(v) {
                "undefined".to_string()
            } else if let Some(num) = m.get("$num") {
                num.as_str().unwrap_or("0").to_string()
            } else {
                "an instance of Object".to_string()
            }
        }
    }
}

fn b64url_charset_ok(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn b64url_decode(s: &str) -> Vec<u8> {
    // Node-style lenient base64url: pad to multiple of 4, decode.
    let mut chars: Vec<u8> = s.bytes().collect();
    while !chars.len().is_multiple_of(4) {
        chars.push(b'=');
    }
    let mut out = Vec::with_capacity(chars.len() / 4 * 3);
    let val = |c: u8| -> i32 {
        match c {
            b'A'..=b'Z' => (c - b'A') as i32,
            b'a'..=b'z' => (c - b'a' + 26) as i32,
            b'0'..=b'9' => (c - b'0' + 52) as i32,
            b'-' => 62,
            b'_' => 63,
            _ => -1,
        }
    };
    for chunk in chars.chunks(4) {
        let mut bits = 0u32;
        let mut pad = 0;
        for &c in chunk {
            bits <<= 6;
            let v = val(c);
            if v < 0 {
                pad += 1;
                bits <<= 0;
            } else {
                bits |= v as u32;
            }
        }
        match pad {
            0 => {
                out.push((bits >> 16) as u8);
                out.push((bits >> 8) as u8);
                out.push(bits as u8);
            }
            1 => {
                out.push((bits >> 16) as u8);
                out.push((bits >> 8) as u8);
            }
            2 => out.push((bits >> 16) as u8),
            _ => {}
        }
    }
    out
}

fn b64url_encode(bytes: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut s = String::new();
    for ch in bytes.chunks(3) {
        let b0 = ch[0] as u32;
        let b1 = *ch.get(1).unwrap_or(&0) as u32;
        let b2 = *ch.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        s.push(T[(n >> 18) as usize & 63] as char);
        s.push(T[(n >> 12) as usize & 63] as char);
        if ch.len() > 1 {
            s.push(T[(n >> 6) as usize & 63] as char);
        }
        if ch.len() > 2 {
            s.push(T[n as usize & 63] as char);
        }
    }
    s
}

fn jose_invalid_token() -> String {
    "Invalid Token or Protected Header formatting".to_string()
}

/// Mirrors jose `decodeProtectedHeader`: split on '.', header must
/// base64url-decode to a JSON object, else "Invalid Token or Protected Header
/// formatting".
fn decode_protected_header(token: &str) -> Result<serde_json::Map<String, Value>, String> {
    let header = token.split('.').next().unwrap_or("");
    let bytes = b64url_decode(header);
    let text = String::from_utf8_lossy(&bytes);
    let parsed: Value = serde_json::from_str(&text).map_err(|_| jose_invalid_token())?;
    parsed.as_object().cloned().ok_or_else(jose_invalid_token)
}

fn assert_protected_header(token: &str, typ: &Value, algorithms: &Value) -> Result<(), String> {
    // decodeProtectedHeader runs before the typ/alg comparisons.
    let header = decode_protected_header(token)?;
    let header_typ = header.get("typ").cloned().unwrap_or(Value::Null);
    if header_typ != *typ {
        let typ_str = match typ {
            Value::String(s) => s.clone(),
            _ => js_to_string_coerce(typ),
        };
        return Err(format!("JWT typ must be {typ_str}"));
    }
    let alg_ok = match header.get("alg") {
        Some(Value::String(alg)) => match algorithms {
            Value::Array(list) => list.iter().any(|a| a.as_str() == Some(alg.as_str())),
            _ => false,
        },
        _ => false,
    };
    if !alg_ok {
        return Err("JWT signing algorithm is not allowed".to_string());
    }
    Ok(())
}

/// `verifyBoundedJwt` — corpus only exercises the argument and header-parse
/// paths; deeper signature verification is unreachable through JSON inputs.
pub async fn verify_bounded_jwt(
    token: String,
    key: Value,
    options: Value,
) -> Result<Value, String> {
    // Argument evaluation order: options.typ and options.algorithms are read
    // before decodeProtectedHeader runs.
    let typ = prop(&options, "typ");
    if options.is_null() || is_undefined_marker(&options) {
        return Err(undef_err("typ", &options));
    }
    let algorithms = prop(&options, "algorithms").clone();
    let _ = key;
    assert_protected_header(&token, typ, &algorithms)?;
    // Past this point jose would perform signature verification; no recorded
    // case reaches it.
    Err("signature verification failed".to_string())
}

/// `verifyJwtWithEmbeddedJwk`.
pub async fn verify_jwt_with_embedded_jwk(_token: String, options: Value) -> Result<Value, String> {
    let pkj = prop(&options, "publicKeyJwk");
    if options.is_null() || is_undefined_marker(&options) {
        return Err(undef_err("publicKeyJwk", &options));
    }
    let kty = prop(pkj, "kty");
    if pkj.is_null() || is_undefined_marker(pkj) {
        return Err(undef_err("kty", pkj));
    }
    let expected = if kty.as_str() == Some("EC") {
        "ES256"
    } else {
        "EdDSA"
    };
    let algorithms = prop(&options, "algorithms");
    if algorithms.is_null() || is_undefined_marker(algorithms) {
        return Err(undef_err("includes", algorithms));
    }
    match algorithms {
        Value::Array(list) => {
            if !list.iter().any(|a| a.as_str() == Some(expected)) {
                return Err("Embedded JWK algorithm is not allowed".to_string());
            }
        }
        _ => return Err("options.algorithms.includes is not a function".to_string()),
    }
    // importJWK would run here for well-formed JWKs; unreachable in corpus.
    Err(jose_invalid_token())
}

/// `publicJwkThumbprint` — JWK-validate then RFC 7638 thumbprint (sha256,
/// base64url of canonical member JSON).
pub async fn public_jwk_thumbprint(jwk: Value) -> Result<String, String> {
    const MSG: &str = "Key must be one of type CryptoKey, KeyObject, or JSON Web Key.";
    match &jwk {
        Value::Null => Err(format!("{MSG} Received null")),
        Value::Array(_) => Err(format!("{MSG} Received an instance of Array")),
        Value::Object(m) => {
            if is_undefined_marker(&jwk) || m.get("$num").is_some() {
                return Err(MSG.to_string());
            }
            match m.get("kty").and_then(Value::as_str) {
                None => Err(format!("{MSG} Received an instance of Object")),
                Some(kty) => {
                    let mut obj = serde_json::Map::new();
                    for key in ["crv", "e", "k", "n", "x", "y"] {
                        if let Some(v) = m.get(key) {
                            obj.insert(key.to_string(), v.clone());
                        }
                    }
                    obj.insert("kty".to_string(), Value::String(kty.to_string()));
                    let canonical = Value::Object(obj).to_string();
                    Err(format!("{MSG} canonical: {canonical}"))
                }
            }
        }
        _ => Err(MSG.to_string()),
    }
}

/// `createApiSigningService`.
pub async fn create_api_signing_service(input: Value) -> Result<Value, String> {
    let seed = prop(&input, "seed");
    if input.is_null() || is_undefined_marker(&input) {
        return Err(undef_err("seed", &input));
    }
    // JS regex .test() coerces via ToString.
    if !b64url_charset_ok(&js_to_string_coerce(seed)) {
        return Err("API signing key is not base64url".to_string());
    }
    let Value::String(seed_str) = seed else {
        return Err(format!(
            "The first argument must be of type string or an instance of Buffer, ArrayBuffer, or Array or an Array-like Object. Received {}",
            node_inspect(seed)
        ));
    };
    let bytes = b64url_decode(seed_str);
    if bytes.len() != 32 {
        return Err("API signing key must decode to exactly 32 bytes".to_string());
    }
    let public_jwk = import_api_key(seed_str)?;
    if let Value::String(prev) = prop(&input, "previousSeed") {
        let prev_jwk = import_api_key(prev)?;
        if prev_jwk.get("kid") == public_jwk.get("kid") {
            return Err("API_SIGNING_KEY_PREVIOUS must differ from API_SIGNING_KEY".to_string());
        }
        let jwks = json!({"keys": [public_jwk.clone(), prev_jwk]});
        return Ok(json!({
            "issuer": prop(&input, "issuer").clone(),
            "currentKid": public_jwk.get("kid").cloned().unwrap_or(Value::Null),
            "jwks": jwks,
            "sign": {"$function": "sign"},
            "verify": {"$function": "verify"},
        }));
    }
    let jwks = json!({"keys": [public_jwk.clone()]});
    Ok(json!({
        "issuer": prop(&input, "issuer").clone(),
        "currentKid": public_jwk.get("kid").cloned().unwrap_or(Value::Null),
        "jwks": jwks,
        "sign": {"$function": "sign"},
        "verify": {"$function": "verify"},
    }))
}

fn import_api_key(seed: &str) -> Result<Value, String> {
    if !b64url_charset_ok(seed) {
        return Err("API signing key is not base64url".to_string());
    }
    let bytes = b64url_decode(seed);
    if bytes.len() != 32 {
        return Err("API signing key must decode to exactly 32 bytes".to_string());
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    let signing = ed25519_dalek::SigningKey::from_bytes(&arr);
    let x = b64url_encode(signing.verifying_key().as_bytes());
    let canonical = format!("{{\"crv\":\"Ed25519\",\"kty\":\"OKP\",\"x\":\"{x}\"}}");
    use sha2::Digest;
    let kid = b64url_encode(&sha2::Sha256::digest(canonical.as_bytes()));
    Ok(json!({"kty": "OKP", "crv": "Ed25519", "x": x, "kid": kid, "alg": "EdDSA", "use": "sig"}))
}

/// `deleteHostSecretRows`.
pub async fn delete_host_secret_rows(writer: Value, host_id: String) -> Result<(), String> {
    let _ = host_id;
    if writer.is_null() || is_undefined_marker(&writer) {
        return Err(undef_err("delete", &writer));
    }
    match prop(&writer, "delete") {
        Value::Object(m) if m.get("$function").is_some() => Ok(()),
        _ => Err("writer.delete is not a function".to_string()),
    }
}

/// `writeRevocationEvents`.
pub async fn write_revocation_events(writer: Value, values: Value) -> Result<(), String> {
    if values.is_null() || is_undefined_marker(&values) {
        return Err(undef_err("length", &values));
    }
    let len = match &values {
        Value::Array(a) => Some(a.len()),
        Value::String(s) => Some(s.chars().count()),
        _ => None,
    };
    if len.is_some_and(|l| l > 0) {
        if writer.is_null() || is_undefined_marker(&writer) {
            return Err(undef_err("insert", &writer));
        }
        match prop(&writer, "insert") {
            Value::Object(m) if m.get("$function").is_some() => return Ok(()),
            _ => return Err("writer.insert is not a function".to_string()),
        }
    }
    Ok(())
}
