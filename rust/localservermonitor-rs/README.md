# localservermonitor-rs

FILE: localServerMonitor.ts Purpose: Finds local development servers listening on localhost/private ports and          stops a selected server process after re-validating it is still a dev listener. Layer: Server runtime utility used by the WebSocket RPC layer.

napi-rs bridge for `apps/server/src/localServerMonitor.ts`.

## Functions

- `parseLsofTcpListenOutput` (`apps/server/src/localServerMonitor.ts:180`)
- `parseLsofCwdOutput` (`apps/server/src/localServerMonitor.ts:230`)
- `parseProcessInfo` (`apps/server/src/localServerMonitor.ts:256`)
- `isIgnoredLocalServerProcess` (`apps/server/src/localServerMonitor.ts:354`)
- `detectDevServerKindFromText` (`apps/server/src/localServerMonitor.ts:384`)
- `extractLocalServerPageTitle` (`apps/server/src/localServerMonitor.ts:493`)
- `buildLocalServerProcesses` (`apps/server/src/localServerMonitor.ts:946`)
- `listLocalServers` (`apps/server/src/localServerMonitor.ts:964`)