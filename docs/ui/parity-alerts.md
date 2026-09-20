# Native Markdown alerts

Date: 2026-09-20. Parent candidate:
`f527aff25079c5a4c1f89d3391dc7b3f7d6a0621`.
Upstream reviewed again before this independent batch:
`Emanuele-web04/synara/main` at
`b58f27381e7ddd59678c9961500e8e43d3cc19ab`.

## Implemented source

D1 gains Note, Tip, Important, Warning and Caution callouts inside the native
transcript and any existing Studio Markdown view that uses the shared renderer.
The current pinned CommonMark parser's GFM blockquote events identify the syntax.
Independent Rust container frames group complete paragraphs, lists, checklists,
code and tables instead of stripping text from the stored message. Nested
callouts preserve their boundaries. Ordinary or incomplete markers remain text
where the parser does not recognize a complete alert.

The native presentation uses a single restrained left edge, an explicit text
label and Synara's already-bundled icons. No new theme defaults, assets or external
media are introduced. The callout is a message-content group, not a live alert or
an application permission request. Rendering cannot start a tool, grant a
permission, change a provider or open a URL. Existing explicit safe-link activation
and exact code copying remain authoritative. The original durable transcript and
whole-message copy/export source are untouched.

[The design synthesis](native-workspace-design.md) remains authoritative for the
shell. This batch does not replace Environment or modify the concurrent Git review
and live work-status implementation. No Zeron or MonoCode source, assets, tokens,
labels or measurements are copied.

## Evidence ledger

| Check | Current evidence |
| --- | --- |
| Scope selection and fail-closed mixed-change behavior | 22 Python selector tests passed locally |
| Native journey syntax and workflow structure | Python AST and YAML parsing passed locally |
| Five alert kinds, nested containers, Unicode spans, code and literal fallback | Focused Rust cases added, CI result pending |
| Native wide/narrow presentation, exact nested-code copy, restart and unchanged events | Owned inert-transcript journey added, CI screenshots pending |
| Existing table/code/checklist and chat split behavior | Retained in the focused native Markdown lane, result pending |
| Formatting, compilation and strict changed-package Clippy | Pending immutable-candidate CI |

The Markdown-only path must contain an explicit renderer/journey anchor. It keeps
formatting, package lints, native build, all Markdown regressions, the rich-text
journey and the chat split-layout journey. Mixed input, provider, storage or other
UI changes cannot use that reduced set. Existing broader lanes and Jev routing
remain in place. Formatting diagnostics operate only on a disposable archive and
cannot turn an unformatted candidate into a passing candidate.

## Limits

D1 and I10 remain open for full transcript/product/platform acceptance. This is
not Browser embedding, multimedia attachment intake, authenticated vendor approval
verification, syntax highlighting, diagram rendering or a replacement provider
capability. Linux fixture behavior is not macOS/Windows or real-vendor acceptance.
Studio uses the same renderer in source, but a dedicated native Studio alert
journey remains separate. Exact parser-specific case/whitespace edge parity with
Electron is not claimed beyond the recorded tests.

The initial local container has no Rust toolchain. CI provides actual compilation
and native desktop execution. No local native launch or mockup is claimed.
