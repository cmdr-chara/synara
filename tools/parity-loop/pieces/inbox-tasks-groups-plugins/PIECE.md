# Piece: inbox-tasks-groups-plugins (Inbox, Tasks, Groups, Plugins routes)

## Scope

The /inbox, /tasks, /groups and /plugins routes as reached from upstream UI (rail More menu / surface switcher): headers, lists, empty states.

Out of scope: palette tokens (`chrome`), rail/sidebar (`rail-sidebar`), composer (`home-composer`),
settings pages. Consume shared building blocks; do not restyle them here.

## Upstream source of truth (under `~/parity/upstream/apps/web/src/`)

routes/_chat.inbox.tsx, routes/_chat.tasks.index.tsx, routes/_chat.groups.index.tsx, routes/_chat.plugins.tsx, components/inbox/*, components/tasks/*, components/githubInbox/*, components/PluginLibrary.tsx, components/SidebarGroupsSurface.tsx

## Port files (likely, under `crates/synara-app/src/`)

shell/activity.rs, shell/integrations*.rs, shell/organization*.rs, shell/hubs*.rs

## Required states (start from a fresh seeded launch unless stated)

1. `inbox` — open the inbox route the way upstream UI reaches it
2. `tasks` — open the tasks route the way upstream UI reaches it
3. `groups` — open the groups route the way upstream UI reaches it
4. `plugins` — open the plugins route the way upstream UI reaches it

If upstream cannot reach a state with the seeded data (for example it needs a GitHub login), capture
what upstream really shows there (its empty / signed-out state) and make the port show the same.

Judge region: main content area.

## Port-only things to delete (in scope)

Anything in this area that upstream does not have: extra panels, tabs, buttons, menu rows, settings,
commands. Delete the code, settings keys and tests behind them, and list them in builder.md.
