# Windows runtime architecture

Synara treats Windows behavior as a platform-runtime concern. Providers and product features submit logical commands; shared runtime code resolves executables, prepares the platform command, launches it, observes startup, and tears down the owned process tree.

```text
Providers / Desktop / Server / Git / Terminal / Updater
                         |
                         v
                Shared runtime APIs
                         |
                         v
                Windows platform boundary
          Executables / Environment / Processes
          Process trees / PTY / Filesystem / WSL
```

## Launching a process

Use one of these APIs:

- `@synara/shared/processRuntime`: Node callback/process APIs (`spawnProcess`, `spawnProcessSync`, `execProcessFile`).
- `apps/server/src/platform/effectProcessRuntime.ts`: Effect commands (`makeEffectProcessCommand`).
- `@synara/shared/platformProcess`: planning or diagnostics that need the resolved launch plan (`prepareProcess`).

Do not call `spawn`, `spawnSync`, `exec`, or `execFile` from providers or application features. Direct child-process APIs are reserved for the shared runtime implementation and narrowly documented platform infrastructure.

Callers provide the already-hydrated environment and logical executable. The runtime then:

1. resolves the executable against the supplied `PATH` and Windows `PATHEXT`;
2. preserves explicit/manual paths, including spaces and Unicode;
3. routes `.cmd` and `.bat` shims through `ComSpec` with the shared quoting rules;
4. keeps native `.exe` and `.com` launches shell-free;
5. selects the WSL bridge only for supported `\\wsl$` or `\\wsl.localhost` working directories;
6. applies Node's Windows launch flags inside the boundary.

A provider must never inspect `process.platform`, invoke `cmd.exe`/`where.exe`, interpret `PATHEXT`, set `windowsHide` or `windowsVerbatimArguments`, or quote batch arguments itself.

## Executable discovery and environment

`@synara/shared/executable` is the single executable-discovery implementation. Health checks, model discovery, startup, updates, editor integration, and version gates must resolve a given command with the same hydrated environment.

Desktop/server startup is responsible for hydrating the process environment before provider runtime code runs. Windows persisted/user environment handling remains in the environment layer; providers receive the final environment and must not query the registry or repair `PATH` themselves.

## Provider lifecycle

Provider startup is internally observable as:

```text
discovering -> starting -> handshaking -> authenticating -> ready -> running
                  |             |              |
                  +-------------+--------------+-> failed / stopped
```

The lifecycle records explicit failure reasons:

- `ExecutableNotFound`
- `SpawnFailed`
- `ExitedDuringStartup`
- `HandshakeTimeout`
- `AuthenticationFailed`
- `ProtocolFailure`
- `Cancelled`

Every startup path needs a deadline and cleanup. A process that starts but never handshakes must transition to `HandshakeTimeout`; an executable lookup failure and an early process exit must fail immediately. The UI may collapse internal phases, but logs and diagnostics must preserve the reason.

## Process-tree teardown

Provider/runtime owners use `apps/server/src/platform/supervisedProcessTeardown.ts`.

The sequence is:

1. capture descendants before signaling;
2. request graceful protocol shutdown when available;
3. signal the owned tree with `SIGTERM` semantics;
4. wait for root exit and inspect captured descendants;
5. escalate surviving identity-matched descendants;
6. report success only when exit is proven.

Windows snapshots use CIM data and include process creation time so a reused PID is not mistaken for the captured child. Snapshot failure is unknown state, never a verified empty tree. `tree-kill`/`taskkill` details remain inside the platform controller. Callers must not invoke `taskkill` directly.

## Terminal and PTY

Windows terminals continue to use `node-pty`/ConPTY, including when the server runtime is Bun. ConPTY and native-module selection belong to the terminal runtime. Provider and feature code must not depend on ConPTY details.

Terminal restore, resize, persistence, shutdown, and process-tree cleanup stay in terminal services; shared process helpers are for non-PTY processes.

## Filesystem and SQLite recovery

Windows file durability differs because open handles and directory sync semantics differ from POSIX. Shared private-path helpers own:

- regular-file flush behavior;
- directory-entry durability where supported;
- guarded file identity checks;
- private-directory/file repair.

Migrations and lifecycle locks call those helpers rather than embedding Windows branches. Storage code must close database/file handles before rename, unlink, backup replacement, or recovery.

## WSL boundary

WSL2 is optional; Windows-native execution remains supported. `@synara/shared/wslBridge` recognizes supported WSL UNC working directories, while `prepareProcess` selects `wsl.exe` and translates the working directory. Providers see the backend-native working directory but do not know how UNC conversion or `wsl.exe` invocation works.

A future first-class `ExecutionBackend` may add richer WSL2 lifecycle and environment support. New WSL special cases must extend this boundary instead of appearing in providers.

## Adding a provider

A new CLI provider should only define:

- its logical executable/configured binary path;
- protocol arguments and environment additions;
- how readiness/authentication is recognized;
- protocol-level graceful shutdown.

It should not contain any Windows-specific launch, quoting, discovery, PTY, or teardown code. Use the shared Effect or Node runtime API, record the startup phase, enforce a handshake deadline, and hand the owned process to supervised tree teardown.
