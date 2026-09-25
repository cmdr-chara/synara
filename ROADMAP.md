# Synara parity roadmap

This file is intentionally short. Use it for **what is done, what remains, and what comes next**.

Detailed parity evidence lives in:
- [Current parity gates](docs/verification/current-parity-gates.md)
- [Electron vs GPUI feature gap](docs/ui/electron-vs-gpui-feature-gap.md)
- [Archived detailed roadmap](docs/history/roadmap-before-simplification-2026-09-24.md)

## Current status

- Shipped feature slices: **63**
- Major remaining: **22**
- Smaller remaining: **3**
- Acceptance/integration remaining: **10**
- Total remaining: **35**
- Completely missing top-level surfaces: **0**
- Broad verification gates still open: **21**

The **35-item count is the execution count to use going forward**. The 21 verification
gates are larger acceptance buckets and are not a feature count.

Current upstream reference: `Emanuele-web04/synara@eaa61eded31b6755d4f30ba8eabc5d905cf817cb`.

Batch 7 validation: roadmap, formatting, native compilation and focused tests passed
after compile corrections. Strict lint corrections accompany batch 8; Xvfb-backed
journeys and the cancelled WebKit job remain unverified.

## Major features remaining

- [ ] M01 Web workspace provider connection/sign-in
- [ ] M02 Rich web approval context for tools, commands and diffs
- [x] M03 Durable web question drafts across navigation/restart
- [x] M04 Direct-model execution from the web workspace
- [x] M05 Remote workspace execution
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
- [x] M17 Editor syntax highlighting
- [x] M18 Advanced editor conflict recovery
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

- [x] S01 Provider-specific onboarding setup/login UX
- [x] S02 Voice interaction controls beyond record/transcribe-to-draft
- [x] S03 Direct-model keyboard cycling
- [x] S04 Context-aware custom keybindings
- [x] S05 Additional native slash-command argument forms
- [ ] S06 Deeper automation orchestration semantics
- [x] S07 Same-task handoff continuation semantics
- [ ] S08 Provider-native fork actions
- [x] S09 Persistent folder references
- [x] S10 Richer structured metadata in thread export
- [x] S11 Reply/context reuse into the current composer
- [x] S12 Reply/context reuse into side/new tasks
- [x] S13 Exact upstream project-search ranking
- [x] S14 Exact ignored/generated-file search behavior
- [x] S15 Per-turn provider/model activity breakdown
- [x] S16 Token heatmap
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

1. **Web workspace:** M01-M02, M06
2. **Editor/review:** M21
3. **Worktrees/handoff:** M22-M24, S08
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

**31 additional slices shipped**, bringing the total to **56**:

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
- Batch 8 (in progress): current-composer reply scaffold with quoted assistant context and
  an editable follow-up. Existing draft and attachments are retained; sending stays manual.
  PDF page-link inspection and explicit opening are implemented; S18 remains open because
  PDF form fields are not interactive. Editor syntax highlighting covers common source and
  document formats with a bounded lexer; unsupported and very large files remain plain text.
  OpenCode/Gemini CLI onboarding guidance and copyable login commands are available; S01
  remains open for the wider dynamic provider catalog. Local and SSH search now share bounded
  nested ignore rules and generated-output skips; S14 remains open for exact upstream parity.
  A reviewed disk comparison now offers a bounded three-way merge for disjoint editor edits;
  overlapping edits stay in the buffer for manual resolution and saving remains explicit.
- Batch 9: pending web question answers persist in the browser profile across task
  navigation and browser restart. Drafts are scoped to the task and request,
  expire after 24 hours, and are removed on submit, decline, cancellation or expiry.
  Answers are sent to the agent only on explicit submission.
  Web approvals also show a matching tool's title, kind and bounded recorded diff
  when those details exist. M02 remains open for command and richer live diff context.
  The editor comparison panel can copy its complete bounded review with change
  markers to the clipboard. M21 remains open for interactive hunk editing.
- Batch 10: Direct-model review offers Fast, Balanced and Thinking presets only
  when the selected model advertises corresponding reasoning levels. It shows
  the model's context window and requested output budget when available.
  The options remain an editable draft until route confirmation; M25 remains
  open for automatic compaction and broader provider context controls.
  The local web workspace can now run a direct-model route previously reviewed
  in the native app. It displays the selected provider/model, requires an explicit
  confirmation, and rejects a stale route stamp before starting; M04 is complete.
- Batch 11 (in progress): unassigned worktrees under Synara's scratch parent with
  a matching generated branch/path are called out as recoverable in the existing
  fork menu. Selecting one creates a new unsent task in that checkout using the
  current source-message context; the recovery operation rechecks the unassigned
  branch/path identity, source and live Git metadata under the lifecycle lock.
  Git checkout is not repeated. M22 remains open for automatic lifecycle cleanup
  and broader crash recovery.
  The editor comparison can restore one selected change block into the unsaved
  buffer after rechecking the file, comparison generation and exact current diff.
  M21 remains open for deeper hunk review and staging.
  Studio text previews can inspect and copy committed Git snapshots for the
  selected file. The current preview and historical snapshot remain visibly
  separate; M30 remains open for uncommitted output versioning and long-running
  lifecycle controls.
  The local web workspace can execute tasks in previously configured SSH
  workspaces. It displays the remote destination and rechecks the pinned SSH
  profile before starting; M05 is complete for existing remote workspaces.
- Batch 12 (in progress): explicitly selected PDF files can be stored as binary
  attachment snapshots. The existing bounded PDF helper validates them before
  import, previews extractable text, and sends up to 12 labeled pages as inert
  text context under a combined prompt limit. M29 remains open for other binary
  document formats and wider document viewing.
  DOCX main-document text can also be extracted from an explicitly selected
  bounded archive, previewed and sent as text context; embedded objects and
  external relationships are never opened. M29 remains open for wider formats
  and page-level document workflows.
  Studio Library now previews bounded DOCX main-document text and can copy the
  extraction while exporting the original document bytes. This is read-only;
  M29 remains open for richer document viewing and page-level workflows.
  For an open PDF snapshot, Studio can also extract and copy labeled text from
  the first 12 pages under a 512 KiB limit. Remaining pages are disclosed;
  scanned pages still need OCR and S18 still includes form interaction.
  Web one-time approvals now show available recorded tool text and terminal
  output with explicit truncation and exit status alongside the existing diff
  context. M02 remains open for live command details and deeper review.
  Studio retains up to 12 bounded text preview snapshots during an open session
  when a file changes between refreshes. Earlier previews can be inspected and
  copied without changing the current file. M30 remains open for durable output
  versioning and long-running lifecycle controls.
  Studio now refreshes its open Library when a tool in the visible Hub finishes;
  completions during a running refresh queue one more refresh so output changes
  are not lost. M30 still needs durable versioning and broader lifecycle controls.
  The editor comparison offers Copy block for a single changed block after
  rechecking its owner, generation and current buffer. It copies bounded diff
  markers without saving or staging. It now also offers a two-step Restore all
  action that replaces only the unsaved buffer with the selected comparison
  reference after rechecking task/project/root/path/tab ownership, generation
  and the exact current diff. Any buffer or comparison change cancels the
  confirmation. M21 remains open for richer hunk review and any staging path
  that can protect the existing Git index correctly.

### September 25 continuation

- Batch 13 (in progress): recoverable Synara scratch worktrees can now be
  explicitly cleaned up from the fork-environment menu. Cleanup is separately
  reviewed, rechecks source/task/worktree/branch/scratch ownership under the
  lifecycle lock, uses ordinary non-force Git worktree removal, preserves dirty
  or locked checkouts, and retains the generated branch. M22 remains open for
  automatic post-task lifecycle cleanup and broader crash reconciliation.
  The worktree fork menu is now a unified environment chooser: it includes the
  current local or SSH workspace, existing linked worktrees, recoverable
  Synara worktrees, and reviewed new local worktrees. Remote workspaces clearly
  disable only new managed checkout creation. M24 remains open for broader
  orchestration policy across task creation and provider handoff.
  ODT joins PDF and DOCX as a bounded binary document attachment. Only
  `content.xml` is read, extracted text is previewed and sent as inert context,
  embedded objects and external resources are ignored, and Studio can preview
  the same read-only extraction. M29 remains open for wider document formats,
  richer viewing and page-level workflows.
  Studio text preview versions are now persisted per Hub under a bounded
  12-entry/1 MiB ledger, deduplicated by file content and restored when the file
  is previewed again after restart. Source files remain read-only and deleting
  the task removes its version ledger. M30 remains open for richer long-running
  lifecycle/version organization beyond text previews.
  Computer Use now supports reviewed pointer movement and double-click in addition
  to click, scroll, literal typing and named keys. Both new actions stay
  window-addressed, coordinate-bounded, one-frame/one-action and stale-frame
  checked. M27 remains open for richer targeting and broader platform behavior.
  Direct-model turns now retain the exact reviewed provider/model route together
  with the real token usage reported by that provider stream, and the transcript
  activity summary shows that per-turn breakdown. ACP turns only show token
  counts when their provider reports them; no provider/model identity is inferred.
  M26 remains open for account/quota/billing telemetry, and S15 remains open for
  provider/model attribution on ACP turns.
- Batch 14: project entry-name search now applies the upstream leading
  `@`/`.`/`/` query normalization before the existing exact/prefix/fuzzy/path
  rank tiers and scores normalized POSIX-style paths on every platform. The
  existing score/depth/path tie-break remains intact. S13 is complete; S14 and
  N2 stay open for exact ignored/generated-file semantics, SSH GUI behavior and
  wider platform acceptance. Profile Activity now prefers a bounded 274-day UTC
  token heatmap when durable turns contain provider-reported input and output
  token counts, otherwise it falls back to the existing turn-start heatmap.
  Missing token telemetry is explicitly omitted rather than inferred. S16 is
  complete; D11 remains open for broader provider/model and real account/quota
  telemetry.
- Batch 15: project file-name and content search now use the
  upstream project-search static generated-directory set exactly:
  `.git`, `.convex`, `node_modules`, `.next`, `.turbo`, `dist`,
  `build`, `out` and `.cache`. Native-only exclusions such as `target`,
  `coverage`, virtual environments, minified files, source maps and compiled
  extensions no longer disappear from project search merely because of their
  name. Git worktrees now use bounded hardened `git ls-files --cached --others
  --exclude-standard` plus chunked `git check-ignore --no-index`; non-Git
  folders apply only the upstream generated-directory set. The same owner backs
  local and SSH-helper search. S14 is complete.
  Provider onboarding guidance now also covers Codex and Claude Code alongside
  OpenCode and Gemini CLI, with exact executable checks before presenting a
  copyable login command. S01 remains open for deeper provider-native setup/login
  state. Voice recording now exposes a bounded elapsed-time indicator and
  five-level live input meter in the composer; S02 remains open for broader
  interaction controls and live-provider/platform acceptance.
- Batch 16: S01 and S02 are complete at the product-feature level. The setup
  flow already owns an explicit unsent setup task, Connect, advertised ACP auth
  methods and connection questions; provider-specific Codex, Claude Code,
  OpenCode and Gemini CLI login guidance now supplies the missing provider UX.
  Real-account fresh-install proof remains A04/D1. Voice now has start, stop,
  cancel, stale-result fencing, bounded transcribe-to-draft, elapsed recording
  feedback and a live input meter. Real microphone/provider/macOS/Windows proof
  remains A01-A03/M1 rather than duplicating that acceptance work under S02.
- Batch 17 (in progress): automation history can now be explicitly pruned only
  for terminal runs whose definitions were already deleted. The operation is
  confirmation-gated, preserves generated conversations, never removes active
  runs or history for current definitions, and leaves run-limit semantics
  unchanged. This removes one source of the 256-entry ledger dead-end without
  silently discarding current automation evidence. Retained run history can also
  be exported through the native save dialog as a versioned JSON snapshot using
  the existing no-overwrite private-file writer. Live-definition terminal
  history can now also be explicitly pruned without resetting cumulative
  max-run accounting; generated conversations, failure streak and schedule state
  remain intact. Automations can opt into Hub shared context for their selected
  project: each claim snapshots the current saved Hub revision and visible
  instructions/knowledge into the owned Studio-scoped conversation before
  provider launch, while legacy/project-mode automations remain project-only.
  S06 remains open for out-of-process scheduling and wider live-provider
  lifecycle acceptance.
  Durable Studio text-version history for a selected file can now be cleared
  through a two-step native confirmation. The operation is task/path-scoped,
  preserves the workspace file and histories for other files, and stale replies
  are ignored by task/path/generation checks. An exact selected durable version
  can also be saved to a new destination after task/path/generation/selection
  revalidation; the backend rechecks that the snapshot still exists and never
  reads or rewrites the current workspace file. M30 remains open for broader
  long-running lifecycle/version organization.
- Batch 18: ACP prompt submission now snapshots the exact task agent and the
  acknowledged single-model selection onto the returned turn ID. The durable
  route event binds correctly whether it replays before or after PromptStarted,
  rejects conflicting duplicate attribution, and survives later task-agent or
  thread-model changes. Transcript activity shows ACP/direct route plus reported
  turn tokens, and Profile aggregates exact per-turn route/model counts with
  explicit legacy-unattributed disclosure. S15 is complete. M26/D11 remain open
  for real account/quota/billing telemetry and live-provider acceptance.
  PDF preview metadata now discloses whether Poppler reports no form, an AcroForm,
  XFA, or an unrecognized form technology from the same immutable snapshot.
  Form data stays read-only and scripts are never run. S18 remains open for safe
  field inspection/editing/submission rather than pretending metadata is interaction.
  PPTX joins the bounded document pipeline: only ordered slide XML is opened,
  DrawingML text is extracted as labeled slide context, and relationships, notes,
  media, macros and embedded objects are ignored. Composer and Studio share the
  same read-only extraction. XLSX now follows the same inert path: bounded
  worksheet/shared-string XML yields coordinate-labeled cached values, formulas
  are never evaluated, and workbook relationships, macros, charts and embedded
  objects are ignored. ODP and ODS use the same OpenDocument safety boundary:
  only bounded content.xml is opened, presentation pages are labeled, spreadsheet
  cells expose visible/cached values, formulas are ignored, and embedded objects
  are never read. M29 remains open for richer page/slide/sheet rendering and
  additional document formats.
- Batch 19: reviewed provider handoff can now continue in the same TaskId/ThreadId
  as an explicit alternative to creating a related conversation. The transaction
  rechecks source transcript, workspace and route identity, refuses a nonempty
  source draft or pending attachments, atomically replaces ACP/direct route state,
  invalidates the old saved session and persists the reviewed continuation as a
  visible unsent draft. No prompt is sent and files/Git state are untouched.
  Changing the route after review makes the review stale. S07 is complete; D12
  remains open for provider-native forks and live provider/worktree acceptance.

Verification receipts:
[batch 1](docs/verification/parity-2026-09-24-batch1.md),
[batch 2](docs/verification/parity-2026-09-24-batch2.md),
[batch 3](docs/verification/parity-2026-09-24-batch3.md),
[batch 4](docs/verification/parity-2026-09-24-batch4.md),
[batch 5](docs/verification/parity-2026-09-24-batch5.md),
[batch 6](docs/verification/parity-2026-09-24-batch6.md),
[batch 7](docs/verification/parity-2026-09-24-batch7.md),
[batch 8](docs/verification/parity-2026-09-24-batch8.md),
[batch 9](docs/verification/parity-2026-09-24-batch9.md),
[batch 10](docs/verification/parity-2026-09-24-batch10.md),
[batch 11](docs/verification/parity-2026-09-24-batch11.md),
[batch 12](docs/verification/parity-2026-09-24-batch12.md),
[batch 13](docs/verification/parity-2026-09-25-batch13.md),
[batch 14](docs/verification/parity-2026-09-25-batch14.md),
[batch 15](docs/verification/parity-2026-09-25-batch15.md),
[batch 16](docs/verification/parity-2026-09-25-batch16.md),
[batch 17](docs/verification/parity-2026-09-25-batch17.md),
[batch 18](docs/verification/parity-2026-09-25-batch18.md),
[batch 19](docs/verification/parity-2026-09-25-batch19.md).

## How to update this roadmap

When a remaining item ships:
1. Check its box.
2. Increment **Shipped feature slices** by one.
3. Decrement the matching remaining count and **Total remaining**.
4. Add the feature to the latest batch receipt.
5. Keep the broader verification gate OPEN until its full workflow and required
   provider/platform failure paths are actually accepted.

Do not use the 21-gate count as the feature count.
