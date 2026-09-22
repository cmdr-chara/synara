# Sprint 2 continuation and integration receipt

Scope: Debug, PR Fix, inline file comments, What's New and Thread recap.
This receipt is feature-development evidence, not production release approval.
No PR or release is authorized.

## Starting identities

- Original integration/session base: `fe515833c565d666204f24e5257d85025dede0e9`.
- Initial recovered Debug head: `e8e70405851ff2999de2d435fc05e661577effa0`.
- Protected main: `b58f27381e7ddd59678c9961500e8e43d3cc19ab`.
- Electron reference checked: `f04341a67bc4941d1b2e91e0b23bbe782dfbc727`.
- The older local handoff at `321a88867024841585947e3de8a7fbf5855c2161`
  was not used to replace newer accepted Debug source.

## Accepted checkpoints

| Workflow | Accepted source | Focused/native evidence |
| --- | --- | --- |
| Debug | `e8e70405851ff2999de2d435fc05e661577effa0` | Run 35730624723, artifact 10695017984; 8 focused tests and 9 native checks |
| PR Fix | `2fafd137e9c3b9533f3850d76d8be94767521b70` | Included in the later combined native regression; preserved task/head/comment fencing |
| Inline comments | `ff96c319a9a54097e9fd7d6b6ae6e730c793ca54` | Run 35746609285, artifact 10704085272; 9 focused tests and Debug/PR Fix/inline native checks 9/8/6 |
| What's New (partial) | `00cf1495f6ae0ac8cbf1f95b26b968896f7c01e4` | Run 35748823515, artifact 10703968681; 7 Releases, 4 updater and 12 recovery tests; 4 native checks |
| Thread recap | `d74ca33fc9a9b3f5fa8ab897b1a2c283a04f8554` | Run 35755112799, artifact 10708280373; 9 recap/output and 12 recovery tests; recap/direct-model native checks 7/7 |

Counts in a preceding row are that run's selected tests, not additive unique
test totals. The final accumulated run must verify these workflows together
before the candidate can be integrated.

## Diagnosed failures retained in history

- The old compressed PR Fix transport failed its CRC check before compilation.
  Subsequent checksum-identical repair and accepted source supersede that run.
- The inline-comment journey read an X11 clipboard selection immediately while
  clearing a field. The test now waits for the exact sentinel before deleting it,
  retaining the original empty-field and no-send assertions.
- What's New initially failed the exact preference-key whitelist. Its key is
  explicitly registered and recovery validates the saved journal. The existing
  whitelist was not replaced with a broad permissive rule.
- The first recap journey stalled before opening recap. Its setup now observes
  durable source text and the rendered enabled native Send state.
- An added recap fixture assertion initially compared a `(task, events,
  session-count)` helper result with a list. It now checks the actual event and
  session components.
- The new Send-state layout probe initially lacked the `gpui::canvas` qualifier.
  This was a compiler error in the candidate, not a baseline failure.
- The recap model row was visible but did not emit the slot requested by the
  native journey. Numbered row probes now describe the actual rendered choices.
- The recap HTTP fixture initially asserted a string body rather than the
  existing normalized text-content-block shape. It now checks the exact one-text
  block request before inspecting the reviewed source. The transport is unchanged.
- The recap cache was generated and persisted correctly, but Copy sat below
  the dialog's clipped scroll area. Copy now lives in the wrapping persistent
  footer, alongside Close/Reload/Stop, instead of relying on offscreen geometry.
- No failing command in these runs is represented as successful validation.

## Ownership and unsupported scope

Debug verification is user-recorded evidence, not independent certification.
PR Fix does not resolve comments, create/merge a PR, mutate a checkout or send
a prompt. Inline comments attach a freshness-checked snapshot, not a live anchor
or hidden Send-time file refresh. A cached recap is model-generated assistance,
not authoritative transcript history, session migration or rollback.

What's New is deliberately partial. It displays bundled development notes and
local first-observed build history. No verified published catalog, production
endpoint, signing authority or platform replacement helper is configured.
No update request, installation or release publication was performed.

Native evidence uses isolated Linux/X11, owned Git/ACP fixtures and loopback
HTTP. It does not establish macOS/Windows/Wayland, authenticated GitHub/provider,
production credential-store, real hardware or accessibility/IME acceptance.

## Baseline and cleanup gates

The original A-Q roadmap ledger is preserved byte-for-byte. The structural
audit still reports the same existing Browser `actions.js` finding. The checker
and Browser functionality remain unchanged. Dependency manifests and Cargo.lock
are unchanged. The known unused `RevisionState::pending` and `ui::ROW_HEIGHT`
warnings are not attributed to these features.

The candidate removes `.synara-sprint-2.json` and replaces the
temporary source publisher with read-only regression CI. That CI builds/tests
the actual committed source. It neither decodes/applies patches nor pushes commits.

## Completion ledger

G1: Preserve accepted source and compatible concurrent work.
STATE: OPEN until final ancestry and diff review.

G2: Complete PR Fix and inline comments with native ownership/recovery checks.
STATE: PASS at the accepted checkpoints above, pending final accumulated rerun.

G3: Complete the recap workflow and advance Releases honestly.
STATE: PASS at the accepted recap checkpoint above, pending accumulated rerun. Releases remains partial.

G4: Accumulated regression, cleanup, non-force integration, containment and main.
STATE: OPEN. No integration is asserted by this draft.
