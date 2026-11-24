# launch-rs

Launches a source desktop build against an isolated data home and Electron profile. Never points at ~/.synara or ~/.synara-beta: the home must live under the bench root.

napi-rs bridge for `benchmarks/desktop-perf/lib/launch.mjs`.

## Functions

- `assertIsolatedHome` (`benchmarks/desktop-perf/lib/launch.mjs:17`)
- `resolveElectronBinary` (`benchmarks/desktop-perf/lib/launch.mjs:36`)
- `freePort` (`benchmarks/desktop-perf/lib/launch.mjs:52`)
- `isolatedEnvironment` (`benchmarks/desktop-perf/lib/launch.mjs:68`)
- `processTree` (`benchmarks/desktop-perf/lib/launch.mjs:104`)
- `classifyProcess` (`benchmarks/desktop-perf/lib/launch.mjs:146`)
- `launchDesktop` (`benchmarks/desktop-perf/lib/launch.mjs:172`)
- `attachMainPage` (`benchmarks/desktop-perf/lib/launch.mjs:209`)
- `quitDesktop` (`benchmarks/desktop-perf/lib/launch.mjs:233`)
- `isFrontmost` (`benchmarks/desktop-perf/lib/launch.mjs:264`)
- `secondsSinceUserInput` (`benchmarks/desktop-perf/lib/launch.mjs:273`)
- `waitForQuietMac` (`benchmarks/desktop-perf/lib/launch.mjs:285`)
- `screenLocked` (`benchmarks/desktop-perf/lib/launch.mjs:306`)
- `frontmostProblem` (`benchmarks/desktop-perf/lib/launch.mjs:321`)