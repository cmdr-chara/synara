// Loads the native module built by `rust-anything build`.
const native = require('./computer_use_fixtures_rs.node')

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
  canaryOpenArgs: [[1, 1], 'json', false],
  parseBrowserBenchmarkRunCount: [[0], 'json', false],
  assessBrowserBenchmarkRuns: [[1], 'json', false],
  parseNeweggPrice: [[0, 0], 'json', false],
  parseNeweggPageEvidence: [[1], 'json', false],
  observedNeweggCategory: [[1], 'json', false],
  assessNeweggCompletion: [[1], 'json', false],
  parseBrowserWitness: [[1], 'json', false],
  parsePullRequestTitles: [[1], 'json', false],
  assessGitHubCompletion: [[1], 'json', false],
  browserBenchmarkPrompt: [[0, 0, 0], 'json', false],
  parseObserverDescriptor: [[1, 0], 'json', false],
  readGitHubReference: [[0], 'json', true],
  createBrowserObserver: [[1], 'json', false],
  assertPackagedAppInstallation: [[0, 0], 'json', false],
  artifactIdentity: [[0], 'json', true],
  assertLoopbackUrl: [[0, 0], 'json', false],
  connectPackagedOwner: [[0], 'json', true],
  connectOwnerUrl: [[0], 'json', true],
  assertPassiveComputerReady: [[1], 'raw', false],
  resolveFixtureComputerWindow: [[1, 1], 'json', false],
  assessFixtureReportCoverage: [[1, 0], 'json', false],
  parseFixtureState: [[1], 'json', false],
  verifyFixtureClick: [[1, 1], 'json', false],
  fixtureUnchanged: [[1, 1], 'raw', false],
  fixtureReceiptTiming: [[1], 'json', false],
  assessContinuousFocus: [[1, 1, 1], 'json', false],
  delay: [[0], 'json', false],
  startNativeTarget: [[0], 'json', true],
  parseResourceSnapshot: [[0], 'json', false],
  resourceIdentity: [[1], 'raw', false],
  makeResourceTracker: [[1], 'json', false],
  resourceSamplingOptions: [[1], 'json', false],
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

// `canaryOpenArgs`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.canaryOpenArgs
const call = native.canaryOpenArgsWire
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
module.exports.canaryOpenArgs = function canaryOpenArgs (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `parseBrowserBenchmarkRunCount`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.parseBrowserBenchmarkRunCount
const call = native.parseBrowserBenchmarkRunCountWire
const encode = (a0) => {
at = 0
if (a0 === undefined) {
wU8(0)
} else if (a0 === null) {
return false
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
t = rF64()
return t
}
module.exports.parseBrowserBenchmarkRunCount = function parseBrowserBenchmarkRunCount (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of parseBrowserBenchmarkRunCount has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `assessBrowserBenchmarkRuns`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.assessBrowserBenchmarkRuns
const call = native.assessBrowserBenchmarkRunsWire
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
module.exports.assessBrowserBenchmarkRuns = function assessBrowserBenchmarkRuns (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `parseNeweggPrice`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.parseNeweggPrice
const call = native.parseNeweggPriceWire
const encode = (a0, a1) => {
at = 0
if (typeof a0 !== 'string') return false
wStr(a0)
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
module.exports.parseNeweggPrice = function parseNeweggPrice (a0, a1) {
  if (illFormed(a0)) throw new TypeError('argument 1 of parseNeweggPrice has a lone surrogate, which a Rust String cannot hold')
  if (illFormed(a1)) throw new TypeError('argument 2 of parseNeweggPrice has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `parseNeweggPageEvidence`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.parseNeweggPageEvidence
const call = native.parseNeweggPageEvidenceWire
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
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
{
const o3 = {}
const n3 = rU8()
for (let j3 = 0; j3 < n3; j3++) {
switch (rU8()) {
case 0: {
let v3
v3 = rStr()
o3["itemId"] = v3
break
}
case 1: {
let v3
v3 = rStr()
o3["title"] = v3
break
}
case 2: {
let v3
if (rU8() === 0) v3 = null
else {
const n4 = rU32()
const a4 = new Array(n4)
for (let i4 = 0; i4 < n4; i4++) {
let v4
v4 = rStr()
a4[i4] = v4
}
v3 = a4
}
o3["breadcrumbs"] = v3
break
}
case 3: {
let v3
if (rU8() === 0) v3 = null
else {
const n4 = rU32()
const a4 = new Array(n4)
for (let i4 = 0; i4 < n4; i4++) {
let v4
{
const o5 = {}
const n5 = rU8()
for (let j5 = 0; j5 < n5; j5++) {
switch (rU8()) {
case 0: {
let v5
v5 = rDyn()
o5["label"] = v5
break
}
case 1: {
let v5
v5 = rDyn()
o5["value"] = v5
break
}
default: {
const k5 = rStr()
o5[k5] = rDyn()
}
}
}
v4 = o5
}
a4[i4] = v4
}
v3 = a4
}
o3["facts"] = v3
break
}
default: {
const k3 = rStr()
o3[k3] = rDyn()
}
}
}
v1 = o3
}
}
}
o1["product"] = v1
break
}
case 1: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
{
const o3 = {}
const n3 = rU8()
for (let j3 = 0; j3 < n3; j3++) {
switch (rU8()) {
case 0: {
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
o3["items"] = v3
break
}
case 1: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["itemCount"] = v3
break
}
case 2: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["subtotalMinor"] = v3
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
o3["currency"] = v3
break
}
case 4: {
let v3
v3 = rU8() === 1
o3["rowCoverageComplete"] = v3
break
}
default: {
const k3 = rStr()
o3[k3] = rDyn()
}
}
}
v1 = o3
}
}
}
o1["cart"] = v1
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
module.exports.parseNeweggPageEvidence = function parseNeweggPageEvidence (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `observedNeweggCategory`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.observedNeweggCategory
const call = native.observedNeweggCategoryWire
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
module.exports.observedNeweggCategory = function observedNeweggCategory (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `assessNeweggCompletion`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.assessNeweggCompletion
const call = native.assessNeweggCompletionWire
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
module.exports.assessNeweggCompletion = function assessNeweggCompletion (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `parseBrowserWitness`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.parseBrowserWitness
const call = native.parseBrowserWitnessWire
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
v1 = rF64()
o1["page"] = v1
break
}
case 1: {
let v1
v1 = rStr()
o1["path"] = v1
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
{
const o3 = {}
const n3 = rU8()
for (let j3 = 0; j3 < n3; j3++) {
switch (rU8()) {
case 0: {
let v3
v3 = rF64()
o3["number"] = v3
break
}
case 1: {
let v3
v3 = rStr()
o3["title"] = v3
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
o1["prs"] = v1
break
}
case 3: {
let v1
v1 = rU8() === 1
o1["cartVisible"] = v1
break
}
case 4: {
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
o1["productPaths"] = v1
break
}
case 5: {
let v1
v1 = rU8() === 1
o1["checkoutVisible"] = v1
break
}
case 6: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
{
const o3 = {}
const n3 = rU8()
for (let j3 = 0; j3 < n3; j3++) {
switch (rU8()) {
case 0: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
{
const o5 = {}
const n5 = rU8()
for (let j5 = 0; j5 < n5; j5++) {
switch (rU8()) {
case 0: {
let v5
v5 = rDyn()
o5["itemId"] = v5
break
}
case 1: {
let v5
v5 = rDyn()
o5["title"] = v5
break
}
case 2: {
let v5
v5 = rDyn()
o5["breadcrumbs"] = v5
break
}
case 3: {
let v5
v5 = rDyn()
o5["facts"] = v5
break
}
default: {
const k5 = rStr()
o5[k5] = rDyn()
}
}
}
v3 = o5
}
}
}
o3["product"] = v3
break
}
case 1: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
{
const o5 = {}
const n5 = rU8()
for (let j5 = 0; j5 < n5; j5++) {
switch (rU8()) {
case 0: {
let v5
v5 = rDyn()
o5["items"] = v5
break
}
case 1: {
let v5
v5 = rDyn()
o5["itemCount"] = v5
break
}
case 2: {
let v5
v5 = rDyn()
o5["subtotalMinor"] = v5
break
}
case 3: {
let v5
v5 = rDyn()
o5["currency"] = v5
break
}
case 4: {
let v5
v5 = rDyn()
o5["rowCoverageComplete"] = v5
break
}
default: {
const k5 = rStr()
o5[k5] = rDyn()
}
}
}
v3 = o5
}
}
}
o3["cart"] = v3
break
}
default: {
const k3 = rStr()
o3[k3] = rDyn()
}
}
}
v1 = o3
}
}
}
o1["newegg"] = v1
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
module.exports.parseBrowserWitness = function parseBrowserWitness (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `parsePullRequestTitles`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.parsePullRequestTitles
const call = native.parsePullRequestTitlesWire
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
if (rU8() === 0) t = null
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
o2["number"] = v2
break
}
case 1: {
let v2
v2 = rStr()
o2["title"] = v2
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
t = a1
}
}
}
return t
}
module.exports.parsePullRequestTitles = function parsePullRequestTitles (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `assessGitHubCompletion`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.assessGitHubCompletion
const call = native.assessGitHubCompletionWire
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
module.exports.assessGitHubCompletion = function assessGitHubCompletion (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `browserBenchmarkPrompt`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.browserBenchmarkPrompt
const call = native.browserBenchmarkPromptWire
const encode = (a0, a1, a2) => {
at = 0
if (typeof a0 !== 'string') return false
wStr(a0)
if (typeof a1 !== 'string') return false
wStr(a1)
if (a2 === undefined) {
wU8(0)
} else if (a2 === null) {
return false
wU8(2)
} else {
wU8(1)
if (typeof a2 !== 'string') return false
wStr(a2)
}
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.browserBenchmarkPrompt = function browserBenchmarkPrompt (a0, a1, a2) {
  if (illFormed(a0)) throw new TypeError('argument 1 of browserBenchmarkPrompt has a lone surrogate, which a Rust String cannot hold')
  if (illFormed(a1)) throw new TypeError('argument 2 of browserBenchmarkPrompt has a lone surrogate, which a Rust String cannot hold')
  if (illFormed(a2)) throw new TypeError('argument 3 of browserBenchmarkPrompt has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `neweggItemIdFromUrl`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.neweggItemIdFromUrl
const call = native.neweggItemIdFromUrlWire
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
module.exports.neweggItemIdFromUrl = function neweggItemIdFromUrl (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of neweggItemIdFromUrl has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `parseObserverDescriptor`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.parseObserverDescriptor
const call = native.parseObserverDescriptorWire
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
o0["profileName"] = v0
break
}
case 1: {
let v0
v0 = rStr()
o0["profileDirectory"] = v0
break
}
case 2: {
let v0
v0 = rF64()
o0["browserPid"] = v0
break
}
case 3: {
let v0
v0 = rStr()
o0["endpoint"] = v0
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
module.exports.parseObserverDescriptor = function parseObserverDescriptor (a0, a1) {
  if (illFormed(a1)) throw new TypeError('argument 2 of parseObserverDescriptor has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `createBrowserObserver`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.createBrowserObserver
const call = native.createBrowserObserverWire
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
module.exports.createBrowserObserver = function createBrowserObserver (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `assertPackagedAppInstallation`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.assertPackagedAppInstallation
const call = native.assertPackagedAppInstallationWire
const encode = (a0, a1) => {
at = 0
if (typeof a0 !== 'string') return false
wStr(a0)
if (a1 === undefined) {
return false
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
module.exports.assertPackagedAppInstallation = function assertPackagedAppInstallation (a0, a1) {
  if (illFormed(a0)) throw new TypeError('argument 1 of assertPackagedAppInstallation has a lone surrogate, which a Rust String cannot hold')
  if (illFormed(a1)) throw new TypeError('argument 2 of assertPackagedAppInstallation has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `assertLoopbackUrl`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.assertLoopbackUrl
const call = native.assertLoopbackUrlWire
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
module.exports.assertLoopbackUrl = function assertLoopbackUrl (a0, a1) {
  if (illFormed(a0)) throw new TypeError('argument 1 of assertLoopbackUrl has a lone surrogate, which a Rust String cannot hold')
  if (illFormed(a1)) throw new TypeError('argument 2 of assertLoopbackUrl has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `assertPassiveComputerReady`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.assertPassiveComputerReady
const call = native.assertPassiveComputerReadyWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
return t
}
module.exports.assertPassiveComputerReady = function assertPassiveComputerReady (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `resolveFixtureComputerWindow`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.resolveFixtureComputerWindow
const call = native.resolveFixtureComputerWindowWire
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
module.exports.resolveFixtureComputerWindow = function resolveFixtureComputerWindow (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `assessFixtureReportCoverage`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.assessFixtureReportCoverage
const call = native.assessFixtureReportCoverageWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'number') return false
wF64(a1)
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.assessFixtureReportCoverage = function assessFixtureReportCoverage (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `parseFixtureState`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.parseFixtureState
const call = native.parseFixtureStateWire
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
v1 = rF64()
o1["pid"] = v1
break
}
case 1: {
let v1
v1 = rF64()
o1["windowId"] = v1
break
}
case 2: {
let v1
v1 = rStr()
o1["title"] = v1
break
}
case 3: {
let v1
v1 = rStr()
o1["label"] = v1
break
}
case 4: {
let v1
v1 = rF64()
o1["clicks"] = v1
break
}
case 5: {
let v1
v1 = rF64()
o1["edits"] = v1
break
}
case 6: {
let v1
v1 = rStr()
o1["text"] = v1
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
module.exports.parseFixtureState = function parseFixtureState (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `verifyFixtureClick`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.verifyFixtureClick
const call = native.verifyFixtureClickWire
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
module.exports.verifyFixtureClick = function verifyFixtureClick (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `fixtureUnchanged`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.fixtureUnchanged
const call = native.fixtureUnchangedWire
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
module.exports.fixtureUnchanged = function fixtureUnchanged (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `fixtureReceiptTiming`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.fixtureReceiptTiming
const call = native.fixtureReceiptTimingWire
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
module.exports.fixtureReceiptTiming = function fixtureReceiptTiming (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `assessContinuousFocus`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.assessContinuousFocus
const call = native.assessContinuousFocusWire
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
module.exports.assessContinuousFocus = function assessContinuousFocus (a0, a1, a2) {
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `delay`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.delay
const call = native.delayWire
const encode = (a0) => {
at = 0
if (typeof a0 !== 'number') return false
wF64(a0)
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.delay = function delay (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `parseLifetimeCpuSeconds`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.parseLifetimeCpuSeconds
const call = native.parseLifetimeCpuSecondsWire
const encode = (a0) => {
at = 0
if (typeof a0 !== 'string') return false
wStr(a0)
return true
}
const decode = () => {
let t
t = rF64()
return t
}
module.exports.parseLifetimeCpuSeconds = function parseLifetimeCpuSeconds (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of parseLifetimeCpuSeconds has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `parseResourceSnapshot`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.parseResourceSnapshot
const call = native.parseResourceSnapshotWire
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
v1 = rF64()
o1["pid"] = v1
break
}
case 1: {
let v1
v1 = rF64()
o1["parentPid"] = v1
break
}
case 2: {
let v1
v1 = rStr()
o1["startedAt"] = v1
break
}
case 3: {
let v1
v1 = rStr()
o1["executable"] = v1
break
}
case 4: {
let v1
v1 = rF64()
o1["rssKiB"] = v1
break
}
case 5: {
let v1
v1 = rF64()
o1["cpuPercent"] = v1
break
}
case 6: {
let v1
v1 = rF64()
o1["lifetimeCpuSeconds"] = v1
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
module.exports.parseResourceSnapshot = function parseResourceSnapshot (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of parseResourceSnapshot has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `resourceIdentity`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.resourceIdentity
const call = native.resourceIdentityWire
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
module.exports.resourceIdentity = function resourceIdentity (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `makeResourceTracker`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.makeResourceTracker
const call = native.makeResourceTrackerWire
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
module.exports.makeResourceTracker = function makeResourceTracker (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `resourceSamplingOptions`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.resourceSamplingOptions
const call = native.resourceSamplingOptionsWire
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
v0 = rF64()
o0["rootPid"] = v0
break
}
case 1: {
let v0
v0 = rF64()
o0["durationMs"] = v0
break
}
case 2: {
let v0
v0 = rF64()
o0["intervalMs"] = v0
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
module.exports.resourceSamplingOptions = function resourceSamplingOptions (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}
const guards = {
  readGitHubReference: [true, [0]],
  artifactIdentity: [true, [0]],
  connectPackagedOwner: [true, []],
  connectOwnerUrl: [true, [0]],
  startNativeTarget: [true, [0]],
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
