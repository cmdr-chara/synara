# Feature-closure sprint 2 verification receipt

Candidate: `20c78d7164e5e6e9be2b5bc1825be9babfeaa769`
Date: 2026-09-22

## Product classification

- Starting inventory: 14 Present / 25 Partial / 9 Missing.
- Current candidate: 18 Present / 26 Partial / 4 Missing.
- Missing -> Present: stacked pull requests, AppSnap, two-task split views, checkpoints/revert.
- Missing -> Partial: rich media in transcript.
- Partial -> Present: none.
- Rich media remains Partial because PDF/document viewing is not implemented.

## Published feature commits and focused evidence

| Feature | Published commit | GitHub Actions run | Result |
| --- | --- | ---: | --- |
| Stacked pull requests | `d603b2c81475ad1713ae0d53c765353ebf48b093` | 35762961299 | focused Rust + native GPUI journey passed |
| Transcript images | `75760b3b98772e470b31354918793a6affb32bd7`, layout fix `8f83af33d91c07e07d4db5b6311e50fb667fb446` | 35766712749 | native image persistence/layout journey passed |
| AppSnap | `37a38f91da7d4a9f9ab848669652b72d49ad5218` | 35767806370 | focused runtime/storage + native AppSnap journey passed |
| Two-task split views | `d186675a5f85bf6b06c38c70480e45a0f30a0bb2` | 35770961136 | focused ACP isolation + native split-view journey passed |
| Checkpoints/revert | `20c78d7164e5e6e9be2b5bc1825be9babfeaa769` | 35772987553 | seven focused storage/controller tests + native bounded rollback journey passed |

The repository structural audit continued to report the known baseline
`crates/synara-browser/src/native/actions.js: non-Rust core source`. It was allowed
as pre-existing evidence and was not removed or hidden.

## Scope boundaries

The evidence above is Linux/X11/owned-fixture evidence where the native journey uses
those facilities. It does not claim macOS, Windows or Wayland acceptance, live
authenticated provider/GitHub acceptance, PDF/document transcript viewing, or final
release readiness. Existing task/session, Git, filesystem and process ownership
remain authoritative.

The original 120 A-Q task bodies and checkbox states are intentionally unchanged.

## Final accumulated validation

The prior four-feature accumulated campaign passed in GitHub Actions run
`35771879974` and published `68e6635cfbf98e1a44aeb8ae4bfb245b928e434e`.
Checkpoints/revert landed afterward. The final five-feature accumulated campaign
passed in GitHub Actions run `35773947442` and published the exact tested tree as
`977acbda229d680a674c05336ce697a6b494bd0c`. It re-ran stacked PRs, transcript
media, AppSnap, two-task split and checkpoints together with integrated backend/app,
roadmap-structure and helper checks.

The checkpoint boundary is intentionally narrow: it restores only app-owned unsent
draft and saved notes/checklist. Workspace files, Git/index, transcript,
provider/session state, approvals and attachments are not presented as reverted.
