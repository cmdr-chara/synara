import fs from "node:fs";

function patchFile(path, patches) {
  let source = fs.readFileSync(path, "utf8");
  let changed = false;
  for (const { before, after, marker } of patches) {
    if (marker && source.includes(marker)) continue;
    if (!source.includes(before)) {
      throw new Error(`Expected patch context not found in ${path}: ${before.slice(0, 120)}`);
    }
    source = source.replace(before, after);
    changed = true;
  }
  if (changed) fs.writeFileSync(path, source);
}

patchFile("packages/contracts/src/orchestration.ts", [
  {
    marker: '  "deepseek",',
    before: '  "grok",\n  "droid",',
    after: '  "grok",\n  "deepseek",\n  "droid",',
  },
  {
    marker: "export const DeepSeekModelSelection",
    before:
      "export type GrokModelSelection = typeof GrokModelSelection.Type;\n\nexport const DroidModelSelection",
    after:
      'export type GrokModelSelection = typeof GrokModelSelection.Type;\n\nexport const DeepSeekModelSelection = Schema.Struct({\n  provider: Schema.Literal("deepseek"),\n  model: TrimmedNonEmptyString,\n});\nexport type DeepSeekModelSelection = typeof DeepSeekModelSelection.Type;\n\nexport const DroidModelSelection',
  },
  {
    marker: "  DeepSeekModelSelection,",
    before: "  GrokModelSelection,\n  DroidModelSelection,",
    after: "  GrokModelSelection,\n  DeepSeekModelSelection,\n  DroidModelSelection,",
  },
  {
    marker: "export const DeepSeekProviderStartOptions",
    before:
      "export const GrokProviderStartOptions = Schema.Struct({\n  binaryPath: Schema.optional(TrimmedNonEmptyString),\n});\n\nexport const DroidProviderStartOptions",
    after:
      "export const GrokProviderStartOptions = Schema.Struct({\n  binaryPath: Schema.optional(TrimmedNonEmptyString),\n});\n\nexport const DeepSeekProviderStartOptions = Schema.Struct({\n  binaryPath: Schema.optional(TrimmedNonEmptyString),\n  configPath: Schema.optional(TrimmedNonEmptyString),\n});\n\nexport const DroidProviderStartOptions",
  },
  {
    marker: "  deepseek: Schema.optional(DeepSeekProviderStartOptions),",
    before:
      "  grok: Schema.optional(GrokProviderStartOptions),\n  droid: Schema.optional(DroidProviderStartOptions),",
    after:
      "  grok: Schema.optional(GrokProviderStartOptions),\n  deepseek: Schema.optional(DeepSeekProviderStartOptions),\n  droid: Schema.optional(DroidProviderStartOptions),",
  },
]);

patchFile("packages/contracts/src/model.ts", [
  {
    marker: "const DEEPSEEK_HARNESS_CAPABILITIES",
    before: "const GROK_BUILD_CAPABILITIES: ModelCapabilities = {\n  reasoningEffortLevels:",
    after:
      "const DEEPSEEK_HARNESS_CAPABILITIES: ModelCapabilities = {\n  reasoningEffortLevels: [],\n  supportsFastMode: false,\n  supportsThinkingToggle: false,\n  promptInjectedEffortLevels: [],\n  contextWindowOptions: [],\n};\n\nconst GROK_BUILD_CAPABILITIES: ModelCapabilities = {\n  reasoningEffortLevels:",
  },
  {
    marker: '      slug: "deepseek-v4-flash",\n      name: "DeepSeek V4 Flash",',
    before: "  droid: [",
    after:
      '  deepseek: [\n    {\n      slug: "deepseek-v4-pro",\n      name: "DeepSeek V4 Pro",\n      capabilities: DEEPSEEK_HARNESS_CAPABILITIES,\n    },\n    {\n      slug: "deepseek-v4-flash",\n      name: "DeepSeek V4 Flash",\n      capabilities: DEEPSEEK_HARNESS_CAPABILITIES,\n    },\n  ],\n  droid: [',
  },
  {
    marker: '  deepseek: "deepseek-v4-pro",',
    before: '  grok: "grok-build",\n  droid:',
    after: '  grok: "grok-build",\n  deepseek: "deepseek-v4-pro",\n  droid:',
  },
  {
    marker: '  deepseek: {\n    deepseek: "deepseek-v4-pro",',
    before: "  droid: {",
    after:
      '  deepseek: {\n    deepseek: "deepseek-v4-pro",\n    pro: "deepseek-v4-pro",\n    flash: "deepseek-v4-flash",\n    "deepseek-v4-pro": "deepseek-v4-pro",\n    "deepseek-v4-flash": "deepseek-v4-flash",\n  },\n  droid: {',
  },
  {
    marker: '  deepseek: "DeepSeek Harness",',
    before: '  grok: "Grok",\n  droid: "Droid",',
    after: '  grok: "Grok",\n  deepseek: "DeepSeek Harness",\n  droid: "Droid",',
  },
]);
