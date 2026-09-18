# Backend native acceptance checkpoint

Source candidate: `602acf819a1075aa8d0befed08cbad3f9a994fe2`.
Source tree: `1f16661bcb9a873709c21d7a546e379d0ea392e4`.

This checkpoint includes the Git stash/porcelain corrections, typed Git live-SSH
coverage, owned cancellation and credential-safe errors in the existing
`GitService`, and a portable ACP crash test. It preserves the concurrent UI
reference commit `9cd76ac07c0161362e03f90c9c83e0ffaaeccede` unchanged.

## Observed evidence

- Isolated local Linux, exact Rust 1.98.1 and locked offline dependencies:
  backend check, strict Clippy, formatting and all backend tests pass.
  Aggregate: 275 passed, 0 failed and 19 intentionally ignored test entries.
  Ignored entries include externally opted-in SSH/vendor tests and the native
  test-child entry that the process-lifecycle tests invoke themselves.
- Workspace structure, 12 audit regressions, 6 publisher regressions and
  9 roadmap regressions pass. The independent root remains
  `43b1fb89bf19dadc388d18008f9ceb21b8215716`.
- Controlled SSH run
  [35386247911](https://github.com/cmdr-chara/synara/actions/runs/35386247911)
  passed at `12a85fc5a38be7e03bffdffb3d6012c2ff05c38c`, including the three new
  typed Git remote journeys. This is earlier-source evidence, not a claim that
  the new lifecycle checkpoint has already passed SSH.
- At that earlier revision, backend tests passed on native Linux x64,
  macOS arm64 and Windows x64. macOS/Windows strict Clippy exposed an unused
  crash-fixture variant. The new portable crash regression exercises that
  variant rather than suppressing the warning.

## Pending qualification

The GitHub native, SSH and three-platform backend runs triggered by this evidence
commit must be checked against their actual checkout revision. A source-package
transport commit does not automatically trigger every downstream workflow.
No final native-platform pass is claimed here until those runs are observed.
macOS/Windows application compilation is not GUI, keychain, ConPTY or installer
interaction acceptance. No vendor authentication, browser embedding, device
support, updater or full B-P completion is implied. Q and releases remain excluded.

See [Git operation evidence](git-operations.md) for public APIs, deliberate
operation policies, failure-path tests, cancellation limits and privacy boundaries.
