# September 24 parity batch 1

Base: `e46b58bb542a5220f099772003b4d70ea55e85c2`, preserving all 25 prior slices.
Upstream inspected: `eaa61eded31b6755d4f30ba8eabc5d905cf817cb`.

## Delivered product boundaries

**Local headless execution.** The browser can explicitly submit a reviewed text
prompt to the existing native Controller, stop that task, and refresh status and
its durable transcript without replacing unsaved input. Start uses the existing
bearer/Host/Origin/JSON checks, rejects stale drafts and active/archived tasks,
limits active web runs to eight and each run to one hour, and retains submitted
drafts for review/retry. Shutdown cancels and drains workers before disconnecting
shared agents and releasing the workspace owner. Status records are bounded and
in-memory, and reopening never replays a prompt. The existing denial-only
interaction handler remains authoritative. This is local ACP execution with an
already configured agent, not direct-model, SSH, interactive sign-in or approvals.

**Goal command lifecycle.** `/synara/goal resume` prepares a draft using the
existing bounded goal lease without sending it. `/synara/goal edit [objective]`
opens an unsaved review, and `/synara/goal clear` uses the revision-checked saved
goal owner only when paused and without pending edits. Selection/loading/write
and IME boundaries remain in force. Provider commands are never shadowed.

**Simulator app lifecycle.** A selected ready Apple Simulator accepts a reviewed
absolute local `.app` directory with a regular, bounded Info.plist. Installation
requires a second click with the same target and path, does not launch, and uses
literal arguments through the existing bounded/cancellable helper. Explicit
bundle termination shares the existing bundle-ID checks. Failure retires stale
viewer state and requests rediscovery. An OS-queued install is not claimed to
roll back when its CLI helper is cancelled.

## Verification contract

The publishing workflow applies the immutable candidate to its exact base, runs
focused server execution and device tests, then one integrated Rust pass across
server/runtime/workspace/app, strict Clippy, formatting and roadmap checks.
Five Node DOM-owner regressions cover inert load, submitted-vs-later draft text,
late replies, status polling and credential changes. The native goal-command
journey uses a real GPUI window under private Xvfb and the repository ACP fixture.
No live provider credentials or Apple hardware are used.

The workflow appends its completed run identity below only after those commands
succeed and publishes the code by an ordinary fast-forward commit. All 21 broad
parity gates remain OPEN.

## Observed validation

All workflow validation steps above passed before this commit was created.
Evidence: https://github.com/cmdr-chara/synara/actions/runs/35931103721
Run attempt: 1. Base of publication: `07a6041294b1396df5d8f928a1a6030ca8357909`.
The initial integration attempt exposed an intermittent existing AppSnap assertion; its diagnostic is now explicit and the unchanged contract passed isolated and integrated runs. Three inherited model-picker Clippy findings were refactored without changing product behavior.
Live provider, Apple Simulator, remote deployment and other-platform acceptance were not exercised.
