# browsersession-rs

FILE: browserSession.ts Purpose: Shared helpers for the in-app browser session so the desktop main process (and any   other surface) agree on the spoofed Chrome user agent and on how `window.open` requests   are classified into OAuth popups vs. ordinary new tabs. Layer: Shared runtime utility Depends on: nothing

napi-rs bridge for `packages/shared/src/browserSession.ts`.

## Functions

- `resolveFloatingBrowserGuestLayout` (`packages/shared/src/browserSession.ts:31`)
- `normalizeBrowserPageZoomFactor` (`packages/shared/src/browserSession.ts:49`)
- `normalizeBrowserUrlInput` (`packages/shared/src/browserSession.ts:84`)
- `resolveCopyableBrowserTabUrl` (`packages/shared/src/browserSession.ts:132`)
- `isBlankBrowserTabUrl` (`packages/shared/src/browserSession.ts:150`)
- `classifyBrowserWindowOpen` (`packages/shared/src/browserSession.ts:208`)
- `deriveChromeUserAgent` (`packages/shared/src/browserSession.ts:237`)
- `chromeClientHintPlatform` (`packages/shared/src/browserSession.ts:258`)
- `buildChromeClientHints` (`packages/shared/src/browserSession.ts:279`)
- `buildAcceptLanguageHeader` (`packages/shared/src/browserSession.ts:296`)