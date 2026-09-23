# Native parity continuation — September 23, 2026

Upstream reference: `Emanuele-web04/synara@eaa61eded31b6755d4f30ba8eabc5d905cf817cb`.
Starting native reference: `000461a5eb1980a59a3a9befc8e6ef07b4153cac`.
Concurrent editor-refresh and context-usage work through `4e06194` was preserved,
not attributed to the browser/command implementation.

## Capture checkpoint

`a6a852ec20b1453417642d08b539c42640e7fd90` added manual bounded viewport-image
copy and inspector teardown. Native WebKit run **35876331487** passed browser
library/history tests, real consent/DOM/isolation/redirect acceptance, the real
capture/revoked-result test, the GPUI application build, native browser smoke and
focused workspace services. Formatting failed in three files, preventing strict
Clippy from running in that candidate. `e3f151b5af6f5f0f4f9535500cc8a8696ff5de3b`
applied the exact formatter corrections without changing behavior.

## Scroll and native commands

The continuation adds approved page scrolling with fresh-reference requirements
and a qualified native command layer reusing existing workflow owners. One parser
regression is new. The existing real WebKit and native chat utility journeys are
extended, not replaced. Exact candidate hashes and final run outcomes are recorded
in the completion receipt beside this file after validation. Until that receipt is
present, this section makes no final integrated test-pass or release-ready claim.

No live provider, macOS, Windows, Wayland, hardware, saved-login, WebMCP or production
release acceptance is inferred from private Linux fixture tests. No historical A-Q
checkbox or missing product surface is closed by this continuation.
