// Loads the native module built by `rust-anything build`.
const native = require('./computer_rs.node')

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
  intersectComputerRects: [[1, 1], 'json', false],
  computerBackendActionResult: [[0, 0, 1], 'json', false],
  computerActivationMetadata: [[1], 'json', false],
  readComputerAuditFileTail: [[0, 0], 'json', true],
  readComputerAuditHistory: [[1, 1], 'json', true],
  computerAuditGatewayRequestId: [[1], 'json', false],
  summarizeComputerAuditArgs: [[1], 'json', false],
  cuaActionSettleMsOverride: [[], 'json', false],
  cuaPreviewStillMsOverride: [[], 'json', false],
  currentComputerCall: [[], 'json', false],
  createComputerCallContext: [[], 'json', false],
  computerDenylistMatch: [[1], 'json', false],
  registerNativeComputerElement: [[1, 0], 'raw', false],
  computerElementRefIdentity: [[1], 'json', false],
  bindComputerTargetRef: [[1, 1], 'json', false],
  observedComputerTargetNode: [[1], 'json', false],
  decodeResyncRequest: [[1], 'json', false],
  windowsCoveringPoint: [[1, 0, 1], 'json', false],
  topmostWindowAtPoint: [[1, 1], 'json', false],
  rectContainsPoint: [[1, 1], 'raw', false],
  requireWindowBounds: [[1, 0], 'json', false],
  responsibleDesktopBundleId: [[1], 'raw', false],
  computerSetupSignal: [[1], 'json', false],
  computerSetupToolNote: [[1], 'raw', false],
  cuaMaskedActivationOptIn: [[], 'json', false],
  maskedActivationOptedIn: [[1, 1], 'raw', false],
  computerSpaceDesignationForMessages: [[1], 'raw', false],
  latestUserAuthoredMessage: [[1], 'json', false],
  currentComputerTask: [[], 'json', false],
  startComputerTurnTiming: [[0, 0], 'json', false],
  endComputerTurnTiming: [[0, 1], 'json', false],
  beginComputerTurnCall: [[0, 1, 0], 'json', false],
  messageRequestsVisibleUse: [[0, 1], 'raw', false],
  computerForegroundScopeChangedSince: [[1, 1], 'raw', false],
  computerForegroundAuthorizationForMessages: [[1, 1], 'json', false],
  cuaSpaceInventory: [[1], 'json', false],
  cursorRuntimeActivity: [[1], 'raw', false],
  screenshotPointToDesktop: [[1, 0, 0], 'json', false],
  screenshotRectToDesktop: [[1, 1], 'json', false],
  screenshotDeltaToDesktop: [[1, 0, 0], 'json', false],
  decodePngLuma: [[1], 'json', true],
  estimateVerticalTravel: [[1, 1, 1], 'json', false],
  candidateDescriptions: [[1], 'json', false],
  describeTarget: [[1], 'raw', false],
  activationPointForNode: [[1], 'json', false],
  computerTargetCandidates: [[1], 'json', false],
  resolveComputerSemanticTarget: [[1, 1, 0], 'json', false],
  resolveComputerUniqueTextTarget: [[1, 0, 0], 'json', false],
  resolveComputerWindowTarget: [[1, 0], 'json', false],
  resolveComputerPoint: [[1, 1], 'json', false],
  actionableElements: [[1, 1], 'json', false],
  diffActionableElements: [[1, 1], 'json', false],
  describeComputerUiTree: [[1], 'raw', false],
}
const classes = {
  ComputerControlState: [[0], { get: [[0], 'json'], allows: [[0, 0], 'raw'], set: [[0, 0], 'raw'], recordChatIntent: [[0, 0, 0], 'raw'] }],
  ScreenshotFrameRegistry: [[], { record: [[0, 1, 1], 'json'], latest: [[0], 'json'], matchLatest: [[0, 1, 1], 'json'], resolve: [[0, 1], 'json'] }],
  ScrollGearingStore: [[], { gearing: [[1], 'raw'], has: [[1], 'raw'], plan: [[1, 0, 0], 'raw'], learn: [[1, 0, 0], 'raw'] }],
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

// `assertComputerClipboardWriteFits`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.assertComputerClipboardWriteFits
const call = native.assertComputerClipboardWriteFitsWire
const encode = (a0) => {
at = 0
if (typeof a0 !== 'string') return false
wStr(a0)
return true
}
const decode = () => {
let t
return t
}
module.exports.assertComputerClipboardWriteFits = function assertComputerClipboardWriteFits (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of assertComputerClipboardWriteFits has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `intersectComputerRects`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.intersectComputerRects
const call = native.intersectComputerRectsWire
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
module.exports.intersectComputerRects = function intersectComputerRects (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `clampComputerMessage`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.clampComputerMessage
const call = native.clampComputerMessageWire
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
t = rStr()
return t
}
module.exports.clampComputerMessage = function clampComputerMessage (a0, a1) {
  if (illFormed(a0)) throw new TypeError('argument 1 of clampComputerMessage has a lone surrogate, which a Rust String cannot hold')
  if (illFormed(a1)) throw new TypeError('argument 2 of clampComputerMessage has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `computerBackendActionResult`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.computerBackendActionResult
const call = native.computerBackendActionResultWire
const encode = (a0, a1, a2) => {
at = 0
if (typeof a0 !== 'string') return false
wStr(a0)
if (typeof a1 !== 'string') return false
wStr(a1)
if (!wDyn(a2)) return false
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.computerBackendActionResult = function computerBackendActionResult (a0, a1, a2) {
  if (illFormed(a0)) throw new TypeError('argument 1 of computerBackendActionResult has a lone surrogate, which a Rust String cannot hold')
  if (illFormed(a1)) throw new TypeError('argument 2 of computerBackendActionResult has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `assertDesktopOperationAdmission`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.assertDesktopOperationAdmission
const call = native.assertDesktopOperationAdmissionWire
const encode = () => {
at = 0
return true
}
const decode = () => {
let t
return t
}
module.exports.assertDesktopOperationAdmission = function assertDesktopOperationAdmission () {
  try {
    if (arguments.length <= 0 && encode() && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `computerActivationMetadata`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.computerActivationMetadata
const call = native.computerActivationMetadataWire
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
module.exports.computerActivationMetadata = function computerActivationMetadata (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `computerAuditGatewayRequestId`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.computerAuditGatewayRequestId
const call = native.computerAuditGatewayRequestIdWire
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
o0["gatewayRequestId"] = v0
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
module.exports.computerAuditGatewayRequestId = function computerAuditGatewayRequestId (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `summarizeComputerAuditArgs`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.summarizeComputerAuditArgs
const call = native.summarizeComputerAuditArgsWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
if (rU8() === 0) t = null
else {
const n0 = rU32()
const o0 = {}
for (let i0 = 0; i0 < n0; i0++) {
const k0 = rStr()
let v0
v0 = rDyn()
o0[k0] = v0
}
t = o0
}
return t
}
module.exports.summarizeComputerAuditArgs = function summarizeComputerAuditArgs (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `cuaTimingLogEnabled`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.cuaTimingLogEnabled
const call = native.cuaTimingLogEnabledWire
const encode = () => {
at = 0
return true
}
const decode = () => {
let t
t = rU8() === 1
return t
}
module.exports.cuaTimingLogEnabled = function cuaTimingLogEnabled () {
  try {
    if (arguments.length <= 0 && encode() && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `cuaConditionalSettleEnabled`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.cuaConditionalSettleEnabled
const call = native.cuaConditionalSettleEnabledWire
const encode = () => {
at = 0
return true
}
const decode = () => {
let t
t = rU8() === 1
return t
}
module.exports.cuaConditionalSettleEnabled = function cuaConditionalSettleEnabled () {
  try {
    if (arguments.length <= 0 && encode() && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `cuaActionSettleMsOverride`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.cuaActionSettleMsOverride
const call = native.cuaActionSettleMsOverrideWire
const encode = () => {
at = 0
return true
}
const decode = () => {
let t
{
const g0 = rU8()
if (g0 === 0) t = undefined
else if (g0 === 2) t = null
else {
t = rF64()
}
}
return t
}
module.exports.cuaActionSettleMsOverride = function cuaActionSettleMsOverride () {
  try {
    if (arguments.length <= 0 && encode() && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `cuaCaptureReuseEnabled`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.cuaCaptureReuseEnabled
const call = native.cuaCaptureReuseEnabledWire
const encode = () => {
at = 0
return true
}
const decode = () => {
let t
t = rU8() === 1
return t
}
module.exports.cuaCaptureReuseEnabled = function cuaCaptureReuseEnabled () {
  try {
    if (arguments.length <= 0 && encode() && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `cuaPreviewStillMsOverride`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.cuaPreviewStillMsOverride
const call = native.cuaPreviewStillMsOverrideWire
const encode = () => {
at = 0
return true
}
const decode = () => {
let t
{
const g0 = rU8()
if (g0 === 0) t = undefined
else if (g0 === 2) t = null
else {
t = rF64()
}
}
return t
}
module.exports.cuaPreviewStillMsOverride = function cuaPreviewStillMsOverride () {
  try {
    if (arguments.length <= 0 && encode() && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `currentComputerCall`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.currentComputerCall
const call = native.currentComputerCallWire
const encode = () => {
at = 0
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.currentComputerCall = function currentComputerCall () {
  try {
    if (arguments.length <= 0 && encode() && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `createComputerCallContext`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.createComputerCallContext
const call = native.createComputerCallContextWire
const encode = () => {
at = 0
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.createComputerCallContext = function createComputerCallContext () {
  try {
    if (arguments.length <= 0 && encode() && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `markComputerCall`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.markComputerCall
const call = native.markComputerCallWire
const encode = (a0) => {
at = 0
if (typeof a0 !== 'string') return false
wStr(a0)
return true
}
const decode = () => {
let t
return t
}
module.exports.markComputerCall = function markComputerCall (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of markComputerCall has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `computerDenylistMatch`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.computerDenylistMatch
const call = native.computerDenylistMatchWire
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
{
const o1 = {}
const n1 = rU8()
for (let j1 = 0; j1 < n1; j1++) {
switch (rU8()) {
case 0: {
let v1
v1 = rStr()
o1["app"] = v1
break
}
case 1: {
let v1
v1 = rStr()
o1["matched"] = v1
break
}
default: {
const k1 = rStr()
o1[k1] = rDyn()
}
}
}
t = o1
}
}
}
return t
}
module.exports.computerDenylistMatch = function computerDenylistMatch (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `registerNativeComputerElement`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.registerNativeComputerElement
const call = native.registerNativeComputerElementWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'string') return false
wStr(a1)
return true
}
const decode = () => {
let t
return t
}
module.exports.registerNativeComputerElement = function registerNativeComputerElement (a0, a1) {
  if (illFormed(a1)) throw new TypeError('argument 2 of registerNativeComputerElement has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `computerElementRefIdentity`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.computerElementRefIdentity
const call = native.computerElementRefIdentityWire
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
module.exports.computerElementRefIdentity = function computerElementRefIdentity (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `bindComputerTargetRef`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.bindComputerTargetRef
const call = native.bindComputerTargetRefWire
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
module.exports.bindComputerTargetRef = function bindComputerTargetRef (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `observedComputerTargetNode`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.observedComputerTargetNode
const call = native.observedComputerTargetNodeWire
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
{
const o1 = {}
const n1 = rU8()
for (let j1 = 0; j1 < n1; j1++) {
switch (rU8()) {
case 0: {
let v1
v1 = rStr()
o1["role"] = v1
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
o1["label"] = v1
break
}
case 2: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["value"] = v1
break
}
case 3: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["description"] = v1
break
}
case 4: {
let v1
v1 = rDyn()
o1["frame"] = v1
break
}
case 5: {
let v1
v1 = rDyn()
o1["activationPoint"] = v1
break
}
case 6: {
let v1
v1 = rU8() === 1
o1["onScreen"] = v1
break
}
case 7: {
let v1
v1 = rDyn()
o1["windowId"] = v1
break
}
case 8: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
if (rU8() === 0) v1 = null
else {
const n3 = rU32()
const a3 = new Array(n3)
for (let i3 = 0; i3 < n3; i3++) {
let v3
v3 = rF64()
a3[i3] = v3
}
v1 = a3
}
}
}
o1["nodePath"] = v1
break
}
case 9: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["accessibilityRoot"] = v1
break
}
case 10: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rU8() === 1
}
}
o1["editable"] = v1
break
}
case 11: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rU8() === 1
}
}
o1["truncated"] = v1
break
}
case 12: {
let v1
if (rU8() === 0) v1 = null
else {
const n2 = rU32()
const a2 = new Array(n2)
for (let i2 = 0; i2 < n2; i2++) {
let v2
{
const o3 = {}
const n3 = rU8()
for (let j3 = 0; j3 < n3; j3++) {
switch (rU8()) {
case 0: {
let v3
v3 = rStr()
o3["role"] = v3
break
}
case 1: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["label"] = v3
break
}
case 2: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["value"] = v3
break
}
case 3: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["description"] = v3
break
}
case 4: {
let v3
v3 = rDyn()
o3["frame"] = v3
break
}
case 5: {
let v3
v3 = rDyn()
o3["activationPoint"] = v3
break
}
case 6: {
let v3
v3 = rU8() === 1
o3["onScreen"] = v3
break
}
case 7: {
let v3
v3 = rDyn()
o3["windowId"] = v3
break
}
case 8: {
let v3
v3 = rDyn()
o3["nodePath"] = v3
break
}
case 9: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["accessibilityRoot"] = v3
break
}
case 10: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rU8() === 1
}
}
o3["editable"] = v3
break
}
case 11: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rU8() === 1
}
}
o3["truncated"] = v3
break
}
case 12: {
let v3
if (rU8() === 0) v3 = null
else {
const n4 = rU32()
const a4 = new Array(n4)
for (let i4 = 0; i4 < n4; i4++) {
let v4
v4 = rDyn()
a4[i4] = v4
}
v3 = a4
}
o3["children"] = v3
break
}
default: {
const k3 = rStr()
o3[k3] = rDyn()
}
}
}
v2 = o3
}
a2[i2] = v2
}
v1 = a2
}
o1["children"] = v1
break
}
default: {
const k1 = rStr()
o1[k1] = rDyn()
}
}
}
t = o1
}
}
}
return t
}
module.exports.observedComputerTargetNode = function observedComputerTargetNode (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `decodeResyncRequest`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.decodeResyncRequest
const call = native.decodeResyncRequestWire
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
module.exports.decodeResyncRequest = function decodeResyncRequest (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `windowsCoveringPoint`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.windowsCoveringPoint
const call = native.windowsCoveringPointWire
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
t = rDyn()
return t
}
module.exports.windowsCoveringPoint = function windowsCoveringPoint (a0, a1, a2) {
  if (illFormed(a1)) throw new TypeError('argument 2 of windowsCoveringPoint has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `topmostWindowAtPoint`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.topmostWindowAtPoint
const call = native.topmostWindowAtPointWire
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
module.exports.topmostWindowAtPoint = function topmostWindowAtPoint (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `rectContainsPoint`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.rectContainsPoint
const call = native.rectContainsPointWire
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
module.exports.rectContainsPoint = function rectContainsPoint (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `requireWindowBounds`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.requireWindowBounds
const call = native.requireWindowBoundsWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'string') return false
wStr(a1)
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.requireWindowBounds = function requireWindowBounds (a0, a1) {
  if (illFormed(a1)) throw new TypeError('argument 2 of requireWindowBounds has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `responsibleDesktopBundleId`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.responsibleDesktopBundleId
const call = native.responsibleDesktopBundleIdWire
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
module.exports.responsibleDesktopBundleId = function responsibleDesktopBundleId (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `computerSetupSignal`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.computerSetupSignal
const call = native.computerSetupSignalWire
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
{
const o1 = {}
const n1 = rU8()
for (let j1 = 0; j1 < n1; j1++) {
switch (rU8()) {
case 0: {
let v1
if (rU8() === 0) v1 = null
else {
const n2 = rU32()
const a2 = new Array(n2)
for (let i2 = 0; i2 < n2; i2++) {
let v2
v2 = rDyn()
a2[i2] = v2
}
v1 = a2
}
o1["missing"] = v1
break
}
case 1: {
let v1
v1 = rU8() === 1
o1["blocking"] = v1
break
}
case 2: {
let v1
v1 = rDyn()
o1["buildSignature"] = v1
break
}
case 3: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["bundleId"] = v1
break
}
default: {
const k1 = rStr()
o1[k1] = rDyn()
}
}
}
t = o1
}
}
}
return t
}
module.exports.computerSetupSignal = function computerSetupSignal (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `computerSetupToolNote`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.computerSetupToolNote
const call = native.computerSetupToolNoteWire
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
module.exports.computerSetupToolNote = function computerSetupToolNote (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `cuaMaskedActivationEnabled`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.cuaMaskedActivationEnabled
const call = native.cuaMaskedActivationEnabledWire
const encode = () => {
at = 0
return true
}
const decode = () => {
let t
t = rU8() === 1
return t
}
module.exports.cuaMaskedActivationEnabled = function cuaMaskedActivationEnabled () {
  try {
    if (arguments.length <= 0 && encode() && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `cuaMaskedActivationOptIn`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.cuaMaskedActivationOptIn
const call = native.cuaMaskedActivationOptInWire
const encode = () => {
at = 0
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
v0 = rStr()
a0[i0] = v0
}
t = a0
}
return t
}
module.exports.cuaMaskedActivationOptIn = function cuaMaskedActivationOptIn () {
  try {
    if (arguments.length <= 0 && encode() && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `maskedActivationOptedIn`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.maskedActivationOptedIn
const call = native.maskedActivationOptedInWire
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
module.exports.maskedActivationOptedIn = function maskedActivationOptedIn (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `messageDesignatesComputerSpaces`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.messageDesignatesComputerSpaces
const call = native.messageDesignatesComputerSpacesWire
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
v0 = rF64()
a0[i0] = v0
}
t = a0
}
return t
}
module.exports.messageDesignatesComputerSpaces = function messageDesignatesComputerSpaces (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of messageDesignatesComputerSpaces has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `computerSpaceDesignationForMessages`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.computerSpaceDesignationForMessages
const call = native.computerSpaceDesignationForMessagesWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
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
v0 = rF64()
a0[i0] = v0
}
t = a0
}
return t
}
module.exports.computerSpaceDesignationForMessages = function computerSpaceDesignationForMessages (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `latestUserAuthoredMessage`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.latestUserAuthoredMessage
const call = native.latestUserAuthoredMessageWire
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
module.exports.latestUserAuthoredMessage = function latestUserAuthoredMessage (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `currentComputerTask`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.currentComputerTask
const call = native.currentComputerTaskWire
const encode = () => {
at = 0
return true
}
const decode = () => {
let t
{
const g0 = rU8()
if (g0 === 0) t = undefined
else if (g0 === 2) t = null
else {
{
const o1 = {}
const n1 = rU8()
for (let j1 = 0; j1 < n1; j1++) {
switch (rU8()) {
case 0: {
let v1
v1 = rStr()
o1["threadId"] = v1
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
o1["turnId"] = v1
break
}
case 2: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["label"] = v1
break
}
default: {
const k1 = rStr()
o1[k1] = rDyn()
}
}
}
t = o1
}
}
}
return t
}
module.exports.currentComputerTask = function currentComputerTask () {
  try {
    if (arguments.length <= 0 && encode() && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `isModelDesktopObservationActive`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.isModelDesktopObservationActive
const call = native.isModelDesktopObservationActiveWire
const encode = () => {
at = 0
return true
}
const decode = () => {
let t
t = rU8() === 1
return t
}
module.exports.isModelDesktopObservationActive = function isModelDesktopObservationActive () {
  try {
    if (arguments.length <= 0 && encode() && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `startComputerTurnTiming`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.startComputerTurnTiming
const call = native.startComputerTurnTimingWire
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
t = rDyn()
return t
}
module.exports.startComputerTurnTiming = function startComputerTurnTiming (a0, a1) {
  if (illFormed(a0)) throw new TypeError('argument 1 of startComputerTurnTiming has a lone surrogate, which a Rust String cannot hold')
  if (illFormed(a1)) throw new TypeError('argument 2 of startComputerTurnTiming has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `endComputerTurnTiming`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.endComputerTurnTiming
const call = native.endComputerTurnTimingWire
const encode = (a0, a1) => {
at = 0
if (typeof a0 !== 'string') return false
wStr(a0)
if (!wDyn(a1)) return false
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.endComputerTurnTiming = function endComputerTurnTiming (a0, a1) {
  if (illFormed(a0)) throw new TypeError('argument 1 of endComputerTurnTiming has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `beginComputerTurnCall`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.beginComputerTurnCall
const call = native.beginComputerTurnCallWire
const encode = (a0, a1, a2) => {
at = 0
if (typeof a0 !== 'string') return false
wStr(a0)
if (!wDyn(a1)) return false
if (typeof a2 !== 'string') return false
wStr(a2)
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.beginComputerTurnCall = function beginComputerTurnCall (a0, a1, a2) {
  if (illFormed(a0)) throw new TypeError('argument 1 of beginComputerTurnCall has a lone surrogate, which a Rust String cannot hold')
  if (illFormed(a2)) throw new TypeError('argument 3 of beginComputerTurnCall has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `messageRequestsVisibleUse`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.messageRequestsVisibleUse
const call = native.messageRequestsVisibleUseWire
const encode = (a0, a1) => {
at = 0
if (typeof a0 !== 'string') return false
wStr(a0)
if (!wDyn(a1)) return false
return true
}
const decode = () => {
let t
t = rU8() === 1
return t
}
module.exports.messageRequestsVisibleUse = function messageRequestsVisibleUse (a0, a1) {
  if (illFormed(a0)) throw new TypeError('argument 1 of messageRequestsVisibleUse has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `computerForegroundScopeChangedSince`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.computerForegroundScopeChangedSince
const call = native.computerForegroundScopeChangedSinceWire
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
module.exports.computerForegroundScopeChangedSince = function computerForegroundScopeChangedSince (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `computerForegroundAuthorizationForMessages`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.computerForegroundAuthorizationForMessages
const call = native.computerForegroundAuthorizationForMessagesWire
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
v0 = rU8() === 1
o0["userRequestedVisibleUse"] = v0
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
module.exports.computerForegroundAuthorizationForMessages = function computerForegroundAuthorizationForMessages (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `cuaSpaceInventory`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.cuaSpaceInventory
const call = native.cuaSpaceInventoryWire
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
module.exports.cuaSpaceInventory = function cuaSpaceInventory (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `cursorToolActivity`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.cursorToolActivity
const call = native.cursorToolActivityWire
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
module.exports.cursorToolActivity = function cursorToolActivity (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of cursorToolActivity has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `cursorRuntimeActivity`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.cursorRuntimeActivity
const call = native.cursorRuntimeActivityWire
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
module.exports.cursorRuntimeActivity = function cursorRuntimeActivity (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `screenshotPointToDesktop`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.screenshotPointToDesktop
const call = native.screenshotPointToDesktopWire
const encode = (a0, a1, a2) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'number') return false
wF64(a1)
if (typeof a2 !== 'number') return false
wF64(a2)
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.screenshotPointToDesktop = function screenshotPointToDesktop (a0, a1, a2) {
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `screenshotRectToDesktop`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.screenshotRectToDesktop
const call = native.screenshotRectToDesktopWire
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
module.exports.screenshotRectToDesktop = function screenshotRectToDesktop (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `screenshotDeltaToDesktop`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.screenshotDeltaToDesktop
const call = native.screenshotDeltaToDesktopWire
const encode = (a0, a1, a2) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'number') return false
wF64(a1)
if (typeof a2 !== 'number') return false
wF64(a2)
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
o0["deltaX"] = v0
break
}
case 1: {
let v0
v0 = rF64()
o0["deltaY"] = v0
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
module.exports.screenshotDeltaToDesktop = function screenshotDeltaToDesktop (a0, a1, a2) {
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `estimateVerticalTravel`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.estimateVerticalTravel
const call = native.estimateVerticalTravelWire
const encode = (a0, a1, a2) => {
at = 0
if (!wDyn(a0)) return false
if (!wDyn(a1)) return false
if (!wDyn(a2)) return false
return true
}
const decode = () => {
let t
{
const g0 = rU8()
if (g0 === 0) t = undefined
else if (g0 === 2) t = null
else {
t = rF64()
}
}
return t
}
module.exports.estimateVerticalTravel = function estimateVerticalTravel (a0, a1, a2) {
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `resolveStillIntervalMs`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.resolveStillIntervalMs
const call = native.resolveStillIntervalMsWire
const encode = (a0) => {
at = 0
if (a0 === undefined) {
wU8(0)
} else if (a0 === null) {
return false
wU8(2)
} else {
wU8(1)
if (typeof a0 !== 'number') return false
wF64(a0)
}
return true
}
const decode = () => {
let t
t = rF64()
return t
}
module.exports.resolveStillIntervalMs = function resolveStillIntervalMs (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `normalizeLabelSpaces`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.normalizeLabelSpaces
const call = native.normalizeLabelSpacesWire
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
module.exports.normalizeLabelSpaces = function normalizeLabelSpaces (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of normalizeLabelSpaces has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `candidateDescriptions`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.candidateDescriptions
const call = native.candidateDescriptionsWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
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
o1["label"] = v1
break
}
case 1: {
let v1
v1 = rStr()
o1["role"] = v1
break
}
case 2: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["windowId"] = v1
break
}
case 3: {
let v1
v1 = rU8() === 1
o1["onScreen"] = v1
break
}
case 4: {
let v1
v1 = rDyn()
o1["frame"] = v1
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
module.exports.candidateDescriptions = function candidateDescriptions (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `describeTarget`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.describeTarget
const call = native.describeTargetWire
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
module.exports.describeTarget = function describeTarget (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `activationPointForNode`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.activationPointForNode
const call = native.activationPointForNodeWire
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
module.exports.activationPointForNode = function activationPointForNode (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `computerTargetCandidates`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.computerTargetCandidates
const call = native.computerTargetCandidatesWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
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
o1["label"] = v1
break
}
case 1: {
let v1
v1 = rStr()
o1["role"] = v1
break
}
case 2: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["windowId"] = v1
break
}
case 3: {
let v1
v1 = rU8() === 1
o1["onScreen"] = v1
break
}
case 4: {
let v1
v1 = rDyn()
o1["frame"] = v1
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
module.exports.computerTargetCandidates = function computerTargetCandidates (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `resolveComputerSemanticTarget`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.resolveComputerSemanticTarget
const call = native.resolveComputerSemanticTargetWire
const encode = (a0, a1, a2) => {
at = 0
if (!wDyn(a0)) return false
if (!wDyn(a1)) return false
if (a2 === undefined) {
wU8(0)
} else if (a2 === null) {
return false
wU8(2)
} else {
wU8(1)
if (typeof a2 !== 'boolean') return false
wU8(a2 ? 1 : 0)
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
v0 = rDyn()
o0["point"] = v0
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
o1["role"] = v1
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
o1["label"] = v1
break
}
case 2: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["value"] = v1
break
}
case 3: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["description"] = v1
break
}
case 4: {
let v1
v1 = rDyn()
o1["frame"] = v1
break
}
case 5: {
let v1
v1 = rDyn()
o1["activationPoint"] = v1
break
}
case 6: {
let v1
v1 = rU8() === 1
o1["onScreen"] = v1
break
}
case 7: {
let v1
v1 = rDyn()
o1["windowId"] = v1
break
}
case 8: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
if (rU8() === 0) v1 = null
else {
const n3 = rU32()
const a3 = new Array(n3)
for (let i3 = 0; i3 < n3; i3++) {
let v3
v3 = rF64()
a3[i3] = v3
}
v1 = a3
}
}
}
o1["nodePath"] = v1
break
}
case 9: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["accessibilityRoot"] = v1
break
}
case 10: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rU8() === 1
}
}
o1["editable"] = v1
break
}
case 11: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rU8() === 1
}
}
o1["truncated"] = v1
break
}
case 12: {
let v1
if (rU8() === 0) v1 = null
else {
const n2 = rU32()
const a2 = new Array(n2)
for (let i2 = 0; i2 < n2; i2++) {
let v2
{
const o3 = {}
const n3 = rU8()
for (let j3 = 0; j3 < n3; j3++) {
switch (rU8()) {
case 0: {
let v3
v3 = rDyn()
o3["role"] = v3
break
}
case 1: {
let v3
v3 = rDyn()
o3["label"] = v3
break
}
case 2: {
let v3
v3 = rDyn()
o3["value"] = v3
break
}
case 3: {
let v3
v3 = rDyn()
o3["description"] = v3
break
}
case 4: {
let v3
v3 = rDyn()
o3["frame"] = v3
break
}
case 5: {
let v3
v3 = rDyn()
o3["activationPoint"] = v3
break
}
case 6: {
let v3
v3 = rDyn()
o3["onScreen"] = v3
break
}
case 7: {
let v3
v3 = rDyn()
o3["windowId"] = v3
break
}
case 8: {
let v3
v3 = rDyn()
o3["nodePath"] = v3
break
}
case 9: {
let v3
v3 = rDyn()
o3["accessibilityRoot"] = v3
break
}
case 10: {
let v3
v3 = rDyn()
o3["editable"] = v3
break
}
case 11: {
let v3
v3 = rDyn()
o3["truncated"] = v3
break
}
case 12: {
let v3
v3 = rDyn()
o3["children"] = v3
break
}
default: {
const k3 = rStr()
o3[k3] = rDyn()
}
}
}
v2 = o3
}
a2[i2] = v2
}
v1 = a2
}
o1["children"] = v1
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
o0["node"] = v0
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
module.exports.resolveComputerSemanticTarget = function resolveComputerSemanticTarget (a0, a1, a2) {
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `resolveComputerUniqueTextTarget`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.resolveComputerUniqueTextTarget
const call = native.resolveComputerUniqueTextTargetWire
const encode = (a0, a1, a2) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'string') return false
wStr(a1)
if (a2 === undefined) {
wU8(0)
} else if (a2 === null) {
return false
wU8(2)
} else {
wU8(1)
if (typeof a2 !== 'boolean') return false
wU8(a2 ? 1 : 0)
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
v0 = rDyn()
o0["point"] = v0
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
o1["role"] = v1
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
o1["label"] = v1
break
}
case 2: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["value"] = v1
break
}
case 3: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["description"] = v1
break
}
case 4: {
let v1
v1 = rDyn()
o1["frame"] = v1
break
}
case 5: {
let v1
v1 = rDyn()
o1["activationPoint"] = v1
break
}
case 6: {
let v1
v1 = rU8() === 1
o1["onScreen"] = v1
break
}
case 7: {
let v1
v1 = rDyn()
o1["windowId"] = v1
break
}
case 8: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
if (rU8() === 0) v1 = null
else {
const n3 = rU32()
const a3 = new Array(n3)
for (let i3 = 0; i3 < n3; i3++) {
let v3
v3 = rF64()
a3[i3] = v3
}
v1 = a3
}
}
}
o1["nodePath"] = v1
break
}
case 9: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["accessibilityRoot"] = v1
break
}
case 10: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rU8() === 1
}
}
o1["editable"] = v1
break
}
case 11: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rU8() === 1
}
}
o1["truncated"] = v1
break
}
case 12: {
let v1
if (rU8() === 0) v1 = null
else {
const n2 = rU32()
const a2 = new Array(n2)
for (let i2 = 0; i2 < n2; i2++) {
let v2
{
const o3 = {}
const n3 = rU8()
for (let j3 = 0; j3 < n3; j3++) {
switch (rU8()) {
case 0: {
let v3
v3 = rDyn()
o3["role"] = v3
break
}
case 1: {
let v3
v3 = rDyn()
o3["label"] = v3
break
}
case 2: {
let v3
v3 = rDyn()
o3["value"] = v3
break
}
case 3: {
let v3
v3 = rDyn()
o3["description"] = v3
break
}
case 4: {
let v3
v3 = rDyn()
o3["frame"] = v3
break
}
case 5: {
let v3
v3 = rDyn()
o3["activationPoint"] = v3
break
}
case 6: {
let v3
v3 = rDyn()
o3["onScreen"] = v3
break
}
case 7: {
let v3
v3 = rDyn()
o3["windowId"] = v3
break
}
case 8: {
let v3
v3 = rDyn()
o3["nodePath"] = v3
break
}
case 9: {
let v3
v3 = rDyn()
o3["accessibilityRoot"] = v3
break
}
case 10: {
let v3
v3 = rDyn()
o3["editable"] = v3
break
}
case 11: {
let v3
v3 = rDyn()
o3["truncated"] = v3
break
}
case 12: {
let v3
v3 = rDyn()
o3["children"] = v3
break
}
default: {
const k3 = rStr()
o3[k3] = rDyn()
}
}
}
v2 = o3
}
a2[i2] = v2
}
v1 = a2
}
o1["children"] = v1
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
o0["node"] = v0
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
module.exports.resolveComputerUniqueTextTarget = function resolveComputerUniqueTextTarget (a0, a1, a2) {
  if (illFormed(a1)) throw new TypeError('argument 2 of resolveComputerUniqueTextTarget has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `resolveComputerWindowTarget`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.resolveComputerWindowTarget
const call = native.resolveComputerWindowTargetWire
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
{
const o1 = {}
const n1 = rU8()
for (let j1 = 0; j1 < n1; j1++) {
switch (rU8()) {
case 0: {
let v1
v1 = rDyn()
o1["point"] = v1
break
}
case 1: {
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
v2 = rStr()
o2["role"] = v2
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
o2["label"] = v2
break
}
case 2: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["value"] = v2
break
}
case 3: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["description"] = v2
break
}
case 4: {
let v2
v2 = rDyn()
o2["frame"] = v2
break
}
case 5: {
let v2
v2 = rDyn()
o2["activationPoint"] = v2
break
}
case 6: {
let v2
v2 = rU8() === 1
o2["onScreen"] = v2
break
}
case 7: {
let v2
v2 = rDyn()
o2["windowId"] = v2
break
}
case 8: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
if (rU8() === 0) v2 = null
else {
const n4 = rU32()
const a4 = new Array(n4)
for (let i4 = 0; i4 < n4; i4++) {
let v4
v4 = rDyn()
a4[i4] = v4
}
v2 = a4
}
}
}
o2["nodePath"] = v2
break
}
case 9: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["accessibilityRoot"] = v2
break
}
case 10: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rU8() === 1
}
}
o2["editable"] = v2
break
}
case 11: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rU8() === 1
}
}
o2["truncated"] = v2
break
}
case 12: {
let v2
if (rU8() === 0) v2 = null
else {
const n3 = rU32()
const a3 = new Array(n3)
for (let i3 = 0; i3 < n3; i3++) {
let v3
v3 = rDyn()
a3[i3] = v3
}
v2 = a3
}
o2["children"] = v2
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
o1["node"] = v1
break
}
default: {
const k1 = rStr()
o1[k1] = rDyn()
}
}
}
t = o1
}
}
}
return t
}
module.exports.resolveComputerWindowTarget = function resolveComputerWindowTarget (a0, a1) {
  if (illFormed(a1)) throw new TypeError('argument 2 of resolveComputerWindowTarget has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `resolveComputerPoint`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.resolveComputerPoint
const call = native.resolveComputerPointWire
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
module.exports.resolveComputerPoint = function resolveComputerPoint (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `actionableElements`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.actionableElements
const call = native.actionableElementsWire
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
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const a1 = new Array(n1)
for (let i1 = 0; i1 < n1; i1++) {
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
v2 = rF64()
o2["ref"] = v2
break
}
case 1: {
let v2
v2 = rStr()
o2["role"] = v2
break
}
case 2: {
let v2
v2 = rStr()
o2["label"] = v2
break
}
case 3: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["value"] = v2
break
}
case 4: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["windowId"] = v2
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
a1[i1] = v1
}
v0 = a1
}
o0["items"] = v0
break
}
case 1: {
let v0
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const a1 = new Array(n1)
for (let i1 = 0; i1 < n1; i1++) {
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
v2 = rStr()
o2["label"] = v2
break
}
case 1: {
let v2
v2 = rStr()
o2["role"] = v2
break
}
case 2: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["windowId"] = v2
break
}
case 3: {
let v2
v2 = rF64()
o2["ordinal"] = v2
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
a1[i1] = v1
}
v0 = a1
}
o0["refIndex"] = v0
break
}
case 2: {
let v0
v0 = rU8() === 1
o0["complete"] = v0
break
}
case 3: {
let v0
v0 = rU8() === 1
o0["sourceIncomplete"] = v0
break
}
case 4: {
let v0
v0 = rF64()
o0["omitted"] = v0
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
module.exports.actionableElements = function actionableElements (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `diffActionableElements`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.diffActionableElements
const call = native.diffActionableElementsWire
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
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const a1 = new Array(n1)
for (let i1 = 0; i1 < n1; i1++) {
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
v2 = rF64()
o2["ref"] = v2
break
}
case 1: {
let v2
v2 = rStr()
o2["role"] = v2
break
}
case 2: {
let v2
v2 = rStr()
o2["label"] = v2
break
}
case 3: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["value"] = v2
break
}
case 4: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["windowId"] = v2
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
a1[i1] = v1
}
v0 = a1
}
o0["added"] = v0
break
}
case 1: {
let v0
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const a1 = new Array(n1)
for (let i1 = 0; i1 < n1; i1++) {
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
v2 = rF64()
o2["ref"] = v2
break
}
case 1: {
let v2
v2 = rStr()
o2["role"] = v2
break
}
case 2: {
let v2
v2 = rStr()
o2["label"] = v2
break
}
case 3: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["value"] = v2
break
}
case 4: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["windowId"] = v2
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
a1[i1] = v1
}
v0 = a1
}
o0["removed"] = v0
break
}
case 2: {
let v0
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const a1 = new Array(n1)
for (let i1 = 0; i1 < n1; i1++) {
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
v2 = rF64()
o2["ref"] = v2
break
}
case 1: {
let v2
v2 = rStr()
o2["role"] = v2
break
}
case 2: {
let v2
v2 = rStr()
o2["label"] = v2
break
}
case 3: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["windowId"] = v2
break
}
case 4: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["was"] = v2
break
}
case 5: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["value"] = v2
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
a1[i1] = v1
}
v0 = a1
}
o0["changed"] = v0
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
module.exports.diffActionableElements = function diffActionableElements (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `describeComputerUiTree`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.describeComputerUiTree
const call = native.describeComputerUiTreeWire
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
module.exports.describeComputerUiTree = function describeComputerUiTree (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `clampNodeText`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.clampNodeText
const call = native.clampNodeTextWire
const encode = (a0, a1) => {
at = 0
if (typeof a0 !== 'string') return false
wStr(a0)
if (typeof a1 !== 'number') return false
wF64(a1)
return true
}
const decode = () => {
let t
t = rStr()
return t
}
module.exports.clampNodeText = function clampNodeText (a0, a1) {
  if (illFormed(a0)) throw new TypeError('argument 1 of clampNodeText has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `clampTextToLength`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.clampTextToLength
const call = native.clampTextToLengthWire
const encode = (a0, a1) => {
at = 0
if (typeof a0 !== 'string') return false
wStr(a0)
if (typeof a1 !== 'number') return false
wF64(a1)
return true
}
const decode = () => {
let t
t = rStr()
return t
}
module.exports.clampTextToLength = function clampTextToLength (a0, a1) {
  if (illFormed(a0)) throw new TypeError('argument 1 of clampTextToLength has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}
const guards = {
  readComputerAuditFileTail: [true, [0]],
  readComputerAuditHistory: [true, []],
  decodePngLuma: [true, []],
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
