# desktopmigrationrecovery-rs

FILE: desktopMigrationRecovery.ts Purpose: Detects pending desktop migration recovery and invokes the server-owned restore CLI. Layer: Desktop startup utility

napi-rs bridge for `apps/desktop/src/desktopMigrationRecovery.ts`.

## Functions

- `resolveDesktopMigrationRecoveryPaths` (`apps/desktop/src/desktopMigrationRecovery.ts:26`)
- `invalidMigrationStartupRecoveryChoices` (`apps/desktop/src/desktopMigrationRecovery.ts:59`)
- `hasPendingDesktopMigrationRecovery` (`apps/desktop/src/desktopMigrationRecovery.ts:172`)
- `hasVerifiedDesktopMigrationRestore` (`apps/desktop/src/desktopMigrationRecovery.ts:176`)
- `resolveDesktopMigrationRestoreCandidate` (`apps/desktop/src/desktopMigrationRecovery.ts:206`)
- `requiresDesktopMigrationRecovery` (`apps/desktop/src/desktopMigrationRecovery.ts:226`)