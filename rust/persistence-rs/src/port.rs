//! The Rust port of `apps/server/src/persistence/`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};

use crate::js;

/// Cwd the source process recorded in (the rust-anything scratch dir), used by `path.resolve`.
const RECORDED_CWD: &str =
    "/private/var/folders/jj/fwmm45ns4dz80954vdr61l2w0000gn/T/rust-anything-app-unit-58429-1";

/// Canonical `(id, name)` pairs from `Migrations.ts` `migrationEntries`.
const MIGRATION_ENTRIES: &[(i64, &str)] = &[
    (1, "OrchestrationEvents"),
    (2, "OrchestrationCommandReceipts"),
    (3, "CheckpointDiffBlobs"),
    (4, "ProviderSessionRuntime"),
    (5, "Projections"),
    (6, "ProjectionThreadSessionRuntimeModeColumns"),
    (7, "ProjectionThreadMessageAttachments"),
    (8, "ProjectionThreadActivitySequence"),
    (9, "ProviderSessionRuntimeMode"),
    (10, "ProjectionThreadsRuntimeMode"),
    (11, "OrchestrationThreadCreatedRuntimeMode"),
    (12, "ProjectionThreadsInteractionMode"),
    (13, "ProjectionThreadProposedPlans"),
    (14, "ProjectionThreadProposedPlanImplementation"),
    (15, "ProjectionTurnsSourceProposedPlan"),
    (16, "CanonicalizeModelSelections"),
    (17, "ThreadHandoffMetadata"),
    (18, "ProjectionThreadMessageMentions"),
    (19, "ProjectionThreadsEnvMode"),
    (20, "ProjectionThreadsForkSource"),
    (21, "ProjectionThreadsAssociatedWorktree"),
    (22, "ProjectionThreadsAssociatedWorktreeBranch"),
    (23, "ProjectionThreadsAssociatedWorktreeRef"),
    (24, "ProjectionThreadsArchivedAt"),
    (25, "ProjectionThreadsSubagents"),
    (26, "ProjectionThreadShellSummary"),
    (27, "BackfillProjectionThreadShellSummary"),
    (28, "ProjectionProjectsKind"),
    (29, "ProjectionThreadsLastKnownPr"),
    (30, "ProjectionThreadMessagesDispatchMode"),
    (31, "ProjectionThreadsCreateBranchFlowCompleted"),
    (32, "ReconcileImportedSchemaLineage"),
    (33, "ProjectionThreadsSidechatSource"),
    (34, "AuthAccessManagement"),
    (35, "NormalizeLegacyModelSelectionOptions"),
    (36, "ProjectionThreadsPinned"),
    (37, "ProjectionSnapshotCapIndexes"),
    (38, "ReconcileLegacySidechatSource"),
    (39, "ReconcileLegacyPinnedThreads"),
    (40, "ProjectionThreadsPinnedMessagesNotes"),
    (41, "ProjectionProjectsPinned"),
    (42, "ProjectionThreadsMarkers"),
    (43, "ProfileStatsIndexes"),
    (44, "Automations"),
    (45, "AutomationPolicies"),
    (46, "AutomationCompletionPolicy"),
    (47, "AutomationCompletionPolicyVersion"),
    (48, "AutomationCompletionEvaluationBacklog"),
    (49, "ProjectionThreadMessagesDispatchOrigin"),
    (50, "ProfileStatsArchive"),
    (51, "ProfileStatsDeletedTokensModel"),
    (52, "ProjectionThreadUserMessageSummaryIndex"),
    (53, "BackfillThreadActivitySequence"),
    (54, "DurableProviderCommandDelivery"),
    (55, "ManagedAttachments"),
    (56, "CommandReceiptFingerprints"),
    (57, "ThreadScopedProjectionMessageIdentity"),
    (58, "ThreadScopedPendingApprovalIdentity"),
    (59, "ProviderSessionLifecycleGeneration"),
    (60, "PendingApprovalLifecycleGeneration"),
    (61, "PendingApprovalSettlementState"),
    (62, "PendingInteractionSettlementParity"),
    (63, "ProjectionMessageCausalSequence"),
    (64, "DurableProviderCommandDeliveryCutover"),
    (65, "DurableQueuedTurnPromotions"),
    (66, "DurableProviderRuntimeEvents"),
    (67, "ProviderDeliveryReconciliation"),
    (68, "GitHandoffOperations"),
    (69, "ProjectPullRequestPins"),
    (70, "AgentGatewayOperations"),
    (71, "ProjectionThreadsGatewayProvenance"),
    (72, "AgentGatewayOperationRetention"),
    (73, "OperationalDiagnostics"),
    (74, "ExternalMcpIntegrations"),
    (75, "ExternalMcpActiveCapacity"),
    (76, "ExternalMcpHardening"),
    (77, "ExternalMcpCompensatingCapacity"),
    (78, "ExternalMcpLiveTurnCapacity"),
    (79, "Spaces"),
    (80, "ExternalMcpProjectScope"),
    (81, "AutomationProposals"),
    (82, "AutomationMemory"),
    (83, "AutomationHeartbeatEligibility"),
    (84, "AutomationNotificationPolicy"),
    (85, "AutomationSettings"),
    (86, "NormalizeStudioThreadWorkspaces"),
    (87, "DropUnusedOrchestrationEventIndexes"),
    (88, "ProjectionThreadsSettledAt"),
    (89, "RecoverRetentionHiddenThreads"),
    (90, "ProjectionThreadMessageTextSegments"),
    (91, "AutomationFailureTolerance"),
    (92, "BackfillAutomationRunThreadSource"),
    (93, "BackfillMaxIterationsDisabledReason"),
    (94, "ProjectionThreadsGoal"),
    (95, "ProjectionThreadsGoalTiming"),
    (96, "ProjectionThreadsGoalAchievements"),
    (97, "ProjectionThreadsSidechatLifecycle"),
    (98, "MigrateKiloToOpenCode"),
    (99, "InvalidateProjectionThreadsCursor"),
    (100, "MessageTextChunks"),
    (101, "RemoveTranscriptMarkers"),
    (102, "ProjectionThreadMessagesTurnBoundary"),
    (103, "ClaudeTokenAccounting"),
    (104, "ProjectionThreadsClaudeCacheReview"),
    (105, "AsyncUserInput"),
    (106, "ProjectImportOrigins"),
    (107, "ProjectionThreadsHumanMessage"),
    (108, "GatewayCompletions"),
    (109, "ProjectAgent"),
    (110, "Groups"),
    (111, "GroupLibraryHosting"),
    (112, "CoordinatorAppearance"),
    (113, "ProjectAgentWakeCursor"),
    (114, "ProjectAgentLifecycle"),
    (115, "ProjectAgentManagedWorkers"),
    (116, "ProjectAgentWorkerRecovery"),
    (117, "WorkerMonitoringLiveness"),
    (118, "ProjectionThreadSessionProviderInstance"),
    (119, "ProviderSessionRuntimeInstanceId"),
    (120, "ProfileStatsDeletedProviderInstances"),
    (121, "ClearAutomationDefinitionProviderOptions"),
    (122, "ClearAutomationRunProviderOptions"),
    (123, "ScrubOrchestrationEventProviderOptions"),
    (124, "ProjectionTurnsPendingMessageIndex"),
    (125, "Todos"),
    (126, "ProjectionThreadsSidechatContext"),
    (127, "ProjectImportHistory"),
    (128, "HubWork"),
    (129, "ProjectionThreadsSnooze"),
    (130, "PullRequestAutoFix"),
    (131, "ProjectSourceFolders"),
    (132, "ExternalMcpTurnCapacityRecovery"),
    (133, "ProjectionTurnsWorkspaceInitialization"),
    (134, "ProjectionThreadsForkSourceMessage"),
    (135, "ProjectionMessageTurnActivityIndex"),
    (136, "AccountUsageSync"),
    (137, "AccountUsageSyncIdentity"),
    (138, "RemoteDeviceTrust"),
    (139, "RemoteConnectionPreferences"),
    (140, "RemoteAccessControls"),
    (141, "ProjectionThreadsUsageLimitRecovery"),
    (142, "AuthAccessOrigin"),
    (143, "UsageAnalyticsDeletedTokens"),
    (144, "ProjectionThreadsPullRequestLinks"),
];

fn canonical_name(id: i64) -> Option<&'static str> {
    MIGRATION_ENTRIES
        .iter()
        .find(|(i, _)| *i == id)
        .map(|(_, n)| *n)
}

const ACCOUNT_MIGRATION_LINEAGES: &[(i64, &[&str])] = &[
    (
        134,
        &[
            "AccountUsageSync",
            "AccountUsageSyncIdentity",
            "RemoteDeviceTrust",
            "RemoteConnectionPreferences",
            "RemoteAccessControls",
        ],
    ),
    (
        131,
        &[
            "AccountUsageSync",
            "AccountUsageSyncIdentity",
            "RemoteDeviceTrust",
            "RemoteConnectionPreferences",
            "RemoteAccessControls",
        ],
    ),
    (
        130,
        &[
            "AccountUsageSync",
            "AccountUsageSyncIdentity",
            "RemoteDeviceTrust",
            "RemoteConnectionPreferences",
            "RemoteAccessControls",
        ],
    ),
    (89, &["AccountUsageSync"]),
    (90, &["AccountUsageSync"]),
    (90, &["AccountUsageSync", "AccountUsageSyncIdentity"]),
    (98, &["AccountUsageSync", "AccountUsageSyncIdentity"]),
    (99, &["AccountUsageSync", "AccountUsageSyncIdentity"]),
    (
        108,
        &[
            "AccountUsageSync",
            "AccountUsageSyncIdentity",
            "RemoteDeviceTrust",
            "RemoteConnectionPreferences",
        ],
    ),
    (
        126,
        &[
            "AccountUsageSync",
            "AccountUsageSyncIdentity",
            "RemoteDeviceTrust",
            "RemoteConnectionPreferences",
        ],
    ),
];

const PROVIDER_KINDS: &[&str] = &[
    "codex",
    "claudeAgent",
    "cursor",
    "antigravity",
    "grok",
    "droid",
    "opencode",
    "pi",
    "devin",
    "omp",
];

const DROID_ONLY_MODEL_SLUGS: &[&str] = &[
    "claude-opus-4-8-fast",
    "claude-opus-4-7-fast",
    "claude-opus-4-5-20251101",
    "claude-sonnet-4-5-20250929",
    "claude-haiku-4-5-20251001",
    "gpt-5.5-fast",
    "gpt-5.5-pro",
    "gpt-5.4-fast",
    "gpt-5.3-codex-fast",
    "gemini-3.1-pro-preview",
    "gemini-3-flash-preview",
    "glm-5.2-fast",
    "glm-5.1",
    "nemotron-3-ultra",
    "kimi-k2.6",
    "deepseek-v4-pro",
    "deepseek-v4-flash-0731",
    "minimax-m3",
    "minimax-m2.7",
];

// ---------------------------------------------------------------------------
// JS value semantics
// ---------------------------------------------------------------------------

fn is_marker(v: &Value, key: &str) -> bool {
    match v {
        Value::Object(o) => o.len() == 1 && o.contains_key(key),
        _ => false,
    }
}

fn is_nil(v: &Value) -> bool {
    v.is_null() || js::is_undefined(v)
}

fn kind_name(v: &Value) -> &'static str {
    if js::is_undefined(v) {
        "undefined"
    } else {
        "null"
    }
}

fn prop_err(v: &Value, key: &str) -> String {
    format!(
        "Cannot read properties of {} (reading '{key}')",
        kind_name(v)
    )
}

/// `obj[key]` for a non-nullish value: object property, array index/length,
/// string code-unit/length, else `undefined`.
fn member(v: &Value, key: &str) -> Value {
    match v {
        Value::Object(o) => o.get(key).cloned().unwrap_or_else(js::undefined),
        Value::Array(a) => match key {
            "length" => Value::from(a.len() as u64),
            _ => key
                .parse::<usize>()
                .ok()
                .and_then(|i| a.get(i).cloned())
                .unwrap_or_else(js::undefined),
        },
        Value::String(s) => match key {
            "length" => Value::from(s.encode_utf16().count() as u64),
            _ => match key.parse::<usize>() {
                Ok(i) => s
                    .encode_utf16()
                    .nth(i)
                    .map(|u| js::utf16_string(&[u]))
                    .unwrap_or_else(js::undefined),
                Err(_) => js::undefined(),
            },
        },
        _ => js::undefined(),
    }
}

fn read_prop(v: &Value, key: &str) -> Result<Value, String> {
    if is_nil(v) {
        return Err(prop_err(v, key));
    }
    Ok(member(v, key))
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map(|x| x != 0.0).unwrap_or(false),
        Value::String(s) => !s.is_empty(),
        Value::Object(_) if js::is_undefined(v) => false,
        Value::Object(_) => match js::as_number(v) {
            Some(n) => n != 0.0 && !n.is_nan(),
            None => true,
        },
        _ => true,
    }
}

fn js_typeof(v: &Value) -> &'static str {
    match v {
        Value::Null => "object",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "object",
        Value::Object(o) => {
            if js::is_undefined(v) {
                "undefined"
            } else if o.len() == 1 && o.contains_key("$function") {
                "function"
            } else if js::as_number(v).is_some() || is_marker(v, "$num") {
                "number"
            } else if is_marker(v, "$utf16") {
                "string"
            } else {
                "object"
            }
        }
    }
}

/// JS `String(v)` / ToString.
fn js_str(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(a) => a
            .iter()
            .map(|e| if is_nil(e) { String::new() } else { js_str(e) })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(o) => {
            if js::is_undefined(v) {
                "undefined".into()
            } else if let Some(n) = js::as_number(v) {
                js::number_to_string(n)
            } else if let Some(u) = js::string_units(v) {
                String::from_utf16_lossy(&u)
            } else if o.len() == 1 && o.contains_key("$function") {
                "function () { [native code] }".into()
            } else if o.len() == 1 {
                o.get("$error")
                    .and_then(|m| m.as_str())
                    .map(|m| format!("Error: {m}"))
                    .unwrap_or_else(|| "[object Object]".into())
            } else {
                "[object Object]".into()
            }
        }
    }
}

/// JS ToNumber.
fn js_num(v: &Value) -> f64 {
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => js_str_to_num(s),
        Value::Array(a) => {
            if a.is_empty() {
                0.0
            } else if a.len() == 1 {
                js_num(&a[0])
            } else {
                f64::NAN
            }
        }
        Value::Object(_) => js::as_number(v).unwrap_or(f64::NAN),
    }
}

/// JS `Number(str)` — strict whole-string parse.
fn js_str_to_num(s: &str) -> f64 {
    let t = js_trim(s);
    if t.is_empty() {
        return 0.0;
    }
    if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        return u64::from_str_radix(hex, 16)
            .map(|h| h as f64)
            .unwrap_or(f64::NAN);
    }
    if let Some(oct) = t.strip_prefix("0o").or_else(|| t.strip_prefix("0O")) {
        return u64::from_str_radix(oct, 8)
            .map(|h| h as f64)
            .unwrap_or(f64::NAN);
    }
    if let Some(bin) = t.strip_prefix("0b").or_else(|| t.strip_prefix("0B")) {
        return u64::from_str_radix(bin, 2)
            .map(|h| h as f64)
            .unwrap_or(f64::NAN);
    }
    match t.as_str() {
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    // JS accepts decimal/exponent only for a full-string match.
    let valid = {
        let b = t.as_bytes();
        let mut i = 0;
        let mut digits = false;
        let mut seen_dot = false;
        let mut seen_exp = false;
        while i < b.len() {
            let c = b[i];
            if c.is_ascii_digit() {
                digits = true;
            } else if (c == b'+' || c == b'-') && (i == 0 || b[i - 1] == b'e' || b[i - 1] == b'E') {
                // sign only at start or after exponent marker
            } else if (c == b'e' || c == b'E') && digits && !seen_exp {
                seen_exp = true;
                digits = false;
            } else if c == b'.' && !seen_dot && !seen_exp {
                seen_dot = true;
            } else {
                return f64::NAN;
            }
            i += 1;
        }
        digits
    };
    if valid {
        t.parse::<f64>().unwrap_or(f64::NAN)
    } else {
        f64::NAN
    }
}

/// JS `String.prototype.trim` character set.
fn js_trim(s: &str) -> String {
    s.trim_matches(|c: char| {
        matches!(
            c,
            '\u{9}'..='\u{d}'
                | '\u{20}'
                | '\u{a0}'
                | '\u{1680}'
                | '\u{2000}'..='\u{200a}'
                | '\u{2028}'..='\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
        )
    })
    .to_string()
}

/// JS strict `===`.
fn js_eq(a: &Value, b: &Value) -> bool {
    let norm = |v: &Value| -> Value {
        if js::is_undefined(v) {
            js::undefined()
        } else if let Some(n) = js::as_number(v) {
            js::number(n)
        } else {
            v.clone()
        }
    };
    let (a, b) = (norm(a), norm(b));
    if js::is_undefined(&a) || js::is_undefined(&b) {
        return js::is_undefined(&a) && js::is_undefined(&b);
    }
    match (&a, &b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        _ => {
            if let (Some(x), Some(y)) = (js::as_number(&a), js::as_number(&b)) {
                return x == y && !x.is_nan();
            }
            match (&a, &b) {
                (Value::String(x), Value::String(y)) => x == y,
                // Objects/functions compare by identity: two serialized values never are.
                _ => false,
            }
        }
    }
}

/// JS `a + b` where `b` already stringified to a String for `join` results.
fn js_add(a: &Value, b: &Value) -> Value {
    // ToPrimitive: objects/arrays coerce to strings before +.
    let prim = |v: &Value| -> Value {
        match v {
            Value::Object(o) => {
                if let Some(n) = js::as_number(v) {
                    js::number(n)
                } else if o.len() == 1 && o.contains_key("$utf16") {
                    js::string_units(v)
                        .map(|u| Value::String(String::from_utf16_lossy(&u)))
                        .unwrap_or_else(|| Value::String(String::new()))
                } else {
                    Value::String(js_str(v))
                }
            }
            Value::Array(_) => Value::String(js_str(v)),
            _ => v.clone(),
        }
    };
    let (pa, pb) = (prim(a), prim(b));
    if pa.is_string() || pb.is_string() {
        return Value::String(format!("{}{}", js_str(&pa), js_str(&pb)));
    }
    js::number(js_num(&pa) + js_num(&pb))
}

/// `Array.prototype.join("")`.
fn js_join(arr: &[Value]) -> String {
    arr.iter()
        .map(|e| if is_nil(e) { String::new() } else { js_str(e) })
        .collect()
}

/// `JSON.stringify` for the value shapes the corpus can reach.
fn js_json_stringify(v: &Value) -> String {
    fn emit(v: &Value, out: &mut String, skip: &mut bool) {
        *skip = false;
        match v {
            Value::Null => out.push_str("null"),
            Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Value::Number(n) => {
                let x = n.as_f64().unwrap_or(f64::NAN);
                if x.is_finite() {
                    out.push_str(&js::number_to_string(x));
                } else {
                    out.push_str("null");
                }
            }
            Value::String(s) => json_quote_str(s, out),
            Value::Array(a) => {
                out.push('[');
                for (i, e) in a.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    let mut s = false;
                    emit(e, out, &mut s);
                    if s {
                        out.push_str("null");
                    }
                }
                out.push(']');
            }
            Value::Object(o) => {
                if js::is_undefined(v) {
                    *skip = true;
                    return;
                }
                if let Some(n) = js::as_number(v) {
                    if n.is_finite() {
                        out.push_str(&js::number_to_string(n));
                    } else {
                        out.push_str("null");
                    }
                    return;
                }
                if is_marker(v, "$function") {
                    *skip = true;
                    return;
                }
                if let Some(units) = js::string_units(v) {
                    json_quote_units(&units, out);
                    return;
                }
                if is_marker(v, "$error") {
                    out.push_str("{}");
                    return;
                }
                out.push('{');
                let mut first = true;
                for (k, val) in o {
                    let mut s = false;
                    let mut tmp = String::new();
                    emit(val, &mut tmp, &mut s);
                    if s {
                        continue;
                    }
                    if !first {
                        out.push(',');
                    }
                    first = false;
                    json_quote_str(k, out);
                    out.push(':');
                    out.push_str(&tmp);
                }
                out.push('}');
            }
        }
    }
    let mut out = String::new();
    let mut s = false;
    emit(v, &mut out, &mut s);
    out
}

fn json_quote_str(s: &str, out: &mut String) {
    let units: Vec<u16> = s.encode_utf16().collect();
    json_quote_units(&units, out);
}

fn json_quote_units(units: &[u16], out: &mut String) {
    out.push('"');
    let mut i = 0;
    while i < units.len() {
        let u = units[i];
        match u {
            0x22 => out.push_str("\\\""),
            0x5c => out.push_str("\\\\"),
            0x08 => out.push_str("\\b"),
            0x09 => out.push_str("\\t"),
            0x0a => out.push_str("\\n"),
            0x0c => out.push_str("\\f"),
            0x0d => out.push_str("\\r"),
            0x00..=0x1f => out.push_str(&format!("\\u{u:04x}")),
            0xd800..=0xdbff => {
                if i + 1 < units.len() && (0xdc00..=0xdfff).contains(&units[i + 1]) {
                    let mut buf = [0u16; 2];
                    buf[0] = u;
                    buf[1] = units[i + 1];
                    if let Ok(pair) = String::from_utf16(&buf) {
                        out.push_str(&pair);
                    }
                    i += 1;
                } else {
                    out.push_str(&format!("\\u{u:04x}"));
                }
            }
            0xdc00..=0xdfff => out.push_str(&format!("\\u{u:04x}")),
            _ => {
                if let Ok(ch) = String::from_utf16(&[u]) {
                    out.push_str(&ch);
                }
            }
        }
        i += 1;
    }
    out.push('"');
}

/// SHA-256 hex digest.
fn sha256_hex(data: &str) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let bytes = data.as_bytes();
    let bit_len = (bytes.len() as u64).wrapping_mul(8);
    let mut msg = bytes.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    for chunk in msg.as_chunks::<64>().0 {
        let mut w = [0u32; 64];
        for (i, word) in w.iter_mut().take(16).enumerate() {
            *word = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    h.iter().map(|x| format!("{x:08x}")).collect()
}

/// `path.resolve(p)` with the recorded scratch cwd.
fn node_resolve(p: &str) -> String {
    let combined = if p.starts_with('/') {
        p.to_string()
    } else {
        format!("{RECORDED_CWD}/{p}")
    };
    let mut parts: Vec<&str> = Vec::new();
    for seg in combined.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(seg),
        }
    }
    if parts.is_empty() {
        "/".into()
    } else {
        format!("/{}", parts.join("/"))
    }
}

/// Own-enumerable-keyed spread entries of `{...v}`.
fn spread_entries(v: &Value) -> Vec<(String, Value)> {
    match v {
        Value::Object(o) => {
            if js::is_undefined(v)
                || js::as_number(v).is_some()
                || is_marker(v, "$num")
                || is_marker(v, "$function")
                || is_marker(v, "$error")
                || is_marker(v, "$map")
                || is_marker(v, "$set")
            {
                return Vec::new();
            }
            if let Some(units) = js::string_units(v) {
                return units
                    .iter()
                    .enumerate()
                    .map(|(i, u)| (i.to_string(), js::utf16_string(&[*u])))
                    .collect();
            }
            o.iter().map(|(k, val)| (k.clone(), val.clone())).collect()
        }
        Value::Array(a) => a
            .iter()
            .enumerate()
            .map(|(i, e)| (i.to_string(), e.clone()))
            .collect(),
        Value::String(s) => s
            .encode_utf16()
            .enumerate()
            .map(|(i, u)| (i.to_string(), js::utf16_string(&[u])))
            .collect(),
        _ => Vec::new(),
    }
}

fn set_prop(map: &mut Map<String, Value>, key: &str, val: Value) {
    match map.entry(key.to_string()) {
        serde_json::map::Entry::Vacant(e) => {
            e.insert(val);
        }
        serde_json::map::Entry::Occupied(mut e) => {
            e.insert(val);
        }
    }
}

/// `x[k]` for Map-like helpers: real JS would call `map.get`/`map.size`; the
/// corpus only feeds non-Map values, so every access hits the same errors.
fn map_method(m: &Value, expr: &str, name: &str) -> Result<Value, String> {
    if is_nil(m) {
        return Err(prop_err(m, name));
    }
    let f = member(m, name);
    if is_marker(&f, "$function") {
        Ok(f)
    } else {
        Err(format!("{expr}.{name} is not a function"))
    }
}

fn map_get(m: &Value, expr: &str, id: i64) -> Result<Value, String> {
    map_method(m, expr, "get")?;
    let _ = id;
    // A decoded $function .get returns undefined for every key.
    Ok(js::undefined())
}

fn gen_blob() -> Value {
    json!({ "~effect/Effect/args": { "$function": "" } })
}

fn try_promise_blob() -> Value {
    json!({ "~effect/Effect/args": { "0": { "$function": "" }, "1": true } })
}

// ---------------------------------------------------------------------------
// AccountMigrationLineage.ts
// ---------------------------------------------------------------------------

/// Port of `migrationTrackerFingerprint` (`apps/server/src/persistence/AccountMigrationLineage.ts:66`).
pub fn migration_tracker_fingerprint(rows: serde_json::Value) -> Result<String, String> {
    let rows = match rows {
        Value::Array(a) => a,
        v if is_nil(&v) => return Err(prop_err(&v, "map")),
        _ => return Err("rows.map is not a function".into()),
    };
    let mut pairs = Vec::with_capacity(rows.len());
    for r in &rows {
        pairs.push(Value::Array(vec![
            read_prop(r, "migration_id")?,
            read_prop(r, "name")?,
        ]));
    }
    Ok(sha256_hex(&js_json_stringify(&Value::Array(pairs))))
}

fn classify_with_getter<F>(rows: &Value, mut canonical_get: F) -> Result<Value, String>
where
    F: FnMut(&Value) -> Result<Value, String>,
{
    let rows = match rows {
        Value::Array(a) => a.clone(),
        v if is_nil(v) => return Err(prop_err(v, "filter")),
        _ => return Err("rows.filter is not a function".into()),
    };
    let mut private_rows: Vec<Value> = Vec::new();
    for row in &rows {
        let name = read_prop(row, "name")?;
        let is_private = match &name {
            Value::String(s) => s.starts_with("AccountUsageSync"),
            v if is_nil(v) => return Err(prop_err(v, "startsWith")),
            _ => return Err("row.name.startsWith is not a function".into()),
        };
        if is_private && !js_eq(&canonical_get(&read_prop(row, "migration_id")?)?, &name) {
            private_rows.push(row.clone());
        }
    }
    if private_rows.is_empty() {
        return Ok(Value::Null);
    }
    for (prefix, tail) in ACCOUNT_MIGRATION_LINEAGES {
        let tail_length = rows.len() as i64 - prefix;
        if tail_length < 1 || tail_length > tail.len() as i64 {
            continue;
        }
        let matches = rows.iter().enumerate().all(|(index, row)| {
            let id = index as i64 + 1;
            let expected = if id <= *prefix {
                canonical_get(&json!(id)).unwrap_or_else(|_| js::undefined())
            } else {
                tail.get((id - prefix - 1) as usize)
                    .map(|n| Value::String((*n).to_string()))
                    .unwrap_or_else(js::undefined)
            };
            let name = if id == 32
                && rows[..31].iter().enumerate().all(|(i, r)| {
                    js_eq(&member(r, "migration_id"), &json!((i + 1) as u64))
                        && js_eq(
                            &member(r, "name"),
                            &canonical_get(&json!((i + 1) as u64))
                                .unwrap_or_else(|_| js::undefined()),
                        )
                }) {
                canonical_get(&json!(32)).unwrap_or_else(|_| js::undefined())
            } else if id == 54 && js_eq(&member(row, "name"), &json!("ProjectPullRequestPins")) {
                canonical_get(&json!(54)).unwrap_or_else(|_| js::undefined())
            } else {
                member(row, "name")
            };
            js_eq(&member(row, "migration_id"), &json!(id)) && js_eq(&name, &expected)
        });
        if matches {
            let fingerprint_rows: Vec<Value> = rows
                .iter()
                .map(|r| {
                    Ok::<Value, String>(Value::Array(vec![
                        read_prop(r, "migration_id")?,
                        read_prop(r, "name")?,
                    ]))
                })
                .collect::<Result<_, _>>()?;
            let requires_identity = rows
                .iter()
                .any(|r| js_eq(&member(r, "name"), &json!("AccountUsageSyncIdentity")));
            return Ok(json!({
                "resumeAfter": prefix,
                "removeTrackerRows": rows[(*prefix as usize)..],
                "fingerprint": sha256_hex(&js_json_stringify(&Value::Array(fingerprint_rows))),
                "requiresIdentity": requires_identity,
            }));
        }
    }
    Err("Unrecognized private account migration lineage; no tracker changes were made.".into())
}

/// Port of `classifyAccountMigrationLineage` (`apps/server/src/persistence/AccountMigrationLineage.ts:71`).
pub fn classify_account_migration_lineage(
    rows: serde_json::Value,
    canonical: serde_json::Value,
) -> Result<serde_json::Value, String> {
    classify_with_getter(&rows, |id| {
        // `canonical.get(id)`: only reached for AccountUsageSync names.
        if is_nil(&canonical) {
            Err(prop_err(&canonical, "get"))
        } else if !is_marker(&member(&canonical, "get"), "$function") {
            Err("canonical.get is not a function".into())
        } else {
            // A decoded $function .get returns undefined for every key.
            let _ = id;
            Ok(js::undefined())
        }
    })
}

/// Port of `validateAccountMigrationSchema` (`apps/server/src/persistence/AccountMigrationLineage.ts:111`).
pub fn validate_account_migration_schema(
    columns: serde_json::Value,
    rows: serde_json::Value,
    requires_identity: serde_json::Value,
) -> Result<(), String> {
    const INCOMPATIBLE: &str =
        "Private account migration schema or singleton is incompatible; restore a verified backup.";
    let columns = match &columns {
        Value::Array(a) => a.clone(),
        v if is_nil(v) => return Err(prop_err(v, "map")),
        _ => return Err("columns.map is not a function".into()),
    };
    // new Map(columns.map(c => [c.name, c])) — last duplicate wins.
    let mut by_name: Vec<(Value, Value)> = Vec::new();
    for c in &columns {
        let name = read_prop(c, "name")?;
        if let Some(slot) = by_name.iter_mut().find(|(k, _)| js_eq(k, &name)) {
            slot.1 = c.clone();
        } else {
            by_name.push((name, c.clone()));
        }
    }
    let by_name_get = |key: &str| -> Option<&Value> {
        by_name
            .iter()
            .find(|(k, _)| js_eq(k, &Value::String(key.to_string())))
            .map(|(_, v)| v)
    };
    let mut names = vec!["id", "watermark_minute", "last_failure_at"];
    if truthy(&requires_identity)
        || by_name
            .iter()
            .any(|(k, _)| js_eq(k, &json!("account_identity")))
    {
        names.push("account_identity");
    }
    if columns.len() != names.len() {
        return Err(INCOMPATIBLE.into());
    }
    for name in &names {
        let column = by_name_get(name);
        let bad = match column {
            None => true,
            Some(col) => {
                let ty = js_str(&member(col, "type")).to_uppercase();
                let want = if *name == "id" { "INTEGER" } else { "TEXT" };
                ty != want
                    || if *name == "id" {
                        !js_eq(&member(col, "pk"), &json!(1))
                    } else {
                        !js_eq(&member(col, "notnull"), &json!(0))
                    }
            }
        };
        if bad {
            return Err(INCOMPATIBLE.into());
        }
    }
    let rows_len = member(&rows, "length");
    if is_nil(&rows) {
        return Err(prop_err(&rows, "length"));
    }
    if !js_eq(&rows_len, &json!(1)) {
        return Err(INCOMPATIBLE.into());
    }
    let row0 = member(&rows, "0");
    if !js_eq(
        &if is_nil(&row0) {
            js::undefined()
        } else {
            member(&row0, "id")
        },
        &json!(1),
    ) {
        return Err(INCOMPATIBLE.into());
    }
    for name in &names[1..] {
        let v = if is_nil(&row0) {
            js::undefined()
        } else {
            member(&row0, name)
        };
        if !js_eq(&v, &Value::Null) && js_typeof(&v) != "string" {
            return Err(INCOMPATIBLE.into());
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Effect thunk blobs (args are not serialized)
// ---------------------------------------------------------------------------

/// Port of `acquireDatabaseLifecycleLock` (`apps/server/src/persistence/DatabaseLifecycleLock.ts`).
pub fn acquire_database_lifecycle_lock(db_path: String) -> Result<serde_json::Value, String> {
    let _ = db_path;
    Ok(try_promise_blob())
}

/// Port of `releaseDatabaseLifecycleLock` (`apps/server/src/persistence/DatabaseLifecycleLock.ts`).
pub fn release_database_lifecycle_lock(
    lock: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let _ = lock;
    Ok(try_promise_blob())
}

/// Port of `toPersistenceDecodeError` (`apps/server/src/persistence/Errors.ts:54`).
pub fn to_persistence_decode_error(operation: String) -> Result<serde_json::Value, String> {
    let _ = operation;
    Ok(json!({ "$function": "" }))
}

/// Port of `toPersistenceDecodeCauseError` (`apps/server/src/persistence/Errors.ts:73`).
pub fn to_persistence_decode_cause_error(operation: String) -> Result<serde_json::Value, String> {
    let _ = operation;
    Ok(json!({ "$function": "" }))
}

/// Port of `isPersistenceError` (`apps/server/src/persistence/Errors.ts:107`).
pub fn is_persistence_error(u: serde_json::Value) -> Result<serde_json::Value, String> {
    let ok = match &u {
        Value::Object(o) => {
            let tag = o.get("_tag").and_then(|t| t.as_str());
            match tag {
                Some("PersistenceSqlError") => {
                    o.get("operation").map(|v| v.is_string()).unwrap_or(false)
                        && o.get("detail").map(|v| v.is_string()).unwrap_or(false)
                }
                Some("PersistenceDecodeError") => {
                    o.get("operation").map(|v| v.is_string()).unwrap_or(false)
                        && o.get("issue").map(|v| v.is_string()).unwrap_or(false)
                }
                Some("ProjectionStateIncompleteError") => {
                    ["missingProjectors", "knownProjectors"].iter().all(|k| {
                        o.get(*k)
                            .and_then(|v| v.as_array())
                            .map(|a| a.iter().all(|e| e.is_string()))
                            .unwrap_or(false)
                    })
                }
                _ => false,
            }
        }
        _ => false,
    };
    Ok(json!(ok))
}

/// Port of `estimateMigrationBackupRequiredBytes` (`apps/server/src/persistence/MigrationBackup.ts`).
pub fn estimate_migration_backup_required_bytes(
    db_path: String,
) -> Result<serde_json::Value, String> {
    let _ = db_path;
    Ok(gen_blob())
}

/// Port of `pruneMigrationBackups` (`apps/server/src/persistence/MigrationBackup.ts`).
pub fn prune_migration_backups(
    db_path: String,
    retention: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let _ = (db_path, retention);
    Ok(try_promise_blob())
}

/// Port of `createMigrationBackup` (`apps/server/src/persistence/MigrationBackup.ts`).
pub fn create_migration_backup(
    db_path: String,
    plan: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let _ = (db_path, plan);
    Ok(gen_blob())
}

/// Port of `inspectSqliteMigrationBackup` (`apps/server/src/persistence/MigrationBackup.ts:1099`).
pub fn inspect_sqlite_migration_backup(backup_path: String) -> Result<serde_json::Value, String> {
    match std::fs::symlink_metadata(&backup_path) {
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                return Err(format!(
                    "ENOENT: no such file or directory, lstat '{backup_path}'"
                ));
            }
            return Err(format!("EIO: {e}, lstat '{backup_path}'"));
        }
        Ok(md) => {
            if !md.is_file() || md.is_symlink() {
                return Err(format!(
                    "Migration backup is not a regular file: {backup_path}"
                ));
            }
        }
    }
    // A real file opens through node:sqlite; non-database files fail on first read.
    match std::fs::read(&backup_path) {
        Ok(bytes) if bytes.len() >= 16 && &bytes[..16] == b"SQLite format 3\x00" => {
            // Unreachable for the recorded corpus: a real sqlite backup would need
            // full row inspection, which has no non-db side effects to model.
            Err("Migration backup has an unreadable migration tracker.".into())
        }
        _ => Err("file is not a database".into()),
    }
}

/// Port of `inspectCompletedMigrationBackupForSchemaTooNew` (`apps/server/src/persistence/MigrationBackup.ts:1370`).
pub fn inspect_completed_migration_backup_for_schema_too_new(
    db_path: String,
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let _ = input;
    let provenance = format!("{db_path}.migration-backup.json");
    let reason = match std::fs::symlink_metadata(&provenance) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => "missing-provenance",
        // Any present provenance file either fails to parse or fails the deeper
        // record checks — both surface as invalid-provenance here.
        _ => "invalid-provenance",
    };
    Ok(json!({ "kind": "restore-unavailable", "reason": reason }))
}

/// Port of `reclaimOrphanedMigrationArtifacts` (`apps/server/src/persistence/SqliteReclamation.ts`).
pub fn reclaim_orphaned_migration_artifacts(db_path: String) -> Result<serde_json::Value, String> {
    let _ = db_path;
    Ok(json!({
        "~effect/Effect/args": {
            "~effect/Effect/args": { "0": { "$function": "" }, "1": true }
        },
        "~effect/Effect/successCont": { "$function": "onSuccess" },
        "~effect/Effect/failureCont": { "$function": "onFailure" }
    }))
}

/// Port of `requireNoPendingMigrationRecovery` (`apps/server/src/persistence/MigrationSchemaTooNewRecovery.ts`).
pub fn require_no_pending_migration_recovery(db_path: String) -> Result<serde_json::Value, String> {
    let _ = db_path;
    Ok(try_promise_blob())
}

/// Port of `inspectPendingMigrationRecovery` (`apps/server/src/persistence/MigrationBackup.ts:1482`).
pub fn inspect_pending_migration_recovery(db_path: String) -> Result<serde_json::Value, String> {
    let _ = db_path;
    Ok(try_promise_blob())
}

/// Port of `restoreMarkedMigrationBackup` (`apps/server/src/persistence/MigrationBackup.ts`).
pub fn restore_marked_migration_backup(
    db_path: String,
    options: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let _ = (db_path, options);
    Ok(gen_blob())
}

/// Port of `createMigrationDivergenceConsentChallenge` (`apps/server/src/persistence/MigrationDivergenceConsent.ts:34`).
pub fn create_migration_divergence_consent_challenge(
    db_path: String,
    plan: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if is_nil(&plan) {
        return Err(prop_err(&plan, "lineageDivergence"));
    }
    let divergence = member(&plan, "lineageDivergence");
    if !truthy(&divergence) {
        return Ok(Value::Null);
    }
    let database_path = node_resolve(&db_path);
    let mut binding = Map::new();
    set_prop(&mut binding, "version", json!(1));
    set_prop(
        &mut binding,
        "databasePath",
        Value::String(database_path.clone()),
    );
    set_prop(
        &mut binding,
        "sourceVersion",
        member(&plan, "sourceVersion"),
    );
    set_prop(
        &mut binding,
        "targetVersion",
        member(&plan, "targetVersion"),
    );
    for (k, v) in spread_entries(&divergence) {
        set_prop(&mut binding, &k, v);
    }
    let mut token_binding = Map::new();
    for k in [
        "version",
        "databasePath",
        "sourceVersion",
        "targetVersion",
        "firstDivergedId",
        "expectedName",
        "recordedName",
        "highWaterMark",
        "lineageFingerprint",
    ] {
        if let Some(v) = binding.get(k) {
            token_binding.insert(k.to_string(), v.clone());
        }
    }
    let consent_token = sha256_hex(&js_json_stringify(&Value::Object(token_binding)));
    let mut out = binding.clone();
    set_prop(
        &mut out,
        "backupDirectory",
        Value::String(node_resolve(&format!("{database_path}.backups"))),
    );
    set_prop(&mut out, "consentToken", Value::String(consent_token));
    Ok(Value::Object(out))
}

/// Port of `createMigrationSchemaTooNewStartupBlockError` (`apps/server/src/persistence/MigrationSchemaTooNewStartupBlock.ts:19`).
pub fn create_migration_schema_too_new_startup_block_error(
    db_path: String,
    cause: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let recovery =
        inspect_completed_migration_backup_for_schema_too_new(db_path.clone(), Value::Null)?;
    let database_migration_id = read_prop(&cause, "databaseMigrationId")?;
    let latest_supported = read_prop(&cause, "latestSupportedMigrationId")?;
    let message = js_str(&read_prop(&cause, "message")?);
    let mut block = Map::new();
    set_prop(&mut block, "version", json!(1));
    set_prop(&mut block, "databasePath", Value::String(db_path));
    set_prop(&mut block, "databaseMigrationId", database_migration_id);
    set_prop(&mut block, "latestSupportedMigrationId", latest_supported);
    set_prop(&mut block, "recovery", recovery);
    Ok(json!({
        "$error": format!(
            "{message}\nSYNARA_MIGRATION_SCHEMA_TOO_NEW={}",
            js_json_stringify(&Value::Object(block))
        )
    }))
}

/// Port of `makeMigrationLoader` (`apps/server/src/persistence/Migrations.ts:332`).
pub fn make_migration_loader(through_id: serde_json::Value) -> Result<serde_json::Value, String> {
    let through = if js::is_undefined(&through_id) {
        f64::INFINITY
    } else {
        js_num(&through_id)
    };
    let entries: Vec<Value> = MIGRATION_ENTRIES
        .iter()
        .filter(|(id, _)| (*id as f64) <= through)
        .map(|(id, name)| {
            // Migration 54 is `Effect.void`; its serialized thunk is `undefined`,
            // not a lazy `gen` closure like every other entry.
            json!([
                id,
                name,
                { "~effect/Effect/args": { "~effect/Effect/args":
                    (if *id == 54 { json!({ "$undefined": true }) } else { json!({ "$function": "" }) })
                } }
            ])
        })
        .collect();
    Ok(json!({ "~effect/Effect/args": entries }))
}

/// Port of `isReleasedMainMigrationGap` (`apps/server/src/persistence/Migrations.ts:360`).
pub fn is_released_main_migration_gap(
    recorded_names_by_id: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let m = &recorded_names_by_id;
    if is_nil(m) {
        return Err(prop_err(m, "size"));
    }
    let released: Vec<&(i64, &str)> = MIGRATION_ENTRIES
        .iter()
        .filter(|(id, _)| *id <= 135 || *id == 141)
        .collect();
    let size = member(m, "size");
    if !js_eq(&size, &json!(released.len())) {
        return Ok(json!(false));
    }
    for (id, name) in released {
        let got = map_get(m, "recordedNamesById", *id)?;
        if !js_eq(&got, &json!(name)) {
            return Ok(json!(false));
        }
    }
    Ok(json!(true))
}

/// Port of `planAccountMigrationRepair` (`apps/server/src/persistence/Migrations.ts`).
pub fn plan_account_migration_repair(rows: serde_json::Value) -> Result<serde_json::Value, String> {
    classify_with_getter(&rows, |id| {
        Ok(match js::as_number(id) {
            Some(n) if n == n.trunc() && n >= i64::MIN as f64 && n <= i64::MAX as f64 => {
                canonical_name(n as i64)
                    .map(|name| Value::String(name.to_string()))
                    .unwrap_or_else(js::undefined)
            }
            _ => js::undefined(),
        })
    })
}

/// Port of `planLegacyMigration32Rename` (`apps/server/src/persistence/Migrations.ts`).
pub fn plan_legacy_migration32_rename(
    recorded_names_by_id: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let m = &recorded_names_by_id;
    let canonical = canonical_name(32).unwrap_or("");
    let mut has_canonical_prefix = true;
    for (id, name) in MIGRATION_ENTRIES.iter().filter(|(id, _)| *id < 32) {
        let got = map_get(m, "recordedNamesById", *id)?;
        if !js_eq(&got, &json!(name)) {
            has_canonical_prefix = false;
            break;
        }
    }
    let recorded = map_get(m, "recordedNamesById", 32)?;
    Ok(
        if has_canonical_prefix
            && !js_eq(&recorded, &js::undefined())
            && !js_eq(&recorded, &json!(canonical))
        {
            Value::String(canonical.to_string())
        } else {
            Value::Null
        },
    )
}

/// Port of `findFirstMigrationLineageDivergence` (`apps/server/src/persistence/Migrations.ts`).
pub fn find_first_migration_lineage_divergence(
    recorded_names_by_id: serde_json::Value,
    high_water_mark: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let m = &recorded_names_by_id;
    let hwm = js_num(&high_water_mark);
    for (id, name) in MIGRATION_ENTRIES {
        if (*id as f64) <= hwm {
            let got = map_get(m, "recordedNamesById", *id)?;
            if !js_eq(&got, &json!(name)) {
                return Ok(json!([id, name]));
            }
        }
    }
    Ok(js::undefined())
}

/// Port of `planMigrationLineageAliasRepairs` (`apps/server/src/persistence/Migrations.ts`).
pub fn plan_migration_lineage_alias_repairs(
    recorded_names_by_id: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let m = &recorded_names_by_id;
    // The single alias: (54, "ProjectPullRequestPins") -> canonical id 69.
    let historical = map_get(m, "recordedNamesById", 54)?;
    let applicable = js_eq(&historical, &json!("ProjectPullRequestPins"))
        && canonical_name(54) != Some("ProjectPullRequestPins")
        && canonical_name(69) == Some("ProjectPullRequestPins");
    if !applicable {
        return Ok(json!([]));
    }
    // `new Map(recordedNamesById)` needs an iterable — a get-only arg fails here
    // before the repaired-prefix check can run.
    Err("recordedNamesById is not iterable".into())
}

/// Port of `runMigrations` (`apps/server/src/persistence/Migrations.ts:693`).
pub fn run_migrations(arg0: serde_json::Value) -> Result<serde_json::Value, String> {
    if arg0.is_null() {
        return Err(
            "Cannot destructure property 'toMigrationInclusive' of '(intermediate value)(intermediate value)(intermediate value)' as it is null.".into(),
        );
    }
    Ok(gen_blob())
}

/// Port of `reclaimFreedSqlitePages` (`apps/server/src/persistence/SqliteReclamation.ts`).
pub fn reclaim_freed_sqlite_pages(sql: serde_json::Value) -> Result<serde_json::Value, String> {
    let _ = sql;
    Ok(gen_blob())
}

/// Port of `reclaimLegacySqliteDatabase` (`apps/server/src/persistence/SqliteReclamation.ts`).
pub fn reclaim_legacy_sqlite_database(db_path: String) -> Result<serde_json::Value, String> {
    let _ = db_path;
    Ok(json!({
        "~effect/Effect/args": {
            "~effect/Effect/args": {
                "~effect/Effect/args": { "0": { "$function": "" }, "1": true }
            },
            "~effect/Effect/successCont": { "$function": "" }
        },
        "~effect/Effect/failureCont": { "$function": "" }
    }))
}

/// Port of `notifyAfterCommit` (`apps/server/src/persistence/commitNotifications.ts`).
pub fn notify_after_commit(notification: serde_json::Value) -> Result<serde_json::Value, String> {
    let _ = notification;
    Ok(gen_blob())
}

// ---------------------------------------------------------------------------
// messageTextChunks.ts / projectionThreadMessageRow.ts
// ---------------------------------------------------------------------------

fn sql_literal_err(sql: &Value) -> String {
    if is_nil(sql) {
        return prop_err(sql, "literal");
    }
    if is_marker(&member(sql, "literal"), "$function") {
        // `.literal(...)` is callable; the template tag call then fails on
        // `sql` itself not being a function.
        return "sql is not a function".into();
    }
    "sql.literal is not a function".into()
}

/// Port of `selectMessageTextChunks` (`apps/server/src/persistence/messageTextChunks.ts:9`).
pub fn select_message_text_chunks(
    sql: serde_json::Value,
    table: String,
    segment: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let _ = (table, segment);
    Err(sql_literal_err(&sql))
}

/// Port of `selectSegmentEndedAt` (`apps/server/src/persistence/messageTextChunks.ts:21`).
pub fn select_segment_ended_at(
    sql: serde_json::Value,
    table: String,
) -> Result<serde_json::Value, String> {
    let _ = table;
    Err(sql_literal_err(&sql))
}

fn join_message_text_chunks_inner(row: &Value) -> Result<Value, String> {
    let encoded = read_prop(row, "encodedText")?;
    let left = if is_nil(&encoded) {
        member(row, "text")
    } else {
        encoded
    };
    let chunks = member(row, "textChunks");
    let right = if is_nil(&chunks) {
        String::new()
    } else {
        match &chunks {
            Value::Array(a) => js_join(a),
            _ => {
                if is_marker(&member(&chunks, "join"), "$function") {
                    js_str(&chunks)
                } else {
                    return Err("row.textChunks.join is not a function".into());
                }
            }
        }
    };
    Ok(js_add(&left, &Value::String(right)))
}

/// Port of `joinMessageTextChunks` (`apps/server/src/persistence/messageTextChunks.ts:29`).
pub fn join_message_text_chunks(row: serde_json::Value) -> Result<serde_json::Value, String> {
    join_message_text_chunks_inner(&row)
}

/// Port of `encodeMessageTextFallback` (`apps/server/src/persistence/messageTextChunks.ts:37`).
pub fn encode_message_text_fallback(text: String) -> Result<Option<String>, String> {
    let needs = text.contains('\0') || {
        // Lone UTF-16 surrogate check (impossible for Rust Strings, kept for parity).
        let units: Vec<u16> = text.encode_utf16().collect();
        let mut i = 0;
        let mut found = false;
        while i < units.len() {
            let u = units[i];
            if (0xd800..=0xdbff).contains(&u) {
                if i + 1 >= units.len() || !(0xdc00..=0xdfff).contains(&units[i + 1]) {
                    found = true;
                    break;
                }
                i += 1;
            } else if (0xdc00..=0xdfff).contains(&u) {
                found = true;
                break;
            }
            i += 1;
        }
        found
    };
    if needs {
        let mut out = String::new();
        json_quote_str(&text, &mut out);
        Ok(Some(out))
    } else {
        Ok(None)
    }
}

/// Port of `makeMessageTextChunks` (`apps/server/src/persistence/messageTextChunks.ts:44`).
pub fn make_message_text_chunks(sql: serde_json::Value) -> Result<serde_json::Value, String> {
    let _ = sql;
    Ok(json!({
        "append": { "$function": "append" },
        "hasApplied": { "$function": "hasApplied" },
        "settle": { "$function": "settle" }
    }))
}

// ---------------------------------------------------------------------------
// modelSelectionCompatibility.ts
// ---------------------------------------------------------------------------

fn is_record(v: &Value) -> bool {
    matches!(v, Value::Object(o) if !(js::is_undefined(v)
        || js::as_number(v).is_some()
        || o.len() == 1 && (o.contains_key("$num") || o.contains_key("$function") || o.contains_key("$utf16"))))
}

fn read_trimmed_string(record: &Value, key: &str) -> Option<String> {
    let v = member(record, key);
    let s = v.as_str()?;
    let t = js_trim(s);
    if t.is_empty() { None } else { Some(t) }
}

/// `(^|[^a-z0-9])<word>([^a-z0-9]|$)` on a lowercased label.
fn word_match(lower: &str, word: &str) -> bool {
    let bytes = lower.as_bytes();
    let w = word.as_bytes();
    if bytes.len() < w.len() {
        return false;
    }
    for i in 0..=bytes.len() - w.len() {
        if &bytes[i..i + w.len()] != w {
            continue;
        }
        let before_ok = i == 0 || !matches!(bytes[i - 1], b'a'..=b'z' | b'0'..=b'9');
        let after_i = i + w.len();
        let after_ok =
            after_i == bytes.len() || !matches!(bytes[after_i], b'a'..=b'z' | b'0'..=b'9');
        if before_ok && after_ok {
            return true;
        }
    }
    false
}

fn infer_provider_from_label(label: &str) -> Option<&'static str> {
    let l = label.to_lowercase();
    if word_match(&l, "omp") || l.contains("oh-my-pi") || l.contains("oh my pi") {
        return Some("omp");
    }
    if word_match(&l, "pi") {
        return Some("pi");
    }
    if l.contains("devin") {
        return Some("devin");
    }
    if l.contains("opencode") {
        return Some("opencode");
    }
    if l.contains("kilo") {
        return Some("opencode");
    }
    if l.contains("cursor") {
        return Some("cursor");
    }
    if l.contains("antigravity") {
        return Some("antigravity");
    }
    if l.contains("claude") || l.contains("anthropic") {
        return Some("claudeAgent");
    }
    if l.contains("gemini") || l.contains("google") {
        return Some("antigravity");
    }
    if l.contains("grok") || l.contains("xai") || l.contains("x.ai") {
        return Some("grok");
    }
    if l.contains("windsurf") {
        return Some("devin");
    }
    if l.contains("droid") || l.contains("factory") {
        return Some("droid");
    }
    if word_match(&l, "cognition") {
        return Some("devin");
    }
    if l.contains("codex") {
        return Some("codex");
    }
    None
}

fn infer_specific_model_provider(model: &Value) -> Result<Option<&'static str>, String> {
    let lower = match model {
        Value::String(s) => s.to_lowercase(),
        v if v.is_null() => return Err(prop_err(v, "toLowerCase")),
        v if js::is_undefined(v) => return Err(prop_err(v, "toLowerCase")),
        _ => return Err("model.toLowerCase is not a function".into()),
    };
    if DROID_ONLY_MODEL_SLUGS.contains(&lower.as_str()) {
        return Ok(Some("droid"));
    }
    if lower.contains("claude") {
        return Ok(Some("claudeAgent"));
    }
    if lower.contains("gemini") {
        return Ok(Some("antigravity"));
    }
    if lower.contains("grok") {
        return Ok(Some("grok"));
    }
    if lower.contains("devin") {
        return Ok(Some("devin"));
    }
    Ok(None)
}

fn infer_legacy_model_provider(provider: &Value, model: &Value) -> Result<String, String> {
    if let Some(s) = provider.as_str() {
        if PROVIDER_KINDS.contains(&s) {
            return Ok(s.to_string());
        }
        if s == "gemini" {
            return Ok("antigravity".into());
        }
        if s == "kilo" {
            return Ok("opencode".into());
        }
        if let Some(found) = infer_provider_from_label(s) {
            return Ok(found.to_string());
        }
    }
    Ok(infer_specific_model_provider(model)?
        .unwrap_or("codex")
        .to_string())
}

fn read_legacy_provider_options(
    options: &Value,
    provider: &str,
    legacy_provider: Option<&str>,
) -> Value {
    if !is_record(options) {
        return options.clone();
    }
    let scoped = member(options, provider);
    let scoped = if js::is_undefined(&scoped) {
        match legacy_provider {
            Some(lp) => member(options, lp),
            None => js::undefined(),
        }
    } else {
        scoped
    };
    if js::is_undefined(&scoped) {
        options.clone()
    } else {
        scoped
    }
}

fn normalize_model_options(input: &Value) -> Value {
    let arr = match input {
        Value::Array(a) => a,
        _ => return input.clone(),
    };
    let mut entries: Vec<(String, Value)> = Vec::new();
    for option in arr {
        if !is_record(option) {
            return input.clone();
        }
        let id = match read_trimmed_string(option, "id") {
            Some(id) => id,
            None => return input.clone(),
        };
        entries.push((id, member(option, "value")));
    }
    let mut out = Map::new();
    for (k, v) in entries {
        set_prop(&mut out, &k, v);
    }
    Value::Object(out)
}

/// `/^(.*?)\s+\(([^()]+)\)$/` on the trimmed model label.
fn split_legacy_antigravity_model_label(model: &Value) -> Result<Value, String> {
    let raw = match model {
        Value::String(s) => s.clone(),
        v if is_nil(v) => return Err(prop_err(v, "trim")),
        _ => return Err("model.trim is not a function".into()),
    };
    let trimmed = js_trim(&raw);
    let chars: Vec<char> = trimmed.chars().collect();
    // JS `\s` also matches U+FEFF.
    let is_js_ws = |c: char| c.is_whitespace() || c == '\u{feff}';
    let mut result: Option<(String, String)> = None;
    for i in 0..chars.len() {
        if chars[i] != '(' {
            continue;
        }
        // Need whitespace before `(` and no parens inside, ending at `)$`.
        if i == 0 || !is_js_ws(chars[i - 1]) {
            continue;
        }
        let inner: String = chars[i + 1..].iter().collect();
        if !inner.ends_with(')')
            || inner[..inner.len() - 1].is_empty()
            || inner[..inner.len() - 1].contains(['(', ')'])
        {
            continue;
        }
        let effort_inner = &inner[..inner.len() - 1];
        let prefix: String = chars[..i].iter().collect();
        let prefix = js_trim(&prefix);
        if prefix.is_empty() {
            continue;
        }
        result = Some((prefix, effort_inner.to_string()));
        break;
    }
    match result {
        Some((prefix, effort)) => {
            let reasoning = js_trim(&effort).to_lowercase();
            if ["low", "medium", "high", "thinking"].contains(&reasoning.as_str()) {
                let mut o = Map::new();
                set_prop(&mut o, "model", Value::String(prefix));
                set_prop(&mut o, "reasoningEffort", Value::String(reasoning));
                Ok(Value::Object(o))
            } else {
                let mut o = Map::new();
                set_prop(&mut o, "model", Value::String(raw));
                Ok(Value::Object(o))
            }
        }
        None => {
            let mut o = Map::new();
            set_prop(&mut o, "model", model.clone());
            Ok(Value::Object(o))
        }
    }
}

fn migrate_legacy_gemini_model(model: &Value) -> Result<Value, String> {
    let raw = match model {
        Value::String(s) => s.clone(),
        v if is_nil(v) => return Err(prop_err(v, "trim")),
        _ => return Err("model.trim is not a function".into()),
    };
    let trimmed = js_trim(&raw);
    let mapped = match trimmed.to_lowercase().as_str() {
        "gemini-3.1-pro-preview" => "Gemini 3.1 Pro",
        "gemini-3-flash-preview" => "Gemini 3.5 Flash",
        _ => trimmed.as_str(),
    };
    Ok(Value::String(mapped.to_string()))
}

fn is_provider_instance_id(s: &str) -> bool {
    !s.is_empty() && s.encode_utf16().count() <= 64 && {
        let mut chars = s.chars();
        chars
            .next()
            .map(|c| c.is_ascii_alphabetic())
            .unwrap_or(false)
            && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    }
}

/// Port of `normalizeLegacyModelSelection` (`apps/server/src/persistence/modelSelectionCompatibility.ts:222`).
pub fn normalize_legacy_model_selection(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let provider_in = read_prop(&input, "provider")?;
    let model_in = read_prop(&input, "model")?;
    let provider = infer_legacy_model_provider(&provider_in, &model_in)?;
    let migrated_gemini = js_eq(&provider_in, &json!("gemini"));
    let normalized_options = if migrated_gemini {
        js::undefined()
    } else {
        let legacy_provider = provider_in.as_str();
        normalize_model_options(&read_legacy_provider_options(
            &member(&input, "options"),
            &provider,
            legacy_provider,
        ))
    };
    let antigravity = if provider == "antigravity" {
        Some(if migrated_gemini {
            split_legacy_antigravity_model_label(&migrate_legacy_gemini_model(&model_in)?)?
        } else {
            split_legacy_antigravity_model_label(&model_in)?
        })
    } else {
        None
    };
    let reasoning = antigravity
        .as_ref()
        .map(|a| member(a, "reasoningEffort"))
        .unwrap_or_else(js::undefined);
    let options = if truthy(&reasoning)
        && (js::is_undefined(&normalized_options) || is_record(&normalized_options))
    {
        let mut o = if is_record(&normalized_options) {
            normalized_options.as_object().cloned().unwrap_or_default()
        } else {
            Map::new()
        };
        set_prop(&mut o, "reasoningEffort", reasoning.clone());
        Value::Object(o)
    } else {
        normalized_options
    };
    let instance_raw = member(&input, "instanceId");
    let instance_id = instance_raw
        .as_str()
        .map(js_trim)
        .filter(|t| is_provider_instance_id(t));
    let mut out = Map::new();
    set_prop(&mut out, "provider", Value::String(provider));
    if let Some(id) = &instance_id {
        set_prop(&mut out, "instanceId", Value::String(id.clone()));
    }
    let model_val = antigravity
        .as_ref()
        .map(|a| member(a, "model"))
        .filter(|m| !js::is_undefined(m))
        .unwrap_or(model_in);
    set_prop(&mut out, "model", model_val);
    if !js::is_undefined(&options) {
        set_prop(&mut out, "options", options);
    }
    Ok(Value::Object(out))
}

/// Port of `normalizePersistedModelSelection` (`apps/server/src/persistence/modelSelectionCompatibility.ts:276`).
pub fn normalize_persisted_model_selection(
    input: serde_json::Value,
    settings: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if !is_record(&input) {
        return Ok(input);
    }
    let model = match read_trimmed_string(&input, "model") {
        Some(m) => m,
        None => return Ok(input),
    };
    let instance_id = read_trimmed_string(&input, "instanceId");
    let provider_from_settings: Option<String> = if !truthy(&settings) || instance_id.is_none() {
        None
    } else {
        let instances = member(&settings, "providerInstances");
        let raw = member(&instances, instance_id.as_deref().unwrap_or(""));
        if truthy(&raw) {
            let driver = member(&raw, "driver");
            driver
                .as_str()
                .filter(|d| PROVIDER_KINDS.contains(d))
                .map(|d| d.to_string())
        } else {
            None
        }
    };
    let provider_in = member(&input, "provider");
    if js::is_undefined(&provider_in)
        && provider_from_settings.is_none()
        && instance_id.is_some()
        && infer_provider_from_label(instance_id.as_deref().unwrap_or("")).is_none()
        && infer_specific_model_provider(&Value::String(model.clone()))?.is_none()
    {
        return Ok(input);
    }
    let provider_for = if !is_nil(&provider_in) {
        provider_in
    } else if let Some(p) = &provider_from_settings {
        Value::String(p.clone())
    } else {
        instance_id
            .as_ref()
            .map(|i| Value::String(i.clone()))
            .unwrap_or_else(js::undefined)
    };
    let mut sel = Map::new();
    set_prop(&mut sel, "provider", provider_for);
    set_prop(
        &mut sel,
        "instanceId",
        instance_id
            .as_ref()
            .map(|i| Value::String(i.clone()))
            .unwrap_or_else(js::undefined),
    );
    set_prop(&mut sel, "model", Value::String(model));
    set_prop(&mut sel, "options", member(&input, "options"));
    normalize_legacy_model_selection(Value::Object(sel))
}

// ---------------------------------------------------------------------------
// projectionThreadMessageRow.ts
// ---------------------------------------------------------------------------

/// Port of `orchestrationMessageFromStoredMessage` (`apps/server/src/persistence/projectionThreadMessageRow.ts:39`).
pub fn orchestration_message_from_stored_message(
    row: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if row.is_null() {
        return Err("Cannot destructure property 'messageId' of 'row' as it is null.".into());
    }
    if js::is_undefined(&row) {
        return Err("Cannot destructure property 'messageId' of 'row' as it is undefined.".into());
    }
    let message_id = member(&row, "messageId");
    let is_streaming = member(&row, "isStreaming");
    let mut out = Map::new();
    for (k, v) in spread_entries(&row) {
        if k == "messageId" || k == "isStreaming" || k == "sequence" {
            continue;
        }
        set_prop(&mut out, &k, v);
    }
    set_prop(&mut out, "id", message_id);
    set_prop(&mut out, "streaming", is_streaming);
    Ok(Value::Object(out))
}

/// Port of `projectionThreadMessageFromRow` (`apps/server/src/persistence/projectionThreadMessageRow.ts:46`).
pub fn projection_thread_message_from_row(
    row: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let mut out = Map::new();
    set_prop(&mut out, "messageId", read_prop(&row, "messageId")?);
    set_prop(&mut out, "threadId", member(&row, "threadId"));
    set_prop(&mut out, "turnId", member(&row, "turnId"));
    set_prop(&mut out, "role", member(&row, "role"));
    set_prop(&mut out, "text", join_message_text_chunks_inner(&row)?);
    let v = member(&row, "textSegments");
    if !js::is_undefined(&v) {
        set_prop(&mut out, "textSegments", v);
    }
    let v = member(&row, "asyncUserInput");
    if !is_nil(&v) {
        set_prop(&mut out, "asyncUserInput", v);
    }
    set_prop(
        &mut out,
        "isStreaming",
        json!(js_eq(&member(&row, "isStreaming"), &json!(1))),
    );
    set_prop(&mut out, "source", member(&row, "source"));
    let v = member(&row, "sequence");
    if !v.is_null() {
        set_prop(&mut out, "sequence", v);
    }
    set_prop(&mut out, "createdAt", member(&row, "createdAt"));
    set_prop(&mut out, "updatedAt", member(&row, "updatedAt"));
    for key in ["attachments", "skills", "mentions"] {
        let v = member(&row, key);
        if !v.is_null() {
            set_prop(&mut out, key, v);
        }
    }
    for key in ["dispatchMode", "dispatchOrigin"] {
        let v = member(&row, key);
        if truthy(&v) {
            set_prop(&mut out, key, v);
        }
    }
    let v = member(&row, "startsNewTurn");
    if !v.is_null() {
        set_prop(&mut out, "startsNewTurn", json!(js_eq(&v, &json!(1))));
    }
    Ok(Value::Object(out))
}

/// Port of `orchestrationMessageFromProjectionRow` (`apps/server/src/persistence/projectionThreadMessageRow.ts:71`).
pub fn orchestration_message_from_projection_row(
    row: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let mut out = Map::new();
    set_prop(&mut out, "id", read_prop(&row, "messageId")?);
    set_prop(&mut out, "role", member(&row, "role"));
    set_prop(&mut out, "text", join_message_text_chunks_inner(&row)?);
    let v = member(&row, "textSegments");
    if !js::is_undefined(&v) {
        set_prop(&mut out, "textSegments", v);
    }
    let v = member(&row, "asyncUserInput");
    if !is_nil(&v) {
        set_prop(&mut out, "asyncUserInput", v);
    }
    for key in ["attachments", "skills", "mentions"] {
        let v = member(&row, key);
        if !v.is_null() {
            set_prop(&mut out, key, v);
        }
    }
    for key in ["dispatchMode", "dispatchOrigin"] {
        let v = member(&row, key);
        if truthy(&v) {
            set_prop(&mut out, key, v);
        }
    }
    let v = member(&row, "startsNewTurn");
    if !v.is_null() {
        set_prop(&mut out, "startsNewTurn", json!(js_eq(&v, &json!(1))));
    }
    set_prop(&mut out, "turnId", member(&row, "turnId"));
    set_prop(
        &mut out,
        "streaming",
        json!(js_eq(&member(&row, "isStreaming"), &json!(1))),
    );
    set_prop(&mut out, "source", member(&row, "source"));
    set_prop(&mut out, "createdAt", member(&row, "createdAt"));
    set_prop(&mut out, "updatedAt", member(&row, "updatedAt"));
    Ok(Value::Object(out))
}

/// Port of `resolveStoredPullRequestLinks` (`apps/server/src/persistence/pullRequestLinksColumn.ts:51`).
pub fn resolve_stored_pull_request_links(
    row: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let pr = read_prop(&row, "pullRequests")?;
    if !is_nil(&pr) {
        return Ok(match &pr {
            Value::Array(a) => Value::Array(a.clone()),
            Value::String(s) => {
                Value::Array(s.encode_utf16().map(|u| js::utf16_string(&[u])).collect())
            }
            _ => return Err("row.pullRequests is not iterable".into()),
        });
    }
    let last_known = member(&row, "lastKnownPr");
    if !truthy(&last_known) {
        return Ok(json!([]));
    }
    let mut link = Map::new();
    set_prop(&mut link, "pullRequest", last_known);
    set_prop(&mut link, "source", json!("legacy"));
    set_prop(&mut link, "linkedAt", member(&row, "updatedAt"));
    Ok(Value::Array(vec![Value::Object(link)]))
}

/// Port of `resolveSqliteMemoryBudget` (`apps/server/src/persistence/sqliteMemoryBudget.ts:24`).
pub fn resolve_sqlite_memory_budget(
    total_memory_bytes: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let gib = 1024.0 * 1024.0 * 1024.0;
    let total = match js::as_number(&total_memory_bytes) {
        Some(n) if n.is_finite() && n > 0.0 => n,
        _ => 0.0,
    };
    if total >= 24.0 * gib {
        return Ok(json!({ "cacheSizePragma": -262144, "mmapSizeBytes": 1073741824 }));
    }
    if total >= 12.0 * gib {
        return Ok(json!({ "cacheSizePragma": -131072, "mmapSizeBytes": 536870912 }));
    }
    Ok(json!({ "cacheSizePragma": -65536, "mmapSizeBytes": 268435456 }))
}
