// Record shim for the launch corpus.
//
// `launch.mjs` mixes pure helpers with functions that spawn Electron, open
// sockets, snapshot the live process table, or poll CDP. Recorded here are the
// exports whose observable behaviour is deterministic for fuzzed inputs:
// pure/error-path functions, the bounded spawnSync probes (`processTree`,
// `isFrontmost`, `screenLocked`), the early-failure `launchDesktop` /
// `quitDesktop` paths, and `attachMainPage`, whose `waitFor` retry loop
// deterministically reaches its 60 s labelled timeout (recording is slow but
// stable). `freePort`, `secondsSinceUserInput` and `waitForQuietMac` are
// volatile or unbounded and stay unrecorded; the Rust port still implements
// them.
export {
  assertIsolatedHome,
  resolveElectronBinary,
  isolatedEnvironment,
  processTree,
  classifyProcess,
  launchDesktop,
  attachMainPage,
  quitDesktop,
  isFrontmost,
  screenLocked,
  frontmostProblem,
} from "../../benchmarks/desktop-perf/lib/launch.mjs";
