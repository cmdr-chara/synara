// Loads the native module built by `rust-anything build`.
const native = require('./theme_rs.node')

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
  isThemeMode: [[1], 'raw', false],
  isThemeVariant: [[1], 'raw', false],
  getAvailableCodeThemes: [[0], 'json', false],
  normalizeCodeThemeId: [[1, 0, 1], 'json', false],
  normalizeThemeFonts: [[1], 'json', false],
  normalizeSemanticColors: [[1, 1], 'json', false],
  normalizeChromeTheme: [[1, 0], 'json', false],
  normalizeWindowTranslucency: [[1, 0], 'json', false],
  normalizeThemePack: [[1, 0], 'json', false],
  getCodeThemeSeed: [[0, 0], 'json', false],
  normalizeThemeState: [[1], 'json', false],
  parseStoredThemeState: [[0], 'json', false],
  serializeThemeState: [[1], 'raw', false],
  createThemeShareString: [[0, 1], 'raw', false],
  parseThemeShareString: [[0], 'json', false],
  parseThemeShareStringForVariant: [[0, 0], 'json', false],
  updateThemePackFromShareString: [[1, 0, 0], 'json', false],
  updateChromeTheme: [[1, 0, 1], 'json', false],
  setThemeCodeThemeId: [[1, 0, 0], 'json', false],
  setThemeFonts: [[1, 0, 1], 'json', false],
  resetThemeVariant: [[1, 0], 'json', false],
  setWindowTranslucency: [[1, 0, 1], 'json', false],
  areWindowTranslucenciesEqual: [[1, 1], 'raw', false],
  resolveThemePack: [[1, 0], 'json', false],
  resolveProfileThemeAccent: [[1], 'json', false],
  areThemePacksEqual: [[1, 1], 'raw', false],
  buildResolvedThemeTokens: [[1, 0], 'json', false],
  buildThemeCssVariables: [[1, 0, 1], 'json', false],
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

// `isThemeMode`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.isThemeMode
const call = native.isThemeModeWire
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
module.exports.isThemeMode = function isThemeMode (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `isThemeVariant`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.isThemeVariant
const call = native.isThemeVariantWire
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
module.exports.isThemeVariant = function isThemeVariant (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `getAvailableCodeThemes`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.getAvailableCodeThemes
const call = native.getAvailableCodeThemesWire
const encode = (a0) => {
at = 0
if (typeof a0 !== 'string') return false
wStr(a0)
return true
}
const decode = () => {
let t
if (rU8() === 0) t = null
else {
const n0 = rU32()
const a0 = new Array(n0)
for (let i0 = 0; i0 < n0; i0++) {
let v0
{
const o1 = {}
const n1 = rU8()
for (let j1 = 0; j1 < n1; j1++) {
switch (rU8()) {
case 0: {
let v1
v1 = rStr()
o1["id"] = v1
break
}
case 1: {
let v1
v1 = rStr()
o1["label"] = v1
break
}
case 2: {
let v1
if (rU8() === 0) v1 = null
else {
const n2 = rU32()
const a2 = new Array(n2)
for (let i2 = 0; i2 < n2; i2++) {
let v2
v2 = rStr()
a2[i2] = v2
}
v1 = a2
}
o1["variants"] = v1
break
}
default: {
const k1 = rStr()
o1[k1] = rDyn()
}
}
}
v0 = o1
}
a0[i0] = v0
}
t = a0
}
return t
}
module.exports.getAvailableCodeThemes = function getAvailableCodeThemes (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of getAvailableCodeThemes has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `isCodeThemeAvailable`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.isCodeThemeAvailable
const call = native.isCodeThemeAvailableWire
const encode = (a0, a1) => {
at = 0
if (typeof a0 !== 'string') return false
wStr(a0)
if (typeof a1 !== 'string') return false
wStr(a1)
return true
}
const decode = () => {
let t
t = rU8() === 1
return t
}
module.exports.isCodeThemeAvailable = function isCodeThemeAvailable (a0, a1) {
  if (illFormed(a0)) throw new TypeError('argument 1 of isCodeThemeAvailable has a lone surrogate, which a Rust String cannot hold')
  if (illFormed(a1)) throw new TypeError('argument 2 of isCodeThemeAvailable has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `normalizeCodeThemeId`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.normalizeCodeThemeId
const call = native.normalizeCodeThemeIdWire
const encode = (a0, a1, a2) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'string') return false
wStr(a1)
if (!wDyn(a2)) return false
return true
}
const decode = () => {
let t
t = rStr()
return t
}
module.exports.normalizeCodeThemeId = function normalizeCodeThemeId (a0, a1, a2) {
  if (illFormed(a1)) throw new TypeError('argument 2 of normalizeCodeThemeId has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `normalizeThemeFonts`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.normalizeThemeFonts
const call = native.normalizeThemeFontsWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
{
const o0 = {}
const n0 = rU8()
for (let j0 = 0; j0 < n0; j0++) {
switch (rU8()) {
case 0: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["ui"] = v0
break
}
case 1: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["code"] = v0
break
}
default: {
const k0 = rStr()
o0[k0] = rDyn()
}
}
}
t = o0
}
return t
}
module.exports.normalizeThemeFonts = function normalizeThemeFonts (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `normalizeSemanticColors`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.normalizeSemanticColors
const call = native.normalizeSemanticColorsWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (!wDyn(a1)) return false
return true
}
const decode = () => {
let t
{
const o0 = {}
const n0 = rU8()
for (let j0 = 0; j0 < n0; j0++) {
switch (rU8()) {
case 0: {
let v0
v0 = rStr()
o0["diffAdded"] = v0
break
}
case 1: {
let v0
v0 = rStr()
o0["diffRemoved"] = v0
break
}
case 2: {
let v0
v0 = rStr()
o0["skill"] = v0
break
}
default: {
const k0 = rStr()
o0[k0] = rDyn()
}
}
}
t = o0
}
return t
}
module.exports.normalizeSemanticColors = function normalizeSemanticColors (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `normalizeChromeTheme`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.normalizeChromeTheme
const call = native.normalizeChromeThemeWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'string') return false
wStr(a1)
return true
}
const decode = () => {
let t
{
const o0 = {}
const n0 = rU8()
for (let j0 = 0; j0 < n0; j0++) {
switch (rU8()) {
case 0: {
let v0
v0 = rStr()
o0["accent"] = v0
break
}
case 1: {
let v0
v0 = rF64()
o0["contrast"] = v0
break
}
case 2: {
let v0
{
const o1 = {}
const n1 = rU8()
for (let j1 = 0; j1 < n1; j1++) {
switch (rU8()) {
case 0: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["ui"] = v1
break
}
case 1: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["code"] = v1
break
}
default: {
const k1 = rStr()
o1[k1] = rDyn()
}
}
}
v0 = o1
}
o0["fonts"] = v0
break
}
case 3: {
let v0
v0 = rStr()
o0["ink"] = v0
break
}
case 4: {
let v0
v0 = rU8() === 1
o0["opaqueWindows"] = v0
break
}
case 5: {
let v0
{
const o1 = {}
const n1 = rU8()
for (let j1 = 0; j1 < n1; j1++) {
switch (rU8()) {
case 0: {
let v1
v1 = rStr()
o1["diffAdded"] = v1
break
}
case 1: {
let v1
v1 = rStr()
o1["diffRemoved"] = v1
break
}
case 2: {
let v1
v1 = rStr()
o1["skill"] = v1
break
}
default: {
const k1 = rStr()
o1[k1] = rDyn()
}
}
}
v0 = o1
}
o0["semanticColors"] = v0
break
}
case 6: {
let v0
v0 = rStr()
o0["surface"] = v0
break
}
default: {
const k0 = rStr()
o0[k0] = rDyn()
}
}
}
t = o0
}
return t
}
module.exports.normalizeChromeTheme = function normalizeChromeTheme (a0, a1) {
  if (illFormed(a1)) throw new TypeError('argument 2 of normalizeChromeTheme has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `normalizeWindowTranslucency`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.normalizeWindowTranslucency
const call = native.normalizeWindowTranslucencyWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'string') return false
wStr(a1)
return true
}
const decode = () => {
let t
{
const o0 = {}
const n0 = rU8()
for (let j0 = 0; j0 < n0; j0++) {
switch (rU8()) {
case 0: {
let v0
v0 = rF64()
o0["opacity"] = v0
break
}
case 1: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rF64()
}
}
o0["blur"] = v0
break
}
case 2: {
let v0
v0 = rU8() === 1
o0["sidebarOnly"] = v0
break
}
default: {
const k0 = rStr()
o0[k0] = rDyn()
}
}
}
t = o0
}
return t
}
module.exports.normalizeWindowTranslucency = function normalizeWindowTranslucency (a0, a1) {
  if (illFormed(a1)) throw new TypeError('argument 2 of normalizeWindowTranslucency has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `normalizeThemePack`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.normalizeThemePack
const call = native.normalizeThemePackWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'string') return false
wStr(a1)
return true
}
const decode = () => {
let t
{
const o0 = {}
const n0 = rU8()
for (let j0 = 0; j0 < n0; j0++) {
switch (rU8()) {
case 0: {
let v0
v0 = rStr()
o0["codeThemeId"] = v0
break
}
case 1: {
let v0
{
const o1 = {}
const n1 = rU8()
for (let j1 = 0; j1 < n1; j1++) {
switch (rU8()) {
case 0: {
let v1
v1 = rStr()
o1["accent"] = v1
break
}
case 1: {
let v1
v1 = rF64()
o1["contrast"] = v1
break
}
case 2: {
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["ui"] = v2
break
}
case 1: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["code"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v1 = o2
}
o1["fonts"] = v1
break
}
case 3: {
let v1
v1 = rStr()
o1["ink"] = v1
break
}
case 4: {
let v1
v1 = rU8() === 1
o1["opaqueWindows"] = v1
break
}
case 5: {
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
v2 = rStr()
o2["diffAdded"] = v2
break
}
case 1: {
let v2
v2 = rStr()
o2["diffRemoved"] = v2
break
}
case 2: {
let v2
v2 = rStr()
o2["skill"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v1 = o2
}
o1["semanticColors"] = v1
break
}
case 6: {
let v1
v1 = rStr()
o1["surface"] = v1
break
}
default: {
const k1 = rStr()
o1[k1] = rDyn()
}
}
}
v0 = o1
}
o0["theme"] = v0
break
}
default: {
const k0 = rStr()
o0[k0] = rDyn()
}
}
}
t = o0
}
return t
}
module.exports.normalizeThemePack = function normalizeThemePack (a0, a1) {
  if (illFormed(a1)) throw new TypeError('argument 2 of normalizeThemePack has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `getCodeThemeSeed`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.getCodeThemeSeed
const call = native.getCodeThemeSeedWire
const encode = (a0, a1) => {
at = 0
if (typeof a0 !== 'string') return false
wStr(a0)
if (typeof a1 !== 'string') return false
wStr(a1)
return true
}
const decode = () => {
let t
{
const o0 = {}
const n0 = rU8()
for (let j0 = 0; j0 < n0; j0++) {
switch (rU8()) {
case 0: {
let v0
v0 = rStr()
o0["accent"] = v0
break
}
case 1: {
let v0
v0 = rF64()
o0["contrast"] = v0
break
}
case 2: {
let v0
{
const o1 = {}
const n1 = rU8()
for (let j1 = 0; j1 < n1; j1++) {
switch (rU8()) {
case 0: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["ui"] = v1
break
}
case 1: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["code"] = v1
break
}
default: {
const k1 = rStr()
o1[k1] = rDyn()
}
}
}
v0 = o1
}
o0["fonts"] = v0
break
}
case 3: {
let v0
v0 = rStr()
o0["ink"] = v0
break
}
case 4: {
let v0
v0 = rU8() === 1
o0["opaqueWindows"] = v0
break
}
case 5: {
let v0
{
const o1 = {}
const n1 = rU8()
for (let j1 = 0; j1 < n1; j1++) {
switch (rU8()) {
case 0: {
let v1
v1 = rStr()
o1["diffAdded"] = v1
break
}
case 1: {
let v1
v1 = rStr()
o1["diffRemoved"] = v1
break
}
case 2: {
let v1
v1 = rStr()
o1["skill"] = v1
break
}
default: {
const k1 = rStr()
o1[k1] = rDyn()
}
}
}
v0 = o1
}
o0["semanticColors"] = v0
break
}
case 6: {
let v0
v0 = rStr()
o0["surface"] = v0
break
}
default: {
const k0 = rStr()
o0[k0] = rDyn()
}
}
}
t = o0
}
return t
}
module.exports.getCodeThemeSeed = function getCodeThemeSeed (a0, a1) {
  if (illFormed(a0)) throw new TypeError('argument 1 of getCodeThemeSeed has a lone surrogate, which a Rust String cannot hold')
  if (illFormed(a1)) throw new TypeError('argument 2 of getCodeThemeSeed has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `normalizeThemeState`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.normalizeThemeState
const call = native.normalizeThemeStateWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
{
const o0 = {}
const n0 = rU8()
for (let j0 = 0; j0 < n0; j0++) {
switch (rU8()) {
case 0: {
let v0
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const o1 = {}
for (let i1 = 0; i1 < n1; i1++) {
const k1 = rStr()
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
v2 = rStr()
o2["accent"] = v2
break
}
case 1: {
let v2
v2 = rF64()
o2["contrast"] = v2
break
}
case 2: {
let v2
v2 = rDyn()
o2["fonts"] = v2
break
}
case 3: {
let v2
v2 = rStr()
o2["ink"] = v2
break
}
case 4: {
let v2
v2 = rU8() === 1
o2["opaqueWindows"] = v2
break
}
case 5: {
let v2
v2 = rDyn()
o2["semanticColors"] = v2
break
}
case 6: {
let v2
v2 = rStr()
o2["surface"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v1 = o2
}
o1[k1] = v1
}
v0 = o1
}
o0["chromeThemes"] = v0
break
}
case 1: {
let v0
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const o1 = {}
for (let i1 = 0; i1 < n1; i1++) {
const k1 = rStr()
let v1
v1 = rStr()
o1[k1] = v1
}
v0 = o1
}
o0["codeThemeIds"] = v0
break
}
case 2: {
let v0
v0 = rStr()
o0["mode"] = v0
break
}
case 3: {
let v0
v0 = rU8() === 1
o0["systemUiFont"] = v0
break
}
case 4: {
let v0
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const o1 = {}
for (let i1 = 0; i1 < n1; i1++) {
const k1 = rStr()
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
v2 = rF64()
o2["opacity"] = v2
break
}
case 1: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rF64()
}
}
o2["blur"] = v2
break
}
case 2: {
let v2
v2 = rU8() === 1
o2["sidebarOnly"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v1 = o2
}
o1[k1] = v1
}
v0 = o1
}
o0["translucency"] = v0
break
}
default: {
const k0 = rStr()
o0[k0] = rDyn()
}
}
}
t = o0
}
return t
}
module.exports.normalizeThemeState = function normalizeThemeState (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `parseStoredThemeState`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.parseStoredThemeState
const call = native.parseStoredThemeStateWire
const encode = (a0) => {
at = 0
if (a0 === undefined) {
wU8(0)
} else if (a0 === null) {
wU8(2)
} else {
wU8(1)
if (typeof a0 !== 'string') return false
wStr(a0)
}
return true
}
const decode = () => {
let t
{
const o0 = {}
const n0 = rU8()
for (let j0 = 0; j0 < n0; j0++) {
switch (rU8()) {
case 0: {
let v0
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const o1 = {}
for (let i1 = 0; i1 < n1; i1++) {
const k1 = rStr()
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
v2 = rStr()
o2["accent"] = v2
break
}
case 1: {
let v2
v2 = rF64()
o2["contrast"] = v2
break
}
case 2: {
let v2
v2 = rDyn()
o2["fonts"] = v2
break
}
case 3: {
let v2
v2 = rStr()
o2["ink"] = v2
break
}
case 4: {
let v2
v2 = rU8() === 1
o2["opaqueWindows"] = v2
break
}
case 5: {
let v2
v2 = rDyn()
o2["semanticColors"] = v2
break
}
case 6: {
let v2
v2 = rStr()
o2["surface"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v1 = o2
}
o1[k1] = v1
}
v0 = o1
}
o0["chromeThemes"] = v0
break
}
case 1: {
let v0
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const o1 = {}
for (let i1 = 0; i1 < n1; i1++) {
const k1 = rStr()
let v1
v1 = rStr()
o1[k1] = v1
}
v0 = o1
}
o0["codeThemeIds"] = v0
break
}
case 2: {
let v0
v0 = rStr()
o0["mode"] = v0
break
}
case 3: {
let v0
v0 = rU8() === 1
o0["systemUiFont"] = v0
break
}
case 4: {
let v0
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const o1 = {}
for (let i1 = 0; i1 < n1; i1++) {
const k1 = rStr()
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
v2 = rF64()
o2["opacity"] = v2
break
}
case 1: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rF64()
}
}
o2["blur"] = v2
break
}
case 2: {
let v2
v2 = rU8() === 1
o2["sidebarOnly"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v1 = o2
}
o1[k1] = v1
}
v0 = o1
}
o0["translucency"] = v0
break
}
default: {
const k0 = rStr()
o0[k0] = rDyn()
}
}
}
t = o0
}
return t
}
module.exports.parseStoredThemeState = function parseStoredThemeState (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of parseStoredThemeState has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `serializeThemeState`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.serializeThemeState
const call = native.serializeThemeStateWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
t = rStr()
return t
}
module.exports.serializeThemeState = function serializeThemeState (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `createThemeShareString`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.createThemeShareString
const call = native.createThemeShareStringWire
const encode = (a0, a1) => {
at = 0
if (typeof a0 !== 'string') return false
wStr(a0)
if (!wDyn(a1)) return false
return true
}
const decode = () => {
let t
t = rStr()
return t
}
module.exports.createThemeShareString = function createThemeShareString (a0, a1) {
  if (illFormed(a0)) throw new TypeError('argument 1 of createThemeShareString has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `parseThemeShareString`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.parseThemeShareString
const call = native.parseThemeShareStringWire
const encode = (a0) => {
at = 0
if (typeof a0 !== 'string') return false
wStr(a0)
return true
}
const decode = () => {
let t
{
const o0 = {}
const n0 = rU8()
for (let j0 = 0; j0 < n0; j0++) {
switch (rU8()) {
case 0: {
let v0
v0 = rStr()
o0["codeThemeId"] = v0
break
}
case 1: {
let v0
{
const o1 = {}
const n1 = rU8()
for (let j1 = 0; j1 < n1; j1++) {
switch (rU8()) {
case 0: {
let v1
v1 = rStr()
o1["accent"] = v1
break
}
case 1: {
let v1
v1 = rF64()
o1["contrast"] = v1
break
}
case 2: {
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["ui"] = v2
break
}
case 1: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["code"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v1 = o2
}
o1["fonts"] = v1
break
}
case 3: {
let v1
v1 = rStr()
o1["ink"] = v1
break
}
case 4: {
let v1
v1 = rU8() === 1
o1["opaqueWindows"] = v1
break
}
case 5: {
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
v2 = rStr()
o2["diffAdded"] = v2
break
}
case 1: {
let v2
v2 = rStr()
o2["diffRemoved"] = v2
break
}
case 2: {
let v2
v2 = rStr()
o2["skill"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v1 = o2
}
o1["semanticColors"] = v1
break
}
case 6: {
let v1
v1 = rStr()
o1["surface"] = v1
break
}
default: {
const k1 = rStr()
o1[k1] = rDyn()
}
}
}
v0 = o1
}
o0["theme"] = v0
break
}
case 2: {
let v0
v0 = rStr()
o0["variant"] = v0
break
}
default: {
const k0 = rStr()
o0[k0] = rDyn()
}
}
}
t = o0
}
return t
}
module.exports.parseThemeShareString = function parseThemeShareString (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of parseThemeShareString has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `canParseThemeShareString`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.canParseThemeShareString
const call = native.canParseThemeShareStringWire
const encode = (a0, a1) => {
at = 0
if (typeof a0 !== 'string') return false
wStr(a0)
if (a1 === undefined) {
wU8(0)
} else if (a1 === null) {
return false
wU8(2)
} else {
wU8(1)
if (typeof a1 !== 'string') return false
wStr(a1)
}
return true
}
const decode = () => {
let t
t = rU8() === 1
return t
}
module.exports.canParseThemeShareString = function canParseThemeShareString (a0, a1) {
  if (illFormed(a0)) throw new TypeError('argument 1 of canParseThemeShareString has a lone surrogate, which a Rust String cannot hold')
  if (illFormed(a1)) throw new TypeError('argument 2 of canParseThemeShareString has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `parseThemeShareStringForVariant`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.parseThemeShareStringForVariant
const call = native.parseThemeShareStringForVariantWire
const encode = (a0, a1) => {
at = 0
if (typeof a0 !== 'string') return false
wStr(a0)
if (a1 === undefined) {
wU8(0)
} else if (a1 === null) {
return false
wU8(2)
} else {
wU8(1)
if (typeof a1 !== 'string') return false
wStr(a1)
}
return true
}
const decode = () => {
let t
{
const o0 = {}
const n0 = rU8()
for (let j0 = 0; j0 < n0; j0++) {
switch (rU8()) {
case 0: {
let v0
v0 = rStr()
o0["codeThemeId"] = v0
break
}
case 1: {
let v0
{
const o1 = {}
const n1 = rU8()
for (let j1 = 0; j1 < n1; j1++) {
switch (rU8()) {
case 0: {
let v1
v1 = rStr()
o1["accent"] = v1
break
}
case 1: {
let v1
v1 = rF64()
o1["contrast"] = v1
break
}
case 2: {
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["ui"] = v2
break
}
case 1: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["code"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v1 = o2
}
o1["fonts"] = v1
break
}
case 3: {
let v1
v1 = rStr()
o1["ink"] = v1
break
}
case 4: {
let v1
v1 = rU8() === 1
o1["opaqueWindows"] = v1
break
}
case 5: {
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
v2 = rStr()
o2["diffAdded"] = v2
break
}
case 1: {
let v2
v2 = rStr()
o2["diffRemoved"] = v2
break
}
case 2: {
let v2
v2 = rStr()
o2["skill"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v1 = o2
}
o1["semanticColors"] = v1
break
}
case 6: {
let v1
v1 = rStr()
o1["surface"] = v1
break
}
default: {
const k1 = rStr()
o1[k1] = rDyn()
}
}
}
v0 = o1
}
o0["theme"] = v0
break
}
case 2: {
let v0
v0 = rStr()
o0["variant"] = v0
break
}
default: {
const k0 = rStr()
o0[k0] = rDyn()
}
}
}
t = o0
}
return t
}
module.exports.parseThemeShareStringForVariant = function parseThemeShareStringForVariant (a0, a1) {
  if (illFormed(a0)) throw new TypeError('argument 1 of parseThemeShareStringForVariant has a lone surrogate, which a Rust String cannot hold')
  if (illFormed(a1)) throw new TypeError('argument 2 of parseThemeShareStringForVariant has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `updateThemePackFromShareString`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.updateThemePackFromShareString
const call = native.updateThemePackFromShareStringWire
const encode = (a0, a1, a2) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'string') return false
wStr(a1)
if (typeof a2 !== 'string') return false
wStr(a2)
return true
}
const decode = () => {
let t
{
const o0 = {}
const n0 = rU8()
for (let j0 = 0; j0 < n0; j0++) {
switch (rU8()) {
case 0: {
let v0
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const o1 = {}
for (let i1 = 0; i1 < n1; i1++) {
const k1 = rStr()
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
v2 = rStr()
o2["accent"] = v2
break
}
case 1: {
let v2
v2 = rF64()
o2["contrast"] = v2
break
}
case 2: {
let v2
v2 = rDyn()
o2["fonts"] = v2
break
}
case 3: {
let v2
v2 = rStr()
o2["ink"] = v2
break
}
case 4: {
let v2
v2 = rU8() === 1
o2["opaqueWindows"] = v2
break
}
case 5: {
let v2
v2 = rDyn()
o2["semanticColors"] = v2
break
}
case 6: {
let v2
v2 = rStr()
o2["surface"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v1 = o2
}
o1[k1] = v1
}
v0 = o1
}
o0["chromeThemes"] = v0
break
}
case 1: {
let v0
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const o1 = {}
for (let i1 = 0; i1 < n1; i1++) {
const k1 = rStr()
let v1
v1 = rStr()
o1[k1] = v1
}
v0 = o1
}
o0["codeThemeIds"] = v0
break
}
case 2: {
let v0
v0 = rStr()
o0["mode"] = v0
break
}
case 3: {
let v0
v0 = rU8() === 1
o0["systemUiFont"] = v0
break
}
case 4: {
let v0
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const o1 = {}
for (let i1 = 0; i1 < n1; i1++) {
const k1 = rStr()
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
v2 = rF64()
o2["opacity"] = v2
break
}
case 1: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rF64()
}
}
o2["blur"] = v2
break
}
case 2: {
let v2
v2 = rU8() === 1
o2["sidebarOnly"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v1 = o2
}
o1[k1] = v1
}
v0 = o1
}
o0["translucency"] = v0
break
}
default: {
const k0 = rStr()
o0[k0] = rDyn()
}
}
}
t = o0
}
return t
}
module.exports.updateThemePackFromShareString = function updateThemePackFromShareString (a0, a1, a2) {
  if (illFormed(a1)) throw new TypeError('argument 2 of updateThemePackFromShareString has a lone surrogate, which a Rust String cannot hold')
  if (illFormed(a2)) throw new TypeError('argument 3 of updateThemePackFromShareString has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `updateChromeTheme`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.updateChromeTheme
const call = native.updateChromeThemeWire
const encode = (a0, a1, a2) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'string') return false
wStr(a1)
if (!wDyn(a2)) return false
return true
}
const decode = () => {
let t
{
const o0 = {}
const n0 = rU8()
for (let j0 = 0; j0 < n0; j0++) {
switch (rU8()) {
case 0: {
let v0
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const o1 = {}
for (let i1 = 0; i1 < n1; i1++) {
const k1 = rStr()
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
v2 = rStr()
o2["accent"] = v2
break
}
case 1: {
let v2
v2 = rF64()
o2["contrast"] = v2
break
}
case 2: {
let v2
v2 = rDyn()
o2["fonts"] = v2
break
}
case 3: {
let v2
v2 = rStr()
o2["ink"] = v2
break
}
case 4: {
let v2
v2 = rU8() === 1
o2["opaqueWindows"] = v2
break
}
case 5: {
let v2
v2 = rDyn()
o2["semanticColors"] = v2
break
}
case 6: {
let v2
v2 = rStr()
o2["surface"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v1 = o2
}
o1[k1] = v1
}
v0 = o1
}
o0["chromeThemes"] = v0
break
}
case 1: {
let v0
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const o1 = {}
for (let i1 = 0; i1 < n1; i1++) {
const k1 = rStr()
let v1
v1 = rStr()
o1[k1] = v1
}
v0 = o1
}
o0["codeThemeIds"] = v0
break
}
case 2: {
let v0
v0 = rStr()
o0["mode"] = v0
break
}
case 3: {
let v0
v0 = rU8() === 1
o0["systemUiFont"] = v0
break
}
case 4: {
let v0
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const o1 = {}
for (let i1 = 0; i1 < n1; i1++) {
const k1 = rStr()
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
v2 = rF64()
o2["opacity"] = v2
break
}
case 1: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rF64()
}
}
o2["blur"] = v2
break
}
case 2: {
let v2
v2 = rU8() === 1
o2["sidebarOnly"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v1 = o2
}
o1[k1] = v1
}
v0 = o1
}
o0["translucency"] = v0
break
}
default: {
const k0 = rStr()
o0[k0] = rDyn()
}
}
}
t = o0
}
return t
}
module.exports.updateChromeTheme = function updateChromeTheme (a0, a1, a2) {
  if (illFormed(a1)) throw new TypeError('argument 2 of updateChromeTheme has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `setThemeCodeThemeId`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.setThemeCodeThemeId
const call = native.setThemeCodeThemeIdWire
const encode = (a0, a1, a2) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'string') return false
wStr(a1)
if (typeof a2 !== 'string') return false
wStr(a2)
return true
}
const decode = () => {
let t
{
const o0 = {}
const n0 = rU8()
for (let j0 = 0; j0 < n0; j0++) {
switch (rU8()) {
case 0: {
let v0
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const o1 = {}
for (let i1 = 0; i1 < n1; i1++) {
const k1 = rStr()
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
v2 = rStr()
o2["accent"] = v2
break
}
case 1: {
let v2
v2 = rF64()
o2["contrast"] = v2
break
}
case 2: {
let v2
v2 = rDyn()
o2["fonts"] = v2
break
}
case 3: {
let v2
v2 = rStr()
o2["ink"] = v2
break
}
case 4: {
let v2
v2 = rU8() === 1
o2["opaqueWindows"] = v2
break
}
case 5: {
let v2
v2 = rDyn()
o2["semanticColors"] = v2
break
}
case 6: {
let v2
v2 = rStr()
o2["surface"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v1 = o2
}
o1[k1] = v1
}
v0 = o1
}
o0["chromeThemes"] = v0
break
}
case 1: {
let v0
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const o1 = {}
for (let i1 = 0; i1 < n1; i1++) {
const k1 = rStr()
let v1
v1 = rStr()
o1[k1] = v1
}
v0 = o1
}
o0["codeThemeIds"] = v0
break
}
case 2: {
let v0
v0 = rStr()
o0["mode"] = v0
break
}
case 3: {
let v0
v0 = rU8() === 1
o0["systemUiFont"] = v0
break
}
case 4: {
let v0
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const o1 = {}
for (let i1 = 0; i1 < n1; i1++) {
const k1 = rStr()
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
v2 = rF64()
o2["opacity"] = v2
break
}
case 1: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rF64()
}
}
o2["blur"] = v2
break
}
case 2: {
let v2
v2 = rU8() === 1
o2["sidebarOnly"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v1 = o2
}
o1[k1] = v1
}
v0 = o1
}
o0["translucency"] = v0
break
}
default: {
const k0 = rStr()
o0[k0] = rDyn()
}
}
}
t = o0
}
return t
}
module.exports.setThemeCodeThemeId = function setThemeCodeThemeId (a0, a1, a2) {
  if (illFormed(a1)) throw new TypeError('argument 2 of setThemeCodeThemeId has a lone surrogate, which a Rust String cannot hold')
  if (illFormed(a2)) throw new TypeError('argument 3 of setThemeCodeThemeId has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `setThemeFonts`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.setThemeFonts
const call = native.setThemeFontsWire
const encode = (a0, a1, a2) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'string') return false
wStr(a1)
if (!wDyn(a2)) return false
return true
}
const decode = () => {
let t
{
const o0 = {}
const n0 = rU8()
for (let j0 = 0; j0 < n0; j0++) {
switch (rU8()) {
case 0: {
let v0
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const o1 = {}
for (let i1 = 0; i1 < n1; i1++) {
const k1 = rStr()
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
v2 = rStr()
o2["accent"] = v2
break
}
case 1: {
let v2
v2 = rF64()
o2["contrast"] = v2
break
}
case 2: {
let v2
v2 = rDyn()
o2["fonts"] = v2
break
}
case 3: {
let v2
v2 = rStr()
o2["ink"] = v2
break
}
case 4: {
let v2
v2 = rU8() === 1
o2["opaqueWindows"] = v2
break
}
case 5: {
let v2
v2 = rDyn()
o2["semanticColors"] = v2
break
}
case 6: {
let v2
v2 = rStr()
o2["surface"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v1 = o2
}
o1[k1] = v1
}
v0 = o1
}
o0["chromeThemes"] = v0
break
}
case 1: {
let v0
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const o1 = {}
for (let i1 = 0; i1 < n1; i1++) {
const k1 = rStr()
let v1
v1 = rStr()
o1[k1] = v1
}
v0 = o1
}
o0["codeThemeIds"] = v0
break
}
case 2: {
let v0
v0 = rStr()
o0["mode"] = v0
break
}
case 3: {
let v0
v0 = rU8() === 1
o0["systemUiFont"] = v0
break
}
case 4: {
let v0
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const o1 = {}
for (let i1 = 0; i1 < n1; i1++) {
const k1 = rStr()
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
v2 = rF64()
o2["opacity"] = v2
break
}
case 1: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rF64()
}
}
o2["blur"] = v2
break
}
case 2: {
let v2
v2 = rU8() === 1
o2["sidebarOnly"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v1 = o2
}
o1[k1] = v1
}
v0 = o1
}
o0["translucency"] = v0
break
}
default: {
const k0 = rStr()
o0[k0] = rDyn()
}
}
}
t = o0
}
return t
}
module.exports.setThemeFonts = function setThemeFonts (a0, a1, a2) {
  if (illFormed(a1)) throw new TypeError('argument 2 of setThemeFonts has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `resetThemeVariant`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.resetThemeVariant
const call = native.resetThemeVariantWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'string') return false
wStr(a1)
return true
}
const decode = () => {
let t
{
const o0 = {}
const n0 = rU8()
for (let j0 = 0; j0 < n0; j0++) {
switch (rU8()) {
case 0: {
let v0
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const o1 = {}
for (let i1 = 0; i1 < n1; i1++) {
const k1 = rStr()
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
v2 = rStr()
o2["accent"] = v2
break
}
case 1: {
let v2
v2 = rF64()
o2["contrast"] = v2
break
}
case 2: {
let v2
v2 = rDyn()
o2["fonts"] = v2
break
}
case 3: {
let v2
v2 = rStr()
o2["ink"] = v2
break
}
case 4: {
let v2
v2 = rU8() === 1
o2["opaqueWindows"] = v2
break
}
case 5: {
let v2
v2 = rDyn()
o2["semanticColors"] = v2
break
}
case 6: {
let v2
v2 = rStr()
o2["surface"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v1 = o2
}
o1[k1] = v1
}
v0 = o1
}
o0["chromeThemes"] = v0
break
}
case 1: {
let v0
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const o1 = {}
for (let i1 = 0; i1 < n1; i1++) {
const k1 = rStr()
let v1
v1 = rStr()
o1[k1] = v1
}
v0 = o1
}
o0["codeThemeIds"] = v0
break
}
case 2: {
let v0
v0 = rStr()
o0["mode"] = v0
break
}
case 3: {
let v0
v0 = rU8() === 1
o0["systemUiFont"] = v0
break
}
case 4: {
let v0
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const o1 = {}
for (let i1 = 0; i1 < n1; i1++) {
const k1 = rStr()
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
v2 = rF64()
o2["opacity"] = v2
break
}
case 1: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rF64()
}
}
o2["blur"] = v2
break
}
case 2: {
let v2
v2 = rU8() === 1
o2["sidebarOnly"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v1 = o2
}
o1[k1] = v1
}
v0 = o1
}
o0["translucency"] = v0
break
}
default: {
const k0 = rStr()
o0[k0] = rDyn()
}
}
}
t = o0
}
return t
}
module.exports.resetThemeVariant = function resetThemeVariant (a0, a1) {
  if (illFormed(a1)) throw new TypeError('argument 2 of resetThemeVariant has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `setWindowTranslucency`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.setWindowTranslucency
const call = native.setWindowTranslucencyWire
const encode = (a0, a1, a2) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'string') return false
wStr(a1)
if (!wDyn(a2)) return false
return true
}
const decode = () => {
let t
{
const o0 = {}
const n0 = rU8()
for (let j0 = 0; j0 < n0; j0++) {
switch (rU8()) {
case 0: {
let v0
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const o1 = {}
for (let i1 = 0; i1 < n1; i1++) {
const k1 = rStr()
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
v2 = rStr()
o2["accent"] = v2
break
}
case 1: {
let v2
v2 = rF64()
o2["contrast"] = v2
break
}
case 2: {
let v2
v2 = rDyn()
o2["fonts"] = v2
break
}
case 3: {
let v2
v2 = rStr()
o2["ink"] = v2
break
}
case 4: {
let v2
v2 = rU8() === 1
o2["opaqueWindows"] = v2
break
}
case 5: {
let v2
v2 = rDyn()
o2["semanticColors"] = v2
break
}
case 6: {
let v2
v2 = rStr()
o2["surface"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v1 = o2
}
o1[k1] = v1
}
v0 = o1
}
o0["chromeThemes"] = v0
break
}
case 1: {
let v0
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const o1 = {}
for (let i1 = 0; i1 < n1; i1++) {
const k1 = rStr()
let v1
v1 = rStr()
o1[k1] = v1
}
v0 = o1
}
o0["codeThemeIds"] = v0
break
}
case 2: {
let v0
v0 = rStr()
o0["mode"] = v0
break
}
case 3: {
let v0
v0 = rU8() === 1
o0["systemUiFont"] = v0
break
}
case 4: {
let v0
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const o1 = {}
for (let i1 = 0; i1 < n1; i1++) {
const k1 = rStr()
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
v2 = rF64()
o2["opacity"] = v2
break
}
case 1: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rF64()
}
}
o2["blur"] = v2
break
}
case 2: {
let v2
v2 = rU8() === 1
o2["sidebarOnly"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v1 = o2
}
o1[k1] = v1
}
v0 = o1
}
o0["translucency"] = v0
break
}
default: {
const k0 = rStr()
o0[k0] = rDyn()
}
}
}
t = o0
}
return t
}
module.exports.setWindowTranslucency = function setWindowTranslucency (a0, a1, a2) {
  if (illFormed(a1)) throw new TypeError('argument 2 of setWindowTranslucency has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `areWindowTranslucenciesEqual`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.areWindowTranslucenciesEqual
const call = native.areWindowTranslucenciesEqualWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (!wDyn(a1)) return false
return true
}
const decode = () => {
let t
t = rU8() === 1
return t
}
module.exports.areWindowTranslucenciesEqual = function areWindowTranslucenciesEqual (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `resolveThemePack`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.resolveThemePack
const call = native.resolveThemePackWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'string') return false
wStr(a1)
return true
}
const decode = () => {
let t
{
const o0 = {}
const n0 = rU8()
for (let j0 = 0; j0 < n0; j0++) {
switch (rU8()) {
case 0: {
let v0
v0 = rStr()
o0["codeThemeId"] = v0
break
}
case 1: {
let v0
{
const o1 = {}
const n1 = rU8()
for (let j1 = 0; j1 < n1; j1++) {
switch (rU8()) {
case 0: {
let v1
v1 = rStr()
o1["accent"] = v1
break
}
case 1: {
let v1
v1 = rF64()
o1["contrast"] = v1
break
}
case 2: {
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["ui"] = v2
break
}
case 1: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["code"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v1 = o2
}
o1["fonts"] = v1
break
}
case 3: {
let v1
v1 = rStr()
o1["ink"] = v1
break
}
case 4: {
let v1
v1 = rU8() === 1
o1["opaqueWindows"] = v1
break
}
case 5: {
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
v2 = rStr()
o2["diffAdded"] = v2
break
}
case 1: {
let v2
v2 = rStr()
o2["diffRemoved"] = v2
break
}
case 2: {
let v2
v2 = rStr()
o2["skill"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v1 = o2
}
o1["semanticColors"] = v1
break
}
case 6: {
let v1
v1 = rStr()
o1["surface"] = v1
break
}
default: {
const k1 = rStr()
o1[k1] = rDyn()
}
}
}
v0 = o1
}
o0["theme"] = v0
break
}
default: {
const k0 = rStr()
o0[k0] = rDyn()
}
}
}
t = o0
}
return t
}
module.exports.resolveThemePack = function resolveThemePack (a0, a1) {
  if (illFormed(a1)) throw new TypeError('argument 2 of resolveThemePack has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `resolveProfileThemeAccent`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.resolveProfileThemeAccent
const call = native.resolveProfileThemeAccentWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
{
const o0 = {}
const n0 = rU8()
for (let j0 = 0; j0 < n0; j0++) {
switch (rU8()) {
case 0: {
let v0
v0 = rStr()
o0["light"] = v0
break
}
case 1: {
let v0
v0 = rStr()
o0["dark"] = v0
break
}
default: {
const k0 = rStr()
o0[k0] = rDyn()
}
}
}
t = o0
}
return t
}
module.exports.resolveProfileThemeAccent = function resolveProfileThemeAccent (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `areThemePacksEqual`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.areThemePacksEqual
const call = native.areThemePacksEqualWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (!wDyn(a1)) return false
return true
}
const decode = () => {
let t
t = rU8() === 1
return t
}
module.exports.areThemePacksEqual = function areThemePacksEqual (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `resolveThemeVariant`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.resolveThemeVariant
const call = native.resolveThemeVariantWire
const encode = (a0, a1) => {
at = 0
if (typeof a0 !== 'string') return false
wStr(a0)
if (typeof a1 !== 'boolean') return false
wU8(a1 ? 1 : 0)
return true
}
const decode = () => {
let t
t = rStr()
return t
}
module.exports.resolveThemeVariant = function resolveThemeVariant (a0, a1) {
  if (illFormed(a0)) throw new TypeError('argument 1 of resolveThemeVariant has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `buildResolvedThemeTokens`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.buildResolvedThemeTokens
const call = native.buildResolvedThemeTokensWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'string') return false
wStr(a1)
return true
}
const decode = () => {
let t
{
const o0 = {}
const n0 = rU8()
for (let j0 = 0; j0 < n0; j0++) {
switch (rU8()) {
case 0: {
let v0
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const o1 = {}
for (let i1 = 0; i1 < n1; i1++) {
const k1 = rStr()
let v1
v1 = rStr()
o1[k1] = v1
}
v0 = o1
}
o0["aliases"] = v0
break
}
case 1: {
let v0
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const o1 = {}
for (let i1 = 0; i1 < n1; i1++) {
const k1 = rStr()
let v1
v1 = rStr()
o1[k1] = v1
}
v0 = o1
}
o0["codexVariables"] = v0
break
}
case 2: {
let v0
{
const o1 = {}
const n1 = rU8()
for (let j1 = 0; j1 < n1; j1++) {
switch (rU8()) {
case 0: {
let v1
v1 = rF64()
o1["contrast"] = v1
break
}
case 1: {
let v1
v1 = rStr()
o1["editorBackground"] = v1
break
}
case 2: {
let v1
v1 = rStr()
o1["panel"] = v1
break
}
case 3: {
let v1
v1 = rStr()
o1["surfaceUnder"] = v1
break
}
default: {
const k1 = rStr()
o1[k1] = rDyn()
}
}
}
v0 = o1
}
o0["computed"] = v0
break
}
case 3: {
let v0
{
const o1 = {}
const n1 = rU8()
for (let j1 = 0; j1 < n1; j1++) {
switch (rU8()) {
case 0: {
let v1
v1 = rStr()
o1["accentBackground"] = v1
break
}
case 1: {
let v1
v1 = rStr()
o1["accentBackgroundActive"] = v1
break
}
case 2: {
let v1
v1 = rStr()
o1["accentBackgroundHover"] = v1
break
}
case 3: {
let v1
v1 = rStr()
o1["border"] = v1
break
}
case 4: {
let v1
v1 = rStr()
o1["borderFocus"] = v1
break
}
case 5: {
let v1
v1 = rStr()
o1["borderHeavy"] = v1
break
}
case 6: {
let v1
v1 = rStr()
o1["borderLight"] = v1
break
}
case 7: {
let v1
v1 = rStr()
o1["buttonPrimaryBackground"] = v1
break
}
case 8: {
let v1
v1 = rStr()
o1["buttonPrimaryBackgroundActive"] = v1
break
}
case 9: {
let v1
v1 = rStr()
o1["buttonPrimaryBackgroundHover"] = v1
break
}
case 10: {
let v1
v1 = rStr()
o1["buttonPrimaryBackgroundInactive"] = v1
break
}
case 11: {
let v1
v1 = rStr()
o1["buttonSecondaryBackground"] = v1
break
}
case 12: {
let v1
v1 = rStr()
o1["buttonSecondaryBackgroundActive"] = v1
break
}
case 13: {
let v1
v1 = rStr()
o1["buttonSecondaryBackgroundHover"] = v1
break
}
case 14: {
let v1
v1 = rStr()
o1["buttonSecondaryBackgroundInactive"] = v1
break
}
case 15: {
let v1
v1 = rStr()
o1["buttonTertiaryBackground"] = v1
break
}
case 16: {
let v1
v1 = rStr()
o1["buttonTertiaryBackgroundActive"] = v1
break
}
case 17: {
let v1
v1 = rStr()
o1["buttonTertiaryBackgroundHover"] = v1
break
}
case 18: {
let v1
v1 = rStr()
o1["controlBackground"] = v1
break
}
case 19: {
let v1
v1 = rStr()
o1["controlBackgroundOpaque"] = v1
break
}
case 20: {
let v1
v1 = rStr()
o1["elevatedPrimary"] = v1
break
}
case 21: {
let v1
v1 = rStr()
o1["elevatedPrimaryOpaque"] = v1
break
}
case 22: {
let v1
v1 = rStr()
o1["elevatedSecondary"] = v1
break
}
case 23: {
let v1
v1 = rStr()
o1["elevatedSecondaryOpaque"] = v1
break
}
case 24: {
let v1
v1 = rStr()
o1["iconAccent"] = v1
break
}
case 25: {
let v1
v1 = rStr()
o1["iconPrimary"] = v1
break
}
case 26: {
let v1
v1 = rStr()
o1["iconSecondary"] = v1
break
}
case 27: {
let v1
v1 = rStr()
o1["iconTertiary"] = v1
break
}
case 28: {
let v1
v1 = rStr()
o1["simpleScrim"] = v1
break
}
case 29: {
let v1
v1 = rStr()
o1["textAccent"] = v1
break
}
case 30: {
let v1
v1 = rStr()
o1["textButtonPrimary"] = v1
break
}
case 31: {
let v1
v1 = rStr()
o1["textButtonSecondary"] = v1
break
}
case 32: {
let v1
v1 = rStr()
o1["textButtonTertiary"] = v1
break
}
case 33: {
let v1
v1 = rStr()
o1["textForeground"] = v1
break
}
case 34: {
let v1
v1 = rStr()
o1["textForegroundSecondary"] = v1
break
}
case 35: {
let v1
v1 = rStr()
o1["textForegroundTertiary"] = v1
break
}
default: {
const k1 = rStr()
o1[k1] = rDyn()
}
}
}
v0 = o1
}
o0["derived"] = v0
break
}
default: {
const k0 = rStr()
o0[k0] = rDyn()
}
}
}
t = o0
}
return t
}
module.exports.buildResolvedThemeTokens = function buildResolvedThemeTokens (a0, a1) {
  if (illFormed(a1)) throw new TypeError('argument 2 of buildResolvedThemeTokens has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `buildThemeCssVariables`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.buildThemeCssVariables
const call = native.buildThemeCssVariablesWire
const encode = (a0, a1, a2) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'string') return false
wStr(a1)
if (!wDyn(a2)) return false
return true
}
const decode = () => {
let t
{
const o0 = {}
const n0 = rU8()
for (let j0 = 0; j0 < n0; j0++) {
switch (rU8()) {
case 0: {
let v0
v0 = rStr()
o0["material"] = v0
break
}
case 1: {
let v0
v0 = rStr()
o0["translucencyScope"] = v0
break
}
case 2: {
let v0
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const o1 = {}
for (let i1 = 0; i1 < n1; i1++) {
const k1 = rStr()
let v1
v1 = rStr()
o1[k1] = v1
}
v0 = o1
}
o0["variables"] = v0
break
}
default: {
const k0 = rStr()
o0[k0] = rDyn()
}
}
}
t = o0
}
return t
}
module.exports.buildThemeCssVariables = function buildThemeCssVariables (a0, a1, a2) {
  if (illFormed(a1)) throw new TypeError('argument 2 of buildThemeCssVariables has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}
const guards = {

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
