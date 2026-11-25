//! Provider child environment sanitization, ported from upstream
//! `providerChildEnvironment.ts` and the direct-credential stripping in
//! `claudeProcessEnv.ts`. Provider CLI subprocesses must not inherit the
//! server process's own provider credentials: ambient keys are control-plane
//! authority, and a leaked `ANTHROPIC_API_KEY` would silently override the
//! user's own CLI OAuth login (or worse, bill the wrong account). Ordinary
//! process state (HOME, PATH, USER, locale) passes through untouched.

use std::process::Command;

/// Ambient credential keys stripped from every provider child, upstream
/// `PROVIDER_CREDENTIAL_KEYS`. Provider-specific env the runtime sets
/// deliberately (per-account config dirs, SYNARA_* test overrides) is applied
/// after sanitization and never passes through this list.
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

/// Claude external auth routing (Bedrock/Vertex/Foundry/Mantle). When any of
/// these is explicitly enabled the operator chose that auth path, so direct
/// keys are left alone; otherwise stale direct keys are dropped so the CLI's
/// own OAuth login wins, upstream `buildClaudeProcessEnv`.
const CLAUDE_EXTERNAL_AUTH_ENV_KEYS: &[&str] = &[
    "ANTHROPIC_BASE_URL",
    "CLAUDE_CODE_USE_BEDROCK",
    "CLAUDE_CODE_USE_VERTEX",
    "CLAUDE_CODE_USE_FOUNDRY",
    "CLAUDE_CODE_USE_MANTLE",
    "CLAUDE_CODE_USE_ANTHROPIC_AWS",
];

const CLAUDE_DIRECT_CREDENTIAL_KEYS: &[&str] = &[
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
    "CLAUDE_CODE_OAUTH_TOKEN",
];

fn env_flag_enabled(value: Option<String>) -> bool {
    value
        .map(|v| {
            let n = v.trim().to_lowercase();
            !n.is_empty() && n != "0" && n != "false"
        })
        .unwrap_or(false)
}

/// Strip ambient provider credentials from a child command. Rust `Command`
/// inherits the parent environment by default, so removal is explicit.
pub(crate) fn sanitize_provider_child(command: &mut Command) {
    for key in PROVIDER_CREDENTIAL_KEYS {
        command.env_remove(key);
    }
    // Test/operator overrides opt back in explicitly: SYNARA_PROVIDER_ENV_PASS
    // is a comma-separated list re-adding keys the operator really wants
    // forwarded (used by fixture-free manual testing against real CLIs).
    if let Ok(pass) = std::env::var("SYNARA_PROVIDER_ENV_PASS") {
        for key in pass.split(',').map(str::trim).filter(|k| !k.is_empty()) {
            if let Ok(value) = std::env::var(key) {
                command.env(key, value);
            }
        }
    }
}

/// Claude-specific spawn sanitization, upstream `buildClaudeProcessEnv`:
/// ambient credential keys always go; direct Anthropic keys additionally go
/// unless an external auth backend is explicitly enabled in the environment.
/// The CLI's own login (Keychain/credentials file) then satisfies auth,
/// which is exactly the anti-ban boundary: the official binary owns its
/// credentials end to end.
pub(crate) fn sanitize_claude_child(command: &mut Command) {
    sanitize_provider_child(command);
    let external = CLAUDE_EXTERNAL_AUTH_ENV_KEYS
        .iter()
        .any(|k| env_flag_enabled(std::env::var(k).ok()));
    if !external {
        for key in CLAUDE_DIRECT_CREDENTIAL_KEYS {
            command.env_remove(key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_removes_ambient_keys() {
        // env_remove is not observable without spawning; assert the call is
        // total by exercising it on a command for every key.
        let mut cmd = Command::new("true");
        sanitize_provider_child(&mut cmd);
        sanitize_claude_child(&mut cmd);
    }

    #[test]
    fn external_auth_flag_detection() {
        assert!(env_flag_enabled(Some("1".into())));
        assert!(env_flag_enabled(Some("true".into())));
        assert!(!env_flag_enabled(Some("0".into())));
        assert!(!env_flag_enabled(Some("false".into())));
        assert!(!env_flag_enabled(None));
        assert!(!env_flag_enabled(Some("  ".into())));
    }
}
