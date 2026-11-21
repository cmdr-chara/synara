//! The Rust port of `apps/api/src/config.ts`.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::{Map, Value, json};

fn member(v: &Value, key: &str) -> Value {
    v.as_object()
        .and_then(|o| o.get(key))
        .cloned()
        .unwrap_or_else(js::undefined)
}

fn null_err(key: &str) -> String {
    format!("Cannot read properties of null (reading '{key}')")
}

fn undef_err(key: &str) -> String {
    format!("Cannot read properties of undefined (reading '{key}')")
}

/// `env[key]` — throws on null/undefined receivers, undefined elsewhere.
fn env_get(env: &Value, key: &str) -> Result<Value, String> {
    if env.is_null() {
        return Err(null_err(key));
    }
    if js::is_undefined(env) {
        return Err(undef_err(key));
    }
    Ok(member(env, key))
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) => true,
        Value::Object(_) => !js::is_undefined(v),
    }
}

fn is_undef(v: &Value) -> bool {
    js::is_undefined(v)
}

fn trim_js(s: &str) -> &str {
    s.trim_matches(|c: char| {
        matches!(
            c,
            '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
                ..='\u{200a}'
                    | '\u{2028}'
                    | '\u{2029}'
                    | '\u{202f}'
                    | '\u{205f}'
                    | '\u{3000}'
                    | '\u{feff}'
        )
    })
}

/// `String(value)` for interpolation/tests.
fn js_string(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(_) => js::number_to_string(js::as_number(v).unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(a) => a.iter().map(js_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => {
            if js::is_undefined(v) {
                "undefined".into()
            } else {
                "[object Object]".into()
            }
        }
    }
}

fn require_vars(env: &Value, names: &[&str]) -> Result<(), String> {
    let mut missing = Vec::new();
    for name in names {
        if !truthy(&env_get(env, name)?) {
            missing.push(*name);
        }
    }
    if !missing.is_empty() {
        return Err(format!(
            "Missing required environment variables: {}",
            missing.join(", ")
        ));
    }
    Ok(())
}

// ---- URL parsing (WHATWG subset: absolute URLs, http/https focus) -----------

struct UrlParts {
    protocol: String,
    username: String,
    password: String,
    hostname: String,
    port: String,
    pathname: String,
    search: String,
    hash: String,
    origin: String,
    href: String,
}

fn parse_url(raw: &str) -> Option<UrlParts> {
    let value = trim_js(raw);
    let colon = value.find(':')?;
    let scheme = &value[..colon];
    if scheme.is_empty()
        || !scheme
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic())
        || !scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
    {
        return None;
    }
    let protocol = scheme.to_lowercase();
    let default_port = match protocol.as_str() {
        "http" | "ws" => Some("80"),
        "https" | "wss" => Some("443"),
        "ftp" => Some("21"),
        _ => None,
    };
    let mut rest = &value[colon + 1..];
    let mut username = String::new();
    let mut password = String::new();
    let mut hostport = String::new();
    if let Some(after) = rest.strip_prefix("//") {
        rest = after;
        let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
        let authority = &rest[..end];
        rest = &rest[end..];
        let (userinfo, hosts) = match authority.rfind('@') {
            Some(at) => (&authority[..at], &authority[at + 1..]),
            None => ("", authority),
        };
        if !userinfo.is_empty() {
            if let Some(ci) = userinfo.find(':') {
                username = userinfo[..ci].to_string();
                password = userinfo[ci + 1..].to_string();
            } else {
                username = userinfo.to_string();
            }
        }
        hostport = hosts.to_string();
        if hostport.is_empty() {
            return None;
        }
    } else if protocol == "http" || protocol == "https" {
        return None;
    }
    let mut hostname = hostport.clone();
    let mut port = String::new();
    if let Some(h) = hostport.strip_prefix('[') {
        if let Some(close) = h.find(']') {
            hostname = format!("[{}]", &h[..close]);
            let tail = &h[close + 1..];
            if let Some(p) = tail.strip_prefix(':') {
                port = p.to_string();
            }
        }
    } else if let Some(ci) = hostport.rfind(':') {
        hostname = hostport[..ci].to_string();
        port = hostport[ci + 1..].to_string();
    }
    if !port.is_empty() && !port.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if Some(port.as_str()) == default_port {
        port.clear();
    }
    hostname = hostname.to_lowercase();
    let path_end = rest.find(['?', '#']).unwrap_or(rest.len());
    let mut pathname = rest[..path_end].to_string();
    rest = &rest[path_end..];
    if pathname.is_empty() {
        pathname = "/".to_string();
    }
    let mut search = String::new();
    if let Some(q) = rest.strip_prefix('?') {
        let end = q.find('#').unwrap_or(q.len());
        search = format!("?{}", &q[..end]);
        rest = &rest[end..];
    }
    let mut hash = String::new();
    if let Some(f) = rest.strip_prefix('#') {
        hash = format!("#{f}");
    }
    let host_out = if port.is_empty() {
        hostname.clone()
    } else {
        format!("{hostname}:{port}")
    };
    let userinfo_out = if password.is_empty() {
        if username.is_empty() {
            String::new()
        } else {
            format!("{username}@")
        }
    } else {
        format!("{username}:{password}@")
    };
    let origin = if hostport.is_empty() {
        "null".to_string()
    } else {
        format!("{protocol}://{host_out}")
    };
    let href = format!("{protocol}://{userinfo_out}{host_out}{pathname}{search}{hash}");
    Some(UrlParts {
        protocol: format!("{protocol}:"),
        username,
        password,
        hostname,
        port,
        pathname,
        search,
        hash,
        origin,
        href,
    })
}

fn polar_return_origins(values: &[Value]) -> Result<Vec<String>, String> {
    let mut seen: Vec<String> = Vec::new();
    for value in values {
        let url = parse_url(&js_string(value))
            .ok_or("POLAR_RETURN_ORIGINS must contain exact HTTPS origins")?;
        if url.protocol != "https:"
            || !url.username.is_empty()
            || !url.password.is_empty()
            || !url.port.is_empty()
            || url.pathname != "/"
            || !url.search.is_empty()
            || !url.hash.is_empty()
        {
            return Err(
                "POLAR_RETURN_ORIGINS must contain exact HTTPS origins without credentials, non-default ports, paths, queries or fragments"
                    .to_string(),
            );
        }
        if !seen.contains(&url.origin) {
            seen.push(url.origin);
        }
    }
    Ok(seen)
}

fn polar_return_url(
    value: &Value,
    name: &str,
    config: &Map<String, Value>,
) -> Result<String, String> {
    let url = parse_url(&js_string(value)).ok_or(format!("{name} must be an absolute URL"))?;
    let environment = config
        .get("environment")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let sandbox_loopback = environment == "sandbox"
        && url.protocol == "http:"
        && ["localhost", "127.0.0.1", "[::1]"].contains(&url.hostname.as_str());
    if !url.username.is_empty()
        || !url.password.is_empty()
        || !url.hash.is_empty()
        || (url.protocol != "https:" && !sandbox_loopback)
    {
        return Err(format!(
            "{name} must be HTTPS or sandbox HTTP loopback without credentials or fragment"
        ));
    }
    if environment == "production" {
        let return_origins = match config.get("returnOrigins") {
            Some(Value::Array(a)) => a.clone(),
            Some(v) if v.is_null() || is_undef(v) => Vec::new(),
            Some(_) => return Err("values.map is not a function".to_string()),
            None => Vec::new(),
        };
        let origins = polar_return_origins(&return_origins)?;
        if !origins.contains(&url.origin) {
            return Err(format!(
                "{name} must use an exact origin from POLAR_RETURN_ORIGINS"
            ));
        }
    }
    Ok(url.href)
}

// ---- APNs -------------------------------------------------------------------

const APNS_NAMES: [&str; 5] = [
    "APNS_PRIVATE_KEY",
    "APNS_KEY_ID",
    "APNS_TEAM_ID",
    "APNS_BUNDLE_ID",
    "APNS_ENVIRONMENT",
];
const APNS_PROD_NAMES: [&str; 2] = ["APNS_PRODUCTION_PRIVATE_KEY", "APNS_PRODUCTION_KEY_ID"];

/// The curve OIDs a valid P-256 key must carry: ecPublicKey algorithm and
/// prime256v1, DER-encoded.
const OID_EC_PUBLIC_KEY: [u8; 9] = [0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01];
const OID_P256: [u8; 10] = [0x06, 0x08, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07];

fn der_contains(der: &[u8], needle: &[u8]) -> bool {
    der.windows(needle.len()).any(|w| w == needle)
}

fn base64_decode(text: &str, alphabet: &str) -> Option<Vec<u8>> {
    let mut table = [0xffu8; 128];
    for (i, c) in alphabet.bytes().enumerate() {
        table[c as usize] = i as u8;
    }
    let mut out = Vec::new();
    let mut acc: u32 = 0;
    let mut bits = 0u32;
    for ch in text.bytes() {
        if ch == b'=' {
            break;
        }
        if ch >= 128 || table[ch as usize] == 0xff {
            return None;
        }
        acc = (acc << 6) | table[ch as usize] as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

fn pem_der(value: &str) -> Option<Vec<u8>> {
    let start = value.find("-----BEGIN")?;
    let header_end = value[start..].find("-----")?;
    let label_end = start + header_end + 5;
    let body_start = value[label_end..].find('\n')? + label_end + 1;
    let body_end = value[body_start..].find("-----END")? + body_start;
    let body: String = value[body_start..body_end]
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '+' || *c == '/' || *c == '=')
        .collect();
    base64_decode(
        &body,
        "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/",
    )
}

fn apns_private_key(value: &Value, name: &str) -> Result<String, String> {
    let text = match value {
        Value::String(s) => s.clone(),
        _ => return Err(format!("env.{name}.replace is not a function")),
    };
    let private_key = text.replace("\\n", "\n");
    let ok = pem_der(&private_key).is_some_and(|der| {
        der_contains(&der, &OID_P256)
            && (der_contains(&der, &OID_EC_PUBLIC_KEY)
                || private_key.contains("BEGIN EC PRIVATE KEY"))
    });
    if !ok {
        return Err(format!("{name} must be a P-256 PEM private key"));
    }
    Ok(private_key)
}

/// Port of `loadApnsConfig` (`apps/api/src/config.ts:53`).
pub fn load_apns_config(env: serde_json::Value) -> Result<serde_json::Value, String> {
    // The original reads `APNS_PRODUCTION_PRIVATE_KEY` first via
    // `productionNames.some`, which is what throws on a null env.
    env_get(&env, "APNS_PRODUCTION_PRIVATE_KEY")?;
    load_apns_inner(&env)
}

fn load_apns_inner(env: &Value) -> Result<Value, String> {
    let has_production = APNS_PROD_NAMES
        .iter()
        .any(|name| !is_undef(&member(env, name)));
    if !APNS_NAMES.iter().any(|name| !is_undef(&member(env, name))) && !has_production {
        return Ok(js::undefined());
    }
    require_vars(env, &APNS_NAMES)?;
    if has_production {
        require_vars(env, &APNS_PROD_NAMES)?;
    }
    let key_id = env_get(env, "APNS_KEY_ID")?;
    let team_id = env_get(env, "APNS_TEAM_ID")?;
    let bundle_id = env_get(env, "APNS_BUNDLE_ID")?;
    let environment = env_get(env, "APNS_ENVIRONMENT")?;
    let prod_key_id = env_get(env, "APNS_PRODUCTION_KEY_ID")?;
    let id10 = |v: &Value| {
        v.as_str().is_some_and(|s| {
            s.len() == 10
                && s.chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        })
    };
    if !id10(&key_id)
        || !id10(&team_id)
        || !bundle_id.as_str().is_some_and(|s| {
            (1..=200).contains(&s.len())
                && s.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
        })
        || !["sandbox", "production", "both"].contains(&environment.as_str().unwrap_or(""))
        || (has_production && !id10(&prod_key_id))
    {
        return Err("Invalid APNs metadata or environment".to_string());
    }
    let private_key = env_get(env, "APNS_PRIVATE_KEY")?;
    let mut out = Map::new();
    out.insert(
        "privateKey".into(),
        json!(apns_private_key(&private_key, "APNS_PRIVATE_KEY")?),
    );
    out.insert("keyId".into(), key_id);
    out.insert("teamId".into(), team_id);
    out.insert("bundleId".into(), bundle_id);
    out.insert("environment".into(), environment);
    if has_production {
        let prod_key = env_get(env, "APNS_PRODUCTION_PRIVATE_KEY")?;
        out.insert(
            "productionCredentials".into(),
            json!({
                "privateKey": apns_private_key(&prod_key, "APNS_PRODUCTION_PRIVATE_KEY")?,
                "keyId": prod_key_id,
            }),
        );
    }
    Ok(Value::Object(out))
}

// ---- Polar billing ----------------------------------------------------------

fn is_uuid(v: &Value) -> bool {
    v.as_str().is_some_and(|s| {
        let parts: Vec<&str> = s.split('-').collect();
        parts.len() == 5
            && parts
                .iter()
                .zip([8usize, 4, 4, 4, 12])
                .all(|(p, n)| p.len() == n && p.chars().all(|c| c.is_ascii_hexdigit()))
    })
}

const POLAR_REQUIRED: [&str; 8] = [
    "POLAR_ACCESS_TOKEN",
    "POLAR_WEBHOOK_SECRET",
    "POLAR_ORGANIZATION_ID",
    "POLAR_MONTHLY_PRODUCT_ID",
    "POLAR_YEARLY_PRODUCT_ID",
    "POLAR_DESKTOP_BENEFIT_ID",
    "POLAR_CHECKOUT_SUCCESS_URL",
    "POLAR_PORTAL_RETURN_URL",
];
const POLAR_CATALOG: [&str; 4] = [
    "POLAR_EXPECTED_CURRENCY",
    "POLAR_EXPECTED_MONTHLY_AMOUNT_MINOR",
    "POLAR_EXPECTED_YEARLY_AMOUNT_MINOR",
    "POLAR_EXPECTED_TAX_BEHAVIOR",
];
const POLAR_UUID_NAMES: [&str; 4] = [
    "POLAR_ORGANIZATION_ID",
    "POLAR_MONTHLY_PRODUCT_ID",
    "POLAR_YEARLY_PRODUCT_ID",
    "POLAR_DESKTOP_BENEFIT_ID",
];

/// `assertPolarBillingConfig` on an already-built config map.
fn assert_polar(config: &Map<String, Value>) -> Result<(), String> {
    let environment = config
        .get("environment")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if environment != "sandbox" && environment != "production" {
        return Err("POLAR_ENVIRONMENT must be sandbox or production".to_string());
    }
    if environment == "production"
        && config.get("productionPaymentsEnabled") != Some(&Value::Bool(true))
    {
        return Err(
            "Production Polar billing requires POLAR_PRODUCTION_PAYMENTS_ENABLED=true".to_string(),
        );
    }
    if environment == "production" {
        let origins = match config.get("returnOrigins") {
            Some(Value::Array(a)) => a.clone(),
            Some(v) if v.is_null() || is_undef(v) => Vec::new(),
            Some(_) => return Err("values.map is not a function".to_string()),
            None => Vec::new(),
        };
        if polar_return_origins(&origins)?.is_empty() {
            return Err("Production Polar billing requires POLAR_RETURN_ORIGINS".to_string());
        }
    }
    let catalog = config.get("expectedCatalog");
    if environment == "production" && !catalog.is_some_and(truthy) {
        return Err(
            "Production Polar billing requires the POLAR_EXPECTED_* catalog settings".to_string(),
        );
    }
    if let Some(catalog) = catalog.filter(|c| truthy(c)) {
        let currency_ok = catalog
            .get("currency")
            .and_then(Value::as_str)
            .is_some_and(|c| c.len() == 3 && c.chars().all(|c| c.is_ascii_uppercase()));
        let amount_ok = |key: &str| {
            catalog
                .get(key)
                .and_then(Value::as_f64)
                .is_some_and(|n| n.fract() == 0.0 && n.abs() <= 9_007_199_254_740_991.0 && n > 0.0)
        };
        if !currency_ok
            || !amount_ok("monthlyAmountMinor")
            || !amount_ok("yearlyAmountMinor")
            || catalog.get("taxBehavior").and_then(Value::as_str) != Some("exclusive")
        {
            return Err("Invalid POLAR_EXPECTED_* catalog settings".to_string());
        }
    }
    polar_return_url(
        config.get("checkoutSuccessUrl").unwrap_or(&Value::Null),
        "POLAR_CHECKOUT_SUCCESS_URL",
        config,
    )?;
    polar_return_url(
        config.get("portalReturnUrl").unwrap_or(&Value::Null),
        "POLAR_PORTAL_RETURN_URL",
        config,
    )?;
    Ok(())
}

/// Port of `assertPolarBillingConfig` (`apps/api/src/config.ts:172`).
pub fn assert_polar_billing_config(config: serde_json::Value) -> Result<(), String> {
    let environment = env_get(&config, "environment")?;
    match config {
        Value::Object(map) => {
            let _ = environment;
            assert_polar(&map)
        }
        _ => {
            if environment.as_str() != Some("sandbox") && environment.as_str() != Some("production")
            {
                return Err("POLAR_ENVIRONMENT must be sandbox or production".to_string());
            }
            Ok(())
        }
    }
}

/// Port of `loadPolarBillingConfig` (`apps/api/src/config.ts:203`).
pub fn load_polar_billing_config(env: serde_json::Value) -> Result<serde_json::Value, String> {
    let enabled = env_get(&env, "POLAR_BILLING_ENABLED")?;
    if is_undef(&enabled) || enabled.as_str() == Some("false") {
        return Ok(js::undefined());
    }
    if enabled.as_str() != Some("true") {
        return Err("POLAR_BILLING_ENABLED must be true or false".to_string());
    }
    let polar_env = env_get(&env, "POLAR_ENVIRONMENT")?;
    let env_str = polar_env.as_str().unwrap_or_default();
    if env_str != "sandbox" && env_str != "production" {
        return Err("POLAR_ENVIRONMENT must be sandbox or production".to_string());
    }
    let live = env_get(&env, "POLAR_PRODUCTION_PAYMENTS_ENABLED")?;
    if !is_undef(&live) && live.as_str() != Some("true") && live.as_str() != Some("false") {
        return Err("POLAR_PRODUCTION_PAYMENTS_ENABLED must be true or false".to_string());
    }
    if env_str == "production" && live.as_str() != Some("true") {
        return Err(
            "Production Polar billing requires POLAR_PRODUCTION_PAYMENTS_ENABLED=true".to_string(),
        );
    }
    require_vars(&env, &POLAR_REQUIRED)?;
    for name in POLAR_UUID_NAMES {
        if !is_uuid(&env_get(&env, name)?) {
            return Err(format!("{name} must be a UUID"));
        }
    }
    if strict_eq(
        &env_get(&env, "POLAR_MONTHLY_PRODUCT_ID")?,
        &env_get(&env, "POLAR_YEARLY_PRODUCT_ID")?,
    ) {
        return Err("Polar monthly and yearly products must differ".to_string());
    }
    let has_catalog = POLAR_CATALOG
        .iter()
        .any(|name| !is_undef(&member(&env, name)));
    if has_catalog || env_str == "production" {
        require_vars(&env, &POLAR_CATALOG)?;
    }
    let amount_minor = |name: &str| -> Result<f64, String> {
        let value = env_get(&env, name)?;
        let text = js_string(&value);
        let positive_int =
            !text.is_empty() && text.chars().all(|c| c.is_ascii_digit()) && !text.starts_with('0');
        let amount = text.parse::<f64>().unwrap_or(f64::NAN);
        if !positive_int || !(amount.fract() == 0.0 && amount.abs() <= 9_007_199_254_740_991.0) {
            return Err(format!(
                "{name} must be a positive integer in minor currency units"
            ));
        }
        Ok(amount)
    };
    let mut config = Map::new();
    config.insert("environment".into(), polar_env);
    if live.as_str() == Some("true") {
        config.insert("productionPaymentsEnabled".into(), json!(true));
    }
    let origins_val = env_get(&env, "POLAR_RETURN_ORIGINS")?;
    if !is_undef(&origins_val) {
        let origins_raw: Vec<Value> = match &origins_val {
            Value::String(s) => s
                .split(',')
                .map(|o| json!(trim_js(o)))
                .filter(|o| o.as_str().is_some_and(|s| !s.is_empty()))
                .collect(),
            _ => {
                return Err("env.POLAR_RETURN_ORIGINS.split is not a function".to_string());
            }
        };
        config.insert(
            "returnOrigins".into(),
            json!(polar_return_origins(&origins_raw)?),
        );
    }
    if has_catalog {
        config.insert(
            "expectedCatalog".into(),
            json!({
                "currency": env_get(&env, "POLAR_EXPECTED_CURRENCY")?,
                "monthlyAmountMinor": amount_minor("POLAR_EXPECTED_MONTHLY_AMOUNT_MINOR")?,
                "yearlyAmountMinor": amount_minor("POLAR_EXPECTED_YEARLY_AMOUNT_MINOR")?,
                "taxBehavior": env_get(&env, "POLAR_EXPECTED_TAX_BEHAVIOR")?,
            }),
        );
    }
    for (key, name) in [
        ("accessToken", "POLAR_ACCESS_TOKEN"),
        ("webhookSecret", "POLAR_WEBHOOK_SECRET"),
        ("organizationId", "POLAR_ORGANIZATION_ID"),
        ("monthlyProductId", "POLAR_MONTHLY_PRODUCT_ID"),
        ("yearlyProductId", "POLAR_YEARLY_PRODUCT_ID"),
        ("desktopBenefitId", "POLAR_DESKTOP_BENEFIT_ID"),
        ("checkoutSuccessUrl", "POLAR_CHECKOUT_SUCCESS_URL"),
        ("portalReturnUrl", "POLAR_PORTAL_RETURN_URL"),
    ] {
        config.insert(key.into(), env_get(&env, name)?);
    }
    assert_polar(&config)?;
    let checkout = polar_return_url(
        config.get("checkoutSuccessUrl").unwrap_or(&Value::Null),
        "POLAR_CHECKOUT_SUCCESS_URL",
        &config,
    )?;
    let portal = polar_return_url(
        config.get("portalReturnUrl").unwrap_or(&Value::Null),
        "POLAR_PORTAL_RETURN_URL",
        &config,
    )?;
    config.insert("checkoutSuccessUrl".into(), json!(checkout));
    config.insert("portalReturnUrl".into(), json!(portal));
    Ok(Value::Object(config))
}

// ---- Cloudflare tunnel ------------------------------------------------------

const CF_NAMES: [&str; 4] = [
    "CLOUDFLARE_ACCOUNT_ID",
    "CLOUDFLARE_ZONE_ID",
    "CLOUDFLARE_API_TOKEN",
    "CLOUDFLARE_TUNNEL_DOMAIN",
];

/// Port of `loadCloudflareTunnelConfig` (`apps/api/src/config.ts:413`).
pub fn load_cloudflare_tunnel_config(env: serde_json::Value) -> Result<serde_json::Value, String> {
    env_get(&env, "CLOUDFLARE_ACCOUNT_ID")?;
    if !CF_NAMES.iter().any(|name| !is_undef(&member(&env, name))) {
        return Ok(js::undefined());
    }
    require_vars(&env, &CF_NAMES)?;
    let domain_val = env_get(&env, "CLOUDFLARE_TUNNEL_DOMAIN")?;
    let domain_text = match &domain_val {
        Value::String(s) => s.clone(),
        _ => return Err("env.CLOUDFLARE_TUNNEL_DOMAIN.trim is not a function".to_string()),
    };
    let domain = trim_js(&domain_text).to_lowercase();
    let dns_ok = domain.len() <= 190 && {
        let labels: Vec<&str> = domain.split('.').collect();
        labels.len() >= 2
            && labels[..labels.len() - 1].iter().all(|l| {
                !l.is_empty()
                    && l.chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
                    && l.chars()
                        .last()
                        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
                    && l.chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            })
            && {
                let tld = labels[labels.len() - 1];
                (2..=63).contains(&tld.len()) && tld.chars().all(|c| c.is_ascii_lowercase())
            }
    };
    if !dns_ok {
        return Err("CLOUDFLARE_TUNNEL_DOMAIN must be a DNS zone name".to_string());
    }
    let hex32 = |v: &Value| {
        let s = js_string(v);
        s.len() == 32 && s.chars().all(|c| c.is_ascii_hexdigit())
    };
    if !hex32(&env_get(&env, "CLOUDFLARE_ACCOUNT_ID")?)
        || !hex32(&env_get(&env, "CLOUDFLARE_ZONE_ID")?)
    {
        return Err(
            "Cloudflare account and zone IDs must be 32 hexadecimal characters".to_string(),
        );
    }
    let token_val = env_get(&env, "CLOUDFLARE_API_TOKEN")?;
    let token_text = match &token_val {
        Value::String(s) => s.clone(),
        _ => return Err("env.CLOUDFLARE_API_TOKEN.trim is not a function".to_string()),
    };
    let token = trim_js(&token_text);
    if token.is_empty() {
        return Err("CLOUDFLARE_API_TOKEN must not be empty".to_string());
    }
    Ok(json!({
        "accountId": env_get(&env, "CLOUDFLARE_ACCOUNT_ID")?,
        "zoneId": env_get(&env, "CLOUDFLARE_ZONE_ID")?,
        "apiToken": token,
        "domain": domain,
    }))
}

// ---- dev identity gate ------------------------------------------------------

/// Port of `assertDevIdentityAllowed` (`apps/api/src/config.ts:467`).
pub fn assert_dev_identity_allowed(env: serde_json::Value) -> Result<(), String> {
    if env_get(&env, "NODE_ENV")?.as_str() == Some("production") {
        return Err(
            "IDENTITY_PROVIDER=dev refuses to start with NODE_ENV=production. \
             The dev identity provider accepts any email and prints sign-in codes \
             to stdout; it is for local development only. Unset IDENTITY_PROVIDER \
             to use the real identity provider."
                .to_string(),
        );
    }
    if truthy(&env_get(&env, "WORKOS_API_KEY")?) {
        return Err(
            "IDENTITY_PROVIDER=dev refuses to start while WORKOS_API_KEY is set. \
             A real identity-provider secret means this environment serves real \
             users; unset IDENTITY_PROVIDER to use it, or unset WORKOS_API_KEY \
             for offline development."
                .to_string(),
        );
    }
    Ok(())
}

// ---- avatar storage ---------------------------------------------------------

const S3_NAMES: [&str; 5] = [
    "S3_ENDPOINT",
    "S3_BUCKET",
    "S3_ACCESS_KEY_ID",
    "S3_SECRET_ACCESS_KEY",
    "S3_PUBLIC_BASE_URL",
];

/// Port of `loadAvatarStorageConfig` (`apps/api/src/config.ts:520`).
pub fn load_avatar_storage_config(env: serde_json::Value) -> Result<serde_json::Value, String> {
    env_get(&env, "S3_ENDPOINT")?;
    let present: Vec<&str> = S3_NAMES
        .iter()
        .copied()
        .filter(|name| truthy(&member(&env, name)))
        .collect();
    if present.is_empty() {
        return Ok(js::undefined());
    }
    if present.len() < S3_NAMES.len() {
        let missing: Vec<&str> = S3_NAMES
            .iter()
            .copied()
            .filter(|name| !truthy(&member(&env, name)))
            .collect();
        return Err(format!(
            "Avatar storage is partially configured — missing: {}. \
             Set all of S3_ENDPOINT, S3_BUCKET, S3_ACCESS_KEY_ID, S3_SECRET_ACCESS_KEY, \
             and S3_PUBLIC_BASE_URL, or unset them all to run without avatar uploads.",
            missing.join(", ")
        ));
    }
    let strip_slashes = |key: &str| -> Result<String, String> {
        match env_get(&env, key)? {
            Value::String(s) => Ok(s.trim_end_matches('/').to_string()),
            _ => Err(format!("env.{key}.replace is not a function")),
        }
    };
    let region = match env_get(&env, "S3_REGION")? {
        Value::String(s) => {
            let t = trim_js(&s);
            if t.is_empty() {
                "auto".to_string()
            } else {
                t.to_string()
            }
        }
        v if is_undef(&v) || v.is_null() => "auto".to_string(),
        _ => return Err("env.S3_REGION.trim is not a function".to_string()),
    };
    Ok(json!({
        "endpoint": strip_slashes("S3_ENDPOINT")?,
        "bucket": env_get(&env, "S3_BUCKET")?,
        "accessKeyId": env_get(&env, "S3_ACCESS_KEY_ID")?,
        "secretAccessKey": env_get(&env, "S3_SECRET_ACCESS_KEY")?,
        "publicBaseUrl": strip_slashes("S3_PUBLIC_BASE_URL")?,
        "region": region,
    }))
}

// ---- api config -------------------------------------------------------------

const REQUIRED_VARS: [&str; 6] = [
    "DATABASE_URL",
    "WORKOS_API_KEY",
    "WORKOS_CLIENT_ID",
    "ACCOUNT_BASE_URL",
    "API_PUBLIC_URL",
    "API_SIGNING_KEY",
];
const REQUIRED_DEV_VARS: [&str; 3] = ["DATABASE_URL", "ACCOUNT_BASE_URL", "API_PUBLIC_URL"];

fn parse_int_10(text: &str) -> f64 {
    let t = text.trim_start();
    let (sign, digits) = match t.strip_prefix('-') {
        Some(d) => (-1.0, d),
        None => (1.0, t.strip_prefix('+').unwrap_or(t)),
    };
    let end = digits
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(digits.len());
    if end == 0 {
        return f64::NAN;
    }
    sign * digits[..end].parse::<f64>().unwrap_or(f64::NAN)
}

fn resolve_trusted_proxy_hops(value: &Value) -> Result<f64, String> {
    match value {
        v if is_undef(v) => Ok(0.0),
        Value::String(s) => {
            let t = trim_js(s);
            if t.is_empty() {
                return Ok(0.0);
            }
            let parsed = parse_int_10(t);
            if parsed.is_nan() || parsed < 0.0 || js::number_to_string(parsed) != t {
                return Ok(0.0);
            }
            Ok(parsed)
        }
        _ => Err("value.trim is not a function".to_string()),
    }
}

/// `a === b` for recorded values: objects never compare equal.
fn strict_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        _ => {
            if is_undef(a) && is_undef(b) {
                return true;
            }
            match (js::as_number(a), js::as_number(b)) {
                (Some(x), Some(y)) => x == y,
                _ => false,
            }
        }
    }
}

fn base64url_decode(text: &str) -> Option<Vec<u8>> {
    base64_decode(
        text,
        "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_",
    )
}

fn signing_seed(env: &Value, name: &str) -> Result<Option<String>, String> {
    let raw = env_get(env, name)?;
    let value = match &raw {
        Value::String(s) => trim_js(s).to_string(),
        v if is_undef(v) || v.is_null() => return Ok(None),
        _ => return Err(format!("env.{name}.trim is not a function")),
    };
    if value.is_empty() {
        return Ok(None);
    }
    if !value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        || base64url_decode(&value).map(|b| b.len()) != Some(32)
    {
        return Err(format!(
            "{name} must be a base64url-encoded 32-byte Ed25519 seed"
        ));
    }
    Ok(Some(value))
}

fn random_signing_seed() -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let bytes = std::fs::read("/dev/urandom")
        .map(|b| b[..32.min(b.len())].to_vec())
        .unwrap_or_else(|_| vec![0u8; 32]);
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = ((chunk[0] as u32) << 16)
            | ((*chunk.get(1).unwrap_or(&0) as u32) << 8)
            | *chunk.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if chunk.len() * 8 / 6 > i {
                out.push(ALPHABET[(n >> (18 - i * 6)) as usize & 63] as char);
            }
        }
    }
    out
}

fn optional_string(env: &Value, name: &str) -> Result<Option<String>, String> {
    match env_get(env, name)? {
        Value::String(s) => {
            let t = trim_js(&s);
            Ok(if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            })
        }
        v if is_undef(&v) || v.is_null() => Ok(None),
        _ => Err(format!("env.{name}.trim is not a function")),
    }
}

fn parse_identity_map(raw: &str, client_id: &str) -> Result<Value, String> {
    let bad = || {
        "WORKOS_IDENTITY_MAP must contain unambiguous user/org ID pairs bound to WORKOS_CLIENT_ID"
            .to_string()
    };
    let parsed: Value = serde_json::from_str(raw).map_err(|_| bad())?;
    let obj = parsed.as_object().ok_or_else(bad)?;
    if obj.get("clientId").and_then(Value::as_str) != Some(client_id) {
        return Err(bad());
    }
    for (prefix, key) in [("user_", "users"), ("org_", "organizations")] {
        let pairs = match obj.get(key) {
            None => continue,
            Some(v) => v.as_object().ok_or_else(bad)?,
        };
        let mut destinations = std::collections::HashSet::new();
        for (previous, current) in pairs {
            let current = current.as_str().ok_or_else(bad)?;
            if !previous.starts_with(prefix)
                || !current.starts_with(prefix)
                || previous.len() <= prefix.len()
                || current.len() <= prefix.len()
                || pairs.contains_key(current)
                || !destinations.insert(current)
            {
                return Err(bad());
            }
        }
    }
    Ok(parsed)
}

/// Port of `loadApiConfig` (`apps/api/src/config.ts:542`).
pub fn load_api_config(env: serde_json::Value) -> Result<serde_json::Value, String> {
    let identity_provider = env_get(&env, "IDENTITY_PROVIDER")?;
    let provider = if is_undef(&identity_provider) {
        "workos".to_string()
    } else {
        js_string(&identity_provider)
    };
    if provider != "workos" && provider != "dev" {
        return Err(format!(
            "Unknown IDENTITY_PROVIDER \"{provider}\" — expected \"workos\" or \"dev\""
        ));
    }
    let port = match env_get(&env, "PORT")? {
        v if truthy(&v) => parse_int_10(&js_string(&v)),
        _ => 8788.0,
    };
    let trusted_proxy_hops = resolve_trusted_proxy_hops(&env_get(&env, "TRUSTED_PROXY_HOPS")?)?;
    let profile_proxy_secret = optional_string(&env, "PROFILE_PROXY_SECRET")?;
    let avatar_storage = load_avatar_storage_config(env.clone())?;
    let polar_billing = load_polar_billing_config(env.clone())?;
    let remote_billing = env_get(&env, "POLAR_REMOTE_ACCESS_ENFORCED")?;
    if !is_undef(&remote_billing)
        && remote_billing.as_str() != Some("true")
        && remote_billing.as_str() != Some("false")
    {
        return Err("POLAR_REMOTE_ACCESS_ENFORCED must be true or false".to_string());
    }
    let remote_billing_enforced = remote_billing.as_str() == Some("true");
    if remote_billing_enforced && is_undef(&polar_billing) {
        return Err("POLAR_REMOTE_ACCESS_ENFORCED requires configured Polar billing".to_string());
    }
    let profile_billing = env_get(&env, "POLAR_PROFILE_ACCESS_ENFORCED")?;
    if !is_undef(&profile_billing)
        && profile_billing.as_str() != Some("true")
        && profile_billing.as_str() != Some("false")
    {
        return Err("POLAR_PROFILE_ACCESS_ENFORCED must be true or false".to_string());
    }
    let profile_billing_enforced = profile_billing.as_str() == Some("true");
    if profile_billing_enforced && is_undef(&polar_billing) {
        return Err("POLAR_PROFILE_ACCESS_ENFORCED requires configured Polar billing".to_string());
    }
    let apns = load_apns_config(env.clone())?;
    let cloudflare_tunnel = load_cloudflare_tunnel_config(env.clone())?;
    let remote_test_user_ids = match env_get(&env, "REMOTE_TEST_USER_IDS")? {
        Value::String(s) => Some(
            s.split(',')
                .map(|id| trim_js(id).to_string())
                .filter(|id| !id.is_empty())
                .collect::<Vec<_>>(),
        ),
        v if is_undef(&v) || v.is_null() => None,
        _ => {
            return Err("env.REMOTE_TEST_USER_IDS.split is not a function".to_string());
        }
    };
    let beta_access = env_get(&env, "REMOTE_BETA_ACCESS_ENABLED")?;
    if !is_undef(&beta_access)
        && beta_access.as_str() != Some("true")
        && beta_access.as_str() != Some("false")
    {
        return Err("REMOTE_BETA_ACCESS_ENABLED must be true or false".to_string());
    }
    let remote_beta_access_enabled = beta_access.as_str() == Some("true");

    let mut out = Map::new();
    out.insert("identityProvider".into(), json!(provider));
    if !is_undef(&cloudflare_tunnel) {
        out.insert("cloudflareTunnel".into(), cloudflare_tunnel);
    }
    if let Some(ids) = remote_test_user_ids {
        out.insert("remoteTestUserIds".into(), json!(ids));
    }
    out.insert(
        "remoteBetaAccessEnabled".into(),
        json!(remote_beta_access_enabled),
    );
    out.insert(
        "remoteBillingEnforced".into(),
        json!(remote_billing_enforced),
    );
    out.insert(
        "profileBillingEnforced".into(),
        json!(profile_billing_enforced),
    );

    let strip_slashes = |key: &str| -> Result<Value, String> {
        match env_get(&env, key)? {
            Value::String(s) => Ok(json!(s.trim_end_matches('/').to_string())),
            _ => Err(format!("env.{key}.replace is not a function")),
        }
    };

    if provider == "dev" {
        assert_dev_identity_allowed(env.clone())?;
        require_vars(&env, &REQUIRED_DEV_VARS)?;
        let configured = signing_seed(&env, "API_SIGNING_KEY")?;
        let previous = signing_seed(&env, "API_SIGNING_KEY_PREVIOUS")?;
        let api_signing_key = configured.unwrap_or_else(random_signing_seed);
        out.insert("databaseUrl".into(), env_get(&env, "DATABASE_URL")?);
        out.insert("baseUrl".into(), env_get(&env, "ACCOUNT_BASE_URL")?);
        out.insert("apiPublicUrl".into(), strip_slashes("API_PUBLIC_URL")?);
        out.insert("apiSigningKey".into(), json!(api_signing_key));
        if let Some(prev) = previous {
            out.insert("apiSigningKeyPrevious".into(), json!(prev));
        }
    } else {
        require_vars(&env, &REQUIRED_VARS)?;
        let workos_client_id = env_get(&env, "WORKOS_CLIENT_ID")?;
        let identity_map = match env_get(&env, "WORKOS_IDENTITY_MAP")? {
            v if truthy(&v) => Some(parse_identity_map(
                &js_string(&v),
                workos_client_id.as_str().unwrap_or_default(),
            )?),
            _ => None,
        };
        let api_signing_key = signing_seed(&env, "API_SIGNING_KEY")?
            .ok_or("API_SIGNING_KEY must be a base64url-encoded 32-byte Ed25519 seed")?;
        let previous = signing_seed(&env, "API_SIGNING_KEY_PREVIOUS")?;
        let workos_api_url = match env_get(&env, "WORKOS_API_URL")? {
            Value::String(s) => s.trim_end_matches('/').to_string(),
            v if is_undef(&v) || v.is_null() => "https://api.workos.com".to_string(),
            _ => {
                return Err("(intermediate value).replace is not a function".to_string());
            }
        };
        out.insert("databaseUrl".into(), env_get(&env, "DATABASE_URL")?);
        out.insert("baseUrl".into(), env_get(&env, "ACCOUNT_BASE_URL")?);
        out.insert("apiPublicUrl".into(), strip_slashes("API_PUBLIC_URL")?);
        out.insert("apiSigningKey".into(), json!(api_signing_key));
        if let Some(prev) = previous {
            out.insert("apiSigningKeyPrevious".into(), json!(prev));
        }
        out.insert("port".into(), json!(port));
        out.insert("trustedProxyHops".into(), json!(trusted_proxy_hops));
        if let Some(secret) = &profile_proxy_secret {
            out.insert("profileProxySecret".into(), json!(secret));
        }
        if !is_undef(&avatar_storage) {
            out.insert("avatarStorage".into(), avatar_storage);
        }
        if !is_undef(&polar_billing) {
            out.insert("polarBilling".into(), polar_billing);
        }
        if !is_undef(&apns) {
            out.insert("apns".into(), apns);
        }
        out.insert("workosApiKey".into(), env_get(&env, "WORKOS_API_KEY")?);
        out.insert("workosClientId".into(), workos_client_id);
        if let Some(map) = identity_map {
            out.insert("workosIdentityMap".into(), map);
        }
        out.insert("workosApiUrl".into(), json!(workos_api_url));
        let jwks = env_get(&env, "WORKOS_JWKS_URL")?;
        if truthy(&jwks) {
            out.insert("workosJwksUrl".into(), jwks);
        }
        let issuer = env_get(&env, "WORKOS_ISSUER")?;
        if truthy(&issuer) {
            out.insert("workosIssuer".into(), issuer);
        }
        return Ok(Value::Object(out));
    }

    out.insert("port".into(), json!(port));
    out.insert("trustedProxyHops".into(), json!(trusted_proxy_hops));
    if let Some(secret) = &profile_proxy_secret {
        out.insert("profileProxySecret".into(), json!(secret));
    }
    if !is_undef(&avatar_storage) {
        out.insert("avatarStorage".into(), avatar_storage);
    }
    if !is_undef(&polar_billing) {
        out.insert("polarBilling".into(), polar_billing);
    }
    if !is_undef(&apns) {
        out.insert("apns".into(), apns);
    }
    Ok(Value::Object(out))
}
