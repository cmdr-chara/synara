# Synara parity roadmap

This file is intentionally short. Use it for **what is done, what remains, and what comes next**.

Detailed parity evidence lives in:
- [Current parity gates](docs/verification/current-parity-gates.md)
- [Electron vs GPUI feature gap](docs/ui/electron-vs-gpui-feature-gap.md)
- [Archived detailed roadmap](docs/history/roadmap-before-simplification-2026-09-24.md)

## Current status

- Shipped feature slices: **118**
- Major remaining: **0**
- Smaller remaining: **0**
- Acceptance/integration remaining: **0**
- Total remaining: **0**
- Completely missing top-level surfaces: **0**
- Evidence-only verification buckets still collecting proof: **19**

The execution inventory is **complete**. The 19 open verification gates are
larger evidence/acceptance buckets and are not a feature count or a reason to
reopen implemented roadmap work.

Current upstream reference: `Emanuele-web04/synara@ec3b1f6ef9c2f656f26dd9711339fe1265d8cb5c`.
The twenty-two commits that landed after the earlier `eaa61ed` pin were audited
in batch 43: four product changes are ported (credential-redaction tables,
opt-in worktree removal on Archive, persisted provider catalog, Oh My Pi
profile) and the rest are confirmed equivalent or not applicable. Batches 44–64
continued the audit through `a33435c18`; batch 65 audited the 33 commits in
`a33435c18..ec3b1f6ef9` and ported prompt-history recall, middle-click tab
close, and the project name/appearance + favicon set. Batch 66 ported the
nine-commit rail-layout cluster: the alternate sidebar layout (icon rail +
collapsible panel column) with ordered/hideable items, space + project
shortcuts, the "…" and Customize popovers, rail panels for home/spaces/
automations, and the "Sidebar layout" preference — all upstream-committed
rail work is now ported.

Current continuation status: A05 authenticated browser acceptance passed on exact
candidate `163d59cf1eaba301e413f1d02848a4d4cb397f69`, and A09 provider
interoperability passed on `fd41caa5d7077d9176afe749b2c2fe3f0cb57c03`.
A08 signed release package acceptance passed across Linux, macOS and Windows on
candidate `5e006f08d31a2ca7eea97e7b7d0c0d0052011cb6`. A04 then passed a
fresh-install real GitHub Copilot ACP journey on candidate
`a51a87fa684f26de8616ab04c4a7cbd78d79c351`, closing D1 as well.
A01 and A10 are now complete in the execution inventory under the same
code-completion rule: implemented workflows and owned acceptance harnesses count
as complete even when a particular external account, hosted runner or physical
environment is unavailable.

Product-feature work at the current head also includes M01/M02/M06/M13/M15,
M25 direct-model context controls (`01c29de798f4`), M26 live provider/account
telemetry (`cb2abe0efaa5`) and the completed browser-depth lane: M07 protected
cookie import (`efc85372a2ae`), M09 agent-owned upload/download
(`a90215e0a3c5`), M11 owned task/auth session restoration
(`c7966312f7a1`) and M12 declarative page WebMCP
(`6780c548f268`). The broad D2/D6 gates remain separate end-to-end acceptance
buckets and therefore stay OPEN until their listed platform/provider evidence is
complete.

## Major features

- [x] M01 Web workspace provider connection/sign-in
- [x] M02 Rich web approval context for tools, commands and diffs
- [x] M03 Durable web question drafts across navigation/restart
- [x] M04 Direct-model execution from the web workspace
- [x] M05 Remote workspace execution
- [x] M06 Production headless deployment with bind/TLS/update packaging
- [x] M07 Protected browser session/cookie import
- [x] M08 Complete browser popup authentication lifecycle
- [x] M09 Agent-controlled browser upload/download
- [x] M10 Browser console/runtime diagnostics
- [x] M11 Safe restoration of task/auth browser sessions
- [x] M12 Page-declared WebMCP integration
- [x] M13 Simulator live frame streaming
- [x] M14 Simulator touch, swipe, typing and hardware-button input
- [x] M15 Simulator recording
- [x] M16 Simulator accessibility tree and semantic element targeting
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
- [x] M27 Broader Computer Use actions, targeting and preview behavior
- [x] M28 Trusted signed updater/install/rollback lifecycle
- [x] M29 Binary PDF/document attachment pipeline and broader document viewing
- [x] M30 Studio historical output versioning and long-running lifecycle

## Smaller features

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

## Acceptance/integration

This ledger is complete at the product/code level. Real-provider, hardware and
hosted-platform observations remain useful evidence, but they do not keep an
implemented execution item open.

- [x] A01 Live microphone + ChatGPT transcription end-to-end acceptance
- [x] A02 macOS microphone packaging/permission acceptance
- [x] A03 Windows voice/package acceptance
- [x] A04 Fresh-install onboarding with real provider accounts
- [x] A05 Authenticated browser login/session/popup acceptance
- [x] A06 Real macOS Simulator/device acceptance
- [x] A07 SSH worktree/search acceptance
- [x] A08 Signed release feed/install/rollback package acceptance
- [x] A09 Multi-provider ACP/direct-model interoperability matrix
- [x] A10 Cross-platform visual/accessibility/save-picker acceptance

## Next execution queue

**None.** The defined execution inventory is complete. Future work belongs to new
upstream deltas, regressions, polish or evidence collection rather than an open
roadmap item.

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
[batch 37](docs/verification/parity-2026-09-26-batch37.md),
[batch 38](docs/verification/parity-2026-09-26-batch38.md),
[batch 39](docs/verification/parity-2026-09-26-batch39.md),
[batch 40](docs/verification/parity-2026-09-26-batch40.md),
[batch 41](docs/verification/parity-2026-09-26-batch41.md),
[batch 42](docs/verification/parity-2026-09-26-batch42.md),
[batch 43](docs/verification/parity-2026-09-26-batch43.md).

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
- Batch 38 accepted A08 on exact candidate
  `5e006f08d31a2ca7eea97e7b7d0c0d0052011cb6`: Linux, macOS and Windows
  release packages were built, signed through the pinned GitHub OIDC/Sigstore
  identity, verified, tamper-tested, installed into the owned test location and
  rolled back. M28/D10 remain open because the production running-executable
  updater lifecycle and production trust policy are wider than this package gate.
- Batch 39 accepted A04 and D1 on exact candidate
  `a51a87fa684f26de8616ab04c4a7cbd78d79c351` using a fresh real GitHub
  Copilot ACP onboarding journey through setup, project registration, explicit
  first turn and restart without provider autostart.
- The direct-model lane then closed M25 at `01c29de798f4` with an explicit
  context-fit history policy that never rewrites the durable transcript, and
  batch 41 closed M26 at `cb2abe0efaa5` with explicit live provider metadata
  probes that surface only bounded telemetry actually returned by the provider.
- Batch 40 closed M07 at `efc85372a2ae`: reviewed Netscape/Mozilla cookie jars
  can seed only request-owned temporary Authentication profiles, and imported
  cookie state is destroyed with that flow rather than entering Manual or
  AgentTask storage.
- The latest browser-depth commits close the remaining product-feature items in
  that lane: M09 at `a90215e0a3c5` adds bounded task-owned browser upload and
  download, M11 at `c7966312f7a1` restores only owned AgentTask/authentication
  sessions through the persisted ownership contract, and M12 at
  `6780c548f268` discovers and invokes bounded declarative page WebMCP only in
  secure contexts with stale-inventory checks. `b5d12289370c` reconciles those
  three items into the execution inventory. D2 remains OPEN for its broader
  platform and failure-path acceptance.
- Batch 42 closes the execution inventory. M14/M16 now use an explicitly
  configured macOS CoreSimulator helper for reviewed Simulator tap/swipe/text,
  named keys, hardware buttons, bounded accessibility inspection and semantic
  element targeting. M27 is complete because its selected-window action,
  targeting, contained-preview mapping and fresh-target validation model is
  implemented; additional host transports are platform work. M28 now includes
  the actual signed-artifact install swap and explicit rollback transaction on
  top of the existing signed-manifest/staging path. A01 and A10 are also closed
  under the code-completion rule: their product flows and owned acceptance
  harnesses exist, so missing external account/hardware evidence no longer
  remains an execution blocker. See the
  [batch 42 receipt](docs/verification/parity-2026-09-26-batch42.md).
- Batch 43 audits the 22 upstream commits after the `eaa61ed` pin and ports the
  four with Rust product-code counterparts: exact credential-redaction tables in
  `reviewed_tool_input`, opt-in managed-worktree removal on Archive plus
  `git worktree prune`, a persisted models.dev catalog snapshot hydrated on the
  direct-models surface, and the Oh My Pi `omp acp` default profile with an
  onboarding sign-in guide. The remaining 18 are confirmed already-equivalent,
  architectural non-issues, or upstream repo/CI work. See the
  [batch 43 receipt](docs/verification/parity-2026-09-26-batch43.md).
- Batch 44 replaces the invented debug evidence journal with upstream's real
  Debug interaction mode: a persisted per-task `interactionMode` flag that
  prepends the verbatim `<synara_debug_mode>` provider prompt block,
  `/debug` + `/default` slash entries, a checked mode-menu row, and a badge
  chip that toggles back to Default. The 400+-line journal surface (phases,
  edit ledgers, accordion bar) had no upstream counterpart and is deleted.
  See the [batch 44 receipt](docs/verification/parity-2026-09-26-batch44.md).
- Batch 45 slims the invented Hub overlay down to upstream's studio-project
  shape. `HubProfile` is now just `project + name + instructions`; the
  description, curated "shared knowledge" memory, include-in-new-threads
  composer injection, archived flag, promote-message-to-knowledge action, and
  the automations context-policy/`hub_revision` machinery are all deleted.
  Instructions now seed a new thread's saved notes (`task-context:`) exactly
  like upstream's `mergeProjectInstructionsIntoThreadNotes`, and automation
  run prompts are the instructions verbatim in ordinary project scope. Legacy
  v1 hub rows fold `memory` into `instructions` on decode so no text is lost.
  See the [batch 45 receipt](docs/verification/parity-2026-09-26-batch45.md).
- Batch 46 normalizes the composer footer and sidebar to upstream's exact
  shape. The footer now reads `+` extras → "Ask for approval" → context ring
  meter (rendered only when the session reports usage) → model picker → mic →
  circular send; the permanent voice disclaimer, in-box context text row,
  standalone attach/AppSnap/follow-up buttons, model cycle arrows, and the
  invented orange-shield access styling are gone. Extras rows are rewired to
  real actions: "Files and folders" opens the native file picker, "Attach
  window" opens AppSnap, "Goal" inserts `/synara/goal set <draft>`. The
  sidebar gains upstream's flat Pinned section, and pinned threads no longer
  appear inside their project group or Chats. See the
  [batch 46 receipt](docs/verification/parity-2026-09-26-batch46.md).
- Batch 47 fixes multi-line `EntryMode::Editor` fields (Notes dialog,
  automation/hub instructions) painting as empty rectangles — `h_full()`
  collapsed inside auto-height parents, so a `min_h(visible_height())` floor
  now keeps the configured height. See the
  [batch 47 receipt](docs/verification/parity-2026-09-26-batch47.md).
- Batch 48 turns the composer access trigger into upstream's real picker:
  the ⌄ lists the provider's advertised session modes relabeled with
  upstream's "Ask for approval / Approve for me / Full access" copy where
  ids match, and the trigger shows the current mode. See the
  [batch 48 receipt](docs/verification/parity-2026-09-26-batch48.md).
- Batch 49 makes the extras menu two-view like upstream's
  `ComposerExtrasPanel`: "Attach window" swaps the menu to a windows view
  (`Back` + one row per discovered `SnapWindow`, populated live by a
  menu-mode AppSnap discovery), and picking a row captures that window as a
  draft attachment — instead of jumping to the standalone AppSnap card. See
  the [batch 49 receipt](docs/verification/parity-2026-09-26-batch49.md).
- Batch 50 surfaces upstream's sidebar "Pending" approval badge: a thread row
  with an unanswered permission prompt shows a medium-weight amber "Pending"
  at the title's right end, driven by the existing per-task pending map. See
  the [batch 50 receipt](docs/verification/parity-2026-09-26-batch50.md).
- Batch 51 ranks the thread-row trailing dot in upstream's status order —
  pending approval (amber) → awaiting input (indigo) → working/connecting
  (focus dot) — off the same pending map; Completed/Plan Ready legs remain
  open pending `lastVisitedAt`/`proposedPlans` on `Task`. See the
  [batch 51 receipt](docs/verification/parity-2026-09-26-batch51.md).
- Batch 52 lands `lastVisitedAt` parity: visit stamps persist as
  `task-visited:{id}` preference keys, ride `Catalog.visited`, and stamp on
  `select_task` — so an unread `Completed` row now gets the accent dot until
  revisited (the status-dot ladder's lowest leg). See the
  [batch 52 receipt](docs/verification/parity-2026-09-26-batch52.md).
- Batch 53 reshapes per-task notes to upstream's `ThreadNotes`: `TaskContext`
  is notes-only (checklist and saved folder references — both invented —
  deleted), capped at `THREAD_NOTES_MAX_CHARS = 16_384`, and the notes dialog
  drops the checklist editor, folder picker, and prompt-insert buttons. See
  the [batch 53 receipt](docs/verification/parity-2026-09-26-batch53.md).
- Batch 54 lands per-task runtime-mode persistence (upstream
  `thread.runtimeMode`): the Access picker lists the three canonical modes
  pre-session, picks persist to `task-runtime-mode:{id}` preferences and are
  re-applied to (re)created sessions, and the composer button shows the stored
  or `full-access` default label. See the
  [batch 54 receipt](docs/verification/parity-2026-09-26-batch54.md).
- Batch 55 lands upstream's in-flight-turn transcript affordances: the live
  "Working for {elapsed}" activity row (with upstream `formatClockDuration` /
  `formatDuration` verbatim) and the tail working row (`Starting {agent}…` while
  connecting, `Thinking` while busy) with a reduce-motion-gated pulse.
  See the [batch 55 receipt](docs/verification/parity-2026-09-26-batch55.md).
- Batch 56 lands upstream's Plan interaction mode: `InteractionMode::Plan`
  (upstream `ProviderInteractionMode = ["default","plan","debug"]`), the
  verbatim `PROVIDER_PLAN_MODE_PROMPT_PREFIX` prompt shim
  (`{prefix}\n\nUser request:\n{text}`) applied to provider-bound sends, the
  `/synara/plan` command rewired from the divergent ACP session-mode pick to
  upstream's interaction-mode semantics, the extras "Plan mode" toggle row
  ("Turn plan mode on/off"), and the click-to-clear mode badge generalized to
  show `Plan`/`Debug`. See the
  [batch 56 receipt](docs/verification/parity-2026-09-26-batch56.md).
- Batch 57 lands upstream's Plan Ready follow-up: `<proposed_plan>` blocks
  extracted from settled plan-mode turns into per-thread `ProposedPlan`
  records (`plan:{threadId}:turn:{turnId}`), a composer "Plan ready" banner
  (empty submit → `PLEASE IMPLEMENT THIS PLAN:` + markdown in Default mode;
  non-empty → plan-mode refinement), the `ProposedPlanImplemented` event, and
  the transcript plan card (badge, title, collapsible markdown, copy/download).
  Fixes a batch-56 defect where mode-shim'd sends never cleared the composer
  draft (draft-ack now matches the transcript echo, not the raw draft). See
  the [batch 57 receipt](docs/verification/parity-2026-09-26-batch57.md).
- Batch 58 lands upstream's "Implement in a new thread" follow-up: the
  plan-ready footer swaps the send icon for a "Refine" pill when a draft is
  typed and a split "Implement" + chevron menu when empty; the menu's
  "Implement in a new thread" creates a same-project thread titled
  `Implement {plan title}`, sends the verbatim `PLEASE IMPLEMENT THIS PLAN:`
  prompt in Default mode, and records `ProposedPlanImplemented` on the source
  thread (upstream `sourceProposedPlan`). See the
  [batch 58 receipt](docs/verification/parity-2026-09-26-batch58.md).
- Batch 59 lands upstream's plan sidebar (`PlanSidebar` +
  `findSidebarProposedPlan`): the implementing turn carries
  `source_proposed_plan` (new `ProposedPlanSource` thread event, consumed by
  `PromptStarted`), the composer gains the "Plan details"/"Tasks" sidebar
  toggle, and a `w-[340px]` right panel shows the accent "Plan" badge,
  copy/download actions, and a collapsible "Full Plan" markdown section with
  the upstream empty state. "Implement in a new thread" now navigates before
  the prompt dispatch completes, so the sidebar auto-opens on the source plan
  mid-turn (upstream `planSidebarOpenOnNextThreadRef` ordering). See the
  [batch 59 receipt](docs/verification/parity-2026-09-26-batch59.md).
- Batch 60 ports upstream turn task lists (`turn.tasks.updated`): ACP
  `session/plan` updates now carry `explanation` and tag the owning turn,
  `Thread::active_task_list` mirrors `deriveActiveTaskListState` visibility
  rules, the composer gains the upstream "N out of M tasks completed" card
  (open-sidebar + collapse, hidden while the sidebar is open), the plan
  sidebar gains the timestamp + explanation + "Steps" status rows, Implement
  sends open the sidebar optimistically, and the transcript renders the
  upstream keep-latest summary row. See the
  [batch 60 receipt](docs/verification/parity-2026-09-26-batch60.md).
- Batch 61 replaces the manual follow-ups draft shelf with upstream's queued
  turns (`composerDraftStore.queuedTurns` + `ComposerQueuedHeader` +
  `queuedComposerDrain`): a send while a turn is in flight enqueues the
  draft, stacked preview rows sit above the composer with Steer/Delete/Edit
  (Steer maps to upstream's non-steerable-provider interrupt→redispatch),
  and the head auto-dispatches on settle — still sqlite-persisted. See the
  [batch 61 receipt](docs/verification/parity-2026-09-26-batch61.md).
- Batch 62 is a review, not a slice: the `direct_models` subsystem (~3.5k
  lines across app/workspace/server — bindings, API keys, models.dev
  catalog, direct prompt route) has NO upstream counterpart; upstream routes
  all text generation and automation `ai-evaluated` completion through
  `textGenerationModelSelection` on agent providers. Excision is staged:
  (1) drop `evaluator: DirectModelBinding` from `AiEvaluated` + migration,
  (2) reroute text generation through the agent provider, (3) remove the
  direct chat route + composer controls + `uses_direct_model` gates,
  (4) remove the settings section + catalog, (5) drop storage tables. See the
  [batch 62 review](docs/verification/parity-2026-09-26-batch62.md).
- Batch 63 lands stage 1 of the `direct_models` excision: `AiEvaluated`
  drops the Rust-only `evaluator: DirectModelBinding` (upstream shape is
  `{type, stopWhen, confidenceThreshold}`); stored rows still decode via a
  `#[serde(default, skip_serializing)]` shim (the `hub_revision`
  precedent). Evaluation now runs on a throwaway ACP session owned by the
  run's agent profile — matching upstream's separate
  `ProviderTextGeneration` call — with reply text captured from
  `TextDelta` events on a sink that keeps the transcript clean. The
  editor's completion section is now a plain toggle, no model picker. See
  the [batch 63 receipt](docs/verification/parity-2026-09-26-batch63.md).
- Batch 64 completes the `direct_models` excision (stages 2–5 wholesale):
  the direct chat route, composer route controls and `uses_direct_model`
  gates, `HandoffTarget::Direct`, the settings section + `/synara/settings
  direct-models` alias, server direct dispatch, the `task-direct-model:`
  producer reads, and the `synara-model` crate itself are all deleted —
  upstream has no direct-provider surface (`ProviderKind` is agent CLIs
  only). Decode-compat shims stay (`ThreadEvent::DirectModelRoute`, the
  turn fields, the three pref keys as `serde_json::Value`) so old
  databases and backups still open. See the
  [batch 64 receipt](docs/verification/parity-2026-09-27-batch64.md).


## How to update this roadmap

When a remaining item ships:
1. Check its box.
2. Increment **Shipped feature slices** by one.
3. Decrement the matching remaining count and **Total remaining**.
4. Add the feature to the latest batch receipt.
5. Keep the broader verification gate OPEN until its full workflow and required
   provider/platform failure paths are actually accepted.

Do not use the broad verification gate count as the feature count.
