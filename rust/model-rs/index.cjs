// Loads the native module built by `rust-anything build`.
const native = require('./model_rs.node')

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
  getModelOptions: [[1], 'json', false],
  getDefaultModel: [[1], 'json', false],
  hasEffortLevel: [[1, 0], 'raw', false],
  getDefaultEffort: [[1], 'raw', false],
  hasContextWindowOption: [[1, 0], 'raw', false],
  getDefaultContextWindow: [[1], 'raw', false],
  getDevinStaticModelVariants: [[0], 'json', false],
  resolveDevinModelVariant: [[1], 'raw', false],
  hasAutoCompactWindowOption: [[1, 0], 'raw', false],
  getDefaultAutoCompactWindow: [[1], 'raw', false],
  resolveLabeledOptionValue: [[1, 0], 'raw', false],
  getProviderOptionBooleanSelectionValue: [[1, 0], 'raw', false],
  getModelSelectionStringOptionValue: [[1, 0], 'raw', false],
  getModelSelectionBooleanOptionValue: [[1, 0], 'raw', false],
  getProviderOptionDescriptors: [[1], 'json', false],
  getProviderOptionCurrentValue: [[1], 'json', false],
  getModelCapabilities: [[1, 0], 'json', false],
  resolveNewestKnownClaudeFamilyModel: [[0], 'json', false],
  normalizeModelSlug: [[0, 1], 'json', false],
  resolveSelectableModel: [[1, 0, 1], 'json', false],
  resolveModelSlug: [[0, 1], 'json', false],
  resolveModelSlugForProvider: [[1, 0], 'json', false],
  normalizeClaudeModelOptions: [[0, 1, 1], 'json', false],
  resolveApiModelId: [[1], 'raw', false],
  getEffectiveClaudeCodeEffort: [[1], 'json', false],
  claudeSelectionRequiresRestart: [[1, 1], 'raw', false],
  normalizeCursorModelOptions: [[0, 1, 1], 'json', false],
  normalizeGrokModelOptions: [[0, 1], 'json', false],
  normalizeAntigravityModelOptions: [[0, 1, 1], 'json', false],
  normalizePiModelOptions: [[1], 'json', false],
  normalizeOmpModelOptions: [[1], 'json', false],
  normalizeOpenCodeModelOptions: [[1], 'json', false],
  applyClaudePromptEffortPrefix: [[0, 1], 'raw', false],
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

// `getModelOptions`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.getModelOptions
const call = native.getModelOptionsWire
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
module.exports.getModelOptions = function getModelOptions (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `getDefaultModel`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.getDefaultModel
const call = native.getDefaultModelWire
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
module.exports.getDefaultModel = function getDefaultModel (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `formatModelDisplayName`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.formatModelDisplayName
const call = native.formatModelDisplayNameWire
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
const g0 = rU8()
if (g0 === 0) t = undefined
else if (g0 === 2) t = null
else {
t = rStr()
}
}
return t
}
module.exports.formatModelDisplayName = function formatModelDisplayName (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of formatModelDisplayName has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `parseCursorCliReasoningEffort`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.parseCursorCliReasoningEffort
const call = native.parseCursorCliReasoningEffortWire
const encode = (a0) => {
at = 0
if (typeof a0 !== 'string') return false
wStr(a0)
return true
}
const decode = () => {
let t
{
const g0 = rU8()
if (g0 === 0) t = undefined
else if (g0 === 2) t = null
else {
t = rStr()
}
}
return t
}
module.exports.parseCursorCliReasoningEffort = function parseCursorCliReasoningEffort (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of parseCursorCliReasoningEffort has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `hasEffortLevel`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.hasEffortLevel
const call = native.hasEffortLevelWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'string') return false
wStr(a1)
return true
}
const decode = () => {
let t
t = rU8() === 1
return t
}
module.exports.hasEffortLevel = function hasEffortLevel (a0, a1) {
  if (illFormed(a1)) throw new TypeError('argument 2 of hasEffortLevel has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `getDefaultEffort`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.getDefaultEffort
const call = native.getDefaultEffortWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
{
const g0 = rU8()
if (g0 === 0) t = undefined
else if (g0 === 2) t = null
else {
t = rStr()
}
}
return t
}
module.exports.getDefaultEffort = function getDefaultEffort (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `hasContextWindowOption`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.hasContextWindowOption
const call = native.hasContextWindowOptionWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'string') return false
wStr(a1)
return true
}
const decode = () => {
let t
t = rU8() === 1
return t
}
module.exports.hasContextWindowOption = function hasContextWindowOption (a0, a1) {
  if (illFormed(a1)) throw new TypeError('argument 2 of hasContextWindowOption has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `getDefaultContextWindow`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.getDefaultContextWindow
const call = native.getDefaultContextWindowWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
{
const g0 = rU8()
if (g0 === 0) t = undefined
else if (g0 === 2) t = null
else {
t = rStr()
}
}
return t
}
module.exports.getDefaultContextWindow = function getDefaultContextWindow (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `getDevinStaticModelVariants`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.getDevinStaticModelVariants
const call = native.getDevinStaticModelVariantsWire
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
t = rDyn()
return t
}
module.exports.getDevinStaticModelVariants = function getDevinStaticModelVariants (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of getDevinStaticModelVariants has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `resolveDevinModelVariant`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.resolveDevinModelVariant
const call = native.resolveDevinModelVariantWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
{
const g0 = rU8()
if (g0 === 0) t = undefined
else if (g0 === 2) t = null
else {
t = rStr()
}
}
return t
}
module.exports.resolveDevinModelVariant = function resolveDevinModelVariant (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `hasAutoCompactWindowOption`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.hasAutoCompactWindowOption
const call = native.hasAutoCompactWindowOptionWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'string') return false
wStr(a1)
return true
}
const decode = () => {
let t
t = rU8() === 1
return t
}
module.exports.hasAutoCompactWindowOption = function hasAutoCompactWindowOption (a0, a1) {
  if (illFormed(a1)) throw new TypeError('argument 2 of hasAutoCompactWindowOption has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `getClaudeContextWindowSuffix`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.getClaudeContextWindowSuffix
const call = native.getClaudeContextWindowSuffixWire
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
const g0 = rU8()
if (g0 === 0) t = undefined
else if (g0 === 2) t = null
else {
t = rStr()
}
}
return t
}
module.exports.getClaudeContextWindowSuffix = function getClaudeContextWindowSuffix (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of getClaudeContextWindowSuffix has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `stripClaudeContextWindowSuffix`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.stripClaudeContextWindowSuffix
const call = native.stripClaudeContextWindowSuffixWire
const encode = (a0) => {
at = 0
if (typeof a0 !== 'string') return false
wStr(a0)
return true
}
const decode = () => {
let t
t = rStr()
return t
}
module.exports.stripClaudeContextWindowSuffix = function stripClaudeContextWindowSuffix (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of stripClaudeContextWindowSuffix has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `getDefaultAutoCompactWindow`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.getDefaultAutoCompactWindow
const call = native.getDefaultAutoCompactWindowWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
{
const g0 = rU8()
if (g0 === 0) t = undefined
else if (g0 === 2) t = null
else {
t = rStr()
}
}
return t
}
module.exports.getDefaultAutoCompactWindow = function getDefaultAutoCompactWindow (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `resolveLabeledOptionValue`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.resolveLabeledOptionValue
const call = native.resolveLabeledOptionValueWire
const encode = (a0, a1) => {
at = 0
if (a0 === undefined) {
wU8(0)
} else if (a0 === null) {
return false
wU8(2)
} else {
wU8(1)
if (!Array.isArray(a0)) return false
wU8(1)
wU32(a0.length)
for (let i0 = 0; i0 < a0.length; i0++) {
const v0 = a0[i0]
if (v0 === null || typeof v0 !== 'object' || Array.isArray(v0)) return false
{
const at1 = at
wU8(0)
let n1 = 0, r1 = 0
for (const k1 in v0) {
switch (k1) {
case "value": {
wU8(0)
r1++
const f1 = v0[k1]
if (typeof f1 !== 'string') return false
wStr(f1)
break
}
case "isDefault": {
wU8(1)
const f1 = v0[k1]
if (f1 === undefined) {
wU8(0)
} else if (f1 === null) {
return false
wU8(2)
} else {
wU8(1)
if (typeof f1 !== 'boolean') return false
wU8(f1 ? 1 : 0)
}
break
}
default: return false
}
if (++n1 > 255) return false
}
if (r1 !== 1) return false
iv.setUint8(at1, n1)
}
}
}
if (a1 === undefined) {
wU8(0)
} else if (a1 === null) {
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
const g0 = rU8()
if (g0 === 0) t = undefined
else if (g0 === 2) t = null
else {
t = rStr()
}
}
return t
}
module.exports.resolveLabeledOptionValue = function resolveLabeledOptionValue (a0, a1) {
  if (illFormed(a1)) throw new TypeError('argument 2 of resolveLabeledOptionValue has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `getProviderOptionBooleanSelectionValue`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.getProviderOptionBooleanSelectionValue
const call = native.getProviderOptionBooleanSelectionValueWire
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
const g0 = rU8()
if (g0 === 0) t = undefined
else if (g0 === 2) t = null
else {
t = rU8() === 1
}
}
return t
}
module.exports.getProviderOptionBooleanSelectionValue = function getProviderOptionBooleanSelectionValue (a0, a1) {
  if (illFormed(a1)) throw new TypeError('argument 2 of getProviderOptionBooleanSelectionValue has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `getModelSelectionStringOptionValue`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.getModelSelectionStringOptionValue
const call = native.getModelSelectionStringOptionValueWire
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
const g0 = rU8()
if (g0 === 0) t = undefined
else if (g0 === 2) t = null
else {
t = rStr()
}
}
return t
}
module.exports.getModelSelectionStringOptionValue = function getModelSelectionStringOptionValue (a0, a1) {
  if (illFormed(a1)) throw new TypeError('argument 2 of getModelSelectionStringOptionValue has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `getModelSelectionBooleanOptionValue`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.getModelSelectionBooleanOptionValue
const call = native.getModelSelectionBooleanOptionValueWire
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
const g0 = rU8()
if (g0 === 0) t = undefined
else if (g0 === 2) t = null
else {
t = rU8() === 1
}
}
return t
}
module.exports.getModelSelectionBooleanOptionValue = function getModelSelectionBooleanOptionValue (a0, a1) {
  if (illFormed(a1)) throw new TypeError('argument 2 of getModelSelectionBooleanOptionValue has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `getProviderOptionDescriptors`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.getProviderOptionDescriptors
const call = native.getProviderOptionDescriptorsWire
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
module.exports.getProviderOptionDescriptors = function getProviderOptionDescriptors (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `getProviderOptionCurrentValue`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.getProviderOptionCurrentValue
const call = native.getProviderOptionCurrentValueWire
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
module.exports.getProviderOptionCurrentValue = function getProviderOptionCurrentValue (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `getModelCapabilities`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.getModelCapabilities
const call = native.getModelCapabilitiesWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (a1 === undefined) {
wU8(0)
} else if (a1 === null) {
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
t = rDyn()
return t
}
module.exports.getModelCapabilities = function getModelCapabilities (a0, a1) {
  if (illFormed(a1)) throw new TypeError('argument 2 of getModelCapabilities has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `resolveNewestKnownClaudeFamilyModel`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.resolveNewestKnownClaudeFamilyModel
const call = native.resolveNewestKnownClaudeFamilyModelWire
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
t = rDyn()
return t
}
module.exports.resolveNewestKnownClaudeFamilyModel = function resolveNewestKnownClaudeFamilyModel (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of resolveNewestKnownClaudeFamilyModel has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `resolveGrokEffortFamily`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.resolveGrokEffortFamily
const call = native.resolveGrokEffortFamilyWire
const encode = (a0) => {
at = 0
if (typeof a0 !== 'string') return false
wStr(a0)
return true
}
const decode = () => {
let t
t = rStr()
return t
}
module.exports.resolveGrokEffortFamily = function resolveGrokEffortFamily (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of resolveGrokEffortFamily has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `isClaudeUltrathinkPrompt`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.isClaudeUltrathinkPrompt
const call = native.isClaudeUltrathinkPromptWire
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
t = rU8() === 1
return t
}
module.exports.isClaudeUltrathinkPrompt = function isClaudeUltrathinkPrompt (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of isClaudeUltrathinkPrompt has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `normalizeModelSlug`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.normalizeModelSlug
const call = native.normalizeModelSlugWire
const encode = (a0, a1) => {
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
if (!wDyn(a1)) return false
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.normalizeModelSlug = function normalizeModelSlug (a0, a1) {
  if (illFormed(a0)) throw new TypeError('argument 1 of normalizeModelSlug has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `resolveSelectableModel`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.resolveSelectableModel
const call = native.resolveSelectableModelWire
const encode = (a0, a1, a2) => {
at = 0
if (!wDyn(a0)) return false
if (a1 === undefined) {
wU8(0)
} else if (a1 === null) {
wU8(2)
} else {
wU8(1)
if (typeof a1 !== 'string') return false
wStr(a1)
}
if (!Array.isArray(a2)) return false
wU8(1)
wU32(a2.length)
for (let i0 = 0; i0 < a2.length; i0++) {
const v0 = a2[i0]
if (v0 === null || typeof v0 !== 'object' || Array.isArray(v0)) return false
{
const at1 = at
wU8(0)
let n1 = 0, r1 = 0
for (const k1 in v0) {
switch (k1) {
case "slug": {
wU8(0)
r1++
const f1 = v0[k1]
if (typeof f1 !== 'string') return false
wStr(f1)
break
}
case "name": {
wU8(1)
r1++
const f1 = v0[k1]
if (typeof f1 !== 'string') return false
wStr(f1)
break
}
default: return false
}
if (++n1 > 255) return false
}
if (r1 !== 2) return false
iv.setUint8(at1, n1)
}
}
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.resolveSelectableModel = function resolveSelectableModel (a0, a1, a2) {
  if (illFormed(a1)) throw new TypeError('argument 2 of resolveSelectableModel has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `resolveModelSlug`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.resolveModelSlug
const call = native.resolveModelSlugWire
const encode = (a0, a1) => {
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
if (!wDyn(a1)) return false
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.resolveModelSlug = function resolveModelSlug (a0, a1) {
  if (illFormed(a0)) throw new TypeError('argument 1 of resolveModelSlug has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `resolveModelSlugForProvider`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.resolveModelSlugForProvider
const call = native.resolveModelSlugForProviderWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (a1 === undefined) {
wU8(0)
} else if (a1 === null) {
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
t = rDyn()
return t
}
module.exports.resolveModelSlugForProvider = function resolveModelSlugForProvider (a0, a1) {
  if (illFormed(a1)) throw new TypeError('argument 2 of resolveModelSlugForProvider has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `trimOrNull`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.trimOrNull
const call = native.trimOrNullWire
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
const g0 = rU8()
if (g0 === 0) t = undefined
else if (g0 === 2) t = null
else {
t = rStr()
}
}
return t
}
module.exports.trimOrNull = function trimOrNull (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of trimOrNull has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `normalizeClaudeModelOptions`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.normalizeClaudeModelOptions
const call = native.normalizeClaudeModelOptionsWire
const encode = (a0, a1, a2) => {
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
if (!wDyn(a1)) return false
if (!wDyn(a2)) return false
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.normalizeClaudeModelOptions = function normalizeClaudeModelOptions (a0, a1, a2) {
  if (illFormed(a0)) throw new TypeError('argument 1 of normalizeClaudeModelOptions has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `resolveApiModelId`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.resolveApiModelId
const call = native.resolveApiModelIdWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
{
const g0 = rU8()
if (g0 === 0) t = undefined
else {
t = rStr()
}
}
return t
}
module.exports.resolveApiModelId = function resolveApiModelId (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `getEffectiveClaudeCodeEffort`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.getEffectiveClaudeCodeEffort
const call = native.getEffectiveClaudeCodeEffortWire
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
module.exports.getEffectiveClaudeCodeEffort = function getEffectiveClaudeCodeEffort (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `claudeSelectionRequiresRestart`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.claudeSelectionRequiresRestart
const call = native.claudeSelectionRequiresRestartWire
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
module.exports.claudeSelectionRequiresRestart = function claudeSelectionRequiresRestart (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `normalizeCursorModelOptions`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.normalizeCursorModelOptions
const call = native.normalizeCursorModelOptionsWire
const encode = (a0, a1, a2) => {
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
if (!wDyn(a1)) return false
if (!wDyn(a2)) return false
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.normalizeCursorModelOptions = function normalizeCursorModelOptions (a0, a1, a2) {
  if (illFormed(a0)) throw new TypeError('argument 1 of normalizeCursorModelOptions has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `normalizeGrokModelOptions`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.normalizeGrokModelOptions
const call = native.normalizeGrokModelOptionsWire
const encode = (a0, a1) => {
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
if (!wDyn(a1)) return false
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.normalizeGrokModelOptions = function normalizeGrokModelOptions (a0, a1) {
  if (illFormed(a0)) throw new TypeError('argument 1 of normalizeGrokModelOptions has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `normalizeAntigravityModelOptions`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.normalizeAntigravityModelOptions
const call = native.normalizeAntigravityModelOptionsWire
const encode = (a0, a1, a2) => {
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
if (!wDyn(a1)) return false
if (!wDyn(a2)) return false
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.normalizeAntigravityModelOptions = function normalizeAntigravityModelOptions (a0, a1, a2) {
  if (illFormed(a0)) throw new TypeError('argument 1 of normalizeAntigravityModelOptions has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `normalizePiModelOptions`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.normalizePiModelOptions
const call = native.normalizePiModelOptionsWire
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
module.exports.normalizePiModelOptions = function normalizePiModelOptions (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `normalizeOmpModelOptions`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.normalizeOmpModelOptions
const call = native.normalizeOmpModelOptionsWire
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
module.exports.normalizeOmpModelOptions = function normalizeOmpModelOptions (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `normalizeOpenCodeModelOptions`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.normalizeOpenCodeModelOptions
const call = native.normalizeOpenCodeModelOptionsWire
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
module.exports.normalizeOpenCodeModelOptions = function normalizeOpenCodeModelOptions (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `applyClaudePromptEffortPrefix`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.applyClaudePromptEffortPrefix
const call = native.applyClaudePromptEffortPrefixWire
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
module.exports.applyClaudePromptEffortPrefix = function applyClaudePromptEffortPrefix (a0, a1) {
  if (illFormed(a0)) throw new TypeError('argument 1 of applyClaudePromptEffortPrefix has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
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
