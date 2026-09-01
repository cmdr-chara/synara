import { edit, replace, replaceRegex } from "./windows-runtime-edit-helpers.mjs";

replace(
  "apps/server/src/privatePathPermissions.ts",
  `export async function syncDirectoryEntry(\n  directoryPath: string,\n  platform: NodeJS.Platform = process.platform,\n): Promise<void> {\n  if (!supportsPosixPermissions(platform)) return;\n\n  const handle = await fs.promises.open(\n    directoryPath,\n    fs.constants.O_RDONLY | fs.constants.O_DIRECTORY | fs.constants.O_NOFOLLOW,\n  );\n  try {\n    await handle.sync().catch((cause) => {\n      const code = (cause as NodeJS.ErrnoException).code;\n      if (!code || !UNSUPPORTED_DIRECTORY_SYNC_CODES.has(code)) throw cause;\n    });\n  } finally {\n    await handle.close();\n  }\n}`,
  `export async function syncDirectoryEntry(\n  directoryPath: string,\n  platform: NodeJS.Platform = process.platform,\n): Promise<void> {\n  if (!supportsPosixPermissions(platform)) return;\n\n  const handle = await fs.promises.open(\n    directoryPath,\n    fs.constants.O_RDONLY | fs.constants.O_DIRECTORY | fs.constants.O_NOFOLLOW,\n  );\n  try {\n    await handle.sync().catch((cause) => {\n      const code = (cause as NodeJS.ErrnoException).code;\n      if (!code || !UNSUPPORTED_DIRECTORY_SYNC_CODES.has(code)) throw cause;\n    });\n  } finally {\n    await handle.close();\n  }\n}\n\n/** Flushes a regular file with the write access Windows FlushFileBuffers requires. */\nexport async function syncRegularFile(\n  filePath: string,\n  platform: NodeJS.Platform = process.platform,\n): Promise<void> {\n  const flags = supportsPosixPermissions(platform)\n    ? fs.constants.O_RDWR | fs.constants.O_NOFOLLOW\n    : fs.constants.O_RDWR;\n  const handle = await fs.promises.open(filePath, flags);\n  try {\n    await handle.sync();\n  } finally {\n    await handle.close();\n  }\n}\n\n/** POSIX inode identity is reliable; Windows callers must rely on guarded path checks. */\nexport function sameFileIdentity(\n  left: Pick<fs.Stats, "dev" | "ino">,\n  right: Pick<fs.Stats, "dev" | "ino">,\n  platform: NodeJS.Platform = process.platform,\n): boolean {\n  return !supportsPosixPermissions(platform) || (left.dev === right.dev && left.ino === right.ino);\n}`,
);

replace(
  "apps/server/src/persistence/MigrationBackup.ts",
  'import { constants as fsConstants, type Stats } from "node:fs";',
  'import { constants as fsConstants } from "node:fs";',
);
replace(
  "apps/server/src/persistence/MigrationBackup.ts",
  'import { ensurePrivateDirectorySync, repairPrivateFile } from "../privatePathPermissions.ts";',
  'import {\n  ensurePrivateDirectorySync,\n  repairPrivateFile,\n  sameFileIdentity,\n  syncDirectoryEntry,\n  syncRegularFile,\n} from "../privatePathPermissions.ts";',
);
replaceRegex(
  "apps/server/src/persistence/MigrationBackup.ts",
  /async function syncDirectory\(directory: string\): Promise<void> \{[\s\S]*?\n}\n\n\/\*\*[\s\S]*?async function syncRegularFile\(filePath: string\): Promise<void> \{[\s\S]*?\n}\n\n/,
  "",
);
replaceRegex(
  "apps/server/src/persistence/MigrationBackup.ts",
  /function sameFileIdentity\(left: Stats, right: Stats\): boolean \{[\s\S]*?\n}\n\n/,
  "",
);
edit("apps/server/src/persistence/MigrationBackup.ts", (source) => {
  const count = (source.match(/\bsyncDirectory\(/g) ?? []).length;
  if (count === 0) throw new Error("MigrationBackup: no syncDirectory calls found");
  return source.replace(/\bsyncDirectory\(/g, "syncDirectoryEntry(");
});

replace(
  "apps/server/src/persistence/DatabaseLifecycleLock.ts",
  'import { PRIVATE_DIRECTORY_MODE, PRIVATE_FILE_MODE } from "../privatePathPermissions.ts";',
  'import {\n  PRIVATE_DIRECTORY_MODE,\n  PRIVATE_FILE_MODE,\n  syncDirectoryEntry,\n} from "../privatePathPermissions.ts";',
);
replaceRegex(
  "apps/server/src/persistence/DatabaseLifecycleLock.ts",
  /async function syncDirectory\(directoryPath: string\): Promise<void> \{[\s\S]*?\n}\n\n/,
  "",
);
edit("apps/server/src/persistence/DatabaseLifecycleLock.ts", (source) => {
  const count = (source.match(/\bsyncDirectory\(/g) ?? []).length;
  if (count === 0) throw new Error("DatabaseLifecycleLock: no syncDirectory calls found");
  return source.replace(/\bsyncDirectory\(/g, "syncDirectoryEntry(");
});

console.log("filesystem runtime migration applied");
