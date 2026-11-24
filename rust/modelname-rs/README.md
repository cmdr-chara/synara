# modelname-rs

FILE: modelName.ts Purpose: Turns raw model slugs and provider labels into Synara's canonical display names ("gpt-6-astra" → "GPT-6 Astra", "claude-opus-4-8" → "Claude Opus 4.8"). Dependency-free so surfaces that avoid the contracts package (the public profile app) share the exact same naming as the desktop; model.ts layers the built-in catalog names on top.

napi-rs bridge for `packages/shared/src/modelName.ts`.

## Functions

- `humanizeModelSlug` (`packages/shared/src/modelName.ts:82`)
- `normalizeModelDisplayName` (`packages/shared/src/modelName.ts:98`)