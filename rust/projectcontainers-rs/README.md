# projectcontainers-rs

FILE: projectContainers.ts Purpose: Classify system-managed project containers versus ordinary projects. Managed          chat and Studio containers carry their own kind, but the legacy Home chat          container kept `kind: "project"` and is recognized by its row shape: the          canonical "Home" title plus the reserved chat/home workspace root.

napi-rs bridge for `packages/shared/src/projectContainers.ts`.

## Functions

- `resolveChatContainerWorkspaceRoot` (`packages/shared/src/projectContainers.ts:22`)
- `matchesLegacyHomeChatWorkspaceRoot` (`packages/shared/src/projectContainers.ts:32`)
- `isLegacyHomeChatContainerRow` (`packages/shared/src/projectContainers.ts:55`)
- `isGroupContainerKind` (`packages/shared/src/projectContainers.ts:71`)
- `isOrdinaryProjectRow` (`packages/shared/src/projectContainers.ts:80`)