# file-icons-rs

FILE: file-icons.ts Purpose: Map file/folder paths to Central icon names (the central-icons-reversed          asset set). Anything we don't have a dedicated glyph for falls back to          the generic `code-brackets` icon.

napi-rs bridge for `apps/web/src/file-icons.ts`.

## Functions

- `basenameOfPath` (`apps/web/src/file-icons.ts:161`)
- `pathLooksLikeKnownFile` (`apps/web/src/file-icons.ts:170`)
- `inferEntryKindFromPath` (`apps/web/src/file-icons.ts:178`)
- `getFileIconName` (`apps/web/src/file-icons.ts:205`)
- `getAttachmentIconName` (`apps/web/src/file-icons.ts:255`)