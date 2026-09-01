import { readdirSync, readFileSync } from "node:fs";
import path from "node:path";

import { describe, expect, it } from "vitest";

const REPO_ROOT = path.resolve(process.cwd(), "../..");

function sourceFiles(directory: string): string[] {
  const absolute = path.join(REPO_ROOT, directory);
  return readdirSync(absolute, { withFileTypes: true }).flatMap((entry) => {
    const relative = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      if (["fixtures", "testing", "__tests__"].includes(entry.name)) return [];
      return sourceFiles(relative);
    }
    if (!/\.(?:ts|tsx)$/.test(entry.name) || /\.test\./.test(entry.name)) return [];
    return [relative];
  });
}

const PROVIDER_AND_GIT_SOURCES = [
  ...sourceFiles("apps/server/src/provider"),
  ...sourceFiles("apps/server/src/git"),
];

const FORBIDDEN_PROVIDER_RUNTIME_TOKENS = [
  'from "node:child_process"',
  'from "node:child_process/promises"',
  "prepareWindowsSafeProcess",
  "windowsVerbatimArguments",
  "windowsHide",
  "cmd.exe",
  "where.exe",
  "taskkill",
  "PATHEXT",
  'process.platform === "win32"',
] as const;

describe("process platform boundary", () => {
  it("keeps provider and Git sources behind shared runtime APIs", () => {
    const violations: string[] = [];
    for (const file of PROVIDER_AND_GIT_SOURCES) {
      const source = readFileSync(path.join(REPO_ROOT, file), "utf8");
      for (const token of FORBIDDEN_PROVIDER_RUNTIME_TOKENS) {
        if (source.includes(token)) violations.push(`${file}: ${token}`);
      }
    }
    expect(violations).toEqual([]);
  });

  it("keeps desktop runtime entry points free of Windows spawn details", () => {
    const entryPoints = [
      "apps/desktop/src/main.ts",
      "apps/desktop/src/voiceTranscription.ts",
      "apps/desktop/src/electronUpdaterSecurity.ts",
    ];
    const violations: string[] = [];
    for (const file of entryPoints) {
      const source = readFileSync(path.join(REPO_ROOT, file), "utf8");
      for (const token of [
        "ChildProcess.spawn(",
        "prepareWindowsSafeProcess",
        "windowsVerbatimArguments",
        "cmd.exe",
        "taskkill",
      ]) {
        if (source.includes(token)) violations.push(`${file}: ${token}`);
      }
    }
    expect(violations).toEqual([]);
  });
});
