# jpegheader-rs

FILE: jpegHeader.ts Purpose: Read a JPEG's pixel dimensions out of its own headers. Layer: Server utility Exports: jpegDimensions  Capture backends derive frame dimensions from encoded pixels rather than assuming the requested size matches the image returned by macOS — the same contract pngHeader.ts keeps for PNG captures.

napi-rs bridge for `apps/server/src/jpegHeader.ts`.

## Functions

- `jpegDimensions` (`apps/server/src/jpegHeader.ts:36`)