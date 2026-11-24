# composer-logic-rs

Inlined helpers (crate stays self-contained):
- `splitPromptIntoComposerSegments` and its token matchers from
  `apps/web/src/composer-editor-mentions.ts` (mentions, skills, slash chips,
  agent mentions, link chips, terminal-context placeholder).
- `createComposerMentionTokenRegex`, `extractComposerMentionPath`,
  `decodeComposerMentionQuotedPath`, `composerMentionQuotedPathHasClosingQuote`
  from `apps/web/src/lib/composerMentions.ts`.
- `LINK_TOKEN_SOURCE`, `trimTrailingLinkPunctuation`,
  `normalizeComposerLinkUrl` from `apps/web/src/lib/linkChips.ts`.
- `isBuiltInComposerSlashCommand`/`BUILT_IN_COMPOSER_SLASH_COMMANDS` from
  `apps/web/src/composerSlashCommands.ts` + `packages/shared`/`contracts`.
- `resolveAgentAlias` (alias → color only) from
  `packages/contracts/src/agentMentions.ts`.
- `INLINE_TERMINAL_CONTEXT_PLACEHOLDER` (`\u{FFFC}`) from
  `apps/web/src/lib/terminalContext.ts`.

napi-rs bridge for `apps/web/src/composer-logic.ts`.

## Functions

- `stripComposerTriggerText` (`apps/web/src/composer-logic.ts:21`)
- `expandCollapsedComposerCursor` (`apps/web/src/composer-logic.ts:81`)
- `clampCollapsedComposerCursor` (`apps/web/src/composer-logic.ts:180`)
- `collapseExpandedComposerCursor` (`apps/web/src/composer-logic.ts:187`)
- `isCollapsedCursorAdjacentToInlineToken` (`apps/web/src/composer-logic.ts:280`)
- `detectComposerTrigger` (`apps/web/src/composer-logic.ts:308`)
- `parseStandaloneComposerSlashCommand` (`apps/web/src/composer-logic.ts:406`)
- `replaceTextRange` (`apps/web/src/composer-logic.ts:420`)