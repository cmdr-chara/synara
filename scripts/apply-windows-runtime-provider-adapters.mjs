import { migratePreparedEffectCommands, replace } from "./windows-runtime-edit-helpers.mjs";

migratePreparedEffectCommands(
  "apps/server/src/provider/Layers/GrokAdapter.ts",
  "../../platform/effectProcessRuntime.ts",
  [/\s*const prepared = prepareWindowsSafeProcess\(binaryPath, \["models"\], \{\s*env: childEnv\s*}\);\s*/],
  [{ command: "binaryPath", args: '["models"]' }],
);

migratePreparedEffectCommands(
  "apps/server/src/provider/Layers/CursorAdapter.ts",
  "../../platform/effectProcessRuntime.ts",
  [/\s*const prepared = prepareWindowsSafeProcess\(command\.command, command\.args, \{\s*env,\s*}\);\s*/],
  [{ command: "command.command", args: "command.args" }],
);

migratePreparedEffectCommands(
  "apps/server/src/provider/Layers/DevinAdapter.ts",
  "../../platform/effectProcessRuntime.ts",
  [
    /\s*const prepared = prepareWindowsSafeProcess\(\s*binaryPath,\s*\["models", "list", "--format", "json"\],\s*\{ env: childEnv },\s*\);\s*/,
  ],
  [{ command: "binaryPath", args: '["models", "list", "--format", "json"]' }],
);

replace(
  "apps/server/src/provider/acp/AcpSessionRuntime.ts",
  'import { parseWindowsWslUncPath, prepareWindowsSafeProcess } from "@synara/shared/windowsProcess";',
  'import { resolveExecutionWorkingDirectory } from "@synara/shared/wslBridge";',
);
replace(
  "apps/server/src/provider/acp/AcpSessionRuntime.ts",
  `  if (platform !== "win32") {\n    return cwd;\n  }\n  return parseWindowsWslUncPath(cwd)?.linuxPath ?? cwd;`,
  `  return resolveExecutionWorkingDirectory(cwd, platform);`,
);
migratePreparedEffectCommands(
  "apps/server/src/provider/acp/AcpSessionRuntime.ts",
  "../../platform/effectProcessRuntime.ts",
  [
    /\s*const prepared = prepareWindowsSafeProcess\(options\.spawn\.command, options\.spawn\.args, \{\s*cwd: options\.spawn\.cwd,\s*env,\s*}\);\s*/,
  ],
  [{ command: "options.spawn.command", args: "options.spawn.args" }],
);

console.log("provider adapter runtime migration applied");
