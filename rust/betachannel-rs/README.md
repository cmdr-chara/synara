# betachannel-rs

FILE: betaChannel.ts Purpose: Stable-side probe + handoff into a parallel Synara Beta install. Layer: Desktop platform adapter (no Electron imports; every path is injectable for tests).

napi-rs bridge for `apps/desktop/src/betaChannel.ts`.

## Functions

- `spawnDetached` (`apps/desktop/src/betaChannel.ts:109`)
- `betaLaunchEnvironment` (`apps/desktop/src/betaChannel.ts:143`)
- `stableLaunchEnvironment` (`apps/desktop/src/betaChannel.ts:163`)
- `detectStableExecutable` (`apps/desktop/src/betaChannel.ts:174`)
- `betaMacInstallDirs` (`apps/desktop/src/betaChannel.ts:205`)
- `detectBetaInstall` (`apps/desktop/src/betaChannel.ts:233`)
- `isBetaServerRunning` (`apps/desktop/src/betaChannel.ts:337`)
- `readBetaImportResult` (`apps/desktop/src/betaChannel.ts:353`)
- `writeBetaImportRequest` (`apps/desktop/src/betaChannel.ts:373`)
- `launchBetaInstall` (`apps/desktop/src/betaChannel.ts:398`)
- `resolveBetaHomeDir` (`apps/desktop/src/betaChannel.ts:620`)