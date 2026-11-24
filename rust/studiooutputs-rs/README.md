# studiooutputs-rs

FILE: studioOutputs.ts Purpose: Resolve the files a single Studio chat produced anywhere under the Studio          workspace root, from checkpoint summaries (Git roots), persisted file-change          activities, and captured per-turn output activities (non-Git roots). Also owns          the bounded workspace tree scan the HubOutputReactor diffs per turn.

napi-rs bridge for `apps/server/src/studioOutputs.ts`.

## Functions

- `isStudioOutputRelativePath` (`apps/server/src/studioOutputs.ts:79`)
- `collectFileChangeActivityPathCandidates` (`apps/server/src/studioOutputs.ts:121`)
- `collectThreadOutputRelativePaths` (`apps/server/src/studioOutputs.ts:155`)
- `diffStudioWorkspaceScans` (`apps/server/src/studioOutputs.ts:306`)
- `studioOutputsCapturedActivityPayload` (`apps/server/src/studioOutputs.ts:325`)