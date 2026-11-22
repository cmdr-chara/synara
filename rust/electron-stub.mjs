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

import { register } from "node:module";

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
// randomBytes/randomUUID overridden for deterministic recordings.
const cryptoNames = [
  "argon2", "argon2Sync", "checkPrime", "checkPrimeSync", "createCipheriv",
  "createDecipheriv", "createDiffieHellman", "createDiffieHellmanGroup",
  "createECDH", "createHash", "createHmac", "createPrivateKey",
  "createPublicKey", "createSecretKey", "createSign", "createVerify",
  "diffieHellman", "generatePrime", "generatePrimeSync", "getCiphers",
  "getCipherInfo", "getCurves", "getDiffieHellman", "getHashes",
];
const CRYPTO_SOURCE = `
import { createRequire } from "node:module";
const crypto = createRequire(import.meta.url)("node:crypto");
const __orig = crypto.randomBytes;
export default crypto;
export const randomBytes = (n) => __orig(n).fill(0x07);
export const randomUUID = () => "ffffffff-ffff-ffff-ffff-ffffffffffff";
${cryptoNames.map((n) => `export const ${n} = crypto[${JSON.stringify(n)}];`).join("\n")}
`;

const HOOKS = `
import { readFile } from "node:fs/promises";
const STUB_MAP = JSON.parse(process.env.RA_STUB_MAP ?? "{}");
const SOURCES = ${JSON.stringify({
  electron: ELECTRON_SOURCE,
  "node:crypto": CRYPTO_SOURCE,
})};
const GENERIC = ${JSON.stringify(GENERIC_SOURCE)};

export async function resolve(specifier, context, next) {
  if (Object.hasOwn(SOURCES, specifier) || Object.hasOwn(STUB_MAP, specifier)) {
    return { url: \`ra-stub:\${specifier}\`, shortCircuit: true };
  }
  try {
    return await next(specifier, context);
  } catch (err) {
    if (err && (err.code === "ERR_MODULE_NOT_FOUND" || err.code === "ERR_UNKNOWN_FILE_EXTENSION")) {
      return { url: \`ra-stub:\${specifier}\`, shortCircuit: true };
    }
    throw err;
  }
}

export async function load(url, context, next) {
  if (url.startsWith("ra-stub:")) {
    const specifier = url.slice("ra-stub:".length);
    if (Object.hasOwn(SOURCES, specifier)) {
      return { format: "module", source: SOURCES[specifier], shortCircuit: true };
    }
    if (Object.hasOwn(STUB_MAP, specifier)) {
      const source = await readFile(STUB_MAP[specifier], "utf8");
      return { format: "module", source, shortCircuit: true };
    }
    return { format: "module", source: GENERIC, shortCircuit: true };
  }
  if (url.endsWith(".json")) {
    const text = await readFile(new URL(url), "utf8");
    const data = JSON.parse(text);
    const keys = data && typeof data === "object" && !Array.isArray(data) ? Object.keys(data) : [];
    const named = keys
      .filter((k) => /^[A-Za-z_$][A-Za-z0-9_$]*$/.test(k))
      .map((k) => \`export const \${k} = __json[\${JSON.stringify(k)}];\`)
      .join("\\n");
    return {
      format: "module",
      source: \`const __json = \${JSON.stringify(data)};\\nexport default __json;\\n\${named}\\n\`,
      shortCircuit: true,
    };
  }
  return next(url, context);
}
`;

register(`data:text/javascript,${encodeURIComponent(HOOKS)}`, import.meta.url);
