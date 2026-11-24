# explorerrevealrequeststore-rs

FILE: explorerRevealRequestStore.ts Purpose: Lets surfaces outside the right dock (e.g. the Cmd+P search palette)          ask a thread's explorer pane to reveal a directory in its file tree. Mirrors composerFocusRequestStore: a per-thread request with a          monotonic nonce, consumed by the pane whenever it is (or becomes)          mounted.

napi-rs bridge for `apps/web/src/explorerRevealRequestStore.ts`.

## Functions

- `requestExplorerReveal` (`apps/web/src/explorerRevealRequestStore.ts:36`)
- `directoryChain` (`apps/web/src/explorerRevealRequestStore.ts:44`)