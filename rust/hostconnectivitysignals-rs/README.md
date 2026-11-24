# hostconnectivitysignals-rs

FILE: hostConnectivitySignals.ts Purpose: Let account reads in the RPC layer nudge host connectivity, which the          server program owns, without threading one through the other's layers          (the same seam as accountUsageReporterRegistry). Layer: server host connectivity  Nothing here authorizes anything.

napi-rs bridge for `apps/server/src/hostConnectivitySignals.ts`.

## Functions

- `noteRemoteAccessGranted` (`apps/server/src/hostConnectivitySignals.ts:19`)
- `nudgeHostAuthorization` (`apps/server/src/hostConnectivitySignals.ts:39`)