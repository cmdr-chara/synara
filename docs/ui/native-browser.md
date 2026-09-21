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
No local server is silently launched or guessed.

Schemes other than HTTP(S), embedded URL credentials, malformed URLs, control
characters and ambiguous backslashes are rejected before navigation. Page
popups, file chooser requests, fullscreen requests, clipboard access and
unapproved device permissions are denied. Download and capture exports are
explicitly unsupported, not advertised as functioning capabilities.

## Explicit agent browser use

Enable browser use for a selected task in the native panel and confirm the
scope. The controller closes an idle agent session before reconfiguration.
Running prompts refuse reconfiguration. Only locally scoped agents that
negotiate HTTP MCP support receive a task-bound authenticated loopback endpoint.
Provider names are not used as a compatibility test.

Every navigate/read/click/fill request requires a separate one-shot native
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

Approval is not a general network sandbox. A loaded website can request its own
resources and run its own scripts. The navigation approval boundary and manual
cookie isolation must not be presented as a guarantee that all page traffic is
same-origin or that a form action has no external effects.

## Validation

See `../verification/pr-automations-browser.md` for immutable candidate and
run-specific results. The real WebKit test uses owned loopback pages to check
consent, document reads, DOM actions, cookie separation, redirect rejection and
teardown. The GPUI smoke uses an isolated Xvfb display and actual rendered pixels
and pointer events. Neither uses simulated page rendering or a mock browser
transport as acceptance evidence.
