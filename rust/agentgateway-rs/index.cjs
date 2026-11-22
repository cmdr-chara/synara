// Loads the native module built by `rust-anything build`.
const native = require('./agentgateway_rs.node')

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
  saveBrowserProof: [[0, 0, 1, 0], 'raw', true],
  normalizeGatewayBrowserArguments: [[1, 1], 'json', false],
  computerApprovalDisplayArgs: [[1], 'raw', false],
  computerBrowserEffect: [[1, 1, 1], 'json', false],
  computerBrowserFieldReadback: [[1, 1, 1], 'raw', false],
  computerBrowserToolRequiresApproval: [[0, 1], 'raw', false],
  readComputerPastePayload: [[1], 'json', false],
  canonicalSynaraComputerToolName: [[1], 'json', false],
  qualifiedSynaraComputerToolName: [[1], 'json', false],
  isSynaraComputerToolFamilyName: [[1], 'raw', false],
  computerToolNameFromProviderPermission: [[1], 'json', false],
  isSynaraGatewayToolName: [[1], 'raw', false],
  isSynaraGatewayToolCall: [[1], 'raw', false],
  shouldAllowSynaraComputerProviderTool: [[1], 'raw', false],
  computerApprovalRequiredError: [[0], 'json', false],
  cuaActionErrorPayload: [[1], 'json', false],
  boundedWorkspacePath: [[1, 0, 0], 'raw', true],
  canonicalJson: [[1], 'raw', false],
  stableGatewayDigest: [[1, 0], 'raw', false],
  makeAgentCreationIds: [[0, 0], 'json', false],
  edgeForSwipeStart: [[1, 1], 'raw', false],
  diagnosticFilterFingerprint: [[1], 'raw', false],
  encodeDiagnosticCursor: [[1], 'raw', false],
  decodeDiagnosticCursor: [[0, 1], 'json', false],
  sanitizeDiagnosticValue: [[1, 0], 'json', false],
  renderSynaraHarnessPolicy: [[1], 'raw', false],
  providerHasSynaraGatewayControl: [[1], 'raw', false],
  takeSynaraHarnessPolicyForSession: [[1, 1], 'raw', false],
  takeSynaraHarnessPolicyForProviderSession: [[1, 1], 'raw', false],
  takeSynaraHarnessPolicyTextPartForProviderSession: [[1, 1], 'json', false],
  readMcpJsonBody: [[1, 1], 'json', false],
  hubWorkWorkItemId: [[1], 'raw', false],
  hubWorkTaskFingerprint: [[1], 'raw', false],
  hubWorkRequestFingerprint: [[1], 'raw', false],
  renderHubWorkPrompt: [[1], 'raw', false],
  buildCodexMcpThreadConfig: [[1], 'json', false],
  buildOpenCodeMcpServer: [[1], 'json', false],
  buildOpenCodeConnectorMcpServers: [[1], 'json', false],
  buildClaudeMcpServers: [[1], 'json', false],
  buildAcpSynaraMcpServers: [[1], 'json', false],
  parseRecoverableCreationPlan: [[0, 0], 'json', false],
  recordCreatedWorktreeInPlan: [[1], 'raw', false],
  redactCreationPlanForPurgedCaller: [[1], 'raw', false],
  mcpToolResultError: [[0], 'json', false],
  mcpToolResultJson: [[1], 'json', false],
  jsonRpcResult: [[1, 1], 'json', false],
  jsonRpcError: [[1, 0, 0], 'json', false],
  parseMcpMessage: [[1], 'json', false],
  negotiateMcpProtocolVersion: [[1], 'raw', false],
  buildMcpInitializeResult: [[1], 'json', false],
  sanitizeToolInputSchema: [[1], 'json', false],
  isJsonRecord: [[1], 'raw', false],
  countSchemaKeyOccurrences: [[1, 0], 'raw', false],
  agentGatewayCapabilitiesFor: [[1], 'raw', false],
  agentGatewayProviderCapabilitiesFor: [[1], 'raw', false],
  agentGatewaySessionLeaseOptionsFor: [[1, 1], 'json', false],
  captureAgentGatewayCapabilityInput: [[1], 'json', false],
  agentGatewayTargetOptionGuidance: [[1], 'json', false],
  readDiagnosticPageLimit: [[1], 'raw', false],
  diagnosticEventMessageId: [[1], 'raw', false],
  groupDiagnosticEvents: [[1], 'json', false],
  shapeDiagnosticEvents: [[1, 0], 'json', false],
  deriveAgentThreadStatus: [[1], 'raw', false],
  summarizeThreadShell: [[1, 0], 'json', false],
  summarizeWaitThreadText: [[0], 'json', false],
  paginateThreadMessages: [[1], 'json', false],
  summarizeThreadDetail: [[1], 'json', false],
  readStringArg: [[1, 0, 1], 'raw', false],
  readVerbatimStringArg: [[1, 0], 'raw', false],
  readNumberArg: [[1, 0], 'raw', false],
  readBooleanArg: [[1, 0], 'raw', false],
  readIsoTimestampArg: [[1, 0], 'raw', false],
  readRecordArg: [[1, 0], 'json', false],
  readStringArrayArg: [[1, 0], 'json', false],
  parseProviderKind: [[0], 'json', false],
  readModelSelectionArg: [[1, 0], 'json', false],
  buildModelSelection: [[1, 0, 1], 'json', false],
  decodeCreateThreadsInput: [[1], 'json', false],
  decodeWaitForThreadsInput: [[1], 'json', false],
  filterToolsByCapability: [[1, 1], 'json', false],
  gatewayToolErrorResult: [[1], 'json', false],
  parseTurnPlanSteps: [[1], 'json', false],
}
const classes = {
  ComputerProgressGuard: [[1, 1], { check: [[1], 'json'], record: [[1, 1], 'raw'], recordVerifiedProgress: [[1, 0], 'raw'] }],
  ToolGuidanceCadence: [[0, 0], { shouldRefresh: [[0], 'raw'] }],
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

// `extractBearerToken`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.extractBearerToken
const call = native.extractBearerTokenWire
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
module.exports.extractBearerToken = function extractBearerToken (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of extractBearerToken has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `normalizeGatewayBrowserArguments`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.normalizeGatewayBrowserArguments
const call = native.normalizeGatewayBrowserArgumentsWire
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
module.exports.normalizeGatewayBrowserArguments = function normalizeGatewayBrowserArguments (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `computerApprovalDisplayArgs`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.computerApprovalDisplayArgs
const call = native.computerApprovalDisplayArgsWire
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
module.exports.computerApprovalDisplayArgs = function computerApprovalDisplayArgs (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `computerBrowserEffect`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.computerBrowserEffect
const call = native.computerBrowserEffectWire
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
v0 = rDyn()
o0["effect"] = v0
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
module.exports.computerBrowserEffect = function computerBrowserEffect (a0, a1, a2) {
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `computerBrowserFieldReadback`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.computerBrowserFieldReadback
const call = native.computerBrowserFieldReadbackWire
const encode = (a0, a1, a2) => {
at = 0
if (!wDyn(a0)) return false
if (!wDyn(a1)) return false
if (!wDyn(a2)) return false
return true
}
const decode = () => {
let t
t = rU8() === 1
return t
}
module.exports.computerBrowserFieldReadback = function computerBrowserFieldReadback (a0, a1, a2) {
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `computerBrowserToolRequiresApproval`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.computerBrowserToolRequiresApproval
const call = native.computerBrowserToolRequiresApprovalWire
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
module.exports.computerBrowserToolRequiresApproval = function computerBrowserToolRequiresApproval (a0, a1) {
  if (illFormed(a0)) throw new TypeError('argument 1 of computerBrowserToolRequiresApproval has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `computerToolInstructions`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.computerToolInstructions
const call = native.computerToolInstructionsWire
const encode = () => {
at = 0
return true
}
const decode = () => {
let t
t = rStr()
return t
}
module.exports.computerToolInstructions = function computerToolInstructions () {
  try {
    if (arguments.length <= 0 && encode() && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `readComputerPastePayload`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.readComputerPastePayload
const call = native.readComputerPastePayloadWire
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
module.exports.readComputerPastePayload = function readComputerPastePayload (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `canonicalSynaraComputerToolName`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.canonicalSynaraComputerToolName
const call = native.canonicalSynaraComputerToolNameWire
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
module.exports.canonicalSynaraComputerToolName = function canonicalSynaraComputerToolName (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `qualifiedSynaraComputerToolName`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.qualifiedSynaraComputerToolName
const call = native.qualifiedSynaraComputerToolNameWire
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
module.exports.qualifiedSynaraComputerToolName = function qualifiedSynaraComputerToolName (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `isSynaraComputerToolFamilyName`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.isSynaraComputerToolFamilyName
const call = native.isSynaraComputerToolFamilyNameWire
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
module.exports.isSynaraComputerToolFamilyName = function isSynaraComputerToolFamilyName (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `computerToolNameFromProviderPermission`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.computerToolNameFromProviderPermission
const call = native.computerToolNameFromProviderPermissionWire
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
module.exports.computerToolNameFromProviderPermission = function computerToolNameFromProviderPermission (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `isSynaraGatewayToolName`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.isSynaraGatewayToolName
const call = native.isSynaraGatewayToolNameWire
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
module.exports.isSynaraGatewayToolName = function isSynaraGatewayToolName (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `isSynaraGatewayToolCall`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.isSynaraGatewayToolCall
const call = native.isSynaraGatewayToolCallWire
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
module.exports.isSynaraGatewayToolCall = function isSynaraGatewayToolCall (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `shouldAllowSynaraComputerProviderTool`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.shouldAllowSynaraComputerProviderTool
const call = native.shouldAllowSynaraComputerProviderToolWire
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
module.exports.shouldAllowSynaraComputerProviderTool = function shouldAllowSynaraComputerProviderTool (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `computerToolRequiresApproval`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.computerToolRequiresApproval
const call = native.computerToolRequiresApprovalWire
const encode = (a0) => {
at = 0
if (typeof a0 !== 'string') return false
wStr(a0)
return true
}
const decode = () => {
let t
t = rU8() === 1
return t
}
module.exports.computerToolRequiresApproval = function computerToolRequiresApproval (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of computerToolRequiresApproval has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `computerApprovalRequiredError`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.computerApprovalRequiredError
const call = native.computerApprovalRequiredErrorWire
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
o0["code"] = v0
break
}
case 1: {
let v0
v0 = rStr()
o0["message"] = v0
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
module.exports.computerApprovalRequiredError = function computerApprovalRequiredError (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of computerApprovalRequiredError has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `cuaActionErrorPayload`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.cuaActionErrorPayload
const call = native.cuaActionErrorPayloadWire
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
o0["error"] = v0
break
}
case 1: {
let v0
v0 = rStr()
o0["effect"] = v0
break
}
case 2: {
let v0
v0 = rStr()
o0["message"] = v0
break
}
case 3: {
let v0
v0 = rU8() === 1
o0["retryAllowed"] = v0
break
}
case 4: {
let v0
v0 = rDyn()
o0["diagnostics"] = v0
break
}
case 5: {
let v0
v0 = rDyn()
o0["layer"] = v0
break
}
case 6: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rF64()
}
}
o0["wait_seconds"] = v0
break
}
case 7: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["requery_hint"] = v0
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
module.exports.cuaActionErrorPayload = function cuaActionErrorPayload (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `gatewayIsoNow`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.gatewayIsoNow
const call = native.gatewayIsoNowWire
const encode = () => {
at = 0
return true
}
const decode = () => {
let t
t = rStr()
return t
}
module.exports.gatewayIsoNow = function gatewayIsoNow () {
  try {
    if (arguments.length <= 0 && encode() && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `canonicalJson`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.canonicalJson
const call = native.canonicalJsonWire
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
module.exports.canonicalJson = function canonicalJson (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `stableGatewayDigest`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.stableGatewayDigest
const call = native.stableGatewayDigestWire
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
t = rStr()
return t
}
module.exports.stableGatewayDigest = function stableGatewayDigest (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `makeAgentCreationIds`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.makeAgentCreationIds
const call = native.makeAgentCreationIdsWire
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
t = rDyn()
return t
}
module.exports.makeAgentCreationIds = function makeAgentCreationIds (a0, a1) {
  if (illFormed(a0)) throw new TypeError('argument 1 of makeAgentCreationIds has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `deviceToolRequiresApproval`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.deviceToolRequiresApproval
const call = native.deviceToolRequiresApprovalWire
const encode = (a0) => {
at = 0
if (typeof a0 !== 'string') return false
wStr(a0)
return true
}
const decode = () => {
let t
t = rU8() === 1
return t
}
module.exports.deviceToolRequiresApproval = function deviceToolRequiresApproval (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of deviceToolRequiresApproval has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `edgeForSwipeStart`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.edgeForSwipeStart
const call = native.edgeForSwipeStartWire
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
module.exports.edgeForSwipeStart = function edgeForSwipeStart (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `diagnosticFilterFingerprint`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.diagnosticFilterFingerprint
const call = native.diagnosticFilterFingerprintWire
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
module.exports.diagnosticFilterFingerprint = function diagnosticFilterFingerprint (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `encodeDiagnosticCursor`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.encodeDiagnosticCursor
const call = native.encodeDiagnosticCursorWire
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
module.exports.encodeDiagnosticCursor = function encodeDiagnosticCursor (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `decodeDiagnosticCursor`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.decodeDiagnosticCursor
const call = native.decodeDiagnosticCursorWire
const encode = (a0, a1) => {
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
o1["kind"] = v1
break
}
case 2: {
let v1
v1 = rStr()
o1["threadId"] = v1
break
}
case 3: {
let v1
v1 = rStr()
o1["filterFingerprint"] = v1
break
}
case 4: {
let v1
v1 = rF64()
o1["highWaterSequence"] = v1
break
}
case 5: {
let v1
v1 = rF64()
o1["beforeSequence"] = v1
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
module.exports.decodeDiagnosticCursor = function decodeDiagnosticCursor (a0, a1) {
  if (illFormed(a0)) throw new TypeError('argument 1 of decodeDiagnosticCursor has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `sanitizeDiagnosticValue`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.sanitizeDiagnosticValue
const call = native.sanitizeDiagnosticValueWire
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
module.exports.sanitizeDiagnosticValue = function sanitizeDiagnosticValue (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `renderSynaraHarnessPolicy`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.renderSynaraHarnessPolicy
const call = native.renderSynaraHarnessPolicyWire
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
module.exports.renderSynaraHarnessPolicy = function renderSynaraHarnessPolicy (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `providerHasSynaraGatewayControl`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.providerHasSynaraGatewayControl
const call = native.providerHasSynaraGatewayControlWire
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
module.exports.providerHasSynaraGatewayControl = function providerHasSynaraGatewayControl (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `takeSynaraHarnessPolicyForSession`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.takeSynaraHarnessPolicyForSession
const call = native.takeSynaraHarnessPolicyForSessionWire
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
module.exports.takeSynaraHarnessPolicyForSession = function takeSynaraHarnessPolicyForSession (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `takeSynaraHarnessPolicyForProviderSession`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.takeSynaraHarnessPolicyForProviderSession
const call = native.takeSynaraHarnessPolicyForProviderSessionWire
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
module.exports.takeSynaraHarnessPolicyForProviderSession = function takeSynaraHarnessPolicyForProviderSession (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `takeSynaraHarnessPolicyTextPartForProviderSession`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.takeSynaraHarnessPolicyTextPartForProviderSession
const call = native.takeSynaraHarnessPolicyTextPartForProviderSessionWire
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
{
const o1 = {}
const n1 = rU8()
for (let j1 = 0; j1 < n1; j1++) {
switch (rU8()) {
case 0: {
let v1
v1 = rStr()
o1["type"] = v1
break
}
case 1: {
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
module.exports.takeSynaraHarnessPolicyTextPartForProviderSession = function takeSynaraHarnessPolicyTextPartForProviderSession (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `readMcpJsonBody`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.readMcpJsonBody
const call = native.readMcpJsonBodyWire
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
module.exports.readMcpJsonBody = function readMcpJsonBody (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `normalizeHubTaskKey`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.normalizeHubTaskKey
const call = native.normalizeHubTaskKeyWire
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
module.exports.normalizeHubTaskKey = function normalizeHubTaskKey (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of normalizeHubTaskKey has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `hubWorkWorkItemId`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.hubWorkWorkItemId
const call = native.hubWorkWorkItemIdWire
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
module.exports.hubWorkWorkItemId = function hubWorkWorkItemId (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `hubWorkTaskFingerprint`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.hubWorkTaskFingerprint
const call = native.hubWorkTaskFingerprintWire
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
module.exports.hubWorkTaskFingerprint = function hubWorkTaskFingerprint (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `hubWorkRequestFingerprint`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.hubWorkRequestFingerprint
const call = native.hubWorkRequestFingerprintWire
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
module.exports.hubWorkRequestFingerprint = function hubWorkRequestFingerprint (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `renderHubWorkPrompt`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.renderHubWorkPrompt
const call = native.renderHubWorkPromptWire
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
module.exports.renderHubWorkPrompt = function renderHubWorkPrompt (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `buildCodexMcpConfigToml`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.buildCodexMcpConfigToml
const call = native.buildCodexMcpConfigTomlWire
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
t = rStr()
return t
}
module.exports.buildCodexMcpConfigToml = function buildCodexMcpConfigToml (a0, a1) {
  if (illFormed(a0)) throw new TypeError('argument 1 of buildCodexMcpConfigToml has a lone surrogate, which a Rust String cannot hold')
  if (illFormed(a1)) throw new TypeError('argument 2 of buildCodexMcpConfigToml has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `buildCodexMcpThreadConfig`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.buildCodexMcpThreadConfig
const call = native.buildCodexMcpThreadConfigWire
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
module.exports.buildCodexMcpThreadConfig = function buildCodexMcpThreadConfig (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `buildOpenCodeMcpServer`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.buildOpenCodeMcpServer
const call = native.buildOpenCodeMcpServerWire
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
o0["type"] = v0
break
}
case 1: {
let v0
v0 = rStr()
o0["url"] = v0
break
}
case 2: {
let v0
v0 = rU8() === 1
o0["enabled"] = v0
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
v1 = rStr()
o1[k1] = v1
}
v0 = o1
}
o0["headers"] = v0
break
}
case 4: {
let v0
v0 = rU8() === 1
o0["oauth"] = v0
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
module.exports.buildOpenCodeMcpServer = function buildOpenCodeMcpServer (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `buildOpenCodeConnectorMcpServers`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.buildOpenCodeConnectorMcpServers
const call = native.buildOpenCodeConnectorMcpServersWire
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
o1["name"] = v1
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
o2["type"] = v2
break
}
case 1: {
let v2
v2 = rStr()
o2["url"] = v2
break
}
case 2: {
let v2
v2 = rU8() === 1
o2["enabled"] = v2
break
}
case 3: {
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
o2["headers"] = v2
break
}
case 4: {
let v2
v2 = rU8() === 1
o2["oauth"] = v2
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
o1["config"] = v1
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
module.exports.buildOpenCodeConnectorMcpServers = function buildOpenCodeConnectorMcpServers (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `buildClaudeMcpServers`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.buildClaudeMcpServers
const call = native.buildClaudeMcpServersWire
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
{
const o1 = {}
const n1 = rU8()
for (let j1 = 0; j1 < n1; j1++) {
switch (rU8()) {
case 0: {
let v1
v1 = rStr()
o1["type"] = v1
break
}
case 1: {
let v1
v1 = rStr()
o1["url"] = v1
break
}
case 2: {
let v1
if (rU8() === 0) v1 = null
else {
const n2 = rU32()
const o2 = {}
for (let i2 = 0; i2 < n2; i2++) {
const k2 = rStr()
let v2
v2 = rStr()
o2[k2] = v2
}
v1 = o2
}
o1["headers"] = v1
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
o0[k0] = v0
}
t = o0
}
return t
}
module.exports.buildClaudeMcpServers = function buildClaudeMcpServers (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `buildAcpSynaraMcpServers`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.buildAcpSynaraMcpServers
const call = native.buildAcpSynaraMcpServersWire
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
module.exports.buildAcpSynaraMcpServers = function buildAcpSynaraMcpServers (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `parseRecoverableCreationPlan`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.parseRecoverableCreationPlan
const call = native.parseRecoverableCreationPlanWire
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
o1["workspaceRoot"] = v1
break
}
case 1: {
let v1
v1 = rStr()
o1["environment"] = v1
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
o1["worktreeRef"] = v1
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
o1["newBranch"] = v1
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
o1["plannedWorktreePath"] = v1
break
}
case 5: {
let v1
v1 = rU8() === 1
o1["ownershipPreflightPassed"] = v1
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
v3 = rStr()
o3["operationId"] = v3
break
}
case 1: {
let v3
v3 = rStr()
o3["path"] = v3
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
o3["branch"] = v3
break
}
case 3: {
let v3
v3 = rStr()
o3["token"] = v3
break
}
case 4: {
let v3
v3 = rStr()
o3["gitDir"] = v3
break
}
case 5: {
let v3
v3 = rStr()
o3["head"] = v3
break
}
case 6: {
let v3
{
const g4 = rU8()
if (g4 === 0) v3 = undefined
else if (g4 === 2) v3 = null
else {
v3 = rStr()
}
}
o3["stateHash"] = v3
break
}
case 7: {
let v3
v3 = rStr()
o3["recordedAt"] = v3
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
o1["worktreeOwnership"] = v1
break
}
case 7: {
let v1
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
v2 = rStr()
o2["threadId"] = v2
break
}
case 1: {
let v2
v2 = rStr()
o2["compensateCommandId"] = v2
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
o1["ids"] = v1
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
module.exports.parseRecoverableCreationPlan = function parseRecoverableCreationPlan (a0, a1) {
  if (illFormed(a0)) throw new TypeError('argument 1 of parseRecoverableCreationPlan has a lone surrogate, which a Rust String cannot hold')
  if (illFormed(a1)) throw new TypeError('argument 2 of parseRecoverableCreationPlan has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `recordCreatedWorktreeInPlan`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.recordCreatedWorktreeInPlan
const call = native.recordCreatedWorktreeInPlanWire
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
module.exports.recordCreatedWorktreeInPlan = function recordCreatedWorktreeInPlan (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `redactCreationPlanForPurgedCaller`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.redactCreationPlanForPurgedCaller
const call = native.redactCreationPlanForPurgedCallerWire
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
module.exports.redactCreationPlanForPurgedCaller = function redactCreationPlanForPurgedCaller (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `mcpToolResultError`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.mcpToolResultError
const call = native.mcpToolResultErrorWire
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
o0["content"] = v0
break
}
case 1: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rU8() === 1
}
}
o0["isError"] = v0
break
}
case 2: {
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
v2 = rDyn()
o2[k2] = v2
}
v0 = o2
}
}
}
o0["structuredContent"] = v0
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
module.exports.mcpToolResultError = function mcpToolResultError (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of mcpToolResultError has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `mcpToolResultJson`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.mcpToolResultJson
const call = native.mcpToolResultJsonWire
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
o0["content"] = v0
break
}
case 1: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rU8() === 1
}
}
o0["isError"] = v0
break
}
case 2: {
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
v2 = rDyn()
o2[k2] = v2
}
v0 = o2
}
}
}
o0["structuredContent"] = v0
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
module.exports.mcpToolResultJson = function mcpToolResultJson (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `jsonRpcResult`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.jsonRpcResult
const call = native.jsonRpcResultWire
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
module.exports.jsonRpcResult = function jsonRpcResult (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `jsonRpcError`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.jsonRpcError
const call = native.jsonRpcErrorWire
const encode = (a0, a1, a2) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'number') return false
wF64(a1)
if (typeof a2 !== 'string') return false
wStr(a2)
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
module.exports.jsonRpcError = function jsonRpcError (a0, a1, a2) {
  if (illFormed(a2)) throw new TypeError('argument 3 of jsonRpcError has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `parseMcpMessage`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.parseMcpMessage
const call = native.parseMcpMessageWire
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
module.exports.parseMcpMessage = function parseMcpMessage (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `negotiateMcpProtocolVersion`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.negotiateMcpProtocolVersion
const call = native.negotiateMcpProtocolVersionWire
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
module.exports.negotiateMcpProtocolVersion = function negotiateMcpProtocolVersion (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `buildMcpInitializeResult`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.buildMcpInitializeResult
const call = native.buildMcpInitializeResultWire
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
module.exports.buildMcpInitializeResult = function buildMcpInitializeResult (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `sanitizeToolInputSchema`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.sanitizeToolInputSchema
const call = native.sanitizeToolInputSchemaWire
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
module.exports.sanitizeToolInputSchema = function sanitizeToolInputSchema (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `isJsonRecord`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.isJsonRecord
const call = native.isJsonRecordWire
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
module.exports.isJsonRecord = function isJsonRecord (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `countSchemaKeyOccurrences`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.countSchemaKeyOccurrences
const call = native.countSchemaKeyOccurrencesWire
const encode = (a0, a1) => {
at = 0
if (!wDyn(a0)) return false
if (typeof a1 !== 'string') return false
wStr(a1)
return true
}
const decode = () => {
let t
t = rF64()
return t
}
module.exports.countSchemaKeyOccurrences = function countSchemaKeyOccurrences (a0, a1) {
  if (illFormed(a1)) throw new TypeError('argument 2 of countSchemaKeyOccurrences has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `agentGatewayCapabilitiesFor`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.agentGatewayCapabilitiesFor
const call = native.agentGatewayCapabilitiesForWire
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
module.exports.agentGatewayCapabilitiesFor = function agentGatewayCapabilitiesFor (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `agentGatewayProviderCapabilitiesFor`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.agentGatewayProviderCapabilitiesFor
const call = native.agentGatewayProviderCapabilitiesForWire
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
module.exports.agentGatewayProviderCapabilitiesFor = function agentGatewayProviderCapabilitiesFor (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `agentGatewaySessionLeaseOptionsFor`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.agentGatewaySessionLeaseOptionsFor
const call = native.agentGatewaySessionLeaseOptionsForWire
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
o1["additionalCapabilities"] = v1
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
module.exports.agentGatewaySessionLeaseOptionsFor = function agentGatewaySessionLeaseOptionsFor (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `captureAgentGatewayCapabilityInput`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.captureAgentGatewayCapabilityInput
const call = native.captureAgentGatewayCapabilityInputWire
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
v0 = rU8() === 1
}
}
o0["enableComputerControl"] = v0
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
module.exports.captureAgentGatewayCapabilityInput = function captureAgentGatewayCapabilityInput (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `agentGatewayTargetOptionGuidance`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.agentGatewayTargetOptionGuidance
const call = native.agentGatewayTargetOptionGuidanceWire
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
o0["primaryOptionKey"] = v0
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
v1 = rStr()
a1[i1] = v1
}
v0 = a1
}
o0["alternativeOptionKeys"] = v0
break
}
case 2: {
let v0
v0 = rStr()
o0["optionSelectionRule"] = v0
break
}
case 3: {
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
o2["key"] = v2
break
}
case 1: {
let v2
v2 = rStr()
o2["valueType"] = v2
break
}
case 2: {
let v2
v2 = rDyn()
o2["allowedValues"] = v2
break
}
case 3: {
let v2
v2 = rStr()
o2["allowedValuesSource"] = v2
break
}
case 4: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rU8() === 1
}
}
o2["allowsCustomValue"] = v2
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
o0["providerOptions"] = v0
break
}
case 4: {
let v0
v0 = rDyn()
o0["optionsByModel"] = v0
break
}
case 5: {
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
v2 = rStr()
o2["instanceId"] = v2
break
}
case 2: {
let v2
v2 = rStr()
o2["model"] = v2
break
}
case 3: {
let v2
v2 = rDyn()
o2["options"] = v2
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
o0["exampleTarget"] = v0
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
module.exports.agentGatewayTargetOptionGuidance = function agentGatewayTargetOptionGuidance (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `readDiagnosticPageLimit`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.readDiagnosticPageLimit
const call = native.readDiagnosticPageLimitWire
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
module.exports.readDiagnosticPageLimit = function readDiagnosticPageLimit (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `diagnosticEventMessageId`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.diagnosticEventMessageId
const call = native.diagnosticEventMessageIdWire
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
module.exports.diagnosticEventMessageId = function diagnosticEventMessageId (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `groupDiagnosticEvents`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.groupDiagnosticEvents
const call = native.groupDiagnosticEventsWire
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
v1 = rDyn()
o1["event"] = v1
break
}
case 1: {
let v1
v1 = rF64()
o1["coalescedEventCount"] = v1
break
}
case 2: {
let v1
v1 = rF64()
o1["oldestSequence"] = v1
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
module.exports.groupDiagnosticEvents = function groupDiagnosticEvents (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `shapeDiagnosticEvents`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.shapeDiagnosticEvents
const call = native.shapeDiagnosticEventsWire
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
module.exports.shapeDiagnosticEvents = function shapeDiagnosticEvents (a0, a1) {
  if (illFormed(a1)) throw new TypeError('argument 2 of shapeDiagnosticEvents has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `deriveAgentThreadStatus`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.deriveAgentThreadStatus
const call = native.deriveAgentThreadStatusWire
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
module.exports.deriveAgentThreadStatus = function deriveAgentThreadStatus (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `summarizeThreadShell`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.summarizeThreadShell
const call = native.summarizeThreadShellWire
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
o0["threadId"] = v0
break
}
case 1: {
let v0
v0 = rStr()
o0["projectId"] = v0
break
}
case 2: {
let v0
v0 = rStr()
o0["title"] = v0
break
}
case 3: {
let v0
v0 = rStr()
o0["provider"] = v0
break
}
case 4: {
let v0
v0 = rStr()
o0["model"] = v0
break
}
case 5: {
let v0
v0 = rStr()
o0["status"] = v0
break
}
case 6: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["parentThreadId"] = v0
break
}
case 7: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["creationSource"] = v0
break
}
case 8: {
let v0
v0 = rStr()
o0["envMode"] = v0
break
}
case 9: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["branch"] = v0
break
}
case 10: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["worktreePath"] = v0
break
}
case 11: {
let v0
v0 = rU8() === 1
o0["archived"] = v0
break
}
case 12: {
let v0
v0 = rU8() === 1
o0["isSelf"] = v0
break
}
case 13: {
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
module.exports.summarizeThreadShell = function summarizeThreadShell (a0, a1) {
  if (illFormed(a1)) throw new TypeError('argument 2 of summarizeThreadShell has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `summarizeWaitThreadText`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.summarizeWaitThreadText
const call = native.summarizeWaitThreadTextWire
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
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["summary"] = v0
break
}
case 1: {
let v0
v0 = rU8() === 1
o0["truncated"] = v0
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
module.exports.summarizeWaitThreadText = function summarizeWaitThreadText (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of summarizeWaitThreadText has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `paginateThreadMessages`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.paginateThreadMessages
const call = native.paginateThreadMessagesWire
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
{
const o2 = {}
const n2 = rU8()
for (let j2 = 0; j2 < n2; j2++) {
switch (rU8()) {
case 0: {
let v2
v2 = rF64()
o2["index"] = v2
break
}
case 1: {
let v2
v2 = rStr()
o2["messageId"] = v2
break
}
case 2: {
let v2
v2 = rStr()
o2["messageVersion"] = v2
break
}
case 3: {
let v2
v2 = rStr()
o2["role"] = v2
break
}
case 4: {
let v2
v2 = rStr()
o2["text"] = v2
break
}
case 5: {
let v2
v2 = rU8() === 1
o2["truncated"] = v2
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
o2["dispatchOrigin"] = v2
break
}
case 7: {
let v2
v2 = rStr()
o2["createdAt"] = v2
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
case 1: {
let v0
v0 = rF64()
o0["totalMessages"] = v0
break
}
case 2: {
let v0
v0 = rF64()
o0["effectiveMessageLimit"] = v0
break
}
case 3: {
let v0
v0 = rF64()
o0["effectiveMaxMessageChars"] = v0
break
}
case 4: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["nextCursor"] = v0
break
}
case 5: {
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
v2 = rF64()
o2["index"] = v2
break
}
case 1: {
let v2
v2 = rStr()
o2["messageId"] = v2
break
}
case 2: {
let v2
v2 = rStr()
o2["messageVersion"] = v2
break
}
case 3: {
let v2
v2 = rF64()
o2["offsetChars"] = v2
break
}
case 4: {
let v2
v2 = rF64()
o2["endOffsetChars"] = v2
break
}
case 5: {
let v2
v2 = rF64()
o2["totalChars"] = v2
break
}
case 6: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rF64()
}
}
o2["nextOffsetChars"] = v2
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
o0["messagePage"] = v0
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
module.exports.paginateThreadMessages = function paginateThreadMessages (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `summarizeThreadDetail`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.summarizeThreadDetail
const call = native.summarizeThreadDetailWire
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
o0["threadId"] = v0
break
}
case 1: {
let v0
v0 = rStr()
o0["projectId"] = v0
break
}
case 2: {
let v0
v0 = rStr()
o0["title"] = v0
break
}
case 3: {
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
case 4: {
let v0
v0 = rStr()
o0["provider"] = v0
break
}
case 5: {
let v0
v0 = rStr()
o0["model"] = v0
break
}
case 6: {
let v0
v0 = rStr()
o0["status"] = v0
break
}
case 7: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["sessionStatus"] = v0
break
}
case 8: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["latestTurnState"] = v0
break
}
case 9: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["parentThreadId"] = v0
break
}
case 10: {
let v0
{
const g1 = rU8()
if (g1 === 0) v0 = undefined
else if (g1 === 2) v0 = null
else {
v0 = rStr()
}
}
o0["creationSource"] = v0
break
}
case 11: {
let v0
v0 = rStr()
o0["envMode"] = v0
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
o0["branch"] = v0
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
o0["worktreePath"] = v0
break
}
case 14: {
let v0
v0 = rU8() === 1
o0["archived"] = v0
break
}
case 15: {
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
o2["url"] = v2
break
}
case 1: {
let v2
v2 = rF64()
o2["number"] = v2
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
v2 = rStr()
o2["state"] = v2
break
}
case 4: {
let v2
v2 = rU8() === 1
o2["isDraft"] = v2
break
}
case 5: {
let v2
v2 = rStr()
o2["headBranch"] = v2
break
}
case 6: {
let v2
v2 = rStr()
o2["baseBranch"] = v2
break
}
case 7: {
let v2
v2 = rStr()
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
o0["pullRequests"] = v0
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
o0["lastError"] = v0
break
}
case 17: {
let v0
v0 = rStr()
o0["createdAt"] = v0
break
}
case 18: {
let v0
v0 = rStr()
o0["updatedAt"] = v0
break
}
case 19: {
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
o2["index"] = v2
break
}
case 1: {
let v2
v2 = rStr()
o2["messageId"] = v2
break
}
case 2: {
let v2
v2 = rStr()
o2["messageVersion"] = v2
break
}
case 3: {
let v2
v2 = rStr()
o2["role"] = v2
break
}
case 4: {
let v2
v2 = rStr()
o2["text"] = v2
break
}
case 5: {
let v2
v2 = rU8() === 1
o2["truncated"] = v2
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
o2["dispatchOrigin"] = v2
break
}
case 7: {
let v2
v2 = rStr()
o2["createdAt"] = v2
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
case 20: {
let v0
v0 = rF64()
o0["totalMessages"] = v0
break
}
case 21: {
let v0
v0 = rF64()
o0["effectiveMessageLimit"] = v0
break
}
case 22: {
let v0
v0 = rF64()
o0["effectiveMaxMessageChars"] = v0
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
o0["nextCursor"] = v0
break
}
case 24: {
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
v2 = rF64()
o2["index"] = v2
break
}
case 1: {
let v2
v2 = rStr()
o2["messageId"] = v2
break
}
case 2: {
let v2
v2 = rStr()
o2["messageVersion"] = v2
break
}
case 3: {
let v2
v2 = rF64()
o2["offsetChars"] = v2
break
}
case 4: {
let v2
v2 = rF64()
o2["endOffsetChars"] = v2
break
}
case 5: {
let v2
v2 = rF64()
o2["totalChars"] = v2
break
}
case 6: {
let v2
{
const g3 = rU8()
if (g3 === 0) v2 = undefined
else if (g3 === 2) v2 = null
else {
v2 = rF64()
}
}
o2["nextOffsetChars"] = v2
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
o0["messagePage"] = v0
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
module.exports.summarizeThreadDetail = function summarizeThreadDetail (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `readStringArg`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.readStringArg
const call = native.readStringArgWire
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
const g0 = rU8()
if (g0 === 0) t = undefined
else if (g0 === 2) t = null
else {
t = rStr()
}
}
return t
}
module.exports.readStringArg = function readStringArg (a0, a1, a2) {
  if (illFormed(a1)) throw new TypeError('argument 2 of readStringArg has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `readVerbatimStringArg`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.readVerbatimStringArg
const call = native.readVerbatimStringArgWire
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
module.exports.readVerbatimStringArg = function readVerbatimStringArg (a0, a1) {
  if (illFormed(a1)) throw new TypeError('argument 2 of readVerbatimStringArg has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `readNumberArg`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.readNumberArg
const call = native.readNumberArgWire
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
t = rF64()
}
}
return t
}
module.exports.readNumberArg = function readNumberArg (a0, a1) {
  if (illFormed(a1)) throw new TypeError('argument 2 of readNumberArg has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `readBooleanArg`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.readBooleanArg
const call = native.readBooleanArgWire
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
module.exports.readBooleanArg = function readBooleanArg (a0, a1) {
  if (illFormed(a1)) throw new TypeError('argument 2 of readBooleanArg has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `readIsoTimestampArg`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.readIsoTimestampArg
const call = native.readIsoTimestampArgWire
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
module.exports.readIsoTimestampArg = function readIsoTimestampArg (a0, a1) {
  if (illFormed(a1)) throw new TypeError('argument 2 of readIsoTimestampArg has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `readRecordArg`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.readRecordArg
const call = native.readRecordArgWire
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
module.exports.readRecordArg = function readRecordArg (a0, a1) {
  if (illFormed(a1)) throw new TypeError('argument 2 of readRecordArg has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `readStringArrayArg`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.readStringArrayArg
const call = native.readStringArrayArgWire
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
if (rU8() === 0) t = null
else {
const n1 = rU32()
const a1 = new Array(n1)
for (let i1 = 0; i1 < n1; i1++) {
let v1
v1 = rStr()
a1[i1] = v1
}
t = a1
}
}
}
return t
}
module.exports.readStringArrayArg = function readStringArrayArg (a0, a1) {
  if (illFormed(a1)) throw new TypeError('argument 2 of readStringArrayArg has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `parseProviderKind`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.parseProviderKind
const call = native.parseProviderKindWire
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
module.exports.parseProviderKind = function parseProviderKind (a0) {
  if (illFormed(a0)) throw new TypeError('argument 1 of parseProviderKind has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `readModelSelectionArg`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.readModelSelectionArg
const call = native.readModelSelectionArgWire
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
module.exports.readModelSelectionArg = function readModelSelectionArg (a0, a1) {
  if (illFormed(a1)) throw new TypeError('argument 2 of readModelSelectionArg has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `buildModelSelection`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.buildModelSelection
const call = native.buildModelSelectionWire
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
t = rDyn()
return t
}
module.exports.buildModelSelection = function buildModelSelection (a0, a1, a2) {
  if (illFormed(a1)) throw new TypeError('argument 2 of buildModelSelection has a lone surrogate, which a Rust String cannot hold')
  try {
    if (arguments.length <= 3 && encode(a0, a1, a2) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `decodeCreateThreadsInput`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.decodeCreateThreadsInput
const call = native.decodeCreateThreadsInputWire
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
module.exports.decodeCreateThreadsInput = function decodeCreateThreadsInput (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `decodeWaitForThreadsInput`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.decodeWaitForThreadsInput
const call = native.decodeWaitForThreadsInputWire
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
module.exports.decodeWaitForThreadsInput = function decodeWaitForThreadsInput (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `filterToolsByCapability`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.filterToolsByCapability
const call = native.filterToolsByCapabilityWire
const encode = (a0, a1) => {
at = 0
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
case "requiredCapability": {
wU8(0)
const f1 = v0[k1]
if (!wDyn(f1)) return false
break
}
default: return false
}
if (++n1 > 255) return false
}
if (r1 !== 0) return false
iv.setUint8(at1, n1)
}
}
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
o1["requiredCapability"] = v1
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
module.exports.filterToolsByCapability = function filterToolsByCapability (a0, a1) {
  try {
    if (arguments.length <= 2 && encode(a0, a1) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `gatewayToolErrorResult`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.gatewayToolErrorResult
const call = native.gatewayToolErrorResultWire
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
module.exports.gatewayToolErrorResult = function gatewayToolErrorResult (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}

// `parseTurnPlanSteps`: binary wire; the JSON version handles inputs of other shapes.
{
const json = module.exports.parseTurnPlanSteps
const call = native.parseTurnPlanStepsWire
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
module.exports.parseTurnPlanSteps = function parseTurnPlanSteps (a0) {
  try {
    if (arguments.length <= 1 && encode(a0) && wireCall(call)) return decode()
    return json(...arguments)
  } catch (e) {
    throw jsError(e)
  }
}
}
const guards = {
  saveBrowserProof: [true, [0, 1, 3]],
  boundedWorkspacePath: [true, [1, 2]],
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
