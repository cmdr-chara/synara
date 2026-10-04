# Piece: home-composer (home / new-thread screen and the composer)

## Scope

- The home (`_chat.index`) / new-thread empty state: logo, "What should we work on?" heading, any
  import banner (`ProjectImportLandingBanner`), "Work in a project" picker chip, layout and vertical
  position.
- The composer (`ComposerPromptEditor`, composer footer: `+` add menu, access mode picker
  ("Full access" etc.), model picker button with effort, microphone, send button), placeholder text,
  focus ring, sizes, radii, menus it opens.

Out of scope: top strip, palette tokens, rail, sidebar, transcript.

## Upstream source of truth

`~/parity/upstream/apps/web/src/routes/_chat.index.tsx`, `routes/-chatIndexRoute.logic.ts`,
`components/ChatView.tsx` (empty-state branch), `components/ComposerPromptEditor.tsx`,
`components/chat/*Composer*`, `composerFooterLayout.ts`, `components/chat/*Picker*`,
`ProjectPicker*`, `ProjectImportLandingBanner*`, `composer-logic.ts`, `composerSlashCommands.ts`.

## Port files (likely)

`crates/synara-app/src/shell/composer.rs`, `shell/composer/*.rs`, `shell/conversation.rs`
(empty state), `shell/controls.rs`, `src/ui/menu/models.rs`.

## Required states

1. `home` — fresh seeded launch, home screen.
2. `composer-focused-text` — composer clicked and `Refactor the auth module` typed.
3. `add-menu` — composer `+` menu open.
4. `access-menu` — access mode menu open.
5. `model-menu` — model picker menu open.
6. `project-picker` — "Work in a project" picker open.
7. `slash-menu` — composer cleared, `/` typed.

Judge region: main content area (everything right of the sidebar), including menus.

## Port-only things to delete (in scope)

Composer buttons, menu rows and slash commands upstream does not have (for example port-only
`/synara/...` command namespaces if upstream's slash menu is different), extra chips or toolbars.
