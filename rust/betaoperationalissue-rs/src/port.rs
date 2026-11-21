//! The Rust port of `apps/server/src/betaOperationalIssue.ts`.

#![forbid(unsafe_code)]

use std::io::Write;

use serde_json::Value;

use crate::js;

const DESKTOP_DIAGNOSTIC_ISSUE_PREFIX: &str = "SYNARA_DIAGNOSTIC_ISSUE ";
const MAX_DURATION_MS: f64 = 604_800_000.0;

const CODES: &[&str] = &[
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

const REASONS: &[&str] = &[
    "unknown",
    "auth",
    "invalid-response",
    "timeout",
    "output-limit",
    "live-owner",
    "unknown-owner",
];

// `Schema.is(DesktopDiagnosticIssue)`: object with `code` in the literal set,
// optional `reason` in its literal set, optional `durationMs` finite.
fn is_diagnostic_issue(issue: &Value) -> bool {
    let Value::Object(m) = issue else {
        return false;
    };
    let lit = |v: Option<&Value>, set: &[&str]| matches!(v, Some(Value::String(s)) if set.contains(&s.as_str()));
    if !lit(m.get("code"), CODES) {
        return false;
    }
    if let Some(reason) = m.get("reason")
        && !js::is_undefined(reason)
        && !lit(Some(reason), REASONS)
    {
        return false;
    }
    if let Some(duration) = m.get("durationMs")
        && !js::is_undefined(duration)
        && !matches!(js::as_number(duration), Some(n) if n.is_finite())
    {
        return false;
    }
    true
}

fn truthy(v: &Value) -> bool {
    !(v.is_null()
        || js::is_undefined(v)
        || matches!(v, Value::Bool(false))
        || matches!(v, Value::String(s) if s.is_empty())
        || js::as_number(v) == Some(0.0))
}

/// Port of `reportBetaOperationalIssue` (`apps/server/src/betaOperationalIssue.ts:6`).
pub fn report_beta_operational_issue(
    issue: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if jscore::synara_desktop_flavor() != "beta" || !is_diagnostic_issue(&issue) {
        return Ok(js::undefined());
    }
    let mut safe = serde_json::Map::new();
    if let Some(code) = issue.get("code") {
        safe.insert("code".to_string(), code.clone());
    }
    if let Some(reason) = issue.get("reason").filter(|v| truthy(v)) {
        safe.insert("reason".to_string(), reason.clone());
    }
    if let Some(duration) = issue.get("durationMs").filter(|v| !js::is_undefined(v))
        && let Some(n) = js::as_number(duration).filter(|n| n.is_finite() && *n >= 0.0)
    {
        safe.insert(
            "durationMs".to_string(),
            js::number(js::math_round(n).min(MAX_DURATION_MS)),
        );
    }
    let _ = std::io::stdout().write_all(
        format!(
            "{DESKTOP_DIAGNOSTIC_ISSUE_PREFIX}{}\n",
            jscore::json_stringify(&Value::Object(safe))
        )
        .as_bytes(),
    );
    Ok(js::undefined())
}
