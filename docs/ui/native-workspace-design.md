# Native workspace design synthesis

Date: 2026-09-20. Starting native source: `156e578aa7cfcdf71e70ad9a1b481ae473743246`.
Product reference: Electron `b58f27381e7ddd59678c9961500e8e43d3cc19ab`.

## Direction and boundaries

Synara remains an agent-oriented application, not a replacement IDE. Spaces,
Projects, Threads, standalone Chats, Studio, Kanban, Pull Requests, Automations,
Environment and Settings retain their meanings and navigation. Keep the bundled
Synara brand and approved icon set. References inform principles only. No Zeron
or MonoCode code, assets, copy, theme values, dimensions or compositions are used.
The uploaded archives contain 73 unique Synara, 71 unique Zeron and 50 unique
MonoCode images by SHA-256. Curated byte-identical copies are not extra screens.

The existing Environment already owns tool tabs, ordering, close guards,
maximization, split resizing and versioned layout persistence. Improve it instead
of adding a competing panel host. A permanent IDE layout would obscure the task.
A wholesale empty-canvas redesign would discard working tool state. The selected
direction is a calm conversation shell whose existing tools become denser only
when the user opens them.

## Implementable rules

- Geometry and navigation: retain the established sidebar and Environment split
  contract. Tool content fills its allocated region, not the whole window. Keep
  the conversation available unless the user explicitly maximizes Environment.
  Navigation never restarts a tool or grants an agent capability.
- Content widths: retain the current bounded transcript/composer measure. Review
  and editor surfaces use their available width. Long paths ellipsize in lists
  but are available through explicit copy and detail views.
- Panels: retain active tabs, order, split preference and close guards. No shadow
  panel framework. Persistent data is scoped by project and working root so two
  worktrees, or local and remote projects, cannot share a draft accidentally.
- Typography: use the existing UI font for navigation and hierarchy, code font
  for source/diff content, and a stable line-number gutter for review. Metadata
  is secondary, never the only explanation of an action or error.
- Spacing: use the existing small spacing cadence. One tool header, one content
  region and one action region are enough. Avoid nested decorative cards and
  redundant borders. Dense rows must still be focusable and readable.
- Surfaces: use the active semantic palette, including light and optional Dracula.
  Keep the default theme unchanged. Add/remove markers and line numbers carry
  meaning independently of color. Empty, loading, failed and clean are different
  states. Never show a failed Git read as a clean repository.
- Icons: only Synara's bundled assets. Pair unfamiliar/destructive actions with
  text. Glyph size is not a substitute for an adequate interaction target.
- Focus: native keyboard actions activate on release. Menus/dialogs retain their
  ownership. Lists support directional selection without sending chat or mutating
  files. Retired controls must not steal focus from a text editor or approval.
- Motion: retain the existing interruptible drawer and reduced-motion boundary.
  Do not animate streamed diff lines or repeatedly replay a menu entrance.
- Narrow windows: stack a bounded file navigator above review content when the
  allocated tool region cannot fit both. Wrap action rows rather than squeeze
  labels. Preserve full diff copy even when the preview is bounded. Do not change
  the saved split ratio merely because the window is narrow.

## Batch acceptance

Each batch implements backend-connected behavior and native presentation together.
Focused tests cover scope isolation, stale responses, write ordering, failure and
bounded rendering. Native Linux captures cover full/narrow tools and keyboard
focus. Source, fixture interaction and visual inspection are recorded separately.
Browser embedding, multimedia attachments, vendor approvals and other unsupported
states remain explicitly open. Compilation alone does not close a visual gate.
