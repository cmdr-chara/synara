# chatrouterestore-rs

FILE: chatRouteRestore.ts Purpose: Validates saved chat routes before restoring them from startup or sidebar navigation. Layer: Route helper Exports: last-thread route resolver plus empty-startup fallback policy helpers.

napi-rs bridge for `apps/web/src/chatRouteRestore.ts`.

## Functions

- `resolveRestorableThreadRoute` (`apps/web/src/chatRouteRestore.ts:15`)
- `shouldStartRememberedRouteRecovery` (`apps/web/src/chatRouteRestore.ts:41`)
- `shouldHoldRememberedRouteFallback` (`apps/web/src/chatRouteRestore.ts:51`)
- `shouldStartMissingThreadRouteRecovery` (`apps/web/src/chatRouteRestore.ts:61`)
- `shouldHoldMissingThreadRouteFallback` (`apps/web/src/chatRouteRestore.ts:69`)