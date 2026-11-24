# localpreviewfiles-rs

FILE: localPreviewFiles.ts Purpose: Single source of truth for the /api/local-image route shape consumed by          both the server (HTTP route + filesystem allowlist) and the web client          (URL builder + markdown image/video source detection). The route serves          every allowlisted preview file: images (rendered via <img>), videos          (rendered via a native <video>, streamed with HTTP Range requests) and          PDFs (rendered by the browser's built-in viewer in an <iframe>).

napi-rs bridge for `packages/shared/src/localPreviewFiles.ts`.

## Functions

- `lowerCaseExtensionOf` (`packages/shared/src/localPreviewFiles.ts:37`)
- `isSupportedLocalImagePath` (`packages/shared/src/localPreviewFiles.ts:43`)
- `isSupportedLocalPdfPath` (`packages/shared/src/localPreviewFiles.ts:50`)
- `isSupportedLocalVideoPath` (`packages/shared/src/localPreviewFiles.ts:63`)
- `isSupportedLocalPreviewFilePath` (`packages/shared/src/localPreviewFiles.ts:71`)