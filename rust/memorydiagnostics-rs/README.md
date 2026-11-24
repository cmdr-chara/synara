# memorydiagnostics-rs

FILE: memoryDiagnostics.ts Purpose: Emits low-volume backend memory counters for diagnosing desktop OOM crashes. Layer: Server runtime observability helper Exports: memory snapshot formatting and desktop diagnostic timer setup.

napi-rs bridge for `apps/server/src/memoryDiagnostics.ts`.

## Functions

- `readServerMemoryDiagnosticSnapshot` (`apps/server/src/memoryDiagnostics.ts:45`)
- `shouldWarnServerMemory` (`apps/server/src/memoryDiagnostics.ts:74`)