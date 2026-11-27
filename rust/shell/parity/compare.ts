// Compare web.png and native.png for each screen directory under <dir>.
//
//   bun rust/shell/parity/compare.ts <dir> [--tolerance 8]
//
// For every <dir>/<name>/ holding both files, writes side.png (web left, native
// right), diff.png (the web capture dimmed, differing pixels in red) and adds a
// row to <dir>/report.md with the share of differing pixels. A pixel differs
// when any channel is more than --tolerance (0-255) apart. A native capture of
// another size is scaled to the web size first, and the report says so.
// Uses Playwright's Chromium canvas for PNG decode/encode (no new dependency).
import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const require = createRequire(join(here, "../../../apps/web/package.json"));
const { chromium } = require("playwright") as typeof import("playwright");

const args = process.argv.slice(2);
const root = args[0];
if (!root || root.startsWith("--")) {
  console.error("usage: bun compare.ts <dir> [--tolerance N]");
  process.exit(2);
}
const ti = args.indexOf("--tolerance");
const tolerance = ti >= 0 ? Number(args[ti + 1]) : 8;

// PARITY_CHROMIUM: optional path to an already installed Chromium/headless shell.
const browser = await chromium.launch({ executablePath: process.env.PARITY_CHROMIUM || undefined });
const page = await browser.newPage();
const dataUrl = (p: string) => `data:image/png;base64,${readFileSync(p).toString("base64")}`;
const rows: string[] = [];

for (const name of readdirSync(root).sort()) {
  const web = join(root, name, "web.png");
  const native = join(root, name, "native.png");
  if (!existsSync(web) || !existsSync(native)) continue;
  const result = await page.evaluate(
    async ({ webUrl, nativeUrl, tolerance }) => {
      const load = (src: string) =>
        new Promise<HTMLImageElement>((resolve, reject) => {
          const img = new Image();
          img.onload = () => resolve(img);
          img.onerror = reject;
          img.src = src;
        });
      const [w, n] = await Promise.all([load(webUrl), load(nativeUrl)]);
      const W = w.naturalWidth;
      const H = w.naturalHeight;
      const canvas = (cw: number, ch: number) => {
        const c = document.createElement("canvas");
        c.width = cw;
        c.height = ch;
        return [c, c.getContext("2d")!] as const;
      };
      const [, wc] = canvas(W, H);
      wc.drawImage(w, 0, 0);
      const [, nc] = canvas(W, H);
      nc.drawImage(n, 0, 0, W, H);
      const a = wc.getImageData(0, 0, W, H);
      const b = nc.getImageData(0, 0, W, H);
      const [dCanvas, dc] = canvas(W, H);
      const d = dc.createImageData(W, H);
      let differing = 0;
      for (let i = 0; i < a.data.length; i += 4) {
        const off =
          Math.abs(a.data[i] - b.data[i]) > tolerance ||
          Math.abs(a.data[i + 1] - b.data[i + 1]) > tolerance ||
          Math.abs(a.data[i + 2] - b.data[i + 2]) > tolerance;
        if (off) differing++;
        const grey = (a.data[i] + a.data[i + 1] + a.data[i + 2]) / 3;
        d.data[i] = off ? 255 : grey * 0.35;
        d.data[i + 1] = off ? 40 : grey * 0.35;
        d.data[i + 2] = off ? 40 : grey * 0.35;
        d.data[i + 3] = 255;
      }
      dc.putImageData(d, 0, 0);
      const [sCanvas, sc] = canvas(W * 2 + 8, H);
      sc.fillStyle = "#808080";
      sc.fillRect(0, 0, W * 2 + 8, H);
      sc.drawImage(w, 0, 0);
      sc.drawImage(n, W + 8, 0, W, H);
      return {
        side: sCanvas.toDataURL("image/png"),
        diff: dCanvas.toDataURL("image/png"),
        share: differing / (W * H),
        web: `${W}x${H}`,
        native: `${n.naturalWidth}x${n.naturalHeight}`,
      };
    },
    { webUrl: dataUrl(web), nativeUrl: dataUrl(native), tolerance },
  );
  const save = (url: string, file: string) =>
    writeFileSync(join(root, name, file), Buffer.from(url.split(",")[1], "base64"));
  save(result.side, "side.png");
  save(result.diff, "diff.png");
  const scaled = result.web === result.native ? "" : ` (native ${result.native} scaled)`;
  rows.push(`| ${name} | ${(result.share * 100).toFixed(2)}% | ${result.web}${scaled} |`);
  console.log(`${name}: ${(result.share * 100).toFixed(2)}% differing${scaled}`);
}
await browser.close();
writeFileSync(
  join(root, "report.md"),
  [`Differing pixels (tolerance ${tolerance}/255)`, "", "| screen | differing | size |", "|---|---|---|", ...rows, ""].join("\n"),
);
