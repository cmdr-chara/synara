# automationmode-rs

FILE: automationMode.ts Purpose: Single source for what an AutomationMode implies about thread ownership. Layer: Shared runtime utility (server dispatch/scheduling + web forms) Exports: mode predicates and the continuation-thread resolver.

napi-rs bridge for `packages/shared/src/automationMode.ts`.

## Functions

- `automationContinuesThread` (`packages/shared/src/automationMode.ts:19`)
- `automationOwnsItsThread` (`packages/shared/src/automationMode.ts:24`)
- `automationRequiresTargetThread` (`packages/shared/src/automationMode.ts:29`)
- `automationContinuationThreadId` (`packages/shared/src/automationMode.ts:38`)
- `isAutomationRunThread` (`packages/shared/src/automationMode.ts:50`)