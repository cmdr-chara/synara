# Turn resume follows the provider trajectory, never a reconstructed prompt

Accepted 2026-11-27.

Synara orchestrates several provider harnesses (claude, codex, acp-compatible,
opencode, pi). A turn that must continue — after a restart, a usage-limit wait,
a crash, a quit-resume — always continues inside the provider's own native
session, addressed by the durable resume cursor recorded in
`provider_session_runtime`. The server never reconstructs a replacement
prompt from the projected transcript, never edits the trajectory, and never
replays credentials outside the official provider binaries.

This is the linear-history invariant: the trajectory is the prompt. The
projected transcript in SQLite is a mirror for display, search and forensics;
it is never a source of model context. The one sanctioned exception is the
provider handoff, which consciously starts a *new* trajectory in another
harness with an explicit, user-visible handoff document rather than silently
rewriting history. Imported external sessions bind their provider-native
cursor at import time so the same rule applies to sessions that did not begin
inside Synara.

Shell execution follows the same shape. The server does not operate a hidden
stateful shell on the agent's behalf: providers execute their own tool calls
inside their own CLI processes, and the server-side surfaces that do execute
work (automations, hub admissions) express it as turn input, not as commands
injected into a long-lived shell whose state could drift from the recorded
trajectory. Server-owned PTY sessions in `terminal.*` are a user-facing
feature with their own lifecycle, deliberately separate from agent execution.

The practical consequences reviewers should enforce:

- A resume path that synthesizes user or assistant messages the provider
  never saw is a bug, however plausible the reconstruction.
- Compaction, truncation or summarization performed inside a provider is the
  provider's own trajectory decision and is recorded, not second-guessed.
- New providers integrate by exposing their native resume mechanism; a
  provider without one starts a fresh trajectory explicitly rather than
  approximating continuity.
