# chatthreads-rs

FILE: chatThreads.ts Purpose: Shared chat-thread title helpers used by web and server flows. Layer: Shared util Exports: generic title checks plus fallback/generated title sanitizers

napi-rs bridge for `packages/shared/src/chatThreads.ts`.

## Functions

- `buildThreadTitleConversationContext` (`packages/shared/src/chatThreads.ts:98`)
- `truncateChatThreadTitle` (`packages/shared/src/chatThreads.ts:141`)
- `buildPromptThreadTitleFallback` (`packages/shared/src/chatThreads.ts:153`)
- `sanitizeGeneratedThreadTitle` (`packages/shared/src/chatThreads.ts:162`)
- `isUsableGeneratedThreadTitle` (`packages/shared/src/chatThreads.ts:171`)
- `isGenericChatThreadTitle` (`packages/shared/src/chatThreads.ts:176`)