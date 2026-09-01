import { migratePreparedEffectCommands } from "./windows-runtime-edit-helpers.mjs";

migratePreparedEffectCommands(
  "apps/server/src/provider/opencodeRuntime.ts",
  "../platform/effectProcessRuntime.ts",
  [
    /\s*const prepared = prepareWindowsSafeProcess\(input\.binaryPath, input\.args, \{\s*cwd: input\.cwd,\s*env: childEnv,\s*}\);\s*/,
    /\s*\/\/ Match runOpenCodeCommand:[\s\S]*?const prepared = prepareWindowsSafeProcess\(input\.binaryPath, args, \{\s*cwd: input\.cwd,\s*env: childEnv,\s*}\);\s*/,
  ],
  [
    { command: "input.binaryPath", args: "input.args" },
    { command: "input.binaryPath", args: "args" },
  ],
);

migratePreparedEffectCommands(
  "apps/server/src/provider/Layers/ProviderHealth.ts",
  "../../platform/effectProcessRuntime.ts",
  [/\s*const prepared = prepareWindowsSafeProcess\(executable, args, \{ env }\);\s*/],
  [{ command: "executable", args: "args" }],
);

migratePreparedEffectCommands(
  "apps/server/src/git/Layers/CodexTextGeneration.ts",
  "../../platform/effectProcessRuntime.ts",
  [/\s*const prepared = prepareWindowsSafeProcess\(codexBinaryPath, args, \{ cwd, env }\);\s*/],
  [{ command: "codexBinaryPath", args: "args" }],
);

console.log("Effect service runtime migration applied");
