# editorappicons-rs

FILE: editorAppIcons.ts Purpose: Extract native installed-app icons for editor integrations and cache          the normalized image files on disk for cheap repeat HTTP requests. Layer: Server HTTP utility Exports: editor icon route constant plus cached icon resolver.

napi-rs bridge for `apps/server/src/editorAppIcons.ts`.

## Functions

- `resolveCachedEditorIcon` (`apps/server/src/editorAppIcons.ts:726`)
- `clearEditorIconInFlightCache` (`apps/server/src/editorAppIcons.ts:759`)