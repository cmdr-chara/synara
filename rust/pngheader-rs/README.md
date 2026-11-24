# pngheader-rs

FILE: pngHeader.ts Purpose: Read a PNG's pixel dimensions out of its own header. Layer: Server utility Exports: pngDimensions  Capture backends derive frame dimensions from encoded pixels rather than assuming the requested size matches the image returned by macOS.

napi-rs bridge for `apps/server/src/pngHeader.ts`.

## Functions

- `pngDimensions` (`apps/server/src/pngHeader.ts:29`)