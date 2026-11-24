# automationrunlifecycle-rs

FILE: automationRunLifecycle.ts Purpose: Single declaration of the automation run state machine. Layer: Shared runtime utility (server reconciliation/persistence guards + web run rows) Exports: the transition table, the active/terminal partitions, and their predicates.

napi-rs bridge for `packages/shared/src/automationRunLifecycle.ts`.

## Functions

- `isActiveAutomationRunStatus` (`packages/shared/src/automationRunLifecycle.ts:81`)
- `isTerminalAutomationRunStatus` (`packages/shared/src/automationRunLifecycle.ts:88`)
- `canTransitionAutomationRun` (`packages/shared/src/automationRunLifecycle.ts:94`)
- `automationRunStatusesLeadingTo` (`packages/shared/src/automationRunLifecycle.ts:102`)