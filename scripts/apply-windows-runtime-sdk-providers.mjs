import { edit, replace, replaceRegex } from "./windows-runtime-edit-helpers.mjs";

// Codex app-server startup and version qualification use the same shared planner.
replace(
  "apps/server/src/codexAppServerManager.ts",
  'import { type ChildProcess, type ChildProcessWithoutNullStreams, spawn } from "node:child_process";',
  'import type { ChildProcess, ChildProcessWithoutNullStreams } from "node:child_process";',
);
replace(
  "apps/server/src/codexAppServerManager.ts",
  'import { prepareWindowsSafeProcess } from "@synara/shared/windowsProcess";',
  'import { spawnProcess } from "@synara/shared/processRuntime";',
);
replaceRegex(
  "apps/server/src/codexAppServerManager.ts",
  /\s*const prepared = prepareWindowsSafeProcess\(input\.binaryPath, \["app-server"\], \{\s*cwd: input\.cwd,\s*env: input\.env,\s*}\);\s*/,
  "\n",
);
replace(
  "apps/server/src/codexAppServerManager.ts",
  "return spawn(prepared.command, prepared.args, {",
  'return spawnProcess(input.binaryPath, ["app-server"], {',
);
replaceRegex(
  "apps/server/src/codexAppServerManager.ts",
  /\s*const prepared = prepareWindowsSafeProcess\(input\.binaryPath, \["--version"\], \{\s*cwd: input\.cwd,\s*env: input\.env,\s*}\);\s*/,
  "\n",
);
replace(
  "apps/server/src/codexAppServerManager.ts",
  "child = spawn(prepared.command, prepared.args, {",
  'child = spawnProcess(input.binaryPath, ["--version"], {',
);
replaceRegex(
  "apps/server/src/codexAppServerManager.ts",
  /^\s*shell: prepared\.shell,\n/gm,
  "",
  2,
);
replaceRegex(
  "apps/server/src/codexAppServerManager.ts",
  /^\s*windowsHide: prepared\.windowsHide,\n/gm,
  "",
  2,
);
replaceRegex(
  "apps/server/src/codexAppServerManager.ts",
  /^\s*windowsVerbatimArguments: prepared\.windowsVerbatimArguments,\n/gm,
  "",
  2,
);
edit("apps/server/src/codexAppServerManager.ts", (source) => {
  const appServer = 'return spawnProcess(input.binaryPath, ["app-server"], {\n';
  const version = 'child = spawnProcess(input.binaryPath, ["--version"], {\n';
  if (!source.includes(appServer) || !source.includes(version)) {
    throw new Error("Codex shared spawn calls were not created");
  }
  return source.replace(appServer, `${appServer}    requireExecutable: true,\n`).replace(
    version,
    `${version}        requireExecutable: true,\n`,
  );
});

// Claude's SDK hook and version probe no longer prepare Windows commands independently.
replace(
  "apps/server/src/provider/Layers/ClaudeAdapter.ts",
  'import { execFile, spawn as spawnChildProcess } from "node:child_process";',
  'import { execProcessFile, spawnProcess } from "@synara/shared/processRuntime";',
);
replace(
  "apps/server/src/provider/Layers/ClaudeAdapter.ts",
  'import { prepareWindowsSafeProcess } from "@synara/shared/windowsProcess";\n',
  "",
);
replaceRegex(
  "apps/server/src/provider/Layers/ClaudeAdapter.ts",
  /\s*const prepared = prepareWindowsSafeProcess\(options\.command, options\.args, \{\s*cwd: options\.cwd,\s*env: options\.env,\s*}\);\s*/,
  "\n",
);
replace(
  "apps/server/src/provider/Layers/ClaudeAdapter.ts",
  "return spawnChildProcess(prepared.command, prepared.args, {",
  "return spawnProcess(options.command, options.args, {",
);
replaceRegex(
  "apps/server/src/provider/Layers/ClaudeAdapter.ts",
  /\s*const prepared = prepareWindowsSafeProcess\(input\.binaryPath, \["--version"\], \{\s*cwd: input\.cwd,\s*env: input\.env,\s*}\);\s*/,
  "\n",
);
replace(
  "apps/server/src/provider/Layers/ClaudeAdapter.ts",
  `    execFile(\n      prepared.command,\n      prepared.args,`,
  `    execProcessFile(\n      input.binaryPath,\n      ["--version"],`,
);
replaceRegex(
  "apps/server/src/provider/Layers/ClaudeAdapter.ts",
  /^\s*shell: prepared\.shell,\n/gm,
  "",
  2,
);
replaceRegex(
  "apps/server/src/provider/Layers/ClaudeAdapter.ts",
  /^\s*\.\.\.\(prepared\.windowsVerbatimArguments \? \{ windowsVerbatimArguments: true \} : \{\}\),\n/gm,
  "",
  2,
);
replaceRegex(
  "apps/server/src/provider/Layers/ClaudeAdapter.ts",
  /^\s*windowsHide: true,\n/gm,
  "",
  2,
);
edit("apps/server/src/provider/Layers/ClaudeAdapter.ts", (source) => {
  const spawnCall = "return spawnProcess(options.command, options.args, {\n";
  const execCall = `    execProcessFile(\n      input.binaryPath,\n      ["--version"],\n      {\n`;
  if (!source.includes(spawnCall) || !source.includes(execCall)) {
    throw new Error("Claude shared process calls were not created");
  }
  return source
    .replace(spawnCall, `${spawnCall}    requireExecutable: true,\n`)
    .replace(execCall, `${execCall}        requireExecutable: true,\n`);
});

console.log("SDK provider runtime migration applied");
