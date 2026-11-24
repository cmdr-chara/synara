# servershutdown-rs

Only the authorization helpers port: the route path constants and the Effect-based
`makeServerShutdownController` are not function declarations the bridge captures. `isLoopbackHost` (`startupAccess.ts`) is inlined: `authorizeDesktopShutdown` calls
it on a raw JSON value, where a truthy non-string host raises a TypeError the
`Option<String>` signature of `startupaccess-rs` cannot express.

napi-rs bridge for `apps/server/src/serverShutdown.ts`.

## Functions

- `matchesDesktopShutdownToken` (`apps/server/src/serverShutdown.ts:50`)
- `isDesktopShutdownLoopbackPeer` (`apps/server/src/serverShutdown.ts:54`)
- `authorizeDesktopShutdown` (`apps/server/src/serverShutdown.ts:70`)