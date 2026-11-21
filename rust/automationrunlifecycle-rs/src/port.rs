//! The Rust port of `packages/shared/src/automationRunLifecycle.ts`.
//!
//! Clean-room reimplementation: inferred solely from recorded cases.jsonl behavior
//! and the exported contract types.

#![forbid(unsafe_code)]

use serde_json::json;

const ACTIVE: [&str; 4] = ["pending", "claimed", "running", "waiting-for-approval"];

const TRANSITIONS: [(&str, &[&str]); 9] = [
    (
        "pending",
        &[
            "running",
            "waiting-for-approval",
            "succeeded",
            "failed",
            "cancelled",
            "interrupted",
            "skipped",
        ],
    ),
    (
        "claimed",
        &[
            "running",
            "waiting-for-approval",
            "succeeded",
            "failed",
            "cancelled",
            "interrupted",
            "skipped",
        ],
    ),
    (
        "running",
        &[
            "waiting-for-approval",
            "succeeded",
            "failed",
            "cancelled",
            "interrupted",
        ],
    ),
    (
        "waiting-for-approval",
        &["running", "succeeded", "failed", "cancelled", "interrupted"],
    ),
    ("succeeded", &[]),
    ("failed", &[]),
    ("cancelled", &[]),
    ("interrupted", &[]),
    ("skipped", &[]),
];

/// Port of `isActiveAutomationRunStatus` (`packages/shared/src/automationRunLifecycle.ts:84`).
pub fn is_active_automation_run_status(status: serde_json::Value) -> Result<bool, String> {
    Ok(status.as_str().is_some_and(|s| ACTIVE.contains(&s)))
}

/// Port of `isTerminalAutomationRunStatus` (`packages/shared/src/automationRunLifecycle.ts:88`).
pub fn is_terminal_automation_run_status(status: serde_json::Value) -> Result<bool, String> {
    Ok(!is_active_automation_run_status(status)?)
}

/// Port of `canTransitionAutomationRun` (`packages/shared/src/automationRunLifecycle.ts:94`).
pub fn can_transition_automation_run(
    from: serde_json::Value,
    to: serde_json::Value,
) -> Result<bool, String> {
    let Some(from_str) = from.as_str() else {
        return Err("Cannot read properties of undefined (reading 'includes')".to_string());
    };
    let Some((_, targets)) = TRANSITIONS.iter().find(|(name, _)| *name == from_str) else {
        return Err("Cannot read properties of undefined (reading 'includes')".to_string());
    };
    Ok(to.as_str().is_some_and(|t| targets.contains(&t)))
}

/// Port of `automationRunStatusesLeadingTo` (`packages/shared/src/automationRunLifecycle.ts:102`).
pub fn automation_run_statuses_leading_to(
    to: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    let to_str = to.as_str();
    Ok(TRANSITIONS
        .iter()
        .filter(|(_, targets)| to_str.is_some_and(|t| targets.contains(&t)))
        .map(|(name, _)| json!(name))
        .collect())
}
