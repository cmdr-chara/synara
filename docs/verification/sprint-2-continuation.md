# Sprint 2 reconciliation and integration evidence

Status: candidate, awaiting accumulated validation and remote integration.
No PR, release or branch deletion is authorized.

## Exact starting identities

- Original common product base: `fe515833c565d666204f24e5257d85025dede0e9`.
- Resumed session: `40db0fa941b40178eea767460636527edc231ce0`.
- Concurrent integration: `0d76c2e775c09147bec3cd3ba7b6bb42bfbfc174`.
- Protected main: `b58f27381e7ddd59678c9961500e8e43d3cc19ab`.

Both development histories independently implemented overlapping workflows.
The reconciliation uses the already-integrated feature-closure implementation
as the canonical product and records both histories as merge parents. It does
not install duplicate task controls or pretend the implementations are identical.

## Deliberate conflict resolution

| Surface | Retained product and rationale |
| --- | --- |
| Debug | Integration's evidence-gated lifecycle and startup/navigation readiness, already exercised with goals and shared conversation state |
| Persistent goals | Integration's durable goal owner, explicit arming, two-follow-up bound, human priority, pause/Stop and restart disarming |
| Thread recap | Integration's reviewed unsent related-task generation and explicitly cached result. Sprint 2's separate direct-model dialog is not installed alongside it |
| PR Fix | Integration's existing scoped review owner and composer flow, strengthened below to reject edits during an asynchronous recheck |
| Inline comments | Integration's task-owned durable multiple-comment queue and stale-file checks, not the competing single-comment UI |
| Releases | Integration's real build version, bundled development notes and local observation/read history. Production catalog/update configuration remains unavailable |
| Verification | Read-only regression CI adapted from sprint 2, targeting the canonical thirteen native journeys and complete workspace/app tests |

Superseded sprint-specific modules, storage shapes, UI documents and journey
scripts remain recoverable in the retained `40db0fa` ancestry. They are not
current APIs or acceptance evidence for the reconciled product. No conversion
of development-only alternate storage is claimed. The integrated durable store,
recovery whitelist and existing user data are not rewritten.

Product counts remain **14 Present / 25 Partial / 9 Missing**. Five formerly
missing capabilities are substantially present and Releases is partial, as
recorded in [the canonical roadmap](../../ROADMAP.md). This reconciliation does
not count duplicate implementations as additional completed features.

## PR Fix defect and correction

An explicit Add starts an asynchronous head/comment recheck. Previously the
completion handler fenced the task, navigation, composer text and close state,
but not the editable PR Fix instruction. Editing that instruction in flight
could therefore insert context generated from the older instruction.

Each recheck now captures the exact instruction. Completion requires that
snapshot still to match and refuses active input-method composition. A mismatch
preserves both user inputs, reports the refusal and requires another explicit
review/Add. Collection, cancellation, comment/head freshness, original transcript,
permission and no-automatic-Send behavior remain unchanged.

Four focused unit cases cover missing snapshots, text edits, composition and
exact multiline/Unicode text. The native PR Fix journey adds deterministic
response barriers for in-flight instruction edits and cancellation, plus a
changed-comment recheck. Barriers exist only in the owned gh fixture, have a
finite timeout and grant no live GitHub authority.

## Evidence before reconciliation

- Sprint 2: run `35756279611`, exact source `40db0fa`, artifact `10708777334`.
  433 Rust tests passed, one ignored; six native journeys passed 41 assertion
  groups. These totals describe that superseded product tree only.
- Canonical feature closure: run `35755789769`, published source `9637069`,
  checked tree `fbea34609ee1f452b238a0419a136e52ecf59cc7`.
  542 backend and 79 app tests passed, 21 backend tests ignored; thirteen
  native journeys passed 67 groups. Cleanup `0d76c2e` changes no retained
  product/build/test inputs. See [its full receipt](feature-closure-sprint.md).
- Neither earlier campaign is substituted for validation of this merge.

## Gates for the reconciled candidate

G1: Preserve compatible integration work, both histories and the original ledger.
STATE: OPEN pending exact merged-tree and ancestry checks.

G2: Fix the instruction-edit race without hidden execution or loss of user work.
STATE: OPEN pending focused unit/native checks against the published candidate.

G3: Run accumulated workspace/app, native and structural-baseline validation.
STATE: OPEN. Local Python parsing and whitespace checks are not native proof.

G4: Publish and non-force integrate, verify containment and unchanged main.
STATE: OPEN. No remote update is asserted by this draft.

## Baselines and acceptance boundaries

The original Browser `actions.js` structural finding is unchanged. Neither
Browser functionality nor the audit is weakened. All 120 historical A-Q task
bodies and checkbox states must remain byte-identical to the original base.

The concurrent integration already failed the full-workspace formatting gate
in run `35757286950` at `0d76c2e`, while its roadmap and packaging checks passed.
The existing static workflow stays unchanged. Native regression CI captures
formatting diagnostics separately, rather than misrepresenting that inherited
failure as a successful formatting check.

All native checks use isolated Linux/X11 and owned ACP/Git/HTTP fixtures. They
do not establish authenticated provider/GitHub, macOS/Windows/Wayland, hardware,
accessibility/IME or signed release/update acceptance. Unit composition checks
do not substitute for real platform IME acceptance. No release is approved.
