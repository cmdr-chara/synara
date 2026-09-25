# Parity batch 26: automation completion policy

Implementation commits:
- `035456c187041133e0a74a2668fefd559acc0817` — AI-evaluated completion policy, evaluator, scheduler integration and UI;
- `e638e044b2d2629a024fdcea16da3d9c1c76e975` — completion-policy module import fix;
- `2df6dd9f55f8939b3bd8f05c59c0c731343f333e` — focused completion-test import fix.

## S06 complete

The remaining pinned-upstream automation semantic is now implemented without
borrowing hidden authority from the automation's ACP agent.

A saved automation can select **No AI stop check** or an explicitly reviewed
direct-model evaluator. Only models that advertise structured-output support are
offered by the native editor. The definition stores:
- a bounded stop condition;
- a 0–1 confidence threshold;
- a direct-model selection with history disabled;
- the reviewed provider-profile digest.

Save-time validation re-resolves that digest against current direct-model
settings and validates the strict JSON-schema request before the definition can
be persisted.

## Evaluation boundary

A stop check runs only after a successful automation run and outside the owned
automation conversation. It receives:
- the stop condition;
- automation name/instructions;
- the exact run prompt snapshot;
- assistant output generated after that run began.

It receives no ACP session state, files, approvals, attachments, tool authority
or hidden reasoning. The model request has no tools, uses a strict
`automation_completion` JSON schema and is bounded to a 30-second request with
no automatic retry. Synara independently decodes the returned object and bounds
confidence/reason even when the provider claims schema compliance.

Run output collection is scoped to messages created after the pre-run message
baseline. If that baseline cannot be read, Synara uses the generic completion
summary rather than leaking older Heartbeat/Dedicated assistant text into the
stop evaluator.

## Failure and stale-result semantics

Stop-check failure, timeout, malformed output, provider/profile drift or a tool
proposal never converts the successful run into a failed run. The failed check is
retained as bounded completion-evaluation metadata and leaves the definition
enabled.

Before spending provider work, the scheduler checks that the saved definition is
still enabled at the exact run revision/policy. The atomic result recorder repeats
that check after the provider returns. Only a positive result at or above the
saved confidence threshold can disable the definition, and only when that exact
policy revision remains current. A user edit/re-enable that races the evaluator
therefore makes the result observational only.

The recorder is idempotent: a run accepts at most one completion evaluation.

## Focused verification

Source-level audit confirms:
- legacy definitions default to no completion policy and legacy runs default to
  no completion evaluation;
- every native definition/run constructor initializes the new persisted fields;
- evaluator bindings are reviewed against the current provider digest;
- the request is tool-free and strict-JSON;
- stale revision/policy checks exist before and after evaluation;
- successful-run assistant output is scoped to the current run;
- no conflict markers or unrelated dependency changes were introduced.

Focused regressions cover current-policy disable, stale-policy non-disable and a
local direct-model transport that asserts the evaluator request contains JSON
schema output and no tools.

No workflow status is attached to these commits, and this environment exposes no
Rust toolchain/checkout. This receipt therefore does not claim a fresh cargo,
rustfmt or Clippy pass. S06 is complete as a product feature. D8 remains OPEN for
live provider, restart/shutdown, native interaction and multi-platform acceptance.
