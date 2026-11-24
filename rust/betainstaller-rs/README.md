# betainstaller-rs

FILE: betaInstaller.ts Purpose: Download, verify, and install Synara Beta on macOS from a          generic update feed (beta-mac.yml + the zip it lists). Layer: Desktop platform adapter (no Electron imports; I/O is injectable for tests).

napi-rs bridge for `apps/desktop/src/betaInstaller.ts`.

## Functions

- `parseBetaMacManifest` (`apps/desktop/src/betaInstaller.ts:82`)
- `selectBetaDownloadFile` (`apps/desktop/src/betaInstaller.ts:128`)
- `httpsFetchText` (`apps/desktop/src/betaInstaller.ts:181`)
- `sha512FileBase64` (`apps/desktop/src/betaInstaller.ts:255`)
- `httpsDownloadFile` (`apps/desktop/src/betaInstaller.ts:266`)
- `verifyBetaAppBundle` (`apps/desktop/src/betaInstaller.ts:379`)