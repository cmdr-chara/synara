# hostconnectivitysupervisor-rs

FILE: hostConnectivitySupervisor.ts Purpose: Keep host connectivity (connector, mint gateway, ssh-forward          listener) in step with the account credentials file for the          lifetime of the server, instead of reading the file once at boot. Layer: server host connectivity  Why: sign-in links this machine as a host and writes the host fields to the credentials file while the server is already running.

napi-rs bridge for `apps/server/src/hostConnectivitySupervisor.ts`.

## Functions

- `hostLinkKey` (`apps/server/src/hostConnectivitySupervisor.ts:26`)