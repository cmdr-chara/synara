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

replace(
  "apps/desktop/src/voiceTranscription.ts",
  'import * as ChildProcess from "node:child_process";',
  'import { spawnProcess } from "@synara/shared/processRuntime";',
);
replace(
  "apps/desktop/src/voiceTranscription.ts",
  'import { prepareWindowsSafeProcess } from "@synara/shared/windowsProcess";\n',
  "",
);
replace(
  "apps/desktop/src/voiceTranscription.ts",
  `    const prepared = prepareWindowsSafeProcess("codex", ["app-server"], {\n      cwd,\n      env: process.env,\n    });\n    const child = ChildProcess.spawn(prepared.command, prepared.args, {\n      cwd,\n      env: process.env,\n      stdio: ["pipe", "pipe", "pipe"],\n      shell: prepared.shell,\n      windowsHide: prepared.windowsHide,\n      windowsVerbatimArguments: prepared.windowsVerbatimArguments,\n    });`,
  `    const child = spawnProcess("codex", ["app-server"], {\n      cwd,\n      env: process.env,\n      stdio: ["pipe", "pipe", "pipe"],\n      requireExecutable: true,\n    });`,
);
replace(
  "apps/desktop/src/voiceTranscription.ts",
  `    child.once("error", (error) => {\n      rejectOnce(new Error(\`Could not start Codex auth discovery: \${error.message}\`));\n    });`,
  `    child.once("error", (error) => {\n      rejectOnce(new Error(\`Could not start Codex auth discovery: \${error.message}\`));\n    });\n    child.once("close", (code, signal) => {\n      if (!settled) {\n        rejectOnce(\n          new Error(\n            \`Codex auth discovery exited before the handshake completed (code=\${code ?? "null"}, signal=\${signal ?? "null"}).\`,\n          ),\n        );\n      }\n    });`,
);
replace(
  "apps/desktop/src/voiceTranscription.ts",
  `    const send = (payload: Record<string, unknown>) => {\n      child.stdin.write(\`\${JSON.stringify(payload)}\\n\`);\n    };`,
  `    const send = (payload: Record<string, unknown>) => {\n      if (!settled && child.stdin.writable) {\n        child.stdin.write(\`\${JSON.stringify(payload)}\\n\`);\n      }\n    };`,
);

replace(
  "apps/desktop/src/electronUpdaterSecurity.ts",
  `import {\n  execFile,\n  spawnSync,\n  type ExecFileException,\n  type ExecFileOptions,\n} from "node:child_process";`,
  `import type { ExecFileException, ExecFileOptions } from "node:child_process";`,
);
replace(
  "apps/desktop/src/electronUpdaterSecurity.ts",
  'import { prepareWindowsSafeProcess, resolveWindowsSystemRoot } from "@synara/shared/windowsProcess";',
  'import { execProcessFile, spawnProcessSync } from "@synara/shared/processRuntime";\nimport { resolveWindowsSystemRoot } from "@synara/shared/windowsProcess";',
);
replace(
  "apps/desktop/src/electronUpdaterSecurity.ts",
  `        execFile(file, [...args], execOptions, (error, stdout, stderr) => {\n          callback(error, String(stdout), String(stderr));\n        });`,
  `        const { shell: _shell, windowsHide: _windowsHide, ...runtimeOptions } = execOptions;\n        execProcessFile(\n          file,\n          args,\n          { ...runtimeOptions, platform: "win32" },\n          (error, stdout, stderr) => {\n            callback(error, String(stdout), String(stderr));\n          },\n        );`,
);
replace(
  "apps/desktop/src/electronUpdaterSecurity.ts",
  `      const prepared = prepareWindowsSafeProcess(cmd, args, { env: mergedEnv });\n      const response = spawnSync(prepared.command, prepared.args, {\n        env: mergedEnv,\n        encoding: "utf8",\n        shell: prepared.shell,\n        windowsHide: prepared.windowsHide,\n        windowsVerbatimArguments: prepared.windowsVerbatimArguments,\n      });`,
  `      const response = spawnProcessSync(cmd, args, {\n        env: mergedEnv,\n        encoding: "utf8",\n        platform: "win32",\n        requireExecutable: true,\n      });`,
);

console.log("desktop runtime migration applied");
