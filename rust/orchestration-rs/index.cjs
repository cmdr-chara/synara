// Loads the native module built by `rust-anything build`.
const native = require('./orchestration_rs.node')

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
  toProjectorDecodeError: [[0], 'json', false],
  fingerprintOrchestrationCommand: [[1], 'json', false],
  threadHasInFlightTurn: [[1], 'raw', false],
  checkpointRevertActiveTurnDetail: [[1], 'raw', false],
  threadHasCheckpointRevertInProgress: [[1], 'raw', false],
  checkpointRevertInProgressDetail: [[1], 'raw', false],
  checkpointRevertDeleteInProgressDetail: [[1], 'raw', false],
  findThreadById: [[1, 1], 'json', false],
  findProjectById: [[1, 1], 'json', false],
  findSpaceById: [[1, 1], 'json', false],
  listActiveSpaces: [[1], 'json', false],
  requireSpace: [[1], 'json', false],
  requireSpaceAbsent: [[1], 'json', false],
  requireSpaceNameAvailable: [[1], 'json', false],
  isLegacyHomeChatContainerRow: [[1], 'raw', false],
  isOrdinaryProjectRow: [[1], 'raw', false],
  requireSpaceAssignableProject: [[1], 'json', false],
  listActiveProjectsByWorkspaceRoot: [[1, 0, 1], 'json', false],
  listThreadsByProjectId: [[1, 1], 'json', false],
  requireProject: [[1], 'json', false],
  requireProjectAbsent: [[1], 'json', false],
  requireValidProjectFolders: [[1], 'json', false],
  requireProjectWorkspaceRootAvailable: [[1], 'json', false],
  requireProjectHasNoThreads: [[1], 'json', false],
  requireThread: [[1], 'json', false],
  requireApprovalNotResponded: [[1], 'json', false],
  requireThreadAbsent: [[1], 'json', false],
  requireThreadArchived: [[1], 'json', false],
  requireThreadNotArchived: [[1], 'json', false],
  threadResumePreconditionViolation: [[1, 1], 'json', false],
  threadResumePreconditionDetail: [[1, 0], 'json', false],
  withoutRemoteResumeComputerGrant: [[1, 0], 'json', false],
  resolveForkSourceCutoff: [[1], 'json', false],
  buildForkThreadTitle: [[1, 1], 'raw', false],
  resolveStableMessageTurnId: [[1], 'json', false],
  usesReservedCommandAdmission: [[1], 'raw', false],
  isQuiescingCommandAdmissible: [[1], 'raw', false],
  orchestrationCommandLane: [[1], 'raw', false],
  isExpiredSidechat: [[1], 'raw', false],
  settleTurnStateFromSession: [[1, 0], 'json', false],
  maxIso: [[0, 0], 'json', false],
  deriveTurnStartModelSelection: [[1], 'json', false],
  countNativeTurnStartMessages: [[1], 'raw', false],
  canAdoptFirstTurnProvider: [[1], 'raw', false],
  deriveTurnStartSession: [[1], 'json', false],
  relocateProjectPath: [[0, 0, 0, 1], 'json', false],
  providerWorkspaceChanged: [[0, 0, 1], 'raw', false],
  isProviderIntentEvent: [[1], 'raw', false],
  isReplaySafeClaimedProviderIntent: [[1], 'raw', false],
  isProviderSideEffectIntent: [[1], 'raw', false],
  isClaimedProviderIntent: [[1], 'raw', false],
  isQuarantineExemptProviderIntent: [[1], 'raw', false],
  isUnsettledPendingInteraction: [[1], 'raw', false],
  pendingInteractionRequestKind: [[1], 'raw', false],
  buildStalePendingRequestSettlementCommand: [[1], 'json', false],
  nativeImportId: [[1, 1], 'json', false],
  makeProjectImportDestinations: [[1], 'json', false],
  threadArchiveFileName: [[1], 'raw', false],
  listImportedHandoffMessages: [[1], 'json', false],
  listImportedForkMessages: [[1], 'json', false],
  hasNativeHandoffMessages: [[1], 'raw', false],
  hasNativeAssistantMessagesBefore: [[1, 0], 'raw', false],
  listPriorTranscriptMessages: [[1, 0], 'json', false],
  buildHandoffBootstrapText: [[1, 1], 'json', false],
  buildPriorTranscriptBootstrapText: [[1, 0, 1], 'json', false],
  buildPriorTranscriptBootstrap: [[1, 0, 1], 'json', false],
  buildForkBootstrapText: [[1, 1], 'json', false],
  claudeHistoricalSessionChildEnvironment: [[1], 'json', false],
  readClaudeSessionMessagePageInEnvironment: [[1], 'json', true],
  listClaudeSessionsInEnvironment: [[1], 'json', true],
  readClaudeSessionMessagesInEnvironment: [[1], 'json', true],
  claudeHistoricalSessionEnvironment: [[1, 1], 'json', false],
  resolveImportedThreadProviderOptionsForSettings: [[1, 1], 'json', false],
  mapCodexSnapshotMessages: [[1], 'json', false],
  mapClaudeSessionMessages: [[1], 'json', false],
  mapOpenCodeSnapshotMessages: [[1], 'json', false],
  mapFactorySnapshotMessages: [[1], 'json', false],
  mapOmpSnapshotMessages: [[1], 'json', false],
  projectImportKey: [[1, 1], 'raw', false],
  resolveProjectImportSources: [[1, 1, 1], 'json', false],
  buildProjectImportCatalog: [[1, 1], 'json', true],
  canProjectTurnModelSelectionForSession: [[1, 0], 'raw', false],
  createEmptyReadModel: [[0], 'json', false],
  projectEvent: [[1, 1], 'json', false],
  sanitizeProviderStartOptionsForPersistence: [[1], 'json', false],
  runtimePayloadRecord: [[1], 'json', false],
  runtimeTurnState: [[1], 'raw', false],
  providerRuntimeProgressKey: [[1], 'raw', false],
  providerRuntimeCommandProgressKey: [[1], 'json', false],
  coalesceProviderRuntimeProgress: [[1], 'json', false],
  providerActivityUpdateFingerprint: [[1], 'raw', false],
  providerActivityUpdateDedupeKey: [[1, 1, 1], 'json', false],
  detectCreatedPullRequestUrls: [[1], 'raw', false],
  buildQuitResumeRecord: [[1], 'json', false],
  buildQuitInterruptCommand: [[1], 'json', false],
  planQuitResumeTurns: [[1], 'json', false],
  persistQuitResumeRecord: [[1], 'json', false],
  clearQuitResumeRecord: [[0], 'json', false],
  persistCrashResumeRecord: [[1], 'json', false],
  readQuitResumeRecord: [[0], 'json', false],
  claimQuitResumeRecord: [[0], 'json', false],
  resumeQuitInterruptedChats: [[1], 'json', false],
  findRestartOrphanedTasks: [[1, 0], 'json', false],
  deriveThreadGitMetadataPatch: [[1], 'json', false],
  threadGoalFileNameFromReference: [[1], 'json', false],
  readMaterializedThreadGoalText: [[1], 'json', true],
  materializeThreadGoalFile: [[1], 'json', true],
  pruneThreadGoalFiles: [[1], 'json', true],
  discardMaterializedThreadGoalFile: [[1], 'json', true],
  formatThreadLatestReplyPreview: [[1], 'json', false],
  shouldApplyThreadsProjection: [[1], 'raw', false],
  shouldApplyDeferredThreadShellSummary: [[1], 'raw', false],
  shouldPublishThreadShellForEvent: [[1], 'raw', false],
  projectProviderRuntimeActivities: [[1, 0], 'json', false],
  findImportGitWorkspace: [[0], 'json', true],
  resolveProjectImportSourceHome: [[1], 'json', true],
  planRestartTurnReconciliation: [[1], 'json', false],
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
// projectImportKey is variadic in the source (`...parts`); arity is lost through
// the encoded wire, so the real argument count is passed explicitly.
module.exports.projectImportKey = function projectImportKey () {
  return native.projectImportKey(encode(arguments.length), encode(Array.from(arguments)))
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

// `threadModelSelectionChangedDetail`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.threadModelSelectionChangedDetail
const call = native.threadModelSelectionChangedDetailWire
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
module.exports.threadModelSelectionChangedDetail = function threadModelSelectionChangedDetail (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of threadModelSelectionChangedDetail has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `toProjectorDecodeError`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.toProjectorDecodeError
const call = native.toProjectorDecodeErrorWire
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
module.exports.toProjectorDecodeError = function toProjectorDecodeError (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of toProjectorDecodeError has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `fingerprintOrchestrationCommand`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.fingerprintOrchestrationCommand
const call = native.fingerprintOrchestrationCommandWire
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
o0["version"] = v0
break
}
case 1: {
let v0
v0 = rStr()
o0["value"] = v0
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
module.exports.fingerprintOrchestrationCommand = function fingerprintOrchestrationCommand (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `threadHasInFlightTurn`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.threadHasInFlightTurn
const call = native.threadHasInFlightTurnWire
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
module.exports.threadHasInFlightTurn = function threadHasInFlightTurn (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `checkpointRevertActiveTurnDetail`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.checkpointRevertActiveTurnDetail
const call = native.checkpointRevertActiveTurnDetailWire
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
module.exports.checkpointRevertActiveTurnDetail = function checkpointRevertActiveTurnDetail (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `threadHasCheckpointRevertInProgress`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.threadHasCheckpointRevertInProgress
const call = native.threadHasCheckpointRevertInProgressWire
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
module.exports.threadHasCheckpointRevertInProgress = function threadHasCheckpointRevertInProgress (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `checkpointRevertInProgressDetail`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.checkpointRevertInProgressDetail
const call = native.checkpointRevertInProgressDetailWire
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
module.exports.checkpointRevertInProgressDetail = function checkpointRevertInProgressDetail (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `checkpointRevertDeleteInProgressDetail`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.checkpointRevertDeleteInProgressDetail
const call = native.checkpointRevertDeleteInProgressDetailWire
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
module.exports.checkpointRevertDeleteInProgressDetail = function checkpointRevertDeleteInProgressDetail (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `findThreadById`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.findThreadById
const call = native.findThreadByIdWire
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
module.exports.findThreadById = function findThreadById (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `findProjectById`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.findProjectById
const call = native.findProjectByIdWire
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
module.exports.findProjectById = function findProjectById (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `findSpaceById`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.findSpaceById
const call = native.findSpaceByIdWire
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
module.exports.findSpaceById = function findSpaceById (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `listActiveSpaces`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.listActiveSpaces
const call = native.listActiveSpacesWire
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
module.exports.listActiveSpaces = function listActiveSpaces (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `requireSpace`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.requireSpace
const call = native.requireSpaceWire
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
module.exports.requireSpace = function requireSpace (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `requireSpaceAbsent`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.requireSpaceAbsent
const call = native.requireSpaceAbsentWire
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
module.exports.requireSpaceAbsent = function requireSpaceAbsent (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `requireSpaceNameAvailable`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.requireSpaceNameAvailable
const call = native.requireSpaceNameAvailableWire
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
module.exports.requireSpaceNameAvailable = function requireSpaceNameAvailable (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `isLegacyHomeChatContainerRow`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.isLegacyHomeChatContainerRow
const call = native.isLegacyHomeChatContainerRowWire
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
module.exports.isLegacyHomeChatContainerRow = function isLegacyHomeChatContainerRow (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `isOrdinaryProjectRow`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.isOrdinaryProjectRow
const call = native.isOrdinaryProjectRowWire
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
module.exports.isOrdinaryProjectRow = function isOrdinaryProjectRow (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `requireSpaceAssignableProject`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.requireSpaceAssignableProject
const call = native.requireSpaceAssignableProjectWire
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
module.exports.requireSpaceAssignableProject = function requireSpaceAssignableProject (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `listActiveProjectsByWorkspaceRoot`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.listActiveProjectsByWorkspaceRoot
const call = native.listActiveProjectsByWorkspaceRootWire
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
module.exports.listActiveProjectsByWorkspaceRoot = function listActiveProjectsByWorkspaceRoot (a0, a1, a2) {
  if (illFormed(a1)) throw new TypeError('argument 2 of listActiveProjectsByWorkspaceRoot has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `listThreadsByProjectId`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.listThreadsByProjectId
const call = native.listThreadsByProjectIdWire
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
module.exports.listThreadsByProjectId = function listThreadsByProjectId (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `requireProject`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.requireProject
const call = native.requireProjectWire
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
module.exports.requireProject = function requireProject (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `requireProjectAbsent`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.requireProjectAbsent
const call = native.requireProjectAbsentWire
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
module.exports.requireProjectAbsent = function requireProjectAbsent (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `requireValidProjectFolders`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.requireValidProjectFolders
const call = native.requireValidProjectFoldersWire
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
module.exports.requireValidProjectFolders = function requireValidProjectFolders (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `requireProjectWorkspaceRootAvailable`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.requireProjectWorkspaceRootAvailable
const call = native.requireProjectWorkspaceRootAvailableWire
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
module.exports.requireProjectWorkspaceRootAvailable = function requireProjectWorkspaceRootAvailable (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `requireProjectHasNoThreads`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.requireProjectHasNoThreads
const call = native.requireProjectHasNoThreadsWire
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
module.exports.requireProjectHasNoThreads = function requireProjectHasNoThreads (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `requireThread`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.requireThread
const call = native.requireThreadWire
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
module.exports.requireThread = function requireThread (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `requireApprovalNotResponded`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.requireApprovalNotResponded
const call = native.requireApprovalNotRespondedWire
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
module.exports.requireApprovalNotResponded = function requireApprovalNotResponded (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `requireThreadAbsent`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.requireThreadAbsent
const call = native.requireThreadAbsentWire
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
module.exports.requireThreadAbsent = function requireThreadAbsent (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `requireThreadArchived`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.requireThreadArchived
const call = native.requireThreadArchivedWire
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
module.exports.requireThreadArchived = function requireThreadArchived (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `requireThreadNotArchived`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.requireThreadNotArchived
const call = native.requireThreadNotArchivedWire
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
module.exports.requireThreadNotArchived = function requireThreadNotArchived (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `threadResumePreconditionViolation`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.threadResumePreconditionViolation
const call = native.threadResumePreconditionViolationWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
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
t = rStr()
}
}
return t
}
module.exports.threadResumePreconditionViolation = function threadResumePreconditionViolation (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `threadResumePreconditionDetail`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.threadResumePreconditionDetail
const call = native.threadResumePreconditionDetailWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'string') return false
wStr(a1)
return true
}
const decode = () => {
let t
t = rStr()
return t
}
module.exports.threadResumePreconditionDetail = function threadResumePreconditionDetail (a0, a1) {
  if (illFormed(a1)) throw new TypeError('argument 2 of threadResumePreconditionDetail has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `withoutRemoteResumeComputerGrant`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.withoutRemoteResumeComputerGrant
const call = native.withoutRemoteResumeComputerGrantWire
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
module.exports.withoutRemoteResumeComputerGrant = function withoutRemoteResumeComputerGrant (a0, a1) {
  if (illFormed(a1)) throw new TypeError('argument 2 of withoutRemoteResumeComputerGrant has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `resolveForkSourceCutoff`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.resolveForkSourceCutoff
const call = native.resolveForkSourceCutoffWire
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
module.exports.resolveForkSourceCutoff = function resolveForkSourceCutoff (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `buildForkThreadTitle`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.buildForkThreadTitle
const call = native.buildForkThreadTitleWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (!Array.isArray(a1)) return false
wU8(1)
wU32(a1.length)
for (let i0 = 0; i0 < a1.length; i0++) {
const v0 = a1[i0]
if (v0 === null || typeof v0 !== 'object' || Array.isArray(v0)) return false
{
const at1 = at
wU8(0)
let n1 = 0, r1 = 0
for (const k1 in v0) {
switch (k1) {
case "id": {
wU8(0)
r1++
const f1 = v0[k1]
if (typeof f1 !== 'string') return false
wStr(f1)
break
}
case "projectId": {
wU8(1)
r1++
const f1 = v0[k1]
if (typeof f1 !== 'string') return false
wStr(f1)
break
}
case "title": {
wU8(2)
r1++
const f1 = v0[k1]
if (typeof f1 !== 'string') return false
wStr(f1)
break
}
case "forkSourceThreadId": {
wU8(3)
const f1 = v0[k1]
if (f1 === undefined) {
wU8(0)
} else if (f1 === null) {
wU8(2)
} else {
wU8(1)
if (typeof f1 !== 'string') return false
wStr(f1)
}
break
}
default: return false
}
if (++n1 > 255) return false
}
if (r1 !== 3) return false
iv.setUint8(at1, n1)
}
}
return true
}
const decode = () => {
let t
t = rStr()
return t
}
module.exports.buildForkThreadTitle = function buildForkThreadTitle (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `resolveStableMessageTurnId`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.resolveStableMessageTurnId
const call = native.resolveStableMessageTurnIdWire
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
module.exports.resolveStableMessageTurnId = function resolveStableMessageTurnId (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `usesReservedCommandAdmission`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.usesReservedCommandAdmission
const call = native.usesReservedCommandAdmissionWire
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
module.exports.usesReservedCommandAdmission = function usesReservedCommandAdmission (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `isQuiescingCommandAdmissible`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.isQuiescingCommandAdmissible
const call = native.isQuiescingCommandAdmissibleWire
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
module.exports.isQuiescingCommandAdmissible = function isQuiescingCommandAdmissible (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `orchestrationCommandLane`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.orchestrationCommandLane
const call = native.orchestrationCommandLaneWire
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
module.exports.orchestrationCommandLane = function orchestrationCommandLane (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `isExpiredSidechat`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.isExpiredSidechat
const call = native.isExpiredSidechatWire
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
module.exports.isExpiredSidechat = function isExpiredSidechat (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `latestSidechatActivityAt`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.latestSidechatActivityAt
const call = native.latestSidechatActivityAtWire
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
if (typeof a1 !== 'string') return false
wStr(a1)
return true
}
const decode = () => {
let t
t = rStr()
return t
}
module.exports.latestSidechatActivityAt = function latestSidechatActivityAt (a0, a1) {
  if (illFormed(a0)) throw new TypeError('argument 1 of latestSidechatActivityAt has a lone surrogate, which a Rust String cannot hold')
  if (illFormed(a1)) throw new TypeError('argument 2 of latestSidechatActivityAt has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `sidechatActivityInstantsEqual`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.sidechatActivityInstantsEqual
const call = native.sidechatActivityInstantsEqualWire
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
module.exports.sidechatActivityInstantsEqual = function sidechatActivityInstantsEqual (a0, a1) {
  if (illFormed(a0)) throw new TypeError('argument 1 of sidechatActivityInstantsEqual has a lone surrogate, which a Rust String cannot hold')
  if (illFormed(a1)) throw new TypeError('argument 2 of sidechatActivityInstantsEqual has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `settleTurnStateFromSession`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.settleTurnStateFromSession
const call = native.settleTurnStateFromSessionWire
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
module.exports.settleTurnStateFromSession = function settleTurnStateFromSession (a0, a1) {
  if (illFormed(a1)) throw new TypeError('argument 2 of settleTurnStateFromSession has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `maxIso`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.maxIso
const call = native.maxIsoWire
const encode = (a0, a1) => {
at = 0
if (a0 === undefined) {
return false
wU8(0)
} else if (a0 === null) {
wU8(2)
} else {
wU8(1)
if (typeof a0 !== 'string') return false
wStr(a0)
}
if (typeof a1 !== 'string') return false
wStr(a1)
return true
}
const decode = () => {
let t
t = rStr()
return t
}
module.exports.maxIso = function maxIso (a0, a1) {
  if (illFormed(a0)) throw new TypeError('argument 1 of maxIso has a lone surrogate, which a Rust String cannot hold')
  if (illFormed(a1)) throw new TypeError('argument 2 of maxIso has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `deriveTurnStartModelSelection`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.deriveTurnStartModelSelection
const call = native.deriveTurnStartModelSelectionWire
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
module.exports.deriveTurnStartModelSelection = function deriveTurnStartModelSelection (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `countNativeTurnStartMessages`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.countNativeTurnStartMessages
const call = native.countNativeTurnStartMessagesWire
const encode = (a0) => {
at = 0
if (!wDyn(a0)) return false
return true
}
const decode = () => {
let t
t = rF64()
return t
}
module.exports.countNativeTurnStartMessages = function countNativeTurnStartMessages (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `canAdoptFirstTurnProvider`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.canAdoptFirstTurnProvider
const call = native.canAdoptFirstTurnProviderWire
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
module.exports.canAdoptFirstTurnProvider = function canAdoptFirstTurnProvider (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `deriveTurnStartSession`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.deriveTurnStartSession
const call = native.deriveTurnStartSessionWire
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
module.exports.deriveTurnStartSession = function deriveTurnStartSession (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `relocateProjectPath`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.relocateProjectPath
const call = native.relocateProjectPathWire
const encode = (a0, a1, a2, a3) => {
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
if (typeof a1 !== 'string') return false
wStr(a1)
if (typeof a2 !== 'string') return false
wStr(a2)
if (!wDyn(a3)) return false
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
module.exports.relocateProjectPath = function relocateProjectPath (a0, a1, a2, a3) {
  if (illFormed(a0)) throw new TypeError('argument 1 of relocateProjectPath has a lone surrogate, which a Rust String cannot hold')
  if (illFormed(a1)) throw new TypeError('argument 2 of relocateProjectPath has a lone surrogate, which a Rust String cannot hold')
  if (illFormed(a2)) throw new TypeError('argument 3 of relocateProjectPath has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 4 && encode(a0, a1, a2, a3) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `providerWorkspaceChanged`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.providerWorkspaceChanged
const call = native.providerWorkspaceChangedWire
const encode = (a0, a1, a2) => {
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
if (!wDyn(a2)) return false
return true
}
const decode = () => {
let t
t = rU8() === 1
return t
}
module.exports.providerWorkspaceChanged = function providerWorkspaceChanged (a0, a1, a2) {
  if (illFormed(a0)) throw new TypeError('argument 1 of providerWorkspaceChanged has a lone surrogate, which a Rust String cannot hold')
  if (illFormed(a1)) throw new TypeError('argument 2 of providerWorkspaceChanged has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `isProviderIntentEventType`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.isProviderIntentEventType
const call = native.isProviderIntentEventTypeWire
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
module.exports.isProviderIntentEventType = function isProviderIntentEventType (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of isProviderIntentEventType has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `isProviderIntentEvent`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.isProviderIntentEvent
const call = native.isProviderIntentEventWire
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
module.exports.isProviderIntentEvent = function isProviderIntentEvent (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `isReplaySafeClaimedProviderIntent`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.isReplaySafeClaimedProviderIntent
const call = native.isReplaySafeClaimedProviderIntentWire
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
module.exports.isReplaySafeClaimedProviderIntent = function isReplaySafeClaimedProviderIntent (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `isProviderSideEffectIntent`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.isProviderSideEffectIntent
const call = native.isProviderSideEffectIntentWire
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
module.exports.isProviderSideEffectIntent = function isProviderSideEffectIntent (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `isClaimedProviderIntent`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.isClaimedProviderIntent
const call = native.isClaimedProviderIntentWire
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
module.exports.isClaimedProviderIntent = function isClaimedProviderIntent (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `isQuarantineExemptProviderIntent`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.isQuarantineExemptProviderIntent
const call = native.isQuarantineExemptProviderIntentWire
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
module.exports.isQuarantineExemptProviderIntent = function isQuarantineExemptProviderIntent (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `isUnsettledPendingInteraction`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.isUnsettledPendingInteraction
const call = native.isUnsettledPendingInteractionWire
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
module.exports.isUnsettledPendingInteraction = function isUnsettledPendingInteraction (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `pendingInteractionRequestKind`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.pendingInteractionRequestKind
const call = native.pendingInteractionRequestKindWire
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
module.exports.pendingInteractionRequestKind = function pendingInteractionRequestKind (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `buildStalePendingRequestSettlementCommand`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.buildStalePendingRequestSettlementCommand
const call = native.buildStalePendingRequestSettlementCommandWire
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
module.exports.buildStalePendingRequestSettlementCommand = function buildStalePendingRequestSettlementCommand (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `nativeImportId`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.nativeImportId
const call = native.nativeImportIdWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
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
t = rStr()
}
}
return t
}
module.exports.nativeImportId = function nativeImportId (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `makeProjectImportDestinations`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.makeProjectImportDestinations
const call = native.makeProjectImportDestinationsWire
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
module.exports.makeProjectImportDestinations = function makeProjectImportDestinations (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `threadArchiveFileName`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.threadArchiveFileName
const call = native.threadArchiveFileNameWire
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
module.exports.threadArchiveFileName = function threadArchiveFileName (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `listImportedHandoffMessages`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.listImportedHandoffMessages
const call = native.listImportedHandoffMessagesWire
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
module.exports.listImportedHandoffMessages = function listImportedHandoffMessages (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `listImportedForkMessages`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.listImportedForkMessages
const call = native.listImportedForkMessagesWire
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
module.exports.listImportedForkMessages = function listImportedForkMessages (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `hasNativeHandoffMessages`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.hasNativeHandoffMessages
const call = native.hasNativeHandoffMessagesWire
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
module.exports.hasNativeHandoffMessages = function hasNativeHandoffMessages (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `hasNativeAssistantMessagesBefore`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.hasNativeAssistantMessagesBefore
const call = native.hasNativeAssistantMessagesBeforeWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
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
module.exports.hasNativeAssistantMessagesBefore = function hasNativeAssistantMessagesBefore (a0, a1) {
  if (illFormed(a1)) throw new TypeError('argument 2 of hasNativeAssistantMessagesBefore has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `listPriorTranscriptMessages`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.listPriorTranscriptMessages
const call = native.listPriorTranscriptMessagesWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
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
t = rDyn()
return t
}
module.exports.listPriorTranscriptMessages = function listPriorTranscriptMessages (a0, a1) {
  if (illFormed(a1)) throw new TypeError('argument 2 of listPriorTranscriptMessages has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `buildHandoffBootstrapText`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.buildHandoffBootstrapText
const call = native.buildHandoffBootstrapTextWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
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
t = rStr()
}
}
return t
}
module.exports.buildHandoffBootstrapText = function buildHandoffBootstrapText (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `buildPriorTranscriptBootstrapText`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.buildPriorTranscriptBootstrapText
const call = native.buildPriorTranscriptBootstrapTextWire
const encode = (a0, a1, a2) => {
at = 0
if (!wDyn(a0)) return false
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
t = rStr()
}
}
return t
}
module.exports.buildPriorTranscriptBootstrapText = function buildPriorTranscriptBootstrapText (a0, a1, a2) {
  if (illFormed(a1)) throw new TypeError('argument 2 of buildPriorTranscriptBootstrapText has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `buildPriorTranscriptBootstrap`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.buildPriorTranscriptBootstrap
const call = native.buildPriorTranscriptBootstrapWire
const encode = (a0, a1, a2) => {
at = 0
if (!wDyn(a0)) return false
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
{
const o1 = {}
const n1 = rU8()
for (let j1 = 0; j1 < n1; j1++) {
switch (rU8()) {
case 0: {
let v1
v1 = rStr()
o1["text"] = v1
break
}
case 1: {
let v1
v1 = rF64()
o1["omittedMessages"] = v1
break
}
case 2: {
let v1
v1 = rF64()
o1["shortenedMessages"] = v1
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
module.exports.buildPriorTranscriptBootstrap = function buildPriorTranscriptBootstrap (a0, a1, a2) {
  if (illFormed(a1)) throw new TypeError('argument 2 of buildPriorTranscriptBootstrap has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `buildForkBootstrapText`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.buildForkBootstrapText
const call = native.buildForkBootstrapTextWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
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
t = rStr()
}
}
return t
}
module.exports.buildForkBootstrapText = function buildForkBootstrapText (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `claudeHistoricalSessionChildEnvironment`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.claudeHistoricalSessionChildEnvironment
const call = native.claudeHistoricalSessionChildEnvironmentWire
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
module.exports.claudeHistoricalSessionChildEnvironment = function claudeHistoricalSessionChildEnvironment (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `claudeHistoricalSessionEnvironment`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.claudeHistoricalSessionEnvironment
const call = native.claudeHistoricalSessionEnvironmentWire
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
module.exports.claudeHistoricalSessionEnvironment = function claudeHistoricalSessionEnvironment (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `resolveImportedThreadProviderOptionsForSettings`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.resolveImportedThreadProviderOptionsForSettings
const call = native.resolveImportedThreadProviderOptionsForSettingsWire
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
module.exports.resolveImportedThreadProviderOptionsForSettings = function resolveImportedThreadProviderOptionsForSettings (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `mapCodexSnapshotMessages`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.mapCodexSnapshotMessages
const call = native.mapCodexSnapshotMessagesWire
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
module.exports.mapCodexSnapshotMessages = function mapCodexSnapshotMessages (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `mapClaudeSessionMessages`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.mapClaudeSessionMessages
const call = native.mapClaudeSessionMessagesWire
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
module.exports.mapClaudeSessionMessages = function mapClaudeSessionMessages (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `mapOpenCodeSnapshotMessages`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.mapOpenCodeSnapshotMessages
const call = native.mapOpenCodeSnapshotMessagesWire
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
module.exports.mapOpenCodeSnapshotMessages = function mapOpenCodeSnapshotMessages (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `mapFactorySnapshotMessages`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.mapFactorySnapshotMessages
const call = native.mapFactorySnapshotMessagesWire
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
module.exports.mapFactorySnapshotMessages = function mapFactorySnapshotMessages (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `mapOmpSnapshotMessages`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.mapOmpSnapshotMessages
const call = native.mapOmpSnapshotMessagesWire
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
module.exports.mapOmpSnapshotMessages = function mapOmpSnapshotMessages (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `normalizeWindowsImportPath`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.normalizeWindowsImportPath
const call = native.normalizeWindowsImportPathWire
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
module.exports.normalizeWindowsImportPath = function normalizeWindowsImportPath (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of normalizeWindowsImportPath has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `importPathIdentity`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.importPathIdentity
const call = native.importPathIdentityWire
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
module.exports.importPathIdentity = function importPathIdentity (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of importPathIdentity has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `resolveProjectImportSources`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.resolveProjectImportSources
const call = native.resolveProjectImportSourcesWire
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
{
const o1 = {}
const n1 = rU8()
for (let j1 = 0; j1 < n1; j1++) {
switch (rU8()) {
case 0: {
let v1
v1 = rDyn()
o1["provider"] = v1
break
}
case 1: {
let v1
v1 = rDyn()
o1["instanceId"] = v1
break
}
case 2: {
let v1
v1 = rU8() === 1
o1["isDefault"] = v1
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
o1["accountLabel"] = v1
break
}
case 4: {
let v1
v1 = rDyn()
o1["providerOptions"] = v1
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
o1["codexHomePath"] = v1
break
}
case 6: {
let v1
v1 = rDyn()
o1["environment"] = v1
break
}
case 7: {
let v1
v1 = rDyn()
o1["claudeEnvironment"] = v1
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
o1["claudeConfigDir"] = v1
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
module.exports.resolveProjectImportSources = function resolveProjectImportSources (a0, a1, a2) {
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `canProjectTurnModelSelectionForSession`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.canProjectTurnModelSelectionForSession
const call = native.canProjectTurnModelSelectionForSessionWire
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
module.exports.canProjectTurnModelSelectionForSession = function canProjectTurnModelSelectionForSession (a0, a1) {
  if (illFormed(a1)) throw new TypeError('argument 2 of canProjectTurnModelSelectionForSession has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `createEmptyReadModel`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.createEmptyReadModel
const call = native.createEmptyReadModelWire
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
module.exports.createEmptyReadModel = function createEmptyReadModel (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of createEmptyReadModel has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `projectEvent`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.projectEvent
const call = native.projectEventWire
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
module.exports.projectEvent = function projectEvent (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `sanitizeProviderStartOptionsForPersistence`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.sanitizeProviderStartOptionsForPersistence
const call = native.sanitizeProviderStartOptionsForPersistenceWire
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
module.exports.sanitizeProviderStartOptionsForPersistence = function sanitizeProviderStartOptionsForPersistence (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `readableReasoningDetail`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.readableReasoningDetail
const call = native.readableReasoningDetailWire
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
module.exports.readableReasoningDetail = function readableReasoningDetail (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of readableReasoningDetail has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `runtimePayloadRecord`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.runtimePayloadRecord
const call = native.runtimePayloadRecordWire
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
module.exports.runtimePayloadRecord = function runtimePayloadRecord (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `runtimeTurnState`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.runtimeTurnState
const call = native.runtimeTurnStateWire
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
module.exports.runtimeTurnState = function runtimeTurnState (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `providerRuntimeProgressKey`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.providerRuntimeProgressKey
const call = native.providerRuntimeProgressKeyWire
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
module.exports.providerRuntimeProgressKey = function providerRuntimeProgressKey (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `providerRuntimeCommandProgressKey`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.providerRuntimeCommandProgressKey
const call = native.providerRuntimeCommandProgressKeyWire
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
module.exports.providerRuntimeCommandProgressKey = function providerRuntimeCommandProgressKey (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `coalesceProviderRuntimeProgress`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.coalesceProviderRuntimeProgress
const call = native.coalesceProviderRuntimeProgressWire
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
module.exports.coalesceProviderRuntimeProgress = function coalesceProviderRuntimeProgress (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `providerActivityUpdateFingerprint`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.providerActivityUpdateFingerprint
const call = native.providerActivityUpdateFingerprintWire
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
module.exports.providerActivityUpdateFingerprint = function providerActivityUpdateFingerprint (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `providerActivityUpdateDedupeKey`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.providerActivityUpdateDedupeKey
const call = native.providerActivityUpdateDedupeKeyWire
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
t = rStr()
}
}
return t
}
module.exports.providerActivityUpdateDedupeKey = function providerActivityUpdateDedupeKey (a0, a1, a2) {
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `detectCreatedPullRequestUrls`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.detectCreatedPullRequestUrls
const call = native.detectCreatedPullRequestUrlsWire
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
v0 = rStr()
a0[i0] = v0
}
t = a0
}
return t
}
module.exports.detectCreatedPullRequestUrls = function detectCreatedPullRequestUrls (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `buildQuitResumeRecord`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.buildQuitResumeRecord
const call = native.buildQuitResumeRecordWire
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
module.exports.buildQuitResumeRecord = function buildQuitResumeRecord (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `buildQuitInterruptCommand`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.buildQuitInterruptCommand
const call = native.buildQuitInterruptCommandWire
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
module.exports.buildQuitInterruptCommand = function buildQuitInterruptCommand (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `planQuitResumeTurns`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.planQuitResumeTurns
const call = native.planQuitResumeTurnsWire
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
const a1 = new Array(n1)
for (let i1 = 0; i1 < n1; i1++) {
let v1
v1 = rDyn()
a1[i1] = v1
}
v0 = a1
}
o0["commands"] = v0
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
o2["threadId"] = v2
break
}
case 1: {
let v2
v2 = rDyn()
o2["reason"] = v2
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
o0["skipped"] = v0
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
module.exports.planQuitResumeTurns = function planQuitResumeTurns (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `persistQuitResumeRecord`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.persistQuitResumeRecord
const call = native.persistQuitResumeRecordWire
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
module.exports.persistQuitResumeRecord = function persistQuitResumeRecord (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `clearQuitResumeRecord`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.clearQuitResumeRecord
const call = native.clearQuitResumeRecordWire
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
module.exports.clearQuitResumeRecord = function clearQuitResumeRecord (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of clearQuitResumeRecord has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `persistCrashResumeRecord`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.persistCrashResumeRecord
const call = native.persistCrashResumeRecordWire
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
module.exports.persistCrashResumeRecord = function persistCrashResumeRecord (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `readQuitResumeRecord`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.readQuitResumeRecord
const call = native.readQuitResumeRecordWire
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
module.exports.readQuitResumeRecord = function readQuitResumeRecord (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of readQuitResumeRecord has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `claimQuitResumeRecord`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.claimQuitResumeRecord
const call = native.claimQuitResumeRecordWire
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
module.exports.claimQuitResumeRecord = function claimQuitResumeRecord (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of claimQuitResumeRecord has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `resumeQuitInterruptedChats`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.resumeQuitInterruptedChats
const call = native.resumeQuitInterruptedChatsWire
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
module.exports.resumeQuitInterruptedChats = function resumeQuitInterruptedChats (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `findRestartOrphanedTasks`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.findRestartOrphanedTasks
const call = native.findRestartOrphanedTasksWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'string') return false
wStr(a1)
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
o1["taskId"] = v1
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
o1["toolUseId"] = v1
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
o1["workflowTaskId"] = v1
break
}
case 3: {
let v1
v1 = rDyn()
o1["turnId"] = v1
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
module.exports.findRestartOrphanedTasks = function findRestartOrphanedTasks (a0, a1) {
  if (illFormed(a1)) throw new TypeError('argument 2 of findRestartOrphanedTasks has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `deriveThreadGitMetadataPatch`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.deriveThreadGitMetadataPatch
const call = native.deriveThreadGitMetadataPatchWire
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
v1 = rStr()
}
}
o1["branch"] = v1
break
}
case 1: {
let v1
v1 = rDyn()
o1["discoveredPullRequest"] = v1
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
o1["associatedWorktreePath"] = v1
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
o1["associatedWorktreeBranch"] = v1
break
}
case 4: {
let v1
{
const g2 = rU8()
if (g2 === 0) v1 = undefined
else if (g2 === 2) v1 = null
else {
v1 = rStr()
}
}
o1["associatedWorktreeRef"] = v1
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
module.exports.deriveThreadGitMetadataPatch = function deriveThreadGitMetadataPatch (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `isOversizedThreadGoal`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.isOversizedThreadGoal
const call = native.isOversizedThreadGoalWire
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
t = rDyn()
return t
}
module.exports.isOversizedThreadGoal = function isOversizedThreadGoal (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of isOversizedThreadGoal has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `threadGoalFileName`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.threadGoalFileName
const call = native.threadGoalFileNameWire
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
module.exports.threadGoalFileName = function threadGoalFileName (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of threadGoalFileName has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `threadGoalFileReference`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.threadGoalFileReference
const call = native.threadGoalFileReferenceWire
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
module.exports.threadGoalFileReference = function threadGoalFileReference (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of threadGoalFileReference has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `threadGoalFileNameFromReference`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.threadGoalFileNameFromReference
const call = native.threadGoalFileNameFromReferenceWire
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
module.exports.threadGoalFileNameFromReference = function threadGoalFileNameFromReference (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `formatThreadLatestReplyPreview`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.formatThreadLatestReplyPreview
const call = native.formatThreadLatestReplyPreviewWire
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
module.exports.formatThreadLatestReplyPreview = function formatThreadLatestReplyPreview (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `shouldApplyThreadsProjection`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.shouldApplyThreadsProjection
const call = native.shouldApplyThreadsProjectionWire
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
module.exports.shouldApplyThreadsProjection = function shouldApplyThreadsProjection (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `shouldApplyDeferredThreadShellSummary`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.shouldApplyDeferredThreadShellSummary
const call = native.shouldApplyDeferredThreadShellSummaryWire
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
module.exports.shouldApplyDeferredThreadShellSummary = function shouldApplyDeferredThreadShellSummary (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `shouldPublishThreadShellForEvent`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.shouldPublishThreadShellForEvent
const call = native.shouldPublishThreadShellForEventWire
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
module.exports.shouldPublishThreadShellForEvent = function shouldPublishThreadShellForEvent (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `projectProviderRuntimeActivities`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.projectProviderRuntimeActivities
const call = native.projectProviderRuntimeActivitiesWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (a1 === undefined) {
wU8(0)
} else if (a1 === null) {
return false
wU8(2)
} else {
wU8(1)
if (typeof a1 !== 'number') return false
wF64(a1)
}
return true
}
const decode = () => {
let t
t = rDyn()
return t
}
module.exports.projectProviderRuntimeActivities = function projectProviderRuntimeActivities (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `planRestartTurnReconciliation`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.planRestartTurnReconciliation
const call = native.planRestartTurnReconciliationWire
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
module.exports.planRestartTurnReconciliation = function planRestartTurnReconciliation (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}
const guards = {
  readClaudeSessionMessagePageInEnvironment: [true, []],
  listClaudeSessionsInEnvironment: [true, []],
  readClaudeSessionMessagesInEnvironment: [true, []],
  projectImportKey: [false, []],
  buildProjectImportCatalog: [true, []],
  readMaterializedThreadGoalText: [true, []],
  materializeThreadGoalFile: [true, []],
  pruneThreadGoalFiles: [true, []],
  discardMaterializedThreadGoalFile: [true, []],
  canonicalImportPath: [true, [0]],
  importDirectoryExists: [true, [0]],
  findImportGitWorkspace: [true, [0]],
  resolveProjectImportSourceHome: [true, []],
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
