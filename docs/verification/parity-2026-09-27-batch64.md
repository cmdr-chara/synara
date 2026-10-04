# Parity batch 64 — Direct-models excision (stages 2–5, wholesale)

Date: 2026-09-27
Upstream reference: `a33435c18` (v0.9.2)
Commit: see `devin/1790450414-upstream-parity-batch43` HEAD

## Scope

Completes the excision plan recorded in the batch-62 review. Stage 1
(batch 63) dropped the evaluator binding; this batch removes the entire
remaining direct-model feature: the direct chat route, the settings
section, the models.dev catalog persistence, and the `synara-model` crate
itself. Upstream has no direct-provider surface at all — `ProviderKind` is
agent CLIs only and all text generation goes through agent providers.

The planned "stage 2 reroute" collapsed: once the direct chat route was
removed, nothing else consumed `synara-model` (`rg` proved the crate had
no non-feature callers), so rerouting text generation was moot — the
whole crate is deleted instead of ported.

## Changes

### Deleted

- `crates/synara-model/` — the entire crate (providers, catalog,
  discovery, protocol, transport, live-provider test).
- `crates/synara-app/src/shell/direct_models.rs` +
  `direct_models/{view,options}.rs` — the settings section UI and the
  composer route controls.
- `crates/synara-workspace/src/direct_models.rs` — `DirectModelBinding`,
  `ProviderSettings` persistence, `suggested_direct_history_turns`,
  catalog snapshot methods.
- `crates/synara-workspace/src/storage/direct_models.rs` —
  `Store::save_direct_model_settings`.
- `crates/synara-workspace/src/controller/direct_models.rs` — renamed to
  `controller/evaluation.rs`; now contains only `EvaluationEvents` +
  `evaluate_automation_completion` + tests. `save_direct_model_settings`,
  `direct_model_key`, `direct_provider_telemetry`, `discover_direct_models`
  and `model_error` are gone.

### Cut points

- `shell.rs`: `mod direct_models`, `Update::DirectModels`, the
  `direct_models` state field, its close-guard conjunct,
  `load_direct_binding` on task select, and the `direct_route_loading`
  send-prompt early return.
- `composer.rs`: the route branch is gone; the composer always renders
  `session_controls`.
- `controls.rs`: model cycling and the cycle shortcut no longer branch on
  `uses_direct_model`.
- `attachments.rs`: the direct-route capability-error branch removed.
- `onboarding/auth.rs`: agent-access task filter no longer excludes
  direct-model tasks.
- `settings.rs` + `settings/navigation.rs`: `Section::DirectModels` and
  its nav item removed (the section enum is in-memory, never serialized);
  the navigation-count test now expects 22 sections.
- `composer/commands.rs`: the `direct-models` settings alias removed.
- `handoff.rs` (app + workspace): `HandoffTarget::Direct` deleted; the
  targets reply carries `Vec<AgentProfile>` only; placeholders and hints
  no longer mention saved models.
- `synara-server`: `execution.rs`/`providers.rs` lose the direct dispatch
  (`run_task` signature drops `expected_route`; test call sites updated);
  `index.html` landing copy no longer advertises direct-model tasks.
- `storage/workflows.rs`: the reviewed-child gate no longer consults the
  `task-direct-model:` preference (no producer remains; a stale key must
  not block a workflow attempt).
- `automations/view.rs`: heartbeat-target explainer no longer mentions a
  direct-model route.
- `Cargo.toml` workspace members + the `synara-model` dep edges in
  `synara-app` and `synara-workspace`; `Cargo.lock` regenerated.

### Deliberately retained (decode-compat)

Following the `hub_revision`/`evaluator` shim precedent, stored-data
decoders stay so existing databases and backups keep opening:

- `ThreadEvent::DirectModelRoute` and `turn.direct_provider_id` /
  `direct_model_id` in `synara-core` (plus its render arm in
  `profile_activity.rs` and the `activity.rs` read path) — old
  transcripts still decode and render.
- `task-direct-model:*` remains a valid preference key and is still
  cleaned up on task delete; `direct-model-providers-v1` and
  `direct-model-catalog-v1` remain valid keys.
- `storage/recovery.rs` decodes those legacy keys as
  `serde_json::Value` (typed structures are gone) with a comment noting
  they exist only so old backups restore.

`ModelFavorite`/`model_favorites`/`SessionModelPreset` are kept: they are
shared with the ACP agent model menus (`ui/menu/models.rs`,
`controls.rs`); only the `direct-model:` favorite prefix died with the
shell module. `Glyph::Brain` stays for `Section::Models` navigation.

## Gates

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --locked --workspace --all-targets`: 16 warnings — nine
  fewer than the 25-warning HEAD baseline (the cut deleted warning
  sites); all remaining warnings are pre-existing in untouched files.
- `cargo test --locked --workspace`: pass except
  `pull_requests::tests::discovery_does_not_change_worktree_or_index`
  (known environment failure — gitconfig remote rewrite). No
  `integrations::probe` flake this run.

## Live test

Release binary, `target/release/synara-app --data-dir
/tmp/synara-e2e/data-eval` (a populated data dir from earlier runs —
tasks, automations, prefs — i.e. an "old database" that may carry
`task-direct-model:` keys; it opened with no error dialog or crash).

Acceptance gestures:

1. Settings navigation contains no "Direct models" entry (Integrations
   now shows only AppSnap, Computer use, MCP connections). Observed and
   screenshotted.
2. The composer renders agent controls only; the model picker is the ACP
   agent surface ("OpenCode" + "Connect and load models") — no
   direct-model routing. Observed and screenshotted.
3. A real chat turn still works end to end: a new thread "Reply briefly
   and stop." streamed the stub agent's reply
   (`{"stopMatched":true,"confidence":0.95,...}`) in ~1 ms — the ACP
   route is the only route and it works. Observed and screenshotted.

## Upstream divergence deliberately kept

- None new. The port now has zero direct-provider surface, matching
  upstream.

## What remains divergent (queued)

- Hub→Studio rename, env-panel inline Notes, auto-mode provider gating,
  "Loading" pre-takeover label, "Fast mode" extras row, terminal-count
  avatar badge.
- Live-verify deferred: batches 48/50/51 (ACP permissions), 49
  (AppSnap), 54, 55.
