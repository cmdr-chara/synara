import fs from "node:fs";

const path = "apps/web/src/components/PluginLibrary.tsx";
let text = fs.readFileSync(path, "utf8");
const marker = "    deepseek: { plugins: false, skills: false },";
if (!text.includes(marker)) {
  const before = `    grok: {\n      plugins: supportsPluginDiscovery(grokCapabilitiesQuery.data),\n      skills: supportsSkillDiscovery(grokCapabilitiesQuery.data),\n    },\n    droid:`;
  const after = `    grok: {\n      plugins: supportsPluginDiscovery(grokCapabilitiesQuery.data),\n      skills: supportsSkillDiscovery(grokCapabilitiesQuery.data),\n    },\n    deepseek: { plugins: false, skills: false },\n    droid:`;
  if (!text.includes(before)) {
    throw new Error("Could not locate PluginLibrary provider capability map");
  }
  text = text.replace(before, after);
  fs.writeFileSync(path, text);
}
