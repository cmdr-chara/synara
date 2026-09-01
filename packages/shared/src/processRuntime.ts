// FILE: processRuntime.ts
// Purpose: Spawns Node child processes from platform-neutral launch requests.
// Layer: Shared platform runtime

import {
  execFile as nodeExecFile,
  spawn as nodeSpawn,
  spawnSync as nodeSpawnSync,
  type ChildProcess,
  type ChildProcessWithoutNullStreams,
  type ExecFileException,
  type ExecFileOptionsWithStringEncoding,
  type SpawnOptions,
  type SpawnSyncOptionsWithBufferEncoding,
  type SpawnSyncOptionsWithStringEncoding,
  type SpawnSyncReturns,
} from "node:child_process";

import { prepareProcess, type ProcessLaunchInput, type ProcessLaunchPlan } from "./platformProcess";

type ProcessPlanningOptions = Pick<ProcessLaunchInput, "platform" | "requireExecutable">;

export type RuntimeSpawnOptions = Omit<
  SpawnOptions,
  "shell" | "windowsHide" | "windowsVerbatimArguments"
> &
  ProcessPlanningOptions;

export type RuntimeSpawnSyncStringOptions = Omit<
  SpawnSyncOptionsWithStringEncoding,
  "shell" | "windowsHide" | "windowsVerbatimArguments"
> &
  ProcessPlanningOptions;

export type RuntimeSpawnSyncBufferOptions = Omit<
  SpawnSyncOptionsWithBufferEncoding,
  "shell" | "windowsHide" | "windowsVerbatimArguments"
> &
  ProcessPlanningOptions;

export type RuntimeExecFileOptions = Omit<
  ExecFileOptionsWithStringEncoding,
  "shell" | "windowsHide" | "windowsVerbatimArguments"
> &
  ProcessPlanningOptions;

type PipeStdio = "pipe" | readonly ["pipe", "pipe", "pipe"];
type PlanningOptions =
  | RuntimeSpawnOptions
  | RuntimeSpawnSyncStringOptions
  | RuntimeSpawnSyncBufferOptions
  | RuntimeExecFileOptions;

function planFromOptions(
  command: string,
  args: ReadonlyArray<string>,
  options: PlanningOptions,
): ProcessLaunchPlan {
  const cwd = typeof options.cwd === "string" ? options.cwd : undefined;
  return prepareProcess(command, args, {
    ...(options.platform !== undefined ? { platform: options.platform } : {}),
    ...(cwd !== undefined ? { cwd } : {}),
    ...(options.env !== undefined ? { env: options.env } : {}),
    ...(options.requireExecutable !== undefined
      ? { requireExecutable: options.requireExecutable }
      : {}),
  });
}

function runtimeOptions<T extends ProcessPlanningOptions>(
  options: T,
): Omit<T, "platform" | "requireExecutable"> {
  const { platform: _platform, requireExecutable: _requireExecutable, ...nodeOptions } = options;
  return nodeOptions;
}

/** Spawn a process without exposing platform-specific Node flags to callers. */
export function spawnProcess(
  command: string,
  args: ReadonlyArray<string>,
  options?: RuntimeSpawnOptions & { readonly stdio?: PipeStdio },
): ChildProcessWithoutNullStreams;
export function spawnProcess(
  command: string,
  args: ReadonlyArray<string>,
  options: RuntimeSpawnOptions,
): ChildProcess;
export function spawnProcess(
  command: string,
  args: ReadonlyArray<string>,
  options: RuntimeSpawnOptions = {},
): ChildProcess {
  const plan = planFromOptions(command, args, options);
  return nodeSpawn(plan.command, plan.args, {
    ...runtimeOptions(options),
    shell: false,
    windowsHide: plan.windowsHide,
    windowsVerbatimArguments: plan.windowsVerbatimArguments,
  });
}

/** Spawn an already planned command. Used by infrastructure that logs the plan first. */
export function spawnPlannedProcess(
  plan: ProcessLaunchPlan,
  options?: RuntimeSpawnOptions & { readonly stdio?: PipeStdio },
): ChildProcessWithoutNullStreams;
export function spawnPlannedProcess(
  plan: ProcessLaunchPlan,
  options: RuntimeSpawnOptions,
): ChildProcess;
export function spawnPlannedProcess(
  plan: ProcessLaunchPlan,
  options: RuntimeSpawnOptions = {},
): ChildProcess {
  return nodeSpawn(plan.command, plan.args, {
    ...runtimeOptions(options),
    shell: false,
    windowsHide: plan.windowsHide,
    windowsVerbatimArguments: plan.windowsVerbatimArguments,
  });
}

/** Synchronous counterpart used by bounded discovery and compatibility probes. */
export function spawnProcessSync(
  command: string,
  args: ReadonlyArray<string>,
  options: RuntimeSpawnSyncStringOptions,
): SpawnSyncReturns<string>;
export function spawnProcessSync(
  command: string,
  args: ReadonlyArray<string>,
  options?: RuntimeSpawnSyncBufferOptions,
): SpawnSyncReturns<Buffer>;
export function spawnProcessSync(
  command: string,
  args: ReadonlyArray<string>,
  options: RuntimeSpawnSyncStringOptions | RuntimeSpawnSyncBufferOptions = {},
): SpawnSyncReturns<string> | SpawnSyncReturns<Buffer> {
  const plan = planFromOptions(command, args, options);
  return nodeSpawnSync(plan.command, plan.args, {
    ...runtimeOptions(options),
    shell: false,
    windowsHide: plan.windowsHide,
    windowsVerbatimArguments: plan.windowsVerbatimArguments,
  }) as SpawnSyncReturns<string> | SpawnSyncReturns<Buffer>;
}

/** Callback-compatible execFile for SDK hooks that require that interface. */
export function execProcessFile(
  command: string,
  args: ReadonlyArray<string>,
  options: RuntimeExecFileOptions,
  callback: (error: ExecFileException | null, stdout: string, stderr: string) => void,
): ChildProcess {
  const plan = planFromOptions(command, args, options);
  return nodeExecFile(
    plan.command,
    plan.args,
    {
      ...runtimeOptions(options),
      shell: false,
      windowsHide: plan.windowsHide,
      windowsVerbatimArguments: plan.windowsVerbatimArguments,
    },
    callback,
  );
}

export interface ProcessFileResult {
  readonly stdout: string;
  readonly stderr: string;
}

/** Promise counterpart for background jobs and platform services. */
export function execProcessFilePromise(
  command: string,
  args: ReadonlyArray<string>,
  options: RuntimeExecFileOptions,
): Promise<ProcessFileResult> {
  return new Promise((resolve, reject) => {
    execProcessFile(command, args, options, (error, stdout, stderr) => {
      if (error) {
        reject(error);
        return;
      }
      resolve({ stdout, stderr });
    });
  });
}
