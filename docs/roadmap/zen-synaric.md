# GUI roadmap supplement: shared components, Zen and Synaric

This supplements ROADMAP.md without renumbering A-Q while parallel sessions use
those identifiers. All implementation checkboxes below remain open. The approved
specification is [shared GUI architecture](../architecture/zen-synaric.md).
This document is a backlog and acceptance contract, not a completed GUI.

Product defaults: Synaric, a global layout preference, Zen as the minimal alternate,
local-first without a Synara account, macOS Apple Silicon before Windows and Linux.
A full manual/agent-use browser is required but does not block the conversational
GUI. Terminal sophistication is not the primary product milestone.

## R. Shared GUI architecture and model picker

Priority: P0. Ownership after integration: one shared-presentation integrator.
Dependencies: accepted B/C/D/F interfaces and review of all session shell changes.

- [ ] R1 Extract shared conversation, composer, tool/plan cards, permission and
  question components without changing domain identities or duplicating services.
- [ ] R2 Define shared layout/theme/typography/focus contracts and validated global
  layout persistence. Default to Synaric on first launch.
- [ ] R3 Implement the PR #1252 unified model picker with starred/provider tabs,
  search, keyboard row selection, effort side panel and capability-driven traits.
- [ ] R4 Persist complete starred model/trait presets, including multiple efforts
  per model. Revalidate stale presets and distinguish agent/provider/model identity.
- [ ] R5 Serialize model/config intents against acknowledged backend state.
  Handle dynamically changed options and partial failure without claiming remote
  atomicity or sending a prompt with a misleading half-applied selection.
- [ ] R6 Switch layouts without restarting agents, discarding drafts/history,
  changing models, resolving pending consent, losing dirty buffers, or recreating
  terminal/browser services. Preserve logical scroll anchors and IME composition.
- [ ] R7 Verify shared components with keyboard, pointer, focus, accessibility and
  fixture-agent tests, then real-agent journeys where authorized credentials exist.

Acceptance: the same active task and agent session survive repeated live switches.
The picker matches the agreed behavior and shows only actually available options.
No legacy implementation code is copied or translated into the new components.

## S. Zen layout

Priority: P0 GUI delivery, implemented after the shared shell contract is stable.
Ownership after integration: Zen presentation only, consuming R-owned components.

- [ ] S1 Build minimal session navigation, conversation column and shared composer.
  Keep empty states useful and persistent chrome restrained.
- [ ] S2 Expose files, changes, registry, browser and terminal contextually without
  hiding unresolved permission/question requests or stopping hidden services.
- [ ] S3 Support responsive sizing, narrow windows and display scaling without
  clipping model controls, consent cards or composer actions.
- [ ] S4 Preserve keyboard discoverability and task selection. Do not substitute
  hover-only interactions or a different preset/agent store for minimalism.
- [ ] S5 Capture native screenshots and interaction evidence on the supported
  platforms. Check light/dark appearance, long conversations and failure states.

Acceptance: users can complete the same core agent workflow without permanently
visible workbench panels. Zen is a layout choice, not a restricted feature tier.

## T. Synaric layout

Priority: P0 and first-launch default. Ownership: Synaric presentation consuming R.

- [ ] T1 Build the conversation-first default workspace, task navigation and shared
  model picker. Keep the design clean rather than exposing every tool at once.
- [ ] T2 Integrate contextual file/editor/diff/Git surfaces from F/G/H, retaining
  dirty state and selection when panels or tasks change.
- [ ] T3 Integrate available terminal, registry and later browser/remote surfaces
  without granting them ownership of core task or connection state.
- [ ] T4 Verify stream, tool/permission, model-change, retry, reconnect, restored
  history and empty/error-state journeys through the native GUI.
- [ ] T5 Verify global layout switching, menus, keyboard shortcuts, native focus,
  IME, accessibility and per-platform screenshots without divergent business rules.

Acceptance: first launch leads to a usable conversational agent workflow, and all
exposed development surfaces behave consistently with the same shared core.

## Delivery sequence and overlap control

Integrate the independent sessions first. Freeze shared service and state contracts.
Then implement R, the default Synaric shell T, and alternate Zen shell S. T and S
may proceed independently only after the shared component and state contracts are
stable. R/S/T have higher product priority than device tools or browser polish.
Security, accessibility and performance constraints apply during implementation,
not only at the end. Dedicated evidence and distribution tasks remain O/P/Q.

Do not infer completion percentages from this checklist. The parent integrator
adds these lanes and their evidence to the root roadmap once parallel changes are
reconciled. The root roadmap validator currently covers A-Q only.
