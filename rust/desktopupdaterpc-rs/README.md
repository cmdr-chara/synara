# desktopupdaterpc-rs

Purpose: Admits owner shells and approved remote devices to desktop updater RPCs. Layer: Server transport; resource leases and provider credentials cannot grant this authority.

napi-rs bridge for `apps/server/src/desktopUpdateRpc.ts`.

## Functions

- `hasDesktopUpdateThreadWork` (`apps/server/src/desktopUpdateRpc.ts:38`)
- `desktopUpdateAuthorityForSession` (`apps/server/src/desktopUpdateRpc.ts:100`)