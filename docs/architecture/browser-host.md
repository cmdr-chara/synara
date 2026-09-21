# Browser host architecture

Status: real Linux/X11 embedding is implemented and has focused native evidence.
Windows, macOS and native Wayland adapters remain open. See the
[candidate-specific receipt](../verification/pr-automations-browser.md) and
[operating guide](../ui/native-browser.md). A preview-only replacement is not the
accepted product. The original macOS Apple Silicon, Windows and Linux target
requirements still apply, even though Linux is the first exercised adapter.

## Ownership

The application, browser state and consent policy remain Rust/GPUI.
`foundations/browser/lib.rs` owns dependency-free consent policy.
`crates/synara-browser` owns `BrowserHost`, `Session`, typed commands, bounded
history and requests. `crates/synara-browser/src/native` implements the existing
`NativePort` using Wry/WebKitGTK, an owned X11 child surface and the native UI
thread. It does not create a second browser session service.

The GPUI shell pumps native work, places the child within the Browser pane, and
hides it for other routes, blank/retired tabs or native overlays. A GTK/WebKit
widget has its own native surface and frame-clock lifetime. Browser cookies are
not stored in conversation transcripts. The workspace controller owns explicit
task enrollment and the revocable authenticated loopback MCP endpoint.

The Linux adapter is enabled by the desktop application's native-webview feature.
Headless domain tests can omit that feature. Unsupported platforms and unsuitable
window handles report unavailable. WKWebView and WebView2 remain candidates for
the unimplemented Apple and Windows adapters, not accepted Synara integrations.

## Navigation and permissions

Raw HTTP(S) URLs are parsed with the pinned URL library before crossing the native
boundary. Embedded credentials, privileged schemes, control characters and
ambiguous backslashes are rejected. Authoritative native callbacks carry a
navigation identity. Old commits, title updates, gestures and crash callbacks
cannot replace a newer document. History controls wait for a native commit rather
than treating a dispatched request or a received HTTP request as a loaded page.

Manual navigation is distinct from agent authority. Blank agent tabs can request
explicit initial navigation, but cannot read or interact with a nonexistent
document. Every agent navigate/read/click/fill request uses an immutable payload
and a one-shot native consent decision. MCP and webpage scripts cannot approve a
request. Navigation, cancellation, task revocation, close and crash revoke stale
work. Restart restores no browser-use grants.

Manual, agent-task and authentication partitions remain distinct. Manual browsing
uses its own persistent directory. Task partitions use separately owned temporary
directories and never receive the manual cookies or credentials. Sharing an
existing authenticated/manual tab is not implemented and must never occur as an
implicit context change.

DOM inventories use opaque, document-scoped IDs held in a retained named WebKit
script world. Only fixed application scripts execute there. Agent tools cannot
supply arbitrary JavaScript, shell commands, paths or generic RPC method names.
Changed or stale elements require a fresh inventory. Native operation responses,
text, element inventories, queues, IPC messages and lifetimes are bounded.

## Native adapter obligations

Adapters must enforce navigation authority before allowing a disallowed target,
not merely report the violation after loading it. The Linux integration test
uses a second loopback origin and asserts that the rejected redirect target
receives no request. This is navigation-boundary evidence, not a promise that all
subresources or all website-generated network effects are same-origin.

Popups, file choosers, fullscreen, clipboard and device permissions are denied
unless a separately implemented policy grants them. Download/capture exports are
unsupported in this slice. Their typed commands and opaque handles are not proof
of an implemented transfer or screenshot feature. OAuth sharing, upload/download
and capture must each have their own scope and bounded resource ownership.

Use a monotonic clock for consent and operation deadlines. Authenticate task
identity from the controller rather than a page-supplied task ID. Freeze request
payloads before approval, consume grants immediately before dispatch and release
queued/native resources during shutdown. Hiding or showing a pane must not create
new grants or silently re-enroll a task.

## Evidence and open acceptance

The Linux/X11 evidence covers real embedded pixels, pointer input, delayed history,
blank-tab surface retirement, cookie separation, consented document/fill/click,
redirect rejection and teardown. It does not establish native Wayland, Windows,
macOS, IME, accessibility, GPU/HiDPI, production authenticated websites, downloads,
OAuth integration or live-model browser-use acceptance. K1-K6 remain broad open
gates until their full multi-platform contracts have evidence.

Run the dependency-free policy tests independently with:

```sh
rustc --edition 2024 -D warnings --test foundations/browser/lib.rs -o /tmp/browser-policy-tests
/tmp/browser-policy-tests
```

Run the focused native commands recorded in the verification receipt, or dispatch
the read-only `native-webview.yml` workflow on the intended branch. The EKOP
policy matrix is not evidence that a native engine is implemented on its targets.

## Platform reference boundaries

- Apple WKWebView API: https://developer.apple.com/documentation/webkit/wkwebview
- Microsoft WebView2 security guidance:
  https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/security

These are platform references, not claims that those engines implement Synara's
policy automatically. The ownership, consent and acceptance rules above remain
application responsibilities.
