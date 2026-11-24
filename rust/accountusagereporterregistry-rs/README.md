# accountusagereporterregistry-rs

accountUsageReporterRegistry - the seam between sign-in and usage sync. The reporter is constructed in the server program (main.ts) and the account session in the RPC layer; threading one into the other would drag the reporter through the whole layer graph for a single method call.

napi-rs bridge for `apps/server/src/accountUsageReporterRegistry.ts`.

## Functions

- `nudgeAccountUsageReporter` (`apps/server/src/accountUsageReporterRegistry.ts:22`)