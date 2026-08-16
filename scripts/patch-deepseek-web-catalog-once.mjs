import { readFileSync, writeFileSync } from "node:fs";

function replace(path, before, after) {
  const source = readFileSync(path, "utf8");
  const count = source.split(before).length - 1;
  if (count !== 1) throw new Error(`${path}: expected 1 match, found ${count}`);
  writeFileSync(path, source.replace(before, after));
}

replace(
  "apps/web/src/components/chat/ProviderModelPicker.browser.tsx",
  '  grok: [\n    { slug: "grok-build-0.1", name: "Grok Build 0.1" },\n    { slug: "grok-build", name: "Grok 4.3" },\n  ],\n  droid: [',
  '  grok: [\n    { slug: "grok-build-0.1", name: "Grok Build 0.1" },\n    { slug: "grok-build", name: "Grok 4.3" },\n  ],\n  deepseek: [\n    { slug: "deepseek-v4-pro", name: "DeepSeek V4 Pro" },\n    { slug: "deepseek-v4-flash", name: "DeepSeek V4 Flash" },\n  ],\n  droid: [',
);

replace(
  "apps/web/src/components/chat/composerProviderRegistry.tsx",
  '  grok: {\n    getState: (input) => getProviderStateFromCapabilities(input),\n    renderTraitsMenuContent: (input) => renderTraitsMenuContentForProvider("grok", input),\n    renderTraitsPicker: (input) => renderTraitsPickerForProvider("grok", input),\n  },\n  droid: {',
  '  grok: {\n    getState: (input) => getProviderStateFromCapabilities(input),\n    renderTraitsMenuContent: (input) => renderTraitsMenuContentForProvider("grok", input),\n    renderTraitsPicker: (input) => renderTraitsPickerForProvider("grok", input),\n  },\n  deepseek: {\n    getState: (input) => getProviderStateFromCapabilities(input),\n    renderTraitsMenuContent: (input) => renderTraitsMenuContentForProvider("deepseek", input),\n    renderTraitsPicker: (input) => renderTraitsPickerForProvider("deepseek", input),\n  },\n  droid: {',
);

replace(
  "apps/web/src/hooks/useProviderModelCatalog.ts",
  '      grok: getAppModelOptions("grok", customModelsByProvider.grok, modelHintByProvider?.grok),\n      droid: getAppModelOptions("droid", customModelsByProvider.droid, modelHintByProvider?.droid),',
  '      grok: getAppModelOptions("grok", customModelsByProvider.grok, modelHintByProvider?.grok),\n      deepseek: getAppModelOptions(\n        "deepseek",\n        customModelsByProvider.deepseek,\n        modelHintByProvider?.deepseek,\n      ),\n      droid: getAppModelOptions("droid", customModelsByProvider.droid, modelHintByProvider?.droid),',
);
replace(
  "apps/web/src/hooks/useProviderModelCatalog.ts",
  '      grok: grokDynamicModelsQuery.data,\n      droid: droidDynamicModelsQuery.data,',
  '      grok: grokDynamicModelsQuery.data,\n      deepseek: undefined,\n      droid: droidDynamicModelsQuery.data,',
);
replace(
  "apps/web/src/hooks/useProviderModelCatalog.ts",
  '      grok: grokDynamicModelsQuery.data?.models ?? [],\n      droid: droidDynamicModelsQuery.data?.models ?? [],',
  '      grok: grokDynamicModelsQuery.data?.models ?? [],\n      deepseek: [],\n      droid: droidDynamicModelsQuery.data?.models ?? [],',
);

replace(
  "apps/web/src/lib/providerModelPrefetch.ts",
  '  | "grokBinaryPath"\n  | "droidBinaryPath"',
  '  | "grokBinaryPath"\n  | "deepSeekBinaryPath"\n  | "droidBinaryPath"',
);
replace(
  "apps/web/src/lib/providerModelPrefetch.ts",
  '    case "grok":\n      return providerModelsQueryOptions({\n        provider: "grok",\n        binaryPath: settings.grokBinaryPath || null,\n      });\n    case "droid":',
  '    case "grok":\n      return providerModelsQueryOptions({\n        provider: "grok",\n        binaryPath: settings.grokBinaryPath || null,\n      });\n    case "deepseek":\n      return providerModelsQueryOptions({\n        provider: "deepseek",\n        binaryPath: settings.deepSeekBinaryPath || null,\n      });\n    case "droid":',
);
