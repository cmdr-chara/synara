# backendsupervisionpolicy-rs

FILE: backendSupervisionPolicy.ts Purpose: Pure crash-supervision policy for the desktop backend: restart backoff,          the give-up circuit breaker, and the output tail used to explain it. Layer: Desktop backend supervision Exports: BackendSupervisionPolicy, BackendOutputTailDetector, summarizeBackendFailureOutput

napi-rs bridge for `apps/desktop/src/backendSupervisionPolicy.ts`.

## Functions

- `backendRestartDelayMs` (`apps/desktop/src/backendSupervisionPolicy.ts:28`)
- `summarizeBackendFailureOutput` (`apps/desktop/src/backendSupervisionPolicy.ts:168`)