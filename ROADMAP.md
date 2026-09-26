# Synara parity roadmap

This file is intentionally short. Use it for **what is done, what remains, and what comes next**.

Detailed parity evidence lives in:
- [Current parity gates](docs/verification/current-parity-gates.md)
- [Electron vs GPUI feature gap](docs/ui/electron-vs-gpui-feature-gap.md)
- [Archived detailed roadmap](docs/history/roadmap-before-simplification-2026-09-24.md)

## Current status

- Shipped feature slices: **88**
- Major remaining: **8**
- Smaller remaining: **0**
- Acceptance/integration remaining: **2**
- Total remaining: **10**
- Completely missing top-level surfaces: **0**
- Broad verification gates still open: **19**

The **10-item count is the execution count to use going forward**. The 19 open verification
gates are larger acceptance buckets and are not a feature count.

Current upstream reference: `Emanuele-web04/synara@eaa61eded31b6755d4f30ba8eabc5d905cf817cb`.

Current continuation status: A05 authenticated browser acceptance passed on exact
candidate `163d59cf1eaba301e413f1d02848a4d4cb397f69`, and A09 provider
interoperability passed on `fd41caa5d7077d9176afe749b2c2fe3f0cb57c03`.
The mainline M01/M02/M06/M13/M15 implementations are also incorporated. A08
signed release package acceptance passed across Linux, macOS and Windows on
candidate `5e006f08d31a2ca7eea97e7b7d0c0d0052011cb6`. A04 then passed a
fresh-install real GitHub Copilot ACP journey on candidate
`a51a87fa684f26de8616ab04c4a7cbd78d79c351`, closing D1 as well.
A01 and A10 remain the final acceptance items.

## Major features remaining

- [x] M01 Web workspace provider connection/sign-in
- [x] M02 Rich web approval context for tools, commands and diffs
- [x] M03 Durable web question drafts across navigation/restart
- [x] M04 Direct-model execution from the web workspace
- [x] M05 Remote workspace execution
- [x] M06 Production headless deployment with bind/TLS/update packaging
- [ ] M07 Protected browser session/cookie import
- [x] M08 Complete browser popup authentication lifecycle
- [ ] M09 Agent-controlled browser upload/download
- [x] M10 Browser console/runtime diagnostics
- [ ] M11 Safe restoration of task/auth browser sessions
- [ ] M12 Page-declared WebMCP integration
- [x] M13 Simulator live frame streaming
- [ ] M14 Simulator touch, swipe, typing and hardware-button input
- [x] M15 Simulator recording
- [ ] M16 Simulator accessibility tree and semantic element targeting
- [x] M17 Editor syntax highlighting
- [x] M18 Advanced editor conflict recovery
- [x] M19 Richer editor comparison scopes
- [x] M20 Safe line blame without repository filter execution
- [x] M21 Deeper diff editing/review workflows
- [x] M22 Managed worktree automatic cleanup/recovery
- [x] M23 SSH managed worktree creation
- [x] M24 Environment-aware task/fork orchestration
- [x] M25 Richer model/context controls, including fast/thinking presets and compaction
- [x] M26 Real provider/account telemetry integration
- [ ] M27 Broader Computer Use actions, targeting and preview behavior
- [ ] M28 Trusted signed updater/install/rollback lifecycle
- [x] M29 Binary PDF/document attachment pipeline and broader document viewing
- [x] M30 Studio historical output versioning and long-running lifecycle

## Smaller features remaining

- [x] S01 Provider-specific onboarding setup/login UX
- [x] S02 Voice interaction controls beyond record/transcribe-to-draft
- [x] S03 Direct-model keyboard cycling
- [x] S04 Context-aware custom keybindings
- [x] S05 Additional native slash-command argument forms
- [x] S06 Deeper automation orchestration semantics
- [x] S07 Same-task handoff continuation semantics
- [x] S08 Provider-native fork actions
- [x] S09 Persistent folder references
- [x] S10 Richer structured metadata in thread export
- [x] S11 Reply/context reuse into the current composer
- [x] S12 Reply/context reuse into side/new tasks
- [x] S13 Exact upstream project-search ranking
- [x] S14 Exact ignored/generated-file search behavior
- [x] S15 Per-turn provider/model activity breakdown
- [x] S16 Token heatmap
- [x] S17 Studio organization/filtering polish
- [x] S18 PDF text/link/form interaction after basic rendering

## Acceptance/integration remaining

These are primarily proof on real providers, platforms or release infrastructure,
not large new product subsystems.

- [ ] A01 Live microphone + ChatGPT transcription end-to-end acceptance
- [x] A02 macOS microphone packaging/permission acceptance
- [x] A03 Windows voice/package acceptance
- [x] A04 Fresh-install onboarding with real provider accounts
- [x] A05 Authenticated browser login/session/popup acceptance
- [x] A06 Real macOS Simulator/device acceptance
- [x] A07 SSH worktree/search acceptance
- [x] A08 Signed release feed/install/rollback package acceptance
- [x] A09 Multi-provider ACP/direct-model interoperability matrix
- [ ] A10 Cross-platform visual/accessibility/save-picker acceptance

## Next execution queue

Finish whole workflows instead of spreading work across every gate:

1. **Browser/auth:** M07, M09, M11-M12
2. **Simulator:** M14, M16 when the required macOS/native input backend is available
3. **Computer Use:** M27
4. **Releases/updater:** M28

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
  or locked checkouts, and retains the generated branch. The same environment
  chooser now offers one reviewed bulk cleanup when multiple recoverable Synara
  worktrees exist: it fixes the reviewed set, revalidates each exact checkout,
  removes only still-safe clean worktrees, and reports every retained dirty,
  assigned, stale, locked or cancelled checkout without force or branch deletion.
  Managed worktree task creation now atomically persists an exact ownership marker.
  Deleting an archived managed task automatically attempts ordinary non-force
  cleanup of only that marked checkout, for local or pinned SSH workspaces.
  Dirty/locked/stale failures retain both checkout and marker; project/workspace
  deletion retries those durable markers and refuses to erase ownership metadata
  while any checkout remains unsafe. Ordinary user-managed worktrees never receive
  a marker and are never auto-removed. Generated branches remain intact. M22 is
  complete at the product-feature level; A07 remains the live SSH acceptance gate.
  The worktree fork menu is now a unified environment chooser: it includes the
  current local or SSH workspace, existing linked worktrees, recoverable Synara
  worktrees, and reviewed new worktrees. SSH managed creation now derives an
  exact UUID sibling checkout from Git's canonical remote worktree root, executes
  worktree list/add/recovery only through the pinned SSH host, rechecks source
  HEAD/branch/path before mutation, and uses the existing remote filesystem owner
  to validate the new task directory. No arbitrary remote destination, ambient
  SSH config, credentials, hooks, network helper or source dirty file is copied.
  Failed task persistence remains recoverable from the exact unassigned
  `synara/<uuid>` worktree. M23 is complete; A07 remains open for live SSH
  acceptance. M24 remains open for broader orchestration policy across task
  creation and provider handoff.
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
  checked. Horizontal scrolling is now a separate typed action using the same
  reviewed window-relative point and 1–8 step bound; it maps only to X11 buttons
  6/7 and retains fresh-window revalidation before each command. Reviewed drag
  now adds bounded start/end coordinates with fresh-window validation before each
  pointer step; if a later step fails after mouse-down, Synara best-effort releases
  the same window-addressed button to avoid persistent input state. M27 remains
  open for richer targeting and broader platform behavior.
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
  Headless scheduling is now also available as an explicit
  `synara-server --automations` owner using the same durable scheduler under the
  workspace process lock. Definitions support upstream-shaped Standalone,
  Heartbeat and Dedicated execution modes. Heartbeat continues a reviewed
  existing ACP task; Dedicated creates one automation-owned task on first run
  and reuses it. Continuation runs defer rather than consume a scheduled slot
  while the target is active, has a draft/attachments, is automation-owned
  elsewhere, or is inside its configurable activity cooldown; the automation's
  own last completed run does not self-throttle. S06 remains open for the
  upstream AI-evaluated completion policy and wider live-provider lifecycle
  acceptance.
  Durable Studio text-version history for a selected file can now be cleared
  through a two-step native confirmation. The operation is task/path-scoped,
  preserves the workspace file and histories for other files, and stale replies
  are ignored by task/path/generation checks. An exact selected durable version
  can also be saved to a new destination after task/path/generation/selection
  revalidation; the backend rechecks that the snapshot still exists and never
  reads or rewrites the current workspace file. Durable snapshots can now also
  be pinned. Automatic bounded retention evicts only unpinned versions and fails
  closed when pinned history fills the retention budget; explicit clear remains
  the only operation that can remove pinned entries. M30 remains open for broader
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
  remains open for live provider/worktree acceptance.

- Batch 20 (in progress): the editor comparison panel now has a Changes only
  review mode that hides unchanged rows while preserving each row's original diff
  index and old/new line numbers. Restore block and Copy block therefore keep the
  same owner, generation, exact-line and exact-current-diff rechecks; filtering is
  presentation-only and never edits the buffer or Git index. A change-block cursor
  now wraps across only real changed-block starts, highlights the selected block,
  and exposes toolbar Copy/Restore actions that delegate to those same guarded
  operations. Any buffer or comparison refresh clears the selection. Together
  with saved/disk/ref scopes, full-review copy, block restore/copy, restore-all
  and guarded three-way conflict merge, this completes M21 at the product-feature
  level. Partial Git staging remains deliberately excluded because no current
  owner can protect a pre-existing index safely.
- Batch 21 (in progress): S06 now has an explicit non-GPUI scheduler owner via
  `synara-server --automations`, plus durable Standalone, Heartbeat and Dedicated
  execution modes and a bounded continuation cooldown. Heartbeat/dedicated target
  reuse is task-owned, ACP-only, project/agent-checked, draft/attachment-safe and
  cross-automation-exclusive. Scheduled target contention/cooldown defers without
  consuming the due slot; manual Run now reports the blocking condition. This
  left S06 open only for the pinned upstream AI-evaluated completion policy and
  live-provider lifecycle acceptance.

- Batch 23: ACP provider-native session fork is now an explicit whole-session
  action. Synara enables only the pinned SDK's `unstable_session_fork` feature,
  trusts only advertised `sessionCapabilities.fork`, and additionally requires
  load/resume recovery support. The ordinary retained-context child is committed
  first; native fork then attaches the provider copy to that child when successful.
  Unsupported, rejected or ambiguous fork never auto-retries and leaves exactly
  that retained-context child intact. The source provider session is unchanged.
  S08 is complete; D12 remains open for live-provider/worktree acceptance.

- Batch 25: M21 and M24 are complete at the product-feature level. Editor review
  now has bounded saved/disk/ref scopes, changes-only filtering, whole-review and
  selected-block copy, selected-block/restore-all buffer edits, changed-block
  navigation and guarded three-way conflict merge. No Git staging was added
  because the current API cannot prove preservation of a pre-existing index.
  Environment-aware fork orchestration now covers current local/SSH workspaces,
  existing and recoverable worktrees, reviewed managed local/SSH creation,
  same-task provider continuation and capability-gated provider-native forks.
  M22 remains separately open for automatic managed-worktree cleanup/recovery,
  and A07/A09 remain the relevant live transport/provider acceptance work.

- Batch 26: S06 is complete at the product-feature level. Automations can now
  opt into a reviewed direct-model stop evaluator with a bounded stop condition
  and confidence threshold. The evaluator is completely separate from the ACP
  automation task: it receives only the saved policy, automation instructions,
  exact submitted run prompt and run-scoped assistant output, has no tools or
  hidden provider/task state, requests strict JSON, times out after 30 seconds,
  and never retries automatically. Failed/timed-out checks are retained as run
  metadata and do not fail or pause the automation. A positive evaluation pauses
  only when the exact saved policy revision is still current; edits/re-enable
  operations fence late results. D8 remains OPEN for live provider/restart,
  shutdown and multi-platform acceptance.

- Batch 27: M30 is complete at the product-feature level. Studio now keeps a
  bounded durable history of text outputs across restart, captures newly reported
  UTF-8 outputs automatically when a completed tool reports them, and stores the
  exact reporting task/turn/timestamp with each historical snapshot. Manual
  preview capture remains available for ordinary text files. Versions can be
  inspected, copied, pinned/unpinned, exported to a new destination and explicitly
  cleared without rewriting the workspace file. Automatic retention evicts only
  unpinned history and fails closed when pinned versions consume the budget.
  Live-provider/platform acceptance remains part of the broad Studio gate rather
  than a reason to keep the product feature open.

- Batch 28: M29 is complete at the product-feature level. Explicit binary
  document intake now covers PDF, DOCX, ODT, ODP, ODS, PPTX and XLSX with bounded
  format-specific validation and inert text projection for prompt context.
  Studio reuses those extractors, keeps original-file export separate, and adds
  immutable PDF page rendering, page/document text extraction, safe HTTP(S) link
  inspection/opening and read-only form-technology disclosure. OOXML/ODF
  relationships, macros, scripts, external resources and embedded objects are
  never executed or followed, and spreadsheet formulas are never evaluated.
  S18 remains open specifically for a safe PDF field-level interaction contract;
  D13/A10 remain open for broad failure/package/cross-platform acceptance.

- Batch 31: S18 is complete at the product-feature level. Studio keeps the PDF
  source as an immutable snapshot, exposes page/document text and explicit
  HTTP(S) link opening, offers opt-in OCR through the fixed system Tesseract
  helper, and allows a reviewed safe subset of AcroForm text/button/choice
  fields to be edited locally before an explicit new-copy export. Password,
  file-select, rich-text/comb, push-button, multi-select, signature, XFA and
  unknown/read-only fields remain inspection-only or inert. The fill path
  revalidates fields against the snapshot, uses bounded XFDF/pdftk helpers,
  round-trips requested values before publishing, never executes scripts or
  SubmitForm actions, and writes only through the existing no-overwrite export
  owner. D13/A10 remain open for broad failure/package/cross-platform acceptance.

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
[batch 19](docs/verification/parity-2026-09-25-batch19.md),
[batch 20](docs/verification/parity-2026-09-25-batch20.md),
[batch 21](docs/verification/parity-2026-09-25-batch21.md),
[batch 22](docs/verification/parity-2026-09-25-batch22.md),
[batch 23](docs/verification/parity-2026-09-25-batch23.md),
[batch 24](docs/verification/parity-2026-09-25-batch24.md),
[batch 25](docs/verification/parity-2026-09-25-batch25.md),
[batch 26](docs/verification/parity-2026-09-25-batch26.md),
[batch 27](docs/verification/parity-2026-09-25-batch27.md),
[batch 28](docs/verification/parity-2026-09-25-batch28.md),
[batch 29](docs/verification/parity-2026-09-25-batch29.md),
[batch 30](docs/verification/parity-2026-09-25-batch30.md),
[batch 31](docs/verification/parity-2026-09-25-batch31.md),
[batch 32](docs/verification/parity-2026-09-25-batch32.md),
[batch 33](docs/verification/parity-2026-09-25-batch33.md),
[batch 34](docs/verification/parity-2026-09-25-batch34.md),
[batch 35 - Simulator](docs/verification/parity-2026-09-25-batch35.md),
[batch 35 - provider/browser](docs/verification/parity-2026-09-26-batch35.md),
[batch 36](docs/verification/parity-2026-09-26-batch36.md),
[batch 37](docs/verification/parity-2026-09-26-batch37,
[batch 38](docs/verification/parity-2026-09-26-batch38.md).md).

- Batch 32: acceptance infrastructure now has a dedicated Linux/macOS/Windows
  native build and development-package matrix. macOS development packaging uses
  a real `Synara.app` bundle layout and declares `NSMicrophoneUsageDescription`
  for explicit user-initiated recording. The matrix preserves exact-candidate
  evidence and runs deterministic package/inventory checks on each platform.
  A02, A03 and A10 remain open for live microphone, installed-package, visual,
  accessibility and save-picker acceptance. A08 remains open because these
  artifacts are intentionally unsigned development packages.

- Batch 33: A02 and A03 are accepted. Hosted macOS arm64 and Windows x64
  runners both pass the native voice regression suite, native application build,
  dependency inventory and deterministic package creation. macOS additionally
  validates the generated Synara.app Info.plist with the system plist tools,
  requires NSMicrophoneUsageDescription and verifies the packaged executable.
  Windows expands the generated package and verifies synara-app.exe plus the
  exact target/development manifest. The later A07-only smoke edits do not touch
  voice or platform packaging. A01 remains the separate live microphone +
  ChatGPT transcription end-to-end gate.


### September 26 continuation

- Batch 35: M08 is complete at the product-feature level. Provider website
  requests now open in request-owned authentication profiles, cookies stay
  isolated from manual and agent-task browsing, and reviewed popup children
  retain the exact authentication partition. Popup navigation is withheld until
  explicit host review, and request completion, cancellation or expiry closes the
  owned authentication flow. A05 remains open because the branch-head real
  WebKit journey still fails to deliver the trusted X11 click to the auth page.
- A09 is accepted on exact candidate
  `fd41caa5d7077d9176afe749b2c2fe3f0cb57c03`. The dedicated native provider
  lane passed external OpenCode and Gemini CLI ACP initialization probes plus
  representative Google and Anthropic direct-model journeys, ACP/direct route
  switching and task/conversation ownership checks. This supplies the deciding
  evidence for N5, which is now PASS. See the
  [batch 35 receipt](docs/verification/parity-2026-09-26-batch35.md).
- Batch 36: A05 is accepted on exact candidate
  `163d59cf1eaba301e413f1d02848a4d4cb397f69`. The dedicated authenticated
  browser lane passed the partition/authority unit contract and a real
  WebKitGTK/Xvfb login journey covering cookie-backed session continuity,
  pre-navigation popup review, same-flow popup ownership, opener callback/form
  POST behavior, isolation from manual/other-auth profiles, final-tab cleanup,
  formatting and strict Clippy. D2 remains OPEN because M07, M09, M11 and M12
  are still product-depth gaps. A01 also gained a manual-only live acceptance
  workflow for a physical microphone plus a ChatGPT-authenticated Codex session.
  See the [batch 36 receipt](docs/verification/parity-2026-09-26-batch36.md).


- Batch 37: six product features complete on the mainline implementation: M01
  adds explicit task-owned web provider connection/reconnection and advertised
  authentication. M02 adds live bounded tool input, proposed diffs and exact-
  context approval receipts. M06 adds an explicit HTTPS origin, local TLS proxy/
  service configuration and versioned Linux headless packaging, activation and
  rollback. M08 adds request-owned private sign-in tabs and reviewed native
  popups; the Rust branch subsequently hardened this path and passed A05. M13
  adds bounded latest-frame Simulator streaming, and M15 adds explicit MOV
  recording with stop/save/discard and navigation cancellation. Computer Use
  also gains screenshot targeting, drag, scroll, typing/key controls, window
  filtering and takeover; M27 remains open for broader platform support. See the
  [batch 37 receipt](docs/verification/parity-2026-09-26-batch37.md).


## How to update this roadmap

When a remaining item ships:
1. Check its box.
2. Increment **Shipped feature slices** by one.
3. Decrement the matching remaining count and **Total remaining**.
4. Add the feature to the latest batch receipt.
5. Keep the broader verification gate OPEN until its full workflow and required
   provider/platform failure paths are actually accepted.

Do not use the broad verification gate count as the feature count.
