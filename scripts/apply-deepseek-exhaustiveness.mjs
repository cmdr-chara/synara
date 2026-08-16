import fs from "node:fs";

const read = (path) => fs.readFileSync(path, "utf8");
const write = (path, text) => fs.writeFileSync(path, text);

function replaceOnce(path, before, after) {
  let text = read(path);
  if (text.includes(after)) return;
  if (!text.includes(before)) {
    throw new Error(`Missing expected text in ${path}: ${before.slice(0, 180)}`);
  }
  text = text.replace(before, after);
  write(path, text);
}

function replaceRegexOnce(path, regex, replacement, marker) {
  let text = read(path);
  if (marker && text.includes(marker)) return;
  if (!regex.test(text)) {
    throw new Error(`Missing expected regex in ${path}: ${regex}`);
  }
  text = text.replace(regex, replacement);
  write(path, text);
}

// Contract: DeepSeek has no provider-specific per-model knobs, but it still
// needs an explicit empty options schema so ProviderModelOptions remains
// exhaustive and consumers can safely index options.deepseek.
replaceOnce(
  "packages/contracts/src/model.ts",
  'export const DroidModelOptions = Schema.Struct({\n  reasoningEffort: Schema.optional(TrimmedNonEmptyString),\n});\nexport type DroidModelOptions = typeof DroidModelOptions.Type;',
  'export const DeepSeekModelOptions = Schema.Struct({});\nexport type DeepSeekModelOptions = typeof DeepSeekModelOptions.Type;\n\nexport const DroidModelOptions = Schema.Struct({\n  reasoningEffort: Schema.optional(TrimmedNonEmptyString),\n});\nexport type DroidModelOptions = typeof DroidModelOptions.Type;',
);
replaceOnce(
  "packages/contracts/src/model.ts",
  '  grok: Schema.optional(GrokModelOptions),\n  droid: Schema.optional(DroidModelOptions),',
  '  grok: Schema.optional(GrokModelOptions),\n  deepseek: Schema.optional(DeepSeekModelOptions),\n  droid: Schema.optional(DroidModelOptions),',
);

// Server settings: binary/config path plus custom model persistence.
replaceOnce(
  "packages/contracts/src/settings.ts",
  'export const GrokServerProviderSettings = Schema.Struct({\n  ...ProviderSettingsBase,\n  binaryPath: StringSetting.pipe(Schema.withDecodingDefault(() => "grok")),\n});\nexport type GrokServerProviderSettings = typeof GrokServerProviderSettings.Type;',
  'export const GrokServerProviderSettings = Schema.Struct({\n  ...ProviderSettingsBase,\n  binaryPath: StringSetting.pipe(Schema.withDecodingDefault(() => "grok")),\n});\nexport type GrokServerProviderSettings = typeof GrokServerProviderSettings.Type;\n\nexport const DeepSeekServerProviderSettings = Schema.Struct({\n  ...ProviderSettingsBase,\n  binaryPath: StringSetting.pipe(Schema.withDecodingDefault(() => "dsh-acp-demo")),\n  configPath: StringSetting.pipe(Schema.withDecodingDefault(() => "")),\n});\nexport type DeepSeekServerProviderSettings = typeof DeepSeekServerProviderSettings.Type;',
);
replaceOnce(
  "packages/contracts/src/settings.ts",
  '    grok: GrokServerProviderSettings.pipe(Schema.withDecodingDefault(() => ({}))),\n    droid:',
  '    grok: GrokServerProviderSettings.pipe(Schema.withDecodingDefault(() => ({}))),\n    deepseek: DeepSeekServerProviderSettings.pipe(Schema.withDecodingDefault(() => ({}))),\n    droid:',
);
replaceOnce(
  "packages/contracts/src/settings.ts",
  '      grok: Schema.optionalKey(Schema.Struct(ProviderSettingsBasePatch)),\n      droid:',
  '      grok: Schema.optionalKey(Schema.Struct(ProviderSettingsBasePatch)),\n      deepseek: Schema.optionalKey(\n        Schema.Struct({\n          ...ProviderSettingsBasePatch,\n          configPath: Schema.optionalKey(StringSetting),\n        }),\n      ),\n      droid:',
);

// Agent Gateway: the public Harness exposes no target option surface. Keep an
// explicit empty entry rather than inventing effort/interaction options.
replaceRegexOnce(
  "apps/server/src/agentGateway/targetResolver.ts",
  /(  grok: defineProviderOptionConfig<"grok">\(\{[\s\S]*?\n  \}\),\n)(  droid:)/,
  '$1  deepseek: {\n    primaryOptionKey: "",\n    options: {},\n  },\n$2',
  '  deepseek: {\n    primaryOptionKey: "",\n    options: {},\n  },',
);

// Web persisted settings and provider-specific model metadata.
{
  const path = "apps/web/src/appSettings.ts";
  replaceOnce(
    path,
    '  | "customGrokModels"\n  | "customDroidModels"',
    '  | "customGrokModels"\n  | "customDeepSeekModels"\n  | "customDroidModels"',
  );
  replaceOnce(
    path,
    '  grok: new Set(getModelOptions("grok").map((option) => option.slug)),\n  droid:',
    '  grok: new Set(getModelOptions("grok").map((option) => option.slug)),\n  deepseek: new Set(getModelOptions("deepseek").map((option) => option.slug)),\n  droid:',
  );
  replaceOnce(
    path,
    '  "gemini",\n  "grok",\n  "droid",',
    '  "gemini",\n  "grok",\n  "deepseek",\n  "droid",',
  );
  replaceOnce(
    path,
    '  grokBinaryPath: Schema.String.check(Schema.isMaxLength(4096)).pipe(withDefaults(() => "")),\n  droidBinaryPath:',
    '  grokBinaryPath: Schema.String.check(Schema.isMaxLength(4096)).pipe(withDefaults(() => "")),\n  deepSeekBinaryPath: Schema.String.check(Schema.isMaxLength(4096)).pipe(withDefaults(() => "")),\n  deepSeekConfigPath: Schema.String.check(Schema.isMaxLength(4096)).pipe(withDefaults(() => "")),\n  droidBinaryPath:',
  );
  replaceOnce(
    path,
    '  customGrokModels: Schema.Array(Schema.String).pipe(withDefaults(() => [])),\n  customDroidModels:',
    '  customGrokModels: Schema.Array(Schema.String).pipe(withDefaults(() => [])),\n  customDeepSeekModels: Schema.Array(Schema.String).pipe(withDefaults(() => [])),\n  customDroidModels:',
  );
  replaceOnce(
    path,
    '  grok: {\n    provider: "grok",\n    settingsKey: "customGrokModels",\n    defaultSettingsKey: "customGrokModels",\n    title: "Grok",\n    description: "Save additional Grok model slugs for the picker and `/model` command.",\n    placeholder: "your-grok-model-slug",\n    example: "grok-build-0.1",\n  },\n  droid:',
    '  grok: {\n    provider: "grok",\n    settingsKey: "customGrokModels",\n    defaultSettingsKey: "customGrokModels",\n    title: "Grok",\n    description: "Save additional Grok model slugs for the picker and `/model` command.",\n    placeholder: "your-grok-model-slug",\n    example: "grok-build-0.1",\n  },\n  deepseek: {\n    provider: "deepseek",\n    settingsKey: "customDeepSeekModels",\n    defaultSettingsKey: "customDeepSeekModels",\n    title: "DeepSeek Harness",\n    description: "Save DeepSeek Harness model slugs for the picker. Harness ACP itself does not expose runtime model discovery.",\n    placeholder: "deepseek-model-slug",\n    example: "deepseek-v4-pro",\n  },\n  droid:',
  );

  // Normalization / server projection / model option maps.
  replaceOnce(
    path,
    '    grokBinaryPath: normalizeProviderBinaryPathOverride("grok", settings.grokBinaryPath),\n    droidBinaryPath:',
    '    grokBinaryPath: normalizeProviderBinaryPathOverride("grok", settings.grokBinaryPath),\n    deepSeekBinaryPath: normalizeProviderBinaryPathOverride("deepseek", settings.deepSeekBinaryPath),\n    deepSeekConfigPath: settings.deepSeekConfigPath.trim(),\n    droidBinaryPath:',
  );
  replaceOnce(
    path,
    '    customGrokModels: normalizeCustomModelSlugs(settings.customGrokModels, "grok"),\n    customDroidModels:',
    '    customGrokModels: normalizeCustomModelSlugs(settings.customGrokModels, "grok"),\n    customDeepSeekModels: normalizeCustomModelSlugs(settings.customDeepSeekModels, "deepseek"),\n    customDroidModels:',
  );
  replaceOnce(
    path,
    '    grokBinaryPath: settings.providers.grok.binaryPath,\n    droidBinaryPath:',
    '    grokBinaryPath: settings.providers.grok.binaryPath,\n    deepSeekBinaryPath: settings.providers.deepseek.binaryPath,\n    deepSeekConfigPath: settings.providers.deepseek.configPath,\n    droidBinaryPath:',
  );
  replaceOnce(
    path,
    '    customGrokModels: settings.providers.grok.customModels,\n    customDroidModels:',
    '    customGrokModels: settings.providers.grok.customModels,\n    customDeepSeekModels: settings.providers.deepseek.customModels,\n    customDroidModels:',
  );
  replaceOnce(
    path,
    '    "customGrokModels",\n    "customDroidModels",',
    '    "customGrokModels",\n    "customDeepSeekModels",\n    "customDroidModels",',
  );
  replaceOnce(
    path,
    '    grok: getCustomModelsForProvider(settings, "grok"),\n    droid:',
    '    grok: getCustomModelsForProvider(settings, "grok"),\n    deepseek: getCustomModelsForProvider(settings, "deepseek"),\n    droid:',
  );
  replaceOnce(
    path,
    '    grok: getAppModelOptions("grok", customModelsByProvider.grok),\n    droid:',
    '    grok: getAppModelOptions("grok", customModelsByProvider.grok),\n    deepseek: getAppModelOptions("deepseek", customModelsByProvider.deepseek),\n    droid:',
  );

  // Start-option inputs and projection.
  replaceOnce(
    path,
    '    | "grokBinaryPath"\n    | "droidBinaryPath"',
    '    | "grokBinaryPath"\n    | "deepSeekBinaryPath"\n    | "deepSeekConfigPath"\n    | "droidBinaryPath"',
  );
  replaceOnce(
    path,
    '  const grokBinaryPath = normalizeProviderBinaryPathOverride("grok", settings.grokBinaryPath);\n  const droidBinaryPath',
    '  const grokBinaryPath = normalizeProviderBinaryPathOverride("grok", settings.grokBinaryPath);\n  const deepSeekBinaryPath = normalizeProviderBinaryPathOverride("deepseek", settings.deepSeekBinaryPath);\n  const deepSeekConfigPath = settings.deepSeekConfigPath.trim();\n  const droidBinaryPath',
  );
  replaceOnce(
    path,
    '    ...(droidBinaryPath\n      ? {',
    '    ...(deepSeekBinaryPath || deepSeekConfigPath\n      ? {\n          deepseek: {\n            ...(deepSeekBinaryPath ? { binaryPath: deepSeekBinaryPath } : {}),\n            ...(deepSeekConfigPath ? { configPath: deepSeekConfigPath } : {}),\n          },\n        }\n      : {}),\n    ...(droidBinaryPath\n      ? {',
  );
  replaceOnce(
    path,
    '    case "grok":\n      return normalizeProviderBinaryPathOverride(provider, settings.grokBinaryPath);\n    case "droid":',
    '    case "grok":\n      return normalizeProviderBinaryPathOverride(provider, settings.grokBinaryPath);\n    case "deepseek":\n      return normalizeProviderBinaryPathOverride(provider, settings.deepSeekBinaryPath);\n    case "droid":',
  );

  // Patch server settings when DeepSeek app settings change.
  replaceOnce(
    path,
    '  if (hasOwn(patch, "droidBinaryPath") || hasOwn(patch, "customDroidModels")) {',
    '  if (\n    hasOwn(patch, "deepSeekBinaryPath") ||\n    hasOwn(patch, "deepSeekConfigPath") ||\n    hasOwn(patch, "customDeepSeekModels")\n  ) {\n    providers.deepseek = {\n      ...(hasOwn(patch, "deepSeekBinaryPath")\n        ? { binaryPath: patch.deepSeekBinaryPath ?? "" }\n        : {}),\n      ...(hasOwn(patch, "deepSeekConfigPath")\n        ? { configPath: patch.deepSeekConfigPath ?? "" }\n        : {}),\n      ...(hasOwn(patch, "customDeepSeekModels")\n        ? { customModels: patch.customDeepSeekModels ?? [] }\n        : {}),\n    };\n  }\n  if (hasOwn(patch, "droidBinaryPath") || hasOwn(patch, "customDroidModels")) {',
  );
}

// UI capability maps. A dedicated DeepSeek glyph can replace this temporary
// shared glyph without touching provider logic.
replaceOnce(
  "apps/web/src/components/ProviderIcon.tsx",
  '  grok: GrokIcon,\n  droid: DroidIcon,',
  '  grok: GrokIcon,\n  deepseek: GrokIcon,\n  droid: DroidIcon,',
);
replaceRegexOnce(
  "apps/web/src/components/PluginLibrary.tsx",
  /(\s+grok:\s*\{\s*plugins:\s*false,\s*skills:\s*false\s*\},\n)/,
  '$1        deepseek: { plugins: false, skills: false },\n',
  'deepseek: { plugins: false, skills: false }',
);

console.log("DeepSeek exhaustiveness pass applied.");
