# Notes and Studio continuation

Base: `ed88914b98e59b82b20f8ee1b3979fc8b2187d9b` plus a temporary read-only
source export. Electron reference: `948875954f432978eab7dd5fa44c3028b8d99a81`.

## Source outcomes

The notes/checklist modules from the interrupted pass were present but were not
wired into the published app. Integration now lives in source, including task
context preference validation/deletion and the IME guard. The malformed render
expression was corrected. CI no longer mutates source or replaces Jev workflows.

Studio provides a read-only file browser per selected Studio task. Infrastructure
folders and hidden files are excluded. A completed structured tool Diff supplies
reported-output attribution. All-files and image filters distinguish unreported
workspace content. Paths are read through the existing no-symlink workspace
handles. Listings cap file/depth/entry count and elapsed scan work. Text previews
are bounded to 1 MiB, with a 128 KiB rendering excerpt. PNG/JPEG previews are bounded
to 8 MiB compressed, 8192 pixels per dimension and 16 million decoded pixels.
SVG, animations, videos and PDFs are not executed or previewed through a browser.

Copy path/text and Add path to draft are explicit. They do not submit a prompt.
Open in editor reuses the existing dirty-file protection. Stale task/preview
replies are ignored. Refresh rescans actual disk content, so reopening after
restart needs no fabricated or cached output rows.

## Acceptance

The combined source passes local `cargo check --locked -p synara-app --bin synara-app`
with Rust 1.98.1 on Linux. Focused storage and native checks are being assembled for
the completed batch. No unexecuted test or screenshot is claimed as accepted.

## Remaining scope

Full Studio attribution still needs turn-boundary scan/checkpoint tracking and
resource-output handling, rather than treating every workspace file as generated.
Remote Studio previews, more image/document formats, thumbnails/gallery layout,
multimodal attachments and cross-platform acceptance remain open. Original
semantic theme defaults are unchanged. Dracula is optional.

## Explorer and cross-surface actions

The companion Explorer batch reuses `create_document`, `create_directory`,
`rename_document`, `delete_document`, `search_files` and their existing SSH
counterparts. It adds explicit native dialogs with one filename per action,
no-clobber destinations, the open document's version for rename/deletion, and an
explicit permanent-delete warning. A dirty or saving editor blocks mutation.
A mutation dialog holds navigation and close ownership until it is resolved.

Content search is literal, explicit-on-submit, confined to the current folder,
and limited to 200 returned matches. Results show path, line and preview and
open through the existing dirty-document guard. The search result does not yet
select/highlight the matching text in the editor. Query/project/directory identity
suppresses stale completions.

Editor actions copy the relative path, append a file reference, or quote selected
text into the existing draft. Quotation is bounded to 128 KiB, respects the 1 MiB
draft limit, does not send a prompt, and never rewrites the open file.

## Observed continuation checks

- The combined notes/Studio/Explorer production source passes
  `cargo check --locked -p synara-app --bin synara-app` with pinned Rust 1.98.1.
- Ten targeted workspace tests pass: three saved-context, three Studio, and four
  organization tests. The organization fixture required an explicit database-error
  conversion in its test closure before the newly active test module compiled.
- Rust formatting and Python syntax checks pass for the changed files.
- Local native acceptance is blocked before the window opens: this container has
  no Vulkan ICD, and GPUI reports `Failed to create surface for any enabled backend`.
  The retained `native-notes-1` failure is an environment failure, not a native
  feature pass. The read-only CI workflow carries two combined graphical journeys
  on its provisioned Ubuntu runner instead. No whole visual/platform gate is closed.

The new `scripts/native_studio_explorer_smoke.py` exercises creation/save/rename,
selection quotation, search, canceled deletion, external-change refusal, explicit
file deletion, Studio Markdown/PNG preview, narrower windows and restart. It is
prepared validation, not a passing result until the recorded CI run completes.
The user's Jev orchestrator and semantic-routing configuration are unchanged.
