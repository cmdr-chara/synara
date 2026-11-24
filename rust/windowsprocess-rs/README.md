# windowsprocess-rs

FILE: windowsProcess.ts Purpose: Prepares Windows child-process launches without Node's `shell: true`. Layer: Shared Node runtime utility Exports: command resolution plus safe spawn/spawnSync argument preparation.

napi-rs bridge for `packages/shared/src/windowsProcess.ts`.

## Functions

- `parseWindowsWslUncPath` (`packages/shared/src/windowsProcess.ts:42`)
- `isWindowsBatchCommand` (`packages/shared/src/windowsProcess.ts:64`)
- `buildWindowsBatchCommandArgs` (`packages/shared/src/windowsProcess.ts:77`)
- `resolveWindowsCommandPath` (`packages/shared/src/windowsProcess.ts:100`)
- `prepareWindowsSafeProcess` (`packages/shared/src/windowsProcess.ts:119`)