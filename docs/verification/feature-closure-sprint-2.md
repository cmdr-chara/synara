# Feature-closure sprint 2 verification receipt

Feature candidate: `20c78d7164e5e6e9be2b5bc1825be9babfeaa769`
Final tested combined candidate: `5fd0d8c4c9c43d30bafd567350e8d04d83417044`
Date: 2026-09-22

## Product classification

- Starting inventory: 14 Present / 25 Partial / 9 Missing.
- Current candidate: 18 Present / 26 Partial / 4 Missing.
- Missing -> Present: stacked pull requests, AppSnap, two-task split views, checkpoints/revert.
- Missing -> Partial: rich media in transcript.
- Partial -> Present: none.
- Rich media remains Partial because PDF/document viewing is not implemented.

Present refers to the explicitly supported user workflow, not exhaustive platform
or state coverage. In particular, checkpoints restore draft and notes/checklist
only. They do not restore files, Git, transcripts or provider sessions. See the
[workflow boundaries](../ui/feature-closure-sprint-2.md).

## Published feature commits and focused evidence

| Feature | Published commit | GitHub Actions run | Result |
| --- | --- | ---: | --- |
| Stacked pull requests | `d603b2c81475ad1713ae0d53c765353ebf48b093` | 35762961299 | 10 focused Rust tests and 6 native groups passed |
| Transcript images | `75760b3b98772e470b31354918793a6affb32bd7`, layout fix `8f83af33d91c07e07d4db5b6311e50fb667fb446` | 35766712749 | 3 core media tests and 6 native groups passed |
| AppSnap | `37a38f91da7d4a9f9ab848669652b72d49ad5218` | 35767806370 | 17 runtime/storage/process tests and 5 native groups passed |
| Two-task split views | `d186675a5f85bf6b06c38c70480e45a0f30a0bb2` | 35770961136 | 1 real ACP isolation test and 5 native groups passed |
| Checkpoints/revert | `20c78d7164e5e6e9be2b5bc1825be9babfeaa769` | 35772987553 | 7 storage/controller tests and 5 native groups passed |

These are 38 distinct focused Rust tests and 27 native assertion groups across
five journeys. Repeated integrated runs are not additional distinct tests.

## Earlier failed attempts retained

The initial split-view native driver used ambiguous composer geometry. A distinct
primary probe corrected the test target. Its next run observed resize geometry too
late. Fresh-render observation corrected that timing issue. Those failed attempts
and the original zero-match Rust filter are not passing evidence. The final split
run above used the actual ACP task-isolation regression and passed.

Checkpoint run `35772451891` passed six tests and failed permanent task deletion:
the new owned metadata key added a thirteenth SQL parameter, but the DELETE list
still contained twelve placeholders. Run `35772987553` passed all seven tests after
that correction, plus the native rollback/recovery journey. Stale-state and injected
partial-failure tests remain permanent regressions.

## Scope boundaries

Native evidence uses Linux/X11 and owned fixtures. AppSnap uses an actual selected
80x60 X11 window and the real installed capture helpers, not a fabricated screenshot.
This does not prove macOS, Windows, Wayland, live authenticated provider/GitHub,
PDF/document transcript viewing, accessibility, hardware or release acceptance.

Existing task/session, Git, filesystem and process ownership remains authoritative.
The checkpoint boundary is intentionally narrow: app-owned unsent draft and saved
notes/checklist only. Workspace files, Git/index, transcript, provider/session state,
approvals and attachments are not presented as reverted.

## Accumulated campaigns before final integration checks

The four-feature accumulated campaign passed in run `35771879974` and published
`68e6635cfbf98e1a44aeb8ae4bfb245b928e434e`. Checkpoints landed afterward.

The five-feature campaign passed in run `35773947442`, publishing the exact tested
tree as `977acbda229d680a674c05336ce697a6b494bd0c`. It passed 576 backend tests
with 21 ignored, 80 app tests, and all 27 native groups. These results precede the
merge with newer integration changes and do not replace the following combined run.

## Cleaned, merged-source validation

Run `35775258536` passed against commit
`5fd0d8c4c9c43d30bafd567350e8d04d83417044`, tree
`879297fc27b9794d6adcc04e9947c918526b0825`.

- Backend: `cargo +1.98.1 test --locked --workspace --exclude synara-app`, 576 passed,
  0 failed, 21 ignored.
- App: `cargo +1.98.1 test --locked -p synara-app`, 84 passed, 0 failed, 0 ignored.
- Native app and ACP fixture build: passed.
- Python structural/router/publisher/scope/integration regression suites: 57 tests
  passed. Roadmap self-tests: 9 passed.
- All 18 real GPUI journeys passed, covering 97 assertion groups.
- Source status remained clean and `git diff --exit-code` passed after all checks.

| Native journey | Passing assertion groups |
| --- | ---: |
| Debug | 5 |
| Goals | 5 |
| Recap | 4 |
| PR Fix | 7 |
| Inline comments | 4 |
| Releases | 3 |
| Chat behavior | 5 |
| Model drafts | 7 |
| Provider continuation | 6 |
| Project Import | 6 |
| Direct models | 7 |
| Integrations | 6 |
| Browser WebView | 5 |
| Stacked PRs | 6 |
| Transcript media | 6 |
| AppSnap | 5 |
| Two-task split | 5 |
| Checkpoints | 5 |

Artifact: `native-conversation-evidence`, ID `10716301303`.
Downloaded archive SHA-256:
`ee3150bc2639fd5bbd0637be51bfbe1abeae83c24f00f5fefbc843e57d8d9105`.
It contains the exact source bundle, logs, native result files and screenshots.

## Baseline failures and residual risk

The structural audit still reports
`crates/synara-browser/src/native/actions.js: non-Rust core source`.
Baseline and candidate reports have the same error, with no structural regression.
The Browser code and validator were not removed or weakened.

Full-workspace formatting still exits 1. Its affected-path set matches the preserved
integration-baseline report. This is a path comparison, not a claim that every
formatting hunk is unchanged. The existing full-workspace formatting gate remains
in place. Eight newly affected sprint paths were separately formatted after merge.

The native build retains two existing warnings: `RevisionState::pending` and
`ROW_HEIGHT` are unused. No claim of a warning-free or fully green release is made.
Platform, real-account, broader rollback and PDF/document acceptance remain open.

## Integration and source integrity

Original integration and initial session head:
`0d76c2e775c09147bec3cd3ba7b6bb42bfbfc174`.
Session branch: `astra/feature-closure-sprint-2`.

The cleaned sprint was integrated by two-parent merge
`1cc4a672d83276d5dba1a6edc1c9e5ee243a709d`, with parents
`7ac652c54372713443ba8292fe87d506f6bf5332` and
`e23ff10653560f8c6c505ca3a40bbaa5d451cd6e`. This preserves the newer PR Fix
instruction/IME/stale-response safeguards and their permanent tests.

Eight sprint-touched Rust paths were formatted in
`a9ea461d98e9cd69c78e3d40fc701039886e8bb7`; the formatter removed its own
workflow. Session transfer parts, request files and write helpers are absent from
the final tested tree. The permanent replacement CI is read-only and retains all
18 journeys. The original 120 detailed A-Q task bodies, checkbox states and historical
evidence are preserved; this was checked both in CI and against the original head.

A documentation-only successor to the final tested commit records this receipt.
Final branch/ref containment and unchanged main are re-read when that successor is
integrated non-force. No release or pull request is authorized, and the session
branch is retained rather than deleted automatically.

## Remaining genuinely Missing development

The next dependency-aware order is native subagents/workflows, Agent Gateway,
external MCP clients connecting to Synara, then Computer Use. These are unimplemented
features, not claims that GitHub writes or native compilation are unavailable.
