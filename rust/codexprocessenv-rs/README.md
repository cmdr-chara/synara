# codexprocessenv-rs

FILE: codexProcessEnv.ts Purpose: Builds the exact environment used when Synara launches Codex subprocesses. Layer: Server runtime utility Exports: Codex process env builder and browser-plugin overlay helpers.

napi-rs bridge for `apps/server/src/codexProcessEnv.ts`.

## Functions

- `buildCodexAppServerArgs` (`apps/server/src/codexProcessEnv.ts:194`)
- `readEffectiveCodexAuthCredentialsStoreMode` (`apps/server/src/codexProcessEnv.ts:332`)
- `readCodexPreparedHomeFileSnapshot` (`apps/server/src/codexProcessEnv.ts:566`)
- `readCodexPreparedAuthTrackingFingerprint` (`apps/server/src/codexProcessEnv.ts:779`)
- `readSynaraConfigSuppressions` (`apps/server/src/codexProcessEnv.ts:1012`)
- `disableCodexConfigSections` (`apps/server/src/codexProcessEnv.ts:1037`)
- `disableCompetingCodexBrowserPluginsInConfig` (`apps/server/src/codexProcessEnv.ts:1108`)
- `linkOrCopyCodexOverlayEntry` (`apps/server/src/codexProcessEnv.ts:1136`)
- `prioritizeCodexOverlayEntries` (`apps/server/src/codexProcessEnv.ts:1159`)
- `extractManagedCodexConfigSection` (`apps/server/src/codexProcessEnv.ts:2270`)
- `configHasTomlTableHeader` (`apps/server/src/codexProcessEnv.ts:2402`)
- `mergeShellEnvPolicyExclude` (`apps/server/src/codexProcessEnv.ts:2521`)