import { readFileSync, writeFileSync } from "node:fs";

function replace(path, before, after) {
  const source = readFileSync(path, "utf8");
  const count = source.split(before).length - 1;
  if (count !== 1) throw new Error(`${path}: expected 1 match, found ${count}`);
  writeFileSync(path, source.replace(before, after));
}

function addEmptyDeepSeek(path, expected) {
  const source = readFileSync(path, "utf8");
  const pattern = /^(\s*)grok: ([^\n]+),\n\1droid:/gm;
  const matches = [...source.matchAll(pattern)];
  if (matches.length !== expected) {
    throw new Error(`${path}: expected ${expected} provider maps, found ${matches.length}`);
  }
  writeFileSync(
    path,
    source.replace(pattern, "$1grok: $2,\n$1deepseek: [],\n$1droid:"),
  );
}

addEmptyDeepSeek("apps/web/src/components/chat/ComposerModelEffortPicker.browser.tsx", 1);
addEmptyDeepSeek("apps/web/src/components/chat/TraitsPicker.browser.tsx", 2);
addEmptyDeepSeek("apps/web/src/composerDraftStore.models.test.ts", 4);

replace(
  "apps/web/src/providerUpdates.test.ts",
  '      grok: { ...provider, binaryPath: "grok" },\n      droid:',
  '      grok: { ...provider, binaryPath: "grok" },\n      deepseek: { ...provider, binaryPath: "dsh-acp-demo", configPath: "" },\n      droid:',
);

replace(
  "apps/web/src/wsNativeApi.test.ts",
  '          grok: { enabled: true, binaryPath: "grok", customModels: [] },\n          droid:',
  '          grok: { enabled: true, binaryPath: "grok", customModels: [] },\n          deepseek: {\n            enabled: true,\n            binaryPath: "dsh-acp-demo",\n            configPath: "",\n            customModels: [],\n          },\n          droid:',
);
