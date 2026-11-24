# messagetextsegments-rs

FILE: messageTextSegments.ts Purpose: Keep a message's streamed text segments consistent with its text after          live deltas land on top of a snapshot. Layer: Web store helper (pure) Why: Segments only arrive with thread snapshots, while later deltas extend the      message text.

napi-rs bridge for `apps/web/src/messageTextSegments.ts`.

## Functions

- `textSegmentsCoverText` (`apps/web/src/messageTextSegments.ts:14`)
- `advanceMessageTextSegments` (`apps/web/src/messageTextSegments.ts:34`)