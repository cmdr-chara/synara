# Parity batch 63 — Stage 1 of the `direct_models` excision

Date: 2026-09-26
Upstream reference: `a33435c18` (v0.9.2)
Commit: see `devin/1790450414-upstream-parity-batch43` HEAD

## Scope

Stage 1 of the excision plan recorded in the batch-62 review: drop the
Rust-only `evaluator: DirectModelBinding` field from
`AutomationCompletionPolicy::AiEvaluated` so it matches upstream's
`{type:"ai-evaluated", stopWhen, confidenceThreshold}` shape, and reroute
evaluation through the run's own agent provider — upstream evaluates via a
separate `ProviderTextGeneration` call (`AutomationService.ts:625-637`),
never through the visible thread.

## Changes

- `crates/synara-workspace/src/automations.rs`: `AiEvaluated` is now
  `{stop_when, confidence_threshold}` plus a
  `#[serde(default, skip_serializing)] evaluator: Option<serde_json::Value>`
  shim so previously persisted policies still decode (the
  `AutomationRun.hub_revision` precedent). `validate()` keeps the
  `stop_when`/threshold bounds and drops the evaluator selection checks.
- `crates/synara-workspace/src/storage/automations.rs`: the
  `validate_completion_policy` cross-check (which resolved the binding
  against `ProviderSettings`) is deleted; `DirectModelBinding` remains only
  for the still-live `task-direct-model:` preference decode.
- `crates/synara-workspace/src/controller/direct_models.rs`:
  `evaluate_automation_completion` now opens a throwaway ACP session on the
  run's agent profile (`definition.agent_id`) instead of calling a direct
  HTTP model. Its `ConnectionContext.events` is an `EvaluationEvents` sink:
  nothing reaches storage or the transcript, but assistant `TextDelta`
  chunks are kept because `session.prompt()` resolves with only the stop
  reason — the reply text streams through the sink. 30s timeout and 16KiB
  reply cap preserved. The stored `completion_evaluation` records
  `stop_matched/confidence/reason` and `policy_applied`.
- `crates/synara-app/src/shell/automations.rs` + `view.rs`: the editor's
  completion section loses the evaluator model picker and the
  `direct_models: ProviderSettings` load; "AI stop check" is a plain toggle
  matching upstream's policy shape. Explainer text updated to describe the
  agent-run evaluation.
- Tests: the HTTP-fixture eval test became a `ReplyBackend`/`ReplySession`
  fixture that emits the eval JSON through `session/update` chunks (real
  ACP semantics) and asserts the run agent — not a side channel — produced
  it; a second fixture site uses `evaluator: None`.

## Defect found and fixed during live test

First live run failed the stop check with "evaluator returned invalid
structured output": `DroppedEvents` swallowed the assistant reply along
with everything else, because `AgentSession::prompt()` returns the
`stopReason` string, not reply text. Fixed by capturing `Role::Assistant`
`TextDelta` events into the sink (`EvaluationEvents`) — verified by the
second live run below and by the updated unit test.

## Gates

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --locked --workspace --all-targets`: 25 warnings — the
  established HEAD baseline; none new.
- `cargo test --locked --workspace`: pass except
  `pull_requests::tests::discovery_does_not_change_worktree_or_index`
  (known environment failure — gitconfig remote rewrite) and three
  `integrations::probe` flakes that pass in isolation on both HEAD and this
  diff (timing under full parallel load).

## Live test

Release binary, `target/release/synara-app --data-dir
/tmp/synara-e2e/data-eval` (agent profile repointed to
`stub-agent-eval.py`, an ACP stub whose every `session/prompt` streams a
fixed `{stopMatched:true, confidence:0.95, reason:"stop condition
satisfied"}` chunk).

Acceptance gestures:

1. The automation editor's completion row offers only "No AI stop check" /
   "AI stop check" — no evaluator model picker — matching upstream's
   three-field policy. Observed in the New-automation editor; screenshot
   captured.
2. An enabled `ai_evaluated` automation runs, the evaluator replies, and
   the automation disables itself when `stopMatched` clears the threshold.
   Observed: "Eval stop test" (every 1m, threshold 0.8) ran; its ledger row
   records `completion_evaluation = {stop_matched:true, confidence:0.95,
   reason:"stop condition satisfied", policy_applied:true, failed:false}`
   and the definition flipped to `enabled:false`, revision 3 — the pane
   now shows the automation "Paused".
3. The evaluation prompt never reaches the visible transcript. The owned
   task's chat shows only the run's own prompt ("Reply briefly and stop.")
   and the run output — no evaluation text.

## Upstream divergence deliberately kept

- Upstream picks the evaluator model from
  `definition.modelSelection ?? textGenerationModelSelection ?? "codex"`;
  the port has no model-selection-inside-agent-profile field, so it uses
  the definition's `agent_id` profile as-is (same provider family as the
  run — the upstream intent).
- `evaluation_prompt` mirrors upstream's quoted-data framing; the exact
  upstream template lives in `ProviderTextGeneration.ts` and is matched in
  substance, not byte-for-byte.

## Next stages (queued)

2. Reroute text generation through the agent provider.
3. Remove the direct chat route + composer controls + `uses_direct_model`
   gates.
4. Remove the settings section + catalog.
5. Drop the storage tables.
