# September 24, 2026: editor and Studio depth

## Product scope

Three bounded additions build on the earlier 25 slices and first continuation
batch, without treating an open gate as a missing existing implementation.

1. Per-buffer opt-in local idle auto-save uses the existing compare-and-swap
   filesystem writer and editor save owner. Concurrent saves/IME/modal closure
   are fenced. A completed write advances only the captured disk baseline, not
   later text or undo state. Any failure turns off auto-save for that buffer and
   retains its unsaved text. Explicit conflict Reload/Overwrite remains available.
2. Still-WebP Studio previews decode within the existing attachment codec limits
   and convert to bounded PNG in memory for the existing native image/zoom view.
   Animated, damaged and oversized images and symlink paths remain refused.
   A single decoder permit bounds concurrent work, including canceled callers.
3. Studio reporting task/turn/time is derived from durable ToolChanged output
   replacements. Later status-only changes and recent chat activity cannot
   claim newer provenance. A selected reporting-turn filter shows associated
   outputs. This is reporting attribution, not filesystem authorship or a
   historical content snapshot. Reopen reconstructs the same metadata.

## Verification contract

Focused unit checks cover opt-in/debounce/retirement, output-origin replay and
failed history replacement, reused tool IDs, actual newest report selection,
persisted reopening, still-WebP pixels and source preservation, and animated
or damaged/symlink rejection. Native `native_editor_studio_depth_smoke.py`
exercises real GPUI input, default-off auto-save, subsequent idle saves, external
disk conflict, explicit recovery, WebP preview/filter/zoom and reopening.

Validation results are appended only by a successful pre-publication workflow.
The source-level existence of this receipt is not itself a passing result.

## Remaining limits

All 21 parity gates remain OPEN. Auto-save is intentionally local and opt-in,
not persisted global permission. Syntax highlighting, deeper diff editing and
remote/platform acceptance remain. Studio does not gain immutable per-turn
file snapshots, arbitrary document rendering, an animation player or provider
acceptance. No live provider, Apple Simulator or non-Linux journey is claimed.

## Observed validation

The affected core/workspace/app tests, strict Clippy, formatting and native journey passed before publication.
Evidence: https://github.com/cmdr-chara/synara/actions/runs/35987911223
Run attempt: 1. Publication base: `33e67248f55bc6d4eec2f979fa508e9508e883af`.
The recovered journey now creates a Hub explicitly. The older mode-switch helper opens the Hubs index and does not create a thread. Native evidence covers opt-in local auto-save, disk conflict refusal, explicit recovery, WebP preview/filter/zoom and restart. Reporting-turn provenance is covered by event/storage tests. No provider or non-Linux acceptance is claimed.
