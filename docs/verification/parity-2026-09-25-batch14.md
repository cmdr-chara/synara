# Parity batch 14: project-search rank semantics and token activity

Native implementation commit:
`6234bbc562af1336bb7a04710471a0f8871f370e`, based on
`df6016c6ddc5641b7a4abbbe136fc853e6f35268`. The upstream comparison point
remains `Emanuele-web04/synara@eaa61eded31b6755d4f30ba8eabc5d905cf817cb`.

## S13: exact project-search ranking semantics

Native entry-name search keeps the existing upstream score tiers and
score/depth/path tie-break, but now also applies the upstream entry-query
normalization before ranking: leading mention and relative-path noise
(`@`, `.`, `/`) is stripped after trimming and the query is lowercased.
Workspace paths are normalized to POSIX separators before path scoring so the
same path tiers apply on Windows. Empty-after-normalization queries retain the
native bounded-search rejection rather than turning into an unbounded browse.

The focused regression test compares prefixed/case-varied queries with their
canonical equivalents and checks POSIX path normalization. S14 remains separate:
the native ignored/generated-file policy is deliberately not changed by this
batch.

## S16: token activity heatmap

Profile activity now accumulates durable per-turn input + output token counts by
UTC day when both values were actually reported while that turn was active. The
274-day heatmap selects that token series whenever positive token telemetry
exists and otherwise falls back to persisted turn starts. Token cells state that
turns without token telemetry are omitted. No provider identity, missing token
count, account quota, billing amount or historical usage is inferred.

The focused selector test proves the prompt/turn fallback and the switch to the
token series once reported tokens exist. Existing heatmap bucketing, Sunday-first
calendar layout and UTC boundary semantics are reused unchanged. D11 stays open
for provider/model mix, account/quota telemetry and its broader integration
journeys.

## Focused verification

Post-commit connector inspection confirmed the search normalization helper,
POSIX path scorer, token-day state, per-turn usage read, token/prompt selector
and both new regression tests are each present exactly once; obsolete search and
heatmap call sites are absent and no conflict markers were introduced. The
implementation commit changes only three Rust files (`+120/-23`).

No automated status was attached to the implementation commit. The available
native workflow is `workflow_call`-only, and this environment has neither a
checked-out repository nor a Rust toolchain, so the new Rust tests and formatter
were not executed here. This receipt does not claim a cargo/CI pass.
