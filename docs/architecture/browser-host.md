# Browser host foundation

Status: policy implementation and integration contract, NOT an embedded browser.
The user's product requirement is a normal built-in browser with manual browsing
and explicitly consented agent browser use. A preview-only replacement is not the
accepted product. Implementation priority is macOS Apple Silicon, Windows, Linux.

## Boundaries

The main application, state, and consent policy remain Rust/GPUI. Use a narrow
platform host for web rendering. Investigate WKWebView first on macOS and WebView2
on Windows. Linux needs a separate supported embedding investigation. These are
adapter candidates, not a claim that GPUI embedding, input, composition, capture,
or browser automation has already been proven.

A platform spike must demonstrate native view attachment, resize/scaling, focus,
IME, window teardown, navigation callbacks and crash recovery before a platform
backend is selected for production. Do not import the legacy Synara application or
upstream implementation code to perform the spike.

## Executable policy

`foundations/browser/lib.rs` is a dependency-free Rust policy foundation with an
independent test target. It intentionally does not modify the shared Cargo
workspace while BCD/AJM/FGH are running. It is NOT wired into the application.

The implemented policy provides bounded tabs and consent records, distinct manual,
agent-task and authentication contexts, document generations, one-operation
grants, expiry, revocation on navigation/close/crash, and stale-callback rejection.
An agent cannot access manual or authentication tabs through this API. Sharing an
existing manual tab requires a future explicit sharing flow, never a silent
context change. Restart begins with no grants.

## Native adapter obligations

1. Parse raw URLs with the platform browser or a standards-compliant URL library.
   `Origin::from_canonical_parts` is only an additional shape validator, not a URL
   parser. Obtain the origin from authoritative native navigation/frame callbacks,
   not page JavaScript, a display title or an agent-provided hostname.
2. Bind a task/actor to the authenticated controller command source. A web message
   cannot claim an arbitrary task ID or resolve its own permission prompt.
3. Keep exact command payloads immutable in a native pending-command record keyed
   by RequestId. The current policy binds action/category, task, tab, document and
   origin, not arbitrary input bytes or download destinations. Never accept a new
   payload alongside an already-approved grant. Validate and consume the grant
   immediately before dispatching the frozen operation.
4. Initial navigation of a blank tab currently needs a separately reviewed native
   host operation. The policy's agent request path requires a committed document.
   Extend and test blank-document navigation consent before exposing it to agents.
5. Call begin_navigation for top-level provisional navigation/reload/history
   replacement and redirects. Commit only the current generation's final origin.
   Update browser permissions and remove privileged bridges on navigation.
6. Use a monotonic clock for now_ms. Bound command bodies, response size, timeout,
   concurrency and IPC frames independently of the policy's record-count bounds.
7. Keep browser profiles/cookies for manual browsing, agent tasks and authentication
   separate. Do not export cookies, passwords, OAuth codes or auth screenshots to
   agents or diagnostic reports. An authenticated browsing session needs explicit
   sharing and clear visibility before an agent can use it.
8. Route manual URL-bar input, links, back/forward, reload, stop, tab creation,
   downloads, popups and native browser permissions through reviewed host paths.
   Support ordinary HTTP(S) browsing and local development explicitly. Privileged
   schemes, file URLs and OS-handler launches need separate policy. Page content
   must not receive general filesystem, process, credential or arbitrary RPC access.
9. Media, clipboard, file upload, downloads and screenshots each require their own
   scope. A page's browser permission is not an agent approval, or the reverse.
10. On crash, disconnect or task shutdown, revoke grants, fail queued requests and
    release native resources. Hiding a pane or switching Zen/Synaric must not
    recreate a browser or grant previously denied access.

## Acceptance still open

A real native browsing surface, tabs/history UI, DOM/capture/input adapter,
permission cards, initial agent navigation, authenticated-browser sharing,
profile persistence, downloads, OAuth integration and platform-level security
checks remain open. Policy unit tests do not close K1-K5 or prove browser sandboxing.
The terminal or device helper must not become a generic privileged browser bridge.

## Test command

```sh
rustc --edition 2024 -D warnings --test foundations/browser/lib.rs -o /tmp/browser-policy-tests
/tmp/browser-policy-tests
```

The EKOP workflow runs this test natively on macOS arm64, Windows x64 and Linux x64.
Read actual job results before claiming a platform passed.

## Primary references inspected

- Apple WKWebView API boundary:
  https://developer.apple.com/documentation/webkit/wkwebview
- Microsoft WebView2 security guidance, particularly origin validation, navigation
  callbacks and narrowly scoped native messages:
  https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/security

The adapter obligations and policy above are Synara design decisions. They are
not a claim that those platforms implement the complete Synara policy for us.
