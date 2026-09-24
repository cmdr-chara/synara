# September 24 batch 8: editor, context and document workflows

Base: `017ab2459bc5e00b79d60201f753ed7f0ecdd191`.

## Completed roadmap items

| ID | Delivered boundary |
| --- | --- |
| M17 | Native editor syntax colors for common source, configuration, markup, SQL and Markdown files, with cached bounded lexical spans and plain-text fallback for files over 256 KiB. |
| M18 | Reviewed disk comparison and bounded three-way merge after a save conflict. Disjoint or identical edits update the unsaved buffer and reviewed disk version; overlapping edits preserve the buffer. Saving remains explicit and version checked. |
| S11 | Separate assistant reply scaffold in the current composer, alongside existing quote action. Draft text and attachments stay intact, and no provider submission occurs. |

## Partial capabilities, items still open

- S01: OpenCode and Gemini CLI onboarding guidance, official links and copyable commands for recognized executables. The wider dynamic provider catalog remains generic.
- S14: Shared local/SSH search honors bounded nested ignore rules and generated-output exclusions. Exact upstream ranking and all Git exclusion sources are unverified.
- S18: Page-scoped PDF web links are inspected from the immutable snapshot and opened only on an explicit click. Form fields remain noninteractive.

## Validation boundary

The roadmap validator and `git diff --check` passed locally. This environment has no Rust toolchain. Focused compilation, clippy, Rust tests and native journeys require branch CI. Batch 7 native compilation and focused tests passed at `017ab245`; strict lint corrections are included with this batch. Its Xvfb-backed native journeys and the cancelled WebKit acceptance do not establish platform acceptance here.
