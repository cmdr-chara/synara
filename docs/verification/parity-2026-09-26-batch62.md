# Parity batch 62 — Review: `direct_models` is a Rust-only invention

Date: 2026-09-26
Upstream reference: `a33435c18` (v0.9.2)
Commit: see `devin/1790450414-upstream-parity-batch43` HEAD

## Scope

Review of the port's `direct_models` subsystem (queued work candidate) for an
upstream counterpart. No code changed — the review verdict + decomposition is
the increment.

## Findings

Upstream (`~/repos/synara`) has **no direct-provider model route**:

- `ProviderKind` (`contracts/orchestration.ts`) is agent CLIs only: codex,
  claudeAgent, cursor, antigravity, grok, droid, opencode, pi, devin, omp.
- `ModelSelection` is `{provider, model, options}` chosen inside each agent
  provider's surface; `customModels` (`contracts/settings.ts`) is a per-
  provider string list shown in `ModelsSettingsPanel` — no API keys, no
  endpoints, no direct HTTP calls.
- Text generation (commit messages, thread titles) runs through
  `settings.textGenerationModelSelection` routed to `GIT_TEXT_GENERATION_
  PROVIDERS` = `[codex, cursor, opencode, droid]` — agent CLIs, via
  `git/Layers/ProviderTextGeneration.ts`.
- `AutomationCompletionPolicy["ai-evaluated"]` is `{stopWhen,
  confidenceThreshold}` only — the evaluator runs on the definition's own
  `modelSelection` or `textGenerationModelSelection` (fallback codex). No
  `evaluator` binding field exists upstream.

The port invented a parallel stack (~3.5k lines):

- `synara-app`: `shell/direct_models.rs` (874) + `view.rs` (584) +
  `options.rs` (287) — bindings, favorites, API keys, models.dev catalog,
  provider telemetry, "Direct · provider/model · N prior turns" composer
  control, `cycle_direct_model_for_shortcut`, a "Direct provider endpoints"
  settings section, and a per-task `direct_route` that replaces ACP dispatch.
- `synara-workspace`: `direct_models.rs` (274), `controller/direct_models.rs`
  (1105), `storage/direct_models.rs` (117) — persisted bindings + catalog.
- `synara-server`: `providers.rs` (277) + `execution.rs` route dispatch on
  `direct_model_binding(id)` (lines 143, 237, 325).
- Load-bearing inventions: `AutomationCompletionPolicy::AiEvaluated
  .evaluator: DirectModelBinding` (upstream: no field), `uses_direct_model`
  gates in composer/controls/attachments/onboarding/transcript/handoff.

## Decision

The subsystem diverges from the 1:1 mandate but is too large to excise safely
in one batch. Staged plan (recorded in ROADMAP):

1. **Automation evaluator**: drop `evaluator: DirectModelBinding` from
   `AiEvaluated`; evaluate with the run's `modelSelection` (upstream shape:
   `stopWhen` + `confidenceThreshold` only). Needs a storage decode
   migration — persisted rows carry `evaluator`.
2. **Text generation**: if the port's git-message/title generation calls
   direct providers, reroute through the task's agent provider
   (`textGenerationModelSelection` equivalent).
3. **Direct chat route**: remove `direct_route` dispatch in
   `execution.rs`/`send_prompt`, the composer direct controls,
   `uses_direct_model` gates, and the cycle shortcut.
4. **Settings/catalog**: remove the "Direct provider endpoints" section,
   provider keys UI, models.dev catalog fetch/persist.
5. **Storage**: drop bindings/catalog tables via migration.

Each stage is its own tick with gates + live test.

## Gates

Docs-only change; fmt/clippy/tests unchanged from the batch-61 baseline
(fmt clean, clippy 25 warnings = HEAD baseline, 357 tests pass + 1 documented
env failure).

## Live test — real app

Not applicable — no behavior change. Review receipts for upstream reads are
the `~/repos/synara` paths cited above.

## Next tick candidates

- Stage 1: drop `evaluator` from `AiEvaluated` (storage migration + UI).
- Hub→Studio rename; env-panel inline Notes; auto-mode provider gating;
  "Loading" pre-takeover label; "Fast mode" extras row; terminal-count badge.
- Deferred live verification: batches 48/50/51, 49, 54, 55.
