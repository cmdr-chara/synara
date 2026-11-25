//! HTTP MCP connector OAuth sign-in — lean port of upstream
//! `apps/server/src/mcpConnectors/oauth.ts`. The dance: probe the server,
//! discover the authorization server metadata, dynamically register the
//! Synara client, build the authorize URL with PKCE S256, park the pending
//! sign-in keyed by state, and complete it when `/oauth/mcp/callback`
//! returns the user with a code. Tokens live in the state database; the
//! callback route lives in main.rs.
use crate::todo::WsErr;
use serde_json::{json, Value};
use sha2::Digest;
use std::sync::{Mutex, OnceLock};

fn err(msg: impl Into<String>) -> WsErr {
    crate::todo::err_pub(msg)
}

pub const SYNARA_OAUTH_CLIENT_NAME: &str = "Synara";
pub const SYNARA_OAUTH_CLIENT_URI: &str = "https://www.trysynara.com";

#[derive(Clone)]
struct PendingSignIn {
    connector_input: Value,
    code_verifier: String,
    client_id: String,
    token_endpoint: String,
    created_at_ms: i64,
}

static PENDING: OnceLock<Mutex<std::collections::HashMap<String, PendingSignIn>>> = OnceLock::new();

fn pending() -> &'static Mutex<std::collections::HashMap<String, PendingSignIn>> {
    PENDING.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn random_urlsafe(len: usize) -> String {
    // /dev/urandom only — OAuth state and verifiers are bearer-equivalent.
    let mut bytes = vec![0u8; len];
    if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
        use std::io::Read;
        let _ = f.read_exact(&mut bytes);
    } else {
        panic!("/dev/urandom unavailable; refusing weak randomness for OAuth secrets");
    }
    crate::auth_sessions::base64url_encode(&bytes)
}

fn pkce_challenge(verifier: &str) -> String {
    let digest = sha2::Sha256::digest(verifier.as_bytes());
    crate::auth_sessions::base64url_encode(&digest)
}

fn http_json(method: &str, url: &str, body: Option<Value>, form: Option<&str>) -> Result<(u16, Value), WsErr> {
    // SSRF guard mirrors upstream: the dance is locked to the connector's own
    // origin by construction. The blocking client owns an internal tokio
    // runtime, which panics when dropped inside an async context — WS handlers
    // run on tokio worker threads, so the request hops onto a plain std
    // thread where dropping the runtime is legal.
    let method = method.to_string();
    let url = url.to_string();
    let form = form.map(str::to_string);
    std::thread::scope(|scope| {
        scope
            .spawn(move || {
                let client = reqwest::blocking::Client::builder()
                    .timeout(std::time::Duration::from_secs(20))
                    .redirect(reqwest::redirect::Policy::limited(3))
                    .build()
                    .map_err(|e| err(format!("OAuth http client: {e}")))?;
                let mut req = match method.as_str() {
                    "POST" => client.post(&url),
                    _ => client.get(&url),
                };
                if let Some(b) = body {
                    req = req.json(&b);
                }
                if let Some(f) = form {
                    req = req
                        .header("Content-Type", "application/x-www-form-urlencoded")
                        .body(f);
                }
                let resp = req.send().map_err(|e| err(format!("OAuth request failed: {e}")))?;
                let status = resp.status().as_u16();
                let text = resp.text().unwrap_or_default();
                let parsed = serde_json::from_str::<Value>(&text).unwrap_or_else(|_| json!({}));
                Ok((status, parsed))
            })
            .join()
            .map_err(|_| err("OAuth request thread panicked"))?
    })
}

fn endpoint_origin(url: &str) -> Result<String, WsErr> {
    let parts: Vec<&str> = url.splitn(4, '/').collect();
    if parts.len() < 3 || !url.starts_with("https://") && !url.starts_with("http://127.0.0.1") && !url.starts_with("http://localhost") {
        return Err(err("Connector URL must be an https URL (loopback http allowed)."));
    }
    Ok(format!("{}//{}", parts[0], parts[2]))
}

/// Begin the OAuth sign-in for one HTTP connector input. Returns the
/// authorize URL to open in the user's browser.
pub fn begin_sign_in(connector_input: &Value, redirect_url: &str) -> Result<String, WsErr> {
    let server_url = connector_input
        .get("url")
        .and_then(Value::as_str)
        .ok_or_else(|| err("missing connector url"))?;
    let origin = endpoint_origin(server_url)?;

    // Discovery: RFC 8414 well-known metadata at the server origin. A missing
    // document means the server does not speak OAuth — fail explicitly rather
    // than guessing endpoints.
    let metadata_url = format!("{origin}/.well-known/oauth-authorization-server");
    let (status, metadata) = http_json("GET", &metadata_url, None, None)?;
    if status != 200 {
        return Err(err(format!(
            "The connector server does not publish OAuth metadata (status {status})."
        )));
    }
    let authorization_endpoint = metadata
        .get("authorization_endpoint")
        .and_then(Value::as_str)
        .ok_or_else(|| err("OAuth metadata is missing authorization_endpoint"))?
        .to_string();
    let token_endpoint = metadata
        .get("token_endpoint")
        .and_then(Value::as_str)
        .ok_or_else(|| err("OAuth metadata is missing token_endpoint"))?
        .to_string();

    // Dynamic client registration (RFC 7591) when the server offers it;
    // metadata without a registration endpoint means the user must register
    // their own app — upstream surfaces that as the client-required error.
    let client_metadata = json!({
        "client_name": SYNARA_OAUTH_CLIENT_NAME,
        "client_uri": SYNARA_OAUTH_CLIENT_URI,
        "redirect_uris": [redirect_url],
        "grant_types": ["authorization_code", "refresh_token"],
        "response_types": ["code"],
        "token_endpoint_auth_method": "none",
    });
    let registration_endpoint = metadata.get("registration_endpoint").and_then(Value::as_str);
    let client_id = if let Some(reg) = registration_endpoint {
        let (reg_status, reg_body) = http_json("POST", reg, Some(client_metadata), None)?;
        if !(200..300).contains(&reg_status) {
            return Err(err(format!("OAuth client registration failed (status {reg_status}).")));
        }
        reg_body
            .get("client_id")
            .and_then(Value::as_str)
            .ok_or_else(|| err("OAuth registration response is missing client_id"))?
            .to_string()
    } else {
        return Err(err("mcp-connector-client-required"));
    };

    let code_verifier = random_urlsafe(64);
    let challenge = pkce_challenge(&code_verifier);
    let state = random_urlsafe(32);
    let pending_entry = PendingSignIn {
        connector_input: connector_input.clone(),
        code_verifier,
        client_id: client_id.clone(),
        token_endpoint,
        created_at_ms: now_ms(),
    };
    {
        let mut map = pending().lock().map_err(|_| err("pending sign-in lock"))?;
        // Cap outstanding sign-ins; entries expire after 10 minutes.
        map.retain(|_, p| now_ms() - p.created_at_ms < 10 * 60 * 1000);
        if map.len() >= 64 {
            return Err(err("Too many pending sign-ins; complete or wait for existing ones."));
        }
        map.insert(state.clone(), pending_entry);
    }

    let mut authorize = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&state={}&code_challenge={}&code_challenge_method=S256",
        authorization_endpoint,
        urlencode(&client_id),
        urlencode(redirect_url),
        urlencode(&state),
        urlencode(&challenge),
    );
    // RFC 8707 resource indicator binds the grant to this MCP server.
    authorize.push_str(&format!("&resource={}", urlencode(server_url)));
    Ok(authorize)
}

fn urlencode(v: &str) -> String {
    v.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            _ => format!("%{:02X}", c as u32),
        })
        .collect()
}

/// Complete a sign-in from the callback query string. On success returns the
/// stored tokens and the original connector input.
pub fn complete_sign_in(query: &str) -> Result<(Value, Value), WsErr> {
    let mut code = None;
    let mut state = None;
    for pair in query.split('&') {
        let mut kv = pair.splitn(2, '=');
        match (kv.next(), kv.next()) {
            (Some("code"), Some(v)) => code = Some(urldecode(v)),
            (Some("state"), Some(v)) => state = Some(v.to_string()),
            _ => {}
        }
    }
    let (code, state) = match (code, state) {
        (Some(c), Some(s)) => (c, s),
        _ => return Err(err("missing code or state")),
    };
    let entry = {
        let mut map = pending().lock().map_err(|_| err("pending sign-in lock"))?;
        map.remove(&state)
            .ok_or_else(|| err("Unknown or expired sign-in state."))?
    };
    if now_ms() - entry.created_at_ms > 10 * 60 * 1000 {
        return Err(err("The sign-in timed out. Start again."));
    }
    let redirect_url = format!(
        "http://127.0.0.1:{}/oauth/mcp/callback",
        crate::mcp_connectors::server_port()
    );
    let form = format!(
        "grant_type=authorization_code&code={}&redirect_uri={}&client_id={}&code_verifier={}",
        urlencode(&code),
        urlencode(&redirect_url),
        urlencode(&entry.client_id),
        urlencode(&entry.code_verifier),
    );
    let (status, tokens) = http_json("POST", &entry.token_endpoint, None, Some(&form))?;
    if !(200..300).contains(&status) {
        let desc = tokens
            .get("error_description")
            .or_else(|| tokens.get("error"))
            .and_then(Value::as_str)
            .unwrap_or("token exchange failed");
        return Err(err(format!("OAuth token exchange failed: {desc}")));
    }
    if tokens.get("access_token").and_then(Value::as_str).is_none() {
        return Err(err("OAuth token response is missing access_token"));
    }
    Ok((entry.connector_input, tokens))
}

fn urldecode(v: &str) -> String {
    let mut out = String::new();
    let bytes = v.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(n) = u8::from_str_radix(&v[i + 1..i + 3], 16) {
                out.push(n as char);
                i += 3;
                continue;
            }
        }
        out.push(if bytes[i] == b'+' { ' ' } else { bytes[i] as char });
        i += 1;
    }
    out
}
