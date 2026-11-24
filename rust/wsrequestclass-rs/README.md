# wsrequestclass-rs

FILE: wsRequestClass.ts Purpose: Per-connection unary RPC classes and their concurrency budgets. Layer: Shared server/web protocol policy Why: The server admits at most this many unary requests per class and      connection and rejects the rest; the web transport queues against the same      budgets so it never sends a request the server would reject.

napi-rs bridge for `packages/shared/src/wsRequestClass.ts`.

## Functions

- `classifyWsRequest` (`packages/shared/src/wsRequestClass.ts:84`)
- `makeWsRequestCapacityError` (`packages/shared/src/wsRequestClass.ts:98`)