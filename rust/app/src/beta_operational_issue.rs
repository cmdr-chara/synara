//! Fixed-category operational issue relay for the owning Beta desktop.
//! Mirrors `apps/server/src/betaOperationalIssue.ts`: the line is machine
//! readable, bounded, and never carries exception text, paths, or arguments.

use serde_json::json;

pub const PREFIX: &str = "SYNARA_DIAGNOSTIC_ISSUE ";

const CODES: [&str; 19] = [
    "server.event-loop.stall",
    "git.request.failed",
    "git.branch.failed",
    "git.commit.failed",
    "git.push.failed",
    "git.pr.failed",
    "voice.record.failed",
    "voice.transcribe.failed",
    "claude.compaction.request-failed",
    "claude.compaction.uncertain",
    "claude.cache.request-failed",
    "claude.cache.failed",
    "claude.cache.uncertain",
    "startup.database-locked",
    "startup.migration-recovery-required",
    "startup.migration-divergence-consent-required",
    "startup.migration-runtime-identity-mismatch",
    "startup.migration-schema-too-new",
    "startup.migration-startup-block-invalid",
];
const REASONS: [&str; 7] = [
    "unknown",
    "auth",
    "invalid-response",
    "timeout",
    "output-limit",
    "live-owner",
    "unknown-owner",
];

fn desktop_flavor() -> String {
    match std::env::var("SYNARA_DESKTOP_BUNDLE_ID")
        .ok()
        .as_deref()
        .map(str::trim)
    {
        Some("com.emanueledipietro.synara") => return "production".into(),
        Some("com.emanueledipietro.synara.dev") => return "development".into(),
        Some("com.emanueledipietro.synara.canary") => return "canary".into(),
        Some("com.emanueledipietro.synara.cua") => return "cua".into(),
        Some("com.emanueledipietro.synara.beta") => return "beta".into(),
        Some("com.emanueledipietro.synara.remote") => return "remote".into(),
        _ => {}
    }
    std::env::var("SYNARA_DESKTOP_FLAVOR").unwrap_or_else(|_| "unknown".into())
}

/// Emit one fixed-shape issue for the desktop diagnostics collector. Missing
/// or non-Beta hosts make this a no-op; telemetry can never alter the result.
pub fn report(code: &str, reason: Option<&str>, duration_ms: Option<u64>) {
    if desktop_flavor() != "beta" || !CODES.contains(&code) {
        return;
    }
    if reason.is_some_and(|value| !REASONS.contains(&value)) {
        return;
    }
    let mut payload = json!({"code": code});
    if let Some(reason) = reason {
        payload["reason"] = json!(reason);
    }
    if let Some(duration) = duration_ms {
        payload["durationMs"] = json!(duration.min(604_800_000));
    }
    println!(
        "{PREFIX}{}",
        serde_json::to_string(&payload).unwrap_or_else(|_| "{}".into())
    );
}

/// Startup migration blocks are already deliberate process exits; duplicate
/// them in the diagnostics channel using only their fixed category.
pub fn report_startup_migration_error(error: &str) {
    let code = if error.contains("SYNARA_MIGRATION_DIVERGENCE_CONSENT_REQUIRED=") {
        "startup.migration-divergence-consent-required"
    } else if error.contains("newer than this build supports") {
        "startup.migration-schema-too-new"
    } else if error.contains("database is locked") || error.contains("database locked") {
        "startup.database-locked"
    } else if error.contains("migration recovery")
        || error.contains("restore")
        || error.contains("resume attempt")
    {
        "startup.migration-recovery-required"
    } else {
        return;
    };
    report(code, None, None);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_code_and_reason_allowlists_are_upstream_shaped() {
        assert!(CODES.contains(&"server.event-loop.stall"));
        assert!(CODES.contains(&"startup.migration-schema-too-new"));
        assert!(!CODES.contains(&"chat.content"));
        assert!(REASONS.contains(&"timeout"));
        assert!(!REASONS.contains(&"private detail"));
    }
}
