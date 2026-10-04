# Parity verification — batch 46 (composer footer + sidebar pinned section)

Batch date: 2026-09-26. Upstream reference: `Emanuele-web04/synara` tip
`a33435c18` (v0.9.2). Working repo: `cmdr-chara/synara` (Rust/GPUI port).

## What changed

Composer footer row reshaped to upstream's `Composer`/`ContextWindowMeter`
layout (`apps/web/src/components/chat/*`): `+` extras → access picker →
context ring meter → flexible space → model/effort picker → mic → circular
send. Removed the port-only surfaces that made the footer read differently:

- Permanent voice-disclosure line under the input — upstream has none; voice
  failures surface as toasts.
- In-box "Context usage … / Usage details" text row — replaced by
  `context_meter`, a stroked ring canvas (radius 6, upstream's proportions)
  rendered only when the session reports `context_used`, red at ≥80%, click
  opens `settings::Section::Usage`, tooltip carries the summary text.
- Standalone attach-files, AppSnap, and follow-ups icon buttons — the real
  actions moved into the `+` extras menu exactly where upstream places them:
  "Files and folders" now runs the native file picker (`choose_attachments`,
  upstream `fileInputRef.click()`), "Attach window" runs `toggle_appsnap`
  (upstream opens its window submenu; a nested submenu is a follow-up), and
  "Goal" inserts `/synara/goal set <draft>` into the composer (upstream
  `insertGoalSlashCommandInComposer` writes `/goal -- <draft>`; the port's
  namespaced form is `set` — no `--` literal exists in its parser).
- Model previous/next cycle arrow buttons — upstream has no such controls;
  the `ModelNext`/`ModelPrevious` keyboard commands remain.
- Access picker label changed to upstream's verbatim copy: "Ask for approval"
  with detail "Always ask to edit external files and use the internet"
  (`lib/runtimeMode.ts` `RUNTIME_MODE_PRESENTATION`), and the invented
  orange shield icon/text tint removed from the trigger.
- "Follow-ups" row appended at the end of the extras menu (invented feature
  kept reachable but off the footer row; `followup_toggle` deleted,
  `FollowupState` gained `saved_count`/`is_open` accessors and
  `Shell::toggle_followups`).

Sidebar (classic view): added upstream's flat **Pinned** section
(`renderPinnedThreadsSection`, `Sidebar.tsx` ~L4530) — a muted "Pinned"
label over flat thread rows, rendered only when pinned threads exist and
shared by both Threads and Studio surfaces. Pinned threads are now excluded
from their project groups and the Chats list (upstream
`getUnpinnedThreadsForSidebar` semantics) instead of merely sorting first.

## Verification

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --locked --workspace --all-targets`: no new warnings — all
  remaining sites are in files untouched by this batch (same baseline sites
  as committed HEAD).
- `cargo test --locked --workspace`: green except the documented env-only
  `pull_requests::tests::discovery_does_not_change_worktree_or_index`
  (`~/.gitconfig` rewrites github.com remotes) and one recurrence of the
  `integrations::probe` loopback `WouldBlock` flake under parallel load —
  the full probe suite passes 7/7 in isolation in 0.13s, same behavior as
  committed HEAD.
- `cargo check -p synara-app`: clean after each edit round.
- `context_summary` tests retained verbatim (percent/warning math unchanged).

## Live test

Acceptance gestures for the live run (verified in `cargo run -p synara-app`
on an isolated `--data-dir`, recording `rec-0b0c512f`):

1. Composer footer reads `+  Ask for approval ⌄  [ring if reported]  Model ⌄  mic  send`
   with no disclaimer line and no context text row. **Passed** — before/after
   screenshots `ss_5d3a45bf` → `ss_20850694`; ring correctly absent (no usage
   reported, so its rendering is untested).
2. `+` menu lists Files and folders / Attach window / Goal / Plan mode /
   Debug mode / Follow-ups; "Attach window" opens AppSnap, "Goal" inserts
   `/synara/goal set ` into the draft. **Passed** — `ss_dac6ca81`,
   `ss_c1aafd4d`, `ss_ca261ff3` (AppSnap shows the expected
   platform-unsupported notice on this box). "Files and folders" (native
   picker), Plan mode, and Follow-ups rows verified present, not clicked.
3. A pinned thread appears under a flat "Pinned" section and disappears from
   its project group / Chats row. **Passed** — hover-revealed "Pin thread" →
   section appeared above Projects, thread left Chats, row exposes
   "Unpin thread". `ss_616b9a14`.
4. Chat header shows the thread title (`ss_74e9218d`). **Passed**.

Defect found during live test, queued for next tick: `EntryMode::Editor`
multi-line fields (Notes dialog, automation instructions) do not visually
paint their text in this build — content stores correctly (verified via the
dialog's Copy button + saved ledger JSON) but renders as an empty rectangle.
Single-line fields paint fine.

Env limits: no agent CLIs installed on the box, so send/run paths stop at
spawn (`I/O operation failed`) — used only to prove controls are enabled.

## Known parity gaps recorded

- Follow-up drafts remain a port-only feature (upstream queues drafts only
  while a turn is busy via `ComposerQueuedHeader`). Inventory for excision or
  reshape into busy-queue semantics.
- "Attach window" opens the AppSnap panel rather than an in-menu window
  submenu (upstream `view === "windows"` view switch).
- Access picker is still a single read-only row; upstream cycles three
  runtime modes (`Ask for approval` / `Approve for me` / `Full access`).
- No `Fast mode` extras row yet (upstream shows it when the provider
  advertises support).
