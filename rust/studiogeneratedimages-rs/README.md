# studiogeneratedimages-rs

FILE: studioGeneratedImages.ts Purpose: Copy Codex-generated images into a Studio workspace and persist direct          thread attribution without moving the transcript's original image file. Layer: Server Studio output helper Exports: Studio image naming, validated atomic copy, and copy+attribution helpers Depends on: node fs promises, Codex generated-image roots, Studio outputs payload shape

napi-rs bridge for `apps/server/src/studioGeneratedImages.ts`.

## Functions

- `studioGeneratedImageFileName` (`apps/server/src/studioGeneratedImages.ts:47`)