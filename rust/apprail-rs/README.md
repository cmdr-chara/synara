# apprail-rs

FILE: appRail.logic.ts Purpose: Pure rules for the rail layout's tab strip: item ids, route item order, and          which item is active for a pathname, plus the Spaces panel's section list. Layer: Web shell logic Exports: rail item ids/types, rail item order and visibility, railItemForPathname,          reconcileActiveRailItem, buildRailSpacesSections

napi-rs bridge for `apps/web/src/appRail.logic.ts`.

## Functions

- `isRailOrderableItemId` (`apps/web/src/appRail.logic.ts:53`)
- `railItemCanHide` (`apps/web/src/appRail.logic.ts:58`)
- `normalizeRailItemOrder` (`apps/web/src/appRail.logic.ts:62`)
- `normalizeHiddenRailItems` (`apps/web/src/appRail.logic.ts:68`)
- `isRailItemAvailable` (`apps/web/src/appRail.logic.ts:80`)
- `buildRailItemOrder` (`apps/web/src/appRail.logic.ts:90`)
- `railSpaceShortcutKey` (`apps/web/src/appRail.logic.ts:110`)
- `railProjectShortcutKey` (`apps/web/src/appRail.logic.ts:114`)
- `resolveRailShortcuts` (`apps/web/src/appRail.logic.ts:122`)
- `toggleRailShortcutKey` (`apps/web/src/appRail.logic.ts:148`)
- `resolveActiveRailShortcutKey` (`apps/web/src/appRail.logic.ts:156`)
- `railItemShowsPanel` (`apps/web/src/appRail.logic.ts:187`)
- `railItemForPathname` (`apps/web/src/appRail.logic.ts:196`)
- `reconcileActiveRailItem` (`apps/web/src/appRail.logic.ts:219`)