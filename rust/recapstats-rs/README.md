# recapstats-rs

FILE: recapStats.ts Purpose: Time-window recap for the Inbox from Synara's local projection DB: prompts, chats, turns, agent run time and token deltas per slot, plus the window's top projects and models. Shares the token SQL with profileStats.ts.

napi-rs bridge for `apps/server/src/recapStats.ts`.

## Functions

- `resolveRecapRanges` (`apps/server/src/recapStats.ts:97`)
- `recapModelName` (`apps/server/src/recapStats.ts:135`)
- `turnRunSpan` (`apps/server/src/recapStats.ts:147`)
- `buildRecap` (`apps/server/src/recapStats.ts:195`)