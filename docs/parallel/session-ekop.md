# EKOP session: delivery contract

## Scope and source identity

Work branch: `astra/session-ekop`.
Common base: `1cd24dd6f5ac9571c1ea2b7329bcb5fc1a4ad121`.
Expected independent root: `43b1fb89bf19dadc388d18008f9ceb21b8215716`.

The user authorized this isolated session for E (registry), K (browser host
foundation), O/P (measurement and platform verification groundwork), the shared
Zen/Synaric specification, and integration preparation. This permission does not
include merging session branches, changing the default branch, releasing packages,
or writing `main`, the archive, or `astra/gpui-clean-rewrite`.

## Confirmed product decisions

- Synara is a native Rust/GPUI graphical coding-agent application.
- Synaric is the default layout, inspired by a richer conversational workspace.
- Zen is the minimal, session-first alternative. Both layouts remain clean.
- Layout choice is a global preference in the same application. Switching must
  preserve the active task, drafts, workspace, connection, model and pending consent.
- The browser is a genuine built-in browsing surface for manual use and consented
  agent browser use, not merely a preview or authentication popup.
- Platform priority is macOS Apple Silicon, then Windows, then Linux. Existing
  Linux tests are useful evidence, not a change to that product priority.
- Local-first operation has no Synara-account requirement.
- Diagnostics are explicit opt-in. Outgoing reports must contain only reviewed,
  useful, non-personal fields. Raw logs, prompts, paths, URLs, files, transcripts,
  credentials and screenshots are not eligible automatic report fields.
- A built-in updater is required. Signing and distribution authority are separate
  owner decisions. Do not invent signing keys or silently enable updates.
- Built-in agent/provider discovery should avoid manual catalog entry. An agent's
  supported provider ecosystem is not automatically exposed by ACP. Consume only
  negotiated catalogs/configuration or an explicitly documented, narrow adapter.
  Never invent providers, models, effort levels or a fixed provider count.
- The description and screenshots of upstream PR #1252 are the model-picker UX
  reference. Do not copy or port its implementation code.

Model-picker behavior reference:
https://github.com/Emanuele-web04/synara/pull/1252

## Exclusive ownership for this session

Write only the following surfaces unless the user or parent integrator explicitly
reassigns ownership:

- `crates/synara-registry/**`
- `crates/synara-app/src/shell/registry.rs`, only registry-local UI changes that do
  not require modifying shared shell/controller contracts
- `foundations/browser/**`, an isolated browser contract/policy test workspace
- `scripts/ekop/**`
- `.github/workflows/ekop.yml`
- `docs/parallel/session-ekop.md`
- `docs/parallel/integration.md`
- `docs/architecture/zen-synaric.md`
- `docs/architecture/browser-host.md`
- `docs/roadmap/zen-synaric.md`
- `docs/verification/ekop.md`

Do not edit the root Cargo manifests/lockfile, database schemas, shared domain
contracts, controller, shell, panels, input widget, or the A/B/C/D/F/G/H/J/M
implementations owned by the other sessions. A new foundation is not an integrated
GUI feature. Record integration seams rather than claiming it is wired in.

Do not edit the shared ROADMAP.md while parallel writers update it. Keep this
session's task evidence and the proposed R/S/T supplement in owned documents.
The parent integrator reconciles the root roadmap after inspecting all branches.
Separate branches do not eliminate semantic or shared-file conflicts.

## Completion gates

| Gate | Outcome | Required evidence | Initial state |
| --- | --- | --- | --- |
| E | Registry failures and integrity checks improved without breaking existing consumers | Focused positive/negative tests and existing API compilation | OPEN |
| K | Browser host contract and isolation/lifecycle policy are executable and tested | Standalone policy tests, explicit native-host gaps | OPEN |
| O | Reproducible, privacy-safe measurement harness | Deterministic harness tests and sample synthetic run | OPEN |
| P | Candidate-specific platform verification groundwork | macOS arm64 / Windows x64 / Linux jobs with honest per-target results | OPEN |
| RST | Shared GUI and picker specification records confirmed product decisions | Requirement and ownership review, no copied legacy code | OPEN |
| INTEGRATION | Read-only branch ancestry/overlap preflight | Disposable Git graph tests, no branch mutations | OPEN |
| DELIVERY | Coherent source published only on the owned branch | Remote SHA and applicable candidate-specific CI | OPEN |

A session may advance a gate without completing its entire roadmap block. Do not
mark E/K/O/P or R/S/T complete from foundations, unit tests or a passing build alone.

## Current environment

The first shell probe and an independent Python probe both returned
`TransportTimeoutError`. Do not repeat the same unavailable local invocation.
Use the authorized GitHub write path and isolated CI, with no repository secrets,
no real user projects, and no external model credentials. CI results must be read
before reporting success.

## Handoff discipline

Record final SHA, files changed, tested platforms, exact checks, remaining gates
and integration seams in `docs/verification/ekop.md`. Preserve successful source
checkpoints even when a different platform or subsystem remains blocked. Do not
reset another session, merge its implementation, or mutate protected refs.
