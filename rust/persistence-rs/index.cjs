// Loads the native module built by `rust-anything build`.
const native = require('./persistence_rs.node')

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
  migrationTrackerFingerprint: [[1], 'raw', false],
  classifyAccountMigrationLineage: [[1, 1], 'json', false],
  validateAccountMigrationSchema: [[1, 1, 1], 'raw', false],
  acquireDatabaseLifecycleLock: [[0], 'json', false],
  releaseDatabaseLifecycleLock: [[1], 'json', false],
  toPersistenceDecodeError: [[0], 'json', false],
  toPersistenceDecodeCauseError: [[0], 'json', false],
  isPersistenceError: [[1], 'json', false],
  estimateMigrationBackupRequiredBytes: [[0], 'json', false],
  pruneMigrationBackups: [[0, 1], 'json', false],
  createMigrationBackup: [[0, 1], 'json', false],
  inspectSqliteMigrationBackup: [[0], 'json', false],
  inspectCompletedMigrationBackupForSchemaTooNew: [[0, 1], 'json', false],
  reclaimOrphanedMigrationArtifacts: [[0], 'json', false],
  requireNoPendingMigrationRecovery: [[0], 'json', false],
  inspectPendingMigrationRecovery: [[0], 'json', false],
  restoreMarkedMigrationBackup: [[0, 1], 'json', false],
  createMigrationDivergenceConsentChallenge: [[0, 1], 'json', false],
  createMigrationSchemaTooNewStartupBlockError: [[0, 1], 'json', false],
  makeMigrationLoader: [[1], 'json', false],
  isReleasedMainMigrationGap: [[1], 'json', false],
  planAccountMigrationRepair: [[1], 'json', false],
  planLegacyMigration32Rename: [[1], 'json', false],
  findFirstMigrationLineageDivergence: [[1, 1], 'json', false],
  planMigrationLineageAliasRepairs: [[1], 'json', false],
  runMigrations: [[1], 'json', false],
  reclaimFreedSqlitePages: [[1], 'json', false],
  reclaimLegacySqliteDatabase: [[0], 'json', false],
  notifyAfterCommit: [[1], 'json', false],
  selectMessageTextChunks: [[1, 0, 1], 'json', false],
  selectSegmentEndedAt: [[1, 0], 'json', false],
  joinMessageTextChunks: [[1], 'json', false],
  makeMessageTextChunks: [[1], 'json', false],
  normalizeLegacyModelSelection: [[1], 'json', false],
  normalizePersistedModelSelection: [[1, 1], 'json', false],
  orchestrationMessageFromStoredMessage: [[1], 'json', false],
  projectionThreadMessageFromRow: [[1], 'json', false],
  orchestrationMessageFromProjectionRow: [[1], 'json', false],
  resolveStoredPullRequestLinks: [[1], 'json', false],
  resolveSqliteMemoryBudget: [[1], 'json', false],
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

// `migrationTrackerFingerprint`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.migrationTrackerFingerprint
const call = native.migrationTrackerFingerprintWire
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
module.exports.migrationTrackerFingerprint = function migrationTrackerFingerprint (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `classifyAccountMigrationLineage`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.classifyAccountMigrationLineage
const call = native.classifyAccountMigrationLineageWire
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
module.exports.classifyAccountMigrationLineage = function classifyAccountMigrationLineage (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `validateAccountMigrationSchema`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.validateAccountMigrationSchema
const call = native.validateAccountMigrationSchemaWire
const encode = (a0, a1, a2) => {
at = 0
if (!wDyn(a0)) return false
if (!wDyn(a1)) return false
if (!wDyn(a2)) return false
return true
}
const decode = () => {
let t
return t
}
module.exports.validateAccountMigrationSchema = function validateAccountMigrationSchema (a0, a1, a2) {
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `acquireDatabaseLifecycleLock`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.acquireDatabaseLifecycleLock
const call = native.acquireDatabaseLifecycleLockWire
const encode = (a0) => {
at = 0
if (typeof a0 !== 'string') return false
wStr(a0)
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.acquireDatabaseLifecycleLock = function acquireDatabaseLifecycleLock (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of acquireDatabaseLifecycleLock has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `releaseDatabaseLifecycleLock`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.releaseDatabaseLifecycleLock
const call = native.releaseDatabaseLifecycleLockWire
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
module.exports.releaseDatabaseLifecycleLock = function releaseDatabaseLifecycleLock (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `toPersistenceDecodeError`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.toPersistenceDecodeError
const call = native.toPersistenceDecodeErrorWire
const encode = (a0) => {
at = 0
if (typeof a0 !== 'string') return false
wStr(a0)
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.toPersistenceDecodeError = function toPersistenceDecodeError (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of toPersistenceDecodeError has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `toPersistenceDecodeCauseError`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.toPersistenceDecodeCauseError
const call = native.toPersistenceDecodeCauseErrorWire
const encode = (a0) => {
at = 0
if (typeof a0 !== 'string') return false
wStr(a0)
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.toPersistenceDecodeCauseError = function toPersistenceDecodeCauseError (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of toPersistenceDecodeCauseError has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `isPersistenceError`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.isPersistenceError
const call = native.isPersistenceErrorWire
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
module.exports.isPersistenceError = function isPersistenceError (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `estimateMigrationBackupRequiredBytes`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.estimateMigrationBackupRequiredBytes
const call = native.estimateMigrationBackupRequiredBytesWire
const encode = (a0) => {
at = 0
if (typeof a0 !== 'string') return false
wStr(a0)
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.estimateMigrationBackupRequiredBytes = function estimateMigrationBackupRequiredBytes (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of estimateMigrationBackupRequiredBytes has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `pruneMigrationBackups`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.pruneMigrationBackups
const call = native.pruneMigrationBackupsWire
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
module.exports.pruneMigrationBackups = function pruneMigrationBackups (a0, a1) {
  if (illFormed(a0)) throw new TypeError('argument 1 of pruneMigrationBackups has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `createMigrationBackup`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.createMigrationBackup
const call = native.createMigrationBackupWire
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
module.exports.createMigrationBackup = function createMigrationBackup (a0, a1) {
  if (illFormed(a0)) throw new TypeError('argument 1 of createMigrationBackup has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `inspectSqliteMigrationBackup`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.inspectSqliteMigrationBackup
const call = native.inspectSqliteMigrationBackupWire
const encode = (a0) => {
at = 0
if (typeof a0 !== 'string') return false
wStr(a0)
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.inspectSqliteMigrationBackup = function inspectSqliteMigrationBackup (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of inspectSqliteMigrationBackup has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `inspectCompletedMigrationBackupForSchemaTooNew`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.inspectCompletedMigrationBackupForSchemaTooNew
const call = native.inspectCompletedMigrationBackupForSchemaTooNewWire
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
module.exports.inspectCompletedMigrationBackupForSchemaTooNew = function inspectCompletedMigrationBackupForSchemaTooNew (a0, a1) {
  if (illFormed(a0)) throw new TypeError('argument 1 of inspectCompletedMigrationBackupForSchemaTooNew has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `reclaimOrphanedMigrationArtifacts`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.reclaimOrphanedMigrationArtifacts
const call = native.reclaimOrphanedMigrationArtifactsWire
const encode = (a0) => {
at = 0
if (typeof a0 !== 'string') return false
wStr(a0)
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.reclaimOrphanedMigrationArtifacts = function reclaimOrphanedMigrationArtifacts (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of reclaimOrphanedMigrationArtifacts has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `requireNoPendingMigrationRecovery`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.requireNoPendingMigrationRecovery
const call = native.requireNoPendingMigrationRecoveryWire
const encode = (a0) => {
at = 0
if (typeof a0 !== 'string') return false
wStr(a0)
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.requireNoPendingMigrationRecovery = function requireNoPendingMigrationRecovery (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of requireNoPendingMigrationRecovery has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `inspectPendingMigrationRecovery`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.inspectPendingMigrationRecovery
const call = native.inspectPendingMigrationRecoveryWire
const encode = (a0) => {
at = 0
if (typeof a0 !== 'string') return false
wStr(a0)
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.inspectPendingMigrationRecovery = function inspectPendingMigrationRecovery (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of inspectPendingMigrationRecovery has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `restoreMarkedMigrationBackup`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.restoreMarkedMigrationBackup
const call = native.restoreMarkedMigrationBackupWire
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
module.exports.restoreMarkedMigrationBackup = function restoreMarkedMigrationBackup (a0, a1) {
  if (illFormed(a0)) throw new TypeError('argument 1 of restoreMarkedMigrationBackup has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `createMigrationDivergenceConsentChallenge`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.createMigrationDivergenceConsentChallenge
const call = native.createMigrationDivergenceConsentChallengeWire
const encode = (a0, a1) => {
at = 0
if (typeof a0 !== 'string') return false
wStr(a0)
if (!wDyn(a1)) return false
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
o1["version"] = v1
break
}
case 1: {
let v1
v1 = rStr()
o1["databasePath"] = v1
break
}
case 2: {
let v1
v1 = rStr()
o1["backupDirectory"] = v1
break
}
case 3: {
let v1
v1 = rStr()
o1["sourceVersion"] = v1
break
}
case 4: {
let v1
v1 = rF64()
o1["targetVersion"] = v1
break
}
case 5: {
let v1
v1 = rF64()
o1["firstDivergedId"] = v1
break
}
case 6: {
let v1
v1 = rStr()
o1["expectedName"] = v1
break
}
case 7: {
let v1
v1 = rStr()
o1["recordedName"] = v1
break
}
case 8: {
let v1
v1 = rF64()
o1["highWaterMark"] = v1
break
}
case 9: {
let v1
v1 = rStr()
o1["lineageFingerprint"] = v1
break
}
case 10: {
let v1
v1 = rStr()
o1["consentToken"] = v1
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
module.exports.createMigrationDivergenceConsentChallenge = function createMigrationDivergenceConsentChallenge (a0, a1) {
  if (illFormed(a0)) throw new TypeError('argument 1 of createMigrationDivergenceConsentChallenge has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `createMigrationSchemaTooNewStartupBlockError`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.createMigrationSchemaTooNewStartupBlockError
const call = native.createMigrationSchemaTooNewStartupBlockErrorWire
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
module.exports.createMigrationSchemaTooNewStartupBlockError = function createMigrationSchemaTooNewStartupBlockError (a0, a1) {
  if (illFormed(a0)) throw new TypeError('argument 1 of createMigrationSchemaTooNewStartupBlockError has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `makeMigrationLoader`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.makeMigrationLoader
const call = native.makeMigrationLoaderWire
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
module.exports.makeMigrationLoader = function makeMigrationLoader (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `isReleasedMainMigrationGap`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.isReleasedMainMigrationGap
const call = native.isReleasedMainMigrationGapWire
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
module.exports.isReleasedMainMigrationGap = function isReleasedMainMigrationGap (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `planAccountMigrationRepair`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.planAccountMigrationRepair
const call = native.planAccountMigrationRepairWire
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
module.exports.planAccountMigrationRepair = function planAccountMigrationRepair (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `planLegacyMigration32Rename`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.planLegacyMigration32Rename
const call = native.planLegacyMigration32RenameWire
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
module.exports.planLegacyMigration32Rename = function planLegacyMigration32Rename (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `findFirstMigrationLineageDivergence`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.findFirstMigrationLineageDivergence
const call = native.findFirstMigrationLineageDivergenceWire
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
module.exports.findFirstMigrationLineageDivergence = function findFirstMigrationLineageDivergence (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `planMigrationLineageAliasRepairs`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.planMigrationLineageAliasRepairs
const call = native.planMigrationLineageAliasRepairsWire
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
module.exports.planMigrationLineageAliasRepairs = function planMigrationLineageAliasRepairs (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `runMigrations`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.runMigrations
const call = native.runMigrationsWire
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
module.exports.runMigrations = function runMigrations (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `reclaimFreedSqlitePages`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.reclaimFreedSqlitePages
const call = native.reclaimFreedSqlitePagesWire
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
module.exports.reclaimFreedSqlitePages = function reclaimFreedSqlitePages (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `reclaimLegacySqliteDatabase`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.reclaimLegacySqliteDatabase
const call = native.reclaimLegacySqliteDatabaseWire
const encode = (a0) => {
at = 0
if (typeof a0 !== 'string') return false
wStr(a0)
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.reclaimLegacySqliteDatabase = function reclaimLegacySqliteDatabase (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of reclaimLegacySqliteDatabase has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `notifyAfterCommit`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.notifyAfterCommit
const call = native.notifyAfterCommitWire
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
module.exports.notifyAfterCommit = function notifyAfterCommit (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `selectMessageTextChunks`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.selectMessageTextChunks
const call = native.selectMessageTextChunksWire
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
module.exports.selectMessageTextChunks = function selectMessageTextChunks (a0, a1, a2) {
  if (illFormed(a1)) throw new TypeError('argument 2 of selectMessageTextChunks has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `selectSegmentEndedAt`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.selectSegmentEndedAt
const call = native.selectSegmentEndedAtWire
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
module.exports.selectSegmentEndedAt = function selectSegmentEndedAt (a0, a1) {
  if (illFormed(a1)) throw new TypeError('argument 2 of selectSegmentEndedAt has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `joinMessageTextChunks`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.joinMessageTextChunks
const call = native.joinMessageTextChunksWire
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
module.exports.joinMessageTextChunks = function joinMessageTextChunks (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `encodeMessageTextFallback`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.encodeMessageTextFallback
const call = native.encodeMessageTextFallbackWire
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
module.exports.encodeMessageTextFallback = function encodeMessageTextFallback (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of encodeMessageTextFallback has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `makeMessageTextChunks`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.makeMessageTextChunks
const call = native.makeMessageTextChunksWire
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
module.exports.makeMessageTextChunks = function makeMessageTextChunks (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `normalizeLegacyModelSelection`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.normalizeLegacyModelSelection
const call = native.normalizeLegacyModelSelectionWire
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
module.exports.normalizeLegacyModelSelection = function normalizeLegacyModelSelection (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `normalizePersistedModelSelection`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.normalizePersistedModelSelection
const call = native.normalizePersistedModelSelectionWire
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
module.exports.normalizePersistedModelSelection = function normalizePersistedModelSelection (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `orchestrationMessageFromStoredMessage`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.orchestrationMessageFromStoredMessage
const call = native.orchestrationMessageFromStoredMessageWire
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
module.exports.orchestrationMessageFromStoredMessage = function orchestrationMessageFromStoredMessage (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `projectionThreadMessageFromRow`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.projectionThreadMessageFromRow
const call = native.projectionThreadMessageFromRowWire
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
module.exports.projectionThreadMessageFromRow = function projectionThreadMessageFromRow (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `orchestrationMessageFromProjectionRow`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.orchestrationMessageFromProjectionRow
const call = native.orchestrationMessageFromProjectionRowWire
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
module.exports.orchestrationMessageFromProjectionRow = function orchestrationMessageFromProjectionRow (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `resolveStoredPullRequestLinks`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.resolveStoredPullRequestLinks
const call = native.resolveStoredPullRequestLinksWire
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
module.exports.resolveStoredPullRequestLinks = function resolveStoredPullRequestLinks (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `resolveSqliteMemoryBudget`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.resolveSqliteMemoryBudget
const call = native.resolveSqliteMemoryBudgetWire
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
o0["cacheSizePragma"] = v0
break
}
case 1: {
let v0
v0 = rF64()
o0["mmapSizeBytes"] = v0
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
module.exports.resolveSqliteMemoryBudget = function resolveSqliteMemoryBudget (a0) {
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
