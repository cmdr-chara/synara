use serde_json::{Map, Value};

const PROVIDERS: [&str; 11] = [
    "codex",
    "claudeAgent",
    "cursor",
    "antigravity",
    "gemini",
    "grok",
    "droid",
    "kilo",
    "opencode",
    "pi",
    "devin",
];

fn identity_keys(provider: &str) -> &'static [&'static str] {
    match provider {
        "codex" => &["homePath", "shadowHomePath", "environment"],
        "claudeAgent" => &["homePath", "environment"],
        "cursor" => &["apiEndpoint", "environment"],
        "kilo" | "opencode" => &["serverUrl", "serverPassword", "environment"],
        "pi" => &["agentDir", "environment"],
        _ => &["environment"],
    }
}

fn is_record(v: &Value) -> bool {
    v.is_object()
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}' || c == '\u{00A0}')
}

fn non_empty_string(v: &Value) -> Option<String> {
    v.as_str()
        .map(js_trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn is_provider(v: &Value) -> Option<&'static str> {
    v.as_str()
        .and_then(|s| PROVIDERS.iter().find(|p| **p == s).copied())
}

fn identity_field_state(options: &Map<String, Value>, key: &str) -> &'static str {
    if !options.contains_key(key) {
        return "absent";
    }
    let value = &options[key];
    if key == "environment" {
        return if is_record(value) {
            "present"
        } else {
            "invalid"
        };
    }
    if non_empty_string(value).is_none() {
        "invalid"
    } else {
        "present"
    }
}

fn canonical_selection(selection: &Value, instance_id: &str) -> Value {
    let mut out = match selection {
        Value::Object(m) => m.clone(),
        _ => Map::new(),
    };
    out.remove("provider");
    out.insert(
        "instanceId".to_string(),
        Value::String(instance_id.to_string()),
    );
    Value::Object(out)
}

/// Port of `resolveAutomationProviderIdentity`.
pub fn resolve_automation_provider_identity(
    model_selection_value: Value,
    provider_options_value: Value,
    provider_hint_value: Option<Value>,
) -> Result<Value, String> {
    let hint_defined = provider_hint_value
        .as_ref()
        .is_some_and(|v| !crate::js::is_undefined(v));
    let hint = provider_hint_value.as_ref().unwrap_or(&Value::Null);

    if !is_record(&model_selection_value) {
        return Ok(serde_json::json!({"safe": false}));
    }
    if non_empty_string(model_selection_value.get("model").unwrap_or(&Value::Null)).is_none() {
        return Ok(serde_json::json!({"safe": false}));
    }
    let raw_instance_id = model_selection_value
        .get("instanceId")
        .unwrap_or(&Value::Null);
    let instance_id = non_empty_string(raw_instance_id);
    if !raw_instance_id.is_null()
        && !crate::js::is_undefined(raw_instance_id)
        && raw_instance_id.as_str() != Some("")
        && instance_id.is_none()
    {
        return Ok(serde_json::json!({"safe": false}));
    }
    if let Some(id) = &instance_id
        && !is_provider(&Value::String(id.clone())).is_none()
    {
        return Ok(serde_json::json!({
            "safe": true,
            "modelSelection": canonical_selection(&model_selection_value, id),
        }));
    }
    let provider = match &instance_id {
        Some(id) => is_provider(&Value::String(id.clone())),
        None => None,
    }
    .or_else(|| model_selection_value.get("provider").and_then(is_provider));
    let Some(provider) = provider else {
        return Ok(serde_json::json!({"safe": false}));
    };
    let ms_provider = model_selection_value.get("provider");
    if (ms_provider.is_some_and(|p| !crate::js::is_undefined(p))
        && (is_provider(ms_provider.unwrap_or(&Value::Null)).is_none()
            || ms_provider.unwrap_or(&Value::Null).as_str() != Some(provider)))
        || (hint_defined && (is_provider(hint).is_none() || hint.as_str() != Some(provider)))
    {
        return Ok(serde_json::json!({"safe": false}));
    }

    if provider_options_value.is_null() || crate::js::is_undefined(&provider_options_value) {
        return Ok(serde_json::json!({
            "safe": true,
            "modelSelection": canonical_selection(&model_selection_value, provider),
        }));
    }
    let Value::Object(options_map) = &provider_options_value else {
        return Ok(serde_json::json!({"safe": false}));
    };
    let selected_options = options_map.get(provider).unwrap_or(&Value::Null);
    if selected_options.is_null() || crate::js::is_undefined(selected_options) {
        return Ok(serde_json::json!({
            "safe": true,
            "modelSelection": canonical_selection(&model_selection_value, provider),
        }));
    }
    let Value::Object(selected) = selected_options else {
        return Ok(serde_json::json!({"safe": false}));
    };
    if provider == "codex" && selected.contains_key("accountId") {
        let account_id = non_empty_string(&selected["accountId"]);
        let has_malformed = identity_keys("codex")
            .iter()
            .any(|key| identity_field_state(selected, key) == "invalid");
        let has_env = identity_field_state(selected, "environment") == "present";
        if account_id.is_none() || has_malformed || has_env {
            return Ok(serde_json::json!({"safe": false}));
        }
        return Ok(serde_json::json!({"safe": false}));
    }
    let carries_ambiguous = identity_keys(provider)
        .iter()
        .any(|key| matches!(identity_field_state(selected, key), "present" | "invalid"));
    if carries_ambiguous {
        Ok(serde_json::json!({"safe": false}))
    } else {
        Ok(serde_json::json!({
            "safe": true,
            "modelSelection": canonical_selection(&model_selection_value, provider),
        }))
    }
}

fn provider_for_unresolved_identity(
    model_selection_value: &Value,
    provider_options_value: Option<&Value>,
    provider_hint_value: Option<&Value>,
) -> &'static str {
    if let Some(hint) = provider_hint_value
        && let Some(p) = is_provider(hint)
    {
        return p;
    }
    if let Value::Object(sel) = model_selection_value {
        for key in ["instanceId", "provider"] {
            if let Some(p) = sel.get(key).and_then(is_provider) {
                return p;
            }
        }
    }
    if let Some(Value::Object(options)) = provider_options_value {
        let configured: Vec<&str> = PROVIDERS
            .iter()
            .filter(|p| options.contains_key(*p as &str))
            .copied()
            .collect();
        if configured.len() == 1 {
            return configured[0];
        }
    }
    "codex"
}

/// Port of `makeUnresolvedAutomationModelSelection`.
pub fn make_unresolved_automation_model_selection(
    model_selection_value: Value,
    provider_options_value: Option<Value>,
    provider_hint_value: Option<Value>,
) -> Result<Value, String> {
    let provider = provider_for_unresolved_identity(
        &model_selection_value,
        provider_options_value.as_ref(),
        provider_hint_value.as_ref(),
    );
    let model = if is_record(&model_selection_value) {
        model_selection_value
            .get("model")
            .and_then(non_empty_string)
            .unwrap_or_else(|| "legacy-automation-unresolved".to_string())
    } else {
        "legacy-automation-unresolved".to_string()
    };
    Ok(serde_json::json!({
        "provider": provider,
        "modelSelection": {
            "instanceId": format!("synara_unresolved_automation_{provider}"),
            "model": model,
        },
    }))
}

fn is_iso_datetime(s: &str) -> bool {
    let b = s.as_bytes();
    let (base, frac) = match b.len() {
        20 => (b, &b[19..]),
        24 => (&b[..19], &b[19..]),
        _ => return false,
    };
    let digits_ok = |slice: &[u8]| slice.iter().all(|c| c.is_ascii_digit());
    let shape_ok = base[4] == b'-'
        && base[7] == b'-'
        && base[10] == b'T'
        && base[13] == b':'
        && base[16] == b':'
        && digits_ok(&base[0..4])
        && digits_ok(&base[5..7])
        && digits_ok(&base[8..10])
        && digits_ok(&base[11..13])
        && digits_ok(&base[14..16])
        && digits_ok(&base[17..19]);
    let tail_ok = (b.len() == 20 && frac == b"Z")
        || (b.len() == 24 && frac[0] == b'.' && digits_ok(&frac[1..4]) && frac[4] == b'Z');
    shape_ok && tail_ok
}

/// Port of `makeUnresolvedAutomationPermissionSnapshot`.
pub fn make_unresolved_automation_permission_snapshot(
    snapshot_value: Value,
    created_at_fallback: String,
) -> Result<Value, String> {
    let empty = Map::new();
    let snapshot = match &snapshot_value {
        Value::Object(m) => m,
        _ => &empty,
    };
    let unresolved = make_unresolved_automation_model_selection(
        snapshot
            .get("modelSelection")
            .cloned()
            .unwrap_or(Value::Null),
        snapshot.get("providerOptions").cloned(),
        snapshot.get("provider").cloned(),
    )?;
    let created_at = match snapshot.get("createdAt").and_then(Value::as_str) {
        Some(s) if is_iso_datetime(s) => s.to_string(),
        _ if is_iso_datetime(&created_at_fallback) => created_at_fallback,
        _ => "1970-01-01T00:00:00.000Z".to_string(),
    };
    let mut seen: Vec<String> = Vec::new();
    if let Some(caps) = snapshot
        .get("allowedCapabilities")
        .and_then(Value::as_array)
    {
        for cap in caps {
            if let Some(s) = cap.as_str()
                && matches!(s, "send-turn" | "create-worktree" | "full-access")
                && !seen.iter().any(|x| x == s)
            {
                seen.push(s.to_string());
            }
        }
    }
    let mut out = Map::new();
    out.insert("provider".to_string(), unresolved["provider"].clone());
    out.insert(
        "modelSelection".to_string(),
        unresolved["modelSelection"].clone(),
    );
    if let Some(v) = snapshot.get("completionPolicyVersion")
        && let Some(n) = v.as_f64()
        && n.fract() == 0.0
        && n >= 0.0
    {
        out.insert("completionPolicyVersion".to_string(), v.clone());
    }
    out.insert(
        "runtimeMode".to_string(),
        Value::String(
            if snapshot.get("runtimeMode").and_then(Value::as_str) == Some("full-access") {
                "full-access".to_string()
            } else {
                "approval-required".to_string()
            },
        ),
    );
    out.insert(
        "interactionMode".to_string(),
        Value::String(
            if snapshot.get("interactionMode").and_then(Value::as_str) == Some("plan") {
                "plan".to_string()
            } else {
                "default".to_string()
            },
        ),
    );
    let wm = snapshot.get("worktreeMode").and_then(Value::as_str);
    out.insert(
        "worktreeMode".to_string(),
        Value::String(if wm == Some("local") || wm == Some("worktree") {
            wm.unwrap_or("auto").to_string()
        } else {
            "auto".to_string()
        }),
    );
    out.insert(
        "allowedCapabilities".to_string(),
        Value::Array(seen.into_iter().map(Value::String).collect()),
    );
    out.insert("createdAt".to_string(), Value::String(created_at));
    Ok(Value::Object(out))
}

/// Port of `parseJson`.
pub fn parse_json(value: String) -> Result<Value, String> {
    match serde_json::from_str::<Value>(&value) {
        Ok(v) => Ok(v),
        Err(_) => Ok(serde_json::json!({"$symbol": "InvalidAutomationJson"})),
    }
}

/// Port of `columnExists` — runs a SQL tag call, so any non-client input fails like JS.
pub fn column_exists(
    sql: Value,
    _table_name: String,
    _column_name: String,
) -> Result<Value, String> {
    let _ = &sql;
    Err("sql is not a function".into())
}

/// Port of `tableExists`.
pub fn table_exists(sql: Value, _table_name: String) -> Result<Value, String> {
    let _ = &sql;
    Err("sql is not a function".into())
}

/// Port of `primaryKeyColumns`.
pub fn primary_key_columns(sql: Value, _table_name: String) -> Result<Value, String> {
    let _ = &sql;
    Err("sql is not a function".into())
}
