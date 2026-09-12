# Synara agent instructions

## Scope

Keep this file about durable repository contracts. Inspect the code and task-specific docs for implementation details rather than treating this file as a repository map or step-by-step recipe.

## Product priorities

- Prefer correctness, reliability, and predictable recovery behavior over local convenience.
- Preserve performance characteristics on hot transcript, provider, process, persistence, and WebSocket paths; measure before introducing complexity for performance alone.
- Keep user-visible failures explicit and actionable. Do not turn uncertain process, provider, migration, or persistence state into success.

## Architecture boundaries

- `apps/server` owns provider/session orchestration, persistence, process lifecycle, and server-side protocol handling.
- `apps/web` owns browser UI and client state. Do not move server authority or security decisions into the client.
- `packages/contracts` is schema/contract territory. Keep runtime behavior out of it.
- `packages/shared` owns genuinely shared runtime primitives. Preserve explicit subpath exports rather than adding catch-all barrel APIs.
- Keep OS-specific executable resolution, command construction, process-tree handling, filesystem semantics, and WSL behavior behind the shared platform/runtime boundary. Feature and provider code should not recreate Windows/POSIX launch rules locally.

## Compatibility and persistence

- Treat persisted state, event schemas, WebSocket contracts, provider thread/session identity, and migration lineage as compatibility surfaces.
- When changing durable data or provider migrations, preserve rollback/recovery behavior and run `bun run migrations:check` when applicable.
- Do not silently discard legacy state unless the change intentionally defines and tests the removal path.

## UI behavior

- Reuse established shared interaction primitives instead of introducing one-off animation, disclosure, transcript-follow, or accessibility behavior.
- Transcript auto-follow must be driven by real content progression, not generic activity such as tool rows, reconnects, buffering, or approvals.
- Avoid measurement/scroll feedback loops when virtualizing or resizing transcript content.

## Development and verification

Use the smallest relevant checks while iterating. Before calling a substantial change complete, run the checks that cover the changed contracts; for broad integrated changes this normally includes:

```bash
bun run fmt:check
bun run lint
bun run typecheck
bun run test
```

Additional contract checks include:

```bash
bun run windows-runtime:check
bun run migrations:check
bun run test:desktop-smoke
bun run release:smoke
```

Run those only when their surface is affected. Use `bun run test`, not bare `bun test`, so the repository's Vitest/Turbo contract is preserved.

When running a development instance alongside another Synara installation, isolate ports and state rather than reusing the user's live instance.

## Completion

A change is complete when the requested behavior is implemented, affected compatibility/security/persistence boundaries are accounted for, focused regression coverage exists for non-trivial defects or contracts, relevant checks pass, and remaining platform or live-integration gaps are stated precisely. Do not substitute a successful build for evidence about behavior that was not exercised.
