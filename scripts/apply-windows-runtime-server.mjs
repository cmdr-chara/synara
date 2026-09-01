import fs from "node:fs";

function read(file) {
  return fs.readFileSync(file, "utf8");
}
function write(file, content) {
  fs.writeFileSync(file, content.endsWith("\n") ? content : `${content}\n`);
}
function replace(file, before, after, expected = 1) {
  const source = read(file);
  const count = source.split(before).length - 1;
  if (count !== expected) throw new Error(`${file}: expected ${expected} matches, found ${count}`);
  write(file, source.split(before).join(after));
}
function replaceRegex(file, pattern, after, expected = 1) {
  const source = read(file);
  const flags = pattern.flags.includes("g") ? pattern.flags : `${pattern.flags}g`;
  const matcher = new RegExp(pattern.source, flags);
  const count = [...source.matchAll(matcher)].length;
  if (count !== expected) throw new Error(`${file}: expected ${expected} regex matches, found ${count}`);
  write(file, source.replace(matcher, after));
}

write(
  "apps/server/src/executableLookup.ts",
  `// Compatibility export. New code imports the shared executable boundary directly.\nexport * from "@synara/shared/executable";`,
);
write(
  "apps/server/src/terminal/processTreeKiller.ts",
  `// Compatibility export for terminal code while ownership lives in the platform runtime.\nexport * from "../platform/processTreeController.ts";`,
);
write(
  "apps/server/src/terminal/windowsProcessSnapshot.ts",
  `// Compatibility export for terminal code while ownership lives in the platform runtime.\nexport * from "../platform/windowsProcessSnapshot.ts";`,
);
write(
  "apps/server/src/provider/supervisedProcessTeardown.ts",
  `// Compatibility export for provider adapters while ownership lives in the platform runtime.\nexport * from "../platform/supervisedProcessTeardown.ts";`,
);

replace(
  "apps/server/src/processRunner.ts",
  'import { type ChildProcess as ChildProcessHandle, spawn, spawnSync } from "node:child_process";\nimport { StringDecoder } from "node:string_decoder";\nimport { prepareWindowsSafeProcess } from "@synara/shared/windowsProcess";',
  'import type { ChildProcess as ChildProcessHandle } from "node:child_process";\nimport { StringDecoder } from "node:string_decoder";\nimport { spawnProcess } from "@synara/shared/processRuntime";\n\nimport { signalProcessTree } from "./platform/processTreeController.ts";',
);
replaceRegex(
  "apps/server/src/processRunner.ts",
  /function isWindowsCommandNotFound[\s\S]*?\n}\n\nfunction normalizeExitError/,
  "function normalizeExitError",
);
replaceRegex(
  "apps/server/src/processRunner.ts",
  /  if \(isWindowsCommandNotFound\(result\.code, result\.stderr\)\) \{\n    return new Error\(`Command not found: \$\{command\}`\);\n  }\n\n/,
  "",
);
replaceRegex(
  "apps/server/src/processRunner.ts",
  /\/\/ Windows `\.cmd` shims[\s\S]*?function killChild[\s\S]*?\n}\n\nfunction appendChunkWithinLimit/,
  `// Process-tree signaling is platform-owned; application code never invokes taskkill.\nfunction killChild(\n  child: ChildProcessHandle,\n  signal: "SIGTERM" | "SIGKILL" = "SIGTERM",\n): void {\n  if (child.pid === undefined) {\n    child.kill(signal);\n    return;\n  }\n  signalProcessTree({ rootPid: child.pid, signal });\n}\n\nfunction appendChunkWithinLimit`,
);
replace(
  "apps/server/src/processRunner.ts",
  `    const prepared = prepareWindowsSafeProcess(command, args, {\n      cwd: options.cwd,\n      env: options.env,\n    });\n    const child = spawn(prepared.command, prepared.args, {\n      cwd: options.cwd,\n      env: options.env,\n      stdio: "pipe",\n      shell: prepared.shell,\n      windowsHide: prepared.windowsHide,\n      windowsVerbatimArguments: prepared.windowsVerbatimArguments,\n    });`,
  `    const child = spawnProcess(command, args, {\n      cwd: options.cwd,\n      env: options.env,\n      stdio: "pipe",\n      requireExecutable: true,\n    });`,
);

replace(
  "apps/server/src/open.ts",
  'import { spawn } from "node:child_process";',
  'import { resolveExecutable } from "@synara/shared/executable";\nimport { spawnProcess } from "@synara/shared/processRuntime";',
);
replace(
  "apps/server/src/open.ts",
  'import { prepareWindowsSafeProcess, resolveWindowsSystemRoot } from "@synara/shared/windowsProcess";',
  'import { resolveWindowsSystemRoot } from "@synara/shared/windowsProcess";',
);
replace("apps/server/src/open.ts", 'import { resolveExecutable } from "./executableLookup.ts";\n', "");
replace(
  "apps/server/src/open.ts",
  `        const prepared = prepareWindowsSafeProcess(launch.command, launch.args);\n        child = spawn(prepared.command, prepared.args, {\n          detached: true,\n          stdio: "ignore",\n          shell: prepared.shell,\n          windowsHide: prepared.windowsHide,\n          windowsVerbatimArguments: prepared.windowsVerbatimArguments,\n        });`,
  `        child = spawnProcess(launch.command, launch.args, {\n          detached: true,\n          stdio: "ignore",\n          requireExecutable: true,\n        });`,
);

console.log("server runtime migration applied");
