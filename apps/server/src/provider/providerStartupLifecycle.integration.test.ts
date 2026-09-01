import { fileURLToPath } from "node:url";

import { spawnProcess } from "@synara/shared/processRuntime";
import { describe, expect, it, vi } from "vitest";

import {
  ProviderStartupError,
  ProviderStartupLifecycle,
  superviseProviderStartup,
} from "./providerStartupLifecycle";

const FIXTURE = fileURLToPath(new URL("./fixtures/fakeProviderCli.mjs", import.meta.url));

type FakeMode = "success" | "slow" | "never" | "auth-failure" | "crash" | "child";

function stopChild(child: ReturnType<typeof spawnProcess> | null): Promise<void> {
  if (!child || child.exitCode !== null || child.signalCode !== null) return Promise.resolve();
  return new Promise((resolve) => {
    const timer = setTimeout(() => {
      child.kill("SIGKILL");
    }, 500);
    child.once("close", () => {
      clearTimeout(timer);
      resolve();
    });
    child.kill("SIGTERM");
  });
}

async function runFakeProvider(input: {
  mode: FakeMode;
  timeoutMs: number;
  signal?: AbortSignal;
  lifecycle?: ProviderStartupLifecycle;
}) {
  let child: ReturnType<typeof spawnProcess> | null = null;
  const cleanup = vi.fn(async () => stopChild(child));
  const result = superviseProviderStartup({
    timeoutMs: input.timeoutMs,
    ...(input.signal ? { signal: input.signal } : {}),
    ...(input.lifecycle ? { lifecycle: input.lifecycle } : {}),
    cleanup,
    start: async ({ markHandshaking, markAuthenticating }) => {
      child = spawnProcess(process.execPath, [FIXTURE, input.mode], {
        stdio: "pipe",
        requireExecutable: true,
      });
      markHandshaking();
      child.stdout.setEncoding("utf8");
      child.stderr.setEncoding("utf8");
      let stdout = "";
      let stderr = "";
      return await new Promise<{ childPid?: number }>((resolve, reject) => {
        let childPid: number | undefined;
        const inspect = () => {
          for (const line of stdout.split(/\r?\n/)) {
            if (!line.trim()) continue;
            const event = JSON.parse(line) as { event?: string; pid?: number };
            if (event.event === "child") childPid = event.pid;
            if (event.event === "ready") {
              resolve(childPid === undefined ? {} : { childPid });
              return;
            }
          }
        };
        child?.stdout.on("data", (chunk: string) => {
          stdout += chunk;
          inspect();
        });
        child?.stderr.on("data", (chunk: string) => {
          stderr += chunk;
          if (stderr.includes("authentication_failed")) markAuthenticating();
        });
        child?.once("error", reject);
        child?.once("close", (code, signal) => {
          reject(
            new Error(
              stderr.includes("authentication_failed")
                ? "authentication failed"
                : `provider exited during startup (code=${code ?? "null"}, signal=${signal ?? "null"})`,
            ),
          );
        });
      });
    },
  });
  return { result, cleanup, getChild: () => child };
}

describe("provider startup with a fake CLI", () => {
  it("observes stdout/stderr and reaches ready", async () => {
    const run = await runFakeProvider({ mode: "success", timeoutMs: 500 });
    await expect(run.result).resolves.toEqual({});
    await stopChild(run.getChild());
  });

  it("fails and cleans up when the handshake never arrives", async () => {
    const run = await runFakeProvider({ mode: "never", timeoutMs: 30 });
    await expect(run.result).rejects.toMatchObject({ reason: "HandshakeTimeout" });
    expect(run.cleanup).toHaveBeenCalledOnce();
  });

  it("reports authentication failure and startup crash explicitly", async () => {
    for (const [mode, reason] of [
      ["auth-failure", "AuthenticationFailed"],
      ["crash", "ExitedDuringStartup"],
    ] as const) {
      const run = await runFakeProvider({ mode, timeoutMs: 500 });
      await expect(run.result).rejects.toMatchObject({ reason });
      expect(run.cleanup).toHaveBeenCalledOnce();
    }
  });

  it("cancels while handshaking and stops the process", async () => {
    const controller = new AbortController();
    const lifecycle = new ProviderStartupLifecycle();
    const run = await runFakeProvider({
      mode: "never",
      timeoutMs: 1_000,
      signal: controller.signal,
      lifecycle,
    });
    controller.abort();
    await expect(run.result).rejects.toBeInstanceOf(ProviderStartupError);
    expect(lifecycle.snapshot()).toMatchObject({ phase: "stopped", failureReason: "Cancelled" });
    expect(run.cleanup).toHaveBeenCalledOnce();
  });

  it("simulates a provider that owns a child process", async () => {
    const run = await runFakeProvider({ mode: "child", timeoutMs: 500 });
    await expect(run.result).resolves.toEqual({ childPid: expect.any(Number) });
    await stopChild(run.getChild());
  });
});
