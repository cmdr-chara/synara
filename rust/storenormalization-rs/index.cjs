// Loads the native module built by `rust-anything build`.
const native = require('./storenormalization_rs.node')

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
  threadSessionsEqual: [[1, 1], 'json', false],
  resolveCreateBranchFlowCompletedMerge: [[1], 'json', false],
  threadShellsEqual: [[1, 1], 'json', false],
  threadTurnStatesEqual: [[1, 1], 'json', false],
  textSegmentArraysEqual: [[1, 1], 'json', false],
  providerReferenceArraysEqual: [[1, 1], 'json', false],
  deepEqualJson: [[1, 1], 'json', false],
  normalizeModelSelection: [[1, 1], 'json', false],
  normalizeProject: [[1, 1], 'json', false],
  normalizeSpace: [[1, 1], 'json', false],
  mapSpaces: [[1, 1], 'json', false],
  normalizeChatMessage: [[1, 1], 'json', false],
  mergeReadModelThreadDetailWithLiveHotPath: [[1, 1, 1], 'json', false],
  normalizeProposedPlans: [[1, 1], 'json', false],
  normalizeTurnDiffFiles: [[1, 1], 'json', false],
  normalizeActivities: [[1, 1], 'json', false],
  withOrchestrationEventSequence: [[1, 1], 'json', false],
  capThreadActivities: [[1], 'json', false],
  dedupeActivitiesByIdAfterAppend: [[1, 1, 1], 'json', false],
  dedupeActivitiesById: [[1], 'json', false],
  asActivityRecord: [[1], 'json', false],
  normalizeThreadErrorMessage: [[1], 'json', false],
  normalizeThreadSession: [[1, 1], 'json', false],
  normalizeThreadFromReadModel: [[1, 1, 1, 1], 'json', false],
  normalizeThreadShellSnapshot: [[1, 1, 1, 1], 'json', false],
  mapProjects: [[1, 1], 'json', false],
  toLegacyProvider: [[1], 'json', false],
  resolveThreadSidebarMetadata: [[1], 'json', false],
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

// `threadSessionsEqual`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.threadSessionsEqual
const call = native.threadSessionsEqualWire
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
module.exports.threadSessionsEqual = function threadSessionsEqual (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `resolveCreateBranchFlowCompletedMerge`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.resolveCreateBranchFlowCompletedMerge
const call = native.resolveCreateBranchFlowCompletedMergeWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
t = rU8() === 1
return t
}
module.exports.resolveCreateBranchFlowCompletedMerge = function resolveCreateBranchFlowCompletedMerge (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `threadShellsEqual`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.threadShellsEqual
const call = native.threadShellsEqualWire
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
module.exports.threadShellsEqual = function threadShellsEqual (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `threadTurnStatesEqual`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.threadTurnStatesEqual
const call = native.threadTurnStatesEqualWire
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
module.exports.threadTurnStatesEqual = function threadTurnStatesEqual (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `textSegmentArraysEqual`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.textSegmentArraysEqual
const call = native.textSegmentArraysEqualWire
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
module.exports.textSegmentArraysEqual = function textSegmentArraysEqual (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `providerReferenceArraysEqual`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.providerReferenceArraysEqual
const call = native.providerReferenceArraysEqualWire
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
module.exports.providerReferenceArraysEqual = function providerReferenceArraysEqual (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `deepEqualJson`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.deepEqualJson
const call = native.deepEqualJsonWire
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
module.exports.deepEqualJson = function deepEqualJson (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `normalizeModelSelection`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.normalizeModelSelection
const call = native.normalizeModelSelectionWire
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
module.exports.normalizeModelSelection = function normalizeModelSelection (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `normalizeProject`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.normalizeProject
const call = native.normalizeProjectWire
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
v0 = rDyn()
o0["id"] = v0
break
}
case 1: {
let v0
v0 = rDyn()
o0["kind"] = v0
break
}
case 2: {
let v0
v0 = rStr()
o0["name"] = v0
break
}
case 3: {
let v0
v0 = rStr()
o0["remoteName"] = v0
break
}
case 4: {
let v0
v0 = rStr()
o0["folderName"] = v0
break
}
case 5: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["localName"] = v0
break
}
case 6: {
let v0
v0 = rDyn()
o0["appearance"] = v0
break
}
case 7: {
let v0
v0 = rStr()
o0["cwd"] = v0
break
}
case 8: {
let v0
v0 = rDyn()
o0["defaultModelSelection"] = v0
break
}
case 9: {
let v0
v0 = rU8() === 1
o0["expanded"] = v0
break
}
case 10: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rU8() === 1
}
}
o0["isPinned"] = v0
break
}
case 11: {
let v0
v0 = rDyn()
o0["spaceId"] = v0
break
}
case 12: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["createdAt"] = v0
break
}
case 13: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["updatedAt"] = v0
break
}
case 14: {
let v0
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const a1 = new Array(n1)
for (let i1 = 0; i1 < n1; i1++) {
let v1
v1 = rDyn()
a1[i1] = v1
}
v0 = a1
}
o0["scripts"] = v0
break
}
case 15: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const a2 = new Array(n2)
for (let i2 = 0; i2 < n2; i2++) {
let v2
v2 = rStr()
a2[i2] = v2
}
v0 = a2
}
}
}
o0["additionalFolders"] = v0
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
module.exports.normalizeProject = function normalizeProject (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `normalizeSpace`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.normalizeSpace
const call = native.normalizeSpaceWire
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
v0 = rDyn()
o0["id"] = v0
break
}
case 1: {
let v0
v0 = rStr()
o0["name"] = v0
break
}
case 2: {
let v0
v0 = rDyn()
o0["icon"] = v0
break
}
case 3: {
let v0
v0 = rF64()
o0["sortOrder"] = v0
break
}
case 4: {
let v0
v0 = rStr()
o0["createdAt"] = v0
break
}
case 5: {
let v0
v0 = rStr()
o0["updatedAt"] = v0
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
module.exports.normalizeSpace = function normalizeSpace (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `mapSpaces`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.mapSpaces
const call = native.mapSpacesWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (!wDyn(a1)) return false
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
v1 = rDyn()
o1["id"] = v1
break
}
case 1: {
let v1
v1 = rStr()
o1["name"] = v1
break
}
case 2: {
let v1
v1 = rDyn()
o1["icon"] = v1
break
}
case 3: {
let v1
v1 = rF64()
o1["sortOrder"] = v1
break
}
case 4: {
let v1
v1 = rStr()
o1["createdAt"] = v1
break
}
case 5: {
let v1
v1 = rStr()
o1["updatedAt"] = v1
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
module.exports.mapSpaces = function mapSpaces (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `normalizeChatMessage`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.normalizeChatMessage
const call = native.normalizeChatMessageWire
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
v0 = rDyn()
o0["id"] = v0
break
}
case 1: {
let v0
v0 = rStr()
o0["role"] = v0
break
}
case 2: {
let v0
v0 = rStr()
o0["text"] = v0
break
}
case 3: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const a2 = new Array(n2)
for (let i2 = 0; i2 < n2; i2++) {
let v2
v2 = rDyn()
a2[i2] = v2
}
v0 = a2
}
}
}
o0["textSegments"] = v0
break
}
case 4: {
let v0
v0 = rDyn()
o0["asyncUserInput"] = v0
break
}
case 5: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const a2 = new Array(n2)
for (let i2 = 0; i2 < n2; i2++) {
let v2
v2 = rDyn()
a2[i2] = v2
}
v0 = a2
}
}
}
o0["attachments"] = v0
break
}
case 6: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const a2 = new Array(n2)
for (let i2 = 0; i2 < n2; i2++) {
let v2
v2 = rDyn()
a2[i2] = v2
}
v0 = a2
}
}
}
o0["skills"] = v0
break
}
case 7: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const a2 = new Array(n2)
for (let i2 = 0; i2 < n2; i2++) {
let v2
v2 = rDyn()
a2[i2] = v2
}
v0 = a2
}
}
}
o0["mentions"] = v0
break
}
case 8: {
let v0
v0 = rDyn()
o0["dispatchMode"] = v0
break
}
case 9: {
let v0
v0 = rDyn()
o0["dispatchOrigin"] = v0
break
}
case 10: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rU8() === 1
}
}
o0["startsNewTurn"] = v0
break
}
case 11: {
let v0
v0 = rDyn()
o0["turnId"] = v0
break
}
case 12: {
let v0
v0 = rStr()
o0["createdAt"] = v0
break
}
case 13: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["updatedAt"] = v0
break
}
case 14: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["completedAt"] = v0
break
}
case 15: {
let v0
v0 = rU8() === 1
o0["streaming"] = v0
break
}
case 16: {
let v0
v0 = rDyn()
o0["source"] = v0
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
module.exports.normalizeChatMessage = function normalizeChatMessage (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `mergeReadModelThreadDetailWithLiveHotPath`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.mergeReadModelThreadDetailWithLiveHotPath
const call = native.mergeReadModelThreadDetailWithLiveHotPathWire
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
module.exports.mergeReadModelThreadDetailWithLiveHotPath = function mergeReadModelThreadDetailWithLiveHotPath (a0, a1, a2) {
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `normalizeProposedPlans`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.normalizeProposedPlans
const call = native.normalizeProposedPlansWire
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
module.exports.normalizeProposedPlans = function normalizeProposedPlans (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `normalizeTurnDiffFiles`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.normalizeTurnDiffFiles
const call = native.normalizeTurnDiffFilesWire
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
module.exports.normalizeTurnDiffFiles = function normalizeTurnDiffFiles (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `normalizeActivities`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.normalizeActivities
const call = native.normalizeActivitiesWire
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
module.exports.normalizeActivities = function normalizeActivities (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `withOrchestrationEventSequence`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.withOrchestrationEventSequence
const call = native.withOrchestrationEventSequenceWire
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
module.exports.withOrchestrationEventSequence = function withOrchestrationEventSequence (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `capThreadActivities`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.capThreadActivities
const call = native.capThreadActivitiesWire
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
v0 = rDyn()
a0[i0] = v0
}
t = a0
}
return t
}
module.exports.capThreadActivities = function capThreadActivities (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `dedupeActivitiesByIdAfterAppend`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.dedupeActivitiesByIdAfterAppend
const call = native.dedupeActivitiesByIdAfterAppendWire
const encode = (a0, a1, a2) => {
at = 0
if (!wDyn(a0)) return false
if (!wDyn(a1)) return false
if (!wDyn(a2)) return false
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
v0 = rDyn()
a0[i0] = v0
}
t = a0
}
return t
}
module.exports.dedupeActivitiesByIdAfterAppend = function dedupeActivitiesByIdAfterAppend (a0, a1, a2) {
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `dedupeActivitiesById`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.dedupeActivitiesById
const call = native.dedupeActivitiesByIdWire
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
v0 = rDyn()
a0[i0] = v0
}
t = a0
}
return t
}
module.exports.dedupeActivitiesById = function dedupeActivitiesById (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `asActivityRecord`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.asActivityRecord
const call = native.asActivityRecordWire
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
const o1 = {}
for (let i1 = 0; i1 < n1; i1++) {
const k1 = rStr()
let v1
v1 = rDyn()
o1[k1] = v1
}
t = o1
}
}
}
return t
}
module.exports.asActivityRecord = function asActivityRecord (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `normalizeThreadErrorMessage`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.normalizeThreadErrorMessage
const call = native.normalizeThreadErrorMessageWire
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
module.exports.normalizeThreadErrorMessage = function normalizeThreadErrorMessage (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `normalizeThreadSession`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.normalizeThreadSession
const call = native.normalizeThreadSessionWire
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
module.exports.normalizeThreadSession = function normalizeThreadSession (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `normalizeThreadFromReadModel`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.normalizeThreadFromReadModel
const call = native.normalizeThreadFromReadModelWire
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
v0 = rU8() === 1
}
}
o0["isProjectImport"] = v0
break
}
case 1: {
let v0
v0 = rDyn()
o0["id"] = v0
break
}
case 2: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["codexThreadId"] = v0
break
}
case 3: {
let v0
v0 = rDyn()
o0["projectId"] = v0
break
}
case 4: {
let v0
v0 = rStr()
o0["title"] = v0
break
}
case 5: {
let v0
v0 = rDyn()
o0["modelSelection"] = v0
break
}
case 6: {
let v0
v0 = rDyn()
o0["runtimeMode"] = v0
break
}
case 7: {
let v0
v0 = rDyn()
o0["interactionMode"] = v0
break
}
case 8: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
v2 = rDyn()
o2["provider"] = v2
break
}
case 1: {
let v2
v2 = rDyn()
o2["runtimeMode"] = v2
break
}
case 2: {
let v2
v2 = rDyn()
o2["providerInstanceId"] = v2
break
}
case 3: {
let v2
v2 = rDyn()
o2["status"] = v2
break
}
case 4: {
let v2
v2 = rDyn()
o2["activeTurnId"] = v2
break
}
case 5: {
let v2
v2 = rStr()
o2["createdAt"] = v2
break
}
case 6: {
let v2
v2 = rStr()
o2["updatedAt"] = v2
break
}
case 7: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["lastError"] = v2
break
}
case 8: {
let v2
v2 = rDyn()
o2["orchestrationStatus"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v0 = o2
}
}
}
o0["session"] = v0
break
}
case 9: {
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
v2 = rDyn()
o2["id"] = v2
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
o2["text"] = v2
break
}
case 3: {
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
o2["textSegments"] = v2
break
}
case 4: {
let v2
v2 = rDyn()
o2["asyncUserInput"] = v2
break
}
case 5: {
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
o2["attachments"] = v2
break
}
case 6: {
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
o2["skills"] = v2
break
}
case 7: {
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
o2["mentions"] = v2
break
}
case 8: {
let v2
v2 = rDyn()
o2["dispatchMode"] = v2
break
}
case 9: {
let v2
v2 = rDyn()
o2["dispatchOrigin"] = v2
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
o2["startsNewTurn"] = v2
break
}
case 11: {
let v2
v2 = rDyn()
o2["turnId"] = v2
break
}
case 12: {
let v2
v2 = rStr()
o2["createdAt"] = v2
break
}
case 13: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["updatedAt"] = v2
break
}
case 14: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["completedAt"] = v2
break
}
case 15: {
let v2
v2 = rU8() === 1
o2["streaming"] = v2
break
}
case 16: {
let v2
v2 = rDyn()
o2["source"] = v2
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
o0["messages"] = v0
break
}
case 10: {
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
v2 = rDyn()
o2["id"] = v2
break
}
case 1: {
let v2
v2 = rDyn()
o2["turnId"] = v2
break
}
case 2: {
let v2
v2 = rStr()
o2["planMarkdown"] = v2
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
o2["implementedAt"] = v2
break
}
case 4: {
let v2
v2 = rDyn()
o2["implementationThreadId"] = v2
break
}
case 5: {
let v2
v2 = rStr()
o2["createdAt"] = v2
break
}
case 6: {
let v2
v2 = rStr()
o2["updatedAt"] = v2
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
o0["proposedPlans"] = v0
break
}
case 11: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["error"] = v0
break
}
case 12: {
let v0
v0 = rStr()
o0["createdAt"] = v0
break
}
case 13: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["archivedAt"] = v0
break
}
case 14: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["settledAt"] = v0
break
}
case 15: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["snoozedUntil"] = v0
break
}
case 16: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["snoozeReminderAt"] = v0
break
}
case 17: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["updatedAt"] = v0
break
}
case 18: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rU8() === 1
}
}
o0["isPinned"] = v0
break
}
case 19: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const a2 = new Array(n2)
for (let i2 = 0; i2 < n2; i2++) {
let v2
v2 = rDyn()
a2[i2] = v2
}
v0 = a2
}
}
}
o0["pinnedMessages"] = v0
break
}
case 20: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["notes"] = v0
break
}
case 21: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["goal"] = v0
break
}
case 22: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["goalStartedAt"] = v0
break
}
case 23: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["goalPausedAt"] = v0
break
}
case 24: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const a2 = new Array(n2)
for (let i2 = 0; i2 < n2; i2++) {
let v2
v2 = rDyn()
a2[i2] = v2
}
v0 = a2
}
}
}
o0["goalAchievements"] = v0
break
}
case 25: {
let v0
v0 = rDyn()
o0["latestTurn"] = v0
break
}
case 26: {
let v0
v0 = rDyn()
o0["pendingSourceProposedPlan"] = v0
break
}
case 27: {
let v0
v0 = rDyn()
o0["pendingTurnStartMessageId"] = v0
break
}
case 28: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["lastVisitedAt"] = v0
break
}
case 29: {
let v0
v0 = rDyn()
o0["parentThreadId"] = v0
break
}
case 30: {
let v0
v0 = rDyn()
o0["creationSource"] = v0
break
}
case 31: {
let v0
v0 = rDyn()
o0["sourceThreadId"] = v0
break
}
case 32: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["subagentAgentId"] = v0
break
}
case 33: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["subagentNickname"] = v0
break
}
case 34: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["subagentRole"] = v0
break
}
case 35: {
let v0
v0 = rDyn()
o0["forkSourceThreadId"] = v0
break
}
case 36: {
let v0
v0 = rDyn()
o0["sidechatSourceThreadId"] = v0
break
}
case 37: {
let v0
v0 = rDyn()
o0["sidechatContext"] = v0
break
}
case 38: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["sidechatLastActivityAt"] = v0
break
}
case 39: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["sidechatExpiredAt"] = v0
break
}
case 40: {
let v0
v0 = rDyn()
o0["handoff"] = v0
break
}
case 41: {
let v0
v0 = rDyn()
o0["usageLimitRecovery"] = v0
break
}
case 42: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rF64()
}
}
o0["usageLimitRecoverySequence"] = v0
break
}
case 43: {
let v0
v0 = rDyn()
o0["claudeCacheReview"] = v0
break
}
case 44: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rF64()
}
}
o0["claudeCacheReviewSequence"] = v0
break
}
case 45: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rF64()
}
}
o0["snoozeSequence"] = v0
break
}
case 46: {
let v0
v0 = rDyn()
o0["lastKnownPr"] = v0
break
}
case 47: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const a2 = new Array(n2)
for (let i2 = 0; i2 < n2; i2++) {
let v2
v2 = rDyn()
a2[i2] = v2
}
v0 = a2
}
}
}
o0["pullRequests"] = v0
break
}
case 48: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["latestUserMessageAt"] = v0
break
}
case 49: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["latestHumanMessageAt"] = v0
break
}
case 50: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rU8() === 1
}
}
o0["hasPendingApprovals"] = v0
break
}
case 51: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rU8() === 1
}
}
o0["hasPendingUserInput"] = v0
break
}
case 52: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rU8() === 1
}
}
o0["hasActionableProposedPlan"] = v0
break
}
case 53: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rF64()
}
}
o0["pendingBackgroundTaskCount"] = v0
break
}
case 54: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["latestReplyPreview"] = v0
break
}
case 55: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const a2 = new Array(n2)
for (let i2 = 0; i2 < n2; i2++) {
let v2
v2 = rDyn()
a2[i2] = v2
}
v0 = a2
}
}
}
o0["pendingInteractions"] = v0
break
}
case 56: {
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
v2 = rDyn()
o2["turnId"] = v2
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
o2["startedAt"] = v2
break
}
case 2: {
let v2
v2 = rStr()
o2["completedAt"] = v2
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
o2["status"] = v2
break
}
case 4: {
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
o2["files"] = v2
break
}
case 5: {
let v2
v2 = rDyn()
o2["checkpointRef"] = v2
break
}
case 6: {
let v2
v2 = rDyn()
o2["assistantMessageId"] = v2
break
}
case 7: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rF64()
}
}
o2["checkpointTurnCount"] = v2
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
o2["checkpointTurnCounts"] = v2
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
o0["turnDiffSummaries"] = v0
break
}
case 57: {
let v0
if (rU8() === 0) v0 = null
else {
const n1 = rU32()
const a1 = new Array(n1)
for (let i1 = 0; i1 < n1; i1++) {
let v1
v1 = rDyn()
a1[i1] = v1
}
v0 = a1
}
o0["activities"] = v0
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
module.exports.normalizeThreadFromReadModel = function normalizeThreadFromReadModel (a0, a1, a2, a3) {
  try {
    if (arguments.length <= 4 && encode(a0, a1, a2, a3) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `normalizeThreadShellSnapshot`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.normalizeThreadShellSnapshot
const call = native.normalizeThreadShellSnapshotWire
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
{
const o0 = {}
const n0 = rU8()
for (let j0 = 0; j0 < n0; j0++) {
switch (rU8()) {
case 0: {
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
v1 = rU8() === 1
}
}
o1["isProjectImport"] = v1
break
}
case 1: {
let v1
v1 = rDyn()
o1["id"] = v1
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
o1["codexThreadId"] = v1
break
}
case 3: {
let v1
v1 = rDyn()
o1["projectId"] = v1
break
}
case 4: {
let v1
v1 = rStr()
o1["title"] = v1
break
}
case 5: {
let v1
v1 = rDyn()
o1["modelSelection"] = v1
break
}
case 6: {
let v1
v1 = rDyn()
o1["runtimeMode"] = v1
break
}
case 7: {
let v1
v1 = rDyn()
o1["interactionMode"] = v1
break
}
case 8: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["error"] = v1
break
}
case 9: {
let v1
v1 = rStr()
o1["createdAt"] = v1
break
}
case 10: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["archivedAt"] = v1
break
}
case 11: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["settledAt"] = v1
break
}
case 12: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["snoozedUntil"] = v1
break
}
case 13: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["snoozeReminderAt"] = v1
break
}
case 14: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["updatedAt"] = v1
break
}
case 15: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rU8() === 1
}
}
o1["isPinned"] = v1
break
}
case 16: {
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
v3 = rDyn()
a3[i3] = v3
}
v1 = a3
}
}
}
o1["pinnedMessages"] = v1
break
}
case 17: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["notes"] = v1
break
}
case 18: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["goal"] = v1
break
}
case 19: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["goalStartedAt"] = v1
break
}
case 20: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["goalPausedAt"] = v1
break
}
case 21: {
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
v3 = rDyn()
a3[i3] = v3
}
v1 = a3
}
}
}
o1["goalAchievements"] = v1
break
}
case 22: {
let v1
v1 = rDyn()
o1["parentThreadId"] = v1
break
}
case 23: {
let v1
v1 = rDyn()
o1["creationSource"] = v1
break
}
case 24: {
let v1
v1 = rDyn()
o1["sourceThreadId"] = v1
break
}
case 25: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["subagentAgentId"] = v1
break
}
case 26: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["subagentNickname"] = v1
break
}
case 27: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["subagentRole"] = v1
break
}
case 28: {
let v1
v1 = rDyn()
o1["forkSourceThreadId"] = v1
break
}
case 29: {
let v1
v1 = rDyn()
o1["sidechatSourceThreadId"] = v1
break
}
case 30: {
let v1
v1 = rDyn()
o1["sidechatContext"] = v1
break
}
case 31: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["sidechatLastActivityAt"] = v1
break
}
case 32: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["sidechatExpiredAt"] = v1
break
}
case 33: {
let v1
v1 = rDyn()
o1["handoff"] = v1
break
}
case 34: {
let v1
v1 = rDyn()
o1["usageLimitRecovery"] = v1
break
}
case 35: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rF64()
}
}
o1["usageLimitRecoverySequence"] = v1
break
}
case 36: {
let v1
v1 = rDyn()
o1["claudeCacheReview"] = v1
break
}
case 37: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rF64()
}
}
o1["claudeCacheReviewSequence"] = v1
break
}
case 38: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rF64()
}
}
o1["snoozeSequence"] = v1
break
}
case 39: {
let v1
v1 = rDyn()
o1["lastKnownPr"] = v1
break
}
case 40: {
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
v3 = rDyn()
a3[i3] = v3
}
v1 = a3
}
}
}
o1["pullRequests"] = v1
break
}
case 41: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["latestUserMessageAt"] = v1
break
}
case 42: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["latestHumanMessageAt"] = v1
break
}
case 43: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rU8() === 1
}
}
o1["hasPendingApprovals"] = v1
break
}
case 44: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rU8() === 1
}
}
o1["hasPendingUserInput"] = v1
break
}
case 45: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rU8() === 1
}
}
o1["hasActionableProposedPlan"] = v1
break
}
case 46: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rF64()
}
}
o1["pendingBackgroundTaskCount"] = v1
break
}
case 47: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["latestReplyPreview"] = v1
break
}
case 48: {
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
v3 = rDyn()
a3[i3] = v3
}
v1 = a3
}
}
}
o1["pendingInteractions"] = v1
break
}
case 49: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["lastVisitedAt"] = v1
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
o0["shell"] = v0
break
}
case 1: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
v2 = rDyn()
o2["provider"] = v2
break
}
case 1: {
let v2
v2 = rDyn()
o2["runtimeMode"] = v2
break
}
case 2: {
let v2
v2 = rDyn()
o2["providerInstanceId"] = v2
break
}
case 3: {
let v2
v2 = rDyn()
o2["status"] = v2
break
}
case 4: {
let v2
v2 = rDyn()
o2["activeTurnId"] = v2
break
}
case 5: {
let v2
v2 = rStr()
o2["createdAt"] = v2
break
}
case 6: {
let v2
v2 = rStr()
o2["updatedAt"] = v2
break
}
case 7: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["lastError"] = v2
break
}
case 8: {
let v2
v2 = rDyn()
o2["orchestrationStatus"] = v2
break
}
default: {
const k2 = rStr()
o2[k2] = rDyn()
}
}
}
v0 = o2
}
}
}
o0["session"] = v0
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
v1 = rDyn()
o1["latestTurn"] = v1
break
}
case 1: {
let v1
v1 = rDyn()
o1["pendingSourceProposedPlan"] = v1
break
}
case 2: {
let v1
v1 = rDyn()
o1["pendingTurnStartMessageId"] = v1
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
o0["turnState"] = v0
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
module.exports.normalizeThreadShellSnapshot = function normalizeThreadShellSnapshot (a0, a1, a2, a3) {
  try {
    if (arguments.length <= 4 && encode(a0, a1, a2, a3) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `mapProjects`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.mapProjects
const call = native.mapProjectsWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (!wDyn(a1)) return false
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
v1 = rDyn()
o1["id"] = v1
break
}
case 1: {
let v1
v1 = rDyn()
o1["kind"] = v1
break
}
case 2: {
let v1
v1 = rStr()
o1["name"] = v1
break
}
case 3: {
let v1
v1 = rStr()
o1["remoteName"] = v1
break
}
case 4: {
let v1
v1 = rStr()
o1["folderName"] = v1
break
}
case 5: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["localName"] = v1
break
}
case 6: {
let v1
v1 = rDyn()
o1["appearance"] = v1
break
}
case 7: {
let v1
v1 = rStr()
o1["cwd"] = v1
break
}
case 8: {
let v1
v1 = rDyn()
o1["defaultModelSelection"] = v1
break
}
case 9: {
let v1
v1 = rU8() === 1
o1["expanded"] = v1
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
o1["isPinned"] = v1
break
}
case 11: {
let v1
v1 = rDyn()
o1["spaceId"] = v1
break
}
case 12: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["createdAt"] = v1
break
}
case 13: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["updatedAt"] = v1
break
}
case 14: {
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
o1["scripts"] = v1
break
}
case 15: {
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
v3 = rStr()
a3[i3] = v3
}
v1 = a3
}
}
}
o1["additionalFolders"] = v1
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
module.exports.mapProjects = function mapProjects (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `toLegacyProvider`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.toLegacyProvider
const call = native.toLegacyProviderWire
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
module.exports.toLegacyProvider = function toLegacyProvider (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `resolveThreadSidebarMetadata`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.resolveThreadSidebarMetadata
const call = native.resolveThreadSidebarMetadataWire
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
module.exports.resolveThreadSidebarMetadata = function resolveThreadSidebarMetadata (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
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
