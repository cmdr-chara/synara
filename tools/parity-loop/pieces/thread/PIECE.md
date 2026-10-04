# Piece: thread (Thread view (open conversation))

## Scope

The open-thread screen: header (title, project/branch chips, header actions), transcript (user bubbles, assistant markdown: headings, lists, inline code, fenced code blocks with copy, links), message action rows (copy, pin, reply, fork...), work-log / tool rows, timestamps, scroll-to-bottom control, the composer as docked in a thread.

Out of scope: palette tokens (`chrome`), rail/sidebar (`rail-sidebar`), composer (`home-composer`),
settings pages. Consume shared building blocks; do not restyle them here.

## Upstream source of truth (under `~/parity/upstream/apps/web/src/`)

routes/_chat.$threadId.tsx, components/ChatView.tsx, components/ChatMarkdown.tsx, components/chat/* (message rows, header, work log), chatMarkdownSpacing.ts, workLog.ts, timestampFormat.ts

## Port files (likely, under `crates/synara-app/src/`)

shell/conversation.rs, shell/transcript*.rs, shell/messages*.rs, src/ui/markdown.rs

## Required states (start from a fresh seeded launch unless stated)

1. `thread-open` — open seeded thread "Fix login redirect loop" from the sidebar
2. `thread-bottom` — same thread scrolled to the bottom
3. `message-hover` — hover the last assistant message (action row visible)
4. `other-thread` — open "Investigate flaky payments test"

If upstream cannot reach a state with the seeded data (for example it needs a GitHub login), capture
what upstream really shows there (its empty / signed-out state) and make the port show the same.

Judge region: transcript and thread header region (right of the sidebar).

## Port-only things to delete (in scope)

Anything in this area that upstream does not have: extra panels, tabs, buttons, menu rows, settings,
commands. Delete the code, settings keys and tests behind them, and list them in builder.md.
