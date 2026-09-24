# September 24 batch 6: web task reviews and live Explorer search

Native base: `49e10568174783b0cacea0da9912fa10d8b73ab2`, preserving the 38
previously delivered slices. Upstream reference remains
`eaa61eded31b6755d4f30ba8eabc5d905cf817cb`. Relevant upstream inspection covers
`ComposerPendingApprovalPanel.tsx`, `usePendingUserInputDrafts.ts` and
`WorkspaceSearchPalette.tsx`. Native ownership remains generic ACP, the existing
InteractionBroker, workspace filesystem/search services and GPUI input owners.

## Two delivered boundaries

**Web reviews.** The authenticated loopback web task can answer Session-scoped
one-time permissions and structured questions. Original schema and task/thread
identity, fresh opaque receipts, broker expiry/cancellation and one-shot channels
are authoritative. Persistent grants, connection requests and URL login are
not enabled. Visible requests and queues are bounded. Invalid responses retain
the request, expired/duplicate replies fail, and shutdown cancels the inbox.
Literal labels remain inert and private protocol session IDs are not returned.
Repeated polls preserve form nodes and typed values, late replies cannot update
another task or credential epoch, empty-valued choices remain distinct from
omission, and oversized answers are retained rather than sent. Question drafts
are in-page only. Rich tool output/context and durable draft recovery remain open.

**Live file search.** Typing schedules a 350 ms idle search for names or content.
Only one local/SSH traversal may run at a time, with later edits coalesced.
Generation, selection, root and directory changes retire queued work, and stale
results cannot populate another scope. A remote setup failure returns through
the same owned reply and releases admission. Search headers and rows no longer
shrink out of their pointer-hit container in narrow panes. Explicit Search and
keyboard result navigation retain the existing owners. No file writes or agent
prompts are added by search.

## Verification contract

The integrated pass covers only server/app unit tests, strict Clippy, formatting,
roadmap structure and the browser DOM-owner tests. Broker/API tests cover thread
isolation, cancellation/shutdown/restart receipts, persistent-grant refusal,
invalid schemas, queue/serialization limits and auth/Origin/content type.
The native Linux/X11 journey exercises live content and name results without
Search/Enter, pointer selection in a narrow tool pane, replacement queries,
Escape retirement, exact editor text and unchanged files/events. It uses private
fixture storage and Xvfb, not live provider credentials or an SSH host.

Passing checks are recorded below only after they actually run. No full gate is
closed by these additions. M2 still needs connection authentication, richer tool
context, durable question drafts, direct/remote execution and deployment policy.
N2 still needs exact upstream ranking, native SSH and wider platform acceptance.
No dependency, manifest or lockfile changes are required for this batch.

## Observed validation

129 app tests, 30 server library tests, one server binary test and 12 browser tests passed in run 36004763291. After removing one unused test-only import, strict Clippy/all-target compilation passed in run 36005707815. Rust source/manifests are byte-identical to that passing candidate, verified by Git-index manifest SHA-256 `8781418c3021cb6113b41acbed4a91a09ca5a11ecae37f21452607458283af57`.
The first native attempt stopped in the Python launcher before opening an app because it called Scenario with the wrong signature. The corrected options-object invocation uses the unchanged fixture helper. A second native run passed live content-search/click checks and reached Ctrl+P, whose layout changed while the test still held old query bounds. The final test waits for a fresh rendered query bound before clicking. The native search journey, formatting and roadmap checks passed before these objects were prepared.
Evidence: https://github.com/cmdr-chara/synara/actions/runs/36007528324
Run attempt: 1. Publication base: `597d55c066b0e607b23881f419d72c80595ca46c`.
Native Linux/X11 evidence covers live name/content results, narrow-pane pointer selection, exact editor contents, query replacement, Escape cancellation and unchanged files/events. The web evidence uses the real InteractionBroker/API and deterministic DOM fixtures, not a live authenticated provider or remote deployment. SSH GUI and other-platform acceptance remain open.
