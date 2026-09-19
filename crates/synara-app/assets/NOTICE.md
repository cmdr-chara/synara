# Native UI assets

- `synara.svg`: Synara's logo from Emanuele-web04/synara, revision
  `33333439c4b9c74d0097bc01196cccc921f67cf3`, `assets/prod/logo.svg`.
  Reused at the user's request to preserve their Synara identity.
- `fonts/CalSans-Regular.ttf`: unmodified Cal Sans from Google Fonts,
  Git blob `8704264069e7d660454b244797a3f1bb1d94a9c2`,
  https://github.com/google/fonts/tree/main/ofl/calsans.
  Copyright 2021 The Cal Sans Project Authors; SIL Open Font License 1.1.
  Full license: `licenses/CalSans-OFL.txt`.
- `icons/synara/*.svg`: the original Central icon assets used by Synara's
  `apps/web/src/lib/icons.tsx` and `ProviderIcon.tsx`, at the same Synara revision
  above. Copied byte-for-byte from `apps/web/public/central-icons-reversed/`
  (and `central-icons-fill/stop.svg` for Stop). These are existing Synara product
  assets reused at the user's request. Central Icons is a third-party icon
  library, https://centralicons.com/; this record does not relicense its artwork.
  The source repository's notice is retained in `licenses/Synara-source-MIT.txt`.
- `icons/tabler/*.svg`: the corresponding Tabler outlines, version 3.44.0,
  matching Synara's pinned `@tabler/icons-react` dependency and component choices.
  Source: https://github.com/tabler/tabler-icons/tree/v3.44.0/icons/outline.
  Full license: `licenses/Tabler-MIT.txt`.
- `icons/providers/openai.svg`: the exact `SiOpenai` vector data from
  `react-icons@5.6.0/si`, as used by Synara's `Icons.tsx`. The data is serialized
  to an SVG for native rendering; its paths and view box are unchanged.
  Simple Icons uses CC0. React Icons uses MIT.
  Full notices: `licenses/Simple-Icons-CC0.txt`, `licenses/React-Icons-MIT.txt`.

`icons/manifest.json` records each native glyph's exact source and SHA-256.
No Electron application logic is embedded or executed by these static assets.

The font and icon notices are also embedded in the application's Help panel.
No wallpaper or desktop notification from the user's captures is an app asset.
