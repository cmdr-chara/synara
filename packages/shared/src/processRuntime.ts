// FILE: processRuntime.ts
// Purpose: Spawns Node child processes from platform-neutral launch requests.
// Layer: Shared platform runtime

import {
  spawn as nodeSpawn,
  spawnSync as nodeSpawnSync,
  type ChildProcess,
  type ChildProcessWithoutNullStreams,
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

type PipeStdio = "pipe" | readonly ["pipe", "pipe", "pipe"];

function planFromOptions(
  command: string,
  args: ReadonlyArray<string>,
  options: RuntimeSpawnOptions | RuntimeSpawnSyncStringOptions | RuntimeSpawnSyncBufferOptions,
): ProcessLaunchPlan {
  return prepareProcess(command, args, {
    platform: options.platform,
    cwd: typeof options.cwd === "string" ? options.cwd : undefined,
    env: options.env,
    requireExecutable: options.requireExecutable,
  });
}

function nodeOptions<T extends RuntimeSpawnOptions>(
  options: T,
): Omit<T, "platform" | "requireExecutable"> {
  const { platform: _platform, requireExecutable: _requireExecutable, ...runtimeOptions } = options;
  return runtimeOptions;
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
    ...nodeOptions(options),
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
    ...nodeOptions(options),
    shell: false,
    windowsHide: plan.windowsHide,
    windowsVerbatimArguments: plan.windowsVerbatimArguments,
  });
}

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
  const { platform: _platform, requireExecutable: _requireExecutable, ...runtimeOptions } = options;
  return nodeSpawnSync(plan.command, plan.args, {
    ...runtimeOptions,
    shell: false,
    windowsHide: plan.windowsHide,
    windowsVerbatimArguments: plan.windowsVerbatimArguments,
  }) as SpawnSyncReturns<string> | SpawnSyncReturns<Buffer>;
}
