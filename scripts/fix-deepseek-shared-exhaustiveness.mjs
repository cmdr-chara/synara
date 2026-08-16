import fs from "node:fs";

function replaceOnce(path, before, after, marker) {
  let source = fs.readFileSync(path, "utf8");
  if (marker && source.includes(marker)) return;
  if (!source.includes(before)) throw new Error(`Missing patch context in ${path}`);
  source = source.replace(before, after);
  fs.writeFileSync(path, source);
}

replaceOnce(
  "packages/contracts/src/orchestration.ts",
  'export const DeepSeekModelSelection = Schema.Struct({\n  provider: Schema.Literal("deepseek"),\n  model: TrimmedNonEmptyString,\n});',
  'export const DeepSeekModelSelection = Schema.Struct({\n  provider: Schema.Literal("deepseek"),\n  model: TrimmedNonEmptyString,\n  options: Schema.optional(Schema.Struct({})),\n});',
  'provider: Schema.Literal("deepseek"),\n  model: TrimmedNonEmptyString,\n  options:',
);

replaceOnce(
  "packages/shared/src/model.ts",
  '  grok: new Set(MODEL_OPTIONS_BY_PROVIDER.grok.map((option) => option.slug)),\n  droid:',
  '  grok: new Set(MODEL_OPTIONS_BY_PROVIDER.grok.map((option) => option.slug)),\n  deepseek: new Set(MODEL_OPTIONS_BY_PROVIDER.deepseek.map((option) => option.slug)),\n  droid:',
  '  deepseek: new Set(MODEL_OPTIONS_BY_PROVIDER.deepseek.map((option) => option.slug)),',
);
