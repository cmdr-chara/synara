# Synara WebSocket RPC method table

Extracted 2026-11-22 from `packages/contracts/src/rpc.ts` (`Rpc.make` declarations, 285 methods).
Transport: `apps/server/src/wsRpc.ts` over the WS upgrade path; this is the app's real API surface — the 30 HTTP routes in ROUTES.md are auxiliary.

Native coverage in rust/app (relay handles the rest): ~102/285.
- `bootstrap.negotiate` (HTTP GET /ws/negotiate)
- `filesystem.browse`
- `git.*` — 20 of 27 (missing: summarizeDiff, runStackedAction, createDetachedWorktree, resolvePullRequest, pullRequestSnapshot, preparePullRequestThread, handoffThread)
- `githubInbox.*` — 3 of 3: list (GraphQL inbox + involvement docs via gh), issueDetail, issueComment; repositories resolved from projection projects' git remotes
- `orchestration.*` — 4 of 27: getSnapshot, getShellSnapshot (heavy embeds stripped + latestReplyPreview), getThreadDetailSnapshot, searchThreads (LIKE over projection_thread_messages) — all read from a byte copy of state.sqlite
- `projects.*` — 16 of 17 + subscribeDevServerEvents/subscribeFileChange streams (missing: provisionFromGitHub)
- `server.*` — 19 of 39: getEnvironment, getRuntimeStatus, getConfig, getSettings, updateSettings, subscribeSettings, subscribeConfig, subscribeProviderStatuses (empty until providers ported), subscribeLifecycle, subscribeKeepAwake, listWorktrees, upsertKeybinding, editKeybindings, listLocalServers, stopLocalServer, getDiagnostics, desktopUpdateStatus, desktopUpdate (unsupported: no updater bridge)
- `shell.openInEditor`
- `stats.*` — 4 of 4: getProfileStats, getProfileTokenStats, getRecap, getUsageAnalytics (computed over a byte copy of state.sqlite; quota/pricing unavailable)
- `terminal.*` — 8 of 8 (incl. subscribeEvents stream)
- `todo.*` — 5 of 5 (incl. subscribe stream)

Deferred with reasons: orchestration/automation/pullRequests/projectAgent/studio contracts or impls are T3-derived (cases-only rule); account/hosts/device need the remote pairing authority; provider.*/getConfig need the provider CLI matrix; readThreadDiagnostics/providerUsage are impl-flagged; voice/generate*/transcribe need the provider runtime; editor.* needs the editor runtime.



## account (19)

`account.authenticateOtp`, `account.beginSso`, `account.billingCheckout`, `account.billingPortal`, `account.billingStatus`, `account.cancelSso`, `account.completeSso`, `account.delete`, `account.deleteAvatar`, `account.deleteInboxRecap`, `account.listInboxRecaps`, `account.openVerificationUrl`, `account.saveInboxRecap`, `account.sendOtp`, `account.signOut`, `account.status`, `account.updateProfile`, `account.uploadAvatar`, `account.usageSummary`

## agentGateway (1)

`agentGateway.call`

## automation (11)

`automation.archiveRun`, `automation.cancelRun`, `automation.create`, `automation.delete`, `automation.getMemory`, `automation.list`, `automation.markRunRead`, `automation.resolveProposal`, `automation.runNow`, `automation.subscribe`, `automation.update`

## bootstrap (1)

`bootstrap.negotiate`

## computer (25)

`computer.click`, `computer.doubleClick`, `computer.drag`, `computer.getAuditHistory`, `computer.getScreenSize`, `computer.getState`, `computer.getStatus`, `computer.getThreadState`, `computer.hotkey`, `computer.input.click`, `computer.input.key`, `computer.input.scroll`, `computer.launchApp`, `computer.listWindows`, `computer.moveCursor`, `computer.performAction`, `computer.pressKey`, `computer.provision`, `computer.rightClick`, `computer.scroll`, `computer.selectText`, `computer.setControlEnabled`, `computer.setValue`, `computer.subscribeEvents`, `computer.typeText`

## device (20)

`device.attach`, `device.boot`, `device.describeUi`, `device.detach`, `device.getThreadState`, `device.installApp`, `device.keyEvent`, `device.launchApp`, `device.list`, `device.openUrl`, `device.pressButton`, `device.screenshot`, `device.scrollToElement`, `device.shutdown`, `device.startRecording`, `device.stopRecording`, `device.subscribeEvents`, `device.swipe`, `device.tap`, `device.typeText`

## editor (4)

`editor.debug.action`, `editor.debug.poll`, `editor.debug.start`, `editor.intelligence`

## filesystem (1)

`filesystem.browse`

## git (27)

`git.blameLine`, `git.checkout`, `git.createBranch`, `git.createDetachedWorktree`, `git.createWorktree`, `git.githubRepository`, `git.handoffThread`, `git.init`, `git.listBranches`, `git.listRecentCommits`, `git.preparePullRequestThread`, `git.pull`, `git.pullRequestSnapshot`, `git.readFileAtRev`, `git.readWorkingTreeDiff`, `git.removeIndexLock`, `git.removeWorktree`, `git.resolvePullRequest`, `git.runStackedAction`, `git.stageFiles`, `git.stashAndCheckout`, `git.stashDrop`, `git.stashInfo`, `git.status`, `git.summarizeDiff`, `git.unstageFiles`, `git.workingTreeDiffStats`

## githubInbox (3)

`githubInbox.issueComment`, `githubInbox.issueDetail`, `githubInbox.list`

## hosts (19)

`hosts.approveDeviceLink`, `hosts.beginSyncKeyPairing`, `hosts.confirmSyncKey`, `hosts.connect`, `hosts.delete`, `hosts.disconnect`, `hosts.endSession`, `hosts.enrollment`, `hosts.list`, `hosts.listConnections`, `hosts.listDevices`, `hosts.listSessions`, `hosts.offerSyncKey`, `hosts.receiveSyncKey`, `hosts.remoteAccess`, `hosts.requestGrant`, `hosts.revokeDevice`, `hosts.unlinkLocalHost`, `hosts.update`

## orchestration (24)

`orchestration.dispatchCommand`, `orchestration.getFullThreadDiff`, `orchestration.getInlineVisual`, `orchestration.getShellSnapshot`, `orchestration.getSnapshot`, `orchestration.getThreadDetailSnapshot`, `orchestration.getTurnDiff`, `orchestration.importProject`, `orchestration.importThread`, `orchestration.listProjectImports`, `orchestration.listProviderDeliveryBlockers`, `orchestration.loadProjectImportHistory`, `orchestration.prepareQuitResume`, `orchestration.reconcileProviderDelivery`, `orchestration.regenerateThreadTitle`, `orchestration.repairState`, `orchestration.replayEvents`, `orchestration.searchThreads`, `orchestration.settleTurnDispatch`, `orchestration.subscribeDomainEvents`, `orchestration.subscribeShell`, `orchestration.subscribeThread`, `orchestration.unsubscribeShell`, `orchestration.unsubscribeThread`

## projectAgent (38)

`projectAgent.archive`, `projectAgent.backfillSummaries`, `projectAgent.configure`, `projectAgent.createTask`, `projectAgent.delete`, `projectAgent.excludeThread`, `projectAgent.exportDocuments`, `projectAgent.getOverview`, `projectAgent.library.delete`, `projectAgent.library.history`, `projectAgent.library.list`, `projectAgent.library.mkdir`, `projectAgent.library.rename`, `projectAgent.library.restore`, `projectAgent.library.status`, `projectAgent.linkProject`, `projectAgent.listActivity`, `projectAgent.listDocuments`, `projectAgent.listEvidence`, `projectAgent.listSummaries`, `projectAgent.listTasks`, `projectAgent.listThreadIndex`, `projectAgent.pause`, `projectAgent.pauseGoal`, `projectAgent.readDocument`, `projectAgent.refreshDigest`, `projectAgent.resolveWorker`, `projectAgent.restartCoordinator`, `projectAgent.resume`, `projectAgent.resumeGoal`, `projectAgent.startGoal`, `projectAgent.stopGoal`, `projectAgent.subscribe`, `projectAgent.unarchive`, `projectAgent.unlinkProject`, `projectAgent.updateGoal`, `projectAgent.updateTask`, `projectAgent.writeDocument`

## projects (17)

`projects.createLocalFilePreviewGrant`, `projects.discoverScripts`, `projects.listDevServers`, `projects.listDirectories`, `projects.prewarmSearchIndex`, `projects.provisionFromGitHub`, `projects.readFile`, `projects.resolveOutOfRootFileReference`, `projects.resolveWorkspaceFileReferences`, `projects.runDevServer`, `projects.searchContent`, `projects.searchEntries`, `projects.searchLocalEntries`, `projects.stopDevServer`, `projects.subscribeDevServerEvents`, `projects.subscribeFileChange`, `projects.writeFile`

## provider (10)

`provider.compactThread`, `provider.getComposerCapabilities`, `provider.listAgents`, `provider.listCommands`, `provider.listModels`, `provider.listPlugins`, `provider.listSkills`, `provider.listSkillsCatalog`, `provider.prewarmThread`, `provider.readPlugin`

## pullRequests (7)

`pullRequests.action`, `pullRequests.comment`, `pullRequests.detail`, `pullRequests.diff`, `pullRequests.getAutoFix`, `pullRequests.setAutoFix`, `pullRequests.setPinned`

## server (39)

`server.connectMcpConnector`, `server.consumeCodexResetCredit`, `server.createExternalMcpIntegration`, `server.desktopUpdate`, `server.desktopUpdateStatus`, `server.disconnectMcpConnector`, `server.editKeybindings`, `server.generateAutomationIntent`, `server.generateComposerPrediction`, `server.generateThreadRecap`, `server.getConfig`, `server.getDiagnostics`, `server.getEnvironment`, `server.getMcpConnectorSetup`, `server.getProviderUsageSnapshot`, `server.getRuntimeStatus`, `server.getSettings`, `server.listExternalMcpIntegrations`, `server.listLocalServers`, `server.listMcpConnectorProposals`, `server.listMcpConnectors`, `server.listProviderUsage`, `server.listWorktrees`, `server.prewarmVoice`, `server.readThreadDiagnostics`, `server.refreshExternalMcpPairing`, `server.refreshProviders`, `server.resolveMcpConnectorProposal`, `server.revokeExternalMcpIntegration`, `server.stopLocalServer`, `server.subscribeConfig`, `server.subscribeKeepAwake`, `server.subscribeLifecycle`, `server.subscribeProviderStatuses`, `server.subscribeSettings`, `server.transcribeVoice`, `server.updateProvider`, `server.updateSettings`, `server.upsertKeybinding`

## shell (1)

`shell.openInEditor`

## stats (4)

`stats.getProfileStats`, `stats.getProfileTokenStats`, `stats.getRecap`, `stats.getUsageAnalytics`

## studio (1)

`studio.listThreadOutputs`

## terminal (8)

`terminal.ackOutput`, `terminal.clear`, `terminal.close`, `terminal.open`, `terminal.resize`, `terminal.restart`, `terminal.subscribeEvents`, `terminal.write`

## todo (5)

`todo.create`, `todo.delete`, `todo.list`, `todo.subscribe`, `todo.update`
