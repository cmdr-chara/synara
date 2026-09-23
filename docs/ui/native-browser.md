# Native Browser and browser use

## Current platform boundary

The desktop application installs the real `synara-browser::native::NativeHost`
on Linux/X11. Wry 0.57.0 owns a WebKitGTK child view beneath the existing
`Session`/`BrowserHost`. It does not introduce another browser session service.
The child occupies the Browser pane, follows its layout, and is hidden when
native overlays or another route cover it.

Native Wayland, Windows and macOS adapters remain unimplemented. These paths
report unavailable instead of simulating a page. A successful Linux acceptance
run does not accept those platforms, accessibility, IME, GPU/HiDPI combinations
or real authenticated websites.

Ubuntu 24.04 builds need `libwebkit2gtk-4.1-dev` and `libgtk-3-dev` in addition
to the native packages in the repository README. Use the pinned Rust toolchain.
Run in an X11 session with `DISPLAY` set. On a desktop with XWayland, an explicit
`GPUI_PLATFORM=x11 cargo run --locked -p synara-app -- --workspace /path`
selects the X11 path when that display is available. No sandbox-disabling engine
flags are required or added by Synara.

## Manual browsing

Open Browser through the command palette. Create a tab and enter a full HTTP(S)
URL, including `http://localhost:port` for a server that is already running.
The pane provides selection, close, back, forward, reload and stop. Titles,
committed URLs, failures and crash state come from native callbacks tied to the
current navigation identity. History is owned by the existing browser domain.
Back, Forward and Reload wait for the native document commit, not merely a
network request. Blank, stopped and closed tabs unmap the shared native surface
instead of leaving the previous page's pixels visible.
No local server is silently launched or guessed.

The Browser panel has an explicit **Restore Manual tabs** toggle, off by
default. When enabled on Linux/X11, Synara saves up to 16 committed Manual-tab
HTTP(S) URLs in an owner-only file beneath its private browser directory and
reopens them at the next app start. Query strings and fragments are removed;
URL paths remain, so leave the toggle off for pages whose paths contain secrets.
Disabling removes the saved URL list. WebKit's existing Manual profile keeps its
own cookies; Synara does not read or export them. AgentTask and Authentication
tabs are excluded from the snapshot and restoration.

Schemes other than HTTP(S), embedded URL credentials, malformed URLs, control
characters and ambiguous backslashes are rejected before navigation. Page
popups, fullscreen requests, page-script clipboard access and unapproved device
permissions are denied. Manual tabs use the engine's native file chooser,
confirm/prompt dialogs and inspector. Agent-task and authentication partitions do
not inherit these manual capabilities. Inspectors close with their owning tab.

In a manual tab, right-click and select **Copy visible page image** to copy the
current viewport pixels to the OS clipboard. No local file is written and no
agent capture capability is enabled. The action is limited to a visible, mapped,
loaded view, 8192 pixels per scaled dimension and 16 megapixels in total. Capture
expires after ten seconds and is cancelled when its owner is destroyed. A result
from a revoked navigation or hidden view does not replace the clipboard. On a
timeout, reload the tab before retrying. Full-page capture, agent screenshots and agent downloads remain unsupported.

A manual HTTP(S) link also offers **Save linked file as...**. Choose a new local
filename in the native save dialog. One transfer per manual profile is allowed,
with a 256 MiB limit and 120-second deadline. The page tooltip reports status and
the originating context menu offers **Cancel download**. Private staging is
published atomically without replacing files or following destination symlinks.
Files are mode 0600 and are never opened automatically. Navigation, Stop or tab
closure revokes pending work. This explicit GET workflow does not replay form
POSTs, handle blob links or grant agents download access. See the
[download verification and limits](../verification/manual-browser-downloads.md).

## Explicit agent browser use

Enable browser use for a selected task in the native panel and confirm the
scope. The controller closes an idle agent session before reconfiguration.
Running prompts refuse reconfiguration. Only locally scoped agents that
negotiate HTTP MCP support receive a task-bound authenticated loopback endpoint.
Provider names are not used as a compatibility test.

Every navigate/read/click/fill/scroll request requires a separate one-shot native
approval. MCP cannot grant consent. Task revocation invalidates old credentials,
requests and views, and does not silently re-enable after restart. Manual and
authentication browser partitions are never exposed to this endpoint. Task
partitions have separately owned temporary data directories.

Document inventories use opaque, document-scoped element IDs in a retained
named WebKit script world, not page-visible properties or a page IPC bridge.
Only fixed application scripts execute. Arbitrary JavaScript is not an agent
tool. Inputs, output inventories, text, queues and execution lifetimes are
bounded. A stale callback after navigation, cancellation, close or crash cannot
restore permission or replace the current document. Changed elements require a
fresh inventory. Links use an explicit navigation operation rather than hidden
navigation inside a click.

Task scrolling uses the existing approved `browser_request` operation:

```json
{"operation":"input","event":{"scroll":{"x":0,"y":300}}}
```

Each axis is a signed CSS-pixel delta limited to 4096. Approval is one-shot, and
both native admission and the isolated script validate the bound. Scrolling clears
the element inventory. Read the document again before a click or fill. This is
page scrolling, not privileged OS input, focused-element scrolling, keyboard input
or an arbitrary pointer action.

Approval is not a general network sandbox. A loaded website can request its own
resources and run its own scripts. The navigation approval boundary and manual
cookie isolation must not be presented as a guarantee that all page traffic is
same-origin or that a form action has no external effects.

## Validation

See `../verification/parity-continuation-2026-09-23.md` and
`../verification/pr-automations-browser.md` for immutable candidate and
run-specific results. The real WebKit test uses owned loopback pages to check
consent, document reads, DOM actions, cookie separation, redirect rejection and
teardown. The GPUI smoke uses an isolated Xvfb display and actual rendered pixels
and pointer events. Neither uses simulated page rendering or a mock browser
transport as acceptance evidence.

The focused acceptance workflow is `.github/workflows/native-webview.yml`,
invoked manually or by an explicit workflow call. It has read-only repository
permissions and never merges or publishes source. Temporary session export and
publishing workflows are not part of the delivered application.

## Manual PNG file export

Right-click a visible, fully loaded manual page and choose **Save visible page
image as PNG...**. Choose a new local filename. Existing files are never replaced.
Only the current viewport is captured, subject to the existing 16-megapixel,
8192-pixel-axis and device-scale budgets. Encoding runs asynchronously with a
30-second timeout and a 72 MiB PNG ceiling. Closing, stopping or navigating the
tab revokes publication. Private staging is removed on every completion path.
The clipboard is unchanged and no exported file is opened automatically.

No agent screenshot capability, full-page capture, background capture or automatic
composer attachment is added. The supported host remains Linux/X11 WebKitGTK.
See [the verification record](../verification/browser-png-export.md).
