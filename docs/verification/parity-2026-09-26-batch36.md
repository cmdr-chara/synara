# Parity continuation verification - batch 36

Date: 2026-09-26

## Accepted

### A05 Authenticated browser login/session/popup acceptance

GitHub Actions run `36204782354`, job `108298878329`, completed successfully
for exact candidate `63c507a7e1f0a4651ae4b8009c502ecac7c27641`.

The dedicated acceptance lane used real WebKitGTK under an isolated X11 display
and passed all deciding checks:

- the domain-level authentication popup test retained the exact
  `BrowserProfile::Authentication { flow }` partition and rejected AgentTask
  popup authority;
- a real login page set an HttpOnly authentication cookie;
- an OAuth-style `window.open` request was denied as an unmanaged native window
  and surfaced to the trusted host instead;
- the trusted host explicitly reopened that popup in the same authentication
  flow partition;
- the popup request received the login cookie and set a second OAuth completion
  cookie;
- the original authentication tab subsequently observed both cookies;
- a Manual-profile tab visiting the same server observed neither authentication
  cookie, proving storage isolation;
- popup preview redacted query/fragment values before display;
- browser formatting and strict Clippy passed;
- the workflow verified that the checked-out source remained the exact candidate.

The retained artifact is `authenticated-browser-acceptance`
(artifact id `10892988432`).

## Scope boundary

A05 proves the implemented authenticated native browser partition, cookie-backed
session continuity and explicit popup handoff on real WebKitGTK. It does not
claim the broader browser depth work is complete. Protected cookie/session
import, the full product-facing popup-auth lifecycle and durable restoration of
task/auth browser sessions remain M07, M08 and M11.

## Still open

A01, A04, A08, A09 and A10 remain open.
