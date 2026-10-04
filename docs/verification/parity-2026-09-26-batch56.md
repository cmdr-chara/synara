# Parity batch 56 — Plan interaction mode (upstream `ProviderInteractionMode`)

Date: 2026-09-26
Upstream reference: `a33435c18` (v0.9.2)
Commit: see `devin/1790450414-upstream-parity-batch43` HEAD

## Scope

Ports upstream's third interaction mode and retires a port-side divergence:

- **`InteractionMode::Plan`** — upstream `ProviderInteractionMode =
  ["default","plan","debug"]` (orchestration.ts). The port had `{Default,
  Debug}`; added `Plan` (serde `"plan"`, label `Plan`).
- **`PLAN_MODE_PROMPT_PREFIX`** — verbatim copy of upstream
  `PROVIDER_PLAN_MODE_PROMPT_PREFIX` (apps/server/src/provider/planMode.ts):
  the "Synara plan mode is active …" block instructing the agent to wrap the
  final plan in `<proposed_plan>` tags.
- **`with_plan_prompt` / `with_interaction_prompt`** — upstream
  `withProviderPlanModePrompt`: `{PREFIX}\n\nUser request:\n{text.trim()}` when
  non-empty, just the prefix when empty (no idempotency check, unlike the debug
  shim which carries one). `with_interaction_prompt` dispatches by mode; the
  controller send path now routes every provider-bound prompt through it.
- **`/plan` rewired to upstream semantics** — the port's `/plan` previously
  invoked `native_plan_mode`, which picked an advertised *ACP session mode*
  (upstream does not do this for `/plan`). Upstream `/plan` sets the thread's
  interaction mode. `native_plan_mode` deleted; `/synara/plan`,
  `/synara/debug`, `/synara/default` all funnel through
  `set_interaction_mode`. `/synara/default` still also restores the provider
  session mode to "default" when the session advertises one (upstream
  behaviour). Slash descriptions now upstream-verbatim: "Switch this thread
  into plan mode" / "…evidence-first debug mode" / "…back to normal chat mode".
- **Composer extras "Plan mode" row** — was the ACP session-mode pick; now
  `ControlAction::PlanMode` toggling Plan↔Default with upstream detail text
  "Turn plan mode on/off" and a checkmark while active. "Debug mode" row
  toggles Debug↔Default the same way.
- **Mode badge** — `debug_compact` generalized to `mode_compact`: renders
  `mode.label()` ("Plan" / "Debug") on the workflow strip, click returns to
  Default (upstream composer badge: click → normal build mode).
- Shell plumbing: `Update::DebugMode { debug }` → `Update::InteractionMode
  { mode }`; `debug_tasks: HashSet` → `mode_tasks: HashMap<TaskId,
  InteractionMode>`.

ACP session modes remain reachable via the Mode picker (`ControlKind::Mode`),
unchanged.

## Live test — real app

`cargo run --release -p synara-app --data-dir /tmp/synara-e2e/data` with the
`opencode` profile pointed at `/tmp/synara-e2e/stub-agent.py` (real ACP stub:
handshakes, emits one `agent_message_chunk`, never resolves `session/prompt`;
also writes the received prompt params to `last-prompt.txt`).

- Extras "+" panel shows **Plan mode · Turn plan mode on** and **Debug mode ·
  Turn debug mode on** (upstream `toggleSecondary` detail text).
- Toggling the row ON wrote `task-interaction-mode:{task}` = `"plan"` to
  `native-workspace.sqlite3` and rendered the "Plan" chip on the workflow
  strip (screenshots `ss_8ea8b33c`, `ss_231d035f`).
- Sending "plan mode probe" produced `last-prompt.txt` containing the verbatim
  upstream prefix and `"User request:\n…"` — the shim reaches the wire
  (screenshot `ss_6b55adb0` shows the prefixed text rendered as the sent
  user message, matching upstream where the shimmed prompt is what ships).
- Clicking the "Plan" chip cleared it and the pref flipped to `"default"`
  (verified via sqlite after the click landed once the busy turn settled).
- `/plan` without the `/synara/` prefix is sent to the provider as literal
  text — correct: the port reserves bare slash commands for
  provider-advertised commands; native commands live under `/synara/`.
- The stub never acks cancellation, so the turn stayed in-flight across
  cancels ("agent did not acknowledge cancellation" notice observed);
  unrelated to this batch — the mode is a send-time prompt shim, not
  session state.

## Gates

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --locked --workspace --all-targets`: no new warnings.
- `cargo test --locked --workspace`: green except the documented env-only
  `pull_requests::tests::discovery_does_not_change_worktree_or_index` failure
  (gitconfig remote rewrite). New unit coverage:
  `plan_prompt_prefix_matches_upstream_contract`,
  `plan_mode_round_trips_per_task`.

## Next tick candidates

- Plan Ready leg: `<proposed_plan>` extraction → `thread.proposedPlans` →
  "Plan ready" composer banner + `PLEASE IMPLEMENT THIS PLAN:` submit.
- Terminal-count avatar badge (needs per-thread terminal scope).
- Upstream `Loading` pre-takeover label (needs send-busy vs turn-busy split).
