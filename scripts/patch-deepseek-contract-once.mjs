import { readFileSync, writeFileSync } from "node:fs";

const path = "packages/contracts/src/providerDiscovery.ts";
const source = readFileSync(path, "utf8");
const before = '  "grok",\n  "droid",';
const after = '  "grok",\n  "deepseek",\n  "droid",';
if (!source.includes(before)) throw new Error("provider discovery insertion point not found");
writeFileSync(path, source.replace(before, after));
