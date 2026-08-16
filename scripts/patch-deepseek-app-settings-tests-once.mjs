import { readFileSync, writeFileSync } from "node:fs";

const path = "apps/web/src/appSettings.test.ts";
let source = readFileSync(path, "utf8");

function replace(before, after, expected = 1) {
  const count = source.split(before).length - 1;
  if (count !== expected) throw new Error(`${path}: expected ${expected} matches, found ${count}`);
  source = source.split(before).join(after);
}

const mapPattern = /^(\s*)grok: \[\],\n\1droid: \[\],/gm;
const mapMatches = [...source.matchAll(mapPattern)];
if (mapMatches.length !== 5) throw new Error(`${path}: expected 5 empty model maps, found ${mapMatches.length}`);
source = source.replace(mapPattern, '$1grok: [],\n$1deepseek: [],\n$1droid: [],');

replace(
  '        grokBinaryPath: "/usr/local/bin/grok",\n        droidBinaryPath: "",',
  '        grokBinaryPath: "/usr/local/bin/grok",\n        deepSeekBinaryPath: "",\n        deepSeekConfigPath: "",\n        droidBinaryPath: "",',
);
replace(
  '        grokBinaryPath: "",\n        droidBinaryPath: "",',
  '        grokBinaryPath: "",\n        deepSeekBinaryPath: "",\n        deepSeekConfigPath: "",\n        droidBinaryPath: "",',
);
replace(
  '        grokBinaryPath: "grok",\n        droidBinaryPath: "droid",',
  '        grokBinaryPath: "grok",\n        deepSeekBinaryPath: "dsh-acp-demo",\n        deepSeekConfigPath: "",\n        droidBinaryPath: "droid",',
);

replace(
  '    customGrokModels: ["grok/custom-fast"],\n    customDroidModels:',
  '    customGrokModels: ["grok/custom-fast"],\n    customDeepSeekModels: ["deepseek/custom-pro"],\n    customDroidModels:',
);
replace('      "grok",\n      "droid",', '      "grok",\n      "deepseek",\n      "droid",');
replace(
  '    expect(getCustomModelsForProvider(settings, "grok")).toEqual(["grok/custom-fast"]);\n    expect(getCustomModelsForProvider(settings, "droid"))',
  '    expect(getCustomModelsForProvider(settings, "grok")).toEqual(["grok/custom-fast"]);\n    expect(getCustomModelsForProvider(settings, "deepseek")).toEqual(["deepseek/custom-pro"]);\n    expect(getCustomModelsForProvider(settings, "droid"))',
);
replace(
  '      customGrokModels: ["grok/default-fast"],\n      customDroidModels:',
  '      customGrokModels: ["grok/default-fast"],\n      customDeepSeekModels: ["deepseek/default-pro"],\n      customDroidModels:',
);
replace(
  '    expect(getDefaultCustomModelsForProvider(defaults, "grok")).toEqual(["grok/default-fast"]);\n    expect(getDefaultCustomModelsForProvider(defaults, "droid"))',
  '    expect(getDefaultCustomModelsForProvider(defaults, "grok")).toEqual(["grok/default-fast"]);\n    expect(getDefaultCustomModelsForProvider(defaults, "deepseek")).toEqual([\n      "deepseek/default-pro",\n    ]);\n    expect(getDefaultCustomModelsForProvider(defaults, "droid"))',
);
replace(
  '  it("patches custom models for droid", () => {',
  '  it("patches custom models for DeepSeek", () => {\n    expect(patchCustomModels("deepseek", ["deepseek/custom-pro"])).toEqual({\n      customDeepSeekModels: ["deepseek/custom-pro"],\n    });\n  });\n\n  it("patches custom models for droid", () => {',
);
replace(
  '      grok: ["grok/custom-fast"],\n      droid:',
  '      grok: ["grok/custom-fast"],\n      deepseek: ["deepseek/custom-pro"],\n      droid:',
);
replace(
  '    expect(modelOptionsByProvider.grok.some((option) => option.slug === "grok/custom-fast")).toBe(\n      true,\n    );\n    expect(\n      modelOptionsByProvider.kilo',
  '    expect(modelOptionsByProvider.grok.some((option) => option.slug === "grok/custom-fast")).toBe(\n      true,\n    );\n    expect(\n      modelOptionsByProvider.deepseek.some((option) => option.slug === "deepseek/custom-pro"),\n    ).toBe(true);\n    expect(\n      modelOptionsByProvider.kilo',
);
replace(
  '      customGrokModels: [" grok-build ", "grok/custom-fast", "grok/custom-fast"],\n      customDroidModels:',
  '      customGrokModels: [" grok-build ", "grok/custom-fast", "grok/custom-fast"],\n      customDeepSeekModels: [" deepseek-v4-pro ", "deepseek/custom-pro", "deepseek/custom-pro"],\n      customDroidModels:',
);

writeFileSync(path, source);
