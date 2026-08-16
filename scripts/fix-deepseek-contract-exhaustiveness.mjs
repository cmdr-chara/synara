import fs from "node:fs";

function replaceOnce(path, before, after, marker) {
  let source = fs.readFileSync(path, "utf8");
  if (marker && source.includes(marker)) return;
  if (!source.includes(before)) {
    throw new Error(`Expected patch context not found in ${path}: ${before.slice(0, 120)}`);
  }
  source = source.replace(before, after);
  fs.writeFileSync(path, source);
}

replaceOnce(
  "packages/contracts/src/agentMentions.ts",
  "  grok: {},\n  droid: {},",
  "  grok: {},\n  deepseek: {},\n  droid: {},",
  "  deepseek: {},\n  droid: {},",
);

replaceOnce(
  "packages/contracts/src/agentMentions.ts",
  "  grok: [],\n  droid: [],",
  "  grok: [],\n  deepseek: [],\n  droid: [],",
  "  deepseek: [],\n  droid: [],",
);

replaceOnce(
  "packages/contracts/src/model.ts",
  '  grok: "grok-build",\n  droid: "claude-opus-4-8",',
  '  grok: "grok-build",\n  deepseek: "deepseek-v4-pro",\n  droid: "claude-opus-4-8",',
  '  deepseek: "deepseek-v4-pro",\n  droid: "claude-opus-4-8",',
);
