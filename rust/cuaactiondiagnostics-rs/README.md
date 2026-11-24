# cuaactiondiagnostics-rs

Driver diagnostics are persisted in logs. Only fixed vocabulary and numeric native error/identity fields may cross this boundary; never copy native error messages, labels, element values, titles or arbitrary structured payloads.

napi-rs bridge for `packages/shared/src/cuaActionDiagnostics.ts`.

## Functions

- `parseCuaActionDiagnostics` (`packages/shared/src/cuaActionDiagnostics.ts:59`)
- `cuaActionDiagnosticMessage` (`packages/shared/src/cuaActionDiagnostics.ts:87`)