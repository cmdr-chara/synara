# Synara parity roadmap

This file is intentionally short. Use it for **what is done, what remains, and what comes next**.

Detailed parity evidence lives in:
- [Current parity gates](docs/verification/current-parity-gates.md)
- [Electron vs GPUI feature gap](docs/ui/electron-vs-gpui-feature-gap.md)
- [Archived detailed roadmap](docs/history/roadmap-before-simplification-2026-09-24.md)

## Current status

- Shipped feature slices: **50**
- Major remaining: **27**
- Smaller remaining: **11**
- Acceptance/integration remaining: **10**
- Total remaining: **48**
- Completely missing top-level surfaces: **0**
- Broad verification gates still open: **21**

The **48-item count is the execution count to use going forward**. The 21 verification
gates are larger acceptance buckets and are not a feature count.

Current upstream reference: `Emanuele-web04/synara@eaa61eded31b6755d4f30ba8eabc5d905cf817cb`.

## Major features remaining

- [ ] M01 Web workspace provider connection/sign-in
- [ ] M02 Rich web approval context for tools, commands and diffs
- [ ] M03 Durable web question drafts across navigation/restart
- [ ] M04 Direct-model execution from the web workspace
- [ ] M05 Remote workspace execution
- [ ] M06 Production headless deployment with bind/TLS/update packaging
- [ ] M07 Protected browser session/cookie import
- [ ] M08 Complete browser popup authentication lifecycle
- [ ] M09 Agent-controlled browser upload/download
- [x] M10 Browser console/runtime diagnostics
- [ ] M11 Safe restoration of task/auth browser sessions
- [ ] M12 Page-declared WebMCP integration
- [ ] M13 Simulator live frame streaming
- [ ] M14 Simulator touch, swipe, typing and hardware-button input
- [ ] M15 Simulator recording
- [ ] M16 Simulator accessibility tree and semantic element targeting
- [ ] M17 Editor syntax highlighting
- [ ] M18 Advanced editor conflict recovery
- [x] M19 Richer editor comparison scopes
- [x] M20 Safe line blame without repository filter execution
- [ ] M21 Deeper diff editing/review workflows
- [ ] M22 Managed worktree automatic cleanup/recovery
- [ ] M23 SSH managed worktree creation
- [ ] M24 Environment-aware task/fork orchestration
- [ ] M25 Richer model/context controls, including fast/thinking presets and compaction
- [ ] M26 Real provider/account telemetry integration
- [ ] M27 Broader Computer Use actions, targeting and preview behavior
- [ ] M28 Trusted signed updater/install/rollback lifecycle
- [ ] M29 Binary PDF/document attachment pipeline and broader document viewing
- [ ] M30 Studio historical output versioning and long-running lifecycle

## Smaller features remaining

- [ ] S01 Provider-specific onboarding setup/login UX
- [ ] S02 Voice interaction controls beyond record/transcribe-to-draft
- [x] S03 Direct-model keyboard cycling
- [x] S04 Context-aware custom keybindings
- [x] S05 Additional native slash-command argument forms
- [ ] S06 Deeper automation orchestration semantics
- [ ] S07 Same-task handoff continuation semantics
- [ ] S08 Provider-native fork actions
- [x] S09 Persistent folder references
- [x] S10 Richer structured metadata in thread export
- [ ] S11 Reply/context reuse into the current composer
- [x] S12 Reply/context reuse into side/new tasks
- [ ] S13 Exact upstream project-search ranking
- [ ] S14 Exact ignored/generated-file search behavior
- [ ] S15 Per-turn provider/model activity breakdown
- [ ] S16 Token heatmap
- [x] S17 Studio organization/filtering polish
- [ ] S18 PDF text/link/form interaction after basic rendering

## Acceptance/integration remaining

These are primarily proof on real providers, platforms or release infrastructure,
not large new product subsystems.

- [ ] A01 Live microphone + ChatGPT transcription end-to-end acceptance
- [ ] A02 macOS microphone packaging/permission acceptance
- [ ] A03 Windows voice/package acceptance
- [ ] A04 Fresh-install onboarding with real provider accounts
- [ ] A05 Authenticated browser login/session/popup acceptance
- [ ] A06 Real macOS Simulator/device acceptance
- [ ] A07 SSH worktree/search acceptance
- [ ] A08 Signed release feed/install/rollback package acceptance
- [ ] A09 Multi-provider ACP/direct-model interoperability matrix
- [ ] A10 Cross-platform visual/accessibility/save-picker acceptance

## Next execution queue

Finish whole workflows instead of spreading work across every gate:

1. **Web workspace:** M01-M06
2. **Editor/review:** M17-M18, M21
3. **Worktrees/handoff:** M22-M24, S07-S08
4. **Documents/Studio:** M29-M30, S18
5. **Simulator:** M13-M16 when the required macOS/native input backend is available

## Shipped

### Delivered feature slices in the September 23 sprint

**25 slices shipped.** They include voice/transcription, the local headless server,
cron/DST automation work, search, analytics, web transcript/task browsing,
onboarding/project setup, model controls, attachments, editor recovery, local web
workspace drafts, Simulator app/URL operations, worktree forks, Computer Use typing,
updater integrity checks and Studio output reopening.

### September 24 continuation

**25 additional slices shipped**, bringing the total to **50**:

- Batch 1: web Run/Stop, goal resume/clear/edit, Simulator app install/terminate
- Batch 2: editor auto-save, Studio WebP preview, Studio reporting-turn attribution
- Batch 3: onboarding ACP sign-in, reviewed new worktree forks, scoped model controls
- Batch 4: Library PDF viewing, original-file export, inline onboarding history import
- Batch 5: committed-file history and read-only revision preview
- Batch 6: web one-time approvals/questions and debounced live Explorer search
- Batch 7: browser runtime diagnostics, editor comparison and blame, direct-model cycling,
  contextual keybindings, settings command arguments, saved folder references, structured
  export metadata, side-chat context reuse, and Studio filters. PDF page-text extraction
  also shipped, while S18 remains open for links and forms.

Verification receipts:
[batch 1](docs/verification/parity-2026-09-24-batch1.md),
[batch 2](docs/verification/parity-2026-09-24-batch2.md),
[batch 3](docs/verification/parity-2026-09-24-batch3.md),
[batch 4](docs/verification/parity-2026-09-24-batch4.md),
[batch 5](docs/verification/parity-2026-09-24-batch5.md),
[batch 6](docs/verification/parity-2026-09-24-batch6.md),
[batch 7](docs/verification/parity-2026-09-24-batch7.md).

## How to update this roadmap

When a remaining item ships:
1. Check its box.
2. Increment **Shipped feature slices** by one.
3. Decrement the matching remaining count and **Total remaining**.
4. Add the feature to the latest batch receipt.
5. Keep the broader verification gate OPEN until its full workflow and required
   provider/platform failure paths are actually accepted.

Do not use the 21-gate count as the feature count.
