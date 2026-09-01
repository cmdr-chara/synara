import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { spawnProcess, spawnProcessSync } from "./processRuntime";

let root: string;

beforeEach(() => {
  root = mkdtempSync(path.join(tmpdir(), "synara-process-runtime-"));
});

afterEach(() => {
  rmSync(root, { recursive: true, force: true });
});

async function collect(
  command: string,
  args: readonly string[],
): Promise<{ code: number | null; stdout: string; stderr: string }> {
  const child = spawnProcess(command, args, {
    stdio: "pipe",
    requireExecutable: true,
  });
  child.stdout.setEncoding("utf8");
  child.stderr.setEncoding("utf8");
  let stdout = "";
  let stderr = "";
  child.stdout.on("data", (chunk: string) => {
    stdout += chunk;
  });
  child.stderr.on("data", (chunk: string) => {
    stderr += chunk;
  });
  const code = await new Promise<number | null>((resolve, reject) => {
    child.once("error", reject);
    child.once("close", resolve);
  });
  return { code, stdout, stderr };
}

describe("processRuntime", () => {
  it("runs a native executable with spaces, quotes, empty arguments, and Unicode", async () => {
    const args = ["", "path with spaces", 'quoted="value"', "日本語"];
    const result = await collect(process.execPath, [
      "-e",
      "console.log(JSON.stringify(process.argv.slice(1)))",
      ...args,
    ]);

    expect(result.code).toBe(0);
    expect(JSON.parse(result.stdout.trim())).toEqual(args);
    expect(result.stderr).toBe("");
  });

  it("preserves non-zero exits and stderr", async () => {
    const result = await collect(process.execPath, [
      "-e",
      'console.error("provider failed"); process.exit(23)',
    ]);

    expect(result.code).toBe(23);
    expect(result.stderr).toContain("provider failed");
  });

  it("fails before spawn when a required executable is missing", () => {
    expect(() =>
      spawnProcessSync("synara-definitely-missing-executable", [], {
        requireExecutable: true,
      }),
    ).toThrow(/Command not found/);
  });

  it.runIf(process.platform === "win32")(
    "executes .cmd and .bat through one quoting path",
    async () => {
      for (const extension of ["cmd", "bat"] as const) {
        const script = path.join(root, `echo-args.${extension}`);
        writeFileSync(
          script,
          [
            "@echo off",
            "setlocal DisableDelayedExpansion",
            "echo(%~1",
            "echo(%~2",
            "echo(%~3",
          ].join("\r\n"),
        );
        const result = await collect(script, ["", "path with spaces", "日本語"]);
        expect(result.code).toBe(0);
        expect(result.stdout.replace(/\r/g, "").split("\n").slice(0, 3)).toEqual([
          "",
          "path with spaces",
          "日本語",
        ]);
      }
    },
  );
});
