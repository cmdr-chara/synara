import { readFileSync, writeFileSync } from "node:fs";

function replace(path, before, after) {
  const source = readFileSync(path, "utf8");
  const count = source.split(before).length - 1;
  if (count !== 1) throw new Error(`${path}: expected 1 match, found ${count}`);
  writeFileSync(path, source.replace(before, after));
}

replace(
  "apps/web/src/appSettings.ts",
  '    | "grokBinaryPath"\n    | "droidBinaryPath"',
  '    | "grokBinaryPath"\n    | "deepSeekBinaryPath"\n    | "droidBinaryPath"',
);

replace(
  "apps/web/src/components/ChatView.tsx",
  '    case "grok":\n      return normalizeCustomBinaryPath(providerOptions?.grok?.binaryPath);\n    case "droid":',
  '    case "grok":\n      return normalizeCustomBinaryPath(providerOptions?.grok?.binaryPath);\n    case "deepseek":\n      return normalizeCustomBinaryPath(providerOptions?.deepseek?.binaryPath);\n    case "droid":',
);
replace(
  "apps/web/src/components/ChatView.tsx",
  '      grok: resolveHint("grok"),\n      droid: resolveHint("droid"),',
  '      grok: resolveHint("grok"),\n      deepseek: resolveHint("deepseek"),\n      droid: resolveHint("droid"),',
);

replace(
  "apps/web/src/components/settings/ProfileSettingsPanel.tsx",
  '    case "grok":\n      return "Grok";\n    case "droid":',
  '    case "grok":\n      return "Grok";\n    case "deepseek":\n      return "DeepSeek";\n    case "droid":',
);

replace(
  "apps/web/src/composerDraftModels.ts",
  '    case "droid":\n      return {',
  '    case "deepseek":\n      return { provider, model };\n    case "droid":\n      return {',
);

replace(
  "apps/web/src/lib/composerSend.ts",
  '    case "antigravity":\n      return null;\n    case "codex":',
  '    case "antigravity":\n    case "deepseek":\n      return null;\n    case "codex":',
);

replace(
  "apps/web/src/providerModelOptions.ts",
  '    case "droid":\n      return options',
  '    case "deepseek":\n      return { provider, model };\n    case "droid":\n      return options',
);
