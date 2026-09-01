import fs from "node:fs";

function replace(file, before, after) {
  const source = fs.readFileSync(file, "utf8");
  const count = source.split(before).length - 1;
  if (count !== 1) throw new Error(`${file}: expected one match, found ${count}`);
  fs.writeFileSync(file, source.replace(before, after));
}

replace(
  "packages/shared/package.json",
  `    "./windowsProcess": {\n      "types": "./src/windowsProcess.ts",\n      "import": "./src/windowsProcess.ts"\n    },`,
  `    "./windowsProcess": {\n      "types": "./src/windowsProcess.ts",\n      "import": "./src/windowsProcess.ts"\n    },\n    "./executable": {\n      "types": "./src/executable.ts",\n      "import": "./src/executable.ts"\n    },\n    "./platformProcess": {\n      "types": "./src/platformProcess.ts",\n      "import": "./src/platformProcess.ts"\n    },\n    "./processRuntime": {\n      "types": "./src/processRuntime.ts",\n      "import": "./src/processRuntime.ts"\n    },\n    "./wslBridge": {\n      "types": "./src/wslBridge.ts",\n      "import": "./src/wslBridge.ts"\n    },`,
);

replace(
  "packages/shared/src/windowsProcess.ts",
  'import { spawnSync } from "node:child_process";\nimport * as Path from "node:path";',
  'import * as Path from "node:path";\n\nimport { resolveExecutable } from "./executable";',
);

replace(
  "packages/shared/src/windowsProcess.ts",
  `// Resolve PATH/PATHEXT commands through where.exe so \`.cmd\` shims can be wrapped\n// explicitly. Prefer candidates that native spawn can execute or that we can\n// wrap, and skip current-directory hits for PATH commands to avoid restoring\n// shell-style CWD command hijacking.\nexport function resolveWindowsCommandPath(\n  command: string,\n  input: WindowsSafeProcessInput = {},\n): string {\n  const pathLikeCommand = isPathLikeCommand(command);\n  if (pathLikeCommand && hasWindowsExecutableExtension(command)) {\n    return command;\n  }\n\n  const env = input.env ?? process.env;\n  const cwd = input.cwd ?? process.cwd();\n  const result = (input.spawnSync ?? spawnSync)(resolveWindowsWhereExe(env), [command], {`,
  `// Production resolution shares the same PATH/PATHEXT walk used by health,\n// startup, updates, editors, and version gates. The injected where.exe branch\n// remains only as a deterministic compatibility seam for historical tests.\nexport function resolveWindowsCommandPath(\n  command: string,\n  input: WindowsSafeProcessInput = {},\n): string {\n  const pathLikeCommand = isPathLikeCommand(command);\n  if (pathLikeCommand && hasWindowsExecutableExtension(command)) {\n    return command;\n  }\n\n  const env = input.env ?? process.env;\n  const cwd = input.cwd ?? process.cwd();\n  if (!input.spawnSync) {\n    return resolveExecutable(command, { platform: "win32", env }) ?? command;\n  }\n\n  const result = input.spawnSync(resolveWindowsWhereExe(env), [command], {`,
);

console.log("core runtime migration applied");
