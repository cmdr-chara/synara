// Loads the native module built by `rust-anything build`.
const native = require('./lib_rs.node')

// Structured values cross as JSON text. JSON has no `undefined`, NaN, -0, bigint, Date, Map, Set
// or bytes, and Rust strings no lone surrogates: those travel as one-key marker objects
// (`{"$undefined": true}`, `{"$num": "NaN"}`, `{"$map": [[k, v]]}`, `{"$bytes": [...]}`, ...),
// the same markers recordings use.
const replacer = function (k, v) {
  const raw = this[k]
  if (raw instanceof Date) return { $date: Number.isNaN(raw.getTime()) ? 'Invalid' : raw.toISOString() }
  if (typeof Buffer !== 'undefined' && Buffer.isBuffer(raw)) return { $buffer: Array.from(raw) }
  if (raw instanceof Uint8Array) return { $bytes: Array.from(raw) }
  if (raw instanceof Map) return { $map: [...raw] }
  if (raw instanceof Set) return { $set: [...raw] }
  return v === undefined ? { $undefined: true }
    : typeof v === 'bigint' ? { $bigint: String(v) }
    : typeof v === 'number' ? (Number.isFinite(v) && !Object.is(v, -0) ? v : { $num: Object.is(v, -0) ? '-0' : String(v) })
    : typeof v === 'string' && !v.isWellFormed() ? { $utf16: Array.from({ length: v.length }, (_, i) => v.charCodeAt(i)) }
    : v
}
const encode = v => JSON.stringify(v, replacer)
const single = (v, key) => Object.keys(v).length === 1 && key in v
const revive = v => Array.isArray(v) ? v.map(revive)
  : v !== null && typeof v === 'object'
    ? (single(v, '$undefined') ? undefined
      : single(v, '$utf16') ? fromUnits(v.$utf16)
      : single(v, '$num') ? Number(v.$num)
      : single(v, '$bigint') ? BigInt(v.$bigint)
      : single(v, '$date') ? new Date(v.$date)
      : single(v, '$bytes') ? Uint8Array.from(v.$bytes)
      : single(v, '$buffer') ? Buffer.from(v.$buffer)
      : single(v, '$map') ? new Map(v.$map.map(([k, x]) => [revive(k), revive(x)]))
      : single(v, '$set') ? new Set(v.$set.map(revive))
      : Object.fromEntries(Object.entries(v).map(([k, x]) => [k, revive(x)])))
    : v
// Most results carry no marker: skip the walk unless the text has one.
const decode = text => {
  const value = JSON.parse(text)
  return text.includes('"$') ? revive(value) : value
}
const fromUnits = (units) => {
  let out = ''
  for (let i = 0; i < units.length; i += 8192)
    out += String.fromCharCode.apply(null, Array.prototype.slice.call(units, i, i + 8192))
  return out
}

module.exports = { ...native }
const specs = {
  useTemplateSet: [[1], 'json', false],
  currentTemplateSet: [[], 'json', false],
  buildName: [[1], 'json', false],
  buildInfo: [[1], 'json', false],
  prepareBuild: [[1, 1, 1], 'json', false],
  templatePaths: [[1], 'json', false],
  templateInfo: [[1], 'json', false],
  freshHomeFromTemplate: [[1], 'json', false],
  removeHome: [[1], 'json', false],
  clearChromiumCaches: [[1], 'json', false],
  seed: [[1, 1, 1, 1], 'json', false],
  createTemplates: [[1, 1], 'json', true],
  fetchJson: [[1, 0], 'json', true],
  listTargets: [[1], 'json', true],
  waitFor: [[1, 1], 'json', true],
  sleep: [[1], 'json', false],
  createProfileSummarizer: [[1], 'json', false],
  elementCenter: [[1, 1], 'json', true],
  clickAt: [[1, 1], 'json', true],
  clickSelector: [[1, 1, 1], 'json', true],
  clickLabel: [[1, 1], 'json', true],
  dismissAnnouncements: [[1, 1], 'json', true],
  focusEmulation: [[1, 0], 'json', true],
  pressKey: [[1, 1, 0], 'json', true],
  typeCharacter: [[1, 1], 'json', true],
  wheel: [[1, 1], 'json', true],
  composerText: [[1], 'json', true],
  screenshot: [[1, 1], 'json', true],
  openThreadFromSidebar: [[1, 1, 1], 'json', true],
  bootHeadlessBackend: [[1, 1, 1], 'json', true],
  assertIsolatedHome: [[1], 'json', false],
  resolveElectronBinary: [[1], 'json', false],
  freePort: [[], 'json', true],
  isolatedEnvironment: [[1], 'json', false],
  processTree: [[1], 'json', false],
  classifyProcess: [[1, 1], 'json', false],
  launchDesktop: [[1], 'json', false],
  attachMainPage: [[1, 1], 'json', true],
  quitDesktop: [[1, 1], 'json', true],
  isFrontmost: [[1], 'json', false],
  secondsSinceUserInput: [[], 'json', false],
  waitForQuietMac: [[1], 'json', true],
  screenLocked: [[], 'json', false],
  frontmostProblem: [[1, 1], 'json', false],
  footprintMb: [[1], 'json', false],
  memorySnapshot: [[1], 'json', false],
  rssTotalMb: [[1], 'json', false],
  cpuOverWindow: [[1, 1], 'json', true],
  wakeupsOverWindow: [[1, 1], 'json', false],
  gpuSystemSample: [[], 'json', false],
  gpuSystemOverWindow: [[1], 'json', true],
  machineState: [[], 'json', false],
  machineIdentity: [[], 'json', false],
  median: [[1], 'json', false],
  percentile: [[1, 1], 'json', false],
  summarize: [[1, 1], 'json', false],
  gap: [[1, 1], 'json', false],
  startApp: [[1], 'json', true],
  windowBounds: [[1], 'json', true],
  measureIdle: [[1], 'json', true],
  measureSwitching: [[1], 'json', true],
  measureLongChat: [[1], 'json', true],
  measureLoad: [[1], 'json', true],
  measureLaunch: [[1], 'json', true],
}
const classes = {

}
const convert = result => result === 'json' ? decode : result === 'units' ? fromUnits : x => x
// An omitted structured argument is `undefined`, not a missing value.
const prepare = (a, structured) => Array.from({ length: Math.max(a.length, structured.length) }, (_, i) => structured[i] ? encode(a[i]) : a[i])
// Functions: [structured arguments, result kind, async].
for (const [name, [structured, result, isAsync]] of Object.entries(specs)) {
  const call = native[name]
  const out = convert(result)
  module.exports[name] = isAsync
    ? async (...a) => out(await call(...prepare(a, structured)))
    : (...a) => out(call(...prepare(a, structured)))
}
// Classes: a JS class around the native one, converting arguments and results.
const INNER = Symbol('native')
for (const [name, [ctor, methods]] of Object.entries(classes)) {
  const Native = native[name]
  const Wrapped = { [name]: class { constructor(...a) { this[INNER] = new Native(...prepare(a, ctor)) } } }[name]
  for (const [method, [structured, result]] of Object.entries(methods)) {
    const out = convert(result)
    Wrapped.prototype[method] = function (...a) { return out(this[INNER][method](...prepare(a, structured))) }
  }
  module.exports[name] = Wrapped
}

// The binary wire (see rust-anything's `wire.rs`): arguments are written into `wIn`, results read
// from `wOut`; both are registered with the native module once.
let wIn = new Uint8Array(1 << 16), wOut = new Uint8Array(1 << 16)
const wLen = new Uint32Array(1)
let iv = new DataView(wIn.buffer), ov = new DataView(wOut.buffer), at = 0, p = 0
native.rustAnythingWireInit(wIn, wOut, wLen)
const regrow = () => native.rustAnythingWireInit(wIn, wOut, wLen)
// Runs a wire function; a result larger than `wOut` is fetched once `wOut` has grown.
const wireCall = fn => {
  wLen[0] = at
  const n = fn()
  if (Number.isNaN(n)) return false
  if (n === Infinity) {
    p = 0
    throw new Error(rStr())
  }
  if (n < 0) {
    wOut = new Uint8Array(Math.max(wOut.length * 2, -n))
    ov = new DataView(wOut.buffer)
    native.rustAnythingWireInit(wIn, wOut, wLen)
    native.rustAnythingWireFlush()
  }
  p = 0
  return true
}

const need = n => {
  if (at + n <= wIn.length) return
  const grown = new Uint8Array(Math.max(wIn.length * 2, at + n))
  grown.set(wIn)
  wIn = grown
  iv = new DataView(wIn.buffer)
  regrow()
}
const wU8 = v => { need(1); iv.setUint8(at++, v) }
const wU32 = v => { need(4); iv.setUint32(at, v, true); at += 4 }
const wF64 = v => { need(8); iv.setFloat64(at, v, true); at += 8 }
const wUnits = s => {
  need(4 + 2 * s.length)
  iv.setUint32(at, s.length, true)
  at += 4
  for (let i = 0; i < s.length; i++) { iv.setUint16(at, s.charCodeAt(i), true); at += 2 }
}
const wStr = s => { wU8(4); wUnits(s) }
const wDyn = v => {
  switch (typeof v) {
    case 'undefined': wU8(0); return true
    case 'boolean': wU8(v ? 3 : 2); return true
    case 'number': wU8(4); wF64(v); return true
    case 'string': wU8(5); wUnits(v); return true
    case 'object':
      if (v === null) { wU8(1); return true }
      if (Array.isArray(v)) {
        wU8(6); wU32(v.length)
        for (let i = 0; i < v.length; i++) if (!wDyn(v[i])) return false
        return true
      }
      if (Object.getPrototypeOf(v) !== Object.prototype && Object.getPrototypeOf(v) !== null) return false
      {
        const keys = Object.keys(v)
        wU8(7); wU32(keys.length)
        for (const k of keys) { wUnits(k); if (!wDyn(v[k])) return false }
        return true
      }
  }
  return false
}
const rU8 = () => ov.getUint8(p++)
const rU32 = () => { const v = ov.getUint32(p, true); p += 4; return v }
const rF64 = () => { const v = ov.getFloat64(p, true); p += 8; return v }
// ignoreBOM: a leading U+FEFF is part of the string, not a byte-order mark.
const utf16 = new TextDecoder('utf-16le', { ignoreBOM: true })
const rUnits = (wellFormed) => {
  const n = rU32()
  let s
  if (n > 24 && wellFormed) s = utf16.decode(wOut.subarray(p, p + 2 * n))
  else {
    s = ''
    for (let i = 0; i < n; i++) s += String.fromCharCode(ov.getUint16(p + 2 * i, true))
  }
  p += 2 * n
  return s
}
// Tag 1: a string from Rust (well-formed); 4: raw code units, lone surrogates included.
const rStr = () => rUnits(rU8() === 1)
const rDyn = () => {
  switch (rU8()) {
    case 0: return undefined
    case 1: return null
    case 2: return false
    case 3: return true
    case 4: return rF64()
    case 5: return rUnits(false)
    case 6: { const n = rU32(); const a = new Array(n); for (let i = 0; i < n; i++) a[i] = rDyn(); return a }
    case 7: { const n = rU32(); const o = {}; for (let i = 0; i < n; i++) { const k = rUnits(false); o[k] = rDyn() } return o }
  }
}

// `useTemplateSet`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.useTemplateSet
const call = native.useTemplateSetWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.useTemplateSet = function useTemplateSet (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `currentTemplateSet`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.currentTemplateSet
const call = native.currentTemplateSetWire
const encode = () => {
at = 0
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.currentTemplateSet = function currentTemplateSet () {
  try {
    if (arguments.length <= 0 && encode() && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `buildName`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.buildName
const call = native.buildNameWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.buildName = function buildName (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `buildInfo`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.buildInfo
const call = native.buildInfoWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.buildInfo = function buildInfo (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `prepareBuild`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.prepareBuild
const call = native.prepareBuildWire
const encode = (a0, a1, a2) => {
at = 0
if (!wDyn(a0)) return false
if (!wDyn(a1)) return false
if (!wDyn(a2)) return false
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.prepareBuild = function prepareBuild (a0, a1, a2) {
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `templatePaths`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.templatePaths
const call = native.templatePathsWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.templatePaths = function templatePaths (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `templateInfo`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.templateInfo
const call = native.templateInfoWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.templateInfo = function templateInfo (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `freshHomeFromTemplate`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.freshHomeFromTemplate
const call = native.freshHomeFromTemplateWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.freshHomeFromTemplate = function freshHomeFromTemplate (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `removeHome`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.removeHome
const call = native.removeHomeWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.removeHome = function removeHome (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `clearChromiumCaches`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.clearChromiumCaches
const call = native.clearChromiumCachesWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.clearChromiumCaches = function clearChromiumCaches (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `seed`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.seed
const call = native.seedWire
const encode = (a0, a1, a2, a3) => {
at = 0
if (!wDyn(a0)) return false
if (!wDyn(a1)) return false
if (!wDyn(a2)) return false
if (!wDyn(a3)) return false
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.seed = function seed (a0, a1, a2, a3) {
  try {
    if (arguments.length <= 4 && encode(a0, a1, a2, a3) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `sleep`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.sleep
const call = native.sleepWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.sleep = function sleep (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `createProfileSummarizer`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.createProfileSummarizer
const call = native.createProfileSummarizerWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.createProfileSummarizer = function createProfileSummarizer (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `assertIsolatedHome`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.assertIsolatedHome
const call = native.assertIsolatedHomeWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.assertIsolatedHome = function assertIsolatedHome (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `resolveElectronBinary`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.resolveElectronBinary
const call = native.resolveElectronBinaryWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.resolveElectronBinary = function resolveElectronBinary (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `isolatedEnvironment`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.isolatedEnvironment
const call = native.isolatedEnvironmentWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.isolatedEnvironment = function isolatedEnvironment (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `processTree`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.processTree
const call = native.processTreeWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.processTree = function processTree (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `classifyProcess`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.classifyProcess
const call = native.classifyProcessWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (!wDyn(a1)) return false
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.classifyProcess = function classifyProcess (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `launchDesktop`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.launchDesktop
const call = native.launchDesktopWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.launchDesktop = function launchDesktop (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `isFrontmost`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.isFrontmost
const call = native.isFrontmostWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.isFrontmost = function isFrontmost (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `secondsSinceUserInput`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.secondsSinceUserInput
const call = native.secondsSinceUserInputWire
const encode = () => {
at = 0
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.secondsSinceUserInput = function secondsSinceUserInput () {
  try {
    if (arguments.length <= 0 && encode() && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `screenLocked`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.screenLocked
const call = native.screenLockedWire
const encode = () => {
at = 0
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.screenLocked = function screenLocked () {
  try {
    if (arguments.length <= 0 && encode() && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `frontmostProblem`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.frontmostProblem
const call = native.frontmostProblemWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (!wDyn(a1)) return false
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.frontmostProblem = function frontmostProblem (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `footprintMb`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.footprintMb
const call = native.footprintMbWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.footprintMb = function footprintMb (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `memorySnapshot`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.memorySnapshot
const call = native.memorySnapshotWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.memorySnapshot = function memorySnapshot (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `rssTotalMb`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.rssTotalMb
const call = native.rssTotalMbWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.rssTotalMb = function rssTotalMb (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `wakeupsOverWindow`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.wakeupsOverWindow
const call = native.wakeupsOverWindowWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (!wDyn(a1)) return false
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.wakeupsOverWindow = function wakeupsOverWindow (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `gpuSystemSample`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.gpuSystemSample
const call = native.gpuSystemSampleWire
const encode = () => {
at = 0
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.gpuSystemSample = function gpuSystemSample () {
  try {
    if (arguments.length <= 0 && encode() && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `machineState`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.machineState
const call = native.machineStateWire
const encode = () => {
at = 0
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.machineState = function machineState () {
  try {
    if (arguments.length <= 0 && encode() && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `machineIdentity`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.machineIdentity
const call = native.machineIdentityWire
const encode = () => {
at = 0
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.machineIdentity = function machineIdentity () {
  try {
    if (arguments.length <= 0 && encode() && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `median`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.median
const call = native.medianWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.median = function median (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `percentile`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.percentile
const call = native.percentileWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (!wDyn(a1)) return false
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.percentile = function percentile (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `summarize`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.summarize
const call = native.summarizeWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (!wDyn(a1)) return false
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.summarize = function summarize (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `gap`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.gap
const call = native.gapWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (!wDyn(a1)) return false
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.gap = function gap (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}
const guards = {
  createTemplates: [true, []],
  fetchJson: [true, []],
  listTargets: [true, []],
  waitFor: [true, []],
  elementCenter: [true, []],
  clickAt: [true, []],
  clickSelector: [true, []],
  clickLabel: [true, []],
  dismissAnnouncements: [true, []],
  focusEmulation: [true, []],
  pressKey: [true, []],
  typeCharacter: [true, []],
  wheel: [true, []],
  composerText: [true, []],
  screenshot: [true, []],
  openThreadFromSidebar: [true, []],
  bootHeadlessBackend: [true, []],
  freePort: [true, []],
  attachMainPage: [true, []],
  quitDesktop: [true, []],
  waitForQuietMac: [true, []],
  cpuOverWindow: [true, []],
  gpuSystemOverWindow: [true, []],
  startApp: [true, []],
  windowBounds: [true, []],
  measureIdle: [true, []],
  measureSwitching: [true, []],
  measureLongChat: [true, []],
  measureLoad: [true, []],
  measureLaunch: [true, []],
}
const JS_ERRORS = /^(TypeError|RangeError|SyntaxError|ReferenceError|URIError|EvalError): ([\s\S]*)$/
const jsError = e => {
  const message = typeof e?.message === 'string' ? e.message : ''
  const typed = JS_ERRORS.exec(message)
  if (typed) return new globalThis[typed[1]](typed[2])
  if (/Expected$/.test(e?.code)) return Object.assign(new TypeError(message), { code: e.code })
  if (message.startsWith('Rust panic:')) e.code = 'RUST_PANIC'
  return e
}
const illFormed = v => typeof v === 'string' ? !v.isWellFormed() : Array.isArray(v) && v.some(illFormed)
for (const [name, [isAsync, strings]] of Object.entries(guards)) {
  const f = module.exports[name]
  if (typeof f !== 'function') continue
  const check = a => {
    for (const i of strings) {
      if (illFormed(a[i])) throw new TypeError(`argument ${i + 1} of ${name} has a lone surrogate, which a Rust String cannot hold`)
    }
  }
  module.exports[name] = isAsync
    ? { [name]: async function (...a) { check(a); try { return await f(...a) } catch (e) { throw jsError(e) } } }[name]
    : { [name]: function (...a) { check(a); try { return f(...a) } catch (e) { throw jsError(e) } } }[name]
}
