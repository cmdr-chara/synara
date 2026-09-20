# Native terminal workspaces

Date: 2026-09-21. Parent: `2b7cb0033bfe8ed4540c6ed181aabc2f0c159ce6`.
Product reference checked at the start: upstream main
`e7cd15281e6d16cf8fc55a91496dcff035475e54`, unchanged from the last recorded audit.

## Feature batch

The existing Environment Terminal tab now hosts a project/root-scoped collection
of native terminal views instead of one replaceable shell. Each tab retains its
process, selection, visible scrollback, paste review and output while another tab
or project is selected. The tab header provides explicit new-shell and stopped-tab
creation, rename and guarded close. Running, starting, stopping and exited states
are distinct. A failed stop retains the process handle and offers a retry rather
than declaring the tab safely closed.

Two terminals can be viewed side by side or stacked. Narrow allocated panels
stack without overwriting the saved orientation. Primary-pane allocation is
adjustable from 25 to 75 percent with pointer- and keyboard-activatable controls.
The existing Environment resize/maximize behavior is retained. The new layout is
independently authored with Synara's existing icons and semantic surfaces.

Each view exposes Interrupt, Stop, Restart, literal case-sensitive visible-row
search, previous/next match, selection/viewport copy, return to live output and an
explicit Add to unsent draft action. Context insertion preserves existing text,
checks draft restoration and size limits, and never sends a prompt. Search maps
UTF-8 offsets back to terminal cells so wide characters are not duplicated. It is
not a full-history search. Scroll to another viewport to search older output.

With a terminal focused, Ctrl/Cmd+Shift+T creates a shell, Ctrl/Cmd+Tab cycles tabs,
Ctrl/Cmd+Shift+Tab cycles backwards, Ctrl/Cmd+Shift+F toggles visible-row search and
Ctrl/Cmd+Shift+W requests tab closure. Existing terminal copy/paste and reviewed
multiline paste remain in the native terminal input owner. The shell's existing
keyboard guards retain a focus-only alias to the currently focused terminal.

## Persistence and process boundaries

Only tab IDs, user-assigned names, ordering, active/secondary choices, orientation
and split ratio are stored. Processes, commands, scrollback, captured output and
credentials are not stored. After app restart, all restored tabs are stopped and
require an explicit Start. Layout saves are debounced, serialized and
revision-checked per project/root. Future/malformed values are preserved, not
silently replaced. Errors expose copying the local layout, retrying a save, and
explicitly confirming a reload after shells are stopped.

Each startup, snapshot and stop reply carries its originating project/root, tab
and process generation. Navigation cannot retarget a late callback. Background
shells remain owned and are checked for exit without moving focus. Snapshot reads
have one in-flight request per process. App close saves pending layouts and then
waits for every owned process, including startup in another workspace. Failures
keep Synara open. The existing contained local launch and pinned SSH policy are
reused. There is no arbitrary command field or automatic shell execution from
persisted data, OSC titles, directory hints or merely opening a tool panel.

Bounds are 12 tabs per workspace, 24 simultaneous processes and 32 open terminal
workspace groups per application session. Terminal parsing/history limits remain
in the existing runtime. These are bounds, not measured performance claims.

## Delivery ledger

| Outcome | Evidence/status |
| --- | --- |
| Independent views and scoped callbacks | Implemented and source-reviewed |
| Multi-pane UI and terminal-specific controls | Implemented, native visual acceptance pending |
| Metadata persistence without automatic process replay | Implemented, restart behavior requires native acceptance |
| Existing Explorer/Repository/Environment ownership preserved | Source integration reviewed, no replacement shell |
| Broad regression and screenshot campaigns | Deferred per the user's feature-first instruction |
| Compiler and runtime check | Not run: no local Rust compiler, static.rust-lang.org DNS lookup failed |
| Transfer/structure checks | Delimiter and whitespace checks run, not a substitute for compilation |

No dependency, CI workflow, release, main/archive ref, default branch setting or
provider capability was changed. No new screenshots are claimed. A8, G7 and I10
remain open for complete terminal, cross-platform, keyboard and visual acceptance.
Terminal output is retained only for the running app session. Full-history search,
terminal process resumption after restart, Browser, Side chats and remaining
settings/provider/PR/Automation parity are separate work.
