# codexconfig-rs

Codex config helpers. Parses the small subset of `CODEX_HOME/config.toml` we need for provider discovery without pulling in a full TOML dependency.

napi-rs bridge for `packages/shared/src/codexConfig.ts`.

## Functions

- `parseCodexConfigModelProvider` (`packages/shared/src/codexConfig.ts:23`)
- `parseCodexConfigProviderEnvKey` (`packages/shared/src/codexConfig.ts:41`)
- `parseCodexConfigActiveProviderEnvKey` (`packages/shared/src/codexConfig.ts:65`)
- `resolveCodexHome` (`packages/shared/src/codexConfig.ts:74`)
- `readCodexConfigContent` (`packages/shared/src/codexConfig.ts:79`)
- `readActiveCodexProviderEnvKey` (`packages/shared/src/codexConfig.ts:88`)