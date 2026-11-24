# electronupdatersecurity-rs

FILE: electronUpdaterSecurity.ts Purpose: Hardens electron-updater Windows process calls against Node deprecations. Layer: Desktop update runtime Exports: updater patching, shell-free PowerShell signature verification helpers.

napi-rs bridge for `apps/desktop/src/electronUpdaterSecurity.ts`.

## Functions

- `buildPowerShellExecutablePath` (`apps/desktop/src/electronUpdaterSecurity.ts:65`)
- `buildPowerShellExecArgs` (`apps/desktop/src/electronUpdaterSecurity.ts:69`)
- `resolveWindowsUpdatePublisherNames` (`apps/desktop/src/electronUpdaterSecurity.ts:244`)
- `hardenElectronUpdater` (`apps/desktop/src/electronUpdaterSecurity.ts:256`)