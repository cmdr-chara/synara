# devservermanager-rs

DevServerManager - Server-owned dev-server process orchestration. Dev servers are first-class background processes keyed by project id, fully decoupled from chat threads.

napi-rs bridge for `apps/server/src/devServerManager.ts`.

## Functions

- `findProjectDevServerForLocalServer` (`apps/server/src/devServerManager.ts:47`)