// Self-contained loader stub for `rust-anything record` runs.
// Use: rust-anything record <crate> --node-arg "--import <repo>/rust/electron-stub.mjs"
//
// Registers resolve/load hooks that:
//  - serve a deep-Proxy stub for `electron` (all named exports),
//  - freeze `node:crypto` randomBytes (0x07) + randomUUID (ff…f) so volatile
//    values record deterministically (replay via the crate's env vars),
//  - serve a generic deep-Proxy module for specifiers that fail to resolve
//    (native addons, desktop-only packages),
//  - load `.json` files without an import attribute (bundler leniency).
// Optional: RA_STUB_MAP='{"specifier":"/abs/stub.mjs"}' serves extra stubs.

import { register, createRequire } from "node:module";

// Detached promise rejections inside recorded code (async helpers called
// without await during module init, stray timers) must not kill record.
process.on("unhandledRejection", () => {});
process.on("uncaughtException", (err) => {
  console.error("stub: swallowed uncaughtException", err && err.message);
});

const PROXY = `
const __deep = new Proxy(function () {}, {
  get(t, k) {
    if (k === "then" || k === "toJSON" || k === Symbol.toPrimitive || k === Symbol.iterator) {
      return undefined;
    }
    if (k === "valueOf") return () => 0;
    if (k === "toString" || k === "inspect" || k === Symbol.toStringTag) return () => "";
    return __deep;
  },
  apply() { return __deep; },
  construct() { return __deep; },
  set() { return true; },
  has() { return true; },
});
export default __deep;
`;

const ELECTRON_NAMES = "app,clipboard,contextBridge,dialog,ipcMain,ipcRenderer,nativeImage,nativeTheme,protocol,safeStorage,session,shell,webContents,webUtils,BrowserWindow,WebContentsView,WebContents,IpcMainEvent,IpcMainInvokeEvent,MessageBoxOptions,MessageBoxReturnValue,Menu,Tray,screen,net,powerMonitor,powerSaveBlocker,globalShortcut,Notification,TouchBar,autoUpdater,crashReporter,inAppPurchase,systemPreferences,utilityProcess,pushNotifications,desktopCapturer,contentTracing".split(
  ",",
);

const ELECTRON_SOURCE =
  PROXY +
  ELECTRON_NAMES.map((n) => `export const ${n} = __deep;`).join("\n") +
  "\n";

const GENERIC_SOURCE = `${PROXY}\nexport const __generic = __deep;\n`;

// Frozen crypto: real module via createRequire (bypasses these hooks), with
// randomBytes/randomUUID overridden for deterministic recordings. Every real
// export is re-exported so named imports (X509Certificate, timingSafeEqual, ...)
// resolve.
const __cryptoRequire = createRequire("file:///ra-stub-base.mjs");
const cryptoNames = Object.getOwnPropertyNames(__cryptoRequire("node:crypto"))
  .filter(
    (n) =>
      !["default", "randomBytes", "randomUUID", "__esModule"].includes(n) &&
      /^[A-Za-z_$][A-Za-z0-9_$]*$/.test(n),
  );
const CRYPTO_SOURCE = `
import { createRequire } from "node:module";
const crypto = createRequire("file:///ra-stub-base.mjs")("node:crypto");
const __orig = crypto.randomBytes;
export default crypto;
export const randomBytes = (n) => __orig(n).fill(0x07);
export const randomUUID = () => "ffffffff-ffff-ffff-ffff-ffffffffffff";
${cryptoNames.map((n) => `export const ${n} = crypto[${JSON.stringify(n)}];`).join("\n")}
`;

const HOOKS = `
import { readFile } from "node:fs/promises";
import { readFileSync } from "node:fs";
const STUB_MAP = JSON.parse(process.env.RA_STUB_MAP ?? "{}");
const WANTED = new Map();
const RESERVED = new Set(
  "break case catch class const continue debugger default delete do else export extends finally for function if import in instanceof new return super switch this throw try typeof var void while with yield enum implements interface package private protected public static let await null true false".split(" "),
);
const SOURCES = ${JSON.stringify({
  electron: ELECTRON_SOURCE,
  "node:crypto": CRYPTO_SOURCE,
})};
const GENERIC = ${JSON.stringify(GENERIC_SOURCE)};

// Named imports a generic stub must provide, parsed from the importing file
// and carried in the stub URL query so each importer gets a module instance
// exporting the names it asks for.
const NAME_RE = /import\\s*\\{([^}]*)\\}\\s*from\\s*["']([^"']+)["']|export\\s*\\{([^}]*)\\}\\s*from\\s*["']([^"']+)["']/g;
const RESOLVE_SUFFIXES = ["", ".ts", ".tsx", ".mts", ".cts", ".js", ".mjs", "/index.ts", "/index.tsx", "/index.mts", "/index.js"];
function namesFor(specifier, parentURL, resolvedURL) {
  if (!parentURL || !parentURL.startsWith("file:")) return [];
  let text;
  try {
    text = readFileSync(new URL(parentURL), "utf8");
  } catch {
    return [];
  }
  const names = new Set();
  for (const m of text.matchAll(NAME_RE)) {
    const spec = m[2] ?? m[4];
    let hit = spec === specifier;
    if (!hit && resolvedURL && spec) {
      try {
        const abs = new URL(spec, parentURL).pathname;
        hit = RESOLVE_SUFFIXES.some(
          (suffix) => abs + suffix === resolvedURL.pathname,
        );
      } catch {}
    }
    if (!hit) continue;
    const list = (m[1] ?? m[3] ?? "").split(",");
    for (const item of list) {
      const trimmed = item.trim().replace(/^type\\s+/, "");
      if (!trimmed) continue;
      const parts = trimmed.split(/\\s+as\\s+/);
      const name = (parts[parts.length - 1] ?? "").trim();
      if (/^[A-Za-z_$][A-Za-z0-9_$]*$/.test(name)) names.add(name);
    }
  }
  return [...names];
}

// Node strip-types rejects TS parameter properties. Expand
// constructor(private readonly x) into a field + plain param + this.x = x.
function fixParamProps(src) {
  let out = src;
  let idx = 0;
  while (true) {
    const ci = out.indexOf("constructor", idx);
    if (ci < 0) break;
    let p = ci + "constructor".length;
    while (p < out.length && /\\s/.test(out[p])) p++;
    if (out[p] !== "(") { idx = p; continue; }
    const start = p;
    let depth = 0;
    let end = -1;
    let str = "";
    for (let i = start; i < out.length; i++) {
      const c = out[i];
      if (str) {
        if (c === "\\\\") { i++; continue; }
        if (c === str) str = "";
        continue;
      }
      if (c === '"' || c === "'") { str = c; continue; }
      if (c === "(") depth++;
      else if (c === ")") { depth--; if (depth === 0) { end = i; break; } }
    }
    if (end < 0) break;
    let q = end + 1;
    while (q < out.length && /\\s/.test(out[q])) q++;
    if (out[q] !== "{") { idx = q; continue; }
    const inner = out.slice(start + 1, end);
    const params = [];
    let cur = "";
    let d2 = 0;
    str = "";
    for (let j = 0; j < inner.length; j++) {
      const ch = inner[j];
      if (str) {
        cur += ch;
        if (ch === "\\\\") { cur += inner[++j]; continue; }
        if (ch === str) str = "";
        continue;
      }
      if (ch === '"' || ch === "'") { str = ch; cur += ch; continue; }
      if (ch === "(" || ch === "[" || ch === "<" || ch === "{") d2++;
      if (ch === ")" || ch === "]" || ch === ">" || ch === "}") d2--;
      if (ch === "," && d2 === 0) { params.push(cur); cur = ""; continue; }
      cur += ch;
    }
    if (cur.trim()) params.push(cur);
    const fields = [];
    const plain = [];
    let changed = false;
    for (const param of params) {
      const mm = param.match(/^\\s*((?:(?:private|public|protected|readonly)\\s+)+)([A-Za-z_$][A-Za-z0-9_$]*)/);
      if (mm && /\\b(private|public|protected|readonly)\\b/.test(mm[1])) {
        changed = true;
        fields.push(mm[2]);
        plain.push(mm[2] + param.slice(mm[0].length));
      } else {
        plain.push(param);
      }
    }
    if (!changed) { idx = q; continue; }
    const decls = fields.map((f) => f + ";").join("");
    const assigns = fields.map((f) => "this." + f + "=" + f + ";").join("");
    // In a derived class this is TDZ until super() returns: put the
    // assignments after the first super(...) call when the body has one.
    const body = out.slice(q + 1);
    let insertAt = 0;
    const superIdx = body.indexOf("super(");
    if (superIdx >= 0) {
      let d3 = 0;
      str = "";
      let closeIdx = -1;
      for (let i = superIdx + 5; i < body.length; i++) {
        const c = body[i];
        if (str) {
          if (c === "\\\\") { i++; continue; }
          if (c === str) str = "";
          continue;
        }
        if (c === '"' || c === "'" || c === "\\x60") { str = c; continue; }
        if (c === "(") d3++;
        else if (c === ")") { d3--; if (d3 === 0) { closeIdx = i; break; } }
      }
      if (closeIdx >= 0) {
        insertAt = closeIdx + 1;
        while (insertAt < body.length && /\\s/.test(body[insertAt])) insertAt++;
        if (body[insertAt] === ";") insertAt++;
      }
    }
    out = out.slice(0, ci) + decls + "constructor(" + plain.join(",") + "){" + body.slice(0, insertAt) + assigns + body.slice(insertAt);
    idx = ci;
  }
  return out;
}

export async function resolve(specifier, context, next) {
  if (Object.hasOwn(SOURCES, specifier) || Object.hasOwn(STUB_MAP, specifier)) {
    return { url: \`ra-stub:\${specifier}\`, shortCircuit: true };
  }
  try {
    const resolved = await next(specifier, context);
    const names = namesFor(specifier, context.parentURL, resolved?.url ? new URL(resolved.url) : undefined);
    if (names.length && resolved && resolved.url) WANTED.set(resolved.url, names);
    if (process.env.RA_DEBUG && specifier.includes("central"))
      console.error("RA_RES", specifier, "->", resolved && resolved.url, "names:", names.join(","), "parent:", context.parentURL);
    return resolved;
  } catch (err) {
    if (err && (err.code === "ERR_MODULE_NOT_FOUND" || err.code === "ERR_UNKNOWN_FILE_EXTENSION")) {
      const names = namesFor(specifier, context.parentURL);
      if (process.env.RA_DEBUG)
        console.error("RA_STUB", specifier, "parent:", context.parentURL, "names:", names.join(","));
      const q = names.length ? \`?names=\${names.join(",")}\` : "";
      return { url: \`ra-stub:\${specifier}\${q}\`, shortCircuit: true };
    }
    throw err;
  }
}

export async function load(url, context, next) {
  if (process.env.RA_DEBUG) console.error("RA_LOAD", url);
  if (url.startsWith("ra-stub:")) {
    const [specifier, query] = url.slice("ra-stub:".length).split("?");
    if (Object.hasOwn(SOURCES, specifier)) {
      return { format: "module", source: SOURCES[specifier], shortCircuit: true };
    }
    if (Object.hasOwn(STUB_MAP, specifier)) {
      const source = await readFile(STUB_MAP[specifier], "utf8");
      return { format: "module", source, shortCircuit: true };
    }
    const names = (query ?? "").replace(/^names=/, "").split(",").filter(Boolean);
    const named = names.map((n) => \`export const \${n} = __deep;\`).join("\\n");
    return { format: "module", source: GENERIC + named, shortCircuit: true };
  }
  if (url.split("?")[0].endsWith(".json")) {
    const text = await readFile(new URL(url), "utf8");
    const data = JSON.parse(text);
    const keys = data && typeof data === "object" && !Array.isArray(data) ? Object.keys(data) : [];
    const named = keys
      .filter((k) => /^[A-Za-z_$][A-Za-z0-9_$]*$/.test(k) && !RESERVED.has(k))
      .map((k) => \`export const \${k} = __json[\${JSON.stringify(k)}];\`)
      .join("\\n");
    return {
      format: "module",
      source: \`const __json = \${JSON.stringify(data)};\\nexport default __json;\\n\${named}\\n\`,
      shortCircuit: true,
    };
  }
  try {
    const result = await next(url, context);
    if (process.env.RA_DEBUG && url.includes("Errors.ts"))
      console.error("RA_DEBUG", url, result.format, typeof result.source);
    // Plain "import { X }" of an "export type X" survives type stripping and
    // fails to link; satisfy it with a dummy runtime export.
    if (
      url.startsWith("file:") &&
      (result.format === "module" || result.format === "module-typescript") &&
      result.source != null
    ) {
      const m = url.match(/\\.(ts|mts|cts|tsx)$/);
      if (m) {
        try {
          const orig = await readFile(new URL(url), "utf8");
          const typeNames = new Set();
          for (const tm of orig.matchAll(
            /export\\s+(?:declare\\s+)?(?:type|interface)\\s+([A-Za-z_$][A-Za-z0-9_$]*)/g,
          )) {
            typeNames.add(tm[1]);
          }
          for (const tm of orig.matchAll(
            /export\\s+(?:const|let|var|function|class|async\\s+function)\\s+([A-Za-z_$][A-Za-z0-9_$]*)/g,
          )) {
            typeNames.delete(tm[1]);
          }
          for (const tm of orig.matchAll(/export\\s*\\{([^}]*)\\}/g)) {
            for (const part of tm[1].split(",")) {
              const nm = part
                .trim()
                .replace(/^type\\s+/, "")
                .split(/\\s+as\\s+/);
              const name = (nm[nm.length - 1] ?? "").trim();
              if (/^[A-Za-z_$][A-Za-z0-9_$]*$/.test(name)) typeNames.delete(name);
            }
          }
          const envHeader =
            result.format === "module-typescript"
              ? "import.meta.env ??= {DEV:false,PROD:true,MODE:\\"production\\"};\\n"
              : "";
          let extras = "";
          if (typeNames.size) {
            const locals = new Set();
            for (const lm of orig.matchAll(
              /[^A-Za-z0-9_$](?:const|let|var|function|class|async\\s+function)\\s+([A-Za-z_$][A-Za-z0-9_$]*)/g,
            )) {
              locals.add(lm[1]);
            }
            extras =
              "\\n" +
              [...typeNames]
                .map((n) =>
                  locals.has(n)
                    ? \`export { \${n} };\`
                    : \`export const \${n} = undefined;\`,
                )
                .join("\\n") +
              "\\n";
          }
          const base =
            typeof result.source === "string"
              ? result.source
              : new TextDecoder().decode(result.source);
          const fixed = fixParamProps(base);
          if (typeNames.size || envHeader || fixed !== base) {
            return {
              format: result.format,
              source: envHeader + fixed + extras,
              shortCircuit: true,
            };
          }
        } catch {}
      }
    }
    return result;
  } catch (err) {
    if (err && (err.code === "ERR_UNKNOWN_FILE_EXTENSION" || err.code === "ERR_MODULE_NOT_FOUND" || err.code === "ERR_IMPORT_ATTRIBUTE_MISSING")) {
      let source = GENERIC;
      try {
        const text = await readFile(new URL(url), "utf8");
        const names = new Set();
        let hasDefault = false;
        for (const m of text.matchAll(
          /export\\s+(?:default\\s+)?(?:async\\s+)?(?:const|let|var|function|class|type|interface|enum)\\s*([A-Za-z_$][A-Za-z0-9_$]*)?/g,
        )) {
          if (m[0].includes("default")) hasDefault = true;
          else if (m[1]) names.add(m[1]);
        }
        for (const m of text.matchAll(/export\\s*\\{([^}]*)\\}/g)) {
          for (const part of m[1].split(",")) {
            const p = part.trim().replace(/^type\\s+/, "").split(/\\s+as\\s+/);
            const nm = (p[p.length - 1] ?? "").trim();
            if (/^[A-Za-z_$][A-Za-z0-9_$]*$/.test(nm) && !RESERVED.has(nm)) names.add(nm);
          }
        }
        const named = [...names]
          .map((n) => \`export const \${n} = __deep;\`)
          .join("\\n");
        source = GENERIC + named + (hasDefault ? "" : "");
      } catch {
        const names = WANTED.get(url) ?? [];
        const named = names
          .map((n) => \`export const \${n} = __deep;\`)
          .join("\\n");
        source = GENERIC + named;
      }
      return { format: "module", source, shortCircuit: true };
    }
    throw err;
  }
}
`;

// Optional clock freeze: `SYNARA_STATE_NOW_MS` pins `Date.now()` and
// argument-free `new Date()` so timestamp-bearing exports record
// deterministically (replay via the crate's env vars, like crypto above).
const NOW_MS = process.env.SYNARA_STATE_NOW_MS;
if (NOW_MS !== undefined) {
  const ms = Number(NOW_MS);
  const RealDate = Date;
  globalThis.Date = new Proxy(RealDate, {
    apply(_t, _this, _args) {
      return new RealDate(ms).toString();
    },
    construct(_t, args) {
      return args.length ? Reflect.construct(RealDate, args) : new RealDate(ms);
    },
    get(t, k) {
      if (k === "now") return () => ms;
      return t[k];
    },
  });
}

register(`data:text/javascript,${encodeURIComponent(HOOKS)}`, import.meta.url);
