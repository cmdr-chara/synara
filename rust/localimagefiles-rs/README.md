# localimagefiles-rs

FILE: localImageFiles.ts Purpose: Resolves local preview-file (image/video/PDF) requests without exposing arbitrary files. Layer: Server HTTP utility Exports: local image route constants and allowlisted path resolver Depends on: fs realpath/stat, Codex generated image roots, safe preview extensions

napi-rs bridge for `apps/server/src/localImageFiles.ts`.

## Functions

- `resolveLocalPreviewGrantRealPath` (`apps/server/src/localImageFiles.ts:59`)
- `resolveAllowedLocalPreviewFile` (`apps/server/src/localImageFiles.ts:133`)
- `createLocalPreviewGrant` (`apps/server/src/localImageFiles.ts:244`)