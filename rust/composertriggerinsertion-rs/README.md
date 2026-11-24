# composertriggerinsertion-rs

FILE: composerTriggerInsertion.ts Purpose: Pure helpers that normalise the text around a composer trigger replacement          so back-to-back chips stay separated and a trailing space is never doubled. Layer: Composer logic utilities (no DOM, no React) Exports: ensureLeadingSpaceForReplacement, extendReplacementRangeForTrailingSpace

napi-rs bridge for `apps/web/src/composerTriggerInsertion.ts`.

## Functions

- `extendReplacementRangeForTrailingSpace` (`apps/web/src/composerTriggerInsertion.ts:10`)
- `ensureLeadingSpaceForReplacement` (`apps/web/src/composerTriggerInsertion.ts:26`)