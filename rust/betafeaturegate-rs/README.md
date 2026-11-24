# betafeaturegate-rs

FILE: betaFeatureGate.ts Purpose: The server's read of the shared Beta-only feature list. Layer: Runtime gate (consumed by settings projection and provider messaging) Exports: SERVER_DESKTOP_FLAVOR, isServerBetaFeatureEnabled

napi-rs bridge for `apps/server/src/betaFeatureGate.ts`.

## Functions

- `isServerBetaFeatureEnabled` (`apps/server/src/betaFeatureGate.ts:14`)