# inlinevisual-rs

FILE: inlineVisual.ts Purpose: Builds the sandboxed frame document for an inline visual and reads the          messages its bootstrap posts back to the host. Layer: Shared runtime (consumed by the web transcript; contracts own the payload schema) Exports: INLINE_VISUAL_CONTENT_SECURITY_POLICY, buildInlineVisualDocument,          readInlineVisualFrameMessage, inlineVisualThemeMessage, inlineVisualLinkResult,          clampInlineVisualHeight, INLINE_VISUAL_SERIES_COLORS, readInlineVisualEmbedIds

napi-rs bridge for `packages/shared/src/inlineVisual.ts`.

## Functions

- `clampInlineVisualHeight` (`packages/shared/src/inlineVisual.ts:65`)
- `readInlineVisualFrameMessage` (`packages/shared/src/inlineVisual.ts:94`)
- `inlineVisualLinkResult` (`packages/shared/src/inlineVisual.ts:134`)
- `inlineVisualThemeMessage` (`packages/shared/src/inlineVisual.ts:139`)
- `buildInlineVisualDocument` (`packages/shared/src/inlineVisual.ts:219`)
- `readInlineVisualEmbedId` (`packages/shared/src/inlineVisual.ts:232`)
- `readInlineVisualEmbedIds` (`packages/shared/src/inlineVisual.ts:245`)