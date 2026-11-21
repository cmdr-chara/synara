# Rust migration plan: synara (TypeScript/JavaScript)

13 packages, 2405 source files (873907 lines), 1609 test files, 973 UI files, grouped into 881 units.

| Ready | Caution | Blocked | Keep in source language |
| ----: | ------: | ------: | ----------------------: |
|   572 |     217 |       5 |                      87 |

## Start here

Best first ports: safe to port (readiness) and worth porting (loops and string work, tests, size).

|   # | Unit                                       | Lines | Priority | Readiness | Tests | Bridge  | Main risks                                                                           |
| --: | ------------------------------------------ | ----: | -------: | --------: | ----: | ------- | ------------------------------------------------------------------------------------ |
|   1 | `packages/shared/src/toolOutputSummary.ts` |    69 |       92 |        97 |     1 | napi-rs | String offsets and slicing (6)                                                       |
|   2 | `apps/desktop/src/backendIssueDetector.ts` |    36 |       90 |        96 |     1 | napi-rs | String offsets and slicing (3); Callback-shaped APIs (1); Exceptions (1)             |
|   3 | `packages/shared/src/githubRepository.ts`  |    72 |       89 |        85 |     1 | napi-rs | String offsets and slicing (8); Regex features the `regex` crate lacks (3)           |
|   4 | `apps/web/src/providerOrdering.ts`         |    59 |       86 |        80 |     2 | napi-rs | String offsets and slicing (4); Insertion-ordered maps and object keys (3)           |
|   5 | `packages/shared/src/projectFolders.ts`    |   114 |       85 |       100 |     1 | napi-rs | String offsets and slicing (1); Insertion-ordered maps and object keys (2)           |
|   6 | `apps/server/src/codexTurnInput.ts`        |    42 |       84 |       100 |     1 | napi-rs | -                                                                                    |
|   7 | `packages/shared/src/threadHierarchy.ts`   |    48 |       84 |       100 |     1 | napi-rs | Insertion-ordered maps and object keys (2)                                           |
|   8 | `apps/desktop/src/windowsShellIco.ts`      |   132 |       84 |       100 |     1 | napi-rs | Insertion-ordered maps and object keys (1); Callback-shaped APIs (1); Exceptions (1) |
|   9 | `apps/desktop/src/backendStartupBlock.ts`  |   134 |       84 |        93 |     1 | napi-rs | String offsets and slicing (6)                                                       |
|  10 | `packages/shared/src/agentMentions.ts`     |   134 |       83 |        98 |     1 | napi-rs | String offsets and slicing (3)                                                       |

### `packages/shared/src/toolOutputSummary.ts`

69 lines in 1 files, wave 1, priority 92, readiness 97, bridge **napi-rs**.

- 1 test files import it directly.
- String offsets and slicing x6: `packages/shared/src/toolOutputSummary.ts:24`, `packages/shared/src/toolOutputSummary.ts:26`, `packages/shared/src/toolOutputSummary.ts:26`

### `apps/desktop/src/backendIssueDetector.ts`

36 lines in 1 files, wave 1, priority 90, readiness 96, bridge **napi-rs**.

- 1 test files import it directly.
- String offsets and slicing x3: `apps/desktop/src/backendIssueDetector.ts:17`, `apps/desktop/src/backendIssueDetector.ts:23`, `apps/desktop/src/backendIssueDetector.ts:28`
- Callback-shaped APIs x1: `apps/desktop/src/backendIssueDetector.ts:10`
- Exceptions x1: `apps/desktop/src/backendIssueDetector.ts:27`
- JSON round-trips x1: `apps/desktop/src/backendIssueDetector.ts:28`

### `packages/shared/src/githubRepository.ts`

72 lines in 1 files, wave 1, priority 89, readiness 85, bridge **napi-rs**.

- 1 test files import it directly.
- String offsets and slicing x8: `packages/shared/src/githubRepository.ts:4`, `packages/shared/src/githubRepository.ts:5`, `packages/shared/src/githubRepository.ts:7`
- Regex features the `regex` crate lacks x3: `packages/shared/src/githubRepository.ts:24`, `packages/shared/src/githubRepository.ts:39`, `packages/shared/src/githubRepository.ts:52`

### `apps/web/src/providerOrdering.ts`

59 lines in 1 files, wave 0, priority 86, readiness 80, bridge **napi-rs**.

- Leaf: imports no other unit.
- 2 test files import it directly.
- String offsets and slicing x4: `apps/web/src/providerOrdering.ts:60`, `apps/web/src/providerOrdering.ts:61`, `apps/web/src/providerOrdering.ts:63`
- Insertion-ordered maps and object keys x3: `apps/web/src/providerOrdering.ts:13`, `apps/web/src/providerOrdering.ts:20`, `apps/web/src/providerOrdering.ts:32`

### `packages/shared/src/projectFolders.ts`

114 lines in 1 files, wave 1, priority 85, readiness 100, bridge **napi-rs**.

- 1 test files import it directly.
- String offsets and slicing x1: `packages/shared/src/projectFolders.ts:82`
- Insertion-ordered maps and object keys x2: `packages/shared/src/projectFolders.ts:13`, `packages/shared/src/projectFolders.ts:76`

## Order by package

Packages come after the workspace packages they use. Inside a package, port by wave: a unit's imports are in earlier waves.

### 1. @synara/monorepo (`.`, node, 0 test files)

| Wave | Unit                              | Status  | Lines | Readiness | Bridge  | Main risks                                                                  |
| ---: | --------------------------------- | ------- | ----: | --------: | ------- | --------------------------------------------------------------------------- |
|    0 | `vitest.shards.ts`                | ready   |    22 |        89 | napi-rs | Async and promises (2); Class inheritance (1)                               |
|    0 | `benchmarks/desktop-perf/lib/`    | caution |  2164 |         0 | napi-rs | String offsets and slicing (18); Regexes built at runtime (1)               |
|    1 | `benchmarks/desktop-perf/`        | caution |  2664 |         0 | napi-rs | String offsets and slicing (46); Regex features the `regex` crate lacks (3) |
|    4 | `docs/computer-use-cua/evidence/` | caution |   777 |        29 | napi-rs | String offsets and slicing (13); Async and promises (63)                    |

### 2. @synara/contracts (`packages/contracts`, portable, 676 test files)

| Wave | Unit                                                       | Status  | Lines | Readiness | Bridge       | Main risks                                                                 |
| ---: | ---------------------------------------------------------- | ------- | ----: | --------: | ------------ | -------------------------------------------------------------------------- |
|    0 | `packages/contracts/src/browserAutomationToolCatalogue.ts` | ready   |    16 |       100 | wasm-bindgen | -                                                                          |
|    0 | `packages/contracts/src/composerSlashCommands.ts`          | ready   |    23 |       100 | wasm-bindgen | -                                                                          |
|    0 | `packages/contracts/src/uiLocale.ts`                       | ready   |     3 |       100 | wasm-bindgen | -                                                                          |
|    0 | `packages/contracts/src/hostUsage.ts`                      | ready   |    47 |        99 | wasm-bindgen | -                                                                          |
|    0 | `packages/contracts/src/computerSpaces.ts`                 | ready   |    52 |        97 | wasm-bindgen | -                                                                          |
|    0 | `packages/contracts/src/computerBrowser.ts`                | ready   |   106 |        96 | wasm-bindgen | -                                                                          |
|    0 | `packages/contracts/src/browserAutomationToolCommon.ts`    | ready   |    14 |        94 | wasm-bindgen | -                                                                          |
|    0 | `packages/contracts/src/inlineVisualSpec.ts`               | ready   |   242 |        92 | wasm-bindgen | -                                                                          |
|    0 | `packages/contracts/src/reliability.ts`                    | ready   |    71 |        91 | wasm-bindgen | String offsets and slicing (1)                                             |
|    0 | `packages/contracts/src/browserAutomationBounds.ts`        | ready   |    19 |        87 | wasm-bindgen | Exceptions (1)                                                             |
|    0 | `packages/contracts/src/baseSchemas.ts`                    | ready   |    84 |        83 | wasm-bindgen | -                                                                          |
|    1 | `packages/contracts/src/computerAudit.ts`                  | ready   |    48 |       100 | wasm-bindgen | -                                                                          |
|    1 | `packages/contracts/src/todo.ts`                           | ready   |   109 |        97 | wasm-bindgen | -                                                                          |
|    1 | `packages/contracts/src/environment.ts`                    | ready   |    42 |        96 | wasm-bindgen | -                                                                          |
|    1 | `packages/contracts/src/liveActivities.ts`                 | ready   |   114 |        96 | wasm-bindgen | -                                                                          |
|    1 | `packages/contracts/src/remoteResources.ts`                | ready   |    53 |        96 | wasm-bindgen | -                                                                          |
|    1 | `packages/contracts/src/asyncUserInput.ts`                 | ready   |    24 |        95 | wasm-bindgen | -                                                                          |
|    1 | `packages/contracts/src/claudeCache.ts`                    | ready   |    30 |        95 | wasm-bindgen | -                                                                          |
|    1 | `packages/contracts/src/hostSessions.ts`                   | ready   |    33 |        95 | wasm-bindgen | -                                                                          |
|    1 | `packages/contracts/src/auth.ts`                           | ready   |   125 |        94 | wasm-bindgen | -                                                                          |
|    1 | `packages/contracts/src/desktopUpdate.ts`                  | ready   |    71 |        94 | wasm-bindgen | -                                                                          |
|    1 | `packages/contracts/src/filesystem.ts`                     | ready   |    21 |        93 | wasm-bindgen | -                                                                          |
|    1 | `packages/contracts/src/hostConnection.ts`                 | ready   |    97 |        93 | wasm-bindgen | -                                                                          |
|    1 | `packages/contracts/src/inlineVisual.ts`                   | ready   |    53 |        93 | wasm-bindgen | -                                                                          |
|    1 | `packages/contracts/src/keybindings.ts`                    | ready   |   214 |        93 | wasm-bindgen | -                                                                          |
|    1 | `packages/contracts/src/studio.ts`                         | ready   |    28 |        93 | wasm-bindgen | -                                                                          |
|    1 | `packages/contracts/src/browserAnnotations.ts`             | ready   |   130 |        92 | wasm-bindgen | Callback-shaped APIs (6)                                                   |
|    1 | `packages/contracts/src/pullRequestAutoFix.ts`             | ready   |    63 |        92 | wasm-bindgen | -                                                                          |
|    1 | `packages/contracts/src/browserAutomationIds.ts`           | ready   |    71 |        91 | wasm-bindgen | -                                                                          |
|    1 | `packages/contracts/src/browserAutomationJson.ts`          | ready   |    35 |        91 | wasm-bindgen | Insertion-ordered maps and object keys (1); Exceptions (1)                 |
|    1 | `packages/contracts/src/editorDebug.ts`                    | ready   |   122 |        91 | wasm-bindgen | -                                                                          |
|    1 | `packages/contracts/src/editorIntelligence.ts`             | ready   |   125 |        91 | wasm-bindgen | -                                                                          |
|    1 | `packages/contracts/src/mcpConnectors.ts`                  | ready   |   134 |        91 | wasm-bindgen | -                                                                          |
|    1 | `packages/contracts/src/providerInstance.ts`               | ready   |    69 |        89 | wasm-bindgen | -                                                                          |
|    1 | `packages/contracts/src/browserVault.ts`                   | ready   |    91 |        88 | wasm-bindgen | Callback-shaped APIs (14)                                                  |
|    1 | `packages/contracts/src/editor.ts`                         | ready   |   252 |        86 | wasm-bindgen | -                                                                          |
|    1 | `packages/contracts/src/project.ts`                        | ready   |   311 |        85 | wasm-bindgen | -                                                                          |
|    1 | `packages/contracts/src/device.ts`                         | ready   |   631 |        84 | wasm-bindgen | -                                                                          |
|    1 | `packages/contracts/src/browserAutomationCssSelector.ts`   | ready   |   344 |        76 | wasm-bindgen | String offsets and slicing (6); Insertion-ordered maps and object keys (5) |
|    1 | `packages/contracts/src/account.ts`                        | ready   |   648 |        75 | wasm-bindgen | Regex features the `regex` crate lacks (1)                                 |
|    1 | `packages/contracts/src/computer.ts`                       | ready   |  1223 |        69 | wasm-bindgen | -                                                                          |
|    2 | `packages/contracts/src/terminal.ts`                       | ready   |   165 |        96 | wasm-bindgen | -                                                                          |
|    2 | `packages/contracts/src/accountUsage.ts`                   | ready   |   236 |        94 | wasm-bindgen | -                                                                          |
|    2 | `packages/contracts/src/usageLimit.ts`                     | ready   |    41 |        94 | wasm-bindgen | -                                                                          |
|    2 | `packages/contracts/src/accountBilling.ts`                 | ready   |    40 |        93 | wasm-bindgen | -                                                                          |
|    2 | `packages/contracts/src/browserAutomationTargets.ts`       | ready   |   109 |        93 | wasm-bindgen | -                                                                          |
|    2 | `packages/contracts/src/hostAuth.ts`                       | ready   |   270 |        92 | wasm-bindgen | -                                                                          |
|    2 | `packages/contracts/src/projectImport.ts`                  | ready   |    87 |        92 | wasm-bindgen | -                                                                          |
|    2 | `packages/contracts/src/browserAutomationErrors.ts`        | ready   |   279 |        85 | wasm-bindgen | Reflection and prototypes (3); Insertion-ordered maps and object keys (1)  |
|    2 | `packages/contracts/src/productAnalytics.ts`               | ready   |   191 |        81 | wasm-bindgen | String offsets and slicing (1); Callback-shaped APIs (3)                   |
|    3 | `packages/contracts/src/agentMentions.ts`                  | ready   |   289 |        93 | wasm-bindgen | Insertion-ordered maps and object keys (7); Exceptions (1)                 |
|    3 | `packages/contracts/src/remotePairing.ts`                  | ready   |   158 |        92 | wasm-bindgen | -                                                                          |
|    3 | `packages/contracts/src/hostSecrets.ts`                    | ready   |   169 |        91 | wasm-bindgen | -                                                                          |
|    3 | `packages/contracts/src/browserAutomationToolOutputs.ts`   | ready   |   405 |        88 | wasm-bindgen | -                                                                          |
|    3 | `packages/contracts/src/providerDiscovery.ts`              | ready   |   347 |        87 | wasm-bindgen | -                                                                          |
|    3 | `packages/contracts/src/browserAutomationToolInputs.ts`    | ready   |   465 |        81 | wasm-bindgen | Insertion-ordered maps and object keys (2); Callback-shaped APIs (1)       |
|    3 | `packages/contracts/src/model.ts`                          | caution |  1336 |        47 | wasm-bindgen | String offsets and slicing (2); Insertion-ordered maps and object keys (3) |
|    3 | `packages/contracts/src/orchestration.ts`                  | caution |  3284 |         9 | wasm-bindgen | -                                                                          |
|    4 | `packages/contracts/src/remoteAgentGateway.ts`             | ready   |    35 |        99 | wasm-bindgen | -                                                                          |
|    4 | `packages/contracts/src/provider.ts`                       | ready   |   241 |        94 | wasm-bindgen | -                                                                          |
|    4 | `packages/contracts/src/settings.ts`                       | caution |   303 |        93 | wasm-bindgen | Class inheritance (1)                                                      |
|    4 | `packages/contracts/src/githubProjectProvisioning.ts`      | ready   |    65 |        92 | wasm-bindgen | -                                                                          |
|    4 | `packages/contracts/src/externalMcp.ts`                    | ready   |   153 |        90 | wasm-bindgen | -                                                                          |
|    4 | `packages/contracts/src/automation.ts`                     | ready   |   490 |        86 | wasm-bindgen | -                                                                          |
|    4 | `packages/contracts/src/stats.ts`                          | ready   |   323 |        84 | wasm-bindgen | -                                                                          |
|    4 | `packages/contracts/src/git.ts`                            | ready   |   646 |        82 | wasm-bindgen | -                                                                          |
|    4 | `packages/contracts/src/providerRuntime.ts`                | ready   |  1129 |        76 | wasm-bindgen | -                                                                          |
|    5 | `packages/contracts/src/wsCompatibility.ts`                | caution |   120 |        92 | wasm-bindgen | Class inheritance (1)                                                      |
|    5 | `packages/contracts/src/pullRequests.ts`                   | caution |   299 |        89 | wasm-bindgen | Class inheritance (1)                                                      |
|    5 | `packages/contracts/src/inboxRecaps.ts`                    | ready   |   152 |        81 | wasm-bindgen | String offsets and slicing (2); Insertion-ordered maps and object keys (1) |
|    5 | `packages/contracts/src/server.ts`                         | caution |   706 |        79 | wasm-bindgen | Class inheritance (1)                                                      |
|    6 | `packages/contracts/src/agentGateway.ts`                   | ready   |   228 |        97 | wasm-bindgen | -                                                                          |
|    6 | `packages/contracts/src/githubInbox.ts`                    | ready   |   166 |        96 | wasm-bindgen | -                                                                          |
|    7 | `packages/contracts/src/hubWork.ts`                        | ready   |   135 |        94 | wasm-bindgen | -                                                                          |
|    8 | `packages/contracts/src/projectAgent.ts`                   | ready   |   926 |        77 | wasm-bindgen | -                                                                          |
|    9 | `packages/contracts/src/ws.ts`                             | ready   |  1009 |        76 | wasm-bindgen | Dynamic typing (1)                                                         |
|    9 | `packages/contracts/src/ipc.ts`                            | caution |  1686 |        38 | wasm-bindgen | Callback-shaped APIs (426)                                                 |
|   10 | `packages/contracts/src/rpc.ts`                            | caution |  2545 |        39 | wasm-bindgen | Class inheritance (1)                                                      |
|   11 | `packages/contracts/src/index.ts`                          | ready   |    76 |       100 | wasm-bindgen | -                                                                          |

### 3. @synara/profile-ui (`packages/profile-ui`, portable, 1 test files)

| Wave | Unit                                       | Status | Lines | Readiness | Bridge       | Main risks                                               |
| ---: | ------------------------------------------ | ------ | ----: | --------: | ------------ | -------------------------------------------------------- |
|    0 | `packages/profile-ui/src/socialIcon.tsx`   | keep   |    32 |       100 | keep         | -                                                        |
|    0 | `packages/profile-ui/src/providerIcon.tsx` | keep   |   290 |        95 | keep         | -                                                        |
|    0 | `packages/profile-ui/src/cn.ts`            | ready  |     9 |        90 | wasm-bindgen | -                                                        |
|    1 | `packages/profile-ui/src/avatar.tsx`       | keep   |    38 |       100 | keep         | -                                                        |
|    1 | `packages/profile-ui/src/logo.tsx`         | keep   |    29 |       100 | keep         | -                                                        |
|    1 | `packages/profile-ui/src/skeleton.tsx`     | keep   |    20 |       100 | keep         | -                                                        |
|    1 | `packages/profile-ui/src/formatting.ts`    | ready  |   129 |        96 | wasm-bindgen | String offsets and slicing (3); Number semantics (2)     |
|    2 | `packages/profile-ui/src/sections.tsx`     | keep   |   122 |       100 | keep         | -                                                        |
|    2 | `packages/profile-ui/src/heatmap.tsx`      | keep   |   254 |        89 | keep         | String offsets and slicing (2); Callback-shaped APIs (1) |

### 4. @synara/shared (`packages/shared`, node, 260 test files, uses @synara/contracts)

| Wave | Unit                                                   | Status  | Lines | Readiness | Bridge  | Main risks                                                                             |
| ---: | ------------------------------------------------------ | ------- | ----: | --------: | ------- | -------------------------------------------------------------------------------------- |
|    0 | `packages/shared/src/Struct.ts`                        | caution |    23 |       100 | napi-rs | Insertion-ordered maps and object keys (2)                                             |
|    0 | `packages/shared/src/approvalSessionGrant.ts`          | ready   |    30 |       100 | napi-rs | -                                                                                      |
|    0 | `packages/shared/src/asyncUserInput.ts`                | ready   |    45 |       100 | napi-rs | -                                                                                      |
|    0 | `packages/shared/src/automationCompletionPolicy.ts`    | ready   |    26 |       100 | napi-rs | -                                                                                      |
|    0 | `packages/shared/src/automationMode.ts`                | ready   |    48 |       100 | napi-rs | -                                                                                      |
|    0 | `packages/shared/src/automationRunLifecycle.ts`        | ready   |    97 |       100 | napi-rs | Insertion-ordered maps and object keys (2)                                             |
|    0 | `packages/shared/src/backgroundTasks.ts`               | ready   |    84 |       100 | napi-rs | Insertion-ordered maps and object keys (1)                                             |
|    0 | `packages/shared/src/betaFeatures.ts`                  | ready   |   109 |       100 | napi-rs | -                                                                                      |
|    0 | `packages/shared/src/binaryTransfer.ts`                | ready   |     5 |       100 | napi-rs | -                                                                                      |
|    0 | `packages/shared/src/browserAutomationErrors.ts`       | ready   |    57 |       100 | napi-rs | Dynamic typing (1)                                                                     |
|    0 | `packages/shared/src/browserAutomationPresentation.ts` | ready   |    16 |       100 | napi-rs | -                                                                                      |
|    0 | `packages/shared/src/browserShortcuts.ts`              | ready   |    57 |       100 | napi-rs | -                                                                                      |
|    0 | `packages/shared/src/byteAccumulator.ts`               | caution |   100 |       100 | napi-rs | Number semantics (7); Exceptions (3)                                                   |
|    0 | `packages/shared/src/claudeCache.ts`                   | ready   |    62 |       100 | napi-rs | -                                                                                      |
|    0 | `packages/shared/src/cli.ts`                           | ready   |    75 |       100 | napi-rs | Reflection and prototypes (1)                                                          |
|    0 | `packages/shared/src/codexConfig.ts`                   | ready   |    79 |       100 | napi-rs | Regexes built at runtime (1)                                                           |
|    0 | `packages/shared/src/composerSlashCommands.ts`         | ready   |    17 |       100 | napi-rs | -                                                                                      |
|    0 | `packages/shared/src/computerGrants.ts`                | ready   |   170 |       100 | napi-rs | String offsets and slicing (1)                                                         |
|    0 | `packages/shared/src/computerInvocation.ts`            | ready   |    22 |       100 | napi-rs | String offsets and slicing (1); Regex features the `regex` crate lacks (1)             |
|    0 | `packages/shared/src/conversationEdit.ts`              | ready   |   136 |       100 | napi-rs | String offsets and slicing (1); Insertion-ordered maps and object keys (1)             |
|    0 | `packages/shared/src/coordinatorCheckin.ts`            | ready   |   106 |       100 | napi-rs | Insertion-ordered maps and object keys (1)                                             |
|    0 | `packages/shared/src/cuaActionDiagnostics.ts`          | ready   |    83 |       100 | napi-rs | Insertion-ordered maps and object keys (3)                                             |
|    0 | `packages/shared/src/desktopChrome.ts`                 | ready   |   105 |       100 | napi-rs | Number semantics (2)                                                                   |
|    0 | `packages/shared/src/desktopIdentity.ts`               | ready   |   228 |       100 | napi-rs | Exceptions (1)                                                                         |
|    0 | `packages/shared/src/desktopTitleBar.ts`               | ready   |    35 |       100 | napi-rs | -                                                                                      |
|    0 | `packages/shared/src/editorIcons.ts`                   | ready   |     5 |       100 | napi-rs | -                                                                                      |
|    0 | `packages/shared/src/errorMessages.ts`                 | ready   |    58 |       100 | napi-rs | Insertion-ordered maps and object keys (1)                                             |
|    0 | `packages/shared/src/githubAvatar.ts`                  | ready   |    13 |       100 | napi-rs | -                                                                                      |
|    0 | `packages/shared/src/groupSlug.ts`                     | ready   |    13 |       100 | napi-rs | String offsets and slicing (1)                                                         |
|    0 | `packages/shared/src/groupThreadState.ts`              | ready   |   267 |       100 | napi-rs | -                                                                                      |
|    0 | `packages/shared/src/installerDownloads.ts`            | ready   |    31 |       100 | napi-rs | Exceptions (3)                                                                         |
|    0 | `packages/shared/src/kanban.ts`                        | ready   |   256 |       100 | napi-rs | Insertion-ordered maps and object keys (1)                                             |
|    0 | `packages/shared/src/modelImageBudget.ts`              | ready   |     2 |       100 | napi-rs | -                                                                                      |
|    0 | `packages/shared/src/nativeLocalizationResources.ts`   | ready   |     4 |       100 | napi-rs | -                                                                                      |
|    0 | `packages/shared/src/path.ts`                          | ready   |   107 |       100 | napi-rs | String offsets and slicing (2)                                                         |
|    0 | `packages/shared/src/pathContainment.ts`               | ready   |    16 |       100 | napi-rs | -                                                                                      |
|    0 | `packages/shared/src/pinnedMessages.ts`                | ready   |    96 |       100 | napi-rs | String offsets and slicing (2)                                                         |
|    0 | `packages/shared/src/platformEnvironment.ts`           | ready   |    33 |       100 | napi-rs | -                                                                                      |
|    0 | `packages/shared/src/processSpawnOutcome.ts`           | ready   |    26 |       100 | napi-rs | -                                                                                      |
|    0 | `packages/shared/src/projectDirectoryName.ts`          | ready   |    19 |       100 | napi-rs | -                                                                                      |
|    0 | `packages/shared/src/providerDeliveryBlock.ts`         | ready   |    27 |       100 | napi-rs | -                                                                                      |
|    0 | `packages/shared/src/pullRequestList.ts`               | ready   |   138 |       100 | napi-rs | Insertion-ordered maps and object keys (2)                                             |
|    0 | `packages/shared/src/releaseDownloads.ts`              | ready   |   106 |       100 | napi-rs | Exceptions (1)                                                                         |
|    0 | `packages/shared/src/remoteResources.ts`               | ready   |    99 |       100 | napi-rs | Exceptions (1); JSON round-trips (1)                                                   |
|    0 | `packages/shared/src/runtimeMode.ts`                   | ready   |    59 |       100 | napi-rs | Insertion-ordered maps and object keys (1)                                             |
|    0 | `packages/shared/src/schemaJson.ts`                    | caution |    13 |       100 | napi-rs | -                                                                                      |
|    0 | `packages/shared/src/searchQuery.ts`                   | ready   |    22 |       100 | napi-rs | -                                                                                      |
|    0 | `packages/shared/src/sidechatThread.ts`                | ready   |    32 |       100 | napi-rs | -                                                                                      |
|    0 | `packages/shared/src/staticSnapshot.ts`                | ready   |   120 |       100 | napi-rs | String offsets and slicing (1); Exceptions (5)                                         |
|    0 | `packages/shared/src/threadDetailEvents.ts`            | ready   |    37 |       100 | napi-rs | Insertion-ordered maps and object keys (1)                                             |
|    0 | `packages/shared/src/threadExport.ts`                  | ready   |    20 |       100 | napi-rs | -                                                                                      |
|    0 | `packages/shared/src/threadHierarchy.ts`               | ready   |    48 |       100 | napi-rs | Insertion-ordered maps and object keys (2)                                             |
|    0 | `packages/shared/src/threadMentions.ts`                | ready   |    16 |       100 | napi-rs | String offsets and slicing (1)                                                         |
|    0 | `packages/shared/src/todo.ts`                          | ready   |    37 |       100 | napi-rs | -                                                                                      |
|    0 | `packages/shared/src/turnResume.ts`                    | ready   |    55 |       100 | napi-rs | -                                                                                      |
|    0 | `packages/shared/src/turnStopActivity.ts`              | ready   |    22 |       100 | napi-rs | -                                                                                      |
|    0 | `packages/shared/src/uiLocale.ts`                      | ready   |    11 |       100 | napi-rs | String offsets and slicing (1)                                                         |
|    0 | `packages/shared/src/uiLocalizationResources.ts`       | ready   |    20 |       100 | napi-rs | -                                                                                      |
|    0 | `packages/shared/src/windowsCertificate.ts`            | ready   |    89 |       100 | napi-rs | String offsets and slicing (1); Insertion-ordered maps and object keys (1)             |
|    0 | `packages/shared/src/workspaceFileWrite.ts`            | ready   |    10 |       100 | napi-rs | Dynamic typing (1)                                                                     |
|    0 | `packages/shared/src/worktreeHandoff.ts`               | ready   |    77 |       100 | napi-rs | -                                                                                      |
|    0 | `packages/shared/src/wsRequestClass.ts`                | ready   |    99 |       100 | napi-rs | Insertion-ordered maps and object keys (2)                                             |
|    0 | `packages/shared/src/betaChannel.ts`                   | ready   |    59 |        99 | napi-rs | -                                                                                      |
|    0 | `packages/shared/src/computerKeyNames.ts`              | ready   |   102 |        99 | napi-rs | Insertion-ordered maps and object keys (1)                                             |
|    0 | `packages/shared/src/providerCliProfiles.ts`           | ready   |   127 |        99 | napi-rs | String offsets and slicing (1); Insertion-ordered maps and object keys (2)             |
|    0 | `packages/shared/src/remoteCapabilities.ts`            | ready   |    20 |        99 | napi-rs | Insertion-ordered maps and object keys (3)                                             |
|    0 | `packages/shared/src/socialLinks.ts`                   | ready   |    88 |        99 | napi-rs | String offsets and slicing (1); Regex features the `regex` crate lacks (1)             |
|    0 | `packages/shared/src/text.ts`                          | ready   |    57 |        99 | napi-rs | String offsets and slicing (2)                                                         |
|    0 | `packages/shared/src/agentMentions.ts`                 | ready   |   134 |        98 | napi-rs | String offsets and slicing (3)                                                         |
|    0 | `packages/shared/src/formatBytes.ts`                   | ready   |    23 |        98 | napi-rs | String offsets and slicing (1); Number semantics (3)                                   |
|    0 | `packages/shared/src/inboxDay.ts`                      | ready   |    83 |        98 | napi-rs | String offsets and slicing (1)                                                         |
|    0 | `packages/shared/src/uiTreeTargeting.ts`               | ready   |   155 |        98 | napi-rs | Callback-shaped APIs (8); Exceptions (2)                                               |
|    0 | `packages/shared/src/filesystemPlatform.ts`            | ready   |    66 |        97 | napi-rs | Async and promises (8); Insertion-ordered maps and object keys (1)                     |
|    0 | `packages/shared/src/synaraHome.ts`                    | ready   |    44 |        97 | napi-rs | String offsets and slicing (1)                                                         |
|    0 | `packages/shared/src/diagnosticIssue.ts`               | caution |    20 |        96 | napi-rs | Regex features the `regex` crate lacks (2); Exceptions (1)                             |
|    0 | `packages/shared/src/keybindingRules.ts`               | ready   |   293 |        96 | napi-rs | String offsets and slicing (2)                                                         |
|    0 | `packages/shared/src/providerMetadata.ts`              | ready   |   147 |        95 | napi-rs | Insertion-ordered maps and object keys (1)                                             |
|    0 | `packages/shared/src/sidechatExpiry.ts`                | ready   |   175 |        95 | napi-rs | Callback-shaped APIs (16); Insertion-ordered maps and object keys (1)                  |
|    0 | `packages/shared/src/appSnapShortcut.ts`               | ready   |   164 |        94 | napi-rs | String offsets and slicing (4); Insertion-ordered maps and object keys (1)             |
|    0 | `packages/shared/src/logging.ts`                       | ready   |   150 |        94 | napi-rs | String offsets and slicing (1); Exceptions (11)                                        |
|    0 | `packages/shared/src/outboundHttpPolicy.ts`            | ready   |   199 |        94 | napi-rs | Insertion-ordered maps and object keys (4); Class inheritance (1)                      |
|    0 | `packages/shared/src/threadSummary.ts`                 | ready   |   383 |        94 | napi-rs | Insertion-ordered maps and object keys (4)                                             |
|    0 | `packages/shared/src/localPreviewFiles.ts`             | ready   |    80 |        92 | napi-rs | String offsets and slicing (3); Regexes built at runtime (1)                           |
|    0 | `packages/shared/src/threadWorkspace.ts`               | ready   |   165 |        92 | napi-rs | String offsets and slicing (3)                                                         |
|    0 | `packages/shared/src/Net.ts`                           | caution |   160 |        91 | napi-rs | Callback-shaped APIs (5); Class inheritance (2)                                        |
|    0 | `packages/shared/src/inlineVisual.ts`                  | ready   |   233 |        90 | napi-rs | String offsets and slicing (1); Regexes built at runtime (1)                           |
|    0 | `packages/shared/src/migrationRecovery.ts`             | ready   |   271 |        90 | napi-rs | String offsets and slicing (3); Callback-shaped APIs (1)                               |
|    0 | `packages/shared/src/browserAnnotations.ts`            | ready   |   348 |        89 | napi-rs | Regex features the `regex` crate lacks (1); Insertion-ordered maps and object keys (9) |
|    0 | `packages/shared/src/git.ts`                           | ready   |   163 |        89 | napi-rs | String offsets and slicing (8); Insertion-ordered maps and object keys (2)             |
|    0 | `packages/shared/src/mcpConnectors.ts`                 | ready   |   317 |        89 | napi-rs | String offsets and slicing (3); Insertion-ordered maps and object keys (1)             |
|    0 | `packages/shared/src/transportRace.ts`                 | ready   |   224 |        89 | napi-rs | String offsets and slicing (1); Callback-shaped APIs (5)                               |
|    0 | `packages/shared/src/deviceHelperCache.ts`             | ready   |   110 |        87 | napi-rs | String offsets and slicing (4); Async and promises (7)                                 |
|    0 | `packages/shared/src/modelName.ts`                     | ready   |   112 |        86 | napi-rs | String offsets and slicing (5); Insertion-ordered maps and object keys (2)             |
|    0 | `packages/shared/src/inlineVisualFormula.ts`           | caution |   192 |        85 | napi-rs | String offsets and slicing (3); Regex features the `regex` crate lacks (1)             |
|    0 | `packages/shared/src/chatThreads.ts`                   | ready   |   161 |        84 | napi-rs | String offsets and slicing (7); Regex features the `regex` crate lacks (2)             |
|    0 | `packages/shared/src/subagents.ts`                     | ready   |   607 |        84 | napi-rs | Regex features the `regex` crate lacks (1); Insertion-ordered maps and object keys (7) |
|    0 | `packages/shared/src/DrainableWorker.ts`               | caution |   268 |        81 | napi-rs | Generators (26); Callback-shaped APIs (3)                                              |
|    0 | `packages/shared/src/browserSession.ts`                | ready   |   284 |        80 | napi-rs | Regex features the `regex` crate lacks (4); String offsets and slicing (1)             |
|    0 | `packages/shared/src/cuaDriverProtocol.ts`             | ready   |   489 |        78 | napi-rs | String offsets and slicing (3); Insertion-ordered maps and object keys (4)             |
|    0 | `packages/shared/src/shell.ts`                         | ready   |   302 |        78 | napi-rs | String offsets and slicing (5); Insertion-ordered maps and object keys (5)             |
|    0 | `packages/shared/src/terminalThreads.ts`               | ready   |   429 |        77 | napi-rs | String offsets and slicing (8); Regex features the `regex` crate lacks (1)             |
|    0 | `packages/shared/src/desktopOperationQueue.ts`         | ready   |   249 |        76 | napi-rs | Async and promises (25); Callback-shaped APIs (6)                                      |
|    0 | `packages/shared/src/deviceKey.ts`                     | ready   |   273 |        74 | napi-rs | String offsets and slicing (6); Async and promises (11)                                |
|    0 | `packages/shared/src/frameTransport.ts`                | ready   |   458 |        74 | napi-rs | Callback-shaped APIs (20); Insertion-ordered maps and object keys (6)                  |
|    0 | `packages/shared/src/projectAgent.ts`                  | ready   |   209 |        74 | napi-rs | String offsets and slicing (11); Regex features the `regex` crate lacks (3)            |
|    0 | `packages/shared/src/KeyedSingleFlightCache.ts`        | caution |   235 |        73 | napi-rs | String offsets and slicing (2); Generators (21)                                        |
|    0 | `packages/shared/src/providerInstances.ts`             | ready   |   637 |        72 | napi-rs | String offsets and slicing (5); Insertion-ordered maps and object keys (4)             |
|    0 | `packages/shared/src/executable.ts`                    | ready   |   299 |        71 | napi-rs | String offsets and slicing (5); Insertion-ordered maps and object keys (7)             |
|    0 | `packages/shared/src/hostSecrets.ts`                   | ready   |   618 |        67 | napi-rs | String offsets and slicing (2); Async and promises (33)                                |
|    0 | `packages/shared/src/diagnosticsRedaction.ts`          | ready   |   416 |        60 | napi-rs | String offsets and slicing (16); Regex features the `regex` crate lacks (6)            |
|    0 | `packages/shared/src/markdownPlainText.ts`             | ready   |   357 |        55 | napi-rs | String offsets and slicing (25); Regexes built at runtime (4)                          |
|    0 | `packages/shared/src/account.ts`                       | caution |   961 |        48 | napi-rs | Async and promises (72); Callback-shaped APIs (50)                                     |
|    1 | `packages/shared/src/computerFrame.ts`                 | ready   |    78 |       100 | napi-rs | Class inheritance (1); Exceptions (3)                                                  |
|    1 | `packages/shared/src/deviceFrame.ts`                   | ready   |    84 |       100 | napi-rs | Class inheritance (1); Exceptions (3)                                                  |
|    1 | `packages/shared/src/lengthPrefixedRecords.ts`         | ready   |    58 |       100 | napi-rs | Class inheritance (1); Exceptions (1)                                                  |
|    1 | `packages/shared/src/localServers.ts`                  | ready   |    66 |       100 | napi-rs | -                                                                                      |
|    1 | `packages/shared/src/pendingInteractions.ts`           | ready   |   123 |       100 | napi-rs | Insertion-ordered maps and object keys (1); Callback-shaped APIs (1)                   |
|    1 | `packages/shared/src/projectContainers.ts`             | ready   |    72 |       100 | napi-rs | -                                                                                      |
|    1 | `packages/shared/src/projectFolders.ts`                | ready   |   114 |       100 | napi-rs | String offsets and slicing (1); Insertion-ordered maps and object keys (2)             |
|    1 | `packages/shared/src/providerUsage.ts`                 | ready   |    54 |       100 | napi-rs | -                                                                                      |
|    1 | `packages/shared/src/serverSettings.ts`                | ready   |   160 |       100 | napi-rs | -                                                                                      |
|    1 | `packages/shared/src/threadEnvironment.ts`             | ready   |    58 |       100 | napi-rs | -                                                                                      |
|    1 | `packages/shared/src/toolOutputSummary.ts`             | ready   |    69 |        97 | napi-rs | String offsets and slicing (6)                                                         |
|    1 | `packages/shared/src/windowsProcess.ts`                | ready   |   144 |        94 | napi-rs | String offsets and slicing (1); Regex features the `regex` crate lacks (1)             |
|    1 | `packages/shared/src/loginShellEnvironment.ts`         | ready   |   435 |        89 | napi-rs | Insertion-ordered maps and object keys (2); Callback-shaped APIs (2)                   |
|    1 | `packages/shared/src/githubRepository.ts`              | ready   |    72 |        85 | napi-rs | String offsets and slicing (8); Regex features the `regex` crate lacks (3)             |
|    1 | `packages/shared/src/KeyedDrainableWorker.ts`          | caution |   313 |        80 | napi-rs | Generators (26); Callback-shaped APIs (4)                                              |
|    1 | `packages/shared/src/browserAutomationCatalogue.ts`    | ready   |   439 |        79 | napi-rs | Insertion-ordered maps and object keys (19); Reflection and prototypes (2)             |
|    1 | `packages/shared/src/codexStderr.ts`                   | ready   |   151 |        78 | napi-rs | String offsets and slicing (5); Regex features the `regex` crate lacks (3)             |
|    1 | `packages/shared/src/jsonrpc-stdio.ts`                 | caution |   560 |        67 | napi-rs | String offsets and slicing (1); Async and promises (13)                                |
|    1 | `packages/shared/src/model.ts`                         | ready   |   894 |        63 | napi-rs | String offsets and slicing (4); Regex features the `regex` crate lacks (1)             |
|    1 | `packages/shared/src/outboundHttp.ts`                  | ready   |   605 |        60 | napi-rs | String offsets and slicing (2); Async and promises (15)                                |
|    2 | `packages/shared/src/browserAutomationMcpError.ts`     | ready   |    16 |       100 | napi-rs | Exceptions (1)                                                                         |
|    2 | `packages/shared/src/chatGptVoiceTranscription.ts`     | ready   |    75 |       100 | napi-rs | Async and promises (2)                                                                 |
|    2 | `packages/shared/src/wslBridge.ts`                     | ready   |    22 |       100 | napi-rs | -                                                                                      |
|    2 | `packages/shared/src/platformProcess.ts`               | ready   |   237 |        96 | napi-rs | Class inheritance (2); Exceptions (6)                                                  |
|    2 | `packages/shared/src/threadPullRequestLinks.ts`        | ready   |   281 |        95 | napi-rs | String offsets and slicing (1); Insertion-ordered maps and object keys (3)             |
|    2 | `packages/shared/src/usageAnalytics.ts`                | ready   |    71 |        93 | napi-rs | Insertion-ordered maps and object keys (11)                                            |
|    3 | `packages/shared/src/processRuntime.ts`                | ready   |   208 |        98 | napi-rs | Dynamic typing (1); Callback-shaped APIs (1)                                           |
|    3 | `packages/shared/src/hostUsage.ts`                     | ready   |   168 |        96 | napi-rs | Insertion-ordered maps and object keys (3)                                             |
|    4 | `packages/shared/src/cloudflared.ts`                   | ready   |    98 |        91 | napi-rs | Async and promises (14); Exceptions (9)                                                |
|    4 | `packages/shared/src/parentBoundProcess.ts`            | caution |   198 |        90 | napi-rs | Async and promises (5); Callback-shaped APIs (1)                                       |

### 5. @synara/analytics (`apps/analytics`, portable, 5 test files, uses @synara/contracts, @synara/shared)

| Wave | Unit                                        | Status  | Lines | Readiness | Bridge       | Main risks                                                                  |
| ---: | ------------------------------------------- | ------- | ----: | --------: | ------------ | --------------------------------------------------------------------------- |
|    0 | `apps/analytics/forwarder/`                 | ready   |    28 |       100 | wasm-bindgen | Async and promises (1)                                                      |
|    0 | `apps/analytics/src/debounce.ts`            | caution |    16 |       100 | wasm-bindgen | Callback-shaped APIs (2)                                                    |
|    0 | `apps/analytics/diagnosticsPrivacySql.ts`   | ready   |    96 |        98 | napi-rs      | Insertion-ordered maps and object keys (1)                                  |
|    0 | `apps/analytics/usagePrivacySql.ts`         | ready   |    28 |        98 | wasm-bindgen | Insertion-ordered maps and object keys (1)                                  |
|    0 | `apps/analytics/src/theme.ts`               | keep    |    33 |        94 | keep         | Callback-shaped APIs (3); Insertion-ordered maps and object keys (1)        |
|    0 | `apps/analytics/testing/`                   | caution |    47 |        94 | napi-rs      | String offsets and slicing (3); Async and promises (5)                      |
|    0 | `apps/analytics/reliability.ts`             | ready   |    90 |        85 | wasm-bindgen | String offsets and slicing (3); Async and promises (4)                      |
|    0 | `apps/analytics/src/api.ts`                 | ready   |   329 |        85 | wasm-bindgen | Async and promises (4); Class inheritance (1)                               |
|    0 | `apps/analytics/installerDownloads.ts`      | ready   |   277 |        79 | wasm-bindgen | String offsets and slicing (2); Async and promises (16)                     |
|    1 | `apps/analytics/src/filters.ts`             | keep    |   113 |        93 | keep         | String offsets and slicing (2); Callback-shaped APIs (1)                    |
|    1 | `apps/analytics/hostUsage.ts`               | ready   |   142 |        86 | wasm-bindgen | String offsets and slicing (2); Async and promises (5)                      |
|    2 | `apps/analytics/src/components.tsx`         | keep    |   749 |        58 | keep         | String offsets and slicing (2); Callback-shaped APIs (7)                    |
|    2 | `apps/analytics/worker.ts`                  | caution |  2766 |         0 | napi-rs      | String offsets and slicing (51); Regex features the `regex` crate lacks (1) |
|    3 | `apps/analytics/src/TurnBreakdownTable.tsx` | keep    |    98 |       100 | keep         | Number semantics (1)                                                        |
|    4 | `apps/analytics/src/views/`                 | keep    |  2534 |         2 | keep         | String offsets and slicing (10); Callback-shaped APIs (13)                  |
|    5 | `apps/analytics/src/App.tsx`                | keep    |   161 |        96 | keep         | Callback-shaped APIs (1)                                                    |
|    6 | `apps/analytics/src/main.tsx`               | keep    |    15 |       100 | keep         | -                                                                           |

### 6. @synara/api (`apps/api`, node, 31 test files, uses @synara/contracts, @synara/shared)

| Wave | Unit                              | Status  | Lines | Readiness | Bridge  | Main risks                                                                 |
| ---: | --------------------------------- | ------- | ----: | --------: | ------- | -------------------------------------------------------------------------- |
|    0 | `apps/api/src/rateLimit.ts`       | caution |    85 |       100 | napi-rs | Callback-shaped APIs (2); Insertion-ordered maps and object keys (1)       |
|    0 | `apps/api/src/clientIp.ts`        | ready   |   103 |        93 | napi-rs | String offsets and slicing (1); Callback-shaped APIs (1)                   |
|    0 | `apps/api/src/perf.ts`            | ready   |   335 |        75 | napi-rs | Callback-shaped APIs (21); Async and promises (7)                          |
|    0 | `apps/api/cloudflare/`            | caution |   344 |        72 | napi-rs | String offsets and slicing (2); Async and promises (20)                    |
|    0 | `apps/api/src/db/`                | ready   |   640 |        70 | napi-rs | Async and promises (4); Exceptions (1)                                     |
|    1 | `apps/api/src/config.ts`          | ready   |   616 |        75 | napi-rs | Insertion-ordered maps and object keys (1); Class inheritance (1)          |
|    1 | `apps/api/src/testing/`           | caution |   717 |        42 | napi-rs | String offsets and slicing (2); Async and promises (36)                    |
|    1 | `apps/api/src/identity/`          | caution |  3314 |         0 | napi-rs | Regex features the `regex` crate lacks (2); String offsets and slicing (1) |
|    2 | `apps/api/src/avatarStorage.ts`   | caution |   154 |        84 | napi-rs | String offsets and slicing (1); Async and promises (6)                     |
|    2 | `apps/api/scripts/`               | ready   |   367 |        63 | napi-rs | String offsets and slicing (12); Async and promises (13)                   |
|    2 | `apps/api/src/remote/`            | ready   |   594 |        51 | napi-rs | String offsets and slicing (2); Async and promises (86)                    |
|    2 | `apps/api/src/billing/`           | caution |  1051 |        37 | napi-rs | Async and promises (115); Callback-shaped APIs (13)                        |
|    2 | `apps/api/src/liveActivities/`    | caution |   892 |        37 | napi-rs | String offsets and slicing (3); Async and promises (74)                    |
|    3 | `apps/api/src/accountDeletion.ts` | caution |   258 |        83 | napi-rs | Async and promises (50); Exceptions (3)                                    |
|    4 | `apps/api/src/routes/`            | caution |  3369 |         0 | napi-rs | String offsets and slicing (4); Async and promises (328)                   |
|    5 | `apps/api/src/app.ts`             | caution |   216 |        89 | napi-rs | Async and promises (16); Callback-shaped APIs (3)                          |
|    6 | `apps/api/src/index.ts`           | ready   |    83 |        88 | napi-rs | Async and promises (11); Callback-shaped APIs (1)                          |

### 7. @synara/desktop (`apps/desktop`, node, 131 test files, uses @synara/contracts, @synara/shared)

| Wave | Unit                                                | Status  | Lines | Readiness | Bridge  | Main risks                                                                             |
| ---: | --------------------------------------------------- | ------- | ----: | --------: | ------- | -------------------------------------------------------------------------------------- |
|    0 | `apps/desktop/src/agentCursorPreference.ts`         | ready   |    77 |       100 | napi-rs | Dynamic typing (1); JSON round-trips (2)                                               |
|    0 | `apps/desktop/src/backendCompileCache.ts`           | ready   |    54 |       100 | napi-rs | Async and promises (4); Exceptions (1)                                                 |
|    0 | `apps/desktop/src/backendNodeOptions.ts`            | ready   |    59 |       100 | napi-rs | -                                                                                      |
|    0 | `apps/desktop/src/backendReliability.ts`            | caution |    24 |       100 | napi-rs | Callback-shaped APIs (1); Exceptions (1)                                               |
|    0 | `apps/desktop/src/bundleSwapDetection.ts`           | ready   |    62 |       100 | napi-rs | -                                                                                      |
|    0 | `apps/desktop/src/clipboardPermissions.ts`          | ready   |    54 |       100 | napi-rs | Exceptions (1)                                                                         |
|    0 | `apps/desktop/src/computerDesktopLifecycle.ts`      | caution |    37 |       100 | napi-rs | Callback-shaped APIs (11)                                                              |
|    0 | `apps/desktop/src/contextMenuTemplate.ts`           | caution |    63 |       100 | napi-rs | Callback-shaped APIs (3)                                                               |
|    0 | `apps/desktop/src/desktopCustomTitleBar.ts`         | ready   |    72 |       100 | napi-rs | JSON round-trips (2); Exceptions (1)                                                   |
|    0 | `apps/desktop/src/desktopProcessErrors.ts`          | ready   |    13 |       100 | napi-rs | Exceptions (1)                                                                         |
|    0 | `apps/desktop/src/desktopQuitIntent.ts`             | caution |    70 |       100 | napi-rs | Callback-shaped APIs (5)                                                               |
|    0 | `apps/desktop/src/desktopStaticProtocol.ts`         | caution |    53 |       100 | napi-rs | Callback-shaped APIs (2); Exceptions (1)                                               |
|    0 | `apps/desktop/src/exclusiveApplyQueue.ts`           | caution |    22 |       100 | napi-rs | Callback-shaped APIs (2); Async and promises (1)                                       |
|    0 | `apps/desktop/src/githubUpdateFeed.ts`              | ready   |    39 |       100 | napi-rs | -                                                                                      |
|    0 | `apps/desktop/src/httpRuntime.ts`                   | ready   |     7 |       100 | napi-rs | -                                                                                      |
|    0 | `apps/desktop/src/i18n.ts`                          | ready   |    21 |       100 | napi-rs | -                                                                                      |
|    0 | `apps/desktop/src/initialBackendWindowOpen.ts`      | caution |    52 |       100 | napi-rs | Callback-shaped APIs (9)                                                               |
|    0 | `apps/desktop/src/linuxCuaAdmission.ts`             | ready   |   150 |       100 | napi-rs | Insertion-ordered maps and object keys (2)                                             |
|    0 | `apps/desktop/src/macIconCacheRefresh.ts`           | ready   |    53 |       100 | napi-rs | Dynamic typing (1); JSON round-trips (2)                                               |
|    0 | `apps/desktop/src/mediaPermissions.ts`              | ready   |    67 |       100 | napi-rs | Callback-shaped APIs (2); Exceptions (1)                                               |
|    0 | `apps/desktop/src/migrationBundleIdentity.ts`       | ready   |    35 |       100 | napi-rs | Exceptions (3)                                                                         |
|    0 | `apps/desktop/src/migrationConsentHandoff.ts`       | ready   |    11 |       100 | napi-rs | -                                                                                      |
|    0 | `apps/desktop/src/rendererCrashRecovery.ts`         | caution |    93 |       100 | napi-rs | Insertion-ordered maps and object keys (1)                                             |
|    0 | `apps/desktop/src/runtimeArch.ts`                   | ready   |    32 |       100 | napi-rs | -                                                                                      |
|    0 | `apps/desktop/src/showInFileManager.ts`             | ready   |    26 |       100 | napi-rs | Async and promises (2); Exceptions (1)                                                 |
|    0 | `apps/desktop/src/shutdownSignals.ts`               | caution |    24 |       100 | napi-rs | Callback-shaped APIs (1)                                                               |
|    0 | `apps/desktop/src/subframeNavigationGuard.ts`       | ready   |    41 |       100 | napi-rs | -                                                                                      |
|    0 | `apps/desktop/src/updateInstallPreparation.ts`      | ready   |    41 |       100 | napi-rs | Class inheritance (1); Reflection and prototypes (1)                                   |
|    0 | `apps/desktop/src/updatePendingCache.ts`            | ready   |    74 |       100 | napi-rs | Callback-shaped APIs (1)                                                               |
|    0 | `apps/desktop/src/windowMaterial.ts`                | ready   |    93 |       100 | napi-rs | Callback-shaped APIs (3); Exceptions (1)                                               |
|    0 | `apps/desktop/src/windowState.ts`                   | ready   |   116 |       100 | napi-rs | Number semantics (2); JSON round-trips (2)                                             |
|    0 | `apps/desktop/src/windowsShellIco.ts`               | ready   |   132 |       100 | napi-rs | Insertion-ordered maps and object keys (1); Callback-shaped APIs (1)                   |
|    0 | `apps/desktop/src/cuaHostSocket.ts`                 | caution |    47 |        99 | napi-rs | Async and promises (8); Exceptions (4)                                                 |
|    0 | `apps/desktop/src/desktopAppIcon.ts`                | ready   |   106 |        99 | napi-rs | Callback-shaped APIs (1); Exceptions (2)                                               |
|    0 | `apps/desktop/src/notificationRetention.ts`         | ready   |    46 |        99 | napi-rs | Callback-shaped APIs (6); Insertion-ordered maps and object keys (1)                   |
|    0 | `apps/desktop/src/syncShellEnvironment.ts`          | caution |   123 |        99 | napi-rs | Callback-shaped APIs (2); Insertion-ordered maps and object keys (1)                   |
|    0 | `apps/desktop/src/linuxAppIcon.ts`                  | ready   |   102 |        98 | napi-rs | String offsets and slicing (1); Callback-shaped APIs (1)                               |
|    0 | `apps/desktop/src/desktopStorageMigration.ts`       | ready   |    70 |        97 | napi-rs | Async and promises (2); Insertion-ordered maps and object keys (1)                     |
|    0 | `apps/desktop/src/desktopUserDataProfile.ts`        | ready   |   233 |        96 | napi-rs | String offsets and slicing (1); Exceptions (8)                                         |
|    0 | `apps/desktop/src/serverListeningDetector.ts`       | caution |    52 |        96 | napi-rs | String offsets and slicing (1); Callback-shaped APIs (4)                               |
|    0 | `apps/desktop/src/backendProcessOutput.ts`          | caution |    64 |        95 | napi-rs | Callback-shaped APIs (6); Async and promises (3)                                       |
|    0 | `apps/desktop/src/keepAwake.ts`                     | ready   |   133 |        95 | napi-rs | Callback-shaped APIs (14); Exceptions (3)                                              |
|    0 | `apps/desktop/src/macUpdateInstallHandoff.ts`       | caution |    67 |        95 | napi-rs | Callback-shaped APIs (7); Async and promises (2)                                       |
|    0 | `apps/desktop/src/resumableUpdateDownloadPolicy.ts` | ready   |   178 |        95 | napi-rs | Insertion-ordered maps and object keys (1); Number semantics (1)                       |
|    0 | `apps/desktop/src/updateArtifactIdentity.ts`        | ready   |    91 |        95 | napi-rs | Async and promises (11); Exceptions (6)                                                |
|    0 | `apps/desktop/src/backendSupervisionPolicy.ts`      | ready   |   169 |        94 | napi-rs | String offsets and slicing (5)                                                         |
|    0 | `apps/desktop/src/menuShortcuts.ts`                 | ready   |   189 |        94 | napi-rs | Callback-shaped APIs (2); Insertion-ordered maps and object keys (1)                   |
|    0 | `apps/desktop/src/backendReadiness.ts`              | caution |    98 |        93 | napi-rs | Async and promises (5); Class inheritance (1)                                          |
|    0 | `apps/desktop/src/backendStartupBlock.ts`           | ready   |   134 |        93 | napi-rs | String offsets and slicing (6)                                                         |
|    0 | `apps/desktop/src/cuaRuntimeOwnership.ts`           | ready   |   139 |        93 | napi-rs | Async and promises (2); Insertion-ordered maps and object keys (1)                     |
|    0 | `apps/desktop/src/desktopUpdateControl.ts`          | ready   |   167 |        93 | napi-rs | Callback-shaped APIs (8); Exceptions (1)                                               |
|    0 | `apps/desktop/src/windowsShellAppUserModel.ts`      | ready   |   304 |        93 | napi-rs | String offsets and slicing (1); Callback-shaped APIs (1)                               |
|    0 | `apps/desktop/src/stopNativeHelper.ts`              | caution |    35 |        92 | napi-rs | Async and promises (3); Callback-shaped APIs (1)                                       |
|    0 | `apps/desktop/src/windowsTaskbarIcon.ts`            | ready   |   217 |        92 | napi-rs | String offsets and slicing (1); Callback-shaped APIs (5)                               |
|    0 | `apps/desktop/src/ipcChannels.ts`                   | ready   |   161 |        91 | napi-rs | -                                                                                      |
|    0 | `apps/desktop/src/updateState.ts`                   | ready   |   278 |        91 | napi-rs | String offsets and slicing (2); Exceptions (1)                                         |
|    0 | `apps/desktop/src/computerEmergencyStopNotice.ts`   | caution |    88 |        90 | napi-rs | Async and promises (5); Dynamic typing (1)                                             |
|    0 | `apps/desktop/src/localHtmlPreviewProtocol.ts`      | ready   |   167 |        89 | napi-rs | Async and promises (4); Insertion-ordered maps and object keys (2)                     |
|    0 | `apps/desktop/src/audioLevelMonitor.ts`             | ready   |   259 |        88 | napi-rs | String offsets and slicing (1); Callback-shaped APIs (2)                               |
|    0 | `apps/desktop/src/runningChatsQuitGuard.ts`         | ready   |   237 |        88 | napi-rs | Callback-shaped APIs (12); Async and promises (4)                                      |
|    0 | `apps/desktop/src/desktopMigrationRecovery.ts`      | ready   |   275 |        85 | napi-rs | Callback-shaped APIs (12); Async and promises (7)                                      |
|    0 | `apps/desktop/src/macUpdateDiagnostics.ts`          | caution |   130 |        85 | napi-rs | Regex features the `regex` crate lacks (1); Async and promises (15)                    |
|    0 | `apps/desktop/src/hostUsageRelay.ts`                | ready   |   139 |        84 | napi-rs | String offsets and slicing (3); Insertion-ordered maps and object keys (3)             |
|    0 | `apps/desktop/src/electronUpdaterSecurity.ts`       | ready   |   287 |        83 | napi-rs | Callback-shaped APIs (7); Async and promises (4)                                       |
|    0 | `apps/desktop/src/backendShutdown.ts`               | ready   |   473 |        75 | napi-rs | Async and promises (12); Callback-shaped APIs (6)                                      |
|    0 | `apps/desktop/src/reliabilityReporter.ts`           | caution |   263 |        75 | napi-rs | String offsets and slicing (3); Async and promises (3)                                 |
|    0 | `apps/desktop/src/betaInstaller.ts`                 | ready   |   469 |        66 | napi-rs | String offsets and slicing (6); Async and promises (8)                                 |
|    0 | `apps/desktop/src/betaDiagnostics.ts`               | caution |  1027 |        35 | napi-rs | String offsets and slicing (5); Async and promises (11)                                |
|    1 | `apps/desktop/src/betaRendererDiagnostics.ts`       | ready   |    70 |       100 | napi-rs | -                                                                                      |
|    1 | `apps/desktop/src/desktopWsBridge.ts`               | ready   |    14 |       100 | napi-rs | -                                                                                      |
|    1 | `apps/desktop/src/startupDiagnosticDialog.ts`       | ready   |    26 |       100 | napi-rs | Async and promises (2); Exceptions (1)                                                 |
|    1 | `apps/desktop/src/updateInstallMarker.ts`           | ready   |   233 |       100 | napi-rs | Exceptions (5); JSON round-trips (2)                                                   |
|    1 | `apps/desktop/src/updateMachine.ts`                 | ready   |   216 |       100 | napi-rs | -                                                                                      |
|    1 | `apps/desktop/src/macAppIcon.ts`                    | ready   |    85 |        99 | napi-rs | Async and promises (9); Exceptions (1)                                                 |
|    1 | `apps/desktop/src/backendIssueDetector.ts`          | ready   |    36 |        96 | napi-rs | String offsets and slicing (3); Callback-shaped APIs (1)                               |
|    1 | `apps/desktop/src/confirmDialog.ts`                 | ready   |    56 |        96 | napi-rs | Async and promises (3); Callback-shaped APIs (2)                                       |
|    1 | `apps/desktop/src/backendStartupReadiness.ts`       | ready   |   113 |        92 | napi-rs | Async and promises (11); Callback-shaped APIs (5)                                      |
|    1 | `apps/desktop/src/voiceTranscription.ts`            | ready   |   253 |        88 | napi-rs | Async and promises (7); Exceptions (12)                                                |
|    1 | `apps/desktop/src/safariAccessIpc.ts`               | caution |   147 |        87 | napi-rs | Async and promises (16); Callback-shaped APIs (6)                                      |
|    1 | `apps/desktop/src/computerShield.ts`                | caution |   310 |        74 | napi-rs | Async and promises (23); Callback-shaped APIs (9)                                      |
|    1 | `apps/desktop/src/browserSessionPolicy.ts`          | ready   |   219 |        73 | napi-rs | String offsets and slicing (3); Regex features the `regex` crate lacks (1)             |
|    1 | `apps/desktop/src/computerFrameTap.ts`              | caution |   362 |        72 | napi-rs | String offsets and slicing (1); Async and promises (27)                                |
|    1 | `apps/desktop/src/betaChannel.ts`                   | ready   |   586 |        69 | napi-rs | String offsets and slicing (1); Regexes built at runtime (1)                           |
|    1 | `apps/desktop/src/resumableUpdateDownload.ts`       | caution |   688 |        67 | napi-rs | Callback-shaped APIs (58); Async and promises (20)                                     |
|    1 | `apps/desktop/src/remoteResourceBroker.ts`          | caution |   265 |        62 | napi-rs | String offsets and slicing (1); Async and promises (8)                                 |
|    1 | `apps/desktop/src/browserWebMcp/`                   | caution |   694 |        38 | napi-rs | String offsets and slicing (4); Async and promises (24)                                |
|    1 | `apps/desktop/src/productAnalytics.ts`              | caution |   929 |        37 | napi-rs | String offsets and slicing (15); Dynamic typing (10)                                   |
|    1 | `apps/desktop/src/appSnapManager.ts`                | caution |  2146 |         1 | napi-rs | String offsets and slicing (11); Async and promises (117)                              |
|    2 | `apps/desktop/src/productAnalyticsIpc.ts`           | caution |    36 |       100 | napi-rs | Callback-shaped APIs (2)                                                               |
|    2 | `apps/desktop/src/preload.ts`                       | ready   |   468 |        84 | napi-rs | Async and promises (1); Exceptions (5)                                                 |
|    2 | `apps/desktop/src/appSnapIpc.ts`                    | ready   |   162 |        83 | napi-rs | Async and promises (17); Callback-shaped APIs (2)                                      |
|    2 | `apps/desktop/src/escapeKillSwitchMonitor.ts`       | caution |   275 |        80 | napi-rs | String offsets and slicing (1); Callback-shaped APIs (5)                               |
|    2 | `apps/desktop/src/browserAnnotations/`              | caution |  2472 |         0 | napi-rs | String offsets and slicing (14); Regex features the `regex` crate lacks (2)            |
|    3 | `apps/desktop/src/linuxEscapeKillSwitchMonitor.ts`  | ready   |   159 |        95 | napi-rs | Callback-shaped APIs (3); Async and promises (1)                                       |
|    3 | `apps/desktop/src/browserManager.ts`                | caution |  3576 |         0 | napi-rs | String offsets and slicing (2); Insertion-ordered maps and object keys (37)            |
|    3 | `apps/desktop/src/cuaDriverHost.ts`                 | caution |  3136 |         0 | napi-rs | String offsets and slicing (8); Async and promises (157)                               |
|    4 | `apps/desktop/src/linuxCuaDriverHost.ts`            | caution |    37 |       100 | napi-rs | Async and promises (3); Callback-shaped APIs (1)                                       |
|    4 | `apps/desktop/src/browserIpc.ts`                    | ready   |   174 |        88 | napi-rs | Async and promises (22); Exceptions (1)                                                |
|    4 | `apps/desktop/src/cuaDriverHostStandalone.ts`       | ready   |   138 |        88 | napi-rs | String offsets and slicing (1); Async and promises (10)                                |
|    4 | `apps/desktop/src/browserAutomation/`               | caution |  6821 |         0 | napi-rs | String offsets and slicing (17); Regex features the `regex` crate lacks (3)            |
|    5 | `apps/desktop/src/browserVaultIpc.ts`               | caution |    68 |        92 | napi-rs | Callback-shaped APIs (3); Async and promises (2)                                       |
|    5 | `apps/desktop/src/browserUsePipeServer.ts`          | ready   |   519 |        75 | napi-rs | Async and promises (11); Insertion-ordered maps and object keys (3)                    |
|    6 | `apps/desktop/scripts/`                             | blocked |  3786 |         0 | napi-rs | String offsets and slicing (6); Dynamic code (eval, new Function, computed import) (2) |
|    6 | `apps/desktop/src/cuaFixtures/`                     | caution |  2494 |         0 | napi-rs | String offsets and slicing (8); Async and promises (216)                               |
|    6 | `apps/desktop/src/main.ts`                          | caution |  6409 |         0 | napi-rs | String offsets and slicing (6); Regex features the `regex` crate lacks (1)             |

### 8. @synara/marketing (`apps/marketing`, node, 14 test files, uses @synara/shared)

| Wave | Unit                                                | Status  | Lines | Readiness | Bridge  | Main risks                                                                  |
| ---: | --------------------------------------------------- | ------- | ----: | --------: | ------- | --------------------------------------------------------------------------- |
|    0 | `apps/marketing/src/app/api/inbound-email/`         | ready   |    74 |        93 | napi-rs | Async and promises (3); Exceptions (1)                                      |
|    0 | `apps/marketing/scripts/`                           | caution |  1206 |        18 | napi-rs | String offsets and slicing (10); Regex features the `regex` crate lacks (2) |
|    0 | `apps/marketing/src/data/`                          | caution |  5448 |         0 | napi-rs | -                                                                           |
|    1 | `apps/marketing/src/components/ui/`                 | keep    |   195 |        90 | keep    | -                                                                           |
|    1 | `apps/marketing/src/components/docs/`               | keep    |   499 |        73 | keep    | String offsets and slicing (1); Exceptions (9)                              |
|    1 | `apps/marketing/src/components/`                    | keep    |  2405 |         1 | keep    | String offsets and slicing (2); Async and promises (6)                      |
|    1 | `apps/marketing/src/lib/`                           | keep    |  1454 |         0 | keep    | String offsets and slicing (3); Async and promises (30)                     |
|    2 | `apps/marketing/src/app/ai.txt/`                    | ready   |    13 |       100 | napi-rs | -                                                                           |
|    2 | `apps/marketing/src/app/api/search/`                | ready   |     5 |       100 | napi-rs | -                                                                           |
|    2 | `apps/marketing/src/app/changelog/`                 | keep    |    44 |       100 | keep    | -                                                                           |
|    2 | `apps/marketing/src/app/llms-full.txt/`             | ready   |    13 |       100 | napi-rs | Async and promises (2)                                                      |
|    2 | `apps/marketing/src/app/llms.txt/`                  | ready   |    13 |       100 | napi-rs | -                                                                           |
|    2 | `apps/marketing/src/app/sitemap-index.xml/`         | ready   |    21 |       100 | napi-rs | -                                                                           |
|    2 | `apps/marketing/src/app/docs/`                      | keep    |    24 |        99 | keep    | -                                                                           |
|    2 | `apps/marketing/src/app/install/`                   | keep    |    59 |        99 | keep    | Async and promises (2)                                                      |
|    2 | `apps/marketing/src/app/api/installer-count/`       | ready   |    34 |        96 | napi-rs | Async and promises (2)                                                      |
|    2 | `apps/marketing/src/app/llms.mdx/docs/[[...slug]]/` | ready   |    28 |        95 | napi-rs | Async and promises (3)                                                      |
|    2 | `apps/marketing/src/app/changelog/[version]/`       | keep    |    65 |        94 | keep    | Async and promises (4)                                                      |
|    2 | `apps/marketing/src/app/sponsors/`                  | keep    |   119 |        93 | keep    | -                                                                           |
|    2 | `apps/marketing/src/app/privacy/`                   | keep    |   298 |        89 | keep    | -                                                                           |
|    2 | `apps/marketing/src/app/sponsor/`                   | keep    |   314 |        84 | keep    | String offsets and slicing (1)                                              |
|    2 | `apps/marketing/src/app/docs/[[...slug]]/`          | keep    |   141 |        83 | keep    | String offsets and slicing (1); Async and promises (4)                      |
|    2 | `apps/marketing/src/app/api/feedback/`              | ready   |   266 |        82 | napi-rs | String offsets and slicing (1); Async and promises (7)                      |
|    2 | `apps/marketing/src/app/`                           | keep    |   406 |        79 | keep    | Async and promises (2)                                                      |

### 9. @synara/profiles (`apps/profiles`, node, 3 test files, uses @synara/profile-ui, @synara/shared)

| Wave | Unit                                             | Status | Lines | Readiness | Bridge  | Main risks                                                         |
| ---: | ------------------------------------------------ | ------ | ----: | --------: | ------- | ------------------------------------------------------------------ |
|    0 | `apps/profiles/src/legacy-redirect.ts`           | ready  |    13 |       100 | napi-rs | -                                                                  |
|    0 | `apps/profiles/src/middleware.ts`                | ready  |    31 |        96 | napi-rs | Async and promises (2)                                             |
|    0 | `apps/profiles/src/lib/`                         | keep   |   547 |        55 | keep    | String offsets and slicing (10); Async and promises (19)           |
|    1 | `apps/profiles/src/components/`                  | keep   |   814 |        70 | keep    | Async and promises (4); Insertion-ordered maps and object keys (1) |
|    2 | `apps/profiles/src/app/`                         | keep   |    62 |        98 | keep    | -                                                                  |
|    2 | `apps/profiles/src/app/[handle]/`                | keep   |   523 |        64 | keep    | String offsets and slicing (8); Async and promises (10)            |
|    3 | `apps/profiles/src/app/[handle]/share/[format]/` | keep   |   363 |        79 | keep    | String offsets and slicing (1); Async and promises (6)             |

### 10. @synara/web (`apps/web`, node, 515 test files, uses @synara/contracts, @synara/profile-ui, @synara/shared)

| Wave | Unit                                                | Status  | Lines | Readiness | Bridge  | Main risks                                                                              |
| ---: | --------------------------------------------------- | ------- | ----: | --------: | ------- | --------------------------------------------------------------------------------------- |
|    0 | `apps/web/src/appSettingsStorage.ts`                | ready   |     2 |       100 | napi-rs | -                                                                                       |
|    0 | `apps/web/src/chat-scroll.ts`                       | ready   |    22 |       100 | napi-rs | -                                                                                       |
|    0 | `apps/web/src/chatRouteRestore.ts`                  | ready   |    64 |       100 | napi-rs | -                                                                                       |
|    0 | `apps/web/src/orchestrationEventCoalescing.ts`      | ready   |    70 |       100 | napi-rs | Dynamic typing (1); Insertion-ordered maps and object keys (1)                          |
|    0 | `apps/web/src/pendingTurnDispatch.ts`               | ready   |    43 |       100 | napi-rs | Insertion-ordered maps and object keys (1)                                              |
|    0 | `apps/web/src/sidebarNavOrdering.ts`                | ready   |    36 |       100 | napi-rs | -                                                                                       |
|    0 | `apps/web/src/storageOriginMigration.ts`            | ready   |    62 |       100 | napi-rs | Insertion-ordered maps and object keys (1); Exceptions (3)                              |
|    0 | `apps/web/src/terminalActivity.ts`                  | ready   |    24 |       100 | napi-rs | -                                                                                       |
|    0 | `apps/web/src/terminalVisualIdentity.ts`            | ready   |    98 |       100 | napi-rs | Insertion-ordered maps and object keys (2)                                              |
|    0 | `apps/web/src/threadDetailCatchupPolicy.ts`         | ready   |    51 |       100 | napi-rs | -                                                                                       |
|    0 | `apps/web/src/truncateTitle.ts`                     | ready   |     7 |       100 | napi-rs | String offsets and slicing (1)                                                          |
|    0 | `apps/web/src/composerTriggerInsertion.ts`          | ready   |    34 |        99 | napi-rs | -                                                                                       |
|    0 | `apps/web/src/diffRouteSearch.ts`                   | ready   |    70 |        99 | napi-rs | -                                                                                       |
|    0 | `apps/web/src/diffRenderMode.ts`                    | ready   |     7 |        98 | napi-rs | -                                                                                       |
|    0 | `apps/web/src/pendingUserInput.ts`                  | ready   |   175 |        98 | napi-rs | Insertion-ordered maps and object keys (4)                                              |
|    0 | `apps/web/src/rowHoverReveal.ts`                    | ready   |    48 |        98 | napi-rs | -                                                                                       |
|    0 | `apps/web/src/pullRequestReference.ts`              | ready   |    18 |        97 | napi-rs | String offsets and slicing (2); Regex features the `regex` crate lacks (1)              |
|    0 | `apps/web/src/branding.ts`                          | ready   |    20 |        95 | napi-rs | -                                                                                       |
|    0 | `apps/web/src/composerModelSelectionCommits.ts`     | ready   |    20 |        95 | napi-rs | Callback-shaped APIs (2); Insertion-ordered maps and object keys (1)                    |
|    0 | `apps/web/src/createProjectDialogStore.ts`          | ready   |    13 |        95 | napi-rs | Callback-shaped APIs (1)                                                                |
|    0 | `apps/web/src/env.ts`                               | ready   |     8 |        95 | napi-rs | -                                                                                       |
|    0 | `apps/web/src/featureFlags.ts`                      | keep    |   150 |        95 | keep    | Callback-shaped APIs (3); Insertion-ordered maps and object keys (1)                    |
|    0 | `apps/web/src/proposedPlan.ts`                      | ready   |   100 |        95 | napi-rs | String offsets and slicing (1); Regex features the `regex` crate lacks (1)              |
|    0 | `apps/web/src/threadDetailResumeCursors.ts`         | ready   |    79 |        95 | napi-rs | Insertion-ordered maps and object keys (1)                                              |
|    0 | `apps/web/src/threadDetailSyncRetry.ts`             | ready   |    16 |        95 | napi-rs | Callback-shaped APIs (2); Async and promises (1)                                        |
|    0 | `apps/web/src/touchTargetStyles.ts`                 | ready   |    13 |        95 | napi-rs | -                                                                                       |
|    0 | `apps/web/src/persistedRecord.ts`                   | ready   |    34 |        92 | napi-rs | Insertion-ordered maps and object keys (2); Callback-shaped APIs (1)                    |
|    0 | `apps/web/src/projectRunStore.ts`                   | ready   |    53 |        92 | napi-rs | Callback-shaped APIs (3)                                                                |
|    0 | `apps/web/src/composerFocusRequestStore.ts`         | ready   |    25 |        91 | napi-rs | Callback-shaped APIs (1)                                                                |
|    0 | `apps/web/src/deviceStateStore.ts`                  | caution |    90 |        91 | napi-rs | Callback-shaped APIs (6); Insertion-ordered maps and object keys (1)                    |
|    0 | `apps/web/src/explorerRevealRequestStore.ts`        | ready   |    45 |        91 | napi-rs | String offsets and slicing (1); Callback-shaped APIs (1)                                |
|    0 | `apps/web/src/pinning.logic.ts`                     | ready   |   158 |        91 | napi-rs | Insertion-ordered maps and object keys (7)                                              |
|    0 | `apps/web/src/temporaryThreadStore.ts`              | ready   |    35 |        91 | napi-rs | Callback-shaped APIs (2)                                                                |
|    0 | `apps/web/src/betaFeatures.ts`                      | ready   |    77 |        90 | napi-rs | Callback-shaped APIs (2)                                                                |
|    0 | `apps/web/src/dockExplorerBrowseStore.ts`           | caution |    84 |        88 | napi-rs | Callback-shaped APIs (6); Insertion-ordered maps and object keys (4)                    |
|    0 | `apps/web/src/cursorModelVariants.ts`               | ready   |   210 |        87 | napi-rs | String offsets and slicing (3); Insertion-ordered maps and object keys (2)              |
|    0 | `apps/web/src/surfaceStyles.ts`                     | ready   |    64 |        87 | napi-rs | -                                                                                       |
|    0 | `apps/web/src/routeRestoreRefreshCoordinator.ts`    | ready   |    96 |        86 | napi-rs | Async and promises (13); Callback-shaped APIs (10)                                      |
|    0 | `apps/web/src/computerStateStore.ts`                | ready   |   136 |        85 | napi-rs | Callback-shaped APIs (8); Insertion-ordered maps and object keys (2)                    |
|    0 | `apps/web/src/projectScripts.ts`                    | ready   |    91 |        83 | napi-rs | String offsets and slicing (4); Insertion-ordered maps and object keys (1)              |
|    0 | `apps/web/src/i18n/`                                | keep    |   114 |        82 | keep    | Insertion-ordered maps and object keys (1)                                              |
|    0 | `apps/web/src/providerOrdering.ts`                  | ready   |    59 |        80 | napi-rs | String offsets and slicing (4); Insertion-ordered maps and object keys (3)              |
|    0 | `apps/web/src/threadSelectionStore.ts`              | ready   |   111 |        80 | napi-rs | String offsets and slicing (2); Insertion-ordered maps and object keys (7)              |
|    0 | `apps/web/src/wsTransportEvents.ts`                 | ready   |   123 |        78 | napi-rs | Callback-shaped APIs (8); Insertion-ordered maps and object keys (4)                    |
|    0 | `apps/web/src/file-icons.ts`                        | ready   |   265 |        77 | napi-rs | String offsets and slicing (7); Insertion-ordered maps and object keys (2)              |
|    0 | `apps/web/public/`                                  | ready   |   297 |        71 | napi-rs | String offsets and slicing (2); Async and promises (16)                                 |
|    0 | `apps/web/src/lib/pdf/`                             | keep    |   837 |        56 | keep    | Async and promises (25); Callback-shaped APIs (11)                                      |
|    1 | `apps/web/src/authSignedOut.ts`                     | caution |    39 |       100 | napi-rs | Callback-shaped APIs (1)                                                                |
|    1 | `apps/web/src/chatRouteRecovery.ts`                 | ready   |    25 |       100 | napi-rs | Async and promises (3)                                                                  |
|    1 | `apps/web/src/projectRunTargets.ts`                 | ready   |    98 |       100 | napi-rs | -                                                                                       |
|    1 | `apps/web/src/projectTerminalRunner.ts`             | caution |    62 |       100 | napi-rs | Async and promises (3); Callback-shaped APIs (1)                                        |
|    1 | `apps/web/src/tasksSurface.ts`                      | ready   |    33 |        96 | napi-rs | Dynamic typing (1)                                                                      |
|    1 | `apps/web/src/feedback.ts`                          | ready   |   164 |        95 | napi-rs | Async and promises (3); Exceptions (4)                                                  |
|    1 | `apps/web/src/providerUpdates.ts`                   | ready   |   167 |        94 | napi-rs | Async and promises (4); Insertion-ordered maps and object keys (2)                      |
|    1 | `apps/web/src/pairingBootstrap.ts`                  | caution |    82 |        92 | napi-rs | String offsets and slicing (1); Callback-shaped APIs (3)                                |
|    1 | `apps/web/src/sidebarRowStyles.ts`                  | ready   |    98 |        90 | napi-rs | -                                                                                       |
|    1 | `apps/web/src/diffRenderModeStore.ts`               | ready   |    93 |        88 | napi-rs | String offsets and slicing (1); Callback-shaped APIs (3)                                |
|    1 | `apps/web/src/providerModelOptions.ts`              | ready   |   594 |        59 | napi-rs | String offsets and slicing (6); Insertion-ordered maps and object keys (5)              |
|    2 | `apps/web/src/authLogout.ts`                        | caution |    18 |        98 | napi-rs | Callback-shaped APIs (4); Async and promises (3)                                        |
|    2 | `apps/web/src/feedbackDialogStore.ts`               | ready   |    34 |        97 | napi-rs | Callback-shaped APIs (2)                                                                |
|    2 | `apps/web/src/settingsPanelStyles.ts`               | ready   |    65 |        90 | napi-rs | -                                                                                       |
|    3 | `apps/web/src/settingsSidebarNavStyles.ts`          | ready   |    34 |       100 | napi-rs | -                                                                                       |
|    4 | `apps/web/src/appSnapShortcut.ts`                   | ready   |    51 |       100 | napi-rs | Insertion-ordered maps and object keys (2)                                              |
|    4 | `apps/web/src/computerControlMode.ts`               | ready   |    16 |       100 | napi-rs | -                                                                                       |
|    4 | `apps/web/src/fixedShortcuts.ts`                    | ready   |   121 |       100 | napi-rs | -                                                                                       |
|    4 | `apps/web/src/focusedChatContext.ts`                | keep    |    69 |       100 | keep    | -                                                                                       |
|    4 | `apps/web/src/groupPanelClosedStore.ts`             | ready   |    53 |       100 | napi-rs | Callback-shaped APIs (1)                                                                |
|    4 | `apps/web/src/recentViewActivation.logic.ts`        | ready   |    29 |       100 | napi-rs | -                                                                                       |
|    4 | `apps/web/src/splitViewRoute.ts`                    | ready   |    37 |       100 | napi-rs | -                                                                                       |
|    4 | `apps/web/src/threadActivation.logic.ts`            | ready   |    74 |       100 | napi-rs | Insertion-ordered maps and object keys (1)                                              |
|    4 | `apps/web/src/worktreeCleanup.ts`                   | ready   |    52 |       100 | napi-rs | -                                                                                       |
|    4 | `apps/web/src/threadGoal.ts`                        | ready   |    32 |        99 | napi-rs | Async and promises (4); Exceptions (2)                                                  |
|    4 | `apps/web/src/pinnedProjectAgentsStore.ts`          | ready   |    74 |        98 | napi-rs | Callback-shaped APIs (4)                                                                |
|    4 | `apps/web/src/components/chat-drop-overlay/`        | keep    |   252 |        97 | keep    | Callback-shaped APIs (4)                                                                |
|    4 | `apps/web/src/projectEnvironmentStore.ts`           | ready   |    36 |        96 | napi-rs | Callback-shaped APIs (1)                                                                |
|    4 | `apps/web/src/remoteProjectSpacesStore.ts`          | ready   |    31 |        96 | napi-rs | Callback-shaped APIs (1)                                                                |
|    4 | `apps/web/src/timestampFormat.ts`                   | ready   |   122 |        96 | napi-rs | Insertion-ordered maps and object keys (2); Number semantics (1)                        |
|    4 | `apps/web/src/appSnapIntake.ts`                     | ready   |   116 |        95 | napi-rs | Async and promises (10); Exceptions (6)                                                 |
|    4 | `apps/web/src/pinnedMessages.ts`                    | ready   |   184 |        95 | napi-rs | String offsets and slicing (1); Async and promises (2)                                  |
|    4 | `apps/web/src/pinnedThreadsStore.ts`                | ready   |    81 |        95 | napi-rs | Callback-shaped APIs (4)                                                                |
|    4 | `apps/web/src/settingsSearchRows.generated.ts`      | ready   |   534 |        95 | napi-rs | -                                                                                       |
|    4 | `apps/web/src/splitView.logic.ts`                   | ready   |   245 |        95 | napi-rs | Dynamic typing (2)                                                                      |
|    4 | `apps/web/src/editorMetadata.ts`                    | keep    |   143 |        94 | keep    | Insertion-ordered maps and object keys (2)                                              |
|    4 | `apps/web/src/editorRailStore.ts`                   | keep    |    28 |        94 | keep    | Callback-shaped APIs (3)                                                                |
|    4 | `apps/web/src/openThreadTabsStore.ts`               | ready   |    84 |        94 | napi-rs | Callback-shaped APIs (5)                                                                |
|    4 | `apps/web/src/pendingUserInputRecovery.ts`          | ready   |    82 |        94 | napi-rs | Insertion-ordered maps and object keys (5)                                              |
|    4 | `apps/web/src/appSnap.logic.ts`                     | ready   |   203 |        93 | napi-rs | Insertion-ordered maps and object keys (4); Callback-shaped APIs (3)                    |
|    4 | `apps/web/src/appSnapAttach.ts`                     | caution |    34 |        93 | napi-rs | Async and promises (3); Callback-shaped APIs (1)                                        |
|    4 | `apps/web/src/composerDraftStoreTestFixtures.ts`    | ready   |   196 |        93 | napi-rs | Insertion-ordered maps and object keys (1)                                              |
|    4 | `apps/web/src/messageTextSegments.ts`               | ready   |    72 |        93 | napi-rs | String offsets and slicing (2)                                                          |
|    4 | `apps/web/src/pinnedProjectsStore.ts`               | ready   |    78 |        93 | napi-rs | String offsets and slicing (1); Callback-shaped APIs (3)                                |
|    4 | `apps/web/src/threadVisitedPersistence.ts`          | ready   |   149 |        93 | napi-rs | String offsets and slicing (1); Insertion-ordered maps and object keys (3)              |
|    4 | `apps/web/src/appNavigation.ts`                     | keep    |   137 |        92 | keep    | -                                                                                       |
|    4 | `apps/web/src/editorViewState.ts`                   | ready   |    63 |        92 | napi-rs | String offsets and slicing (1); Insertion-ordered maps and object keys (1)              |
|    4 | `apps/web/src/projectInstructionsStore.ts`          | ready   |    78 |        92 | napi-rs | Regexes built at runtime (1); Callback-shaped APIs (2)                                  |
|    4 | `apps/web/src/railShellStore.ts`                    | ready   |   128 |        92 | napi-rs | Callback-shaped APIs (5); Exceptions (2)                                                |
|    4 | `apps/web/src/threadDerivation.ts`                  | ready   |   165 |        92 | napi-rs | -                                                                                       |
|    4 | `apps/web/src/components/browser/`                  | keep    |   291 |        91 | keep    | Callback-shaped APIs (3)                                                                |
|    4 | `apps/web/src/confirmedCustomBinaryPathStore.ts`    | ready   |    60 |        91 | napi-rs | Insertion-ordered maps and object keys (1); Exceptions (3)                              |
|    4 | `apps/web/src/latestProjectStore.ts`                | ready   |    42 |        91 | napi-rs | Callback-shaped APIs (2)                                                                |
|    4 | `apps/web/src/threadDetailPrewarm.ts`               | keep    |   181 |        91 | keep    | Callback-shaped APIs (12); Insertion-ordered maps and object keys (3)                   |
|    4 | `apps/web/src/voidSpaceStore.ts`                    | ready   |    98 |        91 | napi-rs | String offsets and slicing (1); Callback-shaped APIs (2)                                |
|    4 | `apps/web/src/keybindingEditor.ts`                  | ready   |   478 |        90 | napi-rs | Dynamic typing (1); Insertion-ordered maps and object keys (1)                          |
|    4 | `apps/web/src/nativeApi.ts`                         | ready   |    54 |        90 | napi-rs | Callback-shaped APIs (2); Exceptions (1)                                                |
|    4 | `apps/web/src/router.ts`                            | keep    |    57 |        90 | keep    | Exceptions (2)                                                                          |
|    4 | `apps/web/src/storeState.ts`                        | ready   |    98 |        90 | napi-rs | Reflection and prototypes (1)                                                           |
|    4 | `apps/web/src/contextMenuFallback.ts`               | ready   |   239 |        89 | napi-rs | Dynamic typing (1); Async and promises (1)                                              |
|    4 | `apps/web/src/recentViews.logic.ts`                 | ready   |   253 |        89 | napi-rs | String offsets and slicing (1); Insertion-ordered maps and object keys (3)              |
|    4 | `apps/web/src/routeTree.gen.ts`                     | ready   |   443 |        88 | napi-rs | Dynamic typing (19)                                                                     |
|    4 | `apps/web/src/editorPreferences.ts`                 | ready   |    30 |        87 | napi-rs | Async and promises (3); Insertion-ordered maps and object keys (1)                      |
|    4 | `apps/web/src/pendingInteractionDerivation.ts`      | ready   |   370 |        87 | napi-rs | Insertion-ordered maps and object keys (5); Callback-shaped APIs (1)                    |
|    4 | `apps/web/src/settingsSearchIndex.ts`               | ready   |   435 |        87 | napi-rs | String offsets and slicing (1); Callback-shaped APIs (2)                                |
|    4 | `apps/web/src/storePersistence.ts`                  | ready   |   217 |        87 | napi-rs | Insertion-ordered maps and object keys (7); Callback-shaped APIs (4)                    |
|    4 | `apps/web/src/computerPreviewStore.ts`              | caution |   240 |        86 | napi-rs | Callback-shaped APIs (14)                                                               |
|    4 | `apps/web/src/confirmDialogFallback.ts`             | ready   |    91 |        86 | napi-rs | String offsets and slicing (1); Async and promises (1)                                  |
|    4 | `apps/web/src/recentViewsStore.ts`                  | ready   |   112 |        86 | napi-rs | String offsets and slicing (1); Callback-shaped APIs (2)                                |
|    4 | `apps/web/src/workflowRunUiStore.ts`                | ready   |   167 |        86 | napi-rs | String offsets and slicing (1); Callback-shaped APIs (4)                                |
|    4 | `apps/web/src/components/pdf/`                      | keep    |   377 |        85 | keep    | Callback-shaped APIs (11); Number semantics (2)                                         |
|    4 | `apps/web/src/types.ts`                             | ready   |   399 |        85 | napi-rs | -                                                                                       |
|    4 | `apps/web/src/appRail.logic.ts`                     | ready   |   258 |        84 | napi-rs | String offsets and slicing (2); Insertion-ordered maps and object keys (4)              |
|    4 | `apps/web/src/browserStateStore.ts`                 | ready   |   211 |        84 | napi-rs | String offsets and slicing (2); Callback-shaped APIs (8)                                |
|    4 | `apps/web/src/components/chat/auxiliary/`           | keep    |   199 |        84 | keep    | Async and promises (17); Callback-shaped APIs (2)                                       |
|    4 | `apps/web/src/repoDiffScopeStore.ts`                | keep    |   134 |        84 | keep    | String offsets and slicing (2); Insertion-ordered maps and object keys (3)              |
|    4 | `apps/web/src/shortcutsSheet.ts`                    | ready   |   714 |        83 | napi-rs | Insertion-ordered maps and object keys (3)                                              |
|    4 | `apps/web/src/kanbanUiStore.ts`                     | ready   |   207 |        82 | napi-rs | Callback-shaped APIs (8); Insertion-ordered maps and object keys (3)                    |
|    4 | `apps/web/src/settingsNavigation.ts`                | ready   |   335 |        82 | napi-rs | Insertion-ordered maps and object keys (1); Callback-shaped APIs (1)                    |
|    4 | `apps/web/src/composerDraftStore.ts`                | keep    |   195 |        81 | keep    | Insertion-ordered maps and object keys (4)                                              |
|    4 | `apps/web/src/workspacePathsStore.ts`               | ready   |   136 |        80 | napi-rs | Callback-shaped APIs (5); Insertion-ordered maps and object keys (1)                    |
|    4 | `apps/web/src/markdown-links.ts`                    | ready   |   283 |        79 | napi-rs | String offsets and slicing (14); Insertion-ordered maps and object keys (2)             |
|    4 | `apps/web/src/threadDetailSubscriptionRetention.ts` | keep    |   355 |        79 | keep    | Insertion-ordered maps and object keys (7); Callback-shaped APIs (7)                    |
|    4 | `apps/web/src/spacesUiStore.ts`                     | ready   |   163 |        78 | napi-rs | Insertion-ordered maps and object keys (8); Callback-shaped APIs (7)                    |
|    4 | `apps/web/src/terminal-links.ts`                    | ready   |   237 |        78 | napi-rs | String offsets and slicing (6); Regex features the `regex` crate lacks (2)              |
|    4 | `apps/web/src/composer-logic.ts`                    | ready   |   392 |        77 | napi-rs | String offsets and slicing (17)                                                         |
|    4 | `apps/web/src/lib/projectCatalog/`                  | ready   |   292 |        76 | napi-rs | String offsets and slicing (2); Insertion-ordered maps and object keys (4)              |
|    4 | `apps/web/src/rightDockStore.logic.ts`              | ready   |   437 |        76 | napi-rs | Insertion-ordered maps and object keys (5)                                              |
|    4 | `apps/web/src/rightDockStore.ts`                    | ready   |   142 |        76 | napi-rs | Callback-shaped APIs (12); Reflection and prototypes (2)                                |
|    4 | `apps/web/src/components/automation/`               | keep    |   534 |        75 | keep    | Callback-shaped APIs (12); Exceptions (3)                                               |
|    4 | `apps/web/src/storeTestFixtures.ts`                 | ready   |   301 |        74 | napi-rs | Dynamic typing (5); Insertion-ordered maps and object keys (4)                          |
|    4 | `apps/web/src/onboarding/stage/`                    | keep    |   897 |        72 | keep    | String offsets and slicing (2)                                                          |
|    4 | `apps/web/src/onboarding/steps/`                    | keep    |   731 |        72 | keep    | String offsets and slicing (1); Callback-shaped APIs (7)                                |
|    4 | `apps/web/src/openThreadTabs.logic.ts`              | ready   |   299 |        70 | napi-rs | String offsets and slicing (2); Callback-shaped APIs (12)                               |
|    4 | `apps/web/src/components/computer/`                 | keep    |   574 |        68 | keep    | Async and promises (39); Callback-shaped APIs (7)                                       |
|    4 | `apps/web/src/store.ts`                             | keep    |   388 |        68 | keep    | Callback-shaped APIs (25)                                                               |
|    4 | `apps/web/src/composerDraftModels.ts`               | ready   |   968 |        66 | napi-rs | Insertion-ordered maps and object keys (11); Callback-shaped APIs (1)                   |
|    4 | `apps/web/src/composerSlashCommands.ts`             | ready   |   593 |        66 | napi-rs | String offsets and slicing (5); Regex features the `regex` crate lacks (2)              |
|    4 | `apps/web/src/components/composer-nodes/`           | keep    |   629 |        64 | keep    | String offsets and slicing (2); Class inheritance (6)                                   |
|    4 | `apps/web/src/featureTour/`                         | keep    |   756 |        64 | keep    | String offsets and slicing (1); Callback-shaped APIs (4)                                |
|    4 | `apps/web/src/session-logic.ts`                     | ready   |   710 |        64 | napi-rs | Insertion-ordered maps and object keys (7); Reflection and prototypes (2)               |
|    4 | `apps/web/src/splitViewStore.ts`                    | ready   |   687 |        62 | napi-rs | Callback-shaped APIs (12); Insertion-ordered maps and object keys (3)                   |
|    4 | `apps/web/src/components/device/`                   | keep    |  1166 |        60 | keep    | String offsets and slicing (4); Callback-shaped APIs (3)                                |
|    4 | `apps/web/src/components/settings/views/`           | keep    |  1442 |        60 | keep    | Async and promises (2); Callback-shaped APIs (2)                                        |
|    4 | `apps/web/src/storeSelectors.ts`                    | ready   |   653 |        60 | napi-rs | Callback-shaped APIs (23); Insertion-ordered maps and object keys (6)                   |
|    4 | `apps/web/src/composer-editor-mentions.ts`          | ready   |   398 |        58 | napi-rs | String offsets and slicing (10); Regex features the `regex` crate lacks (4)             |
|    4 | `apps/web/src/notifications/`                       | keep    |   969 |        57 | keep    | String offsets and slicing (1); Insertion-ordered maps and object keys (15)             |
|    4 | `apps/web/src/storeProjection.ts`                   | ready   |  1592 |        56 | napi-rs | Insertion-ordered maps and object keys (15); Callback-shaped APIs (2)                   |
|    4 | `apps/web/src/keybindings.ts`                       | ready   |   927 |        54 | napi-rs | String offsets and slicing (5); Insertion-ordered maps and object keys (5)              |
|    4 | `apps/web/src/composerDraftPersistence.ts`          | ready   |  1674 |        52 | napi-rs | Insertion-ordered maps and object keys (12); Reflection and prototypes (1)              |
|    4 | `apps/web/src/composerDraftAttachments.ts`          | caution |   577 |        50 | napi-rs | String offsets and slicing (6); Insertion-ordered maps and object keys (16)             |
|    4 | `apps/web/src/projectImport/`                       | keep    |   969 |        44 | keep    | Insertion-ordered maps and object keys (9); Async and promises (9)                      |
|    4 | `apps/web/src/terminalStateStore.ts`                | caution |   939 |        43 | napi-rs | Insertion-ordered maps and object keys (25); Callback-shaped APIs (22)                  |
|    4 | `apps/web/src/storeEventReducer.ts`                 | caution |  1948 |        38 | napi-rs | String offsets and slicing (9); Insertion-ordered maps and object keys (8)              |
|    4 | `apps/web/src/theme/`                               | caution |  2198 |        32 | napi-rs | String offsets and slicing (6); Insertion-ordered maps and object keys (1)              |
|    4 | `apps/web/src/wsNativeApi.ts`                       | caution |  1363 |        25 | napi-rs | String offsets and slicing (1); Async and promises (58)                                 |
|    4 | `apps/web/src/composerDraftDomain.ts`               | caution |  1044 |        24 | napi-rs | String offsets and slicing (2); Callback-shaped APIs (73)                               |
|    4 | `apps/web/src/storeNormalization.ts`                | caution |  2365 |        16 | napi-rs | String offsets and slicing (2); Insertion-ordered maps and object keys (29)             |
|    4 | `apps/web/src/composerDraftActions.ts`              | caution |  2345 |        13 | napi-rs | String offsets and slicing (6); Insertion-ordered maps and object keys (24)             |
|    4 | `apps/web/src/onboarding/`                          | keep    |  1442 |        13 | keep    | String offsets and slicing (7); Callback-shaped APIs (18)                               |
|    4 | `apps/web/src/appSettings.ts`                       | keep    |  2899 |         0 | keep    | String offsets and slicing (5); Regex features the `regex` crate lacks (1)              |
|    4 | `apps/web/src/components/`                          | keep    | 97274 |         0 | keep    | String offsets and slicing (150); Regex features the `regex` crate lacks (13)           |
|    4 | `apps/web/src/components/account/`                  | keep    |  2040 |         0 | keep    | String offsets and slicing (1); Async and promises (99)                                 |
|    4 | `apps/web/src/components/chat/`                     | keep    | 81551 |         0 | keep    | String offsets and slicing (165); Regex features the `regex` crate lacks (12)           |
|    4 | `apps/web/src/components/chat/environment/`         | keep    |  6452 |         0 | keep    | String offsets and slicing (7); Regex features the `regex` crate lacks (1)              |
|    4 | `apps/web/src/components/chat/group/`               | keep    |  5136 |         0 | keep    | String offsets and slicing (10); Regexes built at runtime (1)                           |
|    4 | `apps/web/src/components/chat/project/`             | keep    |  3847 |         0 | keep    | Regexes built at runtime (3); Async and promises (209)                                  |
|    4 | `apps/web/src/components/codeEditor/`               | keep    |  5542 |         0 | keep    | String offsets and slicing (9); Regex features the `regex` crate lacks (1)              |
|    4 | `apps/web/src/components/githubInbox/`              | keep    |  3947 |         0 | keep    | String offsets and slicing (3); Regex features the `regex` crate lacks (1)              |
|    4 | `apps/web/src/components/hosts/`                    | keep    |  3858 |         0 | keep    | String offsets and slicing (3); Async and promises (176)                                |
|    4 | `apps/web/src/components/inbox/`                    | keep    |  2478 |         0 | keep    | String offsets and slicing (4); Async and promises (76)                                 |
|    4 | `apps/web/src/components/kanban/`                   | keep    |  5798 |         0 | keep    | String offsets and slicing (16); Async and promises (89)                                |
|    4 | `apps/web/src/components/profile/`                  | keep    |  2056 |         0 | keep    | String offsets and slicing (4); Async and promises (34)                                 |
|    4 | `apps/web/src/components/pullRequest/`              | keep    |  6188 |         0 | keep    | String offsets and slicing (16); Regex features the `regex` crate lacks (4)             |
|    4 | `apps/web/src/components/settings/`                 | keep    | 16917 |         0 | keep    | String offsets and slicing (25); Regex features the `regex` crate lacks (2)             |
|    4 | `apps/web/src/components/settings/accounts/`        | keep    |  3327 |         0 | keep    | String offsets and slicing (2); Async and promises (182)                                |
|    4 | `apps/web/src/components/settings/providers/`       | keep    |  2121 |         0 | keep    | String offsets and slicing (2); Async and promises (82)                                 |
|    4 | `apps/web/src/components/tasks/`                    | keep    |  4530 |         0 | keep    | String offsets and slicing (6); Async and promises (190)                                |
|    4 | `apps/web/src/components/terminal/`                 | keep    |  2711 |         0 | keep    | String offsets and slicing (1); Async and promises (22)                                 |
|    4 | `apps/web/src/components/ui/`                       | keep    |  6646 |         0 | keep    | String offsets and slicing (1); Async and promises (59)                                 |
|    4 | `apps/web/src/hooks/`                               | keep    | 16545 |         0 | keep    | String offsets and slicing (12); Regex features the `regex` crate lacks (1)             |
|    4 | `apps/web/src/lib/`                                 | keep    | 38535 |         0 | keep    | String offsets and slicing (257); Regex features the `regex` crate lacks (47)           |
|    4 | `apps/web/src/lib/hosts/`                           | keep    |  2745 |         0 | keep    | String offsets and slicing (11); Regex features the `regex` crate lacks (1)             |
|    4 | `apps/web/src/routes/`                              | keep    |  8635 |         0 | keep    | String offsets and slicing (1); Insertion-ordered maps and object keys (76)             |
|    4 | `apps/web/src/whatsNew/`                            | keep    |  7313 |         0 | keep    | Callback-shaped APIs (5)                                                                |
|    4 | `apps/web/src/workLog.ts`                           | caution |  4361 |         0 | napi-rs | String offsets and slicing (10); Regex features the `regex` crate lacks (4)             |
|    4 | `apps/web/src/wsTransport.ts`                       | caution |  3021 |         0 | napi-rs | String offsets and slicing (2); Async and promises (83)                                 |
|    5 | `apps/web/src/main.tsx`                             | keep    |    44 |        89 | keep    | -                                                                                       |
|    5 | `apps/web/perf/`                                    | keep    |  1710 |         5 | keep    | String offsets and slicing (8); Async and promises (103)                                |
|    5 | `apps/web/scripts/`                                 | blocked |  1325 |         0 | napi-rs | String offsets and slicing (10); Dynamic code (eval, new Function, computed import) (1) |
|    6 | `apps/web/src/bootstrap.ts`                         | ready   |    76 |        98 | napi-rs | String offsets and slicing (1)                                                          |

### 11. @synara/cli (`apps/server`, node, 698 test files, uses @synara/web, @synara/contracts, @synara/shared)

| Wave | Unit                                                | Status  | Lines | Readiness | Bridge  | Main risks                                                                    |
| ---: | --------------------------------------------------- | ------- | ----: | --------: | ------- | ----------------------------------------------------------------------------- |
|    0 | `apps/server/src/accountUsageReporterRegistry.ts`   | ready   |    25 |       100 | napi-rs | Callback-shaped APIs (2); Exceptions (1)                                      |
|    0 | `apps/server/src/backendCompileCache.ts`            | ready   |    55 |       100 | napi-rs | Callback-shaped APIs (5); Exceptions (1)                                      |
|    0 | `apps/server/src/codexAppServerTransport.ts`        | ready   |    92 |       100 | napi-rs | Class inheritance (3); Exceptions (1)                                         |
|    0 | `apps/server/src/codexErrorClassification.ts`       | ready   |    14 |       100 | napi-rs | Class inheritance (1)                                                         |
|    0 | `apps/server/src/codexServiceTier.ts`               | ready   |    11 |       100 | napi-rs | -                                                                             |
|    0 | `apps/server/src/codexTurnInput.ts`                 | ready   |    42 |       100 | napi-rs | -                                                                             |
|    0 | `apps/server/src/codexWorkingDirectory.ts`          | ready   |    24 |       100 | napi-rs | Exceptions (4)                                                                |
|    0 | `apps/server/src/eventLoopDeadline.ts`              | caution |    32 |       100 | napi-rs | Callback-shaped APIs (3)                                                      |
|    0 | `apps/server/src/executableLookup.ts`               | ready   |     2 |       100 | napi-rs | -                                                                             |
|    0 | `apps/server/src/jpegHeader.ts`                     | ready   |    65 |       100 | napi-rs | -                                                                             |
|    0 | `apps/server/src/migrationBundleIdentity.ts`        | ready   |    91 |       100 | napi-rs | Class inheritance (1); Exceptions (7)                                         |
|    0 | `apps/server/src/pngHeader.ts`                      | ready   |    34 |       100 | napi-rs | -                                                                             |
|    0 | `apps/server/src/shell-command-detection.ts`        | ready   |    14 |       100 | napi-rs | Regex features the `regex` crate lacks (1)                                    |
|    0 | `apps/server/src/attachmentPaths.ts`                | ready   |    32 |        99 | napi-rs | -                                                                             |
|    0 | `apps/server/src/os-jank.ts`                        | ready   |    76 |        99 | napi-rs | Callback-shaped APIs (1); Exceptions (2)                                      |
|    0 | `apps/server/src/project/Services/`                 | caution |     8 |        99 | napi-rs | Class inheritance (1); Callback-shaped APIs (1)                               |
|    0 | `apps/server/src/voiceUploadAdmission.ts`           | ready   |    45 |        99 | napi-rs | Callback-shaped APIs (1); Exceptions (1)                                      |
|    0 | `apps/server/src/composerPrediction/Services/`      | caution |    29 |        98 | napi-rs | Class inheritance (1); Callback-shaped APIs (1)                               |
|    0 | `apps/server/src/hostConnectivitySignals.ts`        | ready   |    40 |        98 | napi-rs | Callback-shaped APIs (5); Exceptions (2)                                      |
|    0 | `apps/server/src/studioWorkspaceScaffold.ts`        | ready   |    63 |        98 | napi-rs | Generators (5)                                                                |
|    0 | `apps/server/src/accountHttp.ts`                    | caution |    56 |        97 | napi-rs | -                                                                             |
|    0 | `apps/server/src/accountRefusal.ts`                 | ready   |    31 |        97 | napi-rs | -                                                                             |
|    0 | `apps/server/src/stream/`                           | caution |    63 |        97 | napi-rs | String offsets and slicing (1); Number semantics (5)                          |
|    0 | `apps/server/src/wsCompatibility.ts`                | ready   |   169 |        97 | napi-rs | -                                                                             |
|    0 | `apps/server/src/accountSsoCallback.ts`             | ready   |   160 |        96 | napi-rs | Callback-shaped APIs (4); Async and promises (2)                              |
|    0 | `apps/server/src/desktopParentLifetime.ts`          | caution |    62 |        96 | napi-rs | Generators (4); Callback-shaped APIs (1)                                      |
|    0 | `apps/server/src/environment/Services/`             | caution |     9 |        96 | napi-rs | Class inheritance (1)                                                         |
|    0 | `apps/server/src/pullRequestAutoFix/Services/`      | caution |    29 |        96 | napi-rs | Class inheritance (2); Callback-shaped APIs (2)                               |
|    0 | `apps/server/src/transcriptUsageParsers.ts`         | ready   |   290 |        96 | napi-rs | JSON round-trips (2); Exceptions (1)                                          |
|    0 | `apps/server/src/betaFeatureGate.ts`                | ready   |    12 |        95 | napi-rs | -                                                                             |
|    0 | `apps/server/src/endpointReporter.ts`               | ready   |    66 |        95 | napi-rs | Callback-shaped APIs (3); Async and promises (2)                              |
|    0 | `apps/server/src/frameSocketByteStream.ts`          | caution |    42 |        95 | napi-rs | Exceptions (1)                                                                |
|    0 | `apps/server/src/server/`                           | ready   |    66 |        95 | napi-rs | Generators (8)                                                                |
|    0 | `apps/server/src/todo/`                             | caution |     8 |        95 | napi-rs | Class inheritance (1)                                                         |
|    0 | `apps/server/src/editor/test-fixtures/`             | ready   |    72 |        94 | napi-rs | String offsets and slicing (2); JSON round-trips (3)                          |
|    0 | `apps/server/src/voiceTranscription.ts`             | caution |   151 |        94 | napi-rs | Async and promises (6); Callback-shaped APIs (1)                              |
|    0 | `apps/server/src/imageMime.ts`                      | ready   |    63 |        93 | napi-rs | String offsets and slicing (2); Insertion-ordered maps and object keys (1)    |
|    0 | `apps/server/src/lazyModule.ts`                     | caution |    22 |        93 | napi-rs | Callback-shaped APIs (2)                                                      |
|    0 | `apps/server/src/pullRequests/Services/`            | caution |    31 |        93 | napi-rs | Callback-shaped APIs (5); Class inheritance (1)                               |
|    0 | `apps/server/src/staticAssets.ts`                   | ready   |   142 |        93 | napi-rs | String offsets and slicing (4); Insertion-ordered maps and object keys (1)    |
|    0 | `apps/server/src/environment/`                      | ready   |    63 |        92 | napi-rs | String offsets and slicing (3); Async and promises (7)                        |
|    0 | `apps/server/src/providerAccountHomePath.ts`        | ready   |    19 |        92 | napi-rs | String offsets and slicing (1)                                                |
|    0 | `apps/server/src/remoteFeaturePolicy.ts`            | ready   |    59 |        92 | napi-rs | Exceptions (3)                                                                |
|    0 | `apps/server/src/backoff.ts`                        | ready   |    23 |        91 | napi-rs | Callback-shaped APIs (1)                                                      |
|    0 | `apps/server/src/jwtClaims.ts`                      | ready   |    14 |        91 | napi-rs | Exceptions (1)                                                                |
|    0 | `apps/server/src/relaySocket.ts`                    | ready   |   143 |        91 | napi-rs | Callback-shaped APIs (20); Exceptions (1)                                     |
|    0 | `apps/server/src/bunWebSocketCompatibility.ts`      | ready   |   108 |        90 | napi-rs | Callback-shaped APIs (6); Reflection and prototypes (3)                       |
|    0 | `apps/server/src/realpathNearestExisting.ts`        | ready   |    53 |        90 | napi-rs | Generators (5)                                                                |
|    0 | `apps/server/src/serverLifecycleEvents.ts`          | caution |    87 |        90 | napi-rs | Generators (3); Class inheritance (1)                                         |
|    0 | `apps/server/src/serverStatusCli.ts`                | ready   |   165 |        90 | napi-rs | Async and promises (3); Callback-shaped APIs (1)                              |
|    0 | `apps/server/src/editorAppDiscovery.ts`             | ready   |   226 |        89 | napi-rs | Dynamic typing (3); Insertion-ordered maps and object keys (2)                |
|    0 | `apps/server/src/reliability.ts`                    | ready   |    23 |        89 | napi-rs | Insertion-ordered maps and object keys (1); Exceptions (1)                    |
|    0 | `apps/server/src/siteFaviconCache.ts`               | ready   |   161 |        89 | napi-rs | String offsets and slicing (1); Async and promises (9)                        |
|    0 | `apps/server/src/claudeTokenStats.ts`               | ready   |   371 |        88 | napi-rs | Dynamic typing (2)                                                            |
|    0 | `apps/server/src/desktopUpdateBridge.ts`            | ready   |    78 |        88 | napi-rs | Async and promises (3); Insertion-ordered maps and object keys (2)            |
|    0 | `apps/server/src/startupAccess.ts`                  | ready   |    20 |        88 | napi-rs | String offsets and slicing (1); Dynamic typing (1)                            |
|    0 | `apps/server/perf/`                                 | ready   |   166 |        87 | napi-rs | String offsets and slicing (1); Callback-shaped APIs (3)                      |
|    0 | `apps/server/src/serverRuntimeStartup.ts`           | caution |    57 |        87 | napi-rs | Generators (9); Class inheritance (2)                                         |
|    0 | `apps/server/src/tailscaleEndpoint.ts`              | caution |    67 |        87 | napi-rs | String offsets and slicing (2); Async and promises (5)                        |
|    0 | `apps/server/src/logger.ts`                         | ready   |    90 |        86 | napi-rs | String offsets and slicing (1); Insertion-ordered maps and object keys (1)    |
|    0 | `apps/server/src/managedAttachmentPrincipal.ts`     | ready   |    51 |        85 | napi-rs | Insertion-ordered maps and object keys (1); Callback-shaped APIs (1)          |
|    0 | `apps/server/src/gitHandoffOperations.ts`           | ready   |   176 |        83 | napi-rs | Generators (27); Class inheritance (1)                                        |
|    0 | `apps/server/src/groupWorkspaceScaffold.ts`         | ready   |   111 |        83 | napi-rs | Generators (9); Async and promises (5)                                        |
|    0 | `apps/server/src/providerChildEnvironment.ts`       | ready   |   133 |        83 | napi-rs | Insertion-ordered maps and object keys (15)                                   |
|    0 | `apps/server/src/wsStreamAdmission.ts`              | caution |   248 |        82 | napi-rs | Generators (10); Insertion-ordered maps and object keys (8)                   |
|    0 | `apps/server/src/codexAuthIo.ts`                    | caution |   100 |        81 | napi-rs | Async and promises (7); Callback-shaped APIs (7)                              |
|    0 | `apps/server/src/workspace/Services/`               | caution |   195 |        81 | napi-rs | Callback-shaped APIs (14); Class inheritance (11)                             |
|    0 | `apps/server/src/processArgumentRedaction.ts`       | ready   |   347 |        78 | napi-rs | String offsets and slicing (7); Regex features the `regex` crate lacks (1)    |
|    0 | `apps/server/src/studioOutputs.ts`                  | ready   |   379 |        78 | napi-rs | Insertion-ordered maps and object keys (10); Generators (9)                   |
|    0 | `apps/server/src/privatePathPermissions.ts`         | ready   |   177 |        76 | napi-rs | Async and promises (5); Class inheritance (1)                                 |
|    0 | `apps/server/src/browserAutomation/`                | ready   |   330 |        75 | napi-rs | Async and promises (8); Dynamic typing (2)                                    |
|    0 | `apps/server/steer-probe2.mjs`                      | ready   |   104 |        73 | napi-rs | String offsets and slicing (7); Async and promises (5)                        |
|    0 | `apps/server/src/nodeHttpServer.ts`                 | ready   |   280 |        70 | napi-rs | Generators (14); Dynamic typing (4)                                           |
|    0 | `apps/server/src/wsStreamBackpressure.ts`           | ready   |   413 |        69 | napi-rs | Generators (35); Callback-shaped APIs (7)                                     |
|    0 | `apps/server/src/auth/Services/`                    | caution |   362 |        63 | napi-rs | Callback-shaped APIs (47); Class inheritance (12)                             |
|    0 | `apps/server/src/automation/`                       | ready   |   696 |        60 | napi-rs | String offsets and slicing (5); Insertion-ordered maps and object keys (4)    |
|    1 | `apps/server/src/codexPathIdentity.ts`              | ready   |    42 |       100 | napi-rs | Generators (5); Exceptions (1)                                                |
|    1 | `apps/server/vitest.shards.ts`                      | ready   |    16 |       100 | napi-rs | -                                                                             |
|    1 | `apps/server/src/betaOperationalIssue.ts`           | ready   |    22 |        99 | napi-rs | Exceptions (1); Number semantics (1)                                          |
|    1 | `apps/server/src/browserAutomation/Services/`       | caution |    23 |        97 | napi-rs | Class inheritance (1); Callback-shaped APIs (1)                               |
|    1 | `apps/server/src/codexHomePaths.ts`                 | ready   |   117 |        94 | napi-rs | String offsets and slicing (2)                                                |
|    1 | `apps/server/src/sensitiveKeys.ts`                  | ready   |   105 |        93 | napi-rs | String offsets and slicing (1)                                                |
|    1 | `apps/server/src/todo/Services/`                    | caution |    23 |        91 | napi-rs | Callback-shaped APIs (4); Class inheritance (1)                               |
|    1 | `apps/server/src/effectRelaySocket.ts`              | caution |    52 |        90 | napi-rs | Async and promises (3); Class inheritance (1)                                 |
|    1 | `apps/server/src/scratchWorkspaces.ts`              | ready   |   207 |        89 | napi-rs | String offsets and slicing (3); Exceptions (8)                                |
|    1 | `apps/server/src/attachmentStore.ts`                | ready   |   112 |        84 | napi-rs | String offsets and slicing (3); Regexes built at runtime (1)                  |
|    1 | `apps/server/src/automation/Services/`              | caution |   123 |        80 | napi-rs | Callback-shaped APIs (21); Class inheritance (3)                              |
|    1 | `apps/server/src/wsSnapshotLiveStream.ts`           | caution |   315 |        76 | napi-rs | Generators (23); Callback-shaped APIs (6)                                     |
|    1 | `apps/server/src/config.ts`                         | ready   |   323 |        72 | napi-rs | Generators (19); Class inheritance (1)                                        |
|    1 | `apps/server/src/atomicWrite.ts`                    | ready   |   172 |        70 | napi-rs | Async and promises (26); Callback-shaped APIs (1)                             |
|    1 | `apps/server/src/open.ts`                           | ready   |   441 |        64 | napi-rs | String offsets and slicing (1); Generators (9)                                |
|    1 | `apps/server/src/editorAppIcons.ts`                 | caution |   690 |        47 | napi-rs | String offsets and slicing (5); Regex features the `regex` crate lacks (3)    |
|    1 | `apps/server/src/localServerMonitor.ts`             | caution |   940 |        22 | napi-rs | String offsets and slicing (21); Regex features the `regex` crate lacks (19)  |
|    2 | `apps/server/src/memoryDiagnostics.ts`              | ready   |   103 |        98 | napi-rs | Callback-shaped APIs (3); Insertion-ordered maps and object keys (1)          |
|    2 | `apps/server/src/browserAutomation/Layers/`         | caution |    46 |        96 | napi-rs | -                                                                             |
|    2 | `apps/server/src/betaUsageSnapshot.ts`              | ready   |    59 |        95 | napi-rs | Generators (9); JSON round-trips (1)                                          |
|    2 | `apps/server/src/serverLogger.ts`                   | ready   |    15 |        94 | napi-rs | Generators (3)                                                                |
|    2 | `apps/server/src/serverShutdown.ts`                 | ready   |    77 |        94 | napi-rs | String offsets and slicing (1); Generators (2)                                |
|    2 | `apps/server/src/environment/Layers/`               | ready   |   118 |        93 | napi-rs | Generators (15)                                                               |
|    2 | `apps/server/src/trustedOrigins.ts`                 | ready   |   133 |        93 | napi-rs | String offsets and slicing (1); Insertion-ordered maps and object keys (1)    |
|    2 | `apps/server/src/inlineVisualStore.ts`              | ready   |    85 |        90 | napi-rs | Async and promises (8); Exceptions (3)                                        |
|    2 | `apps/server/src/codexGeneratedImages.ts`           | ready   |   346 |        86 | napi-rs | Insertion-ordered maps and object keys (3); JSON round-trips (1)              |
|    2 | `apps/server/src/hostIdentity/`                     | ready   |   167 |        77 | napi-rs | Async and promises (11); Exceptions (7)                                       |
|    2 | `apps/server/src/eventLoopMonitor.ts`               | caution |   284 |        76 | napi-rs | Callback-shaped APIs (9); Generators (6)                                      |
|    2 | `apps/server/src/hostUsageSnapshot.ts`              | ready   |   286 |        72 | napi-rs | String offsets and slicing (2); Generators (25)                               |
|    2 | `apps/server/src/usagePricing.ts`                   | ready   |   279 |        64 | napi-rs | String offsets and slicing (5); Async and promises (12)                       |
|    2 | `apps/server/src/keybindings.ts`                    | caution |  1331 |        30 | napi-rs | String offsets and slicing (2); Generators (83)                               |
|    2 | `apps/server/src/providerUsageSnapshot.ts`          | caution |  1084 |        30 | napi-rs | String offsets and slicing (8); Async and promises (55)                       |
|    2 | `apps/server/src/profileStats.ts`                   | caution |  1627 |         9 | napi-rs | String offsets and slicing (11); Regexes built at runtime (1)                 |
|    3 | `apps/server/src/transcriptUsage.ts`                | ready   |   333 |        75 | napi-rs | String offsets and slicing (2); Async and promises (25)                       |
|    3 | `apps/server/src/hostAuth/`                         | ready   |   451 |        56 | napi-rs | Async and promises (26); Callback-shaped APIs (6)                             |
|    3 | `apps/server/src/recapStats.ts`                     | ready   |   496 |        56 | napi-rs | String offsets and slicing (3); Insertion-ordered maps and object keys (7)    |
|    4 | `apps/server/src/usageAnalyticsStats.ts`            | caution |   870 |        45 | napi-rs | String offsets and slicing (2); Insertion-ordered maps and object keys (17)   |
|    5 | `apps/server/src/accountRpcErrors.ts`               | ready   |    75 |       100 | napi-rs | -                                                                             |
|    5 | `apps/server/src/diagnostics/`                      | caution |    46 |       100 | napi-rs | JSON round-trips (1)                                                          |
|    5 | `apps/server/src/pullRequests.logic.ts`             | ready   |   137 |       100 | napi-rs | Insertion-ordered maps and object keys (1)                                    |
|    5 | `apps/server/src/worktreeSetup.ts`                  | ready   |    29 |       100 | napi-rs | Async and promises (2)                                                        |
|    5 | `apps/server/src/wsOwnerAuthorization.ts`           | ready   |    15 |       100 | napi-rs | Generators (3)                                                                |
|    5 | `apps/server/src/serverRuntimeState.ts`             | ready   |    48 |        97 | napi-rs | Generators (3); JSON round-trips (1)                                          |
|    5 | `apps/server/src/wsHostsRpc.ts`                     | caution |   169 |        97 | napi-rs | Callback-shaped APIs (2)                                                      |
|    5 | `apps/server/src/providerProcessPriority.ts`        | ready   |    23 |        96 | napi-rs | Generators (4)                                                                |
|    5 | `apps/server/src/computer/Services/`                | caution |    11 |        95 | napi-rs | Class inheritance (1)                                                         |
|    5 | `apps/server/src/device/Services/`                  | caution |    15 |        95 | napi-rs | Class inheritance (1)                                                         |
|    5 | `apps/server/src/wsOwnerOnly.ts`                    | ready   |    35 |        95 | napi-rs | Generators (5)                                                                |
|    5 | `apps/server/src/workspaceFileChanges.ts`           | caution |   139 |        94 | napi-rs | Async and promises (3); Class inheritance (1)                                 |
|    5 | `apps/server/src/diagnostics/Services/`             | caution |    61 |        92 | napi-rs | Callback-shaped APIs (4); Class inheritance (1)                               |
|    5 | `apps/server/src/githubInbox/Services/`             | caution |    49 |        92 | napi-rs | Callback-shaped APIs (5); Class inheritance (2)                               |
|    5 | `apps/server/src/wsAccountRpc.ts`                   | caution |   193 |        91 | napi-rs | Generators (4); Callback-shaped APIs (3)                                      |
|    5 | `apps/server/src/providerCredentials.ts`            | caution |    66 |        90 | napi-rs | Callback-shaped APIs (3); Generators (2)                                      |
|    5 | `apps/server/src/mcpConnectors/Layers/`             | ready   |   157 |        89 | napi-rs | Generators (6); Async and promises (3)                                        |
|    5 | `apps/server/src/accountSignInTiming.ts`            | caution |    93 |        87 | napi-rs | Callback-shaped APIs (5); Async and promises (2)                              |
|    5 | `apps/server/src/accountCredentialLock.ts`          | caution |    56 |        85 | napi-rs | Async and promises (11); Insertion-ordered maps and object keys (1)           |
|    5 | `apps/server/src/mcpConnectors/Services/`           | caution |    70 |        85 | napi-rs | Callback-shaped APIs (10); Class inheritance (2)                              |
|    5 | `apps/server/src/wsRequestAdmission.ts`             | caution |   197 |        85 | napi-rs | Insertion-ordered maps and object keys (9); Generators (5)                    |
|    5 | `apps/server/src/accountDeviceRegistration.ts`      | caution |   178 |        84 | napi-rs | Async and promises (27); Class inheritance (1)                                |
|    5 | `apps/server/src/checkpointing/Services/`           | caution |   181 |        83 | napi-rs | Callback-shaped APIs (10); Class inheritance (2)                              |
|    5 | `apps/server/src/desktopUpdateRpc.ts`               | ready   |   177 |        83 | napi-rs | Generators (16); Insertion-ordered maps and object keys (2)                   |
|    5 | `apps/server/src/deviceIdentity/`                   | ready   |   171 |        82 | napi-rs | Async and promises (17); Callback-shaped APIs (3)                             |
|    5 | `apps/server/src/localImageFiles.ts`                | ready   |   284 |        81 | napi-rs | Async and promises (30); Insertion-ordered maps and object keys (4)           |
|    5 | `apps/server/src/auth/`                             | ready   |   167 |        80 | napi-rs | String offsets and slicing (1); Generators (6)                                |
|    5 | `apps/server/src/processRunner.ts`                  | caution |   316 |        80 | napi-rs | Callback-shaped APIs (5); Async and promises (2)                              |
|    5 | `apps/server/src/threadSearch.ts`                   | ready   |   145 |        80 | napi-rs | String offsets and slicing (3); Generators (4)                                |
|    5 | `apps/server/src/devServerManager.ts`               | ready   |   178 |        78 | napi-rs | String offsets and slicing (1); Generators (17)                               |
|    5 | `apps/server/src/terminal/Services/`                | caution |   232 |        78 | napi-rs | Callback-shaped APIs (25); Class inheritance (4)                              |
|    5 | `apps/server/src/wsConnectionSessions.ts`           | ready   |   135 |        78 | napi-rs | Callback-shaped APIs (7); Dynamic typing (2)                                  |
|    5 | `apps/server/src/studioGeneratedImages.ts`          | ready   |   235 |        76 | napi-rs | String offsets and slicing (1); Async and promises (14)                       |
|    5 | `apps/server/src/externalMcp/Services/`             | ready   |   309 |        75 | napi-rs | Callback-shaped APIs (38); Class inheritance (4)                              |
|    5 | `apps/server/src/threadRetention.ts`                | ready   |   317 |        75 | napi-rs | Generators (20); Insertion-ordered maps and object keys (7)                   |
|    5 | `apps/server/src/projectAgent/Services/`            | caution |   313 |        74 | napi-rs | Callback-shaped APIs (54); Class inheritance (2)                              |
|    5 | `apps/server/src/managedAttachmentStore.ts`         | caution |   267 |        72 | napi-rs | String offsets and slicing (1); Generators (17)                               |
|    5 | `apps/server/src/keepAwake.ts`                      | ready   |   290 |        68 | napi-rs | Generators (73); Callback-shaped APIs (7)                                     |
|    5 | `apps/server/src/agentGateway/Services/`            | ready   |   378 |        66 | napi-rs | Callback-shaped APIs (39); Class inheritance (4)                              |
|    5 | `apps/server/src/githubInbox/`                      | ready   |   584 |        63 | napi-rs | String offsets and slicing (2); Generators (10)                               |
|    5 | `apps/server/src/hostSecrets/`                      | ready   |   549 |        63 | napi-rs | Async and promises (66); Callback-shaped APIs (9)                             |
|    5 | `apps/server/src/assistantImageAttachments.ts`      | caution |   328 |        61 | napi-rs | String offsets and slicing (2); Generators (13)                               |
|    5 | `apps/server/src/project/`                          | caution |   543 |        58 | napi-rs | String offsets and slicing (3); Regex features the `regex` crate lacks (1)    |
|    5 | `apps/server/src/checkpointing/`                    | ready   |   397 |        53 | napi-rs | String offsets and slicing (11); Insertion-ordered maps and object keys (2)   |
|    5 | `apps/server/src/workspace/`                        | ready   |   342 |        52 | napi-rs | String offsets and slicing (5); Async and promises (36)                       |
|    5 | `apps/server/src/profileStatsArchive.ts`            | caution |  1022 |        50 | napi-rs | Generators (63); Insertion-ordered maps and object keys (6)                   |
|    5 | `apps/server/src/workspace/Layers/`                 | caution |   698 |        49 | napi-rs | Regex features the `regex` crate lacks (2); String offsets and slicing (1)    |
|    5 | `apps/server/src/orchestration/Services/`           | caution |   959 |        47 | napi-rs | Callback-shaped APIs (56); Class inheritance (15)                             |
|    5 | `apps/server/src/remotePairing/`                    | caution |   855 |        47 | napi-rs | Async and promises (86); Insertion-ordered maps and object keys (2)           |
|    5 | `apps/server/src/managedWorktrees.ts`               | caution |   632 |        46 | napi-rs | String offsets and slicing (4); Async and promises (33)                       |
|    5 | `apps/server/src/provider/Services/`                | caution |  1146 |        40 | napi-rs | Callback-shaped APIs (91); Class inheritance (17)                             |
|    5 | `apps/server/src/remoteSessions/`                   | caution |   645 |        38 | napi-rs | Async and promises (24); Callback-shaped APIs (24)                            |
|    5 | `apps/server/src/git/Services/`                     | caution |  1311 |        37 | napi-rs | Callback-shaped APIs (108); Class inheritance (10)                            |
|    5 | `apps/server/src/pullRequests/`                     | caution |   868 |        36 | napi-rs | String offsets and slicing (7); Generators (38)                               |
|    5 | `apps/server/src/workspaceEntries.ts`               | caution |  1302 |        33 | napi-rs | String offsets and slicing (11); Async and promises (74)                      |
|    5 | `apps/server/src/auth/Layers/`                      | caution |  1681 |        29 | napi-rs | String offsets and slicing (1); Generators (159)                              |
|    5 | `apps/server/src/accountSession.ts`                 | caution |  1361 |        27 | napi-rs | String offsets and slicing (1); Async and promises (147)                      |
|    5 | `apps/server/src/remoteTransport/`                  | caution |   895 |        25 | napi-rs | String offsets and slicing (1); Async and promises (52)                       |
|    5 | `apps/server/src/serverSettings.ts`                 | caution |  1338 |        15 | napi-rs | String offsets and slicing (1); Generators (84)                               |
|    5 | `apps/server/src/terminal/`                         | caution |  1526 |        10 | napi-rs | String offsets and slicing (8); Insertion-ordered maps and object keys (21)   |
|    5 | `apps/server/src/accountAuth.ts`                    | caution |  1671 |         2 | napi-rs | String offsets and slicing (1); Async and promises (140)                      |
|    5 | `apps/server/src/agentGateway/`                     | caution | 20509 |         0 | napi-rs | String offsets and slicing (33); Regex features the `regex` crate lacks (1)   |
|    5 | `apps/server/src/agentGateway/Layers/`              | caution |  2488 |         0 | napi-rs | Generators (144); Insertion-ordered maps and object keys (16)                 |
|    5 | `apps/server/src/codexAppServerManager.ts`          | caution |  5566 |         0 | napi-rs | String offsets and slicing (6); Regex features the `regex` crate lacks (4)    |
|    5 | `apps/server/src/codexProcessEnv.ts`                | caution |  2913 |         0 | napi-rs | String offsets and slicing (29); Regexes built at runtime (1)                 |
|    5 | `apps/server/src/computer/`                         | caution | 15691 |         0 | napi-rs | String offsets and slicing (41); Regex features the `regex` crate lacks (22)  |
|    5 | `apps/server/src/device/`                           | caution |  6796 |         0 | napi-rs | String offsets and slicing (27); Async and promises (496)                     |
|    5 | `apps/server/src/editor/`                           | caution |  1800 |         0 | napi-rs | String offsets and slicing (19); Regex features the `regex` crate lacks (4)   |
|    5 | `apps/server/src/externalMcp/`                      | caution |  1675 |         0 | napi-rs | String offsets and slicing (2); Generators (68)                               |
|    5 | `apps/server/src/git/`                              | caution |  2019 |         0 | napi-rs | String offsets and slicing (35); Regex features the `regex` crate lacks (1)   |
|    5 | `apps/server/src/git/Layers/`                       | caution | 14400 |         0 | napi-rs | String offsets and slicing (48); Regex features the `regex` crate lacks (10)  |
|    5 | `apps/server/src/hostConnections/`                  | caution |  1789 |         0 | napi-rs | String offsets and slicing (2); Async and promises (63)                       |
|    5 | `apps/server/src/mcpConnectors/`                    | caution |  2389 |         0 | napi-rs | String offsets and slicing (10); Regexes built at runtime (1)                 |
|    5 | `apps/server/src/orchestration/`                    | caution | 14047 |         0 | napi-rs | String offsets and slicing (51); Regex features the `regex` crate lacks (3)   |
|    5 | `apps/server/src/orchestration/Layers/`             | caution | 30251 |         0 | napi-rs | String offsets and slicing (15); Regexes built at runtime (1)                 |
|    5 | `apps/server/src/persistence/`                      | caution |  4450 |         0 | napi-rs | String offsets and slicing (5); Regex features the `regex` crate lacks (2)    |
|    5 | `apps/server/src/persistence/Layers/`               | caution | 11560 |         0 | napi-rs | Regex features the `regex` crate lacks (1); Generators (315)                  |
|    5 | `apps/server/src/persistence/Migrations/`           | caution |  5963 |         0 | napi-rs | Generators (917); Insertion-ordered maps and object keys (3)                  |
|    5 | `apps/server/src/persistence/Services/`             | caution |  3278 |         0 | napi-rs | String offsets and slicing (1); Callback-shaped APIs (280)                    |
|    5 | `apps/server/src/platform/`                         | caution |  1697 |         0 | napi-rs | String offsets and slicing (6); Async and promises (74)                       |
|    5 | `apps/server/src/projectAgent/`                     | caution |  2692 |         0 | napi-rs | String offsets and slicing (12); Generators (207)                             |
|    5 | `apps/server/src/provider/`                         | blocked | 16800 |         0 | napi-rs | String offsets and slicing (129); Regex features the `regex` crate lacks (20) |
|    5 | `apps/server/src/provider/Layers/`                  | caution | 47756 |         0 | napi-rs | String offsets and slicing (53); Regex features the `regex` crate lacks (3)   |
|    5 | `apps/server/src/provider/acp/`                     | caution |  8850 |         0 | napi-rs | String offsets and slicing (34); Regex features the `regex` crate lacks (2)   |
|    5 | `apps/server/src/providerUsage/`                    | caution |  1662 |         0 | napi-rs | String offsets and slicing (4); Async and promises (58)                       |
|    5 | `apps/server/src/providerUsage/providers/`          | caution |  3585 |         0 | napi-rs | String offsets and slicing (3); Async and promises (161)                      |
|    5 | `apps/server/src/terminal/Layers/`                  | caution |  2742 |         0 | napi-rs | String offsets and slicing (24); Async and promises (105)                     |
|    5 | `apps/server/src/wsRpc.ts`                          | caution |  4000 |         0 | napi-rs | String offsets and slicing (3); Generators (310)                              |
|    6 | `apps/server/src/orchestration/testing/`            | caution |    47 |       100 | napi-rs | -                                                                             |
|    6 | `apps/server/src/provider/acp/testing/`             | caution |    61 |       100 | napi-rs | -                                                                             |
|    6 | `apps/server/src/provider/testing/`                 | ready   |    92 |       100 | napi-rs | JSON round-trips (3); Number semantics (2)                                    |
|    6 | `apps/server/src/pullRequestAutoFix/`               | ready   |   169 |       100 | napi-rs | String offsets and slicing (1)                                                |
|    6 | `apps/server/src/computer/Layers/`                  | caution |    94 |        97 | napi-rs | Generators (5)                                                                |
|    6 | `apps/server/src/compactDatabase.ts`                | caution |    48 |        95 | napi-rs | String offsets and slicing (1); Async and promises (2)                        |
|    6 | `apps/server/src/restoreMigrationBackup.ts`         | ready   |    69 |        94 | napi-rs | String offsets and slicing (1); Async and promises (2)                        |
|    6 | `apps/server/src/diagnostics/Layers/`               | ready   |   173 |        93 | napi-rs | Generators (6); JSON round-trips (2)                                          |
|    6 | `apps/server/src/pullRequests/Layers/`              | caution |   109 |        93 | napi-rs | Generators (9); Callback-shaped APIs (1)                                      |
|    6 | `apps/server/src/device/Layers/`                    | ready   |    69 |        89 | napi-rs | Generators (3); Async and promises (2)                                        |
|    6 | `apps/server/src/todo/Layers/`                      | ready   |   180 |        89 | napi-rs | Generators (20); Dynamic typing (1)                                           |
|    6 | `apps/server/src/composerPrediction/`               | ready   |   305 |        88 | napi-rs | String offsets and slicing (7)                                                |
|    6 | `apps/server/src/hostConnectivitySupervisor.ts`     | ready   |   249 |        85 | napi-rs | Async and promises (15); Callback-shaped APIs (7)                             |
|    6 | `apps/server/src/git/testing/`                      | ready   |   559 |        81 | napi-rs | String offsets and slicing (1); Dynamic typing (1)                            |
|    6 | `apps/server/src/managedAttachmentCleanup.ts`       | caution |   188 |        80 | napi-rs | String offsets and slicing (1); Generators (24)                               |
|    6 | `apps/server/src/project/Layers/`                   | ready   |   154 |        80 | napi-rs | Regex features the `regex` crate lacks (3); String offsets and slicing (1)    |
|    6 | `apps/server/src/accountInboxReporter.ts`           | caution |   210 |        77 | napi-rs | String offsets and slicing (2); Async and promises (18)                       |
|    6 | `apps/server/src/betaImport.ts`                     | ready   |   423 |        74 | napi-rs | String offsets and slicing (3); Async and promises (9)                        |
|    6 | `apps/server/src/cloudflare/`                       | caution |   496 |        73 | napi-rs | Async and promises (45); Callback-shaped APIs (12)                            |
|    6 | `apps/server/src/remoteBillingPolicy.ts`            | ready   |   345 |        68 | napi-rs | Async and promises (22); Callback-shaped APIs (7)                             |
|    6 | `apps/server/src/githubInbox/Layers/`               | caution |   471 |        67 | napi-rs | String offsets and slicing (1); Generators (32)                               |
|    6 | `apps/server/src/completionNotificationPreviews.ts` | caution |   293 |        65 | napi-rs | String offsets and slicing (1); Generators (9)                                |
|    6 | `apps/server/src/runtimeDependencySmoke.ts`         | ready   |   235 |        57 | napi-rs | String offsets and slicing (1); Regex features the `regex` crate lacks (1)    |
|    6 | `apps/server/src/accountUsageReporter.ts`           | caution |  1075 |        35 | napi-rs | String offsets and slicing (5); Async and promises (28)                       |
|    6 | `apps/server/src/http.ts`                           | caution |  1594 |        31 | napi-rs | String offsets and slicing (3); Generators (183)                              |
|    6 | `apps/server/src/checkpointing/Layers/`             | caution |  1518 |        28 | napi-rs | String offsets and slicing (7); Generators (143)                              |
|    6 | `apps/server/src/externalMcp/Layers/`               | caution |  2129 |        20 | napi-rs | String offsets and slicing (1); Generators (126)                              |
|    6 | `apps/server/scripts/`                              | caution |  2172 |         0 | napi-rs | String offsets and slicing (9); Generators (104)                              |
|    6 | `apps/server/src/automation/Layers/`                | caution |  3969 |         0 | napi-rs | String offsets and slicing (2); Regex features the `regex` crate lacks (1)    |
|    6 | `apps/server/src/projectAgent/Layers/`              | caution |  7113 |         0 | napi-rs | String offsets and slicing (14); Generators (825)                             |
|    7 | `apps/server/src/composerPrediction/Layers/`        | ready   |   257 |        78 | napi-rs | String offsets and slicing (1); Generators (28)                               |
|    7 | `apps/server/src/effectServer.ts`                   | caution |   355 |        77 | napi-rs | Generators (55); Class inheritance (2)                                        |
|    7 | `apps/server/src/hostConnectivity.ts`               | caution |   496 |        72 | napi-rs | Async and promises (55); Callback-shaped APIs (5)                             |
|    7 | `apps/server/src/pullRequestAutoFix/Layers/`        | ready   |   496 |        72 | napi-rs | String offsets and slicing (1); Generators (75)                               |
|    7 | `apps/server/src/accountLiveActivityReporter.ts`    | caution |   383 |        70 | napi-rs | Async and promises (16); Generators (9)                                       |
|    7 | `apps/server/integration/`                          | caution |  1088 |        41 | napi-rs | String offsets and slicing (1); Generators (50)                               |
|    8 | `apps/server/src/serverLayers.ts`                   | ready   |   396 |        91 | napi-rs | -                                                                             |
|    9 | `apps/server/src/main.ts`                           | caution |  1136 |        45 | napi-rs | Generators (103); Async and promises (8)                                      |
|   10 | `apps/server/src/index.ts`                          | ready   |    33 |        95 | napi-rs | -                                                                             |

### 12. @synara/scripts (`scripts`, node, 46 test files, uses @synara/contracts, @synara/shared)

| Wave | Unit                                            | Status  | Lines | Readiness | Bridge  | Main risks                                                                               |
| ---: | ----------------------------------------------- | ------- | ----: | --------: | ------- | ---------------------------------------------------------------------------------------- |
|    0 | `scripts/patch-workspace-typescript.ts`         | caution |    59 |       100 | napi-rs | Dynamic typing (1); Insertion-ordered maps and object keys (1)                           |
|    0 | `scripts/report-build-runner.ts`                | ready   |    15 |       100 | napi-rs | JSON round-trips (1)                                                                     |
|    0 | `scripts/check-server-sync-fs.ts`               | ready   |   180 |        99 | napi-rs | Insertion-ordered maps and object keys (7); JSON round-trips (1)                         |
|    0 | `scripts/bun-lockfile.ts`                       | ready   |    17 |        93 | napi-rs | Regex features the `regex` crate lacks (1); Insertion-ordered maps and object keys (1)   |
|    0 | `scripts/typecheck.ts`                          | ready   |   105 |        93 | napi-rs | String offsets and slicing (1); Async and promises (10)                                  |
|    0 | `scripts/update-release-package-versions.ts`    | ready   |    93 |        93 | napi-rs | String offsets and slicing (1); Exceptions (5)                                           |
|    0 | `scripts/claude-cache-report.ts`                | caution |   263 |        86 | napi-rs | String offsets and slicing (1); Insertion-ordered maps and object keys (8)               |
|    0 | `scripts/device-helper-sweep.ts`                | ready   |   155 |        84 | napi-rs | String offsets and slicing (7); Insertion-ordered maps and object keys (1)               |
|    0 | `scripts/benchmark-cua-build.ts`                | ready   |   195 |        83 | napi-rs | String offsets and slicing (2); Async and promises (4)                                   |
|    0 | `scripts/canary.ts`                             | ready   |   512 |        83 | napi-rs | String offsets and slicing (4); Exceptions (29)                                          |
|    0 | `scripts/check-windows-runtime-boundary.ts`     | ready   |   255 |        78 | napi-rs | Regex features the `regex` crate lacks (3); String offsets and slicing (1)               |
|    0 | `scripts/check-brand-identity.ts`               | ready   |   334 |        70 | napi-rs | String offsets and slicing (8); Regexes built at runtime (8)                             |
|    0 | `scripts/dev-runner.ts`                         | ready   |   536 |        69 | napi-rs | Generators (25); Insertion-ordered maps and object keys (3)                              |
|    0 | `scripts/i18n.ts`                               | ready   |   899 |        58 | napi-rs | String offsets and slicing (12); Insertion-ordered maps and object keys (33)             |
|    0 | `scripts/lib/`                                  | blocked |  3560 |         0 | napi-rs | String offsets and slicing (12); Regex features the `regex` crate lacks (1)              |
|    1 | `scripts/finalize-mac-dmg.ts`                   | ready   |    13 |       100 | napi-rs | Async and promises (2); Exceptions (1)                                                   |
|    1 | `scripts/install-git-hooks.ts`                  | caution |    78 |       100 | napi-rs | Dynamic typing (1); Exceptions (4)                                                       |
|    1 | `scripts/merge-mac-update-manifests.ts`         | ready   |    31 |       100 | napi-rs | String offsets and slicing (1); Exceptions (1)                                           |
|    1 | `scripts/resolve-release-update-policy.ts`      | ready   |    31 |       100 | napi-rs | Exceptions (1); JSON round-trips (1)                                                     |
|    1 | `scripts/ci-install-key.ts`                     | ready   |    45 |        99 | napi-rs | Insertion-ordered maps and object keys (1); JSON round-trips (3)                         |
|    1 | `scripts/node-pty-smoke.mjs`                    | ready   |    65 |        99 | napi-rs | Async and promises (1); Exceptions (4)                                                   |
|    1 | `scripts/resolve-release-build-scope.ts`        | ready   |    20 |        99 | napi-rs | Insertion-ordered maps and object keys (1); Exceptions (3)                               |
|    1 | `scripts/portable-build.ts`                     | ready   |    20 |        98 | napi-rs | String offsets and slicing (1); JSON round-trips (2)                                     |
|    1 | `scripts/wait-for-release-job.ts`               | ready   |    39 |        98 | napi-rs | Async and promises (2); Exceptions (3)                                                   |
|    1 | `scripts/import-portable-build.ts`              | ready   |    44 |        97 | napi-rs | String offsets and slicing (1); Exceptions (3)                                           |
|    1 | `scripts/verify-release-source-provenance.ts`   | ready   |    89 |        94 | napi-rs | String offsets and slicing (1); Exceptions (11)                                          |
|    1 | `scripts/mac-icon-catalog.ts`                   | ready   |   108 |        92 | napi-rs | String offsets and slicing (2); Exceptions (7)                                           |
|    1 | `scripts/notarize-mac-app.ts`                   | ready   |    81 |        92 | napi-rs | Async and promises (8); JSON round-trips (5)                                             |
|    1 | `scripts/prepare-release-update-feed.ts`        | ready   |    37 |        91 | napi-rs | String offsets and slicing (3); Insertion-ordered maps and object keys (1)               |
|    1 | `scripts/verify-release-artifact-checkpoint.ts` | ready   |    70 |        91 | napi-rs | String offsets and slicing (1); Insertion-ordered maps and object keys (2)               |
|    1 | `scripts/pre-push-check.ts`                     | ready   |   185 |        90 | napi-rs | String offsets and slicing (1); Async and promises (10)                                  |
|    1 | `scripts/write-release-artifact-provenance.ts`  | ready   |    96 |        90 | napi-rs | String offsets and slicing (1); Insertion-ordered maps and object keys (2)               |
|    1 | `scripts/smoke-mac-update-artifact.ts`          | ready   |   292 |        83 | napi-rs | Async and promises (14); Insertion-ordered maps and object keys (1)                      |
|    1 | `scripts/build-desktop-artifact.ts`             | caution |  1502 |        20 | napi-rs | String offsets and slicing (8); Generators (212)                                         |
|    1 | `scripts/release-smoke.ts`                      | blocked |  1289 |         9 | napi-rs | String offsets and slicing (31); Dynamic code (eval, new Function, computed import) (10) |
|    6 | `scripts/check-migration-lineage.ts`            | ready   |   305 |        82 | napi-rs | String offsets and slicing (5); Insertion-ordered maps and object keys (4)               |
|    6 | `scripts/verify-packaged-desktop-startup.ts`    | ready   |   562 |        62 | napi-rs | String offsets and slicing (3); Regex features the `regex` crate lacks (1)               |
|    6 | `scripts/device-helper-smoke.ts`                | ready   |   542 |        58 | napi-rs | String offsets and slicing (5); Async and promises (25)                                  |
|    7 | `scripts/computer-use-fixtures/`                | caution |  9857 |         0 | napi-rs | String offsets and slicing (90); Regex features the `regex` crate lacks (32)             |

## npm dependencies

| Package                          | Units | Rust                  | Fit     | Note                                                                          |
| -------------------------------- | ----: | --------------------- | ------- | ----------------------------------------------------------------------------- |
| `vite`                           |     1 | -                     | none    | Dev server or UI framework: keep in JS.                                       |
| `effect`                         |   285 | ?                     | unknown |                                                                               |
| `zustand`                        |    34 | ?                     | unknown |                                                                               |
| `electron`                       |    17 | ?                     | unknown |                                                                               |
| `@effect/platform-node`          |    16 | ?                     | unknown |                                                                               |
| `jose`                           |    11 | ?                     | unknown |                                                                               |
| `drizzle-orm`                    |     5 | ?                     | unknown |                                                                               |
| `@anthropic-ai/claude-agent-sdk` |     3 | ?                     | unknown |                                                                               |
| `@hono/node-server`              |     3 | ?                     | unknown |                                                                               |
| `betterwright`                   |     3 | ?                     | unknown |                                                                               |
| `next`                           |     3 | ?                     | unknown |                                                                               |
| `vitest`                         |     3 | ?                     | unknown |                                                                               |
| `@agentclientprotocol/sdk`       |     2 | ?                     | unknown |                                                                               |
| `@dnd-kit/sortable`              |     2 | ?                     | unknown |                                                                               |
| `@modelcontextprotocol/sdk`      |     2 | ?                     | unknown |                                                                               |
| `@pierre/diffs`                  |     2 | ?                     | unknown |                                                                               |
| `@xterm/headless`                |     2 | ?                     | unknown |                                                                               |
| `mdast-util-from-markdown`       |     2 | ?                     | unknown |                                                                               |
| `node-pty`                       |     2 | ?                     | unknown |                                                                               |
| `open`                           |     2 | ?                     | unknown |                                                                               |
| `pg`                             |     2 | ?                     | unknown |                                                                               |
| `@cloudflare/containers`         |     1 | ?                     | unknown |                                                                               |
| `@effect/sql-sqlite-bun`         |     1 | ?                     | unknown |                                                                               |
| `@opencode-ai/sdk`               |     1 | ?                     | unknown |                                                                               |
| `@opencode/client`               |     1 | ?                     | unknown |                                                                               |
| `@peculiar/x509`                 |     1 | ?                     | unknown |                                                                               |
| `@playwright/test`               |     1 | ?                     | unknown |                                                                               |
| `@polar-sh/sdk`                  |     1 | ?                     | unknown |                                                                               |
| `@rolldown/plugin-babel`         |     1 | ?                     | unknown |                                                                               |
| `@vitejs/plugin-react`           |     1 | ?                     | unknown |                                                                               |
| `class-variance-authority`       |     1 | ?                     | unknown |                                                                               |
| `electron-updater`               |     1 | ?                     | unknown |                                                                               |
| `i18next`                        |     1 | ?                     | unknown |                                                                               |
| `import-meta-resolve`            |     1 | ?                     | unknown |                                                                               |
| `original-fs`                    |     1 | ?                     | unknown |                                                                               |
| `playwright`                     |     1 | ?                     | unknown |                                                                               |
| `reflect-metadata`               |     1 | ?                     | unknown |                                                                               |
| `resend`                         |     1 | ?                     | unknown |                                                                               |
| `tailwind-merge`                 |     1 | ?                     | unknown |                                                                               |
| `wait-on`                        |     1 | ?                     | unknown |                                                                               |
| `typescript`                     |     4 | oxc                   | partial | Parsing is covered. Type checking has no Rust equivalent.                     |
| `hono`                           |     3 | axum                  | partial | Middleware ports by hand.                                                     |
| `yaml`                           |     1 | saphyr, serde_yaml_ng | partial | Parsing is covered. No crate rewrites YAML while keeping comments and layout. |
| `zod`                            |     1 | serde, garde          | partial | Schemas become types plus validation; error messages will differ.             |
| `ws`                             |    10 | tokio-tungstenite     | good    |                                                                               |
| `smol-toml`                      |     3 | toml                  | good    |                                                                               |
| `undici`                         |     2 | reqwest               | good    |                                                                               |

## Blocked

- `apps/desktop/scripts/`: Uses dynamic code (eval, new Function or computed imports).
- `apps/server/src/provider/`: Uses dynamic code (eval, new Function or computed imports).
- `apps/web/scripts/`: `vite` has no Rust equivalent.
- `scripts/lib/`: Uses dynamic code (eval, new Function or computed imports).
- `scripts/release-smoke.ts`: Uses dynamic code (eval, new Function or computed imports).

## Risk guide

- **String offsets and slicing** (High): JS indexes strings in UTF-16 code units, Rust in UTF-8 bytes. Offsets that cross the boundary must be converted, or non-ASCII text gets wrong ranges silently.
- **Regex features the `regex` crate lacks** (High): Lookaround, backreferences and the `y` flag are not in the `regex` crate. Use `fancy-regex` for those patterns, or rewrite them, and test them on non-ASCII text: `\w`, `\b` and case-insensitive matching differ between engines even where a pattern compiles. JS `/i` without `u` never matches a non-ASCII letter to an ASCII one, Rust `(?i)` does (`ſ` matches `s`, the Kelvin sign `K` matches `k`): write `(?i-u)` or compare with `eq_ignore_ascii_case`.
- **Regexes built at runtime** (High): Patterns come from data or config in JS syntax. Rust regex syntax differs, so user-visible behavior can change. Pin it with tests on real patterns.
- **Dynamic code (eval, new Function, computed import)** (High): eval, new Function and computed imports have no Rust equivalent. Keep this part in JS or replace it with an explicit registry.
- **Dynamic typing** (Medium): Values whose shape is checked at runtime need an explicit Rust enum or `serde_json::Value`.
- **Insertion-ordered maps and object keys** (Medium): JS Map, Set and object keys keep insertion order. `HashMap` does not: use `indexmap` wherever order reaches output. With serde_json's `preserve_order`, `Map::remove` swaps the last key into the hole: use `shift_remove` for JS `delete`. Sort with `js::sort` when a comparator can return NaN: the order then depends on V8's algorithm.
- **Exceptions** (Low): Thrown errors become `Result`. Errors crossing the bridge must map back to the same JS error shape.
- **Async and promises** (Medium): Promises become futures. Bridge with napi-rs `async fn` or `wasm-bindgen-futures`, or keep the async shell in JS and port the sync core.
- **Generators** (Medium): Lazy generators become iterators, or return a collected Vec across the bridge.
- **Class inheritance** (Medium): Class hierarchies become traits plus composition; `instanceof` checks become enums.
- **Reflection and prototypes** (Medium): Proxy, Reflect, Symbol and prototype tricks need a redesign, not a translation.
- **Callback-shaped APIs** (Medium): Function-typed parameters and methods cross the bridge on every call. Keep boundaries coarse: pass data in, get data out.
- **Number semantics** (Low): JS numbers are f64; bitwise ops truncate to i32; parseInt is lenient. Pick Rust integer types deliberately. Printing differs: `${n}` gives `1e+21`, `1e-7` and `0` for -0 where Rust's `{}` does not, so use `js::number_to_string`. `Math.round` rounds halves up (-2.5 gives -2), Rust's `round` away from zero: use `js::math_round`. `toFixed` rounds exact halves up ((2.5).toFixed(0) is "3"), Rust's `{:.0}` to even.
- **JSON round-trips** (Low): JSON.stringify key order and number formatting must match `serde_json` output if the result is user-visible.

## Whole app

Shape: **server**.

| Entry point                                                        | Kind   | Found from                                                                         |
| ------------------------------------------------------------------ | ------ | ---------------------------------------------------------------------------------- |
| `apps/desktop/scripts/betterwright-recovery-smoke.ts`              | server | `createServer(` at apps/desktop/scripts/betterwright-recovery-smoke.ts:14          |
| `apps/desktop/scripts/browser-integration-fixture.mjs`             | server | `createServer(` at apps/desktop/scripts/browser-integration-fixture.mjs:12         |
| `apps/desktop/scripts/browser-lifecycle-smoke.ts`                  | server | `createServer(` at apps/desktop/scripts/browser-lifecycle-smoke.ts:165             |
| `apps/desktop/scripts/build-appsnap-helper.mjs`                    | cli    | `#!/usr/bin/env node` at apps/desktop/scripts/build-appsnap-helper.mjs:1           |
| `apps/desktop/scripts/build-window-material-addon.mjs`             | cli    | `#!/usr/bin/env node` at apps/desktop/scripts/build-window-material-addon.mjs:1    |
| `apps/desktop/scripts/credential-recovery-smoke.ts`                | server | `createServer(` at apps/desktop/scripts/credential-recovery-smoke.ts:21            |
| `apps/desktop/scripts/embedded-popup-smoke.ts`                     | server | `createServer(` at apps/desktop/scripts/embedded-popup-smoke.ts:21                 |
| `apps/desktop/scripts/remote-resource-smoke.ts`                    | server | `createServer(` at apps/desktop/scripts/remote-resource-smoke.ts:38                |
| `apps/desktop/src/browserAutomation/betterwrightConnection.ts`     | server | `createServer(` at apps/desktop/src/browserAutomation/betterwrightConnection.ts:25 |
| `apps/desktop/src/browserUsePipeServer.ts`                         | server | `createServer(` at apps/desktop/src/browserUsePipeServer.ts:283                    |
| `apps/desktop/src/computerFrameTap.ts`                             | server | `createServer(` at apps/desktop/src/computerFrameTap.ts:208                        |
| `apps/desktop/src/cuaDriverHost.ts`                                | server | `createServer(` at apps/desktop/src/cuaDriverHost.ts:557                           |
| `apps/desktop/src/cuaDriverHostStandalone.ts`                      | server | `.listen(` at apps/desktop/src/cuaDriverHostStandalone.ts:106                      |
| `apps/desktop/src/cuaFixtures/electron.ts`                         | server | `.listen(` at apps/desktop/src/cuaFixtures/electron.ts:156                         |
| `apps/desktop/src/cuaFixtures/live.ts`                             | server | `.listen(` at apps/desktop/src/cuaFixtures/live.ts:93                              |
| `apps/desktop/src/main.ts`                                         | server | `.listen(` at apps/desktop/src/main.ts:4195                                        |
| `apps/marketing/scripts/check-external-links.mjs`                  | cli    | `#!/usr/bin/env node` at apps/marketing/scripts/check-external-links.mjs:1         |
| `apps/server/scripts/claude-context-window-probe.mjs`              | server | `createServer(` at apps/server/scripts/claude-context-window-probe.mjs:19          |
| `apps/server/scripts/cli.ts`                                       | cli    | `#!/usr/bin/env node` at apps/server/scripts/cli.ts:1                              |
| `apps/server/src/accountSsoCallback.ts`                            | server | `createServer(` at apps/server/src/accountSsoCallback.ts:103                       |
| `apps/server/src/cloudflare/connector.ts`                          | server | `createServer(` at apps/server/src/cloudflare/connector.ts:6                       |
| `apps/server/src/cloudflare/ingress.ts`                            | server | `createServer(` at apps/server/src/cloudflare/ingress.ts:22                        |
| `apps/server/src/device/helperClient.ts`                           | server | `createServer(` at apps/server/src/device/helperClient.ts:380                      |
| `apps/server/src/editor/test-fixtures/noisyDebugAdapter.mjs`       | server | `createServer(` at apps/server/src/editor/test-fixtures/noisyDebugAdapter.mjs:10   |
| `apps/server/src/effectServer.ts`                                  | server | `createServer(` at apps/server/src/effectServer.ts:233                             |
| `apps/server/src/hostConnectivity.ts`                              | server | `createServer(` at apps/server/src/hostConnectivity.ts:472                         |
| `apps/server/src/nodeHttpServer.ts`                                | server | `.listen(` at apps/server/src/nodeHttpServer.ts:197                                |
| `apps/server/src/remoteTransport/tunnel.ts`                        | server | `createServer(` at apps/server/src/remoteTransport/tunnel.ts:92                    |
| `apps/web/perf/concurrent-runner.mjs`                              | server | `createServer(` at apps/web/perf/concurrent-runner.mjs:10                          |
| `apps/web/scripts/hugeicon-snippet.mjs`                            | cli    | `#!/usr/bin/env node` at apps/web/scripts/hugeicon-snippet.mjs:1                   |
| `apps/web/scripts/measure-dev-transforms.mjs`                      | server | `createServer(` at apps/web/scripts/measure-dev-transforms.mjs:26                  |
| `benchmarks/desktop-perf/diagnose-backend-boot.mjs`                | cli    | `#!/usr/bin/env node` at benchmarks/desktop-perf/diagnose-backend-boot.mjs:1       |
| `benchmarks/desktop-perf/diagnose.mjs`                             | cli    | `#!/usr/bin/env node` at benchmarks/desktop-perf/diagnose.mjs:1                    |
| `benchmarks/desktop-perf/lib/launch.mjs`                           | server | `createServer(` at benchmarks/desktop-perf/lib/launch.mjs:54                       |
| `benchmarks/desktop-perf/longchat-report.mjs`                      | cli    | `#!/usr/bin/env node` at benchmarks/desktop-perf/longchat-report.mjs:1             |
| `benchmarks/desktop-perf/longchat.mjs`                             | cli    | `#!/usr/bin/env node` at benchmarks/desktop-perf/longchat.mjs:1                    |
| `benchmarks/desktop-perf/prepare.mjs`                              | cli    | `#!/usr/bin/env node` at benchmarks/desktop-perf/prepare.mjs:1                     |
| `benchmarks/desktop-perf/report.mjs`                               | cli    | `#!/usr/bin/env node` at benchmarks/desktop-perf/report.mjs:1                      |
| `benchmarks/desktop-perf/run.mjs`                                  | cli    | `#!/usr/bin/env node` at benchmarks/desktop-perf/run.mjs:1                         |
| `docs/computer-use-cua/evidence/latency-rev17-probe.ts`            | server | `.listen(` at docs/computer-use-cua/evidence/latency-rev17-probe.ts:137            |
| `docs/computer-use-cua/evidence/latency-rev17-supplement-probe.ts` | server | `.listen(` at docs/computer-use-cua/evidence/latency-rev17-supplement-probe.ts:93  |
| `packages/shared/src/Net.ts`                                       | server | `createServer(` at packages/shared/src/Net.ts:31                                   |
| `scripts/build-desktop-artifact.ts`                                | cli    | `#!/usr/bin/env node` at scripts/build-desktop-artifact.ts:1                       |
| `scripts/computer-use-fixtures/belief-canary.mjs`                  | cli    | `#!/usr/bin/env node` at scripts/computer-use-fixtures/belief-canary.mjs:1         |
| `scripts/computer-use-fixtures/canary-main.ts`                     | server | `.listen(` at scripts/computer-use-fixtures/canary-main.ts:204                     |
| `scripts/computer-use-fixtures/live-cert.ts`                       | server | `.listen(` at scripts/computer-use-fixtures/live-cert.ts:518                       |
| `scripts/computer-use-fixtures/multi-display-cert.ts`              | server | `.listen(` at scripts/computer-use-fixtures/multi-display-cert.ts:588              |
| `scripts/computer-use-fixtures/packaged-e2e.ts`                    | server | `createServer(` at scripts/computer-use-fixtures/packaged-e2e.ts:108               |
| `scripts/dev-runner.ts`                                            | cli    | `#!/usr/bin/env node` at scripts/dev-runner.ts:1                                   |
| `scripts/device-helper-smoke.ts`                                   | server | `createServer(` at scripts/device-helper-smoke.ts:225                              |
| `scripts/node-pty-smoke.mjs`                                       | cli    | `#!/usr/bin/env node` at scripts/node-pty-smoke.mjs:1                              |
| `scripts/smoke-mac-update-artifact.ts`                             | server | `createServer(` at scripts/smoke-mac-update-artifact.ts:157                        |
| `scripts/verify-packaged-desktop-startup.ts`                       | cli    | `#!/usr/bin/env node` at scripts/verify-packaged-desktop-startup.ts:1              |
| `scripts/verify-release-artifact-checkpoint.ts`                    | cli    | `#!/usr/bin/env node` at scripts/verify-release-artifact-checkpoint.ts:1           |
| `scripts/write-release-artifact-provenance.ts`                     | cli    | `#!/usr/bin/env node` at scripts/write-release-artifact-provenance.ts:1            |

### Every dependency

| Dependency                            | Declared | Used by | Rust                  | Fit     | Note                                                                          |
| ------------------------------------- | -------- | ------- | --------------------- | ------- | ----------------------------------------------------------------------------- |
| `vite`                                | no       | 1       | -                     | none    | Dev server or UI framework: keep in JS.                                       |
| `effect`                              | no       | 300     | ?                     | unknown |                                                                               |
| `zustand`                             | no       | 49      | ?                     | unknown |                                                                               |
| `vitest`                              | no       | 27      | ?                     | unknown |                                                                               |
| `@tanstack/react-query`               | no       | 26      | ?                     | unknown |                                                                               |
| `vitest-browser-react`                | no       | 22      | ?                     | unknown |                                                                               |
| `@tanstack/react-router`              | no       | 19      | ?                     | unknown |                                                                               |
| `electron`                            | no       | 17      | ?                     | unknown |                                                                               |
| `@effect/platform-node`               | no       | 16      | ?                     | unknown |                                                                               |
| `next`                                | no       | 16      | ?                     | unknown |                                                                               |
| `jose`                                | no       | 11      | ?                     | unknown |                                                                               |
| `react-dom`                           | no       | 11      | ?                     | unknown |                                                                               |
| `react-icons`                         | no       | 10      | ?                     | unknown |                                                                               |
| `@dnd-kit/sortable`                   | no       | 6       | ?                     | unknown |                                                                               |
| `@pierre/diffs`                       | no       | 6       | ?                     | unknown |                                                                               |
| `@tanstack/react-pacer`               | no       | 6       | ?                     | unknown |                                                                               |
| `class-variance-authority`            | no       | 5       | ?                     | unknown |                                                                               |
| `drizzle-orm`                         | no       | 5       | ?                     | unknown |                                                                               |
| `@base-ui/react`                      | no       | 4       | ?                     | unknown |                                                                               |
| `@dnd-kit/core`                       | no       | 4       | ?                     | unknown |                                                                               |
| `@dnd-kit/utilities`                  | no       | 4       | ?                     | unknown |                                                                               |
| `fumadocs-ui`                         | no       | 4       | ?                     | unknown |                                                                               |
| `@anthropic-ai/claude-agent-sdk`      | no       | 3       | ?                     | unknown |                                                                               |
| `@dnd-kit/modifiers`                  | no       | 3       | ?                     | unknown |                                                                               |
| `@hono/node-server`                   | no       | 3       | ?                     | unknown |                                                                               |
| `betterwright`                        | no       | 3       | ?                     | unknown |                                                                               |
| `tailwind-merge`                      | no       | 3       | ?                     | unknown |                                                                               |
| `@agentclientprotocol/sdk`            | no       | 2       | ?                     | unknown |                                                                               |
| `@legendapp/list`                     | no       | 2       | ?                     | unknown |                                                                               |
| `@modelcontextprotocol/sdk`           | no       | 2       | ?                     | unknown |                                                                               |
| `@xterm/headless`                     | no       | 2       | ?                     | unknown |                                                                               |
| `i18next`                             | no       | 2       | ?                     | unknown |                                                                               |
| `lexical`                             | no       | 2       | ?                     | unknown |                                                                               |
| `mdast-util-from-markdown`            | no       | 2       | ?                     | unknown |                                                                               |
| `msw`                                 | no       | 2       | ?                     | unknown |                                                                               |
| `node-pty`                            | no       | 2       | ?                     | unknown |                                                                               |
| `open`                                | no       | 2       | ?                     | unknown |                                                                               |
| `pg`                                  | no       | 2       | ?                     | unknown |                                                                               |
| `playwright`                          | no       | 2       | ?                     | unknown |                                                                               |
| `@cloudflare/containers`              | no       | 1       | ?                     | unknown |                                                                               |
| `@effect/sql-sqlite-bun`              | no       | 1       | ?                     | unknown |                                                                               |
| `@fontsource-variable/jetbrains-mono` | no       | 1       | ?                     | unknown |                                                                               |
| `@formkit/auto-animate`               | no       | 1       | ?                     | unknown |                                                                               |
| `@lexical/react`                      | no       | 1       | ?                     | unknown |                                                                               |
| `@opencode-ai/sdk`                    | no       | 1       | ?                     | unknown |                                                                               |
| `@opencode/client`                    | no       | 1       | ?                     | unknown |                                                                               |
| `@opennextjs/cloudflare`              | no       | 1       | ?                     | unknown |                                                                               |
| `@peculiar/x509`                      | no       | 1       | ?                     | unknown |                                                                               |
| `@playwright/test`                    | no       | 1       | ?                     | unknown |                                                                               |
| `@polar-sh/sdk`                       | no       | 1       | ?                     | unknown |                                                                               |
| `@rolldown/plugin-babel`              | no       | 1       | ?                     | unknown |                                                                               |
| `@tabler/icons-react`                 | no       | 1       | ?                     | unknown |                                                                               |
| `@tanstack/react-virtual`             | no       | 1       | ?                     | unknown |                                                                               |
| `@vitejs/plugin-react`                | no       | 1       | ?                     | unknown |                                                                               |
| `@xterm/addon-clipboard`              | no       | 1       | ?                     | unknown |                                                                               |
| `@xterm/addon-fit`                    | no       | 1       | ?                     | unknown |                                                                               |
| `@xterm/addon-image`                  | no       | 1       | ?                     | unknown |                                                                               |
| `@xterm/addon-ligatures`              | no       | 1       | ?                     | unknown |                                                                               |
| `@xterm/addon-search`                 | no       | 1       | ?                     | unknown |                                                                               |
| `@xterm/addon-unicode11`              | no       | 1       | ?                     | unknown |                                                                               |
| `@xterm/addon-webgl`                  | no       | 1       | ?                     | unknown |                                                                               |
| `@xterm/xterm`                        | no       | 1       | ?                     | unknown |                                                                               |
| `clsx`                                | no       | 1       | ?                     | unknown |                                                                               |
| `echarts`                             | no       | 1       | ?                     | unknown |                                                                               |
| `electron-updater`                    | no       | 1       | ?                     | unknown |                                                                               |
| `html-to-image`                       | no       | 1       | ?                     | unknown |                                                                               |
| `import-meta-resolve`                 | no       | 1       | ?                     | unknown |                                                                               |
| `katex`                               | no       | 1       | ?                     | unknown |                                                                               |
| `lucide-react`                        | no       | 1       | ?                     | unknown |                                                                               |
| `micromark-util-decode-string`        | no       | 1       | ?                     | unknown |                                                                               |
| `monaco-editor`                       | no       | 1       | ?                     | unknown |                                                                               |
| `original-fs`                         | no       | 1       | ?                     | unknown |                                                                               |
| `pdfjs-dist`                          | no       | 1       | ?                     | unknown |                                                                               |
| `react-colorful`                      | no       | 1       | ?                     | unknown |                                                                               |
| `react-i18next`                       | no       | 1       | ?                     | unknown |                                                                               |
| `react-markdown`                      | no       | 1       | ?                     | unknown |                                                                               |
| `react-tweet`                         | no       | 1       | ?                     | unknown |                                                                               |
| `reflect-metadata`                    | no       | 1       | ?                     | unknown |                                                                               |
| `rehype-katex`                        | no       | 1       | ?                     | unknown |                                                                               |
| `rehype-raw`                          | no       | 1       | ?                     | unknown |                                                                               |
| `rehype-sanitize`                     | no       | 1       | ?                     | unknown |                                                                               |
| `remark-breaks`                       | no       | 1       | ?                     | unknown |                                                                               |
| `remark-gfm`                          | no       | 1       | ?                     | unknown |                                                                               |
| `remark-math`                         | no       | 1       | ?                     | unknown |                                                                               |
| `resend`                              | no       | 1       | ?                     | unknown |                                                                               |
| `server-only`                         | no       | 1       | ?                     | unknown |                                                                               |
| `uqr`                                 | no       | 1       | ?                     | unknown |                                                                               |
| `wait-on`                             | no       | 1       | ?                     | unknown |                                                                               |
| `typescript`                          | no       | 4       | oxc                   | partial | Parsing is covered. Type checking has no Rust equivalent.                     |
| `hono`                                | no       | 3       | axum                  | partial | Middleware ports by hand.                                                     |
| `yaml`                                | no       | 1       | saphyr, serde_yaml_ng | partial | Parsing is covered. No crate rewrites YAML while keeping comments and layout. |
| `zod`                                 | no       | 1       | serde, garde          | partial | Schemas become types plus validation; error messages will differ.             |
| `ws`                                  | no       | 10      | tokio-tungstenite     | good    |                                                                               |
| `smol-toml`                           | no       | 3       | toml                  | good    |                                                                               |
| `undici`                              | no       | 2       | reqwest               | good    |                                                                               |

### Pieces, in port order

- wave 0: `apps/analytics/diagnosticsPrivacySql.ts` (96 lines)
- wave 0: `apps/analytics/forwarder/` (28 lines)
- wave 0: `apps/analytics/installerDownloads.ts` (277 lines)
- wave 0: `apps/analytics/reliability.ts` (90 lines)
- wave 0: `apps/analytics/src/api.ts` (329 lines)
- wave 0: `apps/analytics/src/debounce.ts` (16 lines)
- wave 0: `apps/analytics/testing/` (47 lines)
- wave 0: `apps/analytics/usagePrivacySql.ts` (28 lines)
- wave 0: `apps/api/cloudflare/` (344 lines)
- wave 0: `apps/api/src/clientIp.ts` (103 lines)
- wave 0: `apps/api/src/db/` (640 lines)
- wave 0: `apps/api/src/perf.ts` (335 lines)
- wave 0: `apps/api/src/rateLimit.ts` (85 lines)
- wave 0: `apps/desktop/src/agentCursorPreference.ts` (77 lines)
- wave 0: `apps/desktop/src/audioLevelMonitor.ts` (259 lines)
- wave 0: `apps/desktop/src/backendCompileCache.ts` (54 lines)
- wave 0: `apps/desktop/src/backendNodeOptions.ts` (59 lines)
- wave 0: `apps/desktop/src/backendProcessOutput.ts` (64 lines)
- wave 0: `apps/desktop/src/backendReadiness.ts` (98 lines)
- wave 0: `apps/desktop/src/backendReliability.ts` (24 lines)
- wave 0: `apps/desktop/src/backendShutdown.ts` (473 lines)
- wave 0: `apps/desktop/src/backendStartupBlock.ts` (134 lines)
- wave 0: `apps/desktop/src/backendSupervisionPolicy.ts` (169 lines)
- wave 0: `apps/desktop/src/betaDiagnostics.ts` (1027 lines)
- wave 0: `apps/desktop/src/betaInstaller.ts` (469 lines)
- wave 0: `apps/desktop/src/bundleSwapDetection.ts` (62 lines)
- wave 0: `apps/desktop/src/clipboardPermissions.ts` (54 lines)
- wave 0: `apps/desktop/src/computerDesktopLifecycle.ts` (37 lines)
- wave 0: `apps/desktop/src/computerEmergencyStopNotice.ts` (88 lines)
- wave 0: `apps/desktop/src/contextMenuTemplate.ts` (63 lines)
- wave 0: `apps/desktop/src/cuaHostSocket.ts` (47 lines)
- wave 0: `apps/desktop/src/cuaRuntimeOwnership.ts` (139 lines)
- wave 0: `apps/desktop/src/desktopAppIcon.ts` (106 lines)
- wave 0: `apps/desktop/src/desktopCustomTitleBar.ts` (72 lines)
- wave 0: `apps/desktop/src/desktopMigrationRecovery.ts` (275 lines)
- wave 0: `apps/desktop/src/desktopProcessErrors.ts` (13 lines)
- wave 0: `apps/desktop/src/desktopQuitIntent.ts` (70 lines)
- wave 0: `apps/desktop/src/desktopStaticProtocol.ts` (53 lines)
- wave 0: `apps/desktop/src/desktopStorageMigration.ts` (70 lines)
- wave 0: `apps/desktop/src/desktopUpdateControl.ts` (167 lines)
- wave 0: `apps/desktop/src/desktopUserDataProfile.ts` (233 lines)
- wave 0: `apps/desktop/src/electronUpdaterSecurity.ts` (287 lines)
- wave 0: `apps/desktop/src/exclusiveApplyQueue.ts` (22 lines)
- wave 0: `apps/desktop/src/githubUpdateFeed.ts` (39 lines)
- wave 0: `apps/desktop/src/hostUsageRelay.ts` (139 lines)
- wave 0: `apps/desktop/src/httpRuntime.ts` (7 lines)
- wave 0: `apps/desktop/src/i18n.ts` (21 lines)
- wave 0: `apps/desktop/src/initialBackendWindowOpen.ts` (52 lines)
- wave 0: `apps/desktop/src/ipcChannels.ts` (161 lines)
- wave 0: `apps/desktop/src/keepAwake.ts` (133 lines)
- wave 0: `apps/desktop/src/linuxAppIcon.ts` (102 lines)
- wave 0: `apps/desktop/src/linuxCuaAdmission.ts` (150 lines)
- wave 0: `apps/desktop/src/localHtmlPreviewProtocol.ts` (167 lines)
- wave 0: `apps/desktop/src/macIconCacheRefresh.ts` (53 lines)
- wave 0: `apps/desktop/src/macUpdateDiagnostics.ts` (130 lines)
- wave 0: `apps/desktop/src/macUpdateInstallHandoff.ts` (67 lines)
- wave 0: `apps/desktop/src/mediaPermissions.ts` (67 lines)
- wave 0: `apps/desktop/src/menuShortcuts.ts` (189 lines)
- wave 0: `apps/desktop/src/migrationBundleIdentity.ts` (35 lines)
- wave 0: `apps/desktop/src/migrationConsentHandoff.ts` (11 lines)
- wave 0: `apps/desktop/src/notificationRetention.ts` (46 lines)
- wave 0: `apps/desktop/src/reliabilityReporter.ts` (263 lines)
- wave 0: `apps/desktop/src/rendererCrashRecovery.ts` (93 lines)
- wave 0: `apps/desktop/src/resumableUpdateDownloadPolicy.ts` (178 lines)
- wave 0: `apps/desktop/src/runningChatsQuitGuard.ts` (237 lines)
- wave 0: `apps/desktop/src/runtimeArch.ts` (32 lines)
- wave 0: `apps/desktop/src/serverListeningDetector.ts` (52 lines)
- wave 0: `apps/desktop/src/showInFileManager.ts` (26 lines)
- wave 0: `apps/desktop/src/shutdownSignals.ts` (24 lines)
- wave 0: `apps/desktop/src/stopNativeHelper.ts` (35 lines)
- wave 0: `apps/desktop/src/subframeNavigationGuard.ts` (41 lines)
- wave 0: `apps/desktop/src/syncShellEnvironment.ts` (123 lines)
- wave 0: `apps/desktop/src/updateArtifactIdentity.ts` (91 lines)
- wave 0: `apps/desktop/src/updateInstallPreparation.ts` (41 lines)
- wave 0: `apps/desktop/src/updatePendingCache.ts` (74 lines)
- wave 0: `apps/desktop/src/updateState.ts` (278 lines)
- wave 0: `apps/desktop/src/windowMaterial.ts` (93 lines)
- wave 0: `apps/desktop/src/windowState.ts` (116 lines)
- wave 0: `apps/desktop/src/windowsShellAppUserModel.ts` (304 lines)
- wave 0: `apps/desktop/src/windowsShellIco.ts` (132 lines)
- wave 0: `apps/desktop/src/windowsTaskbarIcon.ts` (217 lines)
- wave 0: `apps/marketing/scripts/` (1206 lines)
- wave 0: `apps/marketing/src/app/api/inbound-email/` (74 lines)
- wave 0: `apps/marketing/src/data/` (5448 lines)
- wave 0: `apps/profiles/src/legacy-redirect.ts` (13 lines)
- wave 0: `apps/profiles/src/middleware.ts` (31 lines)
- wave 0: `apps/server/perf/` (166 lines)
- wave 0: `apps/server/src/accountHttp.ts` (56 lines)
- wave 0: `apps/server/src/accountRefusal.ts` (31 lines)
- wave 0: `apps/server/src/accountSsoCallback.ts` (160 lines)
- wave 0: `apps/server/src/accountUsageReporterRegistry.ts` (25 lines)
- wave 0: `apps/server/src/attachmentPaths.ts` (32 lines)
- wave 0: `apps/server/src/auth/Services/` (362 lines)
- wave 0: `apps/server/src/automation/` (696 lines)
- wave 0: `apps/server/src/backendCompileCache.ts` (55 lines)
- wave 0: `apps/server/src/backoff.ts` (23 lines)
- wave 0: `apps/server/src/betaFeatureGate.ts` (12 lines)
- wave 0: `apps/server/src/browserAutomation/` (330 lines)
- wave 0: `apps/server/src/bunWebSocketCompatibility.ts` (108 lines)
- wave 0: `apps/server/src/claudeTokenStats.ts` (371 lines)
- wave 0: `apps/server/src/codexAppServerTransport.ts` (92 lines)
- wave 0: `apps/server/src/codexAuthIo.ts` (100 lines)
- wave 0: `apps/server/src/codexErrorClassification.ts` (14 lines)
- wave 0: `apps/server/src/codexServiceTier.ts` (11 lines)
- wave 0: `apps/server/src/codexTurnInput.ts` (42 lines)
- wave 0: `apps/server/src/codexWorkingDirectory.ts` (24 lines)
- wave 0: `apps/server/src/composerPrediction/Services/` (29 lines)
- wave 0: `apps/server/src/desktopParentLifetime.ts` (62 lines)
- wave 0: `apps/server/src/desktopUpdateBridge.ts` (78 lines)
- wave 0: `apps/server/src/editor/test-fixtures/` (72 lines)
- wave 0: `apps/server/src/editorAppDiscovery.ts` (226 lines)
- wave 0: `apps/server/src/endpointReporter.ts` (66 lines)
- wave 0: `apps/server/src/environment/` (63 lines)
- wave 0: `apps/server/src/environment/Services/` (9 lines)
- wave 0: `apps/server/src/eventLoopDeadline.ts` (32 lines)
- wave 0: `apps/server/src/executableLookup.ts` (2 lines)
- wave 0: `apps/server/src/frameSocketByteStream.ts` (42 lines)
- wave 0: `apps/server/src/gitHandoffOperations.ts` (176 lines)
- wave 0: `apps/server/src/groupWorkspaceScaffold.ts` (111 lines)
- wave 0: `apps/server/src/hostConnectivitySignals.ts` (40 lines)
- wave 0: `apps/server/src/imageMime.ts` (63 lines)
- wave 0: `apps/server/src/jpegHeader.ts` (65 lines)
- wave 0: `apps/server/src/jwtClaims.ts` (14 lines)
- wave 0: `apps/server/src/lazyModule.ts` (22 lines)
- wave 0: `apps/server/src/logger.ts` (90 lines)
- wave 0: `apps/server/src/managedAttachmentPrincipal.ts` (51 lines)
- wave 0: `apps/server/src/migrationBundleIdentity.ts` (91 lines)
- wave 0: `apps/server/src/nodeHttpServer.ts` (280 lines)
- wave 0: `apps/server/src/os-jank.ts` (76 lines)
- wave 0: `apps/server/src/pngHeader.ts` (34 lines)
- wave 0: `apps/server/src/privatePathPermissions.ts` (177 lines)
- wave 0: `apps/server/src/processArgumentRedaction.ts` (347 lines)
- wave 0: `apps/server/src/project/Services/` (8 lines)
- wave 0: `apps/server/src/providerAccountHomePath.ts` (19 lines)
- wave 0: `apps/server/src/providerChildEnvironment.ts` (133 lines)
- wave 0: `apps/server/src/pullRequestAutoFix/Services/` (29 lines)
- wave 0: `apps/server/src/pullRequests/Services/` (31 lines)
- wave 0: `apps/server/src/realpathNearestExisting.ts` (53 lines)
- wave 0: `apps/server/src/relaySocket.ts` (143 lines)
- wave 0: `apps/server/src/reliability.ts` (23 lines)
- wave 0: `apps/server/src/remoteFeaturePolicy.ts` (59 lines)
- wave 0: `apps/server/src/server/` (66 lines)
- wave 0: `apps/server/src/serverLifecycleEvents.ts` (87 lines)
- wave 0: `apps/server/src/serverRuntimeStartup.ts` (57 lines)
- wave 0: `apps/server/src/serverStatusCli.ts` (165 lines)
- wave 0: `apps/server/src/shell-command-detection.ts` (14 lines)
- wave 0: `apps/server/src/siteFaviconCache.ts` (161 lines)
- wave 0: `apps/server/src/startupAccess.ts` (20 lines)
- wave 0: `apps/server/src/staticAssets.ts` (142 lines)
- wave 0: `apps/server/src/stream/` (63 lines)
- wave 0: `apps/server/src/studioOutputs.ts` (379 lines)
- wave 0: `apps/server/src/studioWorkspaceScaffold.ts` (63 lines)
- wave 0: `apps/server/src/tailscaleEndpoint.ts` (67 lines)
- wave 0: `apps/server/src/todo/` (8 lines)
- wave 0: `apps/server/src/transcriptUsageParsers.ts` (290 lines)
- wave 0: `apps/server/src/voiceTranscription.ts` (151 lines)
- wave 0: `apps/server/src/voiceUploadAdmission.ts` (45 lines)
- wave 0: `apps/server/src/workspace/Services/` (195 lines)
- wave 0: `apps/server/src/wsCompatibility.ts` (169 lines)
- wave 0: `apps/server/src/wsStreamAdmission.ts` (248 lines)
- wave 0: `apps/server/src/wsStreamBackpressure.ts` (413 lines)
- wave 0: `apps/server/steer-probe2.mjs` (104 lines)
- wave 0: `apps/web/public/` (297 lines)
- wave 0: `apps/web/src/appSettingsStorage.ts` (2 lines)
- wave 0: `apps/web/src/betaFeatures.ts` (77 lines)
- wave 0: `apps/web/src/branding.ts` (20 lines)
- wave 0: `apps/web/src/chat-scroll.ts` (22 lines)
- wave 0: `apps/web/src/chatRouteRestore.ts` (64 lines)
- wave 0: `apps/web/src/composerFocusRequestStore.ts` (25 lines)
- wave 0: `apps/web/src/composerModelSelectionCommits.ts` (20 lines)
- wave 0: `apps/web/src/composerTriggerInsertion.ts` (34 lines)
- wave 0: `apps/web/src/computerStateStore.ts` (136 lines)
- wave 0: `apps/web/src/createProjectDialogStore.ts` (13 lines)
- wave 0: `apps/web/src/cursorModelVariants.ts` (210 lines)
- wave 0: `apps/web/src/deviceStateStore.ts` (90 lines)
- wave 0: `apps/web/src/diffRenderMode.ts` (7 lines)
- wave 0: `apps/web/src/diffRouteSearch.ts` (70 lines)
- wave 0: `apps/web/src/dockExplorerBrowseStore.ts` (84 lines)
- wave 0: `apps/web/src/env.ts` (8 lines)
- wave 0: `apps/web/src/explorerRevealRequestStore.ts` (45 lines)
- wave 0: `apps/web/src/file-icons.ts` (265 lines)
- wave 0: `apps/web/src/orchestrationEventCoalescing.ts` (70 lines)
- wave 0: `apps/web/src/pendingTurnDispatch.ts` (43 lines)
- wave 0: `apps/web/src/pendingUserInput.ts` (175 lines)
- wave 0: `apps/web/src/persistedRecord.ts` (34 lines)
- wave 0: `apps/web/src/pinning.logic.ts` (158 lines)
- wave 0: `apps/web/src/projectRunStore.ts` (53 lines)
- wave 0: `apps/web/src/projectScripts.ts` (91 lines)
- wave 0: `apps/web/src/proposedPlan.ts` (100 lines)
- wave 0: `apps/web/src/providerOrdering.ts` (59 lines)
- wave 0: `apps/web/src/pullRequestReference.ts` (18 lines)
- wave 0: `apps/web/src/routeRestoreRefreshCoordinator.ts` (96 lines)
- wave 0: `apps/web/src/rowHoverReveal.ts` (48 lines)
- wave 0: `apps/web/src/sidebarNavOrdering.ts` (36 lines)
- wave 0: `apps/web/src/storageOriginMigration.ts` (62 lines)
- wave 0: `apps/web/src/surfaceStyles.ts` (64 lines)
- wave 0: `apps/web/src/temporaryThreadStore.ts` (35 lines)
- wave 0: `apps/web/src/terminalActivity.ts` (24 lines)
- wave 0: `apps/web/src/terminalVisualIdentity.ts` (98 lines)
- wave 0: `apps/web/src/threadDetailCatchupPolicy.ts` (51 lines)
- wave 0: `apps/web/src/threadDetailResumeCursors.ts` (79 lines)
- wave 0: `apps/web/src/threadDetailSyncRetry.ts` (16 lines)
- wave 0: `apps/web/src/threadSelectionStore.ts` (111 lines)
- wave 0: `apps/web/src/touchTargetStyles.ts` (13 lines)
- wave 0: `apps/web/src/truncateTitle.ts` (7 lines)
- wave 0: `apps/web/src/wsTransportEvents.ts` (123 lines)
- wave 0: `benchmarks/desktop-perf/lib/` (2164 lines)
- wave 0: `packages/contracts/src/baseSchemas.ts` (84 lines)
- wave 0: `packages/contracts/src/browserAutomationBounds.ts` (19 lines)
- wave 0: `packages/contracts/src/browserAutomationToolCatalogue.ts` (16 lines)
- wave 0: `packages/contracts/src/browserAutomationToolCommon.ts` (14 lines)
- wave 0: `packages/contracts/src/composerSlashCommands.ts` (23 lines)
- wave 0: `packages/contracts/src/computerBrowser.ts` (106 lines)
- wave 0: `packages/contracts/src/computerSpaces.ts` (52 lines)
- wave 0: `packages/contracts/src/hostUsage.ts` (47 lines)
- wave 0: `packages/contracts/src/inlineVisualSpec.ts` (242 lines)
- wave 0: `packages/contracts/src/reliability.ts` (71 lines)
- wave 0: `packages/contracts/src/uiLocale.ts` (3 lines)
- wave 0: `packages/profile-ui/src/cn.ts` (9 lines)
- wave 0: `packages/shared/src/DrainableWorker.ts` (268 lines)
- wave 0: `packages/shared/src/KeyedSingleFlightCache.ts` (235 lines)
- wave 0: `packages/shared/src/Net.ts` (160 lines)
- wave 0: `packages/shared/src/Struct.ts` (23 lines)
- wave 0: `packages/shared/src/account.ts` (961 lines)
- wave 0: `packages/shared/src/agentMentions.ts` (134 lines)
- wave 0: `packages/shared/src/appSnapShortcut.ts` (164 lines)
- wave 0: `packages/shared/src/approvalSessionGrant.ts` (30 lines)
- wave 0: `packages/shared/src/asyncUserInput.ts` (45 lines)
- wave 0: `packages/shared/src/automationCompletionPolicy.ts` (26 lines)
- wave 0: `packages/shared/src/automationMode.ts` (48 lines)
- wave 0: `packages/shared/src/automationRunLifecycle.ts` (97 lines)
- wave 0: `packages/shared/src/backgroundTasks.ts` (84 lines)
- wave 0: `packages/shared/src/betaChannel.ts` (59 lines)
- wave 0: `packages/shared/src/betaFeatures.ts` (109 lines)
- wave 0: `packages/shared/src/binaryTransfer.ts` (5 lines)
- wave 0: `packages/shared/src/browserAnnotations.ts` (348 lines)
- wave 0: `packages/shared/src/browserAutomationErrors.ts` (57 lines)
- wave 0: `packages/shared/src/browserAutomationPresentation.ts` (16 lines)
- wave 0: `packages/shared/src/browserSession.ts` (284 lines)
- wave 0: `packages/shared/src/browserShortcuts.ts` (57 lines)
- wave 0: `packages/shared/src/byteAccumulator.ts` (100 lines)
- wave 0: `packages/shared/src/chatThreads.ts` (161 lines)
- wave 0: `packages/shared/src/claudeCache.ts` (62 lines)
- wave 0: `packages/shared/src/cli.ts` (75 lines)
- wave 0: `packages/shared/src/codexConfig.ts` (79 lines)
- wave 0: `packages/shared/src/composerSlashCommands.ts` (17 lines)
- wave 0: `packages/shared/src/computerGrants.ts` (170 lines)
- wave 0: `packages/shared/src/computerInvocation.ts` (22 lines)
- wave 0: `packages/shared/src/computerKeyNames.ts` (102 lines)
- wave 0: `packages/shared/src/conversationEdit.ts` (136 lines)
- wave 0: `packages/shared/src/coordinatorCheckin.ts` (106 lines)
- wave 0: `packages/shared/src/cuaActionDiagnostics.ts` (83 lines)
- wave 0: `packages/shared/src/cuaDriverProtocol.ts` (489 lines)
- wave 0: `packages/shared/src/desktopChrome.ts` (105 lines)
- wave 0: `packages/shared/src/desktopIdentity.ts` (228 lines)
- wave 0: `packages/shared/src/desktopOperationQueue.ts` (249 lines)
- wave 0: `packages/shared/src/desktopTitleBar.ts` (35 lines)
- wave 0: `packages/shared/src/deviceHelperCache.ts` (110 lines)
- wave 0: `packages/shared/src/deviceKey.ts` (273 lines)
- wave 0: `packages/shared/src/diagnosticIssue.ts` (20 lines)
- wave 0: `packages/shared/src/diagnosticsRedaction.ts` (416 lines)
- wave 0: `packages/shared/src/editorIcons.ts` (5 lines)
- wave 0: `packages/shared/src/errorMessages.ts` (58 lines)
- wave 0: `packages/shared/src/executable.ts` (299 lines)
- wave 0: `packages/shared/src/filesystemPlatform.ts` (66 lines)
- wave 0: `packages/shared/src/formatBytes.ts` (23 lines)
- wave 0: `packages/shared/src/frameTransport.ts` (458 lines)
- wave 0: `packages/shared/src/git.ts` (163 lines)
- wave 0: `packages/shared/src/githubAvatar.ts` (13 lines)
- wave 0: `packages/shared/src/groupSlug.ts` (13 lines)
- wave 0: `packages/shared/src/groupThreadState.ts` (267 lines)
- wave 0: `packages/shared/src/hostSecrets.ts` (618 lines)
- wave 0: `packages/shared/src/inboxDay.ts` (83 lines)
- wave 0: `packages/shared/src/inlineVisual.ts` (233 lines)
- wave 0: `packages/shared/src/inlineVisualFormula.ts` (192 lines)
- wave 0: `packages/shared/src/installerDownloads.ts` (31 lines)
- wave 0: `packages/shared/src/kanban.ts` (256 lines)
- wave 0: `packages/shared/src/keybindingRules.ts` (293 lines)
- wave 0: `packages/shared/src/localPreviewFiles.ts` (80 lines)
- wave 0: `packages/shared/src/logging.ts` (150 lines)
- wave 0: `packages/shared/src/markdownPlainText.ts` (357 lines)
- wave 0: `packages/shared/src/mcpConnectors.ts` (317 lines)
- wave 0: `packages/shared/src/migrationRecovery.ts` (271 lines)
- wave 0: `packages/shared/src/modelImageBudget.ts` (2 lines)
- wave 0: `packages/shared/src/modelName.ts` (112 lines)
- wave 0: `packages/shared/src/nativeLocalizationResources.ts` (4 lines)
- wave 0: `packages/shared/src/outboundHttpPolicy.ts` (199 lines)
- wave 0: `packages/shared/src/path.ts` (107 lines)
- wave 0: `packages/shared/src/pathContainment.ts` (16 lines)
- wave 0: `packages/shared/src/pinnedMessages.ts` (96 lines)
- wave 0: `packages/shared/src/platformEnvironment.ts` (33 lines)
- wave 0: `packages/shared/src/processSpawnOutcome.ts` (26 lines)
- wave 0: `packages/shared/src/projectAgent.ts` (209 lines)
- wave 0: `packages/shared/src/projectDirectoryName.ts` (19 lines)
- wave 0: `packages/shared/src/providerCliProfiles.ts` (127 lines)
- wave 0: `packages/shared/src/providerDeliveryBlock.ts` (27 lines)
- wave 0: `packages/shared/src/providerInstances.ts` (637 lines)
- wave 0: `packages/shared/src/providerMetadata.ts` (147 lines)
- wave 0: `packages/shared/src/pullRequestList.ts` (138 lines)
- wave 0: `packages/shared/src/releaseDownloads.ts` (106 lines)
- wave 0: `packages/shared/src/remoteCapabilities.ts` (20 lines)
- wave 0: `packages/shared/src/remoteResources.ts` (99 lines)
- wave 0: `packages/shared/src/runtimeMode.ts` (59 lines)
- wave 0: `packages/shared/src/schemaJson.ts` (13 lines)
- wave 0: `packages/shared/src/searchQuery.ts` (22 lines)
- wave 0: `packages/shared/src/shell.ts` (302 lines)
- wave 0: `packages/shared/src/sidechatExpiry.ts` (175 lines)
- wave 0: `packages/shared/src/sidechatThread.ts` (32 lines)
- wave 0: `packages/shared/src/socialLinks.ts` (88 lines)
- wave 0: `packages/shared/src/staticSnapshot.ts` (120 lines)
- wave 0: `packages/shared/src/subagents.ts` (607 lines)
- wave 0: `packages/shared/src/synaraHome.ts` (44 lines)
- wave 0: `packages/shared/src/terminalThreads.ts` (429 lines)
- wave 0: `packages/shared/src/text.ts` (57 lines)
- wave 0: `packages/shared/src/threadDetailEvents.ts` (37 lines)
- wave 0: `packages/shared/src/threadExport.ts` (20 lines)
- wave 0: `packages/shared/src/threadHierarchy.ts` (48 lines)
- wave 0: `packages/shared/src/threadMentions.ts` (16 lines)
- wave 0: `packages/shared/src/threadSummary.ts` (383 lines)
- wave 0: `packages/shared/src/threadWorkspace.ts` (165 lines)
- wave 0: `packages/shared/src/todo.ts` (37 lines)
- wave 0: `packages/shared/src/transportRace.ts` (224 lines)
- wave 0: `packages/shared/src/turnResume.ts` (55 lines)
- wave 0: `packages/shared/src/turnStopActivity.ts` (22 lines)
- wave 0: `packages/shared/src/uiLocale.ts` (11 lines)
- wave 0: `packages/shared/src/uiLocalizationResources.ts` (20 lines)
- wave 0: `packages/shared/src/uiTreeTargeting.ts` (155 lines)
- wave 0: `packages/shared/src/windowsCertificate.ts` (89 lines)
- wave 0: `packages/shared/src/workspaceFileWrite.ts` (10 lines)
- wave 0: `packages/shared/src/worktreeHandoff.ts` (77 lines)
- wave 0: `packages/shared/src/wsRequestClass.ts` (99 lines)
- wave 0: `scripts/benchmark-cua-build.ts` (195 lines)
- wave 0: `scripts/bun-lockfile.ts` (17 lines)
- wave 0: `scripts/canary.ts` (512 lines)
- wave 0: `scripts/check-brand-identity.ts` (334 lines)
- wave 0: `scripts/check-server-sync-fs.ts` (180 lines)
- wave 0: `scripts/check-windows-runtime-boundary.ts` (255 lines)
- wave 0: `scripts/claude-cache-report.ts` (263 lines)
- wave 0: `scripts/dev-runner.ts` (536 lines)
- wave 0: `scripts/device-helper-sweep.ts` (155 lines)
- wave 0: `scripts/i18n.ts` (899 lines)
- wave 0: `scripts/lib/` (3560 lines)
- wave 0: `scripts/patch-workspace-typescript.ts` (59 lines)
- wave 0: `scripts/report-build-runner.ts` (15 lines)
- wave 0: `scripts/typecheck.ts` (105 lines)
- wave 0: `scripts/update-release-package-versions.ts` (93 lines)
- wave 0: `vitest.shards.ts` (22 lines)
- wave 1: `apps/analytics/hostUsage.ts` (142 lines)
- wave 1: `apps/api/src/config.ts`, `apps/api/src/identity/`, `apps/api/src/testing/`, tangled: one piece (4647 lines)
- wave 1: `apps/desktop/src/appSnapManager.ts` (2146 lines)
- wave 1: `apps/desktop/src/backendIssueDetector.ts` (36 lines)
- wave 1: `apps/desktop/src/backendStartupReadiness.ts` (113 lines)
- wave 1: `apps/desktop/src/betaChannel.ts` (586 lines)
- wave 1: `apps/desktop/src/betaRendererDiagnostics.ts` (70 lines)
- wave 1: `apps/desktop/src/browserSessionPolicy.ts` (219 lines)
- wave 1: `apps/desktop/src/browserWebMcp/` (694 lines)
- wave 1: `apps/desktop/src/computerFrameTap.ts` (362 lines)
- wave 1: `apps/desktop/src/computerShield.ts` (310 lines)
- wave 1: `apps/desktop/src/confirmDialog.ts` (56 lines)
- wave 1: `apps/desktop/src/desktopWsBridge.ts` (14 lines)
- wave 1: `apps/desktop/src/macAppIcon.ts` (85 lines)
- wave 1: `apps/desktop/src/productAnalytics.ts` (929 lines)
- wave 1: `apps/desktop/src/remoteResourceBroker.ts` (265 lines)
- wave 1: `apps/desktop/src/resumableUpdateDownload.ts` (688 lines)
- wave 1: `apps/desktop/src/safariAccessIpc.ts` (147 lines)
- wave 1: `apps/desktop/src/startupDiagnosticDialog.ts` (26 lines)
- wave 1: `apps/desktop/src/updateInstallMarker.ts` (233 lines)
- wave 1: `apps/desktop/src/updateMachine.ts` (216 lines)
- wave 1: `apps/desktop/src/voiceTranscription.ts` (253 lines)
- wave 1: `apps/server/src/atomicWrite.ts` (172 lines)
- wave 1: `apps/server/src/attachmentStore.ts` (112 lines)
- wave 1: `apps/server/src/automation/Services/` (123 lines)
- wave 1: `apps/server/src/betaOperationalIssue.ts` (22 lines)
- wave 1: `apps/server/src/browserAutomation/Services/` (23 lines)
- wave 1: `apps/server/src/codexHomePaths.ts` (117 lines)
- wave 1: `apps/server/src/codexPathIdentity.ts` (42 lines)
- wave 1: `apps/server/src/config.ts` (323 lines)
- wave 1: `apps/server/src/editorAppIcons.ts` (690 lines)
- wave 1: `apps/server/src/effectRelaySocket.ts` (52 lines)
- wave 1: `apps/server/src/localServerMonitor.ts` (940 lines)
- wave 1: `apps/server/src/open.ts` (441 lines)
- wave 1: `apps/server/src/scratchWorkspaces.ts` (207 lines)
- wave 1: `apps/server/src/sensitiveKeys.ts` (105 lines)
- wave 1: `apps/server/src/todo/Services/` (23 lines)
- wave 1: `apps/server/src/wsSnapshotLiveStream.ts` (315 lines)
- wave 1: `apps/server/vitest.shards.ts` (16 lines)
- wave 1: `apps/web/src/authSignedOut.ts` (39 lines)
- wave 1: `apps/web/src/chatRouteRecovery.ts` (25 lines)
- wave 1: `apps/web/src/diffRenderModeStore.ts` (93 lines)
- wave 1: `apps/web/src/feedback.ts` (164 lines)
- wave 1: `apps/web/src/pairingBootstrap.ts` (82 lines)
- wave 1: `apps/web/src/projectRunTargets.ts` (98 lines)
- wave 1: `apps/web/src/projectTerminalRunner.ts` (62 lines)
- wave 1: `apps/web/src/providerModelOptions.ts` (594 lines)
- wave 1: `apps/web/src/providerUpdates.ts` (167 lines)
- wave 1: `apps/web/src/sidebarRowStyles.ts` (98 lines)
- wave 1: `apps/web/src/tasksSurface.ts` (33 lines)
- wave 1: `benchmarks/desktop-perf/` (2664 lines)
- wave 1: `packages/contracts/src/account.ts` (648 lines)
- wave 1: `packages/contracts/src/asyncUserInput.ts` (24 lines)
- wave 1: `packages/contracts/src/auth.ts` (125 lines)
- wave 1: `packages/contracts/src/browserAnnotations.ts` (130 lines)
- wave 1: `packages/contracts/src/browserAutomationCssSelector.ts` (344 lines)
- wave 1: `packages/contracts/src/browserAutomationIds.ts` (71 lines)
- wave 1: `packages/contracts/src/browserAutomationJson.ts` (35 lines)
- wave 1: `packages/contracts/src/browserVault.ts` (91 lines)
- wave 1: `packages/contracts/src/claudeCache.ts` (30 lines)
- wave 1: `packages/contracts/src/computer.ts` (1223 lines)
- wave 1: `packages/contracts/src/computerAudit.ts` (48 lines)
- wave 1: `packages/contracts/src/desktopUpdate.ts` (71 lines)
- wave 1: `packages/contracts/src/device.ts` (631 lines)
- wave 1: `packages/contracts/src/editor.ts` (252 lines)
- wave 1: `packages/contracts/src/editorDebug.ts` (122 lines)
- wave 1: `packages/contracts/src/editorIntelligence.ts` (125 lines)
- wave 1: `packages/contracts/src/environment.ts` (42 lines)
- wave 1: `packages/contracts/src/filesystem.ts` (21 lines)
- wave 1: `packages/contracts/src/hostConnection.ts` (97 lines)
- wave 1: `packages/contracts/src/hostSessions.ts` (33 lines)
- wave 1: `packages/contracts/src/inlineVisual.ts` (53 lines)
- wave 1: `packages/contracts/src/keybindings.ts` (214 lines)
- wave 1: `packages/contracts/src/liveActivities.ts` (114 lines)
- wave 1: `packages/contracts/src/mcpConnectors.ts` (134 lines)
- wave 1: `packages/contracts/src/project.ts` (311 lines)
- wave 1: `packages/contracts/src/providerInstance.ts` (69 lines)
- wave 1: `packages/contracts/src/pullRequestAutoFix.ts` (63 lines)
- wave 1: `packages/contracts/src/remoteResources.ts` (53 lines)
- wave 1: `packages/contracts/src/studio.ts` (28 lines)
- wave 1: `packages/contracts/src/todo.ts` (109 lines)
- wave 1: `packages/profile-ui/src/formatting.ts` (129 lines)
- wave 1: `packages/shared/src/KeyedDrainableWorker.ts` (313 lines)
- wave 1: `packages/shared/src/browserAutomationCatalogue.ts` (439 lines)
- wave 1: `packages/shared/src/codexStderr.ts` (151 lines)
- wave 1: `packages/shared/src/computerFrame.ts` (78 lines)
- wave 1: `packages/shared/src/deviceFrame.ts` (84 lines)
- wave 1: `packages/shared/src/githubRepository.ts` (72 lines)
- wave 1: `packages/shared/src/jsonrpc-stdio.ts` (560 lines)
- wave 1: `packages/shared/src/lengthPrefixedRecords.ts` (58 lines)
- wave 1: `packages/shared/src/localServers.ts` (66 lines)
- wave 1: `packages/shared/src/loginShellEnvironment.ts` (435 lines)
- wave 1: `packages/shared/src/model.ts` (894 lines)
- wave 1: `packages/shared/src/outboundHttp.ts` (605 lines)
- wave 1: `packages/shared/src/pendingInteractions.ts` (123 lines)
- wave 1: `packages/shared/src/projectContainers.ts` (72 lines)
- wave 1: `packages/shared/src/projectFolders.ts` (114 lines)
- wave 1: `packages/shared/src/providerUsage.ts` (54 lines)
- wave 1: `packages/shared/src/serverSettings.ts` (160 lines)
- wave 1: `packages/shared/src/threadEnvironment.ts` (58 lines)
- wave 1: `packages/shared/src/toolOutputSummary.ts` (69 lines)
- wave 1: `packages/shared/src/windowsProcess.ts` (144 lines)
- wave 1: `scripts/build-desktop-artifact.ts` (1502 lines)
- wave 1: `scripts/ci-install-key.ts` (45 lines)
- wave 1: `scripts/finalize-mac-dmg.ts` (13 lines)
- wave 1: `scripts/import-portable-build.ts` (44 lines)
- wave 1: `scripts/install-git-hooks.ts` (78 lines)
- wave 1: `scripts/mac-icon-catalog.ts` (108 lines)
- wave 1: `scripts/merge-mac-update-manifests.ts` (31 lines)
- wave 1: `scripts/node-pty-smoke.mjs` (65 lines)
- wave 1: `scripts/notarize-mac-app.ts` (81 lines)
- wave 1: `scripts/portable-build.ts` (20 lines)
- wave 1: `scripts/pre-push-check.ts` (185 lines)
- wave 1: `scripts/prepare-release-update-feed.ts` (37 lines)
- wave 1: `scripts/release-smoke.ts` (1289 lines)
- wave 1: `scripts/resolve-release-build-scope.ts` (20 lines)
- wave 1: `scripts/resolve-release-update-policy.ts` (31 lines)
- wave 1: `scripts/smoke-mac-update-artifact.ts` (292 lines)
- wave 1: `scripts/verify-release-artifact-checkpoint.ts` (70 lines)
- wave 1: `scripts/verify-release-source-provenance.ts` (89 lines)
- wave 1: `scripts/wait-for-release-job.ts` (39 lines)
- wave 1: `scripts/write-release-artifact-provenance.ts` (96 lines)
- wave 2: `apps/analytics/worker.ts` (2766 lines)
- wave 2: `apps/api/scripts/` (367 lines)
- wave 2: `apps/api/src/avatarStorage.ts` (154 lines)
- wave 2: `apps/api/src/billing/` (1051 lines)
- wave 2: `apps/api/src/liveActivities/` (892 lines)
- wave 2: `apps/api/src/remote/` (594 lines)
- wave 2: `apps/desktop/src/appSnapIpc.ts` (162 lines)
- wave 2: `apps/desktop/src/browserAnnotations/` (2472 lines)
- wave 2: `apps/desktop/src/escapeKillSwitchMonitor.ts` (275 lines)
- wave 2: `apps/desktop/src/preload.ts` (468 lines)
- wave 2: `apps/desktop/src/productAnalyticsIpc.ts` (36 lines)
- wave 2: `apps/marketing/src/app/ai.txt/` (13 lines)
- wave 2: `apps/marketing/src/app/api/feedback/` (266 lines)
- wave 2: `apps/marketing/src/app/api/installer-count/` (34 lines)
- wave 2: `apps/marketing/src/app/api/search/` (5 lines)
- wave 2: `apps/marketing/src/app/llms-full.txt/` (13 lines)
- wave 2: `apps/marketing/src/app/llms.mdx/docs/[[...slug]]/` (28 lines)
- wave 2: `apps/marketing/src/app/llms.txt/` (13 lines)
- wave 2: `apps/marketing/src/app/sitemap-index.xml/` (21 lines)
- wave 2: `apps/server/src/betaUsageSnapshot.ts` (59 lines)
- wave 2: `apps/server/src/browserAutomation/Layers/` (46 lines)
- wave 2: `apps/server/src/codexGeneratedImages.ts` (346 lines)
- wave 2: `apps/server/src/environment/Layers/` (118 lines)
- wave 2: `apps/server/src/eventLoopMonitor.ts` (284 lines)
- wave 2: `apps/server/src/hostIdentity/` (167 lines)
- wave 2: `apps/server/src/hostUsageSnapshot.ts` (286 lines)
- wave 2: `apps/server/src/inlineVisualStore.ts` (85 lines)
- wave 2: `apps/server/src/keybindings.ts` (1331 lines)
- wave 2: `apps/server/src/memoryDiagnostics.ts` (103 lines)
- wave 2: `apps/server/src/profileStats.ts` (1627 lines)
- wave 2: `apps/server/src/providerUsageSnapshot.ts` (1084 lines)
- wave 2: `apps/server/src/serverLogger.ts` (15 lines)
- wave 2: `apps/server/src/serverShutdown.ts` (77 lines)
- wave 2: `apps/server/src/trustedOrigins.ts` (133 lines)
- wave 2: `apps/server/src/usagePricing.ts` (279 lines)
- wave 2: `apps/web/src/authLogout.ts` (18 lines)
- wave 2: `apps/web/src/feedbackDialogStore.ts` (34 lines)
- wave 2: `apps/web/src/settingsPanelStyles.ts` (65 lines)
- wave 2: `packages/contracts/src/accountBilling.ts` (40 lines)
- wave 2: `packages/contracts/src/accountUsage.ts` (236 lines)
- wave 2: `packages/contracts/src/browserAutomationErrors.ts` (279 lines)
- wave 2: `packages/contracts/src/browserAutomationTargets.ts` (109 lines)
- wave 2: `packages/contracts/src/hostAuth.ts` (270 lines)
- wave 2: `packages/contracts/src/productAnalytics.ts` (191 lines)
- wave 2: `packages/contracts/src/projectImport.ts` (87 lines)
- wave 2: `packages/contracts/src/terminal.ts` (165 lines)
- wave 2: `packages/contracts/src/usageLimit.ts` (41 lines)
- wave 2: `packages/shared/src/browserAutomationMcpError.ts` (16 lines)
- wave 2: `packages/shared/src/chatGptVoiceTranscription.ts` (75 lines)
- wave 2: `packages/shared/src/platformProcess.ts` (237 lines)
- wave 2: `packages/shared/src/threadPullRequestLinks.ts` (281 lines)
- wave 2: `packages/shared/src/usageAnalytics.ts` (71 lines)
- wave 2: `packages/shared/src/wslBridge.ts` (22 lines)
- wave 3: `apps/api/src/accountDeletion.ts` (258 lines)
- wave 3: `apps/desktop/src/browserManager.ts` (3576 lines)
- wave 3: `apps/desktop/src/cuaDriverHost.ts` (3136 lines)
- wave 3: `apps/desktop/src/linuxEscapeKillSwitchMonitor.ts` (159 lines)
- wave 3: `apps/server/src/hostAuth/` (451 lines)
- wave 3: `apps/server/src/recapStats.ts` (496 lines)
- wave 3: `apps/server/src/transcriptUsage.ts` (333 lines)
- wave 3: `apps/web/src/settingsSidebarNavStyles.ts` (34 lines)
- wave 3: `packages/contracts/src/agentMentions.ts`, `packages/contracts/src/model.ts`, `packages/contracts/src/orchestration.ts`, `packages/contracts/src/providerDiscovery.ts`, tangled: one piece (5256 lines)
- wave 3: `packages/contracts/src/browserAutomationToolInputs.ts` (465 lines)
- wave 3: `packages/contracts/src/browserAutomationToolOutputs.ts` (405 lines)
- wave 3: `packages/contracts/src/hostSecrets.ts` (169 lines)
- wave 3: `packages/contracts/src/remotePairing.ts` (158 lines)
- wave 3: `packages/shared/src/hostUsage.ts` (168 lines)
- wave 3: `packages/shared/src/processRuntime.ts` (208 lines)
- wave 4: `apps/api/src/routes/` (3369 lines)
- wave 4: `apps/desktop/src/browserAutomation/` (6821 lines)
- wave 4: `apps/desktop/src/browserIpc.ts` (174 lines)
- wave 4: `apps/desktop/src/cuaDriverHostStandalone.ts` (138 lines)
- wave 4: `apps/desktop/src/linuxCuaDriverHost.ts` (37 lines)
- wave 4: `apps/server/src/usageAnalyticsStats.ts` (870 lines)
- wave 4: `apps/web/src/appRail.logic.ts` (258 lines)
- wave 4: `apps/web/src/appSnap.logic.ts` (203 lines)
- wave 4: `apps/web/src/appSnapAttach.ts` (34 lines)
- wave 4: `apps/web/src/appSnapIntake.ts` (116 lines)
- wave 4: `apps/web/src/appSnapShortcut.ts` (51 lines)
- wave 4: `apps/web/src/browserStateStore.ts` (211 lines)
- wave 4: `apps/web/src/composer-editor-mentions.ts` (398 lines)
- wave 4: `apps/web/src/composer-logic.ts` (392 lines)
- wave 4: `apps/web/src/composerDraftActions.ts` (2345 lines)
- wave 4: `apps/web/src/composerDraftAttachments.ts` (577 lines)
- wave 4: `apps/web/src/composerDraftDomain.ts` (1044 lines)
- wave 4: `apps/web/src/composerDraftModels.ts` (968 lines)
- wave 4: `apps/web/src/composerDraftPersistence.ts` (1674 lines)
- wave 4: `apps/web/src/composerDraftStoreTestFixtures.ts` (196 lines)
- wave 4: `apps/web/src/composerSlashCommands.ts` (593 lines)
- wave 4: `apps/web/src/computerControlMode.ts` (16 lines)
- wave 4: `apps/web/src/computerPreviewStore.ts` (240 lines)
- wave 4: `apps/web/src/confirmDialogFallback.ts` (91 lines)
- wave 4: `apps/web/src/confirmedCustomBinaryPathStore.ts` (60 lines)
- wave 4: `apps/web/src/contextMenuFallback.ts` (239 lines)
- wave 4: `apps/web/src/editorPreferences.ts` (30 lines)
- wave 4: `apps/web/src/editorViewState.ts` (63 lines)
- wave 4: `apps/web/src/fixedShortcuts.ts` (121 lines)
- wave 4: `apps/web/src/groupPanelClosedStore.ts` (53 lines)
- wave 4: `apps/web/src/kanbanUiStore.ts` (207 lines)
- wave 4: `apps/web/src/keybindingEditor.ts` (478 lines)
- wave 4: `apps/web/src/keybindings.ts` (927 lines)
- wave 4: `apps/web/src/latestProjectStore.ts` (42 lines)
- wave 4: `apps/web/src/lib/projectCatalog/` (292 lines)
- wave 4: `apps/web/src/markdown-links.ts` (283 lines)
- wave 4: `apps/web/src/messageTextSegments.ts` (72 lines)
- wave 4: `apps/web/src/nativeApi.ts` (54 lines)
- wave 4: `apps/web/src/openThreadTabs.logic.ts` (299 lines)
- wave 4: `apps/web/src/openThreadTabsStore.ts` (84 lines)
- wave 4: `apps/web/src/pendingInteractionDerivation.ts` (370 lines)
- wave 4: `apps/web/src/pendingUserInputRecovery.ts` (82 lines)
- wave 4: `apps/web/src/pinnedMessages.ts` (184 lines)
- wave 4: `apps/web/src/pinnedProjectAgentsStore.ts` (74 lines)
- wave 4: `apps/web/src/pinnedProjectsStore.ts` (78 lines)
- wave 4: `apps/web/src/pinnedThreadsStore.ts` (81 lines)
- wave 4: `apps/web/src/projectEnvironmentStore.ts` (36 lines)
- wave 4: `apps/web/src/projectInstructionsStore.ts` (78 lines)
- wave 4: `apps/web/src/railShellStore.ts` (128 lines)
- wave 4: `apps/web/src/recentViewActivation.logic.ts` (29 lines)
- wave 4: `apps/web/src/recentViews.logic.ts` (253 lines)
- wave 4: `apps/web/src/recentViewsStore.ts` (112 lines)
- wave 4: `apps/web/src/remoteProjectSpacesStore.ts` (31 lines)
- wave 4: `apps/web/src/rightDockStore.logic.ts` (437 lines)
- wave 4: `apps/web/src/rightDockStore.ts` (142 lines)
- wave 4: `apps/web/src/routeTree.gen.ts` (443 lines)
- wave 4: `apps/web/src/session-logic.ts` (710 lines)
- wave 4: `apps/web/src/settingsNavigation.ts` (335 lines)
- wave 4: `apps/web/src/settingsSearchIndex.ts` (435 lines)
- wave 4: `apps/web/src/settingsSearchRows.generated.ts` (534 lines)
- wave 4: `apps/web/src/shortcutsSheet.ts` (714 lines)
- wave 4: `apps/web/src/spacesUiStore.ts` (163 lines)
- wave 4: `apps/web/src/splitView.logic.ts`, `apps/web/src/splitViewStore.ts`, tangled: one piece (932 lines)
- wave 4: `apps/web/src/splitViewRoute.ts` (37 lines)
- wave 4: `apps/web/src/storeEventReducer.ts` (1948 lines)
- wave 4: `apps/web/src/storeNormalization.ts` (2365 lines)
- wave 4: `apps/web/src/storePersistence.ts` (217 lines)
- wave 4: `apps/web/src/storeProjection.ts` (1592 lines)
- wave 4: `apps/web/src/storeSelectors.ts` (653 lines)
- wave 4: `apps/web/src/storeState.ts` (98 lines)
- wave 4: `apps/web/src/storeTestFixtures.ts` (301 lines)
- wave 4: `apps/web/src/terminal-links.ts` (237 lines)
- wave 4: `apps/web/src/terminalStateStore.ts` (939 lines)
- wave 4: `apps/web/src/theme/` (2198 lines)
- wave 4: `apps/web/src/threadActivation.logic.ts` (74 lines)
- wave 4: `apps/web/src/threadDerivation.ts` (165 lines)
- wave 4: `apps/web/src/threadGoal.ts` (32 lines)
- wave 4: `apps/web/src/threadVisitedPersistence.ts` (149 lines)
- wave 4: `apps/web/src/timestampFormat.ts` (122 lines)
- wave 4: `apps/web/src/types.ts` (399 lines)
- wave 4: `apps/web/src/voidSpaceStore.ts` (98 lines)
- wave 4: `apps/web/src/workLog.ts` (4361 lines)
- wave 4: `apps/web/src/workflowRunUiStore.ts` (167 lines)
- wave 4: `apps/web/src/workspacePathsStore.ts` (136 lines)
- wave 4: `apps/web/src/worktreeCleanup.ts` (52 lines)
- wave 4: `apps/web/src/wsNativeApi.ts` (1363 lines)
- wave 4: `apps/web/src/wsTransport.ts` (3021 lines)
- wave 4: `docs/computer-use-cua/evidence/` (777 lines)
- wave 4: `packages/contracts/src/automation.ts` (490 lines)
- wave 4: `packages/contracts/src/externalMcp.ts` (153 lines)
- wave 4: `packages/contracts/src/git.ts` (646 lines)
- wave 4: `packages/contracts/src/githubProjectProvisioning.ts` (65 lines)
- wave 4: `packages/contracts/src/provider.ts` (241 lines)
- wave 4: `packages/contracts/src/providerRuntime.ts` (1129 lines)
- wave 4: `packages/contracts/src/remoteAgentGateway.ts` (35 lines)
- wave 4: `packages/contracts/src/settings.ts` (303 lines)
- wave 4: `packages/contracts/src/stats.ts` (323 lines)
- wave 4: `packages/shared/src/cloudflared.ts` (98 lines)
- wave 4: `packages/shared/src/parentBoundProcess.ts` (198 lines)
- wave 5: `apps/api/src/app.ts` (216 lines)
- wave 5: `apps/desktop/src/browserUsePipeServer.ts` (519 lines)
- wave 5: `apps/desktop/src/browserVaultIpc.ts` (68 lines)
- wave 5: `apps/server/src/accountAuth.ts`, `apps/server/src/accountCredentialLock.ts`, `apps/server/src/accountDeviceRegistration.ts`, `apps/server/src/accountRpcErrors.ts`, `apps/server/src/accountSession.ts`, `apps/server/src/accountSignInTiming.ts`, `apps/server/src/agentGateway/`, `apps/server/src/agentGateway/Layers/`, `apps/server/src/agentGateway/Services/`, `apps/server/src/assistantImageAttachments.ts`, `apps/server/src/auth/`, `apps/server/src/auth/Layers/`, `apps/server/src/checkpointing/`, `apps/server/src/checkpointing/Services/`, `apps/server/src/codexAppServerManager.ts`, `apps/server/src/codexProcessEnv.ts`, `apps/server/src/computer/`, `apps/server/src/computer/Services/`, `apps/server/src/desktopUpdateRpc.ts`, `apps/server/src/devServerManager.ts`, `apps/server/src/device/`, `apps/server/src/device/Services/`, `apps/server/src/deviceIdentity/`, `apps/server/src/diagnostics/`, `apps/server/src/diagnostics/Services/`, `apps/server/src/editor/`, `apps/server/src/externalMcp/`, `apps/server/src/externalMcp/Services/`, `apps/server/src/git/`, `apps/server/src/git/Layers/`, `apps/server/src/git/Services/`, `apps/server/src/githubInbox/`, `apps/server/src/githubInbox/Services/`, `apps/server/src/hostConnections/`, `apps/server/src/hostSecrets/`, `apps/server/src/keepAwake.ts`, `apps/server/src/localImageFiles.ts`, `apps/server/src/managedAttachmentStore.ts`, `apps/server/src/managedWorktrees.ts`, `apps/server/src/mcpConnectors/`, `apps/server/src/mcpConnectors/Layers/`, `apps/server/src/mcpConnectors/Services/`, `apps/server/src/orchestration/`, `apps/server/src/orchestration/Layers/`, `apps/server/src/orchestration/Services/`, `apps/server/src/persistence/`, `apps/server/src/persistence/Layers/`, `apps/server/src/persistence/Migrations/`, `apps/server/src/persistence/Services/`, `apps/server/src/platform/`, `apps/server/src/processRunner.ts`, `apps/server/src/profileStatsArchive.ts`, `apps/server/src/project/`, `apps/server/src/projectAgent/`, `apps/server/src/projectAgent/Services/`, `apps/server/src/provider/`, `apps/server/src/provider/Layers/`, `apps/server/src/provider/Services/`, `apps/server/src/provider/acp/`, `apps/server/src/providerCredentials.ts`, `apps/server/src/providerProcessPriority.ts`, `apps/server/src/providerUsage/`, `apps/server/src/providerUsage/providers/`, `apps/server/src/pullRequests.logic.ts`, `apps/server/src/pullRequests/`, `apps/server/src/remotePairing/`, `apps/server/src/remoteSessions/`, `apps/server/src/remoteTransport/`, `apps/server/src/serverRuntimeState.ts`, `apps/server/src/serverSettings.ts`, `apps/server/src/studioGeneratedImages.ts`, `apps/server/src/terminal/`, `apps/server/src/terminal/Layers/`, `apps/server/src/terminal/Services/`, `apps/server/src/threadRetention.ts`, `apps/server/src/threadSearch.ts`, `apps/server/src/workspace/`, `apps/server/src/workspace/Layers/`, `apps/server/src/workspaceEntries.ts`, `apps/server/src/workspaceFileChanges.ts`, `apps/server/src/worktreeSetup.ts`, `apps/server/src/wsAccountRpc.ts`, `apps/server/src/wsConnectionSessions.ts`, `apps/server/src/wsHostsRpc.ts`, `apps/server/src/wsOwnerAuthorization.ts`, `apps/server/src/wsOwnerOnly.ts`, `apps/server/src/wsRequestAdmission.ts`, `apps/server/src/wsRpc.ts`, tangled: one piece (263978 lines)
- wave 5: `apps/web/scripts/` (1325 lines)
- wave 5: `packages/contracts/src/inboxRecaps.ts` (152 lines)
- wave 5: `packages/contracts/src/pullRequests.ts` (299 lines)
- wave 5: `packages/contracts/src/server.ts` (706 lines)
- wave 5: `packages/contracts/src/wsCompatibility.ts` (120 lines)
- wave 6: `apps/api/src/index.ts` (83 lines)
- wave 6: `apps/desktop/scripts/` (3786 lines)
- wave 6: `apps/desktop/src/cuaFixtures/` (2494 lines)
- wave 6: `apps/desktop/src/main.ts` (6409 lines)
- wave 6: `apps/server/scripts/` (2172 lines)
- wave 6: `apps/server/src/accountInboxReporter.ts` (210 lines)
- wave 6: `apps/server/src/accountUsageReporter.ts` (1075 lines)
- wave 6: `apps/server/src/automation/Layers/` (3969 lines)
- wave 6: `apps/server/src/betaImport.ts` (423 lines)
- wave 6: `apps/server/src/checkpointing/Layers/` (1518 lines)
- wave 6: `apps/server/src/cloudflare/` (496 lines)
- wave 6: `apps/server/src/compactDatabase.ts` (48 lines)
- wave 6: `apps/server/src/completionNotificationPreviews.ts` (293 lines)
- wave 6: `apps/server/src/composerPrediction/` (305 lines)
- wave 6: `apps/server/src/computer/Layers/` (94 lines)
- wave 6: `apps/server/src/device/Layers/` (69 lines)
- wave 6: `apps/server/src/diagnostics/Layers/` (173 lines)
- wave 6: `apps/server/src/externalMcp/Layers/` (2129 lines)
- wave 6: `apps/server/src/git/testing/` (559 lines)
- wave 6: `apps/server/src/githubInbox/Layers/` (471 lines)
- wave 6: `apps/server/src/hostConnectivitySupervisor.ts` (249 lines)
- wave 6: `apps/server/src/http.ts` (1594 lines)
- wave 6: `apps/server/src/managedAttachmentCleanup.ts` (188 lines)
- wave 6: `apps/server/src/orchestration/testing/` (47 lines)
- wave 6: `apps/server/src/project/Layers/` (154 lines)
- wave 6: `apps/server/src/projectAgent/Layers/` (7113 lines)
- wave 6: `apps/server/src/provider/acp/testing/` (61 lines)
- wave 6: `apps/server/src/provider/testing/` (92 lines)
- wave 6: `apps/server/src/pullRequestAutoFix/` (169 lines)
- wave 6: `apps/server/src/pullRequests/Layers/` (109 lines)
- wave 6: `apps/server/src/remoteBillingPolicy.ts` (345 lines)
- wave 6: `apps/server/src/restoreMigrationBackup.ts` (69 lines)
- wave 6: `apps/server/src/runtimeDependencySmoke.ts` (235 lines)
- wave 6: `apps/server/src/todo/Layers/` (180 lines)
- wave 6: `apps/web/src/bootstrap.ts` (76 lines)
- wave 6: `packages/contracts/src/agentGateway.ts` (228 lines)
- wave 6: `packages/contracts/src/githubInbox.ts` (166 lines)
- wave 6: `scripts/check-migration-lineage.ts` (305 lines)
- wave 6: `scripts/device-helper-smoke.ts` (542 lines)
- wave 6: `scripts/verify-packaged-desktop-startup.ts` (562 lines)
- wave 7: `apps/server/integration/` (1088 lines)
- wave 7: `apps/server/src/accountLiveActivityReporter.ts` (383 lines)
- wave 7: `apps/server/src/composerPrediction/Layers/` (257 lines)
- wave 7: `apps/server/src/effectServer.ts` (355 lines)
- wave 7: `apps/server/src/hostConnectivity.ts` (496 lines)
- wave 7: `apps/server/src/pullRequestAutoFix/Layers/` (496 lines)
- wave 7: `packages/contracts/src/hubWork.ts` (135 lines)
- wave 7: `scripts/computer-use-fixtures/` (9857 lines)
- wave 8: `apps/server/src/serverLayers.ts` (396 lines)
- wave 8: `packages/contracts/src/projectAgent.ts` (926 lines)
- wave 9: `apps/server/src/main.ts` (1136 lines)
- wave 9: `packages/contracts/src/ipc.ts` (1686 lines)
- wave 9: `packages/contracts/src/ws.ts` (1009 lines)
- wave 10: `apps/server/src/index.ts` (33 lines)
- wave 10: `packages/contracts/src/rpc.ts` (2545 lines)
- wave 11: `packages/contracts/src/index.ts` (76 lines)

### Decisions for the user

**D1** (shape): The app is a server, entered through apps/desktop/scripts/betterwright-recovery-smoke.ts, apps/desktop/scripts/browser-integration-fixture.mjs, apps/desktop/scripts/browser-lifecycle-smoke.ts, apps/desktop/scripts/build-appsnap-helper.mjs, apps/desktop/scripts/build-window-material-addon.mjs, apps/desktop/scripts/credential-recovery-smoke.ts, apps/desktop/scripts/embedded-popup-smoke.ts, apps/desktop/scripts/remote-resource-smoke.ts, apps/desktop/src/browserAutomation/betterwrightConnection.ts, apps/desktop/src/browserUsePipeServer.ts, apps/desktop/src/computerFrameTap.ts, apps/desktop/src/cuaDriverHost.ts, apps/desktop/src/cuaDriverHostStandalone.ts, apps/desktop/src/cuaFixtures/electron.ts, apps/desktop/src/cuaFixtures/live.ts, apps/desktop/src/main.ts, apps/marketing/scripts/check-external-links.mjs, apps/server/scripts/claude-context-window-probe.mjs, apps/server/scripts/cli.ts, apps/server/src/accountSsoCallback.ts, apps/server/src/cloudflare/connector.ts, apps/server/src/cloudflare/ingress.ts, apps/server/src/device/helperClient.ts, apps/server/src/editor/test-fixtures/noisyDebugAdapter.mjs, apps/server/src/effectServer.ts, apps/server/src/hostConnectivity.ts, apps/server/src/nodeHttpServer.ts, apps/server/src/remoteTransport/tunnel.ts, apps/web/perf/concurrent-runner.mjs, apps/web/scripts/hugeicon-snippet.mjs, apps/web/scripts/measure-dev-transforms.mjs, benchmarks/desktop-perf/diagnose-backend-boot.mjs, benchmarks/desktop-perf/diagnose.mjs, benchmarks/desktop-perf/lib/launch.mjs, benchmarks/desktop-perf/longchat-report.mjs, benchmarks/desktop-perf/longchat.mjs, benchmarks/desktop-perf/prepare.mjs, benchmarks/desktop-perf/report.mjs, benchmarks/desktop-perf/run.mjs, docs/computer-use-cua/evidence/latency-rev17-probe.ts, docs/computer-use-cua/evidence/latency-rev17-supplement-probe.ts, packages/shared/src/Net.ts, scripts/build-desktop-artifact.ts, scripts/computer-use-fixtures/belief-canary.mjs, scripts/computer-use-fixtures/canary-main.ts, scripts/computer-use-fixtures/live-cert.ts, scripts/computer-use-fixtures/multi-display-cert.ts, scripts/computer-use-fixtures/packaged-e2e.ts, scripts/dev-runner.ts, scripts/device-helper-smoke.ts, scripts/node-pty-smoke.mjs, scripts/smoke-mac-update-artifact.ts, scripts/verify-packaged-desktop-startup.ts, scripts/verify-release-artifact-checkpoint.ts, scripts/write-release-artifact-provenance.ts. Port every entry point, in that shape?

> Yes: an axum front door (`app front-door`) takes the server over first

**D2** (keep apps/analytics/src/App.tsx): apps/analytics/src/App.tsx stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D3** (keep apps/analytics/src/TurnBreakdownTable.tsx): apps/analytics/src/TurnBreakdownTable.tsx stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D4** (keep apps/analytics/src/components.tsx): apps/analytics/src/components.tsx stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D5** (keep apps/analytics/src/filters.ts): apps/analytics/src/filters.ts stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D6** (keep apps/analytics/src/main.tsx): apps/analytics/src/main.tsx stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D7** (keep apps/analytics/src/theme.ts): apps/analytics/src/theme.ts stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D8** (keep apps/analytics/src/views/): apps/analytics/src/views/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D9** (keep apps/marketing/src/app/): apps/marketing/src/app/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D10** (keep apps/marketing/src/app/changelog/): apps/marketing/src/app/changelog/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D11** (keep apps/marketing/src/app/changelog/[version]/): apps/marketing/src/app/changelog/[version]/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D12** (keep apps/marketing/src/app/docs/): apps/marketing/src/app/docs/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D13** (keep apps/marketing/src/app/docs/[[...slug]]/): apps/marketing/src/app/docs/[[...slug]]/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D14** (keep apps/marketing/src/app/install/): apps/marketing/src/app/install/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D15** (keep apps/marketing/src/app/privacy/): apps/marketing/src/app/privacy/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D16** (keep apps/marketing/src/app/sponsor/): apps/marketing/src/app/sponsor/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D17** (keep apps/marketing/src/app/sponsors/): apps/marketing/src/app/sponsors/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D18** (keep apps/marketing/src/components/): apps/marketing/src/components/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D19** (keep apps/marketing/src/components/docs/): apps/marketing/src/components/docs/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D20** (keep apps/marketing/src/components/ui/): apps/marketing/src/components/ui/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D21** (keep apps/marketing/src/lib/): apps/marketing/src/lib/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D22** (keep apps/profiles/src/app/): apps/profiles/src/app/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D23** (keep apps/profiles/src/app/[handle]/): apps/profiles/src/app/[handle]/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D24** (keep apps/profiles/src/app/[handle]/share/[format]/): apps/profiles/src/app/[handle]/share/[format]/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D25** (keep apps/profiles/src/components/): apps/profiles/src/components/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D26** (keep apps/profiles/src/lib/): apps/profiles/src/lib/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D27** (keep apps/web/perf/): apps/web/perf/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D28** (keep apps/web/src/appNavigation.ts): apps/web/src/appNavigation.ts stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D29** (keep apps/web/src/appSettings.ts): apps/web/src/appSettings.ts stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D30** (keep apps/web/src/components/): apps/web/src/components/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D31** (keep apps/web/src/components/account/): apps/web/src/components/account/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D32** (keep apps/web/src/components/automation/): apps/web/src/components/automation/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D33** (keep apps/web/src/components/browser/): apps/web/src/components/browser/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D34** (keep apps/web/src/components/chat-drop-overlay/): apps/web/src/components/chat-drop-overlay/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D35** (keep apps/web/src/components/chat/): apps/web/src/components/chat/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D36** (keep apps/web/src/components/chat/auxiliary/): apps/web/src/components/chat/auxiliary/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D37** (keep apps/web/src/components/chat/environment/): apps/web/src/components/chat/environment/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D38** (keep apps/web/src/components/chat/group/): apps/web/src/components/chat/group/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D39** (keep apps/web/src/components/chat/project/): apps/web/src/components/chat/project/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D40** (keep apps/web/src/components/codeEditor/): apps/web/src/components/codeEditor/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D41** (keep apps/web/src/components/composer-nodes/): apps/web/src/components/composer-nodes/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D42** (keep apps/web/src/components/computer/): apps/web/src/components/computer/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D43** (keep apps/web/src/components/device/): apps/web/src/components/device/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D44** (keep apps/web/src/components/githubInbox/): apps/web/src/components/githubInbox/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D45** (keep apps/web/src/components/hosts/): apps/web/src/components/hosts/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D46** (keep apps/web/src/components/inbox/): apps/web/src/components/inbox/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D47** (keep apps/web/src/components/kanban/): apps/web/src/components/kanban/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D48** (keep apps/web/src/components/pdf/): apps/web/src/components/pdf/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D49** (keep apps/web/src/components/profile/): apps/web/src/components/profile/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D50** (keep apps/web/src/components/pullRequest/): apps/web/src/components/pullRequest/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D51** (keep apps/web/src/components/settings/): apps/web/src/components/settings/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D52** (keep apps/web/src/components/settings/accounts/): apps/web/src/components/settings/accounts/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D53** (keep apps/web/src/components/settings/providers/): apps/web/src/components/settings/providers/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D54** (keep apps/web/src/components/settings/views/): apps/web/src/components/settings/views/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D55** (keep apps/web/src/components/tasks/): apps/web/src/components/tasks/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D56** (keep apps/web/src/components/terminal/): apps/web/src/components/terminal/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D57** (keep apps/web/src/components/ui/): apps/web/src/components/ui/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D58** (keep apps/web/src/composerDraftStore.ts): apps/web/src/composerDraftStore.ts stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D59** (keep apps/web/src/editorMetadata.ts): apps/web/src/editorMetadata.ts stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D60** (keep apps/web/src/editorRailStore.ts): apps/web/src/editorRailStore.ts stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D61** (keep apps/web/src/featureFlags.ts): apps/web/src/featureFlags.ts stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D62** (keep apps/web/src/featureTour/): apps/web/src/featureTour/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D63** (keep apps/web/src/focusedChatContext.ts): apps/web/src/focusedChatContext.ts stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D64** (keep apps/web/src/hooks/): apps/web/src/hooks/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D65** (keep apps/web/src/i18n/): apps/web/src/i18n/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D66** (keep apps/web/src/lib/): apps/web/src/lib/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D67** (keep apps/web/src/lib/hosts/): apps/web/src/lib/hosts/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D68** (keep apps/web/src/lib/pdf/): apps/web/src/lib/pdf/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D69** (keep apps/web/src/main.tsx): apps/web/src/main.tsx stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D70** (keep apps/web/src/notifications/): apps/web/src/notifications/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D71** (keep apps/web/src/onboarding/): apps/web/src/onboarding/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D72** (keep apps/web/src/onboarding/stage/): apps/web/src/onboarding/stage/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D73** (keep apps/web/src/onboarding/steps/): apps/web/src/onboarding/steps/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D74** (keep apps/web/src/projectImport/): apps/web/src/projectImport/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D75** (keep apps/web/src/repoDiffScopeStore.ts): apps/web/src/repoDiffScopeStore.ts stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D76** (keep apps/web/src/router.ts): apps/web/src/router.ts stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D77** (keep apps/web/src/routes/): apps/web/src/routes/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D78** (keep apps/web/src/store.ts): apps/web/src/store.ts stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D79** (keep apps/web/src/threadDetailPrewarm.ts): apps/web/src/threadDetailPrewarm.ts stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D80** (keep apps/web/src/threadDetailSubscriptionRetention.ts): apps/web/src/threadDetailSubscriptionRetention.ts stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D81** (keep apps/web/src/whatsNew/): apps/web/src/whatsNew/ stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D82** (keep packages/profile-ui/src/avatar.tsx): packages/profile-ui/src/avatar.tsx stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D83** (keep packages/profile-ui/src/heatmap.tsx): packages/profile-ui/src/heatmap.tsx stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D84** (keep packages/profile-ui/src/logo.tsx): packages/profile-ui/src/logo.tsx stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D85** (keep packages/profile-ui/src/providerIcon.tsx): packages/profile-ui/src/providerIcon.tsx stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D86** (keep packages/profile-ui/src/sections.tsx): packages/profile-ui/src/sections.tsx stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D87** (keep packages/profile-ui/src/skeleton.tsx): packages/profile-ui/src/skeleton.tsx stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D88** (keep packages/profile-ui/src/socialIcon.tsx): packages/profile-ui/src/socialIcon.tsx stays in the source language (UI code: keep in the original language). What replaces it in the Rust app, or does it stay outside?

> Keep it outside the Rust app, behind the front door, and decide its replacement once the rest is Rust

**D89** (blocked apps/desktop/scripts/): apps/desktop/scripts/ is blocked: Uses dynamic code (eval, new Function or computed imports). Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D90** (blocked apps/server/src/provider/): apps/server/src/provider/ is blocked: Uses dynamic code (eval, new Function or computed imports). Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D91** (blocked apps/web/scripts/): apps/web/scripts/ is blocked: `vite` has no Rust equivalent. Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D92** (blocked scripts/lib/): scripts/lib/ is blocked: Uses dynamic code (eval, new Function or computed imports). Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D93** (blocked scripts/release-smoke.ts): scripts/release-smoke.ts is blocked: Uses dynamic code (eval, new Function or computed imports). Rework that part in Rust, or keep the unit outside?

> Research the blocker first; then port it with the piece that uses it

**D94** (dependency vite): No crate does what vite does (used by 1 unit; Dev server or UI framework: keep in JS). Reimplement what the app uses, or drop the feature?

> Reimplement only the part the app uses, proved by its recorded cases

**D95** (dependency effect): Which crate replaces effect (used by 300 units)?

> Research it against primary sources into rust/research/effect.md before porting what uses it

**D96** (dependency zustand): Which crate replaces zustand (used by 49 units)?

> Research it against primary sources into rust/research/zustand.md before porting what uses it

**D97** (dependency vitest): Which crate replaces vitest (used by 27 units)?

> Research it against primary sources into rust/research/vitest.md before porting what uses it

**D98** (dependency @tanstack/react-query): Which crate replaces @tanstack/react-query (used by 26 units)?

> Research it against primary sources into rust/research/@tanstack-react-query.md before porting what uses it

**D99** (dependency vitest-browser-react): Which crate replaces vitest-browser-react (used by 22 units)?

> Research it against primary sources into rust/research/vitest-browser-react.md before porting what uses it

**D100** (dependency @tanstack/react-router): Which crate replaces @tanstack/react-router (used by 19 units)?

> Research it against primary sources into rust/research/@tanstack-react-router.md before porting what uses it

**D101** (dependency electron): Which crate replaces electron (used by 17 units)?

> Research it against primary sources into rust/research/electron.md before porting what uses it

**D102** (dependency @effect/platform-node): Which crate replaces @effect/platform-node (used by 16 units)?

> Research it against primary sources into rust/research/@effect-platform-node.md before porting what uses it

**D103** (dependency next): Which crate replaces next (used by 16 units)?

> Research it against primary sources into rust/research/next.md before porting what uses it

**D104** (dependency jose): Which crate replaces jose (used by 11 units)?

> Research it against primary sources into rust/research/jose.md before porting what uses it

**D105** (dependency react-dom): Which crate replaces react-dom (used by 11 units)?

> Research it against primary sources into rust/research/react-dom.md before porting what uses it

**D106** (dependency react-icons): Which crate replaces react-icons (used by 10 units)?

> Research it against primary sources into rust/research/react-icons.md before porting what uses it

**D107** (dependency @dnd-kit/sortable): Which crate replaces @dnd-kit/sortable (used by 6 units)?

> Research it against primary sources into rust/research/@dnd-kit-sortable.md before porting what uses it

**D108** (dependency @pierre/diffs): Which crate replaces @pierre/diffs (used by 6 units)?

> Research it against primary sources into rust/research/@pierre-diffs.md before porting what uses it

**D109** (dependency @tanstack/react-pacer): Which crate replaces @tanstack/react-pacer (used by 6 units)?

> Research it against primary sources into rust/research/@tanstack-react-pacer.md before porting what uses it

**D110** (dependency class-variance-authority): Which crate replaces class-variance-authority (used by 5 units)?

> Research it against primary sources into rust/research/class-variance-authority.md before porting what uses it

**D111** (dependency drizzle-orm): Which crate replaces drizzle-orm (used by 5 units)?

> Research it against primary sources into rust/research/drizzle-orm.md before porting what uses it

**D112** (dependency @base-ui/react): Which crate replaces @base-ui/react (used by 4 units)?

> Research it against primary sources into rust/research/@base-ui-react.md before porting what uses it

**D113** (dependency @dnd-kit/core): Which crate replaces @dnd-kit/core (used by 4 units)?

> Research it against primary sources into rust/research/@dnd-kit-core.md before porting what uses it

**D114** (dependency @dnd-kit/utilities): Which crate replaces @dnd-kit/utilities (used by 4 units)?

> Research it against primary sources into rust/research/@dnd-kit-utilities.md before porting what uses it

**D115** (dependency fumadocs-ui): Which crate replaces fumadocs-ui (used by 4 units)?

> Research it against primary sources into rust/research/fumadocs-ui.md before porting what uses it

**D116** (dependency @anthropic-ai/claude-agent-sdk): Which crate replaces @anthropic-ai/claude-agent-sdk (used by 3 units)?

> Research it against primary sources into rust/research/@anthropic-ai-claude-agent-sdk.md before porting what uses it

**D117** (dependency @dnd-kit/modifiers): Which crate replaces @dnd-kit/modifiers (used by 3 units)?

> Research it against primary sources into rust/research/@dnd-kit-modifiers.md before porting what uses it

**D118** (dependency @hono/node-server): Which crate replaces @hono/node-server (used by 3 units)?

> Research it against primary sources into rust/research/@hono-node-server.md before porting what uses it

**D119** (dependency betterwright): Which crate replaces betterwright (used by 3 units)?

> Research it against primary sources into rust/research/betterwright.md before porting what uses it

**D120** (dependency tailwind-merge): Which crate replaces tailwind-merge (used by 3 units)?

> Research it against primary sources into rust/research/tailwind-merge.md before porting what uses it

**D121** (dependency @agentclientprotocol/sdk): Which crate replaces @agentclientprotocol/sdk (used by 2 units)?

> Research it against primary sources into rust/research/@agentclientprotocol-sdk.md before porting what uses it

**D122** (dependency @legendapp/list): Which crate replaces @legendapp/list (used by 2 units)?

> Research it against primary sources into rust/research/@legendapp-list.md before porting what uses it

**D123** (dependency @modelcontextprotocol/sdk): Which crate replaces @modelcontextprotocol/sdk (used by 2 units)?

> Research it against primary sources into rust/research/@modelcontextprotocol-sdk.md before porting what uses it

**D124** (dependency @xterm/headless): Which crate replaces @xterm/headless (used by 2 units)?

> Research it against primary sources into rust/research/@xterm-headless.md before porting what uses it

**D125** (dependency i18next): Which crate replaces i18next (used by 2 units)?

> Research it against primary sources into rust/research/i18next.md before porting what uses it

**D126** (dependency lexical): Which crate replaces lexical (used by 2 units)?

> Research it against primary sources into rust/research/lexical.md before porting what uses it

**D127** (dependency mdast-util-from-markdown): Which crate replaces mdast-util-from-markdown (used by 2 units)?

> Research it against primary sources into rust/research/mdast-util-from-markdown.md before porting what uses it

**D128** (dependency msw): Which crate replaces msw (used by 2 units)?

> Research it against primary sources into rust/research/msw.md before porting what uses it

**D129** (dependency node-pty): Which crate replaces node-pty (used by 2 units)?

> Research it against primary sources into rust/research/node-pty.md before porting what uses it

**D130** (dependency open): Which crate replaces open (used by 2 units)?

> Research it against primary sources into rust/research/open.md before porting what uses it

**D131** (dependency pg): Which crate replaces pg (used by 2 units)?

> Research it against primary sources into rust/research/pg.md before porting what uses it

**D132** (dependency playwright): Which crate replaces playwright (used by 2 units)?

> Research it against primary sources into rust/research/playwright.md before porting what uses it

**D133** (dependency @cloudflare/containers): Which crate replaces @cloudflare/containers (used by 1 unit)?

> Research it against primary sources into rust/research/@cloudflare-containers.md before porting what uses it

**D134** (dependency @effect/sql-sqlite-bun): Which crate replaces @effect/sql-sqlite-bun (used by 1 unit)?

> Research it against primary sources into rust/research/@effect-sql-sqlite-bun.md before porting what uses it

**D135** (dependency @fontsource-variable/jetbrains-mono): Which crate replaces @fontsource-variable/jetbrains-mono (used by 1 unit)?

> Research it against primary sources into rust/research/@fontsource-variable-jetbrains-mono.md before porting what uses it

**D136** (dependency @formkit/auto-animate): Which crate replaces @formkit/auto-animate (used by 1 unit)?

> Research it against primary sources into rust/research/@formkit-auto-animate.md before porting what uses it

**D137** (dependency @lexical/react): Which crate replaces @lexical/react (used by 1 unit)?

> Research it against primary sources into rust/research/@lexical-react.md before porting what uses it

**D138** (dependency @opencode-ai/sdk): Which crate replaces @opencode-ai/sdk (used by 1 unit)?

> Research it against primary sources into rust/research/@opencode-ai-sdk.md before porting what uses it

**D139** (dependency @opencode/client): Which crate replaces @opencode/client (used by 1 unit)?

> Research it against primary sources into rust/research/@opencode-client.md before porting what uses it

**D140** (dependency @opennextjs/cloudflare): Which crate replaces @opennextjs/cloudflare (used by 1 unit)?

> Research it against primary sources into rust/research/@opennextjs-cloudflare.md before porting what uses it

**D141** (dependency @peculiar/x509): Which crate replaces @peculiar/x509 (used by 1 unit)?

> Research it against primary sources into rust/research/@peculiar-x509.md before porting what uses it

**D142** (dependency @playwright/test): Which crate replaces @playwright/test (used by 1 unit)?

> Research it against primary sources into rust/research/@playwright-test.md before porting what uses it

**D143** (dependency @polar-sh/sdk): Which crate replaces @polar-sh/sdk (used by 1 unit)?

> Research it against primary sources into rust/research/@polar-sh-sdk.md before porting what uses it

**D144** (dependency @rolldown/plugin-babel): Which crate replaces @rolldown/plugin-babel (used by 1 unit)?

> Research it against primary sources into rust/research/@rolldown-plugin-babel.md before porting what uses it

**D145** (dependency @tabler/icons-react): Which crate replaces @tabler/icons-react (used by 1 unit)?

> Research it against primary sources into rust/research/@tabler-icons-react.md before porting what uses it

**D146** (dependency @tanstack/react-virtual): Which crate replaces @tanstack/react-virtual (used by 1 unit)?

> Research it against primary sources into rust/research/@tanstack-react-virtual.md before porting what uses it

**D147** (dependency @vitejs/plugin-react): Which crate replaces @vitejs/plugin-react (used by 1 unit)?

> Research it against primary sources into rust/research/@vitejs-plugin-react.md before porting what uses it

**D148** (dependency @xterm/addon-clipboard): Which crate replaces @xterm/addon-clipboard (used by 1 unit)?

> Research it against primary sources into rust/research/@xterm-addon-clipboard.md before porting what uses it

**D149** (dependency @xterm/addon-fit): Which crate replaces @xterm/addon-fit (used by 1 unit)?

> Research it against primary sources into rust/research/@xterm-addon-fit.md before porting what uses it

**D150** (dependency @xterm/addon-image): Which crate replaces @xterm/addon-image (used by 1 unit)?

> Research it against primary sources into rust/research/@xterm-addon-image.md before porting what uses it

**D151** (dependency @xterm/addon-ligatures): Which crate replaces @xterm/addon-ligatures (used by 1 unit)?

> Research it against primary sources into rust/research/@xterm-addon-ligatures.md before porting what uses it

**D152** (dependency @xterm/addon-search): Which crate replaces @xterm/addon-search (used by 1 unit)?

> Research it against primary sources into rust/research/@xterm-addon-search.md before porting what uses it

**D153** (dependency @xterm/addon-unicode11): Which crate replaces @xterm/addon-unicode11 (used by 1 unit)?

> Research it against primary sources into rust/research/@xterm-addon-unicode11.md before porting what uses it

**D154** (dependency @xterm/addon-webgl): Which crate replaces @xterm/addon-webgl (used by 1 unit)?

> Research it against primary sources into rust/research/@xterm-addon-webgl.md before porting what uses it

**D155** (dependency @xterm/xterm): Which crate replaces @xterm/xterm (used by 1 unit)?

> Research it against primary sources into rust/research/@xterm-xterm.md before porting what uses it

**D156** (dependency clsx): Which crate replaces clsx (used by 1 unit)?

> Research it against primary sources into rust/research/clsx.md before porting what uses it

**D157** (dependency echarts): Which crate replaces echarts (used by 1 unit)?

> Research it against primary sources into rust/research/echarts.md before porting what uses it

**D158** (dependency electron-updater): Which crate replaces electron-updater (used by 1 unit)?

> Research it against primary sources into rust/research/electron-updater.md before porting what uses it

**D159** (dependency html-to-image): Which crate replaces html-to-image (used by 1 unit)?

> Research it against primary sources into rust/research/html-to-image.md before porting what uses it

**D160** (dependency import-meta-resolve): Which crate replaces import-meta-resolve (used by 1 unit)?

> Research it against primary sources into rust/research/import-meta-resolve.md before porting what uses it

**D161** (dependency katex): Which crate replaces katex (used by 1 unit)?

> Research it against primary sources into rust/research/katex.md before porting what uses it

**D162** (dependency lucide-react): Which crate replaces lucide-react (used by 1 unit)?

> Research it against primary sources into rust/research/lucide-react.md before porting what uses it

**D163** (dependency micromark-util-decode-string): Which crate replaces micromark-util-decode-string (used by 1 unit)?

> Research it against primary sources into rust/research/micromark-util-decode-string.md before porting what uses it

**D164** (dependency monaco-editor): Which crate replaces monaco-editor (used by 1 unit)?

> Research it against primary sources into rust/research/monaco-editor.md before porting what uses it

**D165** (dependency original-fs): Which crate replaces original-fs (used by 1 unit)?

> Research it against primary sources into rust/research/original-fs.md before porting what uses it

**D166** (dependency pdfjs-dist): Which crate replaces pdfjs-dist (used by 1 unit)?

> Research it against primary sources into rust/research/pdfjs-dist.md before porting what uses it

**D167** (dependency react-colorful): Which crate replaces react-colorful (used by 1 unit)?

> Research it against primary sources into rust/research/react-colorful.md before porting what uses it

**D168** (dependency react-i18next): Which crate replaces react-i18next (used by 1 unit)?

> Research it against primary sources into rust/research/react-i18next.md before porting what uses it

**D169** (dependency react-markdown): Which crate replaces react-markdown (used by 1 unit)?

> Research it against primary sources into rust/research/react-markdown.md before porting what uses it

**D170** (dependency react-tweet): Which crate replaces react-tweet (used by 1 unit)?

> Research it against primary sources into rust/research/react-tweet.md before porting what uses it

**D171** (dependency reflect-metadata): Which crate replaces reflect-metadata (used by 1 unit)?

> Research it against primary sources into rust/research/reflect-metadata.md before porting what uses it

**D172** (dependency rehype-katex): Which crate replaces rehype-katex (used by 1 unit)?

> Research it against primary sources into rust/research/rehype-katex.md before porting what uses it

**D173** (dependency rehype-raw): Which crate replaces rehype-raw (used by 1 unit)?

> Research it against primary sources into rust/research/rehype-raw.md before porting what uses it

**D174** (dependency rehype-sanitize): Which crate replaces rehype-sanitize (used by 1 unit)?

> Research it against primary sources into rust/research/rehype-sanitize.md before porting what uses it

**D175** (dependency remark-breaks): Which crate replaces remark-breaks (used by 1 unit)?

> Research it against primary sources into rust/research/remark-breaks.md before porting what uses it

**D176** (dependency remark-gfm): Which crate replaces remark-gfm (used by 1 unit)?

> Research it against primary sources into rust/research/remark-gfm.md before porting what uses it

**D177** (dependency remark-math): Which crate replaces remark-math (used by 1 unit)?

> Research it against primary sources into rust/research/remark-math.md before porting what uses it

**D178** (dependency resend): Which crate replaces resend (used by 1 unit)?

> Research it against primary sources into rust/research/resend.md before porting what uses it

**D179** (dependency server-only): Which crate replaces server-only (used by 1 unit)?

> Research it against primary sources into rust/research/server-only.md before porting what uses it

**D180** (dependency uqr): Which crate replaces uqr (used by 1 unit)?

> Research it against primary sources into rust/research/uqr.md before porting what uses it

**D181** (dependency wait-on): Which crate replaces wait-on (used by 1 unit)?

> Research it against primary sources into rust/research/wait-on.md before porting what uses it

**D182** (dependency typescript): typescript maps to oxc only in part (used by 4 units; Parsing is covered. Type checking has no Rust equivalent). Use it?

> Use oxc, and record cases for the behavior it does not cover

**D183** (dependency hono): hono maps to axum only in part (used by 3 units; Middleware ports by hand). Use it?

> Use axum, and record cases for the behavior it does not cover

**D184** (dependency yaml): yaml maps to saphyr, serde_yaml_ng only in part (used by 1 unit; Parsing is covered. No crate rewrites YAML while keeping comments and layout). Use it?

> Use saphyr, serde_yaml_ng, and record cases for the behavior it does not cover

**D185** (dependency zod): zod maps to serde, garde only in part (used by 1 unit; Schemas become types plus validation; error messages will differ). Use it?

> Use serde, garde, and record cases for the behavior it does not cover

**D186** (tangled units): apps/api/src/config.ts, apps/api/src/identity/, apps/api/src/testing/ import each other (4647 lines). Port them as one piece?

> Yes: one piece in wave 1, no glue between its parts

**D187** (tangled units): packages/contracts/src/agentMentions.ts, packages/contracts/src/model.ts, packages/contracts/src/orchestration.ts, packages/contracts/src/providerDiscovery.ts import each other (5256 lines). Port them as one piece?

> Yes: one piece in wave 3, no glue between its parts

**D188** (tangled units): apps/web/src/splitView.logic.ts, apps/web/src/splitViewStore.ts import each other (932 lines). Port them as one piece?

> Yes: one piece in wave 4, no glue between its parts

**D189** (tangled units): apps/server/src/accountAuth.ts, apps/server/src/accountCredentialLock.ts, apps/server/src/accountDeviceRegistration.ts, apps/server/src/accountRpcErrors.ts, apps/server/src/accountSession.ts, apps/server/src/accountSignInTiming.ts, apps/server/src/agentGateway/, apps/server/src/agentGateway/Layers/, apps/server/src/agentGateway/Services/, apps/server/src/assistantImageAttachments.ts, apps/server/src/auth/, apps/server/src/auth/Layers/, apps/server/src/checkpointing/, apps/server/src/checkpointing/Services/, apps/server/src/codexAppServerManager.ts, apps/server/src/codexProcessEnv.ts, apps/server/src/computer/, apps/server/src/computer/Services/, apps/server/src/desktopUpdateRpc.ts, apps/server/src/devServerManager.ts, apps/server/src/device/, apps/server/src/device/Services/, apps/server/src/deviceIdentity/, apps/server/src/diagnostics/, apps/server/src/diagnostics/Services/, apps/server/src/editor/, apps/server/src/externalMcp/, apps/server/src/externalMcp/Services/, apps/server/src/git/, apps/server/src/git/Layers/, apps/server/src/git/Services/, apps/server/src/githubInbox/, apps/server/src/githubInbox/Services/, apps/server/src/hostConnections/, apps/server/src/hostSecrets/, apps/server/src/keepAwake.ts, apps/server/src/localImageFiles.ts, apps/server/src/managedAttachmentStore.ts, apps/server/src/managedWorktrees.ts, apps/server/src/mcpConnectors/, apps/server/src/mcpConnectors/Layers/, apps/server/src/mcpConnectors/Services/, apps/server/src/orchestration/, apps/server/src/orchestration/Layers/, apps/server/src/orchestration/Services/, apps/server/src/persistence/, apps/server/src/persistence/Layers/, apps/server/src/persistence/Migrations/, apps/server/src/persistence/Services/, apps/server/src/platform/, apps/server/src/processRunner.ts, apps/server/src/profileStatsArchive.ts, apps/server/src/project/, apps/server/src/projectAgent/, apps/server/src/projectAgent/Services/, apps/server/src/provider/, apps/server/src/provider/Layers/, apps/server/src/provider/Services/, apps/server/src/provider/acp/, apps/server/src/providerCredentials.ts, apps/server/src/providerProcessPriority.ts, apps/server/src/providerUsage/, apps/server/src/providerUsage/providers/, apps/server/src/pullRequests.logic.ts, apps/server/src/pullRequests/, apps/server/src/remotePairing/, apps/server/src/remoteSessions/, apps/server/src/remoteTransport/, apps/server/src/serverRuntimeState.ts, apps/server/src/serverSettings.ts, apps/server/src/studioGeneratedImages.ts, apps/server/src/terminal/, apps/server/src/terminal/Layers/, apps/server/src/terminal/Services/, apps/server/src/threadRetention.ts, apps/server/src/threadSearch.ts, apps/server/src/workspace/, apps/server/src/workspace/Layers/, apps/server/src/workspaceEntries.ts, apps/server/src/workspaceFileChanges.ts, apps/server/src/worktreeSetup.ts, apps/server/src/wsAccountRpc.ts, apps/server/src/wsConnectionSessions.ts, apps/server/src/wsHostsRpc.ts, apps/server/src/wsOwnerAuthorization.ts, apps/server/src/wsOwnerOnly.ts, apps/server/src/wsRequestAdmission.ts, apps/server/src/wsRpc.ts import each other (263978 lines). Port them as one piece?

> Yes: one piece in wave 5, no glue between its parts
