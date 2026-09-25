# Parity batch 18: per-turn provider/model activity

Implementation commit:
`fc704f55c4d260bac89e5d5ea2bc595498ee459a`, based on
`3b7aa77eaebf643fe9cf2292f8242dd3530a4dd1`.

## S15 complete: durable turn-scoped route attribution

Direct-model turns already stored their reviewed provider/model route. ACP
submission now snapshots the task's exact agent profile plus the acknowledged
single model selector from `SessionConfiguration` before dispatch, then records
that snapshot against the concrete turn ID returned by the session.

The reducer accepts the route before or after `PromptStarted`: early route
events are bounded and held by turn ID until the matching start arrives; late
events update only that exact turn. Conflicting duplicate attribution is
rejected. This keeps historical route identity stable even if the task later
switches agents or the session later switches models.

The attribution write is observational. If a provider has already accepted the
turn and the local metadata write fails, Synara does not report the prompt as a
failed dispatch or encourage an unsafe duplicate send. Such a turn remains
unattributed and is disclosed as legacy/unavailable data.

## Presentation

Collapsed turn activity now shows:
- direct provider/model plus provider-reported input/output tokens when present;
- ACP agent/model plus provider-reported input/output tokens when present;
- the existing usage-only fallback for historical turns without route metadata.

Profile adds a per-turn provider/model section covering both direct and ACP
routes. Counts and percentages use only attributed turns; historical turns that
lack a route snapshot are counted separately and never backfilled from the
thread's latest model or the task's current agent.

The prior latest-session model snapshot remains below this section as explicitly
thread-level legacy context.

## Focused verification

Source-level checks cover:
- exact-turn ACP route replay when the route arrives before and after
  `PromptStarted`;
- bounded route/model identifiers and conflicting duplicate rejection;
- single-model selector snapshot semantics, including ambiguous-model refusal;
- direct + ACP + legacy-unattributed Profile aggregation;
- generic event persistence/forwarding paths, which require no additional
  exhaustive `ThreadEvent` handling outside the core reducer.

The local container has no Rust toolchain, so this run does not claim a cargo,
rustfmt or Clippy pass. No GitHub workflow status was attached at documentation
time. S15 is complete as a product feature; D11/M26 remain open for real
account/quota/billing telemetry and provider integration acceptance.


## S18 partial: PDF form technology disclosure

Studio now parses Poppler's bounded `Form:` metadata from the same immutable PDF
snapshot used for page rendering. The preview explicitly distinguishes no form,
AcroForm, XFA and unrecognized form metadata.

This is deliberately read-only. No PDF field value is read or changed, no form is
submitted, and document scripts or embedded files are never executed. Duplicate
`Form:` metadata is rejected as ambiguous rather than guessed.

Focused parser coverage verifies supported values, unknown-value disclosure,
non-inference from unrelated metadata and duplicate rejection. S18 remains open
for a safe field-level interaction contract.


## M29 partial: bounded PPTX slide-text pipeline

PPTX files can now be explicitly attached and previewed as inert slide text.
Synara opens only bounded `ppt/slides/slideN.xml` entries, orders them by numeric
slide number, extracts DrawingML text into labeled `[Slide N]` sections, and
uses the same extraction for composer context and Studio preview.

The parser caps archive entries, slide count, decompressed XML per slide and total
extracted text. Duplicate slide numbers, malformed XML and empty presentations are
rejected. Relationships, notes, media, macros, external resources and embedded
objects are never opened.

Focused coverage verifies slide ordering, XML entity decoding, prompt/preview
parity, invalid archive rejection and that an embedded-object canary never enters
the extracted context. M29 remains open for richer document rendering and wider
format coverage.


## M29 partial: bounded XLSX cached-value pipeline

XLSX files can now be explicitly attached and previewed as inert spreadsheet
context. Synara reads only bounded numeric `sheetN.xml` worksheet entries plus
the optional shared-string table. Each non-empty cached value is emitted as
`cell-coordinate<TAB>value` under a labeled `[Sheet N]` section.

Formula expressions are deliberately ignored and never evaluated. Only their
cached values are projected when present. Workbook relationships, sheet names,
macros, charts, external links and embedded objects are never opened. Boolean,
inline-string, shared-string, numeric and cached-error values are bounded and
decoded without executing workbook logic.

The extractor caps archive entries, worksheet count, shared-string count,
worksheet XML size, total visited cells and total projected text. Focused
coverage verifies shared/rich strings, sheet ordering, coordinates, cached
formula values, boolean values, prompt/preview parity and that embedded/formula
canaries never enter the context.

M29 remains open for richer sheet rendering and additional document formats.


## M29 partial: bounded ODP and ODS projections

OpenDocument presentations and spreadsheets now use the same inert attachment
and Studio preview path as ODT.

ODP opens only bounded `content.xml`, labels presentation pages as
`[Slide N]`, and extracts visible paragraph text. ODS opens only bounded
`content.xml`, labels sheets, and projects visible/cached cells as bounded row
and column coordinates. Repeated rows/cells are represented compactly.

ODS formula attributes are never evaluated. Only visible text or cached
`office:*` values are projected. Relationships, scripts, media and embedded
objects are never opened for either format. Parser guards reject malformed
overlapping cells, excessive repetition, oversized archives/XML/text and empty
documents.

Focused coverage checks slide/sheet labeling, XML entity decoding, cached formula
values, repeated cells, preview/prompt parity, and embedded/formula canaries.

M29 remains open for richer rendering and additional document formats.
