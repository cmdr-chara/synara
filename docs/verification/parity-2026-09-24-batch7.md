# September 24 batch 7: native parity workflows

Base: `b81552b7ed09fe7b2bab82267973c7435cf083db`. Upstream reference:
`Emanuele-web04/synara@eaa61eded31b6755d4f30ba8eabc5d905cf817cb`.

## Completed execution items

| ID | Delivered boundary |
| --- | --- |
| M10 | Manual browser runtime exception/rejection counts and bounded location metadata; explicitly opened Web Inspector for full local console. Agent/auth tabs cannot collect or read manual diagnostics. |
| M19 | Read-only editor comparisons against saved snapshot, current disk and selected local Git ref. Dirty buffers remain intact. |
| M20 | Read-only line blame on reviewed committed revisions, with bounded output, stale-head fencing and Git textconv/external-diff/filter avoidance. |
| S03 | Composer model-cycle shortcut now includes configured direct models, preserving explicit route confirmation. |
| S04 | Scoped and editable Composer/Editor keybindings, focus/IME guards, collision validation and refresh of open/restorable editor tabs. |
| S05 | Exact `/synara/settings [section]` arguments route to existing settings views without provider send. |
| S09 | Versioned per-task folder path references selected by the user; explicit Add to draft; no folder read or access grant. |
| S10 | ZIP export includes task scope, durable turn times/state and role-scoped message creation/update times without drafts or tool payloads. |
| S12 | A message can be added to an existing side-chat draft without sending or replacing its text. |
| S17 | Studio Library combines file-type, output, turn and search filters with sorting and counts. |

The existing current-composer reuse action was refined to label assistant text as reference
context. S11 remains open for richer reply/context semantics. PDF page text can now be
extracted and copied from the bounded read-only snapshot; S18 remains open for links and
form fields. No acceptance/integration item was closed without platform/provider evidence.

## Verification boundary

`git diff --check`, the concise roadmap validator and manual Git/Poppler command probes
ran locally. Focused regression tests were added around changed behavior. This execution
environment does not provide `cargo`, `rustc` or `rustfmt`, so Rust formatting, compilation,
unit tests and native UI journeys require branch CI. The local checks alone do not prove
that these workflows compile or run in a GPUI window. Git ref comparison is local-only;
disk comparison supports local and SSH workspaces. The browser inspector is manual-only,
and PDF text extraction currently requires system Poppler on Linux.
