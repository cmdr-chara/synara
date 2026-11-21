//! The Rust port of `apps/web/src/composerDraftModels.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value};

use crate::js;

// ── Static model/provider data (from packages/contracts and packages/shared) ──

const PROVIDER_KINDS: &[&str] = &[
    "codex",
    "claudeAgent",
    "cursor",
    "antigravity",
    "grok",
    "droid",
    "opencode",
    "pi",
    "devin",
    "omp",
];

const COMPOSER_PROVIDER_KINDS: &[&str] = &[
    "codex",
    "claudeAgent",
    "cursor",
    "devin",
    "antigravity",
    "grok",
    "droid",
    "opencode",
    "pi",
    "omp",
];

const DEFAULT_MODEL_BY_PROVIDER: &[(&str, &str)] = &[
    ("codex", "gpt-6-astra"),
    ("claudeAgent", "claude-sonnet-5"),
    ("cursor", "auto"),
    ("devin", "adaptive"),
    ("antigravity", "Gemini 3.5 Flash"),
    ("grok", "grok-4.6"),
    ("droid", "claude-opus-4-8"),
    ("opencode", "openai/gpt-5"),
];

/// `MODEL_OPTIONS_BY_PROVIDER` (slug, name), in declaration order.
const MODEL_OPTIONS: &[(&str, &[(&str, &str)])] = &[
    (
        "codex",
        &[
            ("gpt-6-astra", "GPT-6 Astra"),
            ("gpt-6-sol", "GPT-6 Sol"),
            ("gpt-6-luna", "GPT-6 Luna"),
            ("gpt-5.5", "GPT-5.5"),
            ("gpt-5.4", "GPT-5.4"),
            ("gpt-5.4-mini", "GPT-5.4 Mini"),
            ("gpt-5.3-codex", "GPT-5.3 Codex"),
            ("gpt-5.3-codex-spark", "GPT-5.3 Codex Spark"),
            ("gpt-5.2-codex", "GPT-5.2 Codex"),
            ("gpt-5.2", "GPT-5.2"),
        ],
    ),
    (
        "claudeAgent",
        &[
            ("claude-fable-5-1", "Claude Fable 5.1"),
            ("claude-fable-5", "Claude Fable 5"),
            ("claude-opus-5-5", "Claude Opus 5.5"),
            ("claude-opus-5", "Claude Opus 5"),
            ("claude-opus-4-8", "Claude Opus 4.8"),
            ("claude-opus-4-7", "Claude Opus 4.7"),
            ("claude-opus-4-6", "Claude Opus 4.6"),
            ("claude-opus-4-5", "Claude Opus 4.5"),
            ("claude-sonnet-5", "Claude Sonnet 5"),
            ("claude-sonnet-4-6", "Claude Sonnet 4.6"),
            ("claude-haiku-4-5", "Claude Haiku 4.5"),
        ],
    ),
    ("antigravity", &[]),
    ("grok", &[("grok-4.6", "Grok 4.6")]),
    (
        "droid",
        &[
            ("auto", "Auto Model"),
            ("claude-fable-5", "Claude Fable 5"),
            ("claude-opus-4-8", "Claude Opus 4.8"),
            ("claude-opus-4-8-fast", "Claude Opus 4.8 Fast"),
            ("claude-opus-4-7", "Claude Opus 4.7"),
            ("claude-opus-4-7-fast", "Claude Opus 4.7 Fast"),
            ("claude-opus-4-6", "Claude Opus 4.6"),
            ("claude-sonnet-5", "Claude Sonnet 5"),
            ("claude-sonnet-4-6", "Claude Sonnet 4.6"),
            ("claude-opus-4-5-20251101", "Claude Opus 4.5"),
            ("claude-sonnet-4-5-20250929", "Claude Sonnet 4.5"),
            ("claude-haiku-4-5-20251001", "Claude Haiku 4.5"),
            ("gpt-5.6-sol", "GPT-5.6 Sol"),
            ("gpt-5.6-terra", "GPT-5.6 Terra"),
            ("gpt-5.6-luna", "GPT-5.6 Luna"),
            ("gpt-5.5", "GPT-5.5"),
            ("gpt-5.5-fast", "GPT-5.5 Fast"),
            ("gpt-5.5-pro", "GPT-5.5 Pro"),
            ("gpt-5.4", "GPT-5.4"),
            ("gpt-5.4-fast", "GPT-5.4 Fast"),
            ("gpt-5.4-mini", "GPT-5.4 Mini"),
            ("gpt-5.3-codex", "GPT-5.3 Codex"),
            ("gpt-5.3-codex-fast", "GPT-5.3 Codex Fast"),
            ("gpt-5.2", "GPT-5.2"),
            ("gemini-3.1-pro-preview", "Gemini 3.1 Pro"),
            ("gemini-3.5-flash", "Gemini 3.5 Flash"),
            ("gemini-3-flash-preview", "Gemini 3 Flash"),
            ("glm-5.2", "GLM 5.2"),
            ("glm-5.2-fast", "GLM 5.2 Fast"),
            ("glm-5.1", "GLM 5.1"),
            ("nemotron-3-ultra", "Nemotron 3 Ultra"),
            ("kimi-k2.7-code", "Kimi K2.7 Code"),
            ("kimi-k2.6", "Kimi K2.6"),
            ("deepseek-v4-pro", "DeepSeek V4 Pro"),
            ("deepseek-v4-flash-0731", "DeepSeek V4 Flash 0731"),
            ("minimax-m3", "MiniMax M3"),
            ("minimax-m2.7", "MiniMax M2.7"),
        ],
    ),
    ("opencode", &[("openai/gpt-5", "OpenAI GPT-5")]),
    ("pi", &[]),
    (
        "cursor",
        &[
            ("auto", "Auto"),
            ("composer-2.5", "Composer 2.5"),
            ("claude-opus-5", "Claude Opus 5"),
            ("claude-opus-4-8", "Claude Opus 4.8"),
            ("claude-opus-4-7", "Claude Opus 4.7"),
            ("claude-opus-4-6", "Claude Opus 4.6"),
            ("claude-opus-4-5", "Claude Opus 4.5"),
            ("claude-fable-5", "Claude Fable 5"),
            ("claude-sonnet-5", "Claude Sonnet 5"),
            ("claude-sonnet-4-6", "Claude Sonnet 4.6"),
            ("claude-sonnet-4-5", "Claude Sonnet 4.5"),
            ("claude-sonnet-4", "Claude Sonnet 4"),
            ("claude-haiku-4-5", "Claude Haiku 4.5"),
            ("gpt-5.6-sol", "GPT-5.6 Sol"),
            ("gpt-5.6-terra", "GPT-5.6 Terra"),
            ("gpt-5.6-luna", "GPT-5.6 Luna"),
            ("gpt-5.5", "GPT-5.5"),
            ("gpt-5.4", "GPT-5.4"),
            ("gpt-5.4-mini", "GPT-5.4 Mini"),
            ("gpt-5.4-nano", "GPT-5.4 Nano"),
            ("gpt-5.3-codex", "GPT-5.3 Codex"),
            ("gpt-5.2", "GPT-5.2"),
            ("gpt-5.1", "GPT-5.1"),
            ("gpt-5-mini", "GPT-5 Mini"),
            ("grok-4.5", "Grok 4.5"),
            ("grok-4.6", "Grok 4.6"),
            ("gemini-3.1-pro", "Gemini 3.1 Pro"),
            ("gemini-3.6-flash", "Gemini 3.6 Flash"),
            ("gemini-3.5-flash", "Gemini 3.5 Flash"),
            ("gemini-3-flash", "Gemini 3 Flash"),
            ("gemini-2.5-flash", "Gemini 2.5 Flash"),
            ("kimi-k2.7-code", "Kimi K2.7 Code"),
            ("glm-5.2", "GLM 5.2"),
        ],
    ),
    (
        "devin",
        &[
            ("adaptive", "Adaptive"),
            ("swe-1-6", "SWE 1.6"),
            ("swe-1-7", "SWE 1.7"),
        ],
    ),
    ("omp", &[]),
];

/// `MODEL_SLUG_SET_BY_PROVIDER`: like `MODEL_OPTIONS`, but devin (and the dynamic
/// providers) intentionally keep an empty set.
fn model_slug_set(provider: &str) -> Option<&'static [&'static str]> {
    Some(match provider {
        "claudeAgent" => CLAUDE_SLUGS,
        "codex" => CODEX_SLUGS,
        "cursor" => CURSOR_SLUGS,
        "antigravity" => &[],
        "grok" => &["grok-4.6"],
        "droid" => DROID_SLUGS,
        "opencode" => &["openai/gpt-5"],
        "pi" | "devin" | "omp" => &[],
        _ => return None,
    })
}

const CODEX_SLUGS: &[&str] = &[
    "gpt-6-astra",
    "gpt-6-sol",
    "gpt-6-luna",
    "gpt-5.5",
    "gpt-5.4",
    "gpt-5.4-mini",
    "gpt-5.3-codex",
    "gpt-5.3-codex-spark",
    "gpt-5.2-codex",
    "gpt-5.2",
];
const CLAUDE_SLUGS: &[&str] = &[
    "claude-fable-5-1",
    "claude-fable-5",
    "claude-opus-5-5",
    "claude-opus-5",
    "claude-opus-4-8",
    "claude-opus-4-7",
    "claude-opus-4-6",
    "claude-opus-4-5",
    "claude-sonnet-5",
    "claude-sonnet-4-6",
    "claude-haiku-4-5",
];
const DROID_SLUGS: &[&str] = &[
    "auto",
    "claude-fable-5",
    "claude-opus-4-8",
    "claude-opus-4-8-fast",
    "claude-opus-4-7",
    "claude-opus-4-7-fast",
    "claude-opus-4-6",
    "claude-sonnet-5",
    "claude-sonnet-4-6",
    "claude-opus-4-5-20251101",
    "claude-sonnet-4-5-20250929",
    "claude-haiku-4-5-20251001",
    "gpt-5.6-sol",
    "gpt-5.6-terra",
    "gpt-5.6-luna",
    "gpt-5.5",
    "gpt-5.5-fast",
    "gpt-5.5-pro",
    "gpt-5.4",
    "gpt-5.4-fast",
    "gpt-5.4-mini",
    "gpt-5.3-codex",
    "gpt-5.3-codex-fast",
    "gpt-5.2",
    "gemini-3.1-pro-preview",
    "gemini-3.5-flash",
    "gemini-3-flash-preview",
    "glm-5.2",
    "glm-5.2-fast",
    "glm-5.1",
    "nemotron-3-ultra",
    "kimi-k2.7-code",
    "kimi-k2.6",
    "deepseek-v4-pro",
    "deepseek-v4-flash-0731",
    "minimax-m3",
    "minimax-m2.7",
];
const CURSOR_SLUGS: &[&str] = &[
    "auto",
    "composer-2.5",
    "claude-opus-5",
    "claude-opus-4-8",
    "claude-opus-4-7",
    "claude-opus-4-6",
    "claude-opus-4-5",
    "claude-fable-5",
    "claude-sonnet-5",
    "claude-sonnet-4-6",
    "claude-sonnet-4-5",
    "claude-sonnet-4",
    "claude-haiku-4-5",
    "gpt-5.6-sol",
    "gpt-5.6-terra",
    "gpt-5.6-luna",
    "gpt-5.5",
    "gpt-5.4",
    "gpt-5.4-mini",
    "gpt-5.4-nano",
    "gpt-5.3-codex",
    "gpt-5.2",
    "gpt-5.1",
    "gpt-5-mini",
    "grok-4.5",
    "grok-4.6",
    "gemini-3.1-pro",
    "gemini-3.6-flash",
    "gemini-3.5-flash",
    "gemini-3-flash",
    "gemini-2.5-flash",
    "kimi-k2.7-code",
    "glm-5.2",
];

/// `MODEL_SLUG_ALIASES_BY_PROVIDER` (lowercased lookup keys).
fn model_aliases(provider: &str) -> Option<&'static [(&'static str, &'static str)]> {
    Some(match provider {
        "codex" => ALIASES_CODEX,
        "claudeAgent" => ALIASES_CLAUDE,
        "cursor" => ALIASES_CURSOR,
        "antigravity" => &[],
        "droid" => ALIASES_DROID,
        "grok" => ALIASES_GROK,
        "opencode" => &[],
        "pi" => &[],
        "devin" => ALIASES_DEVIN,
        "omp" => &[],
        _ => return None,
    })
}

const ALIASES_CODEX: &[(&str, &str)] = &[
    ("sol", "gpt-6-sol"),
    ("luna", "gpt-6-luna"),
    ("astra", "gpt-6-astra"),
    ("6", "gpt-6-astra"),
    ("gpt-6", "gpt-6-astra"),
    ("5.5", "gpt-5.5"),
    ("5.4", "gpt-5.4"),
    ("5.3", "gpt-5.3-codex"),
    ("gpt-5.3", "gpt-5.3-codex"),
    ("5.3-spark", "gpt-5.3-codex-spark"),
    ("gpt-5.3-spark", "gpt-5.3-codex-spark"),
];
const ALIASES_CLAUDE: &[(&str, &str)] = &[
    ("fable", "claude-fable-5-1"),
    ("fable-5.1", "claude-fable-5-1"),
    ("claude-fable-5.1", "claude-fable-5-1"),
    ("claude-fable-5-1", "claude-fable-5-1"),
    ("fable-5", "claude-fable-5"),
    ("claude-fable-5", "claude-fable-5"),
    ("opus", "claude-opus-5-5"),
    ("opus-5.5", "claude-opus-5-5"),
    ("claude-opus-5.5", "claude-opus-5-5"),
    ("claude-opus-5-5", "claude-opus-5-5"),
    ("opus-5", "claude-opus-5"),
    ("claude-opus-5", "claude-opus-5"),
    ("opus-4.8", "claude-opus-4-8"),
    ("claude-opus-4.8", "claude-opus-4-8"),
    ("claude-opus-4-8-20260528", "claude-opus-4-8"),
    ("opus-4.7", "claude-opus-4-7"),
    ("claude-opus-4.7", "claude-opus-4-7"),
    ("claude-opus-4-7-20260416", "claude-opus-4-7"),
    ("opus-4.6", "claude-opus-4-6"),
    ("claude-opus-4.6", "claude-opus-4-6"),
    ("claude-opus-4-6-20251117", "claude-opus-4-6"),
    ("opus-4.5", "claude-opus-4-5"),
    ("claude-opus-4.5", "claude-opus-4-5"),
    ("claude-opus-4-5-20250120", "claude-opus-4-5"),
    ("sonnet", "claude-sonnet-5"),
    ("sonnet-5", "claude-sonnet-5"),
    ("claude-sonnet-5", "claude-sonnet-5"),
    ("sonnet-4.6", "claude-sonnet-4-6"),
    ("claude-sonnet-4.6", "claude-sonnet-4-6"),
    ("claude-sonnet-4-6-20251117", "claude-sonnet-4-6"),
    ("haiku", "claude-haiku-4-5"),
    ("haiku-4.5", "claude-haiku-4-5"),
    ("claude-haiku-4.5", "claude-haiku-4-5"),
    ("claude-haiku-4-5-20251001", "claude-haiku-4-5"),
];
const ALIASES_CURSOR: &[(&str, &str)] = &[
    ("auto", "auto"),
    ("default", "auto"),
    ("composer", "composer-2.5"),
    ("composer-2.5", "composer-2.5"),
    ("composer-2", "composer-2.5"),
    ("opus", "claude-opus-5"),
    ("opus-5", "claude-opus-5"),
    ("opus-4.8", "claude-opus-4-8"),
    ("opus-4.7", "claude-opus-4-7"),
    ("opus-4.6", "claude-opus-4-6"),
    ("opus-4.6-thinking", "claude-opus-4-6"),
    ("sonnet", "claude-sonnet-5"),
    ("sonnet-5", "claude-sonnet-5"),
    ("sonnet-4.6", "claude-sonnet-4-6"),
    ("fable", "claude-fable-5"),
    ("fable-5", "claude-fable-5"),
    ("sol", "gpt-5.6-sol"),
    ("5.6", "gpt-5.6-sol"),
    ("gpt-5.3", "gpt-5.3-codex"),
    ("codex-5.3", "gpt-5.3-codex"),
    ("grok", "grok-4.6"),
    ("grok-4.5", "grok-4.5"),
    ("grok-4.6", "grok-4.6"),
    ("cursor-grok-4.5", "grok-4.5"),
    ("cursor-grok-4.6", "grok-4.6"),
    ("gemini", "gemini-3.1-pro"),
    ("gemini-3", "gemini-3.1-pro"),
    ("gemini-3-pro", "gemini-3.1-pro"),
    ("gemini-3.1-pro-preview", "gemini-3.1-pro"),
    ("glm", "glm-5.2"),
    ("kimi", "kimi-k2.7-code"),
];
const ALIASES_DROID: &[(&str, &str)] = &[
    ("droid", "claude-opus-4-8"),
    ("factory", "claude-opus-4-8"),
    ("opus", "claude-opus-4-8"),
    ("opus-4.8", "claude-opus-4-8"),
    ("opus-fast", "claude-opus-4-8-fast"),
    ("opus-4.8-fast", "claude-opus-4-8-fast"),
    ("opus-4.7", "claude-opus-4-7"),
    ("opus-4.7-fast", "claude-opus-4-7-fast"),
    ("opus-4.6", "claude-opus-4-6"),
    ("sonnet", "claude-sonnet-5"),
    ("sonnet-5", "claude-sonnet-5"),
    ("sonnet-4.6", "claude-sonnet-4-6"),
    ("sonnet-4.5", "claude-sonnet-4-5-20250929"),
    ("fable", "claude-fable-5"),
    ("haiku", "claude-haiku-4-5-20251001"),
    ("5.5", "gpt-5.5"),
    ("5.5-fast", "gpt-5.5-fast"),
    ("5.5-pro", "gpt-5.5-pro"),
    ("5.4", "gpt-5.4"),
    ("5.4-fast", "gpt-5.4-fast"),
    ("5.4-mini", "gpt-5.4-mini"),
    ("5.3", "gpt-5.3-codex"),
    ("5.3-fast", "gpt-5.3-codex-fast"),
    ("gpt-5.3", "gpt-5.3-codex"),
    ("gemini-3-pro", "gemini-3.1-pro-preview"),
    ("gemini-3.1-pro", "gemini-3.1-pro-preview"),
    ("gemini-3.5-flash", "gemini-3.5-flash"),
    ("gemini-3-flash", "gemini-3-flash-preview"),
    ("glm", "glm-5.2"),
    ("glm-5.2", "glm-5.2"),
    ("glm-5.1", "glm-5.1"),
    ("nemotron", "nemotron-3-ultra"),
    ("kimi", "kimi-k2.7-code"),
    ("kimi-code", "kimi-k2.7-code"),
    ("deepseek", "deepseek-v4-pro"),
    ("minimax", "minimax-m3"),
];
const ALIASES_GROK: &[(&str, &str)] = &[
    ("grok", "grok-build-0.1"),
    ("build", "grok-build-0.1"),
    ("grok-build-0.1", "grok-build-0.1"),
    ("grok-build", "grok-build"),
    ("4.3", "grok-build"),
    ("grok-4", "grok-build"),
    ("grok-4.3", "grok-build"),
    ("grok-latest", "grok-build"),
    ("grok-code-fast", "grok-build-0.1"),
    ("grok-code-fast-1", "grok-build-0.1"),
    ("grok-code-fast-1-0825", "grok-build-0.1"),
    ("code-fast", "grok-build-0.1"),
    ("4.5", "grok-4.5"),
    ("grok-4.5", "grok-4.5"),
    ("4.6", "grok-4.6"),
    ("grok-4.6", "grok-4.6"),
];
const ALIASES_DEVIN: &[(&str, &str)] = &[
    ("adaptive", "adaptive"),
    ("auto", "adaptive"),
    ("fast", "swe-1-6"),
    ("swe-1.6-fast", "swe-1-6"),
    ("swe-1.6", "swe-1-6"),
    ("swe-1.7", "swe-1-7"),
    ("swe", "swe-1-6"),
    ("swe-1-6", "swe-1-6"),
    ("swe-1-7", "swe-1-7"),
    ("opus", "claude-opus-4-8"),
    ("sonnet", "claude-sonnet-5"),
    ("fable", "claude-fable-5"),
];

/// Reasoning-effort ladder values + the default entry, per model slug
/// (`MODEL_CAPABILITIES_INDEX`, collapsed to the fields the port reads).
type EffCaps = (&'static [&'static str], Option<&'static str>);
const CAPS_EMPTY: EffCaps = (&[], None);

fn caps_index(provider: &str, slug: &str) -> Option<EffCaps> {
    let table: &[(&str, EffCaps)] = match provider {
        "codex" => CAPS_CODEX,
        "claudeAgent" => CAPS_CLAUDE,
        "cursor" => CAPS_CURSOR,
        "droid" => CAPS_DROID,
        "grok" => CAPS_GROK,
        "opencode" => CAPS_OPENCODE,
        "devin" => CAPS_DEVIN,
        "antigravity" | "pi" | "omp" => &[],
        _ => return None,
    };
    table.iter().find(|(s, _)| *s == slug).map(|(_, c)| *c)
}

const CAPS_CODEX: &[(&str, EffCaps)] = &[
    (
        "gpt-6-astra",
        (
            &["low", "medium", "high", "xhigh", "max", "ultra"],
            Some("medium"),
        ),
    ),
    (
        "gpt-6-sol",
        (
            &["low", "medium", "high", "xhigh", "max", "ultra"],
            Some("medium"),
        ),
    ),
    (
        "gpt-6-luna",
        (&["low", "medium", "high", "xhigh", "max"], Some("medium")),
    ),
    (
        "gpt-5.5",
        (&["low", "medium", "high", "xhigh"], Some("medium")),
    ),
    (
        "gpt-5.4",
        (&["low", "medium", "high", "xhigh"], Some("high")),
    ),
    (
        "gpt-5.4-mini",
        (&["low", "medium", "high", "xhigh"], Some("high")),
    ),
    (
        "gpt-5.3-codex",
        (&["low", "medium", "high", "xhigh"], Some("high")),
    ),
    (
        "gpt-5.3-codex-spark",
        (&["low", "medium", "high", "xhigh"], Some("high")),
    ),
    (
        "gpt-5.2-codex",
        (&["low", "medium", "high", "xhigh"], Some("high")),
    ),
    (
        "gpt-5.2",
        (&["low", "medium", "high", "xhigh"], Some("high")),
    ),
];

const CAPS_CLAUDE: &[(&str, EffCaps)] = &[
    (
        "claude-fable-5-1",
        (
            &["low", "medium", "high", "xhigh", "max", "ultracode"],
            Some("high"),
        ),
    ),
    (
        "claude-fable-5",
        (
            &["low", "medium", "high", "xhigh", "max", "ultracode"],
            Some("high"),
        ),
    ),
    (
        "claude-opus-5-5",
        (
            &["low", "medium", "high", "xhigh", "max", "ultracode"],
            Some("high"),
        ),
    ),
    (
        "claude-opus-5",
        (
            &["low", "medium", "high", "xhigh", "max", "ultracode"],
            Some("high"),
        ),
    ),
    (
        "claude-opus-4-8",
        (
            &[
                "low",
                "medium",
                "high",
                "xhigh",
                "max",
                "ultrathink",
                "ultracode",
            ],
            Some("high"),
        ),
    ),
    (
        "claude-opus-4-7",
        (
            &[
                "low",
                "medium",
                "high",
                "xhigh",
                "max",
                "ultrathink",
                "ultracode",
            ],
            Some("high"),
        ),
    ),
    (
        "claude-opus-4-6",
        (
            &["low", "medium", "high", "max", "ultrathink"],
            Some("high"),
        ),
    ),
    (
        "claude-opus-4-5",
        (&["low", "medium", "high"], Some("high")),
    ),
    (
        "claude-sonnet-5",
        (
            &["low", "medium", "high", "xhigh", "max", "ultracode"],
            Some("high"),
        ),
    ),
    (
        "claude-sonnet-4-6",
        (
            &["low", "medium", "high", "max", "ultrathink"],
            Some("high"),
        ),
    ),
    ("claude-haiku-4-5", CAPS_EMPTY),
];

const CAPS_GROK: &[(&str, EffCaps)] = &[
    (
        "grok-4.6",
        (&["low", "medium", "high", "xhigh"], Some("high")),
    ),
    (
        "grok-build-0.1",
        (&["none", "low", "medium", "high"], Some("low")),
    ),
    (
        "grok-build",
        (&["none", "low", "medium", "high"], Some("low")),
    ),
    ("grok-4.5", (&["low", "medium", "high"], Some("high"))),
];

const CAPS_DROID: &[(&str, EffCaps)] = &[
    ("auto", CAPS_EMPTY),
    (
        "claude-fable-5",
        (
            &["off", "low", "medium", "high", "xhigh", "max"],
            Some("high"),
        ),
    ),
    (
        "claude-opus-4-8",
        (
            &["off", "low", "medium", "high", "xhigh", "max"],
            Some("high"),
        ),
    ),
    (
        "claude-opus-4-8-fast",
        (
            &["off", "low", "medium", "high", "xhigh", "max"],
            Some("high"),
        ),
    ),
    (
        "claude-opus-4-7",
        (&["off", "low", "medium", "high", "max"], Some("high")),
    ),
    (
        "claude-opus-4-7-fast",
        (&["off", "low", "medium", "high", "max"], Some("high")),
    ),
    (
        "claude-opus-4-6",
        (&["off", "low", "medium", "high", "max"], Some("high")),
    ),
    (
        "claude-sonnet-5",
        (
            &["off", "low", "medium", "high", "xhigh", "max"],
            Some("high"),
        ),
    ),
    (
        "claude-sonnet-4-6",
        (&["off", "low", "medium", "high", "max"], Some("high")),
    ),
    (
        "claude-opus-4-5-20251101",
        (&["off", "low", "medium", "high"], Some("off")),
    ),
    (
        "claude-sonnet-4-5-20250929",
        (&["off", "low", "medium", "high"], Some("off")),
    ),
    (
        "claude-haiku-4-5-20251001",
        (&["off", "low", "medium", "high"], Some("off")),
    ),
    (
        "gpt-5.6-sol",
        (
            &["none", "low", "medium", "high", "xhigh", "max"],
            Some("medium"),
        ),
    ),
    (
        "gpt-5.6-terra",
        (
            &["none", "low", "medium", "high", "xhigh", "max"],
            Some("medium"),
        ),
    ),
    (
        "gpt-5.6-luna",
        (
            &["none", "low", "medium", "high", "xhigh", "max"],
            Some("medium"),
        ),
    ),
    (
        "gpt-5.5",
        (&["low", "medium", "high", "xhigh"], Some("medium")),
    ),
    (
        "gpt-5.5-fast",
        (&["low", "medium", "high", "xhigh"], Some("medium")),
    ),
    (
        "gpt-5.5-pro",
        (&["medium", "high", "xhigh"], Some("medium")),
    ),
    (
        "gpt-5.4",
        (&["low", "medium", "high", "xhigh"], Some("medium")),
    ),
    (
        "gpt-5.4-fast",
        (&["low", "medium", "high", "xhigh"], Some("medium")),
    ),
    (
        "gpt-5.4-mini",
        (&["none", "low", "medium", "high", "xhigh"], Some("high")),
    ),
    (
        "gpt-5.3-codex",
        (&["low", "medium", "high", "xhigh"], Some("medium")),
    ),
    (
        "gpt-5.3-codex-fast",
        (&["low", "medium", "high", "xhigh"], Some("medium")),
    ),
    (
        "gpt-5.2",
        (&["off", "low", "medium", "high", "xhigh"], Some("off")),
    ),
    (
        "gemini-3.1-pro-preview",
        (&["low", "medium", "high"], Some("high")),
    ),
    (
        "gemini-3.5-flash",
        (&["minimal", "low", "medium", "high"], Some("high")),
    ),
    (
        "gemini-3-flash-preview",
        (&["minimal", "low", "medium", "high"], Some("high")),
    ),
    ("glm-5.2", (&["off", "high"], Some("high"))),
    ("glm-5.2-fast", (&["off", "high"], Some("high"))),
    ("glm-5.1", (&["off", "high"], Some("high"))),
    ("nemotron-3-ultra", (&["off", "high"], Some("high"))),
    ("kimi-k2.7-code", (&["off", "high"], Some("high"))),
    ("kimi-k2.6", (&["off", "high"], Some("high"))),
    (
        "deepseek-v4-pro",
        (&["off", "low", "high", "max"], Some("high")),
    ),
    (
        "deepseek-v4-flash-0731",
        (&["off", "low", "high", "max"], Some("high")),
    ),
    ("minimax-m3", (&["high"], Some("high"))),
    ("minimax-m2.7", (&["high"], Some("high"))),
];

const CAPS_CURSOR: &[(&str, EffCaps)] = &[
    ("auto", CAPS_EMPTY),
    ("composer-2.5", CAPS_EMPTY),
    (
        "claude-opus-5",
        (&["low", "medium", "high", "xhigh", "max"], Some("high")),
    ),
    (
        "claude-opus-4-8",
        (&["low", "medium", "high", "xhigh", "max"], Some("high")),
    ),
    (
        "claude-opus-4-7",
        (&["low", "medium", "high", "xhigh", "max"], Some("high")),
    ),
    ("claude-opus-4-6", (&["high", "max"], Some("high"))),
    ("claude-opus-4-5", (&["high"], Some("high"))),
    (
        "claude-fable-5",
        (&["low", "medium", "high", "xhigh", "max"], Some("high")),
    ),
    (
        "claude-sonnet-5",
        (&["low", "medium", "high", "xhigh", "max"], Some("high")),
    ),
    ("claude-sonnet-4-6", (&["medium"], Some("medium"))),
    ("claude-sonnet-4-5", CAPS_EMPTY),
    ("claude-sonnet-4", CAPS_EMPTY),
    ("claude-haiku-4-5", CAPS_EMPTY),
    (
        "gpt-5.6-sol",
        (
            &["none", "low", "medium", "high", "xhigh", "max"],
            Some("medium"),
        ),
    ),
    (
        "gpt-5.6-terra",
        (
            &["none", "low", "medium", "high", "xhigh", "max"],
            Some("medium"),
        ),
    ),
    (
        "gpt-5.6-luna",
        (
            &["none", "low", "medium", "high", "xhigh", "max"],
            Some("medium"),
        ),
    ),
    (
        "gpt-5.5",
        (&["none", "low", "medium", "high", "xhigh"], Some("medium")),
    ),
    (
        "gpt-5.4",
        (&["none", "low", "medium", "high", "xhigh"], Some("medium")),
    ),
    (
        "gpt-5.4-mini",
        (&["none", "low", "medium", "high", "xhigh"], Some("medium")),
    ),
    (
        "gpt-5.4-nano",
        (&["none", "low", "medium", "high", "xhigh"], Some("medium")),
    ),
    (
        "gpt-5.3-codex",
        (&["low", "medium", "high", "xhigh"], Some("high")),
    ),
    (
        "gpt-5.2",
        (&["low", "medium", "high", "xhigh"], Some("high")),
    ),
    ("gpt-5.1", (&["low", "medium", "high"], Some("high"))),
    ("gpt-5-mini", CAPS_EMPTY),
    ("grok-4.5", (&["low", "medium", "high"], Some("high"))),
    (
        "grok-4.6",
        (&["low", "medium", "high", "xhigh"], Some("high")),
    ),
    ("gemini-3.1-pro", CAPS_EMPTY),
    (
        "gemini-3.6-flash",
        (&["minimal", "low", "medium", "high"], Some("high")),
    ),
    ("gemini-3.5-flash", CAPS_EMPTY),
    ("gemini-3-flash", CAPS_EMPTY),
    ("gemini-2.5-flash", CAPS_EMPTY),
    ("kimi-k2.7-code", CAPS_EMPTY),
    ("glm-5.2", (&["high", "max"], Some("high"))),
];

const CAPS_OPENCODE: &[(&str, EffCaps)] = &[("openai/gpt-5", CAPS_EMPTY)];
const CAPS_DEVIN: &[(&str, EffCaps)] = &[
    ("adaptive", CAPS_EMPTY),
    ("swe-1-6", CAPS_EMPTY),
    ("swe-1-7", CAPS_EMPTY),
];

const GROK_EFFORT_SET: &[&str] = &["none", "low", "medium", "high", "xhigh"];
const ANTIGRAVITY_EFFORT_SET: &[&str] = &["low", "medium", "high", "thinking"];
const CLAUDE_EFFORT_SET: &[&str] = &[
    "low",
    "medium",
    "high",
    "xhigh",
    "max",
    "ultrathink",
    "ultracode",
];
const PI_THINKING_SET: &[&str] = &["off", "minimal", "low", "medium", "high", "xhigh", "max"];
const OMP_THINKING_SET: &[&str] = &[
    "off", "auto", "minimal", "low", "medium", "high", "xhigh", "max",
];

const MODEL_TOKEN_DISPLAY_NAMES: &[(&str, &str)] = &[
    ("deepseek", "DeepSeek"),
    ("glm", "GLM"),
    ("gpt", "GPT"),
    ("minimax", "MiniMax"),
    ("openai", "OpenAI"),
    ("opencode", "OpenCode"),
    ("swe", "SWE"),
    ("xai", "xAI"),
    ("xhigh", "XHigh"),
];

const MAX_CUSTOM_MODEL_COUNT: usize = 32;
const MAX_CUSTOM_MODEL_LENGTH: usize = 256;

// ── JS value semantics ───────────────────────────────────────────────────────

fn is_undef(v: &Value) -> bool {
    js::is_undefined(v)
}

fn is_nullish(v: &Value) -> bool {
    v.is_null() || is_undef(v)
}

/// UTF-16 units of a JS string: `Value::String` or a single-key `{"$utf16": ...}`.
fn str_units(v: &Value) -> Option<Vec<u16>> {
    match v {
        Value::String(s) => Some(s.encode_utf16().collect()),
        Value::Object(o) if o.len() == 1 => o.get("$utf16").map(|x| match x {
            Value::Array(a) => a
                .iter()
                .map(|u| u.as_f64().map(|n| n as i64 as u16).unwrap_or(0))
                .collect(),
            Value::String(s) => s.encode_utf16().collect(),
            _ => Vec::new(),
        }),
        _ => None,
    }
}

fn units_lossy(u: &[u16]) -> String {
    String::from_utf16_lossy(u)
}

fn lower_string(u: &[u16]) -> String {
    units_lossy(u).to_lowercase()
}

/// `Number(x)` for a marker payload.
fn js_to_number(v: &Value) -> f64 {
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => str_to_number(s),
        Value::Array(a) => str_to_number(
            &a.iter()
                .map(|x| {
                    if is_nullish(x) {
                        String::new()
                    } else {
                        js_to_string(x)
                    }
                })
                .collect::<Vec<_>>()
                .join(","),
        ),
        _ => f64::NAN,
    }
}

fn js_to_string(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => {
            if *b {
                "true".into()
            } else {
                "false".into()
            }
        }
        Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(a) => a
            .iter()
            .map(|x| {
                if is_nullish(x) {
                    String::new()
                } else {
                    js_to_string(x)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        _ => "[object Object]".into(),
    }
}

fn str_to_number(s: &str) -> f64 {
    let t = s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}');
    if t.is_empty() {
        return 0.0;
    }
    match t {
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    if let Some(h) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        return u64::from_str_radix(h, 16)
            .map(|x| x as f64)
            .unwrap_or(f64::NAN);
    }
    if let Some(o) = t.strip_prefix("0o").or_else(|| t.strip_prefix("0O")) {
        return u64::from_str_radix(o, 8)
            .map(|x| x as f64)
            .unwrap_or(f64::NAN);
    }
    if let Some(b) = t.strip_prefix("0b").or_else(|| t.strip_prefix("0B")) {
        return u64::from_str_radix(b, 2)
            .map(|x| x as f64)
            .unwrap_or(f64::NAN);
    }
    t.parse::<f64>().unwrap_or(f64::NAN)
}

/// A JS number: `Value::Number` or a single-key `{"$num": ...}` marker.
fn js_num(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::Object(o) if o.len() == 1 => o.get("$num").map(js_to_number),
        _ => None,
    }
}

/// BigInt digits for a single-key `{"$bigint": "..."}` marker.
fn bigint_norm(v: &Value) -> Option<String> {
    let Value::Object(o) = v else { return None };
    if o.len() != 1 {
        return None;
    }
    let s = o.get("$bigint")?;
    let digits = match s {
        Value::String(s) => s.clone(),
        Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or_default()),
        _ => return None,
    };
    let (neg, d) = match digits.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, digits.as_str()),
    };
    let d = d.trim_start_matches('0');
    if d.is_empty() {
        return Some("0".into());
    }
    Some(if neg { format!("-{d}") } else { d.to_string() })
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) => true,
        Value::Object(o) => {
            if is_undef(v) {
                return false;
            }
            if o.len() == 1 {
                if let Some(x) = o.get("$num") {
                    let n = js_to_number(x);
                    return n != 0.0 && !n.is_nan();
                }
                if o.contains_key("$utf16") {
                    return !str_units(v).unwrap_or_default().is_empty();
                }
                if let Some(x) = o.get("$bigint") {
                    return bigint_norm(&Value::Object(
                        [(String::from("$bigint"), x.clone())].into_iter().collect(),
                    ))
                    .is_some_and(|d| d != "0");
                }
            }
            true
        }
    }
}

/// JS `===` for JSON-transport values. Objects/arrays compare by identity,
/// which can never hold between two decoded values.
fn strict_eq(a: &Value, b: &Value) -> bool {
    let ua = is_undef(a);
    let ub = is_undef(b);
    if ua || ub {
        return ua && ub;
    }
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Null, _) | (_, Value::Null) | (Value::Bool(_), _) | (_, Value::Bool(_)) => false,
        _ => {
            if let Some(x) = js_num(a) {
                return js_num(b).is_some_and(|y| x == y);
            }
            if js_num(b).is_some() {
                return false;
            }
            if let Some(x) = str_units(a) {
                return str_units(b).is_some_and(|y| x == y);
            }
            if str_units(b).is_some() {
                return false;
            }
            if let Some(x) = bigint_norm(a) {
                return bigint_norm(b).is_some_and(|y| x == y);
            }
            false
        }
    }
}

/// `v === s` for a string literal.
fn str_eq(v: &Value, s: &str) -> bool {
    str_units(v).is_some_and(|u| u == s.encode_utf16().collect::<Vec<_>>())
}

/// `typeof v === "object"` (the truthy gate is applied by callers).
fn is_object_type(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::Array(_) => true,
        Value::Object(o) => {
            if is_undef(v) {
                return false;
            }
            !(o.len() == 1
                && (o.contains_key("$num")
                    || o.contains_key("$utf16")
                    || o.contains_key("$bigint")))
        }
        _ => false,
    }
}

// ── Property access and object enumeration ───────────────────────────────────

fn array_index_key(k: &str) -> Option<usize> {
    if k.is_empty() || k.len() > 10 {
        return None;
    }
    if !k.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if k.len() > 1 && k.starts_with('0') {
        return None;
    }
    let n: u64 = k.parse().ok()?;
    (n < 4294967295).then_some(n as usize)
}

/// `v[key]`; missing properties read as the `{"$undefined": true}` marker.
fn js_get(v: &Value, key: &str) -> Value {
    match v {
        Value::Null | Value::Bool(_) | Value::Number(_) => js::undefined(),
        Value::String(s) => string_get(&s.encode_utf16().collect::<Vec<_>>(), key),
        Value::Array(a) => {
            if key == "length" {
                return js::number(a.len() as f64);
            }
            if let Some(i) = array_index_key(key) {
                return a.get(i).cloned().unwrap_or_else(js::undefined);
            }
            js::undefined()
        }
        Value::Object(o) => {
            if is_undef(v) {
                return js::undefined();
            }
            if o.len() == 1 {
                if o.contains_key("$num")
                    || o.contains_key("$bigint")
                    || o.contains_key("$map")
                    || o.contains_key("$set")
                    || o.contains_key("$date")
                {
                    return js::undefined();
                }
                if o.contains_key("$utf16") {
                    return string_get(&str_units(v).unwrap_or_default(), key);
                }
                if let Some(b) = o.get("$bytes").or_else(|| o.get("$buffer")) {
                    return bytes_get(b, key);
                }
            }
            o.get(key).cloned().unwrap_or_else(js::undefined)
        }
    }
}

fn string_get(units: &[u16], key: &str) -> Value {
    if key == "length" {
        return js::number(units.len() as f64);
    }
    if let Some(i) = array_index_key(key) {
        return units
            .get(i)
            .map(|u| js::utf16_string(&[*u]))
            .unwrap_or_else(js::undefined);
    }
    js::undefined()
}

fn bytes_get(b: &Value, key: &str) -> Value {
    let items: Vec<Value> = b.as_array().cloned().unwrap_or_default();
    if key == "length" {
        return js::number(items.len() as f64);
    }
    if let Some(i) = array_index_key(key) {
        return items.get(i).cloned().unwrap_or_else(js::undefined);
    }
    js::undefined()
}

/// `String(v)` restricted to property-key usage.
fn prop_key(v: &Value) -> Result<String, String> {
    Ok(match v {
        Value::Null => "null".into(),
        Value::Bool(b) => {
            if *b {
                "true".into()
            } else {
                "false".into()
            }
        }
        Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(a) => a
            .iter()
            .map(|x| {
                if is_nullish(x) {
                    Ok(String::new())
                } else {
                    prop_key(x)
                }
            })
            .collect::<Result<Vec<_>, _>>()?
            .join(","),
        Value::Object(o) => {
            if is_undef(v) {
                "undefined".into()
            } else if o.len() == 1 {
                if let Some(x) = o.get("$num") {
                    js::number_to_string(js_to_number(x))
                } else if o.contains_key("$utf16") {
                    units_lossy(&str_units(v).unwrap_or_default())
                } else if let Some(x) = o.get("$bigint") {
                    bigint_norm(v).unwrap_or_else(|| js_to_string(x))
                } else if o.contains_key("$map") {
                    "[object Map]".into()
                } else if o.contains_key("$set") {
                    "[object Set]".into()
                } else if o.contains_key("$date") {
                    "[object Date]".into()
                } else if let Some(b) = o.get("$bytes").or_else(|| o.get("$buffer")) {
                    b.as_array()
                        .map(|a| {
                            a.iter()
                                .map(|x| {
                                    x.as_f64()
                                        .map(js::number_to_string)
                                        .unwrap_or_else(|| "NaN".into())
                                })
                                .collect::<Vec<_>>()
                                .join(",")
                        })
                        .unwrap_or_default()
                } else if o.contains_key("$symbol") {
                    return Err("TypeError: Cannot convert a Symbol value to a string".into());
                } else {
                    "[object Object]".into()
                }
            } else {
                "[object Object]".into()
            }
        }
    })
}

/// Own enumerable string-keyed property names, JS order: integer indices
/// ascending, then the rest in insertion order.
fn own_keys(o: &Map<String, Value>) -> Vec<String> {
    let mut ints: Vec<(usize, String)> = Vec::new();
    let mut rest: Vec<String> = Vec::new();
    for k in o.keys() {
        if let Some(n) = array_index_key(k) {
            ints.push((n, k.clone()));
        } else {
            rest.push(k.clone());
        }
    }
    ints.sort_by_key(|(n, _)| *n);
    ints.into_iter().map(|(_, k)| k).chain(rest).collect()
}

fn finalize_map(m: Map<String, Value>) -> Map<String, Value> {
    let mut out = Map::new();
    for k in own_keys(&m) {
        if let Some(v) = m.get(&k) {
            out.insert(k, v.clone());
        }
    }
    out
}

fn index_props(units: &[u16]) -> Vec<(String, Value)> {
    units
        .iter()
        .enumerate()
        .map(|(i, u)| (i.to_string(), js::utf16_string(&[*u])))
        .collect()
}

/// `{...v}` — own enumerable string-keyed properties.
fn spread_props(v: &Value) -> Vec<(String, Value)> {
    match v {
        Value::Null | Value::Bool(_) | Value::Number(_) => Vec::new(),
        Value::String(s) => index_props(&s.encode_utf16().collect::<Vec<_>>()),
        Value::Array(a) => a
            .iter()
            .enumerate()
            .map(|(i, x)| (i.to_string(), x.clone()))
            .collect(),
        Value::Object(o) => {
            if is_undef(v) {
                return Vec::new();
            }
            if o.len() == 1 {
                if o.contains_key("$num")
                    || o.contains_key("$bigint")
                    || o.contains_key("$map")
                    || o.contains_key("$set")
                    || o.contains_key("$date")
                {
                    return Vec::new();
                }
                if o.contains_key("$utf16") {
                    return index_props(&str_units(v).unwrap_or_default());
                }
                if let Some(b) = o.get("$bytes").or_else(|| o.get("$buffer")) {
                    return b
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .enumerate()
                                .map(|(i, x)| (i.to_string(), x.clone()))
                                .collect()
                        })
                        .unwrap_or_default();
                }
            }
            own_keys(o)
                .into_iter()
                .map(|k| (k.clone(), o.get(&k).cloned().unwrap_or(Value::Null)))
                .collect()
        }
    }
}

fn spread_to_map(v: &Value) -> Map<String, Value> {
    spread_props(v).into_iter().collect()
}

fn object_values(v: &Value) -> Result<Vec<Value>, String> {
    if is_nullish(v) {
        return Err(
            "TypeError: Object.values requires that input parameter not be null or undefined"
                .into(),
        );
    }
    Ok(spread_props(v).into_iter().map(|(_, x)| x).collect())
}

fn object_entries(v: &Value) -> Result<Vec<(String, Value)>, String> {
    if is_nullish(v) {
        return Err(
            "TypeError: Object.entries requires that input parameter not be null or undefined"
                .into(),
        );
    }
    Ok(spread_props(v))
}

fn object_keys(v: &Value) -> Result<Vec<String>, String> {
    if is_nullish(v) {
        return Err("TypeError: null is not an object (evaluating 'Object.keys(x)')".into());
    }
    Ok(spread_props(v).into_iter().map(|(k, _)| k).collect())
}

// ── ES string helpers on UTF-16 units ────────────────────────────────────────

fn is_es_ws(u: u16) -> bool {
    matches!(
        u,
        0x0009 | 0x000A | 0x000B | 0x000C | 0x000D | 0x0020 | 0x00A0 | 0x1680 | 0x2000
            ..=0x200A | 0x2028 | 0x2029 | 0x202F | 0x205F | 0x3000 | 0xFEFF
    )
}

fn trim_units(u: &[u16]) -> &[u16] {
    let mut s = 0;
    let mut e = u.len();
    while s < e && is_es_ws(u[s]) {
        s += 1;
    }
    while e > s && is_es_ws(u[e - 1]) {
        e -= 1;
    }
    &u[s..e]
}

fn trim_string_or_undefined(v: &Value) -> Option<Vec<u16>> {
    let units = str_units(v)?;
    let t = trim_units(&units);
    (!t.is_empty()).then(|| t.to_vec())
}

fn ends_with_units(u: &[u16], suffix: &str) -> bool {
    let s: Vec<u16> = suffix.encode_utf16().collect();
    u.len() >= s.len() && u[u.len() - s.len()..] == s[..]
}

/// `model.replace(/\[([^\]]+)\]$/, "")` — strips a trailing non-empty `[...]`.
fn strip_bracket_suffix(u: &[u16]) -> Vec<u16> {
    if u.len() < 3 || u[u.len() - 1] != b']' as u16 {
        return u.to_vec();
    }
    for p in 0..u.len() - 2 {
        if u[p] == b'[' as u16 && !u[p + 1..u.len() - 1].contains(&(b']' as u16)) {
            return u[..p].to_vec();
        }
    }
    u.to_vec()
}

/// `model.replace(/\[[^\]]*\]$/, "")` — like above but the inner may be empty.
fn strip_bracket_suffix_star(u: &[u16]) -> Vec<u16> {
    if u.len() < 2 || u[u.len() - 1] != b']' as u16 {
        return u.to_vec();
    }
    for p in 0..u.len() - 1 {
        if u[p] == b'[' as u16 && !u[p + 1..u.len() - 1].contains(&(b']' as u16)) {
            return u[..p].to_vec();
        }
    }
    u.to_vec()
}

/// `getClaudeContextWindowSuffix` — the inner of a trailing `[...]` group.
fn claude_ctx_suffix(u: &[u16]) -> Option<Vec<u16>> {
    if u.len() < 3 || u[u.len() - 1] != b']' as u16 {
        return None;
    }
    for p in 0..u.len() - 2 {
        if u[p] == b'[' as u16 && !u[p + 1..u.len() - 1].contains(&(b']' as u16)) {
            return Some(u[p + 1..u.len() - 1].to_vec());
        }
    }
    None
}

fn is_provider_slug(u: &[u16]) -> bool {
    !u.is_empty()
        && u.len() <= 64
        && matches!(u[0], 65..=90 | 97..=122)
        && u[1..]
            .iter()
            .all(|&c| matches!(c, 65..=90 | 97..=122 | 48..=57 | 95 | 45))
}

fn normalize_instance_id(v: &Value) -> Option<String> {
    let t = trim_string_or_undefined(v)?;
    is_provider_slug(&t).then(|| units_lossy(&t))
}

fn js_repr(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => {
            if *b {
                "true".into()
            } else {
                "false".into()
            }
        }
        Value::Number(n) => format!("{} ", js::number_to_string(n.as_f64().unwrap_or(0.0))),
        Value::String(s) => format!("{s:?}"),
        Value::Array(_) => "([])".into(),
        Value::Object(o) => {
            if is_undef(v) {
                return "void 0".into();
            }
            if o.is_empty() {
                return "({})".into();
            }
            let inner = o
                .iter()
                .map(|(k, x)| format!("{k}: {}", js_repr_short(x)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("({{ {inner} }})")
        }
    }
}

fn js_repr_short(v: &Value) -> String {
    match v {
        Value::Object(o) if !is_undef(v) => {
            if o.is_empty() {
                return "{}".into();
            }
            let inner = o
                .iter()
                .map(|(k, x)| format!("{k}: {}", js_repr_short(x)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{ {inner} }}")
        }
        Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(0.0)),
        _ => js_repr(v),
    }
}

fn nullish_word(v: &Value) -> &'static str {
    if v.is_null() { "null" } else { "undefined" }
}

// ── Model helpers (ports of packages/shared/src/model.ts internals) ──────────

fn default_model_for(provider: &Value) -> Value {
    if str_eq(provider, "pi") || str_eq(provider, "omp") {
        return Value::Null;
    }
    match prop_key(provider)
        .ok()
        .and_then(|k| DEFAULT_MODEL_BY_PROVIDER.iter().find(|(p, _)| *p == k))
    {
        Some((_, m)) => Value::String((*m).to_string()),
        None => js::undefined(),
    }
}

fn model_options(provider: &str) -> Option<&'static [(&'static str, &'static str)]> {
    MODEL_OPTIONS
        .iter()
        .find(|(p, _)| *p == provider)
        .map(|(_, o)| *o)
}

fn model_name_by_slug(lower_key: &str) -> Option<&'static str> {
    // Last match wins (Object.values(...).flat() into a Map).
    let mut found = None;
    for (_, options) in MODEL_OPTIONS {
        for (slug, name) in *options {
            if slug.to_lowercase() == lower_key {
                found = Some(*name);
            }
        }
    }
    found
}

fn normalize_model_slug(
    model: Option<Vec<u16>>,
    provider: &str,
) -> Result<Option<Vec<u16>>, String> {
    let Some(model) = model else {
        return Ok(None);
    };
    let trimmed = trim_units(&model).to_vec();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let provider_scoped = if provider == "claudeAgent" {
        strip_bracket_suffix(&trimmed)
    } else if provider == "devin"
        && lower_string(&trimmed) == units_lossy(&trimmed)
        && ends_with_units(&trimmed, "-medium")
    {
        trimmed[..trimmed.len() - "-medium".len()].to_vec()
    } else {
        trimmed.clone()
    };
    let alias_key = lower_string(&provider_scoped);
    let Some(aliases) = model_aliases(provider) else {
        return Err(format!(
            "TypeError: undefined is not an object (evaluating 'Object.prototype.hasOwnProperty.call(void 0, {})')",
            serde_json::to_string(&alias_key).unwrap_or_else(|_| "\"?\"".into())
        ));
    };
    let mut normalized = aliases
        .iter()
        .find(|(k, _)| *k == alias_key)
        .map(|(_, s)| s.encode_utf16().collect::<Vec<u16>>())
        .unwrap_or(provider_scoped);
    if provider == "claudeAgent"
        && claude_ctx_suffix(&trimmed)
            .map(|s| lower_string(&s))
            .as_deref()
            == Some("1m")
    {
        normalized.extend("[1m]".encode_utf16());
    }
    Ok(Some(normalized))
}

fn resolve_model_slug(model: Option<Vec<u16>>, provider: &str) -> Result<Option<Vec<u16>>, String> {
    let normalized_model = normalize_model_slug(model, provider)?;
    let normalized = if provider == "claudeAgent" {
        normalized_model.map(|m| strip_bracket_suffix(&m))
    } else {
        normalized_model
    };
    if provider == "devin" || provider == "pi" || provider == "omp" {
        return Ok(normalized);
    }
    let Some(n) = normalized else {
        return Ok(default_model_units(provider));
    };
    let Some(set) = model_slug_set(provider) else {
        return Err("TypeError: undefined is not an object (evaluating '(void 0).has')".into());
    };
    let ns = units_lossy(&n);
    if set.contains(&ns.as_str()) {
        Ok(Some(n))
    } else {
        Ok(default_model_units(provider))
    }
}

fn default_model_units(provider: &str) -> Option<Vec<u16>> {
    DEFAULT_MODEL_BY_PROVIDER
        .iter()
        .find(|(p, _)| *p == provider)
        .map(|(_, m)| m.encode_utf16().collect())
}

fn resolve_grok_effort_family(slug: &[u16]) -> &'static str {
    let s = lower_string(slug);
    let (major, minor) = grok_version(&s).unwrap_or((-1, -1));
    let versioned = major > 4 || (major == 4 && minor >= 5);
    if !versioned
        && (s.contains("build")
            || s.contains("code-fast")
            || s == "grok-4"
            || s == "grok-4.3"
            || s.starts_with("grok-4.3-"))
    {
        return "build";
    }
    if major < 0 {
        return "build";
    }
    if major < 4 || (major == 4 && minor <= 3) {
        return "build";
    }
    if major == 4 && minor == 5 {
        return "4.5";
    }
    "4.6"
}

/// First `/grok-(\d+)\.(\d+)/` occurrence.
fn grok_version(s: &str) -> Option<(i64, i64)> {
    let b = s.as_bytes();
    let mut i = 0;
    while i + 5 <= b.len() {
        if &b[i..i + 5] == b"grok-" {
            let mut j = i + 5;
            let a0 = j;
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            if j > a0 && j < b.len() && b[j] == b'.' {
                let m0 = j + 1;
                let mut k = m0;
                while k < b.len() && b[k].is_ascii_digit() {
                    k += 1;
                }
                if k > m0 {
                    let major: i64 = s[a0..j].parse().ok()?;
                    let minor: i64 = s[m0..k].parse().ok()?;
                    return Some((major, minor));
                }
            }
        }
        i += 1;
    }
    None
}

fn grok_caps_for_family(family: &str) -> EffCaps {
    match family {
        "build" => caps_index("grok", "grok-build").unwrap_or(CAPS_EMPTY),
        "4.5" => caps_index("grok", "grok-4.5")
            .or_else(|| caps_index("grok", "grok-4.6"))
            .unwrap_or(CAPS_EMPTY),
        _ => caps_index("grok", "grok-4.6").unwrap_or(CAPS_EMPTY),
    }
}

fn parse_claude_family_version(slug: &[u16]) -> Option<(&'static str, i64, i64)> {
    let s = lower_string(trim_units(slug));
    let rest = s.strip_prefix("claude-")?;
    let (family, rest) = if let Some(r) = rest.strip_prefix("fable-") {
        ("fable", r)
    } else if let Some(r) = rest.strip_prefix("opus-") {
        ("opus", r)
    } else if let Some(r) = rest.strip_prefix("sonnet-") {
        ("sonnet", r)
    } else {
        ("haiku", rest.strip_prefix("haiku-")?)
    };
    let b = rest.as_bytes();
    let mut i = 0;
    while i < b.len() && b[i].is_ascii_digit() && i < 2 {
        i += 1;
    }
    if i == 0 {
        return None;
    }
    let major: i64 = rest[..i].parse().ok()?;
    let mut minor = 0i64;
    let mut rest = &rest[i..];
    if let Some(r) = rest.strip_prefix('-') {
        let digits: String = r
            .chars()
            .take(2)
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if !digits.is_empty() {
            minor = digits.parse().ok()?;
            rest = &r[digits.len()..];
        }
    }
    if let Some(r) = rest.strip_prefix('-')
        && r.len() == 8
        && r.bytes().all(|c| c.is_ascii_digit())
    {
        rest = "";
    }
    rest.is_empty().then_some((family, major, minor))
}

/// Newest catalog model per Claude family, derived from the claudeAgent catalog.
fn newest_claude_catalog(family: &str) -> (&'static str, i64, i64) {
    match family {
        "fable" => ("claude-fable-5-1", 5, 1),
        "opus" => ("claude-opus-5-5", 5, 5),
        "sonnet" => ("claude-sonnet-5", 5, 0),
        _ => ("claude-haiku-4-5", 4, 5),
    }
}

fn resolve_newest_known_claude_family_model(slug: &[u16]) -> Result<Option<&'static str>, String> {
    let Some(normalized) = normalize_model_slug(Some(slug.to_vec()), "claudeAgent")? else {
        return Ok(None);
    };
    let s = strip_bracket_suffix(&normalized);
    if CLAUDE_SLUGS.contains(&units_lossy(&s).as_str()) {
        return Ok(None);
    }
    let Some((family, major, minor)) = parse_claude_family_version(&s) else {
        return Ok(None);
    };
    let (nslug, nmaj, nmin) = newest_claude_catalog(family);
    Ok(if (major, minor) > (nmaj, nmin) {
        Some(nslug)
    } else {
        None
    })
}

fn get_model_capabilities(provider: &str, model: &Value) -> Result<EffCaps, String> {
    let normalized_slug = normalize_model_slug(str_units(model), provider)?;
    let slug = if provider == "claudeAgent" {
        normalized_slug.map(|s| strip_bracket_suffix(&s))
    } else {
        normalized_slug
    };
    if let Some(slug) = &slug
        && let Some(c) = caps_index(provider, &units_lossy(slug))
    {
        return Ok(c);
    }
    match provider {
        "grok" => {
            if let Some(slug) = &slug {
                return Ok(grok_caps_for_family(resolve_grok_effort_family(slug)));
            }
        }
        "claudeAgent" => {
            if let Some(slug) = &slug
                && let Some(newest) = resolve_newest_known_claude_family_model(slug)?
            {
                return Ok(caps_index("claudeAgent", newest).unwrap_or(CAPS_EMPTY));
            }
        }
        _ => {}
    }
    Ok(CAPS_EMPTY)
}

fn units_eq_str(u: &[u16], s: &str) -> bool {
    u == s.encode_utf16().collect::<Vec<_>>().as_slice()
}

fn has_effort(caps: EffCaps, effort: &[u16]) -> bool {
    caps.0.iter().any(|l| units_eq_str(effort, l))
}

fn normalize_grok_model_options(model: &Value, opts: &Value) -> Result<Value, String> {
    let caps = get_model_capabilities("grok", model)?;
    let effort = if is_nullish(opts) {
        None
    } else {
        trim_string_or_undefined(&js_get(opts, "reasoningEffort"))
    };
    let Some(effort) = effort else {
        return Ok(js::undefined());
    };
    if !has_effort(caps, &effort) {
        return Ok(js::undefined());
    }
    if caps.1.is_some_and(|d| units_eq_str(&effort, d)) {
        return Ok(js::undefined());
    }
    let mut o = Map::new();
    o.insert("reasoningEffort".into(), js::utf16_string(&effort));
    Ok(Value::Object(o))
}

fn classify_provider_effort(
    provider: &Value,
    model: &Value,
    effort: &Value,
) -> Result<&'static str, String> {
    let Some(e) = trim_string_or_undefined(effort) else {
        return Ok("unsupported");
    };
    let caps = get_model_capabilities(&prop_key(provider)?, model)?;
    if caps.0.is_empty() {
        return Ok("unknown");
    }
    Ok(if has_effort(caps, &e) {
        "supported"
    } else {
        "unsupported"
    })
}

// ── Display names (packages/shared/src/modelName.ts) ─────────────────────────

fn token_display(lower: &str) -> Option<&'static str> {
    MODEL_TOKEN_DISPLAY_NAMES
        .iter()
        .find(|(k, _)| *k == lower)
        .map(|(_, v)| *v)
}

fn join_model_version_tokens(tokens: Vec<Vec<u16>>) -> Vec<Vec<u16>> {
    let mut merged: Vec<Vec<u16>> = Vec::new();
    for token in tokens {
        let all_digits = !token.is_empty()
            && token
                .iter()
                .all(|&c| c == b'0' as u16 || matches!(c, 48..=57));
        let mergeable = all_digits
            && (token == [b'0' as u16] || token.first() != Some(&(b'0' as u16)))
            && token.len() != 8
            && merged
                .last()
                .is_some_and(|p| p.last().is_some_and(|&c| matches!(c, 48..=57)));
        if mergeable {
            let mut prev = merged.pop().unwrap_or_default();
            prev.push(b'.' as u16);
            prev.extend_from_slice(&token);
            merged.push(prev);
        } else {
            merged.push(token);
        }
    }
    merged
}

fn first_unit_upper(u: u16) -> Vec<u16> {
    match char::from_u32(u as u32) {
        Some(c) => c
            .to_uppercase()
            .collect::<String>()
            .encode_utf16()
            .collect(),
        None => vec![u],
    }
}

fn humanize_token(t: &[u16]) -> Vec<u16> {
    let key = lower_string(t);
    if let Some(name) = token_display(&key) {
        return name.encode_utf16().collect();
    }
    let mut out = Vec::with_capacity(t.len());
    if let Some(&first) = t.first() {
        out.extend(first_unit_upper(first));
        out.extend_from_slice(&t[1..]);
    }
    out
}

fn restore_separators(u: &[u16]) -> Vec<u16> {
    let is_word = |c: u16| matches!(c, 48..=57 | 65..=90 | 97..=122 | 95);
    let gpt: Vec<u16> = "GPT ".encode_utf16().collect();
    let mut out = Vec::with_capacity(u.len());
    let mut i = 0;
    while i < u.len() {
        if i + 4 < u.len()
            && u[i..i + 4] == gpt[..]
            && matches!(u[i + 4], 48..=57)
            && (i == 0 || !is_word(u[i - 1]))
        {
            out.extend("GPT-".encode_utf16());
            i += 4;
        } else {
            out.push(u[i]);
            i += 1;
        }
    }
    out
}

fn split_run(u: &[u16], is_sep: impl Fn(u16) -> bool) -> Vec<Vec<u16>> {
    let mut out = vec![Vec::new()];
    for &c in u {
        if is_sep(c) {
            out.push(Vec::new());
        } else {
            if let Some(t) = out.last_mut() {
                t.push(c)
            }
        }
    }
    out
}

fn humanize_model_slug(slug: &[u16]) -> Vec<u16> {
    if slug.contains(&(b'/' as u16)) {
        return slug.to_vec();
    }
    let tokens = split_run(slug, |c| c == b'-' as u16 || c == b'_' as u16);
    let merged = join_model_version_tokens(tokens);
    let humanized: Vec<Vec<u16>> = merged.iter().map(|t| humanize_token(t)).collect();
    let joined = humanized.join(&(b' ' as u16));
    restore_separators(&joined)
}

fn format_model_display_name(model: &[u16]) -> Option<Vec<u16>> {
    let t = trim_units(model);
    if t.is_empty() {
        return None;
    }
    let key = lower_string(t);
    if let Some(name) = model_name_by_slug(&key) {
        return Some(name.encode_utf16().collect());
    }
    Some(humanize_model_slug(t))
}

fn format_provider_model_option_name(provider: &str, slug: &[u16]) -> Result<Vec<u16>, String> {
    let trimmed_slug = if provider == "cursor" {
        strip_bracket_suffix_star(trim_units(slug))
    } else {
        trim_units(slug).to_vec()
    };
    if trimmed_slug.is_empty() {
        return Ok(trimmed_slug);
    }
    if provider == "opencode" || provider == "pi" || provider == "omp" {
        let model_id = if trimmed_slug.contains(&(b'/' as u16)) {
            let pos = trimmed_slug
                .iter()
                .rposition(|&c| c == b'/' as u16)
                .unwrap_or(0);
            trimmed_slug[pos + 1..].to_vec()
        } else {
            trimmed_slug
        };
        return Ok(
            format_model_display_name(&model_id).unwrap_or_else(|| humanize_model_slug(&model_id))
        );
    }
    Ok(format_model_display_name(&trimmed_slug).unwrap_or(trimmed_slug))
}

// ── Cursor variant base id ───────────────────────────────────────────────────

fn opt_trim(v: &Value) -> Result<Option<Vec<u16>>, String> {
    if is_nullish(v) {
        return Ok(None);
    }
    match str_units(v) {
        Some(u) => Ok(Some(trim_units(&u).to_vec())),
        None => Err(format!(
            "TypeError: {}?.trim is not a function. (In '{}?.trim()', '{}?.trim' is undefined)",
            js_repr(v).trim_end(),
            js_repr(v).trim_end(),
            js_repr(v).trim_end()
        )),
    }
}

fn strip_suffix(u: &[u16], suffix: &str) -> Vec<u16> {
    let s: Vec<u16> = suffix.encode_utf16().collect();
    if u.len() >= s.len() && u[u.len() - s.len()..] == s[..] {
        u[..u.len() - s.len()].to_vec()
    } else {
        u.to_vec()
    }
}

fn strip_effort_suffix(u: &[u16]) -> Vec<u16> {
    for s in ["-extra-high", "-none", "-low", "-medium", "-high", "-xhigh"] {
        let stripped = strip_suffix(u, s);
        if stripped.len() != u.len() {
            return stripped;
        }
    }
    u.to_vec()
}

fn normalize_cursor_variant_base_id(model: &Value) -> Result<Option<Vec<u16>>, String> {
    let Some(trimmed) = opt_trim(model)? else {
        return Ok(None);
    };
    if trimmed.is_empty() {
        return Ok(None);
    }
    let mut base = strip_bracket_suffix_star(&trimmed);
    base = strip_suffix(&base, "-fast");
    base = strip_effort_suffix(&base);
    base = strip_suffix(&base, "-thinking");
    base = strip_suffix(&base, "-fast");
    base = strip_effort_suffix(&base);
    let s = units_lossy(&base);
    if s.ends_with("-max") && !s.contains("codex-max") {
        base = base[..base.len() - 4].to_vec();
    }
    // /^claude-(\d+(?:\.\d+)?)-([a-z]+)-max$/ → claude-$1-$2
    let bs = units_lossy(&base);
    if let Some(new_base) = claude_dotted_max(&bs) {
        base = new_base.encode_utf16().collect();
    }
    base = strip_suffix(&base, "-preview");
    // /^claude-(\d+(?:\.\d+)?)-([a-z]+)$/ → claude-$2-$1(.→-)
    let bs = units_lossy(&base);
    if let Some(reordered) = claude_reordered(&bs) {
        base = reordered.encode_utf16().collect();
    }
    Ok(Some(base))
}

fn claude_dotted_max(s: &str) -> Option<String> {
    let rest = s.strip_prefix("claude-")?;
    let (ver, rest) = rest.split_once('-')?;
    let mut parts = ver.split('.');
    let maj = parts.next()?;
    if maj.is_empty() || !maj.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if let Some(min) = parts.next()
        && (min.is_empty() || !min.bytes().all(|b| b.is_ascii_digit()) || parts.next().is_some())
    {
        return None;
    }
    let family = rest.strip_suffix("-max")?;
    if family.is_empty() || !family.bytes().all(|b| b.is_ascii_lowercase()) {
        return None;
    }
    Some(format!("claude-{ver}-{family}"))
}

fn claude_reordered(s: &str) -> Option<String> {
    let rest = s.strip_prefix("claude-")?;
    let (ver, family) = rest.split_once('-')?;
    let mut parts = ver.split('.');
    let maj = parts.next()?;
    if maj.is_empty() || !maj.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if let Some(min) = parts.next()
        && (min.is_empty() || !min.bytes().all(|b| b.is_ascii_digit()) || parts.next().is_some())
    {
        return None;
    }
    if family.is_empty() || !family.bytes().all(|b| b.is_ascii_lowercase()) {
        return None;
    }
    Some(format!("claude-{family}-{}", ver.replace('.', "-")))
}

// ── normalizeProviderModelOptions ────────────────────────────────────────────

fn fields_object(fields: &[(&str, Option<Value>)]) -> Option<Value> {
    let mut o = Map::new();
    for (k, v) in fields {
        if let Some(v) = v {
            o.insert((*k).to_string(), v.clone());
        }
    }
    (!o.is_empty()).then_some(Value::Object(o))
}

fn obj_field(v: Option<&Value>, key: &str) -> Option<Value> {
    let f = v.map(|x| js_get(x, key)).unwrap_or_else(js::undefined);
    (truthy(&f) && is_object_type(&f)).then_some(f)
}

fn trim_field(v: Option<&Value>, key: &str) -> Option<Vec<u16>> {
    trim_string_or_undefined(&v.map(|x| js_get(x, key)).unwrap_or_else(js::undefined))
}

fn npmo(value: &Value, provider: Option<&Value>, legacy: Option<&Value>) -> Result<Value, String> {
    let candidate = (truthy(value) && is_object_type(value)).then_some(value);
    let codex_c = obj_field(candidate, "codex");
    let claude_c = obj_field(candidate, "claudeAgent");
    let cursor_c = obj_field(candidate, "cursor");
    let devin_c = obj_field(candidate, "devin");
    let ag_c = obj_field(candidate, "antigravity");
    let grok_c = obj_field(candidate, "grok");
    let droid_c = obj_field(candidate, "droid");
    let oc_c = obj_field(candidate, "opencode");
    let pi_c = obj_field(candidate, "pi");
    let omp_c = obj_field(candidate, "omp");

    let lget = |k: &str| -> Value { legacy.map(|l| js_get(l, k)).unwrap_or_else(js::undefined) };
    let provider_is = |k: &str| provider.is_some_and(|p| str_eq(p, k));
    let sget = |c: &Option<Value>, k: &str| -> Value {
        c.as_ref()
            .map(|v| js_get(v, k))
            .unwrap_or_else(js::undefined)
    };

    // codex
    let codex_effort = trim_field(codex_c.as_ref(), "reasoningEffort").or_else(|| {
        if provider_is("codex") {
            trim_string_or_undefined(&lget("effort"))
        } else {
            None
        }
    });
    let fm = sget(&codex_c, "fastMode");
    let codex_fast = if strict_eq(&fm, &Value::Bool(true)) {
        Some(true)
    } else if strict_eq(&fm, &Value::Bool(false)) {
        Some(false)
    } else if (provider_is("codex") && strict_eq(&lget("codexFastMode"), &Value::Bool(true)))
        || str_eq(&lget("serviceTier"), "fast")
    {
        Some(true)
    } else {
        None
    };
    let tier = sget(&codex_c, "serviceTier");
    let codex_tier = str_eq(&tier, "ultrafast").then_some("ultrafast");
    let codex = fields_object(&[
        (
            "reasoningEffort",
            codex_effort.map(|u| js::utf16_string(&u)),
        ),
        ("fastMode", codex_fast.map(Value::Bool)),
        ("serviceTier", codex_tier.map(|t| Value::String(t.into()))),
    ]);

    // claudeAgent
    let claude_thinking = match sget(&claude_c, "thinking") {
        Value::Bool(b) => Some(b),
        _ => None,
    };
    let claude_effort_raw = sget(&claude_c, "effort");
    let claude_effort = CLAUDE_EFFORT_SET
        .iter()
        .any(|e| str_eq(&claude_effort_raw, e))
        .then_some(claude_effort_raw);
    let claude_fast = match sget(&claude_c, "fastMode") {
        Value::Bool(b) => Some(b),
        _ => None,
    };
    let claude_window = trim_field(claude_c.as_ref(), "autoCompactWindow")
        .or_else(|| trim_field(claude_c.as_ref(), "contextWindow"));
    let claude = fields_object(&[
        ("thinking", claude_thinking.map(Value::Bool)),
        ("effort", claude_effort),
        ("fastMode", claude_fast.map(Value::Bool)),
        (
            "autoCompactWindow",
            claude_window.map(|u| js::utf16_string(&u)),
        ),
    ]);

    // cursor
    let cursor = fields_object(&[
        (
            "reasoningEffort",
            trim_field(cursor_c.as_ref(), "reasoningEffort").map(|u| js::utf16_string(&u)),
        ),
        (
            "fastMode",
            match sget(&cursor_c, "fastMode") {
                Value::Bool(b) => Some(Value::Bool(b)),
                _ => None,
            },
        ),
        (
            "thinking",
            match sget(&cursor_c, "thinking") {
                Value::Bool(b) => Some(Value::Bool(b)),
                _ => None,
            },
        ),
        (
            "contextWindow",
            trim_field(cursor_c.as_ref(), "contextWindow").map(|u| js::utf16_string(&u)),
        ),
    ]);

    // antigravity
    let antigravity = fields_object(&[(
        "reasoningEffort",
        trim_field(ag_c.as_ref(), "reasoningEffort").map(|u| js::utf16_string(&u)),
    )]);

    // grok
    let grok_effort_raw = sget(&grok_c, "reasoningEffort");
    let grok_effort = GROK_EFFORT_SET
        .iter()
        .any(|e| str_eq(&grok_effort_raw, e))
        .then_some(grok_effort_raw);
    let grok = fields_object(&[("reasoningEffort", grok_effort)]);

    // droid
    let droid = fields_object(&[(
        "reasoningEffort",
        trim_field(droid_c.as_ref(), "reasoningEffort").map(|u| js::utf16_string(&u)),
    )]);

    // opencode
    let opencode = fields_object(&[
        (
            "variant",
            trim_field(oc_c.as_ref(), "variant").map(|u| js::utf16_string(&u)),
        ),
        (
            "agent",
            trim_field(oc_c.as_ref(), "agent").map(|u| js::utf16_string(&u)),
        ),
    ]);

    // pi
    let pi_raw = sget(&pi_c, "thinkingLevel");
    let pi_level = PI_THINKING_SET
        .iter()
        .any(|e| str_eq(&pi_raw, e))
        .then_some(pi_raw);
    let pi = fields_object(&[("thinkingLevel", pi_level)]);

    // devin
    let devin = fields_object(&[
        (
            "reasoningEffort",
            trim_field(devin_c.as_ref(), "reasoningEffort").map(|u| js::utf16_string(&u)),
        ),
        (
            "fastMode",
            match sget(&devin_c, "fastMode") {
                Value::Bool(b) => Some(Value::Bool(b)),
                _ => None,
            },
        ),
        (
            "thinking",
            match sget(&devin_c, "thinking") {
                Value::Bool(b) => Some(Value::Bool(b)),
                _ => None,
            },
        ),
        (
            "contextWindow",
            trim_field(devin_c.as_ref(), "contextWindow").map(|u| js::utf16_string(&u)),
        ),
        (
            "modelVariant",
            trim_field(devin_c.as_ref(), "modelVariant").map(|u| js::utf16_string(&u)),
        ),
    ]);

    // omp: normalizeOmpModelOptions(ompCandidate)
    let omp_level_raw = sget(&omp_c, "thinkingLevel");
    let omp = {
        let level = trim_string_or_undefined(&omp_level_raw)
            .filter(|u| OMP_THINKING_SET.iter().any(|e| units_eq_str(u, e)));
        fields_object(&[("thinkingLevel", level.map(|u| js::utf16_string(&u)))])
    };

    let mut out = Map::new();
    for (k, v) in [
        ("codex", codex),
        ("claudeAgent", claude),
        ("cursor", cursor),
        ("devin", devin),
        ("antigravity", antigravity),
        ("grok", grok),
        ("droid", droid),
        ("opencode", opencode),
        ("pi", pi),
        ("omp", omp),
    ] {
        if let Some(v) = v {
            out.insert(k.to_string(), v);
        }
    }
    Ok(if out.is_empty() {
        Value::Null
    } else {
        Value::Object(out)
    })
}

// ── Selection helpers ────────────────────────────────────────────────────────

fn pisk(provider: &Value, instance_id: Option<&Value>) -> Value {
    match instance_id.and_then(normalize_instance_id) {
        Some(s) => Value::String(s),
        None => provider.clone(),
    }
}

fn make_selection_value(
    provider: &Value,
    model: &Value,
    options: &Value,
    supports_auto_mode: &Value,
    instance_id: &Value,
) -> Result<Value, String> {
    let Some(kind) = PROVIDER_KINDS.iter().find(|k| str_eq(provider, k)) else {
        return Ok(js::undefined());
    };
    let instance = normalize_instance_id(instance_id);
    let mut o = Map::new();
    o.insert("provider".into(), provider.clone());
    o.insert("model".into(), model.clone());
    if let Some(i) = instance {
        o.insert("instanceId".into(), Value::String(i));
    }
    if truthy(options) {
        o.insert("options".into(), options.clone());
    }
    if *kind == "claudeAgent"
        && let Value::Bool(_) = supports_auto_mode
    {
        o.insert("supportsAutoMode".into(), supports_auto_mode.clone());
    }
    Ok(Value::Object(o))
}

fn matches_provider_instance(selection: &Value, provider: &Value, instance_id: &Value) -> bool {
    if !truthy(selection) {
        return false;
    }
    if !strict_eq(&js_get(selection, "provider"), provider) {
        return false;
    }
    let sel_iid = js_get(selection, "instanceId");
    let left = if is_nullish(&sel_iid) {
        js_get(selection, "provider")
    } else {
        sel_iid
    };
    let right = if is_nullish(instance_id) {
        provider.clone()
    } else {
        instance_id.clone()
    };
    strict_eq(&left, &right)
}

fn merge_opts_from_selections(sels: &[Value]) -> Result<Value, String> {
    let mut result = Map::new();
    for sel in sels {
        if !truthy(sel) {
            continue;
        }
        let key = prop_key(&js_get(sel, "provider"))?;
        let opts = js_get(sel, "options");
        if truthy(&opts) {
            result.insert(key, opts);
        } else {
            result.remove(&key);
        }
    }
    Ok(if result.is_empty() {
        Value::Null
    } else {
        Value::Object(finalize_map(result))
    })
}

fn derive_effective_model_options(input: &Value) -> Result<Value, String> {
    let sp = js_get(input, "selectedProvider");
    let spi = js_get(input, "selectedProviderInstanceId");
    let sel_for_target = |sel: &Value| -> Value {
        let p = if is_nullish(sel) {
            js::undefined()
        } else {
            js_get(sel, "provider")
        };
        if !strict_eq(&p, &sp) || matches_provider_instance(sel, &sp, &spi) {
            sel.clone()
        } else {
            Value::Null
        }
    };
    let base = merge_opts_from_selections(&[
        sel_for_target(&js_get(input, "projectModelSelection")),
        sel_for_target(&js_get(input, "threadModelSelection")),
    ])?;
    let draft = js_get(input, "draft");
    let draft_map = if is_nullish(&draft) {
        js::undefined()
    } else {
        js_get(&draft, "modelSelectionByProvider")
    };
    if !truthy(&draft_map) {
        return Ok(base);
    }
    let mut result = if truthy(&base) {
        spread_to_map(&base)
    } else {
        Map::new()
    };
    for sel in object_values(&draft_map)? {
        if !truthy(&sel) {
            continue;
        }
        let provider = js_get(&sel, "provider");
        if strict_eq(&provider, &sp) && !matches_provider_instance(&sel, &sp, &spi) {
            continue;
        }
        let key = prop_key(&provider)?;
        let opts = js_get(&sel, "options");
        if truthy(&opts) {
            result.insert(key, opts);
        } else {
            result.remove(&key);
        }
    }
    Ok(if result.is_empty() {
        Value::Null
    } else {
        Value::Object(finalize_map(result))
    })
}

// ── App model options / selectable model ─────────────────────────────────────

fn find_not_fn(options: &Value, call_src: &str) -> String {
    let r = js_repr(options).trim_end().to_string();
    format!(
        "TypeError: {r}.find is not a function. (In '{r}.find({call_src})', '{r}.find' is undefined)"
    )
}

fn resolve_selectable_model(
    provider: &Value,
    value: &Value,
    options: &[Value],
) -> Result<Option<Value>, String> {
    let Some(u) = str_units(value) else {
        return Ok(None);
    };
    let trimmed = trim_units(&u);
    if trimmed.is_empty() {
        return Ok(None);
    }
    for o in options {
        if strict_eq(&js_get(o, "slug"), &js::utf16_string(trimmed)) {
            return Ok(Some(js_get(o, "slug")));
        }
    }
    let t_lower = lower_string(trimmed);
    for o in options {
        let name = js_get(o, "name");
        match str_units(&name) {
            Some(nu) => {
                if lower_string(&nu) == t_lower {
                    return Ok(Some(js_get(o, "slug")));
                }
            }
            None => {
                let r = js_repr(o).trim_end().to_string();
                if is_nullish(&name) {
                    return Err(format!(
                        "TypeError: {} is not an object (evaluating '{r}.name.toLowerCase')",
                        nullish_word(&name)
                    ));
                }
                return Err(format!(
                    "TypeError: {r}.name.toLowerCase is not a function. (In '{r}.name.toLowerCase()', '{r}.name.toLowerCase' is undefined)"
                ));
            }
        }
    }
    let provider_key = prop_key(provider)?;
    let Some(normalized) = normalize_model_slug(Some(trimmed.to_vec()), &provider_key)? else {
        return Ok(None);
    };
    let nv = js::utf16_string(&normalized);
    for o in options {
        if strict_eq(&js_get(o, "slug"), &nv) {
            return Ok(Some(js_get(o, "slug")));
        }
    }
    if (provider_key == "omp" || provider_key == "pi" || provider_key == "opencode")
        && !normalized.contains(&(b'/' as u16))
    {
        let mut scoped: Vec<&Value> = Vec::new();
        for o in options {
            let slug = js_get(o, "slug");
            match str_units(&slug) {
                Some(s) => {
                    if s.len() > normalized.len()
                        && s[s.len() - normalized.len() - 1] == b'/' as u16
                        && s.ends_with(&normalized[..])
                    {
                        scoped.push(o);
                    }
                }
                None => {
                    let r = js_repr(o).trim_end().to_string();
                    if is_nullish(&slug) {
                        return Err(format!(
                            "TypeError: {} is not an object (evaluating '{r}.slug.endsWith')",
                            nullish_word(&slug)
                        ));
                    }
                    return Err(format!(
                        "TypeError: {r}.slug.endsWith is not a function. (In '{r}.slug.endsWith(\"/{}\")', '{r}.slug.endsWith' is undefined)",
                        units_lossy(&normalized)
                    ));
                }
            }
        }
        if scoped.len() == 1 {
            return Ok(Some(js_get(scoped[0], "slug")));
        }
    }
    Ok(None)
}

fn iterate(models: &Value) -> Result<Vec<Value>, String> {
    if let Value::Array(a) = models {
        return Ok(a.clone());
    }
    if let Some(u) = str_units(models) {
        return Ok(units_lossy(&u)
            .chars()
            .map(|c| Value::String(c.to_string()))
            .collect());
    }
    if let Value::Object(o) = models
        && o.len() == 1
    {
        if let Some(m) = o.get("$map").and_then(|m| m.as_array()) {
            return Ok(m
                .iter()
                .map(|kv| {
                    Value::Array(vec![
                        kv.get(0).cloned().unwrap_or(Value::Null),
                        kv.get(1).cloned().unwrap_or(Value::Null),
                    ])
                })
                .collect());
        }
        if let Some(s) = o.get("$set").and_then(|s| s.as_array()) {
            return Ok(s.clone());
        }
        if let Some(b) = o
            .get("$bytes")
            .or_else(|| o.get("$buffer"))
            .and_then(|b| b.as_array())
        {
            return Ok(b.clone());
        }
    }
    if is_nullish(models) {
        return Err(format!(
            "TypeError: {} is not an object (evaluating 'candidate of {}')",
            nullish_word(models),
            if models.is_null() { "null" } else { "void 0" }
        ));
    }
    let desc = match models {
        Value::Number(_) => "number".into(),
        Value::Bool(b) => {
            if *b {
                "true".into()
            } else {
                "false".into()
            }
        }
        Value::Object(_) => "{}".into(),
        _ => js_repr(models).trim_end().to_string(),
    };
    Err(format!("TypeError: {desc} is not iterable"))
}

fn normalize_custom_model_slugs(models: &Value, provider: &str) -> Result<Vec<String>, String> {
    let Some(built_in) = model_options(provider).map(|opts| {
        opts.iter()
            .map(|(s, _)| *s)
            .collect::<std::collections::HashSet<_>>()
    }) else {
        return Err("TypeError: undefined is not an object (evaluating '(void 0).has')".into());
    };
    let mut out: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for candidate in iterate(models)? {
        let Some(u) = str_units(&candidate) else {
            continue;
        };
        let Some(norm) = normalize_model_slug(Some(u), provider)? else {
            continue;
        };
        let nstr = units_lossy(&norm);
        if norm.len() > MAX_CUSTOM_MODEL_LENGTH
            || built_in.contains(&nstr.as_str())
            || !seen.insert(nstr.clone())
        {
            continue;
        }
        out.push(nstr);
        if out.len() >= MAX_CUSTOM_MODEL_COUNT {
            break;
        }
    }
    Ok(out)
}

fn get_app_model_options(
    provider: &str,
    custom_models: &Value,
    selected_model: &Value,
) -> Result<Vec<Value>, String> {
    let Some(builtins) = model_options(provider) else {
        return Err(
            "TypeError: undefined is not an object (evaluating 'getModelOptions(provider).map')"
                .into(),
        );
    };
    let mut options: Vec<Value> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (slug, name) in builtins {
        seen.insert((*slug).to_string());
        let mut o = Map::new();
        o.insert("provider".into(), Value::String(provider.into()));
        o.insert("slug".into(), Value::String((*slug).into()));
        o.insert("name".into(), Value::String((*name).into()));
        o.insert("isCustom".into(), Value::Bool(false));
        options.push(Value::Object(o));
    }
    let trimmed_selected = match opt_trim(selected_model)? {
        Some(u) if !u.is_empty() => Some(lower_string(&u)),
        _ => None,
    };
    for slug in normalize_custom_model_slugs(custom_models, provider)? {
        if seen.contains(&slug) {
            continue;
        }
        seen.insert(slug.clone());
        let slug_u: Vec<u16> = slug.encode_utf16().collect();
        let mut o = Map::new();
        o.insert("provider".into(), Value::String(provider.into()));
        o.insert("slug".into(), Value::String(slug.clone()));
        o.insert(
            "name".into(),
            js::utf16_string(&format_provider_model_option_name(provider, &slug_u)?),
        );
        o.insert("isCustom".into(), Value::Bool(true));
        options.push(Value::Object(o));
    }
    let normalized_selected = if provider == "cursor" {
        normalize_cursor_variant_base_id(selected_model)?
    } else {
        normalize_model_slug(str_units(selected_model), provider)?
    };
    let matches_name = trimmed_selected.as_ref().is_some_and(|t| {
        options.iter().any(|o| {
            str_units(&js_get(o, "name"))
                .map(|n| lower_string(&n) == *t)
                .unwrap_or(false)
        })
    });
    if let Some(ns) = normalized_selected {
        let nstr = units_lossy(&ns);
        if !seen.contains(&nstr) && !matches_name {
            let mut o = Map::new();
            o.insert("provider".into(), Value::String(provider.into()));
            o.insert("slug".into(), js::utf16_string(&ns));
            o.insert(
                "name".into(),
                js::utf16_string(&format_provider_model_option_name(provider, &ns)?),
            );
            o.insert("isCustom".into(), Value::Bool(true));
            options.push(Value::Object(o));
        }
    }
    Ok(options)
}

fn resolve_app_model_selection(
    provider: &Value,
    custom_models: &Value,
    selected_model: &Value,
) -> Result<Value, String> {
    let provider_key = prop_key(provider)?;
    let custom_for = js_get(custom_models, &provider_key);
    let options = get_app_model_options(&provider_key, &custom_for, selected_model)?;
    if let Some(v) = resolve_selectable_model(provider, selected_model, &options)? {
        return Ok(v);
    }
    let d = default_model_for(provider);
    Ok(if is_nullish(&d) {
        Value::String(String::new())
    } else {
        d
    })
}

// ── Public ports ─────────────────────────────────────────────────────────────

/// Port of `normalizeProviderKind` (`apps/web/src/composerDraftModels.ts:154`).
pub fn normalize_provider_kind(
    value: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    if str_eq(&value, "gemini") {
        return Ok(Some(Value::String("antigravity".into())));
    }
    if str_eq(&value, "kilo") {
        return Ok(Some(Value::String("opencode".into())));
    }
    if PROVIDER_KINDS.iter().any(|k| str_eq(&value, k)) {
        return Ok(Some(value));
    }
    Ok(None)
}

/// Port of `normalizeProviderInstanceId` (`apps/web/src/composerDraftModels.ts:172`).
pub fn normalize_provider_instance_id(
    value: serde_json::Value,
) -> Result<serde_json::Value, String> {
    Ok(match normalize_instance_id(&value) {
        Some(s) => Value::String(s),
        None => js::undefined(),
    })
}

/// Port of `providerInstanceModelSelectionKey` (`apps/web/src/composerDraftModels.ts:177`).
pub fn provider_instance_model_selection_key(
    provider: serde_json::Value,
    instance_id: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    Ok(pisk(&provider, instance_id.as_ref()))
}

/// Port of `modelSelectionStorageKey` (`apps/web/src/composerDraftModels.ts:184`).
pub fn model_selection_storage_key(
    selection: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if is_nullish(&selection) {
        return Err(format!(
            "TypeError: {} is not an object (evaluating 'selection.provider')",
            nullish_word(&selection)
        ));
    }
    let provider = js_get(&selection, "provider");
    let instance_id = js_get(&selection, "instanceId");
    Ok(pisk(&provider, Some(&instance_id)))
}

/// Port of `readModelSelectionForProviderInstance` (`apps/web/src/composerDraftModels.ts:188`).
pub fn read_model_selection_for_provider_instance(
    selections: Option<serde_json::Value>,
    provider: serde_json::Value,
    instance_id: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let key = pisk(&provider, instance_id.as_ref());
    let Some(selections) = selections else {
        return Ok(js::undefined());
    };
    if is_nullish(&selections) {
        return Ok(js::undefined());
    }
    Ok(js_get(&selections, &prop_key(&key)?))
}

/// Port of `normalizeModelSelectionMapByInstance` (`apps/web/src/composerDraftModels.ts:196`).
pub fn normalize_model_selection_map_by_instance(
    selections: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let mut result = Map::new();
    for sel in object_values(&selections)? {
        if truthy(&sel) {
            let provider = js_get(&sel, "provider");
            let instance_id = js_get(&sel, "instanceId");
            let key = prop_key(&pisk(&provider, Some(&instance_id)))?;
            result.insert(key, sel);
        }
    }
    Ok(Value::Object(finalize_map(result)))
}

/// Port of `makeModelSelection` (`apps/web/src/composerDraftModels.ts:216`).
pub fn make_model_selection(
    provider: serde_json::Value,
    model: serde_json::Value,
    options: Option<serde_json::Value>,
    supports_auto_mode: serde_json::Value,
    instance_id: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    make_selection_value(
        &provider,
        &model,
        &options.unwrap_or_else(js::undefined),
        &supports_auto_mode,
        &instance_id.unwrap_or_else(js::undefined),
    )
}

/// Port of `normalizeProviderModelOptions` (`apps/web/src/composerDraftModels.ts:323`).
pub fn normalize_provider_model_options(
    value: serde_json::Value,
    provider: Option<serde_json::Value>,
    legacy: Option<serde_json::Value>,
) -> Result<Option<serde_json::Value>, String> {
    let out = npmo(&value, provider.as_ref(), legacy.as_ref())?;
    Ok(if out.is_null() { None } else { Some(out) })
}

/// Port of `normalizeModelSelection` (`apps/web/src/composerDraftModels.ts:529`).
pub fn normalize_model_selection(
    value: serde_json::Value,
    legacy: Option<serde_json::Value>,
) -> Result<Option<serde_json::Value>, String> {
    let candidate = (truthy(&value) && is_object_type(&value)).then_some(&value);
    let cand = |k: &str| -> Value {
        candidate
            .map(|c| js_get(c, k))
            .unwrap_or_else(js::undefined)
    };
    let leg = |k: &str| -> Value {
        legacy
            .as_ref()
            .map(|l| js_get(l, k))
            .unwrap_or_else(js::undefined)
    };
    let coalesce = |a: Value, b: Value| if is_nullish(&a) { b } else { a };
    let raw_provider = coalesce(cand("provider"), leg("provider"));
    let migrated_gemini = str_eq(&raw_provider, "gemini");
    let provider = if migrated_gemini {
        Some("antigravity")
    } else if str_eq(&raw_provider, "kilo") {
        Some("opencode")
    } else {
        PROVIDER_KINDS
            .iter()
            .find(|k| str_eq(&raw_provider, k))
            .copied()
    };
    let Some(provider) = provider else {
        return Ok(None);
    };
    let raw_model = coalesce(cand("model"), leg("model"));
    let Some(raw_model_units) = str_units(&raw_model) else {
        return Ok(None);
    };
    let instance_id = cand("instanceId");

    let raw_trimmed = trim_units(&raw_model_units).to_vec();
    let legacy_match = if provider == "antigravity" {
        antigravity_legacy_match(&raw_trimmed)
    } else {
        None
    };
    let legacy_effort = legacy_match
        .as_ref()
        .map(|(_, inner)| lower_string(trim_units(inner)));
    let has_legacy = legacy_match.is_some()
        && legacy_effort
            .as_ref()
            .is_some_and(|e| ANTIGRAVITY_EFFORT_SET.contains(&e.as_str()));
    let normalized_raw = if migrated_gemini {
        "Gemini 3.5 Flash".encode_utf16().collect::<Vec<u16>>()
    } else if has_legacy {
        trim_units(&legacy_match.clone().map(|(g, _)| g).unwrap_or_default()).to_vec()
    } else {
        raw_model_units.clone()
    };
    let Some(model) = normalize_model_slug(Some(normalized_raw), provider)? else {
        return Ok(None);
    };

    let model_options = if migrated_gemini {
        Value::Null
    } else {
        let opts_source = {
            let cand_opts = cand("options");
            if truthy(&cand_opts) {
                let mut m = Map::new();
                m.insert(provider.to_string(), cand_opts);
                Value::Object(m)
            } else {
                leg("modelOptions")
            }
        };
        let legacy_codex = if provider == "codex" {
            leg("legacyCodex")
        } else {
            js::undefined()
        };
        npmo(
            &opts_source,
            Some(&Value::String(provider.into())),
            if is_undef(&legacy_codex) {
                None
            } else {
                Some(&legacy_codex)
            },
        )?
    };
    let options = match provider {
        "codex" => js_get(&model_options, "codex"),
        "claudeAgent" => js_get(&model_options, "claudeAgent"),
        "antigravity" => js_get(&model_options, "antigravity"),
        "grok" => normalize_grok_model_options(
            &js::utf16_string(&model),
            &js_get(&model_options, "grok"),
        )?,
        "droid" => js_get(&model_options, "droid"),
        "cursor" => js_get(&model_options, "cursor"),
        "opencode" => js_get(&model_options, "opencode"),
        "pi" => js_get(&model_options, "pi"),
        "devin" => js_get(&model_options, "devin"),
        _ => js_get(&model_options, "omp"),
    };
    let normalized_options = if provider == "antigravity" && has_legacy {
        let e = js_get(&js_get(&model_options, "antigravity"), "reasoningEffort");
        let e = if is_nullish(&e) {
            Value::String(legacy_effort.unwrap_or_default())
        } else {
            e
        };
        let mut m = Map::new();
        m.insert("reasoningEffort".into(), e);
        Value::Object(m)
    } else {
        options
    };
    let auto = if provider == "claudeAgent" {
        let s = cand("supportsAutoMode");
        if matches!(s, Value::Bool(_)) {
            s
        } else {
            js::undefined()
        }
    } else {
        js::undefined()
    };
    let sel = make_selection_value(
        &Value::String(provider.into()),
        &js::utf16_string(&model),
        &normalized_options,
        &auto,
        &instance_id,
    )?;
    Ok(if is_undef(&sel) { None } else { Some(sel) })
}

/// Port of `reconcileProviderScopedModelSelection` (`apps/web/src/composerDraftModels.ts:612`).
pub fn reconcile_provider_scoped_model_selection(
    requested: serde_json::Value,
    current: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    if is_nullish(&requested) {
        return Err(format!(
            "TypeError: {} is not an object (evaluating 'requested.options')",
            nullish_word(&requested)
        ));
    }
    let req_options = js_get(&requested, "options");
    if !is_undef(&req_options) {
        return Ok(requested);
    }
    let cur = current.unwrap_or(Value::Null);
    let req_provider = js_get(&requested, "provider");
    let req_instance = js_get(&requested, "instanceId");
    if !matches_provider_instance(&cur, &req_provider, &req_instance) {
        return Ok(requested);
    }
    let cur_model = js_get(&cur, "model");
    let req_model = js_get(&requested, "model");
    let cur_provider = js_get(&cur, "provider");
    if strict_eq(&cur_model, &req_model) {
        let cur_auto = if str_eq(&cur_provider, "claudeAgent") {
            js_get(&cur, "supportsAutoMode")
        } else {
            js::undefined()
        };
        let auto = if str_eq(&req_provider, "claudeAgent") {
            let ra = js_get(&requested, "supportsAutoMode");
            if is_nullish(&ra) { cur_auto } else { ra }
        } else {
            js::undefined()
        };
        return make_selection_value(
            &req_provider,
            &req_model,
            &js_get(&cur, "options"),
            &auto,
            &req_instance,
        );
    }
    if !str_eq(&cur_provider, "codex")
        && !str_eq(&cur_provider, "cursor")
        && !str_eq(&cur_provider, "claudeAgent")
    {
        return Ok(requested);
    }
    let cur_options = js_get(&cur, "options");
    let mut preserved = cur_options.clone();
    let effort = if str_eq(&cur_provider, "claudeAgent") {
        if is_nullish(&cur_options) {
            js::undefined()
        } else {
            js_get(&cur_options, "effort")
        }
    } else if is_nullish(&cur_options) {
        js::undefined()
    } else {
        js_get(&cur_options, "reasoningEffort")
    };
    if !is_undef(&effort)
        && classify_provider_effort(&req_provider, &req_model, &effort)? != "supported"
    {
        let drop_key = if str_eq(&cur_provider, "claudeAgent") {
            "effort"
        } else {
            "reasoningEffort"
        };
        let mut rest = Map::new();
        if !is_nullish(&cur_options) {
            for (k, v) in spread_props(&cur_options) {
                if k != drop_key {
                    rest.insert(k, v);
                }
            }
        }
        preserved = if rest.is_empty() {
            js::undefined()
        } else {
            Value::Object(finalize_map(rest))
        };
    }
    let auto = if str_eq(&req_provider, "claudeAgent") {
        js_get(&requested, "supportsAutoMode")
    } else {
        js::undefined()
    };
    make_selection_value(&req_provider, &req_model, &preserved, &auto, &req_instance)
}

/// Port of `stripNonStickyModelOptions` (`apps/web/src/composerDraftModels.ts:674`).
pub fn strip_non_sticky_model_options(
    selection: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if is_nullish(&selection) {
        return Err(format!(
            "TypeError: {} is not an object (evaluating 'selection.provider')",
            nullish_word(&selection)
        ));
    }
    let provider = js_get(&selection, "provider");
    let options = js_get(&selection, "options");
    let cw = if is_nullish(&options) {
        js::undefined()
    } else {
        js_get(&options, "contextWindow")
    };
    let acw = if is_nullish(&options) {
        js::undefined()
    } else {
        js_get(&options, "autoCompactWindow")
    };
    if !str_eq(&provider, "claudeAgent") || (!truthy(&cw) && !truthy(&acw)) {
        return Ok(selection);
    }
    let mut rest = Map::new();
    for (k, v) in spread_props(&options) {
        if k != "contextWindow" && k != "autoCompactWindow" {
            rest.insert(k, v);
        }
    }
    let opts = if rest.is_empty() {
        js::undefined()
    } else {
        Value::Object(finalize_map(rest))
    };
    make_selection_value(
        &provider,
        &js_get(&selection, "model"),
        &opts,
        &js_get(&selection, "supportsAutoMode"),
        &js_get(&selection, "instanceId"),
    )
}

/// Port of `sanitizeStickyModelSelectionMap` (`apps/web/src/composerDraftModels.ts:695`).
pub fn sanitize_sticky_model_selection_map(
    map: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let entries = object_entries(&map)?;
    let mut next: Option<Map<String, Value>> = None;
    for (key, sel) in entries {
        let sp = if is_nullish(&sel) {
            js::undefined()
        } else {
            js_get(&sel, "provider")
        };
        if !str_eq(&sp, "claudeAgent") {
            continue;
        }
        let so = js_get(&sel, "options");
        let has = !is_nullish(&so)
            && (truthy(&js_get(&so, "contextWindow")) || truthy(&js_get(&so, "autoCompactWindow")));
        if has {
            let m = next.get_or_insert_with(|| spread_to_map(&map));
            m.insert(key, strip_non_sticky_model_options(sel)?);
        }
    }
    Ok(match next {
        Some(m) => Value::Object(finalize_map(m)),
        None => map,
    })
}

/// Port of `legacySyncModelSelectionOptions` (`apps/web/src/composerDraftModels.ts:711`).
pub fn legacy_sync_model_selection_options(
    model_selection: serde_json::Value,
    model_options: Option<serde_json::Value>,
) -> Result<Option<serde_json::Value>, String> {
    if model_selection.is_null() {
        return Ok(None);
    }
    if is_undef(&model_selection) {
        return Err(
            "TypeError: undefined is not an object (evaluating 'modelSelection.provider')".into(),
        );
    }
    let provider = js_get(&model_selection, "provider");
    let normalized_options = if str_eq(&provider, "grok") {
        let g = model_options
            .as_ref()
            .map(|m| js_get(m, "grok"))
            .unwrap_or_else(js::undefined);
        normalize_grok_model_options(&js_get(&model_selection, "model"), &g)?
    } else {
        let key = prop_key(&provider)?;
        model_options
            .as_ref()
            .map(|m| js_get(m, &key))
            .unwrap_or_else(js::undefined)
    };
    let auto = if str_eq(&provider, "claudeAgent") {
        js_get(&model_selection, "supportsAutoMode")
    } else {
        js::undefined()
    };
    let sel = make_selection_value(
        &provider,
        &js_get(&model_selection, "model"),
        &normalized_options,
        &auto,
        &js_get(&model_selection, "instanceId"),
    )?;
    Ok(Some(sel))
}

/// Port of `legacyMergeModelSelectionIntoProviderModelOptions` (`apps/web/src/composerDraftModels.ts:731`).
pub fn legacy_merge_model_selection_into_provider_model_options(
    model_selection: Option<serde_json::Value>,
    current_model_options: Option<serde_json::Value>,
) -> Result<Option<serde_json::Value>, String> {
    let cmo = current_model_options.unwrap_or_else(js::undefined);
    let ms_opts = model_selection
        .as_ref()
        .map(|m| js_get(m, "options"))
        .unwrap_or_else(js::undefined);
    if is_undef(&ms_opts) {
        let out = npmo(&cmo, None, None)?;
        return Ok(if out.is_null() { None } else { Some(out) });
    }
    let provider = js_get(model_selection.as_ref().unwrap_or(&Value::Null), "provider");
    let current = npmo(&cmo, None, None)?;
    let pkey = prop_key(&provider)?;
    let mut others = Map::new();
    if !is_nullish(&current) {
        for (k, v) in spread_props(&current) {
            if k != pkey {
                others.insert(k, v);
            }
        }
    }
    let mut single = Map::new();
    single.insert(pkey, ms_opts);
    let normalized_next = npmo(&Value::Object(single), Some(&provider), None)?;
    let mut merged = others;
    if !is_nullish(&normalized_next) {
        for (k, v) in spread_props(&normalized_next) {
            merged.insert(k, v);
        }
    }
    let out = npmo(&Value::Object(finalize_map(merged)), None, None)?;
    Ok(if out.is_null() { None } else { Some(out) })
}

/// Port of `legacyToModelSelectionByProvider` (`apps/web/src/composerDraftModels.ts:763`).
pub fn legacy_to_model_selection_by_provider(
    model_selection: Option<serde_json::Value>,
    model_options: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let mut result = Map::new();
    if let Some(mo) = model_options.as_ref().filter(|m| truthy(m)) {
        for provider in COMPOSER_PROVIDER_KINDS {
            let options = js_get(mo, provider);
            if truthy(&options) && !object_keys(&options)?.is_empty() {
                let msp = model_selection
                    .as_ref()
                    .map(|m| js_get(m, "provider"))
                    .unwrap_or_else(js::undefined);
                let model = if strict_eq(&msp, &Value::String((*provider).into())) {
                    model_selection
                        .as_ref()
                        .map(|m| js_get(m, "model"))
                        .unwrap_or_else(js::undefined)
                } else {
                    default_model_for(&Value::String((*provider).into()))
                };
                if truthy(&model) {
                    let sel_opts = if *provider == "grok" {
                        normalize_grok_model_options(&model, &js_get(mo, "grok"))?
                    } else {
                        options.clone()
                    };
                    let sel = make_selection_value(
                        &Value::String((*provider).into()),
                        &model,
                        &sel_opts,
                        &js::undefined(),
                        &js::undefined(),
                    )?;
                    result.insert((*provider).to_string(), sel);
                }
            }
        }
    }
    if let Some(ms) = &model_selection
        && truthy(ms)
    {
        let provider = js_get(ms, "provider");
        let instance_id = js_get(ms, "instanceId");
        let key = prop_key(&pisk(&provider, Some(&instance_id)))?;
        result.insert(key, ms.clone());
    }
    Ok(Value::Object(finalize_map(result)))
}

/// Port of `deriveEffectiveComposerModelState` (`apps/web/src/composerDraftModels.ts:792`).
pub fn derive_effective_composer_model_state(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if is_nullish(&input) {
        return Err(format!(
            "TypeError: {} is not an object (evaluating 'input.selectedProvider')",
            nullish_word(&input)
        ));
    }
    let sp = js_get(&input, "selectedProvider");
    let spi = js_get(&input, "selectedProviderInstanceId");
    let thread_sel = js_get(&input, "threadModelSelection");
    let project_sel = js_get(&input, "projectModelSelection");
    let custom_models = js_get(&input, "customModelsByProvider");
    let ambp = js_get(&input, "availableModelOptionsByProvider");
    let draft = js_get(&input, "draft");
    let draft_map = if is_nullish(&draft) {
        js::undefined()
    } else {
        js_get(&draft, "modelSelectionByProvider")
    };

    let matches_sel = |sel: &Value| matches_provider_instance(sel, &sp, &spi);

    let avail = if is_nullish(&ambp) {
        js::undefined()
    } else {
        js_get(&ambp, &prop_key(&sp)?)
    };
    let resolve_available = |candidate: &Value| -> Result<Option<Value>, String> {
        if !truthy(&avail) {
            return Ok(None);
        }
        let len = js_get(&avail, "length");
        if strict_eq(&len, &Value::from(0i64)) {
            return Ok(None);
        }
        let Some(opts) = avail.as_array() else {
            return Err(find_not_fn(&avail, "(option) => option.slug === trimmed"));
        };
        resolve_selectable_model(&sp, candidate, opts)
    };

    let base_chain = {
        let m = if matches_sel(&thread_sel) {
            js_get(&thread_sel, "model")
        } else {
            Value::Null
        };
        let m = if is_nullish(&m) {
            if matches_sel(&project_sel) {
                js_get(&project_sel, "model")
            } else {
                Value::Null
            }
        } else {
            m
        };
        if is_nullish(&m) {
            default_model_for(&sp)
        } else {
            m
        }
    };
    let base_model =
        resolve_model_slug(str_units(&base_chain), &prop_key(&sp)?)?.map(|u| js::utf16_string(&u));

    let persisted = |sel: &Value| -> Result<Value, String> {
        if !matches_sel(sel) {
            return Ok(Value::Null);
        }
        let m = js_get(sel, "model");
        match normalize_model_slug(str_units(&m), &prop_key(&sp)?)? {
            Some(u) => Ok(js::utf16_string(&u)),
            None => Ok(m),
        }
    };
    let persisted_thread = persisted(&thread_sel)?;
    let persisted_project = persisted(&project_sel)?;

    let active_selection = if is_nullish(&draft_map) {
        js::undefined()
    } else {
        let key = pisk(&sp, Some(&spi));
        js_get(&draft_map, &prop_key(&key)?)
    };

    if str_eq(&sp, "omp") {
        let requested = {
            let a = if matches_sel(&active_selection) {
                active_selection.clone()
            } else {
                Value::Null
            };
            let a = if is_nullish(&a) {
                if matches_sel(&thread_sel) {
                    thread_sel.clone()
                } else {
                    Value::Null
                }
            } else {
                a
            };
            if is_nullish(&a) {
                if matches_sel(&project_sel) {
                    project_sel.clone()
                } else {
                    Value::Null
                }
            } else {
                a
            }
        };
        if truthy(&requested) {
            let m = js_get(&requested, "model");
            let selected = match normalize_model_slug(str_units(&m), "omp")? {
                Some(u) => js::utf16_string(&u),
                None => m,
            };
            let mut out = Map::new();
            out.insert("selectedModel".into(), selected);
            out.insert(
                "modelOptions".into(),
                derive_effective_model_options(&input)?,
            );
            return Ok(Value::Object(out));
        }
    }

    let selected_draft_model = {
        let m = js_get(&active_selection, "model");
        if truthy(&m) {
            resolve_app_model_selection(&sp, &custom_models, &m)?
        } else {
            Value::Null
        }
    };
    let unlisted_draft_model = if str_eq(&sp, "pi") || str_eq(&sp, "omp") {
        selected_draft_model.clone()
    } else {
        Value::Null
    };

    let first_slug = {
        if is_nullish(&ambp) {
            js::undefined()
        } else {
            let ao = js_get(&ambp, &prop_key(&sp)?);
            if is_nullish(&ao) {
                js::undefined()
            } else {
                let first = js_get(&ao, "0");
                if is_nullish(&first) {
                    js::undefined()
                } else {
                    js_get(&first, "slug")
                }
            }
        }
    };

    let thread_model = if matches_sel(&thread_sel) {
        js_get(&thread_sel, "model")
    } else {
        Value::Null
    };
    let project_model = if matches_sel(&project_sel) {
        js_get(&project_sel, "model")
    } else {
        Value::Null
    };
    let chain: Vec<Value> = vec![
        resolve_available(&js_get(&active_selection, "model"))?.unwrap_or_else(js::undefined),
        resolve_available(&thread_model)?.unwrap_or_else(js::undefined),
        resolve_available(&project_model)?.unwrap_or_else(js::undefined),
        resolve_available(&selected_draft_model)?.unwrap_or_else(js::undefined),
        persisted_thread,
        persisted_project,
        unlisted_draft_model,
        first_slug,
        selected_draft_model,
        base_model.unwrap_or_else(js::undefined),
        Value::String("gpt-6-astra".into()),
    ];
    let selected_model = chain
        .iter()
        .find(|v| !is_nullish(v))
        .cloned()
        .unwrap_or_else(|| Value::String("gpt-6-astra".into()));

    let model_options = derive_effective_model_options(&input)?;

    let mut out = Map::new();
    out.insert("selectedModel".into(), selected_model);
    out.insert("modelOptions".into(), model_options);
    Ok(Value::Object(out))
}

// ── Antigravity legacy effort match ──────────────────────────────────────────

/// `rawModel.trim().match(/^(.*?)\s+\(([^()]+)\)$/u)` — first `\s+ (` group.
fn antigravity_legacy_match(u: &[u16]) -> Option<(Vec<u16>, Vec<u16>)> {
    if u.len() < 4 || u[u.len() - 1] != b')' as u16 {
        return None;
    }
    for i in 0..u.len() {
        if !is_es_ws(u[i]) {
            continue;
        }
        let mut j = i + 1;
        while j < u.len() && is_es_ws(u[j]) {
            j += 1;
        }
        if j + 1 < u.len() - 1
            && u[j] == b'(' as u16
            && !u[j + 1..u.len() - 1]
                .iter()
                .any(|&c| c == b'(' as u16 || c == b')' as u16)
        {
            return Some((u[..i].to_vec(), u[j + 1..u.len() - 1].to_vec()));
        }
    }
    None
}
