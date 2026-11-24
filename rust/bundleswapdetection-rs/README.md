# bundleswapdetection-rs

FILE: bundleSwapDetection.ts Purpose: Detects the packaged app.asar being replaced on disk beneath a running app. Layer: Desktop update utility  Electron caches the asar header per process, so once app.asar is swapped on disk (updater retry, manual reinstall, a local build copied over the bundle) every subsequent archive read resolves to stale offsets and returns bytes from the wrong file.

napi-rs bridge for `apps/desktop/src/bundleSwapDetection.ts`.

## Functions

- `bundleSignatureFromStats` (`apps/desktop/src/bundleSwapDetection.ts:27`)
- `isWatchableBundlePath` (`apps/desktop/src/bundleSwapDetection.ts:35`)
- `isBundleSwapped` (`apps/desktop/src/bundleSwapDetection.ts:45`)
- `isBundleStable` (`apps/desktop/src/bundleSwapDetection.ts:63`)