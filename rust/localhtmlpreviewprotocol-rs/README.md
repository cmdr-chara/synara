# localhtmlpreviewprotocol-rs

FILE: localHtmlPreviewProtocol.ts Purpose: Maps explicit local-file navigations onto a scoped custom protocol so HTML previews   can load relative assets without granting file:// pages unrestricted filesystem access. Layer: Desktop browser infrastructure

napi-rs bridge for `apps/desktop/src/localHtmlPreviewProtocol.ts`.

## Functions

- `isLocalFileUrl` (`apps/desktop/src/localHtmlPreviewProtocol.ts:79`)
- `isLocalHtmlPreviewUrl` (`apps/desktop/src/localHtmlPreviewProtocol.ts:83`)
- `isSameLocalHtmlPreviewGrant` (`apps/desktop/src/localHtmlPreviewProtocol.ts:87`)