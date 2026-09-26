# Parity continuation verification - batch 35

Date: 2026-09-26

## Accepted

### A09 Multi-provider ACP/direct-model interoperability matrix

GitHub Actions run `36194519926`, job `108267198945`, completed successfully
for exact candidate `fd41caa5d7077d9176afe749b2c2fe3f0cb57c03`.

The dedicated Direct conversations acceptance lane proved:

- reviewed external OpenCode and Gemini CLI ACP releases initialized successfully
  without user credentials;
- the native Google direct-model route completed its SSE schema/usage journey
  without creating an ACP session;
- the native Anthropic route streamed usage correctly and returned to ACP while
  preserving the conversation;
- explicit direct targeting affected only the new task and did not transfer an
  ACP session;
- provider tabs remained inert until an explicit selection;
- the representative interoperability matrix reported `status: passed`.

The same job passed the relevant backend contracts and native application build
before running the provider journeys.

This is the deciding evidence for N5 Multi-provider workspace, which is now PASS.

## Product completion

### M08 Complete browser popup authentication lifecycle

The branch now routes provider website requests into request-owned
`BrowserProfile::Authentication` flows. Authentication cookies remain isolated
from Manual and AgentTask partitions while remaining continuous inside one
reviewed sign-in flow.

Authentication popup requests are captured without granting page-owned
navigation. Explicit host review opens the child inside the same authentication
partition, and completion, decline, cancellation or expiry closes the owned
authentication tabs. Agent-task popups remain denied.

The implementation is represented by the target-branch chain beginning with
`c916fed2f91e607da9de9b8748e171b6a8ebec5b` and
`e0f034958127d959300923adfcc687375651a702`, with real WebKit lifecycle coverage
added in `f815797f523f115c669d4ecc2dd1ae34a500fc74` and subsequent reviewed-popup
policy fixes through branch head `ed61115c005cb9d837fadc6af9c303e086551c34`.

M08 is therefore complete at the product-feature level.

## Still open

### A05 Authenticated browser login/session/popup acceptance

A05 is not accepted. Branch-head Native WebKit run `36195025196`, job
`108268868564`, passed focused browser tests, formatting/strict Clippy, the
native application build and the GPUI browser rendering/input smoke, but the real
WebKit authentication journey failed.

The deciding failure is:

`trusted X11 click did not reach the authentication WebKit page`

from
`native::tests::real_webkit_navigation_consent_input_redirect_and_isolation`.

A05 stays OPEN until the real authentication journey passes on an exact current
candidate. D2 also stays OPEN because M07, M09, M11 and M12 remain product-depth
gaps.

## Inventory effect

- M08 complete: major remaining 16 -> 15.
- A09 accepted: acceptance/integration remaining 6 -> 5.
- Execution total: 22 -> 20.
- N5 PASS: broad verification gates still open 21 -> 20.
