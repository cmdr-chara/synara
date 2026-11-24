# betadiagnostics-rs

FILE: betaDiagnostics.ts Purpose: Diagnostics queue for Synara Beta and Synara Remote desktop builds. Layer: Desktop telemetry (runs only when the baked build flavor is "beta" or "remote"; Remote follows the user's usage-sharing switch).

napi-rs bridge for `apps/desktop/src/betaDiagnostics.ts`.

## Functions

- `sanitizeBetaDiagnosticsPayload` (`apps/desktop/src/betaDiagnostics.ts:210`)
- `resolveBetaDiagnosticsEndpoint` (`apps/desktop/src/betaDiagnostics.ts:273`)