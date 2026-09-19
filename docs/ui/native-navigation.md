# Native navigation foundation

Status: published implementation checkpoint, not visual parity or release readiness.
Source commit: `495bbfd13dec8d6f75419cf53092139446a77273`.

## Product reference

Emanuele's Synara is the design reference, including the user-supplied home-screen
capture and the pinned first-party references in `reference-register.md`. Its
narrow sidebar, compact project/chat rows, Settings footer and secondary menus
are the target. No web frontend source or another application's UI code was
imported. The small line icons are independently authored GPUI path geometry.
Native fixture screenshots under `captures/` are verification evidence only, not
application assets, reference-image substitutes or proof of parity.

## Source ownership

Paths below are relative to `crates/synara-app`.

| Module | Responsibility |
| --- | --- |
| `src/ui.rs` | Semantic dark material roles, geometry, platform font choices, shared buttons, action rows, native tooltips and line icons |
| `src/shell/navigation.rs` | Compact project/chat navigation, bounded pages, disclosure and transient focus state |
| `src/shell/chrome.rs` | Native shell layout, panel navigation, More menu, keyboard traversal and focus restoration |
| `src/input.rs` | Existing native text/IME/selection ownership with tab participation and optional non-content diagnostics |
| Existing shell/conversation/terminal/panels | Retained backend consumers, transcript, permissions, files, Git, registry and terminal operations |

There is no second persistent catalog/task model. Project and task selections use
`select_task`, `create_task`, `WorkspaceService` and existing async jobs. Creating
a chat preserves the previous in-session draft. Dirty-file project switching
retains the existing guard. Registry approval reached through the new menu still
uses the real review/consent implementation and never implicitly starts an agent.
No production backend API changed for this UI checkpoint.

GPUI owns native keyboard press/release activation. The action primitive uses one
click callback, not a second keydown handler. Menus restore trigger focus on
Escape, and the shell repairs focus when a removed transient control retires its
dispatch path. Deliberate moves to other views and close confirmation ownership
are not overridden. Details and causal evidence are in `continuation.md`.

## Verification

[Candidate run 35440664654](https://github.com/cmdr-chara/synara/actions/runs/35440664654)
passed all ten commands recorded in `native-navigation-evidence.json`, including
workspace formatting/check/tests, strict Clippy, structure/roadmap/publisher
checks and the native application/fixture build. Publication matched the exact
exported tree, with only the evidence receipt and native captures added after
source checks. The temporary preparation and recovery workflows were removed.

`native_smoke.py`: eleven passing journeys, including streaming/tool completion,
permission denial, contained file callbacks, second fixture selection, explicit
registry approval without execution, dirty-close cancellation, guarded save,
restart without autostart, save conflict and explicit discard.

`native_navigation_smoke.py`: eight passing journeys, including keyboard menu
activation and dismissal, focus recovery after approval, exactly-once key-release
thread creation, real ACP streaming, draft restoration/submission to the correct
thread without changing its sibling, sidebar collapse, resize and shutdown.
The multi-thread oracle scopes events by task/thread ID and sequence.

Captures were produced at native window sizes 1420 x 930, 1280 x 800 and 960 x 700
on separately owned Linux/X11/Xvfb displays. The capture files include the private
display area. Resize survival and screenshots do not establish visual fidelity.
Image-inspection paths were unavailable, so visual comparison remains explicitly
unperformed. No screenshot was generated from a mockup or inferred from code.

Regular native CI retains both interaction suites and uploads their evidence.
A scheduled, pending, skipped or failed run is not counted as a passing result.

## Parity and gaps

| Area | Current status |
| --- | --- |
| Native shell/navigation primitives | Published and interaction-tested foundation; visual comparison open |
| Project/chat rows and Settings footer | Compact native layout with real backend selection and creation |
| Menus/tooltips/keyboard | More popover, shared focusable controls and retired-focus recovery implemented; full menu/context coverage incomplete |
| Sidebar list size | Bounded 64-row pages; mature virtual/infinite-sidebar behavior remains different |
| Theme and typography | Centralized dark roles for migrated surfaces; full persisted appearance wiring and reference font matching open |
| Conversation/header/composer/Markdown | Existing published implementation retained; earlier local overhaul unrecovered; product polish still required |
| Provider/model/mode controls | Existing backend-driven controls retained; mature dropdown/composer layout still incomplete |
| Transparency, wallpaper and welcome state | Not reproduced by this checkpoint |
| Pinned/unread metadata and context actions | Missing or partial; no synthetic activity status invented |
| Files/Git/terminal/remote | Existing service boundaries retained; surrounding mature product UI remains partial |
| Search, Kanban, PRs, automations, handoffs | Missing or backend/UI integration incomplete; no false working destinations |
| Browser/device host surfaces | Concrete backend/platform gaps remain; no screenshot placeholders presented as functionality |
| Linux UI | Native interaction and capture evidence only for private X11/Xvfb |
| Wayland/macOS/Windows UI | No interactive or visual acceptance established |
| Accessibility | Roles, labels, native activation and focus behavior added; no complete assistive-technology audit |

## Recovery remains separate

The earlier local UI at `/mnt/data/ui-work/native-integrated` is not established
as recovered. These published changes were developed against the remote native
branch. Preserve the old worktree and explicitly reconcile overlapping files when
access returns. An early archived envelope failed compressed-data integrity and
was not applied. See `continuation.md` for the complete recovery boundary.
