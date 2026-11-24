# accountrpcerrors-rs

FILE: accountRpcErrors.ts Purpose: Map account-session failures to WsRpcError for the account RPC handlers. Layer: server RPC error mapping Why: `Effect.tryPromise` wraps every rejection in `Cause.UnknownError`, so without unwrapping the AccountApiError underneath, the client would only ever see the generic fallback message — and never the `AccountErrorCode` the UI branches on (`invalid_verification_code`, `handle_taken`, ...).

napi-rs bridge for `apps/server/src/accountRpcErrors.ts`.

## Functions

- `unwrapTryPromiseError` (`apps/server/src/accountRpcErrors.ts:16`)