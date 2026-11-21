//! The Rust port of `apps/server/src/sensitiveKeys.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

const PROVIDER_CREDENTIAL_KEYS: &[&str] = &[
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
    "CLAUDE_CODE_OAUTH_TOKEN",
    "AWS_ACCESS_KEY_ID",
    "AWS_SECRET_ACCESS_KEY",
    "AWS_SESSION_TOKEN",
    "GEMINI_API_KEY",
    "GOOGLE_API_KEY",
    "GOOGLE_APPLICATION_CREDENTIALS",
    "OPENAI_API_KEY",
    "XAI_API_KEY",
    "GROK_CODE_XAI_API_KEY",
    "FACTORY_API_KEY",
    "CURSOR_API_KEY",
    "DEVIN_API_KEY",
    "WINDSURF_API_KEY",
    "DOCKER_AUTH_CONFIG",
];

const EXACT_SENSITIVE_KEYS: &[&str] = &[
    "accesskey",
    "accesskeyid",
    "apikey",
    "authtoken",
    "authorization",
    "clientsecret",
    "cookie",
    "cookies",
    "credential",
    "credentials",
    "idtoken",
    "passphrase",
    "passwd",
    "password",
    "privatekey",
    "proxyauthorization",
    "pwd",
    "refreshtoken",
    "secret",
    "secretkey",
    "sessiontoken",
    "setcookie",
    "token",
];

const SECRET_TERMINAL_WORDS: &[&str] = &[
    "authorization",
    "cookie",
    "cookies",
    "credential",
    "credentials",
    "passphrase",
    "passwd",
    "password",
    "pwd",
    "secret",
    "secrets",
];

const TOKEN_TERMINAL_WORDS: &[&str] = &["token", "tokens"];

const SECRET_TOKEN_QUALIFIERS: &[&str] = &[
    "access", "api", "auth", "bearer", "bot", "client", "gateway", "id", "jwt", "machine", "oauth",
    "personal", "refresh", "secret", "service", "session", "sso", "user", "webhook",
];

/// JS trim (strips U+FEFF too).
fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
}

/// `isProviderCredentialKey`: `PROVIDER_CREDENTIAL_KEYS.has(key.trim().toUpperCase())`.
fn is_provider_credential_key(key: &str) -> bool {
    PROVIDER_CREDENTIAL_KEYS.contains(&js_trim(key).to_uppercase().as_str())
}

/// `keyTokens`: `([A-Z]+)([A-Z][a-z])` → "$1 $2", `([a-z0-9])([A-Z])` → "$1 $2",
/// then lowercase and match `/[a-z0-9]+/gu`.
fn key_tokens(key: &str) -> Vec<String> {
    let c: Vec<char> = key.chars().collect();
    let n = c.len();
    // Pass 1: acronym boundary — a maximal [A-Z] run of >=2 followed by [a-z]
    // splits before its last char ([A-Z]+ backtracks one so [A-Z][a-z] matches).
    let mut p1 = String::new();
    let mut i = 0usize;
    while i < n {
        if c[i].is_ascii_uppercase() {
            let mut j = i;
            while j < n && c[j].is_ascii_uppercase() {
                j += 1;
            }
            if j - i >= 2 && j < n && c[j].is_ascii_lowercase() {
                p1.extend(&c[i..j - 1]);
                p1.push(' ');
                p1.push(c[j - 1]);
                p1.push(c[j]);
                i = j + 1;
                continue;
            }
            p1.extend(&c[i..j]);
            i = j;
            continue;
        }
        p1.push(c[i]);
        i += 1;
    }
    // Pass 2: ([a-z0-9])([A-Z]) → "$1 $2".
    let c2: Vec<char> = p1.chars().collect();
    let mut s = String::new();
    for (i, &ch) in c2.iter().enumerate() {
        if i > 0
            && (c2[i - 1].is_ascii_lowercase() || c2[i - 1].is_ascii_digit())
            && ch.is_ascii_uppercase()
        {
            s.push(' ');
        }
        s.push(ch);
    }
    // .toLowerCase().match(/[a-z0-9]+/gu)
    let lower = s.to_lowercase();
    let lc: Vec<char> = lower.chars().collect();
    let mut tokens = Vec::new();
    let mut j = 0usize;
    while j < lc.len() {
        if lc[j].is_ascii_lowercase() || lc[j].is_ascii_digit() {
            let start = j;
            while j < lc.len() && (lc[j].is_ascii_lowercase() || lc[j].is_ascii_digit()) {
                j += 1;
            }
            tokens.push(lc[start..j].iter().collect::<String>());
        } else {
            j += 1;
        }
    }
    tokens
}

/// Port of `isSensitiveKey` (`apps/server/src/sensitiveKeys.ts:78`).
pub fn is_sensitive_key(key: String) -> Result<bool, String> {
    if is_provider_credential_key(&key) {
        return Ok(true);
    }
    let normalized: String = key
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_lowercase();
    if EXACT_SENSITIVE_KEYS.contains(&normalized.as_str()) {
        return Ok(true);
    }
    let tokens = key_tokens(&key);
    let Some(terminal) = tokens.last() else {
        return Ok(false);
    };
    let terminal = terminal.as_str();
    if SECRET_TERMINAL_WORDS.contains(&terminal) {
        return Ok(true);
    }
    if TOKEN_TERMINAL_WORDS.contains(&terminal) {
        return Ok(match tokens.get(tokens.len() - 2) {
            None => true,
            Some(q) => SECRET_TOKEN_QUALIFIERS.contains(&q.as_str()),
        });
    }
    Ok(terminal == "key"
        && tokens[..tokens.len() - 1]
            .iter()
            .any(|t| matches!(t.as_str(), "api" | "private" | "proxy" | "secret")))
}

/// Port of `redactSensitiveJsonFields` (`apps/server/src/sensitiveKeys.ts:108`).
pub fn redact_sensitive_json_fields(
    key: String,
    value: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if is_sensitive_key(key)? {
        Ok(Value::String("[redacted]".to_string()))
    } else {
        Ok(value)
    }
}
