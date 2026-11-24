# openthreadtabs-rs

FILE: openThreadTabs.logic.ts Purpose: Pure helpers for the open-thread tabs: maintaining the ordered open list,          deciding which open threads render as tabs, how each tab is labelled, and          which tab takes over when the active one closes. Layer: UI state logic Exports: open-list transitions, persisted-list normalization, tab derivation, close flow

napi-rs bridge for `apps/web/src/openThreadTabs.logic.ts`.

## Functions

- `addOpenThreadTab` (`apps/web/src/openThreadTabs.logic.ts:43`)
- `removeOpenThreadTab` (`apps/web/src/openThreadTabs.logic.ts:50`)
- `moveOpenThreadTab` (`apps/web/src/openThreadTabs.logic.ts:63`)
- `normalizeOpenThreadTabIds` (`apps/web/src/openThreadTabs.logic.ts:83`)
- `canKeepOpenThreadTab` (`apps/web/src/openThreadTabs.logic.ts:103`)
- `buildOpenThreadTabs` (`apps/web/src/openThreadTabs.logic.ts:146`)
- `resolveOpenThreadTabCloseTarget` (`apps/web/src/openThreadTabs.logic.ts:168`)
- `resolveOpenThreadTabsInCloseScope` (`apps/web/src/openThreadTabs.logic.ts:261`)