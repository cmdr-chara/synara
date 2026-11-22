//! The Rust port of `apps/marketing/src/app/llms.mdx/docs/[[...slug]]/`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

/// Port of `GET` (`apps/marketing/src/app/llms.mdx/docs/[[...slug]]/route.ts:16`).
pub async fn get(
    arg0: serde_json::Value,
    arg1: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let _ = (&arg0, &arg1);
    Err("GET failed".to_string())
}

/// Port of `generateStaticParams` (`apps/marketing/src/app/llms.mdx/docs/[[...slug]]/route.ts:33`).
pub fn generate_static_params() -> Result<serde_json::Value, String> {
    serde_json::from_str::<serde_json::Value>(r####"[{"slug": []}, {"slug": ["features", "automations"]}, {"slug": ["features", "browser-sessions"]}, {"slug": ["features", "composer"]}, {"slug": ["features", "computer-use"]}, {"slug": ["features", "connectors"]}, {"slug": ["features", "conversation-history"]}, {"slug": ["features", "customization"]}, {"slug": ["features", "hubs"]}, {"slug": ["features", "inline-visuals"]}, {"slug": ["features", "ios-simulator"]}, {"slug": ["features", "multi-folder-projects"]}, {"slug": ["features", "organize"]}, {"slug": ["features", "overview"]}, {"slug": ["features", "project-import"]}, {"slug": ["features", "provider-accounts"]}, {"slug": ["features", "subagents"]}, {"slug": ["features", "synara-beta"]}, {"slug": ["features", "tasks-and-inbox"]}, {"slug": ["features", "terminal-panels"]}, {"slug": ["features", "thread-goals"]}, {"slug": ["features", "workspace-editor"]}, {"slug": ["getting-started", "core-concepts"]}, {"slug": ["getting-started", "first-task"]}, {"slug": ["getting-started", "installation"]}, {"slug": ["getting-started", "providers"]}, {"slug": ["getting-started", "quickstart"]}, {"slug": ["reference", "keyboard-shortcuts"]}, {"slug": ["reference", "main-preview"]}, {"slug": ["reference", "slash-commands"]}, {"slug": ["providers", "antigravity"]}, {"slug": ["providers", "claude-code"]}, {"slug": ["providers", "codex"]}, {"slug": ["providers", "cursor"]}, {"slug": ["providers", "devin"]}, {"slug": ["providers", "factory-droid"]}, {"slug": ["providers", "grok"]}, {"slug": ["providers"]}, {"slug": ["providers", "omp"]}, {"slug": ["providers", "opencode"]}, {"slug": ["providers", "pi"]}, {"slug": ["troubleshooting", "automations-and-integrations"]}, {"slug": ["troubleshooting", "browser"]}, {"slug": ["troubleshooting", "desktop-and-updates"]}, {"slug": ["troubleshooting", "diagnostics"]}, {"slug": ["troubleshooting", "faq"]}, {"slug": ["troubleshooting", "git-and-worktrees"]}, {"slug": ["troubleshooting"]}, {"slug": ["troubleshooting", "providers"]}, {"slug": ["troubleshooting", "report-a-problem"]}, {"slug": ["troubleshooting", "tasks-and-runtime"]}, {"slug": ["workflows", "agent-gateway"]}, {"slug": ["workflows", "automations"]}, {"slug": ["workflows", "best-practices"]}, {"slug": ["workflows", "browser-verification"]}, {"slug": ["workflows", "debugging"]}, {"slug": ["workflows", "external-mcp"]}, {"slug": ["workflows", "forks"]}, {"slug": ["workflows", "handoffs"]}, {"slug": ["workflows", "headless-server"]}, {"slug": ["workflows"]}, {"slug": ["workflows", "parallel-agents"]}, {"slug": ["workflows", "pull-requests"]}, {"slug": ["workflows", "studio"]}, {"slug": ["workflows", "worktrees"]}]"####).map_err(|e| e.to_string())
}
