# T3 Code provenance audit — Synara Rust port (`rust/`)

**Goal:** verify that the Rust rewrite of Synara (`rust/` tree) contains no code derived from
[`pingdotgg/t3code`](https://github.com/pingdotgg/t3code). Read-only audit; no code changed.

## TL;DR

- **6 committed Rust ports are line-derived from T3-authored code** and should be rewritten
  cases-only per ADR-002: `worklog-rs`, `keybindingrules-rs`, `shell-rs`, `storeeventreducer-rs`,
  `terminal-links-rs`, `markdown-links-rs`.
- **15 further ports are suspect** (1–2 internal names shared with a t3code counterpart) — listed below.
- **25 ports of t3code-derived sources are clean** (zero shared internals/strings/comments),
  consistent with the cases-only method — incl. `proposedplan-rs` whose source is 90% T3-derived.
- **18 ported units are cua-imported** (MIT © Cua AI, Inc.) — no source-mirroring found, but
  `rust/LICENSE` credits only Di Pietro + cmdr-chara; it carries neither T3 nor Cua attribution.
- The TS tree itself remains heavily T3-derived: **140 files ≥50%** (15 at 100%), **156 at 30–50%**,
  and **243 more files <50% contain verbatim copied blocks** (largest: 172-line block,
  477 copied lines in `projector.test.ts`).

## Methodology

| Step | Detail |
|---|---|
| Baseline | `t3code` @ `d15210cd3da79f9a1a495a6309d912d76362a046` — matches the SHA pinned in `docs/implementation/cloudflare-remote/UPSTREAM-COMPATIBILITY.md` |
| Scope | 5,064 synara source files (`apps/`, `packages/`, `scripts/`, `benchmarks/`; `.ts/.tsx/.mjs/.cjs/.js/.swift/.sh`) vs 3,943 t3code files (whole repo) |
| Normalization | strip `//`/`/* */` (or `#`) comments → lowercase → brand-canonicalize (`synara`,`t3code`,`t3tools`,`t3 code`,`t3`,`.t3`→`b`, `t3.codes`→`b`) → collapse all whitespace; drop lines <4 alnum chars |
| Whole-file | inverted index of normalized line hashes → top-5 t3code candidates + same-path candidate → `difflib.SequenceMatcher.ratio()` on normalized line sequences |
| Moved/renamed | candidate selection is purely content-based (line-hash index), so path mismatches are found automatically; string constants function as matching lines |
| Sub-file | for files <50%: `get_matching_blocks()` ≥8 normalized lines vs each candidate |
| Logic-level | set overlap of exported/internal identifier names + string literals ≥14 chars, brand-normalized; flagged when line-ratio <0.3 but identifier Jaccard ≥0.25–0.4 |
| Rust clean-room check | per `rust/*-rs/src/port.rs`: internal `fn` names (minus exports listed in `rust-anything.json` and generic names), string literals ≥14 chars, comments ≥20 chars — compared against (a) the synara TS unit's internals, (b) the matched t3code counterpart's internals, (c) repo-wide t3code internal-name index (catches moved/merged origins) |

**Normalization is deliberately aggressive** (over-matching), so the derived counts are an upper bound;
sub-2% rows are mostly generic boilerplate, listed for completeness.

## 1. File-level results

| Band | Files |
|---|---:|
| 100% identical (normalized) | 15 |
| 50–99% | 125 |
| 30–50% | 156 |
| 10–30% | 482 |
| 2–10% | 2,652 |
| <2% | 1,531 |
| 0% (no shared lines) | 103 |

Complete ranked table of all 4334 files with any signal is in the appendix.

### 100% identical after normalization

All persistence migrations — verbatim T3 code:

| Synara file | Derived from |
|---|---|
| `apps/server/src/persistence/Migrations/001_OrchestrationEvents.ts` | `apps/server/src/persistence/Migrations/001_OrchestrationEvents.ts` |
| `apps/server/src/persistence/Migrations/002_OrchestrationCommandReceipts.ts` | `apps/server/src/persistence/Migrations/002_OrchestrationCommandReceipts.ts` |
| `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| `apps/server/src/persistence/Migrations/004_ProviderSessionRuntime.ts` | `apps/server/src/persistence/Migrations/004_ProviderSessionRuntime.ts` |
| `apps/server/src/persistence/Migrations/006_ProjectionThreadSessionRuntimeModeColumns.ts` | `apps/server/src/persistence/Migrations/006_ProjectionThreadSessionRuntimeModeColumns.ts` |
| `apps/server/src/persistence/Migrations/007_ProjectionThreadMessageAttachments.ts` | `apps/server/src/persistence/Migrations/007_ProjectionThreadMessageAttachments.ts` |
| `apps/server/src/persistence/Migrations/008_ProjectionThreadActivitySequence.ts` | `apps/server/src/persistence/Migrations/008_ProjectionThreadActivitySequence.ts` |
| `apps/server/src/persistence/Migrations/010_ProjectionThreadsRuntimeMode.ts` | `apps/server/src/persistence/Migrations/010_ProjectionThreadsRuntimeMode.ts` |
| `apps/server/src/persistence/Migrations/011_OrchestrationThreadCreatedRuntimeMode.ts` | `apps/server/src/persistence/Migrations/011_OrchestrationThreadCreatedRuntimeMode.ts` |
| `apps/server/src/persistence/Migrations/012_ProjectionThreadsInteractionMode.ts` | `apps/server/src/persistence/Migrations/012_ProjectionThreadsInteractionMode.ts` |
| `apps/server/src/persistence/Migrations/013_ProjectionThreadProposedPlans.ts` | `apps/server/src/persistence/Migrations/013_ProjectionThreadProposedPlans.ts` |
| `apps/server/src/persistence/Migrations/014_ProjectionThreadProposedPlanImplementation.ts` | `apps/server/src/persistence/Migrations/014_ProjectionThreadProposedPlanImplementation.ts` |
| `apps/server/src/persistence/Migrations/015_ProjectionTurnsSourceProposedPlan.ts` | `apps/server/src/persistence/Migrations/015_ProjectionTurnsSourceProposedPlan.ts` |
| `apps/server/src/persistence/Migrations/026_ProjectionThreadShellSummary.ts` | `apps/server/src/persistence/Migrations/023_ProjectionThreadShellSummary.ts` |
| `apps/server/src/persistence/Migrations/027_BackfillProjectionThreadShellSummary.ts` | `apps/server/src/persistence/Migrations/024_BackfillProjectionThreadShellSummary.ts` |

### ≥50% derived (excluding the 100% files above)

| Sim | Synara file | Derived from |
|---:|---|---|
| 100% | `apps/server/src/persistence/Migrations/005_Projections.ts` | `apps/server/src/persistence/Migrations/005_Projections.ts` |
| 99% | `apps/server/src/persistence/Migrations/016_CanonicalizeModelSelections.ts` | `apps/server/src/persistence/Migrations/016_CanonicalizeModelSelections.ts` |
| 97% | `apps/server/src/provider/providerMaintenanceCommandCoordinator.ts` | `apps/server/src/provider/providerMaintenanceCommandCoordinator.ts` |
| 97% | `apps/server/integration/fixtures/providerRuntime.ts` | `apps/server/integration/fixtures/providerRuntime.ts` |
| 95% | `apps/web/src/lib/lruCache.ts` | `apps/web/src/lib/lruCache.ts` |
| 93% | `apps/web/src/components/ui/label.tsx` | `apps/web/src/components/ui/label.tsx` |
| 91% | `apps/web/src/lib/projectScriptKeybindings.test.ts` | `apps/web/src/lib/projectScriptKeybindings.test.ts` |
| 91% | `apps/server/integration/TestProviderAdapter.integration.ts` | `apps/server/integration/TestProviderAdapter.integration.ts` |
| 90% | `apps/web/src/proposedPlan.ts` | `apps/web/src/proposedPlan.ts` |
| 89% | `apps/web/src/components/ui/collapsible.tsx` | `apps/web/src/components/ui/collapsible.tsx` |
| 88% | `apps/server/src/persistence/Migrations/034_AuthAccessManagement.ts` | `apps/server/src/persistence/Migrations/020_AuthAccessManagement.ts` |
| 87% | `apps/server/src/persistence/Services/ProjectionTurns.ts` | `apps/server/src/persistence/Services/ProjectionTurns.ts` |
| 86% | `apps/server/src/provider/acp/CursorAcpExtension.ts` | `apps/server/src/provider/acp/CursorAcpExtension.ts` |
| 86% | `apps/web/src/components/ui/textarea.tsx` | `apps/web/src/components/ui/textarea.tsx` |
| 84% | `apps/web/src/hooks/useMediaQuery.ts` | `apps/web/src/hooks/useMediaQuery.ts` |
| 83% | `apps/server/src/persistence/Migrations/016_CanonicalizeModelSelections.test.ts` | `apps/server/src/persistence/Migrations/016_CanonicalizeModelSelections.test.ts` |
| 82% | `apps/web/src/proposedPlan.test.ts` | `apps/web/src/proposedPlan.test.ts` |
| 82% | `apps/server/src/provider/acp/CursorAcpExtension.test.ts` | `apps/server/src/provider/acp/CursorAcpExtension.test.ts` |
| 82% | `apps/server/src/orchestration/Services/RuntimeReceiptBus.ts` | `apps/server/src/orchestration/Services/RuntimeReceiptBus.ts` |
| 82% | `apps/server/integration/orchestrationEngine.integration.test.ts` | `apps/server/integration/orchestrationEngine.integration.test.ts` |
| 81% | `apps/web/src/components/ui/empty.tsx` | `apps/web/src/components/ui/empty.tsx` |
| 80% | `apps/web/src/components/ui/autocomplete.tsx` | `apps/web/src/components/ui/autocomplete.tsx` |
| 80% | `apps/web/src/truncateTitle.ts` | `packages/shared/src/String.ts` |
| 78% | `apps/web/src/components/ui/badge.tsx` | `apps/web/src/components/ui/badge.tsx` |
| 76% | `apps/web/src/worktreeCleanup.ts` | `apps/web/src/worktreeCleanup.ts` |
| 76% | `packages/contracts/src/providerRuntime.ts` | `packages/contracts/src/providerRuntime.ts` |
| 76% | `apps/server/src/persistence/Services/ProjectionThreadActivities.ts` | `apps/server/src/persistence/Services/ProjectionThreadActivities.ts` |
| 76% | `apps/server/src/persistence/Services/ProjectionState.ts` | `apps/server/src/persistence/Services/ProjectionState.ts` |
| 76% | `apps/web/src/components/ui/separator.tsx` | `apps/web/src/components/ui/separator.tsx` |
| 76% | `apps/server/src/provider/acp/CursorAcpCliProbe.test.ts` | `apps/server/src/provider/acp/CursorAcpCliProbe.test.ts` |
| 76% | `apps/web/src/components/ui/toggle.tsx` | `apps/web/src/components/ui/toggle.tsx` |
| 76% | `apps/web/src/terminal-links.ts` | `apps/web/src/terminal-links.ts` |
| 76% | `apps/server/src/provider/acp/AcpCoreRuntimeEvents.ts` | `apps/server/src/provider/acp/AcpCoreRuntimeEvents.ts` |
| 75% | `apps/marketing/src/lib/utils.ts` | `apps/mobile/src/lib/cn.ts` |
| 75% | `apps/server/src/persistence/Migrations/042_ProjectionThreadsMarkers.ts` | `apps/server/src/persistence/Migrations/012_ProjectionThreadsInteractionMode.ts` |
| 75% | `apps/server/src/persistence/Migrations/094_ProjectionThreadsGoal.ts` | `apps/server/src/persistence/Migrations/012_ProjectionThreadsInteractionMode.ts` |
| 75% | `apps/server/src/persistence/Migrations/096_ProjectionThreadsGoalAchievements.ts` | `apps/server/src/persistence/Migrations/012_ProjectionThreadsInteractionMode.ts` |
| 75% | `apps/server/src/persistence/Migrations/104_ProjectionThreadsClaudeCacheReview.ts` | `apps/server/src/persistence/Migrations/012_ProjectionThreadsInteractionMode.ts` |
| 75% | `apps/server/src/persistence/Migrations/126_ProjectionThreadsSidechatContext.ts` | `apps/server/src/persistence/Migrations/012_ProjectionThreadsInteractionMode.ts` |
| 74% | `apps/web/src/components/ui/alert-dialog.tsx` | `apps/web/src/components/ui/alert-dialog.tsx` |
| 74% | `apps/server/src/persistence/Layers/ProjectionTurns.ts` | `apps/server/src/persistence/Layers/ProjectionTurns.ts` |
| 73% | `scripts/lib/mac-update-manifest.ts` | `scripts/lib/update-manifest.ts` |
| 72% | `packages/contracts/src/providerInstance.ts` | `packages/contracts/src/providerInstance.ts` |
| 72% | `apps/server/src/persistence/Layers/OrchestrationCommandReceipts.ts` | `apps/server/src/persistence/Layers/OrchestrationCommandReceipts.ts` |
| 72% | `apps/server/src/persistence/Layers/ProjectionState.ts` | `apps/server/src/persistence/Layers/ProjectionState.ts` |
| 72% | `apps/desktop/src/updateMachine.ts` | `apps/desktop/src/updates/updateMachine.ts` |
| 71% | `apps/server/src/persistence/Migrations/036_ProjectionThreadsPinned.ts` | `apps/server/src/persistence/Migrations/012_ProjectionThreadsInteractionMode.ts` |
| 70% | `apps/web/src/lib/projectScriptKeybindings.ts` | `apps/web/src/lib/projectScriptKeybindings.ts` |
| 70% | `apps/web/src/components/ui/input-group.tsx` | `apps/web/src/components/ui/input-group.tsx` |
| 70% | `apps/server/src/persistence/Services/ProjectionThreadProposedPlans.ts` | `apps/server/src/persistence/Services/ProjectionThreadProposedPlans.ts` |
| 68% | `apps/web/src/components/ui/dialog.tsx` | `apps/web/src/components/ui/dialog.tsx` |
| 68% | `packages/shared/src/keybindingRules.ts` | `packages/shared/src/keybindings.ts` |
| 68% | `apps/web/src/components/ui/sheet.tsx` | `apps/web/src/components/ui/sheet.tsx` |
| 68% | `apps/server/src/provider/Services/ProviderSessionDirectory.ts` | `apps/server/src/provider/Services/ProviderSessionDirectory.ts` |
| 68% | `apps/web/src/components/ui/preview-card.tsx` | `apps/web/src/components/ui/preview-card.tsx` |
| 67% | `apps/server/src/persistence/Migrations/020_ProjectionThreadsForkSource.ts` | `apps/server/src/persistence/Migrations/012_ProjectionThreadsInteractionMode.ts` |
| 67% | `apps/server/src/persistence/Migrations/024_ProjectionThreadsArchivedAt.ts` | `apps/server/src/persistence/Migrations/012_ProjectionThreadsInteractionMode.ts` |
| 67% | `apps/server/src/git/PrTemplateDetection.test.ts` | `apps/server/src/sourceControl/PrTemplateDetection.test.ts` |
| 66% | `apps/web/src/components/ui/command.tsx` | `apps/web/src/components/ui/command.tsx` |
| 66% | `packages/shared/src/Net.test.ts` | `packages/shared/src/Net.test.ts` |
| 66% | `apps/server/src/persistence/Layers/ProjectionThreadProposedPlans.ts` | `apps/server/src/persistence/Layers/ProjectionThreadProposedPlans.ts` |
| 65% | `apps/server/src/persistence/Layers/ProjectionThreadSessions.ts` | `apps/server/src/persistence/Layers/ProjectionThreadSessions.ts` |
| 64% | `apps/server/src/orchestration/Schemas.ts` | `apps/server/src/orchestration/Schemas.ts` |
| 64% | `apps/web/src/lib/terminalFocus.test.ts` | `apps/web/src/lib/terminalFocus.test.ts` |
| 64% | `apps/server/src/provider/Errors.ts` | `apps/server/src/provider/Errors.ts` |
| 63% | `apps/server/src/persistence/Migrations/017_ThreadHandoffMetadata.ts` | `apps/server/src/persistence/Migrations/007_ProjectionThreadMessageAttachments.ts` |
| 63% | `apps/server/src/persistence/Migrations/018_ProjectionThreadMessageMentions.ts` | `apps/server/src/persistence/Migrations/007_ProjectionThreadMessageAttachments.ts` |
| 63% | `apps/web/src/components/ui/popover.tsx` | `apps/web/src/components/ui/popover.tsx` |
| 63% | `packages/contracts/src/provider.ts` | `packages/contracts/src/provider.ts` |
| 63% | `apps/server/src/persistence/Services/OrchestrationCommandReceipts.ts` | `apps/server/src/persistence/Services/OrchestrationCommandReceipts.ts` |
| 62% | `apps/server/src/persistence/Migrations/049_ProjectionThreadMessagesDispatchOrigin.ts` | `apps/server/src/persistence/Migrations/007_ProjectionThreadMessageAttachments.ts` |
| 62% | `apps/server/src/persistence/Migrations/102_ProjectionThreadMessagesTurnBoundary.ts` | `apps/server/src/persistence/Migrations/007_ProjectionThreadMessageAttachments.ts` |
| 62% | `apps/server/src/provider/Services/ProviderSessionReaper.ts` | `apps/server/src/provider/Services/ProviderSessionReaper.ts` |
| 62% | `apps/web/src/pendingUserInput.ts` | `apps/web/src/pendingUserInput.ts` |
| 62% | `apps/web/src/components/chat/ComposerPendingTerminalContexts.tsx` | `apps/web/src/components/chat/ComposerPendingTerminalContexts.tsx` |
| 61% | `apps/web/src/terminal-links.test.ts` | `apps/web/src/terminal-links.test.ts` |
| 61% | `apps/server/integration/OrchestrationEngineHarness.integration.ts` | `apps/server/integration/OrchestrationEngineHarness.integration.ts` |
| 61% | `apps/server/src/persistence/Layers/ProjectionThreadActivities.ts` | `apps/server/src/persistence/Layers/ProjectionThreadActivities.ts` |
| 61% | `apps/web/src/components/ui/tooltip.tsx` | `apps/web/src/components/ui/tooltip.tsx` |
| 61% | `apps/web/src/lib/projectPaths.ts` | `packages/client-runtime/src/state/projects.ts` |
| 61% | `apps/server/src/persistence/Migrations/021_ProjectionThreadsAssociatedWorktree.ts` | `apps/server/src/persistence/Migrations/010_ProjectionThreadsRuntimeMode.ts` |
| 61% | `apps/server/src/persistence/Migrations/022_ProjectionThreadsAssociatedWorktreeBranch.ts` | `apps/server/src/persistence/Migrations/010_ProjectionThreadsRuntimeMode.ts` |
| 61% | `apps/web/src/components/ui/checkbox.tsx` | `apps/web/src/components/ui/checkbox.tsx` |
| 60% | `apps/web/src/components/ui/combobox.tsx` | `apps/web/src/components/ui/combobox.tsx` |
| 60% | `apps/server/src/persistence/Migrations/040_ProjectionThreadsPinnedMessagesNotes.ts` | `apps/server/src/persistence/Migrations/012_ProjectionThreadsInteractionMode.ts` |
| 60% | `apps/server/src/persistence/Migrations/041_ProjectionProjectsPinned.ts` | `apps/server/src/persistence/Migrations/039_ProjectionProjectsDefaultThreadEnvMode.ts` |
| 60% | `apps/server/src/persistence/Migrations/095_ProjectionThreadsGoalTiming.ts` | `apps/server/src/persistence/Migrations/012_ProjectionThreadsInteractionMode.ts` |
| 60% | `apps/web/src/components/ui/scroll-area.tsx` | `apps/web/src/components/ui/scroll-area.tsx` |
| 60% | `apps/server/src/persistence/Layers/ProjectionProjects.ts` | `apps/server/src/persistence/Layers/ProjectionProjects.ts` |
| 59% | `apps/server/src/persistence/Services/ProjectionProjects.ts` | `apps/server/src/persistence/Services/ProjectionProjects.ts` |
| 59% | `apps/server/src/persistence/Migrations/039_ReconcileLegacyPinnedThreads.ts` | `apps/server/src/persistence/Migrations/012_ProjectionThreadsInteractionMode.ts` |
| 58% | `apps/server/src/persistence/Migrations/019_ProjectionThreadsEnvMode.ts` | `apps/server/src/persistence/Migrations/010_ProjectionThreadsRuntimeMode.ts` |
| 58% | `apps/web/src/worktreeCleanup.test.ts` | `apps/web/src/worktreeCleanup.test.ts` |
| 58% | `apps/server/src/persistence/Migrations/068_GitHandoffOperations.ts` | `apps/server/src/persistence/Migrations/013_ProjectionThreadProposedPlans.ts` |
| 58% | `apps/server/src/persistence/Services/ProjectionThreadSessions.ts` | `apps/server/src/persistence/Services/ProjectionThreadSessions.ts` |
| 57% | `apps/web/src/projectScripts.ts` | `apps/web/src/projectScripts.ts` |
| 57% | `apps/server/src/orchestration/Layers/RuntimeReceiptBus.ts` | `apps/server/src/orchestration/Layers/RuntimeReceiptBus.ts` |
| 57% | `apps/server/src/persistence/Migrations/030_ProjectionThreadMessagesDispatchMode.ts` | `apps/server/src/persistence/Migrations/007_ProjectionThreadMessageAttachments.ts` |
| 57% | `apps/server/src/persistence/Migrations/097_ProjectionThreadsSidechatLifecycle.ts` | `apps/server/src/persistence/Migrations/010_ProjectionThreadsRuntimeMode.ts` |
| 56% | `apps/server/src/terminal/Services/PTY.ts` | `apps/server/src/terminal/PtyAdapter.ts` |
| 56% | `apps/server/src/persistence/Services/ProjectionThreadMessages.ts` | `apps/server/src/persistence/Services/ProjectionThreadMessages.ts` |
| 56% | `apps/server/src/orchestration/Services/ThreadDeletionReactor.ts` | `apps/server/src/orchestration/Services/ThreadDeletionReactor.ts` |
| 55% | `apps/web/src/composer-logic.ts` | `apps/web/src/composer-logic.ts` |
| 55% | `apps/server/src/orchestration/Services/ProjectionPipeline.ts` | `apps/server/src/orchestration/Services/ProjectionPipeline.ts` |
| 55% | `apps/server/src/persistence/Migrations/060_PendingApprovalLifecycleGeneration.ts` | `apps/server/src/persistence/Migrations/017_ProjectionThreadsArchivedAt.ts` |
| 54% | `apps/server/src/persistence/NodeSqliteClient.ts` | `packages/shared/src/nodeSqliteClient.ts` |
| 54% | `apps/server/src/workspace/Layers/WorkspacePaths.test.ts` | `apps/server/src/workspace/WorkspacePaths.test.ts` |
| 54% | `apps/web/src/components/ui/sidebar.tsx` | `apps/web/src/components/ui/sidebar.tsx` |
| 54% | `apps/desktop/scripts/dev-electron.mjs` | `apps/desktop/scripts/dev-electron.mjs` |
| 53% | `apps/web/src/components/ui/select.tsx` | `apps/web/src/components/ui/select.tsx` |
| 53% | `apps/server/integration/providerService.integration.test.ts` | `apps/server/integration/providerService.integration.test.ts` |
| 53% | `apps/server/src/persistence/Migrations/131_ProjectSourceFolders.ts` | `apps/server/src/persistence/Migrations/039_ProjectionProjectsDefaultThreadEnvMode.ts` |
| 53% | `apps/server/src/persistence/Migrations/133_ProjectionTurnsWorkspaceInitialization.ts` | `apps/server/src/persistence/Migrations/015_ProjectionTurnsSourceProposedPlan.ts` |
| 52% | `apps/web/src/threadSelectionStore.test.ts` | `apps/web/src/threadSelectionStore.test.ts` |
| 51% | `packages/shared/src/Net.ts` | `packages/shared/src/Net.ts` |
| 51% | `apps/server/src/attachmentPaths.ts` | `apps/server/src/attachmentPaths.ts` |
| 51% | `apps/web/src/components/ui/menu.tsx` | `apps/web/src/components/ui/menu.tsx` |
| 51% | `apps/web/src/projectScripts.test.ts` | `apps/web/src/projectScripts.test.ts` |
| 50% | `apps/server/src/orchestration/Services/ProviderRuntimeIngestion.ts` | `apps/server/src/orchestration/Services/ProviderRuntimeIngestion.ts` |
| 50% | `apps/server/src/persistence/Migrations/033_ProjectionThreadsSidechatSource.ts` | `apps/server/src/persistence/Migrations/017_ProjectionThreadsArchivedAt.ts` |
| 50% | `apps/server/src/persistence/Migrations/085_AutomationSettings.ts` | `apps/server/src/persistence/Migrations/013_ProjectionThreadProposedPlans.ts` |
| 50% | `apps/server/src/persistence/Migrations/134_ProjectionThreadsForkSourceMessage.ts` | `apps/server/src/persistence/Migrations/052_ProjectionThreadTitleState.ts` |
| 50% | `apps/server/src/persistence/Migrations/139_RemoteConnectionPreferences.ts` | `apps/server/src/persistence/Migrations/052_ProjectionThreadTitleState.ts` |
| 50% | `apps/server/src/provider/Services/CodexAdapter.ts` | `apps/server/src/provider/Services/CodexAdapter.ts` |
| 50% | `apps/web/src/lib/terminalFocus.ts` | `apps/web/src/lib/previewFocus.ts` |

### 30–50% derived

| Sim | Synara file | Derived from |
|---:|---|---|
| 50% | `apps/server/src/terminal/Layers/NodePTY.ts` | `apps/server/src/terminal/NodePtyAdapter.ts` |
| 49% | `apps/server/src/persistence/Layers/AuthPairingLinks.ts` | `apps/server/src/persistence/AuthPairingLinks.ts` |
| 49% | `apps/server/src/provider/acp/AcpRuntimeModel.ts` | `apps/server/src/provider/acp/AcpRuntimeModel.ts` |
| 49% | `apps/server/src/provider/Layers/ProviderSessionDirectory.ts` | `apps/server/src/provider/Layers/ProviderSessionDirectory.ts` |
| 49% | `apps/server/src/attachmentStore.ts` | `apps/server/src/attachmentStore.ts` |
| 48% | `apps/web/src/vite-env.d.ts` | `apps/web/src/vite-env.d.ts` |
| 48% | `apps/server/src/persistence/Migrations/029_ProjectionThreadsLastKnownPr.ts` | `apps/server/src/persistence/Migrations/017_ProjectionThreadsArchivedAt.ts` |
| 48% | `apps/server/src/persistence/Migrations/031_ProjectionThreadsCreateBranchFlowCompleted.ts` | `apps/server/src/persistence/Migrations/017_ProjectionThreadsArchivedAt.ts` |
| 48% | `apps/web/src/hooks/useTurnDiffSummaries.ts` | `apps/web/src/hooks/useTurnDiffSummaries.ts` |
| 48% | `apps/server/src/persistence/Layers/AuthSessions.ts` | `apps/server/src/persistence/AuthSessions.ts` |
| 48% | `apps/web/src/components/chat/ProjectActionDialog.tsx` | `apps/web/src/components/projectScriptEditor.tsx` |
| 48% | `apps/web/src/components/PullRequestThreadDialog.tsx` | `apps/web/src/components/PullRequestThreadDialog.tsx` |
| 48% | `apps/server/src/persistence/Migrations/088_ProjectionThreadsSettledAt.ts` | `apps/server/src/persistence/Migrations/012_ProjectionThreadsInteractionMode.ts` |
| 47% | `apps/server/src/provider/acp/AcpCoreRuntimeEvents.test.ts` | `apps/server/src/provider/acp/AcpCoreRuntimeEvents.test.ts` |
| 47% | `apps/server/src/persistence/Migrations/143_UsageAnalyticsDeletedTokens.ts` | `apps/server/src/persistence/Migrations/013_ProjectionThreadProposedPlans.ts` |
| 47% | `apps/server/src/orchestration/Layers/OrchestrationReactor.ts` | `apps/server/src/orchestration/Layers/OrchestrationReactor.ts` |
| 46% | `apps/server/src/orchestration/projector.ts` | `apps/server/src/orchestration/projector.ts` |
| 46% | `apps/server/src/auth/Layers/ServerAuthPolicy.ts` | `apps/server/src/auth/EnvironmentAuthPolicy.ts` |
| 46% | `apps/server/src/persistence/Migrations/107_ProjectionThreadsHumanMessage.ts` | `apps/server/src/persistence/Migrations/010_ProjectionThreadsRuntimeMode.ts` |
| 46% | `packages/contracts/src/terminal.ts` | `packages/contracts/src/terminal.ts` |
| 46% | `apps/web/src/lib/lruCache.test.ts` | `apps/web/src/lib/lruCache.test.ts` |
| 46% | `apps/server/src/orchestration/projector.test.ts` | `apps/server/src/orchestration/projector.test.ts` |
| 46% | `scripts/dev-runner.ts` | `scripts/dev-runner.ts` |
| 45% | `apps/server/src/persistence/Migrations/059_ProviderSessionLifecycleGeneration.ts` | `apps/server/src/persistence/Migrations/027_ProviderSessionRuntimeInstanceId.ts` |
| 45% | `packages/contracts/src/auth.ts` | `packages/contracts/src/auth.ts` |
| 45% | `apps/server/src/imageMime.ts` | `apps/server/src/imageMime.ts` |
| 45% | `apps/web/src/components/desktopUpdate.logic.test.ts` | `apps/web/src/components/desktopUpdate.logic.test.ts` |
| 44% | `apps/server/src/orchestration/Services/OrchestrationReactor.ts` | `apps/server/src/orchestration/Services/OrchestrationReactor.ts` |
| 44% | `apps/server/src/persistence/Migrations/009_ProviderSessionRuntimeMode.ts` | `apps/server/src/persistence/Migrations/052_ProjectionThreadTitleState.ts` |
| 44% | `apps/server/src/persistence/Migrations/072_AgentGatewayOperationRetention.ts` | `apps/server/src/persistence/Migrations/051_ProjectionThreadMessageContext.ts` |
| 44% | `apps/server/src/provider/Layers/CursorAdapter.ts` | `apps/server/src/provider/Layers/CursorAdapter.ts` |
| 44% | `apps/server/src/attachmentStore.test.ts` | `apps/server/src/attachmentStore.test.ts` |
| 44% | `apps/web/src/components/ui/switch.tsx` | `apps/web/src/components/ui/switch.tsx` |
| 44% | `apps/server/src/orchestration/commandInvariants.test.ts` | `apps/server/src/orchestration/commandInvariants.test.ts` |
| 44% | `apps/server/src/persistence/Migrations/063_ProjectionMessageCausalSequence.ts` | `apps/server/src/persistence/Migrations/008_ProjectionThreadActivitySequence.ts` |
| 43% | `apps/server/src/persistence/Migrations/090_ProjectionThreadMessageTextSegments.ts` | `apps/server/src/persistence/Migrations/051_ProjectionThreadMessageContext.ts` |
| 43% | `packages/contracts/src/git.ts` | `packages/contracts/src/git.ts` |
| 43% | `apps/desktop/src/updateMachine.test.ts` | `apps/desktop/src/updates/updateMachine.test.ts` |
| 43% | `apps/server/src/persistence/Layers/ProviderSessionRuntime.ts` | `apps/server/src/persistence/ProviderSessionRuntime.ts` |
| 42% | `apps/server/src/orchestration/Services/ProjectionSnapshotQuery.ts` | `apps/server/src/orchestration/Services/ProjectionSnapshotQuery.ts` |
| 42% | `apps/server/src/checkpointing/Services/CheckpointStore.ts` | `apps/server/src/checkpointing/CheckpointStore.ts` |
| 42% | `apps/server/src/persistence/Migrations/028_ProjectionProjectsKind.ts` | `apps/server/src/persistence/Migrations/039_ProjectionProjectsDefaultThreadEnvMode.ts` |
| 41% | `apps/server/src/provider/Layers/ProviderSessionReaper.ts` | `apps/server/src/provider/Layers/ProviderSessionReaper.ts` |
| 41% | `apps/server/src/orchestration/runtimeLayer.ts` | `apps/server/src/orchestration/runtimeLayer.ts` |
| 41% | `apps/web/src/keybindings.ts` | `apps/web/src/keybindings.ts` |
| 41% | `apps/server/src/orchestration/Errors.ts` | `apps/server/src/orchestration/Errors.ts` |
| 40% | `apps/server/src/persistence/Services/ProjectionThreads.ts` | `apps/server/src/persistence/Services/ProjectionThreads.ts` |
| 40% | `apps/server/src/workspace/Layers/WorkspacePaths.ts` | `apps/server/src/workspace/WorkspacePaths.ts` |
| 40% | `apps/server/scripts/cli.ts` | `apps/server/scripts/cli.ts` |
| 40% | `apps/server/src/persistence/Migrations/025_ProjectionThreadsSubagents.ts` | `apps/server/src/persistence/Migrations/051_ProjectionThreadMessageContext.ts` |
| 40% | `apps/server/src/persistence/Migrations/061_PendingApprovalSettlementState.ts` | `apps/server/src/persistence/Migrations/017_ProjectionThreadsArchivedAt.ts` |
| 40% | `apps/server/src/persistence/Migrations/101_RemoveTranscriptMarkers.ts` | `apps/server/src/persistence/Migrations/011_OrchestrationThreadCreatedRuntimeMode.ts` |
| 40% | `apps/server/src/persistence/Migrations/127_ProjectImportHistory.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 40% | `apps/server/src/provider/Services/CursorAdapter.ts` | `apps/server/src/provider/Services/OpenCodeAdapter.ts` |
| 40% | `apps/server/src/provider/Services/DroidAdapter.ts` | `apps/server/src/provider/Services/OpenCodeAdapter.ts` |
| 40% | `apps/server/src/provider/Services/GrokAdapter.ts` | `apps/server/src/provider/Services/OpenCodeAdapter.ts` |
| 40% | `apps/server/src/provider/Services/OmpAdapter.ts` | `apps/server/src/provider/Services/OpenCodeAdapter.ts` |
| 40% | `apps/server/src/provider/Services/OpenCodeAdapter.ts` | `apps/server/src/provider/Services/OpenCodeAdapter.ts` |
| 40% | `apps/server/src/provider/Services/PiAdapter.ts` | `apps/server/src/provider/Services/OpenCodeAdapter.ts` |
| 40% | `apps/web/src/components/RowHoverActions.tsx` | `apps/web/src/components/settings/SettingsGroup.tsx` |
| 40% | `packages/profile-ui/src/skeleton.tsx` | `apps/web/src/components/ui/kbd.tsx` |
| 40% | `scripts/lib/resolve-catalog.ts` | `scripts/lib/resolve-catalog.ts` |
| 40% | `apps/server/src/auth/utils.ts` | `apps/server/src/auth/utils.ts` |
| 39% | `apps/web/src/components/ui/kbd.tsx` | `apps/web/src/components/ui/kbd.tsx` |
| 39% | `packages/shared/src/logging.ts` | `packages/shared/src/logging.ts` |
| 39% | `apps/web/src/pullRequestReference.test.ts` | `apps/web/src/pullRequestReference.test.ts` |
| 39% | `apps/web/src/router.ts` | `apps/web/src/router.ts` |
| 39% | `apps/web/src/components/ui/toast.logic.test.ts` | `apps/web/src/components/ui/toast.logic.test.ts` |
| 39% | `apps/server/src/persistence/Migrations/067_ProviderDeliveryReconciliation.ts` | `apps/server/src/persistence/Migrations/013_ProjectionThreadProposedPlans.ts` |
| 39% | `apps/server/src/persistence/Migrations/089_RecoverRetentionHiddenThreads.ts` | `apps/server/src/persistence/Migrations/010_ProjectionThreadsRuntimeMode.ts` |
| 39% | `apps/server/src/persistence/Migrations/092_BackfillAutomationRunThreadSource.ts` | `apps/server/src/persistence/Migrations/010_ProjectionThreadsRuntimeMode.ts` |
| 39% | `packages/contracts/src/keybindings.ts` | `packages/contracts/src/keybindings.ts` |
| 39% | `packages/contracts/src/keybindings.test.ts` | `packages/contracts/src/keybindings.test.ts` |
| 38% | `apps/web/src/components/ui/button.tsx` | `apps/web/src/components/ui/button.tsx` |
| 38% | `packages/contracts/src/orchestration.ts` | `packages/contracts/src/orchestration.ts` |
| 38% | `apps/server/src/persistence/Migrations/086_NormalizeStudioThreadWorkspaces.ts` | `apps/server/src/persistence/Migrations/010_ProjectionThreadsRuntimeMode.ts` |
| 38% | `apps/server/src/persistence/Migrations/118_ProjectionThreadSessionProviderInstance.ts` | `apps/server/src/persistence/Migrations/028_ProjectionThreadSessionInstanceId.ts` |
| 38% | `apps/server/src/persistence/NodeSqliteClient.test.ts` | `packages/shared/src/nodeSqliteClient.test.ts` |
| 38% | `apps/server/src/persistence/Migrations/079_Spaces.ts` | `apps/server/src/persistence/Migrations/013_ProjectionThreadProposedPlans.ts` |
| 38% | `apps/web/src/lib/syntaxHighlighting.ts` | `apps/web/src/lib/syntaxHighlighting.ts` |
| 38% | `apps/server/src/persistence/Migrations/043_ProfileStatsIndexes.ts` | `apps/server/src/persistence/Migrations/002_OrchestrationCommandReceipts.ts` |
| 38% | `apps/server/src/persistence/Migrations/140_RemoteAccessControls.ts` | `apps/server/src/persistence/Migrations/052_ProjectionThreadTitleState.ts` |
| 38% | `apps/server/src/persistence/Migrations/142_AuthAccessOrigin.ts` | `apps/server/src/persistence/Migrations/052_ProjectionThreadTitleState.ts` |
| 37% | `apps/server/src/serverRuntimeState.ts` | `apps/server/src/serverRuntimeState.ts` |
| 37% | `apps/server/src/provider/cliVersion.ts` | `packages/shared/src/semver.ts` |
| 37% | `apps/server/src/persistence/Migrations/136_AccountUsageSync.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 37% | `apps/web/src/components/GitActionsControl.logic.ts` | `packages/client-runtime/src/state/gitActions.ts` |
| 37% | `apps/server/src/git/Layers/OpenCodeTextGeneration.test.ts` | `apps/server/src/textGeneration/OpenCodeTextGeneration.test.ts` |
| 37% | `apps/server/src/persistence/Migrations/119_ProviderSessionRuntimeInstanceId.ts` | `apps/server/src/persistence/Migrations/027_ProviderSessionRuntimeInstanceId.ts` |
| 37% | `apps/server/src/orchestration/Layers/OrchestrationReactor.test.ts` | `apps/server/src/orchestration/Layers/OrchestrationReactor.test.ts` |
| 36% | `apps/server/src/persistence/Migrations/052_ProjectionThreadUserMessageSummaryIndex.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 36% | `apps/server/src/persistence/Migrations/065_DurableQueuedTurnPromotions.ts` | `apps/server/src/persistence/Migrations/013_ProjectionThreadProposedPlans.ts` |
| 36% | `apps/server/src/persistence/Migrations/094_ProjectionThreadsGoal.test.ts` | `apps/server/src/persistence/Migrations/042_ProjectionThreadLinkedPullRequest.test.ts` |
| 36% | `apps/server/src/persistence/Migrations/099_InvalidateProjectionThreadsCursor.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 36% | `apps/server/src/persistence/Migrations/124_ProjectionTurnsPendingMessageIndex.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 36% | `apps/server/src/persistence/Migrations/135_ProjectionMessageTurnActivityIndex.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 36% | `apps/server/src/persistence/Migrations/083_AutomationHeartbeatEligibility.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 36% | `apps/web/src/components/desktopUpdate.logic.ts` | `apps/web/src/components/desktopUpdate.logic.ts` |
| 36% | `packages/contracts/src/baseSchemas.ts` | `packages/contracts/src/baseSchemas.ts` |
| 36% | `packages/contracts/src/providerRuntime.test.ts` | `packages/contracts/src/providerRuntime.test.ts` |
| 35% | `packages/contracts/src/terminal.test.ts` | `packages/contracts/src/terminal.test.ts` |
| 35% | `apps/web/src/components/BranchToolbar.logic.test.ts` | `apps/web/src/components/BranchToolbar.logic.test.ts` |
| 35% | `apps/web/src/components/chat/ProposedPlanCard.tsx` | `apps/web/src/components/chat/ProposedPlanCard.tsx` |
| 35% | `apps/server/src/persistence/Migrations/120_ProfileStatsDeletedProviderInstances.ts` | `apps/server/src/persistence/Migrations/027_ProviderSessionRuntimeInstanceId.ts` |
| 35% | `apps/server/src/persistence/Migrations/123_ScrubOrchestrationEventProviderOptions.ts` | `apps/server/src/persistence/Migrations/011_OrchestrationThreadCreatedRuntimeMode.ts` |
| 35% | `apps/server/src/persistence/Migrations/032_ReconcileImportedSchemaLineage.test.ts` | `apps/server/src/persistence/Migrations/024_BackfillProjectionThreadShellSummary.test.ts` |
| 35% | `apps/server/scripts/acp-mock-agent.ts` | `apps/server/scripts/acp-mock-agent.ts` |
| 35% | `apps/server/src/orchestration/decider.projectScripts.test.ts` | `apps/server/src/orchestration/decider.projectScripts.test.ts` |
| 35% | `packages/shared/src/DrainableWorker.test.ts` | `packages/shared/src/DrainableWorker.test.ts` |
| 35% | `apps/web/src/components/ui/toast.tsx` | `apps/web/src/components/ui/toast.tsx` |
| 35% | `apps/server/src/provider/Services/ProviderAdapter.ts` | `apps/server/src/provider/Services/ProviderAdapter.ts` |
| 34% | `apps/server/src/persistence/Migrations/066_DurableProviderRuntimeEvents.ts` | `apps/server/src/persistence/Migrations/013_ProjectionThreadProposedPlans.ts` |
| 34% | `apps/web/src/components/chat/ComposerPlanFollowUpBanner.tsx` | `apps/web/src/components/chat/ComposerPlanFollowUpBanner.tsx` |
| 34% | `apps/web/src/components/pullRequest/PullRequestDiffStat.tsx` | `apps/web/src/components/chat/DiffStatLabel.tsx` |
| 34% | `apps/server/src/persistence/Migrations/040_ProjectionThreadsPinnedMessagesNotes.test.ts` | `apps/server/src/persistence/Migrations/040_ProjectionProjectFaviconPath.test.ts` |
| 34% | `apps/desktop/scripts/smoke-test.mjs` | `apps/desktop/scripts/smoke-test.mjs` |
| 34% | `apps/server/src/provider/Services/ProviderAdapterRegistry.ts` | `apps/server/src/provider/Services/ProviderAdapterRegistry.ts` |
| 34% | `apps/web/src/pendingInteractionDerivation.test.ts` | `packages/client-runtime/src/pendingRequests.test.ts` |
| 34% | `apps/server/src/persistence/Layers/OrchestrationEventStore.ts` | `apps/server/src/persistence/Layers/OrchestrationEventStore.ts` |
| 34% | `apps/web/src/components/ui/input.tsx` | `apps/web/src/components/ui/input.tsx` |
| 33% | `apps/server/src/orchestration/commandInvariants.ts` | `apps/server/src/orchestration/commandInvariants.ts` |
| 33% | `apps/server/src/persistence/Migrations/046_AutomationCompletionPolicy.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 33% | `apps/server/src/persistence/Migrations/051_ProfileStatsDeletedTokensModel.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 33% | `apps/server/src/persistence/Migrations/071_ProjectionThreadsGatewayProvenance.ts` | `apps/server/src/persistence/Migrations/051_ProjectionThreadMessageContext.ts` |
| 33% | `apps/server/src/persistence/Migrations/106_ProjectImportOrigins.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 33% | `apps/server/src/persistence/Migrations/112_CoordinatorAppearance.ts` | `apps/server/src/persistence/Migrations/052_ProjectionThreadTitleState.ts` |
| 33% | `apps/server/src/persistence/Migrations/137_AccountUsageSyncIdentity.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 33% | `apps/server/src/provider/Services/AntigravityAdapter.ts` | `apps/server/src/provider/Services/OpenCodeAdapter.ts` |
| 33% | `apps/web/src/routes/_chat.groups.index.tsx` | `apps/web/src/routes/connect.tsx` |
| 33% | `apps/web/src/routes/_chat.studio.index.tsx` | `apps/web/src/routes/connect.tsx` |
| 33% | `apps/server/src/provider/Layers/ProviderSessionDirectory.test.ts` | `apps/server/src/provider/Layers/ProviderSessionDirectory.test.ts` |
| 33% | `apps/server/src/persistence/Layers/ProjectionThreads.ts` | `apps/server/src/persistence/Layers/ProjectionThreads.ts` |
| 33% | `scripts/merge-mac-update-manifests.test.ts` | `scripts/merge-update-manifests.test.ts` |
| 33% | `apps/web/src/threadSelectionStore.ts` | `apps/web/src/threadSelectionStore.ts` |
| 33% | `apps/server/src/keybindings.test.ts` | `apps/server/src/keybindings.test.ts` |
| 32% | `apps/server/src/persistence/Layers/ProjectionThreadMessages.test.ts` | `apps/server/src/persistence/Layers/ProjectionThreadMessages.test.ts` |
| 32% | `packages/contracts/src/filesystem.ts` | `packages/contracts/src/filesystem.ts` |
| 32% | `apps/server/src/git/Layers/CursorTextGeneration.test.ts` | `apps/server/src/textGeneration/CursorTextGeneration.test.ts` |
| 32% | `apps/server/src/config.ts` | `apps/server/src/config.ts` |
| 32% | `apps/server/src/persistence/Migrations/081_AutomationProposals.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 32% | `apps/server/src/persistence/Migrations/082_AutomationMemory.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 32% | `apps/server/src/persistence/Layers/ProjectionThreadMessages.ts` | `apps/server/src/persistence/Layers/ProjectionThreadMessages.ts` |
| 32% | `apps/marketing/src/components/ui/card.tsx` | `apps/web/src/components/ui/number-field.tsx` |
| 32% | `apps/web/src/pullRequestReference.ts` | `apps/web/src/pullRequestReference.ts` |
| 32% | `apps/server/src/workspace/Services/WorkspacePaths.ts` | `apps/server/src/workspace/WorkspacePaths.ts` |
| 32% | `apps/server/src/persistence/Migrations/080_ExternalMcpProjectScope.ts` | `apps/server/src/persistence/Migrations/052_ProjectionThreadTitleState.ts` |
| 31% | `apps/server/src/persistence/Migrations/144_ProjectionThreadsPullRequestLinks.ts` | `apps/server/src/persistence/Migrations/010_ProjectionThreadsRuntimeMode.ts` |
| 31% | `apps/web/src/hooks/useLocalStorage.ts` | `apps/web/src/hooks/useLocalStorage.ts` |
| 31% | `packages/shared/src/shell.test.ts` | `packages/shared/src/shell.test.ts` |
| 31% | `apps/server/src/provider/Services/ProviderService.ts` | `apps/server/src/provider/Services/ProviderService.ts` |
| 31% | `apps/server/src/persistence/Migrations/073_OperationalDiagnostics.ts` | `apps/server/src/persistence/Migrations/002_OrchestrationCommandReceipts.ts` |
| 31% | `apps/server/src/provider/Services/DevinAdapter.ts` | `apps/server/src/provider/Services/OpenCodeAdapter.ts` |
| 30% | `apps/server/src/git/Layers/GitManager.ts` | `apps/server/src/git/GitManager.ts` |
| 30% | `apps/web/src/components/DiffPanelShell.tsx` | `apps/web/src/components/DiffPanelShell.tsx` |
| 30% | `apps/web/src/components/ui/alert.tsx` | `apps/web/src/components/ui/alert.tsx` |
| 30% | `apps/server/src/persistence/Migrations/070_AgentGatewayOperations.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
## 2. Logic-level matches (rewritten-but-derived candidates)

Files with low line-similarity (<30%) whose identifier set + string constants substantially
match a t3code file — same public/internal surface, different text. `idJ` = identifier Jaccard,
`strC` = fraction of this file's long string constants also present in the counterpart.

| Sim | idJ | strC | shared ids | Synara file | t3code counterpart |
|---:|---:|---:|---|---|---|
| 17% | 1.00 | 0.12 | 6/6 | `apps/web/src/routeTree.gen.ts` | `apps/web/src/routeTree.gen.ts` |
| 27% | 0.80 | 0.08 | 4/4 | `apps/marketing/src/components/ui/alert.tsx` | `apps/web/src/components/ui/alert.tsx` |
| 22% | 0.75 | 0.29 | 6/8 | `apps/web/src/keybindings.test.ts` | `apps/web/src/keybindings.test.ts` |
| 6% | 0.75 | 0.11 | 3/4 | `apps/web/src/lib/desktopProjectRecovery.test.ts` | `apps/mobile/src/features/archive/archivedThreadList.test.ts` |
| 20% | 0.67 | 0.20 | 2/3 | `apps/server/src/orchestration/Services/ProviderCommandReactor.ts` | `apps/server/src/orchestration/Services/ProviderCommandReactor.ts` |
| 16% | 0.67 | 0.57 | 4/4 | `apps/server/src/os-jank.ts` | `apps/server/src/os-jank.ts` |
| 26% | 0.67 | 0.00 | 2/3 | `packages/shared/src/Struct.ts` | `packages/shared/src/Struct.ts` |
| 13% | 0.60 | 0.07 | 3/4 | `apps/server/src/persistence/Layers/Sqlite.ts` | `apps/server/src/persistence/Layers/Sqlite.ts` |
| 10% | 0.60 | 0.08 | 3/5 | `apps/web/src/components/Sidebar.logic.test.ts` | `apps/web/src/components/Sidebar.logic.test.ts` |
| 17% | 0.57 | 0.12 | 4/7 | `apps/server/src/orchestration/Layers/CheckpointReactor.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.ts` |
| 26% | 0.56 | 0.17 | 9/10 | `apps/server/src/persistence/Services/AuthSessions.ts` | `apps/server/src/persistence/AuthSessions.ts` |
| 26% | 0.54 | 0.25 | 7/10 | `apps/server/src/persistence/Services/AuthPairingLinks.ts` | `apps/server/src/persistence/AuthPairingLinks.ts` |
| 13% | 0.50 | 0.21 | 3/6 | `apps/server/src/git/Layers/DroidTextGeneration.test.ts` | `apps/server/src/textGeneration/CursorTextGeneration.test.ts` |
| 28% | 0.50 | 0.35 | 9/10 | `apps/server/src/git/Layers/GitManager.test.ts` | `apps/server/src/git/GitManager.test.ts` |
| 10% | 0.50 | 0.06 | 9/15 | `apps/server/src/orchestration/Layers/CheckpointReactor.test.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.test.ts` |
| 9% | 0.50 | 0.18 | 2/4 | `apps/server/src/orchestration/decider.checkpointRevert.test.ts` | `apps/server/src/orchestration/decider.settled.test.ts` |
| 18% | 0.50 | 0.00 | 2/3 | `apps/web/src/components/chat/DiffStatLabel.tsx` | `apps/web/src/components/chat/DiffStatLabel.tsx` |
| 3% | 0.50 | 0.00 | 2/3 | `apps/web/src/hooks/useWorkspaceFileEditorSession.browser.tsx` | `apps/web/src/components/device/useDeviceControls.test.tsx` |
| 6% | 0.50 | 0.14 | 3/4 | `apps/web/src/lib/desktopProjectRecovery.test.ts` | `apps/server/src/orchestration/ThreadSettlementReactor.test.ts` |
| 2% | 0.50 | 0.03 | 2/3 | `apps/web/src/lib/projectCreation.test.ts` | `apps/mobile/src/features/archive/archivedThreadList.test.ts` |
| 9% | 0.50 | 0.06 | 3/6 | `apps/web/src/routes/_chat.tsx` | `apps/web/src/routes/_chat.tsx` |
| 6% | 0.43 | 0.18 | 3/4 | `apps/web/src/lib/desktopProjectRecovery.test.ts` | `apps/server/src/orchestration/PullRequestSyncReactor.test.ts` |
| 22% | 0.42 | 0.12 | 11/21 | `apps/server/src/persistence/Errors.ts` | `apps/server/src/persistence/Errors.ts` |
| 1% | 0.42 | 0.17 | 17/33 | `apps/web/src/types.ts` | `apps/web/src/types.ts` |
| 12% | 0.40 | 0.08 | 2/5 | `apps/server/src/orchestration/Layers/OrchestrationEngine.ts` | `apps/server/src/orchestration/Layers/OrchestrationEngine.ts` |
| 30% | 0.40 | 0.50 | 2/4 | `apps/server/src/orchestration/Services/OrchestrationEngine.ts` | `apps/server/src/orchestration/Services/OrchestrationEngine.ts` |
| 28% | 0.40 | 0.60 | 2/3 | `apps/server/src/persistence/Services/OrchestrationEventStore.ts` | `apps/server/src/persistence/Services/OrchestrationEventStore.ts` |
| 24% | 0.40 | 0.25 | 4/5 | `apps/server/src/persistence/Services/ProviderSessionRuntime.ts` | `apps/server/src/persistence/ProviderSessionRuntime.ts` |
| 28% | 0.40 | 1.00 | 2/4 | `apps/server/src/stream/collectUint8StreamText.ts` | `apps/server/src/stream/collectUint8StreamText.ts` |
| 2% | 0.40 | 0.01 | 2/4 | `apps/web/src/hooks/useAccount.test.tsx` | `apps/web/src/components/device/useDeviceControls.test.tsx` |
| 22% | 0.40 | 0.60 | 6/11 | `apps/web/src/lib/utils.ts` | `apps/web/src/lib/utils.ts` |
| 12% | 0.36 | 0.67 | 4/5 | `apps/server/src/provider/acp/AcpErrors.ts` | `packages/effect-acp/src/errors.ts` |
| 20% | 0.36 | 0.57 | 4/4 | `apps/server/src/startupAccess.ts` | `apps/server/src/startupAccess.ts` |
| 20% | 0.30 | 0.40 | 15/27 | `apps/server/src/git/Services/GitCore.ts` | `apps/server/src/vcs/GitVcsDriver.ts` |
| 9% | 0.30 | 0.57 | 3/3 | `apps/web/src/components/chat/composerProviderRegistry.tsx` | `apps/web/src/components/chat/composerProviderState.tsx` |
| 16% | 0.29 | 0.31 | 2/3 | `scripts/update-release-package-versions.ts` | `scripts/update-release-package-versions.ts` |
| 29% | 0.28 | 0.50 | 5/5 | `packages/contracts/src/environment.ts` | `packages/contracts/src/environment.ts` |
| 18% | 0.27 | 0.58 | 7/13 | `packages/shared/src/git.ts` | `packages/shared/src/git.ts` |
| 14% | 0.25 | 0.75 | 2/3 | `apps/desktop/src/runtimeArch.ts` | `apps/desktop/src/app/DesktopEnvironment.ts` |
| 29% | 0.25 | 0.32 | 5/14 | `apps/server/src/orchestration/Layers/ProjectionPipeline.ts` | `apps/server/src/orchestration/Layers/ProjectionPipeline.ts` |

Notable: `routeTree.gen.ts` (idJ 1.0 — same generated route surface, regenerated output),
`apps/web/src/types.ts` (17/33 shared exported type names — same public contract),
`persistence/Errors.ts` (11/21 — same error-type surface). These are expected contract-level
overlap rather than copied implementation, but the generator/route tables themselves are
T3-derived structures.

## 3. Sub-file copied blocks (files <50% similar overall)

Files whose body contains at least one **contiguous verbatim block ≥8 normalized lines**
copied from a t3code file. `block` = largest contiguous run; `copied` = total matched lines
across all blocks vs that counterpart.

| Max block | Copied | Sim | Synara file | Source |
|---:|---:|---:|---|---|
| 64 | 1156 | 22% | `apps/server/src/provider/Layers/ClaudeAdapter.test.ts` | `apps/server/src/provider/Layers/ClaudeAdapter.test.ts` |
| 36 | 752 | 28% | `apps/server/src/provider/Layers/ClaudeAdapter.ts` | `apps/server/src/provider/Layers/ClaudeAdapter.ts` |
| 172 | 477 | 46% | `apps/server/src/orchestration/projector.test.ts` | `apps/server/src/orchestration/projector.test.ts` |
| 37 | 389 | 28% | `apps/server/src/git/Layers/GitManager.test.ts` | `apps/server/src/git/GitManager.test.ts` |
| 38 | 298 | 30% | `apps/server/src/git/Layers/GitManager.ts` | `apps/server/src/git/GitManager.ts` |
| 30 | 286 | 38% | `packages/contracts/src/orchestration.ts` | `packages/contracts/src/orchestration.ts` |
| 20 | 264 | 24% | `apps/server/src/orchestration/Layers/ProjectionSnapshotQuery.ts` | `apps/server/src/orchestration/Layers/ProjectionSnapshotQuery.ts` |
| 51 | 257 | 46% | `apps/server/src/orchestration/projector.ts` | `apps/server/src/orchestration/projector.ts` |
| 28 | 254 | 17% | `apps/server/src/orchestration/Layers/ProjectionPipeline.test.ts` | `apps/server/src/orchestration/Layers/ProjectionPipeline.test.ts` |
| 90 | 217 | 35% | `apps/server/scripts/acp-mock-agent.ts` | `apps/server/scripts/acp-mock-agent.ts` |
| 34 | 199 | 27% | `apps/server/src/provider/acp/AcpJsonRpcConnection.test.ts` | `apps/server/src/provider/acp/AcpJsonRpcConnection.test.ts` |
| 38 | 191 | 10% | `apps/web/src/workLog.test.ts` | `apps/web/src/session-logic.test.ts` |
| 30 | 176 | 49% | `apps/server/src/provider/acp/AcpRuntimeModel.ts` | `apps/server/src/provider/acp/AcpRuntimeModel.ts` |
| 21 | 167 | 9% | `apps/server/src/provider/Layers/ProviderService.test.ts` | `apps/server/src/provider/Layers/ProviderService.test.ts` |
| 29 | 162 | 27% | `apps/server/src/provider/Layers/CodexAdapter.ts` | `apps/server/src/provider/Layers/CodexAdapter.ts` |
| 44 | 161 | 30% | `apps/server/src/provider/acp/AcpRuntimeModel.test.ts` | `apps/server/src/provider/acp/AcpRuntimeModel.test.ts` |
| 29 | 160 | 44% | `apps/server/src/provider/Layers/CursorAdapter.ts` | `apps/server/src/provider/Layers/CursorAdapter.ts` |
| 14 | 159 | 21% | `apps/server/src/orchestration/decider.ts` | `apps/server/src/orchestration/decider.ts` |
| 31 | 145 | 17% | `apps/server/src/provider/Layers/OpenCodeAdapter.ts` | `apps/server/src/provider/Layers/OpenCodeAdapter.ts` |
| 25 | 145 | 34% | `apps/web/src/pendingInteractionDerivation.test.ts` | `packages/client-runtime/src/pendingRequests.test.ts` |
| 23 | 138 | 15% | `apps/server/src/orchestration/Layers/ProviderRuntimeIngestion.test.ts` | `apps/server/src/orchestration/Layers/ProviderRuntimeIngestion.test.ts` |
| 18 | 132 | 6% | `apps/server/src/orchestration/Layers/ProviderCommandReactor.test.ts` | `apps/server/src/orchestration/Layers/ProviderCommandReactor.test.ts` |
| 26 | 118 | 14% | `apps/server/src/terminal/Layers/Manager.ts` | `apps/server/src/terminal/Manager.ts` |
| 25 | 118 | 37% | `apps/web/src/components/GitActionsControl.logic.ts` | `packages/client-runtime/src/state/gitActions.ts` |
| 18 | 117 | 33% | `apps/server/src/keybindings.test.ts` | `apps/server/src/keybindings.test.ts` |
| 22 | 117 | 21% | `packages/contracts/src/orchestration.test.ts` | `packages/contracts/src/orchestration.test.ts` |
| 14 | 108 | 19% | `apps/server/src/git/Layers/GitCore.ts` | `apps/server/src/vcs/GitVcsDriverCore.ts` |
| 18 | 108 | 29% | `apps/server/src/orchestration/Layers/ProjectionPipeline.ts` | `apps/server/src/orchestration/Layers/ProjectionPipeline.ts` |
| 36 | 106 | 41% | `apps/web/src/keybindings.ts` | `apps/web/src/keybindings.ts` |
| 14 | 102 | 18% | `apps/server/src/orchestration/Layers/OrchestrationEngine.test.ts` | `apps/server/src/orchestration/Layers/OrchestrationEngine.test.ts` |
| 29 | 101 | 15% | `apps/server/src/orchestration/Layers/ProviderRuntimeIngestion.ts` | `apps/server/src/orchestration/Layers/ProviderRuntimeIngestion.ts` |
| 27 | 100 | 27% | `apps/server/src/provider/opencodeRuntime.ts` | `apps/server/src/provider/opencodeRuntime.ts` |
| 47 | 98 | 43% | `packages/contracts/src/git.ts` | `packages/contracts/src/git.ts` |
| 59 | 94 | 31% | `packages/shared/src/shell.test.ts` | `packages/shared/src/shell.test.ts` |
| 18 | 91 | 27% | `apps/web/src/components/GitActionsControl.logic.test.ts` | `apps/web/src/components/GitActionsControl.logic.test.ts` |
| 29 | 91 | 46% | `scripts/dev-runner.ts` | `scripts/dev-runner.ts` |
| 21 | 87 | 5% | `apps/web/src/components/Sidebar.tsx` | `apps/web/src/components/LegacySidebar.tsx` |
| 20 | 87 | 35% | `apps/web/src/components/ui/toast.tsx` | `apps/web/src/components/ui/toast.tsx` |
| 43 | 85 | 34% | `apps/server/src/persistence/Layers/OrchestrationEventStore.ts` | `apps/server/src/persistence/Layers/OrchestrationEventStore.ts` |
| 25 | 85 | 17% | `apps/web/src/components/GitActionsControl.tsx` | `apps/web/src/components/GitActionsControl.tsx` |
| 30 | 85 | 12% | `apps/web/src/composerDraftPersistence.ts` | `apps/web/src/composerDraftStore.ts` |
| 28 | 83 | 30% | `apps/server/src/keybindings.ts` | `apps/server/src/keybindings.ts` |
| 38 | 79 | 22% | `apps/web/src/keybindings.test.ts` | `apps/web/src/keybindings.test.ts` |
| 47 | 76 | 26% | `apps/web/src/components/Icons.tsx` | `apps/web/src/components/Icons.tsx` |
| 22 | 75 | 19% | `scripts/build-desktop-artifact.ts` | `scripts/build-desktop-artifact.ts` |
| 14 | 71 | 10% | `apps/server/src/orchestration/Layers/CheckpointReactor.test.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.test.ts` |
| 32 | 70 | 12% | `apps/web/src/workLog.ts` | `apps/web/src/session-logic.ts` |
| 22 | 68 | 8% | `apps/web/src/composerDraftStore.persistence.test.ts` | `apps/web/src/composerDraftStore.test.ts` |
| 65 | 65 | 4% | `apps/server/src/codexAppServerManager.ts` | `apps/server/src/provider/CodexDeveloperInstructions.ts` |
| 13 | 65 | 6% | `apps/server/src/profileStats.test.ts` | `apps/server/src/orchestration/Layers/ProjectionSnapshotQuery.test.ts` |
| 12 | 63 | 24% | `apps/server/src/provider/acp/AcpSessionRuntime.ts` | `apps/server/src/provider/acp/AcpSessionRuntime.ts` |
| 18 | 61 | 39% | `packages/contracts/src/keybindings.test.ts` | `packages/contracts/src/keybindings.test.ts` |
| 13 | 60 | 37% | `apps/server/src/git/Layers/OpenCodeTextGeneration.test.ts` | `apps/server/src/textGeneration/OpenCodeTextGeneration.test.ts` |
| 12 | 57 | 48% | `apps/server/src/persistence/Layers/AuthSessions.ts` | `apps/server/src/persistence/AuthSessions.ts` |
| 16 | 54 | 35% | `apps/web/src/components/BranchToolbar.logic.test.ts` | `apps/web/src/components/BranchToolbar.logic.test.ts` |
| 16 | 53 | 47% | `apps/server/src/provider/acp/AcpCoreRuntimeEvents.test.ts` | `apps/server/src/provider/acp/AcpCoreRuntimeEvents.test.ts` |
| 13 | 52 | 20% | `apps/server/src/git/Services/GitCore.ts` | `apps/server/src/vcs/GitVcsDriver.ts` |
| 11 | 51 | 17% | `apps/server/src/orchestration/Layers/CheckpointReactor.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.ts` |
| 24 | 51 | 45% | `apps/web/src/components/desktopUpdate.logic.test.ts` | `apps/web/src/components/desktopUpdate.logic.test.ts` |
| 18 | 51 | 39% | `packages/contracts/src/keybindings.ts` | `packages/contracts/src/keybindings.ts` |
| 13 | 50 | 8% | `apps/server/src/git/testing/fakeGitHubCli.ts` | `apps/server/src/git/GitManager.test.ts` |
| 17 | 50 | 26% | `apps/server/src/persistence/Services/AuthSessions.ts` | `apps/server/src/persistence/AuthSessions.ts` |
| 16 | 49 | 32% | `apps/server/src/persistence/Layers/ProjectionThreadMessages.test.ts` | `apps/server/src/persistence/Layers/ProjectionThreadMessages.test.ts` |
| 12 | 49 | 26% | `apps/server/src/provider/Layers/DroidAdapter.ts` | `apps/server/src/provider/Layers/CursorAdapter.ts` |
| 14 | 49 | 33% | `apps/server/src/provider/Layers/ProviderSessionDirectory.test.ts` | `apps/server/src/provider/Layers/ProviderSessionDirectory.test.ts` |
| 12 | 49 | 45% | `packages/contracts/src/auth.ts` | `packages/contracts/src/auth.ts` |
| 12 | 48 | 14% | `apps/server/src/provider/Layers/CodexAdapter.test.ts` | `apps/server/src/provider/Layers/CodexAdapter.test.ts` |
| 24 | 47 | 7% | `apps/web/src/composerDraftStore.attachments.test.ts` | `apps/web/src/composerDraftStore.test.ts` |
| 16 | 46 | 23% | `apps/server/src/auth/Layers/SessionCredentialService.ts` | `apps/server/src/auth/SessionStore.ts` |
| 12 | 46 | 26% | `apps/server/src/provider/Layers/OmpAdapter.ts` | `apps/server/src/provider/Layers/CursorAdapter.ts` |
| 11 | 45 | 6% | `apps/server/src/provider/Layers/ProviderHealth.test.ts` | `apps/server/src/provider/Layers/ProviderRegistry.test.ts` |
| 19 | 45 | 11% | `apps/web/src/session-logic.ts` | `apps/web/src/session-logic.ts` |
| 24 | 45 | 36% | `packages/contracts/src/providerRuntime.test.ts` | `packages/contracts/src/providerRuntime.test.ts` |
| 16 | 44 | 11% | `apps/server/src/orchestration/providerRuntimeActivityProjection.ts` | `apps/server/src/orchestration/Layers/ProviderRuntimeIngestion.ts` |
| 11 | 44 | 27% | `packages/contracts/src/pullRequests.ts` | `packages/contracts/src/pullRequest.ts` |
| 25 | 44 | 19% | `scripts/dev-runner.test.ts` | `scripts/dev-runner.test.ts` |
| 15 | 43 | 23% | `apps/web/src/lib/terminalContext.ts` | `apps/web/src/lib/terminalContext.ts` |
| 17 | 42 | 27% | `packages/shared/src/shell.ts` | `packages/shared/src/shell.ts` |
| 15 | 40 | 6% | `apps/server/src/orchestration/Layers/ProjectionSnapshotQuery.test.ts` | `apps/server/src/orchestration/Layers/ProjectionSnapshotQuery.test.ts` |
| 13 | 39 | 13% | `apps/server/src/provider/Layers/ProviderAdapterRegistry.test.ts` | `apps/server/src/provider/Layers/ProviderAdapterRegistry.test.ts` |
| 19 | 39 | 9% | `apps/server/src/workspaceEntries.ts` | `packages/shared/src/searchRanking.ts` |
| 12 | 39 | 48% | `apps/web/src/components/PullRequestThreadDialog.tsx` | `apps/web/src/components/PullRequestThreadDialog.tsx` |
| 15 | 37 | 26% | `apps/server/src/persistence/Services/AuthPairingLinks.ts` | `apps/server/src/persistence/AuthPairingLinks.ts` |
| 12 | 37 | 25% | `apps/server/src/provider/Layers/AntigravityAdapter.ts` | `apps/server/src/provider/Layers/CursorAdapter.ts` |
| 12 | 37 | 15% | `apps/server/src/provider/Layers/DevinAdapter.ts` | `apps/server/src/provider/Layers/CursorAdapter.ts` |
| 12 | 37 | 26% | `apps/server/src/provider/Layers/GrokAdapter.ts` | `apps/server/src/provider/Layers/CursorAdapter.ts` |
| 11 | 37 | 8% | `apps/web/src/composerDraftStore.test.ts` | `apps/web/src/composerDraftStore.test.ts` |
| 15 | 35 | 22% | `apps/server/src/provider/Layers/EventNdjsonLogger.test.ts` | `apps/server/src/provider/Layers/EventNdjsonLogger.test.ts` |
| 10 | 35 | 48% | `apps/web/src/components/chat/ProjectActionDialog.tsx` | `apps/web/src/components/projectScriptEditor.tsx` |
| 14 | 34 | 29% | `apps/web/src/composer-logic.test.ts` | `apps/web/src/composer-logic.test.ts` |
| 21 | 33 | 29% | `apps/server/src/project/Layers/ProjectFaviconResolver.ts` | `apps/server/src/project/ProjectFaviconResolver.ts` |
| 17 | 33 | 11% | `apps/web/src/composerDraftActions.ts` | `apps/web/src/composerDraftStore.ts` |
| 25 | 33 | 5% | `apps/web/src/composerDraftAttachments.ts` | `apps/web/src/composerDraftStore.ts` |
| 16 | 33 | 27% | `apps/web/src/lib/contextWindow.ts` | `apps/web/src/lib/contextWindow.ts` |
| 12 | 33 | 15% | `apps/web/src/session-logic.test.ts` | `apps/web/src/session-logic.test.ts` |
| 15 | 33 | 46% | `packages/contracts/src/terminal.ts` | `packages/contracts/src/terminal.ts` |
| 12 | 32 | 43% | `apps/desktop/src/updateMachine.test.ts` | `apps/desktop/src/updates/updateMachine.test.ts` |
| 16 | 32 | 15% | `apps/server/src/persistence/Migrations.ts` | `apps/server/src/persistence/Migrations.ts` |
| 32 | 32 | 9% | `apps/server/src/provider/acp/CursorAcpSupport.test.ts` | `apps/server/src/provider/acp/CursorAcpSupport.test.ts` |
| 23 | 32 | 18% | `apps/web/src/lib/diffRendering.ts` | `apps/web/src/lib/diffRendering.ts` |
| 13 | 31 | 40% | `apps/server/scripts/cli.ts` | `apps/server/scripts/cli.ts` |
| 31 | 31 | 12% | `apps/server/src/git/textGenerationShared.ts` | `packages/shared/src/schemaJson.ts` |
| 17 | 31 | 17% | `apps/server/src/persistence/Layers/OrchestrationEventStore.test.ts` | `apps/server/src/persistence/Layers/OrchestrationEventStore.test.ts` |
| 17 | 31 | 2% | `apps/web/src/components/chat/useChatTimelineMessages.ts` | `apps/web/src/components/ChatView.tsx` |
| 22 | 31 | 4% | `apps/web/src/composerDraftStoreTestFixtures.ts` | `apps/web/src/composerDraftStore.test.ts` |
| 31 | 31 | 35% | `packages/shared/src/DrainableWorker.test.ts` | `packages/shared/src/DrainableWorker.test.ts` |
| 11 | 30 | 49% | `apps/server/src/provider/Layers/ProviderSessionDirectory.ts` | `apps/server/src/provider/Layers/ProviderSessionDirectory.ts` |
| 11 | 29 | 43% | `apps/server/src/persistence/Layers/ProviderSessionRuntime.ts` | `apps/server/src/persistence/ProviderSessionRuntime.ts` |
| 11 | 28 | 49% | `apps/server/src/attachmentStore.ts` | `apps/server/src/attachmentStore.ts` |
| 28 | 28 | 32% | `apps/server/src/git/Layers/CursorTextGeneration.test.ts` | `apps/server/src/textGeneration/CursorTextGeneration.test.ts` |
| 28 | 28 | 33% | `apps/server/src/orchestration/commandInvariants.ts` | `apps/server/src/orchestration/commandInvariants.ts` |
| 15 | 28 | 24% | `apps/server/src/persistence/Migrations/035_NormalizeLegacyModelSelectionOptions.test.ts` | `apps/server/src/persistence/Migrations/016_CanonicalizeModelSelections.test.ts` |
| 19 | 28 | 40% | `apps/server/src/workspace/Layers/WorkspacePaths.ts` | `apps/server/src/workspace/WorkspacePaths.ts` |
| 16 | 28 | 39% | `apps/web/src/components/ui/toast.logic.test.ts` | `apps/web/src/components/ui/toast.logic.test.ts` |
| 17 | 28 | 7% | `apps/web/src/composerDraftDomain.ts` | `apps/web/src/composerDraftStore.ts` |
| 10 | 26 | 15% | `apps/server/src/git/Layers/CodexTextGeneration.test.ts` | `apps/server/src/textGeneration/CodexTextGeneration.test.ts` |
| 10 | 26 | 22% | `apps/server/src/provider/providerMaintenance.ts` | `apps/server/src/provider/providerMaintenance.ts` |
| 24 | 24 | 44% | `apps/server/src/attachmentStore.test.ts` | `apps/server/src/attachmentStore.test.ts` |
| 14 | 24 | 8% | `apps/server/src/provider/acp/CursorAcpSupport.ts` | `apps/server/src/provider/Layers/CursorProvider.ts` |
| 24 | 24 | 37% | `apps/server/src/provider/cliVersion.ts` | `packages/shared/src/semver.ts` |
| 13 | 23 | 6% | `apps/server/src/git/Layers/GitHubCli.ts` | `apps/server/src/sourceControl/GitHubCli.ts` |
| 15 | 23 | 7% | `apps/web/src/components/ChatMarkdown.tsx` | `apps/web/src/components/ChatMarkdown.tsx` |
| 14 | 23 | 9% | `apps/web/src/components/chat/MessagesTimeline.logic.ts` | `apps/web/src/components/chat/MessagesTimeline.logic.ts` |
| 13 | 22 | 35% | `apps/server/src/orchestration/decider.projectScripts.test.ts` | `apps/server/src/orchestration/decider.projectScripts.test.ts` |
| 22 | 22 | 7% | `packages/contracts/src/model.ts` | `packages/contracts/src/model.ts` |
| 11 | 20 | 35% | `apps/server/src/persistence/Migrations/032_ReconcileImportedSchemaLineage.test.ts` | `apps/server/src/orchestration/Layers/ProjectionSnapshotQuery.test.ts` |
| 12 | 20 | 8% | `apps/server/src/provider/acp/AcpAdapterSessionSupport.ts` | `apps/server/src/provider/Layers/CursorAdapter.ts` |
| 11 | 20 | 12% | `apps/web/src/components/Sidebar.logic.ts` | `apps/web/src/components/Sidebar.logic.ts` |
| 12 | 20 | 17% | `apps/web/src/routeTree.gen.ts` | `apps/web/src/routeTree.gen.ts` |
| 20 | 20 | 18% | `packages/shared/src/git.ts` | `packages/shared/src/git.ts` |
| 10 | 19 | 25% | `apps/server/src/checkpointing/Layers/CheckpointDiffQuery.test.ts` | `apps/server/src/checkpointing/CheckpointDiffQuery.test.ts` |
| 10 | 19 | 42% | `apps/server/src/checkpointing/Services/CheckpointStore.ts` | `apps/server/src/checkpointing/CheckpointStore.ts` |
| 10 | 18 | 29% | `apps/server/src/git/Layers/ClaudeTextGeneration.ts` | `apps/server/src/textGeneration/ClaudeTextGeneration.ts` |
| 18 | 18 | 45% | `apps/server/src/imageMime.ts` | `apps/server/src/imageMime.ts` |
| 10 | 18 | 21% | `apps/server/src/persistence/Layers/ProjectionRepositories.test.ts` | `apps/server/src/persistence/Layers/ProjectionRepositories.test.ts` |
| 10 | 18 | 32% | `apps/server/src/persistence/Layers/ProjectionThreadMessages.ts` | `apps/server/src/persistence/Layers/ProjectionThreadMessages.ts` |
| 9 | 18 | 33% | `apps/server/src/persistence/Layers/ProjectionThreads.ts` | `apps/server/src/persistence/Layers/ProjectionThreads.ts` |
| 9 | 17 | 5% | `apps/desktop/src/updateState.ts` | `apps/desktop/src/updates/DesktopUpdates.ts` |
| 9 | 17 | 6% | `apps/server/src/orchestration/Layers/ProviderCommandReactor.ts` | `apps/server/src/orchestration/Layers/ProviderCommandReactor.ts` |
| 9 | 17 | 14% | `apps/server/src/terminal/Layers/Manager.test.ts` | `apps/server/src/terminal/Manager.test.ts` |
| 17 | 17 | 11% | `apps/web/src/components/composerFooterLayout.test.ts` | `apps/web/src/components/composerFooterLayout.test.ts` |
| 9 | 17 | 31% | `apps/web/src/hooks/useLocalStorage.ts` | `apps/web/src/hooks/useLocalStorage.ts` |
| 17 | 17 | 46% | `apps/web/src/lib/lruCache.test.ts` | `apps/web/src/lib/lruCache.test.ts` |
| 8 | 16 | 5% | `apps/server/src/persistence/Migrations.test.ts` | `apps/server/src/orchestration/Layers/ProjectionSnapshotQuery.test.ts` |
| 15 | 15 | 12% | `apps/web/src/lib/keybindingCapture.ts` | `apps/web/src/components/settings/KeybindingsSettings.logic.ts` |
| 15 | 15 | 17% | `apps/web/src/lib/providerDiscovery.ts` | `packages/shared/src/searchRanking.ts` |
| 15 | 15 | 4% | `apps/web/src/storeEventReducer.ts` | `apps/server/src/orchestration/projector.ts` |
| 14 | 14 | 10% | `apps/server/src/persistence/Migrations/123_ScrubOrchestrationEventProviderOptions.test.ts` | `apps/server/src/persistence/Layers/OrchestrationEventStore.test.ts` |
| 14 | 14 | 38% | `apps/server/src/persistence/NodeSqliteClient.test.ts` | `packages/shared/src/nodeSqliteClient.test.ts` |
| 14 | 14 | 41% | `apps/server/src/provider/Layers/ProviderSessionReaper.ts` | `apps/server/src/provider/Layers/ProviderSessionReaper.ts` |
| 14 | 14 | 6% | `apps/web/src/components/chat/useChatPendingInteractions.ts` | `apps/web/src/components/chat/ChatComposer.tsx` |
| 14 | 14 | 36% | `packages/contracts/src/baseSchemas.ts` | `packages/contracts/src/baseSchemas.ts` |
| 14 | 14 | 33% | `scripts/merge-mac-update-manifests.test.ts` | `scripts/merge-update-manifests.test.ts` |
| 13 | 13 | 16% | `apps/server/src/persistence/Migrations/053_BackfillThreadActivitySequence.test.ts` | `apps/server/src/persistence/Migrations/026_CanonicalizeModelSelectionOptions.test.ts` |
| 13 | 13 | 8% | `apps/server/src/provider/Layers/ProviderService.ts` | `apps/server/src/provider/Layers/ProviderService.ts` |
| 13 | 13 | 11% | `apps/server/src/provider/claudeAuthStatus.ts` | `apps/server/src/provider/Layers/ClaudeProvider.ts` |
| 13 | 13 | 23% | `apps/web/src/components/BranchToolbar.logic.ts` | `packages/shared/src/git.ts` |
| 13 | 13 | 8% | `apps/web/src/components/ChatView.logic.ts` | `apps/web/src/components/ChatView.logic.ts` |
| 12 | 12 | 3% | `apps/server/src/orchestration/importThreadRoute.ts` | `apps/server/src/orchestration/Layers/ProviderCommandReactor.ts` |
| 12 | 12 | 17% | `apps/server/src/persistence/Layers/ProjectionCheckpoints.ts` | `apps/server/src/orchestration/Layers/ProjectionSnapshotQuery.ts` |
| 12 | 12 | 20% | `apps/web/src/timestampFormat.ts` | `apps/web/src/timestampFormat.ts` |
| 11 | 11 | 14% | `apps/desktop/src/runtimeArch.ts` | `apps/desktop/src/app/DesktopEnvironment.ts` |
| 11 | 11 | 46% | `apps/server/src/auth/Layers/ServerAuthPolicy.ts` | `apps/server/src/auth/EnvironmentAuthPolicy.ts` |
| 11 | 11 | 32% | `apps/server/src/config.ts` | `apps/server/src/config.ts` |
| 11 | 11 | 3% | `apps/server/src/git/Layers/GitCore.test.ts` | `apps/server/src/checkpointing/CheckpointStore.test.ts` |
| 11 | 11 | 38% | `apps/server/src/persistence/Migrations/118_ProjectionThreadSessionProviderInstance.ts` | `apps/server/src/persistence/Migrations/028_ProjectionThreadSessionInstanceId.ts` |
| 11 | 11 | 37% | `apps/server/src/persistence/Migrations/119_ProviderSessionRuntimeInstanceId.ts` | `apps/server/src/persistence/Migrations/027_ProviderSessionRuntimeInstanceId.ts` |
| 11 | 11 | 35% | `apps/server/src/provider/Services/ProviderAdapter.ts` | `apps/server/src/provider/Services/ProviderAdapter.ts` |
| 11 | 11 | 19% | `apps/web/src/components/RenameDialog.tsx` | `apps/web/src/components/PullRequestThreadDialog.tsx` |
| 11 | 11 | 27% | `apps/web/src/components/ThreadWorktreeHandoffDialog.tsx` | `apps/web/src/components/PullRequestThreadDialog.tsx` |
| 11 | 11 | 7% | `apps/web/src/components/pullRequest/PullRequestMetaLine.tsx` | `apps/web/src/components/pullRequest/pullRequestPresentation.tsx` |
| 11 | 11 | 8% | `apps/web/src/components/terminal/TerminalViewport.tsx` | `apps/web/src/components/ThreadTerminalDrawer.tsx` |
| 11 | 11 | 6% | `packages/contracts/src/ipc.ts` | `packages/contracts/src/ipc.ts` |
| 11 | 11 | 12% | `packages/contracts/src/server.ts` | `packages/contracts/src/server.ts` |
| 11 | 11 | 18% | `packages/shared/src/model.ts` | `packages/shared/src/model.ts` |
| 10 | 10 | 12% | `apps/server/src/orchestration/Layers/OrchestrationEngine.ts` | `apps/server/src/orchestration/Layers/OrchestrationEngine.ts` |
| 10 | 10 | 31% | `apps/server/src/provider/Services/ProviderService.ts` | `apps/server/src/provider/Services/ProviderService.ts` |
| 10 | 10 | 19% | `apps/server/src/provider/codexCliVersion.ts` | `packages/shared/src/semver.ts` |
| 10 | 10 | 4% | `apps/web/src/components/DiffPanel.tsx` | `apps/web/src/components/DiffPanel.tsx` |
| 10 | 10 | 24% | `apps/web/src/components/ProjectScriptsControl.tsx` | `apps/web/src/components/ProjectScriptsControl.tsx` |
| 10 | 10 | 22% | `apps/web/src/components/ReleaseHistoryDialog.tsx` | `apps/web/src/components/PullRequestThreadDialog.tsx` |
| 10 | 10 | 14% | `apps/web/src/components/SpaceProjectPickerDialog.tsx` | `apps/web/src/components/LegacySidebar.tsx` |
| 10 | 10 | 12% | `apps/web/src/components/WhatsNewDialog.tsx` | `apps/web/src/components/PullRequestThreadDialog.tsx` |
| 10 | 10 | 17% | `apps/web/src/components/chat/ComposerSlashStatusDialog.tsx` | `apps/web/src/components/pullRequest/LinkPullRequestDialog.tsx` |
| 10 | 10 | 35% | `apps/web/src/components/chat/ProposedPlanCard.tsx` | `apps/web/src/components/chat/ProposedPlanCard.tsx` |
| 10 | 10 | 6% | `apps/web/src/components/chat/WorkspaceFileEditorChrome.tsx` | `apps/web/src/components/settings/SettingsPanels.tsx` |
| 10 | 10 | 29% | `apps/web/src/editorPreferences.ts` | `apps/web/src/editorPreferences.ts` |
| 10 | 10 | 29% | `apps/web/src/hooks/useCopyToClipboard.ts` | `apps/web/src/hooks/useCopyToClipboard.ts` |
| 10 | 10 | 19% | `apps/web/src/lib/terminalContext.test.ts` | `apps/web/src/lib/terminalContext.test.ts` |
| 9 | 9 | 2% | `apps/desktop/src/main.ts` | `apps/desktop/src/updates/DesktopUpdates.ts` |
| 9 | 9 | 19% | `apps/server/src/auth/Layers/BootstrapCredentialService.ts` | `apps/server/src/auth/PairingGrantStore.ts` |
| 9 | 9 | 9% | `apps/server/src/auth/Layers/SessionCredentialService.test.ts` | `apps/server/src/auth/SessionStore.test.ts` |
| 9 | 9 | 7% | `apps/server/src/git/Layers/GitHubCli.test.ts` | `apps/server/src/sourceControl/GitHubCli.test.ts` |
| 9 | 9 | 7% | `apps/server/src/provider/acp/AcpSdkConformance.test.ts` | `apps/server/scripts/acp-mock-agent.ts` |
| 9 | 9 | 27% | `apps/server/src/provider/providerStatusCache.ts` | `apps/server/src/provider/providerStatusCache.ts` |
| 9 | 9 | 22% | `apps/server/src/workspace/Layers/WorkspaceFileSystem.test.ts` | `apps/server/src/workspace/WorkspaceFileSystem.test.ts` |
| 9 | 9 | 17% | `apps/web/src/components/BranchToolbarBranchSelector.tsx` | `apps/web/src/components/LegacySidebar.tsx` |
| 9 | 9 | 4% | `apps/web/src/components/DevicePanel.tsx` | `apps/web/src/components/ChatView.tsx` |
| 9 | 9 | 17% | `apps/web/src/components/SpaceEditorDialog.tsx` | `apps/web/src/components/PullRequestThreadDialog.tsx` |
| 9 | 9 | 23% | `apps/web/src/components/chat/ExpandedImagePreview.tsx` | `apps/web/src/components/chat/ExpandedImagePreview.tsx` |
| 9 | 9 | 3% | `apps/web/src/components/chat/useChatComposerCommands.ts` | `apps/web/src/components/chat/ChatComposer.tsx` |
| 9 | 9 | 9% | `apps/web/src/components/codeEditor/EditorDebugPanel.tsx` | `apps/web/src/components/settings/SettingsPanels.tsx` |
| 9 | 9 | 14% | `apps/web/src/components/settings/ConnectionsAddComputerDialog.tsx` | `apps/web/src/components/settings/SettingsPanels.tsx` |
| 9 | 9 | 5% | `apps/web/src/components/settings/McpConnectorsSettingsPanel.tsx` | `apps/web/src/components/GitActionsControl.tsx` |
| 9 | 9 | 29% | `apps/web/src/lib/storage.ts` | `apps/web/src/lib/storage.ts` |
| 9 | 9 | 6% | `apps/web/src/lib/toolCallDetails.ts` | `apps/server/src/orchestration/ActivityPayloadProjection.ts` |
| 9 | 9 | 35% | `packages/contracts/src/terminal.test.ts` | `packages/contracts/src/terminal.test.ts` |
| 9 | 9 | 6% | `packages/shared/src/threadSummary.ts` | `apps/server/src/orchestration/Layers/ProviderRuntimeIngestion.ts` |
| 8 | 8 | 40% | `apps/server/src/auth/utils.ts` | `apps/server/src/auth/utils.ts` |
| 8 | 8 | 19% | `apps/server/src/environment/Layers/ServerEnvironment.ts` | `apps/server/src/environment/ServerEnvironment.ts` |
| 8 | 8 | 19% | `apps/server/src/git/Layers/OpenCodeTextGeneration.ts` | `apps/server/src/textGeneration/OpenCodeTextGeneration.ts` |
| 8 | 8 | 18% | `apps/server/src/git/Services/GitHubCli.ts` | `apps/server/src/sourceControl/GitHubCli.ts` |
| 8 | 8 | 49% | `apps/server/src/persistence/Layers/AuthPairingLinks.ts` | `apps/server/src/persistence/AuthPairingLinks.ts` |
| 8 | 8 | 18% | `apps/server/src/persistence/Migrations/058_ThreadScopedPendingApprovalIdentity.ts` | `apps/server/src/persistence/Migrations/024_BackfillProjectionThreadShellSummary.ts` |
| 8 | 8 | 22% | `apps/server/src/persistence/Services/ProjectionCheckpoints.ts` | `packages/contracts/src/orchestration.ts` |
| 8 | 8 | 24% | `apps/server/src/persistence/Services/ProviderSessionRuntime.ts` | `apps/server/src/persistence/ProviderSessionRuntime.ts` |
| 8 | 8 | 3% | `apps/server/src/provider/Layers/ProviderHealth.ts` | `apps/server/src/provider/providerMaintenanceRunner.ts` |
| 8 | 8 | 6% | `apps/server/src/pullRequests.logic.ts` | `apps/server/src/pullRequest/AzureDevOpsPullRequestProvider.ts` |
| 8 | 8 | 1% | `apps/web/src/components/ChatView.browser.tsx` | `apps/web/src/composerDraftStore.test.ts` |
| 8 | 8 | 8% | `apps/web/src/components/ComposerPromptEditor.tsx` | `apps/web/src/components/ComposerPromptEditorTiptap.tsx` |
| 8 | 8 | 10% | `apps/web/src/components/Sidebar.logic.test.ts` | `apps/web/src/components/Sidebar.logic.test.ts` |
| 8 | 8 | 2% | `apps/web/src/components/SidebarCustomizeList.tsx` | `apps/web/src/components/LegacySidebar.tsx` |
| 8 | 8 | 6% | `apps/web/src/components/account/DeleteAccountControl.tsx` | `apps/web/src/components/settings/ConnectionsSettings.tsx` |
| 8 | 8 | 13% | `apps/web/src/components/chat/ComposerCommandMenu.tsx` | `apps/web/src/components/chat/ComposerCommandMenu.tsx` |
| 8 | 8 | 3% | `apps/web/src/components/chat/MessagesTimeline.logic.test.ts` | `apps/web/src/components/chat/MessagesTimeline.logic.test.ts` |
| 8 | 8 | 12% | `apps/web/src/components/chat/group/GroupSettingsDialog.tsx` | `apps/web/src/components/settings/ConnectionsSettings.tsx` |
| 8 | 8 | 6% | `apps/web/src/components/chat/group/LibraryPanel.tsx` | `apps/web/src/components/settings/ConnectionsSettings.tsx` |
| 8 | 8 | 10% | `apps/web/src/components/composerFooterLayout.ts` | `apps/web/src/components/composerFooterLayout.ts` |
| 8 | 8 | 5% | `apps/web/src/components/pullRequest/PullRequestConfirmActionDialog.tsx` | `apps/web/src/components/pullRequest/PullRequestDetailPanel.tsx` |
| 8 | 8 | 5% | `apps/web/src/components/settings/DesktopSettingsPanels.tsx` | `apps/web/src/components/settings/SettingsPanels.tsx` |
| 8 | 8 | 15% | `apps/web/src/components/settings/McpLocalConnectorDialog.tsx` | `apps/web/src/components/CustomSnoozeDialog.tsx` |
| 8 | 8 | 4% | `apps/web/src/components/settings/providers/ProviderAgentRow.tsx` | `apps/web/src/components/LegacySidebar.tsx` |
| 8 | 8 | 39% | `apps/web/src/components/ui/kbd.tsx` | `apps/web/src/components/ui/kbd.tsx` |
| 8 | 8 | 18% | `apps/web/src/lib/hosts/ExecutionFolderPicker.tsx` | `apps/web/src/components/CustomSnoozeDialog.tsx` |
| 8 | 8 | 5% | `apps/web/src/routes/__root.tsx` | `apps/web/src/routes/__root.tsx` |
| 8 | 8 | 11% | `packages/contracts/src/automation.test.ts` | `packages/contracts/src/keybindings.test.ts` |
| 8 | 8 | 2% | `packages/contracts/src/desktopUpdate.ts` | `packages/contracts/src/ipc.ts` |
| 8 | 8 | 29% | `packages/contracts/src/environment.ts` | `packages/contracts/src/environment.ts` |
| 8 | 8 | 10% | `packages/contracts/src/ws.test.ts` | `packages/contracts/src/keybindings.test.ts` |
| 8 | 8 | 27% | `packages/shared/src/path.ts` | `packages/shared/src/path.ts` |
| 8 | 8 | 9% | `packages/shared/src/schemaJson.ts` | `packages/shared/src/schemaJson.ts` |
| 8 | 8 | 21% | `scripts/lib/brand-assets.ts` | `scripts/lib/brand-assets.test.ts` |
| 8 | 8 | 8% | `scripts/release-smoke.ts` | `scripts/release-smoke.ts` |
## 4. Rust port clean-room verdicts

410 committed `*-rs` crates, all with `src/port.rs` and a `rust-anything.json` unit mapping.
Each port was compared against its TS unit **and** the unit's t3code counterpart (matched by
content, including moved paths) plus a repo-wide t3code internal-name index.

Legend — **fn·T3**: port-internal helper `fn` names also present in the t3code counterpart's
internal functions (strongest derivation signal; export names excluded since interop names
are not copyrightable expression). **fn·syn**: same vs the synara TS (derivation from synara-
original code is legal but shows the port was written reading source, contrary to ADR-001).
**str·T3**: long string literals shared with the t3code counterpart.

### 4.1 Line-derived from T3 code — rewrite required

| Crate | TS unit (sim) | t3code counterpart | fn·T3 | fn·syn | str·T3 | Evidence |
|---|---|---|---:|---:|---:|---|
| `worklog-rs` | `apps/web/src/workLog.ts` (12%) | `apps/web/src/session-logic.ts` | 15 | 53 | 2 | 15 internals incl. `extract_work_log_request_kind`, `extract_tool_call_id`, `compare_activity_lifecycle_rank`, `to_derived_work_log_entry`; unit is ALSO a cua import (double provenance); wide-match also hits `mobile/lib/threadActivity.ts` (12) and `shared/toolActivity.ts` (5) |
| `keybindingrules-rs` | `packages/shared/src/keybindingRules.ts` (68%) | `packages/shared/src/keybindings.ts` | 5 | 5 | 0 | reproduces the exact recursive-descent decomposition `parseOr→parseAnd→parseUnary→parsePrimary` + `normalize_key_token` from T3 `keybindings.ts` — same internal grammar structure |
| `shell-rs` | `packages/shared/src/shell.ts` (27%) | `packages/shared/src/shell.ts` | 5 | 5 | 0 | 5 internals `env_capture_start/end`, `extract_environment_value`, `read_user_login_shell`, `trim_non_empty` identical to T3 `shell.ts` |
| `storeeventreducer-rs` | `apps/web/src/storeEventReducer.ts` (4%) | `apps/server/src/orchestration/projector.ts` | 4 | 25 | 8 | 4 internals incl. `retain_thread_proposed_plans_after_revert`, `retain_thread_activities_after_revert`, `checkpoint_status_to_latest_turn_state` from T3 `projector.ts`; shared strings are event-type contract constants (`thread.reverted` etc., non-expressive) |
| `terminal-links-rs` | `apps/web/src/terminal-links.ts` (76%) | `apps/web/src/terminal-links.ts` | 4 | 4 | 0 | unit is 76% T3-derived; 4 internals `infer_home_from_cwd`, `is_absolute_path`, `is_windows_path_style`, `join_path`; ADR-002 already queued this unit for cases-only rewrite |
| `markdown-links-rs` | `apps/web/src/markdown-links.ts` (16%) | `packages/client-runtime/src/markdownLinks.ts` | 3 | 15 | 0 | 3 internals vs `client-runtime/markdownLinks.ts` + 6 vs `web/terminal-links.ts` + 4 vs `mobile/filePath.ts` — synara unit merged several T3 files; port mirrors all of them |

Verdict: these ports **do not satisfy ADR-001**. Identical distinctive helper names (and in
`keybindingrules-rs`, identical function decomposition) across the TS→Rust boundary are
not producible from `cases.jsonl` alone; they mirror T3-authored expression. Rewrite from
recorded behavior only, as done for `proposedplan-rs`/`worktreecleanup-rs`.

### 4.2 Suspect — single/dual shared internal names

| Crate | TS unit (sim) | t3code counterpart | Shared internal names |
|---|---|---|---|
| `shortcutssheet-rs` | `apps/web/src/shortcutsSheet.ts` (3%) | `apps/web/src/keybindings.ts` | `format_shortcut_label`, `resolve_context` |
| `backendsupervisionpolicy-rs` | `apps/desktop/src/backendSupervisionPolicy.ts` (1%) | `packages/shared/src/composerContextLegacy.ts` | `read` |
| `composer-editor-mentions-rs` | `apps/web/src/composer-editor-mentions.ts` (15%) | `apps/web/src/composer-editor-mentions.ts` | `push_text_segment` |
| `model-rs` | `packages/shared/src/model.ts` (18%) | `packages/shared/src/model.ts` | `clone_descriptor` |
| `pendinguserinput-rs` | `apps/web/src/pendingUserInput.ts` (62%) | `apps/web/src/pendingUserInput.ts` | `normalize_draft_answer` |
| `projectscripts-rs` | `apps/web/src/projectScripts.ts` (57%) | `apps/web/src/projectScripts.ts` | `normalize_script_id` |
| `providerusagesnapshot-rs` | `apps/server/src/providerUsageSnapshot.ts` (4%) | `apps/server/src/usage/usageTranscripts.ts` | `parse_timestamp_ms` |
| `runtimearch-rs` | `apps/desktop/src/runtimeArch.ts` (14%) | `apps/desktop/src/app/DesktopEnvironment.ts` | `normalize_desktop_arch` |
| `serversettings-rs` | `packages/shared/src/serverSettings.ts` (7%) | `packages/shared/src/serverSettings.ts` | `should_replace_text_generation_model_selection` |
| `session-logic-rs` | `apps/web/src/session-logic.ts` (11%) | `apps/web/src/session-logic.ts` | `as_record` |
| `storenormalization-rs` | `apps/web/src/storeNormalization.ts` (2%) | `apps/web/src/session-logic.ts` | `as_record` |
| `terminalstatestore-rs` | `apps/web/src/terminalStateStore.ts` (9%) | `apps/web/src/terminalUiStateStore.ts` | `normalize_terminal_ids` |
| `transcriptusageparsers-rs` | `apps/server/src/transcriptUsageParsers.ts` (19%) | `apps/server/src/usage/usageTranscripts.ts` | `is_forked_session_meta` |
| `worker-rs` | `apps/analytics/worker.ts` (1%) | `apps/web/src/openVsxThemes.ts` | `is_record` |
| `workspaceentries-rs` | `apps/server/src/workspaceEntries.ts` (9%) | `packages/shared/src/searchRanking.ts` | `score_subsequence_match` |

`as_record`/`is_record`/`read` are generic JS-helper names (weak signal). Distinctive names —
`normalize_draft_answer`, `normalize_script_id`, `should_replace_text_generation_model_selection`,
`push_text_segment`, `normalize_terminal_ids`, `is_forked_session_meta`, `score_subsequence_match`,
`format_shortcut_label`/`resolve_context` — warrant a cases.jsonl re-derivation pass or a
documented explanation before these ports ship.

### 4.3 Line-derived from synara-original sources — legal, but ADR-001 process deviation

Ports mirroring ≥10 internal names of a synara-original (non-T3, non-cua) unit. Ownership is
synara's so no copyright exposure — but it proves the porting agent reads sources, which is
how the §4.1 contamination happened:

- `keybindingeditor-rs` ← `apps/web/src/keybindingEditor.ts` — 20 shared internal names
- `providerinstances-rs` ← `packages/shared/src/providerInstances.ts` — 12 shared internal names
- `codexprocessenv-rs` ← `apps/server/src/codexProcessEnv.ts` — 11 shared internal names
- `canary-rs` ← `scripts/canary.ts` — 10 shared internal names
- `usageanalyticsstats-rs` ← `apps/server/src/usageAnalyticsStats.ts` — 10 shared internal names

### 4.4 cua-imported units (MIT © Cua AI, Inc.)

24 ported crates map to units listed in `import-provenance.json`. Their ports show **no**
internal-name mirroring of either t3code or the adapted TS source → consistent with spec-
derived ports. Caveat: `trycua/cua` itself descends from t3code, and this audit compares
against pinned t3code `d15210cd` only — T3 content inside cua files is under-detected.

| Crate | Unit | fn·syn | fn·T3 | sim vs t3code |
|---|---|---:|---:|---:|
| `codexappservermanager-rs` | `apps/server/src/codexAppServerManager.ts` | 2 | 0 | 4% |
| `codexappservertransport-rs` | `apps/server/src/codexAppServerTransport.ts` | 0 | 0 | 2% |
| `composerdraftdomain-rs` | `apps/web/src/composerDraftDomain.ts` | 2 | 0 | 7% |
| `composerdraftpersistence-rs` | `apps/web/src/composerDraftPersistence.ts` | 0 | 0 | 12% |
| `computerframe-rs` | `packages/shared/src/computerFrame.ts` | 0 | 0 | 4% |
| `computerkeynames-rs` | `packages/shared/src/computerKeyNames.ts` | 0 | 0 | 8% |
| `computerstatestore-rs` | `apps/web/src/computerStateStore.ts` | 0 | 0 | 1% |
| `deviceframe-rs` | `packages/shared/src/deviceFrame.ts` | 0 | 0 | 4% |
| `frametransport-rs` | `packages/shared/src/frameTransport.ts` | 0 | 0 | 1% |
| `lengthprefixedrecords-rs` | `packages/shared/src/lengthPrefixedRecords.ts` | 0 | 0 | 1% |
| `main-rs` | `apps/server/src/main.ts` | 0 | 0 | 4% |
| `pendinginteractionderivation-rs` | `apps/web/src/pendingInteractionDerivation.ts` | 2 | 0 | 3% |
| `pngheader-rs` | `apps/server/src/pngHeader.ts` | 0 | 0 | 13% |
| `providerruntime-rs` | `packages/contracts/src/providerRuntime.ts` | 0 | 0 | 76% |
| `rightdockstore-rs` | `apps/web/src/rightDockStore.logic.ts` | 1 | 0 | 1% |
| `serverlayers-rs` | `apps/server/src/serverLayers.ts` | 0 | 0 | 4% |
| `settingsnavigation-rs` | `apps/web/src/settingsNavigation.ts` | 1 | 0 | 1% |
| `settingssearchindex-rs` | `apps/web/src/settingsSearchIndex.ts` | 0 | 0 | 1% |
| `threadsummary-rs` | `packages/shared/src/threadSummary.ts` | 7 | 0 | 6% |
| `uitreetargeting-rs` | `packages/shared/src/uiTreeTargeting.ts` | 0 | 0 | 9% |
| `worklog-rs` | `apps/web/src/workLog.ts` | 53 | 15 | 12% |
| `wsnativeapi-rs` | `apps/web/src/wsNativeApi.ts` | 0 | 0 | 3% |
| `wsrpc-rs` | `apps/server/src/wsRpc.ts` | 0 | 0 | 2% |
| `wstransport-rs` | `apps/web/src/wsTransport.ts` | 0 | 0 | 0% |

### 4.5 Clean ports of T3-derived sources (ADR-002 working as intended)

| Crate | Unit sim | fn·T3 | str·T3 |
|---|---:|---:|---:|
| `proposedplan-rs` | 90% | 0 | 0 |
| `truncatetitle-rs` | 80% | 0 | 0 |
| `providerruntime-rs` | 76% | 0 | 0 |
| `worktreecleanup-rs` | 76% | 0 | 0 |
| `providerinstance-rs` | 72% | 0 | 0 |
| `updatemachine-rs` | 72% | 0 | 0 |
| `composer-logic-rs` | 55% | 0 | 0 |
| `attachmentpaths-rs` | 51% | 0 | 0 |
| `attachmentstore-rs` | 49% | 0 | 0 |
| `dev-runner-rs` | 46% | 0 | 0 |
| `imagemime-rs` | 45% | 0 | 0 |
| `keybindings-rs` | 41% | 0 | 0 |
| `baseschemas-rs` | 36% | 0 | 0 |
| `editorpreferences-rs` | 29% | 0 | 0 |
| `path-rs` | 27% | 0 | 0 |
| `timestampformat-rs` | 20% | 0 | 0 |
| `git-rs` | 18% | 0 | 0 |
| `composerdraftpersistence-rs` | 12% | 0 | 0 |
| `composerdraftdomain-rs` | 7% | 0 | 0 |
| `pullrequests-rs` | 6% | 0 | 0 |
| `threadsummary-rs` | 6% | 0 | 0 |
| `composerdraftattachments-rs` | 5% | 0 | 0 |
| `updatestate-rs` | 5% | 0 | 0 |
| `codexappservermanager-rs` | 4% | 0 | 0 |
| `composerdraftstoretestfixtures-rs` | 4% | 0 | 0 |

Remaining **341 crates**: unit is synara-original, no T3/cua provenance, no mirroring signals — clean.
## 5. trycua/cua imports

`docs/computer-use-cua/import-provenance.json` records **223 files** imported from
`trycua/cua` PR heads (base `8599826d`, heads `3e1512e7`/`aed8e338`/`ebaa1d26`/`d9c1bdab`),
then adapted in synara. License: **MIT, © 2025 Cua AI, Inc.** (`docs/computer-use-cua/CUA-LICENSE.txt`)
— same MIT grant as t3code, but a different copyright holder: any ported expression needs
Cua's notice, which `rust/LICENSE` does not currently carry.

- Only **7 of 223** cua files show ≥30% similarity vs pinned t3code — cua pulled from different
  snapshots and adapted heavily, so direct comparison under-detects T3 content inside cua files.
- **24 cua-imported files are ported units** (§4.4); `workLog.ts` is both cua-imported and
  T3-derived — `worklog-rs` is double-exposed.
- ADR-001 already treats cua files as interface+behavior only; the §4.4 table shows the ports
  honoring that.

## 6. Licenses and attribution

| File | License | Copyright | Notes |
|---|---|---|---|
| `t3code/LICENSE` | MIT | 2026 T3 Tools Inc. | upstream |
| `synara/LICENSE` | MIT | 2026 T3 Tools Inc. + 2026 Emanuele Di Pietro | covers the TS tree; T3 notice correctly retained |
| `synara/rust/LICENSE` | MIT | 2026 Emanuele Di Pietro + 2026 cmdr-chara | **no T3 notice** — any T3-derived Rust violates MIT attribution; also no Cua AI notice |
| `docs/computer-use-cua/CUA-LICENSE.txt` | MIT | 2025 Cua AI, Inc. | covers the 223 imported files in-repo |
| NOTICE files | — | — | none in either repo |
| `t3code/third-party-licenses.config.json` | CC0 notices | deadrobotmusic, BaggoNotes, etc. | notification-sound attributions; **no equivalent file in synara**, and no `*.mp3` assets were carried over |
| `package.json` attributions | — | — | no `author`/license fields crediting T3 found in synara package manifests |

## 7. Risk list — ordered by exposure

| # | Item | Exposure | Action |
|---:|---|---|---|
| 1 | `worklog-rs` | 15 T3 internal names + cua double-provenance | rewrite cases-only (ADR-002) |
| 2 | `storeeventreducer-rs` | 4 T3 internals from `projector.ts` revert path + 25 synara internals | rewrite cases-only |
| 3 | `keybindingrules-rs` | identical parser decomposition to T3 `keybindings.ts` | rewrite cases-only |
| 4 | `shell-rs` | 5 internals identical to T3 `shell.ts` | rewrite cases-only |
| 5 | `terminal-links-rs` | unit 76% derived; 4 internals shared | rewrite cases-only (already queued in ADR-002) |
| 6 | `markdown-links-rs` | internals spread across 3 T3 files (merged origin) | rewrite cases-only |
| 7 | 15 suspect crates (§4.2) | 1–2 shared internal names each | document provenance or re-derive |
| 8 | 18 cua-unit ports (§4.4) | no mirroring found; license attribution gap + cua→t3code under-detection | add Cua AI notice to `rust/LICENSE` if any cua expression ships; spot-check vs actual cua source when a checkout is available |
| 9 | `rust/LICENSE` | missing T3 Tools + Cua AI notices | add both notices if any derived material ships; keep clean-room bar so none is needed |
| 10 | Future `plan.md` units on derived files | 140 ≥50% files incl. all persistence migrations (100%) are plan units; none ported yet | gate every new unit against this list before porting; apply ADR-002 |
| 11 | `docs/computer-use-cua/evidence/*.ts` plan unit | evidence scripts, minimal sim — low risk | n/a |

### Verification gaps

- No `trycua/cua` checkout locally — cua-source comparison is indirect (via synara's adapted
  copies + pinned t3code). Files listed as `A` (added) in `import-provenance.json` are
  cua-original and only verifiable against a cua checkout.
- Similarity is textual/structural. Independent reimplementation converging on identical
  helper names is possible but unlikely for the multi-name matches flagged in §4.1; the
  single-name §4.2 hits carry more uncertainty.
- `cases.jsonl`/`recorded-from.json` provenance data was trusted as the spec source; the audit
  checks port→source mirroring, not whether recordings cover behavior.

## Appendix — complete ranked file table

Every synara source file with normalized-similarity ≥2%, ≥5 shared normalized lines, or a
verbatim block. `sim` = difflib ratio on normalized line sequences; `shared` = distinct
normalized lines present in the counterpart.

| Sim | Shared | Lines | Synara file | Best t3code counterpart |
|---:|---:|---:|---|---|
| 100% | 27 | 31 | `apps/server/src/persistence/Migrations/001_OrchestrationEvents.ts` | `apps/server/src/persistence/Migrations/001_OrchestrationEvents.ts` |
| 100% | 17 | 19 | `apps/server/src/persistence/Migrations/002_OrchestrationCommandReceipts.ts` | `apps/server/src/persistence/Migrations/002_OrchestrationCommandReceipts.ts` |
| 100% | 14 | 15 | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 100% | 18 | 20 | `apps/server/src/persistence/Migrations/004_ProviderSessionRuntime.ts` | `apps/server/src/persistence/Migrations/004_ProviderSessionRuntime.ts` |
| 100% | 11 | 12 | `apps/server/src/persistence/Migrations/006_ProjectionThreadSessionRuntimeModeColumns.ts` | `apps/server/src/persistence/Migrations/006_ProjectionThreadSessionRuntimeModeColumns.ts` |
| 100% | 7 | 7 | `apps/server/src/persistence/Migrations/007_ProjectionThreadMessageAttachments.ts` | `apps/server/src/persistence/Migrations/007_ProjectionThreadMessageAttachments.ts` |
| 100% | 9 | 10 | `apps/server/src/persistence/Migrations/008_ProjectionThreadActivitySequence.ts` | `apps/server/src/persistence/Migrations/008_ProjectionThreadActivitySequence.ts` |
| 100% | 10 | 11 | `apps/server/src/persistence/Migrations/010_ProjectionThreadsRuntimeMode.ts` | `apps/server/src/persistence/Migrations/010_ProjectionThreadsRuntimeMode.ts` |
| 100% | 9 | 9 | `apps/server/src/persistence/Migrations/011_OrchestrationThreadCreatedRuntimeMode.ts` | `apps/server/src/persistence/Migrations/011_OrchestrationThreadCreatedRuntimeMode.ts` |
| 100% | 7 | 7 | `apps/server/src/persistence/Migrations/012_ProjectionThreadsInteractionMode.ts` | `apps/server/src/persistence/Migrations/012_ProjectionThreadsInteractionMode.ts` |
| 100% | 14 | 15 | `apps/server/src/persistence/Migrations/013_ProjectionThreadProposedPlans.ts` | `apps/server/src/persistence/Migrations/013_ProjectionThreadProposedPlans.ts` |
| 100% | 8 | 10 | `apps/server/src/persistence/Migrations/014_ProjectionThreadProposedPlanImplementation.ts` | `apps/server/src/persistence/Migrations/014_ProjectionThreadProposedPlanImplementation.ts` |
| 100% | 8 | 10 | `apps/server/src/persistence/Migrations/015_ProjectionTurnsSourceProposedPlan.ts` | `apps/server/src/persistence/Migrations/015_ProjectionTurnsSourceProposedPlan.ts` |
| 100% | 11 | 20 | `apps/server/src/persistence/Migrations/026_ProjectionThreadShellSummary.ts` | `apps/server/src/persistence/Migrations/023_ProjectionThreadShellSummary.ts` |
| 100% | 150 | 232 | `apps/server/src/persistence/Migrations/027_BackfillProjectionThreadShellSummary.ts` | `apps/server/src/persistence/Migrations/024_BackfillProjectionThreadShellSummary.ts` |
| 100% | 80 | 112 | `apps/server/src/persistence/Migrations/005_Projections.ts` | `apps/server/src/persistence/Migrations/005_Projections.ts` |
| 99% | 89 | 174 | `apps/server/src/persistence/Migrations/016_CanonicalizeModelSelections.ts` | `apps/server/src/persistence/Migrations/016_CanonicalizeModelSelections.ts` |
| 97% | 54 | 57 | `apps/server/src/provider/providerMaintenanceCommandCoordinator.ts` | `apps/server/src/provider/providerMaintenanceCommandCoordinator.ts` |
| 97% | 55 | 116 | `apps/server/integration/fixtures/providerRuntime.ts` | `apps/server/integration/fixtures/providerRuntime.ts` |
| 95% | 39 | 42 | `apps/web/src/lib/lruCache.ts` | `apps/web/src/lib/lruCache.ts` |
| 93% | 14 | 15 | `apps/web/src/components/ui/label.tsx` | `apps/web/src/components/ui/label.tsx` |
| 91% | 39 | 55 | `apps/web/src/lib/projectScriptKeybindings.test.ts` | `apps/web/src/lib/projectScriptKeybindings.test.ts` |
| 91% | 305 | 445 | `apps/server/integration/TestProviderAdapter.integration.ts` | `apps/server/integration/TestProviderAdapter.integration.ts` |
| 90% | 66 | 75 | `apps/web/src/proposedPlan.ts` | `apps/web/src/proposedPlan.ts` |
| 89% | 19 | 23 | `apps/web/src/components/ui/collapsible.tsx` | `apps/web/src/components/ui/collapsible.tsx` |
| 88% | 22 | 38 | `apps/server/src/persistence/Migrations/034_AuthAccessManagement.ts` | `apps/server/src/persistence/Migrations/020_AuthAccessManagement.ts` |
| 87% | 75 | 130 | `apps/server/src/persistence/Services/ProjectionTurns.ts` | `apps/server/src/persistence/Services/ProjectionTurns.ts` |
| 86% | 54 | 64 | `apps/server/src/provider/acp/CursorAcpExtension.ts` | `apps/server/src/provider/acp/CursorAcpExtension.ts` |
| 86% | 27 | 33 | `apps/web/src/components/ui/textarea.tsx` | `apps/web/src/components/ui/textarea.tsx` |
| 84% | 46 | 56 | `apps/web/src/hooks/useMediaQuery.ts` | `apps/web/src/hooks/useMediaQuery.ts` |
| 83% | 178 | 260 | `apps/server/src/persistence/Migrations/016_CanonicalizeModelSelections.test.ts` | `apps/server/src/persistence/Migrations/016_CanonicalizeModelSelections.test.ts` |
| 82% | 46 | 53 | `apps/web/src/proposedPlan.test.ts` | `apps/web/src/proposedPlan.test.ts` |
| 82% | 59 | 71 | `apps/server/src/provider/acp/CursorAcpExtension.test.ts` | `apps/server/src/provider/acp/CursorAcpExtension.test.ts` |
| 82% | 23 | 38 | `apps/server/src/orchestration/Services/RuntimeReceiptBus.ts` | `apps/server/src/orchestration/Services/RuntimeReceiptBus.ts` |
| 82% | 359 | 912 | `apps/server/integration/orchestrationEngine.integration.test.ts` | `apps/server/integration/orchestrationEngine.integration.test.ts` |
| 81% | 36 | 63 | `apps/web/src/components/ui/empty.tsx` | `apps/web/src/components/ui/empty.tsx` |
| 80% | 113 | 204 | `apps/web/src/components/ui/autocomplete.tsx` | `apps/web/src/components/ui/autocomplete.tsx` |
| 80% | 4 | 5 | `apps/web/src/truncateTitle.ts` | `packages/shared/src/String.ts` |
| 78% | 31 | 38 | `apps/web/src/components/ui/badge.tsx` | `apps/web/src/components/ui/badge.tsx` |
| 76% | 23 | 39 | `apps/web/src/worktreeCleanup.ts` | `apps/web/src/worktreeCleanup.ts` |
| 76% | 618 | 944 | `packages/contracts/src/providerRuntime.ts` | `packages/contracts/src/providerRuntime.ts` |
| 76% | 37 | 47 | `apps/server/src/persistence/Services/ProjectionThreadActivities.ts` | `apps/server/src/persistence/Services/ProjectionThreadActivities.ts` |
| 76% | 18 | 23 | `apps/server/src/persistence/Services/ProjectionState.ts` | `apps/server/src/persistence/Services/ProjectionState.ts` |
| 76% | 11 | 17 | `apps/web/src/components/ui/separator.tsx` | `apps/web/src/components/ui/separator.tsx` |
| 76% | 43 | 109 | `apps/server/src/provider/acp/CursorAcpCliProbe.test.ts` | `apps/server/src/provider/acp/CursorAcpCliProbe.test.ts` |
| 76% | 28 | 34 | `apps/web/src/components/ui/toggle.tsx` | `apps/web/src/components/ui/toggle.tsx` |
| 76% | 127 | 192 | `apps/web/src/terminal-links.ts` | `apps/web/src/terminal-links.ts` |
| 76% | 95 | 236 | `apps/server/src/provider/acp/AcpCoreRuntimeEvents.ts` | `apps/server/src/provider/acp/AcpCoreRuntimeEvents.ts` |
| 75% | 3 | 4 | `apps/marketing/src/lib/utils.ts` | `apps/mobile/src/lib/cn.ts` |
| 75% | 6 | 9 | `apps/server/src/persistence/Migrations/042_ProjectionThreadsMarkers.ts` | `apps/server/src/persistence/Migrations/012_ProjectionThreadsInteractionMode.ts` |
| 75% | 6 | 9 | `apps/server/src/persistence/Migrations/094_ProjectionThreadsGoal.ts` | `apps/server/src/persistence/Migrations/012_ProjectionThreadsInteractionMode.ts` |
| 75% | 6 | 9 | `apps/server/src/persistence/Migrations/096_ProjectionThreadsGoalAchievements.ts` | `apps/server/src/persistence/Migrations/012_ProjectionThreadsInteractionMode.ts` |
| 75% | 6 | 9 | `apps/server/src/persistence/Migrations/104_ProjectionThreadsClaudeCacheReview.ts` | `apps/server/src/persistence/Migrations/012_ProjectionThreadsInteractionMode.ts` |
| 75% | 6 | 9 | `apps/server/src/persistence/Migrations/126_ProjectionThreadsSidechatContext.ts` | `apps/server/src/persistence/Migrations/012_ProjectionThreadsInteractionMode.ts` |
| 74% | 52 | 97 | `apps/web/src/components/ui/alert-dialog.tsx` | `apps/web/src/components/ui/alert-dialog.tsx` |
| 74% | 173 | 407 | `apps/server/src/persistence/Layers/ProjectionTurns.ts` | `apps/server/src/persistence/Layers/ProjectionTurns.ts` |
| 73% | 106 | 171 | `scripts/lib/mac-update-manifest.ts` | `scripts/lib/update-manifest.ts` |
| 72% | 34 | 51 | `packages/contracts/src/providerInstance.ts` | `packages/contracts/src/providerInstance.ts` |
| 72% | 48 | 71 | `apps/server/src/persistence/Layers/OrchestrationCommandReceipts.ts` | `apps/server/src/persistence/Layers/OrchestrationCommandReceipts.ts` |
| 72% | 53 | 86 | `apps/server/src/persistence/Layers/ProjectionState.ts` | `apps/server/src/persistence/Layers/ProjectionState.ts` |
| 72% | 50 | 182 | `apps/desktop/src/updateMachine.ts` | `apps/desktop/src/updates/updateMachine.ts` |
| 71% | 6 | 10 | `apps/server/src/persistence/Migrations/036_ProjectionThreadsPinned.ts` | `apps/server/src/persistence/Migrations/012_ProjectionThreadsInteractionMode.ts` |
| 70% | 32 | 44 | `apps/web/src/lib/projectScriptKeybindings.ts` | `apps/web/src/lib/projectScriptKeybindings.ts` |
| 70% | 42 | 63 | `apps/web/src/components/ui/input-group.tsx` | `apps/web/src/components/ui/input-group.tsx` |
| 70% | 36 | 56 | `apps/server/src/persistence/Services/ProjectionThreadProposedPlans.ts` | `apps/server/src/persistence/Services/ProjectionThreadProposedPlans.ts` |
| 68% | 70 | 153 | `apps/web/src/components/ui/dialog.tsx` | `apps/web/src/components/ui/dialog.tsx` |
| 68% | 136 | 220 | `packages/shared/src/keybindingRules.ts` | `packages/shared/src/keybindings.ts` |
| 68% | 73 | 152 | `apps/web/src/components/ui/sheet.tsx` | `apps/web/src/components/ui/sheet.tsx` |
| 68% | 37 | 53 | `apps/server/src/provider/Services/ProviderSessionDirectory.ts` | `apps/server/src/provider/Services/ProviderSessionDirectory.ts` |
| 68% | 25 | 45 | `apps/web/src/components/ui/preview-card.tsx` | `apps/web/src/components/ui/preview-card.tsx` |
| 67% | 6 | 8 | `apps/server/src/persistence/Migrations/020_ProjectionThreadsForkSource.ts` | `apps/server/src/persistence/Migrations/012_ProjectionThreadsInteractionMode.ts` |
| 67% | 6 | 8 | `apps/server/src/persistence/Migrations/024_ProjectionThreadsArchivedAt.ts` | `apps/server/src/persistence/Migrations/012_ProjectionThreadsInteractionMode.ts` |
| 67% | 97 | 265 | `apps/server/src/git/PrTemplateDetection.test.ts` | `apps/server/src/sourceControl/PrTemplateDetection.test.ts` |
| 66% | 78 | 171 | `apps/web/src/components/ui/command.tsx` | `apps/web/src/components/ui/command.tsx` |
| 66% | 35 | 63 | `packages/shared/src/Net.test.ts` | `packages/shared/src/Net.test.ts` |
| 66% | 79 | 112 | `apps/server/src/persistence/Layers/ProjectionThreadProposedPlans.ts` | `apps/server/src/persistence/Layers/ProjectionThreadProposedPlans.ts` |
| 65% | 76 | 167 | `apps/server/src/persistence/Layers/ProjectionThreadSessions.ts` | `apps/server/src/persistence/Layers/ProjectionThreadSessions.ts` |
| 64% | 47 | 77 | `apps/server/src/orchestration/Schemas.ts` | `apps/server/src/orchestration/Schemas.ts` |
| 64% | 26 | 34 | `apps/web/src/lib/terminalFocus.test.ts` | `apps/web/src/lib/terminalFocus.test.ts` |
| 64% | 38 | 83 | `apps/server/src/provider/Errors.ts` | `apps/server/src/provider/Errors.ts` |
| 63% | 6 | 12 | `apps/server/src/persistence/Migrations/017_ThreadHandoffMetadata.ts` | `apps/server/src/persistence/Migrations/007_ProjectionThreadMessageAttachments.ts` |
| 63% | 6 | 12 | `apps/server/src/persistence/Migrations/018_ProjectionThreadMessageMentions.ts` | `apps/server/src/persistence/Migrations/007_ProjectionThreadMessageAttachments.ts` |
| 63% | 51 | 86 | `apps/web/src/components/ui/popover.tsx` | `apps/web/src/components/ui/popover.tsx` |
| 63% | 76 | 184 | `packages/contracts/src/provider.ts` | `packages/contracts/src/provider.ts` |
| 63% | 28 | 50 | `apps/server/src/persistence/Services/OrchestrationCommandReceipts.ts` | `apps/server/src/persistence/Services/OrchestrationCommandReceipts.ts` |
| 62% | 6 | 9 | `apps/server/src/persistence/Migrations/049_ProjectionThreadMessagesDispatchOrigin.ts` | `apps/server/src/persistence/Migrations/007_ProjectionThreadMessageAttachments.ts` |
| 62% | 6 | 9 | `apps/server/src/persistence/Migrations/102_ProjectionThreadMessagesTurnBoundary.ts` | `apps/server/src/persistence/Migrations/007_ProjectionThreadMessageAttachments.ts` |
| 62% | 5 | 7 | `apps/server/src/provider/Services/ProviderSessionReaper.ts` | `apps/server/src/provider/Services/ProviderSessionReaper.ts` |
| 62% | 68 | 135 | `apps/web/src/pendingUserInput.ts` | `apps/web/src/pendingUserInput.ts` |
| 62% | 13 | 17 | `apps/web/src/components/chat/ComposerPendingTerminalContexts.tsx` | `apps/web/src/components/chat/ComposerPendingTerminalContexts.tsx` |
| 61% | 87 | 167 | `apps/web/src/terminal-links.test.ts` | `apps/web/src/terminal-links.test.ts` |
| 61% | 248 | 471 | `apps/server/integration/OrchestrationEngineHarness.integration.ts` | `apps/server/integration/OrchestrationEngineHarness.integration.ts` |
| 61% | 97 | 130 | `apps/server/src/persistence/Layers/ProjectionThreadActivities.ts` | `apps/server/src/persistence/Layers/ProjectionThreadActivities.ts` |
| 61% | 35 | 67 | `apps/web/src/components/ui/tooltip.tsx` | `apps/web/src/components/ui/tooltip.tsx` |
| 61% | 75 | 125 | `apps/web/src/lib/projectPaths.ts` | `packages/client-runtime/src/state/projects.ts` |
| 61% | 7 | 12 | `apps/server/src/persistence/Migrations/021_ProjectionThreadsAssociatedWorktree.ts` | `apps/server/src/persistence/Migrations/010_ProjectionThreadsRuntimeMode.ts` |
| 61% | 7 | 12 | `apps/server/src/persistence/Migrations/022_ProjectionThreadsAssociatedWorktreeBranch.ts` | `apps/server/src/persistence/Migrations/010_ProjectionThreadsRuntimeMode.ts` |
| 61% | 28 | 47 | `apps/web/src/components/ui/checkbox.tsx` | `apps/web/src/components/ui/checkbox.tsx` |
| 60% | 118 | 289 | `apps/web/src/components/ui/combobox.tsx` | `apps/web/src/components/ui/combobox.tsx` |
| 60% | 6 | 13 | `apps/server/src/persistence/Migrations/040_ProjectionThreadsPinnedMessagesNotes.ts` | `apps/server/src/persistence/Migrations/012_ProjectionThreadsInteractionMode.ts` |
| 60% | 6 | 10 | `apps/server/src/persistence/Migrations/041_ProjectionProjectsPinned.ts` | `apps/server/src/persistence/Migrations/039_ProjectionProjectsDefaultThreadEnvMode.ts` |
| 60% | 6 | 13 | `apps/server/src/persistence/Migrations/095_ProjectionThreadsGoalTiming.ts` | `apps/server/src/persistence/Migrations/012_ProjectionThreadsInteractionMode.ts` |
| 60% | 34 | 54 | `apps/web/src/components/ui/scroll-area.tsx` | `apps/web/src/components/ui/scroll-area.tsx` |
| 60% | 71 | 146 | `apps/server/src/persistence/Layers/ProjectionProjects.ts` | `apps/server/src/persistence/Layers/ProjectionProjects.ts` |
| 59% | 27 | 56 | `apps/server/src/persistence/Services/ProjectionProjects.ts` | `apps/server/src/persistence/Services/ProjectionProjects.ts` |
| 59% | 6 | 10 | `apps/server/src/persistence/Migrations/039_ReconcileLegacyPinnedThreads.ts` | `apps/server/src/persistence/Migrations/012_ProjectionThreadsInteractionMode.ts` |
| 58% | 7 | 13 | `apps/server/src/persistence/Migrations/019_ProjectionThreadsEnvMode.ts` | `apps/server/src/persistence/Migrations/010_ProjectionThreadsRuntimeMode.ts` |
| 58% | 41 | 100 | `apps/web/src/worktreeCleanup.test.ts` | `apps/web/src/worktreeCleanup.test.ts` |
| 58% | 8 | 16 | `apps/server/src/persistence/Migrations/068_GitHandoffOperations.ts` | `apps/server/src/persistence/Migrations/013_ProjectionThreadProposedPlans.ts` |
| 58% | 34 | 84 | `apps/server/src/persistence/Services/ProjectionThreadSessions.ts` | `apps/server/src/persistence/Services/ProjectionThreadSessions.ts` |
| 57% | 39 | 72 | `apps/web/src/projectScripts.ts` | `apps/web/src/projectScripts.ts` |
| 57% | 10 | 13 | `apps/server/src/orchestration/Layers/RuntimeReceiptBus.ts` | `apps/server/src/orchestration/Layers/RuntimeReceiptBus.ts` |
| 57% | 6 | 14 | `apps/server/src/persistence/Migrations/030_ProjectionThreadMessagesDispatchMode.ts` | `apps/server/src/persistence/Migrations/007_ProjectionThreadMessageAttachments.ts` |
| 57% | 7 | 17 | `apps/server/src/persistence/Migrations/097_ProjectionThreadsSidechatLifecycle.ts` | `apps/server/src/persistence/Migrations/010_ProjectionThreadsRuntimeMode.ts` |
| 56% | 18 | 29 | `apps/server/src/terminal/Services/PTY.ts` | `apps/server/src/terminal/PtyAdapter.ts` |
| 56% | 42 | 96 | `apps/server/src/persistence/Services/ProjectionThreadMessages.ts` | `apps/server/src/persistence/Services/ProjectionThreadMessages.ts` |
| 56% | 5 | 8 | `apps/server/src/orchestration/Services/ThreadDeletionReactor.ts` | `apps/server/src/orchestration/Services/ThreadDeletionReactor.ts` |
| 55% | 104 | 284 | `apps/web/src/composer-logic.ts` | `apps/web/src/composer-logic.ts` |
| 55% | 10 | 24 | `apps/server/src/orchestration/Services/ProjectionPipeline.ts` | `apps/server/src/orchestration/Services/ProjectionPipeline.ts` |
| 55% | 6 | 11 | `apps/server/src/persistence/Migrations/060_PendingApprovalLifecycleGeneration.ts` | `apps/server/src/persistence/Migrations/017_ProjectionThreadsArchivedAt.ts` |
| 54% | 97 | 174 | `apps/server/src/persistence/NodeSqliteClient.ts` | `packages/shared/src/nodeSqliteClient.ts` |
| 54% | 42 | 72 | `apps/server/src/workspace/Layers/WorkspacePaths.test.ts` | `apps/server/src/workspace/WorkspacePaths.test.ts` |
| 54% | 308 | 806 | `apps/web/src/components/ui/sidebar.tsx` | `apps/web/src/components/ui/sidebar.tsx` |
| 54% | 79 | 201 | `apps/desktop/scripts/dev-electron.mjs` | `apps/desktop/scripts/dev-electron.mjs` |
| 53% | 94 | 233 | `apps/web/src/components/ui/select.tsx` | `apps/web/src/components/ui/select.tsx` |
| 53% | 81 | 189 | `apps/server/integration/providerService.integration.test.ts` | `apps/server/integration/providerService.integration.test.ts` |
| 53% | 5 | 9 | `apps/server/src/persistence/Migrations/131_ProjectSourceFolders.ts` | `apps/server/src/persistence/Migrations/039_ProjectionProjectsDefaultThreadEnvMode.ts` |
| 53% | 5 | 9 | `apps/server/src/persistence/Migrations/133_ProjectionTurnsWorkspaceInitialization.ts` | `apps/server/src/persistence/Migrations/015_ProjectionTurnsSourceProposedPlan.ts` |
| 52% | 52 | 140 | `apps/web/src/threadSelectionStore.test.ts` | `apps/web/src/threadSelectionStore.test.ts` |
| 51% | 51 | 100 | `packages/shared/src/Net.ts` | `packages/shared/src/Net.ts` |
| 51% | 9 | 25 | `apps/server/src/attachmentPaths.ts` | `apps/server/src/attachmentPaths.ts` |
| 51% | 109 | 374 | `apps/web/src/components/ui/menu.tsx` | `apps/web/src/components/ui/menu.tsx` |
| 51% | 45 | 49 | `apps/web/src/projectScripts.test.ts` | `apps/web/src/projectScripts.test.ts` |
| 50% | 5 | 10 | `apps/server/src/orchestration/Services/ProviderRuntimeIngestion.ts` | `apps/server/src/orchestration/Services/ProviderRuntimeIngestion.ts` |
| 50% | 7 | 13 | `apps/server/src/persistence/Migrations/033_ProjectionThreadsSidechatSource.ts` | `apps/server/src/persistence/Migrations/017_ProjectionThreadsArchivedAt.ts` |
| 50% | 6 | 9 | `apps/server/src/persistence/Migrations/085_AutomationSettings.ts` | `apps/server/src/persistence/Migrations/013_ProjectionThreadProposedPlans.ts` |
| 50% | 3 | 7 | `apps/server/src/persistence/Migrations/134_ProjectionThreadsForkSourceMessage.ts` | `apps/server/src/persistence/Migrations/052_ProjectionThreadTitleState.ts` |
| 50% | 3 | 7 | `apps/server/src/persistence/Migrations/139_RemoteConnectionPreferences.ts` | `apps/server/src/persistence/Migrations/052_ProjectionThreadTitleState.ts` |
| 50% | 3 | 7 | `apps/server/src/provider/Services/CodexAdapter.ts` | `apps/server/src/provider/Services/CodexAdapter.ts` |
| 50% | 3 | 6 | `apps/web/src/lib/terminalFocus.ts` | `apps/web/src/lib/previewFocus.ts` |
| 50% | 48 | 104 | `apps/server/src/terminal/Layers/NodePTY.ts` | `apps/server/src/terminal/NodePtyAdapter.ts` |
| 49% | 74 | 165 | `apps/server/src/persistence/Layers/AuthPairingLinks.ts` | `apps/server/src/persistence/AuthPairingLinks.ts` |
| 49% | 248 | 582 | `apps/server/src/provider/acp/AcpRuntimeModel.ts` | `apps/server/src/provider/acp/AcpRuntimeModel.ts` |
| 49% | 84 | 205 | `apps/server/src/provider/Layers/ProviderSessionDirectory.ts` | `apps/server/src/provider/Layers/ProviderSessionDirectory.ts` |
| 49% | 55 | 87 | `apps/server/src/attachmentStore.ts` | `apps/server/src/attachmentStore.ts` |
| 48% | 7 | 11 | `apps/web/src/vite-env.d.ts` | `apps/web/src/vite-env.d.ts` |
| 48% | 7 | 14 | `apps/server/src/persistence/Migrations/029_ProjectionThreadsLastKnownPr.ts` | `apps/server/src/persistence/Migrations/017_ProjectionThreadsArchivedAt.ts` |
| 48% | 7 | 14 | `apps/server/src/persistence/Migrations/031_ProjectionThreadsCreateBranchFlowCompleted.ts` | `apps/server/src/persistence/Migrations/017_ProjectionThreadsArchivedAt.ts` |
| 48% | 6 | 12 | `apps/web/src/hooks/useTurnDiffSummaries.ts` | `apps/web/src/hooks/useTurnDiffSummaries.ts` |
| 48% | 110 | 204 | `apps/server/src/persistence/Layers/AuthSessions.ts` | `apps/server/src/persistence/AuthSessions.ts` |
| 48% | 141 | 286 | `apps/web/src/components/chat/ProjectActionDialog.tsx` | `apps/web/src/components/projectScriptEditor.tsx` |
| 48% | 113 | 274 | `apps/web/src/components/PullRequestThreadDialog.tsx` | `apps/web/src/components/PullRequestThreadDialog.tsx` |
| 48% | 6 | 14 | `apps/server/src/persistence/Migrations/088_ProjectionThreadsSettledAt.ts` | `apps/server/src/persistence/Migrations/012_ProjectionThreadsInteractionMode.ts` |
| 47% | 62 | 278 | `apps/server/src/provider/acp/AcpCoreRuntimeEvents.test.ts` | `apps/server/src/provider/acp/AcpCoreRuntimeEvents.test.ts` |
| 47% | 7 | 19 | `apps/server/src/persistence/Migrations/143_UsageAnalyticsDeletedTokens.ts` | `apps/server/src/persistence/Migrations/013_ProjectionThreadProposedPlans.ts` |
| 47% | 16 | 32 | `apps/server/src/orchestration/Layers/OrchestrationReactor.ts` | `apps/server/src/orchestration/Layers/OrchestrationReactor.ts` |
| 46% | 276 | 1139 | `apps/server/src/orchestration/projector.ts` | `apps/server/src/orchestration/projector.ts` |
| 46% | 17 | 34 | `apps/server/src/auth/Layers/ServerAuthPolicy.ts` | `apps/server/src/auth/EnvironmentAuthPolicy.ts` |
| 46% | 6 | 15 | `apps/server/src/persistence/Migrations/107_ProjectionThreadsHumanMessage.ts` | `apps/server/src/persistence/Migrations/010_ProjectionThreadsRuntimeMode.ts` |
| 46% | 68 | 128 | `packages/contracts/src/terminal.ts` | `packages/contracts/src/terminal.ts` |
| 46% | 14 | 45 | `apps/web/src/lib/lruCache.test.ts` | `apps/web/src/lib/lruCache.test.ts` |
| 46% | 312 | 1506 | `apps/server/src/orchestration/projector.test.ts` | `apps/server/src/orchestration/projector.test.ts` |
| 46% | 187 | 415 | `scripts/dev-runner.ts` | `scripts/dev-runner.ts` |
| 45% | 6 | 9 | `apps/server/src/persistence/Migrations/059_ProviderSessionLifecycleGeneration.ts` | `apps/server/src/persistence/Migrations/027_ProviderSessionRuntimeInstanceId.ts` |
| 45% | 67 | 106 | `packages/contracts/src/auth.ts` | `packages/contracts/src/auth.ts` |
| 45% | 31 | 47 | `apps/server/src/imageMime.ts` | `apps/server/src/imageMime.ts` |
| 45% | 76 | 341 | `apps/web/src/components/desktopUpdate.logic.test.ts` | `apps/web/src/components/desktopUpdate.logic.test.ts` |
| 44% | 4 | 9 | `apps/server/src/orchestration/Services/OrchestrationReactor.ts` | `apps/server/src/orchestration/Services/OrchestrationReactor.ts` |
| 44% | 3 | 4 | `apps/server/src/persistence/Migrations/009_ProviderSessionRuntimeMode.ts` | `apps/server/src/persistence/Migrations/052_ProjectionThreadTitleState.ts` |
| 44% | 4 | 8 | `apps/server/src/persistence/Migrations/072_AgentGatewayOperationRetention.ts` | `apps/server/src/persistence/Migrations/051_ProjectionThreadMessageContext.ts` |
| 44% | 381 | 1535 | `apps/server/src/provider/Layers/CursorAdapter.ts` | `apps/server/src/provider/Layers/CursorAdapter.ts` |
| 44% | 35 | 51 | `apps/server/src/attachmentStore.test.ts` | `apps/server/src/attachmentStore.test.ts` |
| 44% | 11 | 20 | `apps/web/src/components/ui/switch.tsx` | `apps/web/src/components/ui/switch.tsx` |
| 44% | 54 | 227 | `apps/server/src/orchestration/commandInvariants.test.ts` | `apps/server/src/orchestration/commandInvariants.test.ts` |
| 44% | 6 | 22 | `apps/server/src/persistence/Migrations/063_ProjectionMessageCausalSequence.ts` | `apps/server/src/persistence/Migrations/008_ProjectionThreadActivitySequence.ts` |
| 43% | 5 | 13 | `apps/server/src/persistence/Migrations/090_ProjectionThreadMessageTextSegments.ts` | `apps/server/src/persistence/Migrations/051_ProjectionThreadMessageContext.ts` |
| 43% | 135 | 389 | `packages/contracts/src/git.ts` | `packages/contracts/src/git.ts` |
| 43% | 76 | 218 | `apps/desktop/src/updateMachine.test.ts` | `apps/desktop/src/updates/updateMachine.test.ts` |
| 43% | 81 | 146 | `apps/server/src/persistence/Layers/ProviderSessionRuntime.ts` | `apps/server/src/persistence/ProviderSessionRuntime.ts` |
| 42% | 63 | 172 | `apps/server/src/orchestration/Services/ProjectionSnapshotQuery.ts` | `apps/server/src/orchestration/Services/ProjectionSnapshotQuery.ts` |
| 42% | 26 | 58 | `apps/server/src/checkpointing/Services/CheckpointStore.ts` | `apps/server/src/checkpointing/CheckpointStore.ts` |
| 42% | 6 | 14 | `apps/server/src/persistence/Migrations/028_ProjectionProjectsKind.ts` | `apps/server/src/persistence/Migrations/039_ProjectionProjectsDefaultThreadEnvMode.ts` |
| 41% | 32 | 66 | `apps/server/src/provider/Layers/ProviderSessionReaper.ts` | `apps/server/src/provider/Layers/ProviderSessionReaper.ts` |
| 41% | 9 | 21 | `apps/server/src/orchestration/runtimeLayer.ts` | `apps/server/src/orchestration/runtimeLayer.ts` |
| 41% | 163 | 654 | `apps/web/src/keybindings.ts` | `apps/web/src/keybindings.ts` |
| 41% | 23 | 82 | `apps/server/src/orchestration/Errors.ts` | `apps/server/src/orchestration/Errors.ts` |
| 40% | 39 | 125 | `apps/server/src/persistence/Services/ProjectionThreads.ts` | `apps/server/src/persistence/Services/ProjectionThreads.ts` |
| 40% | 38 | 71 | `apps/server/src/workspace/Layers/WorkspacePaths.ts` | `apps/server/src/workspace/WorkspacePaths.ts` |
| 40% | 61 | 209 | `apps/server/scripts/cli.ts` | `apps/server/scripts/cli.ts` |
| 40% | 5 | 15 | `apps/server/src/persistence/Migrations/025_ProjectionThreadsSubagents.ts` | `apps/server/src/persistence/Migrations/051_ProjectionThreadMessageContext.ts` |
| 40% | 6 | 19 | `apps/server/src/persistence/Migrations/061_PendingApprovalSettlementState.ts` | `apps/server/src/persistence/Migrations/017_ProjectionThreadsArchivedAt.ts` |
| 40% | 6 | 21 | `apps/server/src/persistence/Migrations/101_RemoveTranscriptMarkers.ts` | `apps/server/src/persistence/Migrations/011_OrchestrationThreadCreatedRuntimeMode.ts` |
| 40% | 5 | 10 | `apps/server/src/persistence/Migrations/127_ProjectImportHistory.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 40% | 2 | 7 | `apps/server/src/provider/Services/CursorAdapter.ts` | `apps/server/src/provider/Services/OpenCodeAdapter.ts` |
| 40% | 2 | 7 | `apps/server/src/provider/Services/DroidAdapter.ts` | `apps/server/src/provider/Services/OpenCodeAdapter.ts` |
| 40% | 2 | 7 | `apps/server/src/provider/Services/GrokAdapter.ts` | `apps/server/src/provider/Services/OpenCodeAdapter.ts` |
| 40% | 2 | 7 | `apps/server/src/provider/Services/OmpAdapter.ts` | `apps/server/src/provider/Services/OpenCodeAdapter.ts` |
| 40% | 2 | 7 | `apps/server/src/provider/Services/OpenCodeAdapter.ts` | `apps/server/src/provider/Services/OpenCodeAdapter.ts` |
| 40% | 2 | 7 | `apps/server/src/provider/Services/PiAdapter.ts` | `apps/server/src/provider/Services/OpenCodeAdapter.ts` |
| 40% | 6 | 14 | `apps/web/src/components/RowHoverActions.tsx` | `apps/web/src/components/settings/SettingsGroup.tsx` |
| 40% | 5 | 10 | `packages/profile-ui/src/skeleton.tsx` | `apps/web/src/components/ui/kbd.tsx` |
| 40% | 9 | 17 | `scripts/lib/resolve-catalog.ts` | `scripts/lib/resolve-catalog.ts` |
| 40% | 39 | 68 | `apps/server/src/auth/utils.ts` | `apps/server/src/auth/utils.ts` |
| 39% | 10 | 46 | `apps/web/src/components/ui/kbd.tsx` | `apps/web/src/components/ui/kbd.tsx` |
| 39% | 41 | 105 | `packages/shared/src/logging.ts` | `packages/shared/src/logging.ts` |
| 39% | 10 | 12 | `apps/web/src/pullRequestReference.test.ts` | `apps/web/src/pullRequestReference.test.ts` |
| 39% | 10 | 39 | `apps/web/src/router.ts` | `apps/web/src/router.ts` |
| 39% | 37 | 83 | `apps/web/src/components/ui/toast.logic.test.ts` | `apps/web/src/components/ui/toast.logic.test.ts` |
| 39% | 6 | 21 | `apps/server/src/persistence/Migrations/067_ProviderDeliveryReconciliation.ts` | `apps/server/src/persistence/Migrations/013_ProjectionThreadProposedPlans.ts` |
| 39% | 6 | 25 | `apps/server/src/persistence/Migrations/089_RecoverRetentionHiddenThreads.ts` | `apps/server/src/persistence/Migrations/010_ProjectionThreadsRuntimeMode.ts` |
| 39% | 6 | 25 | `apps/server/src/persistence/Migrations/092_BackfillAutomationRunThreadSource.ts` | `apps/server/src/persistence/Migrations/010_ProjectionThreadsRuntimeMode.ts` |
| 39% | 73 | 188 | `packages/contracts/src/keybindings.ts` | `packages/contracts/src/keybindings.ts` |
| 39% | 74 | 199 | `packages/contracts/src/keybindings.test.ts` | `packages/contracts/src/keybindings.test.ts` |
| 38% | 38 | 95 | `apps/web/src/components/ui/button.tsx` | `apps/web/src/components/ui/button.tsx` |
| 38% | 520 | 2593 | `packages/contracts/src/orchestration.ts` | `packages/contracts/src/orchestration.ts` |
| 38% | 7 | 31 | `apps/server/src/persistence/Migrations/086_NormalizeStudioThreadWorkspaces.ts` | `apps/server/src/persistence/Migrations/010_ProjectionThreadsRuntimeMode.ts` |
| 38% | 11 | 50 | `apps/server/src/persistence/Migrations/118_ProjectionThreadSessionProviderInstance.ts` | `apps/server/src/persistence/Migrations/028_ProjectionThreadSessionInstanceId.ts` |
| 38% | 18 | 20 | `apps/server/src/persistence/NodeSqliteClient.test.ts` | `packages/shared/src/nodeSqliteClient.test.ts` |
| 38% | 6 | 22 | `apps/server/src/persistence/Migrations/079_Spaces.ts` | `apps/server/src/persistence/Migrations/013_ProjectionThreadProposedPlans.ts` |
| 38% | 17 | 62 | `apps/web/src/lib/syntaxHighlighting.ts` | `apps/web/src/lib/syntaxHighlighting.ts` |
| 38% | 5 | 13 | `apps/server/src/persistence/Migrations/043_ProfileStatsIndexes.ts` | `apps/server/src/persistence/Migrations/002_OrchestrationCommandReceipts.ts` |
| 38% | 3 | 11 | `apps/server/src/persistence/Migrations/140_RemoteAccessControls.ts` | `apps/server/src/persistence/Migrations/052_ProjectionThreadTitleState.ts` |
| 38% | 3 | 11 | `apps/server/src/persistence/Migrations/142_AuthAccessOrigin.ts` | `apps/server/src/persistence/Migrations/052_ProjectionThreadTitleState.ts` |
| 37% | 30 | 42 | `apps/server/src/serverRuntimeState.ts` | `apps/server/src/serverRuntimeState.ts` |
| 37% | 34 | 60 | `apps/server/src/provider/cliVersion.ts` | `packages/shared/src/semver.ts` |
| 37% | 5 | 12 | `apps/server/src/persistence/Migrations/136_AccountUsageSync.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 37% | 143 | 682 | `apps/web/src/components/GitActionsControl.logic.ts` | `packages/client-runtime/src/state/gitActions.ts` |
| 37% | 91 | 423 | `apps/server/src/git/Layers/OpenCodeTextGeneration.test.ts` | `apps/server/src/textGeneration/OpenCodeTextGeneration.test.ts` |
| 37% | 11 | 52 | `apps/server/src/persistence/Migrations/119_ProviderSessionRuntimeInstanceId.ts` | `apps/server/src/persistence/Migrations/027_ProviderSessionRuntimeInstanceId.ts` |
| 37% | 28 | 93 | `apps/server/src/orchestration/Layers/OrchestrationReactor.test.ts` | `apps/server/src/orchestration/Layers/OrchestrationReactor.test.ts` |
| 36% | 5 | 7 | `apps/server/src/persistence/Migrations/052_ProjectionThreadUserMessageSummaryIndex.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 36% | 7 | 29 | `apps/server/src/persistence/Migrations/065_DurableQueuedTurnPromotions.ts` | `apps/server/src/persistence/Migrations/013_ProjectionThreadProposedPlans.ts` |
| 36% | 6 | 17 | `apps/server/src/persistence/Migrations/094_ProjectionThreadsGoal.test.ts` | `apps/server/src/persistence/Migrations/042_ProjectionThreadLinkedPullRequest.test.ts` |
| 36% | 5 | 7 | `apps/server/src/persistence/Migrations/099_InvalidateProjectionThreadsCursor.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 36% | 5 | 7 | `apps/server/src/persistence/Migrations/124_ProjectionTurnsPendingMessageIndex.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 36% | 5 | 7 | `apps/server/src/persistence/Migrations/135_ProjectionMessageTurnActivityIndex.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 36% | 5 | 13 | `apps/server/src/persistence/Migrations/083_AutomationHeartbeatEligibility.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 36% | 36 | 123 | `apps/web/src/components/desktopUpdate.logic.ts` | `apps/web/src/components/desktopUpdate.logic.ts` |
| 36% | 35 | 69 | `packages/contracts/src/baseSchemas.ts` | `packages/contracts/src/baseSchemas.ts` |
| 36% | 43 | 103 | `packages/contracts/src/providerRuntime.test.ts` | `packages/contracts/src/providerRuntime.test.ts` |
| 35% | 39 | 81 | `packages/contracts/src/terminal.test.ts` | `packages/contracts/src/terminal.test.ts` |
| 35% | 77 | 322 | `apps/web/src/components/BranchToolbar.logic.test.ts` | `apps/web/src/components/BranchToolbar.logic.test.ts` |
| 35% | 42 | 62 | `apps/web/src/components/chat/ProposedPlanCard.tsx` | `apps/web/src/components/chat/ProposedPlanCard.tsx` |
| 35% | 6 | 21 | `apps/server/src/persistence/Migrations/120_ProfileStatsDeletedProviderInstances.ts` | `apps/server/src/persistence/Migrations/027_ProviderSessionRuntimeInstanceId.ts` |
| 35% | 6 | 25 | `apps/server/src/persistence/Migrations/123_ScrubOrchestrationEventProviderOptions.ts` | `apps/server/src/persistence/Migrations/011_OrchestrationThreadCreatedRuntimeMode.ts` |
| 35% | 52 | 194 | `apps/server/src/persistence/Migrations/032_ReconcileImportedSchemaLineage.test.ts` | `apps/server/src/persistence/Migrations/024_BackfillProjectionThreadShellSummary.test.ts` |
| 35% | 192 | 693 | `apps/server/scripts/acp-mock-agent.ts` | `apps/server/scripts/acp-mock-agent.ts` |
| 35% | 62 | 526 | `apps/server/src/orchestration/decider.projectScripts.test.ts` | `apps/server/src/orchestration/decider.projectScripts.test.ts` |
| 35% | 31 | 148 | `packages/shared/src/DrainableWorker.test.ts` | `packages/shared/src/DrainableWorker.test.ts` |
| 35% | 201 | 617 | `apps/web/src/components/ui/toast.tsx` | `apps/web/src/components/ui/toast.tsx` |
| 35% | 44 | 203 | `apps/server/src/provider/Services/ProviderAdapter.ts` | `apps/server/src/provider/Services/ProviderAdapter.ts` |
| 34% | 9 | 43 | `apps/server/src/persistence/Migrations/066_DurableProviderRuntimeEvents.ts` | `apps/server/src/persistence/Migrations/013_ProjectionThreadProposedPlans.ts` |
| 34% | 5 | 14 | `apps/web/src/components/chat/ComposerPlanFollowUpBanner.tsx` | `apps/web/src/components/chat/ComposerPlanFollowUpBanner.tsx` |
| 34% | 9 | 25 | `apps/web/src/components/pullRequest/PullRequestDiffStat.tsx` | `apps/web/src/components/chat/DiffStatLabel.tsx` |
| 34% | 7 | 23 | `apps/server/src/persistence/Migrations/040_ProjectionThreadsPinnedMessagesNotes.test.ts` | `apps/server/src/persistence/Migrations/040_ProjectionProjectFaviconPath.test.ts` |
| 34% | 25 | 101 | `apps/desktop/scripts/smoke-test.mjs` | `apps/desktop/scripts/smoke-test.mjs` |
| 34% | 8 | 19 | `apps/server/src/provider/Services/ProviderAdapterRegistry.ts` | `apps/server/src/provider/Services/ProviderAdapterRegistry.ts` |
| 34% | 77 | 723 | `apps/web/src/pendingInteractionDerivation.test.ts` | `packages/client-runtime/src/pendingRequests.test.ts` |
| 34% | 131 | 652 | `apps/server/src/persistence/Layers/OrchestrationEventStore.ts` | `apps/server/src/persistence/Layers/OrchestrationEventStore.ts` |
| 34% | 24 | 69 | `apps/web/src/components/ui/input.tsx` | `apps/web/src/components/ui/input.tsx` |
| 33% | 54 | 386 | `apps/server/src/orchestration/commandInvariants.ts` | `apps/server/src/orchestration/commandInvariants.ts` |
| 33% | 5 | 9 | `apps/server/src/persistence/Migrations/046_AutomationCompletionPolicy.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 33% | 5 | 9 | `apps/server/src/persistence/Migrations/051_ProfileStatsDeletedTokensModel.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 33% | 5 | 20 | `apps/server/src/persistence/Migrations/071_ProjectionThreadsGatewayProvenance.ts` | `apps/server/src/persistence/Migrations/051_ProjectionThreadMessageContext.ts` |
| 33% | 5 | 15 | `apps/server/src/persistence/Migrations/106_ProjectImportOrigins.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 33% | 3 | 13 | `apps/server/src/persistence/Migrations/112_CoordinatorAppearance.ts` | `apps/server/src/persistence/Migrations/052_ProjectionThreadTitleState.ts` |
| 33% | 5 | 9 | `apps/server/src/persistence/Migrations/137_AccountUsageSyncIdentity.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 33% | 2 | 9 | `apps/server/src/provider/Services/AntigravityAdapter.ts` | `apps/server/src/provider/Services/OpenCodeAdapter.ts` |
| 33% | 2 | 4 | `apps/web/src/routes/_chat.groups.index.tsx` | `apps/web/src/routes/connect.tsx` |
| 33% | 2 | 4 | `apps/web/src/routes/_chat.studio.index.tsx` | `apps/web/src/routes/connect.tsx` |
| 33% | 72 | 320 | `apps/server/src/provider/Layers/ProviderSessionDirectory.test.ts` | `apps/server/src/provider/Layers/ProviderSessionDirectory.test.ts` |
| 33% | 110 | 410 | `apps/server/src/persistence/Layers/ProjectionThreads.ts` | `apps/server/src/persistence/Layers/ProjectionThreads.ts` |
| 33% | 35 | 72 | `scripts/merge-mac-update-manifests.test.ts` | `scripts/merge-update-manifests.test.ts` |
| 33% | 21 | 71 | `apps/web/src/threadSelectionStore.ts` | `apps/web/src/threadSelectionStore.ts` |
| 33% | 161 | 1298 | `apps/server/src/keybindings.test.ts` | `apps/server/src/keybindings.test.ts` |
| 32% | 50 | 354 | `apps/server/src/persistence/Layers/ProjectionThreadMessages.test.ts` | `apps/server/src/persistence/Layers/ProjectionThreadMessages.test.ts` |
| 32% | 10 | 16 | `packages/contracts/src/filesystem.ts` | `packages/contracts/src/filesystem.ts` |
| 32% | 49 | 275 | `apps/server/src/git/Layers/CursorTextGeneration.test.ts` | `apps/server/src/textGeneration/CursorTextGeneration.test.ts` |
| 32% | 77 | 237 | `apps/server/src/config.ts` | `apps/server/src/config.ts` |
| 32% | 5 | 10 | `apps/server/src/persistence/Migrations/081_AutomationProposals.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 32% | 5 | 10 | `apps/server/src/persistence/Migrations/082_AutomationMemory.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 32% | 93 | 357 | `apps/server/src/persistence/Layers/ProjectionThreadMessages.ts` | `apps/server/src/persistence/Layers/ProjectionThreadMessages.ts` |
| 32% | 8 | 49 | `apps/marketing/src/components/ui/card.tsx` | `apps/web/src/components/ui/number-field.tsx` |
| 32% | 9 | 14 | `apps/web/src/pullRequestReference.ts` | `apps/web/src/pullRequestReference.ts` |
| 32% | 20 | 40 | `apps/server/src/workspace/Services/WorkspacePaths.ts` | `apps/server/src/workspace/WorkspacePaths.ts` |
| 32% | 3 | 14 | `apps/server/src/persistence/Migrations/080_ExternalMcpProjectScope.ts` | `apps/server/src/persistence/Migrations/052_ProjectionThreadTitleState.ts` |
| 31% | 5 | 21 | `apps/server/src/persistence/Migrations/144_ProjectionThreadsPullRequestLinks.ts` | `apps/server/src/persistence/Migrations/010_ProjectionThreadsRuntimeMode.ts` |
| 31% | 43 | 122 | `apps/web/src/hooks/useLocalStorage.ts` | `apps/web/src/hooks/useLocalStorage.ts` |
| 31% | 69 | 170 | `packages/shared/src/shell.test.ts` | `packages/shared/src/shell.test.ts` |
| 31% | 38 | 186 | `apps/server/src/provider/Services/ProviderService.ts` | `apps/server/src/provider/Services/ProviderService.ts` |
| 31% | 4 | 20 | `apps/server/src/persistence/Migrations/073_OperationalDiagnostics.ts` | `apps/server/src/persistence/Migrations/002_OrchestrationCommandReceipts.ts` |
| 31% | 2 | 10 | `apps/server/src/provider/Services/DevinAdapter.ts` | `apps/server/src/provider/Services/OpenCodeAdapter.ts` |
| 30% | 488 | 2051 | `apps/server/src/git/Layers/GitManager.ts` | `apps/server/src/git/GitManager.ts` |
| 30% | 19 | 65 | `apps/web/src/components/DiffPanelShell.tsx` | `apps/web/src/components/DiffPanelShell.tsx` |
| 30% | 21 | 47 | `apps/web/src/components/ui/alert.tsx` | `apps/web/src/components/ui/alert.tsx` |
| 30% | 5 | 25 | `apps/server/src/persistence/Migrations/070_AgentGatewayOperations.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 30% | 105 | 646 | `apps/server/src/provider/acp/AcpRuntimeModel.test.ts` | `apps/server/src/provider/acp/AcpRuntimeModel.test.ts` |
| 30% | 15 | 73 | `apps/server/src/persistence/Services/ProjectionPendingInteractions.ts` | `apps/server/src/persistence/Services/ProjectionPendingApprovals.ts` |
| 30% | 5 | 12 | `apps/server/src/persistence/Migrations/084_AutomationNotificationPolicy.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 30% | 5 | 12 | `apps/server/src/persistence/Migrations/093_BackfillMaxIterationsDisabledReason.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 30% | 164 | 882 | `apps/server/src/keybindings.ts` | `apps/server/src/keybindings.ts` |
| 30% | 22 | 80 | `apps/server/src/orchestration/Services/OrchestrationEngine.ts` | `apps/server/src/orchestration/Services/OrchestrationEngine.ts` |
| 29% | 48 | 125 | `apps/server/src/project/Layers/ProjectFaviconResolver.ts` | `apps/server/src/project/ProjectFaviconResolver.ts` |
| 29% | 45 | 147 | `apps/web/src/hooks/useCopyToClipboard.ts` | `apps/web/src/hooks/useCopyToClipboard.ts` |
| 29% | 5 | 26 | `apps/server/src/persistence/Migrations/110_Groups.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 29% | 89 | 431 | `apps/server/src/git/Layers/ClaudeTextGeneration.ts` | `apps/server/src/textGeneration/ClaudeTextGeneration.ts` |
| 29% | 382 | 1900 | `apps/server/src/orchestration/Layers/ProjectionPipeline.ts` | `apps/server/src/orchestration/Layers/ProjectionPipeline.ts` |
| 29% | 23 | 38 | `apps/web/src/components/DiffWorkerPoolProvider.tsx` | `apps/web/src/components/DiffWorkerPoolProvider.tsx` |
| 29% | 15 | 26 | `apps/web/src/editorPreferences.ts` | `apps/web/src/editorPreferences.ts` |
| 29% | 90 | 243 | `apps/web/src/composer-logic.test.ts` | `apps/web/src/composer-logic.test.ts` |
| 29% | 15 | 76 | `apps/web/src/lib/storage.ts` | `apps/web/src/lib/storage.ts` |
| 29% | 5 | 25 | `apps/server/src/persistence/Migrations/045_AutomationPolicies.ts` | `apps/server/src/persistence/Migrations/001_OrchestrationEvents.ts` |
| 29% | 4 | 6 | `apps/server/src/persistence/Migrations/087_DropUnusedOrchestrationEventIndexes.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 29% | 4 | 17 | `apps/server/src/persistence/Migrations/114_ProjectAgentLifecycle.ts` | `apps/server/src/persistence/Migrations/017_ProjectionThreadsArchivedAt.ts` |
| 29% | 5 | 20 | `apps/server/src/persistence/Migrations/130_PullRequestAutoFix.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 29% | 1 | 4 | `apps/web/src/routes/_chat.link.tsx` | `apps/web/src/routes/_chat.$environmentId.$threadId.tsx` |
| 29% | 1 | 4 | `apps/web/src/routes/_chat.plugins.tsx` | `apps/web/src/routes/_chat.$environmentId.$threadId.tsx` |
| 29% | 21 | 32 | `packages/contracts/src/environment.ts` | `packages/contracts/src/environment.ts` |
| 28% | 40 | 201 | `apps/server/src/git/Services/TextGeneration.ts` | `apps/server/src/textGeneration/TextGeneration.ts` |
| 28% | 1094 | 6677 | `apps/server/src/provider/Layers/ClaudeAdapter.ts` | `apps/server/src/provider/Layers/ClaudeAdapter.ts` |
| 28% | 13 | 50 | `apps/server/src/stream/collectUint8StreamText.ts` | `apps/server/src/stream/collectUint8StreamText.ts` |
| 28% | 22 | 63 | `apps/web/src/components/ui/toast.logic.ts` | `apps/web/src/components/ui/toast.logic.ts` |
| 28% | 5 | 21 | `apps/server/src/persistence/Migrations/125_Todos.ts` | `apps/server/src/persistence/Migrations/013_ProjectionThreadProposedPlans.ts` |
| 28% | 15 | 55 | `apps/server/src/persistence/Services/OrchestrationEventStore.ts` | `apps/server/src/persistence/Services/OrchestrationEventStore.ts` |
| 28% | 386 | 2062 | `apps/server/src/git/Layers/GitManager.test.ts` | `apps/server/src/git/GitManager.test.ts` |
| 27% | 265 | 1300 | `apps/server/src/provider/opencodeRuntime.ts` | `apps/server/src/provider/opencodeRuntime.ts` |
| 27% | 6 | 25 | `apps/server/src/persistence/Migrations/069_ProjectPullRequestPins.ts` | `apps/server/src/persistence/Migrations/025_CleanupInvalidProjectionPendingApprovals.ts` |
| 27% | 4 | 7 | `apps/server/src/persistence/Migrations/105_AsyncUserInput.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 27% | 3 | 17 | `apps/server/src/persistence/Migrations/111_GroupLibraryHosting.ts` | `apps/server/src/persistence/Migrations/052_ProjectionThreadTitleState.ts` |
| 27% | 28 | 90 | `apps/server/src/provider/providerStatusCache.ts` | `apps/server/src/provider/providerStatusCache.ts` |
| 27% | 118 | 942 | `apps/server/src/provider/acp/AcpJsonRpcConnection.test.ts` | `apps/server/src/provider/acp/AcpJsonRpcConnection.test.ts` |
| 27% | 10 | 40 | `apps/marketing/src/components/ui/alert.tsx` | `apps/web/src/components/ui/empty.tsx` |
| 27% | 102 | 233 | `packages/contracts/src/pullRequests.ts` | `packages/contracts/src/pullRequest.ts` |
| 27% | 9 | 38 | `apps/server/src/persistence/Migrations/126_ProjectionThreadsSidechatContext.test.ts` | `apps/server/src/persistence/Migrations/054_ProjectionThreadsAutoSettleDisabledAt.test.ts` |
| 27% | 124 | 841 | `apps/web/src/components/GitActionsControl.logic.test.ts` | `apps/web/src/components/GitActionsControl.logic.test.ts` |
| 27% | 52 | 285 | `apps/server/src/checkpointing/Layers/CheckpointDiffQuery.ts` | `apps/server/src/checkpointing/CheckpointDiffQuery.ts` |
| 27% | 394 | 2107 | `apps/server/src/provider/Layers/CodexAdapter.ts` | `apps/server/src/provider/Layers/CodexAdapter.ts` |
| 27% | 59 | 151 | `apps/web/src/pendingUserInput.test.ts` | `apps/web/src/pendingUserInput.test.ts` |
| 27% | 88 | 204 | `packages/shared/src/shell.ts` | `packages/shared/src/shell.ts` |
| 27% | 2 | 12 | `apps/server/src/provider/Services/ClaudeAdapter.ts` | `apps/server/src/provider/Services/OpenCodeAdapter.ts` |
| 27% | 2 | 7 | `apps/web/src/routes/_chat.inbox.tsx` | `apps/web/src/routes/connect.tsx` |
| 27% | 2 | 7 | `apps/web/src/routes/_chat.tasks.index.tsx` | `apps/web/src/routes/connect.tsx` |
| 27% | 14 | 73 | `packages/shared/src/path.ts` | `packages/shared/src/path.ts` |
| 27% | 54 | 334 | `apps/web/src/lib/contextWindow.ts` | `apps/web/src/lib/contextWindow.ts` |
| 27% | 44 | 88 | `apps/web/src/components/ThreadWorktreeHandoffDialog.tsx` | `apps/web/src/components/PullRequestThreadDialog.tsx` |
| 26% | 5 | 37 | `apps/server/src/persistence/Migrations/128_HubWork.ts` | `apps/server/src/persistence/Migrations/001_OrchestrationEvents.ts` |
| 26% | 5 | 23 | `apps/server/src/persistence/Migrations/100_MessageTextChunks.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 26% | 39 | 114 | `apps/server/src/persistence/Services/AuthPairingLinks.ts` | `apps/server/src/persistence/AuthPairingLinks.ts` |
| 26% | 282 | 1903 | `apps/server/src/provider/Layers/GrokAdapter.ts` | `apps/server/src/provider/Layers/CursorAdapter.ts` |
| 26% | 6 | 20 | `apps/server/src/persistence/Migrations/088_ProjectionThreadsSettledAt.test.ts` | `apps/server/src/persistence/Migrations/051_ProjectionThreadMessageContext.test.ts` |
| 26% | 3 | 18 | `apps/server/src/persistence/Migrations/108_GatewayCompletions.ts` | `apps/server/src/persistence/Migrations/052_ProjectionThreadTitleState.ts` |
| 26% | 7 | 25 | `apps/server/src/persistence/Migrations/102_ProjectionThreadMessagesTurnBoundary.test.ts` | `apps/server/src/persistence/Migrations/054_ProjectionThreadsAutoSettleDisabledAt.test.ts` |
| 26% | 262 | 1700 | `apps/server/src/provider/Layers/OmpAdapter.ts` | `apps/server/src/provider/Layers/CursorAdapter.ts` |
| 26% | 4 | 17 | `packages/shared/src/Struct.ts` | `packages/shared/src/Struct.ts` |
| 26% | 18 | 77 | `apps/server/src/auth/Layers/AuthControlPlane.test.ts` | `apps/server/src/auth/EnvironmentAuthAdmin.test.ts` |
| 26% | 5 | 20 | `apps/desktop/scripts/start-electron.mjs` | `apps/desktop/scripts/start-electron.mjs` |
| 26% | 51 | 75 | `apps/server/src/persistence/Services/AuthSessions.ts` | `apps/server/src/persistence/AuthSessions.ts` |
| 26% | 263 | 1672 | `apps/server/src/provider/Layers/DroidAdapter.ts` | `apps/server/src/provider/Layers/CursorAdapter.ts` |
| 26% | 74 | 185 | `apps/web/src/components/Icons.tsx` | `apps/web/src/components/Icons.tsx` |
| 25% | 5 | 32 | `apps/server/src/persistence/Migrations/091_AutomationFailureTolerance.ts` | `apps/server/src/persistence/Migrations/001_OrchestrationEvents.ts` |
| 25% | 8 | 34 | `apps/server/src/persistence/Migrations/104_ProjectionThreadsClaudeCacheReview.test.ts` | `apps/server/src/persistence/Migrations/054_ProjectionThreadsAutoSettleDisabledAt.test.ts` |
| 25% | 218 | 1161 | `apps/server/src/provider/Layers/AntigravityAdapter.ts` | `apps/server/src/provider/Layers/CursorAdapter.ts` |
| 25% | 31 | 125 | `apps/server/src/orchestration/decider.settle.test.ts` | `apps/server/src/orchestration/decider.delete.test.ts` |
| 25% | 2 | 8 | `apps/server/src/automation/Services/AutomationRunReactor.ts` | `apps/server/src/orchestration/Services/CheckpointReactor.ts` |
| 25% | 5 | 33 | `apps/server/src/persistence/Migrations/116_ProjectAgentWorkerRecovery.ts` | `apps/server/src/persistence/Migrations/001_OrchestrationEvents.ts` |
| 25% | 4 | 9 | `apps/server/src/persistence/Migrations/129_ProjectionThreadsSnooze.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 25% | 6 | 27 | `apps/web/src/components/chat/group/CharacterCountTextarea.tsx` | `apps/web/src/components/settings/SettingsGroup.tsx` |
| 25% | 1 | 4 | `packages/profile-ui/src/cn.ts` | `apps/mobile/src/lib/cn.ts` |
| 25% | 55 | 374 | `apps/server/src/checkpointing/Layers/CheckpointDiffQuery.test.ts` | `apps/server/src/checkpointing/CheckpointDiffQuery.test.ts` |
| 25% | 16 | 70 | `apps/server/src/serverLifecycleEvents.ts` | `apps/server/src/serverLifecycleEvents.ts` |
| 25% | 8 | 50 | `apps/server/src/persistence/Migrations/064_DurableProviderCommandDelivery.ts` | `apps/server/src/persistence/Migrations/013_ProjectionThreadProposedPlans.ts` |
| 24% | 54 | 145 | `apps/web/src/components/ProjectScriptsControl.tsx` | `apps/web/src/components/ProjectScriptsControl.tsx` |
| 24% | 5 | 26 | `apps/server/src/persistence/Migrations/115_ProjectAgentManagedWorkers.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 24% | 4 | 11 | `apps/web/src/components/SidebarRowHoverActions.tsx` | `apps/mobile/src/components/MaterialScreenContent.tsx` |
| 24% | 7 | 42 | `apps/web/src/lib/utils.test.ts` | `apps/web/src/lib/utils.test.ts` |
| 24% | 37 | 203 | `apps/server/src/git/PrTemplateDetection.ts` | `apps/server/src/sourceControl/PrTemplateDetection.ts` |
| 24% | 5 | 31 | `apps/server/src/persistence/Migrations/056_CommandReceiptFingerprints.ts` | `apps/server/src/persistence/Migrations/002_OrchestrationCommandReceipts.ts` |
| 24% | 8 | 38 | `apps/server/src/persistence/Migrations/105_AsyncUserInput.test.ts` | `apps/server/src/persistence/Migrations/054_ProjectionThreadsAutoSettleDisabledAt.test.ts` |
| 24% | 503 | 3374 | `apps/server/src/orchestration/Layers/ProjectionSnapshotQuery.ts` | `apps/server/src/orchestration/Layers/ProjectionSnapshotQuery.ts` |
| 24% | 11 | 43 | `apps/server/src/persistence/Services/ProviderSessionRuntime.ts` | `apps/server/src/persistence/Services/ProjectionThreadSessions.ts` |
| 24% | 289 | 1856 | `apps/server/src/provider/acp/AcpSessionRuntime.ts` | `apps/server/src/provider/acp/AcpSessionRuntime.ts` |
| 24% | 55 | 339 | `apps/server/src/persistence/Migrations/035_NormalizeLegacyModelSelectionOptions.test.ts` | `apps/server/src/persistence/Migrations/016_CanonicalizeModelSelections.test.ts` |
| 24% | 2 | 9 | `apps/server/src/orchestration/Services/CheckpointReactor.ts` | `apps/server/src/orchestration/Services/CheckpointReactor.ts` |
| 24% | 6 | 39 | `apps/server/src/persistence/Migrations/117_WorkerMonitoringLiveness.ts` | `apps/server/src/persistence/Migrations/006_ProjectionThreadSessionRuntimeModeColumns.ts` |
| 24% | 6 | 24 | `apps/web/src/hooks/useThreadComputerStateSeed.ts` | `apps/mobile/src/features/cloud/connectOnboardingNavigation.ts` |
| 23% | 40 | 157 | `apps/web/src/components/BranchToolbar.logic.ts` | `apps/web/src/components/BranchToolbar.logic.ts` |
| 23% | 135 | 540 | `apps/server/src/auth/Layers/SessionCredentialService.ts` | `apps/server/src/auth/SessionStore.ts` |
| 23% | 43 | 327 | `apps/web/src/lib/terminalContext.ts` | `apps/web/src/lib/terminalContext.ts` |
| 23% | 47 | 194 | `apps/web/src/components/chat/UserInputQuestionForm.tsx` | `apps/web/src/components/chat/ComposerPendingUserInputPanel.tsx` |
| 23% | 18 | 22 | `apps/web/src/components/chat/ExpandedImagePreview.tsx` | `apps/web/src/components/chat/ExpandedImagePreview.tsx` |
| 23% | 27 | 142 | `apps/web/src/components/chat/ContextWindowMeter.tsx` | `apps/web/src/components/chat/ContextWindowMeter.tsx` |
| 23% | 32 | 180 | `packages/contracts/src/editor.ts` | `packages/contracts/src/editor.ts` |
| 23% | 7 | 33 | `apps/server/src/persistence/Migrations/063_ProjectionMessageCausalSequence.test.ts` | `apps/server/src/persistence/Migrations/054_ProjectionThreadsAutoSettleDisabledAt.test.ts` |
| 22% | 65 | 145 | `apps/server/src/provider/Layers/EventNdjsonLogger.test.ts` | `apps/server/src/provider/Layers/EventNdjsonLogger.test.ts` |
| 22% | 20 | 113 | `apps/server/src/persistence/Errors.ts` | `apps/server/src/persistence/Errors.ts` |
| 22% | 9 | 41 | `apps/server/src/persistence/Services/ProjectionCheckpoints.ts` | `apps/server/src/persistence/Services/ProjectionThreadActivities.ts` |
| 22% | 20 | 48 | `apps/web/src/components/ReleaseHistoryDialog.tsx` | `apps/web/src/components/settings/AddUsageLimitSourceDialog.tsx` |
| 22% | 8 | 40 | `apps/web/src/lib/utils.ts` | `apps/web/src/lib/utils.ts` |
| 22% | 1 | 6 | `apps/web/src/routes/_chat.kanban.index.tsx` | `apps/web/src/routes/_chat.$environmentId.$threadId.tsx` |
| 22% | 48 | 507 | `apps/server/src/workspace/Layers/WorkspaceFileSystem.test.ts` | `apps/server/src/workspace/WorkspaceFileSystem.test.ts` |
| 22% | 27 | 89 | `apps/server/src/orchestration/decider.providerHandoff.test.ts` | `apps/server/src/orchestration/decider.active-order.test.ts` |
| 22% | 115 | 534 | `apps/server/src/provider/providerMaintenance.ts` | `apps/server/src/provider/providerMaintenance.ts` |
| 22% | 150 | 886 | `apps/web/src/keybindings.test.ts` | `apps/web/src/keybindings.test.ts` |
| 22% | 57 | 345 | `apps/server/src/git/Layers/CursorTextGeneration.ts` | `apps/server/src/textGeneration/CursorTextGeneration.ts` |
| 22% | 853 | 12474 | `apps/server/src/provider/Layers/ClaudeAdapter.test.ts` | `apps/server/src/provider/Layers/ClaudeAdapter.test.ts` |
| 22% | 6 | 45 | `apps/server/src/persistence/Migrations/050_ProfileStatsArchive.ts` | `apps/server/src/persistence/Migrations/004_ProviderSessionRuntime.ts` |
| 21% | 189 | 2873 | `apps/server/src/orchestration/decider.ts` | `apps/server/src/orchestration/decider.ts` |
| 21% | 9 | 31 | `apps/server/src/persistence/Migrations/053_BackfillThreadActivitySequence.ts` | `apps/server/src/persistence/Migrations/046_RepairAutomaticSettlementTimestamps.ts` |
| 21% | 8 | 89 | `apps/server/src/project/Layers/ProjectFaviconResolver.test.ts` | `apps/server/src/project/T3ProjectFileLoader.test.ts` |
| 21% | 2 | 4 | `apps/server/src/persistence/Migrations/038_ReconcileLegacySidechatSource.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 21% | 2 | 6 | `apps/web/src/components/InlineChip.tsx` | `apps/web/src/components/clerk/BrowserManagedAuthShell.tsx` |
| 21% | 7 | 28 | `apps/web/src/components/ui/button-group.tsx` | `apps/web/src/components/chat/ComposerSurface.tsx` |
| 21% | 8 | 30 | `apps/web/src/components/ui/copyTextButton.tsx` | `apps/web/src/components/chat/MessageCopyButton.tsx` |
| 21% | 131 | 616 | `packages/contracts/src/orchestration.test.ts` | `packages/contracts/src/orchestration.test.ts` |
| 21% | 7 | 33 | `apps/web/src/components/terminal/TerminalActivityIndicator.tsx` | `apps/web/src/components/ProjectMonogram.tsx` |
| 21% | 8 | 39 | `apps/web/src/components/ProviderAccountMark.tsx` | `apps/web/src/components/chat/ProviderInstanceIcon.tsx` |
| 21% | 17 | 84 | `scripts/lib/brand-assets.ts` | `scripts/lib/brand-assets.ts` |
| 21% | 71 | 471 | `apps/server/src/persistence/Layers/ProjectionRepositories.test.ts` | `apps/server/src/persistence/Layers/ProjectionRepositories.test.ts` |
| 20% | 4 | 30 | `apps/web/src/threadDetailCatchupPolicy.test.ts` | `apps/mobile/src/features/showcase/showcaseRenderSignal.test.ts` |
| 20% | 9 | 60 | `apps/server/src/persistence/Migrations/078_ExternalMcpLiveTurnCapacity.test.ts` | `apps/server/src/persistence/Migrations/054_ProjectionThreadsAutoSettleDisabledAt.test.ts` |
| 20% | 14 | 77 | `apps/server/src/persistence/Migrations/057_ThreadScopedProjectionMessageIdentity.ts` | `apps/server/src/persistence/Migrations/005_Projections.ts` |
| 20% | 93 | 277 | `apps/server/src/git/Services/GitCore.ts` | `apps/server/src/vcs/GitVcsDriver.ts` |
| 20% | 4 | 20 | `apps/server/src/checkpointing/Services/CheckpointDiffQuery.ts` | `packages/client-runtime/src/state/checkpointDiff.ts` |
| 20% | 5 | 40 | `apps/server/src/orchestration/Services/ProviderCommandReactor.ts` | `apps/server/src/orchestration/Services/ProviderCommandReactor.ts` |
| 20% | 6 | 34 | `apps/server/src/persistence/Migrations/038_ReconcileLegacySidechatSource.test.ts` | `apps/server/src/persistence/Migrations/051_ProjectionThreadMessageContext.test.ts` |
| 20% | 6 | 34 | `apps/server/src/persistence/Migrations/039_ReconcileLegacyPinnedThreads.test.ts` | `apps/server/src/persistence/Migrations/051_ProjectionThreadMessageContext.test.ts` |
| 20% | 3 | 25 | `apps/server/src/persistence/Migrations/113_ProjectAgentWakeCursor.ts` | `apps/server/src/persistence/Migrations/052_ProjectionThreadTitleState.ts` |
| 20% | 2 | 7 | `apps/web/src/components/chat/ChatSurfaceHeader.tsx` | `apps/web/src/components/clerk/BrowserManagedAuthShell.tsx` |
| 20% | 1 | 7 | `apps/web/src/routes/_chat.kanban.$projectId.tsx` | `apps/web/src/routes/_chat.$environmentId.$threadId.tsx` |
| 20% | 15 | 37 | `apps/web/src/projectImport/ProjectImportDialog.tsx` | `apps/web/src/components/settings/AddUsageLimitSourceDialog.tsx` |
| 20% | 34 | 248 | `apps/server/src/orchestration/decider.approvalIdempotency.test.ts` | `apps/server/src/orchestration/decider.delete.test.ts` |
| 20% | 7 | 42 | `apps/server/src/persistence/Migrations/077_ExternalMcpCompensatingCapacity.test.ts` | `apps/server/src/persistence/Migrations/054_ProjectionThreadsAutoSettleDisabledAt.test.ts` |
| 20% | 7 | 21 | `apps/web/src/components/SidebarSectionToolbar.tsx` | `apps/web/src/components/ui/group.tsx` |
| 20% | 12 | 15 | `apps/server/src/startupAccess.ts` | `apps/server/src/startupAccess.ts` |
| 20% | 26 | 96 | `apps/web/src/timestampFormat.ts` | `apps/web/src/timestampFormat.ts` |
| 19% | 45 | 194 | `apps/server/src/transcriptUsageParsers.ts` | `apps/server/src/usage/usageTranscripts.ts` |
| 19% | 74 | 222 | `scripts/dev-runner.test.ts` | `scripts/dev-runner.test.ts` |
| 19% | 6 | 40 | `apps/web/src/components/chat/MessageCopyButton.tsx` | `apps/web/src/components/ui/anchoredCopyToast.ts` |
| 19% | 5 | 26 | `apps/server/src/persistence/Migrations/129_ProjectionThreadsSnooze.test.ts` | `apps/server/src/persistence/Migrations/051_ProjectionThreadMessageContext.test.ts` |
| 19% | 56 | 180 | `apps/server/src/auth/Layers/BootstrapCredentialService.ts` | `apps/server/src/auth/PairingGrantStore.ts` |
| 19% | 333 | 1118 | `scripts/build-desktop-artifact.ts` | `scripts/build-desktop-artifact.ts` |
| 19% | 7 | 13 | `apps/marketing/src/components/SectionEyebrow.tsx` | `apps/web/src/components/ui/group.tsx` |
| 19% | 13 | 44 | `apps/marketing/src/components/ui/button.tsx` | `apps/web/src/components/ui/toggle.tsx` |
| 19% | 8 | 51 | `apps/server/src/persistence/Migrations/087_DropUnusedOrchestrationEventIndexes.test.ts` | `apps/server/src/persistence/Migrations/049_ProjectionThreadsActiveOrderKey.test.ts` |
| 19% | 2 | 9 | `apps/web/src/hooks/useDefaultDesktopAppIcon.ts` | `apps/mobile/src/features/settings/appearance/useScaledTextRole.ts` |
| 19% | 27 | 87 | `apps/server/src/environment/Layers/ServerEnvironment.ts` | `apps/server/src/environment/ServerEnvironment.ts` |
| 19% | 31 | 256 | `apps/web/src/lib/terminalContext.test.ts` | `apps/web/src/lib/terminalContext.test.ts` |
| 19% | 17 | 56 | `apps/server/src/provider/codexCliVersion.ts` | `packages/shared/src/semver.ts` |
| 19% | 24 | 93 | `apps/web/src/components/kanban/KanbanTaskExtrasMenu.tsx` | `apps/web/src/components/preview/PreviewMoreMenu.tsx` |
| 19% | 29 | 128 | `apps/web/src/components/RenameDialog.tsx` | `apps/web/src/components/desktop/SshPasswordPromptDialog.tsx` |
| 19% | 5 | 30 | `apps/web/src/lib/composerSendOwnership.ts` | `apps/web/src/hooks/threadUndo.ts` |
| 19% | 23 | 217 | `apps/server/src/persistence/Layers/ProjectionPendingInteractions.ts` | `apps/server/src/persistence/Layers/ProjectionPendingApprovals.ts` |
| 19% | 469 | 3261 | `apps/server/src/git/Layers/GitCore.ts` | `apps/server/src/vcs/GitVcsDriverCore.ts` |
| 19% | 4 | 25 | `apps/marketing/src/lib/useActiveAnchor.ts` | `apps/web/src/components/diffs/DiffFileLoadingBoundary.tsx` |
| 19% | 8 | 29 | `apps/web/src/components/settings/McpConnectorGlyph.tsx` | `apps/web/src/components/chat/ProviderInstanceIcon.tsx` |
| 19% | 9 | 36 | `apps/web/src/components/ui/status-chip.tsx` | `apps/web/src/components/ui/group.tsx` |
| 19% | 75 | 655 | `apps/server/src/git/Layers/OpenCodeTextGeneration.ts` | `apps/server/src/textGeneration/OpenCodeTextGeneration.ts` |
| 18% | 39 | 119 | `packages/shared/src/git.ts` | `packages/shared/src/git.ts` |
| 18% | 6 | 27 | `apps/server/src/persistence/Migrations/100_MessageTextChunks.test.ts` | `apps/server/src/persistence/Migrations/019_ProjectionSnapshotLookupIndexes.test.ts` |
| 18% | 15 | 96 | `apps/web/src/components/pullRequest/PullRequestStateGlyph.tsx` | `apps/web/src/components/pullRequest/PullRequestListRow.tsx` |
| 18% | 65 | 312 | `apps/web/src/lib/diffRendering.ts` | `apps/web/src/lib/diffRendering.ts` |
| 18% | 1 | 5 | `apps/server/src/pullRequests/KeyedSingleFlightCache.ts` | `packages/client-runtime/src/authorization/index.ts` |
| 18% | 4 | 16 | `apps/web/src/components/chat/DiffStatLabel.tsx` | `apps/web/src/components/chat/DiffStatLabel.tsx` |
| 18% | 6 | 49 | `apps/web/src/onboarding/steps/AccountStep.tsx` | `apps/web/src/components/settings/PanelAnimationsPreview.tsx` |
| 18% | 13 | 44 | `apps/server/src/provider/providerModelCatalogCache.ts` | `apps/server/src/provider/providerStatusCache.ts` |
| 18% | 17 | 44 | `apps/server/src/persistence/Migrations/060_PendingApprovalLifecycleGeneration.test.ts` | `apps/server/src/persistence/Migrations/025_CleanupInvalidProjectionPendingApprovals.test.ts` |
| 18% | 32 | 157 | `apps/web/src/lib/hosts/ExecutionFolderPicker.tsx` | `apps/web/src/components/settings/DeviceHostEditor.tsx` |
| 18% | 21 | 67 | `apps/server/src/persistence/Migrations/058_ThreadScopedPendingApprovalIdentity.test.ts` | `apps/server/src/persistence/Migrations/025_CleanupInvalidProjectionPendingApprovals.test.ts` |
| 18% | 43 | 278 | `apps/server/src/git/Services/GitHubCli.ts` | `apps/server/src/pullRequest/PullRequestProvider.ts` |
| 18% | 4 | 35 | `apps/server/src/persistence/Migrations/048_AutomationCompletionEvaluationBacklog.ts` | `apps/server/src/persistence/Migrations/051_ProjectionThreadMessageContext.ts` |
| 18% | 4 | 15 | `apps/web/src/components/QueuedComposerDrainCoordinator.tsx` | `apps/web/src/planAgentSelectionHeal.tsx` |
| 18% | 3 | 45 | `packages/shared/src/appSnapShortcut.test.ts` | `apps/mobile/src/features/threads/markdownImageSize.test.ts` |
| 18% | 188 | 2954 | `apps/server/src/orchestration/Layers/OrchestrationEngine.test.ts` | `apps/server/src/orchestration/Layers/OrchestrationEngine.test.ts` |
| 18% | 76 | 667 | `packages/shared/src/model.ts` | `packages/shared/src/model.ts` |
| 18% | 4 | 19 | `apps/server/src/persistence/Migrations/141_ProjectionThreadsUsageLimitRecovery.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 18% | 29 | 128 | `apps/server/src/orchestration/decider.snooze.test.ts` | `apps/server/src/orchestration/decider.active-order.test.ts` |
| 18% | 12 | 49 | `apps/server/src/persistence/Migrations/058_ThreadScopedPendingApprovalIdentity.ts` | `apps/server/src/persistence/Migrations/005_Projections.ts` |
| 17% | 129 | 853 | `apps/web/src/components/BranchToolbarBranchSelector.tsx` | `apps/web/src/components/BranchToolbarBranchSelector.tsx` |
| 17% | 4 | 64 | `apps/web/src/routes/-threadDetailSnapshotOrdering.test.ts` | `apps/mobile/src/features/showcase/showcaseRenderSignal.test.ts` |
| 17% | 429 | 4213 | `apps/server/src/orchestration/Layers/ProjectionPipeline.test.ts` | `apps/server/src/orchestration/Layers/ProjectionPipeline.test.ts` |
| 17% | 34 | 178 | `apps/web/src/components/SpaceEditorDialog.tsx` | `apps/web/src/components/pullRequest/LinkPullRequestDialog.tsx` |
| 17% | 10 | 85 | `apps/server/src/persistence/Migrations/044_Automations.ts` | `apps/server/src/persistence/Migrations/005_Projections.ts` |
| 17% | 86 | 742 | `apps/server/src/persistence/Layers/OrchestrationEventStore.test.ts` | `apps/server/src/persistence/Layers/OrchestrationEventStore.test.ts` |
| 17% | 7 | 35 | `apps/server/src/persistence/Migrations/120_ProfileStatsDeletedProviderInstances.test.ts` | `apps/server/src/persistence/Migrations/031_AuthAuthorizationScopes.test.ts` |
| 17% | 9 | 57 | `apps/server/src/stream/collectUint8StreamText.test.ts` | `apps/server/src/stream/collectUint8StreamText.test.ts` |
| 17% | 3 | 6 | `apps/web/src/components/chat/DelayedLoaderFade.tsx` | `apps/mobile/src/features/settings/components/SettingsSection.tsx` |
| 17% | 3 | 12 | `apps/web/src/composerModelSelectionCommits.ts` | `apps/web/src/hooks/threadUndo.ts` |
| 17% | 22 | 81 | `apps/server/src/persistence/Layers/ProjectionCheckpoints.ts` | `apps/server/src/persistence/Layers/ProjectionTurns.ts` |
| 17% | 34 | 240 | `packages/contracts/src/project.ts` | `packages/contracts/src/project.ts` |
| 17% | 510 | 3776 | `apps/server/src/provider/Layers/OpenCodeAdapter.ts` | `apps/server/src/provider/Layers/OpenCodeAdapter.ts` |
| 17% | 66 | 396 | `apps/web/src/routeTree.gen.ts` | `apps/web/src/routeTree.gen.ts` |
| 17% | 23 | 157 | `apps/web/src/components/chat/ComposerSlashStatusDialog.tsx` | `apps/web/src/components/settings/AddUsageLimitSourceDialog.tsx` |
| 17% | 6 | 45 | `apps/server/src/persistence/Migrations/103_ClaudeTokenAccounting.test.ts` | `apps/server/src/persistence/Migrations/051_ProjectionThreadMessageContext.test.ts` |
| 17% | 26 | 183 | `apps/web/src/components/settings/accounts/AddProviderAccountDialog.tsx` | `apps/web/src/components/settings/AddUsageLimitSourceDialog.tsx` |
| 17% | 23 | 163 | `apps/web/src/lib/providerDiscovery.ts` | `packages/shared/src/searchRanking.ts` |
| 17% | 177 | 2554 | `apps/server/src/orchestration/Layers/CheckpointReactor.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.ts` |
| 17% | 1 | 7 | `apps/analytics/vite.config.ts` | `apps/marketing/astro.config.mjs` |
| 17% | 1 | 7 | `apps/api/drizzle.config.ts` | `apps/marketing/astro.config.mjs` |
| 17% | 6 | 15 | `apps/server/src/persistence/Migrations/037_ProjectionSnapshotCapIndexes.ts` | `apps/server/src/persistence/Migrations/050_ProjectionThreadPullRequests.ts` |
| 17% | 4 | 38 | `apps/server/src/persistence/Migrations/075_ExternalMcpActiveCapacity.ts` | `apps/server/src/persistence/Migrations/051_ProjectionThreadMessageContext.ts` |
| 17% | 5 | 57 | `apps/server/src/persistence/Migrations/076_ExternalMcpHardening.ts` | `apps/server/src/persistence/Migrations/013_ProjectionThreadProposedPlans.ts` |
| 17% | 4 | 38 | `apps/server/src/persistence/Migrations/077_ExternalMcpCompensatingCapacity.ts` | `apps/server/src/persistence/Migrations/051_ProjectionThreadMessageContext.ts` |
| 17% | 10 | 63 | `apps/web/src/components/chat/InlineVisualSurface.tsx` | `apps/web/src/components/settings/FoldedSettingsSection.tsx` |
| 17% | 7 | 54 | `apps/web/src/components/kanban/KanbanStatusIcon.tsx` | `apps/web/src/components/ui/checkbox.tsx` |
| 17% | 1 | 5 | `apps/web/src/diffRenderMode.ts` | `apps/web/src/cloud/connectOnboarding.ts` |
| 17% | 268 | 1442 | `apps/web/src/components/GitActionsControl.tsx` | `apps/web/src/components/GitActionsControl.tsx` |
| 17% | 6 | 91 | `apps/server/src/persistence/Migrations/074_ExternalMcpIntegrations.ts` | `apps/server/src/persistence/Migrations/020_AuthAccessManagement.ts` |
| 16% | 7 | 25 | `apps/profiles/src/components/ProfileCard.tsx` | `apps/web/src/components/clerk/ClerkUserProfilePage.tsx` |
| 16% | 11 | 53 | `apps/server/src/auth/Layers/ServerAuthPolicy.test.ts` | `apps/server/src/auth/EnvironmentAuthPolicy.test.ts` |
| 16% | 5 | 42 | `apps/desktop/scripts/dev.mjs` | `apps/desktop/scripts/start-electron.mjs` |
| 16% | 13 | 51 | `apps/server/src/os-jank.ts` | `apps/server/src/os-jank.ts` |
| 16% | 19 | 87 | `apps/server/src/persistence/Migrations/053_BackfillThreadActivitySequence.test.ts` | `apps/server/src/persistence/Migrations/024_BackfillProjectionThreadShellSummary.test.ts` |
| 16% | 4 | 39 | `apps/server/src/persistence/Migrations/078_ExternalMcpLiveTurnCapacity.ts` | `apps/server/src/persistence/Migrations/051_ProjectionThreadMessageContext.ts` |
| 16% | 10 | 61 | `apps/server/src/persistence/Migrations/062_PendingInteractionSettlementParity.ts` | `apps/server/src/persistence/Migrations/005_Projections.ts` |
| 16% | 28 | 236 | `apps/server/src/persistence/Migrations/099_InvalidateProjectionThreadsCursor.test.ts` | `apps/server/src/orchestration/decider.import.test.ts` |
| 16% | 30 | 172 | `apps/server/src/git/Layers/AcpTextGeneration.ts` | `apps/server/src/textGeneration/CursorTextGeneration.ts` |
| 16% | 30 | 352 | `apps/web/src/lib/contextWindow.test.ts` | `apps/web/src/lib/contextWindow.test.ts` |
| 16% | 41 | 250 | `apps/server/src/git/Layers/DroidTextGeneration.ts` | `apps/server/src/textGeneration/CursorTextGeneration.ts` |
| 16% | 14 | 74 | `apps/web/src/components/chat/FileLineCommentBox.tsx` | `apps/web/src/components/chat/AssistantCitationCommentEditor.tsx` |
| 16% | 21 | 115 | `apps/server/src/orchestration/decider.archive.test.ts` | `apps/server/src/orchestration/decider.autoSettleSet.test.ts` |
| 16% | 6 | 26 | `apps/server/src/startupAccess.test.ts` | `apps/server/src/startupAccess.test.ts` |
| 16% | 3 | 18 | `apps/web/src/components/AppShellTopStrip.tsx` | `apps/web/src/components/WorkspacePageHeader.tsx` |
| 16% | 3 | 16 | `apps/web/src/components/SidebarPanelTitle.tsx` | `apps/mobile/src/components/MaterialScreenContent.tsx` |
| 16% | 7 | 28 | `apps/web/src/components/chat/UserInputQuestionForm.test.tsx` | `apps/web/src/components/chat/ComposerPendingUserInputPanel.test.tsx` |
| 16% | 24 | 102 | `apps/web/src/components/chat/group/GroupLinkProjectDialog.tsx` | `apps/web/src/components/settings/ProjectIconPickerDialog.tsx` |
| 16% | 20 | 62 | `apps/web/src/components/hosts/HostDiscoverabilityPrompt.tsx` | `apps/web/src/components/chat/ProposedPlanCard.tsx` |
| 16% | 36 | 218 | `apps/web/src/markdown-links.ts` | `packages/client-runtime/src/markdownLinks.ts` |
| 16% | 13 | 109 | `apps/web/src/components/chat/ThreadErrorBanner.tsx` | `apps/web/src/components/pullRequest/PullRequestCommentBody.tsx` |
| 16% | 4 | 22 | `apps/web/src/components/ui/spinner.tsx` | `apps/web/src/components/ui/spinner.tsx` |
| 16% | 22 | 250 | `apps/server/src/orchestration/Layers/ThreadDeletionReactor.ts` | `apps/server/src/orchestration/Layers/ThreadDeletionReactor.ts` |
| 16% | 5 | 25 | `apps/server/src/persistence/Services/ProjectionSpaces.ts` | `apps/server/src/persistence/Services/ProjectionProjects.ts` |
| 16% | 15 | 72 | `scripts/update-release-package-versions.ts` | `scripts/update-release-package-versions.ts` |
| 16% | 5 | 28 | `apps/web/src/components/SplashScreen.tsx` | `apps/web/src/components/pullRequest/PullRequestsUnavailableState.tsx` |
| 16% | 13 | 69 | `apps/server/src/persistence/Layers/ProjectionSpaces.ts` | `apps/server/src/persistence/Layers/ProjectionProjects.ts` |
| 16% | 22 | 111 | `apps/server/src/persistence/Migrations/086_NormalizeStudioThreadWorkspaces.test.ts` | `apps/server/src/persistence/Migrations/025_CleanupInvalidProjectionPendingApprovals.test.ts` |
| 15% | 98 | 1539 | `apps/server/src/git/Layers/CodexTextGeneration.test.ts` | `apps/server/src/textGeneration/CodexTextGeneration.test.ts` |
| 15% | 3 | 17 | `apps/marketing/src/app/docs/layout.tsx` | `apps/mobile/src/components/MaterialScreenContent.tsx` |
| 15% | 1 | 7 | `apps/server/src/device/Services/DeviceService.ts` | `apps/mobile/src/features/settings/SettingsRouteScreen.logic.ts` |
| 15% | 28 | 171 | `apps/server/src/orchestration/decider.runtimeMode.test.ts` | `apps/server/src/orchestration/decider.delete.test.ts` |
| 15% | 2 | 9 | `apps/web/src/components/codeEditor/editorRecovery.ts` | `apps/web/src/lib/chunkReloadGuard.ts` |
| 15% | 4 | 52 | `apps/web/src/hooks/useDesktopTopBarGutter.test.ts` | `apps/mobile/src/features/showcase/showcaseRenderSignal.test.ts` |
| 15% | 2 | 15 | `apps/web/src/lib/temporaryThread.ts` | `apps/server/src/orchestration/threadDetailCursor.ts` |
| 15% | 8 | 124 | `apps/server/src/persistence/Migrations/055_ManagedAttachments.ts` | `apps/server/src/persistence/Migrations/005_Projections.ts` |
| 15% | 30 | 286 | `apps/web/src/composer-editor-mentions.ts` | `apps/web/src/composer-editor-mentions.ts` |
| 15% | 31 | 249 | `apps/server/src/orchestration/decider.claudeCacheReview.test.ts` | `apps/server/src/orchestration/decider.active-order.test.ts` |
| 15% | 47 | 485 | `apps/server/src/persistence/Migrations.ts` | `apps/server/src/persistence/Migrations.ts` |
| 15% | 14 | 66 | `apps/server/src/persistence/Migrations/057_ThreadScopedProjectionMessageIdentity.test.ts` | `apps/server/src/persistence/Migrations/024_BackfillProjectionThreadShellSummary.test.ts` |
| 15% | 8 | 72 | `apps/web/src/components/chat/ComposerPickerMenuPopup.tsx` | `apps/web/src/components/ui/select.tsx` |
| 15% | 97 | 881 | `apps/web/src/session-logic.test.ts` | `apps/web/src/session-logic.test.ts` |
| 15% | 5 | 50 | `apps/server/src/provider/keyedLock.ts` | `apps/server/src/workspace/workspaceLease.ts` |
| 15% | 12 | 110 | `apps/web/src/lib/keybindingCapture.test.ts` | `apps/web/src/lib/snapShotShortcut.test.ts` |
| 15% | 13 | 134 | `apps/server/src/persistence/Migrations/098_MigrateKiloToOpenCode.ts` | `apps/server/src/persistence/Migrations/026_CanonicalizeModelSelectionOptions.ts` |
| 15% | 5 | 39 | `apps/server/src/serverLifecycleEvents.test.ts` | `apps/server/src/serverLifecycleEvents.test.ts` |
| 15% | 6 | 52 | `apps/server/src/persistence/Layers/OrchestrationCommandReceipts.test.ts` | `apps/server/src/persistence/Migrations/054_ProjectionThreadsAutoSettleDisabledAt.test.ts` |
| 15% | 4 | 23 | `apps/web/src/components/SidebarMetaChip.tsx` | `apps/web/src/components/ui/panel-tab-close-button.tsx` |
| 15% | 4 | 69 | `packages/shared/src/projectContainers.test.ts` | `apps/mobile/src/features/showcase/showcaseRenderSignal.test.ts` |
| 15% | 83 | 825 | `apps/server/src/git/Layers/CodexTextGeneration.ts` | `apps/server/src/textGeneration/CodexTextGeneration.ts` |
| 15% | 21 | 123 | `apps/web/src/components/chat/ComposerMenuPanel.tsx` | `apps/web/src/components/chat/ComposerCommandMenu.tsx` |
| 15% | 265 | 3504 | `apps/server/src/provider/Layers/DevinAdapter.ts` | `apps/server/src/provider/Layers/CursorAdapter.ts` |
| 15% | 27 | 189 | `apps/server/src/orchestration/decider.sidechatExpiry.test.ts` | `apps/server/src/orchestration/commandInvariants.test.ts` |
| 15% | 7 | 75 | `packages/shared/src/githubRepository.test.ts` | `packages/shared/src/changeRequestUrl.test.ts` |
| 15% | 520 | 8009 | `apps/server/src/orchestration/Layers/ProviderRuntimeIngestion.test.ts` | `apps/server/src/orchestration/Layers/ProviderRuntimeIngestion.test.ts` |
| 15% | 7 | 17 | `apps/web/src/fixedShortcuts.test.ts` | `apps/web/src/lib/terminalCloseShortcut.test.ts` |
| 15% | 6 | 46 | `apps/web/src/lib/remarkGithubAlerts.ts` | `apps/web/src/markdown-github-alerts.ts` |
| 15% | 6 | 35 | `apps/web/src/main.tsx` | `apps/web/src/main.tsx` |
| 15% | 312 | 3187 | `apps/server/src/orchestration/Layers/ProviderRuntimeIngestion.ts` | `apps/server/src/orchestration/Layers/ProviderRuntimeIngestion.ts` |
| 15% | 23 | 174 | `apps/web/vite.config.ts` | `apps/web/vite.config.ts` |
| 15% | 4 | 20 | `apps/analytics/src/theme.ts` | `apps/web/src/hooks/useNowMinute.ts` |
| 15% | 34 | 171 | `apps/web/src/components/settings/McpLocalConnectorDialog.tsx` | `apps/web/src/components/CustomSnoozeDialog.tsx` |
| 15% | 5 | 89 | `packages/shared/src/threadEnvironment.test.ts` | `apps/mobile/src/features/threads/new-task-context-presentation.test.ts` |
| 14% | 10 | 120 | `apps/server/src/checkpointing/Utils.ts` | `apps/server/src/checkpointing/Utils.ts` |
| 14% | 18 | 221 | `apps/server/src/persistence/Migrations/109_ProjectAgent.ts` | `apps/server/src/persistence/Migrations/005_Projections.ts` |
| 14% | 7 | 52 | `apps/web/src/hooks/useThrottledStreamingValue.ts` | `apps/web/src/components/preview/ZoomIndicator.tsx` |
| 14% | 34 | 373 | `apps/server/src/orchestration/decider.worktreeMetadata.test.ts` | `apps/server/src/orchestration/decider.delete.test.ts` |
| 14% | 14 | 95 | `apps/web/src/components/chat/ComposerUltrafastMenu.tsx` | `apps/web/src/components/settings/EnvironmentIconPicker.tsx` |
| 14% | 205 | 2431 | `apps/server/src/provider/Layers/CodexAdapter.test.ts` | `apps/server/src/provider/Layers/CodexAdapter.test.ts` |
| 14% | 2 | 10 | `apps/web/src/hooks/useInPageGlassOverlay.ts` | `apps/web/src/components/diffs/DiffFileLoadingBoundary.tsx` |
| 14% | 5 | 34 | `apps/web/src/lib/composerTransportStatus.ts` | `packages/client-runtime/src/delayedStatus.ts` |
| 14% | 4 | 27 | `apps/web/src/lib/projectSnapshotHydration.ts` | `apps/desktop/src/snapShot/GlobalShiftShortcutProcess.ts` |
| 14% | 4 | 68 | `packages/shared/src/browserShortcuts.test.ts` | `apps/web/src/components/chat/ContextWindowMeter.logic.test.ts` |
| 14% | 159 | 1261 | `apps/server/src/terminal/Layers/Manager.test.ts` | `apps/server/src/terminal/Manager.test.ts` |
| 14% | 31 | 194 | `apps/web/src/components/SpaceProjectPickerDialog.tsx` | `apps/web/src/components/PullRequestThreadDialog.tsx` |
| 14% | 8 | 42 | `apps/web/src/components/DialogHeroBand.tsx` | `apps/web/src/components/chat/ProviderInstanceIcon.tsx` |
| 14% | 5 | 45 | `apps/server/src/persistence/Migrations/141_ProjectionThreadsUsageLimitRecovery.test.ts` | `apps/server/src/persistence/Migrations/051_ProjectionThreadMessageContext.test.ts` |
| 14% | 9 | 58 | `apps/server/src/diagnostics/Layers/ThreadDiagnosticsQuery.test.ts` | `apps/server/src/persistence/Layers/ProjectionThreadActivities.test.ts` |
| 14% | 4 | 12 | `apps/marketing/src/components/docs/steps.tsx` | `apps/web/src/components/WorkspaceBreadcrumb.tsx` |
| 14% | 15 | 70 | `apps/web/src/components/chat/BrowserAnnotationStrip.tsx` | `apps/web/src/components/chat/ContextWindowMeter.tsx` |
| 14% | 7 | 47 | `apps/web/src/lib/editableEventTarget.test.ts` | `apps/web/src/lib/terminalFocus.test.ts` |
| 14% | 11 | 65 | `apps/server/src/auth/Layers/ServerSecretStore.ts` | `apps/server/src/auth/ServerSecretStore.ts` |
| 14% | 7 | 75 | `apps/server/src/persistence/Migrations/023_ProjectionThreadsAssociatedWorktreeRef.ts` | `apps/server/src/persistence/Migrations/010_ProjectionThreadsRuntimeMode.ts` |
| 14% | 3 | 13 | `apps/server/src/serverLogger.ts` | `apps/server/src/serverLogger.ts` |
| 14% | 23 | 116 | `apps/web/src/components/ShortcutsDialog.tsx` | `apps/web/src/components/desktop/SshPasswordPromptDialog.tsx` |
| 14% | 14 | 47 | `apps/server/src/workspace/Services/WorkspaceFileSystem.ts` | `apps/server/src/workspace/WorkspaceFileSystem.ts` |
| 14% | 16 | 25 | `apps/desktop/src/runtimeArch.ts` | `apps/desktop/src/app/DesktopEnvironment.ts` |
| 14% | 16 | 121 | `apps/server/src/persistence/Layers/PullRequestAutoFixRepository.ts` | `apps/server/src/persistence/Layers/ProjectionThreadProposedPlans.ts` |
| 14% | 15 | 84 | `apps/server/src/persistence/Migrations/089_RecoverRetentionHiddenThreads.test.ts` | `apps/server/src/persistence/Migrations/046_RepairAutomaticSettlementTimestamps.test.ts` |
| 14% | 314 | 1830 | `apps/server/src/terminal/Layers/Manager.ts` | `apps/server/src/terminal/Manager.ts` |
| 14% | 2 | 12 | `apps/web/src/components/chat/ComposerTransportNotice.tsx` | `apps/web/src/hooks/useEnvironmentDisconnectDelay.ts` |
| 14% | 4 | 37 | `apps/web/src/components/settings/ProductAnalyticsSettingsPanel.browser.tsx` | `apps/web/src/environments/primary/desktopAuth.test.ts` |
| 14% | 31 | 370 | `apps/server/src/orchestration/decider.goalTiming.test.ts` | `apps/server/src/orchestration/decider.delete.test.ts` |
| 14% | 31 | 243 | `apps/web/src/components/settings/ConnectionsAddComputerDialog.tsx` | `apps/web/src/components/PullRequestThreadDialog.tsx` |
| 14% | 22 | 61 | `apps/web/src/components/tasks/TaskDelegateChatPicker.tsx` | `apps/web/src/components/preview/PreviewMoreMenu.tsx` |
| 13% | 7 | 41 | `apps/web/src/hooks/useCommittedPathname.browser.tsx` | `apps/web/src/lib/assistantCitationNavigation.test.ts` |
| 13% | 27 | 376 | `apps/web/src/components/chat/ComposerCommandMenu.tsx` | `apps/web/src/components/chat/ComposerCommandMenu.tsx` |
| 13% | 9 | 36 | `apps/server/src/jpegHeader.ts` | `packages/shared/src/imageDimensions.ts` |
| 13% | 2 | 22 | `apps/analytics/scripts/export.sh` | `.devcontainer/update-content.sh` |
| 13% | 1 | 9 | `apps/server/src/computer/Services/ComputerService.ts` | `apps/mobile/src/features/settings/SettingsRouteScreen.logic.ts` |
| 13% | 1 | 9 | `apps/server/src/terminal/terminalThreadTitle.ts` | `packages/client-runtime/src/authorization/index.ts` |
| 13% | 18 | 87 | `apps/web/src/components/DebugFeatureFlagsMenu.tsx` | `apps/web/src/components/settings/ProjectActionsSettings.tsx` |
| 13% | 13 | 94 | `packages/shared/src/pullRequestList.test.ts` | `apps/web/src/components/pullRequest/pullRequestChecks.test.tsx` |
| 13% | 20 | 123 | `apps/server/src/git/Layers/DroidTextGeneration.test.ts` | `apps/server/src/textGeneration/CursorTextGeneration.test.ts` |
| 13% | 5 | 9 | `apps/web/src/components/ui/skeleton.tsx` | `apps/web/src/components/chat/ComposerSurface.tsx` |
| 13% | 9 | 34 | `apps/web/src/components/chat/ComposerActionNotice.tsx` | `apps/web/src/components/files/fileSurfaceChrome.tsx` |
| 13% | 12 | 72 | `apps/web/src/components/pullRequest/PullRequestCommentComposer.tsx` | `apps/web/src/components/pullRequest/PullRequestCommentForm.tsx` |
| 13% | 8 | 94 | `apps/web/src/components/useGitProgressToastPreview.ts` | `apps/web/src/components/SlowRpcRequestToastCoordinator.tsx` |
| 13% | 3 | 12 | `apps/web/src/hooks/useNowMs.ts` | `apps/web/src/components/QuitHoldOverlay.tsx` |
| 13% | 6 | 48 | `apps/web/src/lib/queuedComposerPause.ts` | `apps/web/src/hooks/useWorkspaceMutationRefresh.ts` |
| 13% | 2 | 14 | `apps/server/src/pngHeader.ts` | `apps/mobile/src/components/MaterialMenuPopup.tsx` |
| 13% | 6 | 39 | `apps/server/src/provider/goalMode.ts` | `apps/mobile/plugins/withShareExtensionDisplayName.cjs` |
| 13% | 6 | 17 | `apps/web/src/components/chat/ComposerAutomationSetupBanner.tsx` | `apps/web/src/components/chat/AssistantCitationCommentEditor.tsx` |
| 13% | 2 | 23 | `apps/web/src/components/chat/useComposerTraitCommit.ts` | `apps/mobile/src/lib/useNativePaste.ts` |
| 13% | 19 | 137 | `apps/web/src/components/BrowserVaultMaster.tsx` | `apps/web/src/components/desktop/SshPasswordPromptDialog.tsx` |
| 13% | 15 | 197 | `apps/server/src/persistence/Layers/Sqlite.ts` | `apps/server/src/persistence/Layers/Sqlite.ts` |
| 13% | 5 | 95 | `apps/marketing/src/components/AntigravityIcon.tsx` | `apps/mobile/src/components/SourceControlIcon.tsx` |
| 13% | 21 | 538 | `apps/server/src/provider/Layers/ProviderAdapterRegistry.test.ts` | `apps/server/src/provider/Layers/ProviderAdapterRegistry.test.ts` |
| 13% | 19 | 200 | `apps/web/src/composer-editor-mentions.test.ts` | `apps/web/src/composer-editor-mentions.test.ts` |
| 13% | 3 | 23 | `apps/server/src/checkpointing/Errors.ts` | `apps/server/src/orchestration/Errors.ts` |
| 13% | 5 | 53 | `apps/server/src/persistence/Migrations/044_Automations.test.ts` | `apps/server/src/persistence/Migrations/051_ProjectionThreadMessageContext.test.ts` |
| 13% | 14 | 234 | `apps/desktop/src/syncShellEnvironment.test.ts` | `apps/desktop/src/shell/DesktopShellEnvironment.test.ts` |
| 13% | 28 | 257 | `apps/server/src/orchestration/decider.resumePrecondition.test.ts` | `apps/server/src/orchestration/decider.active-order.test.ts` |
| 12% | 1 | 9 | `apps/e2e/vitest.config.ts` | `packages/client-runtime/vite.config.ts` |
| 12% | 1 | 7 | `apps/server/src/automation/Services/AutomationScheduler.ts` | `apps/server/src/provider/Services/ProviderSessionReaper.ts` |
| 12% | 7 | 62 | `apps/web/src/components/AppIconTile.tsx` | `apps/web/src/components/ProjectMonogram.tsx` |
| 12% | 8 | 16 | `apps/web/src/components/pullRequest/PullRequestActorLabel.tsx` | `apps/web/src/components/pullRequest/PullRequestListRow.tsx` |
| 12% | 2 | 14 | `apps/web/src/composerFocusRequestStore.ts` | `apps/web/src/browser/browserPointerStore.ts` |
| 12% | 1 | 9 | `apps/web/vitest.browser.geometry.config.ts` | `packages/client-runtime/vite.config.ts` |
| 12% | 1 | 9 | `apps/web/vitest.browser.stable.config.ts` | `packages/client-runtime/vite.config.ts` |
| 12% | 260 | 1316 | `apps/web/src/composerDraftPersistence.ts` | `apps/web/src/composerDraftStore.ts` |
| 12% | 7 | 55 | `apps/web/src/components/ThreadRunningSpinner.tsx` | `apps/web/src/components/ui/checkbox.tsx` |
| 12% | 5 | 41 | `apps/web/src/components/chat/ContextWindowMeter.browser.tsx` | `apps/web/src/components/chat/ContextWindowMeter.test.tsx` |
| 12% | 16 | 31 | `apps/server/src/provider/acp/AcpErrors.ts` | `packages/effect-acp/src/errors.ts` |
| 12% | 13 | 112 | `apps/web/src/components/chat/ExpandedImageOverlay.tsx` | `apps/web/src/components/chat/ExpandedImageDialog.tsx` |
| 12% | 3 | 10 | `apps/web/src/wsTransportEvents.test.ts` | `apps/web/src/lib/desktopSnapShot.test.ts` |
| 12% | 44 | 599 | `apps/server/src/git/textGenerationShared.ts` | `apps/server/src/textGeneration/TextGenerationPrompts.ts` |
| 12% | 68 | 547 | `apps/web/src/components/chat/TraitsPicker.tsx` | `apps/web/src/components/chat/TraitsPicker.tsx` |
| 12% | 2 | 9 | `apps/web/src/authSignedOut.test.ts` | `packages/client-runtime/src/device/renderScheduler.test.ts` |
| 12% | 22 | 117 | `apps/web/src/components/WhatsNewDialog.tsx` | `apps/web/src/components/settings/AddUsageLimitSourceDialog.tsx` |
| 12% | 2 | 20 | `apps/web/src/components/pullRequest/usePullRequestPaneStateIcon.tsx` | `apps/web/src/components/clerk/BrowserManagedAuthShell.tsx` |
| 12% | 127 | 1314 | `apps/web/src/components/Sidebar.logic.ts` | `apps/web/src/components/Sidebar.logic.ts` |
| 12% | 9 | 65 | `apps/web/src/lib/fileDiffTree.ts` | `apps/web/src/lib/turnDiffTree.ts` |
| 12% | 7 | 22 | `apps/web/src/appSnapShortcut.test.ts` | `apps/web/src/lib/projectScriptKeybindings.test.ts` |
| 12% | 81 | 545 | `packages/contracts/src/server.ts` | `packages/contracts/src/server.ts` |
| 12% | 41 | 405 | `apps/web/src/components/chat/group/GroupSettingsDialog.tsx` | `apps/web/src/components/projectScriptEditor.tsx` |
| 12% | 104 | 1454 | `apps/server/src/orchestration/Layers/OrchestrationEngine.ts` | `apps/server/src/orchestration/Layers/OrchestrationEngine.ts` |
| 12% | 13 | 160 | `apps/server/src/persistence/Layers/TodoRepository.ts` | `apps/server/src/persistence/Layers/ProjectionThreadProposedPlans.ts` |
| 12% | 6 | 30 | `apps/server/src/voiceUploadAdmission.ts` | `apps/mobile/src/lib/composerAttachmentFiles.ts` |
| 12% | 5 | 23 | `apps/web/src/components/RouteInsetSurface.tsx` | `apps/web/src/components/ui/alert-dialog.tsx` |
| 12% | 4 | 41 | `apps/server/src/persistence/Migrations/106_ProjectImportOrigins.test.ts` | `apps/server/src/persistence/Migrations/051_ProjectionThreadMessageContext.test.ts` |
| 12% | 5 | 44 | `apps/web/src/components/chat/subagentRunContext.ts` | `apps/web/src/hooks/threadUndo.ts` |
| 12% | 9 | 122 | `apps/server/src/persistence/Migrations/132_ExternalMcpTurnCapacityRecovery.test.ts` | `apps/server/src/persistence/Migrations/054_ProjectionThreadsAutoSettleDisabledAt.test.ts` |
| 12% | 27 | 81 | `apps/web/src/lib/keybindingCapture.ts` | `apps/web/src/components/settings/KeybindingsSettings.logic.ts` |
| 12% | 3 | 45 | `packages/shared/src/threadWorkspace.test.ts` | `apps/mobile/src/features/showcase/showcaseRenderSignal.test.ts` |
| 12% | 9 | 41 | `apps/web/src/lib/hosts/activeHost.test.ts` | `apps/web/src/localApi.test.ts` |
| 12% | 26 | 246 | `apps/server/src/provider/acp/AcpNativeLogging.ts` | `apps/server/src/provider/acp/AcpNativeLogging.ts` |
| 12% | 20 | 72 | `apps/web/src/markdown-links.test.ts` | `apps/web/src/markdown-links.test.ts` |
| 12% | 243 | 3238 | `apps/web/src/workLog.ts` | `apps/web/src/session-logic.ts` |
| 12% | 1 | 9 | `apps/server/src/orchestration/Services/ThreadGitMetadataReactor.ts` | `apps/server/src/orchestration/Services/CheckpointReactor.ts` |
| 12% | 1 | 8 | `apps/server/src/orchestration/Services/ThreadSnoozeReactor.ts` | `apps/server/src/provider/Services/ProviderSessionReaper.ts` |
| 12% | 1 | 8 | `apps/server/src/orchestration/Services/UsageLimitRecoveryReactor.ts` | `apps/server/src/provider/Services/ProviderSessionReaper.ts` |
| 12% | 4 | 31 | `apps/web/src/components/ProjectSourceSegmentedPicker.tsx` | `apps/mobile/src/features/settings/components/SettingsSwitchRow.tsx` |
| 12% | 1 | 10 | `apps/web/src/lib/pin.tsx` | `apps/web/src/components/T3Wordmark.tsx` |
| 12% | 3 | 25 | `apps/web/src/lib/visibleInterval.ts` | `apps/desktop/src/snapShot/GlobalShiftShortcutProcess.ts` |
| 12% | 4 | 22 | `packages/profile-ui/src/avatar.tsx` | `apps/web/src/components/settings/EnvironmentRow.tsx` |
| 12% | 82 | 332 | `apps/server/src/auth/Layers/ServerAuth.ts` | `apps/server/src/auth/EnvironmentAuth.ts` |
| 12% | 124 | 992 | `apps/server/src/serverSettings.ts` | `apps/server/src/serverSettings.ts` |
| 12% | 13 | 94 | `apps/server/src/persistence/Migrations/092_BackfillAutomationRunThreadSource.test.ts` | `apps/server/src/persistence/Migrations/046_RepairAutomaticSettlementTimestamps.test.ts` |
| 12% | 3 | 25 | `apps/web/src/hooks/useAppDensity.ts` | `apps/mobile/src/features/cloud/connectOnboardingNavigation.ts` |
| 12% | 3 | 25 | `apps/web/src/hooks/useChatWidth.ts` | `apps/mobile/src/features/cloud/connectOnboardingNavigation.ts` |
| 12% | 7 | 54 | `apps/web/src/pinnedThreadsStore.ts` | `apps/web/src/diffPanelStore.ts` |
| 12% | 16 | 144 | `apps/web/src/components/chat/ComposerModelPickerRow.tsx` | `apps/web/src/components/settings/EnvironmentIconPicker.tsx` |
| 11% | 5 | 75 | `apps/web/src/hooks/useCopyToClipboard.test.ts` | `apps/web/src/environments/primary/httpLayer.test.ts` |
| 11% | 97 | 512 | `apps/web/src/session-logic.ts` | `apps/web/src/session-logic.ts` |
| 11% | 2 | 21 | `apps/profiles/src/lib/profileAccent.ts` | `apps/web/src/pierre-tree-theme.ts` |
| 11% | 12 | 124 | `apps/server/src/pullRequests.logic.test.ts` | `apps/web/src/components/pullRequest/pullRequestChecks.test.tsx` |
| 11% | 2 | 20 | `apps/web/src/lib/providerUsageSnapshot.test.ts` | `apps/mobile/src/features/layout/native-mail-search-toolbar.ts` |
| 11% | 2 | 14 | `packages/shared/src/wslBridge.ts` | `apps/web/src/components/files/fileContentRevision.ts` |
| 11% | 9 | 43 | `apps/server/src/checkpointing/Diffs.ts` | `apps/web/src/components/diffs/diffFileTree.logic.ts` |
| 11% | 26 | 121 | `apps/server/src/orchestration/decider.subagentMessaging.test.ts` | `apps/server/src/orchestration/commandInvariants.test.ts` |
| 11% | 5 | 28 | `apps/marketing/src/components/docs/callout.tsx` | `apps/web/src/components/clerk/ClerkUserProfilePage.tsx` |
| 11% | 11 | 30 | `apps/server/src/persistence/Migrations/061_PendingApprovalSettlementState.test.ts` | `apps/server/src/persistence/Migrations/025_CleanupInvalidProjectionPendingApprovals.test.ts` |
| 11% | 9 | 53 | `apps/server/src/orchestration/Layers/ThreadDeletionReactor.test.ts` | `apps/server/src/orchestration/Layers/ThreadDeletionReactor.test.ts` |
| 11% | 9 | 42 | `apps/web/src/components/SidebarProjectRowContent.tsx` | `apps/web/src/components/pullRequest/PullRequestCandidatePicker.tsx` |
| 11% | 3 | 23 | `apps/web/src/components/settings/agentCursorDesktopSync.ts` | `apps/web/src/planAgentSelectionHeal.tsx` |
| 11% | 15 | 55 | `apps/server/src/persistence/Migrations/119_ProviderSessionRuntimeInstanceId.test.ts` | `apps/server/src/persistence/RepositoryErrorCorrelation.test.ts` |
| 11% | 5 | 74 | `apps/server/src/persistence/Migrations/103_ClaudeTokenAccounting.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 11% | 14 | 92 | `apps/web/src/components/chat/ProviderHealthBanner.tsx` | `apps/web/src/components/chat/ProviderStatusBanner.tsx` |
| 11% | 16 | 53 | `apps/web/src/components/ui/icon-button.tsx` | `apps/web/src/components/sidebar/SidebarThreadHeader.tsx` |
| 11% | 3 | 69 | `apps/server/src/provider/codexAccountIdentity.test.ts` | `apps/mobile/src/features/connection/pairing.test.ts` |
| 11% | 10 | 18 | `apps/web/src/components/pullRequest/PullRequestMetaRow.tsx` | `apps/web/src/components/settings/SettingsScopeSentence.tsx` |
| 11% | 4 | 75 | `apps/web/src/lib/threadEnvironment.test.ts` | `apps/mobile/src/features/threads/new-task-context-presentation.test.ts` |
| 11% | 30 | 193 | `apps/web/src/components/settings/SettingsPanelPrimitives.tsx` | `apps/web/src/components/settings/settingsLayout.tsx` |
| 11% | 1 | 9 | `apps/server/src/provider/Services/ProviderRuntimeReconciler.ts` | `apps/server/src/provider/Services/ProviderSessionReaper.ts` |
| 11% | 4 | 51 | `apps/web/src/chat-scroll.test.ts` | `apps/mobile/src/features/showcase/showcaseRenderSignal.test.ts` |
| 11% | 7 | 120 | `apps/web/src/components/tasks/TaskGlyphs.tsx` | `apps/web/src/components/ui/checkbox.tsx` |
| 11% | 3 | 9 | `apps/web/src/components/ui/DisclosureChevron.tsx` | `apps/web/src/components/settings/FoldedSettingsSection.tsx` |
| 11% | 2 | 23 | `apps/web/src/lib/hosts/workspacePresentation.test.ts` | `apps/web/src/lib/snapShotSetupResume.test.ts` |
| 11% | 132 | 1724 | `apps/web/src/composerDraftActions.ts` | `apps/web/src/composerDraftStore.ts` |
| 11% | 4 | 44 | `apps/server/src/git/Errors.ts` | `apps/server/src/provider/Errors.ts` |
| 11% | 5 | 32 | `apps/desktop/src/runtimeArch.test.ts` | `apps/desktop/src/app/DesktopAssets.test.ts` |
| 11% | 7 | 56 | `apps/server/src/projectAgent/principal.ts` | `apps/server/src/orchestration/Services/ProjectionSnapshotQuery.ts` |
| 11% | 121 | 1195 | `apps/server/src/orchestration/providerRuntimeActivityProjection.ts` | `apps/server/src/orchestration/Layers/ProviderRuntimeIngestion.ts` |
| 11% | 5 | 100 | `packages/shared/src/browserAnnotations.test.ts` | `apps/web/src/lib/openPullRequestLink.test.ts` |
| 11% | 13 | 188 | `packages/contracts/src/automation.test.ts` | `packages/contracts/src/keybindings.test.ts` |
| 11% | 3 | 58 | `apps/web/src/components/ThreadTerminalDrawer.test.ts` | `apps/mobile/src/features/cloud/publicConfig.test.ts` |
| 11% | 5 | 18 | `apps/web/src/components/chat/PanelStateMessage.tsx` | `apps/web/src/components/DiffPanelShell.tsx` |
| 11% | 19 | 44 | `apps/server/src/auth/Services/BootstrapCredentialService.ts` | `apps/server/src/auth/PairingGrantStore.ts` |
| 11% | 15 | 117 | `apps/web/src/components/settings/views/GeneralSettingsView.tsx` | `apps/web/src/components/settings/SourceControlWritingSettings.tsx` |
| 11% | 2 | 19 | `apps/web/src/components/settings/accounts/ProviderGuidedSignIn.tsx` | `apps/web/src/components/diffs/DiffFileLoadingBoundary.tsx` |
| 11% | 67 | 823 | `apps/server/src/checkpointing/Layers/CheckpointStore.ts` | `apps/server/src/vcs/GitVcsDriver.ts` |
| 11% | 34 | 136 | `apps/server/src/provider/claudeAuthStatus.ts` | `apps/server/src/provider/Layers/ClaudeProvider.ts` |
| 11% | 25 | 105 | `apps/server/src/orchestration/decider.forkFromTurn.test.ts` | `apps/server/src/orchestration/decider.projectScripts.test.ts` |
| 11% | 7 | 40 | `packages/shared/src/localPreviewFiles.ts` | `apps/server/src/imageMime.ts` |
| 11% | 8 | 74 | `apps/server/src/provider/acp/GrokAcpExtension.ts` | `apps/server/src/provider/acp/CursorAcpExtension.ts` |
| 11% | 16 | 156 | `apps/web/src/components/account/AccountFooterControl.tsx` | `apps/web/src/components/preview/PreviewMoreMenu.tsx` |
| 11% | 5 | 53 | `apps/server/src/provider/Services/ProviderDiscoveryService.ts` | `apps/server/src/provider/Services/ProviderSessionDirectory.ts` |
| 11% | 3 | 61 | `apps/server/src/computer/computerActivation.test.ts` | `apps/mobile/src/features/cloud/publicConfig.test.ts` |
| 11% | 14 | 83 | `apps/web/src/components/AppRailMoreMenu.tsx` | `apps/web/src/components/preview/PreviewMoreMenu.tsx` |
| 11% | 20 | 72 | `apps/web/src/components/composerFooterLayout.test.ts` | `apps/web/src/components/composerFooterLayout.test.ts` |
| 11% | 24 | 219 | `apps/server/src/orchestration/Layers/ThreadSnoozeReactor.test.ts` | `apps/server/src/orchestration/decider.active-order.test.ts` |
| 11% | 5 | 3579 | `apps/marketing/src/data/changelog.ts` | `packages/effect-acp/src/_generated/schema.gen.ts` |
| 11% | 16 | 154 | `packages/contracts/src/pullRequests.test.ts` | `apps/web/src/components/pullRequest/PullRequestSummaryTab.test.tsx` |
| 11% | 43 | 310 | `apps/server/src/open.ts` | `apps/server/src/process/externalLauncher.ts` |
| 11% | 1 | 11 | `apps/analytics/scripts/import.sh` | `.devcontainer/update-content.sh` |
| 11% | 3 | 35 | `apps/profiles/src/components/chrome.tsx` | `apps/mobile/src/components/MaterialScreenContent.tsx` |
| 11% | 1 | 12 | `apps/server/vitest.config.ts` | `packages/client-runtime/vite.config.ts` |
| 11% | 4 | 37 | `apps/web/src/components/chat/ComposerLiveChangesHeader.tsx` | `apps/web/src/components/chat/DiffStatLabel.tsx` |
| 11% | 2 | 16 | `apps/web/src/components/chat/LiveElapsedTimer.tsx` | `apps/mobile/src/features/agent-awareness/notificationNavigation.ts` |
| 11% | 10 | 30 | `apps/web/src/components/chat/group/GroupPausedBanner.tsx` | `apps/web/src/components/files/fileSurfaceChrome.tsx` |
| 11% | 7 | 21 | `apps/web/src/components/ui/diff-stat.tsx` | `apps/web/src/components/pullRequest/PullRequestListRow.tsx` |
| 11% | 4 | 69 | `apps/web/src/lib/hosts/workspaceKeybindings.test.ts` | `apps/mobile/src/features/showcase/showcaseRenderSignal.ts` |
| 10% | 12 | 55 | `apps/server/src/environment/Layers/ServerEnvironment.test.ts` | `apps/server/src/environment/ServerEnvironment.test.ts` |
| 10% | 28 | 129 | `apps/server/src/persistence/Migrations/123_ScrubOrchestrationEventProviderOptions.test.ts` | `apps/server/src/persistence/Layers/OrchestrationEventStore.test.ts` |
| 10% | 11 | 115 | `apps/web/src/components/RestoreOrCreateChatRoute.tsx` | `apps/web/src/components/ThreadRouteView.tsx` |
| 10% | 12 | 167 | `packages/contracts/src/ws.test.ts` | `packages/contracts/src/keybindings.test.ts` |
| 10% | 8 | 60 | `apps/server/src/persistence/Migrations/107_ProjectionThreadsHumanMessage.test.ts` | `apps/server/src/persistence/Migrations/024_BackfillProjectionThreadShellSummary.test.ts` |
| 10% | 173 | 5136 | `apps/web/src/workLog.test.ts` | `apps/web/src/session-logic.test.ts` |
| 10% | 281 | 6166 | `apps/server/src/orchestration/Layers/CheckpointReactor.test.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.test.ts` |
| 10% | 14 | 138 | `apps/web/src/components/pullRequest/GitHubItemHeader.tsx` | `apps/web/src/components/pullRequest/PullRequestListRow.tsx` |
| 10% | 10 | 83 | `apps/desktop/src/syncShellEnvironment.ts` | `apps/server/src/os-jank.ts` |
| 10% | 9 | 42 | `apps/web/src/components/pullRequest/PullRequestAvatar.tsx` | `apps/web/src/components/pullRequest/PullRequestListRow.tsx` |
| 10% | 11 | 183 | `packages/shared/src/serverSettings.test.ts` | `apps/web/src/onboarding/providerReadiness.logic.test.ts` |
| 10% | 4 | 33 | `apps/marketing/src/data/product.ts` | `apps/mobile/src/features/usage/usageProviders.ts` |
| 10% | 5 | 60 | `apps/web/src/lib/threadImport.test.ts` | `packages/contracts/src/server.test.ts` |
| 10% | 5 | 59 | `apps/web/src/test/reactHookHarness.ts` | `apps/web/src/test/reactHookHarness.ts` |
| 10% | 2 | 19 | `apps/server/src/attachmentPaths.test.ts` | `apps/desktop/src/app/DesktopAppErrors.test.ts` |
| 10% | 7 | 41 | `apps/web/src/components/tasks/taskRowContextMenu.ts` | `apps/web/src/components/threadActionMenu.logic.ts` |
| 10% | 31 | 246 | `apps/server/src/transcriptUsage.ts` | `apps/server/src/usage/usageTranscriptReader.ts` |
| 10% | 7 | 26 | `apps/web/src/featureFlags.test.ts` | `apps/web/src/localApi.test.ts` |
| 10% | 4 | 55 | `apps/web/src/components/chat/deferredChatMount.ts` | `apps/desktop/src/snapShot/GlobalShiftShortcutProcess.ts` |
| 10% | 5 | 19 | `apps/web/src/components/settings/UsageLimitRecoverySetting.tsx` | `apps/web/src/components/settings/LocalEnvironmentSetting.tsx` |
| 10% | 10 | 48 | `apps/web/src/components/composerFooterLayout.ts` | `apps/web/src/components/composerFooterLayout.ts` |
| 10% | 9 | 46 | `apps/web/src/components/SidebarThreadRowContent.browser.tsx` | `apps/mobile/src/features/archive/archivedThreadList.test.ts` |
| 10% | 4 | 40 | `apps/web/src/lib/terminalContextComposerRegistry.test.ts` | `apps/mobile/src/features/showcase/showcaseRenderSignal.test.ts` |
| 10% | 26 | 258 | `apps/server/src/threadRetention.test.ts` | `apps/server/src/orchestration/decider.active-order.test.ts` |
| 10% | 146 | 1804 | `apps/web/src/components/Sidebar.logic.test.ts` | `apps/web/src/components/Sidebar.logic.test.ts` |
| 10% | 4 | 37 | `apps/server/src/auth/Layers/ServerSecretStore.test.ts` | `apps/server/src/cloud/CliState.test.ts` |
| 10% | 4 | 99 | `apps/desktop/src/windowMaterial.test.ts` | `apps/mobile/src/features/agent-awareness/notificationNavigation.test.ts` |
| 10% | 6 | 56 | `apps/web/src/hooks/usePreloadRouteChunks.browser.tsx` | `apps/web/src/lib/assistantCitationNavigation.test.ts` |
| 10% | 17 | 208 | `apps/server/src/persistence/Migrations/122_ClearAutomationRunProviderOptions.test.ts` | `apps/server/src/persistence/Migrations/024_BackfillProjectionThreadShellSummary.test.ts` |
| 10% | 6 | 66 | `apps/marketing/src/components/BrandIcons.tsx` | `apps/mobile/src/components/ProviderIcon.tsx` |
| 10% | 5 | 37 | `apps/marketing/src/components/SplitShowcase.tsx` | `apps/web/src/components/ui/standalone-page.tsx` |
| 10% | 1 | 9 | `apps/server/src/codexServiceTier.ts` | `apps/server/src/codexModelOptions.ts` |
| 10% | 2 | 24 | `apps/server/src/orchestration/Services/TurnCheckpointCoordinator.ts` | `apps/server/src/workspace/workspaceLease.ts` |
| 10% | 11 | 103 | `apps/server/src/terminal/Layers/BunPTY.ts` | `apps/server/src/terminal/NodePtyAdapter.ts` |
| 10% | 8 | 39 | `apps/web/src/components/SponsorLandingBanner.tsx` | `apps/web/src/browser/BrowserViewportResizeHandles.tsx` |
| 10% | 7 | 70 | `apps/web/src/components/pullRequest/PullRequestChecksRing.tsx` | `apps/web/src/components/chat/ContextWindowMeter.tsx` |
| 10% | 1 | 7 | `apps/web/src/hooks/useKeepAwakeState.ts` | `apps/web/src/connection/useDesktopLocalBootstraps.ts` |
| 10% | 4 | 18 | `packages/profile-ui/src/logo.tsx` | `apps/web/src/components/ui/checkbox.tsx` |
| 10% | 27 | 116 | `apps/server/src/git/Layers/GitStatusBroadcaster.ts` | `apps/server/src/vcs/VcsStatusBroadcaster.ts` |
| 10% | 14 | 61 | `apps/web/src/components/chat/QueuedComposerActions.tsx` | `apps/web/src/components/pullRequest/ThreadPullRequestsPanel.tsx` |
| 10% | 14 | 80 | `apps/web/src/components/chat/environment/EnvironmentEditorSection.tsx` | `apps/web/src/components/settings/SettingsScopeSentence.tsx` |
| 10% | 5 | 36 | `apps/web/src/lib/terminalContextComposerRegistry.ts` | `apps/mobile/src/connection/background-activity-scopes.ts` |
| 10% | 8 | 70 | `apps/web/src/components/chat/AttachmentCard.tsx` | `apps/web/src/components/pullRequest/PullRequestListRow.tsx` |
| 10% | 18 | 115 | `apps/web/src/components/pullRequest/PullRequestRow.tsx` | `apps/web/src/components/pullRequest/PullRequestRow.tsx` |
| 10% | 3 | 21 | `apps/web/src/components/ReviewFileTreePanel.test.tsx` | `apps/web/src/components/settings/ProjectIconPickerDialog.test.tsx` |
| 10% | 8 | 23 | `apps/web/src/components/pullRequest/PullRequestWarningNote.tsx` | `apps/web/src/components/ui/dialog.tsx` |
| 10% | 16 | 147 | `apps/web/src/test/effectRpcWebSocketMock.ts` | `apps/mobile/src/state/use-thread-selection.ts` |
| 10% | 2 | 19 | `apps/profiles/src/app/layout.tsx` | `apps/mobile/src/components/MaterialScreenContent.tsx` |
| 10% | 6 | 32 | `apps/server/src/externalMcp/auditCompletion.test.ts` | `apps/server/src/cloud/managedTunnelStartup.test.ts` |
| 10% | 5 | 49 | `apps/web/src/components/settings/McpConnectorCard.tsx` | `apps/web/src/components/device/DeviceLoadingView.tsx` |
| 10% | 2 | 23 | `apps/web/src/explorerRevealRequestStore.ts` | `apps/web/src/browser/browserPointerStore.ts` |
| 10% | 2 | 17 | `apps/web/src/routes/-threadDetailOwnership.ts` | `apps/server/src/orchestration/threadDetailCursor.ts` |
| 10% | 4 | 13 | `apps/web/src/lib/textGenerationCapabilities.ts` | `apps/mobile/src/lib/projectThreadStartTurn.ts` |
| 10% | 32 | 80 | `apps/server/src/auth/Services/SessionCredentialService.ts` | `apps/server/src/auth/SessionStore.ts` |
| 10% | 16 | 177 | `apps/server/src/provider/acp/GrokAcpSupport.ts` | `apps/server/src/provider/acp/GrokAcpSupport.ts` |
| 10% | 4 | 25 | `apps/web/src/temporaryThreadStore.ts` | `apps/web/src/previewMiniPlayerStore.ts` |
| 10% | 10 | 248 | `apps/server/src/provider/acp/AcpAdapterSupport.test.ts` | `apps/server/src/provider/acp/AcpAdapterSupport.test.ts` |
| 10% | 13 | 66 | `apps/web/src/components/SidebarIconButton.tsx` | `apps/web/src/components/sidebar/SidebarThreadHeader.tsx` |
| 10% | 20 | 95 | `apps/web/src/components/chat/environment/EnvironmentActionsSection.tsx` | `apps/web/src/components/ProjectScriptsControl.tsx` |
| 10% | 6 | 45 | `apps/marketing/src/components/docs/card.tsx` | `apps/web/src/components/clerk/ClerkUserProfilePage.tsx` |
| 10% | 4 | 21 | `apps/marketing/src/lib/ctaButton.ts` | `apps/web/src/components/ui/badge.tsx` |
| 10% | 2 | 22 | `apps/server/src/agentGateway/toolInput.test.ts` | `apps/desktop/src/app/DesktopAppErrors.test.ts` |
| 10% | 4 | 48 | `apps/server/src/persistence/Migrations/138_RemoteDeviceTrust.ts` | `apps/server/src/persistence/Migrations/003_CheckpointDiffBlobs.ts` |
| 10% | 1 | 16 | `apps/web/playwright.electron.config.ts` | `apps/marketing/astro.config.mjs` |
| 10% | 6 | 14 | `apps/web/src/components/chat/ChatColumnBannerFrame.tsx` | `apps/web/src/components/pullRequest/PullRequestListRow.tsx` |
| 10% | 3 | 29 | `apps/web/src/components/chat/FileCommentsSummaryChip.tsx` | `apps/web/src/components/files/fileCommentAnnotations.ts` |
| 10% | 1 | 11 | `apps/web/src/hooks/useDismissedThreadStatusKeys.ts` | `apps/web/src/hooks/useTerminalFocus.ts` |
| 10% | 2 | 15 | `apps/web/src/hooks/useNativeFontSmoothing.ts` | `apps/mobile/src/features/cloud/connectOnboardingNavigation.ts` |
| 10% | 1 | 22 | `apps/web/src/lib/pullRequestReactQuery.ts` | `packages/client-runtime/src/voice-input/index.ts` |
| 10% | 24 | 375 | `apps/web/src/components/chat/ChatComposerFooter.tsx` | `apps/web/src/components/chat/ComposerPrimaryActions.tsx` |
| 9% | 14 | 216 | `apps/server/src/provider/acp/DroidAcpSupport.ts` | `apps/server/src/provider/acp/CursorAcpSupport.ts` |
| 9% | 139 | 1706 | `apps/web/src/components/chat/MessagesTimeline.logic.ts` | `apps/web/src/components/chat/MessagesTimeline.logic.ts` |
| 9% | 9 | 41 | `apps/server/src/worktreeSetup.test.ts` | `apps/web/src/projectScripts.test.ts` |
| 9% | 16 | 106 | `apps/server/src/persistence/Layers/AuthAccessOrigin.test.ts` | `apps/server/src/persistence/RepositoryErrorCorrelation.test.ts` |
| 9% | 6 | 36 | `apps/server/src/auth/utils.test.ts` | `apps/web/src/connection/clientMetadata.test.ts` |
| 9% | 4 | 64 | `apps/web/src/components/BetaWelcomeDialog.browser.tsx` | `apps/web/src/environments/primary/desktopAuth.test.ts` |
| 9% | 8 | 63 | `apps/marketing/src/components/ProviderMarkRow.tsx` | `apps/web/src/components/settings/providerDriverMeta.ts` |
| 9% | 24 | 137 | `apps/server/src/diagnostics/Layers/ThreadDiagnosticsQuery.ts` | `apps/server/src/persistence/Layers/ProjectionThreadActivities.ts` |
| 9% | 4 | 28 | `apps/web/src/hooks/useRefreshOnWindowReturn.ts` | `apps/web/src/components/permissions/usePermissionStatus.ts` |
| 9% | 26 | 292 | `apps/server/src/wsStreamBackpressure.ts` | `apps/server/src/orchestration/LiveStreamBudget.ts` |
| 9% | 17 | 210 | `apps/web/src/components/chat/composerProviderRegistry.tsx` | `apps/web/src/components/chat/composerProviderState.tsx` |
| 9% | 39 | 484 | `apps/web/src/components/codeEditor/EditorDebugPanel.tsx` | `apps/web/src/components/PullRequestThreadDialog.tsx` |
| 9% | 14 | 78 | `apps/server/src/persistence/projectionThreadMessageRow.ts` | `apps/server/src/persistence/Layers/ProjectionThreadMessages.ts` |
| 9% | 4 | 22 | `apps/marketing/src/components/docs/DocsSmoothScroll.tsx` | `apps/mobile/src/features/keyboard/hardwareKeyboardCommands.ts` |
| 9% | 4 | 19 | `apps/web/src/components/DiffTruncationWarning.tsx` | `apps/web/src/components/chat/ComposerSurface.tsx` |
| 9% | 7 | 32 | `apps/web/src/components/tasks/TaskCardPrimitives.tsx` | `apps/web/src/components/ui/dialog.tsx` |
| 9% | 2 | 18 | `benchmarks/desktop-perf/prepare.mjs` | `apps/desktop/src/snapShot/GlobalShiftShortcutWorker.ts` |
| 9% | 39 | 384 | `apps/server/src/orchestration/decider.checkpointRevert.test.ts` | `apps/server/src/orchestration/decider.settled.test.ts` |
| 9% | 6 | 28 | `apps/web/src/latestProjectStore.ts` | `apps/web/src/diffPanelStore.ts` |
| 9% | 4 | 17 | `apps/web/src/lib/nativeSurfaceOcclusion.ts` | `apps/web/src/components/AnimatedHeight.tsx` |
| 9% | 6 | 46 | `packages/shared/src/pendingInteractions.test.ts` | `apps/web/src/components/chat/ModelPickerContent.test.ts` |
| 9% | 6 | 54 | `apps/marketing/src/components/HomepageRail.tsx` | `apps/web/src/components/preview/AgentBrowserCursor.tsx` |
| 9% | 5 | 55 | `apps/server/src/agentGateway/turnPlanTools.test.ts` | `packages/ssh/src/auth.test.ts` |
| 9% | 6 | 56 | `apps/server/src/persistence/Migrations/132_ExternalMcpTurnCapacityRecovery.ts` | `apps/server/src/persistence/Migrations/046_RepairAutomaticSettlementTimestamps.ts` |
| 9% | 14 | 198 | `apps/web/src/components/chat/ComposerModelPickerTraitRows.tsx` | `apps/web/src/components/chat/composerProviderState.tsx` |
| 9% | 2 | 10 | `apps/analytics/src/debounce.ts` | `apps/mobile/src/features/review/reviewDiffHighlightScheduler.ts` |
| 9% | 10 | 52 | `apps/server/src/persistence/Migrations/062_PendingInteractionSettlementParity.test.ts` | `apps/server/src/persistence/Migrations/025_CleanupInvalidProjectionPendingApprovals.test.ts` |
| 9% | 4 | 60 | `apps/server/src/projectAgent/groupCoordinatorHost.test.ts` | `apps/mobile/src/features/settings/environment-maintenance.test.ts` |
| 9% | 8 | 55 | `apps/web/src/components/MoveProjectToSpaceMenu.tsx` | `apps/web/src/components/settings/EnvironmentIconPicker.tsx` |
| 9% | 8 | 9 | `packages/shared/src/schemaJson.ts` | `packages/shared/src/schemaJson.ts` |
| 9% | 1 | 9 | `packages/shared/src/threadExport.ts` | `apps/web/src/clientPersistenceStorage.ts` |
| 9% | 4 | 71 | `packages/shared/src/uiTreeTargeting.ts` | `apps/mobile/src/components/MaterialMenuPopup.tsx` |
| 9% | 58 | 700 | `apps/web/src/components/chat/ChatHeader.tsx` | `apps/web/src/components/chat/ChatHeader.tsx` |
| 9% | 11 | 114 | `apps/web/src/lib/binaryFrameSource.ts` | `apps/desktop/src/electron/WindowsForegroundFocusThread.ts` |
| 9% | 11 | 227 | `apps/web/src/shortcutsSheet.test.ts` | `apps/web/src/components/settings/KeybindingsSettings.logic.test.ts` |
| 9% | 7 | 122 | `apps/server/src/persistence/Migrations/032_ReconcileImportedSchemaLineage.ts` | `apps/server/src/persistence/Migrations/010_ProjectionThreadsRuntimeMode.ts` |
| 9% | 7 | 104 | `apps/server/src/persistence/Migrations/MigrationReplay.test.ts` | `apps/server/src/persistence/Migrations/054_ProjectionThreadsAutoSettleDisabledAt.test.ts` |
| 9% | 5 | 125 | `apps/server/src/providerUsage/providers/cursorSafeStorage.test.ts` | `apps/mobile/src/features/agent-awareness/notificationNavigation.test.ts` |
| 9% | 28 | 247 | `apps/server/src/auth/Layers/SessionCredentialService.test.ts` | `apps/server/src/auth/SessionStore.test.ts` |
| 9% | 3 | 44 | `apps/desktop/src/macIconCacheRefresh.test.ts` | `apps/mobile/src/features/threads/markdownImageSize.test.ts` |
| 9% | 43 | 946 | `apps/server/src/workspaceEntries.ts` | `packages/shared/src/searchRanking.ts` |
| 9% | 11 | 88 | `apps/web/src/hooks/useThreadActionShortcuts.ts` | `apps/web/src/routes/_chat.tsx` |
| 9% | 8 | 394 | `apps/server/src/open.test.ts` | `scripts/sync-reference-repos.test.ts` |
| 9% | 6 | 39 | `apps/web/src/components/chat/useExpandedImagePreview.ts` | `apps/web/src/components/chat/ExpandedImageDialog.tsx` |
| 9% | 5 | 72 | `apps/analytics/src/TurnBreakdownTable.tsx` | `apps/web/src/components/files/DelimitedTablePreview.tsx` |
| 9% | 33 | 182 | `apps/web/src/components/EditProjectDialog.tsx` | `apps/web/src/components/projectScriptEditor.tsx` |
| 9% | 5 | 110 | `apps/web/src/components/settings/ConnectionsRows.test.tsx` | `apps/web/src/components/pullRequest/pullRequestEditing.logic.test.ts` |
| 9% | 429 | 8620 | `apps/server/src/provider/Layers/ProviderService.test.ts` | `apps/server/src/provider/Layers/ProviderService.test.ts` |
| 9% | 56 | 888 | `apps/server/src/provider/acp/CursorAcpSupport.test.ts` | `apps/server/src/provider/acp/CursorAcpSupport.test.ts` |
| 9% | 2 | 20 | `apps/server/src/eventLoopDeadline.ts` | `apps/desktop/src/snapShot/GlobalShiftShortcutWorker.ts` |
| 9% | 2 | 25 | `apps/server/src/provider/codexCliVersion.test.ts` | `apps/desktop/src/app/DesktopAppErrors.test.ts` |
| 9% | 4 | 18 | `apps/web/src/chat-scroll.ts` | `apps/web/src/state/paginatedBranches.ts` |
| 9% | 29 | 316 | `apps/web/src/components/hosts/ProjectCatalogDialog.tsx` | `apps/web/src/components/settings/DeviceHostEditor.tsx` |
| 9% | 2 | 20 | `apps/web/src/routes/_chat.remote.tsx` | `apps/web/src/routes/settings.providers.tsx` |
| 9% | 27 | 250 | `apps/web/src/components/settings/ConnectionsAddDeviceDialog.tsx` | `apps/web/src/components/settings/DeviceHostEditor.tsx` |
| 9% | 26 | 363 | `apps/desktop/src/updateState.test.ts` | `apps/web/src/components/desktopUpdate.logic.test.ts` |
| 9% | 12 | 192 | `apps/server/src/persistence/Migrations/098_MigrateKiloToOpenCode.test.ts` | `apps/server/src/persistence/Migrations/024_BackfillProjectionThreadShellSummary.test.ts` |
| 9% | 8 | 64 | `apps/web/src/components/codeEditor/EditorActionPicker.tsx` | `apps/web/src/components/pullRequest/PullRequestCandidatePicker.tsx` |
| 9% | 6 | 67 | `apps/web/src/lib/expensiveReadRetry.test.ts` | `apps/mobile/src/features/threads/thread-settings-sheet-state.test.ts` |
| 9% | 14 | 250 | `apps/web/src/components/settings/views/ChatsSettingsView.tsx` | `apps/web/src/components/settings/SourceControlWritingSettings.tsx` |
| 9% | 2 | 118 | `apps/server/src/git/gitBlameParsing.test.ts` | `apps/server/src/provider/opencodeRuntime.cliParsers.test.ts` |
| 9% | 3 | 27 | `apps/web/src/components/chat/FileAttachmentChip.test.tsx` | `apps/mobile/src/features/threads/QuestionAnswerHistory.test.tsx` |
| 9% | 5 | 66 | `apps/web/src/hooks/useProjectImportAnnouncement.browser.tsx` | `apps/web/src/components/files/AttachmentFilePreview.test.tsx` |
| 9% | 4 | 78 | `apps/web/src/lib/localImageUrls.test.ts` | `apps/web/src/components/ThreadTerminalDrawer.test.ts` |
| 9% | 6 | 26 | `apps/web/src/components/ui/search-input.tsx` | `apps/web/src/components/chat/ComposerControl.tsx` |
| 9% | 10 | 87 | `apps/web/src/hooks/useDesktopMenuShortcuts.test.ts` | `apps/web/src/lib/snapShotShortcut.test.ts` |
| 9% | 4 | 71 | `apps/web/src/lib/providerDiscovery.test.ts` | `apps/web/src/providerSkillSearch.test.ts` |
| 9% | 9 | 40 | `apps/web/src/components/UsageProgressTrack.tsx` | `apps/web/src/components/sidebar/SidebarThreadHeader.tsx` |
| 9% | 6 | 136 | `apps/web/src/components/hosts/RemoteConnectionsAnnouncementHero.tsx` | `apps/mobile/src/components/SourceControlIcon.tsx` |
| 9% | 6 | 67 | `apps/server/src/orchestration/providerOptionsSecurity.ts` | `apps/mobile/src/features/usage/usageProviders.ts` |
| 9% | 7 | 111 | `apps/server/src/provider/providerLifecycleCoordinator.test.ts` | `apps/server/src/project/WorktreeSetupTracker.test.ts` |
| 9% | 15 | 136 | `apps/web/src/components/RunningChatsQuitDialog.tsx` | `apps/web/src/components/ui/alert-dialog.tsx` |
| 9% | 2 | 22 | `apps/web/src/hooks/useThreadFastModeNotice.ts` | `apps/server/src/orchestration/threadDetailCursor.ts` |
| 9% | 9 | 24 | `apps/web/src/components/ui/keycap.tsx` | `apps/web/src/components/settings/ThemePreviewCircles.tsx` |
| 9% | 53 | 748 | `apps/web/src/terminalStateStore.ts` | `apps/web/src/terminalUiStateStore.ts` |
| 9% | 20 | 268 | `apps/web/src/components/settings/McpConnectorConnectDialog.tsx` | `apps/web/src/components/PullRequestThreadDialog.tsx` |
| 9% | 4 | 72 | `apps/web/src/lib/toolArgumentSummary.test.ts` | `apps/mobile/src/features/threads/thread-feed-live-follow.test.ts` |
| 9% | 3 | 24 | `apps/web/src/whatsNew/FeatureSection.tsx` | `apps/web/src/components/settings/EnvironmentRow.tsx` |
| 9% | 4 | 36 | `packages/shared/src/worktreeHandoff.test.ts` | `apps/mobile/src/features/threads/new-task-context-presentation.test.ts` |
| 9% | 34 | 531 | `apps/web/src/routes/_chat.tsx` | `apps/web/src/components/AppSidebarLayout.tsx` |
| 9% | 6 | 74 | `apps/web/src/lib/runtimeMode.test.ts` | `apps/web/src/components/chat/ContextWindowMeter.logic.test.ts` |
| 9% | 18 | 148 | `apps/web/src/components/chat/ProposedPlanActions.tsx` | `apps/web/src/components/chat/ProposedPlanCard.tsx` |
| 9% | 14 | 84 | `apps/web/src/components/pullRequest/PullRequestCommentCard.tsx` | `apps/web/src/components/chat/ComposerPendingUserInputPanel.tsx` |
| 9% | 2 | 16 | `apps/web/src/components/profile/ProfileUsageCoverage.tsx` | `apps/mobile/src/components/MaterialNewThreadButton.shared.tsx` |
| 9% | 6 | 50 | `apps/web/src/lib/fastModeState.ts` | `apps/web/src/hooks/useWorkspaceMutationRefresh.ts` |
| 8% | 29 | 169 | `apps/web/src/components/pullRequest/PullRequestsUnavailableState.tsx` | `apps/web/src/components/settings/SourceControlSettings.tsx` |
| 8% | 62 | 428 | `apps/web/src/composerDraftStore.test.ts` | `apps/web/src/composerDraftStore.test.ts` |
| 8% | 10 | 101 | `apps/web/src/storageOriginMigration.test.ts` | `apps/web/src/localApi.test.ts` |
| 8% | 5 | 99 | `apps/server/src/provider/openCodeBackgroundTasks.ts` | `packages/client-runtime/src/work-log/toolPresentation.ts` |
| 8% | 7 | 66 | `apps/web/src/components/inbox/InboxTasks.tsx` | `apps/web/src/components/ConnectionStatusDot.tsx` |
| 8% | 2 | 53 | `apps/web/src/lib/draftFallbackModelSelection.test.ts` | `apps/desktop/src/preview/PickedElementPayload.test.ts` |
| 8% | 20 | 39 | `apps/web/src/components/chat/ComposerPendingUserInputPanel.tsx` | `apps/web/src/components/chat/ComposerPendingUserInputPanel.tsx` |
| 8% | 3 | 40 | `apps/server/src/provider/claudeAuthStatusLock.test.ts` | `apps/mobile/src/features/showcase/showcaseRetry.ts` |
| 8% | 9 | 70 | `packages/contracts/src/projectImport.ts` | `apps/server/src/persistence/Services/ProjectionThreadMessages.ts` |
| 8% | 10 | 76 | `packages/profile-ui/src/sections.tsx` | `apps/web/src/components/settings/SettingsScopeSentence.tsx` |
| 8% | 6 | 20 | `apps/server/src/persistence/Migrations/047_AutomationCompletionPolicyVersion.ts` | `apps/server/src/persistence/Migrations/025_CleanupInvalidProjectionPendingApprovals.test.ts` |
| 8% | 7 | 25 | `apps/web/src/components/chat/ReviewChangesButton.tsx` | `apps/web/src/browser/BrowserViewportResizeHandles.tsx` |
| 8% | 8 | 56 | `apps/server/src/persistence/Migrations/091_AutomationFailureTolerance.test.ts` | `apps/server/src/persistence/Migrations/050_ProjectionThreadPullRequests.test.ts` |
| 8% | 3 | 54 | `apps/web/src/components/chat/TimelineWorkEntryRow.icon.test.ts` | `apps/desktop/src/preview/PickedElementPayload.test.ts` |
| 8% | 4 | 20 | `apps/web/src/components/terminal/TerminalIdentityIcon.tsx` | `apps/web/src/components/ThreadCommandSubtitle.tsx` |
| 8% | 16 | 252 | `apps/web/src/components/settings/views/LayoutSettingsView.tsx` | `apps/web/src/components/settings/SourceControlWritingSettings.tsx` |
| 8% | 35 | 462 | `apps/server/src/auth/Layers/ServerAuth.test.ts` | `apps/server/src/auth/EnvironmentAuth.test.ts` |
| 8% | 6 | 103 | `apps/analytics/src/views/Releases.tsx` | `apps/web/src/components/files/DelimitedTablePreview.tsx` |
| 8% | 7 | 79 | `apps/web/src/components/ProviderIcon.tsx` | `apps/web/src/components/pullRequest/PullRequestListRow.tsx` |
| 8% | 7 | 86 | `apps/web/src/hooks/useProviderStatusRefresh.test.ts` | `apps/web/src/components/permissions/usePermissionStatus.test.ts` |
| 8% | 4 | 32 | `apps/marketing/src/components/SponsorRow.tsx` | `apps/web/src/components/pullRequest/PullRequestListRow.tsx` |
| 8% | 2 | 23 | `apps/server/src/orchestration/sidechatLifecycle.ts` | `packages/shared/src/dateTime.ts` |
| 8% | 10 | 44 | `apps/web/src/components/chat/TaskProgressSteps.tsx` | `apps/web/src/components/chat/ComposerTasksBadge.tsx` |
| 8% | 9 | 60 | `apps/web/src/components/pullRequest/pullRequestStack.logic.test.ts` | `apps/mobile/src/state/use-thread-pr.test.ts` |
| 8% | 2 | 23 | `apps/web/src/hooks/useHandleNewChat.ts` | `apps/web/src/hooks/useActiveProjectTarget.ts` |
| 8% | 3 | 33 | `apps/web/src/routes/-threadDetailOwnership.test.ts` | `packages/client-runtime/src/textPaste.test.ts` |
| 8% | 7 | 153 | `packages/shared/src/browserSession.test.ts` | `apps/web/src/lib/openPullRequestLink.test.ts` |
| 8% | 23 | 130 | `packages/contracts/src/githubInbox.ts` | `packages/contracts/src/pullRequest.ts` |
| 8% | 7 | 84 | `apps/web/src/components/settings/KeyboardShortcutsSettingsPanel.browser.tsx` | `apps/web/src/lib/projectScriptKeybindings.test.ts` |
| 8% | 5 | 37 | `apps/web/src/components/chat/useThreadBackgroundTasks.ts` | `apps/web/src/components/settings/useScopedSettings.ts` |
| 8% | 107 | 980 | `apps/web/src/components/ComposerPromptEditor.tsx` | `apps/web/src/components/ComposerPromptEditorTiptap.tsx` |
| 8% | 33 | 129 | `apps/server/src/auth/Layers/AuthControlPlane.ts` | `apps/server/src/auth/EnvironmentAuth.ts` |
| 8% | 4 | 16 | `apps/web/src/components/ThreadStatusPillChip.tsx` | `apps/web/src/components/chat/ComposerPendingApprovalPanel.tsx` |
| 8% | 3 | 42 | `apps/web/src/voidSpaceStore.test.ts` | `apps/web/src/lib/snapShotSetupResume.test.ts` |
| 8% | 5 | 91 | `apps/web/src/hooks/useComputerEventBridge.ts` | `apps/mobile/src/features/review/useReviewDiffPrewarming.ts` |
| 8% | 24 | 281 | `apps/desktop/scripts/electron-launcher.mjs` | `apps/desktop/scripts/electron-launcher.mjs` |
| 8% | 43 | 975 | `apps/server/src/provider/acp/CursorAcpSupport.ts` | `apps/server/src/provider/acp/CursorAcpSupport.ts` |
| 8% | 2 | 17 | `apps/desktop/src/browserAnnotations/guestIdentity.ts` | `apps/web/src/lib/utils.ts` |
| 8% | 2 | 63 | `apps/server/native/device-helper/build.sh` | `.agents/skills/test-t3-mobile/scripts/pair-client.sh` |
| 8% | 6 | 30 | `apps/web/src/components/SnoozeUntilDialog.tsx` | `apps/web/src/components/pullRequest/PullRequestCandidatePicker.tsx` |
| 8% | 5 | 36 | `apps/web/src/components/chat/useTimelineRowHeightHints.browser.tsx` | `apps/web/src/components/chat/timelineMinimapItems.test.ts` |
| 8% | 52 | 462 | `apps/web/src/components/BranchToolbar.tsx` | `apps/web/src/components/BranchToolbar.tsx` |
| 8% | 58 | 346 | `apps/web/src/components/terminal/TerminalViewport.tsx` | `apps/web/src/components/ThreadTerminalDrawer.tsx` |
| 8% | 3 | 20 | `apps/server/src/imageMime.test.ts` | `apps/server/src/imageMime.test.ts` |
| 8% | 133 | 1626 | `apps/web/src/components/ChatView.logic.ts` | `apps/web/src/components/ChatView.logic.ts` |
| 8% | 19 | 147 | `apps/web/src/components/ProjectMenuPicker.tsx` | `apps/web/src/components/files/FileBreadcrumbs.tsx` |
| 8% | 4 | 26 | `apps/web/src/components/NewThreadDefaultsSyncCoordinator.tsx` | `apps/mobile/src/state/composer-attachment-uploads.ts` |
| 8% | 28 | 164 | `apps/server/src/orchestration/decider.standaloneSidechat.test.ts` | `apps/server/src/orchestration/decider.projectScripts.test.ts` |
| 8% | 9 | 41 | `apps/analytics/src/views/Login.tsx` | `apps/web/src/components/desktop/SshPasswordPromptDialog.tsx` |
| 8% | 8 | 58 | `apps/marketing/src/components/FAQ.tsx` | `apps/web/src/browser/BrowserViewportResizeHandles.tsx` |
| 8% | 13 | 264 | `apps/server/src/persistence/Migrations/121_ClearAutomationDefinitionProviderOptions.test.ts` | `apps/server/src/persistence/Migrations/050_ProjectionThreadPullRequests.test.ts` |
| 8% | 8 | 89 | `apps/web/src/components/EditorDirtyRouteGuard.browser.tsx` | `apps/web/src/components/settings/IntegrationsSettings.test.tsx` |
| 8% | 15 | 274 | `apps/server/src/provider/acp/OmpAcpSupport.ts` | `apps/server/src/provider/acp/CursorAcpSupport.ts` |
| 8% | 23 | 105 | `apps/web/src/components/GitDialogChrome.tsx` | `apps/web/src/components/PullRequestThreadDialog.tsx` |
| 8% | 1 | 9 | `apps/server/src/agentGateway/schemaTestUtils.ts` | `apps/web/src/test/reactElementTree.ts` |
| 8% | 2 | 6 | `apps/server/src/auth/Services/ServerAuthPolicy.ts` | `apps/server/src/auth/EnvironmentAuthPolicy.ts` |
| 8% | 4 | 68 | `apps/server/src/migrationBundleIdentity.ts` | `apps/mobile/src/features/files/preload-workspace-file.ts` |
| 8% | 1 | 6 | `apps/server/src/provider/openCodeServerPassword.ts` | `apps/web/src/components/pullRequest/pullRequestEditing.logic.ts` |
| 8% | 1 | 17 | `apps/web/src/editorRailStore.ts` | `apps/mobile/src/components/NativePresentation.ios.tsx` |
| 8% | 98 | 685 | `apps/web/src/composerDraftStore.persistence.test.ts` | `apps/web/src/composerDraftStore.test.ts` |
| 8% | 36 | 437 | `apps/server/src/git/testing/fakeGitHubCli.ts` | `apps/server/src/sourceControl/GitHubCli.ts` |
| 8% | 9 | 87 | `apps/server/src/os-jank.test.ts` | `apps/desktop/src/shell/DesktopShellEnvironment.test.ts` |
| 8% | 17 | 113 | `apps/server/src/persistence/Migrations/035_NormalizeLegacyModelSelectionOptions.ts` | `apps/server/src/persistence/Migrations/026_CanonicalizeModelSelectionOptions.test.ts` |
| 8% | 5 | 89 | `apps/web/src/components/settings/ConnectionsAddDeviceHero.tsx` | `apps/web/src/components/pullRequest/PullRequestListRow.tsx` |
| 8% | 20 | 108 | `apps/web/src/components/FeedbackDialog.tsx` | `apps/web/src/components/PullRequestThreadDialog.tsx` |
| 8% | 25 | 128 | `apps/web/src/components/chat/ProviderInstancePicker.tsx` | `apps/web/src/components/files/FileBreadcrumbs.tsx` |
| 8% | 28 | 408 | `apps/web/src/components/settings/ModelsSettingsPanel.tsx` | `apps/web/src/components/settings/SourceControlWritingSettings.tsx` |
| 8% | 7 | 134 | `packages/shared/src/appSnapShortcut.ts` | `apps/desktop/src/preview/RecordingInput.ts` |
| 8% | 24 | 352 | `apps/web/src/lib/diffRendering.test.ts` | `apps/web/src/lib/diffRendering.test.ts` |
| 8% | 8 | 68 | `apps/server/src/persistence/Migrations/093_BackfillMaxIterationsDisabledReason.test.ts` | `apps/server/src/persistence/Migrations/050_ProjectionThreadPullRequests.test.ts` |
| 8% | 10 | 42 | `apps/server/src/persistence/Migrations/059_ProviderSessionLifecycleGeneration.test.ts` | `apps/server/src/persistence/RepositoryErrorCorrelation.test.ts` |
| 8% | 7 | 114 | `apps/web/src/components/inbox/InboxView.browser.tsx` | `apps/web/src/lib/assistantCitationNavigation.test.ts` |
| 8% | 7 | 49 | `apps/web/src/components/chat/ComposerTipRow.tsx` | `apps/web/src/components/files/fileSurfaceChrome.tsx` |
| 8% | 7 | 90 | `apps/web/src/components/chat/timelineRowHeight.logic.test.ts` | `apps/web/src/components/chat/timelineMinimapItems.test.ts` |
| 8% | 6 | 114 | `apps/web/src/hooks/useBrowserPanelDesktopBridge.test.ts` | `apps/web/src/test/reactHookHarness.ts` |
| 8% | 6 | 34 | `apps/web/src/hooks/useRemoteHostState.ts` | `apps/web/src/components/device/DeviceAndroidFoldControls.tsx` |
| 8% | 3 | 22 | `apps/web/src/routes/-desktopProjectRecoveryAttempt.ts` | `apps/web/src/browser/desktopTabLifetime.ts` |
| 8% | 3 | 51 | `apps/web/src/lib/disclosureMotion.ts` | `apps/web/src/components/ui/scroll-area.tsx` |
| 8% | 15 | 39 | `packages/shared/src/computerKeyNames.ts` | `apps/web/src/terminal/ghostty/keyCodes.ts` |
| 8% | 29 | 469 | `apps/web/src/components/kanban/KanbanNewTaskDialog.tsx` | `apps/web/src/components/PullRequestThreadDialog.tsx` |
| 8% | 4 | 33 | `apps/web/src/hooks/useComputerPreviewBudgetPx.ts` | `apps/web/src/browser/BrowserSurfaceSlot.tsx` |
| 8% | 2 | 25 | `apps/web/src/lib/desktopZoom.test.ts` | `apps/mobile/src/features/showcase/showcaseRenderSignal.ts` |
| 8% | 25 | 470 | `apps/web/src/components/settings/views/AppearanceSettingsView.tsx` | `apps/web/src/components/settings/SourceControlWritingSettings.tsx` |
| 8% | 51 | 225 | `apps/server/src/provider/acp/AcpAdapterSessionSupport.ts` | `apps/server/src/provider/Layers/CursorAdapter.ts` |
| 8% | 3 | 81 | `apps/web/src/components/DesktopWindowControls.browser.tsx` | `apps/web/src/environments/primary/httpLayer.test.ts` |
| 8% | 3 | 24 | `apps/web/src/components/chat/ComposerComputerControlEffortHint.tsx` | `apps/mobile/src/features/threads/ComposerFeedback.tsx` |
| 8% | 6 | 141 | `apps/web/src/components/computer/useComputerImageStream.ts` | `apps/mobile/src/features/review/useNativeReviewDiffHighlighting.ts` |
| 8% | 3 | 50 | `apps/web/src/hooks/useAppTypography.ts` | `apps/mobile/src/features/cloud/connectOnboardingNavigation.ts` |
| 8% | 2 | 18 | `apps/web/src/lib/claudeArtifactCommands.test.ts` | `apps/web/src/cloud/connectCliAuth.test.ts` |
| 8% | 15 | 134 | `apps/web/src/components/chat/ToolCallGroupSummaryRow.tsx` | `apps/web/src/components/AgentsPanel.tsx` |
| 8% | 2 | 20 | `apps/web/src/components/chat/userMessageTerminalContexts.test.ts` | `apps/desktop/src/linuxSecretStorage.test.ts` |
| 8% | 4 | 51 | `apps/web/src/lib/nativeMenuIcons.ts` | `apps/web/src/components/media/mediaContent.ts` |
| 8% | 5 | 59 | `apps/server/src/git/textGenerationSelection.ts` | `apps/mobile/src/lib/projectThreadStartTurn.ts` |
| 8% | 11 | 184 | `apps/server/src/provider/acp/AntigravityAcpSupport.ts` | `apps/server/src/provider/acp/CursorAcpSupport.ts` |
| 8% | 11 | 56 | `apps/web/src/components/chat/ModelCatalogRefresh.tsx` | `apps/web/src/components/pullRequest/LinkPullRequestDialog.tsx` |
| 8% | 12 | 58 | `apps/server/src/workspace/Services/WorkspaceEntries.ts` | `apps/server/src/workspace/WorkspaceEntries.ts` |
| 8% | 222 | 4181 | `apps/server/src/provider/Layers/ProviderService.ts` | `apps/server/src/provider/Layers/ProviderService.ts` |
| 8% | 3 | 28 | `apps/marketing/src/components/DownloadButton.tsx` | `apps/web/src/components/ui/group.tsx` |
| 8% | 3 | 37 | `apps/profiles/src/lib/publicProfile.test.ts` | `apps/web/src/lib/previewAnnotation.test.ts` |
| 8% | 3 | 49 | `apps/server/src/computer/computerShield.test.ts` | `apps/web/src/cloud/publicConfig.test.ts` |
| 8% | 23 | 146 | `apps/web/src/components/AnnouncementSheet.tsx` | `apps/web/src/components/PullRequestThreadDialog.tsx` |
| 8% | 3 | 52 | `apps/web/src/lib/codeFence.ts` | `packages/shared/src/desktopAppControl.ts` |
| 8% | 2 | 11 | `apps/web/src/lib/hosts/controllerRecovery.ts` | `apps/mobile/src/features/devices/device-stream-document.ts` |
| 8% | 1 | 13 | `apps/web/src/lib/tabStrip.ts` | `apps/web/src/clientPersistenceStorage.ts` |
| 8% | 7 | 45 | `packages/contracts/src/projectAgent.test.ts` | `packages/contracts/src/keybindings.test.ts` |
| 8% | 6 | 114 | `apps/server/src/relaySocket.ts` | `apps/desktop/src/snapShot/GlobalShiftShortcutProcess.ts` |
| 8% | 4 | 41 | `packages/shared/src/cli.test.ts` | `apps/server/src/cloud/publicConfig.test.ts` |
| 8% | 4 | 31 | `apps/web/vitest.browser.config.ts` | `apps/server/vite.config.ts` |
| 8% | 11 | 69 | `packages/contracts/src/device.test.ts` | `packages/contracts/src/terminal.test.ts` |
| 8% | 8 | 102 | `apps/web/src/components/tasks/TaskPropertyMenus.tsx` | `apps/web/src/components/settings/SettingsScopeSentence.tsx` |
| 8% | 3 | 11 | `apps/server/src/shell-command-detection.ts` | `apps/web/src/lib/chatThreadActions.ts` |
| 8% | 37 | 974 | `scripts/release-smoke.ts` | `scripts/release-smoke.ts` |
| 8% | 4 | 75 | `apps/server/src/computer/waitForWindow.ts` | `apps/mobile/src/features/showcase/showcaseRetry.ts` |
| 8% | 4 | 33 | `apps/server/src/eventLoopDeadline.test.ts` | `apps/web/src/rpc/requestLatencyState.test.ts` |
| 8% | 2 | 20 | `apps/web/src/i18n/Translation.tsx` | `apps/mobile/src/components/MediaActionsMenu.tsx` |
| 8% | 2 | 34 | `packages/contracts/src/pullRequestAutoFix.ts` | `apps/server/src/persistence/Services/ProjectionThreadMessages.ts` |
| 8% | 6 | 67 | `apps/desktop/src/windowState.test.ts` | `scripts/lib/public-config.test.ts` |
| 8% | 9 | 43 | `apps/web/src/components/chat/ComposerColumnFrame.tsx` | `apps/web/src/components/sidebar/SidebarThreadHeader.tsx` |
| 8% | 5 | 70 | `apps/web/src/lib/terminalCloseConfirmation.test.ts` | `apps/mobile/src/features/settings/environment-maintenance.test.ts` |
| 8% | 5 | 21 | `apps/web/src/components/chat/AmbientRailSlot.tsx` | `apps/mobile/src/features/home/WorkspaceConnectionTitle.tsx` |
| 8% | 5 | 13 | `scripts/lib/release-workspace-manifests.ts` | `scripts/update-release-package-versions.ts` |
| 8% | 37 | 398 | `apps/web/src/components/githubInbox/GitHubInboxFilterBar.tsx` | `apps/web/src/components/preview/PreviewMoreMenu.tsx` |
| 8% | 8 | 101 | `packages/shared/src/KeyedSingleFlightCache.test.ts` | `apps/server/src/pullRequest/PullRequestReadCache.test.ts` |
| 8% | 26 | 236 | `apps/web/src/components/chat/group/GroupMemorySection.tsx` | `apps/web/src/components/PullRequestThreadDialog.tsx` |
| 8% | 18 | 100 | `apps/web/src/components/hosts/HostConnectionControl.tsx` | `apps/web/src/components/pullRequest/PullRequestStackMenu.tsx` |
| 8% | 6 | 35 | `apps/web/src/hooks/useDesktopCustomTitleBar.ts` | `apps/mobile/src/features/diagnostics/SettingsDiagnosticsRouteScreen.tsx` |
| 8% | 3 | 16 | `apps/web/src/hooks/usePreloadRouteChunks.ts` | `apps/mobile/src/features/keyboard/hardwareKeyboardCommands.ts` |
| 7% | 18 | 233 | `apps/server/src/provider/acp/GrokAcpSupport.test.ts` | `apps/server/src/provider/acp/GrokAcpSupport.test.ts` |
| 7% | 7 | 65 | `apps/web/src/lib/temporaryThread.test.ts` | `apps/server/src/git/linkCreatedPullRequest.test.ts` |
| 7% | 11 | 48 | `apps/web/src/components/chat/PickerTriggerButton.tsx` | `apps/web/src/components/chat/ComposerBanner.tsx` |
| 7% | 107 | 873 | `apps/web/src/composerDraftStore.attachments.test.ts` | `apps/web/src/composerDraftStore.test.ts` |
| 7% | 10 | 279 | `apps/server/src/persistence/Layers/ProjectionPendingInteractions.test.ts` | `apps/server/src/persistence/Layers/ProjectionThreadActivities.test.ts` |
| 7% | 8 | 75 | `apps/desktop/src/browserAutomation/betterwrightInput.test.ts` | `apps/desktop/src/preview/PreviewKeyboard.test.ts` |
| 7% | 21 | 233 | `apps/web/src/components/chat/OpenInPicker.tsx` | `apps/web/src/components/device/DeviceControlsRail.tsx` |
| 7% | 2 | 30 | `apps/desktop/src/browserAutomation/betterwrightFocus.ts` | `apps/mobile/src/features/home/thread-dismissal.ts` |
| 7% | 4 | 38 | `apps/server/src/agentGateway/hubWorkRouting.ts` | `apps/mobile/src/lib/projectThreadStartTurn.ts` |
| 7% | 7 | 100 | `apps/server/src/nodeHttpServer.test.ts` | `apps/server/src/httpResponseErrorGuard.test.ts` |
| 7% | 1 | 5 | `apps/server/src/provider/claudeCliVersion.ts` | `apps/web/src/commandPaletteBus.ts` |
| 7% | 1 | 19 | `apps/server/src/provider/tokenUsage.ts` | `apps/mobile/src/features/terminal/terminalPreferences.ts` |
| 7% | 11 | 55 | `apps/web/src/components/SidebarChildThreadsChip.tsx` | `apps/web/src/components/settings/ThemePreviewCircles.tsx` |
| 7% | 3 | 55 | `apps/web/src/components/chat/ComposerImageAttachmentChip.test.tsx` | `apps/web/src/components/chat/ComposerPendingApprovalPanel.test.tsx` |
| 7% | 2 | 16 | `apps/web/src/lib/contextMenuGroup.ts` | `apps/web/src/components/pullRequest/pullRequestLinkContextMenu.ts` |
| 7% | 2 | 15 | `apps/web/src/pendingTurnDispatch.ts` | `.github/scripts/check-nightly-release.cjs` |
| 7% | 1 | 15 | `packages/shared/src/githubAvatar.test.ts` | `apps/web/src/pullRequestReference.test.ts` |
| 7% | 12 | 81 | `apps/web/src/components/settings/accounts/DefaultNewChatAccountRow.tsx` | `apps/web/src/components/settings/SourceControlWritingSettings.tsx` |
| 7% | 7 | 115 | `apps/server/src/orchestration/threadGitMetadata.test.ts` | `apps/mobile/src/state/use-thread-pr.test.ts` |
| 7% | 15 | 19 | `apps/web/src/components/pullRequest/PullRequestMetaLine.tsx` | `apps/web/src/components/pullRequest/pullRequestPresentation.tsx` |
| 7% | 6 | 109 | `apps/web/src/featureFlags.ts` | `apps/web/src/confirmDialog.ts` |
| 7% | 9 | 62 | `apps/server/src/provider/providerCliOutput.ts` | `apps/server/src/provider/providerSnapshot.ts` |
| 7% | 5 | 58 | `apps/web/src/components/chat/userMessageOverflowObserver.test.ts` | `apps/web/src/components/chat/ComposerBannerStack.test.tsx` |
| 7% | 5 | 89 | `packages/contracts/src/server.test.ts` | `packages/contracts/src/environment.test.ts` |
| 7% | 22 | 161 | `apps/web/src/components/chat/AsyncUserInputCard.tsx` | `apps/web/src/components/chat/ComposerPendingUserInputPanel.tsx` |
| 7% | 8 | 164 | `apps/web/src/components/settings/ProviderUsageSettingsPanel.test.tsx` | `apps/mobile/src/features/settings/settings-scoped-server.test.ts` |
| 7% | 9 | 97 | `apps/server/src/persistence/Layers/ProjectPullRequestPins.ts` | `apps/server/src/persistence/Layers/ProjectionThreadActivities.ts` |
| 7% | 2 | 54 | `packages/shared/src/errorMessages.test.ts` | `apps/desktop/src/linuxSecretStorage.test.ts` |
| 7% | 5 | 61 | `packages/shared/src/windowsCertificate.ts` | `packages/shared/src/cliArgs.ts` |
| 7% | 24 | 336 | `apps/web/src/components/chat/FloatingBrowserPanel.tsx` | `apps/web/src/components/preview/ThreadPreviewMiniPlayer.tsx` |
| 7% | 2 | 14 | `apps/marketing/src/lib/githubStars.ts` | `apps/mobile/src/features/devices/device-stream-document.ts` |
| 7% | 2 | 14 | `apps/profiles/src/lib/imageAvatar.ts` | `apps/mobile/src/features/devices/device-stream-document.ts` |
| 7% | 3 | 20 | `apps/server/src/frameSocketByteStream.ts` | `packages/client-runtime/src/work-log/toolPresentation.ts` |
| 7% | 2 | 28 | `apps/web/src/components/useRouteSpaceSync.ts` | `apps/mobile/src/features/cloud/connectOnboardingNavigation.ts` |
| 7% | 4 | 50 | `apps/web/src/lib/composerImagePreparation.browser.ts` | `apps/web/src/assets/projectFaviconCache.ts` |
| 7% | 8 | 26 | `apps/web/src/components/SidebarLeadingIcon.tsx` | `apps/web/src/components/ui/autocomplete.tsx` |
| 7% | 5 | 100 | `apps/web/src/hooks/useOnboarding.browser.tsx` | `apps/web/src/components/files/AttachmentFilePreview.test.tsx` |
| 7% | 10 | 143 | `apps/web/src/components/settings/ThemeModePicker.tsx` | `apps/web/src/components/settings/ThemePreviewCircles.tsx` |
| 7% | 4 | 47 | `apps/desktop/src/httpRuntime.test.ts` | `apps/server/src/compileCache.test.ts` |
| 7% | 3 | 10 | `apps/server/src/environment/Layers/ServerEnvironmentLabel.ts` | `apps/desktop/src/app/DesktopEarlyElectronStartup.ts` |
| 7% | 3 | 13 | `apps/server/src/provider/claudeAuthStatusLock.ts` | `apps/mobile/src/lib/composerAttachmentFiles.ts` |
| 7% | 10 | 92 | `apps/web/src/components/device/DeviceScreenStates.tsx` | `apps/web/src/components/AgentsPanel.tsx` |
| 7% | 4 | 78 | `packages/shared/src/cloudflared.ts` | `packages/client-runtime/src/filePreview.ts` |
| 7% | 11 | 269 | `apps/desktop/src/menuShortcuts.test.ts` | `apps/desktop/src/snapShot/snapShot.test.ts` |
| 7% | 6 | 80 | `scripts/computer-use-fixtures/packaged-fixture.ts` | `apps/desktop/src/snapShot/NativeCaptureFeedback.ts` |
| 7% | 7 | 88 | `apps/desktop/src/confirmDialog.test.ts` | `apps/desktop/src/electron/ElectronDialog.test.ts` |
| 7% | 24 | 94 | `apps/server/src/auth/Services/AuthControlPlane.ts` | `apps/server/src/auth/EnvironmentAuth.ts` |
| 7% | 8 | 134 | `apps/desktop/src/browserAutomation/betterwrightNetworkGuard.test.ts` | `apps/desktop/src/snapShot/NativeCaptureFeedback.test.ts` |
| 7% | 1 | 6 | `apps/marketing/src/components/HandoffChatMock.tsx` | `apps/web/src/commandPaletteBus.ts` |
| 7% | 1 | 6 | `apps/marketing/src/components/OneClickPrMock.tsx` | `apps/web/src/commandPaletteBus.ts` |
| 7% | 1 | 6 | `apps/marketing/src/components/ParallelChatMock.tsx` | `apps/web/src/commandPaletteBus.ts` |
| 7% | 1 | 6 | `apps/marketing/src/components/TerminalTabsMock.tsx` | `apps/web/src/commandPaletteBus.ts` |
| 7% | 1 | 6 | `apps/marketing/src/components/WorktreeMock.tsx` | `apps/web/src/commandPaletteBus.ts` |
| 7% | 1 | 3 | `apps/server/src/agentGateway/approvalGate.ts` | `apps/mobile/src/features/usage/usageProviders.ts` |
| 7% | 4 | 67 | `apps/server/src/remoteFeaturePolicy.test.ts` | `packages/shared/src/changeRequestUrl.test.ts` |
| 7% | 2 | 33 | `apps/web/src/components/terminal/terminalEventDispatcher.ts` | `apps/web/src/hooks/threadUndo.ts` |
| 7% | 3 | 81 | `apps/web/src/lib/editorGutterDiff.test.ts` | `apps/web/src/reviewCommentContext.test.ts` |
| 7% | 14 | 327 | `apps/web/src/components/WorkspaceSearchPalette.tsx` | `apps/web/src/components/CommandPaletteResults.tsx` |
| 7% | 19 | 122 | `packages/shared/src/serverSettings.ts` | `packages/shared/src/serverSettings.ts` |
| 7% | 7 | 65 | `apps/web/src/components/ComposerPredictionGhost.tsx` | `apps/web/src/components/ProjectFavicon.tsx` |
| 7% | 13 | 136 | `apps/web/src/components/chat/ComposerEnvironmentPicker.tsx` | `apps/web/src/components/pullRequest/PullRequestCandidatePicker.tsx` |
| 7% | 6 | 30 | `apps/web/src/components/chat/ComposerStackedPanel.tsx` | `apps/web/src/components/ui/autocomplete.tsx` |
| 7% | 5 | 62 | `apps/web/src/components/chat/FileEntryIcon.tsx` | `apps/web/src/components/chat/PierreEntryIcon.tsx` |
| 7% | 42 | 252 | `apps/server/src/orchestration/Layers/HubOutputReactor.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.ts` |
| 7% | 4 | 25 | `apps/web/src/components/sidebarGlyphs.tsx` | `apps/web/src/components/chat/ComposerControl.tsx` |
| 7% | 2 | 41 | `apps/server/src/persistence/Services/ProjectPullRequestPins.ts` | `apps/mobile/src/features/threads/projectThreadCreationValidation.ts` |
| 7% | 37 | 330 | `apps/web/src/components/chat/composerTraits.ts` | `apps/web/src/components/chat/TraitsPicker.tsx` |
| 7% | 5 | 56 | `apps/web/src/components/chat/project/coordinatorSuggestions.logic.test.ts` | `packages/client-runtime/src/state/sharedSettings.test.ts` |
| 7% | 26 | 357 | `apps/web/src/components/chat/project/groupOverview.logic.test.ts` | `apps/server/src/orchestration/ThreadSettlementPolicy.test.ts` |
| 7% | 10 | 221 | `apps/web/src/components/DesktopUpdateRailButton.browser.tsx` | `apps/web/src/components/sidebar/SidebarUpdatePill.test.tsx` |
| 7% | 4 | 47 | `apps/server/src/projectAgent/materializer.ts` | `apps/server/src/orchestration/workflowScriptQuery.ts` |
| 7% | 2 | 19 | `apps/web/src/components/terminal/terminalFontSettle.ts` | `apps/web/src/components/pullRequest/pullRequestLinkContextMenu.ts` |
| 7% | 2 | 31 | `scripts/lib/settle-concurrently.test.ts` | `apps/server/src/persistence/Migrations/051_ProjectionThreadMessageContext.test.ts` |
| 7% | 8 | 101 | `apps/web/src/components/tasks/TasksView.tsx` | `apps/web/src/components/sidebar/SidebarProviderUpdatePill.tsx` |
| 7% | 7 | 96 | `apps/web/src/lib/composerImagePreparation.test.ts` | `apps/web/src/components/ThreadTerminalDrawer.test.ts` |
| 7% | 3 | 26 | `apps/marketing/src/components/docs/pagination.tsx` | `apps/web/src/components/clerk/ClerkUserProfilePage.tsx` |
| 7% | 7 | 148 | `apps/web/src/routes/-chatThreadRoute.logic.ts` | `packages/client-runtime/src/threadPullRequestCompatibility.ts` |
| 7% | 17 | 95 | `apps/web/src/components/pullRequest/PullRequestMarkdown.tsx` | `apps/web/src/components/pullRequest/PullRequestTimelineTab.tsx` |
| 7% | 14 | 366 | `apps/api/src/db/schema.ts` | `infra/relay/src/persistence/schema.ts` |
| 7% | 8 | 103 | `apps/web/src/projectRunTargets.test.ts` | `apps/web/src/projectScripts.test.ts` |
| 7% | 11 | 234 | `apps/web/src/onboarding/stage/IphoneMock.tsx` | `apps/web/src/components/pullRequest/PullRequestGhosts.tsx` |
| 7% | 1 | 24 | `apps/marketing/playwright.config.ts` | `apps/marketing/astro.config.mjs` |
| 7% | 3 | 32 | `apps/marketing/src/components/ChangelogNav.tsx` | `apps/web/src/components/preview/AgentBrowserCursor.tsx` |
| 7% | 9 | 138 | `apps/marketing/src/components/Features.tsx` | `apps/web/src/components/settings/providerDriverMeta.ts` |
| 7% | 1 | 9 | `apps/server/src/agentGateway/bearerToken.test.ts` | `apps/desktop/src/app/DesktopAppErrors.test.ts` |
| 7% | 4 | 41 | `apps/server/src/provider/acp/AcpExtensions.ts` | `apps/server/src/provider/acp/CursorAcpExtension.ts` |
| 7% | 5 | 36 | `apps/web/src/components/chat/ComposerBranchMismatchBanner.tsx` | `apps/web/src/components/pullRequest/PullRequestCopyableCode.tsx` |
| 7% | 1 | 10 | `apps/web/src/components/chat/project/projectAgentOverview.logic.ts` | `apps/web/src/components/pullRequest/pullRequestEditing.logic.ts` |
| 7% | 1 | 7 | `apps/web/src/components/settings/views/AboutSettingsView.tsx` | `apps/web/src/commandPaletteBus.ts` |
| 7% | 1 | 7 | `apps/web/src/components/settings/views/PrivacySettingsView.tsx` | `apps/web/src/commandPaletteBus.ts` |
| 7% | 2 | 40 | `apps/web/src/components/terminal/terminalPerformance.ts` | `apps/web/src/vite-env.d.ts` |
| 7% | 2 | 18 | `apps/web/src/composerTriggerInsertion.ts` | `packages/shared/src/composerContextClipboard.ts` |
| 7% | 1 | 5 | `apps/web/src/lib/dockTerminalScope.ts` | `apps/server/src/orchestration/threadDetailCursor.ts` |
| 7% | 7 | 80 | `apps/web/src/lib/openUsageRateLimits.test.ts` | `apps/server/src/provider/Layers/claudeUsageLimits.test.ts` |
| 7% | 13 | 300 | `packages/shared/src/providerInstances.test.ts` | `apps/web/src/onboarding/providerReadiness.logic.test.ts` |
| 7% | 163 | 1365 | `apps/web/src/components/ChatMarkdown.tsx` | `apps/web/src/components/ChatMarkdown.tsx` |
| 7% | 170 | 867 | `apps/web/src/composerDraftDomain.ts` | `apps/web/src/composerDraftStore.ts` |
| 7% | 6 | 63 | `apps/web/src/components/chat/SubagentAvatar.tsx` | `apps/web/src/components/pullRequest/PullRequestListRow.tsx` |
| 7% | 6 | 110 | `apps/web/src/editorMetadata.ts` | `apps/web/src/components/settings/providerDriverMeta.ts` |
| 7% | 2 | 63 | `apps/server/src/provider/Layers/computerHostContext.test.ts` | `apps/desktop/src/linuxSecretStorage.test.ts` |
| 7% | 16 | 221 | `apps/server/src/provider/acp/DroidAcpSupport.test.ts` | `apps/server/src/provider/acp/CursorAcpSupport.test.ts` |
| 7% | 10 | 34 | `apps/server/src/provider/openCodeReasoningOptions.ts` | `packages/shared/src/toolActivity.ts` |
| 7% | 9 | 72 | `apps/web/src/lib/draftThreadCreate.ts` | `apps/web/src/queuedMessageStore.ts` |
| 7% | 5 | 68 | `apps/web/src/composerDraftDomain.test.ts` | `packages/client-runtime/src/state/threadSnoozed.test.ts` |
| 7% | 3 | 32 | `apps/web/src/components/codeEditor/editorRecovery.test.ts` | `apps/web/src/bootstrap.test.ts` |
| 7% | 24 | 268 | `apps/web/src/featureTour/AppTour.tsx` | `apps/web/src/components/preview/ThreadPreviewMiniPlayer.tsx` |
| 7% | 3 | 21 | `apps/web/src/lib/appSnapIconStore.test.ts` | `apps/web/src/hooks/useThreadActions.test.ts` |
| 7% | 9 | 73 | `apps/web/src/lib/threadFork.ts` | `apps/web/src/queuedMessageStore.ts` |
| 7% | 10 | 161 | `apps/server/src/mcpConnectors/httpRoute.ts` | `apps/server/src/cloud/cliAuthHtml.ts` |
| 7% | 4 | 220 | `apps/web/src/routes/-chatThreadRoute.logic.test.ts` | `apps/mobile/src/features/threads/new-task-context-presentation.test.ts` |
| 7% | 4 | 60 | `apps/web/src/featureTour/logic.ts` | `apps/web/src/browser/hostedBrowserWebviewStyle.ts` |
| 7% | 2 | 24 | `apps/web/src/hooks/useStarredModels.ts` | `apps/mobile/src/features/settings/components/AutoSettleDaysField.tsx` |
| 7% | 2 | 46 | `apps/web/src/lib/appSnapSound.ts` | `apps/mobile/src/lib/foundation-fast-refresh.ts` |
| 7% | 3 | 63 | `apps/web/src/pinning.logic.test.ts` | `apps/mobile/src/features/keyboard/commandPaletteItems.test.ts` |
| 7% | 4 | 88 | `scripts/canary.test.ts` | `apps/mobile/src/features/cloud/publicConfig.test.ts` |
| 7% | 18 | 317 | `apps/web/src/components/BrowserVault.tsx` | `apps/web/src/components/PullRequestThreadDialog.tsx` |
| 7% | 8 | 45 | `apps/web/src/components/chat/CrossTaskOriginLabel.tsx` | `apps/web/src/components/chat/ComposerPendingUserInputPanel.tsx` |
| 7% | 7 | 168 | `apps/web/src/components/Sidebar.uiState.test.ts` | `apps/web/src/lib/desktopSnapShot.test.ts` |
| 7% | 5 | 78 | `packages/shared/src/outboundHttpPolicy.test.ts` | `packages/shared/src/changeRequestUrl.test.ts` |
| 7% | 9 | 131 | `apps/web/src/routes/-threadTerminalFence.test.ts` | `packages/client-runtime/src/state/threadSnoozed.test.ts` |
| 7% | 2 | 30 | `apps/web/src/components/chat/workspaceFilePreviewBreadcrumb.test.ts` | `apps/web/src/cloud/connectCliAuth.test.ts` |
| 7% | 29 | 416 | `apps/server/src/provider/acp/AcpSdkConformance.test.ts` | `apps/server/src/provider/acp/AcpJsonRpcConnection.test.ts` |
| 7% | 23 | 134 | `apps/web/src/components/chat/ComposerEffortSliderCard.tsx` | `apps/web/src/components/chat/TraitsPicker.tsx` |
| 7% | 9 | 95 | `apps/web/src/components/chat/usePendingUserInputDrafts.browser.tsx` | `apps/server/src/orchestration/decider.userInputDismiss.test.ts` |
| 7% | 12 | 88 | `apps/web/src/components/chat/PullRequestContextCard.tsx` | `apps/web/src/components/pullRequest/PullRequestReviewAnnotation.tsx` |
| 7% | 34 | 968 | `packages/contracts/src/model.ts` | `packages/contracts/src/model.ts` |
| 7% | 29 | 284 | `apps/server/src/orchestration/decider.pullRequestLinks.test.ts` | `apps/server/src/orchestration/projector.pullRequests.test.ts` |
| 7% | 16 | 201 | `apps/server/src/usagePricing.ts` | `apps/server/src/usage/usagePricing.ts` |
| 7% | 15 | 134 | `apps/server/src/provider/Layers/OmpAdapter.test.ts` | `apps/server/src/provider/Drivers/CodexDriver.test.ts` |
| 7% | 1 | 2 | `apps/server/src/persistence/Migrations/054_ReservedDurableProviderCommandDelivery.ts` | `apps/web/src/cloud/dpop.test.ts` |
| 7% | 13 | 134 | `apps/web/src/components/PlanSidebar.tsx` | `apps/web/src/components/AgentsPanel.tsx` |
| 7% | 3 | 63 | `apps/web/src/components/chat/useFileLineCommenting.browser.tsx` | `apps/web/src/components/preview/PreviewUnreachable.tsx` |
| 7% | 2 | 34 | `apps/web/src/i18n/index.ts` | `apps/mobile/src/features/settings/appearance/useAppearanceCodeSurface.ts` |
| 7% | 4 | 31 | `apps/web/src/lib/modelFavorites.migration.test.ts` | `apps/web/src/hooks/useLocalStorage.test.ts` |
| 7% | 2 | 13 | `scripts/lib/resolve-catalog.test.ts` | `apps/web/src/components/KeybindingsUpdateToast.logic.test.ts` |
| 7% | 34 | 1424 | `apps/server/src/git/Layers/GitHubCli.test.ts` | `apps/server/src/sourceControl/GitHubCli.test.ts` |
| 7% | 13 | 248 | `apps/web/src/components/chat/composerTraits.test.ts` | `apps/web/src/components/settings/customModelEditor.logic.test.ts` |
| 7% | 24 | 393 | `apps/web/src/hooks/useHandleNewThread.ts` | `apps/web/src/hooks/useHandleNewThread.ts` |
| 7% | 14 | 139 | `apps/web/src/onboarding/steps/ReadyStep.tsx` | `apps/web/src/components/preview/PreviewChromeRow.tsx` |
| 7% | 4 | 83 | `apps/server/src/remotePairing/management.accountScope.test.ts` | `apps/web/src/components/files/projectFilesQueryState.test.ts` |
| 7% | 4 | 54 | `apps/web/src/lib/providerInstancePresentation.test.ts` | `packages/client-runtime/src/state/providerInstanceDisplay.test.ts` |
| 7% | 3 | 24 | `apps/server/src/git/Services/GitStatusBroadcaster.ts` | `apps/server/src/vcs/VcsDriver.ts` |
| 7% | 3 | 21 | `apps/web/src/lib/remotePairingLink.ts` | `apps/mobile/src/lib/composerAttachmentFiles.ts` |
| 7% | 11 | 110 | `packages/contracts/src/hubWork.ts` | `apps/server/src/persistence/Services/ProjectionThreadMessages.ts` |
| 7% | 5 | 76 | `apps/web/src/components/FeatureTourDialog.tsx` | `apps/web/src/browser/ElectronBrowserHost.tsx` |
| 7% | 8 | 97 | `apps/web/src/components/chat/DockExplorerPane.tsx` | `apps/mobile/src/features/cloud/CloudAuthProvider.tsx` |
| 7% | 10 | 133 | `apps/web/src/components/GitCreatePrDialog.tsx` | `apps/web/src/components/pullRequest/LinkPullRequestDialog.tsx` |
| 7% | 3 | 55 | `apps/api/cloudflare/legacy-proxy.test.ts` | `packages/shared/src/favicon.test.ts` |
| 7% | 2 | 22 | `apps/web/src/sidebarNavOrdering.ts` | `apps/server/src/provider/Drivers/ClaudeExecutable.ts` |
| 7% | 7 | 57 | `apps/web/src/components/settings/ComputerGettingStarted.tsx` | `apps/web/src/components/pullRequest/PullRequestThreadLinks.tsx` |
| 7% | 3 | 35 | `apps/web/src/components/chat/environment/EnvironmentProjectInstructionsSection.browser.tsx` | `apps/web/src/components/preview/PreviewUnreachable.tsx` |
| 7% | 3 | 20 | `apps/web/src/lib/siteFavicon.test.ts` | `apps/web/src/components/settings/settingsLayout.test.tsx` |
| 6% | 6 | 92 | `apps/server/src/cloudflare/ingress.test.ts` | `apps/server/src/httpResponseErrorGuard.test.ts` |
| 6% | 2 | 58 | `apps/server/src/orchestration/forkSourceCutoff.test.ts` | `apps/web/src/components/usage/usagePriceTable.test.ts` |
| 6% | 8 | 89 | `apps/web/src/fixedShortcuts.ts` | `apps/web/src/lib/terminalCloseShortcut.test.ts` |
| 6% | 7 | 69 | `apps/web/src/lib/providerDiscoveryInvalidation.test.ts` | `apps/mobile/src/features/settings/environment-maintenance.test.ts` |
| 6% | 35 | 532 | `apps/server/src/orchestration/usageLimitRecovery.test.ts` | `apps/server/src/orchestration/decider.settled.test.ts` |
| 6% | 7 | 47 | `apps/web/src/projectImport/ProjectImportHeroScene.tsx` | `apps/web/src/components/pullRequest/PullRequestRow.tsx` |
| 6% | 8 | 11 | `packages/contracts/src/provider.test.ts` | `packages/contracts/src/provider.test.ts` |
| 6% | 16 | 283 | `packages/shared/src/threadSummary.ts` | `packages/client-runtime/src/pendingRequests.ts` |
| 6% | 1 | 9 | `apps/marketing/src/components/WebAnalytics.tsx` | `apps/web/src/commandPaletteBus.ts` |
| 6% | 2 | 13 | `apps/marketing/src/lib/docsMarkdown.ts` | `apps/mobile/src/features/diagnostics/crash-log-model.ts` |
| 6% | 2 | 30 | `apps/server/src/orchestration/projectImportPaths.test.ts` | `apps/server/src/provider/cursorCredentialStore.test.ts` |
| 6% | 10 | 70 | `apps/web/src/components/KeepAwakeControls.tsx` | `apps/web/src/components/settings/SourceControlWritingSettings.tsx` |
| 6% | 6 | 72 | `apps/web/src/components/chat/composerComputerControlHint.test.ts` | `apps/mobile/src/features/threads/thread-settings-sheet-state.test.ts` |
| 6% | 14 | 205 | `apps/web/src/components/pullRequest/PullRequestRow.browser.tsx` | `apps/web/src/components/pullRequest/PullRequestSummaryTab.test.tsx` |
| 6% | 4 | 22 | `apps/web/src/lib/claudeCompactionRequests.ts` | `apps/web/src/diffPanelStore.ts` |
| 6% | 2 | 15 | `packages/shared/src/projectDirectoryName.ts` | `apps/server/src/git/remoteRefs.ts` |
| 6% | 28 | 169 | `apps/web/src/lib/desktopProjectRecovery.test.ts` | `packages/client-runtime/src/state/entities.test.ts` |
| 6% | 63 | 1932 | `apps/server/src/git/Layers/GitHubCli.ts` | `apps/server/src/sourceControl/GitHubCli.ts` |
| 6% | 6 | 42 | `apps/server/src/git/gitActionPullRequest.ts` | `apps/server/src/git/linkCreatedPullRequest.test.ts` |
| 6% | 8 | 551 | `apps/server/src/orchestration/spaces.test.ts` | `apps/server/src/orchestration/decider.projectThreadEnvMode.test.ts` |
| 6% | 6 | 149 | `apps/web/src/components/computer/useComputerImageStream.test.ts` | `apps/web/src/test/reactHookHarness.ts` |
| 6% | 30 | 621 | `apps/web/src/components/CreateProjectDialog.tsx` | `apps/web/src/components/projectScriptEditor.tsx` |
| 6% | 13 | 194 | `apps/web/src/components/chat/group/GroupEnvironmentSection.tsx` | `apps/web/src/components/settings/SnapShotSettings.tsx` |
| 6% | 5 | 50 | `apps/server/src/persistence/Layers/RemoteHostTrust.test.ts` | `packages/shared/src/nodeSqliteClient.test.ts` |
| 6% | 11 | 259 | `apps/web/src/components/chat/SubagentsDockPane.tsx` | `apps/web/src/components/chat/WorktreeSetupCard.tsx` |
| 6% | 7 | 43 | `apps/web/src/components/codeEditor/CodeEditBoundary.tsx` | `apps/web/src/components/media/MediaVideoPlayer.tsx` |
| 6% | 3 | 41 | `apps/web/src/appSnapShortcut.ts` | `packages/client-runtime/src/threadPullRequestCompatibility.ts` |
| 6% | 3 | 50 | `apps/web/src/hooks/useProfileThemeAccentSync.ts` | `apps/web/src/components/SlowRpcRequestToastCoordinator.tsx` |
| 6% | 4 | 63 | `apps/web/src/lib/hosts/workspaceReconnect.test.ts` | `apps/web/src/lib/snapShotSetupResume.test.ts` |
| 6% | 3 | 34 | `apps/web/src/lib/indexedDb.ts` | `apps/web/src/assets/projectFaviconCache.ts` |
| 6% | 9 | 230 | `apps/profiles/src/app/[handle]/page.tsx` | `apps/web/src/components/usage/UsageLimits.tsx` |
| 6% | 6 | 200 | `apps/web/src/lib/computerProvisioning.test.ts` | `apps/web/src/components/pullRequest/pullRequestEditing.logic.test.ts` |
| 6% | 5 | 91 | `packages/shared/src/mcpConnectors.test.ts` | `packages/shared/src/changeRequestUrl.test.ts` |
| 6% | 5 | 124 | `apps/server/src/agentGateway/computerSpaceTools.ts` | `oxlint-plugin-t3code/rules/no-manual-effect-runtime-in-tests.ts` |
| 6% | 2 | 17 | `apps/web/src/components/chat/ComposerRunningAgentsCapsule.tsx` | `apps/web/src/components/chat/ComposerStashBadge.tsx` |
| 6% | 28 | 356 | `apps/server/src/orchestration/Layers/UsageLimitRecoveryReactor.test.ts` | `apps/server/src/orchestration/ThreadPullRequestReactor.test.ts` |
| 6% | 14 | 310 | `apps/server/src/provider/acp/DevinAcpSupport.ts` | `apps/server/src/provider/acp/CursorAcpSupport.ts` |
| 6% | 16 | 338 | `apps/web/src/components/device/DeviceFrame.tsx` | `apps/web/src/components/pullRequest/pullRequestPresentation.tsx` |
| 6% | 4 | 315 | `apps/web/src/lib/computerToolPresentation.test.ts` | `apps/desktop/src/snapShot/snapShot.test.ts` |
| 6% | 4 | 82 | `apps/web/src/lib/fontFamily.ts` | `packages/shared/src/cliArgs.ts` |
| 6% | 6 | 81 | `apps/desktop/src/backendStartupReadiness.ts` | `apps/desktop/scripts/dev-electron.mjs` |
| 6% | 13 | 106 | `apps/server/src/pullRequests.logic.ts` | `apps/server/src/pullRequest/BitbucketPullRequestProvider.ts` |
| 6% | 9 | 47 | `apps/web/src/components/chat/AttachmentRemoveButton.tsx` | `apps/web/src/components/sidebar/SidebarThreadHeader.tsx` |
| 6% | 4 | 31 | `apps/web/src/hooks/useComposerThreadMentionDrop.test.ts` | `packages/client-runtime/src/state/threadSnoozed.test.ts` |
| 6% | 3 | 49 | `packages/contracts/src/githubProjectProvisioning.ts` | `packages/contracts/src/relayClient.ts` |
| 6% | 3 | 24 | `packages/shared/src/synaraHome.ts` | `apps/server/src/os-jank.ts` |
| 6% | 10 | 33 | `apps/web/src/hooks/useOpenFavoriteEditorShortcut.ts` | `apps/web/src/components/chat/OpenInPicker.tsx` |
| 6% | 13 | 205 | `apps/web/src/components/kanban/KanbanCardView.tsx` | `apps/web/src/components/chat/WorktreeSetupCard.tsx` |
| 6% | 38 | 641 | `apps/web/src/components/ThemePackEditor.tsx` | `apps/web/src/components/projectScriptEditor.tsx` |
| 6% | 6 | 101 | `apps/web/src/components/kanban/useKanbanTaskSubmit.browser.tsx` | `apps/web/src/components/settings/ProjectFaviconPickerDialog.test.tsx` |
| 6% | 4 | 80 | `packages/shared/src/claudeCache.test.ts` | `packages/client-runtime/src/textPaste.test.ts` |
| 6% | 3 | 27 | `apps/desktop/src/githubUpdateFeed.ts` | `apps/server/src/git/linkCreatedPullRequest.ts` |
| 6% | 3 | 66 | `apps/server/src/computer/modelDesktopObservation.test.ts` | `apps/mobile/src/lib/serialized-async-queue.test.ts` |
| 6% | 8 | 143 | `apps/server/src/gitHandoffOperations.ts` | `apps/server/src/persistence/Layers/OrchestrationEventStore.test.ts` |
| 6% | 3 | 36 | `apps/server/src/githubInbox/Services/GitHubInboxService.ts` | `apps/web/src/state/desktopNetworkAccess.ts` |
| 6% | 1 | 10 | `apps/web/src/components/SnoozeCountdown.tsx` | `apps/web/src/commandPaletteBus.ts` |
| 6% | 4 | 99 | `apps/web/src/components/chat/useThreadErrorToast.test.ts` | `apps/mobile/src/state/pending-thread-creation.test.ts` |
| 6% | 2 | 24 | `apps/web/src/components/profile/EditProfileDialog.browser.tsx` | `apps/web/src/components/settings/ProjectIconPickerDialog.test.tsx` |
| 6% | 7 | 35 | `apps/web/src/hooks/useDiffChangeNavigationShortcuts.ts` | `apps/web/src/routes/_chat.tsx` |
| 6% | 18 | 332 | `apps/web/src/providerUpdates.test.ts` | `apps/mobile/src/features/settings/environment-maintenance.test.ts` |
| 6% | 1 | 10 | `packages/shared/src/providerDeliveryBlock.ts` | `apps/web/src/commandPaletteBus.ts` |
| 6% | 8 | 178 | `packages/shared/src/groupThreadState.test.ts` | `apps/server/src/orchestration/ThreadSettlementPolicy.test.ts` |
| 6% | 3 | 47 | `apps/desktop/src/desktopStorageMigration.ts` | `apps/web/src/browserFaviconLogic.ts` |
| 6% | 8 | 117 | `apps/server/src/pullRequestAutoFix/pullRequestAutoFixDecision.ts` | `packages/shared/src/agentAwareness.ts` |
| 6% | 6 | 64 | `apps/web/src/components/chat/useCodeSelectionAction.ts` | `apps/web/src/components/chat/AssistantSelectionToolbar.tsx` |
| 6% | 68 | 1823 | `apps/server/src/profileStats.test.ts` | `apps/server/src/orchestration/Layers/ProjectionSnapshotQuery.test.ts` |
| 6% | 26 | 255 | `apps/web/src/hooks/useTheme.ts` | `apps/web/src/hooks/useTheme.ts` |
| 6% | 5 | 63 | `packages/shared/src/worktreeHandoff.ts` | `apps/mobile/src/features/threads/new-task-context-presentation.ts` |
| 6% | 3 | 38 | `apps/desktop/src/migrationBundleIdentity.test.ts` | `apps/web/src/cloud/connectCliAuth.test.ts` |
| 6% | 19 | 342 | `apps/web/src/keybindingEditor.ts` | `apps/web/src/components/settings/KeybindingsSettings.logic.ts` |
| 6% | 18 | 349 | `apps/web/src/lib/toolCallDetails.ts` | `packages/shared/src/toolActivity.ts` |
| 6% | 2 | 38 | `packages/shared/src/deviceHelperCache.test.ts` | `apps/web/src/cloud/connectCliAuth.test.ts` |
| 6% | 2 | 49 | `apps/web/src/projectInstructionsStore.test.ts` | `apps/web/src/components/settings/SettingsPanels.logic.test.ts` |
| 6% | 356 | 21795 | `apps/server/src/orchestration/Layers/ProviderCommandReactor.test.ts` | `apps/server/src/orchestration/Layers/ProviderCommandReactor.test.ts` |
| 6% | 6 | 93 | `apps/server/src/orchestration/decider.projectFolders.test.ts` | `apps/server/src/orchestration/decider.projectThreadEnvMode.test.ts` |
| 6% | 4 | 18 | `apps/web/src/components/ui/DisclosureRegion.tsx` | `apps/web/src/components/pullRequest/PullRequestListRow.tsx` |
| 6% | 58 | 848 | `apps/web/src/composerDraftStore.models.test.ts` | `apps/web/src/composerDraftStore.test.ts` |
| 6% | 2 | 7 | `apps/web/src/lib/pullRequestReactQuery.testUtils.ts` | `apps/mobile/src/features/terminal/terminalPaste.test.ts` |
| 6% | 4 | 112 | `packages/shared/src/projectAgent.test.ts` | `apps/mobile/src/features/settings/environment-maintenance.test.ts` |
| 6% | 9 | 69 | `apps/web/src/routes/_chat.automations.index.tsx` | `apps/web/src/browser/BrowserDeviceToolbar.tsx` |
| 6% | 8 | 56 | `apps/web/src/components/settings/BooleanSettingRow.tsx` | `apps/web/src/components/settings/SourceControlWritingSettings.tsx` |
| 6% | 3 | 37 | `apps/web/src/projectImport/useProjectImportAnnouncement.ts` | `apps/mobile/src/features/cloud/managedRelayState.ts` |
| 6% | 2 | 58 | `packages/shared/src/inboxDay.ts` | `apps/mobile/src/state/environments.ts` |
| 6% | 4 | 25 | `apps/server/src/persistence/Errors.test.ts` | `apps/server/src/persistence/PullRequestFilesViewed.ts` |
| 6% | 4 | 16 | `apps/server/src/provider/providerRuntimeEventIdentity.test.ts` | `apps/server/src/orchestration/Layers/ProviderRuntimeIngestion.activity.test.ts` |
| 6% | 21 | 181 | `apps/web/src/lib/threadCreatePromotion.test.ts` | `apps/server/src/relay/AgentAwarenessRelay.test.ts` |
| 6% | 44 | 189 | `apps/server/src/orchestration/Layers/pinnedMessagesRoundTrip.test.ts` | `apps/server/src/orchestration/Layers/OrchestrationEngine.test.ts` |
| 6% | 18 | 275 | `apps/web/src/components/chat/ComputerPreviewPopover.tsx` | `apps/web/src/components/device/DeviceStreamView.tsx` |
| 6% | 27 | 212 | `apps/web/src/components/chat/WorkspaceFileEditorChrome.tsx` | `apps/web/src/components/settings/ThemeSettings.tsx` |
| 6% | 7 | 139 | `apps/web/src/components/settings/skillsSettingsModel.ts` | `apps/mobile/src/features/usage/usageProviders.ts` |
| 6% | 10 | 54 | `apps/web/src/components/chat/ComposerStackedPanelContent.tsx` | `apps/web/src/components/sidebar/SidebarThreadHeader.tsx` |
| 6% | 7 | 110 | `apps/web/src/routes/_chat.hubs.index.tsx` | `apps/web/src/components/ThreadRouteView.tsx` |
| 6% | 11 | 319 | `packages/shared/src/terminalThreads.ts` | `packages/shared/src/cliArgs.ts` |
| 6% | 17 | 109 | `apps/web/src/components/chat/environment/EnvironmentRow.tsx` | `apps/web/src/components/pullRequest/PullRequestTimelineTab.tsx` |
| 6% | 13 | 154 | `apps/web/src/onboarding/steps/ProvidersStep.tsx` | `apps/web/src/components/AgentsPanel.tsx` |
| 6% | 298 | 8151 | `apps/server/src/orchestration/Layers/ProviderCommandReactor.ts` | `apps/server/src/orchestration/Layers/ProviderCommandReactor.ts` |
| 6% | 3 | 32 | `apps/web/src/components/chat/TimelineWorkEntryRow.computerDenial.test.tsx` | `apps/web/src/components/preview/PreviewEmptyState.test.tsx` |
| 6% | 5 | 100 | `apps/web/src/components/chat/environment/EnvironmentEditorSection.browser.tsx` | `apps/web/src/lib/terminalCloseShortcut.test.ts` |
| 6% | 3 | 39 | `apps/web/src/components/chat/environment/subagentRosterFixtures.ts` | `apps/web/src/lib/terminalContext.test.ts` |
| 6% | 1 | 11 | `apps/web/src/components/chat/group/GroupPluginsSection.tsx` | `apps/web/src/commandPaletteBus.ts` |
| 6% | 12 | 122 | `apps/web/src/components/kanban/KanbanOverview.tsx` | `apps/web/src/components/AgentsPanel.tsx` |
| 6% | 2 | 20 | `apps/web/src/hooks/useCommittedPathname.ts` | `packages/client-runtime/src/state/archivedThreads.ts` |
| 6% | 5 | 76 | `apps/web/src/hooks/useComputerEventBridge.test.ts` | `apps/web/src/hooks/useLocalStorage.test.ts` |
| 6% | 6 | 84 | `apps/web/src/lib/visibleInterval.test.ts` | `apps/web/src/components/permissions/usePermissionStatus.test.ts` |
| 6% | 1 | 9 | `apps/web/src/threadDetailSyncRetry.ts` | `apps/server/src/orchestration/threadDetailCursor.ts` |
| 6% | 1 | 13 | `scripts/env-bool.test.ts` | `apps/desktop/src/app/DesktopAppErrors.test.ts` |
| 6% | 21 | 184 | `apps/web/src/contextMenuFallback.ts` | `apps/web/src/contextMenuFallback.ts` |
| 6% | 13 | 79 | `apps/web/src/components/pullRequest/GitHubItemAgentActions.tsx` | `apps/web/src/components/device/DeviceControlsRail.tsx` |
| 6% | 13 | 211 | `apps/analytics/installerDownloads.ts` | `packages/client-runtime/src/projectFaviconCache.ts` |
| 6% | 71 | 1369 | `packages/contracts/src/ipc.ts` | `packages/contracts/src/ipc.ts` |
| 6% | 4 | 76 | `apps/server/src/agentGateway/computerBrowserEffect.ts` | `packages/client-runtime/src/work-log/toolPresentation.ts` |
| 6% | 34 | 346 | `apps/server/src/provider/Layers/ProviderSessionReaper.test.ts` | `apps/server/src/provider/Layers/ProviderSessionReaper.test.ts` |
| 6% | 5 | 25 | `apps/web/src/components/chat/floatingBrowserRequestStore.test.ts` | `apps/web/src/terminalUiStateStore.test.ts` |
| 6% | 16 | 161 | `apps/server/src/pullRequestAutoFix/pullRequestAutoFixDecision.test.ts` | `apps/server/src/orchestration/ThreadSettlementPolicy.test.ts` |
| 6% | 5 | 44 | `apps/web/src/lib/automationForm.test.ts` | `apps/mobile/src/widgets/subscriptionUsageSnapshot.ts` |
| 6% | 113 | 2704 | `apps/server/src/provider/Layers/ProviderHealth.test.ts` | `apps/server/src/provider/Layers/ProviderRegistry.test.ts` |
| 6% | 2 | 14 | `apps/server/src/composerPrediction/Services/ComposerPredictionService.ts` | `packages/client-runtime/src/threadPullRequestCompatibility.ts` |
| 6% | 2 | 13 | `apps/web/src/components/ChatShell.tsx` | `apps/web/src/components/EnvironmentMachineIcon.tsx` |
| 6% | 8 | 40 | `apps/web/src/components/chat/SidePanelOverlay.tsx` | `apps/web/src/components/ui/command.tsx` |
| 6% | 3 | 87 | `apps/web/src/components/profile/profileSelectors.ts` | `apps/web/src/components/pullRequest/pullRequestEditing.logic.test.ts` |
| 6% | 2 | 20 | `apps/web/src/components/profile/socialLinkDrafts.test.ts` | `apps/web/src/components/KeybindingsUpdateToast.logic.test.ts` |
| 6% | 4 | 30 | `apps/web/src/hooks/useProjectFileChangeSubscription.ts` | `apps/mobile/src/features/review/useReviewDiffPrewarming.ts` |
| 6% | 2 | 26 | `apps/web/src/lib/siteFavicon.ts` | `apps/mobile/src/features/devices/device-stream-document.ts` |
| 6% | 5 | 66 | `packages/shared/src/socialLinks.ts` | `packages/shared/src/assistantCitations.ts` |
| 6% | 7 | 33 | `apps/web/src/components/FilterPillGroup.tsx` | `apps/web/src/components/settings/ProjectIconPickerDialog.tsx` |
| 6% | 18 | 200 | `apps/web/src/components/inbox/InboxTaskList.tsx` | `apps/web/src/components/chat/WorktreeSetupCard.tsx` |
| 6% | 13 | 49 | `apps/web/src/components/pullRequest/PullRequestStackPosition.tsx` | `apps/web/src/components/pullRequest/pullRequestPresentation.tsx` |
| 6% | 5 | 78 | `apps/web/src/whatsNew/useWhatsNew.ts` | `apps/mobile/src/features/review/useNativeReviewDiffHighlighting.ts` |
| 6% | 4 | 46 | `apps/server/src/devServerManager.test.ts` | `apps/mobile/src/state/use-thread-pr.test.ts` |
| 6% | 6 | 136 | `apps/web/src/components/SnoozeReminderNotifications.browser.tsx` | `apps/web/src/components/chat/useComposerMenuState.test.tsx` |
| 6% | 2 | 63 | `apps/web/src/lib/linkChips.test.ts` | `apps/mobile/src/features/connection/pairing.test.ts` |
| 6% | 3 | 26 | `apps/web/src/lib/projectRelocation.ts` | `apps/web/src/components/desktop/DesktopAppActivationCoordinator.tsx` |
| 6% | 3 | 40 | `packages/shared/src/runtimeMode.ts` | `apps/mobile/src/features/threads/thread-settings-sheet-state.ts` |
| 6% | 7 | 206 | `apps/server/src/computer/DesktopOperationQueue.test.ts` | `apps/mobile/src/lib/serialized-async-queue.test.ts` |
| 6% | 11 | 93 | `apps/web/src/components/chat/BrowserAnnotationChip.tsx` | `apps/web/src/components/sidebar/SidebarThreadHeader.tsx` |
| 6% | 5 | 36 | `apps/web/src/components/chat/project/useHubWorkItems.ts` | `apps/web/src/components/device/DevicePhoneViewport.tsx` |
| 6% | 27 | 170 | `apps/server/src/provider/Layers/EventNdjsonLogger.ts` | `apps/server/src/provider/Layers/EventNdjsonLogger.ts` |
| 6% | 10 | 138 | `apps/web/src/components/TerminalSearch.tsx` | `apps/web/src/components/search/ProjectContentSearchDialog.tsx` |
| 6% | 9 | 283 | `apps/server/src/provider/boundedCallbackIngress.test.ts` | `apps/server/src/pullRequest/PullRequestReadCache.test.ts` |
| 6% | 1 | 23 | `apps/analytics/usagePrivacySql.ts` | `apps/web/src/lib/editableFocus.ts` |
| 6% | 4 | 89 | `apps/web/src/components/chat/AsciiWaveBackground.browser.tsx` | `apps/web/src/components/usage/usagePagePreferences.test.ts` |
| 6% | 1 | 10 | `apps/web/src/components/chat/dockPaneOpenRequest.ts` | `apps/server/src/orchestration/threadDetailCursor.ts` |
| 6% | 1 | 12 | `apps/web/src/components/chat/environment/EnvironmentToggle.tsx` | `apps/web/src/commandPaletteBus.ts` |
| 6% | 3 | 84 | `apps/web/src/components/profile/useProfileIdentity.test.tsx` | `apps/mobile/src/lib/openExternalUrl.test.ts` |
| 6% | 4 | 66 | `apps/web/src/components/settings/ComputerAuditHistorySection.test.tsx` | `apps/web/src/components/chat/ProviderStatusBanner.test.tsx` |
| 6% | 2 | 29 | `apps/web/src/hooks/useRepoDiffTotals.ts` | `apps/web/src/components/chat/DiffStatLabel.tsx` |
| 6% | 2 | 51 | `apps/web/src/lib/pdf/pdfZoom.ts` | `apps/mobile/src/components/MaterialMenuPopup.tsx` |
| 6% | 3 | 28 | `apps/web/vitest.unit.ci.config.ts` | `apps/server/vite.config.ts` |
| 6% | 1 | 21 | `packages/shared/src/installerDownloads.ts` | `apps/web/src/clientPersistenceStorage.ts` |
| 6% | 3 | 21 | `packages/shared/src/platformEnvironment.ts` | `apps/server/src/sourceControl/bitbucketPullRequests.ts` |
| 6% | 33 | 104 | `apps/web/src/components/account/DeleteAccountControl.tsx` | `apps/web/src/components/settings/IntegrationsSettings.tsx` |
| 6% | 22 | 422 | `apps/web/src/components/chat/useChatPendingInteractions.ts` | `apps/mobile/src/state/use-selected-thread-requests.ts` |
| 6% | 5 | 29 | `apps/server/src/terminal/runtimeSelection.test.ts` | `apps/desktop/src/app/DesktopEnvironment.test.ts` |
| 6% | 4 | 22 | `apps/web/src/lib/contextMenuGroup.test.ts` | `apps/web/src/components/threadActionMenu.logic.ts` |
| 6% | 3 | 70 | `apps/web/src/lib/voiceRecorderEncoding.test.ts` | `packages/client-runtime/src/filePreview.ts` |
| 6% | 9 | 182 | `packages/profile-ui/src/providerIcon.tsx` | `apps/mobile/src/components/SourceControlIcon.tsx` |
| 6% | 4 | 62 | `apps/desktop/src/initialBackendWindowOpen.test.ts` | `apps/web/src/desktopAppActivation.test.ts` |
| 6% | 4 | 41 | `apps/server/src/terminal/terminalThreadTitle.test.ts` | `apps/web/src/components/preview/previewAutomationOpenReadiness.test.ts` |
| 6% | 2 | 26 | `apps/web/src/components/chat/ComposerBlockedHandoffNotice.tsx` | `apps/mobile/src/features/files/WorkspaceFileWebPreview.tsx` |
| 6% | 3 | 175 | `apps/web/src/components/chat/openSubagentThread.logic.test.ts` | `packages/client-runtime/src/threadPullRequestCompatibility.test.ts` |
| 6% | 9 | 108 | `apps/web/src/components/settings/accounts/AccountsSettingsPanel.tsx` | `apps/web/src/components/settings/ProjectActionsSettings.tsx` |
| 6% | 2 | 17 | `packages/shared/src/sidechatThread.ts` | `packages/client-runtime/src/textPaste.ts` |
| 6% | 14 | 288 | `apps/web/src/components/BrowserPanel.menuOcclusion.browser.tsx` | `apps/web/src/components/preview/PreviewMoreMenu.tsx` |
| 6% | 24 | 210 | `apps/server/src/orchestration/Layers/HubOutputReactor.test.ts` | `apps/server/src/provider/Layers/ProviderSessionReaper.test.ts` |
| 6% | 7 | 107 | `apps/server/src/agentGateway/harnessPolicy.ts` | `apps/server/src/provider/providerStatusCache.ts` |
| 6% | 6 | 87 | `apps/server/src/remoteTransport/resourceProxy.ts` | `apps/server/src/assets/GitHubMediaFetch.ts` |
| 6% | 6 | 22 | `apps/web/src/components/RenameThreadDialog.tsx` | `apps/web/src/components/pullRequest/PullRequestCandidatePicker.tsx` |
| 6% | 5 | 40 | `apps/web/src/workspacePathsStore.clearLegacy.test.ts` | `apps/web/src/clientPersistenceStorage.test.ts` |
| 6% | 6 | 180 | `packages/contracts/src/agentGateway.ts` | `apps/server/src/persistence/Services/ProjectionThreads.ts` |
| 6% | 7 | 16 | `apps/server/src/auth/Services/ServerSecretStore.ts` | `apps/server/src/auth/ServerSecretStore.ts` |
| 6% | 6 | 60 | `apps/server/src/orchestration/threadShellEvents.ts` | `packages/client-runtime/src/pendingRequests.ts` |
| 6% | 8 | 56 | `scripts/lib/node-pty-smoke.ts` | `apps/server/src/cli/app.ts` |
| 6% | 10 | 115 | `apps/web/src/components/chat/BackgroundTasksDockPane.tsx` | `apps/web/src/components/chat/WorktreeSetupCard.tsx` |
| 6% | 13 | 236 | `apps/web/src/components/chat/useChatProjectScripts.ts` | `apps/web/src/components/settings/useProjectScriptSettings.ts` |
| 6% | 9 | 95 | `apps/web/src/workspacePathsStore.ts` | `apps/web/src/browserFaviconStore.ts` |
| 6% | 4 | 33 | `apps/e2e/src/harness/network.ts` | `apps/desktop/src/electron/WindowsForegroundFocusThread.ts` |
| 6% | 5 | 61 | `apps/server/src/backendCompileCache.test.ts` | `apps/mobile/src/features/threads/thread-settings-sheet-state.test.ts` |
| 6% | 2 | 24 | `apps/server/src/codexTurnInput.test.ts` | `apps/web/src/components/diffs/diffFileTree.logic.test.ts` |
| 6% | 1 | 11 | `apps/server/src/orchestration/Services/SidechatExpiryReactor.ts` | `apps/server/src/orchestration/threadDetailCursor.ts` |
| 6% | 4 | 26 | `apps/server/src/persistence/commitNotifications.ts` | `apps/desktop/src/wsl/DesktopWslServerTree.ts` |
| 6% | 2 | 17 | `apps/server/src/provider/Services/ProviderHealth.ts` | `packages/client-runtime/src/threadPullRequestCompatibility.ts` |
| 6% | 1 | 17 | `apps/server/src/provider/promptReferenceProjection.ts` | `apps/server/src/checkpointing/Utils.ts` |
| 6% | 2 | 19 | `apps/web/src/components/chat/surfaceContentTabs.logic.test.ts` | `apps/mobile/src/features/diagnostics/crash-log-model.test.ts` |
| 6% | 3 | 30 | `apps/web/src/hooks/useBrowserPanelDesktopBridge.ts` | `apps/web/src/cloud/managedAuth.tsx` |
| 6% | 5 | 125 | `apps/web/src/lib/browserDownload.test.ts` | `apps/web/src/components/preview/openTerminalLinkInPreview.test.ts` |
| 6% | 8 | 134 | `apps/web/src/lib/pdf/usePdfPageRender.ts` | `apps/mobile/src/features/cloud/CloudAuthProvider.tsx` |
| 6% | 1 | 19 | `apps/web/src/sidebarNavOrdering.test.ts` | `apps/server/src/pullRequest/http.ts` |
| 6% | 4 | 167 | `packages/shared/src/desktopOperationQueue.ts` | `apps/mobile/src/lib/serialized-async-queue.ts` |
| 6% | 8 | 93 | `apps/server/src/remotePairing/cli.ts` | `apps/server/src/cli/app.ts` |
| 6% | 3 | 45 | `apps/web/src/components/chat/project/projectPanel.logic.test.ts` | `apps/server/src/provider/Layers/codexUsageLimits.test.ts` |
| 6% | 4 | 8 | `apps/desktop/src/desktopWsBridge.ts` | `apps/web/src/pendingUserInput.ts` |
| 6% | 8 | 105 | `apps/web/src/components/PdfFilePreview.tsx` | `apps/web/src/components/chat/ComposerStashMenu.tsx` |
| 6% | 8 | 41 | `apps/web/src/components/automation/AutomationRiskConfirmPopover.tsx` | `apps/web/src/components/usage/UsageLimits.tsx` |
| 6% | 33 | 603 | `apps/web/src/components/chat/ProjectPicker.tsx` | `apps/web/src/components/BranchToolbarBranchSelector.tsx` |
| 6% | 3 | 58 | `apps/server/src/wsCompatibility.test.ts` | `apps/server/src/mcp/McpInvocationContext.test.ts` |
| 6% | 3 | 46 | `apps/web/src/dockExplorerBrowseStore.test.ts` | `apps/web/src/lib/terminalContext.test.ts` |
| 6% | 3 | 17 | `packages/shared/src/workspaceFileWrite.test.ts` | `apps/mobile/src/features/cloud/publicConfig.test.ts` |
| 6% | 5 | 58 | `apps/desktop/src/agentCursorPreference.test.ts` | `scripts/lib/public-config.test.ts` |
| 6% | 3 | 146 | `apps/server/src/agentGateway/computerForegroundConsent.test.ts` | `apps/mobile/src/lib/videoThumbnails.test.ts` |
| 6% | 4 | 33 | `apps/server/src/betaUsageSnapshot.ts` | `apps/server/scripts/migrate-dev-db.test.ts` |
| 6% | 5 | 44 | `apps/web/src/lib/audioLevel.test.ts` | `apps/web/src/hooks/useCopyToClipboard.test.ts` |
| 6% | 4 | 93 | `apps/desktop/scripts/build-window-material-addon.mjs` | `apps/mobile/scripts/permissions-service.test.ts` |
| 6% | 12 | 233 | `apps/web/src/components/ThreadHoverCardContent.tsx` | `apps/web/src/components/AgentsPanel.tsx` |
| 6% | 11 | 75 | `apps/web/src/components/chat/ActiveTaskListCard.tsx` | `apps/web/src/components/chat/ChangedFilesTree.tsx` |
| 6% | 5 | 46 | `apps/web/src/components/profile/ShareCard.browser.tsx` | `apps/mobile/src/features/usage/usageProviders.ts` |
| 6% | 12 | 32 | `apps/web/src/components/tasks/NewTaskButton.tsx` | `apps/web/src/components/device/DeviceControlsRail.tsx` |
| 6% | 2 | 95 | `apps/web/src/lib/chatReferences.test.ts` | `apps/desktop/src/linuxSecretStorage.test.ts` |
| 6% | 7 | 177 | `apps/web/src/components/SidebarGroupsSurface.logic.test.ts` | `apps/web/src/components/chat/ModelPickerContent.test.ts` |
| 6% | 9 | 149 | `apps/server/src/agentGateway/toolInput.ts` | `apps/server/src/provider/providerStatusCache.ts` |
| 6% | 87 | 2317 | `apps/web/src/components/ChatView.logic.test.ts` | `apps/web/src/components/ChatView.logic.test.ts` |
| 6% | 4 | 18 | `apps/server/src/betaFeatureGate.test.ts` | `apps/web/src/timestampFormat.test.ts` |
| 6% | 202 | 2605 | `apps/server/src/orchestration/Layers/ProjectionSnapshotQuery.test.ts` | `apps/server/src/orchestration/Layers/ProjectionSnapshotQuery.test.ts` |
| 6% | 14 | 150 | `apps/web/src/components/chat/ComposerModelMenuTrigger.tsx` | `apps/web/src/components/chat/ChangedFilesTree.tsx` |
| 6% | 6 | 112 | `apps/desktop/src/browserAutomation/betterwrightKeyboardPolicy.test.ts` | `apps/desktop/src/preview/PreviewKeyboard.test.ts` |
| 6% | 3 | 14 | `apps/marketing/src/components/InstallerCount.tsx` | `apps/web/src/components/chat/ContextWindowMeter.tsx` |
| 6% | 2 | 49 | `apps/server/src/agentGateway/Services/AgentGatewaySessionRegistry.ts` | `apps/server/src/provider/Services/ProviderSessionDirectory.ts` |
| 6% | 2 | 19 | `apps/server/src/todo/Services/TodoService.ts` | `packages/client-runtime/src/threadPullRequestCompatibility.ts` |
| 6% | 12 | 339 | `apps/web/src/components/chat/ComposerPendingApprovalPanel.tsx` | `apps/web/src/components/chat/ComposerPendingApprovalPanel.tsx` |
| 6% | 2 | 25 | `apps/web/src/components/chat/project/projectAgentDialog.logic.test.ts` | `apps/desktop/src/linuxSecretStorage.test.ts` |
| 6% | 8 | 147 | `apps/web/src/lib/fileComments.ts` | `apps/web/src/lib/terminalContext.ts` |
| 6% | 4 | 25 | `packages/shared/src/chatGptVoiceTranscription.test.ts` | `apps/mobile/src/features/agent-awareness/notificationNavigation.test.ts` |
| 6% | 16 | 358 | `apps/desktop/src/preload.ts` | `apps/desktop/src/preload.ts` |
| 6% | 11 | 256 | `apps/web/src/hooks/useThreadActionShortcuts.browser.tsx` | `apps/web/src/lib/snapShotShortcut.test.ts` |
| 6% | 6 | 90 | `apps/server/src/device/deviceTypeCatalogue.test.ts` | `apps/server/src/environment/ServerEnvironmentMachine.test.ts` |
| 6% | 30 | 296 | `apps/server/src/git/Layers/GitStatusBroadcaster.test.ts` | `apps/server/src/vcs/VcsStatusBroadcaster.test.ts` |
| 6% | 15 | 233 | `apps/web/src/components/profile/ShareDialog.tsx` | `apps/web/src/components/settings/ProjectIconPickerDialog.tsx` |
| 6% | 4 | 73 | `apps/web/src/lib/projectCatalog/model.test.ts` | `apps/mobile/src/features/threads/thread-provider-instance.test.ts` |
| 6% | 11 | 58 | `apps/web/src/lib/providerInstancePresentation.ts` | `apps/web/src/components/settings/AddProviderInstanceDialog.tsx` |
| 6% | 11 | 184 | `apps/server/src/provider/opencodeRuntime.poolIdentity.test.ts` | `packages/tailscale/src/tailscale.test.ts` |
| 6% | 3 | 63 | `apps/web/src/components/SpaceIcon.tsx` | `apps/web/src/components/settings/EnvironmentRow.tsx` |
| 6% | 44 | 614 | `apps/web/src/components/chat/group/LibraryPanel.tsx` | `apps/web/src/components/settings/IntegrationsSettings.tsx` |
| 5% | 15 | 131 | `apps/web/src/components/chat/WorkerMonitorNoticePill.tsx` | `apps/web/src/components/pullRequest/PullRequestSummaryTab.tsx` |
| 5% | 5 | 67 | `packages/shared/src/coordinatorCheckin.ts` | `apps/web/src/components/chat/composerPromptHistory.ts` |
| 5% | 2 | 56 | `apps/server/src/diagnostics/Services/ThreadDiagnosticsQuery.ts` | `apps/mobile/src/lib/commandMetadata.ts` |
| 5% | 42 | 658 | `apps/web/src/components/settings/DesktopSettingsPanels.tsx` | `apps/web/src/components/settings/IntegrationsSettings.tsx` |
| 5% | 2 | 33 | `apps/web/src/components/settings/UsageSettingsPanel.tsx` | `apps/web/src/components/preview/PreviewFaviconIcon.tsx` |
| 5% | 3 | 30 | `apps/web/src/settingsNavigation.test.ts` | `apps/mobile/src/features/settings/environment-maintenance.test.ts` |
| 5% | 4 | 81 | `apps/web/src/wsTransportEvents.ts` | `apps/mobile/src/connection/background-activity-scopes.ts` |
| 5% | 7 | 104 | `apps/web/src/lib/workspaceFileOpener.test.ts` | `apps/web/src/markdown-links.test.ts` |
| 5% | 7 | 74 | `apps/web/src/components/chat/project/useProjectInstructionsSource.ts` | `apps/web/src/remoteOpen.ts` |
| 5% | 6 | 24 | `apps/web/src/components/ui/proposal-actions.tsx` | `apps/web/src/components/pullRequest/PullRequestReviewAnnotation.tsx` |
| 5% | 14 | 131 | `apps/web/src/onboarding/OnboardingScreen.tsx` | `apps/web/src/components/search/ProjectContentSearchDialog.tsx` |
| 5% | 12 | 191 | `apps/web/src/components/ProjectPicker.browser.tsx` | `apps/server/src/git/linkCreatedPullRequest.test.ts` |
| 5% | 10 | 71 | `apps/web/src/components/TerminalWorkspaceTabs.tsx` | `apps/web/src/components/AgentsPanel.tsx` |
| 5% | 3 | 20 | `apps/web/src/lib/localFoldersGroupLabel.ts` | `apps/web/src/lib/snapShotShortcut.ts` |
| 5% | 20 | 77 | `apps/server/src/auth/Services/ServerAuth.ts` | `apps/server/src/auth/EnvironmentAuth.ts` |
| 5% | 4 | 102 | `apps/web/src/components/pullRequest/pullRequestStatePresentation.tsx` | `apps/web/src/components/pullRequest/pullRequestIcons.tsx` |
| 5% | 4 | 33 | `packages/contracts/src/rpc.test.ts` | `apps/mobile/src/features/threads/thread-settings-sheet-state.test.ts` |
| 5% | 9 | 92 | `apps/server/src/persistence/Migrations/101_RemoveTranscriptMarkers.test.ts` | `apps/server/src/persistence/Migrations/026_CanonicalizeModelSelectionOptions.test.ts` |
| 5% | 1 | 15 | `apps/desktop/src/i18n.test.ts` | `apps/web/src/portDiscoveryState.test.ts` |
| 5% | 2 | 28 | `apps/server/src/environment/machineId.test.ts` | `apps/desktop/src/linuxSecretStorage.test.ts` |
| 5% | 3 | 52 | `apps/server/src/nodeHttpServer.upgradePath.test.ts` | `apps/web/src/portDiscoveryState.test.ts` |
| 5% | 1 | 13 | `apps/server/src/orchestration/Services/HubOutputReactor.ts` | `apps/server/src/orchestration/threadDetailCursor.ts` |
| 5% | 4 | 16 | `apps/web/src/components/chat/FastModeBadgeIcon.tsx` | `apps/web/src/components/ProjectFavicon.tsx` |
| 5% | 6 | 177 | `packages/contracts/src/account.test.ts` | `apps/mobile/src/state/use-thread-pr.test.ts` |
| 5% | 5 | 122 | `apps/web/src/recentViews.logic.test.ts` | `packages/client-runtime/src/threadPullRequestCompatibility.test.ts` |
| 5% | 11 | 259 | `apps/server/src/agentGateway/Layers/AgentGatewayOperationRepository.ts` | `apps/server/src/persistence/Layers/ProjectionPendingApprovals.ts` |
| 5% | 3 | 45 | `apps/server/src/projectAgent/groupsBetaGate.test.ts` | `apps/web/src/hooks/useThreadActions.test.ts` |
| 5% | 5 | 70 | `apps/web/src/routes/-chatIndexRoute.logic.ts` | `apps/server/src/orchestration/commandInvariants.ts` |
| 5% | 6 | 19 | `apps/web/src/terminalActivity.ts` | `packages/client-runtime/src/state/terminalSession.ts` |
| 5% | 6 | 95 | `scripts/lib/desktop-runtime-resources.test.ts` | `apps/server/src/provider/Layers/ClaudeCapabilitiesProbe.test.ts` |
| 5% | 2 | 9 | `apps/analytics/src/main.tsx` | `apps/web/src/components/chat/useComposerMenuState.test.tsx` |
| 5% | 2 | 57 | `apps/server/src/inlineVisualStore.ts` | `apps/server/src/attachmentPaths.ts` |
| 5% | 2 | 22 | `apps/server/src/pullRequestAutoFix/Services/PullRequestAutoFixService.ts` | `packages/client-runtime/src/threadPullRequestCompatibility.ts` |
| 5% | 8 | 34 | `apps/web/src/components/pullRequest/PullRequestCheckStatusIcon.tsx` | `apps/web/src/components/chat/WorktreeSetupCard.tsx` |
| 5% | 13 | 37 | `apps/web/src/components/pullRequest/PullRequestDisclosureSection.tsx` | `apps/web/src/components/pullRequest/PullRequestTimelineTab.tsx` |
| 5% | 7 | 60 | `apps/web/src/terminalActivity.test.ts` | `apps/mobile/src/features/terminal/terminalMenu.test.ts` |
| 5% | 1 | 16 | `packages/shared/src/threadExport.test.ts` | `apps/web/src/cloud/connectCliAuth.test.ts` |
| 5% | 23 | 897 | `apps/server/src/http.ts` | `apps/server/src/http.ts` |
| 5% | 7 | 225 | `apps/web/src/components/chat/useChatTerminalController.test.ts` | `apps/web/src/test/reactHookHarness.ts` |
| 5% | 6 | 66 | `apps/web/src/lib/composerTransportStatus.test.ts` | `apps/web/src/components/Sidebar.pointer.test.ts` |
| 5% | 3 | 38 | `apps/web/src/lib/providerSignInOutput.ts` | `apps/desktop/scripts/wait-for-resources.mjs` |
| 5% | 27 | 257 | `apps/web/src/storeTestFixtures.ts` | `packages/client-runtime/src/state/entities.test.ts` |
| 5% | 4 | 128 | `apps/server/src/computer/computerEventInterests.test.ts` | `apps/web/src/hooks/threadUndo.ts` |
| 5% | 8 | 63 | `apps/web/src/whatsNew/ChangelogAccordion.tsx` | `apps/web/src/components/settings/OpenSourceLicenses.tsx` |
| 5% | 6 | 35 | `apps/marketing/src/components/ThemeToggle.tsx` | `apps/web/src/components/settings/ProviderAccentColorPicker.tsx` |
| 5% | 5 | 49 | `apps/server/src/mcpConnectors/Layers/McpConnectors.test.ts` | `apps/server/scripts/evaluate-thread-titles.ts` |
| 5% | 6 | 25 | `apps/web/src/components/chat/ComposerExtrasTrigger.tsx` | `apps/web/src/components/settings/ProjectIconPickerDialog.tsx` |
| 5% | 3 | 99 | `apps/web/src/hooks/useSmoothStreamedText.ts` | `apps/mobile/src/features/review/useNativeReviewDiffHighlighting.ts` |
| 5% | 1 | 18 | `apps/server/src/agentGateway/Services/AgentGateway.ts` | `packages/client-runtime/src/device/screenshot.ts` |
| 5% | 3 | 53 | `apps/server/src/mcpConnectors/Services/McpConnectors.ts` | `apps/server/src/provider/ProviderDriver.ts` |
| 5% | 5 | 53 | `apps/server/src/orchestration/projectImportPaths.ts` | `apps/server/src/vcs/VcsProjectConfig.ts` |
| 5% | 2 | 22 | `apps/server/src/orchestration/providerSessionThread.ts` | `apps/mobile/src/features/threads/fileChipMenu.ts` |
| 5% | 2 | 28 | `apps/server/src/persistence/Services/PullRequestAutoFixRepository.ts` | `apps/server/src/persistence/Services/ProjectionPendingApprovals.ts` |
| 5% | 4 | 108 | `apps/server/src/terminal/providerTerminalProfiles.test.ts` | `apps/mobile/src/features/settings/settings-scoped-server.test.ts` |
| 5% | 1 | 16 | `apps/web/src/components/HubsEmptyState.tsx` | `apps/web/src/commandPaletteBus.ts` |
| 5% | 1 | 14 | `apps/web/src/components/chat/browserPanelOpenRequest.ts` | `apps/server/src/orchestration/threadDetailCursor.ts` |
| 5% | 7 | 139 | `apps/web/src/components/chat/threadModelSelectionSync.test.ts` | `apps/web/src/browser/desktopTabLifetime.test.ts` |
| 5% | 2 | 56 | `apps/web/src/file-icons.test.ts` | `apps/desktop/src/app/DesktopAppErrors.test.ts` |
| 5% | 3 | 44 | `apps/web/src/lib/toolArgumentSummary.ts` | `apps/web/src/browserFaviconLogic.ts` |
| 5% | 2 | 12 | `packages/shared/src/groupSlug.ts` | `apps/web/src/projectScripts.ts` |
| 5% | 397 | 6960 | `apps/web/src/components/Sidebar.tsx` | `apps/web/src/components/LegacySidebar.tsx` |
| 5% | 5 | 68 | `apps/server/src/betaOperationalIssue.test.ts` | `apps/web/src/timestampFormat.test.ts` |
| 5% | 81 | 1358 | `apps/web/src/components/chat/MessagesTimeline.test.tsx` | `apps/web/src/components/chat/MessagesTimeline.test.tsx` |
| 5% | 16 | 279 | `apps/web/src/components/settings/accounts/ProviderAccountsList.tsx` | `apps/web/src/components/settings/ProviderInstanceCard.tsx` |
| 5% | 4 | 41 | `apps/server/src/environment/machineId.ts` | `apps/web/src/connection/clientMetadata.ts` |
| 5% | 3 | 46 | `apps/server/src/orchestration/Layers/ProviderCommandReactor.skillMentions.test.ts` | `apps/web/src/components/chat/ContextWindowMeter.logic.test.ts` |
| 5% | 48 | 538 | `apps/server/src/orchestration/startupPendingInteractionReconciliation.test.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.test.ts` |
| 5% | 15 | 266 | `apps/web/src/components/chat/group/GroupModelEffortRow.tsx` | `apps/web/src/components/settings/SourceControlWritingSettings.tsx` |
| 5% | 6 | 123 | `apps/web/src/lib/assistantSelections.ts` | `apps/web/src/lib/terminalContext.ts` |
| 5% | 2 | 18 | `apps/desktop/src/desktopWsBridge.test.ts` | `apps/web/src/cloud/connectCliAuth.test.ts` |
| 5% | 5 | 152 | `apps/desktop/src/linuxAppIcon.test.ts` | `apps/web/src/browserFaviconStore.test.ts` |
| 5% | 6 | 61 | `apps/desktop/src/mediaPermissions.test.ts` | `apps/web/src/components/desktopUpdate.logic.test.ts` |
| 5% | 2 | 57 | `apps/server/src/agentGateway/protocol.test.ts` | `apps/desktop/src/app/DesktopAppErrors.test.ts` |
| 5% | 28 | 200 | `apps/server/src/orchestration/decider.computerControl.test.ts` | `apps/server/src/orchestration/decider.settled.test.ts` |
| 5% | 2 | 9 | `apps/server/src/projectAgent/groupCreateAllowlist.ts` | `apps/web/src/lib/chatThreadActions.ts` |
| 5% | 2 | 24 | `apps/server/src/worktreeSetup.ts` | `apps/web/src/components/device/deviceFold.ts` |
| 5% | 3 | 33 | `apps/web/src/components/settings/accounts/CodexDefaultAccountControl.tsx` | `apps/web/src/components/BranchToolbarEnvironmentSelector.tsx` |
| 5% | 4 | 12 | `apps/web/src/lib/requestOutcome.ts` | `packages/client-runtime/src/state/attachments.ts` |
| 5% | 2 | 25 | `apps/web/src/pinnedProjectsStore.test.ts` | `apps/web/src/components/pullRequest/pullRequestReviewStore.test.ts` |
| 5% | 13 | 63 | `apps/web/src/components/chat/TranscriptSelectionAction.tsx` | `apps/web/src/components/settings/settingsLayout.tsx` |
| 5% | 16 | 102 | `apps/web/src/components/tasks/TaskListItem.tsx` | `apps/web/src/components/pullRequest/PullRequestRow.tsx` |
| 5% | 3 | 42 | `apps/server/src/orchestration/orchestrationAdmission.ts` | `apps/server/src/project/AgentSessionJson.ts` |
| 5% | 12 | 341 | `apps/server/src/project/githubProjectProvisioning.test.ts` | `apps/server/src/sourceControl/SourceControlRepositoryService.test.ts` |
| 5% | 9 | 53 | `apps/web/src/pinnedProjectsStore.ts` | `apps/web/src/browserHistoryStore.ts` |
| 5% | 6 | 44 | `apps/desktop/src/contextMenuTemplate.ts` | `apps/desktop/src/electron/ElectronMenu.ts` |
| 5% | 26 | 317 | `apps/server/src/agentGateway/creationCoordinator.hubWork.test.ts` | `apps/server/src/orchestration/ThreadPullRequestReactor.test.ts` |
| 5% | 4 | 36 | `apps/server/src/git/textGenerationSelection.test.ts` | `packages/contracts/src/provider.test.ts` |
| 5% | 4 | 38 | `apps/web/src/components/pullRequest/pullRequestStatePresentation.test.tsx` | `apps/mobile/src/state/use-thread-pr.test.ts` |
| 5% | 7 | 52 | `apps/web/src/components/pullRequest/useHostThreadSidechatAsk.ts` | `apps/web/src/components/pullRequest/PullRequestStackMenu.tsx` |
| 5% | 16 | 275 | `apps/server/src/processArgumentRedaction.ts` | `packages/client-runtime/src/work-log/commandLabel.ts` |
| 5% | 8 | 112 | `apps/server/src/threadSearch.ts` | `apps/server/src/persistence/ProjectionThreadPullRequests.ts` |
| 5% | 4 | 66 | `apps/desktop/scripts/betterwright-recovery-smoke.ts` | `scripts/install.test.ts` |
| 5% | 24 | 188 | `apps/desktop/src/updateState.ts` | `apps/desktop/src/updates/DesktopUpdates.ts` |
| 5% | 3 | 35 | `apps/server/src/serverRuntimeStartup.test.ts` | `apps/server/src/mcp/toolkits/preview/handlers.test.ts` |
| 5% | 10 | 257 | `apps/web/e2e/fixtures/siteServer.ts` | `apps/server/src/cloud/cliAuthHtml.ts` |
| 5% | 1 | 18 | `apps/web/src/branding.ts` | `apps/web/src/branding.ts` |
| 5% | 4 | 110 | `apps/web/src/components/DiffPanelPatchViewport.tsx` | `apps/web/src/components/chat/ComposerStashBadge.tsx` |
| 5% | 3 | 54 | `apps/web/src/components/chat/WorkflowRunCard.browser.tsx` | `apps/web/src/components/device/DeviceStreamView.test.tsx` |
| 5% | 2 | 64 | `apps/web/src/components/chat/chatTypography.ts` | `apps/web/src/pierre-tree-theme.ts` |
| 5% | 2 | 12 | `apps/web/src/components/chat/userMessageCollapse.ts` | `.github/scripts/check-nightly-release.cjs` |
| 5% | 5 | 47 | `apps/web/src/components/profile/shareCardExport.ts` | `apps/web/src/remoteOpen.ts` |
| 5% | 2 | 20 | `apps/web/src/lib/diffScrollSurface.test.ts` | `apps/web/src/hooks/useLiveRefresh.test.ts` |
| 5% | 10 | 55 | `apps/web/src/pinnedProjectAgentsStore.ts` | `apps/web/src/browserHistoryStore.ts` |
| 5% | 2 | 40 | `apps/web/src/projectEnvironmentStore.test.ts` | `apps/server/src/pullRequest/pullRequestSyncKey.test.ts` |
| 5% | 3 | 60 | `apps/web/src/routeRestoreRefreshCoordinator.ts` | `apps/web/src/browser/browserViewportActions.ts` |
| 5% | 3 | 48 | `apps/web/src/threadActivation.logic.ts` | `apps/web/src/lib/terminalContext.ts` |
| 5% | 4 | 56 | `packages/contracts/src/threadDiagnosticsRpc.test.ts` | `packages/contracts/src/providerRuntime.test.ts` |
| 5% | 2 | 42 | `scripts/lib/cua-benchmark.test.ts` | `apps/server/src/compileCache.test.ts` |
| 5% | 14 | 143 | `apps/web/src/components/SidebarThreadRowContent.tsx` | `apps/web/src/components/ThreadStatusIndicators.tsx` |
| 5% | 6 | 31 | `apps/server/src/index.ts` | `apps/desktop/src/main.ts` |
| 5% | 16 | 256 | `apps/web/src/components/pullRequest/GitHubItemInfo.tsx` | `apps/web/src/components/pullRequest/PullRequestTimelineTab.tsx` |
| 5% | 9 | 403 | `apps/server/src/persistence/Services/AutomationRepository.ts` | `apps/server/src/persistence/Services/ProjectionTurns.ts` |
| 5% | 3 | 48 | `apps/server/src/persistence/Services/HubWorkRepository.ts` | `apps/mobile/src/lib/projectThreadStartTurn.ts` |
| 5% | 3 | 67 | `apps/web/src/components/EditorWorkspaceView.test-support.tsx` | `apps/web/src/components/ui/switch.test.tsx` |
| 5% | 3 | 81 | `packages/shared/src/computerGrants.test.ts` | `apps/mobile/src/features/threads/use-composer-command-menu.test.ts` |
| 5% | 2 | 47 | `packages/shared/src/computerInvocation.test.ts` | `packages/client-runtime/src/textPaste.test.ts` |
| 5% | 6 | 151 | `scripts/check-migration-lineage.test.ts` | `packages/client-runtime/src/state/sharedSettings.test.ts` |
| 5% | 44 | 1278 | `apps/web/src/components/SidebarActivityView.tsx` | `apps/web/src/components/BranchToolbar.tsx` |
| 5% | 10 | 48 | `apps/web/src/components/pullRequest/GitHubLabelChips.tsx` | `apps/web/src/components/pullRequest/pullRequestPresentation.tsx` |
| 5% | 3 | 81 | `apps/web/src/lib/threadModelSummary.test.ts` | `packages/contracts/src/provider.test.ts` |
| 5% | 8 | 86 | `packages/shared/src/git.test.ts` | `packages/shared/src/git.test.ts` |
| 5% | 4 | 54 | `apps/desktop/src/desktopStorageMigration.test.ts` | `apps/mobile/src/lib/composerContext.test.ts` |
| 5% | 4 | 49 | `packages/shared/src/windowsWslProcess.test.ts` | `apps/web/src/connection/clientMetadata.test.ts` |
| 5% | 3 | 82 | `apps/desktop/src/rotatingFileSink.test.ts` | `packages/shared/src/logging.test.ts` |
| 5% | 3 | 38 | `apps/server/src/config.remoteAccess.test.ts` | `apps/server/src/auth/utils.test.ts` |
| 5% | 4 | 48 | `apps/server/src/serverRuntimeStartup.ts` | `packages/shared/src/Net.ts` |
| 5% | 3 | 72 | `packages/shared/src/providerUsage.test.ts` | `apps/mobile/src/features/cloud/cloudEnvironmentPresentation.test.ts` |
| 5% | 11 | 115 | `apps/server/src/persistence/projectImportRepository.ts` | `apps/server/src/persistence/ProviderSessionRuntime.ts` |
| 5% | 10 | 371 | `apps/server/src/usageAnalyticsStats.test.ts` | `apps/server/src/persistence/Migrations/016_CanonicalizeModelSelections.test.ts` |
| 5% | 8 | 220 | `packages/shared/src/KeyedDrainableWorker.test.ts` | `apps/server/src/provider/ProviderAuthFlow.test.ts` |
| 5% | 1 | 29 | `apps/desktop/src/productAnalyticsIpc.ts` | `apps/mobile/src/lib/composerAttachmentPreviewRetention.ts` |
| 5% | 1 | 15 | `apps/server/src/__fixtures__/credentialLockWorker.mjs` | `apps/desktop/src/snapShot/GlobalShiftShortcutWorker.ts` |
| 5% | 4 | 21 | `apps/server/src/realpathNearestExisting.ts` | `apps/server/src/vcs/VcsProjectConfig.ts` |
| 5% | 1 | 21 | `apps/web/src/components/chat/ComposerGoalHeader.test.ts` | `apps/web/src/cloud/connectCliAuth.test.ts` |
| 5% | 4 | 19 | `apps/web/src/components/chat/InlineAgentChip.tsx` | `apps/web/src/browser/BrowserViewportResizeHandles.tsx` |
| 5% | 5 | 112 | `apps/web/src/components/chat/UserInputQuestionForm.browser.tsx` | `apps/web/src/components/chat/ComposerPendingUserInputPanel.test.tsx` |
| 5% | 1 | 12 | `apps/web/src/components/device/DeviceFrame.test.ts` | `apps/web/src/cloud/dpop.test.ts` |
| 5% | 5 | 56 | `apps/web/src/components/terminal/terminalRuntimeAppearance.browser.tsx` | `apps/web/src/components/ThreadTerminalDrawer.test.ts` |
| 5% | 1 | 10 | `apps/web/src/lib/openUsageReactQuery.test.ts` | `apps/web/src/components/usage/usageShortcuts.test.ts` |
| 5% | 5 | 171 | `apps/server/src/computer/computerControlInvariants.test.ts` | `apps/mobile/src/lib/serialized-async-queue.test.ts` |
| 5% | 12 | 329 | `apps/server/src/device/helperClient.ts` | `apps/desktop/src/app/DesktopAppActivation.ts` |
| 5% | 4 | 46 | `apps/desktop/src/contextMenuTemplate.test.ts` | `apps/web/src/components/threadActionMenu.logic.ts` |
| 5% | 8 | 37 | `apps/web/src/components/settings/accounts/ProviderAccentColorControl.tsx` | `apps/web/src/components/AgentsPanel.tsx` |
| 5% | 5 | 12 | `apps/web/src/lib/pendingInteraction.ts` | `packages/client-runtime/src/pendingRequests.ts` |
| 5% | 3 | 32 | `apps/desktop/scripts/source-desktop-bootstrap.cjs` | `apps/mobile/metro.config.js` |
| 5% | 8 | 258 | `apps/server/src/persistence/Services/ProjectAgentRepository.ts` | `apps/server/src/orchestration/Services/ProjectionSnapshotQuery.ts` |
| 5% | 4 | 47 | `apps/server/tsdown.config.ts` | `apps/server/vite.config.ts` |
| 5% | 1 | 28 | `apps/web/src/lib/chatWorkspaceFolders.test.ts` | `apps/web/src/browserFaviconLogic.test.ts` |
| 5% | 4 | 194 | `packages/shared/src/desktopIdentity.test.ts` | `apps/mobile/src/features/cloud/publicConfig.test.ts` |
| 5% | 2 | 28 | `apps/server/src/pullRequests/Services/PullRequestService.ts` | `packages/client-runtime/src/threadPullRequestCompatibility.ts` |
| 5% | 2 | 27 | `apps/web/src/components/chat/floatingBrowserRequestStore.ts` | `apps/mobile/src/features/threads/fileChipMenu.ts` |
| 5% | 2 | 15 | `apps/web/src/components/chat/project/projectAgentDialog.logic.ts` | `apps/mobile/src/features/threads/new-task-context-presentation.ts` |
| 5% | 4 | 43 | `apps/web/src/lib/matchHighlight.ts` | `apps/web/src/composer-editor-mentions.ts` |
| 5% | 2 | 13 | `packages/shared/src/desktopTitleBar.ts` | `apps/web/src/lib/chatThreadActions.ts` |
| 5% | 30 | 399 | `apps/server/src/orchestration/Layers/ThreadGitMetadataReactor.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.ts` |
| 5% | 36 | 858 | `apps/web/src/components/chat/ProviderModelPicker.tsx` | `apps/web/src/components/chat/TraitsPicker.tsx` |
| 5% | 3 | 21 | `apps/web/src/components/settings/views/UsageLimitSettingsSection.tsx` | `apps/mobile/src/features/connection/GitHubRoutingSettings.tsx` |
| 5% | 101 | 434 | `apps/web/src/composerDraftAttachments.ts` | `apps/web/src/composerDraftStore.ts` |
| 5% | 4 | 91 | `apps/web/src/lib/pdf/usePdfDocument.ts` | `apps/web/src/components/ThreadRouteView.tsx` |
| 5% | 7 | 184 | `apps/server/src/agentGateway/harnessPolicy.test.ts` | `apps/server/src/provider/providerStatusCache.ts` |
| 5% | 4 | 78 | `apps/server/src/providerUsage/rateLimitResilience.ts` | `apps/server/src/orchestration/ThreadBackgroundLiveness.ts` |
| 5% | 8 | 158 | `apps/web/src/lib/newThreadDefaultsSync.test.ts` | `apps/web/src/browser/desktopTabLifetime.test.ts` |
| 5% | 7 | 42 | `apps/web/src/components/RelocateProjectDialog.tsx` | `apps/web/src/components/ServerUpdateAction.tsx` |
| 5% | 12 | 85 | `apps/web/src/components/SidebarPrimaryAction.tsx` | `apps/web/src/components/sidebar/SidebarChrome.tsx` |
| 5% | 5 | 90 | `apps/web/src/lib/pullRequestReactQuery.query.test.ts` | `apps/mobile/src/features/threads/thread-settings-sheet-state.test.ts` |
| 5% | 13 | 76 | `apps/server/src/git/Services/GitManager.ts` | `apps/server/src/git/GitWorkflowService.ts` |
| 5% | 11 | 126 | `apps/web/src/components/chat/SubagentThreadIntro.tsx` | `apps/web/src/components/AgentsPanel.tsx` |
| 5% | 2 | 85 | `apps/desktop/native/appsnap/CaptureFeedback.swift` | `apps/mobile/plugins/withIosSceneLifecycle.cjs` |
| 5% | 2 | 44 | `apps/desktop/src/rendererCrashRecovery.ts` | `apps/server/src/cloud/servicePreflight.ts` |
| 5% | 2 | 14 | `apps/server/src/hostConnectivitySignals.ts` | `apps/web/src/remoteOpen.ts` |
| 5% | 5 | 92 | `apps/server/src/outboundHttp.test.ts` | `packages/contracts/src/preview.test.ts` |
| 5% | 5 | 88 | `apps/server/src/terminal/Services/Manager.ts` | `apps/server/src/terminal/PtyAdapter.ts` |
| 5% | 2 | 49 | `apps/server/src/workspace/Layers/WorkspaceEntries.ts` | `apps/mobile/src/features/files/sourceHighlightingState.ts` |
| 5% | 12 | 164 | `apps/web/src/components/chat/AgentActivityDetailView.tsx` | `apps/web/src/components/pullRequest/PullRequestTimelineTab.tsx` |
| 5% | 3 | 24 | `apps/web/src/components/settings/agentCursorDesktopSync.test.ts` | `apps/web/src/components/chat/draftHeroTransition.test.ts` |
| 5% | 1 | 16 | `apps/web/src/hooks/useStableValue.ts` | `.github/scripts/check-nightly-release.cjs` |
| 5% | 1 | 19 | `apps/web/src/i18n/index.test.ts` | `apps/web/src/portDiscoveryState.test.ts` |
| 5% | 1 | 19 | `apps/web/src/lib/desktopProjectRecovery.ts` | `apps/web/src/commandPaletteBus.ts` |
| 5% | 3 | 16 | `scripts/lib/env-bool.ts` | `.github/scripts/check-nightly-release.cjs` |
| 5% | 15 | 152 | `apps/web/src/components/inbox/InboxRecapHistory.tsx` | `apps/web/src/components/device/DeviceToolsPanel.tsx` |
| 5% | 4 | 80 | `apps/web/src/composerDraftStore.browserAnnotations.test.ts` | `apps/web/src/terminalUiStateStore.test.ts` |
| 5% | 14 | 322 | `apps/server/src/provider/Layers/GrokAdapter.test.ts` | `apps/server/src/provider/acp/XAiAcpExtension.test.ts` |
| 5% | 4 | 26 | `packages/shared/src/providerDeliveryBlock.test.ts` | `apps/desktop/src/snapShot/HyprlandSnapShot.test.ts` |
| 5% | 2 | 51 | `apps/web/src/lib/deviceFrameSource.ts` | `apps/desktop/src/app/DesktopEarlyElectronStartup.ts` |
| 5% | 28 | 1556 | `apps/server/src/persistence/Migrations.test.ts` | `apps/server/src/persistence/Migrations/016_CanonicalizeModelSelections.test.ts` |
| 5% | 5 | 46 | `apps/web/src/components/ui/menuSubmenu.browser.tsx` | `apps/web/src/components/preview/PreviewMoreMenu.tsx` |
| 5% | 116 | 2148 | `packages/contracts/src/rpc.ts` | `packages/contracts/src/rpc.ts` |
| 5% | 5 | 47 | `apps/desktop/src/updateArtifactIdentity.test.ts` | `scripts/lib/public-config.test.ts` |
| 5% | 15 | 225 | `apps/server/src/persistence/Layers/QueuedTurnPromotions.test.ts` | `apps/server/src/persistence/Layers/OrchestrationEventStore.test.ts` |
| 5% | 4 | 76 | `apps/server/src/provider/OpenRouterDiscovery.ts` | `packages/client-runtime/src/work-log/toolPresentation.ts` |
| 5% | 2 | 12 | `apps/server/src/providerAccountHomePath.ts` | `apps/server/src/os-jank.ts` |
| 5% | 2 | 29 | `apps/web/src/hooks/useComputerDesktopControl.ts` | `apps/mobile/src/features/threads/fileChipMenu.ts` |
| 5% | 2 | 36 | `scripts/import-portable-build.ts` | `apps/desktop/scripts/build-browser-secret.mjs` |
| 5% | 3 | 82 | `apps/server/src/providerUsage/providers/droidSecureStorage.ts` | `apps/mobile/scripts/notification-center-manager.test.ts` |
| 5% | 2 | 32 | `apps/web/src/components/settings/ModelsSettingsPanel.test.ts` | `apps/web/src/browserFaviconLogic.test.ts` |
| 5% | 12 | 151 | `apps/web/src/components/settings/SkillsSettingsPanel.tsx` | `apps/web/src/components/settings/ProjectDefaultsSettings.tsx` |
| 5% | 3 | 6 | `apps/web/src/components/tasks/useOpenChat.ts` | `apps/web/src/components/ProviderUpdateLaunchNotification.tsx` |
| 5% | 17 | 244 | `apps/web/src/routes/-rootEventInvalidation.test.ts` | `apps/server/src/orchestration/messageContext.test.ts` |
| 5% | 12 | 197 | `apps/marketing/src/components/docs/media.tsx` | `apps/web/src/components/pullRequest/pullRequestPresentation.tsx` |
| 5% | 9 | 252 | `apps/profiles/src/app/[handle]/share/[format]/route.tsx` | `apps/mobile/modules/t3-markdown-text/src/NativeMarkdownBlock.tsx` |
| 5% | 4 | 78 | `apps/server/src/betaUsageSnapshot.test.ts` | `apps/server/scripts/t3-sqlite-state.test.ts` |
| 5% | 4 | 56 | `apps/server/src/workspaceEntries.chunking.test.ts` | `apps/web/src/localApi.test.ts` |
| 5% | 7 | 124 | `apps/web/src/components/chat/ChatPaneKeepAlive.browser.tsx` | `apps/web/src/components/pullRequest/PullRequestReactions.tsx` |
| 5% | 6 | 120 | `apps/web/src/lib/hosts/executionStorage.test.ts` | `apps/web/src/hooks/useLocalStorage.test.ts` |
| 5% | 14 | 359 | `apps/server/src/persistence/Layers/OrchestrationEventDeliveries.ts` | `apps/server/src/persistence/Layers/ProjectionTurns.ts` |
| 5% | 8 | 228 | `apps/web/src/lib/wsHttpUrl.test.ts` | `apps/web/src/environments/primary/bootstrap.test.ts` |
| 5% | 4 | 34 | `apps/desktop/src/githubUpdateFeed.test.ts` | `apps/server/src/git/linkCreatedPullRequest.test.ts` |
| 5% | 3 | 79 | `apps/server/src/agentGateway/Layers/AgentGatewayCredentials.test.ts` | `apps/web/src/components/KeybindingsUpdateToast.logic.test.ts` |
| 5% | 3 | 14 | `apps/server/src/computer/uiTreeText.ts` | `packages/shared/src/composerContextLegacySend.ts` |
| 5% | 6 | 94 | `apps/server/src/orchestration/pullRequestCommandDetection.ts` | `packages/shared/src/toolActivity.ts` |
| 5% | 2 | 41 | `apps/web/src/components/chat/ClaudeCacheDetails.test.tsx` | `apps/web/src/components/clerk/T3ConnectUserProfilePage.test.tsx` |
| 5% | 1 | 19 | `apps/web/src/components/chat/group/groupLifecycle.logic.ts` | `apps/mobile/src/features/threads/thread-settings-options.ts` |
| 5% | 2 | 49 | `apps/web/src/components/hosts/DeviceLinkApproval.test.tsx` | `apps/mobile/src/features/threads/RequestActionButton.tsx` |
| 5% | 1 | 27 | `apps/web/src/components/settings/AppIconPicker.test.ts` | `apps/server/src/textGeneration/TextGenerationPolicy.ts` |
| 5% | 2 | 22 | `apps/web/src/i18n/presentation.test.tsx` | `apps/web/src/components/chat/ProviderStatusBanner.test.tsx` |
| 5% | 3 | 76 | `apps/web/src/lib/rpcRequestQueue.ts` | `apps/desktop/src/snapShot/NiriCaptureShortcut.ts` |
| 5% | 1 | 20 | `apps/web/src/projectImport/ProjectImportAnnouncementDialog.tsx` | `apps/web/src/commandPaletteBus.ts` |
| 5% | 8 | 82 | `apps/web/src/recentViewsStore.ts` | `apps/web/src/browserFaviconStore.ts` |
| 5% | 1 | 20 | `packages/profile-ui/src/socialIcon.tsx` | `apps/web/src/commandPaletteBus.ts` |
| 5% | 1 | 6 | `packages/shared/src/pathContainment.ts` | `oxlint-plugin-t3code/rules/namespace-node-imports.test.ts` |
| 5% | 2 | 121 | `apps/marketing/scripts/seo-aeo.test.mjs` | `packages/shared/src/dpop.test.ts` |
| 5% | 6 | 150 | `apps/server/src/provider/usageLimit.ts` | `packages/client-runtime/src/work-log/userInput.ts` |
| 5% | 12 | 315 | `apps/profiles/src/components/ProfileCharts.tsx` | `apps/web/src/components/usage/UsageProviderChart.tsx` |
| 5% | 17 | 209 | `apps/server/src/provider/Layers/ProviderAdapterRegistry.ts` | `apps/server/integration/TestProviderAdapter.integration.ts` |
| 5% | 14 | 165 | `apps/web/src/components/settings/DevicePairing.tsx` | `apps/web/src/components/settings/ThemeImportDialog.tsx` |
| 5% | 6 | 57 | `apps/web/src/lib/automationInlineDraft.ts` | `apps/web/src/components/settings/ProviderAccentColorPicker.tsx` |
| 5% | 3 | 58 | `apps/web/src/appNavigation.test.ts` | `apps/web/src/components/RightPanelTabs.test.tsx` |
| 5% | 8 | 84 | `apps/desktop/src/windowState.ts` | `apps/web/src/components/device/deviceHubApi.ts` |
| 5% | 5 | 29 | `apps/web/src/components/chat/ThreadDetailHydrationState.tsx` | `apps/web/src/components/device/DeviceSetup.tsx` |
| 5% | 2 | 126 | `packages/contracts/src/remotePairing.ts` | `packages/contracts/src/device.ts` |
| 5% | 2 | 28 | `apps/desktop/src/computerDesktopLifecycle.ts` | `apps/web/src/browser/browserViewportActions.ts` |
| 5% | 8 | 101 | `apps/web/src/components/settings/DebouncedSettingTextInput.tsx` | `apps/web/src/components/settings/ProviderAccentColorPicker.tsx` |
| 5% | 10 | 162 | `apps/web/src/hooks/useComposerImageIntake.test.ts` | `apps/web/src/lib/attachmentUploadQueue.test.ts` |
| 5% | 7 | 53 | `apps/web/src/hooks/useUsageLimitRecoverySetting.ts` | `apps/web/src/components/pullRequest/LinkPullRequestDialog.tsx` |
| 5% | 7 | 80 | `apps/server/src/provider/FactorySessionHistory.ts` | `apps/mobile/src/features/sharing/incoming-share-model.ts` |
| 5% | 6 | 171 | `apps/web/src/components/ui/toast.browser.tsx` | `apps/web/src/browser/desktopTabLifetime.test.ts` |
| 5% | 54 | 2110 | `apps/web/src/routes/__root.tsx` | `apps/web/src/routes/__root.tsx` |
| 5% | 8 | 281 | `apps/server/src/projectAgent/Services/ProjectAgentService.ts` | `apps/server/src/orchestration/Services/ProjectionSnapshotQuery.ts` |
| 5% | 5 | 123 | `apps/web/src/hooks/useComposerImageIntake.ts` | `apps/mobile/src/features/review/useNativeReviewDiffHighlighting.ts` |
| 5% | 9 | 331 | `apps/desktop/src/betaInstaller.ts` | `scripts/lib/update-manifest.ts` |
| 5% | 6 | 213 | `packages/shared/src/deviceKey.test.ts` | `packages/shared/src/dpop.test.ts` |
| 5% | 2 | 18 | `apps/desktop/src/browserAnnotations/webviewSecurity.ts` | `apps/web/src/lib/chatThreadActions.ts` |
| 5% | 1 | 21 | `apps/marketing/src/app/docs/[[...slug]]/not-found.tsx` | `apps/web/src/commandPaletteBus.ts` |
| 5% | 1 | 15 | `apps/server/src/provider/acp/AcpWslCwd.test.ts` | `apps/web/src/cloud/dpop.test.ts` |
| 5% | 1 | 15 | `apps/server/src/wsOwnerOnly.ts` | `apps/web/src/cloud/dpop.test.ts` |
| 5% | 1 | 15 | `apps/web/src/lib/editableEventTarget.ts` | `apps/web/src/components/usage/usageShortcuts.ts` |
| 5% | 2 | 34 | `apps/web/src/railShellStore.test.ts` | `apps/web/src/components/pullRequest/pullRequestReviewStore.test.ts` |
| 5% | 3 | 38 | `packages/shared/src/todo.test.ts` | `packages/shared/src/orchestrationTiming.test.ts` |
| 5% | 17 | 325 | `apps/web/src/store.ts` | `apps/web/src/uiStateStore.ts` |
| 5% | 5 | 181 | `apps/web/src/appSnap.logic.test.ts` | `apps/web/src/terminalUiStateStore.test.ts` |
| 5% | 6 | 113 | `apps/web/src/lib/stagedDraftNavigation.test.ts` | `apps/web/src/browser/browserViewportActions.test.ts` |
| 5% | 3 | 54 | `apps/api/src/liveActivities/presentation.test.ts` | `apps/mobile/src/features/agent-awareness/notificationNavigation.test.ts` |
| 5% | 5 | 68 | `apps/server/src/git/githubReadGate.test.ts` | `apps/server/src/pullRequest/PullRequestReadCache.test.ts` |
| 5% | 3 | 18 | `apps/web/src/components/chat/InlineSkillChip.tsx` | `apps/mobile/src/features/home/HomeHeader.tsx` |
| 5% | 9 | 104 | `apps/server/src/persistence/projectionThreadMessageRow.test.ts` | `packages/contracts/src/composerContext.test.ts` |
| 5% | 4 | 13 | `apps/web/src/components/chat/ExpiredSidechatNotice.tsx` | `apps/web/src/components/preview/PreviewMoreMenu.tsx` |
| 5% | 10 | 91 | `apps/web/src/components/chat/group/coordinatorAppearance.ts` | `apps/web/src/projectIconColors.ts` |
| 5% | 9 | 107 | `apps/web/src/components/ui/sidebar.browser.tsx` | `apps/web/src/components/AppSidebarLayout.tsx` |
| 5% | 23 | 215 | `apps/web/src/file-icons.ts` | `apps/mobile/modules/t3-markdown-text/src/markdownLinks.ts` |
| 5% | 5 | 77 | `apps/web/src/projectImport/logic.test.ts` | `apps/web/src/components/chat/ContextWindowMeter.logic.test.ts` |
| 5% | 6 | 128 | `packages/shared/src/deviceKey.ts` | `apps/web/src/cloud/dpop.ts` |
| 5% | 6 | 129 | `scripts/mac-update-zip.test.ts` | `scripts/merge-update-manifests.test.ts` |
| 5% | 28 | 437 | `apps/server/src/composerPrediction/Layers/ComposerPredictionService.test.ts` | `apps/server/src/orchestration/ThreadPullRequestReactor.test.ts` |
| 5% | 3 | 30 | `apps/server/src/remoteSessions/httpRoute.ts` | `scripts/mock-update-server.ts` |
| 5% | 3 | 29 | `apps/web/src/components/chat/environment/EnvironmentPanel.logic.test.ts` | `apps/web/src/browserFaviconStore.test.ts` |
| 5% | 3 | 22 | `apps/web/src/hooks/useAdditionalProjectFolders.ts` | `apps/web/src/cloud/useCloudLinkController.ts` |
| 5% | 3 | 16 | `apps/web/src/lib/nativeMenuIcons.browser.ts` | `apps/web/src/components/threadActionMenu.logic.ts` |
| 5% | 3 | 31 | `apps/web/src/projectRunStore.ts` | `apps/web/src/previewMiniPlayerStore.ts` |
| 5% | 4 | 93 | `apps/server/scripts/acp-conformance-agent.ts` | `packages/effect-acp/test/fixtures/acp-mock-peer.ts` |
| 5% | 5 | 45 | `apps/server/src/orchestration/commandFingerprint.ts` | `apps/mobile/src/native/StackHeader.tsx` |
| 5% | 6 | 81 | `apps/web/src/chatRouteRestore.test.ts` | `apps/web/src/components/BranchToolbar.logic.test.ts` |
| 5% | 4 | 63 | `apps/web/src/components/AnnouncementSheet.browser.tsx` | `apps/web/src/components/chat/AssistantCitationChip.test.tsx` |
| 5% | 6 | 47 | `apps/web/src/components/chat/explorerListNavigation.ts` | `apps/web/src/components/ui/color-picker.tsx` |
| 5% | 5 | 66 | `apps/web/src/lib/threadSnooze.ts` | `apps/web/src/cloud/useCloudLinkController.ts` |
| 5% | 18 | 150 | `apps/web/src/projectImport/ProjectImportProjectCard.tsx` | `apps/web/src/components/device/DeviceToolsPanel.tsx` |
| 5% | 4 | 170 | `packages/shared/src/migrationRecovery.test.ts` | `apps/mobile/src/lib/composerContext.test.ts` |
| 5% | 5 | 105 | `scripts/desktop-bundle-files.test.ts` | `apps/mobile/src/features/threads/thread-settings-sheet-state.test.ts` |
| 5% | 8 | 47 | `apps/profiles/src/components/ThemeToggle.tsx` | `apps/web/src/components/ui/menu.tsx` |
| 5% | 6 | 76 | `apps/server/src/remotePairing/client.ts` | `apps/desktop/src/snapShot/NiriSnapShot.ts` |
| 5% | 29 | 119 | `apps/web/src/components/pullRequest/PullRequestConfirmActionDialog.tsx` | `apps/web/src/components/settings/IntegrationsSettings.tsx` |
| 5% | 8 | 250 | `apps/desktop/src/browserAutomation/betterwrightHostTarget.test.ts` | `apps/mobile/src/native/voiceTranscription.ios.test.ts` |
| 5% | 2 | 35 | `apps/marketing/src/lib/installerCount.ts` | `apps/web/src/components/device/deviceFold.ts` |
| 5% | 1 | 16 | `apps/server/src/computer/computerShield.ts` | `apps/mobile/src/features/connection/environmentSections.ts` |
| 5% | 20 | 152 | `apps/server/src/orchestration/decider.projectRelocation.test.ts` | `apps/server/src/orchestration/projector.test.ts` |
| 5% | 1 | 16 | `apps/server/src/tailscaleEndpoint.test.ts` | `apps/web/src/cloud/dpop.test.ts` |
| 5% | 4 | 90 | `apps/web/src/components/chat/useTimelineRowOverlapGuard.browser.tsx` | `apps/mobile/src/components/MaterialScrollComposeButton.android.tsx` |
| 5% | 12 | 110 | `apps/web/src/projectImport/ImportedHistoryButton.tsx` | `apps/web/src/components/settings/SnapShotSettings.tsx` |
| 5% | 2 | 47 | `scripts/server-ci-shards.test.ts` | `apps/desktop/gnome-extension/captureService.js` |
| 5% | 5 | 52 | `apps/web/src/components/LocalServerIdentity.tsx` | `apps/web/src/browser/BrowserDeviceToolbar.tsx` |
| 5% | 49 | 427 | `apps/web/src/components/settings/McpConnectorsSettingsPanel.tsx` | `apps/web/src/components/GitActionsControl.tsx` |
| 5% | 6 | 165 | `apps/web/src/computerPreviewStore.ts` | `apps/web/src/previewMiniPlayerStore.ts` |
| 5% | 8 | 108 | `apps/web/src/workflowRunUiStore.ts` | `apps/web/src/terminalUiStateStore.ts` |
| 5% | 10 | 380 | `packages/contracts/src/automation.ts` | `apps/server/src/persistence/Services/ProjectionThreads.ts` |
| 4% | 5 | 110 | `apps/web/src/components/DiffPanel.browser.tsx` | `apps/web/src/components/chat/AssistantCitationChip.test.tsx` |
| 4% | 28 | 461 | `apps/web/src/components/DiffPanelToolbar.tsx` | `apps/web/src/components/preview/PreviewMoreMenu.tsx` |
| 4% | 7 | 57 | `apps/web/src/openThreadTabsStore.ts` | `apps/web/src/browserFaviconStore.ts` |
| 4% | 42 | 827 | `apps/server/src/main.ts` | `apps/server/src/cli/config.ts` |
| 4% | 9 | 178 | `apps/server/src/provider/acp/AcpNativeLogging.test.ts` | `apps/server/src/provider/acp/AcpNativeLogging.test.ts` |
| 4% | 14 | 115 | `apps/web/src/components/chat/PickerPanelShell.tsx` | `apps/web/src/components/settings/SettingsSidebarNav.tsx` |
| 4% | 10 | 84 | `apps/web/src/components/chat/ProviderHandoffDivider.tsx` | `apps/web/src/components/AgentsPanel.tsx` |
| 4% | 2 | 27 | `apps/web/src/hooks/useComputerPreviewBudgetPx.browser.tsx` | `apps/web/src/components/settings/NotificationSettings.tsx` |
| 4% | 2 | 27 | `apps/web/src/lib/deletedThreadClientReconciliation.test.ts` | `apps/web/src/components/composerContextPresentation.test.ts` |
| 4% | 2 | 24 | `packages/contracts/src/browserAutomationJson.ts` | `apps/mobile/src/features/shortcuts/appShortcuts.ts` |
| 4% | 3 | 27 | `apps/web/src/components/chat/environment/EnvironmentNotesSection.tsx` | `apps/web/src/components/diffs/DiffCommentAnnotation.tsx` |
| 4% | 5 | 48 | `apps/web/src/components/pullRequest/githubItemAgentContext.ts` | `apps/web/src/components/BranchToolbar.logic.ts` |
| 4% | 7 | 428 | `apps/web/src/providerModelOptions.test.ts` | `apps/web/src/lib/openPullRequestLink.test.ts` |
| 4% | 2 | 153 | `packages/contracts/src/inlineVisualSpec.ts` | `packages/contracts/src/device.ts` |
| 4% | 50 | 936 | `packages/contracts/src/ws.ts` | `packages/contracts/src/rpc.ts` |
| 4% | 8 | 129 | `scripts/lib/mac-update-zip.ts` | `scripts/lib/update-manifest.ts` |
| 4% | 6 | 157 | `apps/server/src/config.permissions.test.ts` | `packages/shared/src/nodeRuntime.test.ts` |
| 4% | 11 | 335 | `apps/web/src/components/computer/useComputerPreviewTap.test.ts` | `apps/web/src/components/ProviderUpdateEnvironmentRows.test.tsx` |
| 4% | 5 | 467 | `apps/web/src/lib/automationIntent.test.ts` | `apps/mobile/src/features/threads/thread-feed-live-follow.test.ts` |
| 4% | 2 | 61 | `apps/api/src/identity/devProvider.test.ts` | `apps/web/src/cloud/publicConfig.test.ts` |
| 4% | 3 | 33 | `apps/desktop/src/voiceTranscription.test.ts` | `apps/web/src/components/preview/openPreviewSession.test.ts` |
| 4% | 3 | 12 | `apps/server/src/codexWorkingDirectory.ts` | `apps/mobile/src/features/sharing/incoming-share-inbox.ts` |
| 4% | 1 | 40 | `apps/web/playwright.editor.config.ts` | `apps/marketing/astro.config.mjs` |
| 4% | 9 | 44 | `apps/web/src/components/chat/DockPaneHeader.tsx` | `apps/web/src/components/pullRequest/PullRequestGhosts.tsx` |
| 4% | 3 | 48 | `apps/web/src/components/chat/GeneratedMarkdownVideo.tsx` | `apps/web/src/components/pullRequest/PullRequestMarkdownEditor.tsx` |
| 4% | 9 | 104 | `apps/web/src/lib/projectCreation.ts` | `apps/server/src/cli/project.ts` |
| 4% | 3 | 18 | `apps/web/src/lib/sidechatCreatorRegistry.test.ts` | `apps/desktop/src/electron/WindowsForegroundFocusThread.test.ts` |
| 4% | 5 | 54 | `benchmarks/desktop-perf/lib/headlessBackend.mjs` | `apps/desktop/src/snapShot/SnapShotAccessibilityProcess.ts` |
| 4% | 5 | 65 | `packages/shared/src/computerFrame.ts` | `apps/mobile/src/lib/composerContextClipboard.ts` |
| 4% | 5 | 65 | `packages/shared/src/deviceFrame.ts` | `apps/mobile/src/lib/composerContextClipboard.ts` |
| 4% | 1 | 4 | `packages/shared/src/nativeLocalizationResources.ts` | `apps/mobile/src/features/devices/device-stream-document.ts` |
| 4% | 1 | 9 | `packages/shared/src/pathContainment.test.ts` | `oxlint-plugin-t3code/rules/namespace-node-imports.test.ts` |
| 4% | 4 | 64 | `apps/server/src/migrationBundleIdentity.test.ts` | `packages/shared/src/logging.test.ts` |
| 4% | 5 | 34 | `apps/server/src/serverRuntimeState.test.ts` | `apps/server/src/cli/pair.test.ts` |
| 4% | 5 | 38 | `apps/web/src/components/ProviderUsageLineList.tsx` | `apps/web/src/components/sidebar/SidebarThreadHeader.tsx` |
| 4% | 3 | 53 | `apps/desktop/src/browserVaultIpc.ts` | `apps/desktop/src/permissions/MacSettingsWindow.ts` |
| 4% | 8 | 48 | `apps/web/src/components/RouteSurface.tsx` | `apps/web/src/components/pullRequest/PullRequestTimelineTab.tsx` |
| 4% | 6 | 51 | `apps/web/src/components/chat/TurnHeaderLine.tsx` | `apps/web/src/components/chat/ComposerPendingUserInputPanel.tsx` |
| 4% | 23 | 355 | `apps/web/src/components/settings/ConnectionsThisComputer.tsx` | `apps/web/src/components/settings/SnapShotSettings.tsx` |
| 4% | 8 | 51 | `apps/web/src/onboarding/OnboardingAsciiBackdrop.tsx` | `apps/web/src/components/cloud/CloudEnvironmentConnectList.tsx` |
| 4% | 6 | 222 | `packages/contracts/src/computer.test.ts` | `packages/contracts/src/preview.test.ts` |
| 4% | 6 | 106 | `apps/desktop/src/hostUsageRelay.ts` | `apps/mobile/src/features/sharing/incoming-share-model.ts` |
| 4% | 3 | 54 | `apps/server/src/desktopUpdateBridge.test.ts` | `apps/web/src/components/clerk/electronPasskeys.test.ts` |
| 4% | 24 | 346 | `apps/server/src/keepAwake.test.ts` | `apps/server/src/orchestration/ThreadPullRequestReactor.test.ts` |
| 4% | 31 | 339 | `apps/server/src/orchestration/importThreadRoute.test.ts` | `apps/server/src/project/AgentSessionImporter.test.ts` |
| 4% | 8 | 227 | `apps/server/src/persistence/Layers/HubWorkRepository.requestIdentity.test.ts` | `apps/server/src/persistence/Layers/OrchestrationEventStore.test.ts` |
| 4% | 15 | 221 | `apps/web/src/components/ProjectAppearancePicker.tsx` | `apps/web/src/components/sidebar/SidebarThreadHeader.tsx` |
| 4% | 4 | 74 | `apps/web/src/lib/browserDownload.ts` | `apps/web/src/components/device/DeviceWorkspace.tsx` |
| 4% | 6 | 128 | `apps/web/src/lib/composerDropPaths.test.ts` | `apps/web/src/connection/storage.test.ts` |
| 4% | 4 | 82 | `apps/web/src/lib/composerImageSource.test.ts` | `apps/mobile/src/lib/layout.test.ts` |
| 4% | 3 | 36 | `apps/web/src/lib/openUsageReactQuery.ts` | `apps/web/src/browserFaviconLogic.ts` |
| 4% | 4 | 215 | `apps/web/src/components/chat/streamingMarkdownBlocks.test.ts` | `apps/web/src/components/chat/ProviderStatusBanner.test.tsx` |
| 4% | 6 | 81 | `apps/desktop/src/clipboardPermissions.test.ts` | `apps/web/src/lib/openPullRequestLink.test.ts` |
| 4% | 3 | 106 | `apps/server/src/persistence/modelSelectionCompatibility.test.ts` | `apps/server/src/provider/Drivers/ClaudeSkills.test.ts` |
| 4% | 94 | 3144 | `apps/server/src/provider/Layers/PiAdapter.ts` | `apps/server/src/provider/Layers/CursorAdapter.ts` |
| 4% | 21 | 269 | `apps/web/src/components/settings/ConnectionsRows.tsx` | `apps/web/src/components/settings/KeybindingsSettings.tsx` |
| 4% | 4 | 19 | `apps/web/src/hooks/useDiffRouteSearch.ts` | `packages/client-runtime/src/state/threadShell.ts` |
| 4% | 8 | 136 | `apps/web/src/kanbanUiStore.ts` | `apps/web/src/uiStateStore.ts` |
| 4% | 2 | 41 | `apps/api/src/clientIp.ts` | `apps/desktop/src/app/DesktopConfig.ts` |
| 4% | 2 | 33 | `apps/server/src/externalMcp/launcher.test.ts` | `apps/web/src/cloud/connectCliAuth.test.ts` |
| 4% | 10 | 479 | `apps/server/src/recapStats.test.ts` | `apps/server/src/persistence/Migrations/016_CanonicalizeModelSelections.test.ts` |
| 4% | 4 | 33 | `apps/web/src/components/chat/ForkSourceDivider.tsx` | `apps/web/src/components/media/MediaActions.tsx` |
| 4% | 4 | 45 | `apps/web/src/components/terminal/terminalSelectionActions.ts` | `apps/web/src/lib/selectionActions.ts` |
| 4% | 2 | 25 | `apps/web/src/hooks/useDeviceEventBridge.ts` | `apps/mobile/src/components/FilePreview.tsx` |
| 4% | 9 | 179 | `apps/web/src/components/ComputerPanel.logic.ts` | `apps/web/src/components/ProviderUpdateLaunchNotification.logic.ts` |
| 4% | 6 | 89 | `apps/web/src/hooks/useComputerControlModeChange.test.tsx` | `apps/web/src/hooks/useTheme.test.ts` |
| 4% | 3 | 54 | `apps/web/src/components/chat/BrowserAnnotationStrip.browser.tsx` | `apps/web/src/lib/elementContext.test.ts` |
| 4% | 3 | 128 | `apps/web/src/components/chat/ComputerSetupRequiredCard.test.tsx` | `apps/web/src/components/ThreadStatusIndicators.test.tsx` |
| 4% | 11 | 305 | `apps/web/src/components/chat/useComposerVoiceController.test.ts` | `apps/web/src/components/ProviderUpdateEnvironmentRows.test.tsx` |
| 4% | 14 | 110 | `apps/web/src/whatsNew/WhatsNewPopoutCard.tsx` | `apps/web/src/components/device/DeviceStreamView.tsx` |
| 4% | 4 | 60 | `packages/shared/src/automationRunLifecycle.ts` | `packages/contracts/src/worktreeSetup.ts` |
| 4% | 4 | 33 | `packages/shared/src/desktopChrome.ts` | `apps/web/src/browser/recordingCompositor.ts` |
| 4% | 7 | 68 | `apps/desktop/src/updateArtifactIdentity.ts` | `apps/server/src/usage/usageTranscriptReader.ts` |
| 4% | 22 | 135 | `apps/server/perf/engineStreamingThroughput.perf.test.ts` | `apps/server/src/project/AgentSessionImporter.test.ts` |
| 4% | 2 | 44 | `apps/web/perf/workload.ts` | `apps/mobile/src/features/diagnostics/crash-log-model.ts` |
| 4% | 5 | 58 | `apps/web/src/components/ProjectScriptsControl.browser.tsx` | `apps/web/src/projectScripts.test.ts` |
| 4% | 2 | 38 | `apps/web/src/components/chat/transcriptScroll.ts` | `apps/mobile/src/state/thread-outbox-removal.ts` |
| 4% | 3 | 19 | `apps/server/src/externalMcp/credentialVerification.ts` | `apps/server/src/diagnostics/ProcessDiagnostics.ts` |
| 4% | 5 | 46 | `apps/server/src/windowsProcessEffect.test.ts` | `packages/ssh/src/runnerProcess.test.ts` |
| 4% | 20 | 380 | `apps/web/src/components/AppSnapCoordinator.tsx` | `apps/web/src/components/desktop/SnapShotCoordinator.tsx` |
| 4% | 12 | 114 | `apps/web/src/components/pdf/PdfPageView.tsx` | `apps/web/src/components/device/DeviceStreamView.tsx` |
| 4% | 18 | 249 | `apps/web/src/components/settings/accounts/ProviderAccountRuntimeFields.tsx` | `apps/web/src/components/pullRequest/PullRequestTimelineTab.tsx` |
| 4% | 3 | 93 | `apps/web/src/lib/composerMentions.test.ts` | `apps/desktop/src/preview/PreviewKeyboard.test.ts` |
| 4% | 7 | 116 | `packages/contracts/src/inboxRecaps.ts` | `packages/shared/src/usageFormat.ts` |
| 4% | 6 | 68 | `apps/desktop/src/desktopCustomTitleBar.test.ts` | `scripts/lib/public-config.test.ts` |
| 4% | 9 | 85 | `apps/server/src/orchestration/projectImportSources.ts` | `apps/web/src/components/settings/SettingsPanels.logic.ts` |
| 4% | 2 | 52 | `scripts/check-server-sync-fs.test.ts` | `apps/web/src/state/terminalSessions.test.ts` |
| 4% | 4 | 81 | `apps/web/src/components/chat/useOpenSubagentThread.ts` | `apps/web/src/components/ThreadRouteView.tsx` |
| 4% | 1 | 6 | `apps/server/src/accountUsageReporterRegistry.ts` | `apps/mobile/src/features/devices/device-stream-document.ts` |
| 4% | 6 | 80 | `apps/server/src/computer/computerFrameRoute.ts` | `apps/server/src/device/DeviceHubProxy.ts` |
| 4% | 6 | 80 | `apps/server/src/device/deviceFrameRoute.ts` | `apps/server/src/device/DeviceHubProxy.ts` |
| 4% | 4 | 36 | `apps/server/src/effectServer.remoteAccess.test.ts` | `apps/server/src/auth/EnvironmentAuthPolicy.test.ts` |
| 4% | 13 | 35 | `apps/web/src/components/chat/TerminalContextInlineChip.tsx` | `apps/web/src/components/pullRequest/pullRequestPresentation.tsx` |
| 4% | 4 | 63 | `apps/web/src/components/chat/projectScriptRuntime.ts` | `apps/desktop/scripts/dev-electron.mjs` |
| 4% | 7 | 33 | `apps/web/src/groupPanelClosedStore.ts` | `apps/web/src/browserHistoryStore.ts` |
| 4% | 2 | 34 | `apps/web/src/hooks/useComposerImageIntake.browser.tsx` | `apps/web/src/assets/projectFaviconCache.ts` |
| 4% | 1 | 6 | `apps/web/src/lib/binaryFrameSource.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 4% | 3 | 51 | `apps/web/src/lib/pdf/usePdfZoomController.ts` | `apps/mobile/src/features/review/useNativeReviewDiffHighlighting.ts` |
| 4% | 2 | 34 | `apps/web/src/notifications/snoozeReminderReceipts.test.ts` | `apps/web/src/lib/terminalContext.test.ts` |
| 4% | 5 | 125 | `apps/web/src/onboarding/useOnboarding.ts` | `apps/web/src/components/cloud/ConnectOnboardingDialog.tsx` |
| 4% | 2 | 15 | `packages/contracts/src/studio.ts` | `packages/contracts/src/worktreeSetup.ts` |
| 4% | 11 | 565 | `apps/web/src/components/DevicePanel.logic.test.ts` | `apps/web/src/lib/snapShotShortcut.test.ts` |
| 4% | 108 | 1516 | `apps/web/src/storeEventReducer.ts` | `apps/server/src/orchestration/projector.ts` |
| 4% | 3 | 71 | `apps/server/src/computer/computerPreviewEviction.test.ts` | `apps/desktop/src/snapShot/HyprlandSnapShot.ts` |
| 4% | 40 | 452 | `apps/server/src/git/Layers/ClaudeTextGeneration.test.ts` | `apps/server/src/provider/Layers/ProviderRegistry.test.ts` |
| 4% | 9 | 237 | `apps/web/src/components/chat/ExpandedImageOverlay.browser.tsx` | `apps/web/src/components/pullRequest/PullRequestThreadLinks.tsx` |
| 4% | 20 | 399 | `apps/web/src/components/chat/InlineVisualBlocks.tsx` | `apps/web/src/components/ui/toast.tsx` |
| 4% | 14 | 215 | `apps/web/src/lib/hosts/workspaceSidebar.test.ts` | `apps/server/src/mcp/toolkits/pullRequests/handlers.test.ts` |
| 4% | 19 | 294 | `apps/web/src/notifications/taskCompletion.tsx` | `apps/web/src/components/ThreadNotificationCoordinator.tsx` |
| 4% | 5 | 99 | `apps/marketing/src/app/page.tsx` | `apps/web/src/components/usage/usageProviders.ts` |
| 4% | 14 | 134 | `apps/server/src/git/gitActionRunner.ts` | `packages/client-runtime/src/state/vcsAction.ts` |
| 4% | 10 | 268 | `packages/shared/src/markdownPlainText.ts` | `packages/client-runtime/src/work-log/commandLabel.ts` |
| 4% | 19 | 222 | `apps/web/src/components/pullRequest/PullRequestActions.tsx` | `apps/web/src/components/device/DeviceControlsRail.tsx` |
| 4% | 3 | 50 | `packages/shared/src/terminalThreads.test.ts` | `apps/mobile/src/lib/layout.test.ts` |
| 4% | 2 | 44 | `apps/marketing/src/lib/feedbackRateLimit.ts` | `apps/web/src/browserFaviconLogic.ts` |
| 4% | 4 | 81 | `apps/web/src/components/EditorWorkspaceView.browser.tsx` | `apps/mobile/src/features/review/shikiReviewHighlighter.test.ts` |
| 4% | 2 | 26 | `apps/web/src/lib/deletedThreadClientReconciliation.ts` | `apps/web/src/lib/terminalContext.ts` |
| 4% | 1 | 36 | `packages/shared/src/text.test.ts` | `apps/web/src/cloud/connectCliAuth.test.ts` |
| 4% | 3 | 51 | `apps/web/src/components/settings/skillsSettingsModel.test.ts` | `apps/mobile/src/state/workspaceModel.test.ts` |
| 4% | 4 | 33 | `apps/web/src/onboarding/useProviderDetection.ts` | `apps/mobile/src/features/settings/SettingsAboutRouteScreen.tsx` |
| 4% | 3 | 17 | `packages/shared/src/installerDownloads.test.ts` | `apps/desktop/src/ipc/methods/preview.test.ts` |
| 4% | 3 | 51 | `apps/server/src/provider/cliVersion.test.ts` | `apps/desktop/src/preview/PreviewKeyboard.test.ts` |
| 4% | 3 | 55 | `apps/web/src/lib/liveActivityPresentation.test.ts` | `apps/desktop/src/preview/PickedElementPayload.test.ts` |
| 4% | 16 | 574 | `apps/server/src/persistence/Layers/ProviderRuntimeEvents.ts` | `apps/server/src/persistence/Layers/OrchestrationEventStore.ts` |
| 4% | 5 | 112 | `apps/web/src/deviceStateStore.test.ts` | `packages/shared/src/agentAwareness.test.ts` |
| 4% | 7 | 41 | `apps/web/src/projectTerminalRunner.test.ts` | `packages/contracts/src/terminal.test.ts` |
| 4% | 2 | 17 | `apps/server/native/device-helper/Sources/Screenshot.swift` | `apps/mobile/plugins/withIosSceneLifecycle.cjs` |
| 4% | 1 | 5 | `apps/server/src/automation/Errors.ts` | `oxlint-plugin-t3code/rules/no-inline-schema-compile.test.ts` |
| 4% | 2 | 6 | `apps/server/src/jwtClaims.ts` | `packages/client-runtime/src/work-log/toolPresentation.ts` |
| 4% | 1 | 7 | `apps/server/src/orchestration/Layers/ProviderRuntimeIngestion.buffer.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 4% | 13 | 259 | `apps/server/src/provider/providerAttachmentPaths.test.ts` | `apps/server/src/orchestration/Normalizer.attachments.test.ts` |
| 4% | 1 | 7 | `apps/server/src/providerUsage/registry.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 4% | 1 | 5 | `apps/server/src/todo/Errors.ts` | `oxlint-plugin-t3code/rules/no-inline-schema-compile.test.ts` |
| 4% | 2 | 27 | `apps/web/src/components/SponsorLandingBanner.browser.tsx` | `apps/mobile/src/features/diffs/nativeReviewDiffSurface.test.ts` |
| 4% | 4 | 174 | `apps/web/src/components/chat/ComposerModelPicker.logic.test.ts` | `apps/mobile/src/features/threads/thread-settings-sheet-state.test.ts` |
| 4% | 1 | 26 | `apps/web/src/components/chat/DockFilePane.tsx` | `apps/web/src/commandPaletteBus.ts` |
| 4% | 1 | 15 | `apps/web/src/components/chat/composerPickerSize.ts` | `apps/web/src/cloud/connectCliAuth.ts` |
| 4% | 3 | 63 | `apps/web/src/components/settings/UsageLimitRecoverySetting.browser.tsx` | `apps/web/src/components/permissions/usePermissionStatus.test.ts` |
| 4% | 1 | 16 | `apps/web/src/components/tasks/useTaskProjects.ts` | `apps/mobile/src/state/device.ts` |
| 4% | 3 | 41 | `apps/web/src/lib/codexResetAttempt.test.ts` | `apps/web/src/components/chat/timelineScrollTarget.test.ts` |
| 4% | 2 | 36 | `apps/web/src/lib/sidechatCreatorRegistry.ts` | `apps/mobile/src/state/entities.ts` |
| 4% | 1 | 7 | `apps/web/src/truncateTitle.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 4% | 2 | 5 | `packages/shared/src/searchQuery.ts` | `packages/shared/src/thirdPartyLicenses.ts` |
| 4% | 8 | 287 | `apps/server/src/orchestration/Layers/TurnCheckpointCoordinator.test.ts` | `apps/server/src/provider/Layers/EventNdjsonLogger.test.ts` |
| 4% | 49 | 629 | `apps/web/src/components/DevicePanel.tsx` | `apps/web/src/components/pullRequest/PullRequestDetailPanel.tsx` |
| 4% | 8 | 45 | `apps/web/src/components/settings/providers/ProviderDocsLinks.tsx` | `apps/web/src/components/PullRequestThreadDialog.tsx` |
| 4% | 6 | 95 | `apps/desktop/src/safariAccessIpc.ts` | `apps/desktop/src/snapShot/NiriCaptureShortcut.ts` |
| 4% | 12 | 286 | `apps/server/src/githubInbox/githubInboxSnapshotStore.ts` | `apps/server/src/pullRequest/GitHubPullRequestProvider.ts` |
| 4% | 42 | 220 | `apps/server/src/orchestration/Layers/AsyncUserInput.test.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.test.ts` |
| 4% | 3 | 43 | `apps/server/src/siteFaviconCache.test.ts` | `apps/web/src/components/preview/openPreviewSession.test.ts` |
| 4% | 3 | 87 | `apps/web/src/lib/pdf/pdfLinks.test.ts` | `apps/web/src/browser/hostedBrowserWebviewStyle.ts` |
| 4% | 8 | 129 | `apps/web/src/onboarding/stage/ProviderRing.tsx` | `apps/web/src/components/settings/ThemePreviewCircles.tsx` |
| 4% | 9 | 213 | `apps/web/src/components/ComputerPanel.logic.test.ts` | `apps/server/src/relay/AgentAwarenessRelay.test.ts` |
| 4% | 4 | 83 | `apps/server/src/accountSsoCallback.ts` | `packages/shared/src/Net.ts` |
| 4% | 15 | 331 | `apps/server/src/serverLayers.ts` | `apps/server/integration/OrchestrationEngineHarness.integration.ts` |
| 4% | 6 | 74 | `apps/web/src/components/chat/InlineMentionChip.tsx` | `apps/web/src/components/pullRequest/PullRequestRow.tsx` |
| 4% | 9 | 70 | `apps/web/src/components/settings/PairingQrCode.tsx` | `apps/web/src/components/usage/UsageProviderChart.tsx` |
| 4% | 12 | 153 | `apps/web/src/hooks/useThreadPullRequests.test.ts` | `apps/web/src/components/ThreadStatusIndicators.test.ts` |
| 4% | 3 | 49 | `apps/web/src/components/WorkspaceSearchPalette.logic.test.ts` | `apps/mobile/src/lib/markdownLinks.test.ts` |
| 4% | 36 | 392 | `apps/server/src/pullRequestAutoFix/Layers/PullRequestAutoFixService.test.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.test.ts` |
| 4% | 7 | 88 | `apps/web/src/repoDiffScopeStore.ts` | `apps/web/src/browserFaviconStore.ts` |
| 4% | 1 | 8 | `apps/desktop/src/migrationConsentHandoff.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 4% | 1 | 8 | `apps/server/src/provider/openCodeServerPassword.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 4% | 2 | 43 | `apps/server/src/provider/terminalTurnApplicability.ts` | `apps/mobile/src/state/pending-thread-creation.ts` |
| 4% | 1 | 21 | `apps/server/src/terminal/runtimeLayer.ts` | `apps/web/src/cloud/dpop.test.ts` |
| 4% | 1 | 21 | `apps/web/src/components/chat/project/hubWorkItems.ts` | `apps/mobile/src/components/headerMenu.android.ts` |
| 4% | 4 | 152 | `apps/web/src/hooks/useProfileThemeAccentSync.browser.tsx` | `apps/web/src/components/device/useDeviceControls.test.tsx` |
| 4% | 1 | 8 | `apps/web/src/lib/externalUrl.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 4% | 2 | 13 | `apps/web/src/lib/remarkHtmlBreaks.ts` | `apps/web/src/markdown-list-indentation.ts` |
| 4% | 1 | 8 | `packages/shared/src/groupSlug.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 4% | 0 | 9 | `vitest.config.ts` | `vite.config.ts` |
| 4% | 9 | 71 | `apps/server/src/orchestration/Layers/usageActivityBounds.test.ts` | `apps/server/src/persistence/Layers/OrchestrationEventStore.test.ts` |
| 4% | 21 | 183 | `apps/web/src/components/chat/SelectionNewChatComposer.tsx` | `apps/web/src/components/settings/ThemeEditorPanel.tsx` |
| 4% | 12 | 183 | `apps/web/src/components/tasks/TaskAgentPanel.tsx` | `apps/web/src/components/AgentsPanel.tsx` |
| 4% | 20 | 1548 | `apps/server/src/provider/opencodeRuntime.test.ts` | `apps/desktop/src/backend/DesktopBackendManager.test.ts` |
| 4% | 3 | 93 | `apps/web/src/components/chat/SubagentsDockPane.test.tsx` | `apps/web/src/components/chat/ProviderStatusBanner.test.tsx` |
| 4% | 2 | 20 | `apps/marketing/src/app/changelog/page.tsx` | `apps/web/src/components/cloud/ConnectCliAuthSurface.tsx` |
| 4% | 10 | 228 | `apps/web/src/components/chat/ToolCallDetailsDialog.tsx` | `apps/web/src/components/pullRequest/PullRequestGhosts.tsx` |
| 4% | 2 | 25 | `apps/web/src/components/ui/notificationSurface.ts` | `apps/web/src/components/DiffPanelShell.tsx` |
| 4% | 2 | 31 | `apps/web/src/components/ui/toastRouteVisibility.ts` | `apps/web/src/lib/chatThreadActions.ts` |
| 4% | 2 | 9 | `apps/web/src/lib/externalUrl.ts` | `packages/client-runtime/src/work-log/toolPresentation.ts` |
| 4% | 2 | 26 | `apps/web/src/splitViewRoute.ts` | `apps/desktop/src/app/DesktopEarlyElectronStartup.ts` |
| 4% | 5 | 99 | `apps/web/src/workspacePathsStore.test.ts` | `apps/web/src/clientPersistenceStorage.test.ts` |
| 4% | 3 | 48 | `apps/api/src/liveActivities/presentation.ts` | `apps/mobile/src/features/showcase/showcaseAgentActivity.ts` |
| 4% | 5 | 74 | `apps/e2e/src/harness/workspace.ts` | `apps/desktop/scripts/wait-for-resources.mjs` |
| 4% | 35 | 490 | `apps/server/src/provider/Layers/ProviderRuntimeReconciler.test.ts` | `apps/server/src/relay/AgentAwarenessRelay.test.ts` |
| 4% | 7 | 94 | `apps/server/src/provider/OmpSessionHistory.ts` | `apps/mobile/src/features/sharing/incoming-share-model.ts` |
| 4% | 3 | 60 | `apps/web/src/lib/desktopWindowTranslucency.test.ts` | `apps/web/src/components/chat/ExpandedImagePreview.test.ts` |
| 4% | 3 | 7 | `packages/shared/src/workspaceFileWrite.ts` | `packages/client-runtime/src/state/attachments.ts` |
| 4% | 4 | 96 | `apps/server/src/computer/computerEmergencyStop.test.ts` | `apps/mobile/src/features/threads/checkout-new-task-branch.test.ts` |
| 4% | 14 | 55 | `apps/web/src/components/AppNavigationButtons.tsx` | `apps/web/src/components/DiffPanel.tsx` |
| 4% | 14 | 176 | `apps/web/src/components/chat/ProviderModelOptionGroupList.tsx` | `apps/web/src/components/chat/TraitsPicker.tsx` |
| 4% | 4 | 93 | `apps/web/src/lib/hosts/enrollment.test.ts` | `apps/mobile/src/state/use-thread-pr.test.ts` |
| 4% | 6 | 89 | `packages/shared/src/remoteResources.ts` | `apps/mobile/src/lib/composerContextClipboard.ts` |
| 4% | 6 | 228 | `apps/server/src/persistence/Layers/Sqlite.test.ts` | `apps/desktop/src/preview/BrowserImport/ChromiumCookies.test.ts` |
| 4% | 1 | 10 | `apps/desktop/src/shutdownSignals.ts` | `apps/web/src/components/files/fileTreeExpansion.ts` |
| 4% | 31 | 650 | `apps/server/src/agentGateway/kanbanTools.test.ts` | `apps/server/src/relay/AgentAwarenessRelay.test.ts` |
| 4% | 4 | 28 | `apps/server/src/backendCompileCache.ts` | `apps/desktop/src/electron/WindowsForegroundFocusWorker.ts` |
| 4% | 1 | 9 | `apps/server/src/computer/modelDesktopObservation.ts` | `apps/desktop/gnome-extension/captureService.js` |
| 4% | 3 | 40 | `apps/server/src/providerUsage/providers/localCredential.ts` | `apps/web/src/components/settings/providerStatus.ts` |
| 4% | 1 | 14 | `apps/web/scripts/generate-settings-search-rows.ts` | `oxlint-plugin-t3code/rules/namespace-node-imports.test.ts` |
| 4% | 2 | 28 | `apps/web/src/components/PluginLibrary.test.tsx` | `apps/web/src/components/settings/settingsLayout.test.tsx` |
| 4% | 1 | 10 | `apps/web/src/components/chat/project/groupPanelSections.ts` | `apps/mobile/src/state/environments.ts` |
| 4% | 2 | 25 | `apps/web/src/components/codeEditor/pierreEdit.ts` | `apps/desktop/scripts/wait-for-resources.mjs` |
| 4% | 4 | 58 | `apps/web/src/lib/expensiveReadRetry.ts` | `packages/client-runtime/src/state/attachments.ts` |
| 4% | 1 | 12 | `apps/web/src/pinnedProjectAgentsStore.test.ts` | `apps/server/src/pullRequest/pullRequestSyncKey.test.ts` |
| 4% | 14 | 316 | `apps/web/src/components/kanban/useKanbanTaskComposerEditor.ts` | `apps/mobile/src/features/threads/use-composer-command-menu.ts` |
| 4% | 6 | 163 | `apps/web/src/lib/projectReactQuery.test.ts` | `apps/web/src/components/files/projectFilesQueryState.test.ts` |
| 4% | 4 | 25 | `packages/shared/src/automationMode.test.ts` | `apps/server/src/git/linkCreatedPullRequest.test.ts` |
| 4% | 5 | 31 | `apps/web/src/components/chat/ComposerInputBanners.tsx` | `apps/web/src/components/chat/ComposerPendingUserInputPanel.tsx` |
| 4% | 2 | 28 | `apps/marketing/src/app/robots.ts` | `apps/desktop/src/app/DesktopEarlyElectronStartup.ts` |
| 4% | 4 | 41 | `apps/web/src/components/chat/ComposerActiveTaskListCard.tsx` | `apps/web/src/components/settings/SettingsScopeSentence.tsx` |
| 4% | 3 | 26 | `apps/web/src/components/chat/editedFilePathActions.ts` | `apps/server/src/assets/AttachmentUpload.ts` |
| 4% | 21 | 246 | `apps/web/src/components/settings/AppSnapPermissionSection.tsx` | `apps/web/src/components/settings/ProviderInstanceCard.tsx` |
| 4% | 2 | 68 | `apps/web/src/hooks/useInlineVisualTheme.ts` | `apps/web/src/components/chat/useComposerMultilinePrompt.ts` |
| 4% | 2 | 41 | `apps/web/src/threadDetailResumeCursors.test.ts` | `apps/web/src/lib/terminalContext.test.ts` |
| 4% | 21 | 362 | `apps/web/src/threadDetailSubscriptionRetention.test.ts` | `apps/server/src/relay/AgentAwarenessRelay.test.ts` |
| 4% | 4 | 14 | `packages/shared/src/approvalSessionGrant.ts` | `packages/client-runtime/src/providerSkills.ts` |
| 4% | 66 | 1287 | `apps/web/src/components/DiffPanel.tsx` | `apps/web/src/components/DiffPanel.tsx` |
| 4% | 7 | 53 | `apps/web/src/components/automation/AutomationComputerPicker.tsx` | `apps/web/src/components/cloud/ConnectOnboardingDialog.tsx` |
| 4% | 21 | 207 | `apps/web/src/components/pullRequest/PullRequestSummaryTab.tsx` | `apps/web/src/components/pullRequest/PullRequestTimelineTab.tsx` |
| 4% | 31 | 468 | `apps/web/src/components/settings/providers/ProvidersSettingsPanel.tsx` | `apps/web/src/components/settings/ProviderSettingsPanel.tsx` |
| 4% | 2 | 69 | `apps/web/src/lib/composerPastedText.test.ts` | `apps/desktop/src/linuxSecretStorage.test.ts` |
| 4% | 12 | 246 | `apps/web/src/lib/threadRecap.ts` | `packages/shared/src/agentAwareness.ts` |
| 4% | 5 | 114 | `apps/server/src/wsRequestAdmission.test.ts` | `apps/server/scripts/evaluate-thread-titles.ts` |
| 4% | 46 | 166 | `apps/web/src/composerDraftStoreTestFixtures.ts` | `apps/web/src/composerDraftStore.test.ts` |
| 4% | 3 | 50 | `apps/web/src/lib/rateLimitStatus.test.ts` | `apps/web/src/providerInstances.test.ts` |
| 4% | 4 | 81 | `apps/web/src/components/chat/rightDockPaneMeta.test.ts` | `apps/mobile/src/lib/composerAttachmentUploadQueue.test.ts` |
| 4% | 2 | 78 | `apps/desktop/src/browserAutomation/betterwrightCookieReader.test.ts` | `packages/shared/src/legacyCliLauncher.ts` |
| 4% | 2 | 80 | `apps/desktop/src/browserAutomation/betterwrightFocus.test.ts` | `native/kde-snap-shot/src/activate.test.ts` |
| 4% | 2 | 19 | `apps/desktop/src/startupDiagnosticDialog.ts` | `apps/desktop/src/permissions/MacSettingsWindow.ts` |
| 4% | 1 | 7 | `apps/marketing/.source/dynamic.ts` | `oxlint-plugin-t3code/rules/no-hermes-unsupported-apis.ts` |
| 4% | 1 | 10 | `apps/profiles/src/legacy-redirect.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 4% | 3 | 27 | `apps/server/src/mcpConnectors/config.ts` | `apps/desktop/scripts/wait-for-resources.mjs` |
| 4% | 1 | 10 | `apps/server/src/orchestration/providerOptionsSecurity.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 4% | 18 | 791 | `apps/server/src/providerUsageSnapshot.ts` | `apps/server/src/usage/usageTranscripts.ts` |
| 4% | 1 | 13 | `apps/web/src/components/LinkChipIcon.tsx` | `apps/web/src/components/pullRequest/pullRequestLinkContextMenu.ts` |
| 4% | 17 | 425 | `apps/web/src/components/githubInbox/githubInbox.logic.test.ts` | `packages/shared/src/threadPullRequests.test.ts` |
| 4% | 2 | 38 | `apps/web/src/lib/chatWorkspaceFolders.ts` | `apps/web/src/projectScripts.ts` |
| 4% | 1 | 7 | `apps/web/src/lib/shellQuote.ts` | `packages/shared/src/oauthScope.ts` |
| 4% | 3 | 66 | `apps/web/vitest.browser.ci.config.ts` | `packages/client-runtime/src/providerSkills.ts` |
| 4% | 1 | 10 | `scripts/bun-lockfile.ts` | `apps/mobile/src/features/devices/device-stream-document.ts` |
| 4% | 17 | 160 | `apps/server/src/provider/acp/AcpAdapterSupport.ts` | `apps/server/src/provider/acp/AcpRuntimeModel.ts` |
| 4% | 17 | 174 | `apps/web/src/components/SettingsSidebarNav.tsx` | `apps/web/src/components/settings/SettingsSidebarNav.tsx` |
| 4% | 26 | 371 | `apps/web/src/components/chat/RightDock.tsx` | `apps/web/src/components/RightPanelTabs.tsx` |
| 4% | 5 | 119 | `apps/web/src/hooks/useDeviceEventBridge.browser.tsx` | `apps/web/src/components/pullRequest/PullRequestThreadLinks.tsx` |
| 4% | 3 | 123 | `apps/server/perf/providerRuntimeJournalAppend.bench.ts` | `apps/server/src/persistence/Migrations/001_OrchestrationEvents.ts` |
| 4% | 9 | 81 | `apps/server/src/git/gitStatusCache.ts` | `packages/shared/src/git.ts` |
| 4% | 3 | 90 | `packages/profile-ui/src/formatting.ts` | `apps/web/src/projectScripts.ts` |
| 4% | 3 | 18 | `packages/shared/src/windowsCertificate.test.ts` | `apps/desktop/src/preview/PickedElementPayload.test.ts` |
| 4% | 14 | 108 | `apps/web/src/components/chat/ComposerImageAttachmentChip.tsx` | `apps/web/src/components/settings/ProviderInstanceCard.tsx` |
| 4% | 3 | 29 | `apps/server/src/externalMcp/executionAdmission.ts` | `apps/server/src/resourceTelemetry/DesktopTelemetryReceiver.test.ts` |
| 4% | 4 | 67 | `apps/web/src/components/chat/floatingBrowserPanel.logic.test.ts` | `apps/desktop/src/snapShot/HyprlandSnapShot.test.ts` |
| 4% | 14 | 147 | `apps/web/src/components/settings/ComputerAuditHistorySection.tsx` | `apps/web/src/components/pullRequest/PullRequestSummaryTab.tsx` |
| 4% | 9 | 109 | `apps/web/src/components/settings/McpConnectorProposalCard.tsx` | `apps/web/src/components/preview/ThreadPreviewMiniPlayer.tsx` |
| 4% | 12 | 55 | `apps/web/src/diffRenderModeStore.ts` | `apps/web/src/browserFaviconStore.ts` |
| 4% | 1 | 24 | `packages/shared/src/threadHierarchy.test.ts` | `apps/web/src/worktreeCleanup.test.ts` |
| 4% | 23 | 99 | `apps/server/src/orchestration/projectImportGuard.test.ts` | `apps/server/src/project/AgentSessionImporter.test.ts` |
| 4% | 13 | 98 | `apps/web/src/components/chat/SynaraThreadCreationCard.tsx` | `apps/web/src/components/pullRequest/PullRequestSummaryTab.tsx` |
| 4% | 6 | 44 | `apps/web/src/components/chat/UsageLimitPauseDivider.tsx` | `apps/web/src/components/chat/ModelListRow.tsx` |
| 4% | 7 | 56 | `apps/server/src/provider/attachmentProjection.ts` | `apps/server/src/provider/acp/AntigravityAcpSupport.ts` |
| 4% | 7 | 60 | `apps/web/src/components/chat/FileDiffHeader.tsx` | `apps/web/src/components/AgentsPanel.tsx` |
| 4% | 4 | 49 | `apps/web/src/components/chat/useSubagentRunControls.browser.tsx` | `apps/web/src/components/onboarding/WelcomeWizard.test.tsx` |
| 4% | 5 | 132 | `apps/web/src/components/hosts/HostDiscoverabilityPrompt.test.tsx` | `packages/shared/src/agentAwareness.test.ts` |
| 4% | 1 | 8 | `apps/marketing/source.config.ts` | `oxlint-plugin-t3code/rules/no-hermes-unsupported-apis.ts` |
| 4% | 1 | 12 | `apps/server/src/hostAuth/replayCache.ts` | `apps/web/src/components/files/fileTreeExpansion.ts` |
| 4% | 2 | 24 | `apps/server/src/provider/settleConcurrentTeardowns.test.ts` | `apps/server/src/assets/MediaFile.ts` |
| 4% | 1 | 16 | `apps/web/perf/vite.config.ts` | `oxlint-plugin-t3code/rules/namespace-node-imports.test.ts` |
| 4% | 14 | 116 | `apps/web/src/components/ProjectFolderList.tsx` | `apps/web/src/components/device/DeviceToolsPanel.tsx` |
| 4% | 2 | 25 | `apps/web/src/components/chat/ComputerControlDeniedCard.tsx` | `apps/web/src/components/settings/LocalEnvironmentSetting.tsx` |
| 4% | 1 | 11 | `apps/web/src/components/kanban/KanbanNewTaskDialog.logic.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 4% | 1 | 11 | `apps/web/src/hooks/useThrottledStreamingValue.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 4% | 5 | 63 | `apps/web/src/lib/bootstrapScreens.browser.ts` | `apps/web/src/components/files/AttachmentFilePreview.test.tsx` |
| 4% | 1 | 11 | `apps/web/src/lib/glassOverlayCutout.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 4% | 4 | 68 | `apps/web/src/lib/hosts/executionStorage.ts` | `apps/web/src/lib/chunkReloadGuard.test.ts` |
| 4% | 1 | 11 | `packages/contracts/src/git.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 4% | 1 | 11 | `packages/shared/src/approvalSessionGrant.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 4% | 29 | 429 | `apps/server/src/orchestration/startupTurnReconciliation.ts` | `apps/server/src/serverRuntimeStartup.ts` |
| 4% | 14 | 560 | `apps/server/src/persistence/Layers/OrchestrationEventDeliveries.test.ts` | `apps/server/src/persistence/Layers/OrchestrationEventStore.test.ts` |
| 4% | 23 | 547 | `apps/web/src/components/chat/MessageTrail.tsx` | `apps/web/src/components/ThreadTerminalDrawer.tsx` |
| 4% | 19 | 716 | `apps/web/src/components/chat/composerProviderRegistry.test.tsx` | `apps/web/src/components/chat/composerProviderState.test.tsx` |
| 4% | 4 | 119 | `apps/desktop/scripts/credential-recovery-smoke.ts` | `scripts/install.test.ts` |
| 4% | 5 | 85 | `apps/web/src/components/RunningChatsQuitCoordinator.tsx` | `apps/web/src/components/settings/ThemeSearchSection.tsx` |
| 4% | 4 | 109 | `apps/desktop/src/windowsShellIco.ts` | `scripts/lib/icon-export.ts` |
| 4% | 8 | 189 | `apps/server/src/pullRequests/pullRequestOperations.ts` | `apps/server/src/pullRequest/GitHubPullRequestProvider.ts` |
| 4% | 7 | 66 | `apps/web/src/lib/fileReferenceContextMenu.ts` | `apps/web/src/components/files/FileBrowserPanel.tsx` |
| 4% | 5 | 63 | `apps/web/src/components/chat/ClaudeCacheDetails.tsx` | `apps/web/src/components/chat/WorktreeSetupCard.tsx` |
| 4% | 18 | 302 | `apps/web/src/lib/composerSend.ts` | `apps/mobile/src/lib/composerImages.ts` |
| 4% | 2 | 18 | `apps/web/src/lib/hosts/workspaceKeybindings.ts` | `packages/client-runtime/src/connection/presentation.ts` |
| 4% | 2 | 22 | `packages/shared/src/platformEnvironment.test.ts` | `apps/desktop/src/linuxSecretStorage.test.ts` |
| 4% | 2 | 54 | `packages/shared/src/shell.nu.test.ts` | `apps/mobile/src/features/diagnostics/crash-log-model.test.ts` |
| 4% | 7 | 336 | `scripts/computer-use-fixtures/browser-benchmark-evidence.test.ts` | `packages/contracts/src/settings.test.ts` |
| 4% | 4 | 79 | `apps/marketing/src/app/docs/[[...slug]]/page.tsx` | `apps/web/src/components/cloud/ConnectCliAuthSurface.tsx` |
| 4% | 4 | 82 | `apps/server/src/device/helperCapabilities.test.ts` | `apps/web/src/desktopAppActivation.test.ts` |
| 4% | 3 | 36 | `apps/server/src/projectAgent/groupCoordinatorHost.ts` | `apps/web/src/onboarding/firstRun.logic.ts` |
| 4% | 3 | 17 | `apps/web/src/components/chat/InlineSlashCommandChip.tsx` | `apps/web/src/browser/BrowserViewportResizeHandles.tsx` |
| 4% | 5 | 73 | `apps/web/src/components/chat/TranscriptSelectionActionLayer.tsx` | `apps/web/src/components/clerk/T3ConnectUserProfilePage.tsx` |
| 4% | 4 | 58 | `apps/desktop/src/updatePendingCache.ts` | `apps/mobile/src/lib/composerAttachmentUploadQueue.ts` |
| 4% | 7 | 55 | `apps/server/src/orchestration/projectRelocation.ts` | `apps/server/src/orchestration/projector.pullRequests.test.ts` |
| 4% | 9 | 386 | `apps/server/src/terminal/windowsProcessSnapshot.test.ts` | `apps/web/src/components/preview/previewAutomationHostBudget.test.ts` |
| 4% | 7 | 69 | `apps/web/src/components/chat/AutomationCreatedCard.tsx` | `apps/web/src/components/AgentsPanel.tsx` |
| 4% | 5 | 83 | `apps/web/src/components/chat/environment/EnvironmentActionsSection.browser.tsx` | `apps/web/src/projectScripts.test.ts` |
| 4% | 4 | 69 | `apps/web/src/hooks/useSmoothStreamedText.browser.tsx` | `apps/web/src/hooks/useTheme.test.ts` |
| 4% | 3 | 80 | `packages/contracts/src/agentMentions.test.ts` | `apps/web/src/components/chat/ContextWindowMeter.logic.test.ts` |
| 4% | 9 | 212 | `apps/web/src/lib/workspaceEditorSession.test.ts` | `apps/web/src/components/ProviderUpdateEnvironmentRows.test.tsx` |
| 4% | 1 | 12 | `apps/e2e/src/headlessClient.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 4% | 1 | 9 | `apps/marketing/.source/browser.ts` | `oxlint-plugin-t3code/rules/no-hermes-unsupported-apis.ts` |
| 4% | 5 | 114 | `apps/server/src/codexAppServerTransport.test.ts` | `packages/shared/src/remote.test.ts` |
| 4% | 2 | 40 | `apps/server/src/computer/UnavailableComputerBackend.test.ts` | `packages/shared/src/searchRanking.test.ts` |
| 4% | 1 | 12 | `apps/server/src/editor/nativeEditorGate.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 4% | 2 | 34 | `apps/server/src/orchestration/testing/fakeProjectionSnapshotQuery.ts` | `apps/desktop/src/ipc/DesktopIpc.test.ts` |
| 4% | 2 | 15 | `apps/web/src/chatRouteRecovery.ts` | `apps/mobile/src/connection/background-activity.ts` |
| 4% | 2 | 63 | `apps/web/src/components/profile/ProfileUsageCoverage.test.tsx` | `apps/web/src/components/clerk/T3ConnectUserProfilePage.test.tsx` |
| 4% | 4 | 60 | `apps/web/src/kanbanUiStore.persistence.test.ts` | `apps/web/src/clientPersistenceStorage.test.ts` |
| 4% | 3 | 33 | `apps/web/src/lib/contextMenuIcons.tsx` | `apps/web/src/components/settings/ProviderInstanceCard.test.ts` |
| 4% | 2 | 9 | `apps/web/src/lib/threadUndoToast.ts` | `apps/web/src/components/SlowRpcRequestToastCoordinator.tsx` |
| 4% | 5 | 81 | `scripts/write-release-artifact-provenance.ts` | `scripts/lib/update-manifest.ts` |
| 4% | 44 | 375 | `apps/server/src/orchestration/Layers/messageTextChunks.test.ts` | `apps/server/src/orchestration/Layers/OrchestrationEngine.test.ts` |
| 4% | 5 | 48 | `apps/web/src/projectTerminalRunner.ts` | `apps/mobile/src/features/terminal/terminalMenu.test.ts` |
| 4% | 4 | 49 | `apps/web/src/timestampFormat.test.ts` | `packages/shared/src/usageFormat.ts` |
| 4% | 7 | 583 | `apps/api/src/billing/polar.test.ts` | `apps/web/src/lib/openPullRequestLink.test.ts` |
| 4% | 6 | 128 | `apps/desktop/scripts/remote-resource-smoke.ts` | `apps/desktop/src/electron/ElectronProtocol.ts` |
| 4% | 10 | 182 | `apps/web/src/components/GitActionsControl.browser.tsx` | `packages/shared/src/git.test.ts` |
| 4% | 20 | 452 | `apps/web/src/components/chat/environment/EnvironmentPanel.tsx` | `apps/web/src/components/chat/ChatHeader.tsx` |
| 4% | 11 | 84 | `apps/web/src/components/ProjectHoverCardContent.tsx` | `apps/web/src/components/pullRequest/PullRequestTimelineTab.tsx` |
| 4% | 2 | 24 | `apps/web/src/lib/toolCallDetailsFormatting.test.ts` | `apps/desktop/src/linuxSecretStorage.test.ts` |
| 4% | 3 | 53 | `packages/profile-ui/src/formatting.test.ts` | `apps/web/src/timestampFormat.test.ts` |
| 4% | 3 | 57 | `packages/shared/src/lengthPrefixedRecords.test.ts` | `apps/desktop/src/snapShot/NiriCaptureShortcut.ts` |
| 4% | 3 | 6 | `apps/web/scripts/react-compiler-plugin.mjs` | `apps/web/vite.config.ts` |
| 4% | 7 | 81 | `apps/web/src/diffRenderModeStore.test.ts` | `apps/web/src/uiStateStore.test.ts` |
| 4% | 3 | 106 | `apps/web/src/hooks/useActivityThreads.ts` | `apps/web/src/state/device.ts` |
| 4% | 4 | 45 | `apps/desktop/src/browserAutomation/waitAndEvaluate.ts` | `apps/web/src/components/device/deviceHubApi.ts` |
| 4% | 7 | 71 | `apps/server/src/platform/effectProcessRuntime.ts` | `packages/tailscale/src/tailscale.test.ts` |
| 4% | 6 | 80 | `apps/web/src/components/computer/useComputerPreviewFloat.ts` | `apps/web/src/components/chat/ZoomableImage.tsx` |
| 4% | 4 | 82 | `apps/web/src/lib/fileReferenceContextMenu.test.ts` | `apps/web/src/hooks/useCopyToClipboard.test.ts` |
| 4% | 9 | 137 | `packages/shared/src/threadPullRequestLinks.test.ts` | `apps/server/src/orchestration/ThreadSettlementPolicy.test.ts` |
| 4% | 7 | 128 | `apps/desktop/src/desktopUpdateControl.ts` | `apps/desktop/src/window/DesktopWindow.ts` |
| 4% | 7 | 124 | `apps/server/src/device/helperCapabilities.ts` | `apps/server/src/orchestration/ActivityPayloadProjection.ts` |
| 4% | 20 | 938 | `apps/web/src/components/terminal/terminalRuntime.ts` | `apps/web/src/components/ThreadTerminalDrawer.tsx` |
| 4% | 2 | 61 | `apps/api/src/routes/v1.orgSession.test.ts` | `apps/web/src/components/usage/usagePagePreferences.test.ts` |
| 4% | 2 | 16 | `apps/desktop/src/browserAutomation/browserShutdown.ts` | `packages/client-runtime/src/state/usage.ts` |
| 4% | 2 | 35 | `apps/marketing/src/lib/releases.ts` | `apps/desktop/src/app/DesktopEarlyElectronStartup.ts` |
| 4% | 4 | 40 | `apps/server/src/orchestration/projectImportSources.test.ts` | `packages/client-runtime/src/state/sharedSettings.test.ts` |
| 4% | 1 | 11 | `apps/web/src/components/profile/useProfileName.ts` | `oxlint-plugin-t3code/rules/no-inline-schema-compile.test.ts` |
| 4% | 15 | 408 | `apps/web/src/components/settings/UsageAnalyticsPanel.tsx` | `apps/web/src/components/pullRequest/PullRequestSummaryTab.tsx` |
| 4% | 1 | 13 | `apps/web/src/lib/spaceIconSuggestion.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 4% | 1 | 13 | `apps/web/src/lib/studioOutputDisplay.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 4% | 1 | 11 | `packages/contracts/src/browserAutomationToolCommon.ts` | `oxlint-plugin-t3code/rules/no-inline-schema-compile.test.ts` |
| 4% | 1 | 13 | `packages/shared/src/desktopTitleBar.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 4% | 4 | 70 | `packages/shared/src/modelName.ts` | `packages/contracts/src/usage.ts` |
| 4% | 1 | 26 | `packages/shared/src/providerCliProfiles.test.ts` | `apps/web/src/cloud/dpop.test.ts` |
| 4% | 1 | 13 | `scripts/computer-use-fixtures.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 4% | 80 | 4155 | `apps/server/src/codexAppServerManager.ts` | `apps/server/src/provider/CodexDeveloperInstructions.ts` |
| 4% | 4 | 105 | `apps/web/src/components/hosts/IosBetaAnnouncement.browser.tsx` | `apps/web/src/components/chat/AssistantCitationChip.test.tsx` |
| 4% | 7 | 74 | `apps/server/src/editor/languageService.test.ts` | `apps/desktop/src/permissions/MacPermissionHelper.test.ts` |
| 4% | 28 | 259 | `apps/server/src/orchestration/Layers/SidechatExpiryReactor.test.ts` | `apps/server/src/orchestration/projector.test.ts` |
| 4% | 4 | 42 | `apps/server/src/platform/processLiveness.test.ts` | `apps/server/src/provider/acp/XAiAcpExtension.test.ts` |
| 4% | 7 | 98 | `apps/server/src/workspaceFileChanges.ts` | `apps/desktop/src/electron/ElectronWindow.ts` |
| 4% | 3 | 31 | `apps/web/src/components/ChatMarkdown.find.browser.tsx` | `apps/web/src/components/settings/UsageProviderSettings.tsx` |
| 4% | 3 | 73 | `apps/web/src/components/githubInbox/GitHubInboxSidechatDock.tsx` | `apps/web/src/hooks/usePullRequestLinking.ts` |
| 4% | 2 | 81 | `apps/desktop/src/cuaHostSocket.test.ts` | `apps/desktop/src/electron/WindowsForegroundFocusWorker.test.ts` |
| 4% | 5 | 162 | `apps/server/src/hostConnectivitySupervisor.ts` | `apps/desktop/src/app/DesktopAppActivation.ts` |
| 4% | 15 | 163 | `apps/web/src/components/pdf/PdfViewerToolbar.tsx` | `apps/web/src/components/BranchToolbar.tsx` |
| 4% | 10 | 436 | `apps/web/src/components/tasks/tasks.logic.ts` | `packages/client-runtime/src/state/threadReducer.ts` |
| 4% | 7 | 109 | `apps/web/src/hooks/useTerminalSurfaceController.ts` | `apps/web/src/cloud/useCloudLinkController.ts` |
| 4% | 2 | 12 | `packages/shared/src/diagnosticIssue.ts` | `apps/desktop/src/app/DesktopAppActivationBroker.ts` |
| 4% | 8 | 194 | `apps/server/src/provider/FactoryPluginDiscovery.ts` | `apps/server/src/diagnostics/TraceDiagnostics.ts` |
| 4% | 9 | 209 | `apps/web/src/components/chat/ComposerPredictionAccept.browser.tsx` | `apps/web/src/components/settings/SettingsFontPreviews.tsx` |
| 4% | 21 | 641 | `apps/web/src/components/inbox/InboxView.tsx` | `apps/web/src/components/pullRequest/PullRequestSummaryTab.tsx` |
| 4% | 17 | 308 | `apps/web/src/lib/composerImagePreparation.ts` | `apps/desktop/src/preview/FaviconCapture.ts` |
| 4% | 7 | 264 | `apps/desktop/src/linuxCuaAdmission.test.ts` | `apps/server/src/provider/acp/XAiAcpExtension.test.ts` |
| 4% | 20 | 642 | `apps/server/src/main.test.ts` | `apps/server/src/cli/config.test.ts` |
| 4% | 9 | 132 | `apps/desktop/src/menuShortcuts.ts` | `apps/desktop/src/snapShot/snapShot.ts` |
| 4% | 1 | 11 | `apps/marketing/.source/source.config.mjs` | `oxlint-plugin-t3code/rules/no-hermes-unsupported-apis.ts` |
| 4% | 7 | 318 | `apps/server/src/claudeTokenStats.ts` | `apps/server/src/persistence/Migrations/024_BackfillProjectionThreadShellSummary.ts` |
| 4% | 2 | 71 | `apps/server/src/persistence/Services/ProviderRuntimeEvents.ts` | `apps/server/src/persistence/Services/OrchestrationEventStore.ts` |
| 4% | 1 | 24 | `apps/server/src/provider/providerCliVersionProbe.ts` | `apps/mobile/src/features/showcase/showcaseRetry.ts` |
| 4% | 4 | 113 | `apps/server/src/providerUsage/providers/droidSecureStorage.test.ts` | `apps/mobile/src/lib/openExternalUrl.test.ts` |
| 4% | 6 | 103 | `apps/web/src/appSnapIntake.test.ts` | `apps/web/src/questionAttachments.test.ts` |
| 4% | 1 | 14 | `apps/web/src/components/ChatMarkdown.compiler.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 4% | 2 | 41 | `apps/web/src/components/ProjectImportAnnouncementDialog.browser.tsx` | `apps/mobile/src/features/diffs/nativeReviewDiffSurface.test.ts` |
| 4% | 3 | 68 | `apps/web/src/components/account/AccountFooterControl.browser.tsx` | `apps/mobile/src/state/use-selected-thread-requests.test.tsx` |
| 4% | 4 | 32 | `apps/web/src/components/automation/AutomationProposalActions.tsx` | `apps/web/src/components/pullRequest/PullRequestThreadLinks.tsx` |
| 4% | 1 | 12 | `apps/web/src/components/profile/useProfileAvatarImage.ts` | `oxlint-plugin-t3code/rules/no-inline-schema-compile.test.ts` |
| 4% | 1 | 12 | `apps/web/src/featureTour/store.ts` | `oxlint-plugin-t3code/rules/no-inline-schema-compile.test.ts` |
| 4% | 2 | 37 | `apps/web/src/lib/appTypography.ts` | `apps/desktop/src/app/DesktopEarlyElectronStartup.ts` |
| 4% | 1 | 14 | `apps/web/src/tasksSurface.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 4% | 5 | 82 | `benchmarks/desktop-perf/lib/tests-startup.mjs` | `apps/mobile/src/features/sharing/incoming-share-model.ts` |
| 4% | 3 | 222 | `packages/contracts/src/settings.ts` | `packages/contracts/src/device.ts` |
| 4% | 2 | 13 | `packages/shared/src/automationCompletionPolicy.ts` | `apps/desktop/src/app/DesktopAppActivationBroker.ts` |
| 4% | 4 | 39 | `packages/shared/src/remoteResources.test.ts` | `packages/contracts/src/provider.test.ts` |
| 4% | 6 | 96 | `apps/server/src/processRunner.test.ts` | `apps/server/src/processRunner.test.ts` |
| 4% | 15 | 203 | `apps/web/src/components/chat/useTranscriptAssistantSelectionAction.ts` | `apps/web/src/components/settings/ThemeEditorPanel.tsx` |
| 4% | 7 | 348 | `apps/web/src/hooks/useSidebarProjectRunController.test.ts` | `apps/web/src/test/reactHookHarness.ts` |
| 4% | 28 | 262 | `apps/server/src/orchestration/Layers/SidechatExpiryReactor.ts` | `apps/server/src/orchestration/Layers/ProviderCommandReactor.ts` |
| 4% | 6 | 174 | `apps/server/src/usagePricing.test.ts` | `apps/web/src/components/KeybindingsUpdateToast.logic.test.ts` |
| 4% | 4 | 110 | `apps/web/src/components/CopyPathActions.browser.tsx` | `apps/web/src/localApi.test.ts` |
| 4% | 5 | 74 | `apps/web/src/lib/linkContextMenu.ts` | `apps/web/src/components/pullRequest/LinkPullRequestDialog.tsx` |
| 4% | 10 | 188 | `apps/server/src/agentGateway/delegatedAttachments.ts` | `apps/server/src/provider/acp/AntigravityAcpSupport.ts` |
| 4% | 5 | 70 | `scripts/computer-use-fixtures/collect-measurement.ts` | `apps/server/src/usage/cursorUsageReader.ts` |
| 4% | 3 | 59 | `apps/desktop/src/cuaDriverOrphanSweep.test.ts` | `apps/mobile/src/lib/openExternalUrl.test.ts` |
| 4% | 2 | 49 | `apps/server/src/editor/dapConnection.test.ts` | `apps/server/src/httpResponseErrorGuard.test.ts` |
| 4% | 2 | 85 | `apps/server/src/provider/keyedLock.test.ts` | `apps/web/src/lib/desktopPasteAsText.test.ts` |
| 4% | 6 | 125 | `apps/server/src/wsCompatibility.ts` | `apps/desktop/src/app/DesktopAppActivationBroker.ts` |
| 4% | 12 | 148 | `apps/web/src/browserStateStore.ts` | `apps/web/src/browserHistoryStore.ts` |
| 4% | 4 | 209 | `apps/web/src/components/CreateProjectDialog.browser.tsx` | `apps/mobile/src/features/diffs/nativeReviewDiffSurface.test.ts` |
| 4% | 28 | 515 | `apps/web/src/components/profile/EditProfileDialog.tsx` | `apps/web/src/components/settings/KeybindingsSettings.tsx` |
| 4% | 2 | 38 | `packages/shared/src/errorMessages.ts` | `apps/server/src/textGeneration/TextGenerationUtils.ts` |
| 4% | 5 | 58 | `packages/shared/src/staticSnapshot.ts` | `apps/server/src/usage/usageTranscriptReader.ts` |
| 4% | 2 | 77 | `scripts/computer-use-fixtures/build-electron.mjs` | `apps/mobile/plugins/withWidgetLogoAsset.cjs` |
| 4% | 8 | 164 | `apps/desktop/src/browserSessionPolicy.ts` | `apps/desktop/src/permissions/MacPermissionHelper.ts` |
| 4% | 5 | 86 | `apps/desktop/src/localHtmlPreviewProtocol.test.ts` | `scripts/lib/third-party-licenses.test.ts` |
| 4% | 5 | 71 | `apps/e2e/src/harness/rpc.ts` | `apps/desktop/src/app/DesktopAppActivation.test.ts` |
| 4% | 3 | 59 | `apps/server/src/auth/Layers/BootstrapCredentialService.test.ts` | `apps/server/src/auth/http.test.ts` |
| 4% | 7 | 254 | `apps/server/src/browserAutomation/browserHostRpcClient.ts` | `apps/server/src/orchestration/ActivityPayloadProjection.ts` |
| 4% | 6 | 163 | `apps/server/src/provider/providerBinaryResolution.test.ts` | `apps/server/src/provider/acp/XAiAcpExtension.test.ts` |
| 4% | 20 | 284 | `apps/web/src/components/settings/SyncKeyPairingPanel.tsx` | `apps/web/src/components/settings/ThemeImportDialog.tsx` |
| 4% | 13 | 98 | `apps/web/src/lib/activeThreadDelete.ts` | `apps/web/src/hooks/useThreadActions.ts` |
| 4% | 10 | 107 | `apps/web/src/components/ProviderUsagePanelContent.tsx` | `apps/web/src/components/pullRequest/PullRequestTimelineTab.tsx` |
| 4% | 7 | 92 | `apps/web/src/lib/projectShortcutTargets.test.ts` | `packages/client-runtime/src/state/projectGrouping.test.ts` |
| 4% | 2 | 29 | `apps/server/src/codexPathIdentity.ts` | `apps/desktop/src/permissions/MacSettingsWindow.ts` |
| 4% | 3 | 184 | `apps/server/src/computer/computerLockResume.test.ts` | `apps/desktop/src/snapShot/KdeSnapShot.ts` |
| 4% | 6 | 142 | `apps/server/src/externalMcp/Layers/ExternalMcpService.test.ts` | `apps/server/src/auth/EnvironmentAuth.test.ts` |
| 4% | 4 | 85 | `apps/server/src/hostConnections/supervisor.ts` | `apps/desktop/src/permissions/MacSettingsWindow.ts` |
| 4% | 7 | 189 | `apps/server/src/hostConnectivity.integration.test.ts` | `packages/shared/src/relayClient.test.ts` |
| 4% | 1 | 15 | `apps/server/src/memoryDiagnostics.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 4% | 1 | 13 | `apps/server/src/reliability.ts` | `oxlint-plugin-t3code/rules/no-inline-schema-compile.test.ts` |
| 4% | 1 | 15 | `apps/web/src/components/chat/ComposerQueuedHeader.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 4% | 9 | 160 | `apps/web/src/components/chat/runtimeModelCapabilities.ts` | `apps/server/src/provider/Layers/CodexProvider.ts` |
| 4% | 9 | 237 | `apps/web/src/components/chat/useChatComposerEditing.ts` | `apps/mobile/src/features/threads/use-composer-command-menu.ts` |
| 4% | 3 | 57 | `apps/web/src/components/settings/ProfileSettingsPanel.browser.tsx` | `apps/web/src/components/usage/UsagePage.test.tsx` |
| 4% | 1 | 15 | `apps/web/src/components/sidebarHoverCardStyles.ts` | `apps/mobile/src/features/devices/device-stream-document.ts` |
| 4% | 1 | 15 | `apps/web/src/hooks/useStableCallback.browser.tsx` | `apps/desktop/gnome-extension/captureService.js` |
| 4% | 3 | 138 | `apps/web/src/threadVisitedPersistence.test.ts` | `packages/client-runtime/src/state/threadSnoozed.test.ts` |
| 4% | 1 | 13 | `packages/contracts/src/browserAutomationBounds.ts` | `oxlint-plugin-t3code/rules/no-inline-schema-compile.test.ts` |
| 4% | 1 | 13 | `packages/contracts/src/todo.test.ts` | `oxlint-plugin-t3code/rules/no-inline-schema-compile.test.ts` |
| 4% | 1 | 13 | `packages/shared/src/browserAutomationMcpError.ts` | `oxlint-plugin-t3code/rules/no-inline-schema-compile.test.ts` |
| 4% | 5 | 53 | `packages/shared/src/githubRepository.ts` | `packages/shared/src/git.ts` |
| 4% | 1 | 15 | `packages/shared/src/searchQuery.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 4% | 4 | 65 | `apps/marketing/src/components/Navbar.tsx` | `apps/web/src/components/sidebar/SidebarChrome.tsx` |
| 4% | 10 | 342 | `apps/server/src/automation/schedule.ts` | `packages/shared/src/usageFormat.ts` |
| 4% | 2 | 34 | `apps/server/src/codexTurnInput.ts` | `apps/web/src/fileContextMenu.ts` |
| 4% | 6 | 94 | `apps/web/src/components/chat/MessagesTimeline.hubWork.browser.tsx` | `apps/web/src/components/pullRequest/PullRequestThreadLinks.tsx` |
| 4% | 4 | 46 | `packages/contracts/src/remoteAgentGateway.test.ts` | `packages/contracts/src/provider.test.ts` |
| 4% | 2 | 41 | `apps/api/src/routes/v1.refresh.test.ts` | `apps/desktop/src/electron/WindowsForegroundFocusThread.test.ts` |
| 4% | 4 | 33 | `apps/server/src/provider/acp/acpFork.ts` | `apps/server/src/auth/ServerSecretStore.ts` |
| 4% | 6 | 144 | `apps/web/src/components/chat/ComposerClaudeCacheReviewPanel.tsx` | `apps/web/src/components/cloud/ConnectOnboardingDialog.tsx` |
| 4% | 2 | 33 | `apps/web/src/components/profile/profileFormatting.ts` | `apps/mobile/src/features/connection/EnvironmentConnectionNotice.tsx` |
| 4% | 2 | 30 | `apps/web/src/hooks/useThreadComputerStateSeed.browser.tsx` | `apps/web/src/components/chat/composerEventScope.test.ts` |
| 4% | 2 | 38 | `apps/web/src/lib/codexResetAttempt.ts` | `apps/web/src/cloud/managedAuth.tsx` |
| 4% | 2 | 62 | `packages/contracts/src/hostConnection.ts` | `packages/contracts/src/remoteAccess.ts` |
| 4% | 4 | 63 | `apps/server/src/externalMcp/runtimePolicy.test.ts` | `packages/contracts/src/preview.test.ts` |
| 4% | 20 | 412 | `apps/server/src/provider/Layers/AntigravityAdapter.test.ts` | `apps/server/src/provider/Layers/AntigravityAdapter.test.ts` |
| 4% | 2 | 30 | `apps/server/src/provider/claudeEdgeBlock.test.ts` | `apps/desktop/src/preview/PreviewKeyboard.test.ts` |
| 4% | 14 | 518 | `apps/server/src/remoteSessions/httpRoute.test.ts` | `apps/mobile/src/connection/environment-cache-store.test.ts` |
| 4% | 3 | 101 | `apps/web/src/components/ThemePackEditor.translucency.browser.tsx` | `apps/mobile/src/features/diffs/nativeReviewDiffSurface.test.ts` |
| 4% | 19 | 295 | `apps/web/src/components/chat/WorkspaceFilePreviewHeader.tsx` | `apps/web/src/components/RightPanelTabs.tsx` |
| 4% | 24 | 241 | `apps/web/src/components/settings/providers/ProviderAgentRow.tsx` | `apps/web/src/components/pullRequest/PullRequestSummaryTab.tsx` |
| 4% | 15 | 259 | `apps/web/src/lib/spaceNavigation.test.ts` | `packages/client-runtime/src/state/entities.test.ts` |
| 4% | 7 | 302 | `apps/web/src/terminalStateStore.test.ts` | `apps/web/src/terminalUiStateStore.test.ts` |
| 4% | 3 | 43 | `packages/shared/src/filesystemPlatform.test.ts` | `apps/web/src/browser/desktopTabLifetime.test.ts` |
| 4% | 5 | 96 | `apps/desktop/src/browserAutomation/betterwrightConnection.ts` | `apps/server/src/cli/app.ts` |
| 4% | 7 | 147 | `apps/desktop/src/desktopUpdateControl.test.ts` | `apps/web/src/components/desktopUpdate.toast.test.tsx` |
| 4% | 4 | 165 | `apps/desktop/src/desktopUserDataProfile.test.ts` | `apps/web/src/browserFaviconStore.test.ts` |
| 4% | 4 | 83 | `apps/web/src/components/chat/SurfaceContentTabs.browser.tsx` | `apps/desktop/src/snapShot/GnomeCaptureSetup.test.ts` |
| 4% | 8 | 253 | `apps/web/src/onboarding/stage/StageIllustrations.tsx` | `apps/web/src/components/settings/ProviderInstanceCard.tsx` |
| 4% | 2 | 17 | `apps/desktop/src/browserAutomation/hostErrors.ts` | `apps/desktop/src/app/DesktopAppActivationBroker.ts` |
| 4% | 3 | 181 | `apps/desktop/src/cuaFixtures/focusProbe.test.ts` | `apps/web/src/components/KeybindingsUpdateToast.logic.test.ts` |
| 4% | 1 | 4 | `apps/marketing/postcss.config.mjs` | `.github/scripts/relay-state-output.test.cjs` |
| 4% | 1 | 4 | `apps/profiles/postcss.config.mjs` | `.github/scripts/relay-state-output.test.cjs` |
| 4% | 1 | 21 | `apps/server/src/agentGateway/browserProof.ts` | `oxlint-plugin-t3code/rules/namespace-node-imports.test.ts` |
| 4% | 1 | 16 | `apps/server/src/agentGateway/computerApprovalDisplay.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 4% | 2 | 45 | `apps/server/src/agentGateway/diagnosticCursor.ts` | `apps/server/src/persistence/Migrations/050_ProjectionThreadPullRequests.ts` |
| 4% | 4 | 59 | `apps/server/src/agentGateway/inFlightRequestRegistry.ts` | `apps/mobile/src/features/voice-input/useVoiceInputController.ts` |
| 4% | 2 | 24 | `apps/server/src/computer/DesktopOperationQueue.ts` | `apps/mobile/src/state/use-thread-pr.ts` |
| 4% | 4 | 133 | `apps/server/src/hostSecrets/syncKeyStore.test.ts` | `apps/desktop/src/app/DesktopAppActivation.test.ts` |
| 4% | 12 | 603 | `apps/server/src/provider/OpenCodeDiscovery.test.ts` | `apps/server/src/provider/Layers/OpenCodeProvider.test.ts` |
| 4% | 1 | 16 | `apps/server/src/provider/promptReferenceProjection.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 4% | 2 | 90 | `apps/server/src/terminal/terminalModeReplay.ts` | `packages/shared/src/legacyCliLauncher.ts` |
| 4% | 1 | 16 | `apps/web/src/components/sidebarHoverCardAnchors.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 4% | 9 | 167 | `apps/web/src/hooks/useThreadPullRequests.ts` | `apps/mobile/src/state/queries.ts` |
| 4% | 3 | 27 | `scripts/lib/desktop-runtime-resources.ts` | `apps/server/scripts/migrate-dev-db.test.ts` |
| 4% | 2 | 33 | `scripts/pre-push-check.test.ts` | `apps/web/src/components/diffs/diffFileTree.logic.test.ts` |
| 4% | 29 | 126 | `apps/server/src/orchestration/Layers/ThreadSnoozeProjection.test.ts` | `apps/server/src/orchestration/Layers/OrchestrationEngine.test.ts` |
| 4% | 9 | 108 | `apps/web/src/components/tasks/TaskHandOff.tsx` | `apps/web/src/components/AgentsPanel.tsx` |
| 3% | 3 | 61 | `apps/desktop/src/desktopStaticProtocol.test.ts` | `packages/client-runtime/src/textPaste.test.ts` |
| 3% | 9 | 233 | `apps/server/src/git/textGenerationShared.test.ts` | `apps/server/src/textGeneration/TextGenerationPrompts.test.ts` |
| 3% | 51 | 3232 | `apps/server/src/provider/Layers/DevinAdapter.test.ts` | `apps/server/src/provider/Layers/OpenCodeAdapter.test.ts` |
| 3% | 3 | 160 | `apps/server/src/providerUsage/providers/cursor.test.ts` | `apps/mobile/src/features/diffs/nativeReviewDiffSurface.test.ts` |
| 3% | 7 | 100 | `apps/web/src/onboarding/useProviderSetup.ts` | `apps/web/src/components/ProviderUpdateEnvironmentRows.tsx` |
| 3% | 6 | 178 | `apps/api/scripts/measure-perf.ts` | `packages/shared/src/schemaJson.ts` |
| 3% | 4 | 127 | `apps/server/src/computer/computerRevoke.test.ts` | `apps/mobile/src/features/threads/checkout-new-task-branch.test.ts` |
| 3% | 3 | 69 | `apps/web/src/hooks/useComposerDropzone.test.ts` | `apps/mobile/src/features/threads/thread-feed-live-follow.test.ts` |
| 3% | 3 | 11 | `apps/web/src/lib/clientPointMenuAnchor.ts` | `apps/web/src/components/ThreadTerminalDrawer.test.ts` |
| 3% | 2 | 19 | `apps/web/src/lib/groupProjects.disabled.test.ts` | `apps/web/src/components/pullRequest/LinkPullRequestDialog.logic.test.ts` |
| 3% | 11 | 265 | `apps/analytics/src/views/Usage.tsx` | `apps/web/src/components/usage/UsagePage.tsx` |
| 3% | 3 | 21 | `apps/api/src/identity/grantIssuer.ts` | `apps/desktop/src/app/DesktopAppActivation.test.ts` |
| 3% | 5 | 54 | `apps/desktop/src/desktopCustomTitleBar.ts` | `apps/desktop/src/preview/BrowserImport/ChromiumCookies.ts` |
| 3% | 10 | 141 | `apps/server/src/providerUsage/providers/droidCredentials.ts` | `apps/server/src/cli/theme.ts` |
| 3% | 18 | 147 | `apps/server/src/pullRequests/pullRequestOperations.test.ts` | `apps/server/src/orchestration/PullRequestSyncReactor.test.ts` |
| 3% | 14 | 732 | `apps/server/src/wsSnapshotLiveStream.test.ts` | `apps/server/src/orchestration/ThreadPullRequestReactor.test.ts` |
| 3% | 6 | 97 | `apps/web/src/components/DiffPanelToolbar.browser.tsx` | `packages/shared/src/git.test.ts` |
| 3% | 5 | 47 | `apps/web/src/hooks/useDesktopAppIcon.ts` | `apps/web/src/components/AppSidebarLayout.tsx` |
| 3% | 3 | 79 | `apps/server/src/hostAuth/grantVerifier.test.ts` | `apps/desktop/src/app/DesktopAppActivation.test.ts` |
| 3% | 29 | 151 | `apps/server/src/serverLayers.device.test.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.test.ts` |
| 3% | 28 | 634 | `apps/web/src/components/EditorWorkspaceView.tsx` | `apps/web/src/components/ThreadTerminalDrawer.tsx` |
| 3% | 6 | 93 | `apps/web/src/railShellStore.ts` | `apps/web/src/browserFaviconStore.ts` |
| 3% | 1 | 17 | `apps/api/src/db/migrate.ts` | `apps/desktop/gnome-extension/captureService.js` |
| 3% | 1 | 17 | `apps/desktop/src/serverListeningDetector.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 6 | 180 | `apps/desktop/src/updateInstallMarker.test.ts` | `apps/server/src/bootstrap.test.ts` |
| 3% | 1 | 16 | `apps/server/src/agentGateway/toolGuidanceCadence.ts` | `apps/web/src/lib/lruCache.ts` |
| 3% | 1 | 15 | `apps/server/src/betaOperationalIssue.ts` | `oxlint-plugin-t3code/rules/no-inline-schema-compile.test.ts` |
| 3% | 3 | 66 | `apps/server/src/externalMcp/Services/ExternalMcpService.ts` | `packages/shared/src/agentAwareness.ts` |
| 3% | 18 | 249 | `apps/server/src/provider/Layers/ProviderRuntimeReconciler.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.ts` |
| 3% | 1 | 17 | `apps/server/src/provider/claudeAuthStatus.malformedJson.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 3 | 115 | `apps/web/perf/metrics.ts` | `packages/client-runtime/src/state/filesystem.ts` |
| 3% | 1 | 6 | `apps/web/src/components/chat/environment/EnvironmentPanel.logic.ts` | `packages/client-runtime/src/textPaste.ts` |
| 3% | 2 | 15 | `apps/web/src/components/settings/views/WorktreeDefaultsSection.tsx` | `apps/mobile/src/features/connection/GitHubRoutingSettings.tsx` |
| 3% | 5 | 52 | `apps/web/src/lib/fastModeState.test.ts` | `apps/server/src/orchestration/ActivityPayloadProjection.test.ts` |
| 3% | 1 | 20 | `apps/web/src/lib/hosts/controlQueryScope.ts` | `apps/web/src/components/pullRequest/pullRequestLinkContextMenu.ts` |
| 3% | 1 | 17 | `apps/web/src/providerOrdering.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 1 | 17 | `apps/web/src/routes/-desktopProjectRecoveryAttempt.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 1 | 15 | `packages/contracts/src/providerInstance.test.ts` | `oxlint-plugin-t3code/rules/no-inline-schema-compile.test.ts` |
| 3% | 1 | 17 | `packages/shared/src/Struct.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 1 | 17 | `packages/shared/src/projectDirectoryName.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 1 | 17 | `packages/shared/src/threadMentions.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 16 | 278 | `packages/shared/src/loginShellEnvironment.ts` | `apps/desktop/src/shell/DesktopShellEnvironment.ts` |
| 3% | 7 | 221 | `apps/api/cloudflare/worker.ts` | `packages/client-runtime/src/projectFaviconCache.ts` |
| 3% | 3 | 42 | `apps/desktop/src/backendNodeOptions.ts` | `apps/web/src/pendingUserInput.ts` |
| 3% | 4 | 89 | `apps/server/src/agentGateway/computerSpaceTools.test.ts` | `apps/desktop/src/snapShot/GnomeCaptureSetup.test.ts` |
| 3% | 4 | 69 | `apps/server/src/logger.ts` | `infra/relay/src/observability.ts` |
| 3% | 3 | 74 | `apps/web/src/lib/hosts/syncKeyPairing.test.ts` | `apps/desktop/src/snapShot/captureConfigEdit.test.ts` |
| 3% | 36 | 2959 | `apps/server/src/git/Layers/GitCore.test.ts` | `apps/server/src/checkpointing/CheckpointStore.test.ts` |
| 3% | 2 | 68 | `apps/web/src/components/ProviderUsagePanelContent.browser.tsx` | `apps/web/src/components/diffs/DiffFileTree.test.tsx` |
| 3% | 7 | 40 | `apps/web/src/components/chat/ComposerVoiceButton.tsx` | `apps/web/src/components/device/DeviceControlsRail.tsx` |
| 3% | 2 | 34 | `apps/web/src/components/chat/auxiliary/useAuxiliarySurface.ts` | `apps/mobile/src/features/settings/settings-scoped-server.ts` |
| 3% | 5 | 51 | `apps/web/src/hooks/useWindowFolderDrop.ts` | `apps/web/src/components/AppSidebarLayout.tsx` |
| 3% | 2 | 34 | `packages/shared/src/automationRunLifecycle.test.ts` | `apps/desktop/src/linuxSecretStorage.test.ts` |
| 3% | 10 | 117 | `apps/analytics/src/views/Issues.tsx` | `apps/web/src/components/settings/CustomModelEditor.tsx` |
| 3% | 8 | 172 | `apps/desktop/src/runningChatsQuitGuard.ts` | `apps/server/src/orchestration/ActivityPayloadProjection.ts` |
| 3% | 6 | 197 | `apps/server/src/eventLoopMonitor.test.ts` | `apps/mobile/src/features/devices/device-stream.browser.test.ts` |
| 3% | 5 | 68 | `apps/server/src/externalMcp/overview.test.ts` | `apps/mobile/src/features/home/homeThreadList.test.ts` |
| 3% | 35 | 169 | `apps/server/src/orchestration/Layers/StreamingPersistence.benchmark.test.ts` | `apps/server/src/orchestration/Layers/OrchestrationEngine.test.ts` |
| 3% | 3 | 23 | `apps/web/src/components/tasks/TasksViewSwitch.tsx` | `apps/web/src/components/ThreadRouteView.tsx` |
| 3% | 4 | 67 | `apps/web/src/hooks/useSidebarThreadOrderHold.ts` | `apps/web/src/remoteOpen.ts` |
| 3% | 17 | 833 | `packages/shared/src/model.test.ts` | `packages/shared/src/model.test.ts` |
| 3% | 3 | 62 | `packages/shared/src/path.test.ts` | `apps/mobile/src/features/threads/thread-settings-sheet-state.test.ts` |
| 3% | 4 | 83 | `apps/desktop/scripts/source-desktop-launch.mjs` | `apps/server/src/resourceTelemetry/ResourceMonitorBinary.ts` |
| 3% | 4 | 128 | `apps/server/src/computer/computerApprovalLease.test.ts` | `apps/web/src/components/files/projectFilesQueryState.test.tsx` |
| 3% | 27 | 380 | `apps/server/src/pullRequestAutoFix/Layers/PullRequestAutoFixService.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.ts` |
| 3% | 5 | 183 | `apps/web/src/components/SafariAccessOnboarding.browser.tsx` | `apps/web/src/localApi.test.ts` |
| 3% | 7 | 69 | `apps/web/src/components/chat/ComposerPullRequestAutoFixHint.tsx` | `apps/web/src/components/chat/ChatHeader.tsx` |
| 3% | 4 | 43 | `apps/api/src/rateLimit.ts` | `apps/desktop/scripts/wait-for-resources.mjs` |
| 3% | 5 | 83 | `apps/desktop/src/browserAutomation/electronCookieSession.ts` | `apps/mobile/src/lib/composerAttachmentUploadQueue.ts` |
| 3% | 16 | 165 | `apps/marketing/src/components/InstallOptions.tsx` | `apps/web/src/components/pullRequest/PullRequestSummaryTab.tsx` |
| 3% | 1 | 18 | `apps/server/src/agentGateway/toolGuidanceCadence.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 1 | 18 | `apps/server/src/computer/computerTaskContext.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 1 | 18 | `apps/server/src/persistence/sqliteMemoryBudget.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 1 | 18 | `apps/server/src/provider/debugMode.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 1 | 18 | `apps/web/src/components/chat/explorerListNavigation.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 1 | 18 | `apps/web/src/lib/folderDrop.ts` | `apps/desktop/gnome-extension/captureService.js` |
| 3% | 1 | 18 | `apps/web/src/lib/fontFamily.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 1 | 31 | `apps/web/src/repoDiffScopeStore.test.ts` | `apps/web/src/cloud/dpop.test.ts` |
| 3% | 1 | 18 | `packages/shared/src/uiLocalizationResources.ts` | `apps/mobile/src/features/devices/device-stream-document.ts` |
| 3% | 14 | 227 | `apps/server/src/agentGateway/diagnosticCursor.test.ts` | `apps/server/test/ActivityPayloadProjection.test.ts` |
| 3% | 10 | 86 | `apps/web/src/components/chat/PastedTextChip.tsx` | `apps/web/src/components/pullRequest/PullRequestReviewAnnotation.tsx` |
| 3% | 8 | 385 | `apps/web/src/components/chat/useChatKeyboardShortcuts.test.ts` | `apps/web/src/components/ProjectFavicon.test.tsx` |
| 3% | 9 | 111 | `apps/web/src/lib/threadSnooze.test.ts` | `packages/client-runtime/src/state/entities.test.ts` |
| 3% | 4 | 153 | `apps/marketing/scripts/marketing-copy.test.mjs` | `apps/mobile/src/features/usage/usageProviders.ts` |
| 3% | 3 | 113 | `apps/server/src/agentGateway/sanitizeToolInputSchema.test.ts` | `oxlint-plugin-t3code/rules/no-manual-effect-runtime-in-tests.ts` |
| 3% | 2 | 38 | `apps/server/src/orchestration/orchestrationAdmission.test.ts` | `apps/desktop/src/preview/PreviewKeyboard.test.ts` |
| 3% | 6 | 101 | `apps/server/src/orchestration/projectImportHistory.ts` | `apps/server/src/project/WorktreeSetupTracker.ts` |
| 3% | 2 | 39 | `apps/desktop/src/exclusiveApplyQueue.test.ts` | `apps/mobile/src/components/FilePreviewModal.tsx` |
| 3% | 17 | 538 | `apps/server/src/provider/acp/AcpSessionRuntime.test.ts` | `apps/server/src/provider/acp/AcpJsonRpcConnection.test.ts` |
| 3% | 4 | 90 | `apps/web/src/components/ChatMarkdown.tables.browser.tsx` | `apps/desktop/src/snapShot/KdeSnapShot.ts` |
| 3% | 10 | 201 | `apps/web/src/components/LocalVideoPreview.tsx` | `apps/web/src/components/chat/ComposerBannerStack.tsx` |
| 3% | 4 | 162 | `apps/web/src/components/chat/DockExplorerPane.browser.tsx` | `apps/web/src/branding.test.ts` |
| 3% | 129 | 2550 | `apps/web/src/components/chat/MessagesTimeline.logic.test.ts` | `apps/web/src/components/chat/MessagesTimeline.logic.test.ts` |
| 3% | 10 | 53 | `apps/web/src/components/settings/AppSnapPermissionGuide.tsx` | `apps/web/src/components/chat/WorktreeSetupCard.tsx` |
| 3% | 3 | 93 | `packages/contracts/src/mcpConnectors.ts` | `packages/contracts/src/device.ts` |
| 3% | 2 | 18 | `scripts/lib/portable-build-archive.test.ts` | `apps/desktop/src/snapShot/captureConfigEdit.test.ts` |
| 3% | 3 | 40 | `apps/desktop/src/backendCompileCache.test.ts` | `apps/desktop/src/snapShot/HyprlandSnapShot.test.ts` |
| 3% | 18 | 236 | `apps/server/src/pullRequests/Layers/PullRequestService.test.ts` | `apps/server/src/orchestration/PullRequestSyncReactor.test.ts` |
| 3% | 7 | 43 | `apps/web/src/components/DesktopUpdateRailButton.tsx` | `apps/web/src/components/sidebar/SidebarUpdatePill.tsx` |
| 3% | 3 | 20 | `apps/web/src/components/EditorDirtyRouteGuard.tsx` | `apps/web/src/components/ProjectCloneToastCoordinator.tsx` |
| 3% | 3 | 50 | `apps/web/src/components/codeEditor/CodeDiffEditorPane.tsx` | `apps/web/src/components/chat/ComposerCommandMenu.tsx` |
| 3% | 6 | 36 | `apps/web/src/confirmedCustomBinaryPathStore.ts` | `apps/web/src/uiStateStore.ts` |
| 3% | 16 | 1075 | `apps/web/src/wsNativeApi.ts` | `apps/web/src/localApi.ts` |
| 3% | 7 | 228 | `apps/server/src/computer/scrollCalibration.ts` | `packages/shared/src/qrCode.ts` |
| 3% | 4 | 45 | `apps/web/src/components/chat-drop-overlay/ChatPaneDropOverlay.test.tsx` | `packages/client-runtime/src/state/threadSnoozed.test.ts` |
| 3% | 1 | 19 | `apps/server/src/hostAuth/apiJwksCache.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 1 | 21 | `apps/server/src/persistence/Services/TodoRepository.ts` | `apps/server/src/persistence/Services/ProjectionProjects.ts` |
| 3% | 5 | 60 | `apps/web/src/components/SidebarStatusTrailingGlyph.tsx` | `apps/web/src/components/pullRequest/ThreadPullRequestsPanel.tsx` |
| 3% | 1 | 19 | `apps/web/src/components/profile/profileFormatting.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 6 | 31 | `apps/web/src/components/pullRequest/PullRequestTimelineTab.tsx` | `apps/web/src/components/pullRequest/PullRequestReviewAnnotation.tsx` |
| 3% | 2 | 33 | `apps/web/src/components/useRouteSpaceSync.browser.tsx` | `apps/web/src/components/pullRequest/PullRequestMarkdownEditor.tsx` |
| 3% | 1 | 19 | `apps/web/src/lib/keepAwake.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 1 | 19 | `apps/web/src/lib/localFoldersGroupLabel.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 1 | 19 | `apps/web/src/lib/relativeTime.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 1 | 17 | `packages/contracts/src/asyncUserInput.ts` | `oxlint-plugin-t3code/rules/no-inline-schema-compile.test.ts` |
| 3% | 10 | 83 | `packages/contracts/src/githubInbox.test.ts` | `apps/server/src/pullRequest/GitHubPullRequestProvider.test.ts` |
| 3% | 1 | 27 | `scripts/lib/desktop-artifact-naming.ts` | `apps/web/src/cloud/connectCliAuth.ts` |
| 3% | 22 | 209 | `apps/server/integration/overlappingTurnLifecycle.integration.test.ts` | `apps/server/integration/orchestrationEngine.integration.test.ts` |
| 3% | 42 | 599 | `apps/server/src/orchestration/importThreadRoute.ts` | `apps/server/src/orchestration/Layers/ProviderCommandReactor.ts` |
| 3% | 4 | 172 | `apps/web/src/components/chat/MessageTrail.browser.tsx` | `apps/mobile/src/features/diffs/nativeReviewDiffSurface.test.ts` |
| 3% | 26 | 310 | `apps/web/src/projectImport/ProjectImportPanel.tsx` | `apps/web/src/components/GitActionsControl.tsx` |
| 3% | 4 | 207 | `apps/api/src/config.test.ts` | `packages/client-runtime/src/work-log/presentation.test.ts` |
| 3% | 2 | 38 | `apps/desktop/src/backendNodeOptions.test.ts` | `apps/desktop/src/linuxSecretStorage.test.ts` |
| 3% | 3 | 21 | `apps/marketing/src/app/not-found.tsx` | `apps/web/src/components/sidebar/SidebarChrome.tsx` |
| 3% | 4 | 24 | `apps/server/src/shell-command-detection.test.ts` | `apps/server/src/provider/acp/XAiAcpExtension.test.ts` |
| 3% | 39 | 195 | `apps/web/src/components/GitCommitDialog.tsx` | `apps/web/src/components/GitActionsControl.tsx` |
| 3% | 4 | 63 | `apps/web/src/components/announcementSheetSlot.ts` | `apps/web/src/browser/browserSurfaceStore.ts` |
| 3% | 2 | 70 | `apps/web/src/components/codeEditor/CodeEditorPane.pierre.browser.tsx` | `apps/web/src/components/pullRequest/PullRequestCopyableCode.tsx` |
| 3% | 6 | 308 | `apps/web/src/lib/hugeicons.tsx` | `apps/web/src/components/EnvironmentMachineIcon.tsx` |
| 3% | 3 | 65 | `apps/web/src/components/terminal/terminalRuntimeRegistry.test.ts` | `apps/mobile/src/lib/composerContextClipboard.test.ts` |
| 3% | 5 | 19 | `apps/web/src/lib/revealFolder.ts` | `apps/web/src/components/files/FileBrowserPanel.tsx` |
| 3% | 45 | 1233 | `apps/web/src/routes/-automations.shared.tsx` | `apps/web/src/components/pullRequest/PullRequestDetailPanel.tsx` |
| 3% | 7 | 99 | `apps/server/src/projectAgent/Layers/ProjectAgentReactor.ts` | `apps/desktop/src/backend/DesktopBackendManager.ts` |
| 3% | 13 | 138 | `apps/web/src/components/chat/SplitPaneLayout.tsx` | `apps/web/src/components/ui/toast.tsx` |
| 3% | 10 | 74 | `apps/web/src/components/ui/time-picker.tsx` | `apps/web/src/components/diffs/DiffFileTree.tsx` |
| 3% | 9 | 199 | `apps/server/src/completionNotificationPreviews.test.ts` | `apps/server/src/provider/Layers/ProviderSessionDirectory.test.ts` |
| 3% | 8 | 194 | `apps/server/src/persistence/Migrations/automationProviderIdentity.ts` | `apps/server/src/orchestration/ActivityPayloadProjection.ts` |
| 3% | 1 | 20 | `apps/server/src/provider/piTurnFailure.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 4 | 136 | `apps/server/src/provider/providerStartupLifecycle.ts` | `packages/shared/src/agentAwareness.ts` |
| 3% | 7 | 129 | `apps/server/src/threadSearch.test.ts` | `apps/server/src/auth/EnvironmentAuth.test.ts` |
| 3% | 8 | 151 | `apps/web/src/components/chat/useChatProviderStatus.ts` | `apps/web/src/components/chat/TraitsPicker.tsx` |
| 3% | 3 | 91 | `apps/web/src/components/hosts/WorkspacePanels.browser.tsx` | `packages/client-runtime/src/state/usage.ts` |
| 3% | 1 | 4 | `apps/web/src/lib/accountBillingFlag.ts` | `apps/web/src/components/settings/useAvailableSettingsSearchItems.ts` |
| 3% | 1 | 20 | `apps/web/src/lib/matchHighlight.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 5 | 134 | `apps/server/src/computer/screenshotFrames.test.ts` | `apps/mobile/src/lib/composerImages.test.ts` |
| 3% | 5 | 267 | `apps/server/src/editor/typescriptService.test.ts` | `apps/web/src/modelSelection.test.ts` |
| 3% | 5 | 80 | `apps/server/src/siteFaviconCache.ts` | `packages/shared/src/git.ts` |
| 3% | 7 | 42 | `apps/web/src/components/chat/environment/EnvironmentAutomationsSection.tsx` | `apps/web/src/components/pullRequest/pullRequestPresentation.tsx` |
| 3% | 10 | 255 | `apps/desktop/src/browserAnnotations/protocol.ts` | `apps/web/src/vscodeThemeImport.ts` |
| 3% | 8 | 119 | `apps/server/src/atomicWrite.ts` | `apps/server/src/serviceLauncher.ts` |
| 3% | 5 | 55 | `apps/web/src/components/GitDialogChrome.test.tsx` | `packages/shared/src/git.test.ts` |
| 3% | 5 | 118 | `apps/web/src/components/chat/ComposerRow.browser.tsx` | `apps/web/src/components/clerk/T3ConnectUserProfilePage.tsx` |
| 3% | 3 | 72 | `apps/web/src/components/profile/profileTestFixtures.ts` | `packages/contracts/src/provider.test.ts` |
| 3% | 2 | 40 | `apps/web/src/hooks/useComputerDesktopControl.test.tsx` | `apps/web/src/components/chat/composerEventScope.test.ts` |
| 3% | 4 | 124 | `apps/api/src/liveActivities/apns.ts` | `packages/shared/src/Net.test.ts` |
| 3% | 4 | 93 | `apps/desktop/scripts/session-restore-smoke.ts` | `packages/shared/src/dpop.test.ts` |
| 3% | 4 | 95 | `apps/desktop/src/linuxCuaDriverHost.test.ts` | `apps/desktop/src/app/DesktopAppActivation.test.ts` |
| 3% | 3 | 127 | `apps/server/src/hostConnections/port.test.ts` | `apps/mobile/src/features/terminal/terminalPaste.test.ts` |
| 3% | 3 | 95 | `apps/server/src/provider/acp/DevinSessionConfig.test.ts` | `apps/desktop/src/app/DesktopAppActivation.test.ts` |
| 3% | 13 | 144 | `apps/server/src/transcriptUsage.test.ts` | `apps/server/src/usage/usageTranscripts.test.ts` |
| 3% | 13 | 51 | `apps/web/src/components/chat/ComputerActionCard.tsx` | `apps/web/src/components/settings/SourceControlSettings.tsx` |
| 3% | 19 | 111 | `apps/web/src/components/pullRequest/PrStateChip.tsx` | `apps/web/src/components/RightPanelTabs.tsx` |
| 3% | 20 | 233 | `apps/web/src/components/settings/AdvancedSettingsPanel.tsx` | `apps/web/src/components/GitActionsControl.tsx` |
| 3% | 1 | 25 | `apps/api/src/avatarStorage.test.ts` | `packages/client-runtime/src/device/screenshot.test.ts` |
| 3% | 6 | 159 | `apps/desktop/src/safariAccessIpc.test.ts` | `apps/web/src/authBootstrap.test.ts` |
| 3% | 37 | 151 | `apps/server/src/orchestration/Layers/ThreadHistory.benchmark.test.ts` | `apps/server/src/orchestration/Layers/OrchestrationEngine.test.ts` |
| 3% | 4 | 50 | `apps/server/src/persistence/MigrationDivergenceConsent.ts` | `apps/mobile/src/components/ComposerEditor.tsx` |
| 3% | 1 | 19 | `apps/server/src/projectAgent/Errors.ts` | `oxlint-plugin-t3code/rules/no-inline-schema-compile.test.ts` |
| 3% | 1 | 21 | `apps/server/src/providerUsage/credentials.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 5 | 75 | `apps/server/src/providerUsage/providers/cursorCache.test.ts` | `apps/web/src/localApi.test.ts` |
| 3% | 2 | 24 | `apps/server/src/studioWorkspaceScaffold.test.ts` | `apps/server/src/provider/acp/CursorAcpSupport.test.ts` |
| 3% | 1 | 21 | `apps/web/src/bootstrap.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 1 | 32 | `apps/web/src/hooks/useThrottledStreamingValue.browser.tsx` | `packages/shared/src/threadReference.ts` |
| 3% | 5 | 187 | `apps/web/src/lib/accountUsageAdapter.test.ts` | `apps/web/src/components/chat/ContextWindowMeter.logic.test.ts` |
| 3% | 5 | 74 | `apps/web/src/lib/hosts/workspaceCommands.ts` | `apps/mobile/src/features/sharing/IncomingShareProvider.tsx` |
| 3% | 1 | 23 | `apps/web/src/threadGoal.test.ts` | `packages/client-runtime/src/threadPullRequestCompatibility.test.ts` |
| 3% | 1 | 4 | `packages/shared/src/githubAvatar.ts` | `packages/shared/src/favicon.ts` |
| 3% | 1 | 21 | `packages/shared/src/runtimeMode.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 4 | 57 | `apps/web/src/voidSpaceStore.ts` | `apps/web/src/browserFaviconStore.ts` |
| 3% | 4 | 79 | `apps/desktop/scripts/betterwright-asar-smoke.mjs` | `apps/desktop/src/permissions/MacPermissionHelper.ts` |
| 3% | 7 | 214 | `apps/desktop/src/browserAutomation/browserSessionRestore.ts` | `apps/desktop/src/snapShot/CaptureShortcutConfig.ts` |
| 3% | 11 | 73 | `apps/server/src/automation/Layers/AutomationRunReactor.ts` | `apps/server/src/relay/AgentAwarenessRelay.ts` |
| 3% | 4 | 57 | `apps/server/src/computer/computerAuditFile.ts` | `apps/web/src/components/search/HighlightedSearchLine.tsx` |
| 3% | 6 | 125 | `apps/server/src/editor/languageService.ts` | `apps/desktop/src/snapShot/NiriSnapShot.ts` |
| 3% | 3 | 171 | `apps/desktop/native/appsnap/WindowFrameStream.swift` | `apps/mobile/plugins/withIosSceneLifecycle.cjs` |
| 3% | 4 | 95 | `apps/server/src/codexAuthFingerprint.test.ts` | `apps/web/src/components/preview/openTerminalLinkInPreview.test.ts` |
| 3% | 3 | 50 | `apps/server/src/git/gitBlameParsing.ts` | `packages/shared/src/toolActivity.ts` |
| 3% | 2 | 20 | `apps/server/src/inlineVisualStore.test.ts` | `apps/desktop/scripts/electron-launcher.test.mjs` |
| 3% | 21 | 1416 | `apps/server/src/persistence/Layers/ProjectAgentRepository.ts` | `apps/server/src/persistence/Layers/OrchestrationEventStore.ts` |
| 3% | 3 | 67 | `apps/server/src/wsConnectionSessions.test.ts` | `apps/server/src/mcp/toolkits/preview/handlers.ts` |
| 3% | 18 | 288 | `apps/web/src/components/chat/project/GroupOverview.tsx` | `apps/web/src/components/settings/KeybindingsSettings.tsx` |
| 3% | 4 | 42 | `apps/web/src/lib/composerSendOwnership.test.ts` | `apps/web/src/terminalUiStateStore.test.ts` |
| 3% | 12 | 185 | `packages/shared/src/DrainableWorker.ts` | `apps/desktop/src/backend/DesktopBackendManager.ts` |
| 3% | 2 | 100 | `packages/shared/src/providerCliProfiles.ts` | `apps/mobile/src/features/usage/usageProviders.ts` |
| 3% | 3 | 38 | `apps/server/src/computer/computerGeometry.test.ts` | `apps/web/src/state/threads.test.ts` |
| 3% | 7 | 239 | `apps/server/src/trustedOrigins.test.ts` | `packages/client-runtime/src/state/server.test.ts` |
| 3% | 9 | 279 | `apps/web/src/lib/composerPrediction.test.ts` | `apps/desktop/src/snapShot/snapShot.test.ts` |
| 3% | 12 | 190 | `apps/web/src/onboarding/steps/PairStep.tsx` | `apps/web/src/components/RightPanelTabs.tsx` |
| 3% | 8 | 55 | `apps/web/src/components/chat/ComposerSnoozeNotice.tsx` | `apps/web/src/components/settings/DeviceHostsSettings.tsx` |
| 3% | 18 | 518 | `apps/web/src/components/chat/useChatTranscriptScroll.ts` | `apps/mobile/src/features/threads/ThreadDetailScreen.tsx` |
| 3% | 13 | 95 | `apps/web/src/hooks/useActivityThreads.test.ts` | `apps/server/src/orchestration/PullRequestSyncReactor.test.ts` |
| 3% | 4 | 155 | `apps/api/src/remote/pairing.ts` | `apps/desktop/src/app/DesktopAppActivationBroker.ts` |
| 3% | 6 | 218 | `apps/marketing/src/app/privacy/page.tsx` | `apps/web/src/components/sidebar/SidebarChrome.tsx` |
| 3% | 4 | 128 | `apps/server/src/agentGateway/computerBrowserEffect.test.ts` | `apps/web/src/components/ThreadStatusIndicators.test.ts` |
| 3% | 1 | 11 | `apps/server/src/backoff.ts` | `apps/web/src/components/pullRequest/pullRequestFilesViewed.logic.ts` |
| 3% | 2 | 89 | `apps/server/src/computer/computerCallContext.ts` | `scripts/lib/build-target-arch.ts` |
| 3% | 1 | 22 | `apps/server/src/mcpConnectors/pairedClientView.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 21 | 303 | `apps/web/src/components/hosts/ComputerPicker.tsx` | `apps/web/src/components/settings/IntegrationsSettings.tsx` |
| 3% | 3 | 74 | `apps/web/src/lib/contextMenuFallback.browser.ts` | `apps/web/src/components/threadActionMenu.logic.ts` |
| 3% | 1 | 22 | `apps/web/src/lib/shellQuote.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 5 | 84 | `apps/web/src/workflowRunUiStore.test.ts` | `apps/web/src/components/desktop/SnapShotCoordinator.test.ts` |
| 3% | 1 | 35 | `packages/shared/src/inlineVisualFormula.test.ts` | `apps/web/src/cloud/dpop.test.ts` |
| 3% | 3 | 29 | `packages/shared/src/turnResume.ts` | `apps/mobile/src/state/use-thread-selection.ts` |
| 3% | 4 | 93 | `apps/api/src/avatarStorage.ts` | `apps/mobile/src/lib/composerContextClipboard.ts` |
| 3% | 12 | 310 | `apps/server/src/provider/Layers/OmpAdapter.live.test.ts` | `apps/server/src/textGeneration/AntigravityTextGeneration.test.ts` |
| 3% | 4 | 64 | `apps/server/src/workspaceFileChanges.lifecycle.test.ts` | `apps/web/src/components/files/useFileSaveCoordinator.test.tsx` |
| 3% | 14 | 44 | `apps/web/src/components/chat/AttachmentSummaryChip.tsx` | `apps/web/src/components/pullRequest/PullRequestSummaryTab.tsx` |
| 3% | 4 | 20 | `benchmarks/desktop-perf/lib/cover-window.cjs` | `apps/desktop/src/permissions/MacPermissionHelper.ts` |
| 3% | 2 | 54 | `apps/desktop/src/desktopQuitIntent.test.ts` | `apps/desktop/src/app/DesktopEarlyElectronStartup.ts` |
| 3% | 2 | 78 | `apps/marketing/scripts/theme-smoke.mjs` | `apps/mobile/scripts/permissions-service.test.ts` |
| 3% | 25 | 305 | `apps/server/src/externalMcp/Layers/ExternalMcpComputerControl.test.ts` | `apps/server/src/relay/AgentAwarenessRelay.test.ts` |
| 3% | 35 | 131 | `apps/server/src/orchestration/Layers/threadDetailSnapshotFence.test.ts` | `apps/server/src/orchestration/Layers/OrchestrationEngine.test.ts` |
| 3% | 30 | 646 | `apps/web/src/components/githubInbox/GitHubInbox.browser.tsx` | `apps/web/src/components/pullRequest/pullRequestDetail.logic.test.ts` |
| 3% | 7 | 166 | `packages/shared/src/KeyedSingleFlightCache.ts` | `apps/mobile/src/persistence/mobile-preferences.ts` |
| 3% | 4 | 100 | `apps/api/src/clientIp.test.ts` | `apps/desktop/src/snapShot/snapShot.test.ts` |
| 3% | 3 | 27 | `apps/desktop/src/bundleSwapDetection.ts` | `apps/server/src/usage/usageTranscriptReader.ts` |
| 3% | 3 | 69 | `apps/profiles/src/lib/heatmapCells.ts` | `packages/shared/src/searchRanking.ts` |
| 3% | 26 | 376 | `apps/web/src/components/DiffPanel.logic.test.ts` | `apps/web/src/components/ChatView.logic.test.ts` |
| 3% | 24 | 209 | `apps/web/src/components/chat/SurfaceContentTabs.tsx` | `apps/web/src/components/RightPanelTabs.tsx` |
| 3% | 24 | 252 | `apps/web/src/lib/threadRecap.test.ts` | `apps/web/src/components/ChatView.logic.test.ts` |
| 3% | 13 | 124 | `apps/web/src/providerUpdates.ts` | `apps/web/src/components/ProviderUpdateLaunchNotification.logic.ts` |
| 3% | 8 | 312 | `packages/contracts/src/browserAutomationTools.test.ts` | `apps/web/src/components/ChatView.logic.test.ts` |
| 3% | 5 | 192 | `apps/desktop/src/escapeKillSwitchMonitor.ts` | `apps/web/src/components/desktop/SnapShotCoordinator.tsx` |
| 3% | 14 | 621 | `apps/server/src/agentGateway/targetResolver.test.ts` | `apps/server/src/provider/Layers/OpenCodeProvider.ts` |
| 3% | 6 | 94 | `apps/server/src/provider/providerModelCatalogCache.test.ts` | `apps/web/src/promptStashStore.test.ts` |
| 3% | 5 | 145 | `apps/web/src/components/settings/ComputerSettingsPanel.test.tsx` | `apps/web/src/components/RightPanelTabs.test.tsx` |
| 3% | 3 | 32 | `apps/marketing/src/lib/docsLayout.tsx` | `apps/web/src/components/sidebar/SidebarChrome.tsx` |
| 3% | 4 | 260 | `apps/server/src/automation/schedule.test.ts` | `apps/web/src/lib/projectScriptKeybindings.test.ts` |
| 3% | 2 | 31 | `apps/server/src/computer/cuaSpaceInventory.ts` | `apps/desktop/src/app/DesktopAppActivationBroker.ts` |
| 3% | 3 | 101 | `apps/server/src/computer/cursorActivity.test.ts` | `apps/web/src/components/device/phoneTrackpad.test.ts` |
| 3% | 1 | 11 | `apps/server/src/editor/languageWorker.ts` | `apps/mobile/src/state/pending-task-editor-writes.ts` |
| 3% | 11 | 169 | `apps/server/src/provider/providerStatusCache.test.ts` | `apps/server/src/provider/Layers/CursorProvider.test.ts` |
| 3% | 1 | 23 | `apps/web/src/components/chat/TimelineWorkEntryRow.subagentProgress.test.tsx` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 9 | 467 | `apps/web/src/components/chat/group/groupSettingsDialog.logic.test.ts` | `apps/server/src/orchestration/ThreadSettlementPolicy.test.ts` |
| 3% | 6 | 95 | `apps/web/src/hooks/useWorkspaceFileEditor.ts` | `apps/mobile/src/state/queries.ts` |
| 3% | 1 | 12 | `apps/web/src/lib/hosts/workspaceReconnect.ts` | `apps/web/src/components/media/mediaContent.ts` |
| 3% | 7 | 113 | `apps/web/src/lib/workspaceFileOpener.ts` | `apps/web/src/components/settings/KeybindingsSettings.logic.ts` |
| 3% | 3 | 129 | `packages/contracts/src/externalMcp.ts` | `apps/server/src/persistence/Services/ProjectionThreads.ts` |
| 3% | 2 | 27 | `packages/shared/src/toolOutputSummary.test.ts` | `packages/client-runtime/src/threadPullRequestCompatibility.test.ts` |
| 3% | 4 | 33 | `apps/marketing/src/components/SiteFooter.tsx` | `apps/web/src/components/sidebar/SidebarChrome.tsx` |
| 3% | 3 | 214 | `apps/profiles/src/lib/profilePage.test.ts` | `packages/contracts/src/provider.test.ts` |
| 3% | 10 | 185 | `apps/web/src/components/RateLimitsPanel.test.tsx` | `apps/server/src/provider/Layers/codexUsageLimits.test.ts` |
| 3% | 13 | 56 | `apps/web/src/components/chat/BackgroundTaskRow.tsx` | `apps/web/src/components/pullRequest/PullRequestSummaryTab.tsx` |
| 3% | 5 | 127 | `apps/web/src/components/chat/group/libraryPanel.logic.test.ts` | `packages/client-runtime/src/work-log/presentation.test.ts` |
| 3% | 5 | 145 | `apps/web/src/components/ui/toastRouteVisibility.test.ts` | `apps/web/src/uiStateStore.test.ts` |
| 3% | 3 | 60 | `packages/shared/src/betaFeatures.ts` | `apps/server/src/environment/ServerEnvironment.ts` |
| 3% | 9 | 261 | `apps/server/src/agentGateway/inlineVisualTools.test.ts` | `apps/server/src/provider/Layers/EventNdjsonLogger.test.ts` |
| 3% | 4 | 186 | `apps/server/src/processArgumentRedaction.test.ts` | `apps/server/src/assets/AssetAccess.test.ts` |
| 3% | 3 | 22 | `apps/server/src/provider/Layers/ProviderStartInstanceRouting.test.ts` | `packages/contracts/src/provider.test.ts` |
| 3% | 2 | 31 | `apps/server/src/voiceUploadAdmission.test.ts` | `apps/mobile/src/state/composer-attachment-uploads.ts` |
| 3% | 29 | 533 | `apps/web/src/components/account/SignInDialog.tsx` | `apps/web/src/components/onboarding/WelcomeWizard.tsx` |
| 3% | 7 | 48 | `apps/web/src/components/chat/useAsyncUserInputResponse.ts` | `apps/server/integration/orphanedProviderSessionStartup.integration.test.ts` |
| 3% | 11 | 484 | `apps/web/src/lib/providerAvailability.test.ts` | `apps/web/src/providerInstances.test.ts` |
| 3% | 5 | 132 | `apps/desktop/src/backendStartupReadiness.test.ts` | `apps/web/src/browser/desktopTabLifetime.test.ts` |
| 3% | 6 | 94 | `apps/web/src/components/chat/runtimeModelCapabilities.test.ts` | `packages/shared/src/model.test.ts` |
| 3% | 32 | 759 | `packages/contracts/src/projectAgent.ts` | `packages/contracts/src/orchestration.ts` |
| 3% | 6 | 224 | `packages/shared/src/windowsProcess.test.ts` | `packages/shared/src/shell.test.ts` |
| 3% | 2 | 29 | `apps/desktop/scripts/cua-cache-key.test.mjs` | `packages/client-runtime/src/threadPullRequestCompatibility.test.ts` |
| 3% | 1 | 12 | `apps/desktop/scripts/synthetic-cookie-app.cjs` | `.github/scripts/relay-state-output.test.cjs` |
| 3% | 4 | 105 | `apps/server/src/agentGateway/httpRoute.ts` | `apps/server/src/device/DeviceHubProxy.ts` |
| 3% | 1 | 12 | `apps/server/src/automation/stopPolicy.ts` | `packages/client-runtime/src/threadPullRequestCompatibility.ts` |
| 3% | 1 | 24 | `apps/server/src/computer/computerActivation.ts` | `apps/desktop/gnome-extension/captureService.js` |
| 3% | 1 | 43 | `apps/server/src/computer/computerGeometry.ts` | `apps/web/src/commandPaletteBus.ts` |
| 3% | 1 | 24 | `apps/server/src/git/gitAttribution.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 4 | 86 | `apps/server/src/hostAuth/credential.ts` | `apps/mobile/src/features/cloud/linkEnvironment.ts` |
| 3% | 1 | 24 | `apps/web/src/betaFeatures.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 18 | 107 | `apps/web/src/components/BrowserTabStrip.tsx` | `apps/web/src/components/RightPanelTabs.tsx` |
| 3% | 1 | 24 | `apps/web/src/components/TerminalWorkspaceTabs.browser.tsx` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 1 | 22 | `apps/web/src/components/profile/useProfileAvatarColor.ts` | `oxlint-plugin-t3code/rules/no-inline-schema-compile.test.ts` |
| 3% | 4 | 26 | `apps/web/src/lib/computerProvisionRequest.test.ts` | `apps/mobile/src/widgets/subscriptionUsageSnapshot.test.ts` |
| 3% | 1 | 24 | `apps/web/src/lib/projectPaths.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 8 | 122 | `apps/marketing/src/components/ChangelogContent.tsx` | `apps/web/src/components/settings/settingsLayout.tsx` |
| 3% | 33 | 650 | `apps/server/src/orchestration/Layers/ThreadGitMetadataReactor.test.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.test.ts` |
| 3% | 55 | 2501 | `apps/web/src/components/EventRouter.browser.tsx` | `packages/client-runtime/src/state/threadReducer.test.ts` |
| 3% | 6 | 237 | `apps/desktop/src/escapeKillSwitchMonitor.test.ts` | `apps/desktop/src/snapShot/NativeCaptureFeedback.test.ts` |
| 3% | 4 | 83 | `apps/server/src/desktopParentLifetime.integration.test.ts` | `apps/server/src/project/AgentSessionJson.ts` |
| 3% | 8 | 532 | `apps/server/src/projectAgent/library.test.ts` | `apps/desktop/src/backend/DesktopBackendConfiguration.test.ts` |
| 3% | 8 | 276 | `apps/web/src/components/chat/ComputerPreviewPopover.logic.test.ts` | `packages/shared/src/usageLimits.test.ts` |
| 3% | 57 | 521 | `apps/web/src/components/pullRequest/PullRequestDetailPanel.tsx` | `apps/web/src/components/pullRequest/PullRequestDetailPanel.tsx` |
| 3% | 5 | 160 | `scripts/benchmark-cua-build.ts` | `apps/server/src/compileCache.test.ts` |
| 3% | 8 | 266 | `scripts/computer-use-fixtures/packaged-evidence.test.ts` | `apps/desktop/src/ipc/methods/preview.test.ts` |
| 3% | 4 | 88 | `apps/analytics/src/filters.ts` | `apps/web/src/remoteOpen.ts` |
| 3% | 4 | 90 | `apps/server/src/computer/scrollGearingFile.ts` | `apps/desktop/scripts/dev-electron.mjs` |
| 3% | 7 | 37 | `apps/web/src/components/DiagnosticReportAction.tsx` | `apps/web/src/components/cloud/CloudEnvironmentConnectList.tsx` |
| 3% | 2 | 35 | `apps/web/src/viteConfig.test.ts` | `apps/mobile/src/features/devices/device-stream.browser.test.ts` |
| 3% | 9 | 267 | `apps/server/src/externalMcp/httpRoute.ts` | `apps/server/src/http.ts` |
| 3% | 9 | 151 | `apps/server/src/persistence/Layers/QueuedTurnPromotions.ts` | `apps/server/src/persistence/Layers/ProjectionThreadMessages.ts` |
| 3% | 321 | 5909 | `apps/web/src/components/ChatView.tsx` | `apps/web/src/components/ChatView.tsx` |
| 3% | 7 | 104 | `apps/web/src/components/chat/useAppSnapWindows.ts` | `apps/web/src/components/cloud/CloudEnvironmentConnectList.tsx` |
| 3% | 9 | 76 | `apps/web/src/components/tasks/TaskCardSurface.tsx` | `apps/web/src/components/pullRequest/PullRequestTimelineTab.tsx` |
| 3% | 10 | 235 | `apps/web/src/lib/composerAutomation.ts` | `apps/server/src/relay/AgentAwarenessRelay.ts` |
| 3% | 5 | 74 | `packages/shared/src/chatThreads.test.ts` | `apps/mobile/src/state/pending-thread-creation.test.ts` |
| 3% | 1 | 25 | `apps/desktop/src/updateInstallPreparation.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 3 | 38 | `apps/desktop/src/windowsShellIco.test.ts` | `apps/mobile/src/components/ProjectFavicon.tsx` |
| 3% | 1 | 33 | `apps/marketing/src/data/sponsors.ts` | `apps/marketing/src/lib/releases.ts` |
| 3% | 1 | 11 | `apps/marketing/src/lib/scrollToAnchor.ts` | `apps/web/src/components/preview/AgentBrowserCursor.tsx` |
| 3% | 4 | 132 | `apps/server/native/device-helper/Sources/CapabilityProbe.swift` | `apps/mobile/scripts/fixtures/NotificationCenterManagerRegression.swift` |
| 3% | 5 | 73 | `apps/server/src/git/gitStatusParsing.test.ts` | `packages/shared/src/git.test.ts` |
| 3% | 1 | 45 | `apps/server/src/persistence/Services/RemoteDeviceTrust.ts` | `packages/shared/src/projectFavicon.ts` |
| 3% | 1 | 25 | `apps/server/src/provider/acp/AcpSessionRuntime.env.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 1 | 36 | `apps/web/src/components/chat/InlineVisualFrame.web.browser.tsx` | `apps/web/src/components/usage/usageShortcuts.test.ts` |
| 3% | 2 | 42 | `apps/web/src/components/chat/SidePanelOverlay.browser.tsx` | `scripts/install.test.ts` |
| 3% | 12 | 157 | `apps/web/src/components/chat/environment/EnvironmentSubagentsSection.logic.test.ts` | `packages/client-runtime/src/state/subagentRuntime.test.ts` |
| 3% | 2 | 11 | `apps/web/src/components/sidebarContextMenuStyles.tsx` | `apps/web/src/components/BranchToolbarEnvironmentSelector.tsx` |
| 3% | 11 | 335 | `apps/web/src/keybindingEditor.test.ts` | `apps/web/src/components/settings/KeybindingsSettings.logic.test.ts` |
| 3% | 14 | 489 | `apps/web/src/shortcutsSheet.ts` | `apps/web/src/keybindings.ts` |
| 3% | 1 | 25 | `packages/contracts/src/accountUsage.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 1 | 23 | `packages/contracts/src/claudeCache.ts` | `oxlint-plugin-t3code/rules/no-inline-schema-compile.test.ts` |
| 3% | 1 | 23 | `packages/contracts/src/providerDiscovery.test.ts` | `oxlint-plugin-t3code/rules/no-inline-schema-compile.test.ts` |
| 3% | 3 | 46 | `packages/shared/src/browserAutomationErrors.ts` | `apps/desktop/src/app/DesktopAppActivation.test.ts` |
| 3% | 1 | 25 | `packages/shared/src/desktopChrome.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 2 | 49 | `packages/shared/src/socialLinks.test.ts` | `apps/desktop/src/linuxSecretStorage.test.ts` |
| 3% | 5 | 187 | `scripts/check-brand-identity.test.ts` | `apps/desktop/src/preview/FaviconCapture.test.ts` |
| 3% | 1 | 13 | `scripts/lib/mac-after-sign.cjs` | `.github/scripts/relay-state-output.test.cjs` |
| 3% | 3 | 199 | `apps/desktop/native/appsnap/EscapeKillSwitchMonitor.swift` | `apps/mobile/scripts/fixtures/NotificationCenterManagerRegression.swift` |
| 3% | 7 | 57 | `apps/desktop/src/windowMaterial.ts` | `apps/desktop/src/snapShot/snapShot.ts` |
| 3% | 4 | 14 | `apps/web/src/components/kanban/NeedsReviewFilter.tsx` | `apps/web/src/components/cloud/ConnectOnboardingDialog.tsx` |
| 3% | 4 | 123 | `apps/desktop/src/browserAnnotations/elementInspection.ts` | `packages/shared/src/previewViewport.ts` |
| 3% | 7 | 295 | `apps/server/src/agentGateway/stdioProxyScript.test.ts` | `apps/server/src/serviceLauncher.ts` |
| 3% | 3 | 87 | `apps/server/src/computer/waitForControl.test.ts` | `packages/client-runtime/src/projectFaviconCache.test.ts` |
| 3% | 6 | 140 | `apps/server/src/platform/effectProcessSignals.test.ts` | `scripts/mobile-native-client.ts` |
| 3% | 18 | 835 | `apps/server/src/serverSettings.test.ts` | `apps/server/src/provider/Layers/ProviderRegistry.test.ts` |
| 3% | 2 | 36 | `apps/server/src/wsRpc.errors.test.ts` | `apps/desktop/src/app/DesktopAppActivationBroker.ts` |
| 3% | 2 | 90 | `apps/web/src/components/chat/ChatTranscriptPane.test.tsx` | `apps/web/src/components/clerk/T3ConnectUserProfilePage.test.tsx` |
| 3% | 12 | 85 | `apps/web/src/components/chat/ComposerChoiceRow.tsx` | `apps/web/src/components/pullRequest/PullRequestSummaryTab.tsx` |
| 3% | 28 | 348 | `benchmarks/desktop-perf/seed-home.ts` | `apps/server/src/orchestration/Layers/OrchestrationEngine.test.ts` |
| 3% | 7 | 110 | `packages/shared/src/windowsProcess.ts` | `apps/server/src/provider/providerMaintenance.ts` |
| 3% | 4 | 20 | `apps/api/src/testing/isolatedDatabase.ts` | `apps/desktop/src/snapShot/GnomeCaptureSetup.ts` |
| 3% | 4 | 111 | `apps/desktop/scripts/cua-artifact-provenance.test.mjs` | `apps/mobile/src/state/use-thread-pr.test.ts` |
| 3% | 7 | 373 | `apps/desktop/src/betaChannel.test.ts` | `apps/web/src/components/settings/ProviderSetupSection.test.tsx` |
| 3% | 3 | 124 | `apps/desktop/src/cuaRuntimeOwnership.test.ts` | `apps/server/src/device/DeviceHubProxy.test.ts` |
| 3% | 3 | 63 | `apps/server/src/groupWorkspaceScaffold.ts` | `apps/server/src/workspace/WorkspacePaths.ts` |
| 3% | 8 | 216 | `apps/server/src/hostConnections/httpRoute.ts` | `apps/web/src/connection/storage.ts` |
| 3% | 21 | 149 | `apps/web/src/components/ChatView.queuedAutoDispatch.test.ts` | `apps/web/src/components/ChatView.logic.test.ts` |
| 3% | 5 | 156 | `apps/web/src/components/chat/InlineVisualBlocks.browser.tsx` | `apps/web/src/components/usage/UsagePage.test.tsx` |
| 3% | 54 | 2006 | `apps/web/src/storeEventReducer.test.ts` | `apps/mobile/src/lib/threadActivity.test.ts` |
| 3% | 6 | 88 | `apps/desktop/src/browserAutomation/betterwrightNetworkGuard.ts` | `apps/desktop/src/snapShot/GnomeCaptureSetup.ts` |
| 3% | 6 | 213 | `apps/desktop/src/remoteResourceBroker.ts` | `apps/server/src/assets/GitHubMediaFetch.ts` |
| 3% | 1 | 26 | `apps/desktop/src/showInFileManager.test.ts` | `apps/desktop/gnome-extension/captureService.js` |
| 3% | 1 | 26 | `apps/server/src/computer/FakeComputerBackend.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 1 | 14 | `apps/server/src/computer/computerTaskContext.ts` | `.github/scripts/relay-state-output.test.cjs` |
| 3% | 10 | 311 | `apps/server/src/provider/codexDiscoveryCatalog.ts` | `apps/server/src/provider/Layers/GrokProvider.ts` |
| 3% | 2 | 49 | `apps/server/src/restoreMigrationBackup.test.ts` | `apps/desktop/src/app/DesktopAppActivation.test.ts` |
| 3% | 4 | 129 | `apps/server/src/serverStatusCli.test.ts` | `packages/client-runtime/src/authorization/remote.test.ts` |
| 3% | 4 | 40 | `apps/server/src/studioWorkspaceScaffold.ts` | `apps/server/src/project/ProjectFaviconResolver.ts` |
| 3% | 1 | 26 | `apps/web/src/lib/diffEditBaseRev.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 3 | 128 | `apps/web/src/lib/markdownTableLayout.test.ts` | `apps/desktop/src/preview/PreviewKeyboard.test.ts` |
| 3% | 1 | 26 | `apps/web/src/lib/markdownTaskList.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 2 | 37 | `apps/web/src/lib/projectCatalog/navigation.ts` | `apps/desktop/src/app/DesktopAppActivationBroker.ts` |
| 3% | 13 | 343 | `apps/web/src/onboarding/OnboardingDialog.tsx` | `apps/web/src/components/RightPanelTabs.tsx` |
| 3% | 1 | 24 | `packages/contracts/src/hostSessions.ts` | `oxlint-plugin-t3code/rules/no-inline-schema-compile.test.ts` |
| 3% | 5 | 101 | `packages/shared/src/executable.test.ts` | `apps/web/src/browserFaviconStore.test.ts` |
| 3% | 1 | 39 | `scripts/typecheck.test.ts` | `apps/web/src/cloud/dpop.test.ts` |
| 3% | 8 | 265 | `apps/server/src/mcpConnectors/proxy.ts` | `apps/server/src/usage/cursorUsageReader.ts` |
| 3% | 4 | 25 | `apps/server/src/persistence/Migrations/schemaHelpers.ts` | `apps/server/src/persistence/Layers/ProjectionThreadMessages.ts` |
| 3% | 8 | 66 | `apps/web/src/components/chat/environment/EnvironmentSidechatsSection.tsx` | `apps/web/src/components/AgentsPanel.tsx` |
| 3% | 26 | 695 | `apps/web/src/components/githubInbox/GitHubInboxAgentActions.browser.tsx` | `apps/web/src/components/pullRequest/pullRequestDetail.logic.test.ts` |
| 3% | 5 | 149 | `apps/web/src/hooks/useAccount.ts` | `apps/server/src/project/WorktreeSetupTracker.ts` |
| 3% | 12 | 660 | `apps/web/src/hooks/useProviderModelCatalog.test.tsx` | `apps/web/src/components/settings/ProviderSettingsPanel.environment.test.tsx` |
| 3% | 11 | 280 | `apps/web/src/lib/threadBootstrap.ts` | `apps/mobile/src/state/use-thread-composer-state.ts` |
| 3% | 11 | 260 | `apps/analytics/src/views/IssueDetail.tsx` | `apps/web/src/components/usage/UsagePage.tsx` |
| 3% | 5 | 230 | `apps/desktop/src/reliabilityReporter.test.ts` | `apps/web/src/components/files/fileSaveCoordinator.test.ts` |
| 3% | 7 | 138 | `apps/server/src/computer/waitForWindow.test.ts` | `apps/desktop/src/snapShot/snapShot.test.ts` |
| 3% | 1 | 45 | `apps/server/src/remoteFeaturePolicy.ts` | `packages/client-runtime/src/work-log/toolPresentation.ts` |
| 3% | 4 | 187 | `apps/server/src/terminal/providerTerminalProfiles.ts` | `apps/mobile/src/features/settings/settings-scoped-server.ts` |
| 3% | 2 | 25 | `apps/server/src/wsOwnerAuthorization.test.ts` | `apps/server/src/orchestration/commandInvariants.test.ts` |
| 3% | 2 | 48 | `apps/web/src/components/chat/useDockPaneRuntimeActivation.browser.tsx` | `apps/mobile/src/features/files/sourceHighlightingState.test.ts` |
| 3% | 15 | 97 | `apps/web/src/components/device/DeviceControlRail.tsx` | `apps/web/src/components/settings/ProviderInstanceCard.tsx` |
| 3% | 13 | 109 | `apps/web/src/hooks/useComposerPrediction.ts` | `apps/web/src/components/BranchToolbarBranchSelector.tsx` |
| 3% | 2 | 46 | `apps/web/src/hooks/useOptimisticTabSelection.ts` | `apps/web/src/components/pullRequest/PullRequestLabelPicker.tsx` |
| 3% | 3 | 122 | `apps/web/src/lib/automationInlineDraft.test.ts` | `apps/web/src/components/permissions/usePermissionStatus.test.ts` |
| 3% | 3 | 35 | `apps/web/src/lib/automationStatus.test.ts` | `packages/client-runtime/src/providerSkills.test.ts` |
| 3% | 7 | 252 | `apps/web/src/lib/subagentPresentation.test.ts` | `apps/server/test/ActivityPayloadProjection.test.ts` |
| 3% | 4 | 60 | `apps/server/src/orchestration/threadGitMetadata.ts` | `apps/mobile/src/state/use-selected-thread-git-actions.ts` |
| 3% | 14 | 101 | `apps/web/src/components/pullRequest/pullRequestDetail.logic.ts` | `apps/web/src/components/pullRequest/pullRequestDetail.logic.ts` |
| 3% | 7 | 29 | `apps/web/src/onboarding/stage/FitStage.tsx` | `apps/web/src/components/device/DeviceStreamView.tsx` |
| 3% | 9 | 136 | `apps/web/src/pinnedMessages.ts` | `apps/mobile/src/state/thread-outbox-model.ts` |
| 3% | 4 | 145 | `packages/shared/src/releaseDownloads.test.ts` | `apps/web/src/components/chat/composerAttachmentFiles.test.ts` |
| 3% | 2 | 35 | `apps/desktop/src/browserAutomation/betterwrightLocator.test.ts` | `packages/client-runtime/src/threadPullRequestCompatibility.test.ts` |
| 3% | 1 | 27 | `apps/desktop/src/desktopProcessErrors.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 3 | 93 | `apps/marketing/src/app/layout.tsx` | `apps/mobile/modules/t3-markdown-text/src/MarkdownTextPrimitive.tsx` |
| 3% | 4 | 57 | `apps/server/src/cloudflare/ingress.ts` | `apps/server/src/cli/app.test.ts` |
| 3% | 1 | 27 | `apps/server/src/hostAuth/replayCache.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 1 | 27 | `apps/server/src/projectAgent/lifecycle.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 1 | 27 | `apps/web/src/components/chat/composerOverlay.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 10 | 236 | `apps/web/src/components/device/useDeviceVideoStream.ts` | `packages/client-runtime/src/device/stream.ts` |
| 3% | 2 | 49 | `apps/web/src/components/pullRequest/PullRequestMarkdown.browser.tsx` | `apps/web/src/components/pullRequest/PullRequestMarkdownEditor.tsx` |
| 3% | 17 | 510 | `apps/web/src/components/settings/ProfileSettingsPanel.tsx` | `apps/web/src/components/settings/ProviderInstanceCard.tsx` |
| 3% | 1 | 27 | `apps/web/src/lib/composerImageBlobStore.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 1 | 27 | `apps/web/src/lib/hosts/reachability.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 1 | 27 | `apps/web/src/lib/serverWorkspacePaths.ts` | `apps/desktop/gnome-extension/captureService.js` |
| 3% | 1 | 16 | `packages/shared/src/automationMode.ts` | `packages/client-runtime/src/textPaste.ts` |
| 3% | 4 | 171 | `packages/shared/src/desktopIdentity.ts` | `packages/client-runtime/src/codexArtifactTemplates.ts` |
| 3% | 2 | 61 | `scripts/computer-use-fixtures/sample-resources.mjs` | `apps/desktop/scripts/browser-secret-native.test.mjs` |
| 3% | 5 | 112 | `apps/desktop/src/linuxEscapeKillSwitchMonitor.ts` | `apps/server/src/serviceLauncher.ts` |
| 3% | 4 | 165 | `apps/server/src/provider/claudeCacheObservation.test.ts` | `apps/server/src/git/linkCreatedPullRequest.test.ts` |
| 3% | 11 | 176 | `apps/web/src/components/AppRailUsage.tsx` | `apps/web/src/components/pullRequest/PullRequestSummaryTab.tsx` |
| 3% | 11 | 102 | `apps/web/src/lib/computerProvisioning.ts` | `apps/web/src/components/ProviderUpdateLaunchNotification.logic.ts` |
| 3% | 4 | 26 | `apps/web/src/lib/toolCallDetailsFormatting.ts` | `apps/mobile/src/features/review/reviewCommentSelection.ts` |
| 3% | 10 | 368 | `packages/contracts/src/device.ts` | `packages/contracts/src/device.ts` |
| 3% | 6 | 211 | `packages/contracts/src/hostAuth.ts` | `packages/contracts/src/composerContext.ts` |
| 3% | 2 | 50 | `apps/api/src/identity/index.ts` | `apps/desktop/src/snapShot/NativeCaptureFeedback.ts` |
| 3% | 5 | 151 | `apps/server/src/atomicWrite.test.ts` | `apps/server/src/cli/pair.test.ts` |
| 3% | 30 | 1809 | `apps/server/src/persistence/Layers/AutomationRepository.ts` | `apps/server/src/persistence/Layers/ProjectionThreadMessages.ts` |
| 3% | 20 | 110 | `apps/web/src/components/chat/ComposerCapsule.tsx` | `apps/web/src/components/pullRequest/PullRequestSummaryTab.tsx` |
| 3% | 19 | 652 | `apps/server/src/agentGateway/mcpTransport.test.ts` | `apps/server/src/orchestration/ThreadPullRequestReactor.test.ts` |
| 3% | 20 | 96 | `apps/server/src/orchestration/Layers/SidechatExpiryReactor.unarchive-race.test.ts` | `apps/server/src/project/AgentSessionImporter.test.ts` |
| 3% | 3 | 42 | `apps/server/src/orchestration/threadLatestReplyPreview.test.ts` | `apps/mobile/src/lib/composerImages.test.ts` |
| 3% | 6 | 139 | `apps/server/src/remoteSessions/localRpcBridge.ts` | `apps/desktop/src/app/DesktopAppActivation.ts` |
| 3% | 3 | 45 | `apps/web/src/components/chat/inlineVisualBlocks.logic.test.ts` | `apps/mobile/src/features/threads/threadOrderAvailability.test.ts` |
| 3% | 21 | 226 | `apps/web/src/components/settings/AppSnapShortcutControl.tsx` | `apps/web/src/components/GitActionsControl.tsx` |
| 3% | 26 | 432 | `apps/web/src/components/settings/ComputerSettingsPanel.tsx` | `apps/web/src/components/GitActionsControl.tsx` |
| 3% | 7 | 42 | `apps/web/src/components/tasks/TaskDelegateTargetPicker.tsx` | `apps/web/src/components/device/DeviceToolsPanel.tsx` |
| 3% | 60 | 793 | `apps/web/src/composerDraftModels.ts` | `apps/web/src/composerDraftStore.ts` |
| 3% | 4 | 42 | `apps/desktop/src/clipboardPermissions.ts` | `apps/mobile/src/features/sharing/incoming-share-model.ts` |
| 3% | 3 | 81 | `apps/server/src/hostConnections/dialer.test.ts` | `apps/web/src/components/files/useFileSaveCoordinator.test.tsx` |
| 3% | 4 | 168 | `apps/server/src/hostSecrets/syncKeyStore.ts` | `infra/relay/src/agentActivity/ApnsDeliveryQueue.ts` |
| 3% | 1 | 29 | `apps/server/src/provider/providerCliVersionProbe.test.ts` | `apps/server/src/orchestration/commandInvariants.test.ts` |
| 3% | 2 | 23 | `apps/server/src/provider/runtimeTaskList.test.ts` | `apps/server/src/orchestration/Layers/ProviderRuntimeIngestion.activity.test.ts` |
| 3% | 28 | 372 | `apps/web/src/components/chat/ComposerUsageLimitCapsules.tsx` | `apps/web/src/components/onboarding/WelcomeWizard.tsx` |
| 3% | 7 | 26 | `apps/web/src/components/chat/DraftAttachmentWarning.tsx` | `apps/web/src/components/pullRequest/pullRequestPresentation.tsx` |
| 3% | 3 | 120 | `apps/web/src/components/chat/chatHeaderControls.browser.tsx` | `apps/mobile/src/features/files/sourceHighlightingState.test.ts` |
| 3% | 1 | 28 | `apps/web/src/components/chat/editedFilePathActions.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 2 | 50 | `apps/web/src/components/pullRequest/pullRequestMarkdown.logic.test.ts` | `packages/client-runtime/src/state/pullRequestDiffHttp.test.ts` |
| 3% | 1 | 28 | `apps/web/src/hooks/useHandleNewGroupChat.ts` | `apps/web/src/hooks/useRemoveClonedProject.ts` |
| 3% | 3 | 61 | `packages/shared/src/cuaActionDiagnostics.test.ts` | `apps/web/src/components/chat/ModelPickerContent.test.ts` |
| 3% | 1 | 28 | `packages/shared/src/wsRequestClass.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 2 | 58 | `scripts/verify-release-artifact-checkpoint.ts` | `scripts/merge-update-manifests.ts` |
| 3% | 4 | 20 | `apps/marketing/src/components/ChangelogPicker.tsx` | `apps/web/src/browser/BrowserDeviceToolbar.tsx` |
| 3% | 11 | 204 | `apps/server/src/persistence/modelSelectionCompatibility.ts` | `packages/shared/src/model.ts` |
| 3% | 3 | 89 | `apps/api/src/billing/accessPolicy.test.ts` | `apps/desktop/src/snapShot/snapShot.test.ts` |
| 3% | 4 | 69 | `apps/desktop/src/macUpdateInstallHandoff.test.ts` | `apps/desktop/src/snapShot/RegionSnapShot.test.ts` |
| 3% | 3 | 89 | `apps/desktop/src/updatePendingCache.test.ts` | `apps/desktop/src/snapShot/snapShot.test.ts` |
| 3% | 12 | 171 | `apps/server/src/editorAppDiscovery.ts` | `packages/shared/src/shell.ts` |
| 3% | 2 | 30 | `apps/server/src/externalMcp/executionAdmission.test.ts` | `apps/server/src/orchestration/commandInvariants.test.ts` |
| 3% | 20 | 515 | `apps/web/src/components/PluginLibrary.tsx` | `apps/web/src/components/settings/KeybindingsSettings.tsx` |
| 3% | 5 | 47 | `apps/web/src/components/chat/FileDiffView.tsx` | `apps/web/src/components/diffs/AnnotatableCodeView.tsx` |
| 3% | 26 | 514 | `apps/web/src/components/chat/workspaceExplorer.tsx` | `apps/web/src/components/settings/KeybindingsSettings.tsx` |
| 3% | 36 | 538 | `apps/web/src/components/settings/ExternalMcpSettingsPanel.tsx` | `apps/web/src/components/settings/ConnectionsSettings.tsx` |
| 3% | 5 | 204 | `apps/web/src/recentViews.logic.ts` | `apps/web/src/components/preview/usePreviewBridge.ts` |
| 3% | 2 | 71 | `scripts/computer-use-fixtures/packaged-artifact.ts` | `apps/desktop/src/snapShot/MacModifierPairShortcutProcess.ts` |
| 3% | 1 | 123 | `apps/desktop/native/appsnap/OptionChordMonitor.swift` | `apps/web/scripts/build-libghostty-wasm.sh` |
| 3% | 4 | 59 | `apps/desktop/src/linuxAppIcon.ts` | `apps/web/src/components/BranchToolbar.logic.ts` |
| 3% | 3 | 20 | `apps/server/src/provider/promptAttachments.test.ts` | `apps/mobile/src/state/pending-thread-creation.test.ts` |
| 3% | 3 | 279 | `apps/web/src/components/BrowserVault.browser.tsx` | `apps/web/src/components/settings/SnapShotSettings.test.tsx` |
| 3% | 4 | 65 | `apps/web/src/components/chat/useTimelineRowHeightHints.ts` | `apps/web/src/components/chat/AssistantCitationSource.tsx` |
| 3% | 4 | 78 | `apps/web/src/lib/providerUsageDisplay.test.ts` | `apps/server/src/provider/Layers/codexUsageLimits.test.ts` |
| 3% | 5 | 37 | `apps/web/src/lib/threadImport.ts` | `apps/web/src/components/settings/SettingsPanels.logic.ts` |
| 3% | 3 | 30 | `apps/analytics/testing/sqliteD1.mjs` | `apps/desktop/src/snapShot/GnomeCaptureSetup.ts` |
| 3% | 3 | 58 | `apps/server/src/automation/runResult.ts` | `apps/server/src/sourceControl/azureDevOpsPullRequests.ts` |
| 3% | 1 | 20 | `apps/server/src/browserAutomation/Services/BrowserAutomationHost.ts` | `apps/mobile/src/features/review/useReviewHeaderPresentation.tsx` |
| 3% | 1 | 40 | `apps/server/src/computer/computerProvision.test.ts` | `apps/web/src/components/usage/usageShortcuts.test.ts` |
| 3% | 6 | 326 | `apps/server/src/device/IosSimulatorBackend.test.ts` | `apps/server/src/environment/ServerEnvironmentMachine.test.ts` |
| 3% | 1 | 29 | `apps/server/src/orchestration/runtimeJournalPoisonGate.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 1 | 24 | `apps/server/src/platform/supervisedProcessSpawnFailure.test.ts` | `oxlint-plugin-t3code/rules/no-global-process-runtime.test.ts` |
| 3% | 4 | 25 | `apps/server/src/provider/acp/SessionTeardownGate.ts` | `apps/server/src/project/WorktreeSetupTracker.ts` |
| 3% | 7 | 146 | `apps/server/src/remotePairing/gateway.ts` | `apps/desktop/src/app/DesktopAppActivation.ts` |
| 3% | 35 | 972 | `apps/web/src/components/chat/TimelineWorkEntryRow.tsx` | `apps/web/src/components/settings/ConnectionsSettings.tsx` |
| 3% | 4 | 46 | `apps/web/src/components/hosts/ComputerPicker.browser.tsx` | `apps/mobile/src/widgets/subscriptionUsageSnapshot.test.ts` |
| 3% | 2 | 60 | `apps/web/src/components/settings/AppIconPicker.browser.tsx` | `apps/mobile/src/components/FilePreviewModal.tsx` |
| 3% | 5 | 96 | `apps/web/src/components/settings/providers/providerInstallSettings.test.ts` | `apps/mobile/src/features/threads/thread-settings-sheet-state.test.ts` |
| 3% | 4 | 45 | `apps/web/src/lib/hosts/activeHost.ts` | `apps/desktop/scripts/electron-launcher.mjs` |
| 3% | 2 | 31 | `packages/shared/src/threadDetailEvents.ts` | `apps/server/src/orchestration/decider.userMessageAppend.test.ts` |
| 3% | 6 | 223 | `apps/server/src/provider/providerMaintenance.test.ts` | `apps/server/src/provider/Drivers/CursorDriver.ts` |
| 3% | 22 | 278 | `apps/web/src/pendingInteractionDerivation.ts` | `apps/web/src/session-logic.ts` |
| 3% | 16 | 398 | `apps/server/src/externalMcp/Layers/ExternalMcpService.ts` | `apps/server/src/resourceTelemetry/NativeTelemetryClient.ts` |
| 3% | 6 | 92 | `apps/server/src/persistence/Layers/RemoteHostTrust.ts` | `apps/server/src/orchestration/Layers/OrchestrationEngine.ts` |
| 3% | 2 | 88 | `apps/web/src/lib/hosts/api.ts` | `packages/client-runtime/src/threadPullRequestCompatibility.ts` |
| 3% | 6 | 51 | `apps/web/src/lib/providerAccountSummary.ts` | `apps/web/src/components/ProviderUpdateLaunchNotification.logic.ts` |
| 3% | 3 | 74 | `apps/api/src/identity/userCache.ts` | `apps/desktop/src/window/QuitHold.ts` |
| 3% | 4 | 149 | `apps/server/src/assistantImageAttachments.test.ts` | `apps/mobile/src/lib/composerAttachmentUploadQueue.test.ts` |
| 3% | 3 | 74 | `apps/server/src/mcpConnectors/importServers.ts` | `apps/web/src/promptStashStore.ts` |
| 3% | 3 | 50 | `apps/server/src/orchestration/projectRelocationPaths.test.ts` | `apps/web/src/browserFaviconStore.test.ts` |
| 3% | 2 | 29 | `apps/web/src/components/chat/composerComputerControlHint.ts` | `apps/web/src/browserFaviconStore.ts` |
| 3% | 7 | 152 | `apps/web/src/lib/hosts/connectionClients.ts` | `apps/mobile/src/lib/composerImages.ts` |
| 3% | 6 | 49 | `apps/web/src/test/browserHarness.ts` | `apps/mobile/src/widgets/subscriptionUsageSnapshot.test.ts` |
| 3% | 63 | 2952 | `apps/server/src/provider/Layers/ProviderHealth.ts` | `apps/server/src/provider/providerMaintenanceRunner.ts` |
| 3% | 9 | 410 | `apps/server/src/provider/supervisedProcessTeardown.test.ts` | `apps/web/src/components/ProviderUpdateEnvironmentRows.test.tsx` |
| 3% | 3 | 168 | `apps/web/src/components/codeEditor/editorHighlighting.ts` | `apps/web/src/lib/incrementalHighlighting.ts` |
| 3% | 3 | 128 | `apps/web/src/cursorModelVariants.test.ts` | `apps/mobile/src/features/review/nativeReviewDiffAdapter.test.ts` |
| 3% | 1 | 14 | `apps/web/src/lib/desktopWindowTranslucency.ts` | `apps/web/src/hooks/useDefaultTheme.ts` |
| 3% | 13 | 709 | `apps/web/src/lib/gitReactQuery.ts` | `apps/web/src/state/sourceControlActions.ts` |
| 3% | 2 | 89 | `apps/web/src/lib/hosts/queries.ts` | `packages/client-runtime/src/threadPullRequestCompatibility.ts` |
| 3% | 1 | 30 | `apps/web/src/persistedRecord.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 1 | 30 | `apps/web/src/uiFontSize.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 4 | 76 | `benchmarks/desktop-perf/lib/cdp.mjs` | `apps/desktop/src/snapShot/KdeSnapShot.ts` |
| 3% | 1 | 30 | `benchmarks/desktop-perf/lib/stats.mjs` | `apps/desktop/gnome-extension/captureService.js` |
| 3% | 1 | 47 | `scripts/lib/mac-bundle-architecture.test.ts` | `packages/shared/src/legacyCliLauncher.ts` |
| 3% | 11 | 287 | `apps/analytics/src/views/Overview.tsx` | `apps/web/src/components/pullRequest/PullRequestSummaryTab.tsx` |
| 3% | 6 | 222 | `apps/desktop/src/browserAutomation/browserSessionRestore.test.ts` | `apps/desktop/src/preview/BrowserImport/BrowserImport.test.ts` |
| 3% | 6 | 226 | `apps/desktop/src/desktopAppIcon.test.ts` | `apps/web/src/terminal/ghostty/surface.test.ts` |
| 3% | 20 | 911 | `apps/e2e/src/workspace.e2e.test.ts` | `apps/server/src/orchestration/Layers/OrchestrationEngine.test.ts` |
| 3% | 13 | 513 | `apps/web/src/components/chat/environment/EnvironmentPullRequestSection.browser.tsx` | `apps/web/src/components/pullRequest/PullRequestDetailPanel.test.tsx` |
| 3% | 17 | 149 | `apps/web/src/components/chat/group/GroupLifecycleSection.tsx` | `apps/web/src/components/GitActionsControl.tsx` |
| 3% | 5 | 127 | `apps/web/src/components/codeEditor/PierreEditorPanes.tsx` | `apps/mobile/src/features/files/FileMarkdownPreview.tsx` |
| 3% | 8 | 109 | `apps/web/src/components/settings/ConnectionsSettingsPanel.tsx` | `apps/web/src/components/settings/SnapShotSettings.tsx` |
| 3% | 4 | 77 | `apps/desktop/src/browserAutomation/browserVaultCapture.test.ts` | `apps/desktop/src/snapShot/RegionSnapShot.test.ts` |
| 3% | 2 | 26 | `apps/server/src/providerUsage/credentials.atomic.test.ts` | `packages/shared/src/logging.test.ts` |
| 3% | 3 | 172 | `apps/web/src/appRail.logic.test.ts` | `apps/mobile/src/features/threads/thread-settings-sheet-state.test.ts` |
| 3% | 10 | 149 | `apps/web/src/components/chat/environment/EnvironmentLocalServersSection.tsx` | `apps/web/src/components/pullRequest/PullRequestSummaryTab.tsx` |
| 3% | 5 | 58 | `apps/web/src/diffRouteSearch.ts` | `packages/shared/src/serverSettings.ts` |
| 3% | 6 | 130 | `apps/web/src/feedback.ts` | `apps/mobile/src/lib/attachmentUpload.ts` |
| 3% | 6 | 131 | `apps/web/src/hooks/useProviderUsageSummary.ts` | `apps/server/src/provider/Layers/CodexProvider.ts` |
| 3% | 3 | 152 | `apps/web/src/lib/glassOverlayCutout.browser.tsx` | `apps/web/src/components/settings/NotificationSettings.tsx` |
| 3% | 5 | 61 | `apps/web/src/recentViewActivation.logic.test.ts` | `apps/web/src/promptStashStore.test.ts` |
| 3% | 2 | 42 | `packages/contracts/src/hostUsage.ts` | `apps/server/src/provider/Errors.ts` |
| 3% | 10 | 221 | `apps/server/src/desktopUpdateRpc.test.ts` | `apps/web/src/components/ChatView.logic.test.ts` |
| 3% | 3 | 25 | `apps/server/src/providerProcessPriority.test.ts` | `apps/server/src/assets/AttachmentUpload.test.ts` |
| 3% | 12 | 203 | `apps/web/perf/main.tsx` | `apps/mobile/src/features/threads/ThreadDetailScreen.tsx` |
| 3% | 11 | 398 | `apps/web/src/components/BrowserPanel.logic.test.ts` | `apps/web/src/components/CommandPalette.logic.test.ts` |
| 3% | 15 | 152 | `apps/web/src/components/chat/FileAttachmentChip.tsx` | `apps/web/src/components/pullRequest/PullRequestSummaryTab.tsx` |
| 3% | 12 | 153 | `apps/web/src/components/chat/environment/EnvironmentUsageSection.tsx` | `apps/web/src/components/pullRequest/PullRequestSummaryTab.tsx` |
| 3% | 9 | 79 | `apps/web/src/components/chat/group/HubWorkItemCard.tsx` | `apps/web/src/components/pullRequest/PullRequestSummaryTab.tsx` |
| 3% | 3 | 96 | `apps/web/src/components/terminal/terminalAutomaticFocus.browser.tsx` | `apps/web/src/hooks/showThreadUndoNotice.test.ts` |
| 3% | 5 | 160 | `apps/web/src/lib/hosts/syncKeyPairing.ts` | `apps/mobile/src/lib/composerImages.ts` |
| 3% | 1 | 46 | `apps/desktop/src/browserAutomation/browserShutdown.test.ts` | `apps/web/src/lib/desktopPasteAsText.test.ts` |
| 3% | 1 | 6 | `apps/desktop/src/desktopProcessErrors.ts` | `.github/scripts/check-nightly-release.cjs` |
| 3% | 1 | 23 | `apps/marketing/next.config.ts` | `apps/mobile/src/features/diagnostics/crash-log-model.ts` |
| 3% | 4 | 51 | `apps/server/src/hostConnections/unaryRpc.ts` | `apps/desktop/src/snapShot/HyprlandSnapShot.ts` |
| 3% | 6 | 44 | `apps/web/src/components/chat/DockList.tsx` | `apps/web/src/components/pullRequest/pullRequestPresentation.tsx` |
| 3% | 3 | 96 | `apps/web/src/components/chat/ThreadErrorBanner.browser.tsx` | `apps/mobile/src/features/review/useReviewDiffPrewarming.test.ts` |
| 3% | 1 | 31 | `apps/web/src/lib/automationFailurePolicy.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 4 | 99 | `apps/web/src/lib/dockPaneActivation.ts` | `apps/desktop/src/electron/ElectronMenu.ts` |
| 3% | 1 | 31 | `apps/web/src/lib/hosts/navigation.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 1 | 31 | `apps/web/src/lib/markdownTableRepair.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 1 | 31 | `packages/shared/src/localPreviewFiles.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 1 | 14 | `packages/shared/src/processSpawnOutcome.ts` | `apps/web/src/components/chat/SkillInlineText.tsx` |
| 3% | 16 | 288 | `apps/server/src/agentGateway/hubWorkSource.test.ts` | `apps/server/src/orchestration/Layers/ProjectionPipeline.ts` |
| 3% | 14 | 208 | `apps/server/src/git/Layers/ProviderTextGeneration.ts` | `apps/server/src/sourceControl/BitbucketApi.ts` |
| 3% | 9 | 288 | `apps/server/src/persistence/DatabaseLifecycleLock.ts` | `apps/server/src/serviceLauncher.ts` |
| 3% | 5 | 215 | `apps/server/src/wsStreamBackpressure.test.ts` | `apps/server/src/device/DeviceService.test.ts` |
| 3% | 5 | 69 | `apps/web/src/projectRunTargets.ts` | `apps/web/src/components/BranchToolbar.logic.ts` |
| 3% | 2 | 15 | `apps/server/src/accountRefusal.ts` | `apps/web/src/components/search/HighlightedSearchLine.tsx` |
| 3% | 5 | 139 | `apps/server/src/executableLookup.test.ts` | `apps/web/src/components/desktop/SnapShotCoordinator.test.ts` |
| 3% | 53 | 4218 | `apps/server/src/provider/Layers/OpenCodeAdapter.test.ts` | `apps/server/src/provider/Layers/CursorAdapter.test.ts` |
| 3% | 2 | 37 | `apps/server/src/provider/openCodeAuthPaths.ts` | `packages/shared/src/agentAwareness.ts` |
| 3% | 3 | 68 | `apps/web/src/components/ConnectorIcon.tsx` | `apps/mobile/src/features/devices/device-stream.browser.ts` |
| 3% | 4 | 13 | `apps/web/src/components/account/signOutWithToast.ts` | `apps/web/src/components/files/AttachmentFilePreview.tsx` |
| 3% | 2 | 13 | `apps/web/src/components/chat/ChatEmptyStateHero.tsx` | `apps/web/src/components/ProjectFavicon.tsx` |
| 3% | 2 | 102 | `apps/web/src/components/inbox/InboxView.tasks.browser.tsx` | `apps/web/src/components/ui/standalone-page.tsx` |
| 3% | 2 | 102 | `apps/web/src/components/settings/ModelsSettingsPanel.browser.tsx` | `apps/mobile/src/features/files/WorkspaceFileWebPreview.tsx` |
| 3% | 2 | 42 | `apps/web/src/i18n/Translation.test.tsx` | `apps/web/src/markdown-incremental.test.tsx` |
| 3% | 8 | 108 | `apps/web/src/rightDockStore.ts` | `apps/web/src/browserFaviconStore.ts` |
| 3% | 6 | 59 | `packages/shared/src/localServers.test.ts` | `apps/mobile/src/features/updates/app-updates.test.ts` |
| 3% | 3 | 75 | `apps/server/src/hostIdentity/jwt.test.ts` | `apps/server/src/device/DeviceHubProxy.test.ts` |
| 3% | 4 | 180 | `apps/server/src/mcpConnectors/localConnectors.test.ts` | `packages/effect-acp/src/agent.test.ts` |
| 3% | 18 | 1326 | `apps/server/src/profileStatsArchive.test.ts` | `apps/server/src/persistence/Migrations/016_CanonicalizeModelSelections.test.ts` |
| 3% | 10 | 128 | `apps/server/src/provider/acp/AcpLoadReplayGate.ts` | `apps/desktop/src/updates/DesktopRemoteUpdates.ts` |
| 3% | 3 | 57 | `apps/web/scripts/measure-dev-transforms.mjs` | `apps/web/src/bundledDev.test.ts` |
| 3% | 1 | 40 | `apps/web/src/authLogout.test.ts` | `apps/server/src/orchestration/commandInvariants.test.ts` |
| 3% | 12 | 110 | `apps/web/src/components/DiffPanelCompareRefMenuSection.tsx` | `apps/web/src/components/BranchToolbar.tsx` |
| 3% | 36 | 453 | `apps/web/src/components/SpaceSwitcher.tsx` | `apps/web/src/components/pullRequest/PullRequestDetailPanel.tsx` |
| 3% | 5 | 41 | `apps/web/src/editorViewState.ts` | `apps/web/src/uiStateStore.ts` |
| 3% | 4 | 57 | `apps/web/src/hooks/useRefreshOnWindowReturn.test.ts` | `apps/mobile/src/widgets/subscriptionUsageSnapshot.test.ts` |
| 3% | 3 | 56 | `apps/web/src/lib/terminalStateCleanup.test.ts` | `apps/mobile/src/connection/environment-cache-store.test.ts` |
| 3% | 2 | 187 | `apps/web/src/storePersistence.test.ts` | `apps/web/src/lib/snapShotSetupResume.test.ts` |
| 3% | 1 | 31 | `apps/desktop/src/linuxCuaDriverHost.ts` | `infra/relay/src/Config.ts` |
| 3% | 2 | 33 | `apps/marketing/scripts/check-docs.test.mjs` | `apps/desktop/src/backend/tailscaleEndpointProvider.test.ts` |
| 3% | 1 | 8 | `apps/server/src/environment/Services/ServerEnvironment.ts` | `apps/web/src/environments/primary/context.ts` |
| 3% | 5 | 61 | `apps/web/src/components/chat/useSubagentRunControls.ts` | `apps/web/src/components/settings/AddProviderInstanceDialog.tsx` |
| 3% | 2 | 93 | `apps/web/src/components/settings/externalMcpSetup.test.ts` | `apps/desktop/src/ipc/methods/preview.test.ts` |
| 3% | 4 | 75 | `packages/shared/src/computerGrants.ts` | `packages/client-runtime/src/state/projectGrouping.ts` |
| 3% | 3 | 74 | `apps/desktop/src/rendererCrashRecovery.test.ts` | `packages/client-runtime/src/connection/presentation.test.ts` |
| 3% | 4 | 113 | `apps/server/src/hostAuth/mintService.ts` | `apps/server/src/cloud/selfUpdate.ts` |
| 3% | 7 | 183 | `apps/server/src/persistence/Layers/HubWorkRepository.ts` | `apps/server/src/orchestration/Layers/OrchestrationEngine.ts` |
| 3% | 13 | 444 | `apps/web/src/components/chat/useChatKeyboardShortcuts.ts` | `apps/web/src/components/CommandPalette.tsx` |
| 3% | 8 | 217 | `apps/desktop/src/browserAnnotations/protocol.test.ts` | `apps/mobile/src/lib/composerContext.test.ts` |
| 3% | 5 | 113 | `apps/server/src/todo/Layers/TodoService.test.ts` | `scripts/build-npm-platform-packages.test.ts` |
| 3% | 4 | 40 | `apps/web/src/components/chat/ComposerPullRequestAutoFixHint.logic.test.ts` | `packages/client-runtime/src/state/threadSnoozed.test.ts` |
| 3% | 21 | 173 | `apps/web/src/components/pullRequest/GitHubItemFloatingComposer.tsx` | `apps/web/src/components/settings/KeybindingsSettings.tsx` |
| 3% | 6 | 311 | `apps/web/src/lib/icons.tsx` | `apps/web/src/components/ServerUpdateAction.tsx` |
| 3% | 5 | 115 | `packages/shared/src/transportRace.ts` | `apps/server/src/cloud/serviceLauncherClient.ts` |
| 3% | 2 | 27 | `scripts/lib/cloudflared.ts` | `apps/desktop/scripts/ensure-electron-runtime.mjs` |
| 3% | 16 | 808 | `apps/analytics/src/views/Product.tsx` | `apps/web/src/components/usage/UsagePage.tsx` |
| 3% | 7 | 156 | `apps/server/src/agentGateway/computerToolPermission.test.ts` | `apps/mobile/src/state/thread-outbox.test.ts` |
| 3% | 6 | 204 | `apps/web/src/components/chat/useChatComposerCommands.test.ts` | `apps/web/src/components/RightPanelTabs.test.tsx` |
| 3% | 4 | 183 | `apps/web/src/components/hosts/RemoteConnectionsAnnouncement.browser.tsx` | `apps/web/src/components/chat/AssistantCitationChip.test.tsx` |
| 3% | 8 | 285 | `apps/web/src/hooks/useComposerPrediction.test.ts` | `apps/web/src/components/ProviderUpdateEnvironmentRows.test.tsx` |
| 3% | 5 | 76 | `packages/contracts/src/browserVault.ts` | `packages/contracts/src/preview.ts` |
| 3% | 2 | 51 | `apps/web/src/components/chat/ComputerPreviewPopover.browser.tsx` | `apps/mobile/src/state/use-selected-thread-requests.test.tsx` |
| 3% | 2 | 51 | `apps/web/src/components/hosts/RemoteControls.browser.tsx` | `apps/mobile/src/state/use-selected-thread-requests.test.tsx` |
| 3% | 1 | 8 | `apps/web/src/lib/newTaskShortcut.ts` | `.github/scripts/check-nightly-release.cjs` |
| 3% | 2 | 56 | `apps/web/src/lib/spaceNavigation.ts` | `apps/web/src/components/chat/ContextWindowMeter.logic.ts` |
| 3% | 4 | 95 | `apps/web/src/lib/voiceRecorder.browser.tsx` | `apps/web/src/browser/desktopTabLifetime.test.ts` |
| 3% | 29 | 545 | `apps/web/src/storeNormalization.test.ts` | `apps/mobile/src/lib/threadActivity.test.ts` |
| 3% | 1 | 31 | `packages/contracts/src/inlineVisual.ts` | `oxlint-plugin-t3code/rules/no-inline-schema-compile.test.ts` |
| 3% | 1 | 33 | `scripts/lib/desktop-artifact-naming.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 6 | 117 | `apps/server/src/persistence/Layers/AuthPairingLinksRemote.ts` | `apps/server/src/project/WorktreeSetupTracker.ts` |
| 3% | 3 | 89 | `apps/server/src/provider/providerStartupLifecycle.test.ts` | `apps/desktop/src/preview/BrowserImport/BrowserImport.test.ts` |
| 3% | 4 | 165 | `apps/server/src/scratchWorkspaces.test.ts` | `apps/server/src/provider/Layers/EventNdjsonLogger.test.ts` |
| 3% | 13 | 515 | `apps/web/src/components/SidebarActivityView.logic.ts` | `apps/mobile/src/features/threads/threadListV2.ts` |
| 3% | 16 | 427 | `apps/web/src/components/chat/ChatTranscriptPane.browser.tsx` | `apps/web/src/components/chat/MessagesTimeline.test.tsx` |
| 3% | 39 | 921 | `apps/web/src/components/chat/environment/EnvironmentPullRequestSection.tsx` | `apps/web/src/components/settings/ConnectionsSettings.tsx` |
| 3% | 4 | 95 | `apps/web/src/components/composerInlineChip.ts` | `apps/web/src/components/settings/ProjectIconPickerDialog.tsx` |
| 3% | 11 | 166 | `apps/web/src/components/kanban/useKanbanTaskComposerMenu.ts` | `apps/mobile/src/features/threads/ThreadDetailScreen.tsx` |
| 3% | 3 | 39 | `apps/web/src/components/profile/profileSelectors.test.ts` | `apps/web/src/components/chat/ContextWindowMeter.logic.test.ts` |
| 3% | 18 | 292 | `apps/web/src/components/tasks/tasks.logic.test.ts` | `apps/web/src/components/ChatView.logic.test.ts` |
| 3% | 3 | 79 | `apps/web/src/hooks/useHosts.ts` | `apps/desktop/src/snapShot/GnomeCaptureSetup.test.ts` |
| 3% | 3 | 102 | `apps/web/src/lib/rpcTransportFailure.test.ts` | `apps/server/src/auth/utils.test.ts` |
| 3% | 3 | 136 | `benchmarks/desktop-perf/diagnose-backend-boot.mjs` | `apps/desktop/src/snapShot/NativeCaptureFeedback.ts` |
| 3% | 6 | 149 | `apps/desktop/src/browserAutomation/screenshotCapture.ts` | `apps/mobile/src/lib/attachmentUpload.ts` |
| 3% | 4 | 156 | `apps/desktop/src/electronUpdaterSecurity.test.ts` | `apps/mobile/src/lib/storage.test.ts` |
| 3% | 6 | 111 | `apps/server/src/editor/dapConnection.ts` | `apps/desktop/src/snapShot/NiriSnapShot.ts` |
| 3% | 6 | 134 | `apps/server/src/hostConnections/resourcePool.ts` | `apps/desktop/src/snapShot/PortalCaptureShortcut.ts` |
| 3% | 6 | 60 | `apps/server/src/orchestration/pullRequestCommandDetection.test.ts` | `apps/server/src/orchestration/ActivityPayloadProjection.test.ts` |
| 3% | 15 | 692 | `apps/server/src/usageAnalyticsStats.ts` | `apps/server/src/persistence/Layers/ProjectionThreadActivities.ts` |
| 3% | 17 | 145 | `apps/web/src/components/SidebarHeaderNavigationControls.tsx` | `apps/web/src/components/ThreadTerminalDrawer.tsx` |
| 3% | 5 | 27 | `apps/web/src/hooks/useDesktopKeepAwake.ts` | `apps/mobile/src/features/review/ReviewCommentComposerSheet.tsx` |
| 3% | 2 | 34 | `apps/desktop/src/macAppIcon.native.test.ts` | `scripts/knip-schemas.test.ts` |
| 3% | 2 | 42 | `apps/desktop/src/startupDiagnosticDialog.test.ts` | `apps/mobile/src/lib/localAttachmentPreview.test.ts` |
| 3% | 10 | 304 | `apps/server/src/agentGateway/hubWorkGateway.ts` | `apps/server/src/serverRuntimeStartup.ts` |
| 3% | 5 | 72 | `apps/server/src/checkpointing/Diffs.test.ts` | `apps/server/src/pullRequest/bitbucketDiffRevisions.test.ts` |
| 3% | 4 | 145 | `apps/server/src/computer/computerCallContext.test.ts` | `apps/web/src/components/preview/openTerminalLinkInPreview.test.ts` |
| 3% | 8 | 84 | `apps/server/src/orchestration/projectMetadataProjection.ts` | `scripts/mobile-showcase-environment.ts` |
| 3% | 7 | 142 | `apps/server/src/pullRequests/repositoryResolution.ts` | `apps/server/src/sourceControl/AzureDevOpsCli.ts` |
| 3% | 4 | 38 | `apps/web/src/components/chat/ComposerExpiredUserInputNotice.tsx` | `apps/mobile/src/features/devices/DevicePreviewRouteScreen.tsx` |
| 3% | 11 | 316 | `apps/web/src/components/settings/providers/ProvidersSettingsPanel.browser.tsx` | `apps/web/src/components/settings/ProviderSettingsPanel.environment.test.tsx` |
| 3% | 1 | 32 | `packages/shared/src/betaChannel.ts` | `oxlint-plugin-t3code/rules/no-inline-schema-compile.test.ts` |
| 3% | 3 | 25 | `packages/shared/src/browserAutomationMcpError.test.ts` | `packages/contracts/src/composerContext.test.ts` |
| 3% | 1 | 51 | `scripts/import-portable-build.test.ts` | `packages/shared/src/legacyCliLauncher.ts` |
| 3% | 2 | 27 | `scripts/merge-mac-update-manifests.ts` | `apps/mobile/src/features/sharing/incoming-share-inbox.ts` |
| 3% | 2 | 27 | `scripts/prepare-release-update-feed.ts` | `apps/mobile/src/features/sharing/incoming-share-inbox.ts` |
| 3% | 18 | 289 | `apps/web/src/components/chat/GitPanel.tsx` | `apps/web/src/components/RightPanelTabs.tsx` |
| 3% | 5 | 122 | `apps/web/src/lib/threadUnblock.test.ts` | `packages/client-runtime/src/state/threadSnoozed.test.ts` |
| 3% | 3 | 50 | `apps/desktop/src/desktopQuitIntent.ts` | `apps/server/src/cloud/serviceProtocol.ts` |
| 3% | 8 | 216 | `apps/marketing/src/app/sponsor/page.tsx` | `apps/web/src/components/pullRequest/pullRequestPresentation.tsx` |
| 3% | 2 | 8 | `apps/marketing/src/components/ThemeScript.tsx` | `apps/web/src/components/preview/usePreviewBridge.ts` |
| 3% | 7 | 237 | `apps/server/src/browserAutomation/browserHostRpcClient.test.ts` | `apps/server/src/cli/app.test.ts` |
| 3% | 3 | 43 | `apps/server/src/provider/acp/AcpConfigOptions.ts` | `apps/server/src/provider/providerSnapshot.ts` |
| 3% | 10 | 233 | `apps/server/src/studioOutputs.ts` | `packages/shared/src/shell.ts` |
| 3% | 4 | 101 | `apps/web/src/appNavigation.ts` | `apps/mobile/src/features/diagnostics/SettingsDiagnosticsRouteScreen.tsx` |
| 3% | 11 | 168 | `apps/web/src/components/DevicePanel.attach.browser.tsx` | `packages/client-runtime/src/device/stream.test.ts` |
| 3% | 26 | 441 | `apps/web/src/components/chat/WorkflowRunCard.tsx` | `apps/web/src/components/settings/ConnectionsSettings.tsx` |
| 3% | 5 | 48 | `apps/web/src/projectImport/logic.ts` | `apps/server/src/project/ProjectCloneTracker.ts` |
| 3% | 15 | 635 | `apps/web/src/routes/-automations.shared.test.tsx` | `apps/mobile/src/state/thread-outbox.test.ts` |
| 3% | 3 | 75 | `apps/web/src/storeState.ts` | `packages/client-runtime/src/state/threadLifecycle.ts` |
| 3% | 6 | 349 | `apps/e2e/src/harness/fixture.ts` | `apps/mobile/src/lib/composerImages.ts` |
| 3% | 3 | 102 | `apps/server/src/agentGateway/sessionLeaseCallSites.test.ts` | `apps/web/src/markdown-clipboard.ts` |
| 3% | 3 | 107 | `apps/server/src/device/deviceUiSnapshot.ts` | `apps/server/integration/TransferBudgetReport.integration.ts` |
| 3% | 11 | 237 | `apps/server/src/provider/claudeCredentialKeepalive.ts` | `apps/web/src/openVsxThemes.ts` |
| 3% | 18 | 289 | `apps/web/src/components/chat/InlineVisualCharts.tsx` | `apps/web/src/components/usage/UsageProviderChart.tsx` |
| 3% | 5 | 170 | `apps/web/src/components/chat/SubagentsDockPane.browser.tsx` | `apps/desktop/src/snapShot/RegionSnapShot.test.ts` |
| 3% | 18 | 310 | `apps/web/src/components/inbox/inbox.logic.test.ts` | `apps/server/src/relay/AgentAwarenessRelay.test.ts` |
| 3% | 13 | 159 | `packages/shared/src/platformProcess.ts` | `packages/shared/src/shell.ts` |
| 3% | 1 | 30 | `apps/desktop/scripts/synthetic-cookie-profile.mjs` | `oxlint-plugin-t3code/rules/no-global-process-runtime.test.ts` |
| 3% | 4 | 78 | `apps/server/src/provider/acp/AcpLoadReplayGate.test.ts` | `apps/server/src/textGeneration/GrokTextGeneration.test.ts` |
| 3% | 5 | 42 | `apps/server/src/provider/appSnapPromptContext.test.ts` | `apps/web/src/components/chat/composerAttachmentFiles.test.ts` |
| 3% | 4 | 84 | `apps/web/src/components/chat/ComposerCommandMenu.test.ts` | `apps/server/src/usage/cliproxyApi.test.ts` |
| 3% | 1 | 24 | `apps/web/src/components/chat/auxiliary/auxiliaryPanel.logic.ts` | `packages/client-runtime/src/textPaste.ts` |
| 3% | 2 | 52 | `apps/web/src/deviceStateStore.ts` | `apps/web/src/previewMiniPlayerStore.ts` |
| 3% | 7 | 126 | `apps/web/src/spacesUiStore.ts` | `apps/web/src/browserFaviconStore.ts` |
| 3% | 1 | 33 | `packages/contracts/src/accountBilling.ts` | `oxlint-plugin-t3code/rules/no-inline-schema-compile.test.ts` |
| 3% | 1 | 33 | `packages/contracts/src/usageLimit.ts` | `oxlint-plugin-t3code/rules/no-inline-schema-compile.test.ts` |
| 3% | 2 | 55 | `packages/shared/src/computerFrame.test.ts` | `apps/desktop/src/app/DesktopAppActivationBroker.ts` |
| 3% | 7 | 214 | `apps/server/src/agentGateway/computerToolPermission.ts` | `apps/desktop/src/snapShot/snapShot.ts` |
| 3% | 3 | 22 | `apps/server/src/agentGateway/creationUtils.ts` | `apps/server/src/usage/cursorUsageReader.ts` |
| 3% | 3 | 92 | `apps/server/src/desktopParentLifetime.test.ts` | `apps/mobile/src/lib/videoThumbnails.test.ts` |
| 3% | 10 | 405 | `apps/server/src/projectAgent/libraryStore.ts` | `apps/desktop/src/electron/ElectronWindow.ts` |
| 3% | 32 | 497 | `apps/web/src/components/chat/environment/environmentPullRequest.logic.test.ts` | `apps/web/src/components/pullRequest/pullRequestDetail.logic.test.ts` |
| 3% | 3 | 117 | `apps/web/src/hooks/useComposerDropzone.browser.tsx` | `apps/web/src/components/chat/AssistantCitationChip.test.tsx` |
| 3% | 19 | 347 | `apps/web/src/lib/threadBootstrap.test.ts` | `apps/web/src/components/ChatView.logic.test.ts` |
| 3% | 5 | 99 | `packages/shared/src/conversationEdit.test.ts` | `apps/mobile/src/lib/layout.test.ts` |
| 3% | 30 | 630 | `apps/server/src/externalMcp/Layers/ExternalMcpGateway.e2e.test.ts` | `apps/server/src/relay/AgentAwarenessRelay.test.ts` |
| 3% | 31 | 115 | `apps/server/src/orchestration/Layers/SidechatExpiryReactor.queued-expiry.test.ts` | `apps/server/src/orchestration/Layers/OrchestrationEngine.test.ts` |
| 3% | 4 | 33 | `apps/server/src/persistence/pullRequestLinksColumn.ts` | `apps/desktop/src/app/DesktopAppActivation.ts` |
| 3% | 12 | 442 | `packages/shared/src/outboundHttp.ts` | `packages/client-runtime/src/device/stream.ts` |
| 3% | 12 | 449 | `scripts/computer-use-fixtures/browser-benchmark-observer.ts` | `apps/desktop/src/snapShot/snapShot.ts` |
| 3% | 1 | 36 | `apps/server/src/bunWebSocketCompatibility.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 21 | 816 | `apps/web/src/components/chat/SubagentRunCard.logic.ts` | `packages/shared/src/agentAwareness.ts` |
| 3% | 1 | 36 | `apps/web/src/hasSelectorVariants.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 3% | 3 | 134 | `apps/web/src/hooks/useProvisionComputer.test.tsx` | `apps/mobile/src/state/use-selected-thread-requests.test.tsx` |
| 3% | 5 | 212 | `apps/web/src/hooks/useWorkspaceFileEditorSession.browser.tsx` | `apps/web/src/components/projectScriptEditor.test.tsx` |
| 3% | 5 | 148 | `apps/server/src/computer/computerForegroundGuard.test.ts` | `apps/mobile/src/state/use-thread-outbox-drain.test.ts` |
| 3% | 8 | 210 | `apps/server/src/managedAttachmentStore.ts` | `apps/server/src/assets/AttachmentUpload.ts` |
| 3% | 21 | 92 | `apps/server/src/orchestration/threadShellEvents.test.ts` | `apps/server/src/orchestration/projector.test.ts` |
| 3% | 6 | 75 | `apps/server/src/projectAgent/hubWorkLifecycle.ts` | `apps/web/src/components/pullRequest/pullRequestPresentation.tsx` |
| 3% | 6 | 125 | `apps/server/src/remoteTransport/resourceGateway.ts` | `apps/desktop/src/electron/WindowsForegroundFocusThread.ts` |
| 3% | 3 | 79 | `apps/web/src/components/chat/SynaraThreadCreationCard.browser.tsx` | `apps/web/src/browser/HostedBrowserWebview.test.tsx` |
| 3% | 44 | 1633 | `apps/web/src/storeProjection.test.ts` | `apps/server/src/orchestration/projector.test.ts` |
| 3% | 2 | 97 | `apps/desktop/src/macAppIcon.test.ts` | `apps/web/src/components/chat/useComposerFocusState.test.tsx` |
| 3% | 4 | 160 | `apps/server/src/accountInboxReporter.ts` | `native/kde-snap-shot/src/activate.test.ts` |
| 3% | 7 | 202 | `apps/server/src/externalMcp/Services/ExternalMcpRepository.ts` | `apps/server/src/pullRequest/PullRequestProvider.ts` |
| 3% | 17 | 615 | `apps/server/src/git/Layers/GitCore.concurrency.test.ts` | `scripts/dev-runner.test.ts` |
| 3% | 13 | 190 | `apps/server/src/provider/acp/AcpSessionRuntime.epoch.test.ts` | `apps/server/scripts/acp-mock-agent.ts` |
| 3% | 10 | 210 | `apps/server/src/provider/acp/CursorAcpCommand.ts` | `apps/server/src/provider/providerMaintenance.ts` |
| 3% | 4 | 87 | `apps/server/src/pullRequests/Layers/PullRequestService.ts` | `infra/relay/src/worker.ts` |
| 3% | 5 | 56 | `apps/web/src/focusedChatContext.ts` | `apps/web/src/hooks/useHandleNewThread.ts` |
| 3% | 3 | 114 | `apps/web/src/hooks/useCommittedPathname.test.ts` | `apps/mobile/src/features/agent-awareness/notificationNavigation.test.ts` |
| 3% | 4 | 30 | `apps/web/src/lib/threadArchive.ts` | `apps/mobile/src/state/thread-outbox-model.ts` |
| 3% | 14 | 187 | `packages/shared/src/browserSession.ts` | `apps/web/src/components/ProviderUpdateLaunchNotification.logic.ts` |
| 3% | 16 | 678 | `apps/server/src/http.test.ts` | `apps/server/src/bin.test.ts` |
| 3% | 6 | 260 | `apps/web/src/components/FeatureTourDialog.browser.tsx` | `apps/web/src/components/ui/switch.test.tsx` |
| 3% | 18 | 173 | `apps/web/src/components/automation/automationInlineFields.tsx` | `apps/web/src/components/settings/KeybindingsSettings.tsx` |
| 3% | 5 | 96 | `apps/desktop/scripts/find-cua-artifact.test.mjs` | `.github/scripts/thread-transfer-report.test.cjs` |
| 3% | 1 | 26 | `apps/desktop/src/backendIssueDetector.ts` | `apps/mobile/src/features/terminal/terminalInput.ts` |
| 3% | 1 | 25 | `apps/desktop/src/notificationRetention.ts` | `.github/scripts/relay-state-output.test.cjs` |
| 3% | 1 | 35 | `apps/server/src/hostConnections/failure.ts` | `oxlint-plugin-t3code/rules/no-inline-schema-compile.test.ts` |
| 3% | 12 | 26 | `apps/server/src/orchestration/turnLifecycle.ts` | `packages/client-runtime/src/state/threadReducer.ts` |
| 3% | 11 | 192 | `apps/server/src/provider/antigravityCli.ts` | `apps/desktop/src/preview/FaviconCapture.ts` |
| 3% | 33 | 740 | `apps/server/src/provider/providerRuntimeReconciliation.test.ts` | `apps/server/src/relay/AgentAwarenessRelay.test.ts` |
| 3% | 5 | 229 | `apps/web/src/components/codeEditor/editorModels.browser.tsx` | `apps/web/src/components/files/projectFilesQueryState.ts` |
| 3% | 3 | 107 | `apps/web/src/components/settings/SyncKeyPairingPanel.browser.tsx` | `apps/web/src/browser/desktopTabLifetime.test.ts` |
| 3% | 33 | 487 | `apps/web/src/components/settings/accounts/ProviderSignInDialog.tsx` | `apps/web/src/components/settings/ConnectionsSettings.tsx` |
| 3% | 8 | 67 | `apps/web/src/hooks/useEditorLaunchers.ts` | `apps/web/src/components/chat/ChatHeader.tsx` |
| 3% | 2 | 57 | `apps/web/src/lib/archivedThreadDelete.test.ts` | `packages/client-runtime/src/mediaSource.test.ts` |
| 3% | 1 | 25 | `apps/web/src/lib/codexReasoningEffort.ts` | `packages/client-runtime/src/threadPullRequestCompatibility.ts` |
| 3% | 1 | 16 | `apps/web/src/pinnedThreadsStore.test.ts` | `apps/web/src/components/composerContextPresentation.test.ts` |
| 3% | 3 | 101 | `packages/shared/src/betaFeatures.test.ts` | `scripts/apply-web-brand-assets.ts` |
| 3% | 7 | 211 | `packages/shared/src/mcpConnectors.ts` | `apps/server/src/provider/providerStatusCache.ts` |
| 3% | 3 | 48 | `apps/desktop/src/macUpdateInstallHandoff.ts` | `apps/desktop/src/snapShot/NiriSnapShot.ts` |
| 3% | 5 | 516 | `apps/server/src/persistence/MigrationSchemaTooNewRecovery.test.ts` | `apps/server/src/assets/AttachmentUpload.test.ts` |
| 3% | 46 | 910 | `apps/web/src/components/WorkspaceFilePreview.tsx` | `apps/web/src/components/ChatMarkdown.tsx` |
| 3% | 9 | 411 | `apps/web/src/composerSlashCommands.test.ts` | `apps/web/src/components/ChatView.logic.test.ts` |
| 3% | 5 | 70 | `apps/web/src/lib/accountBilling.test.ts` | `apps/mobile/src/lib/attachmentDownload.test.ts` |
| 3% | 3 | 83 | `scripts/lib/release-build-scope.ts` | `apps/server/src/cloud/pinnedRuntime.test.ts` |
| 3% | 3 | 81 | `apps/server/src/provider/openCodeConfigPaths.test.ts` | `apps/web/src/components/preview/openTerminalLinkInPreview.test.ts` |
| 3% | 20 | 186 | `apps/web/src/components/ReviewFileTreePanel.tsx` | `apps/web/src/components/settings/KeybindingsSettings.tsx` |
| 3% | 7 | 159 | `apps/web/src/focusedChatContext.test.ts` | `apps/server/src/orchestration/projector.pullRequests.test.ts` |
| 3% | 5 | 130 | `apps/server/scripts/provider-runtime-journal-benchmark.ts` | `apps/server/integration/TestProviderAdapter.integration.ts` |
| 3% | 3 | 77 | `apps/server/src/agentGateway/computerForegroundConsent.ts` | `apps/mobile/src/lib/composerContextClipboard.ts` |
| 3% | 5 | 227 | `apps/server/src/computer/stillFramePublisher.test.ts` | `apps/web/src/components/preview/previewAutomationHostBudget.test.ts` |
| 3% | 3 | 85 | `apps/server/src/device/deviceLogs.test.ts` | `apps/desktop/src/app/DesktopAppActivation.test.ts` |
| 3% | 10 | 355 | `apps/server/src/platform/windowsProcessSnapshot.ts` | `apps/web/src/openVsxThemes.ts` |
| 3% | 7 | 70 | `apps/server/src/provider/providerAdapterConformance.test.ts` | `apps/server/integration/orphanedProviderSessionStartup.integration.test.ts` |
| 3% | 2 | 122 | `apps/web/e2e/inlineVisualIsolation.e2e.ts` | `apps/server/src/compileCache.test.ts` |
| 3% | 2 | 46 | `apps/web/src/chatRouteRestore.ts` | `apps/web/src/cloud/connectCliAuth.ts` |
| 3% | 5 | 107 | `apps/web/src/hooks/useScratchComposerDraft.ts` | `apps/mobile/src/state/queries.ts` |
| 3% | 7 | 129 | `apps/web/src/hooks/useSidebarThreadOrderHold.browser.tsx` | `packages/client-runtime/src/state/threadSort.test.ts` |
| 3% | 18 | 462 | `apps/web/src/lib/composerSend.test.ts` | `apps/web/src/lib/attachmentUploadQueue.test.ts` |
| 3% | 1 | 19 | `apps/web/src/lib/threadArchive.test.ts` | `apps/web/src/lib/terminalContext.test.ts` |
| 3% | 1 | 33 | `packages/shared/src/processPriority.test.ts` | `oxlint-plugin-t3code/rules/no-global-process-runtime.test.ts` |
| 3% | 28 | 376 | `packages/shared/src/threadSummary.test.ts` | `packages/client-runtime/src/state/threadReducer.test.ts` |
| 3% | 5 | 208 | `scripts/computer-use-fixtures/verify-background-launch.mjs` | `apps/server/src/cli/app.ts` |
| 3% | 6 | 388 | `apps/desktop/src/betaInstaller.test.ts` | `apps/server/src/bootstrap.test.ts` |
| 3% | 5 | 136 | `apps/profiles/src/app/[handle]/opengraph-image.tsx` | `apps/mobile/modules/t3-markdown-text/src/NativeMarkdownBlock.tsx` |
| 3% | 2 | 46 | `apps/server/src/agentGateway/computerPasteInput.ts` | `apps/desktop/src/backend/tailscaleEndpointProvider.ts` |
| 3% | 4 | 88 | `apps/server/src/computer/computerSpaceChurn.test.ts` | `apps/mobile/src/state/use-thread-outbox-drain.test.ts` |
| 3% | 2 | 35 | `apps/server/src/device/deviceFrameTransport.ts` | `packages/client-runtime/src/rpc/http.ts` |
| 3% | 4 | 117 | `apps/server/src/groupWorkspaceScaffold.test.ts` | `scripts/lib/third-party-licenses.test.ts` |
| 3% | 3 | 31 | `apps/server/src/projectAgent/hubContextPacket.ts` | `packages/shared/src/composerContextReferences.ts` |
| 3% | 4 | 80 | `apps/web/src/components/profile/useProfileIdentity.ts` | `packages/client-runtime/src/mediaSource.ts` |
| 3% | 2 | 54 | `apps/web/src/lib/nativeSurfaceOcclusion.browser.tsx` | `apps/desktop/scripts/electron-launcher.test.mjs` |
| 3% | 6 | 81 | `apps/web/src/onboarding/useProjectAdder.ts` | `apps/mobile/src/features/sharing/IncomingShareProvider.tsx` |
| 3% | 8 | 155 | `apps/web/src/storePersistence.ts` | `apps/web/src/uiStateStore.ts` |
| 3% | 2 | 44 | `packages/contracts/src/remoteResources.ts` | `packages/contracts/src/environment.ts` |
| 3% | 3 | 55 | `packages/shared/src/cli.ts` | `apps/server/src/provider/providerSnapshot.ts` |
| 3% | 7 | 139 | `apps/desktop/scripts/embedded-popup-smoke.ts` | `apps/server/src/provider/Layers/EventNdjsonLogger.test.ts` |
| 3% | 4 | 175 | `apps/server/src/hostAuth/credential.test.ts` | `apps/desktop/src/snapShot/GnomeCaptureSetup.test.ts` |
| 3% | 7 | 177 | `apps/server/src/hostConnections/supervisor.test.ts` | `apps/web/src/authBootstrap.test.ts` |
| 3% | 14 | 627 | `apps/server/src/localImageRoute.test.ts` | `apps/server/src/bin.test.ts` |
| 3% | 4 | 38 | `apps/web/src/components/DiffPanelChangeNavigation.tsx` | `apps/web/src/components/ProjectScriptsControl.tsx` |
| 3% | 94 | 507 | `apps/web/src/components/chat/useChatComposerCommands.ts` | `apps/web/src/components/chat/ChatComposer.tsx` |
| 3% | 9 | 92 | `apps/web/src/lib/composerImagePreparation.worker.ts` | `apps/web/src/lib/imageCompression.ts` |
| 3% | 4 | 47 | `apps/web/src/lib/localFolderMentions.ts` | `apps/web/src/components/settings/KeybindingsSettings.logic.ts` |
| 3% | 11 | 218 | `apps/web/src/threadDetailSubscriptionRetention.ts` | `apps/mobile/src/state/use-thread-outbox-drain.ts` |
| 2% | 1 | 39 | `apps/marketing/src/app/manifest.ts` | `apps/desktop/gnome-extension/captureService.js` |
| 2% | 4 | 27 | `apps/server/src/editor/debugAdapterInstaller.test.ts` | `apps/web/src/hooks/useEnvironmentTheme.test.ts` |
| 2% | 11 | 103 | `apps/server/src/provider/Layers/DroidAdapter.test.ts` | `apps/server/src/provider/acp/AcpRuntimeModel.test.ts` |
| 2% | 3 | 128 | `apps/web/src/components/KeepAwakeControls.browser.tsx` | `apps/web/src/components/chat/AssistantCitationChip.test.tsx` |
| 2% | 10 | 103 | `apps/web/src/components/chat/InlineVisualTable.tsx` | `apps/web/src/components/settings/DiagnosticsSettings.tsx` |
| 2% | 1 | 39 | `apps/web/src/components/kanban/useKanbanTaskComposerDiscovery.test.tsx` | `apps/web/src/components/chat/ComposerPendingApprovalActions.test.tsx` |
| 2% | 6 | 99 | `apps/web/src/components/pullRequest/PullRequestList.tsx` | `apps/web/src/components/settings/BrowserImportWizard.tsx` |
| 2% | 4 | 63 | `apps/web/src/diffRouteSearch.test.ts` | `apps/web/src/components/BranchToolbar.logic.test.ts` |
| 2% | 8 | 102 | `apps/web/src/hooks/useWorkspaceFileEditorSession.ts` | `apps/web/src/components/projectScriptEditor.tsx` |
| 2% | 1 | 12 | `apps/web/src/lib/projectDelete.ts` | `apps/web/src/lib/chatThreadActions.ts` |
| 2% | 1 | 37 | `packages/contracts/src/computerAudit.ts` | `oxlint-plugin-t3code/rules/no-inline-schema-compile.test.ts` |
| 2% | 3 | 69 | `apps/api/src/index.ts` | `apps/desktop/scripts/dev-electron.mjs` |
| 2% | 6 | 88 | `apps/server/src/providerUsage/http.test.ts` | `apps/web/src/browserFaviconStore.test.ts` |
| 2% | 6 | 124 | `apps/server/src/serverStatusCli.ts` | `apps/server/src/provider/opencodeRuntime.ts` |
| 2% | 5 | 244 | `apps/web/src/components/settings/BillingSettingsPanel.browser.tsx` | `apps/web/src/components/onboarding/WelcomeWizard.test.tsx` |
| 2% | 6 | 254 | `apps/web/src/components/settings/usageAnalytics.logic.ts` | `packages/shared/src/usageMerge.ts` |
| 2% | 4 | 54 | `apps/desktop/src/browserWebMcp/guestBridgeLegacyNative.test.ts` | `apps/web/src/components/preview/PreviewAutomationHosts.test.tsx` |
| 2% | 5 | 138 | `apps/server/src/provider/skillPromptInjection.test.ts` | `apps/web/src/components/chat/ContextWindowMeter.logic.test.ts` |
| 2% | 3 | 71 | `apps/server/src/provider/testing/piSdkFixture.ts` | `scripts/install.test.ts` |
| 2% | 5 | 43 | `apps/server/src/providerUsage/sqlite.ts` | `apps/server/src/provider/acp/AntigravityProtocol.ts` |
| 2% | 2 | 92 | `apps/web/src/components/ThemePackEditor.browser.tsx` | `apps/mobile/src/features/diffs/nativeReviewDiffSurface.test.ts` |
| 2% | 4 | 44 | `apps/web/src/components/chat/BackgroundTaskRow.browser.tsx` | `apps/web/src/components/ThreadNotificationCoordinator.test.tsx` |
| 2% | 14 | 371 | `apps/web/src/components/chat/project/useProjectAgentSummaries.test.ts` | `apps/mobile/src/features/threads/threadListV2.test.ts` |
| 2% | 8 | 59 | `apps/web/src/components/chat/respondToThreadApproval.ts` | `apps/server/integration/TestProviderAdapter.integration.ts` |
| 2% | 13 | 167 | `apps/web/src/components/chat/useChatAutomationSetup.ts` | `apps/mobile/src/features/threads/ThreadDetailScreen.tsx` |
| 2% | 2 | 34 | `apps/web/src/components/tasks/useTaskDelegateTarget.ts` | `apps/web/src/components/chat/useAutoBalanceUpdateBanner.tsx` |
| 2% | 3 | 31 | `apps/web/src/lib/kanbanTaskCreate.ts` | `apps/server/src/project/AgentSessionImporter.ts` |
| 2% | 8 | 78 | `packages/contracts/src/todo.ts` | `packages/contracts/src/server.ts` |
| 2% | 2 | 46 | `apps/analytics/forwarder/worker.test.ts` | `apps/mobile/src/features/settings/environment-maintenance.test.ts` |
| 2% | 1 | 40 | `apps/desktop/src/windowsShellAppUserModel.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 2% | 1 | 45 | `apps/marketing/scripts/visual-quality.test.mjs` | `oxlint-plugin-t3code/rules/namespace-node-imports.test.ts` |
| 2% | 4 | 172 | `apps/server/src/computer/computerAuditHistory.test.ts` | `apps/desktop/src/app/DesktopAppActivation.test.ts` |
| 2% | 4 | 154 | `apps/server/src/computer/screenshotFrames.ts` | `apps/web/src/components/device/deviceHubApi.ts` |
| 2% | 5 | 85 | `apps/server/src/hostAuth/grantVerifier.ts` | `apps/web/src/state/pullRequests.ts` |
| 2% | 10 | 461 | `apps/web/src/components/DevicePanel.logic.ts` | `apps/web/src/components/ProviderUpdateLaunchNotification.logic.ts` |
| 2% | 2 | 77 | `apps/web/src/components/ProviderUsageResetCredits.browser.tsx` | `apps/web/src/components/diffs/DiffFileTree.test.tsx` |
| 2% | 57 | 1063 | `apps/web/src/components/SidebarSearchPalette.tsx` | `apps/web/src/components/CommandPalette.tsx` |
| 2% | 1 | 40 | `apps/web/src/components/chat/ComposerTransportNotice.browser.tsx` | `apps/desktop/gnome-extension/captureService.js` |
| 2% | 8 | 181 | `apps/web/src/components/chat/project/useProjectAgentSummaries.browser.tsx` | `apps/mobile/src/state/use-thread-outbox-drain.test.ts` |
| 2% | 10 | 357 | `apps/web/src/components/chat/streamingMarkdownBlocks.ts` | `apps/web/src/terminal/ghostty/surface.ts` |
| 2% | 90 | 596 | `apps/web/src/components/chat/useChatTurnFollowUps.ts` | `apps/web/src/components/ChatView.tsx` |
| 2% | 2 | 16 | `apps/web/src/lib/markdownTaskList.ts` | `apps/web/src/browserFaviconLogic.ts` |
| 2% | 7 | 207 | `apps/web/src/lib/workspaceEditorSession.ts` | `apps/mobile/src/features/showcase/ShowcaseCaptureCoordinator.tsx` |
| 2% | 1 | 20 | `apps/web/src/test/reactCompiler.ts` | `apps/web/src/components/chat/workspaceFileDrop.ts` |
| 2% | 4 | 47 | `packages/contracts/src/orchestration.claudeCacheReview.test.ts` | `packages/contracts/src/preview.test.ts` |
| 2% | 4 | 61 | `scripts/lib/mac-dmg-finalize.test.ts` | `apps/desktop/src/snapShot/captureConfigEdit.test.ts` |
| 2% | 50 | 1474 | `apps/web/src/components/BrowserPanel.tsx` | `apps/web/src/components/Sidebar.tsx` |
| 2% | 6 | 307 | `apps/web/src/components/LocalVideoPreview.browser.tsx` | `apps/web/src/browser/browserRecording.ts` |
| 2% | 4 | 48 | `apps/web/src/components/chat/project/projectAgentOverview.logic.test.ts` | `packages/contracts/src/preview.test.ts` |
| 2% | 16 | 469 | `apps/web/src/composerSlashCommands.ts` | `apps/web/src/components/ChatView.logic.ts` |
| 2% | 3 | 51 | `scripts/install-git-hooks.ts` | `apps/desktop/src/app/DesktopAppActivation.ts` |
| 2% | 2 | 37 | `apps/desktop/src/serverListeningDetector.ts` | `apps/mobile/src/features/updates/app-updates.ts` |
| 2% | 10 | 517 | `apps/server/src/editor/debugService.ts` | `apps/web/src/environments/primary/auth.ts` |
| 2% | 11 | 376 | `apps/server/src/githubInbox/Layers/GitHubInboxService.ts` | `apps/server/src/pullRequest/GitHubPullRequestProvider.ts` |
| 2% | 2 | 35 | `apps/server/src/managedAttachmentPrincipal.ts` | `apps/server/src/auth/utils.ts` |
| 2% | 3 | 66 | `apps/server/src/platform/mcpStdioTransport.ts` | `apps/desktop/src/app/DesktopAppActivationBroker.ts` |
| 2% | 5 | 109 | `apps/server/src/provider/acp/DevinSessionConfig.ts` | `apps/mobile/src/lib/attachmentUpload.ts` |
| 2% | 3 | 43 | `apps/server/src/provider/stripDiagnosticImages.ts` | `scripts/update-release-package-versions.ts` |
| 2% | 19 | 164 | `apps/web/src/components/account/OnboardingDialog.tsx` | `apps/web/src/components/onboarding/WelcomeWizard.tsx` |
| 2% | 12 | 278 | `apps/web/src/components/githubInbox/useGitHubInboxSidechat.ts` | `apps/web/src/components/BranchToolbarBranchSelector.tsx` |
| 2% | 4 | 173 | `apps/web/src/hooks/useExecutionImageSource.browser.tsx` | `apps/web/src/browser/HostedBrowserWebview.test.tsx` |
| 2% | 4 | 32 | `apps/web/src/hooks/useProviderStatusesForLocalConfig.ts` | `apps/web/src/modelSelection.ts` |
| 2% | 14 | 60 | `packages/contracts/src/desktopUpdate.ts` | `packages/contracts/src/ipc.ts` |
| 2% | 7 | 102 | `packages/shared/src/agentMentions.ts` | `apps/server/src/textGeneration/AntigravityTextGeneration.ts` |
| 2% | 1 | 41 | `apps/desktop/src/computerDesktopLifecycle.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 2% | 1 | 41 | `apps/desktop/src/notificationRetention.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 2% | 16 | 541 | `apps/server/src/editorAppIcons.ts` | `packages/shared/src/shell.ts` |
| 2% | 11 | 708 | `apps/server/src/git/Layers/codexTextGenerationIsolation.test.ts` | `apps/server/src/provider/Drivers/AntigravityDriver.test.ts` |
| 2% | 9 | 378 | `apps/server/src/provider/acp/OmpAcpSupport.test.ts` | `apps/server/src/provider/acp/AntigravityAcpSupport.test.ts` |
| 2% | 2 | 106 | `apps/server/src/pullRequests/repositoryResolution.test.ts` | `.github/scripts/check-nightly-release.test.cjs` |
| 2% | 17 | 326 | `apps/web/src/components/chat/SubagentRunCard.tsx` | `apps/web/src/components/pullRequest/PullRequestCodeTab.tsx` |
| 2% | 1 | 22 | `apps/web/src/components/chat/dockPaneOpenRequest.test.ts` | `apps/web/src/lib/terminalContext.test.ts` |
| 2% | 1 | 23 | `apps/web/src/components/chat/environment/EnvironmentRow.browser.tsx` | `apps/web/src/cloud/connectCliAuth.test.ts` |
| 2% | 2 | 35 | `apps/web/src/components/chat/group/groupLifecycle.logic.test.ts` | `apps/server/src/persistence/RepositoryErrorCorrelation.test.ts` |
| 2% | 7 | 110 | `apps/web/src/components/chat/project/useHubWorkItems.browser.tsx` | `apps/mobile/src/state/use-thread-outbox-drain.test.ts` |
| 2% | 10 | 301 | `apps/web/src/components/chat/toolCallGroup.logic.test.ts` | `packages/client-runtime/src/work-log/presentation.test.ts` |
| 2% | 17 | 328 | `apps/web/src/components/settings/BillingSettingsPanel.tsx` | `apps/web/src/components/GitActionsControl.tsx` |
| 2% | 7 | 114 | `apps/web/src/lib/rendererErrorDiagnostics.test.ts` | `apps/web/src/components/desktop/SnapShotCoordinator.test.ts` |
| 2% | 20 | 1530 | `apps/server/src/persistence/Layers/AutomationRepository.test.ts` | `apps/server/src/persistence/Layers/ProjectionRepositories.test.ts` |
| 2% | 6 | 142 | `apps/server/src/providerUsage/credentials.ts` | `apps/server/src/serviceLauncher.ts` |
| 2% | 6 | 148 | `apps/web/src/threadDetailPrewarm.test.ts` | `apps/desktop/src/window/QuitHold.test.ts` |
| 2% | 3 | 22 | `apps/marketing/src/components/docs/DocsA11yLabels.tsx` | `apps/web/src/components/preview/usePreviewBridge.ts` |
| 2% | 2 | 37 | `apps/profiles/src/lib/modelUsage.ts` | `apps/web/src/components/chat/ProviderModelPicker.test.tsx` |
| 2% | 19 | 340 | `apps/server/src/composerPrediction/composerPrediction.test.ts` | `apps/server/src/relay/AgentAwarenessRelay.test.ts` |
| 2% | 7 | 87 | `apps/server/src/providerUsage/http.ts` | `apps/web/src/lib/openPullRequestLink.ts` |
| 2% | 3 | 57 | `apps/server/src/remoteTransport/runtimeSmoke.ts` | `apps/desktop/src/snapShot/SnapShotAccessibility.ts` |
| 2% | 10 | 230 | `apps/web/src/components/chat/group/useGroupLibrary.ts` | `apps/mobile/src/features/usage/UsageLimitsSection.tsx` |
| 2% | 7 | 52 | `apps/web/src/hooks/useCustomTitleBarPreference.ts` | `apps/web/src/routes/__root.tsx` |
| 2% | 3 | 39 | `apps/web/src/messageTextSegments.ts` | `apps/web/src/composer-logic.ts` |
| 2% | 4 | 19 | `apps/web/src/tasksSurface.ts` | `packages/client-runtime/src/state/pullRequestRouting.ts` |
| 2% | 2 | 32 | `packages/shared/src/asyncUserInput.test.ts` | `apps/web/src/pendingUserInput.test.ts` |
| 2% | 3 | 93 | `apps/desktop/scripts/betterwright-smoke.ts` | `apps/mobile/src/lib/attachmentDownload.test.ts` |
| 2% | 5 | 109 | `apps/marketing/src/components/Testimonials.tsx` | `apps/web/src/components/pullRequest/pullRequestPresentation.tsx` |
| 2% | 3 | 86 | `apps/server/src/device/DeviceManager.contractLimits.test.ts` | `apps/server/src/environment/ServerEnvironmentMachine.test.ts` |
| 2% | 7 | 143 | `apps/server/src/githubInbox/githubInbox.logic.ts` | `apps/server/src/pullRequest/GitHubPullRequestProvider.ts` |
| 2% | 13 | 141 | `apps/server/src/persistence/Layers/ProjectPullRequestPins.test.ts` | `apps/server/src/persistence/Layers/ProjectionRepositories.test.ts` |
| 2% | 8 | 224 | `apps/web/src/hooks/useProviderUsageSummary.test.tsx` | `apps/web/src/providerInstances.test.ts` |
| 2% | 3 | 156 | `apps/web/src/lib/deviceFrameSource.test.ts` | `apps/desktop/src/app/DesktopAppActivationBroker.test.ts` |
| 2% | 1 | 42 | `apps/web/src/lib/pdf/pdfZoom.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 2% | 3 | 99 | `apps/web/src/lib/pullRequestContext.test.ts` | `apps/desktop/src/updates/remoteUpdateFlow.test.ts` |
| 2% | 4 | 76 | `apps/server/src/auth/Layers/RemotePairingControlPlane.ts` | `apps/server/integration/TestProviderAdapter.integration.ts` |
| 2% | 4 | 59 | `apps/server/src/desktopUpdateBridge.ts` | `apps/desktop/src/app/DesktopAppActivation.ts` |
| 2% | 14 | 458 | `apps/server/src/provider/skillsCatalog.ts` | `apps/server/src/provider/Drivers/ClaudeSkills.ts` |
| 2% | 4 | 59 | `apps/server/src/server/readiness.ts` | `apps/server/src/cli/service.test.ts` |
| 2% | 7 | 111 | `apps/server/src/terminal/providerAuthentication.ts` | `apps/server/src/sourceControl/ForgejoSourceControlProvider.ts` |
| 2% | 29 | 2119 | `apps/web/src/appSettings.ts` | `apps/web/src/modelSelection.ts` |
| 2% | 5 | 216 | `apps/web/src/onboarding/logic.test.ts` | `apps/web/src/components/ChatView.logic.test.ts` |
| 2% | 2 | 78 | `scripts/computer-use-fixtures/background-input-target.cjs` | `apps/mobile/metro.config.js` |
| 2% | 13 | 308 | `apps/server/src/computer/computerVisibleUse.test.ts` | `apps/web/src/components/ChatView.logic.test.ts` |
| 2% | 7 | 118 | `apps/server/src/mcpConnectors/Layers/McpConnectors.ts` | `apps/server/src/provider/Drivers/AntigravityDriver.ts` |
| 2% | 12 | 510 | `apps/server/src/persistence/Layers/ManagedAttachments.ts` | `apps/server/src/persistence/Layers/ProjectionThreadMessages.ts` |
| 2% | 5 | 152 | `apps/server/src/provider/openCodeV2Mapping.ts` | `apps/server/src/provider/providerSnapshot.ts` |
| 2% | 15 | 107 | `apps/web/src/components/OneTimeCoachmark.tsx` | `apps/web/src/components/RightPanelTabs.tsx` |
| 2% | 7 | 57 | `apps/web/src/lib/projectRelocation.test.ts` | `apps/server/src/orchestration/decider.pullRequests.test.ts` |
| 2% | 9 | 188 | `apps/desktop/src/desktopMigrationRecovery.ts` | `apps/mobile/src/lib/composerImages.ts` |
| 2% | 2 | 14 | `apps/desktop/src/showInFileManager.ts` | `apps/server/src/provider/Drivers/AntigravitySkills.ts` |
| 2% | 22 | 441 | `apps/server/src/orchestration/quitResume.ts` | `apps/server/src/provider/Layers/GrokAdapter.ts` |
| 2% | 5 | 302 | `apps/server/src/providerUsage/providers/codex.test.ts` | `apps/mobile/src/features/agent-awareness/notificationNavigation.test.ts` |
| 2% | 4 | 151 | `apps/web/scripts/parallel-transform.ts` | `apps/mobile/modules/t3-markdown-text/src/useHighlightedCode.ts` |
| 2% | 1 | 48 | `apps/web/scripts/production-assets.test.ts` | `oxlint-plugin-t3code/rules/namespace-node-imports.test.ts` |
| 2% | 2 | 22 | `apps/web/src/components/chat/AssistantSelectionsSummaryChip.tsx` | `apps/web/src/browserFaviconLogic.ts` |
| 2% | 16 | 563 | `apps/web/src/components/chat/environment/environmentPullRequest.logic.ts` | `apps/web/src/components/pullRequest/pullRequestDetail.logic.ts` |
| 2% | 1 | 43 | `apps/web/src/hooks/useSmoothStreamedText.energy.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 2% | 1 | 35 | `apps/web/src/lib/providerDiscoveryInvalidation.ts` | `apps/mobile/src/features/diagnostics/crash-log-model.ts` |
| 2% | 8 | 271 | `packages/shared/src/diagnosticsRedaction.ts` | `packages/contracts/src/resourceTelemetry.ts` |
| 2% | 5 | 180 | `apps/api/src/identity/orgProvisioning.test.ts` | `apps/web/src/components/ProviderUpdateEnvironmentRows.test.tsx` |
| 2% | 9 | 96 | `apps/server/perf/threadDetailSnapshot.perf.test.ts` | `apps/server/integration/OrchestrationEngineHarness.integration.ts` |
| 2% | 27 | 614 | `apps/server/src/orchestration/startupTurnReconciliation.test.ts` | `apps/server/src/orchestration/projector.test.ts` |
| 2% | 5 | 139 | `apps/api/src/identity/hostSecretStore.ts` | `infra/relay/src/environments/EnvironmentCredentials.ts` |
| 2% | 3 | 67 | `apps/desktop/src/fixtures/posixBackendShutdown.mjs` | `packages/ssh/src/runnerProcess.test.ts` |
| 2% | 4 | 266 | `apps/server/src/device/DeviceManager.observation.test.ts` | `apps/web/src/components/chat/SnapShotAttachmentDetails.test.tsx` |
| 2% | 4 | 74 | `apps/server/src/hostIdentity/store.ts` | `apps/server/src/serviceLauncher.ts` |
| 2% | 9 | 159 | `apps/server/src/projectAgent/workerHealth.test.ts` | `apps/web/src/components/ChatView.logic.test.ts` |
| 2% | 12 | 208 | `apps/web/src/components/AppRail.tsx` | `apps/web/src/components/RightPanelTabs.tsx` |
| 2% | 3 | 86 | `apps/web/src/hooks/useProviderModelCatalog.browser.tsx` | `apps/server/src/bootstrap.test.ts` |
| 2% | 2 | 12 | `apps/profiles/src/app/not-found.tsx` | `apps/web/src/components/desktop/SshPasswordPromptDialog.tsx` |
| 2% | 3 | 107 | `apps/server/src/codexHomePaths.test.ts` | `apps/desktop/src/settings/DesktopClientSettings.test.ts` |
| 2% | 3 | 182 | `apps/server/src/computer/ComputerSpaceBroker.test.ts` | `apps/web/src/components/ProviderUpdateEnvironmentRows.test.tsx` |
| 2% | 52 | 1222 | `apps/server/src/orchestration/providerRuntimeActivityProjection.test.ts` | `apps/server/src/orchestration/Layers/ProviderRuntimeIngestion.test.ts` |
| 2% | 11 | 72 | `apps/server/src/persistence/Migrations/144_ProjectionThreadsPullRequestLinks.test.ts` | `apps/server/src/persistence/Layers/ProjectionRepositories.test.ts` |
| 2% | 2 | 30 | `apps/server/src/provider/acp/TurnCancellation.test.ts` | `apps/server/scripts/evaluate-thread-titles.ts` |
| 2% | 5 | 37 | `apps/web/src/components/kanban/KanbanTaskProjectPicker.tsx` | `apps/web/src/components/chat/WorktreeSetupCard.tsx` |
| 2% | 8 | 49 | `apps/web/src/components/settings/ProductAnalyticsSettingsPanel.tsx` | `apps/web/src/components/settings/SnapShotSettings.tsx` |
| 2% | 1 | 47 | `apps/web/src/components/tasks/useTaskDelegateTarget.browser.tsx` | `apps/server/src/pullRequest/pullRequestSyncKey.test.ts` |
| 2% | 10 | 176 | `apps/web/src/hooks/useComposerCommandMenuItems.test.ts` | `packages/client-runtime/src/state/entities.test.ts` |
| 2% | 15 | 610 | `apps/web/src/lib/pullRequestReactQuery.action.test.ts` | `apps/web/src/components/pullRequest/pullRequestDetail.logic.test.ts` |
| 2% | 1 | 10 | `apps/web/src/lib/suppressQueryResponses.ts` | `apps/web/src/cloud/managedAuth.tsx` |
| 2% | 2 | 21 | `apps/web/src/recentViewActivation.logic.ts` | `apps/desktop/src/preview/BrowserImport/ChromiumCookies.ts` |
| 2% | 6 | 290 | `packages/shared/src/hostSecrets.ts` | `apps/web/src/lib/imageCompression.ts` |
| 2% | 3 | 67 | `apps/e2e/src/harness/cloudflaredFixture.mjs` | `apps/desktop/src/snapShot/NiriSnapShot.test.ts` |
| 2% | 4 | 42 | `apps/server/src/computer/waitForControl.ts` | `apps/web/src/environments/primary/auth.ts` |
| 2% | 21 | 1359 | `apps/web/src/appSettings.test.ts` | `apps/server/src/serverSettings.test.ts` |
| 2% | 6 | 560 | `apps/web/src/components/ChatMarkdown.test.tsx` | `apps/web/src/components/ChatMarkdown.test.tsx` |
| 2% | 2 | 26 | `apps/web/src/components/chat/McpServerIcon.tsx` | `packages/client-runtime/src/work-log/userInput.ts` |
| 2% | 137 | 2996 | `apps/web/src/components/chat/MessagesTimeline.tsx` | `apps/web/src/components/chat/MessagesTimeline.tsx` |
| 2% | 3 | 87 | `apps/web/src/components/pullRequest/pullRequestStack.logic.ts` | `apps/web/src/components/ProviderUpdateLaunchNotification.logic.ts` |
| 2% | 9 | 194 | `apps/web/src/lib/projectCreation.test.ts` | `apps/server/src/orchestration/PullRequestSyncReactor.test.ts` |
| 2% | 7 | 219 | `packages/shared/src/subagents.test.ts` | `packages/client-runtime/src/state/subagentRuntime.test.ts` |
| 2% | 3 | 112 | `scripts/computer-use-fixtures/resource-sampling.test.ts` | `apps/desktop/src/snapShot/GnomeCaptureSetup.test.ts` |
| 2% | 2 | 40 | `apps/desktop/native/appsnap/EscapePhysicalClassifier.swift` | `apps/mobile/scripts/fixtures/NotificationCenterManagerRegression.swift` |
| 2% | 2 | 50 | `apps/marketing/src/app/api/inbound-email/route.ts` | `apps/web/src/components/permissions/usePermissionStatus.ts` |
| 2% | 2 | 67 | `apps/server/src/accountSsoCallback.test.ts` | `apps/desktop/scripts/electron-launcher.test.mjs` |
| 2% | 1 | 45 | `apps/server/src/computer/computerTurnTiming.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 2% | 4 | 49 | `apps/server/src/desktopParentLifetime.ts` | `apps/server/src/textGeneration/ClaudeTextGeneration.test.ts` |
| 2% | 2 | 56 | `apps/server/src/mcpConnectors/importServers.test.ts` | `scripts/knip-schemas.test.ts` |
| 2% | 4 | 79 | `apps/server/src/orchestration/providerSessionThread.test.ts` | `apps/server/src/orchestration/commandInvariants.test.ts` |
| 2% | 1 | 40 | `apps/server/src/providerUsage/providers/localCredential.test.ts` | `oxlint-plugin-t3code/rules/no-global-process-runtime.test.ts` |
| 2% | 15 | 140 | `apps/web/src/components/CreateGitHubProjectFields.tsx` | `apps/web/src/components/settings/KeybindingsSettings.tsx` |
| 2% | 4 | 60 | `apps/web/src/components/chat/BackgroundTasksDockPane.browser.tsx` | `apps/web/src/components/ThreadNotificationCoordinator.test.tsx` |
| 2% | 2 | 58 | `apps/web/src/hooks/useTranslation.browser.tsx` | `apps/web/src/session-logic.command-output.test.ts` |
| 2% | 2 | 106 | `apps/web/src/lib/runningChatsQuitConfirmation.test.ts` | `apps/desktop/src/app/DesktopAppActivation.test.ts` |
| 2% | 9 | 358 | `apps/web/src/lib/toolCallLabel.test.ts` | `apps/web/src/components/ChatView.logic.test.ts` |
| 2% | 3 | 103 | `packages/contracts/src/browserAnnotations.ts` | `apps/web/src/lib/terminalContext.ts` |
| 2% | 1 | 45 | `packages/shared/src/agentMentions.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 2% | 8 | 197 | `apps/desktop/src/browserAutomation/vaultKeyProtection.ts` | `apps/server/src/cli/theme.ts` |
| 2% | 10 | 321 | `apps/server/src/agentGateway/delegatedAttachments.test.ts` | `packages/contracts/src/orchestration.test.ts` |
| 2% | 32 | 590 | `apps/web/src/components/chat/ComposerModelPicker.tsx` | `apps/web/src/components/pullRequest/PullRequestDetailPanel.tsx` |
| 2% | 6 | 224 | `apps/web/src/components/chat/ComposerUsageLimitCapsules.browser.tsx` | `apps/web/src/components/settings/ProviderSetupSection.test.tsx` |
| 2% | 18 | 162 | `apps/web/src/components/settings/accounts/ProviderInstanceEnvironmentEditor.tsx` | `apps/web/src/components/GitActionsControl.tsx` |
| 2% | 3 | 18 | `apps/web/src/lib/desktopZoom.ts` | `apps/web/src/components/AppSidebarLayout.tsx` |
| 2% | 3 | 27 | `apps/server/src/agentGateway/creationCoordinator.groupAllowlist.test.ts` | `apps/web/src/browserFaviconStore.test.ts` |
| 2% | 3 | 49 | `apps/server/src/auth/remoteAccess.ts` | `apps/server/src/project/AgentSessionImporter.ts` |
| 2% | 5 | 33 | `apps/server/src/hostAuth/apiJwksCache.ts` | `apps/desktop/src/snapShot/PortalCaptureShortcut.ts` |
| 2% | 9 | 135 | `apps/server/src/provider/claudePluginSkills.ts` | `apps/desktop/src/snapShot/snapShot.ts` |
| 2% | 12 | 365 | `apps/server/src/provider/providerProcessEnv.ts` | `apps/server/src/provider/antigravityAuthSupport.ts` |
| 2% | 3 | 16 | `apps/server/src/provider/providerRuntimeEventIdentity.ts` | `apps/server/src/orchestration/decider.active-order.test.ts` |
| 2% | 3 | 24 | `apps/server/src/providerUsage/types.ts` | `apps/desktop/src/app/DesktopEnvironment.ts` |
| 2% | 5 | 52 | `apps/web/src/lib/editorCenterMode.test.ts` | `apps/server/src/relay/AgentAwarenessRelay.test.ts` |
| 2% | 4 | 35 | `packages/shared/src/browserShortcuts.ts` | `apps/desktop/src/window/QuitHold.ts` |
| 2% | 2 | 68 | `apps/profiles/src/lib/shareCards.ts` | `packages/shared/src/themePreview.ts` |
| 2% | 1 | 8 | `apps/server/src/codexErrorClassification.ts` | `packages/contracts/src/browserImport.ts` |
| 2% | 11 | 148 | `apps/server/src/provider/acp/AcpElicitationSupport.ts` | `apps/server/src/provider/acp/XAiAcpExtension.ts` |
| 2% | 1 | 46 | `apps/server/src/serverShutdown.ts` | `apps/desktop/gnome-extension/captureService.js` |
| 2% | 2 | 110 | `apps/web/perf/status-animation-runner.mjs` | `apps/web/src/components/chat/draftHeroTransition.ts` |
| 2% | 6 | 140 | `apps/web/src/components/OnboardingProvidersStep.browser.tsx` | `apps/mobile/src/lib/modelOptions.test.ts` |
| 2% | 4 | 35 | `apps/web/src/components/chat/environment/EnvironmentBackgroundTasksSection.tsx` | `apps/mobile/src/features/threads/worktree-setup-card.tsx` |
| 2% | 2 | 41 | `apps/web/src/components/profile/ActivityHeatmap.tsx` | `apps/mobile/src/features/threads/git/gitSheetComponents.tsx` |
| 2% | 5 | 158 | `apps/web/src/computerStateStore.test.ts` | `apps/desktop/src/snapShot/snapShot.test.ts` |
| 2% | 12 | 436 | `apps/web/src/hooks/useComposerCommandMenuItems.ts` | `apps/mobile/src/features/threads/use-composer-command-menu.ts` |
| 2% | 7 | 93 | `apps/web/src/lib/groupProjects.test.ts` | `apps/web/src/components/CommandPalette.logic.test.ts` |
| 2% | 1 | 46 | `apps/web/src/lib/localFolderMentions.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 2% | 4 | 52 | `apps/web/src/projectInstructionsStore.ts` | `apps/web/src/browserHistoryStore.ts` |
| 2% | 6 | 61 | `apps/desktop/src/subframeNavigationGuard.test.ts` | `apps/web/src/lib/openPullRequestLink.test.ts` |
| 2% | 2 | 23 | `apps/server/src/persistence/MigrationSchemaTooNewStartupBlock.ts` | `apps/desktop/src/app/DesktopAppActivation.test.ts` |
| 2% | 11 | 82 | `apps/server/src/terminal/Layers/NodePTY.test.ts` | `apps/server/src/process/externalLauncher.test.ts` |
| 2% | 12 | 160 | `apps/web/src/components/chat/group/GroupGeneralSection.tsx` | `apps/web/src/components/settings/KeybindingsSettings.tsx` |
| 2% | 2 | 29 | `apps/web/src/components/chat/userTurnMarker.ts` | `apps/web/src/browserFaviconLogic.ts` |
| 2% | 8 | 389 | `apps/web/src/components/codeEditor/editorDebugStore.ts` | `apps/mobile/src/lib/composerImages.ts` |
| 2% | 3 | 41 | `apps/server/src/accountCredentialLock.ts` | `apps/desktop/src/snapShot/CaptureShortcutConfig.ts` |
| 2% | 11 | 145 | `apps/server/src/managedAttachmentCleanup.ts` | `apps/server/src/serverRuntimeStartup.ts` |
| 2% | 7 | 165 | `apps/server/src/provider/acp/AcpElicitationSupport.test.ts` | `apps/server/src/provider/Layers/CodexSessionRuntime.test.ts` |
| 2% | 18 | 412 | `apps/web/src/components/chat/ComposerSubagentStrip.logic.test.ts` | `apps/server/src/orchestration/projector.test.ts` |
| 2% | 6 | 33 | `apps/web/src/components/chat/project/CoordinatorSuggestions.tsx` | `apps/web/src/components/AgentsPanel.tsx` |
| 2% | 3 | 22 | `apps/web/src/components/codeEditor/editorSaveShortcut.ts` | `apps/web/src/components/AppSidebarLayout.tsx` |
| 2% | 4 | 43 | `packages/shared/src/threadEnvironment.ts` | `apps/web/src/components/BranchToolbar.logic.ts` |
| 2% | 8 | 107 | `apps/api/src/remote/cloudflare.ts` | `apps/desktop/src/preview/FaviconCapture.ts` |
| 2% | 2 | 44 | `apps/desktop/native/appsnap/tests/permission-registration/main.swift` | `apps/mobile/scripts/fixtures/NotificationCenterManagerRegression.swift` |
| 2% | 2 | 32 | `apps/server/src/project/githubProjectRegistration.test.ts` | `apps/desktop/src/snapShot/GnomeCaptureSetup.test.ts` |
| 2% | 5 | 80 | `apps/server/src/workspaceFileChanges.test.ts` | `scripts/lib/third-party-licenses.test.ts` |
| 2% | 4 | 55 | `apps/web/scripts/hugeicon-snippet.mjs` | `apps/web/src/vscodeThemeImport.ts` |
| 2% | 3 | 20 | `apps/web/src/components/pullRequest/pullRequestFocus.ts` | `apps/web/src/lib/openPullRequestLink.ts` |
| 2% | 1 | 23 | `apps/web/src/computerControlMode.test.ts` | `apps/web/src/environments/primary/context.ts` |
| 2% | 1 | 47 | `apps/web/src/featureTour/logic.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 2% | 10 | 329 | `apps/web/src/lib/pullRequestMutationOptions.ts` | `apps/server/src/pullRequest/AzureDevOpsPullRequestCli.ts` |
| 2% | 37 | 940 | `apps/web/src/notifications/taskCompletion.logic.test.ts` | `apps/web/src/components/ChatView.logic.test.ts` |
| 2% | 4 | 138 | `packages/contracts/src/hostAuth.test.ts` | `apps/desktop/src/ipc/methods/preview.test.ts` |
| 2% | 2 | 24 | `scripts/lib/desktop-artifact-identity.ts` | `apps/desktop/src/app/DesktopAppActivation.test.ts` |
| 2% | 9 | 410 | `apps/api/src/billing/service.ts` | `apps/mobile/src/lib/attachmentUpload.ts` |
| 2% | 7 | 237 | `apps/server/src/provider/codexDiscoveryCatalog.test.ts` | `apps/mobile/src/lib/modelOptions.test.ts` |
| 2% | 4 | 49 | `apps/server/src/tailscaleEndpoint.ts` | `apps/desktop/scripts/electron-launcher.mjs` |
| 2% | 2 | 97 | `apps/web/src/components/chat/deferredChatMount.test.ts` | `apps/desktop/src/electron/WindowsForeground.test.ts` |
| 2% | 4 | 38 | `apps/web/src/components/pullRequest/PullRequestDockPane.tsx` | `apps/web/src/components/pullRequest/PullRequestGhosts.tsx` |
| 2% | 16 | 391 | `apps/web/src/components/settings/UsageAnalyticsCharts.tsx` | `apps/web/src/components/usage/UsageProviderChart.tsx` |
| 2% | 3 | 65 | `apps/web/src/lib/relativeTime.ts` | `apps/web/src/timestampFormat.ts` |
| 2% | 18 | 185 | `packages/shared/src/executable.ts` | `packages/shared/src/shell.ts` |
| 2% | 13 | 370 | `apps/server/src/agentGateway/threadSummary.ts` | `apps/server/src/pullRequest/PullRequestProvider.ts` |
| 2% | 4 | 109 | `apps/web/src/components/chat/DockExplorerPane.autosave.browser.tsx` | `apps/web/src/components/onboarding/WelcomeWizard.test.tsx` |
| 2% | 4 | 122 | `apps/web/src/components/chat/TranscriptSelectionAction.browser.tsx` | `apps/mobile/src/widgets/subscriptionUsageSnapshot.test.ts` |
| 2% | 7 | 61 | `apps/web/src/components/settings/providers/ProviderInstallFieldControl.tsx` | `apps/web/src/components/settings/CustomModelEditor.tsx` |
| 2% | 1 | 25 | `apps/web/src/lib/hosts/editorRecovery.ts` | `apps/web/src/portDiscoveryState.ts` |
| 2% | 19 | 544 | `apps/analytics/src/components.tsx` | `apps/web/src/components/settings/KeybindingsSettings.tsx` |
| 2% | 5 | 120 | `apps/desktop/src/localHtmlPreviewProtocol.ts` | `apps/web/src/openVsxThemes.ts` |
| 2% | 11 | 308 | `apps/server/src/agentGateway/remoteTools.ts` | `apps/web/src/connection/storage.ts` |
| 2% | 3 | 105 | `apps/server/src/device/IosSimulatorBackend.simctl.test.ts` | `apps/server/src/environment/ServerEnvironmentMachine.test.ts` |
| 2% | 8 | 205 | `apps/server/src/provider/claudeWorkflowRuntime.test.ts` | `apps/server/src/usage/UsageService.test.ts` |
| 2% | 3 | 84 | `apps/server/src/providerUsage/rateLimitResilience.test.ts` | `apps/web/src/components/chat/ContextWindowMeter.logic.test.ts` |
| 2% | 3 | 168 | `packages/shared/src/processRuntime.ts` | `apps/server/src/orchestration/ThreadLiveEventCoalescer.test.ts` |
| 2% | 3 | 235 | `scripts/build-desktop-artifact-mac-config.test.ts` | `apps/server/src/persistence/RepositoryErrorCorrelation.test.ts` |
| 2% | 5 | 124 | `scripts/lib/mac-notarization.test.ts` | `apps/mobile/src/lib/attachmentDownload.test.ts` |
| 2% | 8 | 186 | `apps/desktop/src/audioLevelMonitor.ts` | `apps/server/src/diagnostics/TraceDiagnostics.ts` |
| 2% | 6 | 47 | `apps/web/src/components/tasks/TaskQuickAdd.tsx` | `apps/web/src/components/chat/ChatHeader.tsx` |
| 2% | 14 | 465 | `apps/web/src/providerModelOptions.ts` | `apps/web/src/components/chat/TraitsPicker.tsx` |
| 2% | 21 | 232 | `apps/web/src/routes/-automations.list.tsx` | `apps/web/src/components/GitActionsControl.tsx` |
| 2% | 7 | 213 | `scripts/claude-cache-report.ts` | `apps/server/src/orchestration/ActivityPayloadProjection.ts` |
| 2% | 6 | 126 | `apps/desktop/src/appSnapIpc.ts` | `apps/server/src/orchestration/ActivityPayloadProjection.ts` |
| 2% | 3 | 42 | `apps/desktop/src/bundleSwapDetection.test.ts` | `apps/server/src/pullRequest/bitbucketPullRequestJson.test.ts` |
| 2% | 12 | 245 | `apps/server/src/codexGeneratedImages.ts` | `apps/server/src/provider/Layers/CursorAdapter.ts` |
| 2% | 9 | 821 | `apps/server/src/externalMcp/Layers/ExternalMcpRepository.test.ts` | `apps/server/src/persistence/Migrations/016_CanonicalizeModelSelections.test.ts` |
| 2% | 3 | 70 | `apps/server/src/provider/openCodeAuthPaths.test.ts` | `apps/desktop/src/shell/DesktopShellEnvironment.test.ts` |
| 2% | 15 | 110 | `apps/web/src/components/DiffPanelFileJumpMenu.tsx` | `apps/web/src/components/RightPanelTabs.tsx` |
| 2% | 1 | 47 | `apps/web/src/components/profile/avatarImage.ts` | `apps/desktop/src/snapShot/GlobalShiftShortcutProcess.ts` |
| 2% | 3 | 64 | `scripts/verify-release-source-provenance.ts` | `scripts/release-smoke.ts` |
| 2% | 2 | 29 | `apps/desktop/src/stopNativeHelper.ts` | `apps/desktop/src/app/DesktopAppActivation.test.ts` |
| 2% | 13 | 180 | `apps/desktop/src/voiceTranscription.ts` | `apps/web/src/components/ChatView.logic.ts` |
| 2% | 3 | 182 | `apps/marketing/scripts/seo-smoke.mjs` | `scripts/install.test.ts` |
| 2% | 2 | 40 | `apps/marketing/src/app/changelog/[version]/page.tsx` | `apps/mobile/src/features/threads/git/GitConfirmSheet.tsx` |
| 2% | 11 | 252 | `apps/server/src/agentGateway/todoTools.test.ts` | `apps/server/src/orchestration/ThreadPullRequestReactor.test.ts` |
| 2% | 2 | 75 | `apps/server/src/device/wsDeviceHandlers.test.ts` | `apps/server/src/orchestration/Layers/ThreadDeletionReactor.test.ts` |
| 2% | 4 | 63 | `apps/web/src/components/tasks/TaskCardProperties.tsx` | `apps/web/src/components/chat/ChangedFilesTree.tsx` |
| 2% | 3 | 148 | `apps/web/src/lib/composerImageBlobStore.ts` | `apps/mobile/src/widgets/subscriptionUsageSnapshot.ts` |
| 2% | 1 | 26 | `apps/api/cloudflare/worker-configuration.d.ts` | `apps/web/src/lib/terminalCloseShortcut.test.ts` |
| 2% | 4 | 219 | `apps/desktop/native/appsnap/ActivationShield.swift` | `apps/mobile/modules/t3-terminal/ios/T3TerminalView.swift` |
| 2% | 5 | 66 | `apps/marketing/src/components/PrivacySection.tsx` | `apps/web/src/components/pullRequest/pullRequestPresentation.tsx` |
| 2% | 1 | 50 | `apps/server/src/device/bootOwnership.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 2% | 8 | 99 | `apps/web/src/components/chat/useFileLineCommenting.ts` | `apps/web/src/components/settings/ThemeEditorPanel.tsx` |
| 2% | 38 | 855 | `apps/web/src/components/kanban/kanban.logic.test.ts` | `apps/web/src/components/ChatView.logic.test.ts` |
| 2% | 2 | 83 | `apps/web/src/components/terminal/terminalRuntimeTypes.ts` | `apps/mobile/src/features/terminal/TerminalContextSheet.tsx` |
| 2% | 3 | 103 | `apps/web/src/lib/accountReactQuery.ts` | `apps/desktop/src/permissions/MacPermissionHelper.ts` |
| 2% | 1 | 32 | `apps/web/src/lib/animationTimelineSync.browser.ts` | `apps/web/src/cloud/connectCliAuth.test.ts` |
| 2% | 4 | 337 | `apps/web/src/rightDockStore.logic.test.ts` | `apps/web/src/terminalUiStateStore.test.ts` |
| 2% | 1 | 36 | `scripts/ci-install-key.ts` | `scripts/apply-web-brand-assets.ts` |
| 2% | 8 | 387 | `apps/server/src/hostConnectivity.ts` | `apps/server/src/serviceLauncher.ts` |
| 2% | 14 | 177 | `apps/server/src/orchestration/Layers/UsageLimitRecoveryReactor.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.ts` |
| 2% | 19 | 415 | `apps/server/src/provider/providerRuntimeEventPump.test.ts` | `apps/server/src/provider/Layers/GrokAdapter.test.ts` |
| 2% | 5 | 95 | `apps/server/src/workspace/realPathContainment.ts` | `apps/web/src/hooks/useTheme.ts` |
| 2% | 4 | 115 | `apps/web/src/components/hosts/RemoteConnectionsAnnouncement.tsx` | `apps/web/src/components/cloud/ConnectOnboardingDialog.tsx` |
| 2% | 2 | 50 | `apps/web/src/components/settings/ProviderUsageSettingsPanel.browser.tsx` | `apps/server/src/usage/opencodeUsageReader.ts` |
| 2% | 12 | 113 | `apps/web/src/components/tasks/useTaskAgent.ts` | `apps/mobile/src/features/threads/ThreadDetailScreen.tsx` |
| 2% | 4 | 69 | `apps/web/src/components/terminal/terminalRuntimeRegistry.ts` | `apps/desktop/src/preview/BrowserImport/BrowserImport.ts` |
| 2% | 7 | 105 | `apps/web/src/lib/hosts/workspaceResources.test.ts` | `apps/server/src/provider/acp/AntigravityAcpSupport.test.ts` |
| 2% | 5 | 89 | `apps/web/src/lib/rpcRequestQueue.test.ts` | `apps/web/src/authBootstrap.test.ts` |
| 2% | 3 | 20 | `apps/web/src/remoteProjectSpacesStore.ts` | `apps/web/src/browserFaviconStore.ts` |
| 2% | 6 | 57 | `apps/web/src/routes/_chat.pull-requests.index.tsx` | `apps/web/src/components/settings/settingsLayout.tsx` |
| 2% | 4 | 30 | `packages/shared/src/threadHierarchy.ts` | `packages/client-runtime/src/state/threadSort.ts` |
| 2% | 7 | 63 | `apps/server/src/automation/Layers/AutomationScheduler.ts` | `apps/server/src/relay/AgentAwarenessRelay.ts` |
| 2% | 3 | 261 | `apps/server/src/provider/providerProcessEnv.test.ts` | `apps/desktop/src/shell/DesktopShellEnvironment.test.ts` |
| 2% | 3 | 148 | `apps/server/src/terminal/managedTerminalWrappers.test.ts` | `apps/web/src/browser/desktopTabLifetime.test.ts` |
| 2% | 3 | 120 | `apps/web/src/components/BrowserTabStrip.browser.tsx` | `apps/web/src/components/preview/openTerminalLinkInPreview.test.ts` |
| 2% | 18 | 447 | `apps/web/src/components/SidebarSearchPalette.logic.ts` | `apps/web/src/components/Sidebar.logic.ts` |
| 2% | 9 | 117 | `apps/web/src/components/chat/useComposerAttachmentPersistence.ts` | `apps/web/src/lib/attachmentUploadQueue.ts` |
| 2% | 8 | 93 | `apps/web/src/lib/markdownTableRepair.ts` | `packages/client-runtime/src/work-log/commandLabel.ts` |
| 2% | 6 | 183 | `apps/web/src/threadActivation.logic.test.ts` | `apps/mobile/src/state/thread-outbox.test.ts` |
| 2% | 1 | 28 | `apps/api/cloudflare/legacy-proxy.ts` | `apps/web/src/portDiscoveryState.ts` |
| 2% | 4 | 112 | `apps/api/src/app.ts` | `apps/desktop/src/snapShot/HyprlandSnapShot.ts` |
| 2% | 5 | 70 | `apps/desktop/src/backendReadiness.ts` | `apps/mobile/src/lib/attachmentUpload.ts` |
| 2% | 3 | 188 | `apps/server/src/computer/wsComputerHandlers.test.ts` | `packages/client-runtime/src/state/pullRequestDiffHttp.test.ts` |
| 2% | 15 | 561 | `apps/server/src/externalMcp/Layers/ExternalMcpRepository.ts` | `apps/server/src/serverSettings.ts` |
| 2% | 6 | 162 | `apps/server/src/hostConnections/port.ts` | `apps/mobile/src/lib/attachmentUpload.ts` |
| 2% | 4 | 96 | `apps/server/src/persistence/AccountMigrationLineage.ts` | `apps/web/src/components/settings/KeybindingsSettings.logic.ts` |
| 2% | 5 | 87 | `apps/server/src/projectAgent/principal.test.ts` | `apps/mobile/src/state/pending-thread-creation.test.ts` |
| 2% | 20 | 474 | `apps/web/src/components/chat/ComposerLocalDirectoryMenu.tsx` | `apps/web/src/components/ThreadTerminalDrawer.tsx` |
| 2% | 6 | 176 | `apps/web/src/components/chat/handleChatAutomationSend.ts` | `apps/web/src/components/chat/AssistantCitationSource.tsx` |
| 2% | 39 | 585 | `apps/web/src/components/hosts/WorkspaceProjects.tsx` | `apps/web/src/components/Sidebar.tsx` |
| 2% | 12 | 191 | `apps/web/src/components/kanban/useKanbanTaskSubmit.ts` | `apps/web/src/components/GitActionsControl.tsx` |
| 2% | 22 | 163 | `apps/web/src/components/pullRequest/pullRequestList.logic.ts` | `apps/web/src/components/pullRequest/pullRequestList.logic.ts` |
| 2% | 1 | 25 | `apps/web/src/hooks/useDesktopCustomTitleBar.browser.tsx` | `apps/web/src/hooks/useThreadActions.test.ts` |
| 2% | 1 | 51 | `apps/web/src/pinnedMessages.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 2% | 12 | 534 | `apps/web/src/splitViewStore.ts` | `apps/web/src/browserHistoryStore.ts` |
| 2% | 12 | 744 | `apps/web/src/store.test.ts` | `apps/server/src/orchestration/commandInvariants.test.ts` |
| 2% | 2 | 61 | `packages/contracts/src/computerBrowser.ts` | `packages/contracts/src/auth.ts` |
| 2% | 2 | 72 | `packages/shared/src/deviceFrame.test.ts` | `apps/web/src/lib/imageCompression.test.ts` |
| 2% | 7 | 350 | `apps/desktop/scripts/electron-launcher.test.mjs` | `apps/server/src/provider/Layers/EventNdjsonLogger.test.ts` |
| 2% | 3 | 102 | `apps/desktop/src/browserAutomation/waitAndEvaluate.test.ts` | `apps/server/src/git/linkCreatedPullRequest.test.ts` |
| 2% | 8 | 211 | `apps/server/src/composerPrediction/composerPrediction.ts` | `apps/web/src/components/ChatView.logic.ts` |
| 2% | 10 | 281 | `apps/server/src/provider/piOpenCodeCatalog.test.ts` | `apps/web/src/openVsxThemes.test.ts` |
| 2% | 6 | 159 | `apps/server/src/providerUsage/providers/cursor.ts` | `apps/mobile/src/features/sharing/incoming-share-model.ts` |
| 2% | 2 | 60 | `apps/web/src/bootstrap.ts` | `apps/mobile/src/features/diagnostics/SettingsDiagnosticsRouteScreen.tsx` |
| 2% | 12 | 300 | `apps/web/src/components/ChatView.selectors.ts` | `apps/web/src/components/ChatView.logic.ts` |
| 2% | 16 | 147 | `apps/web/src/components/ProviderUsageMenuControl.tsx` | `apps/web/src/components/settings/KeybindingsSettings.tsx` |
| 2% | 15 | 297 | `apps/web/src/components/kanban/KanbanProjectBoardView.tsx` | `apps/web/src/components/GitActionsControl.tsx` |
| 2% | 3 | 120 | `apps/web/src/lib/archiveThreadWorktreeCleanup.test.ts` | `apps/web/src/components/onboarding/WelcomeWizard.test.tsx` |
| 2% | 7 | 143 | `apps/web/src/lib/projectCreateRecovery.test.ts` | `packages/client-runtime/src/state/environmentHttpAuth.test.ts` |
| 2% | 6 | 211 | `apps/web/src/routeRestoreRefreshCoordinator.test.ts` | `apps/desktop/src/window/QuitHold.test.ts` |
| 2% | 7 | 85 | `apps/server/src/trustedOrigins.ts` | `apps/desktop/src/snapShot/snapShot.ts` |
| 2% | 7 | 97 | `apps/web/src/components/chat/MessagesTimeline.find.browser.tsx` | `apps/web/src/components/diffs/StyledDiffCodeView.test.tsx` |
| 2% | 22 | 152 | `apps/web/src/components/chat/environment/EnvironmentEditableChecklistRow.tsx` | `apps/web/src/components/settings/KeybindingsSettings.tsx` |
| 2% | 5 | 60 | `apps/web/src/components/hosts/IosBetaAnnouncement.tsx` | `apps/web/src/components/AgentsPanel.tsx` |
| 2% | 3 | 25 | `apps/web/src/components/tasks/taskDelegationState.ts` | `apps/web/src/browserFaviconStore.ts` |
| 2% | 2 | 34 | `apps/web/src/lib/projectDelete.test.ts` | `apps/desktop/src/app/DesktopAppActivation.test.ts` |
| 2% | 2 | 46 | `apps/web/src/lib/providerDiscovery.capacity.browser.ts` | `apps/web/src/components/files/fileEditorLanguageReadiness.test.ts` |
| 2% | 8 | 268 | `packages/contracts/src/account.ts` | `packages/contracts/src/pullRequest.ts` |
| 2% | 2 | 75 | `apps/desktop/native/appsnap/ExternalTriggerListener.swift` | `apps/mobile/modules/t3-terminal/scripts/build-libghostty-android.sh` |
| 2% | 3 | 63 | `apps/server/src/codexAppServerTransport.ts` | `packages/contracts/src/project.ts` |
| 2% | 5 | 165 | `apps/server/src/remoteSessions/gateway.ts` | `apps/mobile/src/features/sharing/incoming-share-model.ts` |
| 2% | 5 | 116 | `apps/web/src/browserStateStore.test.ts` | `apps/server/src/mcp/toolkits/pullRequests/handlers.test.ts` |
| 2% | 3 | 136 | `apps/web/src/components/chat/RightDockGlassSurface.browser.tsx` | `apps/desktop/src/snapShot/GnomeCaptureSetup.test.ts` |
| 2% | 21 | 356 | `apps/web/src/components/settings/ConversationStorageSettingsPanels.tsx` | `apps/web/src/components/pullRequest/PullRequestDetailPanel.tsx` |
| 2% | 4 | 26 | `apps/web/src/projectEnvironmentStore.ts` | `apps/web/src/browserFaviconStore.ts` |
| 2% | 2 | 71 | `apps/desktop/src/browserVaultIpc.test.ts` | `apps/mobile/src/lib/composerContextClipboard.test.ts` |
| 2% | 12 | 700 | `apps/server/src/agentGateway/computerBrowserTools.ts` | `apps/server/src/provider/Layers/GrokProvider.ts` |
| 2% | 15 | 44 | `apps/server/src/orchestration/commandFingerprint.test.ts` | `apps/server/src/orchestration/Layers/OrchestrationEngine.test.ts` |
| 2% | 1 | 53 | `apps/server/src/projectAgent/hubContextPacket.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 2% | 7 | 153 | `apps/server/src/wsRequestAdmission.ts` | `apps/server/src/mcp/PreviewAutomationBroker.ts` |
| 2% | 4 | 54 | `apps/web/src/dockExplorerBrowseStore.ts` | `apps/web/src/uiStateStore.ts` |
| 2% | 1 | 10 | `packages/shared/src/composerSlashCommands.ts` | `apps/web/src/composer-list-continuation.ts` |
| 2% | 5 | 171 | `scripts/computer-use-fixtures/probe-native-cancellation.mjs` | `apps/mobile/src/lib/composerImages.ts` |
| 2% | 1 | 41 | `scripts/lib/mac-bundle-architecture.ts` | `.github/scripts/relay-state-output.test.cjs` |
| 2% | 5 | 132 | `scripts/pre-push-check.ts` | `apps/web/src/lib/openPullRequestLink.ts` |
| 2% | 2 | 52 | `apps/desktop/scripts/cookie-import-smoke.ts` | `apps/mobile/src/features/settings/SettingsProjectOverviewRouteScreen.tsx` |
| 2% | 6 | 104 | `apps/desktop/src/backendReadiness.test.ts` | `apps/web/src/openVsxThemes.test.ts` |
| 2% | 1 | 56 | `apps/desktop/src/browserWebMcp/guestBridgeSecurity.test.ts` | `apps/web/src/hooks/useCopyToClipboard.test.ts` |
| 2% | 4 | 75 | `apps/server/integration/fixtures/codexLifecycle.cjs` | `.github/scripts/thread-transfer-report.cjs` |
| 2% | 5 | 353 | `apps/server/src/providerUsage/providers/antigravity.test.ts` | `apps/mobile/src/features/agent-awareness/notificationNavigation.test.ts` |
| 2% | 16 | 138 | `apps/web/src/components/DiffLineBlamePopover.tsx` | `apps/web/src/components/settings/ThemeEditorPanel.tsx` |
| 2% | 10 | 138 | `apps/web/src/hooks/useClaudeContextCompaction.ts` | `apps/mobile/src/state/use-thread-outbox-drain.ts` |
| 2% | 6 | 58 | `apps/web/src/hooks/useComposerThreadMentionDrop.ts` | `apps/mobile/src/features/threads/NewTaskContextPickerScreens.tsx` |
| 2% | 5 | 152 | `apps/web/src/splitView.logic.test.ts` | `apps/web/src/promptStashStore.test.ts` |
| 2% | 2 | 154 | `apps/server/src/computer/ComputerControlState.test.ts` | `apps/web/src/components/files/projectFilesQueryState.test.tsx` |
| 2% | 8 | 166 | `apps/server/src/git/gitActionRunner.test.ts` | `packages/client-runtime/src/state/vcsAction.test.ts` |
| 2% | 6 | 248 | `apps/server/src/mcpConnectors/stdio.ts` | `apps/server/src/cli/theme.ts` |
| 2% | 7 | 128 | `apps/server/src/platform/effectProcessRuntime.test.ts` | `apps/desktop/src/backend/DesktopBackendConfiguration.test.ts` |
| 2% | 3 | 58 | `apps/server/src/provider/claudeImportedCopyDates.ts` | `apps/mobile/src/features/review/nativeReviewDiffAdapter.test.ts` |
| 2% | 3 | 166 | `apps/server/src/provider/claudeProjectImport.test.ts` | `scripts/lib/public-config.test.ts` |
| 2% | 10 | 366 | `apps/server/src/remotePairing/management.ts` | `apps/mobile/src/lib/attachmentUpload.ts` |
| 2% | 1 | 54 | `apps/server/src/terminal/terminalModeReplay.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 2% | 7 | 190 | `apps/server/src/wsRpc.auth.test.ts` | `packages/contracts/src/orchestration.test.ts` |
| 2% | 13 | 171 | `apps/web/src/components/kanban/KanbanView.tsx` | `apps/web/src/components/RightPanelTabs.tsx` |
| 2% | 1 | 39 | `apps/web/src/lib/automationFailurePolicy.ts` | `apps/web/src/components/chat/modelPickerSearch.ts` |
| 2% | 4 | 99 | `apps/web/src/lib/starredModels.ts` | `apps/server/src/usage/usageTranscripts.ts` |
| 2% | 1 | 16 | `packages/contracts/src/browserAutomationToolCatalogue.ts` | `packages/contracts/src/browserImport.ts` |
| 2% | 6 | 231 | `apps/server/src/agentGateway/mcpInjection.test.ts` | `apps/desktop/src/preview/BrowserImport/Sources.test.ts` |
| 2% | 14 | 255 | `apps/server/src/provider/providerRuntimeReconciliation.ts` | `apps/server/src/orchestration/projector.ts` |
| 2% | 5 | 96 | `apps/web/src/components/WorkspaceFilePreview.selection.browser.tsx` | `apps/desktop/src/preview/PreviewKeyboard.ts` |
| 2% | 7 | 468 | `apps/web/src/lib/gitReactQuery.test.ts` | `apps/web/src/browser/browserViewportActions.test.ts` |
| 2% | 3 | 194 | `apps/api/src/accountDeletion.ts` | `infra/relay/src/agentActivity/AgentActivityRows.ts` |
| 2% | 5 | 137 | `apps/server/src/provider/piSdkImports.ts` | `apps/mobile/src/state/thread-outbox-manager.ts` |
| 2% | 5 | 77 | `apps/server/src/providerUsage/providers/cursorSafeStorage.ts` | `apps/desktop/scripts/electron-launcher.mjs` |
| 2% | 7 | 247 | `apps/server/src/studioOutputs.test.ts` | `apps/server/test/ActivityPayloadProjection.test.ts` |
| 2% | 5 | 219 | `apps/web/src/components/chat/environment/EnvironmentUsageSection.browser.tsx` | `apps/web/src/providerInstances.test.ts` |
| 2% | 5 | 53 | `apps/web/src/components/chat/project/projectPanel.logic.ts` | `apps/server/src/project/ProjectCloneTracker.ts` |
| 2% | 19 | 257 | `apps/web/src/components/settings/accounts/ProviderAccountEditor.tsx` | `apps/web/src/components/GitActionsControl.tsx` |
| 2% | 8 | 154 | `apps/web/src/composerDraftStore.ts` | `apps/web/src/components/ChatView.logic.ts` |
| 2% | 6 | 290 | `packages/contracts/src/providerDiscovery.ts` | `packages/contracts/src/usage.ts` |
| 2% | 1 | 55 | `apps/api/src/rateLimit.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 2% | 1 | 38 | `apps/desktop/src/backendReliability.test.ts` | `apps/web/src/components/chat/SkillInlineText.tsx` |
| 2% | 2 | 31 | `apps/marketing/src/lib/sponsors.ts` | `apps/web/src/components/settings/ThemePreviewCircles.tsx` |
| 2% | 3 | 130 | `apps/server/src/computer/ComputerManager.spaces.test.ts` | `apps/web/src/components/onboarding/WelcomeWizard.test.tsx` |
| 2% | 9 | 143 | `apps/server/src/computer/computerAuditHistory.ts` | `apps/server/src/project/AgentSessionScanner.ts` |
| 2% | 25 | 730 | `apps/web/src/components/chat/SplitChatSurface.tsx` | `apps/web/src/routes/_chat.pull-requests.tsx` |
| 2% | 18 | 90 | `apps/web/src/components/pullRequest/PullRequestCodeTab.tsx` | `apps/web/src/components/pullRequest/PullRequestCodeTab.tsx` |
| 2% | 7 | 69 | `apps/web/src/hooks/useDesktopTopBarGutter.ts` | `apps/web/src/components/BranchToolbarBranchSelector.tsx` |
| 2% | 4 | 137 | `apps/web/src/lib/accountReactQuery.test.ts` | `apps/desktop/src/snapShot/snapShot.test.ts` |
| 2% | 11 | 303 | `apps/web/src/lib/pullRequestCache.ts` | `apps/web/src/rightPanelStore.ts` |
| 2% | 1 | 55 | `apps/web/src/lib/spaceGrouping.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 2% | 1 | 55 | `apps/web/src/lib/usagePace.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 2% | 9 | 135 | `apps/web/src/routes/_chat.$threadId.tsx` | `apps/web/src/components/ThreadRouteView.tsx` |
| 2% | 2 | 32 | `packages/contracts/src/browserAutomationCssSelector.test.ts` | `apps/web/src/components/settings/SnapShotSetupDialog.logic.test.ts` |
| 2% | 1 | 55 | `packages/shared/src/byteAccumulator.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 2% | 7 | 381 | `scripts/device-helper-smoke.ts` | `apps/mobile/src/lib/composerImages.ts` |
| 2% | 3 | 138 | `apps/desktop/src/computerShield.test.ts` | `apps/desktop/src/app/DesktopAppActivation.test.ts` |
| 2% | 10 | 135 | `apps/server/src/desktopUpdateRpc.ts` | `apps/server/src/provider/Layers/CursorAdapter.ts` |
| 2% | 4 | 28 | `apps/server/src/git/worktreeSetupStream.ts` | `apps/server/src/storageCleanup.ts` |
| 2% | 11 | 265 | `apps/server/src/provider/claudeWorkflowScript.ts` | `packages/client-runtime/src/state/subagentRuntime.ts` |
| 2% | 10 | 507 | `apps/server/src/workspaceEntries.test.ts` | `apps/server/src/workspace/WorkspaceEntries.test.ts` |
| 2% | 17 | 77 | `apps/web/src/components/ThreadTerminalDrawer.tsx` | `apps/web/src/components/ThreadTerminalDrawer.tsx` |
| 2% | 4 | 143 | `apps/web/src/components/account/SignInDialog.browser.tsx` | `apps/web/src/components/PullRequestThreadDialog.tsx` |
| 2% | 8 | 195 | `apps/web/src/hooks/useThreadRecap.ts` | `apps/mobile/src/state/queries.ts` |
| 2% | 24 | 1794 | `apps/web/src/lib/toolCallLabel.ts` | `apps/web/src/session-logic.ts` |
| 2% | 3 | 68 | `scripts/notarize-mac-app.ts` | `apps/desktop/src/snapShot/CaptureShortcutConfig.ts` |
| 2% | 14 | 460 | `apps/desktop/src/resumableUpdateDownload.ts` | `apps/web/src/components/ChatView.logic.ts` |
| 2% | 11 | 280 | `apps/server/src/hostConnections/dialer.ts` | `apps/web/src/openVsxThemes.ts` |
| 2% | 21 | 566 | `apps/server/src/orchestration/projectImportRoute.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.test.ts` |
| 2% | 6 | 135 | `apps/web/e2e/fixtures/mcpBrowserHarness.ts` | `apps/server/src/mcp/toolkits/pullRequests/handlers.test.ts` |
| 2% | 11 | 739 | `apps/web/src/components/chat/ComposerModelPicker.browser.tsx` | `apps/mobile/src/widgets/subscriptionUsageSnapshot.test.ts` |
| 2% | 11 | 213 | `apps/web/src/components/chat/ComposerModelPicker.logic.ts` | `apps/web/src/keybindings.ts` |
| 2% | 29 | 945 | `apps/web/src/components/chat/SingleChatSurface.tsx` | `apps/web/src/components/CommandPalette.tsx` |
| 2% | 23 | 395 | `benchmarks/desktop-perf/seed-realistic-threads.ts` | `apps/server/src/orchestration/Layers/OrchestrationEngine.test.ts` |
| 2% | 6 | 98 | `scripts/lib/release-job-wait.ts` | `apps/mobile/src/lib/composerImages.ts` |
| 2% | 4 | 130 | `apps/server/src/externalMcp/waitForTask.test.ts` | `apps/server/src/environment/ServerEnvironmentMachine.test.ts` |
| 2% | 9 | 45 | `apps/server/src/orchestration/stalePendingInteractions.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.ts` |
| 2% | 5 | 105 | `apps/server/src/serverShutdown.test.ts` | `apps/server/src/device/DeviceService.test.ts` |
| 2% | 15 | 191 | `apps/web/src/components/chat/GeneratedMarkdownImage.tsx` | `apps/web/src/components/ThreadTerminalDrawer.tsx` |
| 2% | 5 | 48 | `apps/web/src/components/chat/userMessageTerminalContexts.ts` | `packages/shared/src/composerContextLegacy.ts` |
| 2% | 7 | 273 | `apps/web/src/lib/pullRequestMutationCoordinator.ts` | `packages/client-runtime/src/relay/managedRelayState.ts` |
| 2% | 2 | 43 | `packages/shared/src/processSpawnOutcome.test.ts` | `apps/desktop/src/app/DesktopAppActivation.test.ts` |
| 2% | 14 | 639 | `apps/desktop/src/backendShutdown.test.ts` | `apps/desktop/src/snapShot/snapShot.test.ts` |
| 2% | 1 | 13 | `apps/marketing/src/app/sitemap-index.xml/route.ts` | `apps/web/src/components/pullRequest/pullRequestMarkdown.logic.test.ts` |
| 2% | 43 | 1387 | `apps/server/src/agentGateway/Layers/AgentGateway.ts` | `apps/server/src/orchestration/Layers/ProviderCommandReactor.ts` |
| 2% | 5 | 54 | `apps/server/src/device/bootOwnership.ts` | `apps/server/scripts/migrate-dev-db.ts` |
| 2% | 1 | 26 | `apps/server/src/provider/claudeRequestUsage.ts` | `apps/server/src/persistence/Services/ProjectionThreadMessages.ts` |
| 2% | 1 | 19 | `apps/server/src/provider/piTurnFailure.ts` | `packages/contracts/src/browserImport.ts` |
| 2% | 15 | 211 | `apps/web/src/components/DiffPanelFileList.tsx` | `apps/web/src/components/RightPanelTabs.tsx` |
| 2% | 10 | 189 | `apps/web/src/components/chat/MessagesTimeline.scroll.browser.tsx` | `apps/web/src/components/chat/MessagesTimeline.test.tsx` |
| 2% | 2 | 65 | `apps/web/src/components/chat/SplitChatSurface.browserOpenRequest.test.ts` | `apps/server/src/provider/providerInstallation.test.ts` |
| 2% | 10 | 475 | `apps/web/src/components/chat/useChatQueuedTurns.ts` | `apps/web/src/components/BranchToolbarBranchSelector.tsx` |
| 2% | 15 | 292 | `apps/web/src/components/chat/useComposerVoiceController.ts` | `apps/web/src/components/ThreadTerminalDrawer.tsx` |
| 2% | 10 | 118 | `apps/web/src/components/chat/useSidechatShortcut.ts` | `apps/web/src/components/ComposerPromptEditorTiptap.tsx` |
| 2% | 23 | 224 | `apps/web/src/components/settings/KeyboardShortcutsSettingsPanel.tsx` | `apps/web/src/components/settings/KeybindingsSettings.tsx` |
| 2% | 12 | 226 | `packages/shared/src/KeyedDrainableWorker.ts` | `apps/desktop/src/backend/DesktopBackendManager.ts` |
| 2% | 2 | 53 | `packages/shared/src/pinnedMessages.test.ts` | `apps/mobile/src/features/keyboard/HardwareKeyboardCommandProvider.tsx` |
| 2% | 2 | 37 | `apps/desktop/src/cuaHostSocket.ts` | `apps/mobile/src/lib/composerContextClipboard.ts` |
| 2% | 5 | 107 | `apps/e2e/src/harness/websocket.ts` | `apps/server/src/cli/app.ts` |
| 2% | 7 | 334 | `apps/web/src/components/chat/ComposerExtrasPanel.browser.tsx` | `apps/web/src/components/ProviderUpdateEnvironmentRows.test.tsx` |
| 2% | 15 | 287 | `apps/web/src/components/settings/ProviderUsageSettingsPanel.tsx` | `apps/web/src/components/settings/ProviderSettingsPanel.tsx` |
| 2% | 2 | 24 | `apps/api/src/identity/hostRecords.ts` | `apps/desktop/scripts/wait-for-resources.mjs` |
| 2% | 10 | 198 | `apps/marketing/src/app/api/feedback/route.ts` | `apps/web/src/openVsxThemes.ts` |
| 2% | 4 | 118 | `apps/marketing/src/lib/llmText.ts` | `packages/effect-codex-app-server/scripts/generate.ts` |
| 2% | 1 | 58 | `apps/server/src/accountRpcErrors.test.ts` | `apps/mobile/plugins/withIosSceneLifecycle.test.mjs` |
| 2% | 4 | 82 | `apps/server/src/cloudflare/connector.ts` | `apps/server/src/cli/app.test.ts` |
| 2% | 1 | 23 | `apps/server/src/computer/computerSpaceDesignation.ts` | `apps/web/src/providerModels.ts` |
| 2% | 16 | 153 | `apps/server/src/studioGeneratedImages.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.ts` |
| 2% | 12 | 140 | `apps/web/src/components/chat/ComposerQueuedHeader.tsx` | `apps/web/src/components/settings/KeybindingsSettings.tsx` |
| 2% | 1 | 80 | `apps/web/src/components/hosts/ProjectCatalog.browser.tsx` | `oxlint-plugin-t3code/index.ts` |
| 2% | 1 | 44 | `benchmarks/desktop-perf/report.mjs` | `scripts/apply-web-brand-assets.ts` |
| 2% | 3 | 53 | `apps/server/src/checkpointing/historyIntegration.test.ts` | `apps/server/src/provider/opencodeRuntime.cliParsers.test.ts` |
| 2% | 13 | 275 | `apps/server/src/effectServer.ts` | `apps/server/src/serverRuntimeStartup.ts` |
| 2% | 5 | 120 | `apps/server/src/provider/continuationIdentity.test.ts` | `apps/server/src/provider/Layers/EventNdjsonLogger.test.ts` |
| 2% | 2 | 97 | `apps/web/src/components/ThreadHoverCardContent.browser.tsx` | `apps/server/src/provider/Layers/CodexProvider.test.ts` |
| 2% | 24 | 855 | `apps/web/src/components/chat/SubagentRunCard.logic.test.ts` | `apps/web/src/session-logic.test.ts` |
| 2% | 18 | 243 | `apps/web/src/components/pullRequest/GitHubItemPageLayout.tsx` | `apps/web/src/components/settings/KeybindingsSettings.tsx` |
| 2% | 5 | 149 | `apps/web/src/lib/startContainerChat.test.ts` | `apps/mobile/src/state/use-thread-outbox-drain.test.ts` |
| 2% | 3 | 82 | `packages/shared/src/keybindingRules.test.ts` | `packages/contracts/src/keybindings.test.ts` |
| 2% | 2 | 76 | `scripts/computer-use-fixtures/NativeFixture.swift` | `apps/mobile/modules/t3-native-controls/ios/T3NativeVideoPresentation.swift` |
| 2% | 2 | 52 | `apps/server/src/accountCredentialLock.test.ts` | `apps/web/src/components/settings/SourceControlWritingSettings.test.tsx` |
| 2% | 6 | 43 | `apps/server/src/hostConnections/accountObserver.ts` | `apps/web/src/components/cloud/CloudEnvironmentConnectList.tsx` |
| 2% | 20 | 956 | `apps/server/src/persistence/Layers/ProviderRuntimeEvents.test.ts` | `apps/server/src/orchestration/Layers/ProjectionSnapshotQuery.test.ts` |
| 2% | 5 | 119 | `apps/web/src/components/chat/environment/EnvironmentSubagentsSection.logic.ts` | `packages/shared/src/model.ts` |
| 2% | 5 | 167 | `apps/web/src/components/inbox/inboxStories.test.ts` | `apps/mobile/src/widgets/subscriptionUsageSnapshot.test.ts` |
| 2% | 5 | 298 | `apps/web/src/hooks/useAccount.test.tsx` | `apps/mobile/src/native/voiceTranscription.ios.test.ts` |
| 2% | 2 | 99 | `packages/contracts/src/browserAutomationTargets.ts` | `packages/client-runtime/src/codexArtifactTemplates.ts` |
| 2% | 3 | 50 | `packages/shared/src/kanban.test.ts` | `apps/desktop/src/updates/remoteUpdateFlow.test.ts` |
| 2% | 5 | 210 | `packages/shared/src/outboundHttp.test.ts` | `scripts/install.test.ts` |
| 2% | 11 | 417 | `apps/server/src/accountLiveActivityReporter.test.ts` | `packages/client-runtime/src/device/stream.test.ts` |
| 2% | 5 | 79 | `apps/server/src/codexHomePaths.ts` | `packages/shared/src/shell.ts` |
| 2% | 7 | 201 | `apps/server/src/device/IosSimulatorBackend.capabilities.test.ts` | `apps/server/src/processRunner.ts` |
| 2% | 6 | 109 | `apps/server/src/editor/debugAdapterInstaller.ts` | `apps/web/src/openVsxThemes.ts` |
| 2% | 5 | 256 | `packages/shared/src/loginShellEnvironment.test.ts` | `apps/mobile/src/state/use-thread-outbox-drain.test.ts` |
| 2% | 7 | 99 | `apps/marketing/src/components/AskAISection.tsx` | `apps/web/src/components/preview/ThreadPreviewMiniPlayer.tsx` |
| 2% | 12 | 295 | `apps/server/src/providerUsageSnapshot.test.ts` | `apps/server/src/usage/usageTranscripts.test.ts` |
| 2% | 127 | 2967 | `apps/server/src/wsRpc.ts` | `apps/server/src/ws.ts` |
| 2% | 47 | 438 | `apps/web/src/components/SidebarGroupsSurface.tsx` | `apps/web/src/components/LegacySidebar.tsx` |
| 2% | 18 | 260 | `apps/web/src/components/chat/ComposerExtrasPanel.tsx` | `apps/web/src/components/GitActionsControl.tsx` |
| 2% | 17 | 153 | `apps/web/src/components/pullRequest/pullRequestDetail.logic.test.ts` | `apps/web/src/components/pullRequest/pullRequestDetail.logic.test.ts` |
| 2% | 5 | 280 | `apps/web/src/hooks/useThreadActivationController.test.ts` | `apps/web/src/promptStashStore.test.ts` |
| 2% | 9 | 172 | `apps/web/src/hooks/useThreadHandoff.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.ts` |
| 2% | 8 | 157 | `apps/web/src/lib/accountLogic.test.ts` | `apps/web/src/components/pullRequest/pullRequestDetail.logic.test.ts` |
| 2% | 12 | 115 | `apps/web/src/lib/threadRename.ts` | `apps/web/src/components/ChatView.logic.ts` |
| 2% | 5 | 376 | `apps/desktop/src/desktopMigrationRecovery.test.ts` | `apps/mobile/src/lib/attachmentUpload.test.ts` |
| 2% | 11 | 147 | `apps/profiles/src/components/ShareProfileButton.tsx` | `apps/web/src/components/settings/ThemeSettings.tsx` |
| 2% | 5 | 285 | `apps/server/src/computer/scrollCalibration.test.ts` | `apps/desktop/src/preview/FaviconCapture.test.ts` |
| 2% | 22 | 985 | `apps/web/src/components/SidebarActivityView.logic.test.ts` | `apps/server/src/orchestration/ThreadSettlementPolicy.test.ts` |
| 2% | 5 | 120 | `apps/web/src/components/chat/useChatAutomationSetup.test.ts` | `apps/web/src/components/cloud/CloudEnvironmentConnectList.test.tsx` |
| 2% | 8 | 176 | `apps/web/src/featureTour/AppTourPreview.tsx` | `apps/web/src/components/GitActionsControl.tsx` |
| 2% | 10 | 226 | `apps/web/src/lib/chatProjects.ts` | `apps/mobile/src/state/use-thread-outbox-drain.ts` |
| 2% | 5 | 154 | `scripts/lib/desktop-platform-build-config.ts` | `apps/server/src/provider/Layers/CursorProvider.ts` |
| 2% | 10 | 2700 | `apps/api/src/routes/v1.test.ts` | `apps/mobile/src/state/thread-outbox.test.ts` |
| 2% | 7 | 195 | `apps/server/src/localImageFiles.ts` | `apps/server/src/assets/AssetAccess.ts` |
| 2% | 16 | 466 | `apps/server/src/provider/OpenCodeDiscovery.ts` | `apps/server/src/provider/Layers/CursorProvider.ts` |
| 2% | 22 | 296 | `apps/server/src/provider/providerRuntimeEventPump.ts` | `apps/server/src/orchestration/Layers/ProviderCommandReactor.ts` |
| 2% | 6 | 97 | `apps/server/src/providerChildEnvironment.ts` | `apps/server/src/provider/antigravityAuthSupport.ts` |
| 2% | 5 | 190 | `apps/server/src/providerUsage/providers/grok.test.ts` | `apps/mobile/src/lib/attachmentDownload.test.ts` |
| 2% | 8 | 193 | `apps/web/src/components/chat/ComposerSubagentStrip.logic.ts` | `packages/client-runtime/src/work-log/presentation.ts` |
| 2% | 7 | 475 | `apps/server/src/editor/typescriptService.ts` | `apps/server/src/provider/Layers/CursorProvider.ts` |
| 2% | 14 | 98 | `apps/server/src/orchestration/Layers/ThreadSnoozeReactor.ts` | `apps/server/src/orchestration/Layers/ProviderCommandReactor.ts` |
| 2% | 16 | 335 | `apps/web/src/components/chat/project/ProjectPanel.browser.tsx` | `apps/server/src/relay/AgentAwarenessRelay.test.ts` |
| 2% | 22 | 521 | `apps/web/src/components/githubInbox/GitHubInbox.tsx` | `apps/web/src/components/pullRequest/PullRequestDetailPanel.tsx` |
| 2% | 12 | 171 | `apps/web/src/components/kanban/KanbanColumn.tsx` | `apps/web/src/components/RightPanelTabs.tsx` |
| 2% | 31 | 1892 | `apps/web/src/storeNormalization.ts` | `apps/web/src/session-logic.ts` |
| 2% | 5 | 314 | `benchmarks/desktop-perf/run.mjs` | `apps/mobile/src/features/sharing/incoming-share-model.ts` |
| 2% | 5 | 93 | `apps/server/src/automation/Services/AutomationService.ts` | `apps/server/integration/TestProviderAdapter.integration.ts` |
| 2% | 16 | 552 | `apps/web/src/components/chat/project/useProjectAgent.ts` | `apps/mobile/src/state/use-thread-outbox-drain.ts` |
| 2% | 15 | 381 | `apps/web/src/components/kanban/KanbanProjectBoardView.browser.tsx` | `apps/mobile/src/features/threads/threadListV2.test.ts` |
| 2% | 18 | 101 | `apps/web/src/components/pullRequest/PullRequestStackPopover.tsx` | `apps/web/src/components/GitActionsControl.tsx` |
| 2% | 20 | 298 | `apps/web/src/components/pullRequest/pullRequestList.logic.test.ts` | `apps/web/src/components/pullRequest/pullRequestDetail.logic.test.ts` |
| 2% | 5 | 173 | `apps/web/src/computerPreviewStore.test.ts` | `apps/web/src/uiStateStore.test.ts` |
| 2% | 7 | 175 | `packages/shared/src/browserAutomationErrors.test.ts` | `apps/server/src/vcs/VcsProcess.test.ts` |
| 2% | 8 | 154 | `packages/shared/src/groupThreadState.ts` | `apps/web/src/session-logic.ts` |
| 2% | 9 | 207 | `apps/desktop/src/computerShield.ts` | `apps/desktop/src/snapShot/PortalCaptureShortcut.ts` |
| 2% | 5 | 58 | `apps/desktop/src/macAppIcon.ts` | `apps/web/src/browserFaviconStore.ts` |
| 2% | 12 | 273 | `apps/e2e/src/harness/host.ts` | `apps/server/src/auth/EnvironmentAuth.ts` |
| 2% | 34 | 120 | `apps/web/src/components/SidebarCustomizeList.tsx` | `apps/web/src/components/LegacySidebar.tsx` |
| 2% | 7 | 296 | `packages/contracts/src/browserAutomationErrors.test.ts` | `apps/web/src/components/ChatView.logic.test.ts` |
| 2% | 10 | 253 | `packages/shared/src/cuaDriverProtocol.ts` | `apps/web/src/components/ChatView.logic.ts` |
| 2% | 8 | 231 | `apps/server/src/remoteTransport/tunnel.ts` | `apps/desktop/src/snapShot/NiriSnapShot.ts` |
| 2% | 12 | 309 | `apps/web/src/components/chat/MessagesTimeline.inlineVisual.browser.tsx` | `apps/web/src/components/chat/MessagesTimeline.test.tsx` |
| 2% | 7 | 156 | `apps/web/src/lib/chatReferences.ts` | `apps/web/src/components/ChatView.logic.ts` |
| 2% | 10 | 160 | `packages/shared/src/projectAgent.ts` | `apps/web/src/components/pullRequest/pullRequestDetail.logic.ts` |
| 2% | 13 | 1075 | `apps/desktop/src/betaDiagnostics.test.ts` | `apps/desktop/src/updates/updateMachine.test.ts` |
| 2% | 42 | 1008 | `apps/server/src/agentGateway/creationCoordinator.ts` | `apps/server/src/ws.ts` |
| 2% | 7 | 84 | `apps/server/src/agentGateway/threadDiagnosticSummary.ts` | `apps/server/src/persistence/Layers/OrchestrationEventStore.ts` |
| 2% | 9 | 234 | `apps/server/src/orchestration/dispatchCommandNormalization.ts` | `apps/server/src/orchestration/decider.settled.test.ts` |
| 2% | 5 | 189 | `apps/web/src/components/chat/EditedFileRow.browser.tsx` | `apps/mobile/src/state/use-thread-outbox-drain.test.ts` |
| 2% | 12 | 93 | `apps/web/src/components/chat/EditedFileRow.tsx` | `apps/web/src/components/RightPanelTabs.tsx` |
| 2% | 7 | 106 | `apps/web/src/lib/liveActivityPresentation.ts` | `packages/client-runtime/src/work-log/presentation.ts` |
| 2% | 5 | 52 | `apps/web/src/lib/localImageUrls.ts` | `apps/mobile/src/features/layout/AdaptiveWorkspaceLayout.tsx` |
| 2% | 8 | 164 | `apps/web/src/lib/newThreadDefaultsSync.ts` | `packages/client-runtime/src/device/duoViewer.ts` |
| 2% | 9 | 146 | `apps/web/src/lib/spaces.ts` | `apps/mobile/src/lib/composerImages.ts` |
| 2% | 6 | 340 | `scripts/lib/release-artifact-provenance.ts` | `apps/server/src/process/externalLauncher.ts` |
| 2% | 8 | 267 | `apps/web/src/components/tasks/useTaskDelegation.browser.tsx` | `apps/server/src/relay/AgentAwarenessRelay.test.ts` |
| 2% | 19 | 156 | `apps/web/src/orchestrationEventCoalescing.test.ts` | `apps/server/src/orchestration/Layers/OrchestrationEngine.test.ts` |
| 2% | 59 | 2805 | `apps/server/src/automation/Layers/AutomationService.ts` | `apps/server/src/orchestration/Layers/ProviderCommandReactor.ts` |
| 2% | 81 | 163 | `apps/web/src/components/chat/useChatTimelineMessages.ts` | `apps/web/src/components/ChatView.tsx` |
| 2% | 8 | 383 | `apps/web/src/lib/serverReactQuery.ts` | `apps/server/src/project/WorktreeSetupTracker.ts` |
| 2% | 6 | 318 | `apps/desktop/src/browserAutomation/betterwrightRuntime.test.ts` | `apps/mobile/src/lib/attachmentDownload.test.ts` |
| 2% | 6 | 382 | `apps/desktop/src/browserUsePipeServer.test.ts` | `apps/desktop/src/app/DesktopAppActivation.test.ts` |
| 2% | 13 | 226 | `apps/desktop/src/electronUpdaterSecurity.ts` | `packages/shared/src/shell.ts` |
| 2% | 7 | 97 | `apps/server/src/diagnostics/ownerThreadDiagnostics.test.ts` | `apps/server/src/orchestration/ThreadPullRequestReactor.test.ts` |
| 2% | 5 | 138 | `apps/server/src/provider/claudeEnvironment.ts` | `apps/server/src/process/externalLauncher.ts` |
| 2% | 9 | 199 | `apps/web/src/components/chat/project/groupOverview.logic.ts` | `apps/web/src/components/ThreadStatusIndicators.tsx` |
| 2% | 74 | 418 | `apps/web/src/components/chat/useChatComposerDraft.ts` | `apps/web/src/components/chat/ChatComposer.tsx` |
| 2% | 11 | 132 | `apps/web/src/hooks/useExecutionImageSource.ts` | `apps/desktop/src/preview/FaviconCapture.ts` |
| 2% | 9 | 409 | `apps/web/src/lib/providerDiscoveryReactQuery.test.ts` | `apps/web/src/terminal/ghostty/surface.test.ts` |
| 2% | 7 | 266 | `apps/desktop/src/computerFrameTap.ts` | `apps/mobile/src/lib/composerImages.ts` |
| 2% | 5 | 261 | `apps/server/src/provider/acp/CursorAcpCommand.test.ts` | `apps/server/src/bootstrap.test.ts` |
| 2% | 16 | 386 | `apps/web/src/components/SidebarSearchPalette.logic.test.ts` | `packages/contracts/src/orchestration.test.ts` |
| 2% | 8 | 78 | `apps/web/src/components/chat/queuedComposerPreview.ts` | `apps/web/src/components/ChatView.logic.ts` |
| 2% | 14 | 206 | `apps/web/src/storeTimelineSequence.test.ts` | `apps/web/src/session-logic.test.ts` |
| 2% | 5 | 260 | `apps/desktop/src/browserAutomation/browserCookieImport.test.ts` | `apps/web/src/lib/imageCompression.test.ts` |
| 2% | 20 | 297 | `apps/server/src/orchestration/projectImportRecovery.test.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.test.ts` |
| 2% | 5 | 89 | `apps/desktop/src/browserAutomation/electronCookieSession.test.ts` | `apps/desktop/src/window/QuitHold.test.ts` |
| 2% | 27 | 701 | `apps/web/src/components/chat/MessagesTimeline.toolGroupCollapse.browser.tsx` | `apps/web/src/components/chat/MessagesTimeline.test.tsx` |
| 2% | 12 | 237 | `apps/web/src/hooks/useComposerDropzone.ts` | `apps/web/src/components/CommandPalette.tsx` |
| 2% | 7 | 207 | `packages/shared/src/threadPullRequestLinks.ts` | `packages/shared/src/git.ts` |
| 2% | 62 | 5007 | `apps/server/src/agentGateway/Layers/AgentGateway.test.ts` | `apps/server/src/relay/AgentAwarenessRelay.test.ts` |
| 2% | 13 | 354 | `apps/server/src/externalMcp/httpRoute.test.ts` | `infra/relay/src/http/Api.test.ts` |
| 2% | 42 | 155 | `apps/server/src/git/gitStatusParsing.ts` | `apps/server/src/vcs/GitVcsDriverCore.ts` |
| 2% | 12 | 142 | `apps/web/src/components/chat/environment/EnvironmentProjectInstructionsSection.tsx` | `apps/web/src/components/RightPanelTabs.tsx` |
| 2% | 5 | 373 | `apps/server/src/providerUsage/providers/grok.ts` | `apps/server/src/assets/AttachmentUpload.ts` |
| 2% | 5 | 100 | `apps/web/src/components/chat/WorkspaceFileEditorPane.tsx` | `apps/web/src/components/usage/UsagePriceOverrides.tsx` |
| 2% | 36 | 367 | `apps/web/src/components/chat/chatHeaderControls.tsx` | `apps/web/src/components/pullRequest/PullRequestDetailPanel.tsx` |
| 2% | 12 | 89 | `apps/web/src/components/chat/useChatPendingInteractions.browser.tsx` | `apps/server/src/relay/AgentAwarenessRelay.test.ts` |
| 2% | 5 | 61 | `apps/web/src/lib/groupWorkerRouting.ts` | `apps/server/src/relay/AgentAwarenessRelay.ts` |
| 2% | 11 | 256 | `apps/web/src/lib/providerAvailability.ts` | `apps/web/src/components/ProviderUpdateLaunchNotification.logic.ts` |
| 2% | 6 | 231 | `benchmarks/desktop-perf/lib/builds.mjs` | `apps/desktop/src/snapShot/KdeSnapShot.ts` |
| 2% | 6 | 122 | `apps/desktop/src/backendShutdown.posix.integration.test.ts` | `apps/server/src/cli/theme.ts` |
| 2% | 17 | 723 | `apps/server/src/orchestration/projectImportRoute.test.ts` | `apps/server/src/project/AgentSessionImporter.test.ts` |
| 2% | 12 | 1221 | `apps/server/src/persistence/MigrationBackup.test.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.test.ts` |
| 2% | 5 | 147 | `apps/server/src/provider/acp/AcpTurnIdleWatchdog.test.ts` | `apps/mobile/src/connection/platform.ts` |
| 2% | 7 | 309 | `apps/web/src/components/SidebarHeaderNavigationControls.browser.tsx` | `apps/mobile/modules/t3-markdown-text/src/NativeMarkdownSelectableText.tsx` |
| 2% | 8 | 108 | `apps/web/src/hooks/useOpenThreadTabs.ts` | `apps/web/src/components/preview/PreviewAutomationHosts.tsx` |
| 2% | 7 | 103 | `apps/web/src/components/chat/SidechatDockPane.tsx` | `apps/mobile/src/state/use-thread-outbox-drain.ts` |
| 2% | 5 | 41 | `apps/web/src/hooks/useExecutionImageSource.test.ts` | `apps/desktop/src/preview/FaviconCapture.test.ts` |
| 2% | 14 | 983 | `apps/web/src/hooks/useSidebarThreadActions.test.ts` | `apps/server/src/mcp/toolkits/pullRequests/handlers.test.ts` |
| 2% | 9 | 426 | `apps/web/src/splitViewStore.test.ts` | `apps/web/src/uiStateStore.test.ts` |
| 2% | 5 | 110 | `apps/server/src/agentGateway/Services/AgentGatewayOperationRepository.ts` | `packages/client-runtime/src/operations/projects.ts` |
| 2% | 16 | 506 | `apps/server/src/git/Layers/ProviderTextGeneration.test.ts` | `apps/server/src/provider/Layers/ProviderRegistry.test.ts` |
| 2% | 5 | 124 | `apps/server/src/privatePathPermissions.ts` | `apps/server/src/serviceLauncher.ts` |
| 2% | 11 | 482 | `apps/web/src/components/chat/WorkflowRunCard.logic.test.ts` | `packages/client-runtime/src/state/subagentRuntime.test.ts` |
| 2% | 19 | 260 | `apps/web/src/components/chat/toolCallGroup.logic.ts` | `packages/client-runtime/src/work-log/presentation.ts` |
| 2% | 10 | 185 | `apps/web/src/lib/composerTurnSettlement.ts` | `apps/mobile/src/lib/composerImages.ts` |
| 2% | 9 | 135 | `apps/server/scripts/orchestration-thread-replay-benchmark.ts` | `apps/server/src/orchestration/ThreadPullRequestReactor.test.ts` |
| 2% | 25 | 585 | `apps/web/src/components/chat/WorkflowRunCard.logic.ts` | `apps/mobile/src/lib/threadActivity.ts` |
| 2% | 7 | 144 | `apps/web/src/components/settings/externalMcpSetup.ts` | `apps/server/src/provider/Layers/CursorProvider.ts` |
| 2% | 13 | 543 | `apps/web/src/lib/providerModelPrefetch.test.ts` | `apps/web/src/components/pullRequest/pullRequestDetail.logic.test.ts` |
| 2% | 23 | 1027 | `apps/desktop/src/browserAnnotations/guestPreload.ts` | `apps/desktop/src/preview/PickPreload.ts` |
| 2% | 6 | 83 | `apps/web/src/components/ProviderUsageLimitRows.tsx` | `apps/web/src/components/settings/DiagnosticsSettings.tsx` |
| 2% | 29 | 1400 | `apps/web/src/components/SidebarActivityView.browser.tsx` | `apps/server/src/orchestration/ThreadSettlementReactor.test.ts` |
| 2% | 9 | 587 | `apps/web/src/components/chat/RightDock.browser.tsx` | `apps/web/src/components/settings/ProviderSetupSection.tsx` |
| 2% | 8 | 101 | `apps/web/src/components/chat/useComposerReferences.ts` | `apps/mobile/src/features/threads/ThreadDetailScreen.tsx` |
| 2% | 8 | 280 | `apps/web/src/components/settings/DesktopSettingsPanels.browser.tsx` | `apps/web/src/components/settings/ProviderSettingsPanel.environment.test.tsx` |
| 2% | 7 | 103 | `apps/server/src/agentGateway/connectorTools.test.ts` | `apps/server/src/device/DeviceService.test.ts` |
| 2% | 8 | 240 | `apps/server/src/hostSecrets/coordinator.ts` | `apps/mobile/src/state/use-thread-outbox-drain.ts` |
| 2% | 11 | 329 | `apps/server/src/platform/supervisedProcessTeardown.ts` | `packages/client-runtime/src/device/duoViewer.ts` |
| 2% | 30 | 973 | `apps/web/src/routes/_chat.automations.$automationId.tsx` | `apps/web/src/components/LegacySidebar.tsx` |
| 2% | 10 | 685 | `apps/analytics/worker.test.ts` | `apps/web/src/components/pullRequest/pullRequestDetail.logic.test.ts` |
| 2% | 5 | 236 | `apps/api/src/db/schema.test.ts` | `apps/mobile/src/features/updates/app-updates.test.ts` |
| 2% | 5 | 147 | `apps/server/src/device/deviceUiSnapshot.test.ts` | `apps/desktop/src/snapShot/snapShot.test.ts` |
| 2% | 13 | 187 | `apps/server/src/mcpConnectors/httpRoute.test.ts` | `infra/relay/src/http/Api.test.ts` |
| 2% | 28 | 415 | `apps/server/src/provider/Layers/ProviderDiscoveryService.ts` | `apps/server/src/provider/Layers/ProviderService.ts` |
| 2% | 10 | 230 | `apps/server/src/provider/boundedCallbackIngress.ts` | `apps/server/src/resourceTelemetry/NativeTelemetryClient.ts` |
| 2% | 7 | 155 | `apps/server/src/provider/claudeProjectImport.ts` | `apps/desktop/src/snapShot/NiriSnapShot.ts` |
| 2% | 8 | 147 | `apps/web/src/appSnap.logic.ts` | `apps/web/src/components/ChatView.logic.ts` |
| 2% | 11 | 117 | `apps/web/src/hooks/useScratchModelCatalog.ts` | `apps/mobile/src/features/threads/new-task-flow-provider.tsx` |
| 2% | 12 | 752 | `apps/web/src/lib/automationIntent.ts` | `apps/web/src/session-logic.ts` |
| 2% | 7 | 43 | `apps/web/src/orchestrationEventCoalescing.ts` | `apps/server/src/orchestration/ActivityPayloadProjection.ts` |
| 2% | 53 | 4600 | `apps/desktop/src/main.ts` | `apps/desktop/src/window/DesktopWindow.ts` |
| 2% | 9 | 137 | `apps/server/src/agentGateway/httpRoute.test.ts` | `apps/server/src/provider/Layers/ProviderSessionReaper.test.ts` |
| 2% | 5 | 94 | `apps/server/src/agentGateway/turnPlanTools.ts` | `apps/server/src/provider/Layers/CodexSessionRuntime.test.ts` |
| 2% | 5 | 60 | `apps/server/src/externalMcp/overview.ts` | `scripts/mobile-showcase-environment.ts` |
| 2% | 9 | 97 | `apps/web/src/components/codeEditor/CodeEditorPane.tsx` | `apps/web/src/components/settings/ResourceTelemetryDiagnostics.tsx` |
| 2% | 7 | 238 | `apps/web/src/lib/browserAnnotations.test.ts` | `apps/web/src/components/pullRequest/pullRequestDetail.logic.test.ts` |
| 2% | 8 | 133 | `apps/web/src/lib/productAnalytics.test.ts` | `apps/server/src/relay/AgentAwarenessRelay.test.ts` |
| 2% | 6 | 44 | `apps/desktop/src/mediaPermissions.ts` | `apps/web/src/components/ChatView.logic.ts` |
| 2% | 12 | 442 | `apps/server/src/auth/Layers/RemotePairingControlPlane.test.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.test.ts` |
| 2% | 9 | 223 | `apps/web/src/components/chat/useChatTerminalController.ts` | `apps/web/src/components/BranchToolbarBranchSelector.tsx` |
| 2% | 19 | 509 | `apps/web/src/lib/queuedComposerDrain.ts` | `apps/web/src/components/ChatView.logic.ts` |
| 2% | 10 | 161 | `apps/server/src/orchestration/Layers/ProjectionSnapshotQuery.revision.test.ts` | `apps/server/src/orchestration/ThreadPullRequestReactor.test.ts` |
| 2% | 34 | 1280 | `apps/server/src/profileStats.ts` | `apps/server/src/orchestration/Layers/ProjectionSnapshotQuery.ts` |
| 2% | 9 | 109 | `apps/web/src/components/chat/ComposerGoalHeader.tsx` | `apps/web/src/components/DiffPanel.tsx` |
| 2% | 8 | 171 | `apps/web/src/lib/composerMentions.ts` | `apps/web/src/components/ChatView.logic.ts` |
| 2% | 6 | 94 | `apps/web/src/lib/linkChips.ts` | `apps/mobile/src/features/review/shikiReviewHighlighter.ts` |
| 2% | 18 | 404 | `apps/web/src/lib/subagentPresentation.ts` | `packages/client-runtime/src/work-log/presentation.ts` |
| 2% | 5 | 69 | `packages/shared/src/releaseDownloads.ts` | `packages/shared/src/observability.ts` |
| 2% | 5 | 149 | `apps/server/src/cloudflare/connector.integration.test.ts` | `apps/server/src/cli/theme.ts` |
| 2% | 11 | 233 | `apps/server/src/provider/acp/AcpAdapterSessionSupport.test.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.test.ts` |
| 2% | 6 | 477 | `apps/server/src/provider/claudeProcessEnv.test.ts` | `apps/server/src/provider/Layers/EventNdjsonLogger.test.ts` |
| 2% | 8 | 69 | `apps/web/src/components/ThreadRowQuickActions.tsx` | `apps/web/src/components/settings/settingsLayout.tsx` |
| 2% | 17 | 289 | `apps/web/src/components/chat/MessagesTimeline.rowOverlap.browser.tsx` | `apps/web/src/components/chat/MessagesTimeline.test.tsx` |
| 2% | 6 | 157 | `apps/web/src/components/githubInbox/GitHubIssueDetailPanel.tsx` | `apps/web/src/components/pullRequest/PullRequestSummaryTab.tsx` |
| 2% | 6 | 190 | `benchmarks/desktop-perf/lib/metrics.mjs` | `apps/desktop/src/preview/FaviconCapture.ts` |
| 2% | 5 | 67 | `apps/server/src/persistence/Layers/PullRequestAutoFixRepository.test.ts` | `apps/server/integration/orphanedProviderSessionStartup.integration.test.ts` |
| 2% | 8 | 484 | `apps/desktop/src/browserManager.test.ts` | `apps/desktop/src/window/QuitHold.test.ts` |
| 2% | 8 | 345 | `apps/desktop/src/cuaFixtures/live.ts` | `apps/web/src/components/settings/themeInspector.ts` |
| 2% | 10 | 240 | `apps/server/src/completionNotificationPreviews.ts` | `apps/desktop/src/snapShot/snapShot.ts` |
| 2% | 5 | 174 | `apps/server/src/provider/usageLimit.test.ts` | `apps/desktop/src/snapShot/snapShot.test.ts` |
| 2% | 7 | 237 | `apps/server/src/providerUsage/providers/devin.ts` | `apps/desktop/src/snapShot/snapShot.ts` |
| 2% | 16 | 219 | `apps/web/src/components/LocalImagePreview.tsx` | `apps/web/src/components/GitActionsControl.tsx` |
| 2% | 8 | 136 | `apps/web/src/components/diffPanelSelectors.ts` | `apps/web/src/components/BranchToolbar.logic.ts` |
| 2% | 15 | 370 | `apps/web/src/lib/queuedComposerDispatch.test.ts` | `apps/server/src/orchestration/Layers/OrchestrationEngine.test.ts` |
| 2% | 21 | 112 | `apps/web/src/lib/threadFork.test.ts` | `apps/web/src/components/ChatView.logic.test.ts` |
| 2% | 6 | 140 | `apps/server/src/agentGateway/connectorTools.ts` | `packages/effect-codex-app-server/scripts/generate.ts` |
| 2% | 7 | 398 | `apps/web/e2e/browserAnnotations.e2e.ts` | `apps/desktop/src/snapShot/LinuxSnapShot.ts` |
| 2% | 18 | 103 | `apps/web/src/components/settings/SettingControls.tsx` | `apps/web/src/components/GitActionsControl.tsx` |
| 2% | 7 | 216 | `apps/server/src/provider/claudeProcessEnv.ts` | `apps/server/src/process/externalLauncher.ts` |
| 2% | 12 | 298 | `apps/server/src/accountLiveActivityReporter.ts` | `packages/client-runtime/src/device/stream.ts` |
| 2% | 6 | 141 | `apps/server/src/nodeHttpServer.ts` | `apps/web/src/connection/storage.ts` |
| 2% | 9 | 189 | `apps/server/src/provider/providerBinaryResolution.ts` | `apps/server/src/cloud/bootService.ts` |
| 2% | 6 | 259 | `apps/server/src/remoteBillingPolicy.ts` | `packages/client-runtime/src/voice-input/controller.ts` |
| 2% | 10 | 201 | `apps/web/src/components/chat-drop-overlay/ChatPaneDropOverlay.tsx` | `apps/web/src/components/settings/settingsLayout.tsx` |
| 2% | 5 | 35 | `apps/web/src/hooks/useContainerSize.ts` | `apps/web/src/components/device/DeviceStreamView.tsx` |
| 2% | 5 | 65 | `apps/web/src/lib/hosts/executionImages.browser.ts` | `apps/mobile/src/lib/attachmentUpload.test.ts` |
| 2% | 9 | 308 | `apps/web/src/openThreadTabs.logic.test.ts` | `apps/server/src/orchestration/ThreadPullRequestReactor.test.ts` |
| 2% | 5 | 182 | `apps/api/src/identity/deviceRegistry.ts` | `apps/mobile/src/features/sharing/incoming-share-model.ts` |
| 2% | 17 | 1101 | `apps/desktop/src/browserAutomation/desktopBrowserAutomationHost.ts` | `apps/web/src/openVsxThemes.ts` |
| 2% | 12 | 334 | `apps/server/src/agentGateway/Layers/AgentGatewayOperationRepository.test.ts` | `apps/server/src/persistence/Layers/OrchestrationEventStore.test.ts` |
| 2% | 15 | 137 | `apps/web/src/components/ProviderUsageResetCredits.tsx` | `apps/web/src/components/GitActionsControl.tsx` |
| 2% | 15 | 360 | `apps/web/src/components/chat/MessagesTimeline.toolDetails.browser.tsx` | `apps/web/src/components/chat/MessagesTimeline.test.tsx` |
| 2% | 8 | 149 | `packages/shared/src/inlineVisual.ts` | `apps/server/src/provider/Layers/ClaudeProvider.ts` |
| 2% | 9 | 199 | `scripts/mac-update-zip-finalize.test.ts` | `apps/web/src/terminal/ghostty/surface.test.ts` |
| 2% | 9 | 179 | `apps/server/src/agentGateway/startupRecovery.ts` | `apps/server/src/resourceTelemetry/NativeTelemetryClient.ts` |
| 2% | 6 | 238 | `apps/server/src/persistence/DatabaseLifecycleLock.test.ts` | `packages/shared/src/observability.test.ts` |
| 2% | 11 | 291 | `apps/server/src/projectAgent/workerHealth.ts` | `apps/mobile/src/lib/threadActivity.ts` |
| 2% | 5 | 407 | `apps/server/src/providerUsage/providers/droid.test.ts` | `apps/server/src/provider/Layers/codexUsageLimits.test.ts` |
| 2% | 6 | 119 | `packages/shared/src/chatThreads.ts` | `apps/web/src/keybindings.ts` |
| 2% | 10 | 300 | `apps/server/src/agentGateway/inlineVisualTools.ts` | `apps/server/src/provider/Layers/CursorAdapter.ts` |
| 2% | 5 | 133 | `apps/server/src/provider/acp/AntigravityAcpSupport.test.ts` | `apps/server/src/provider/acp/AntigravityProtocol.test.ts` |
| 2% | 6 | 88 | `apps/server/src/wsConnectionSessions.ts` | `apps/server/src/provider/AntigravityAuth.ts` |
| 2% | 5 | 140 | `apps/web/src/components/chat/threadModelSelectionSync.ts` | `apps/mobile/src/state/use-thread-composer-state.ts` |
| 2% | 17 | 317 | `apps/web/src/components/chat/useChatAutomationCreation.ts` | `apps/web/src/components/CommandPalette.tsx` |
| 2% | 9 | 233 | `apps/web/src/lib/voiceRecorder.ts` | `apps/web/src/components/ChatView.logic.ts` |
| 2% | 6 | 165 | `scripts/verify-packaged-desktop-startup.test.ts` | `apps/server/src/provider/antigravityAuthSupport.test.ts` |
| 2% | 7 | 374 | `apps/desktop/src/browserAutomation/browserDiagnostics.test.ts` | `apps/desktop/src/preview/FaviconCapture.test.ts` |
| 2% | 9 | 204 | `apps/desktop/src/reliabilityReporter.ts` | `packages/client-runtime/src/device/stream.ts` |
| 2% | 13 | 172 | `apps/web/src/components/chat/InlineVisualFrame.tsx` | `apps/web/src/components/RightPanelTabs.tsx` |
| 2% | 16 | 209 | `apps/web/src/components/chat/ThreadFindBar.tsx` | `apps/web/src/routes/_chat.pull-requests.tsx` |
| 2% | 12 | 94 | `apps/web/src/components/settings/accounts/providerAccountShared.tsx` | `apps/web/src/components/onboarding/WelcomeWizard.tsx` |
| 2% | 6 | 148 | `apps/web/src/lib/projectCatalog/model.ts` | `apps/web/src/openVsxThemes.ts` |
| 2% | 5 | 125 | `scripts/browser-ci-partitions.test.ts` | `apps/desktop/src/preview/FaviconCapture.test.ts` |
| 2% | 9 | 426 | `scripts/verify-packaged-desktop-startup.ts` | `apps/desktop/src/snapShot/PortalCaptureShortcut.ts` |
| 2% | 5 | 51 | `apps/server/src/endpointReporter.ts` | `apps/web/src/components/device/DeviceStreamView.tsx` |
| 2% | 11 | 408 | `apps/server/src/hostConnections/registry.test.ts` | `packages/client-runtime/src/device/stream.test.ts` |
| 2% | 10 | 138 | `apps/server/src/persistence/Layers/RemoteDeviceTrust.ts` | `apps/server/src/serverSettings.ts` |
| 2% | 10 | 219 | `apps/web/src/lib/chatProjects.test.ts` | `apps/server/src/orchestration/ThreadPullRequestReactor.test.ts` |
| 2% | 10 | 252 | `apps/web/src/lib/groupWorkerRouting.test.ts` | `apps/server/src/orchestration/projector.test.ts` |
| 2% | 9 | 336 | `apps/server/src/agentGateway/sessionLease.test.ts` | `apps/desktop/src/snapShot/snapShot.test.ts` |
| 2% | 17 | 1105 | `apps/server/src/device/DeviceManager.ts` | `apps/mobile/src/state/use-thread-outbox-drain.ts` |
| 2% | 12 | 493 | `apps/server/src/managedWorktrees.test.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.test.ts` |
| 2% | 16 | 315 | `apps/server/src/orchestration/dispatchCommandNormalization.test.ts` | `apps/server/src/project/AgentSessionImporter.test.ts` |
| 2% | 19 | 479 | `apps/server/src/orchestration/quitResume.test.ts` | `apps/server/src/relay/AgentAwarenessRelay.test.ts` |
| 2% | 13 | 98 | `apps/server/src/provider/Layers/GrokAdapter.resume.test.ts` | `apps/server/src/provider/Layers/CursorAdapter.test.ts` |
| 2% | 8 | 154 | `apps/server/src/terminal/subprocessActivity.ts` | `apps/desktop/src/snapShot/snapShot.ts` |
| 2% | 11 | 63 | `apps/web/src/components/tasks/TaskRowTitle.tsx` | `apps/web/src/components/RightPanelTabs.tsx` |
| 2% | 26 | 392 | `apps/web/src/lib/threadHandoff.test.ts` | `apps/mobile/src/lib/threadActivity.test.ts` |
| 2% | 5 | 120 | `apps/server/src/deviceIdentity/store.ts` | `apps/server/src/orchestration/Normalizer.attachments.test.ts` |
| 2% | 7 | 176 | `apps/server/src/wsStreamAdmission.test.ts` | `packages/client-runtime/src/authorization/layer.test.ts` |
| 2% | 52 | 1043 | `apps/web/src/hooks/useComposerSlashCommands.ts` | `apps/web/src/components/chat/ChatComposer.tsx` |
| 2% | 8 | 104 | `apps/web/src/hooks/useOpenThreadTabs.browser.tsx` | `apps/server/src/relay/AgentAwarenessRelay.test.ts` |
| 2% | 8 | 100 | `apps/web/src/lib/spaceGrouping.ts` | `apps/web/src/components/pullRequest/pullRequestDetail.logic.ts` |
| 2% | 22 | 3133 | `apps/web/src/wsTransport.test.ts` | `apps/server/src/orchestration/Layers/OrchestrationEngine.test.ts` |
| 2% | 8 | 430 | `scripts/computer-use-fixtures/background-input-regression.mjs` | `apps/server/src/provider/Layers/EventNdjsonLogger.test.ts` |
| 2% | 9 | 1037 | `apps/desktop/src/browserAutomation/desktopBrowserAutomationHost.test.ts` | `apps/server/src/provider/Layers/ClaudeAdapter.test.ts` |
| 2% | 8 | 208 | `apps/server/src/accountInboxReporter.test.ts` | `apps/server/src/auth/SessionStore.test.ts` |
| 2% | 6 | 445 | `apps/server/src/provider/openCodeV2Client.test.ts` | `apps/server/src/provider/opencodeRuntime.inventory.test.ts` |
| 2% | 6 | 57 | `apps/server/src/provider/providerRuntimeTextBatch.ts` | `apps/server/src/provider/Layers/EventNdjsonLogger.ts` |
| 2% | 13 | 312 | `apps/web/src/components/kanban/useKanbanCardContextMenu.tsx` | `apps/web/src/components/ChatMarkdown.tsx` |
| 2% | 6 | 290 | `apps/web/src/lib/hosts/connectionClients.test.ts` | `apps/web/src/components/preview/PreviewView.test.tsx` |
| 2% | 9 | 165 | `packages/contracts/src/agentGateway.test.ts` | `packages/contracts/src/orchestration.test.ts` |
| 2% | 23 | 898 | `apps/web/src/hooks/useSidebarThreadActions.ts` | `apps/web/src/hooks/useThreadActions.ts` |
| 2% | 8 | 287 | `apps/web/src/lib/hosts/workspaceSessions.test.ts` | `apps/web/src/components/settings/SnapShotSettings.test.tsx` |
| 2% | 5 | 53 | `apps/web/src/lib/remarkImageRows.ts` | `apps/mobile/src/features/review/nativeReviewDiffAdapter.ts` |
| 2% | 14 | 411 | `apps/desktop/src/betaChannel.ts` | `apps/server/src/provider/Layers/CursorProvider.ts` |
| 2% | 8 | 366 | `apps/web/src/lib/projectReactQuery.ts` | `apps/web/src/state/queries.ts` |
| 2% | 6 | 159 | `scripts/computer-use-fixtures/packaged-evidence.ts` | `apps/web/src/components/ChatView.logic.ts` |
| 2% | 6 | 86 | `apps/desktop/src/browserAutomation/navigationHistory.ts` | `apps/web/src/environments/primary/auth.ts` |
| 2% | 6 | 91 | `apps/desktop/src/keepAwake.ts` | `apps/web/src/browserHistoryStore.ts` |
| 2% | 5 | 64 | `apps/web/src/components/chat/auxiliary/auxiliaryPanel.logic.test.ts` | `apps/mobile/src/state/thread-outbox.test.ts` |
| 2% | 13 | 228 | `apps/web/src/lib/queuedComposerDispatch.ts` | `apps/server/src/orchestration/Layers/ProviderCommandReactor.ts` |
| 2% | 12 | 418 | `apps/server/src/provider/unmappedProviderEvents.ts` | `packages/client-runtime/src/work-log/commandLabel.ts` |
| 2% | 5 | 145 | `apps/desktop/native/appsnap/main.swift` | `apps/mobile/modules/t3-terminal/ios/T3TerminalView.swift` |
| 2% | 9 | 350 | `apps/e2e/src/headlessClient.ts` | `apps/mobile/src/features/sharing/incoming-share-model.ts` |
| 2% | 7 | 54 | `apps/server/src/persistence/Migrations/121_ClearAutomationDefinitionProviderOptions.ts` | `apps/server/src/serverSettings.ts` |
| 2% | 8 | 262 | `apps/web/src/components/chat/project/useProjectAgentSummaries.ts` | `apps/mobile/src/state/use-composer-drafts.ts` |
| 2% | 10 | 266 | `apps/web/src/lib/automationDraft.ts` | `apps/mobile/src/features/threads/new-task-flow-provider.tsx` |
| 2% | 8 | 186 | `apps/web/src/lib/draftThreadCreate.test.ts` | `apps/server/src/orchestration/Layers/OrchestrationEngine.test.ts` |
| 2% | 12 | 1805 | `apps/api/src/routes/v1.hostAuth.test.ts` | `infra/relay/src/http/Api.test.ts` |
| 2% | 22 | 1559 | `apps/desktop/src/appSnapManager.ts` | `apps/web/src/composerDraftStore.ts` |
| 2% | 28 | 764 | `apps/server/src/profileStatsArchive.ts` | `apps/server/src/orchestration/Layers/ProjectionSnapshotQuery.ts` |
| 2% | 7 | 253 | `apps/server/src/remotePairing/management.test.ts` | `apps/mobile/src/lib/attachmentUpload.test.ts` |
| 2% | 9 | 135 | `apps/server/src/todo/Layers/TodoService.ts` | `apps/server/src/cloud/bootService.ts` |
| 2% | 7 | 127 | `apps/web/src/components/chat/environment/useThreadNotesAutosave.ts` | `apps/web/src/components/ComposerPromptEditorTiptap.tsx` |
| 2% | 15 | 408 | `apps/web/src/components/chat/threadFind.logic.ts` | `apps/mobile/src/state/use-composer-drafts.ts` |
| 2% | 6 | 95 | `apps/web/src/components/computer/useComputerPreviewTap.ts` | `apps/mobile/src/features/threads/QuestionAttachments.tsx` |
| 2% | 12 | 121 | `apps/web/src/lib/composerPastedText.ts` | `apps/web/src/session-logic.ts` |
| 2% | 8 | 55 | `apps/web/src/lib/runtimeMode.ts` | `apps/server/src/provider/Layers/CursorProvider.ts` |
| 2% | 5 | 256 | `apps/desktop/src/browserAutomation/dialogHandling.test.ts` | `apps/mobile/src/lib/attachmentUpload.test.ts` |
| 2% | 6 | 228 | `apps/desktop/src/cuaFixtures/cancellation.ts` | `apps/desktop/src/permissions/MacPermissionHelper.ts` |
| 2% | 15 | 635 | `apps/server/src/authEffectRoute.test.ts` | `infra/relay/src/http/Api.test.ts` |
| 2% | 19 | 759 | `apps/server/src/checkpointing/Layers/CheckpointStore.test.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.test.ts` |
| 2% | 14 | 519 | `apps/server/src/persistence/Layers/ProjectAgentRepository.test.ts` | `apps/server/src/orchestration/Layers/ProjectionPipeline.test.ts` |
| 2% | 5 | 107 | `apps/server/src/providerUsage/parse.ts` | `apps/server/integration/TestProviderAdapter.integration.ts` |
| 2% | 5 | 115 | `apps/web/src/pinning.logic.ts` | `apps/server/src/orchestration/ActivityPayloadProjection.ts` |
| 2% | 9 | 478 | `apps/server/integration/device.integration.test.ts` | `apps/server/src/cli/theme.ts` |
| 2% | 14 | 299 | `apps/server/src/agentGateway/remoteTools.test.ts` | `apps/server/src/relay/AgentAwarenessRelay.test.ts` |
| 2% | 28 | 1338 | `apps/server/src/device/IosSimulatorBackend.ts` | `scripts/mobile-showcase.ts` |
| 2% | 10 | 405 | `apps/web/perf/pipeline.tsx` | `apps/web/src/components/RightPanelTabs.tsx` |
| 2% | 7 | 172 | `apps/web/src/components/DiffPanel.capacity.browser.tsx` | `packages/shared/src/git.ts` |
| 2% | 7 | 158 | `apps/web/src/components/Sidebar.uiState.ts` | `apps/mobile/src/features/review/reviewModel.ts` |
| 2% | 14 | 101 | `apps/web/src/components/SidebarListSection.tsx` | `apps/web/src/routes/_chat.pull-requests.tsx` |
| 2% | 9 | 167 | `apps/web/src/splitView.logic.ts` | `apps/web/src/components/settings/ThemeEditorPanel.tsx` |
| 2% | 11 | 424 | `packages/shared/src/jsonrpc-stdio.ts` | `apps/desktop/src/preview/FaviconCapture.ts` |
| 1% | 7 | 38 | `apps/server/src/orchestration/providerIntentClassification.test.ts` | `apps/server/src/relay/AgentAwarenessRelay.test.ts` |
| 1% | 28 | 637 | `apps/server/src/projectAgent/hubWorkService.test.ts` | `apps/web/src/components/ChatView.logic.test.ts` |
| 1% | 8 | 210 | `apps/server/src/provider/claudeTokenUsage.ts` | `apps/server/src/provider/Layers/CursorProvider.ts` |
| 1% | 6 | 79 | `apps/web/src/components/SidebarGroupsSurface.logic.ts` | `apps/web/src/components/ChatView.logic.ts` |
| 1% | 6 | 301 | `apps/web/src/components/chat/InlineVisualFrame.browser.tsx` | `apps/web/src/components/ChatMarkdown.test.tsx` |
| 1% | 15 | 301 | `apps/web/src/components/chat/threadFind.logic.test.ts` | `apps/web/src/components/chat/MessagesTimeline.test.tsx` |
| 1% | 7 | 557 | `packages/contracts/src/computer.ts` | `packages/contracts/src/previewAutomation.ts` |
| 1% | 7 | 234 | `scripts/smoke-mac-update-artifact.ts` | `apps/server/src/serviceLauncher.ts` |
| 1% | 6 | 263 | `apps/desktop/src/browserAutomation/browserVault.test.ts` | `apps/mobile/src/state/use-thread-outbox-drain.test.ts` |
| 1% | 9 | 182 | `apps/desktop/src/updateInstallMarker.ts` | `apps/mobile/src/state/use-thread-outbox-drain.ts` |
| 1% | 12 | 127 | `apps/server/src/agentGateway/completionDelivery.ts` | `apps/server/src/serverRuntimeStartup.ts` |
| 1% | 5 | 86 | `apps/web/src/components/chat/ComposerCommandMenu.browser.tsx` | `packages/client-runtime/src/device/stream.test.ts` |
| 1% | 6 | 32 | `apps/web/src/components/chat/MessageActionButton.tsx` | `apps/web/src/components/pullRequest/PullRequestSummaryTab.tsx` |
| 1% | 22 | 630 | `apps/web/src/components/kanban/kanban.logic.ts` | `apps/web/src/components/ChatView.logic.ts` |
| 1% | 5 | 55 | `apps/web/src/hooks/useDesktopMenuShortcuts.ts` | `apps/web/src/components/chat/ModelPickerContent.tsx` |
| 1% | 10 | 210 | `apps/web/src/lib/browserAnnotations.ts` | `apps/server/src/project/AgentSessionScanner.ts` |
| 1% | 28 | 796 | `apps/web/src/lib/queuedComposerDrain.test.ts` | `apps/mobile/src/lib/threadActivity.test.ts` |
| 1% | 14 | 275 | `apps/web/src/routes/_chat.settings.tsx` | `apps/web/src/components/pullRequest/PullRequestDetailPanel.tsx` |
| 1% | 7 | 277 | `apps/desktop/scripts/provision-cua-driver.mjs` | `apps/server/src/cli/update.ts` |
| 1% | 8 | 374 | `apps/desktop/src/browserAutomation/browserDiagnostics.ts` | `apps/desktop/src/preview/PickPreload.ts` |
| 1% | 5 | 163 | `apps/server/src/computer/computerVisibleUse.ts` | `apps/web/src/lib/openPullRequestLink.ts` |
| 1% | 5 | 101 | `apps/server/src/config.test.ts` | `apps/server/src/provider/providerMaintenance.test.ts` |
| 1% | 9 | 107 | `apps/server/src/gitHandoffOperations.test.ts` | `apps/server/src/serverSettings.test.ts` |
| 1% | 5 | 108 | `apps/server/src/voiceTranscription.ts` | `apps/mobile/src/lib/attachmentUpload.ts` |
| 1% | 9 | 26 | `apps/web/src/components/chat/ModelStarButton.tsx` | `apps/web/src/components/RightPanelTabs.tsx` |
| 1% | 5 | 163 | `apps/web/src/components/codeEditor/CodeEditorPane.performance.browser.tsx` | `apps/web/src/components/ProviderUpdateEnvironmentRows.tsx` |
| 1% | 5 | 89 | `apps/web/src/lib/central-icons.tsx` | `apps/web/src/components/ui/toast.tsx` |
| 1% | 5 | 53 | `apps/desktop/scripts/native-build-cache.mjs` | `apps/server/integration/OrchestrationEngineHarness.integration.ts` |
| 1% | 7 | 406 | `apps/desktop/src/browserAnnotations/coordinator.test.ts` | `apps/mobile/src/state/thread-outbox.test.ts` |
| 1% | 7 | 352 | `apps/desktop/src/cuaFixtures/focusProbe.ts` | `apps/server/src/vcs/GitVcsDriver.ts` |
| 1% | 5 | 227 | `apps/desktop/src/cuaFixtures/gateway.ts` | `apps/web/src/openVsxThemes.ts` |
| 1% | 22 | 198 | `apps/web/src/components/chat/OpenThreadTabStrip.tsx` | `apps/web/src/components/CommandPalette.tsx` |
| 1% | 9 | 236 | `apps/web/src/components/chat/useChatLocalDispatch.ts` | `apps/web/src/components/ThreadTerminalDrawer.tsx` |
| 1% | 9 | 336 | `packages/shared/src/frameTransport.ts` | `apps/mobile/src/lib/composerImages.ts` |
| 1% | 5 | 202 | `scripts/computer-use-fixtures/sample-packaged-resources.ts` | `apps/mobile/src/lib/composerImages.ts` |
| 1% | 5 | 56 | `apps/desktop/src/computerEmergencyStopNotice.ts` | `apps/mobile/src/features/agent-awareness/remoteRegistration.ts` |
| 1% | 6 | 261 | `apps/server/src/computer/ComputerSpaceBroker.ts` | `apps/mobile/src/state/queries.ts` |
| 1% | 5 | 65 | `apps/server/src/device/helperSandbox.ts` | `apps/server/src/provider/providerMaintenance.ts` |
| 1% | 10 | 96 | `apps/web/src/components/AppSnapWelcomeDialog.tsx` | `apps/web/src/components/GitActionsControl.tsx` |
| 1% | 5 | 182 | `apps/web/src/components/PdfResourceLifecycle.browser.tsx` | `apps/web/src/components/diffs/StyledDiffCodeView.test.tsx` |
| 1% | 8 | 41 | `apps/web/src/components/chat/SubagentRunningChip.tsx` | `apps/web/src/components/RightPanelTabs.tsx` |
| 1% | 6 | 430 | `apps/web/src/components/codeEditor/CodeEditorPane.browser.tsx` | `apps/web/src/components/settings/DeviceHostsSettings.tsx` |
| 1% | 5 | 71 | `apps/web/src/lib/providerSetupStatus.ts` | `apps/server/src/provider/opencodeRuntime.ts` |
| 1% | 10 | 91 | `apps/server/src/externalMcp/waitForTask.ts` | `packages/client-runtime/src/state/subagentRuntime.ts` |
| 1% | 9 | 107 | `apps/web/src/asyncUserInput.test.ts` | `packages/client-runtime/src/state/threadReducer.test.ts` |
| 1% | 8 | 135 | `apps/web/src/components/chat/ChatThreadSurfacePrimitives.tsx` | `apps/web/src/components/files/FilePreviewPanel.tsx` |
| 1% | 6 | 101 | `apps/web/src/components/chat/resolveChatPromptCaptures.ts` | `apps/web/src/components/settings/KeybindingsSettings.tsx` |
| 1% | 14 | 378 | `apps/web/src/components/useSpacesController.ts` | `apps/web/src/components/CommandPalette.tsx` |
| 1% | 14 | 619 | `apps/web/src/lib/automationForm.ts` | `apps/mobile/src/features/threads/new-task-flow-provider.tsx` |
| 1% | 6 | 167 | `packages/profile-ui/src/heatmap.tsx` | `apps/mobile/src/features/threads/thread-work-log.tsx` |
| 1% | 7 | 76 | `packages/shared/src/pinnedMessages.ts` | `apps/mobile/src/state/use-thread-outbox-drain.ts` |
| 1% | 9 | 125 | `apps/analytics/src/App.tsx` | `apps/web/src/components/ThreadTerminalDrawer.tsx` |
| 1% | 10 | 468 | `apps/desktop/src/browserAutomation/workspaceUpload.ts` | `apps/desktop/src/preview/FaviconCapture.ts` |
| 1% | 6 | 65 | `apps/server/perf/threadDetailStages.perf.test.ts` | `apps/server/integration/OrchestrationEngineHarness.integration.ts` |
| 1% | 7 | 396 | `apps/server/src/device/FakeDeviceBackend.ts` | `apps/server/src/provider/Layers/CursorProvider.ts` |
| 1% | 12 | 54 | `apps/server/src/provider/promptAttachments.ts` | `apps/server/src/provider/Layers/CursorAdapter.ts` |
| 1% | 6 | 39 | `apps/web/src/components/pullRequest/PullRequestStackPopover.browser.tsx` | `apps/server/src/orchestration/PullRequestSyncReactor.test.ts` |
| 1% | 7 | 92 | `apps/web/src/computerStateStore.ts` | `apps/mobile/src/state/use-composer-drafts.ts` |
| 1% | 22 | 970 | `apps/web/src/wsNativeApi.test.ts` | `apps/server/src/orchestration/projector.test.ts` |
| 1% | 6 | 98 | `packages/shared/src/pullRequestList.ts` | `apps/server/src/provider/acp/XAiAcpExtension.ts` |
| 1% | 5 | 340 | `apps/server/src/agentGateway/projectAgentTools.ts` | `apps/server/src/orchestration/Layers/ProviderRuntimeIngestion.ts` |
| 1% | 6 | 445 | `apps/server/src/externalMcp/bridge.test.ts` | `apps/mobile/src/state/thread-outbox.test.ts` |
| 1% | 6 | 74 | `apps/server/src/orchestration/providerRuntimeProgressCoalescing.ts` | `apps/server/src/relay/AgentAwarenessRelay.ts` |
| 1% | 6 | 134 | `apps/server/src/provider/claudeCredentialKeepalive.test.ts` | `apps/server/integration/orchestrationEngine.integration.test.ts` |
| 1% | 7 | 92 | `apps/server/src/remotePairing/cli.test.ts` | `packages/client-runtime/src/rpc/session.test.ts` |
| 1% | 7 | 104 | `apps/web/src/components/chat/useTimelineRowOverlapGuard.ts` | `apps/web/src/components/ThreadNotificationCoordinator.tsx` |
| 1% | 24 | 930 | `apps/web/src/components/codeEditor/MonacoEditorSurface.tsx` | `apps/web/src/components/pullRequest/PullRequestDetailPanel.tsx` |
| 1% | 13 | 369 | `apps/web/src/components/githubInbox/githubInbox.logic.ts` | `apps/web/src/components/pullRequest/pullRequestList.logic.ts` |
| 1% | 6 | 96 | `apps/web/src/components/profile/ShareCard.tsx` | `apps/web/src/components/usage/UsagePage.tsx` |
| 1% | 27 | 1176 | `apps/web/src/storeProjection.ts` | `apps/web/src/composerDraftStore.ts` |
| 1% | 11 | 347 | `apps/web/src/types.ts` | `packages/contracts/src/ipc.ts` |
| 1% | 8 | 513 | `apps/api/src/identity/workos.ts` | `apps/server/src/provider/Layers/ProviderService.ts` |
| 1% | 12 | 397 | `apps/desktop/src/browserUsePipeServer.ts` | `apps/web/src/openVsxThemes.ts` |
| 1% | 9 | 71 | `apps/marketing/src/components/MobileNav.tsx` | `apps/web/src/components/pullRequest/PullRequestSummaryTab.tsx` |
| 1% | 50 | 4792 | `apps/server/src/automation/Layers/AutomationService.test.ts` | `apps/server/src/orchestration/Layers/ProjectionPipeline.test.ts` |
| 1% | 7 | 223 | `apps/server/src/provider/codexProjectImport.ts` | `apps/mobile/src/features/review/shikiReviewHighlighter.ts` |
| 1% | 17 | 219 | `apps/web/src/components/chat/ComposerModelPickerTabs.tsx` | `apps/web/src/components/ChatMarkdown.tsx` |
| 1% | 11 | 333 | `apps/web/src/components/chat/agentActivity.logic.test.ts` | `packages/client-runtime/src/work-log/presentation.test.ts` |
| 1% | 7 | 233 | `apps/web/src/components/chat/floatingBrowserPanel.logic.ts` | `apps/web/src/components/settings/ThemeEditorPanel.tsx` |
| 1% | 98 | 970 | `apps/web/src/components/chat/useChatTurnSubmission.ts` | `apps/web/src/components/ChatView.tsx` |
| 1% | 5 | 63 | `apps/server/src/provider/piStoragePaths.ts` | `apps/server/src/cloud/bootService.ts` |
| 1% | 6 | 121 | `apps/web/src/components/kanban/useKanbanTaskScratchDraft.ts` | `apps/mobile/src/state/use-thread-composer-state.ts` |
| 1% | 5 | 230 | `apps/web/src/lib/accountUsageAdapter.ts` | `apps/mobile/src/lib/composerImages.ts` |
| 1% | 9 | 134 | `apps/web/src/lib/groupProjects.ts` | `apps/server/src/project/AgentSessionScanner.ts` |
| 1% | 6 | 95 | `apps/web/src/lib/openUsageRateLimits.ts` | `apps/web/src/session-logic.ts` |
| 1% | 6 | 104 | `apps/web/src/lib/pullRequestGitCache.ts` | `apps/mobile/src/state/use-thread-outbox-drain.ts` |
| 1% | 6 | 109 | `apps/desktop/src/cuaRuntimeOwnership.ts` | `apps/mobile/src/state/use-thread-outbox-drain.ts` |
| 1% | 15 | 249 | `apps/server/src/assistantImageAttachments.ts` | `apps/server/src/provider/Layers/CodexSessionRuntime.ts` |
| 1% | 16 | 519 | `apps/server/src/projectAgent/hubWorkService.ts` | `apps/server/src/ws.ts` |
| 1% | 28 | 311 | `apps/server/src/projectAgent/libraryGit.ts` | `apps/server/src/vcs/GitVcsDriverCore.ts` |
| 1% | 6 | 215 | `apps/web/src/components/chat/useComposerDiscovery.ts` | `apps/mobile/src/features/threads/ThreadRouteScreen.tsx` |
| 1% | 7 | 284 | `apps/web/src/components/inbox/inboxStories.ts` | `apps/server/src/provider/Layers/CursorProvider.ts` |
| 1% | 15 | 139 | `apps/web/src/components/tasks/TaskCard.tsx` | `apps/web/src/components/pullRequest/PullRequestDetailPanel.tsx` |
| 1% | 6 | 109 | `apps/web/src/lib/pdf/pdfLinks.ts` | `apps/web/src/components/settings/ThemeEditorPanel.tsx` |
| 1% | 5 | 51 | `apps/web/src/lib/stagedDraftNavigation.ts` | `apps/mobile/src/features/sharing/IncomingShareProvider.tsx` |
| 1% | 12 | 142 | `apps/web/src/routes/-rootEventInvalidation.ts` | `apps/server/src/orchestration/Layers/ProjectionPipeline.ts` |
| 1% | 7 | 224 | `benchmarks/desktop-perf/lib/launch.mjs` | `apps/server/integration/OrchestrationEngineHarness.integration.ts` |
| 1% | 11 | 842 | `apps/api/src/liveActivities/service.test.ts` | `apps/server/src/relay/AgentAwarenessRelay.test.ts` |
| 1% | 14 | 257 | `apps/server/src/provider/claudeTaskTracker.ts` | `apps/web/src/session-logic.ts` |
| 1% | 5 | 211 | `apps/server/src/provider/codexProjectImport.test.ts` | `scripts/mobile-showcase-environment.ts` |
| 1% | 9 | 253 | `apps/server/src/provider/providerModelDiscoveryCache.ts` | `apps/server/src/provider/opencodeRuntime.ts` |
| 1% | 5 | 132 | `apps/server/src/providerChildEnvironment.test.ts` | `apps/server/src/usage/usageScanCache.test.ts` |
| 1% | 21 | 643 | `apps/web/src/components/hosts/WorkspacePanels.tsx` | `apps/mobile/src/features/threads/ThreadFeed.tsx` |
| 1% | 21 | 320 | `apps/web/src/components/settings/ShortcutRecorderDialog.tsx` | `apps/web/src/components/GitActionsControl.tsx` |
| 1% | 13 | 244 | `apps/web/src/lib/hosts/workspaceSessions.ts` | `apps/mobile/src/lib/threadActivity.ts` |
| 1% | 10 | 521 | `apps/web/src/storeSelectors.test.ts` | `apps/server/src/mcp/toolkits/pullRequests/handlers.test.ts` |
| 1% | 5 | 461 | `apps/desktop/native/appsnap/WindowCapture.swift` | `apps/mobile/modules/t3-native-controls/ios/T3NativeVideoPresentation.swift` |
| 1% | 46 | 5142 | `apps/server/src/codexAppServerManager.test.ts` | `apps/server/src/provider/Layers/CodexSessionRuntime.test.ts` |
| 1% | 5 | 286 | `apps/server/src/compactDatabase.test.ts` | `apps/desktop/src/preview/FaviconCapture.test.ts` |
| 1% | 9 | 169 | `apps/server/src/computer/computerAuditLog.ts` | `apps/server/src/orchestration/ActivityPayloadProjection.ts` |
| 1% | 6 | 235 | `apps/web/src/components/kanban/useKanbanCardContextMenu.test.ts` | `apps/server/src/mcp/toolkits/pullRequests/handlers.test.ts` |
| 1% | 11 | 411 | `apps/web/src/components/tasks/useTodos.browser.tsx` | `apps/server/src/relay/AgentAwarenessRelay.test.ts` |
| 1% | 8 | 121 | `apps/web/src/lib/remarkComposerChips.ts` | `apps/mobile/src/state/use-thread-outbox-drain.ts` |
| 1% | 15 | 266 | `apps/web/src/lib/threadHandoff.ts` | `apps/mobile/src/lib/threadActivity.ts` |
| 1% | 5 | 121 | `apps/desktop/scripts/source-desktop-launch.test.mjs` | `packages/client-runtime/src/relay/managedRelayState.test.ts` |
| 1% | 13 | 68 | `apps/server/src/orchestration/providerIntentClassification.ts` | `packages/contracts/src/orchestration.ts` |
| 1% | 5 | 40 | `apps/web/src/storageOriginMigration.ts` | `apps/web/src/browserFaviconStore.ts` |
| 1% | 5 | 136 | `apps/web/src/threadDerivation.ts` | `packages/client-runtime/src/state/threadReducer.ts` |
| 1% | 24 | 1010 | `apps/server/src/accountUsageReporter.test.ts` | `apps/server/src/orchestration/Layers/ProjectionSnapshotQuery.test.ts` |
| 1% | 15 | 181 | `apps/server/src/composerPrediction/Layers/ComposerPredictionService.ts` | `apps/server/src/orchestration/Layers/ProviderCommandReactor.ts` |
| 1% | 7 | 297 | `apps/server/src/computer/ComputerApprovalGate.test.ts` | `apps/desktop/src/preview/FaviconCapture.test.ts` |
| 1% | 12 | 473 | `apps/server/src/providerUsage/index.ts` | `apps/desktop/src/snapShot/DesktopSnapShot.ts` |
| 1% | 10 | 59 | `apps/web/src/components/TerminalScrollToBottom.tsx` | `apps/web/src/components/ThreadTerminalDrawer.tsx` |
| 1% | 10 | 302 | `apps/web/src/components/chat/useChatWorkspaceSelection.ts` | `apps/web/src/composerDraftStore.test.ts` |
| 1% | 8 | 433 | `benchmarks/desktop-perf/lib/tests-session.mjs` | `apps/mobile/src/state/use-thread-outbox-drain.ts` |
| 1% | 12 | 364 | `apps/desktop/src/browserAutomation/navigationTracker.ts` | `apps/web/src/terminal/ghostty/surface.ts` |
| 1% | 13 | 553 | `apps/server/src/hostUsageSnapshot.test.mjs` | `apps/mobile/src/state/use-composer-drafts.test.ts` |
| 1% | 8 | 124 | `apps/server/src/provider/skillPromptInjection.ts` | `apps/desktop/src/snapShot/snapShot.ts` |
| 1% | 11 | 47 | `apps/web/src/components/hosts/ShortCodeField.tsx` | `apps/web/src/components/onboarding/WelcomeWizard.tsx` |
| 1% | 14 | 617 | `apps/web/src/hooks/useProviderModelCatalog.ts` | `apps/mobile/src/features/threads/ThreadSettingsSheet.tsx` |
| 1% | 5 | 140 | `apps/web/src/lib/chatFirstSend.test.ts` | `apps/web/src/components/CommandPalette.logic.test.ts` |
| 1% | 15 | 494 | `packages/shared/src/subagents.ts` | `apps/server/src/provider/Layers/CodexAdapter.ts` |
| 1% | 13 | 159 | `apps/web/src/lib/pullRequestContext.ts` | `apps/web/src/session-logic.ts` |
| 1% | 11 | 122 | `apps/server/src/devServerManager.ts` | `apps/server/src/terminal/Manager.ts` |
| 1% | 13 | 66 | `apps/server/src/orchestration/turnStartSession.ts` | `apps/server/src/orchestration/Layers/ProviderCommandReactor.ts` |
| 1% | 7 | 124 | `apps/server/src/provider/claudeMonitorEvents.test.ts` | `apps/server/src/project/AgentSessionImporter.test.ts` |
| 1% | 9 | 226 | `apps/server/src/provider/claudeWorkflowRuntime.ts` | `apps/desktop/src/preview/BrowserImport/Sources.ts` |
| 1% | 8 | 122 | `apps/web/scripts/transform-cache.ts` | `apps/mobile/src/features/review/shikiReviewHighlighter.ts` |
| 1% | 11 | 86 | `apps/web/src/components/DiffPanelChangeMarkers.tsx` | `apps/web/src/components/ThreadTerminalDrawer.tsx` |
| 1% | 17 | 244 | `apps/web/src/components/chat/ImportedHistory.browser.tsx` | `apps/web/src/components/chat/MessagesTimeline.test.tsx` |
| 1% | 5 | 49 | `apps/web/src/components/chat/environment/EnvironmentStudioOutputsSection.tsx` | `apps/web/src/components/AgentsPanel.tsx` |
| 1% | 10 | 154 | `apps/web/src/components/chat/environment/usePinnedMessageActions.ts` | `apps/web/src/components/CommandPalette.tsx` |
| 1% | 8 | 126 | `apps/web/src/components/chat/group/libraryPanel.logic.ts` | `apps/web/src/components/pullRequest/pullRequestDetail.logic.ts` |
| 1% | 13 | 112 | `apps/web/src/components/hosts/DeviceLinkApproval.tsx` | `apps/web/src/components/pullRequest/PullRequestDetailPanel.tsx` |
| 1% | 7 | 92 | `apps/web/src/lib/assistantSelections.test.ts` | `apps/web/src/components/chat/MessagesTimeline.test.tsx` |
| 1% | 5 | 31 | `apps/web/src/routes/-threadTerminalFence.ts` | `infra/relay/src/agentActivity/ApnsDeliveries.ts` |
| 1% | 6 | 104 | `packages/contracts/src/editorDebug.ts` | `packages/contracts/src/previewAutomation.ts` |
| 1% | 8 | 108 | `packages/contracts/src/editorIntelligence.ts` | `packages/contracts/src/previewAutomation.ts` |
| 1% | 7 | 630 | `packages/shared/src/account.ts` | `apps/server/src/pullRequest/BitbucketPullRequestApi.ts` |
| 1% | 5 | 155 | `packages/shared/src/kanban.ts` | `packages/client-runtime/src/state/subagentRuntime.ts` |
| 1% | 15 | 718 | `apps/desktop/src/betaDiagnostics.ts` | `apps/server/src/provider/Layers/GrokAdapter.ts` |
| 1% | 5 | 253 | `apps/server/scripts/acp-wire-benchmark.ts` | `packages/shared/src/observability.ts` |
| 1% | 13 | 448 | `apps/server/src/agentGateway/threadDiagnosticTools.ts` | `apps/server/src/orchestration/Layers/ProjectionSnapshotQuery.ts` |
| 1% | 6 | 366 | `apps/server/src/orchestration/projectImportCatalog.test.ts` | `apps/mobile/src/state/use-thread-outbox-drain.test.ts` |
| 1% | 11 | 182 | `apps/server/src/orchestration/providerRuntimeProgressCoalescing.test.ts` | `packages/contracts/src/providerRuntime.ts` |
| 1% | 20 | 612 | `apps/server/src/provider/Layers/ProviderDiscoveryService.test.ts` | `apps/server/src/provider/Layers/ProviderRegistry.test.ts` |
| 1% | 6 | 63 | `apps/web/src/hooks/useComputerControlModeChange.ts` | `apps/web/src/components/BranchToolbarBranchSelector.tsx` |
| 1% | 5 | 165 | `apps/web/src/lib/spaces.test.ts` | `apps/mobile/src/lib/composerFiles.test.ts` |
| 1% | 6 | 48 | `apps/desktop/src/agentCursorPreference.ts` | `apps/server/src/orchestration/ActivityPayloadProjection.ts` |
| 1% | 9 | 203 | `apps/server/src/providerUsage/codexResetCredits.test.ts` | `apps/web/src/components/Sidebar.logic.test.ts` |
| 1% | 7 | 377 | `apps/server/src/remoteBillingPolicy.test.ts` | `apps/mobile/src/state/use-thread-outbox-drain.test.ts` |
| 1% | 6 | 174 | `apps/desktop/src/runningChatsQuitGuard.test.ts` | `apps/web/src/components/CommandPalette.logic.test.ts` |
| 1% | 6 | 90 | `apps/server/src/device/deviceLogs.ts` | `apps/web/src/components/ChatView.logic.ts` |
| 1% | 8 | 221 | `apps/server/src/device/wsDeviceHandlers.ts` | `apps/server/src/device/DeviceService.ts` |
| 1% | 13 | 735 | `apps/server/src/persistence/Layers/ManagedAttachments.test.ts` | `apps/desktop/src/snapShot/DesktopSnapShot.test.ts` |
| 1% | 7 | 148 | `apps/web/src/components/chat/WorkspaceFileDiffEditorPane.tsx` | `apps/web/src/components/DiffPanel.tsx` |
| 1% | 14 | 360 | `scripts/canary.ts` | `scripts/mobile-showcase.ts` |
| 1% | 39 | 2636 | `apps/desktop/src/browserManager.ts` | `apps/desktop/src/preview/Manager.ts` |
| 1% | 10 | 877 | `apps/server/src/agentGateway/deviceTools.ts` | `apps/server/src/cloud/http.ts` |
| 1% | 13 | 390 | `apps/server/src/hostConnections/httpRoute.test.ts` | `apps/server/src/orchestration/Layers/OrchestrationEngine.test.ts` |
| 1% | 7 | 195 | `apps/server/src/remoteSessions/sessionRegistry.test.ts` | `packages/client-runtime/src/device/stream.test.ts` |
| 1% | 17 | 390 | `apps/server/src/workspace/Layers/WorkspaceFileSystem.ts` | `apps/server/src/vcs/GitVcsDriverCore.ts` |
| 1% | 20 | 228 | `apps/web/src/components/chat/ChatPaneKeepAlive.tsx` | `apps/mobile/src/features/threads/ThreadFeed.tsx` |
| 1% | 7 | 491 | `apps/api/cloudflare/worker.test.ts` | `apps/web/src/components/desktop/SnapShotCoordinator.test.ts` |
| 1% | 7 | 108 | `apps/server/src/automation/runEnvelope.test.ts` | `apps/server/src/orchestration/projector.test.ts` |
| 1% | 7 | 132 | `apps/server/src/orchestration/exportThreadArchive.test.ts` | `packages/client-runtime/src/state/threadReducer.test.ts` |
| 1% | 7 | 118 | `apps/web/src/lib/runningChatsQuitConfirmation.ts` | `apps/web/src/components/ProviderUpdateLaunchNotification.logic.ts` |
| 1% | 8 | 278 | `apps/desktop/src/resumableUpdateDownload.test.ts` | `apps/web/src/components/ChatView.logic.test.ts` |
| 1% | 11 | 675 | `apps/server/src/accountUsageReporter.ts` | `packages/client-runtime/src/state/vcsAction.ts` |
| 1% | 6 | 321 | `apps/server/src/localServerMonitor.test.ts` | `apps/web/src/components/chat/MessagesTimeline.test.tsx` |
| 1% | 8 | 412 | `apps/server/src/providerUsage/index.test.ts` | `apps/server/src/provider/Layers/ProviderRegistry.test.ts` |
| 1% | 10 | 281 | `apps/web/src/components/chat/group/groupSettingsDialog.logic.ts` | `apps/mobile/src/state/use-composer-drafts.ts` |
| 1% | 7 | 39 | `apps/web/src/lib/accountBilling.ts` | `apps/web/src/openVsxThemes.ts` |
| 1% | 9 | 72 | `packages/shared/src/pendingInteractions.ts` | `apps/web/src/session-logic.ts` |
| 1% | 6 | 169 | `apps/desktop/src/desktopUserDataProfile.ts` | `packages/client-runtime/src/relay/managedRelayState.ts` |
| 1% | 11 | 613 | `apps/server/src/git/Layers/codexTextGenerationIsolation.ts` | `apps/server/src/process/externalLauncher.ts` |
| 1% | 9 | 587 | `apps/server/src/provider/acp/DevinAcpSupport.test.ts` | `apps/server/src/orchestration/Layers/OrchestrationEngine.test.ts` |
| 1% | 5 | 117 | `apps/web/e2e/fixtures/visibleBrowserMain.ts` | `apps/desktop/src/window/DesktopWindow.ts` |
| 1% | 6 | 131 | `apps/web/src/components/chat/MessagesTimeline.segments.browser.tsx` | `apps/server/src/orchestration/projector.test.ts` |
| 1% | 13 | 247 | `apps/web/src/components/chat/agentActivity.logic.ts` | `apps/web/src/session-logic.ts` |
| 1% | 13 | 303 | `apps/web/src/components/chat/prepareChatSendWorkspace.ts` | `apps/server/src/serverRuntimeStartup.ts` |
| 1% | 27 | 578 | `apps/web/src/components/chat/project/ProjectPanel.tsx` | `apps/web/src/components/Sidebar.tsx` |
| 1% | 7 | 127 | `apps/web/src/lib/threadEnvironment.ts` | `apps/web/src/components/ChatView.logic.ts` |
| 1% | 5 | 124 | `apps/web/src/threadDetailPrewarm.ts` | `apps/web/src/components/Sidebar.logic.ts` |
| 1% | 7 | 86 | `apps/server/src/git/worktreeSetupStream.test.ts` | `apps/server/src/vcs/GitVcsDriver.test.ts` |
| 1% | 6 | 94 | `apps/web/src/components/ChatMarkdownFind.tsx` | `apps/mobile/modules/t3-markdown-text/src/nativeMarkdownText.ts` |
| 1% | 8 | 69 | `apps/web/src/components/chat/useChatWorkLog.browser.tsx` | `apps/server/src/relay/AgentAwarenessRelay.test.ts` |
| 1% | 7 | 53 | `apps/web/src/components/chat/useComposerEffortCycle.ts` | `apps/mobile/src/features/threads/new-task-flow-provider.tsx` |
| 1% | 5 | 49 | `apps/web/src/lib/threadDrag.ts` | `apps/web/src/components/pullRequest/pullRequestList.logic.ts` |
| 1% | 7 | 171 | `apps/web/src/lib/wsHttpUrl.ts` | `apps/desktop/src/snapShot/snapShot.ts` |
| 1% | 11 | 260 | `scripts/lib/mac-update-zip-finalize.ts` | `scripts/mobile-showcase.ts` |
| 1% | 7 | 178 | `apps/server/src/provider/claudeWorkflowScript.test.ts` | `packages/client-runtime/src/state/subagentRuntime.test.ts` |
| 1% | 5 | 445 | `apps/server/src/provider/skillsCatalog.test.ts` | `apps/server/src/provider/Layers/CursorProvider.test.ts` |
| 1% | 19 | 1337 | `apps/server/src/wsRpc.connectionLifecycle.test.ts` | `apps/server/src/auth/EnvironmentAuth.ts` |
| 1% | 10 | 70 | `apps/web/src/components/SegmentedPicker.tsx` | `apps/web/src/components/GitActionsControl.tsx` |
| 1% | 12 | 208 | `apps/web/src/components/chat/MessagesTimeline.worktreeSetup.browser.tsx` | `apps/web/src/components/chat/MessagesTimeline.test.tsx` |
| 1% | 8 | 378 | `apps/web/src/components/settings/providers/providerInstallSettings.tsx` | `apps/web/src/components/pullRequest/pullRequestList.logic.ts` |
| 1% | 5 | 139 | `apps/desktop/src/browserAutomation/betterwrightKeyboardPolicy.ts` | `apps/web/src/terminal/ghostty/keyCodes.ts` |
| 1% | 15 | 711 | `apps/desktop/src/productAnalytics.ts` | `apps/web/src/terminal/ghostty/surface.ts` |
| 1% | 10 | 151 | `apps/e2e/src/harness/remoteMcp.ts` | `apps/server/src/orchestration/Layers/OrchestrationEngine.test.ts` |
| 1% | 8 | 58 | `apps/server/src/computer/computerSpaceDesignation.test.ts` | `apps/web/src/components/chat/MessagesTimeline.test.tsx` |
| 1% | 15 | 717 | `apps/server/src/externalMcp/Layers/ExternalMcpGateway.ts` | `apps/server/src/orchestration/Layers/ProviderRuntimeIngestion.ts` |
| 1% | 19 | 720 | `apps/server/src/externalMcp/bridge.ts` | `apps/web/src/openVsxThemes.ts` |
| 1% | 10 | 379 | `apps/server/src/recapStats.ts` | `apps/server/src/orchestration/Layers/ProjectionPipeline.ts` |
| 1% | 5 | 45 | `apps/web/src/components/account/GlobalAccountDialogs.tsx` | `apps/web/src/components/DiffPanel.tsx` |
| 1% | 7 | 216 | `apps/web/src/components/chat/timelineRowHeight.logic.ts` | `apps/web/src/terminal/ghostty/surface.ts` |
| 1% | 55 | 237 | `apps/web/src/components/chat/useChatRuntimeModes.ts` | `apps/web/src/components/ChatView.tsx` |
| 1% | 11 | 206 | `apps/web/src/components/chat/useSubagentStripSource.ts` | `apps/server/src/orchestration/Layers/ProjectionPipeline.ts` |
| 1% | 9 | 163 | `apps/web/src/components/settings/ConnectionsOtherComputers.tsx` | `apps/web/src/components/settings/KeybindingsSettings.tsx` |
| 1% | 19 | 393 | `apps/web/src/lib/draftThreadDispatch.ts` | `apps/web/src/composerDraftStore.ts` |
| 1% | 5 | 315 | `apps/web/src/lib/projectAppearance.ts` | `apps/web/src/components/settings/customModelEditor.logic.ts` |
| 1% | 7 | 118 | `apps/server/src/automation/Layers/AutomationScheduler.test.ts` | `apps/server/src/provider/Layers/GrokAdapter.test.ts` |
| 1% | 9 | 56 | `apps/server/src/orchestration/turnStartSession.test.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.test.ts` |
| 1% | 8 | 349 | `apps/server/src/providerUsage/providers/codex.ts` | `apps/desktop/src/snapShot/DesktopSnapShot.ts` |
| 1% | 5 | 230 | `apps/web/scripts/parallel-transform.test.ts` | `apps/mobile/src/lib/composerFiles.test.ts` |
| 1% | 6 | 44 | `apps/web/src/hooks/useWorkspaceFileEditorShortcuts.ts` | `apps/web/src/components/ThreadTerminalDrawer.tsx` |
| 1% | 14 | 1118 | `apps/web/src/theme/theme.logic.ts` | `apps/web/src/session-logic.ts` |
| 1% | 6 | 195 | `apps/server/src/device/uiTreeTargeting.ts` | `apps/desktop/src/snapShot/snapShot.ts` |
| 1% | 7 | 180 | `apps/server/src/orchestration/importedThreadMessages.test.ts` | `apps/server/src/orchestration/Layers/OrchestrationEngine.test.ts` |
| 1% | 6 | 136 | `apps/server/src/provider/piOpenCodeCatalog.ts` | `apps/mobile/src/components/ComposerEditor.tsx` |
| 1% | 7 | 191 | `apps/server/src/wsSnapshotLiveStream.ts` | `apps/server/src/provider/Layers/CodexCollabRuntime.integration.test.ts` |
| 1% | 9 | 143 | `apps/web/src/components/chat/project/useProjectAgent.browser.tsx` | `apps/server/src/orchestration/ThreadSettlementReactor.test.ts` |
| 1% | 17 | 229 | `apps/web/src/hooks/useSidebarProjectRunController.ts` | `apps/web/src/components/CommandPalette.tsx` |
| 1% | 8 | 195 | `apps/web/src/lib/productAnalytics.ts` | `apps/desktop/src/snapShot/DesktopSnapShot.ts` |
| 1% | 18 | 208 | `apps/web/src/turnFailure.test.ts` | `apps/web/src/components/ChatView.logic.test.ts` |
| 1% | 7 | 54 | `packages/shared/src/turnResume.test.ts` | `apps/web/src/components/ChatView.logic.test.ts` |
| 1% | 15 | 187 | `apps/server/src/orchestration/Layers/ProjectionSnapshotQuery.tail.test.ts` | `apps/server/src/orchestration/Layers/ProjectionSnapshotQuery.test.ts` |
| 1% | 12 | 198 | `apps/server/src/studioGeneratedImages.test.ts` | `apps/server/src/orchestration/Layers/OrchestrationEngine.test.ts` |
| 1% | 6 | 176 | `apps/server/src/wsStreamAdmission.ts` | `packages/client-runtime/src/state/runtime.ts` |
| 1% | 9 | 116 | `apps/web/src/components/account/SignInOrbit.tsx` | `apps/web/src/components/RightPanelTabs.tsx` |
| 1% | 5 | 125 | `apps/web/src/components/chat/useChatProjectScripts.browser.tsx` | `apps/web/src/components/projectScriptEditor.tsx` |
| 1% | 12 | 570 | `apps/web/src/components/settings/accounts/AccountsSettingsPanel.browser.tsx` | `apps/web/src/components/ThreadTerminalDrawer.tsx` |
| 1% | 7 | 52 | `apps/web/src/hooks/useVisibleDiffFilePath.ts` | `apps/web/src/components/chat/AssistantCitationSource.tsx` |
| 1% | 7 | 260 | `apps/desktop/src/windowsTaskbarIcon.test.ts` | `packages/client-runtime/src/device/stream.test.ts` |
| 1% | 5 | 114 | `apps/server/src/agentGateway/completionRepository.ts` | `apps/server/src/serverSettings.ts` |
| 1% | 11 | 360 | `apps/server/src/hostConnections/registry.ts` | `apps/web/src/components/ChatView.logic.ts` |
| 1% | 11 | 217 | `apps/server/src/keepAwake.ts` | `apps/server/src/terminal/Manager.ts` |
| 1% | 11 | 100 | `apps/server/src/provider/providerRuntimeEventIngress.test.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.test.ts` |
| 1% | 7 | 127 | `apps/web/src/components/chat/ComputerSetupRequiredCard.tsx` | `apps/web/src/components/RightPanelTabs.tsx` |
| 1% | 7 | 274 | `packages/shared/src/platformProcess.test.ts` | `apps/server/src/provider/antigravityAuthSupport.test.ts` |
| 1% | 5 | 123 | `apps/server/src/__fixtures__/remoteDial.ts` | `apps/server/src/device/LocalDeviceHost.ts` |
| 1% | 14 | 231 | `apps/server/src/threadRetention.ts` | `apps/server/src/terminal/Manager.ts` |
| 1% | 13 | 124 | `apps/web/src/components/chat/MessagesTimeline.messageEnter.browser.tsx` | `apps/web/src/components/chat/MessagesTimeline.test.tsx` |
| 1% | 38 | 406 | `apps/web/src/components/chat/useChatProviderModels.ts` | `apps/web/src/components/chat/ChatComposer.tsx` |
| 1% | 5 | 92 | `apps/web/src/lib/fileComments.test.ts` | `apps/server/src/project/AgentSessionImporter.test.ts` |
| 1% | 7 | 199 | `apps/web/src/lib/hosts/workspaceSidebar.ts` | `apps/server/src/orchestration/ThreadPullRequestReactor.test.ts` |
| 1% | 18 | 360 | `apps/web/src/lib/kanbanDispatch.test.ts` | `apps/web/src/composerDraftStore.test.ts` |
| 1% | 6 | 282 | `packages/shared/src/browserAnnotations.ts` | `apps/server/src/provider/Layers/EventNdjsonLogger.ts` |
| 1% | 6 | 144 | `packages/shared/src/hostUsage.ts` | `apps/server/src/vcs/GitVcsDriver.ts` |
| 1% | 5 | 122 | `apps/desktop/src/browserAutomation/browserVaultCapture.runtime.test.ts` | `apps/desktop/src/preview/FaviconCapture.test.ts` |
| 1% | 17 | 2103 | `apps/server/src/computer/CuaComputerBackend.ts` | `apps/web/src/composerDraftStore.ts` |
| 1% | 7 | 101 | `apps/web/src/components/ProjectSidebarIcon.tsx` | `apps/web/src/components/ThreadTerminalDrawer.tsx` |
| 1% | 5 | 38 | `apps/web/src/components/WorkspaceSearchPalette.logic.ts` | `apps/web/src/components/ChatView.logic.ts` |
| 1% | 9 | 472 | `apps/web/src/lib/serverReactQuery.test.ts` | `apps/desktop/src/preview/FaviconCapture.test.ts` |
| 1% | 17 | 848 | `apps/server/src/mcpConnectors/runtime.ts` | `apps/server/src/provider/Layers/ClaudeAdapter.ts` |
| 1% | 8 | 50 | `apps/web/src/components/SiteFavicon.tsx` | `apps/mobile/src/features/threads/NewTaskDraftScreen.tsx` |
| 1% | 20 | 490 | `apps/web/src/components/chat/ChatTranscriptPane.tsx` | `apps/web/src/components/ChatMarkdown.tsx` |
| 1% | 8 | 212 | `apps/web/src/components/inbox/inbox.logic.ts` | `apps/web/src/components/Sidebar.logic.ts` |
| 1% | 9 | 316 | `apps/web/src/rightDockStore.logic.ts` | `apps/mobile/src/lib/threadActivity.ts` |
| 1% | 7 | 563 | `apps/desktop/src/cuaFixtures/electron.ts` | `apps/mobile/src/features/review/shikiReviewHighlighter.ts` |
| 1% | 8 | 84 | `apps/server/src/checkpointing/Utils.test.ts` | `apps/web/src/composerDraftStore.test.ts` |
| 1% | 5 | 124 | `apps/server/src/computer/UnavailableComputerBackend.ts` | `apps/server/src/pullRequest/BitbucketPullRequestApi.ts` |
| 1% | 5 | 228 | `scripts/check-brand-identity.ts` | `apps/mobile/src/features/files/SourceFileSurface.tsx` |
| 1% | 5 | 70 | `apps/desktop/src/browserAnnotations/guestProtocol.ts` | `apps/server/src/provider/acp/XAiAcpExtension.ts` |
| 1% | 9 | 238 | `apps/server/src/agentGateway/completionDelivery.test.ts` | `apps/server/src/orchestration/ThreadSettlementReactor.test.ts` |
| 1% | 7 | 172 | `apps/server/src/mcpConnectors/proxy.test.ts` | `apps/server/src/provider/AntigravityInstallation.test.ts` |
| 1% | 43 | 4250 | `apps/server/src/projectAgent/Layers/ProjectAgentService.test.ts` | `apps/server/src/orchestration/Layers/ProjectionPipeline.test.ts` |
| 1% | 7 | 204 | `apps/server/src/providerUsage/codexResetCredits.ts` | `apps/server/src/vcs/VcsStatusBroadcaster.ts` |
| 1% | 5 | 87 | `apps/web/src/hooks/useDockPaneRuntimeActivation.ts` | `apps/mobile/src/features/threads/ThreadDetailScreen.tsx` |
| 1% | 5 | 77 | `apps/web/src/lib/pdf/usePdfPageNavigation.ts` | `apps/web/src/hooks/useTheme.ts` |
| 1% | 10 | 821 | `apps/api/src/billing/service.test.ts` | `apps/web/src/components/CommandPalette.logic.test.ts` |
| 1% | 10 | 395 | `apps/desktop/src/browserAnnotations/coordinator.ts` | `apps/server/src/provider/Layers/GrokAdapter.ts` |
| 1% | 7 | 131 | `apps/server/src/provider/continuationIdentity.ts` | `apps/server/src/provider/opencodeRuntime.ts` |
| 1% | 5 | 16 | `apps/server/src/provider/projectImportTypes.ts` | `apps/server/src/pullRequest/PullRequestProvider.ts` |
| 1% | 8 | 131 | `apps/server/src/provider/providerAttachmentPaths.ts` | `apps/server/src/provider/opencodeRuntime.ts` |
| 1% | 7 | 141 | `apps/server/src/provider/threadMentionContext.test.ts` | `apps/server/src/project/AgentSessionImporter.test.ts` |
| 1% | 88 | 12199 | `apps/web/src/components/ChatView.browser.tsx` | `apps/mobile/src/lib/threadActivity.test.ts` |
| 1% | 8 | 257 | `apps/web/src/components/DiffPanel.logic.ts` | `apps/web/src/components/ChatView.logic.ts` |
| 1% | 8 | 99 | `apps/web/src/components/chat/InlineVisualCalculator.tsx` | `apps/web/src/components/settings/KeybindingsSettings.tsx` |
| 1% | 6 | 294 | `apps/web/src/settingsSearchIndex.ts` | `apps/web/src/openVsxThemes.ts` |
| 1% | 12 | 260 | `apps/web/src/hooks/useRecentViewSwitcher.ts` | `apps/web/src/components/CommandPalette.tsx` |
| 1% | 5 | 499 | `packages/shared/src/account.test.ts` | `apps/server/src/pullRequest/BitbucketPullRequestApi.ts` |
| 1% | 9 | 267 | `scripts/computer-use-fixtures/measurement.test.ts` | `apps/server/src/orchestration/projector.test.ts` |
| 1% | 5 | 242 | `apps/desktop/src/windowsShellAppUserModel.ts` | `apps/desktop/scripts/electron-launcher.mjs` |
| 1% | 10 | 771 | `apps/server/src/computer/FakeComputerBackend.ts` | `apps/web/src/composerDraftStore.ts` |
| 1% | 8 | 154 | `apps/server/src/provider/Layers/PiOpenRouter.test.ts` | `apps/server/src/project/AgentSessionImporter.test.ts` |
| 1% | 5 | 139 | `apps/web/src/components/chat/SplitPaneLayout.browser.tsx` | `packages/client-runtime/src/device/stream.test.ts` |
| 1% | 20 | 565 | `apps/web/src/components/chat/TraitsPicker.browser.tsx` | `apps/web/src/composerDraftStore.ts` |
| 1% | 7 | 124 | `apps/web/src/components/chat/useThreadErrorToast.ts` | `apps/web/src/components/onboarding/WelcomeWizard.tsx` |
| 1% | 8 | 92 | `apps/web/src/lib/devicePromptContext.ts` | `apps/web/src/components/ChatView.logic.ts` |
| 1% | 5 | 121 | `apps/web/src/lib/providerUsageDisplay.ts` | `apps/server/src/orchestration/projector.ts` |
| 1% | 10 | 243 | `apps/server/src/providerUsage/providers/droid.ts` | `apps/mobile/src/lib/threadActivity.ts` |
| 1% | 8 | 328 | `apps/server/src/remoteSessions/gateway.test.ts` | `packages/client-runtime/src/device/stream.test.ts` |
| 1% | 7 | 136 | `apps/server/src/remoteSessions/sessionRegistry.ts` | `apps/web/src/components/pullRequest/pullRequestDetail.logic.ts` |
| 1% | 9 | 98 | `apps/web/src/components/automation/AutomationHeadingFields.tsx` | `apps/web/src/components/GitActionsControl.tsx` |
| 1% | 14 | 296 | `apps/web/src/components/tasks/useTaskDelegation.ts` | `apps/web/src/components/CommandPalette.tsx` |
| 1% | 12 | 456 | `apps/web/src/storeSelectors.ts` | `apps/web/src/session-logic.ts` |
| 1% | 7 | 346 | `apps/desktop/src/browserAutomation/workspaceUpload.test.ts` | `infra/relay/src/http/Api.test.ts` |
| 1% | 16 | 953 | `apps/server/src/agentGateway/kanbanTools.ts` | `apps/server/src/orchestration/Layers/ProviderCommandReactor.ts` |
| 1% | 6 | 58 | `apps/server/src/computer/computerSetupSignal.ts` | `apps/server/src/provider/Layers/CursorProvider.ts` |
| 1% | 21 | 886 | `apps/server/src/githubInbox/Layers/GitHubInboxService.test.ts` | `apps/server/src/orchestration/ThreadSettlementReactor.test.ts` |
| 1% | 12 | 65 | `apps/web/src/components/InlineLinkChip.tsx` | `apps/web/src/components/pullRequest/PullRequestDetailPanel.tsx` |
| 1% | 15 | 270 | `apps/web/src/components/chat/SubagentRunCard.browser.tsx` | `apps/web/src/components/chat/MessagesTimeline.test.tsx` |
| 1% | 5 | 43 | `apps/web/src/components/chat/environment/EnvironmentUsageSection.logic.ts` | `apps/web/src/components/ProviderUpdateLaunchNotification.logic.ts` |
| 1% | 13 | 389 | `apps/web/src/lib/sidechatCreation.test.ts` | `packages/client-runtime/src/state/threadReducer.test.ts` |
| 1% | 7 | 136 | `packages/shared/src/parentBoundProcess.ts` | `packages/client-runtime/src/device/stream.ts` |
| 1% | 5 | 152 | `apps/desktop/src/browserAutomation/screenshotCapture.test.ts` | `apps/web/src/openVsxThemes.ts` |
| 1% | 26 | 3112 | `apps/server/src/agentGateway/computerTools.ts` | `apps/web/src/composerDraftStore.ts` |
| 1% | 16 | 465 | `apps/web/src/components/BrowserPanel.logic.ts` | `apps/mobile/src/lib/threadActivity.ts` |
| 1% | 6 | 97 | `apps/web/src/storeSnooze.test.ts` | `apps/server/src/orchestration/ThreadPullRequestReactor.test.ts` |
| 1% | 5 | 191 | `scripts/check-migration-lineage.ts` | `scripts/lib/third-party-licenses.ts` |
| 1% | 7 | 124 | `apps/desktop/src/resumableUpdateDownloadPolicy.ts` | `apps/web/src/session-logic.ts` |
| 1% | 6 | 342 | `apps/web/src/components/chat/group/GroupSettingsDialog.browser.tsx` | `apps/web/src/components/CommandPalette.logic.test.ts` |
| 1% | 8 | 54 | `apps/web/src/components/chat/useLiveThreadModelSelection.ts` | `apps/web/src/components/BranchToolbarBranchSelector.tsx` |
| 1% | 5 | 126 | `scripts/computer-use-fixtures/belief-canary.mjs` | `apps/web/src/session-logic.ts` |
| 1% | 10 | 131 | `scripts/lib/mac-notarization.ts` | `scripts/mobile-showcase.ts` |
| 1% | 9 | 930 | `apps/server/src/device/DeviceManager.test.ts` | `packages/client-runtime/src/device/stream.test.ts` |
| 1% | 8 | 248 | `apps/server/src/processRunner.ts` | `apps/desktop/src/preview/FaviconCapture.ts` |
| 1% | 10 | 187 | `apps/web/src/components/chat/ComposerPendingApprovalPanel.browser.tsx` | `apps/server/src/provider/Layers/CodexSessionRuntime.ts` |
| 1% | 7 | 80 | `apps/web/src/components/tasks/TaskTextFields.tsx` | `apps/web/src/components/RightPanelTabs.tsx` |
| 1% | 6 | 110 | `packages/contracts/src/accountUsage.ts` | `packages/contracts/src/previewAutomation.ts` |
| 1% | 6 | 356 | `scripts/computer-use-fixtures/canary-main.ts` | `apps/web/src/openVsxThemes.ts` |
| 1% | 14 | 742 | `apps/server/src/agentGateway/automationTools.ts` | `apps/server/src/provider/Layers/ProviderService.ts` |
| 1% | 5 | 47 | `apps/server/src/provider/acp/SessionTeardownGate.test.ts` | `apps/desktop/src/backend/DesktopBackendManager.test.ts` |
| 1% | 6 | 278 | `apps/api/src/billing/polar.ts` | `apps/mobile/src/state/use-thread-outbox-drain.ts` |
| 1% | 9 | 258 | `apps/desktop/src/browserAutomation/browserVault.ts` | `apps/desktop/src/snapShot/DesktopSnapShot.ts` |
| 1% | 8 | 243 | `apps/server/src/agentGateway/todoTools.ts` | `apps/server/src/orchestration/ThreadSettlementReactor.test.ts` |
| 1% | 9 | 279 | `apps/server/src/betaImport.ts` | `apps/mobile/src/state/use-thread-outbox-drain.ts` |
| 1% | 73 | 5210 | `apps/server/src/projectAgent/Layers/ProjectAgentService.ts` | `apps/server/src/orchestration/Layers/ProviderCommandReactor.ts` |
| 1% | 7 | 140 | `apps/web/src/components/settings/ConversationStorageSettingsPanels.browser.tsx` | `apps/server/src/orchestration/decider.pullRequests.test.ts` |
| 1% | 5 | 26 | `apps/web/src/lib/claudeArtifactCommands.ts` | `apps/server/src/provider/opencodeRuntime.ts` |
| 1% | 7 | 59 | `apps/web/src/pendingUserInputRecovery.ts` | `apps/mobile/src/state/use-composer-drafts.ts` |
| 1% | 18 | 397 | `apps/server/src/agentGateway/hubWorkGateway.test.ts` | `apps/server/src/provider/Layers/ProviderService.test.ts` |
| 1% | 6 | 52 | `apps/server/src/automation/proposalActivity.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.ts` |
| 1% | 23 | 2089 | `apps/server/src/codexProcessEnv.ts` | `apps/web/src/composerDraftStore.ts` |
| 1% | 6 | 320 | `apps/server/src/computer/ComputerBackend.ts` | `apps/desktop/src/snapShot/snapShot.ts` |
| 1% | 5 | 71 | `apps/server/src/platform/protocolProcessTeardown.ts` | `apps/desktop/src/snapShot/DesktopSnapShot.ts` |
| 1% | 7 | 66 | `apps/server/src/provider/acp/AcpTurnIdleWatchdog.ts` | `apps/server/src/provider/acp/AcpSessionRuntime.ts` |
| 1% | 10 | 128 | `apps/server/src/provider/providerLifecycleCoordinator.ts` | `apps/server/src/orchestration/Layers/ProviderCommandReactor.ts` |
| 1% | 10 | 538 | `apps/web/e2e/editorIntelligence.e2e.ts` | `apps/server/src/orchestration/Layers/OrchestrationEngine.test.ts` |
| 1% | 12 | 209 | `apps/web/src/lib/composerPrediction.ts` | `apps/web/src/components/pullRequest/pullRequestDetail.logic.ts` |
| 1% | 7 | 169 | `apps/web/src/lib/projectCreateRecovery.ts` | `packages/client-runtime/src/operations/projects.ts` |
| 1% | 17 | 513 | `scripts/computer-use-fixtures/packaged-browser-benchmark.ts` | `apps/web/src/composerDraftStore.ts` |
| 1% | 7 | 209 | `apps/desktop/src/browserAutomation/cdpRuntime.ts` | `apps/web/src/openVsxThemes.ts` |
| 1% | 5 | 41 | `apps/server/src/orchestration/threadLatestReplyPreview.ts` | `apps/server/src/provider/opencodeRuntime.ts` |
| 1% | 15 | 101 | `apps/web/src/components/chat/ComposerVoiceRecorderBar.tsx` | `apps/web/src/components/chat/MessagesTimeline.tsx` |
| 1% | 5 | 129 | `apps/web/src/lib/hosts/workspaceFrame.ts` | `apps/server/src/provider/Layers/CursorProvider.ts` |
| 1% | 9 | 840 | `apps/desktop/src/productAnalytics.test.ts` | `apps/web/src/browser/browserRecording.test.ts` |
| 1% | 15 | 1021 | `apps/server/src/accountAuth.ts` | `apps/desktop/src/preview/Manager.ts` |
| 1% | 8 | 426 | `apps/server/src/provider/openCodeV2Client.ts` | `packages/client-runtime/src/device/stream.ts` |
| 1% | 12 | 177 | `apps/web/perf/clientHotPath.perf.test.ts` | `apps/server/src/orchestration/Layers/OrchestrationEngine.test.ts` |
| 1% | 6 | 71 | `apps/web/src/components/chat/auxiliary/useAuxiliarySurface.browser.tsx` | `apps/web/src/components/onboarding/WelcomeWizard.tsx` |
| 1% | 6 | 536 | `apps/api/src/routes/v1.hostSecrets.test.ts` | `apps/mobile/src/state/thread-outbox.test.ts` |
| 1% | 10 | 273 | `apps/server/src/hostConnections/resourcePool.test.ts` | `infra/relay/src/http/Api.test.ts` |
| 1% | 16 | 186 | `apps/web/src/components/SafariAccessOnboarding.tsx` | `apps/web/src/components/ChatMarkdown.tsx` |
| 1% | 12 | 272 | `apps/web/src/components/chat/messageTrail.logic.ts` | `apps/mobile/src/lib/threadActivity.ts` |
| 1% | 5 | 77 | `apps/web/src/hooks/useStartGitHubItemThread.ts` | `apps/web/src/components/settings/ResourceTelemetryDiagnostics.tsx` |
| 1% | 5 | 42 | `packages/shared/src/projectContainers.ts` | `apps/mobile/src/features/agent-awareness/remoteRegistration.ts` |
| 1% | 5 | 147 | `scripts/lib/mac-dmg-finalize.ts` | `apps/server/src/sourceControl/GitHubCli.ts` |
| 1% | 13 | 815 | `apps/server/src/accountSession.ts` | `apps/web/src/components/ChatMarkdown.tsx` |
| 1% | 10 | 156 | `apps/web/src/lib/providerReactQuery.ts` | `apps/server/src/orchestration/Layers/ProviderRuntimeIngestion.ts` |
| 1% | 19 | 2118 | `apps/analytics/worker.ts` | `apps/web/src/openVsxThemes.ts` |
| 1% | 11 | 281 | `apps/server/src/agentGateway/browserTools.ts` | `apps/server/src/git/GitManager.ts` |
| 1% | 9 | 213 | `apps/server/src/codexGeneratedImages.test.ts` | `apps/server/src/provider/Layers/ProviderRegistry.test.ts` |
| 1% | 5 | 152 | `apps/server/src/remoteSessions/localRpcBridge.test.ts` | `packages/client-runtime/src/rpc/session.test.ts` |
| 1% | 9 | 110 | `apps/web/src/components/BetaWelcomeDialog.tsx` | `apps/web/src/routes/_chat.pull-requests.tsx` |
| 1% | 12 | 226 | `apps/web/src/components/kanban/useKanbanBoard.ts` | `apps/web/src/components/CommandPalette.tsx` |
| 1% | 11 | 127 | `apps/web/src/components/ui/slider.tsx` | `apps/web/src/components/pullRequest/PullRequestDetailPanel.tsx` |
| 1% | 6 | 113 | `apps/web/src/feedback.test.ts` | `apps/server/src/project/AgentSessionImporter.test.ts` |
| 1% | 5 | 74 | `apps/web/src/lib/modelFavorites.ts` | `apps/web/src/components/pullRequest/pullRequestList.logic.ts` |
| 1% | 17 | 681 | `scripts/computer-use-fixtures/packaged-e2e.ts` | `apps/web/src/composerDraftStore.ts` |
| 1% | 5 | 37 | `scripts/patch-workspace-typescript.ts` | `apps/web/src/browser/browserRecording.ts` |
| 1% | 5 | 106 | `apps/desktop/src/browserAutomation/betterwrightHostTarget.ts` | `apps/web/src/hooks/useSettings.ts` |
| 1% | 24 | 3930 | `apps/server/src/agentGateway/computerTools.test.ts` | `apps/desktop/src/snapShot/DesktopSnapShot.test.ts` |
| 1% | 10 | 530 | `apps/server/src/cloudflare/authorization.test.ts` | `apps/desktop/src/preview/FaviconCapture.test.ts` |
| 1% | 8 | 317 | `apps/server/src/platform/processTreeController.ts` | `packages/shared/src/shell.ts` |
| 1% | 10 | 222 | `apps/web/src/components/browser/useBrowserAnnotations.ts` | `apps/web/src/components/chat/ChatComposer.tsx` |
| 1% | 10 | 104 | `apps/web/src/components/chat/MessagesTimeline.changedFiles.browser.tsx` | `apps/web/src/components/chat/MessagesTimeline.test.tsx` |
| 1% | 6 | 328 | `apps/web/src/components/chat/MessagesTimeline.tailAnchor.browser.tsx` | `apps/desktop/src/preview/FaviconCapture.ts` |
| 1% | 8 | 111 | `apps/web/src/lib/selectionChat.test.ts` | `apps/web/src/components/ChatView.logic.test.ts` |
| 1% | 13 | 343 | `apps/server/src/agentGateway/mcpTransport.ts` | `apps/server/src/provider/Layers/OpenCodeAdapter.ts` |
| 1% | 12 | 155 | `apps/server/src/agentGateway/sessionLease.ts` | `apps/server/src/provider/Layers/ProviderService.ts` |
| 1% | 5 | 350 | `apps/desktop/native/appsnap/AppSnapProtocol.swift` | `apps/mobile/modules/t3-terminal/ios/T3TerminalView.swift` |
| 1% | 5 | 157 | `apps/desktop/scripts/build-appsnap-helper.mjs` | `apps/mobile/src/lib/composerImages.ts` |
| 1% | 5 | 166 | `scripts/lib/mac-icon-catalog.test.ts` | `apps/server/src/cloud/http.test.ts` |
| 1% | 27 | 557 | `apps/desktop/src/backendShutdown.windows.integration.test.ts` | `scripts/mobile-showcase.ts` |
| 1% | 12 | 233 | `apps/server/src/hostUsageSnapshot.ts` | `apps/server/src/terminal/Manager.ts` |
| 1% | 16 | 1093 | `apps/server/src/persistence/MigrationBackup.ts` | `apps/desktop/src/snapShot/DesktopSnapShot.ts` |
| 1% | 13 | 412 | `apps/web/src/components/SidebarGroupsSurface.browser.tsx` | `apps/mobile/src/lib/threadActivity.test.ts` |
| 1% | 6 | 34 | `apps/web/src/components/ThreadArchiveActionButton.tsx` | `apps/web/src/components/ThreadTerminalDrawer.tsx` |
| 1% | 5 | 96 | `apps/desktop/src/backendSupervisionPolicy.ts` | `packages/shared/src/composerContextLegacy.ts` |
| 1% | 10 | 232 | `apps/desktop/src/browserAutomation/dialogHandling.ts` | `apps/desktop/src/snapShot/DesktopSnapShot.ts` |
| 1% | 6 | 384 | `apps/server/src/agentGateway/computerTools.menusFrames.test.ts` | `apps/server/src/project/AgentSessionImporter.test.ts` |
| 1% | 10 | 224 | `apps/server/src/device/deviceFrameRoute.test.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.test.ts` |
| 1% | 7 | 200 | `apps/server/src/orchestration/projectImportCatalog.ts` | `apps/mobile/src/lib/threadActivity.ts` |
| 1% | 7 | 470 | `apps/web/src/components/chat/AsciiWaveBackground.tsx` | `apps/web/src/components/RightPanelTabs.tsx` |
| 1% | 5 | 226 | `apps/web/src/lib/automationDraft.test.ts` | `apps/server/src/orchestration/Layers/ProviderRuntimeIngestion.test.ts` |
| 1% | 8 | 269 | `apps/web/src/lib/providerReactQuery.test.ts` | `apps/web/src/composerDraftStore.test.ts` |
| 1% | 5 | 194 | `apps/server/src/provider/claudeTokenUsage.test.ts` | `apps/web/src/components/Sidebar.logic.test.ts` |
| 1% | 9 | 241 | `apps/web/src/components/tasks/useTodos.ts` | `apps/mobile/src/state/use-composer-drafts.ts` |
| 1% | 9 | 369 | `apps/web/src/lib/rateLimits.ts` | `apps/web/src/session-logic.ts` |
| 1% | 7 | 100 | `packages/shared/src/codexStderr.ts` | `apps/server/src/provider/Layers/CodexSessionRuntime.ts` |
| 1% | 5 | 120 | `packages/shared/src/processRuntime.test.ts` | `apps/web/src/terminal/ghostty/surface.test.ts` |
| 1% | 7 | 214 | `scripts/lib/release-update-policy.ts` | `apps/web/src/themePalette.ts` |
| 1% | 7 | 262 | `apps/server/src/computer/computerAuditLog.test.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.test.ts` |
| 1% | 7 | 235 | `apps/server/src/eventLoopMonitor.ts` | `apps/web/src/components/preview/PreviewAutomationHosts.tsx` |
| 1% | 7 | 710 | `apps/server/src/mcpConnectors/runtime.test.ts` | `apps/web/src/lib/attachmentUploadQueue.test.ts` |
| 1% | 6 | 132 | `scripts/computer-use-fixtures/packaged-client.ts` | `apps/web/src/components/ChatView.logic.ts` |
| 1% | 7 | 49 | `apps/web/src/lib/diffScrollSurface.ts` | `apps/mobile/src/state/use-composer-drafts.ts` |
| 1% | 14 | 507 | `apps/server/src/providerUsage/providers/claude.ts` | `apps/web/src/components/ChatMarkdown.tsx` |
| 1% | 12 | 1029 | `apps/web/e2e/visibleBrowserMcp.e2e.ts` | `apps/mobile/src/lib/threadActivity.test.ts` |
| 1% | 8 | 266 | `apps/web/src/components/chat/useChatTranscriptScroll.browser.tsx` | `apps/web/src/components/chat/MessagesTimeline.test.tsx` |
| 1% | 7 | 193 | `apps/web/src/components/chat/useChatWorkLog.ts` | `apps/web/src/components/chat/MessagesTimeline.logic.ts` |
| 1% | 5 | 39 | `apps/web/src/lib/providerAccountSummary.test.ts` | `apps/web/src/modelSelection.test.ts` |
| 1% | 7 | 167 | `apps/server/src/persistence/Services/OrchestrationEventDeliveries.ts` | `packages/contracts/src/orchestration.ts` |
| 1% | 7 | 97 | `apps/web/src/components/chat/useChatRuntimeModes.browser.tsx` | `apps/server/src/orchestration/Layers/CheckpointReactor.test.ts` |
| 1% | 16 | 801 | `apps/web/src/components/chat/useChatTurnExecution.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.test.ts` |
| 1% | 6 | 144 | `apps/web/src/onboarding/logic.ts` | `apps/web/src/components/pullRequest/pullRequestDetail.logic.ts` |
| 1% | 7 | 369 | `scripts/computer-use-fixtures/measurement.ts` | `apps/server/src/orchestration/projector.ts` |
| 1% | 14 | 1750 | `apps/api/src/routes/v1.ts` | `apps/mobile/src/state/use-composer-drafts.ts` |
| 1% | 7 | 369 | `apps/server/src/agentGateway/browserTools.test.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.test.ts` |
| 1% | 6 | 56 | `apps/server/src/orchestration/spaceMetadataProjection.ts` | `apps/server/src/orchestration/Layers/ProjectionPipeline.ts` |
| 1% | 5 | 28 | `apps/server/src/serverLayers.test.ts` | `apps/server/src/provider/Layers/CursorAdapter.test.ts` |
| 1% | 5 | 32 | `apps/server/src/terminal/terminalThreadTitleTracker.ts` | `apps/mobile/src/features/threads/new-task-flow-provider.tsx` |
| 1% | 16 | 481 | `packages/shared/src/providerInstances.ts` | `apps/web/src/composerDraftStore.ts` |
| 1% | 17 | 702 | `apps/server/src/localServerMonitor.ts` | `apps/web/src/session-logic.ts` |
| 1% | 6 | 155 | `apps/web/src/components/chat/useTailAnchorScroll.browser.tsx` | `packages/client-runtime/src/state/threadReducer.test.ts` |
| 1% | 16 | 210 | `apps/web/src/hooks/useThreadActivationController.ts` | `apps/web/src/components/LegacySidebar.tsx` |
| 1% | 6 | 195 | `apps/server/src/agentGateway/stdioProxyScript.ts` | `apps/server/src/project/AgentSessionScanner.ts` |
| 1% | 7 | 190 | `apps/server/src/codexNestedOverlay.test.ts` | `apps/web/src/terminal/ghostty/surface.test.ts` |
| 1% | 8 | 390 | `apps/server/src/computer/uiTreeTargeting.ts` | `apps/web/src/components/RightPanelTabs.tsx` |
| 1% | 6 | 85 | `apps/server/src/device/deviceTypeCatalogue.ts` | `apps/server/src/vcs/GitVcsDriver.ts` |
| 1% | 8 | 622 | `apps/server/src/providerUsage/providers/claude.test.ts` | `apps/mobile/src/features/agent-awareness/remoteRegistration.test.ts` |
| 1% | 8 | 97 | `apps/web/src/components/chat/useSubagentStripSource.test.tsx` | `apps/web/src/session-logic.test.ts` |
| 1% | 12 | 83 | `apps/web/src/components/settings/AppIconPicker.tsx` | `apps/web/src/components/settings/SettingsPanels.tsx` |
| 1% | 7 | 193 | `packages/shared/src/migrationRecovery.ts` | `apps/server/src/project/AgentSessionScanner.ts` |
| 1% | 5 | 163 | `apps/server/src/agentGateway/Layers/AgentGatewayCredentials.ts` | `apps/server/src/provider/Layers/AntigravityAdapter.test.ts` |
| 1% | 7 | 168 | `apps/web/scripts/settingsSearchRows.ts` | `apps/mobile/src/lib/threadActivity.ts` |
| 1% | 7 | 32 | `packages/shared/src/sidechatThread.test.ts` | `apps/server/src/pullRequest/gitHubPullRequestJson.test.ts` |
| 1% | 11 | 224 | `apps/server/src/computer/computerFrameRoute.test.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.test.ts` |
| 1% | 9 | 203 | `apps/server/src/orchestration/handoff.ts` | `apps/mobile/src/lib/threadActivity.ts` |
| 1% | 25 | 404 | `apps/server/src/project/githubProjectProvisioning.ts` | `apps/server/src/vcs/GitVcsDriverCore.ts` |
| 1% | 6 | 224 | `apps/server/src/terminal/providerAuthentication.test.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.test.ts` |
| 1% | 12 | 119 | `apps/web/src/components/RecentViewSwitcher.tsx` | `apps/web/src/components/Sidebar.tsx` |
| 1% | 17 | 460 | `apps/server/src/agentGateway/threadReadTools.ts` | `apps/server/src/ws.ts` |
| 1% | 24 | 3167 | `apps/server/src/computer/ComputerManager.ts` | `apps/desktop/src/snapShot/DesktopSnapShot.ts` |
| 1% | 6 | 598 | `apps/server/src/hostSecrets/coordinator.test.ts` | `apps/desktop/src/app/DesktopAppActivation.test.ts` |
| 1% | 16 | 446 | `apps/server/src/managedWorktrees.ts` | `apps/server/src/ws.ts` |
| 1% | 5 | 84 | `apps/server/src/memoryDiagnostics.ts` | `apps/web/src/terminal/ghostty/surface.ts` |
| 1% | 6 | 156 | `apps/server/src/orchestration/exportThreadArchive.ts` | `apps/web/src/openVsxThemes.ts` |
| 1% | 13 | 1087 | `apps/server/src/provider/Layers/PiAdapter.test.ts` | `apps/server/src/cloud/http.test.ts` |
| 1% | 5 | 209 | `apps/server/src/remoteTransport/certificates.ts` | `apps/mobile/src/features/threads/ThreadRouteScreen.tsx` |
| 1% | 13 | 711 | `apps/web/src/components/chat/ProviderModelPicker.browser.tsx` | `apps/web/src/components/ChatView.logic.test.ts` |
| 1% | 9 | 471 | `apps/web/src/components/composer-nodes/index.tsx` | `apps/mobile/src/lib/threadActivity.ts` |
| 1% | 6 | 120 | `apps/web/src/lib/composerDropPaths.ts` | `apps/desktop/src/preview/PickPreload.ts` |
| 1% | 9 | 218 | `apps/web/src/lib/glassOverlayCutout.ts` | `apps/web/src/components/chat/AssistantCitationSource.tsx` |
| 1% | 5 | 693 | `packages/shared/src/hostSecrets.test.ts` | `apps/desktop/src/updates/DesktopRemoteUpdates.ts` |
| 1% | 11 | 1176 | `apps/server/src/accountSession.test.ts` | `packages/client-runtime/src/device/stream.test.ts` |
| 1% | 15 | 111 | `apps/web/src/components/DesktopWindowControls.tsx` | `apps/web/src/components/chat/MessagesTimeline.tsx` |
| 1% | 9 | 183 | `apps/web/src/components/chat/messageTrail.logic.test.ts` | `apps/web/src/components/chat/MessagesTimeline.test.tsx` |
| 1% | 13 | 148 | `apps/web/src/cursorModelVariants.ts` | `apps/server/src/provider/Layers/CursorProvider.ts` |
| 1% | 5 | 74 | `apps/web/src/pendingUserInputRecovery.test.ts` | `apps/server/src/orchestration/Layers/ProviderCommandReactor.ts` |
| 1% | 6 | 232 | `apps/desktop/src/browserAutomation/betterwrightFailure.ts` | `apps/desktop/src/snapShot/DesktopSnapShot.ts` |
| 1% | 12 | 88 | `apps/server/src/agentGateway/hubWorkSource.ts` | `apps/server/src/orchestration/Layers/ProviderCommandReactor.ts` |
| 1% | 14 | 317 | `apps/server/src/codexSessionInspection.test.ts` | `apps/server/src/provider/Layers/CodexAdapter.test.ts` |
| 1% | 7 | 242 | `apps/server/src/editor/debugService.test.ts` | `packages/client-runtime/src/device/stream.ts` |
| 1% | 10 | 133 | `apps/web/src/components/RailAutomationsPanel.tsx` | `apps/web/src/components/pullRequest/PullRequestDetailPanel.tsx` |
| 1% | 21 | 264 | `apps/web/src/lib/sidechatCreation.ts` | `apps/web/src/composerDraftStore.ts` |
| 1% | 27 | 1751 | `apps/desktop/src/browserAutomation/browserManagerAutomation.test.ts` | `apps/desktop/src/preview/Manager.test.ts` |
| 1% | 9 | 1530 | `apps/server/src/accountAuth.test.ts` | `apps/mobile/src/lib/attachmentUpload.test.ts` |
| 1% | 10 | 246 | `apps/server/src/codexSessionStartup.test.ts` | `apps/desktop/src/snapShot/snapShot.test.ts` |
| 1% | 11 | 259 | `apps/web/src/hooks/useClaudeContextCompaction.browser.tsx` | `apps/mobile/src/lib/threadActivity.test.ts` |
| 1% | 6 | 46 | `apps/web/src/lib/terminalCloseConfirmation.ts` | `apps/web/src/components/ChatView.logic.ts` |
| 1% | 21 | 1127 | `apps/server/src/provider/Layers/PiAdapter.lifecycle.test.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.test.ts` |
| 1% | 5 | 79 | `apps/server/src/provider/OmpSessionHistory.test.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.test.ts` |
| 1% | 12 | 107 | `apps/web/src/components/chat/group/HubPendingApprovals.tsx` | `apps/web/src/components/settings/ConnectionsSettings.tsx` |
| 1% | 8 | 54 | `apps/web/src/lib/threadRename.test.ts` | `apps/web/src/components/ChatView.logic.test.ts` |
| 1% | 14 | 344 | `apps/desktop/src/backendShutdown.ts` | `apps/web/src/components/ChatView.logic.ts` |
| 1% | 5 | 68 | `apps/server/src/orchestration/threadGoalMaterialization.ts` | `apps/web/src/components/ChatView.logic.ts` |
| 1% | 12 | 341 | `apps/web/src/components/ThreadTerminalPanel.browser.tsx` | `apps/server/src/orchestration/ThreadSettlementReactor.test.ts` |
| 1% | 6 | 90 | `apps/web/src/lib/browserPromptContext.ts` | `apps/web/src/components/ChatView.logic.ts` |
| 1% | 8 | 194 | `apps/web/src/openThreadTabs.logic.ts` | `apps/web/src/composerDraftStore.ts` |
| 1% | 7 | 153 | `apps/server/src/persistence/SqliteCompaction.ts` | `apps/server/src/provider/Layers/ProviderService.ts` |
| 1% | 7 | 135 | `apps/server/src/provider/claudeMonitorEvents.ts` | `apps/mobile/src/lib/threadActivity.ts` |
| 1% | 6 | 369 | `apps/server/src/providerUsage/providers/antigravity.ts` | `apps/server/src/project/AgentSessionScanner.ts` |
| 1% | 18 | 220 | `apps/web/src/components/BrowserCookieImport.tsx` | `apps/web/src/components/settings/ConnectionsSettings.tsx` |
| 1% | 8 | 32 | `apps/web/src/components/settings/providers/ProviderUpdateAction.tsx` | `apps/web/src/components/pullRequest/PullRequestDetailPanel.tsx` |
| 1% | 12 | 439 | `apps/web/src/lib/providerModelPrefetch.ts` | `apps/server/src/provider/Layers/ProviderRegistry.test.ts` |
| 1% | 7 | 267 | `packages/shared/src/frameTransport.test.ts` | `apps/mobile/src/state/thread-outbox.test.ts` |
| 1% | 8 | 116 | `packages/shared/src/threadWorkspace.ts` | `apps/server/src/git/GitManager.ts` |
| 1% | 5 | 191 | `apps/desktop/src/cuaFixtures/native.ts` | `scripts/mobile-showcase.ts` |
| 1% | 11 | 730 | `apps/server/src/agentGateway/computerBrowserTools.test.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.test.ts` |
| 1% | 5 | 80 | `apps/server/src/provider/openCodeMessageState.ts` | `apps/mobile/src/state/use-composer-drafts.ts` |
| 1% | 9 | 45 | `apps/web/src/components/ThreadPinToggleButton.tsx` | `apps/web/src/components/pullRequest/PullRequestDetailPanel.tsx` |
| 1% | 5 | 63 | `apps/web/src/components/WorkspaceFilePreview.capacity.browser.tsx` | `apps/web/src/terminal/ghostty/surface.test.ts` |
| 1% | 6 | 72 | `apps/web/src/lib/accountLogic.ts` | `apps/desktop/src/snapShot/DesktopSnapShot.ts` |
| 1% | 19 | 2038 | `apps/desktop/src/cuaDriverHost.ts` | `apps/server/src/provider/Layers/ClaudeAdapter.ts` |
| 1% | 7 | 248 | `apps/marketing/src/lib/seo.ts` | `apps/web/src/components/settings/SettingsPanels.tsx` |
| 1% | 5 | 108 | `apps/server/src/git/Layers/GitCommandAdmission.ts` | `apps/desktop/src/snapShot/DesktopSnapShot.ts` |
| 1% | 5 | 116 | `apps/server/src/persistence/messageTextChunks.ts` | `apps/server/src/project/AgentSessionScanner.ts` |
| 1% | 10 | 102 | `apps/server/src/pullRequests/projectRepositoryInventory.ts` | `apps/server/src/pullRequest/PullRequestService.ts` |
| 1% | 14 | 384 | `apps/web/src/notifications/taskCompletion.logic.ts` | `apps/server/src/orchestration/Layers/ProviderRuntimeIngestion.ts` |
| 1% | 5 | 146 | `packages/shared/src/outboundHttpPolicy.ts` | `apps/web/src/components/ChatView.logic.ts` |
| 1% | 7 | 424 | `apps/api/src/config.ts` | `apps/server/src/provider/Layers/CodexAdapter.ts` |
| 1% | 8 | 422 | `apps/api/src/identity/hostKeyRegistry.ts` | `apps/mobile/src/state/use-composer-drafts.ts` |
| 1% | 6 | 253 | `apps/api/src/perf.ts` | `apps/web/src/routes/_chat.pull-requests.tsx` |
| 1% | 5 | 119 | `apps/e2e/src/harness/codexFixture.mjs` | `apps/server/src/provider/Layers/CodexSessionRuntime.ts` |
| 1% | 8 | 258 | `apps/marketing/scripts/performance-smoke.mjs` | `scripts/mobile-showcase.ts` |
| 1% | 6 | 140 | `apps/server/src/agentGateway/computerProgressGuard.ts` | `apps/desktop/src/snapShot/DesktopSnapShot.ts` |
| 1% | 12 | 127 | `apps/web/src/hooks/useProviderStatusRefresh.ts` | `apps/web/src/components/ChatMarkdown.tsx` |
| 1% | 5 | 74 | `apps/web/src/terminalVisualIdentity.ts` | `apps/server/src/orchestration/projector.ts` |
| 1% | 5 | 169 | `scripts/lib/release-artifact-checkpoint.ts` | `apps/web/src/components/GitActionsControl.tsx` |
| 1% | 7 | 87 | `apps/server/src/provider/Layers/CursorAdapter.test.ts` | `apps/mobile/src/lib/threadActivity.test.ts` |
| 1% | 12 | 157 | `apps/server/src/pullRequests/pullRequestPinRecovery.ts` | `apps/server/src/pullRequest/PullRequestService.ts` |
| 1% | 7 | 258 | `apps/web/src/components/AppRailUsage.browser.tsx` | `apps/web/src/components/Sidebar.logic.test.ts` |
| 1% | 7 | 97 | `apps/web/src/components/SidebarHoverCards.browser.tsx` | `apps/web/src/components/CommandPalette.tsx` |
| 1% | 12 | 488 | `apps/web/src/lib/providerDiscoveryReactQuery.ts` | `apps/server/src/terminal/Manager.ts` |
| 1% | 5 | 142 | `apps/desktop/src/browserAutomation/browserVaultCapture.ts` | `apps/web/src/hooks/useTheme.ts` |
| 1% | 14 | 531 | `apps/desktop/src/browserWebMcp/guestBridge.ts` | `apps/web/src/composerDraftStore.ts` |
| 1% | 21 | 3016 | `apps/server/src/computer/ComputerManager.test.ts` | `apps/desktop/src/snapShot/DesktopSnapShot.test.ts` |
| 1% | 11 | 72 | `apps/server/src/persistence/Migrations/122_ClearAutomationRunProviderOptions.ts` | `apps/server/src/orchestration/Layers/ProjectionSnapshotQuery.ts` |
| 1% | 6 | 126 | `apps/server/src/provider/openCodeBackgroundTasks.test.ts` | `apps/server/src/provider/Layers/AntigravityAdapter.test.ts` |
| 1% | 9 | 430 | `apps/server/src/terminal/managedTerminalWrappers.ts` | `apps/web/src/components/CommandPalette.tsx` |
| 1% | 5 | 210 | `apps/web/src/components/chat/ComputerPreviewPopover.test.tsx` | `apps/web/src/components/chat/MessagesTimeline.test.tsx` |
| 1% | 10 | 140 | `apps/web/src/hooks/useThreadWorkspaceHandoff.ts` | `apps/web/src/components/CommandPalette.tsx` |
| 1% | 15 | 1511 | `scripts/computer-use-fixtures/live-cert.ts` | `apps/mobile/src/lib/threadActivity.ts` |
| 1% | 10 | 535 | `apps/web/src/components/chat/GeneratedMarkdownImage.browser.tsx` | `apps/web/src/components/chat/MessagesTimeline.test.tsx` |
| 1% | 6 | 230 | `apps/web/src/components/terminal/terminalRuntimeAppearance.ts` | `apps/web/src/components/ThreadTerminalDrawer.tsx` |
| 1% | 9 | 111 | `apps/web/src/lib/kanbanDispatch.ts` | `apps/web/src/components/LegacySidebar.tsx` |
| 1% | 6 | 280 | `apps/desktop/src/browserAutomation/betterwrightCdp.ts` | `apps/web/src/terminal/ghostty/core.ts` |
| 1% | 7 | 182 | `apps/server/src/agentGateway/mcpInjection.ts` | `scripts/lib/third-party-licenses.ts` |
| 1% | 6 | 87 | `apps/server/src/orchestration/Layers/TurnCheckpointCoordinator.ts` | `apps/server/src/orchestration/Layers/ProviderCommandReactor.ts` |
| 1% | 8 | 73 | `apps/web/src/lib/providerSetupStatus.test.ts` | `apps/web/src/components/ChatView.logic.test.ts` |
| 1% | 14 | 1742 | `apps/analytics/worker.product.integration.test.mjs` | `apps/mobile/src/state/use-composer-drafts.test.ts` |
| 1% | 5 | 95 | `apps/desktop/src/backendStartupBlock.ts` | `apps/mobile/src/lib/threadActivity.ts` |
| 1% | 7 | 118 | `apps/server/src/agentGateway/Layers/AgentGatewaySessionRegistry.ts` | `apps/server/src/provider/Layers/ProviderService.ts` |
| 1% | 12 | 399 | `apps/web/src/components/codeEditor/editorModels.ts` | `apps/mobile/src/lib/threadActivity.ts` |
| 1% | 5 | 99 | `apps/web/src/lib/chatFirstSend.ts` | `apps/web/src/components/Sidebar.logic.ts` |
| 1% | 5 | 629 | `apps/desktop/src/browserAutomation/betterwrightCdp.test.ts` | `apps/mobile/src/lib/threadActivity.test.ts` |
| 1% | 6 | 163 | `apps/web/src/appRail.logic.ts` | `apps/desktop/src/snapShot/snapShot.ts` |
| 1% | 11 | 53 | `apps/web/src/hooks/useThreadUnblock.ts` | `apps/web/src/components/ChatMarkdown.tsx` |
| 1% | 8 | 522 | `apps/web/src/lib/computerToolPresentation.ts` | `apps/web/src/components/RightPanelTabs.tsx` |
| 1% | 13 | 697 | `scripts/i18n.ts` | `apps/web/src/session-logic.ts` |
| 1% | 15 | 623 | `apps/server/src/agentGateway/targetResolver.ts` | `apps/mobile/src/state/use-composer-drafts.ts` |
| 1% | 5 | 299 | `apps/server/src/computer/wsComputerHandlers.ts` | `apps/desktop/src/snapShot/DesktopSnapShot.ts` |
| 1% | 5 | 90 | `apps/server/src/managedAttachmentCleanup.test.ts` | `apps/server/src/orchestration/ThreadSettlementReactor.test.ts` |
| 1% | 8 | 127 | `apps/web/src/components/chat/rightDockPaneMeta.tsx` | `apps/web/src/components/ChatMarkdown.tsx` |
| 1% | 5 | 301 | `benchmarks/desktop-perf/diagnose.mjs` | `apps/desktop/src/snapShot/DesktopSnapShot.ts` |
| 1% | 13 | 250 | `apps/server/src/persistence/Layers/HubWorkRepository.test.ts` | `apps/server/src/provider/Layers/ProviderService.test.ts` |
| 1% | 7 | 154 | `apps/server/src/scratchWorkspaces.ts` | `apps/server/src/process/externalLauncher.ts` |
| 1% | 9 | 449 | `benchmarks/desktop-perf/longchat.mjs` | `scripts/mobile-showcase.ts` |
| 1% | 9 | 202 | `apps/server/src/agentGateway/threadSummary.test.ts` | `apps/web/src/components/ChatView.logic.test.ts` |
| 1% | 5 | 71 | `apps/server/src/provider/unmappedProviderEvents.test.ts` | `apps/server/src/provider/Layers/CursorAdapter.ts` |
| 1% | 9 | 168 | `apps/web/src/components/ChatView.selectors.test.ts` | `apps/web/src/session-logic.test.ts` |
| 1% | 10 | 210 | `apps/web/src/components/chat/useTailAnchorScroll.ts` | `apps/web/src/hooks/useTheme.ts` |
| 1% | 8 | 386 | `apps/server/src/device/IosSimulatorBackend.helperProcess.test.ts` | `apps/desktop/src/snapShot/DesktopSnapShot.test.ts` |
| 1% | 7 | 106 | `apps/web/src/lib/rendererErrorDiagnostics.ts` | `apps/server/src/provider/Layers/GrokAdapter.ts` |
| 1% | 12 | 491 | `apps/api/src/liveActivities/service.ts` | `apps/mobile/src/features/threads/ThreadFeed.tsx` |
| 1% | 8 | 1095 | `apps/server/src/codexProcessEnv.test.ts` | `apps/mobile/src/state/use-composer-drafts.test.ts` |
| 1% | 8 | 145 | `apps/web/src/components/chat/MessagesTimeline.renderScope.browser.tsx` | `apps/web/src/components/chat/MessagesTimeline.logic.test.ts` |
| 1% | 8 | 51 | `apps/web/src/lib/kanbanTaskCreate.test.ts` | `apps/web/src/composerDraftStore.test.ts` |
| 1% | 6 | 299 | `apps/web/src/lib/pullRequestReactQuery.refresh.test.ts` | `apps/mobile/src/state/thread-outbox.test.ts` |
| 1% | 6 | 240 | `apps/web/src/settingsNavigation.ts` | `apps/server/src/cloud/http.ts` |
| 1% | 7 | 293 | `packages/contracts/src/browserAutomationCssSelector.ts` | `apps/server/src/provider/Layers/OpenCodeAdapter.ts` |
| 1% | 13 | 932 | `scripts/computer-use-fixtures/multi-display-cert.ts` | `apps/server/src/orchestration/Layers/ProviderRuntimeIngestion.ts` |
| 1% | 8 | 257 | `scripts/lib/release-artifact-checkpoint.test.ts` | `scripts/build-desktop-artifact.ts` |
| 1% | 7 | 584 | `apps/api/src/identity/workos.test.ts` | `apps/server/src/preview/PortScanner.test.ts` |
| 1% | 21 | 420 | `apps/server/src/provider/openCodeV2Messages.ts` | `apps/server/src/provider/Layers/OpenCodeAdapter.ts` |
| 1% | 6 | 66 | `apps/web/src/lib/threadUnblock.ts` | `apps/server/src/provider/Layers/OpenCodeAdapter.ts` |
| 1% | 16 | 1863 | `apps/desktop/src/appSnapManager.test.ts` | `apps/desktop/src/snapShot/DesktopSnapShot.test.ts` |
| 1% | 5 | 203 | `apps/server/src/cloudflare/authorization.ts` | `apps/mobile/src/state/use-thread-outbox-drain.ts` |
| 1% | 7 | 104 | `packages/shared/src/coordinatorCheckin.test.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.test.ts` |
| 1% | 8 | 453 | `scripts/computer-use-fixtures/browser-benchmark-evidence.ts` | `apps/web/src/components/RightPanelTabs.tsx` |
| 1% | 6 | 157 | `apps/server/src/computer/ComputerApprovalGate.ts` | `apps/server/src/terminal/Manager.ts` |
| 1% | 9 | 633 | `apps/web/src/components/WorkspaceFilePreview.editing.browser.tsx` | `apps/mobile/src/state/use-thread-outbox-drain.ts` |
| 1% | 16 | 3087 | `apps/server/src/computer/CuaComputerBackend.test.ts` | `apps/mobile/src/state/use-composer-drafts.test.ts` |
| 1% | 7 | 126 | `apps/server/src/provider/openCodeV2Messages.test.ts` | `apps/web/src/session-logic.test.ts` |
| 1% | 5 | 290 | `apps/web/src/lib/pullRequestReactQuery.pin.test.ts` | `packages/client-runtime/src/state/threadReducer.test.ts` |
| 0% | 7 | 70 | `apps/server/src/orchestration/projectImportHistory.claude.test.ts` | `apps/server/src/project/AgentSessionScanner.test.ts` |
| 0% | 7 | 579 | `apps/web/src/components/settings/ConnectionsSettingsPanel.browser.tsx` | `apps/desktop/src/preview/Manager.test.ts` |
| 0% | 5 | 262 | `apps/api/src/routes/v1.billing.test.ts` | `infra/relay/src/http/Api.test.ts` |
| 0% | 11 | 2950 | `apps/desktop/src/cuaDriverHost.test.ts` | `apps/server/src/orchestration/Layers/ProviderRuntimeIngestion.test.ts` |
| 0% | 6 | 266 | `apps/server/src/agentGateway/computerTools.visibility.test.ts` | `apps/server/src/orchestration/Layers/CheckpointReactor.test.ts` |
| 0% | 8 | 52 | `apps/web/src/components/chat/environment/EnvironmentSubagentsSection.tsx` | `apps/web/src/components/settings/ConnectionsSettings.tsx` |
| 0% | 7 | 240 | `apps/web/src/components/kanban/useKanbanTaskComposerDiscovery.ts` | `apps/mobile/src/features/threads/ThreadFeed.tsx` |
| 0% | 5 | 137 | `apps/web/perf/concurrent.tsx` | `apps/web/src/components/chat/MessagesTimeline.test.tsx` |
| 0% | 8 | 376 | `packages/shared/src/browserAutomationCatalogue.ts` | `apps/server/src/terminal/Manager.ts` |
| 0% | 8 | 221 | `apps/web/src/components/BrowserPanel.annotations.browser.tsx` | `apps/web/src/composerDraftStore.test.ts` |
| 0% | 8 | 609 | `apps/server/src/agentGateway/deviceTools.test.ts` | `apps/desktop/src/snapShot/DesktopSnapShot.test.ts` |
| 0% | 6 | 250 | `apps/server/src/orchestration/importedThreadMessages.ts` | `apps/server/src/orchestration/decider.ts` |
| 0% | 6 | 137 | `apps/server/src/provider/threadMentionContext.ts` | `apps/server/src/terminal/Manager.ts` |
| 0% | 10 | 122 | `benchmarks/desktop-perf/lib/drive.mjs` | `apps/desktop/src/preview/Manager.ts` |
| 0% | 26 | 2138 | `apps/web/src/wsTransport.ts` | `apps/server/src/terminal/Manager.ts` |
| 0% | 5 | 156 | `scripts/check-windows-runtime-boundary.ts` | `apps/web/src/components/CommandPalette.tsx` |
| 0% | 7 | 105 | `apps/web/perf/toolOutput.perf.test.ts` | `apps/web/src/components/chat/MessagesTimeline.logic.test.ts` |
| 0% | 6 | 61 | `apps/server/src/persistence/SqliteReclamation.ts` | `apps/desktop/src/preview/Manager.ts` |
| 0% | 6 | 186 | `apps/web/src/components/settings/ComputerProductGuidance.browser.tsx` | `apps/desktop/src/snapShot/DesktopSnapShot.test.ts` |
| 0% | 7 | 83 | `apps/server/steer-probe2.mjs` | `apps/server/src/provider/Layers/ClaudeAdapter.ts` |
| 0% | 5 | 101 | `apps/web/src/lib/selectionChat.ts` | `apps/web/src/components/ChatView.logic.ts` |
| 0% | 6 | 62 | `apps/web/src/lib/threadCreatePromotion.ts` | `apps/web/src/components/settings/ConnectionsSettings.tsx` |
| 0% | 5 | 84 | `apps/web/scripts/measure-lcp.mjs` | `apps/web/src/components/CommandPalette.tsx` |
| 0% | 5 | 102 | `apps/web/src/hooks/useTemporaryThreadLifecycle.ts` | `apps/web/src/components/CommandPalette.tsx` |
| 0% | 5 | 69 | `apps/web/src/routes/_chat.index.tsx` | `apps/web/src/components/CommandPalette.tsx` |
| 0% | 7 | 140 | `apps/server/src/orchestration/handoff.test.ts` | `apps/server/src/orchestration/Layers/ProjectionSnapshotQuery.test.ts` |
| 0% | 7 | 58 | `apps/web/src/lib/startContainerChat.ts` | `apps/web/src/composerDraftStore.ts` |
| 0% | 6 | 261 | `apps/server/src/device/helperClient.test.ts` | `apps/desktop/src/preview/Manager.ts` |

## Post-audit verification (2026-11-22)

- §4.1 line-derived crates: all 6 re-derived cases-only and gate-green (worklog, storeeventreducer, keybindingrules, shell, terminal-links, markdown-links).
- §4.2 batch-4 re-derived cases-only and gate-green: backendsupervisionpolicy, model, providerusagesnapshot, runtimearch, session-logic, storenormalization, worker.
- §4.2 remaining 8 (shortcutssheet, composer-editor-mentions, pendinguserinput, projectscripts, serversettings, terminalstatestore, transcriptusageparsers, workspaceentries): verified clean — none of the flagged shared internal names appear in the committed `port.rs` (grep-verified); the flagged names live in the synara TS, not our ports.
- cua: confirmed out of copyright scope (upstream trycua/cua, MIT, adapted copies already in synara); no notice required in `rust/LICENSE`.
- proposedplan: re-derived cases-only (1997 cases), gate green.
