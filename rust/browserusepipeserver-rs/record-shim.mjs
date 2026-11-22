// Record shim for the browserUsePipeServer corpus.
// Freezes volatile process fields so the recorded pipe paths are deterministic;
// the Rust port replays them via SYNARA_BROWSER_PIPE_* env vars.
try {
  Object.defineProperty(process, "pid", { value: 424242, configurable: true });
} catch {}
export {
  resolveDefaultBrowserHostPipePath,
  resolveConfiguredBrowserHostPipePath,
  resolveBrowserHostPipeBackendEnv,
} from "../../apps/desktop/src/browserUsePipeServer.ts";
