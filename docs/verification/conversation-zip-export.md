# Conversation ZIP export verification

September 23, 2026. Source baseline `e4a740f11450e03f26b0435f0d3e442dc1325e7b`.
Validation input `deca98cda22b2a4eaccf38c8f19808a36503a7ec`. Exact source is the subsequent validation commit in the same run.
Upstream: `Emanuele-web04/synara@eaa61eded31b6755d4f30ba8eabc5d905cf817cb`.
Compared `apps/server/src/orchestration/exportThreadArchive.ts` and
`packages/shared/src/threadExport.ts`, including upstream's in-flight export guard.

## Implemented workflow

The native Conversation actions menu and `/synara/export-zip` open the existing
system save dialog for a ZIP containing `thread.json` and `transcript.md`.
The old Markdown menu and `/synara/export` remain available. Both ZIP entries
come from one durable SQLite read snapshot. Running, waiting, unfinished or
history-restoring conversations are refused, and the native shell rechecks its
busy/control state after the dialog. No agent is started by export.

Structured data is an explicit native `synara-thread-export-v1` projection:
IDs, title, actual model/mode IDs, state, snapshot sequence, timeline-ordered
message roles and exact UTF-8 text, known timestamps and recorded image metadata.
Ambiguous role-reused message IDs have null timestamps, not invented values.
Unsent drafts, credentials/session configuration, tool payloads, workspace files
and embedded image bytes are excluded. This is a portable native snapshot, not
an upstream import format or hidden provider-session transfer.

Markdown is limited to 8 MiB, structured JSON to 24 MiB before compression and the
archive to 36 MiB. ZIP entries use deflate and fixed safe names. The existing
private no-overwrite publisher is reused. Existing destination files are never
replaced. The already-locked zip 8.6.0 dependency is reused without upgrades.

## Actual evidence

[Validation run 35896910604](https://github.com/cmdr-chara/synara/actions/runs/35896910604) passed:

- Source-transport SHA-256 and exact patch application checks.
- Rust 1.98.1 formatting and `cargo fmt --all --check`.
- Lockfile verification permitting only existing ZIP and gdk-pixbuf direct links, with no changed package versions.
- Existing conversation-tool tests plus one new ZIP regression covering exact entries/text/metadata, private draft exclusion, active-conversation refusal and existing-target preservation.
- Existing native-command parser tests, now including `/synara/export-zip`.
- Strict workspace/app Clippy for all targets with warnings denied.
- Repository structural and roadmap checks. All 120 historical task bodies and 16 checked states remain unchanged.
- The real GPUI application and ACP fixture build.
- The existing native chat-tools journey, extended to prove the ZIP action is discoverable without sending or changing conversation history.
- The real WebKit owned-download regression from the preceding verified feature slice.

Exactly one new Rust test was added for ZIP export. The existing native journey
was extended rather than adding another harness. The native menu was exercised,
but system save-dialog interaction was not automated. File bytes, privacy and
publication behavior are covered by the service-level integration test. This
is Linux fixture evidence, not macOS/Windows or live-provider acceptance.

## Preserved menu-journey failure

Run 35893224398 passed compilation, all focused tests, real PNG/download/capture
checks, strict Clippy and structure. Its native journey reached and captured the
ZIP menu successfully, then failed the unchanged Reuse-last-prompt assertion.
`ChoiceMenu.key_down` first clears a nonempty query on Escape, and only dismisses
on the next Escape. The newly added journey used one Escape, leaving the popup
open over the actions button. The following click selected its first item, Find
in conversation. The failure screenshot and search-layout log confirm that the
subsequent `Reuse last prompt` text entered the search field instead of the menu.

The new ZIP check now follows the existing clear-then-dismiss contract with two
Escape presses. No product shortcut behavior, assertion or timeout was weakened.
The complete original chat journey is rerun, including the draft and event checks.

## Preserved GTK compile failure

Run 35892782790 passed formatting, workspace check and exact dependency-link
validation, then failed native compilation with E0308. GTK 0.18.2's
`FileChooserExt::add_filter` takes ownership of `FileFilter`, not a borrowed
reference. Passing the value fixed the exact type mismatch without changing
runtime guards or tests. Its successful real PNG test is recorded above.

## Preserved queued-menu timing failure

Run 35894658901 again passed Rust compilation, real export/capture/download
regressions, strict Clippy and structure. The native journey failed earlier,
waiting for the Markdown clipboard. Its log records a queued pinned-message jump
at 17:21:28.166247Z, followed by repeated one-item pin menus and jumps rather than
the requested Conversation actions menu. The failure screenshot preserves that
pin menu, and the unsent draft remains intact.

Native menu open/close transitions now emit non-content diagnostic events. The
journey waits for those actual owner transitions, including the selected pinned
message jump, before issuing the next action. It retains keyboard activation,
all original clipboard/draft/event assertions and existing timeouts. This is
explicit synchronization, not retry-until-green or skipping the failed check.

## Preserved post-copy layout race

Run 35896046274 passed Rust, real file exports and strict Clippy. The explicit
menu transitions fixed the queued-action race, but the native journey then
attempted to open ZIP actions immediately after receiving clipboard data.
The copy-result banner changes the toolbar's vertical position. The captured
layout moved from y=124 at 17:32:51.371001Z to y=161 at 17:32:51.441788Z.
The failed click happened while the harness still held the earlier geometry.
The screenshot shows the successful copy notice and unchanged draft, not a
broken export or missing menu implementation.

The journey now records the pre-copy toolbar position and waits for the actual
post-banner geometry before its next click. It uses no hard-coded pixels,
injected state, extra timeout or retry. Existing assertions remain unchanged.

## Remaining parity

Upstream-specific skill/mention/attachment projections are not fabricated where
native history does not retain equivalent data. Binary attachment bundling,
archive import, historical model changes, rich tool-payload export and broader
platform acceptance are outside this slice. The export lane advances within
near parity. No missing product surface or broad release gate is closed.

## Preserved failed attempt and correction

Run 35890799883 passed compilation, eight conversation tests, the native-command
regression, strict Clippy and structural checks. The native journey timed out
before reaching export. Its captured app log records the Return key while
`loading_thread`, `loading_route`, `loading_draft`, `attachment_pending` and
`goal_pending` were true. `composer-submit` was disabled at 16:49:05.425701Z,
the send attempt occurred at 16:49:05.433238Z, and it became enabled at
16:49:05.451572Z. The saved draft remained `hello`, no durable events were
written, and the app stayed open. This is an observed harness readiness race,
not a ZIP export failure or an unexplained flaky retry.

`Scenario.prompt` now waits for the real logged enabled Send control before
pressing Return. Application hydration guards, timeouts and existing assertions
are unchanged. The subsequent run above re-exercises the whole native journey.
The combined batch also links the already-locked gdk-pixbuf 0.18.5 package for
async PNG encoding. The lockfile check permits exactly those two direct links.
