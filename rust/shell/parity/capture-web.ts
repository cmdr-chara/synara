// Capture the Electron/web reference at an exact viewport, one PNG per screen.
//
//   bun rust/shell/parity/capture-web.ts <out-dir> [--base http://localhost:5733]
//       [--size 1562x1075] [--light] [--only name,name] [--styles "<css selector>"]
//
// Writes <out-dir>/<name>/web.png for every entry in screens.json. With --styles,
// also writes web-styles.json: getComputedStyle + rect for each matching element,
// which is where the exact numbers for a native fix come from.
// Uses the Playwright that apps/web already depends on (no new dependency);
// run `bun run --cwd apps/web test:browser:install` once if Chromium is missing,
// or set PARITY_CHROMIUM to an existing Chromium executable.
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const require = createRequire(join(here, "../../../apps/web/package.json"));
const { chromium } = require("playwright") as typeof import("playwright");

const args = process.argv.slice(2);
const out = args[0];
if (!out || out.startsWith("--")) {
  console.error("usage: bun capture-web.ts <out-dir> [--base URL] [--size WxH] [--light] [--only a,b] [--styles SELECTOR]");
  process.exit(2);
}
const opt = (name: string) => {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : undefined;
};
const base = opt("--base") ?? "http://localhost:5733";
const [width, height] = (opt("--size") ?? "1562x1075").split("x").map(Number);
const only = opt("--only")?.split(",");
const styles = opt("--styles");

type Screen = { name: string; web: string; native: string };
const screens = (JSON.parse(readFileSync(join(here, "screens.json"), "utf8")) as Screen[]).filter(
  (s) => !only || only.includes(s.name),
);

// One-off overlays that are not part of the screen under test.
const DISMISS = [/skip intro/i, /close tour/i, /dismiss provider status/i, /dismiss what's new/i, /^not now$/i];

// PARITY_CHROMIUM: optional path to an already installed Chromium/headless shell.
const browser = await chromium.launch({ executablePath: process.env.PARITY_CHROMIUM || undefined });
const ctx = await browser.newContext({
  viewport: { width, height },
  deviceScaleFactor: 1,
  colorScheme: args.includes("--light") ? "light" : "dark",
  reducedMotion: "reduce",
});
const page = await ctx.newPage();

for (const screen of screens) {
  await page.goto(base + screen.web, { waitUntil: "load" });
  await page.waitForTimeout(3000);
  for (let round = 0; round < 3; round++) {
    let dismissed = false;
    for (const name of DISMISS) {
      const button = page.getByRole("button", { name }).first();
      if (await button.count()) {
        await button.click({ force: true, timeout: 2000 }).catch(() => {});
        dismissed = true;
      }
    }
    if (!dismissed) break;
    await page.waitForTimeout(800);
  }
  // Park the pointer where it hovers nothing, so no hover state is captured.
  await page.mouse.move(width - 1, height - 1);
  await page.waitForTimeout(600);
  const dir = join(out, screen.name);
  mkdirSync(dir, { recursive: true });
  await page.screenshot({ path: join(dir, "web.png") });
  if (styles) {
    const dump = await page.$$eval(styles, (els) =>
      els.map((el) => {
        const cs = getComputedStyle(el);
        const r = el.getBoundingClientRect();
        const pick = [
          "font-family", "font-size", "font-weight", "line-height", "letter-spacing", "color",
          "background-color", "padding", "margin", "gap", "border-radius", "border-top-width",
          "border-top-color", "opacity", "width", "height",
        ];
        return {
          tag: el.tagName.toLowerCase(),
          text: (el.textContent ?? "").trim().slice(0, 60),
          rect: [r.x, r.y, r.width, r.height].map((v) => Math.round(v * 10) / 10),
          style: Object.fromEntries(pick.map((p) => [p, cs.getPropertyValue(p)])),
        };
      }),
    );
    writeFileSync(join(dir, "web-styles.json"), JSON.stringify(dump, null, 2));
  }
  console.log(`${screen.name}: ${page.url()}`);
}
await browser.close();
