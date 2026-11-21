// Loads the native module built by `rust-anything build`.
const native = require('./storeeventreducer_rs.node')

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
  applyOrchestrationEvents: [[1, 1], 'json', false],
  applyOrchestrationEventsHotPath: [[1, 1, 1], 'json', false],
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

// `applyOrchestrationEvents`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.applyOrchestrationEvents
const call = native.applyOrchestrationEventsWire
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
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rF64()
}
}
o0["shellSnapshotSequence"] = v0
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
v2 = rDyn()
o2["id"] = v2
break
}
case 1: {
let v2
v2 = rStr()
o2["name"] = v2
break
}
case 2: {
let v2
v2 = rDyn()
o2["icon"] = v2
break
}
case 3: {
let v2
v2 = rF64()
o2["sortOrder"] = v2
break
}
case 4: {
let v2
v2 = rStr()
o2["createdAt"] = v2
break
}
case 5: {
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
o0["spaces"] = v0
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
v2 = rDyn()
o2["id"] = v2
break
}
case 1: {
let v2
v2 = rDyn()
o2["kind"] = v2
break
}
case 2: {
let v2
v2 = rStr()
o2["name"] = v2
break
}
case 3: {
let v2
v2 = rStr()
o2["remoteName"] = v2
break
}
case 4: {
let v2
v2 = rStr()
o2["folderName"] = v2
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
o2["localName"] = v2
break
}
case 6: {
let v2
v2 = rDyn()
o2["appearance"] = v2
break
}
case 7: {
let v2
v2 = rStr()
o2["cwd"] = v2
break
}
case 8: {
let v2
v2 = rDyn()
o2["defaultModelSelection"] = v2
break
}
case 9: {
let v2
v2 = rU8() === 1
o2["expanded"] = v2
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
o2["isPinned"] = v2
break
}
case 11: {
let v2
v2 = rDyn()
o2["spaceId"] = v2
break
}
case 12: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
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
o2["scripts"] = v2
break
}
case 15: {
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
o2["additionalFolders"] = v2
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
o0["projects"] = v0
break
}
case 3: {
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
v2 = rDyn()
o2["id"] = v2
break
}
case 1: {
let v2
v2 = rDyn()
o2["projectId"] = v2
break
}
case 2: {
let v2
v2 = rStr()
o2["title"] = v2
break
}
case 3: {
let v2
v2 = rDyn()
o2["modelSelection"] = v2
break
}
case 4: {
let v2
v2 = rDyn()
o2["interactionMode"] = v2
break
}
case 5: {
let v2
v2 = rDyn()
o2["envMode"] = v2
break
}
case 6: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["branch"] = v2
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
o2["worktreePath"] = v2
break
}
case 8: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["workingDirectory"] = v2
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
o2["associatedWorktreePath"] = v2
break
}
case 10: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["associatedWorktreeBranch"] = v2
break
}
case 11: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["associatedWorktreeRef"] = v2
break
}
case 12: {
let v2
v2 = rDyn()
o2["session"] = v2
break
}
case 13: {
let v2
v2 = rStr()
o2["createdAt"] = v2
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
o2["archivedAt"] = v2
break
}
case 15: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["settledAt"] = v2
break
}
case 16: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["snoozedUntil"] = v2
break
}
case 17: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["snoozeReminderAt"] = v2
break
}
case 18: {
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
case 19: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rU8() === 1
}
}
o2["isPinned"] = v2
break
}
case 20: {
let v2
v2 = rDyn()
o2["latestTurn"] = v2
break
}
case 21: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["lastVisitedAt"] = v2
break
}
case 22: {
let v2
v2 = rDyn()
o2["parentThreadId"] = v2
break
}
case 23: {
let v2
v2 = rDyn()
o2["creationSource"] = v2
break
}
case 24: {
let v2
v2 = rDyn()
o2["sourceThreadId"] = v2
break
}
case 25: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["subagentAgentId"] = v2
break
}
case 26: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["subagentNickname"] = v2
break
}
case 27: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["subagentRole"] = v2
break
}
case 28: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["latestUserMessageAt"] = v2
break
}
case 29: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["latestHumanMessageAt"] = v2
break
}
case 30: {
let v2
v2 = rU8() === 1
o2["hasPendingApprovals"] = v2
break
}
case 31: {
let v2
v2 = rU8() === 1
o2["hasPendingUserInput"] = v2
break
}
case 32: {
let v2
v2 = rU8() === 1
o2["hasActionableProposedPlan"] = v2
break
}
case 33: {
let v2
v2 = rU8() === 1
o2["hasLiveTailWork"] = v2
break
}
case 34: {
let v2
v2 = rF64()
o2["pendingBackgroundWorkCount"] = v2
break
}
case 35: {
let v2
v2 = rDyn()
o2["forkSourceThreadId"] = v2
break
}
case 36: {
let v2
v2 = rDyn()
o2["sidechatSourceThreadId"] = v2
break
}
case 37: {
let v2
v2 = rDyn()
o2["sidechatContext"] = v2
break
}
case 38: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["sidechatLastActivityAt"] = v2
break
}
case 39: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["sidechatExpiredAt"] = v2
break
}
case 40: {
let v2
v2 = rDyn()
o2["handoff"] = v2
break
}
case 41: {
let v2
v2 = rDyn()
o2["lastKnownPr"] = v2
break
}
case 42: {
let v2
v2 = rDyn()
o2["pullRequests"] = v2
break
}
case 43: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["latestReplyPreview"] = v2
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
o0["sidebarThreadSummaryById"] = v0
break
}
case 4: {
let v0
v0 = rU8() === 1
o0["threadsHydrated"] = v0
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
o0["threadIds"] = v0
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
const o2 = {}
for (let i2 = 0; i2 < n2; i2++) {
const k2 = rStr()
let v2
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
v3 = rU8() === 1
}
}
o3["isProjectImport"] = v3
break
}
case 1: {
let v3
v3 = rDyn()
o3["id"] = v3
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
o3["codexThreadId"] = v3
break
}
case 3: {
let v3
v3 = rDyn()
o3["projectId"] = v3
break
}
case 4: {
let v3
v3 = rStr()
o3["title"] = v3
break
}
case 5: {
let v3
v3 = rDyn()
o3["modelSelection"] = v3
break
}
case 6: {
let v3
v3 = rDyn()
o3["runtimeMode"] = v3
break
}
case 7: {
let v3
v3 = rDyn()
o3["interactionMode"] = v3
break
}
case 8: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["error"] = v3
break
}
case 9: {
let v3
v3 = rStr()
o3["createdAt"] = v3
break
}
case 10: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["archivedAt"] = v3
break
}
case 11: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["settledAt"] = v3
break
}
case 12: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["snoozedUntil"] = v3
break
}
case 13: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["snoozeReminderAt"] = v3
break
}
case 14: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["updatedAt"] = v3
break
}
case 15: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rU8() === 1
}
}
o3["isPinned"] = v3
break
}
case 16: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
v5 = rDyn()
a5[i5] = v5
}
v3 = a5
}
}
}
o3["pinnedMessages"] = v3
break
}
case 17: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["notes"] = v3
break
}
case 18: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["goal"] = v3
break
}
case 19: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["goalStartedAt"] = v3
break
}
case 20: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["goalPausedAt"] = v3
break
}
case 21: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
v5 = rDyn()
a5[i5] = v5
}
v3 = a5
}
}
}
o3["goalAchievements"] = v3
break
}
case 22: {
let v3
v3 = rDyn()
o3["parentThreadId"] = v3
break
}
case 23: {
let v3
v3 = rDyn()
o3["creationSource"] = v3
break
}
case 24: {
let v3
v3 = rDyn()
o3["sourceThreadId"] = v3
break
}
case 25: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["subagentAgentId"] = v3
break
}
case 26: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["subagentNickname"] = v3
break
}
case 27: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["subagentRole"] = v3
break
}
case 28: {
let v3
v3 = rDyn()
o3["forkSourceThreadId"] = v3
break
}
case 29: {
let v3
v3 = rDyn()
o3["sidechatSourceThreadId"] = v3
break
}
case 30: {
let v3
v3 = rDyn()
o3["sidechatContext"] = v3
break
}
case 31: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["sidechatLastActivityAt"] = v3
break
}
case 32: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["sidechatExpiredAt"] = v3
break
}
case 33: {
let v3
v3 = rDyn()
o3["handoff"] = v3
break
}
case 34: {
let v3
v3 = rDyn()
o3["usageLimitRecovery"] = v3
break
}
case 35: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["usageLimitRecoverySequence"] = v3
break
}
case 36: {
let v3
v3 = rDyn()
o3["claudeCacheReview"] = v3
break
}
case 37: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["claudeCacheReviewSequence"] = v3
break
}
case 38: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["snoozeSequence"] = v3
break
}
case 39: {
let v3
v3 = rDyn()
o3["lastKnownPr"] = v3
break
}
case 40: {
let v3
v3 = rDyn()
o3["pullRequests"] = v3
break
}
case 41: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["latestUserMessageAt"] = v3
break
}
case 42: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["latestHumanMessageAt"] = v3
break
}
case 43: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rU8() === 1
}
}
o3["hasPendingApprovals"] = v3
break
}
case 44: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rU8() === 1
}
}
o3["hasPendingUserInput"] = v3
break
}
case 45: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rU8() === 1
}
}
o3["hasActionableProposedPlan"] = v3
break
}
case 46: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["pendingBackgroundTaskCount"] = v3
break
}
case 47: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["latestReplyPreview"] = v3
break
}
case 48: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
v5 = rDyn()
a5[i5] = v5
}
v3 = a5
}
}
}
o3["pendingInteractions"] = v3
break
}
case 49: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["lastVisitedAt"] = v3
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
o2[k2] = v2
}
v0 = o2
}
}
}
o0["threadShellById"] = v0
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
const o2 = {}
for (let i2 = 0; i2 < n2; i2++) {
const k2 = rStr()
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
{
const o4 = {}
const n4 = rU8()
for (let j4 = 0; j4 < n4; j4++) {
switch (rU8()) {
case 0: {
let v4
v4 = rDyn()
o4["provider"] = v4
break
}
case 1: {
let v4
v4 = rDyn()
o4["runtimeMode"] = v4
break
}
case 2: {
let v4
v4 = rDyn()
o4["providerInstanceId"] = v4
break
}
case 3: {
let v4
v4 = rDyn()
o4["status"] = v4
break
}
case 4: {
let v4
v4 = rDyn()
o4["activeTurnId"] = v4
break
}
case 5: {
let v4
v4 = rDyn()
o4["createdAt"] = v4
break
}
case 6: {
let v4
v4 = rDyn()
o4["updatedAt"] = v4
break
}
case 7: {
let v4
v4 = rDyn()
o4["lastError"] = v4
break
}
case 8: {
let v4
v4 = rDyn()
o4["orchestrationStatus"] = v4
break
}
default: {
const k4 = rStr()
o4[k4] = rDyn()
}
}
}
v2 = o4
}
}
}
o2[k2] = v2
}
v0 = o2
}
}
}
o0["threadSessionById"] = v0
break
}
case 8: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const o2 = {}
for (let i2 = 0; i2 < n2; i2++) {
const k2 = rStr()
let v2
{
const o3 = {}
const n3 = rU8()
for (let j3 = 0; j3 < n3; j3++) {
switch (rU8()) {
case 0: {
let v3
v3 = rDyn()
o3["latestTurn"] = v3
break
}
case 1: {
let v3
v3 = rDyn()
o3["pendingSourceProposedPlan"] = v3
break
}
case 2: {
let v3
v3 = rDyn()
o3["pendingTurnStartMessageId"] = v3
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
o2[k2] = v2
}
v0 = o2
}
}
}
o0["threadTurnStateById"] = v0
break
}
case 9: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const o2 = {}
for (let i2 = 0; i2 < n2; i2++) {
const k2 = rStr()
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
o2[k2] = v2
}
v0 = o2
}
}
}
o0["messageIdsByThreadId"] = v0
break
}
case 10: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const o2 = {}
for (let i2 = 0; i2 < n2; i2++) {
const k2 = rStr()
let v2
if (rU8() === 0) v2 = null
else {
const n3 = rU32()
const o3 = {}
for (let i3 = 0; i3 < n3; i3++) {
const k3 = rStr()
let v3
v3 = rDyn()
o3[k3] = v3
}
v2 = o3
}
o2[k2] = v2
}
v0 = o2
}
}
}
o0["messageByThreadId"] = v0
break
}
case 11: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const o2 = {}
for (let i2 = 0; i2 < n2; i2++) {
const k2 = rStr()
let v2
if (rU8() === 0) v2 = null
else {
const n3 = rU32()
const a3 = new Array(n3)
for (let i3 = 0; i3 < n3; i3++) {
let v3
v3 = rStr()
a3[i3] = v3
}
v2 = a3
}
o2[k2] = v2
}
v0 = o2
}
}
}
o0["activityIdsByThreadId"] = v0
break
}
case 12: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const o2 = {}
for (let i2 = 0; i2 < n2; i2++) {
const k2 = rStr()
let v2
if (rU8() === 0) v2 = null
else {
const n3 = rU32()
const o3 = {}
for (let i3 = 0; i3 < n3; i3++) {
const k3 = rStr()
let v3
v3 = rDyn()
o3[k3] = v3
}
v2 = o3
}
o2[k2] = v2
}
v0 = o2
}
}
}
o0["activityByThreadId"] = v0
break
}
case 13: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const o2 = {}
for (let i2 = 0; i2 < n2; i2++) {
const k2 = rStr()
let v2
if (rU8() === 0) v2 = null
else {
const n3 = rU32()
const a3 = new Array(n3)
for (let i3 = 0; i3 < n3; i3++) {
let v3
v3 = rStr()
a3[i3] = v3
}
v2 = a3
}
o2[k2] = v2
}
v0 = o2
}
}
}
o0["proposedPlanIdsByThreadId"] = v0
break
}
case 14: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const o2 = {}
for (let i2 = 0; i2 < n2; i2++) {
const k2 = rStr()
let v2
if (rU8() === 0) v2 = null
else {
const n3 = rU32()
const o3 = {}
for (let i3 = 0; i3 < n3; i3++) {
const k3 = rStr()
let v3
v3 = rDyn()
o3[k3] = v3
}
v2 = o3
}
o2[k2] = v2
}
v0 = o2
}
}
}
o0["proposedPlanByThreadId"] = v0
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
const o2 = {}
for (let i2 = 0; i2 < n2; i2++) {
const k2 = rStr()
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
o2[k2] = v2
}
v0 = o2
}
}
}
o0["turnDiffIdsByThreadId"] = v0
break
}
case 16: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const o2 = {}
for (let i2 = 0; i2 < n2; i2++) {
const k2 = rStr()
let v2
if (rU8() === 0) v2 = null
else {
const n3 = rU32()
const o3 = {}
for (let i3 = 0; i3 < n3; i3++) {
const k3 = rStr()
let v3
v3 = rDyn()
o3[k3] = v3
}
v2 = o3
}
o2[k2] = v2
}
v0 = o2
}
}
}
o0["turnDiffSummaryByThreadId"] = v0
break
}
case 17: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const o2 = {}
for (let i2 = 0; i2 < n2; i2++) {
const k2 = rStr()
let v2
v2 = rStr()
o2[k2] = v2
}
v0 = o2
}
}
}
o0["threadDetailSyncById"] = v0
break
}
case 18: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const o2 = {}
for (let i2 = 0; i2 < n2; i2++) {
const k2 = rStr()
let v2
v2 = rF64()
o2[k2] = v2
}
v0 = o2
}
}
}
o0["threadDetailRevisionSequenceById"] = v0
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
const o2 = {}
for (let i2 = 0; i2 < n2; i2++) {
const k2 = rStr()
let v2
v2 = rF64()
o2[k2] = v2
}
v0 = o2
}
}
}
o0["deletedProjectIdsById"] = v0
break
}
case 20: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const o2 = {}
for (let i2 = 0; i2 < n2; i2++) {
const k2 = rStr()
let v2
v2 = rF64()
o2[k2] = v2
}
v0 = o2
}
}
}
o0["deletedThreadIdsById"] = v0
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
module.exports.applyOrchestrationEvents = function applyOrchestrationEvents (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `applyOrchestrationEventsHotPath`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.applyOrchestrationEventsHotPath
const call = native.applyOrchestrationEventsHotPathWire
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
v0 = rF64()
}
}
o0["shellSnapshotSequence"] = v0
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
v2 = rDyn()
o2["id"] = v2
break
}
case 1: {
let v2
v2 = rStr()
o2["name"] = v2
break
}
case 2: {
let v2
v2 = rDyn()
o2["icon"] = v2
break
}
case 3: {
let v2
v2 = rF64()
o2["sortOrder"] = v2
break
}
case 4: {
let v2
v2 = rStr()
o2["createdAt"] = v2
break
}
case 5: {
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
o0["spaces"] = v0
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
v2 = rDyn()
o2["id"] = v2
break
}
case 1: {
let v2
v2 = rDyn()
o2["kind"] = v2
break
}
case 2: {
let v2
v2 = rStr()
o2["name"] = v2
break
}
case 3: {
let v2
v2 = rStr()
o2["remoteName"] = v2
break
}
case 4: {
let v2
v2 = rStr()
o2["folderName"] = v2
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
o2["localName"] = v2
break
}
case 6: {
let v2
v2 = rDyn()
o2["appearance"] = v2
break
}
case 7: {
let v2
v2 = rStr()
o2["cwd"] = v2
break
}
case 8: {
let v2
v2 = rDyn()
o2["defaultModelSelection"] = v2
break
}
case 9: {
let v2
v2 = rU8() === 1
o2["expanded"] = v2
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
o2["isPinned"] = v2
break
}
case 11: {
let v2
v2 = rDyn()
o2["spaceId"] = v2
break
}
case 12: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
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
o2["scripts"] = v2
break
}
case 15: {
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
o2["additionalFolders"] = v2
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
o0["projects"] = v0
break
}
case 3: {
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
v2 = rDyn()
o2["id"] = v2
break
}
case 1: {
let v2
v2 = rDyn()
o2["projectId"] = v2
break
}
case 2: {
let v2
v2 = rStr()
o2["title"] = v2
break
}
case 3: {
let v2
v2 = rDyn()
o2["modelSelection"] = v2
break
}
case 4: {
let v2
v2 = rDyn()
o2["interactionMode"] = v2
break
}
case 5: {
let v2
v2 = rDyn()
o2["envMode"] = v2
break
}
case 6: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["branch"] = v2
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
o2["worktreePath"] = v2
break
}
case 8: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["workingDirectory"] = v2
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
o2["associatedWorktreePath"] = v2
break
}
case 10: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["associatedWorktreeBranch"] = v2
break
}
case 11: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["associatedWorktreeRef"] = v2
break
}
case 12: {
let v2
v2 = rDyn()
o2["session"] = v2
break
}
case 13: {
let v2
v2 = rStr()
o2["createdAt"] = v2
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
o2["archivedAt"] = v2
break
}
case 15: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["settledAt"] = v2
break
}
case 16: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["snoozedUntil"] = v2
break
}
case 17: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["snoozeReminderAt"] = v2
break
}
case 18: {
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
case 19: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rU8() === 1
}
}
o2["isPinned"] = v2
break
}
case 20: {
let v2
v2 = rDyn()
o2["latestTurn"] = v2
break
}
case 21: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["lastVisitedAt"] = v2
break
}
case 22: {
let v2
v2 = rDyn()
o2["parentThreadId"] = v2
break
}
case 23: {
let v2
v2 = rDyn()
o2["creationSource"] = v2
break
}
case 24: {
let v2
v2 = rDyn()
o2["sourceThreadId"] = v2
break
}
case 25: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["subagentAgentId"] = v2
break
}
case 26: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["subagentNickname"] = v2
break
}
case 27: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["subagentRole"] = v2
break
}
case 28: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["latestUserMessageAt"] = v2
break
}
case 29: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["latestHumanMessageAt"] = v2
break
}
case 30: {
let v2
v2 = rU8() === 1
o2["hasPendingApprovals"] = v2
break
}
case 31: {
let v2
v2 = rU8() === 1
o2["hasPendingUserInput"] = v2
break
}
case 32: {
let v2
v2 = rU8() === 1
o2["hasActionableProposedPlan"] = v2
break
}
case 33: {
let v2
v2 = rU8() === 1
o2["hasLiveTailWork"] = v2
break
}
case 34: {
let v2
v2 = rF64()
o2["pendingBackgroundWorkCount"] = v2
break
}
case 35: {
let v2
v2 = rDyn()
o2["forkSourceThreadId"] = v2
break
}
case 36: {
let v2
v2 = rDyn()
o2["sidechatSourceThreadId"] = v2
break
}
case 37: {
let v2
v2 = rDyn()
o2["sidechatContext"] = v2
break
}
case 38: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["sidechatLastActivityAt"] = v2
break
}
case 39: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["sidechatExpiredAt"] = v2
break
}
case 40: {
let v2
v2 = rDyn()
o2["handoff"] = v2
break
}
case 41: {
let v2
v2 = rDyn()
o2["lastKnownPr"] = v2
break
}
case 42: {
let v2
v2 = rDyn()
o2["pullRequests"] = v2
break
}
case 43: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rStr()
}
}
o2["latestReplyPreview"] = v2
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
o0["sidebarThreadSummaryById"] = v0
break
}
case 4: {
let v0
v0 = rU8() === 1
o0["threadsHydrated"] = v0
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
o0["threadIds"] = v0
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
const o2 = {}
for (let i2 = 0; i2 < n2; i2++) {
const k2 = rStr()
let v2
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
v3 = rU8() === 1
}
}
o3["isProjectImport"] = v3
break
}
case 1: {
let v3
v3 = rDyn()
o3["id"] = v3
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
o3["codexThreadId"] = v3
break
}
case 3: {
let v3
v3 = rDyn()
o3["projectId"] = v3
break
}
case 4: {
let v3
v3 = rStr()
o3["title"] = v3
break
}
case 5: {
let v3
v3 = rDyn()
o3["modelSelection"] = v3
break
}
case 6: {
let v3
v3 = rDyn()
o3["runtimeMode"] = v3
break
}
case 7: {
let v3
v3 = rDyn()
o3["interactionMode"] = v3
break
}
case 8: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["error"] = v3
break
}
case 9: {
let v3
v3 = rStr()
o3["createdAt"] = v3
break
}
case 10: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["archivedAt"] = v3
break
}
case 11: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["settledAt"] = v3
break
}
case 12: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["snoozedUntil"] = v3
break
}
case 13: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["snoozeReminderAt"] = v3
break
}
case 14: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["updatedAt"] = v3
break
}
case 15: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rU8() === 1
}
}
o3["isPinned"] = v3
break
}
case 16: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
v5 = rDyn()
a5[i5] = v5
}
v3 = a5
}
}
}
o3["pinnedMessages"] = v3
break
}
case 17: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["notes"] = v3
break
}
case 18: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["goal"] = v3
break
}
case 19: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["goalStartedAt"] = v3
break
}
case 20: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["goalPausedAt"] = v3
break
}
case 21: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
v5 = rDyn()
a5[i5] = v5
}
v3 = a5
}
}
}
o3["goalAchievements"] = v3
break
}
case 22: {
let v3
v3 = rDyn()
o3["parentThreadId"] = v3
break
}
case 23: {
let v3
v3 = rDyn()
o3["creationSource"] = v3
break
}
case 24: {
let v3
v3 = rDyn()
o3["sourceThreadId"] = v3
break
}
case 25: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["subagentAgentId"] = v3
break
}
case 26: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["subagentNickname"] = v3
break
}
case 27: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["subagentRole"] = v3
break
}
case 28: {
let v3
v3 = rDyn()
o3["forkSourceThreadId"] = v3
break
}
case 29: {
let v3
v3 = rDyn()
o3["sidechatSourceThreadId"] = v3
break
}
case 30: {
let v3
v3 = rDyn()
o3["sidechatContext"] = v3
break
}
case 31: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["sidechatLastActivityAt"] = v3
break
}
case 32: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["sidechatExpiredAt"] = v3
break
}
case 33: {
let v3
v3 = rDyn()
o3["handoff"] = v3
break
}
case 34: {
let v3
v3 = rDyn()
o3["usageLimitRecovery"] = v3
break
}
case 35: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["usageLimitRecoverySequence"] = v3
break
}
case 36: {
let v3
v3 = rDyn()
o3["claudeCacheReview"] = v3
break
}
case 37: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["claudeCacheReviewSequence"] = v3
break
}
case 38: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["snoozeSequence"] = v3
break
}
case 39: {
let v3
v3 = rDyn()
o3["lastKnownPr"] = v3
break
}
case 40: {
let v3
v3 = rDyn()
o3["pullRequests"] = v3
break
}
case 41: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["latestUserMessageAt"] = v3
break
}
case 42: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["latestHumanMessageAt"] = v3
break
}
case 43: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rU8() === 1
}
}
o3["hasPendingApprovals"] = v3
break
}
case 44: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rU8() === 1
}
}
o3["hasPendingUserInput"] = v3
break
}
case 45: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rU8() === 1
}
}
o3["hasActionableProposedPlan"] = v3
break
}
case 46: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rF64()
}
}
o3["pendingBackgroundTaskCount"] = v3
break
}
case 47: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["latestReplyPreview"] = v3
break
}
case 48: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
if (rU8() === 0) v3 = null
else {
const n5 = rU32()
const a5 = new Array(n5)
for (let i5 = 0; i5 < n5; i5++) {
let v5
v5 = rDyn()
a5[i5] = v5
}
v3 = a5
}
}
}
o3["pendingInteractions"] = v3
break
}
case 49: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["lastVisitedAt"] = v3
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
o2[k2] = v2
}
v0 = o2
}
}
}
o0["threadShellById"] = v0
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
const o2 = {}
for (let i2 = 0; i2 < n2; i2++) {
const k2 = rStr()
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
{
const o4 = {}
const n4 = rU8()
for (let j4 = 0; j4 < n4; j4++) {
switch (rU8()) {
case 0: {
let v4
v4 = rDyn()
o4["provider"] = v4
break
}
case 1: {
let v4
v4 = rDyn()
o4["runtimeMode"] = v4
break
}
case 2: {
let v4
v4 = rDyn()
o4["providerInstanceId"] = v4
break
}
case 3: {
let v4
v4 = rDyn()
o4["status"] = v4
break
}
case 4: {
let v4
v4 = rDyn()
o4["activeTurnId"] = v4
break
}
case 5: {
let v4
v4 = rDyn()
o4["createdAt"] = v4
break
}
case 6: {
let v4
v4 = rDyn()
o4["updatedAt"] = v4
break
}
case 7: {
let v4
v4 = rDyn()
o4["lastError"] = v4
break
}
case 8: {
let v4
v4 = rDyn()
o4["orchestrationStatus"] = v4
break
}
default: {
const k4 = rStr()
o4[k4] = rDyn()
}
}
}
v2 = o4
}
}
}
o2[k2] = v2
}
v0 = o2
}
}
}
o0["threadSessionById"] = v0
break
}
case 8: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const o2 = {}
for (let i2 = 0; i2 < n2; i2++) {
const k2 = rStr()
let v2
{
const o3 = {}
const n3 = rU8()
for (let j3 = 0; j3 < n3; j3++) {
switch (rU8()) {
case 0: {
let v3
v3 = rDyn()
o3["latestTurn"] = v3
break
}
case 1: {
let v3
v3 = rDyn()
o3["pendingSourceProposedPlan"] = v3
break
}
case 2: {
let v3
v3 = rDyn()
o3["pendingTurnStartMessageId"] = v3
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
o2[k2] = v2
}
v0 = o2
}
}
}
o0["threadTurnStateById"] = v0
break
}
case 9: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const o2 = {}
for (let i2 = 0; i2 < n2; i2++) {
const k2 = rStr()
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
o2[k2] = v2
}
v0 = o2
}
}
}
o0["messageIdsByThreadId"] = v0
break
}
case 10: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const o2 = {}
for (let i2 = 0; i2 < n2; i2++) {
const k2 = rStr()
let v2
if (rU8() === 0) v2 = null
else {
const n3 = rU32()
const o3 = {}
for (let i3 = 0; i3 < n3; i3++) {
const k3 = rStr()
let v3
v3 = rDyn()
o3[k3] = v3
}
v2 = o3
}
o2[k2] = v2
}
v0 = o2
}
}
}
o0["messageByThreadId"] = v0
break
}
case 11: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const o2 = {}
for (let i2 = 0; i2 < n2; i2++) {
const k2 = rStr()
let v2
if (rU8() === 0) v2 = null
else {
const n3 = rU32()
const a3 = new Array(n3)
for (let i3 = 0; i3 < n3; i3++) {
let v3
v3 = rStr()
a3[i3] = v3
}
v2 = a3
}
o2[k2] = v2
}
v0 = o2
}
}
}
o0["activityIdsByThreadId"] = v0
break
}
case 12: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const o2 = {}
for (let i2 = 0; i2 < n2; i2++) {
const k2 = rStr()
let v2
if (rU8() === 0) v2 = null
else {
const n3 = rU32()
const o3 = {}
for (let i3 = 0; i3 < n3; i3++) {
const k3 = rStr()
let v3
v3 = rDyn()
o3[k3] = v3
}
v2 = o3
}
o2[k2] = v2
}
v0 = o2
}
}
}
o0["activityByThreadId"] = v0
break
}
case 13: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const o2 = {}
for (let i2 = 0; i2 < n2; i2++) {
const k2 = rStr()
let v2
if (rU8() === 0) v2 = null
else {
const n3 = rU32()
const a3 = new Array(n3)
for (let i3 = 0; i3 < n3; i3++) {
let v3
v3 = rStr()
a3[i3] = v3
}
v2 = a3
}
o2[k2] = v2
}
v0 = o2
}
}
}
o0["proposedPlanIdsByThreadId"] = v0
break
}
case 14: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const o2 = {}
for (let i2 = 0; i2 < n2; i2++) {
const k2 = rStr()
let v2
if (rU8() === 0) v2 = null
else {
const n3 = rU32()
const o3 = {}
for (let i3 = 0; i3 < n3; i3++) {
const k3 = rStr()
let v3
v3 = rDyn()
o3[k3] = v3
}
v2 = o3
}
o2[k2] = v2
}
v0 = o2
}
}
}
o0["proposedPlanByThreadId"] = v0
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
const o2 = {}
for (let i2 = 0; i2 < n2; i2++) {
const k2 = rStr()
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
o2[k2] = v2
}
v0 = o2
}
}
}
o0["turnDiffIdsByThreadId"] = v0
break
}
case 16: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const o2 = {}
for (let i2 = 0; i2 < n2; i2++) {
const k2 = rStr()
let v2
if (rU8() === 0) v2 = null
else {
const n3 = rU32()
const o3 = {}
for (let i3 = 0; i3 < n3; i3++) {
const k3 = rStr()
let v3
v3 = rDyn()
o3[k3] = v3
}
v2 = o3
}
o2[k2] = v2
}
v0 = o2
}
}
}
o0["turnDiffSummaryByThreadId"] = v0
break
}
case 17: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const o2 = {}
for (let i2 = 0; i2 < n2; i2++) {
const k2 = rStr()
let v2
v2 = rStr()
o2[k2] = v2
}
v0 = o2
}
}
}
o0["threadDetailSyncById"] = v0
break
}
case 18: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const o2 = {}
for (let i2 = 0; i2 < n2; i2++) {
const k2 = rStr()
let v2
v2 = rF64()
o2[k2] = v2
}
v0 = o2
}
}
}
o0["threadDetailRevisionSequenceById"] = v0
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
const o2 = {}
for (let i2 = 0; i2 < n2; i2++) {
const k2 = rStr()
let v2
v2 = rF64()
o2[k2] = v2
}
v0 = o2
}
}
}
o0["deletedProjectIdsById"] = v0
break
}
case 20: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
if (rU8() === 0) v0 = null
else {
const n2 = rU32()
const o2 = {}
for (let i2 = 0; i2 < n2; i2++) {
const k2 = rStr()
let v2
v2 = rF64()
o2[k2] = v2
}
v0 = o2
}
}
}
o0["deletedThreadIdsById"] = v0
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
module.exports.applyOrchestrationEventsHotPath = function applyOrchestrationEventsHotPath (a0, a1, a2) {
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
