//! The Rust port of `packages/shared/src/subagents.ts`.

#![forbid(unsafe_code)]

use std::collections::HashSet;

use serde_json::{Map, Value};

use crate::js;

fn trim_js(s: &str) -> &str {
    s.trim_matches(jscore::is_space)
}

fn as_trimmed_string(value: Option<&Value>) -> Option<String> {
    let s = value?.as_str()?;
    let t = trim_js(s);
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

fn first_string_value(object: Option<&Map<String, Value>>, keys: &[&str]) -> Option<String> {
    let object = object?;
    for key in keys {
        if let Some(value) = as_trimmed_string(object.get(*key)) {
            return Some(value);
        }
    }
    None
}

/// The `a ?? b` chain over `item[key]`: nullish entries are skipped.
fn nullish_chain<'a>(object: &'a Map<String, Value>, keys: &[&str]) -> Option<&'a Value> {
    for key in keys {
        match object.get(*key) {
            Some(v) if !v.is_null() && !js::is_undefined(v) => return Some(v),
            _ => continue,
        }
    }
    None
}

fn is_worker_tier_role_str(role: &str) -> bool {
    // /^worker-(?:low|medium|high|xhigh)$/i
    matches!(
        role.to_lowercase().as_str(),
        "worker-low" | "worker-medium" | "worker-high" | "worker-xhigh"
    )
}

fn sanitize_role(role: Option<String>) -> Option<String> {
    match role {
        Some(r) if is_worker_tier_role_str(trim_js(&r)) => None,
        other => other,
    }
}

fn spread(obj: &mut Map<String, Value>, key: &str, value: Option<Value>) {
    if let Some(v) = value {
        obj.insert(key.to_string(), v);
    }
}

/// Port of `isWorkerTierSubagentRole` (`packages/shared/src/subagents.ts:51`).
pub fn is_worker_tier_subagent_role(role: Option<String>) -> Result<bool, String> {
    Ok(role.is_some_and(|r| is_worker_tier_role_str(trim_js(&r))))
}

/// `extractSubagentIdentityFromSource` → (fields, keys-in-spread-order).
fn extract_identity_from_source(
    item: &Map<String, Value>,
) -> Option<(Map<String, Value>, Vec<&'static str>)> {
    let source = item.get("source").and_then(Value::as_object);
    let subagent = source
        .and_then(|s| s.get("subAgent").and_then(Value::as_object))
        .or_else(|| source.and_then(|s| s.get("sub_agent").and_then(Value::as_object)))
        .or_else(|| item.get("subAgent").and_then(Value::as_object));
    let thread_spawn = subagent
        .and_then(|s| s.get("thread_spawn").and_then(Value::as_object))
        .or_else(|| subagent.and_then(|s| s.get("threadSpawn").and_then(Value::as_object)));
    let provider_thread_id = nullish_chain(
        item,
        &[
            "threadId",
            "thread_id",
            "conversationId",
            "conversation_id",
            "receiverThreadId",
            "receiver_thread_id",
        ],
    )
    .and_then(|v| as_trimmed_string(Some(v)))
    .or_else(|| first_string_value(thread_spawn, &["threadId", "thread_id"]));
    let agent_id = nullish_chain(item, &["agentId", "agent_id", "id"])
        .and_then(|v| as_trimmed_string(Some(v)))
        .or_else(|| first_string_value(thread_spawn, &["agentId", "agent_id", "id"]))
        .or_else(|| first_string_value(subagent, &["agentId", "agent_id", "id"]));
    let nickname = first_string_value(Some(item), &["agentNickname", "agent_nickname", "nickname"])
        .or_else(|| {
            first_string_value(
                thread_spawn,
                &["agentNickname", "agent_nickname", "nickname", "name"],
            )
        })
        .or_else(|| {
            first_string_value(
                subagent,
                &["agentNickname", "agent_nickname", "nickname", "name"],
            )
        });
    let role = sanitize_role(
        first_string_value(
            Some(item),
            &["agentRole", "agent_role", "agentType", "agent_type"],
        )
        .or_else(|| {
            first_string_value(
                thread_spawn,
                &["agentRole", "agent_role", "agentType", "agent_type"],
            )
        })
        .or_else(|| {
            first_string_value(
                subagent,
                &["agentRole", "agent_role", "agentType", "agent_type"],
            )
        }),
    );
    if provider_thread_id.is_none() && agent_id.is_none() && nickname.is_none() && role.is_none() {
        return None;
    }
    let mut out = Map::new();
    let mut keys = Vec::new();
    for (key, value) in [
        ("providerThreadId", provider_thread_id),
        ("agentId", agent_id),
        ("nickname", nickname),
        ("role", role),
    ] {
        if let Some(v) = value {
            keys.push(key);
            out.insert(key.to_string(), Value::String(v));
        }
    }
    Some((out, keys))
}

/// Port of `decodeSubagentReceiverThreadIds` (`packages/shared/src/subagents.ts:147`).
pub fn decode_subagent_receiver_thread_ids(
    item: Option<serde_json::Value>,
) -> Result<Vec<String>, String> {
    let Some(item) = item else {
        return Ok(Vec::new());
    };
    if !jscore::js_truthy(&item) {
        return Ok(Vec::new());
    }
    let Some(object) = item.as_object() else {
        return Ok(Vec::new());
    };
    for key in [
        "receiverThreadIds",
        "receiver_thread_ids",
        "threadIds",
        "thread_ids",
    ] {
        let Some(values) = object.get(key).and_then(Value::as_array) else {
            continue;
        };
        let ids: Vec<String> = values
            .iter()
            .filter_map(|v| as_trimmed_string(Some(v)))
            .collect();
        if !ids.is_empty() {
            return Ok(ids);
        }
    }
    let singular = first_string_value(
        Some(object),
        &[
            "receiverThreadId",
            "receiver_thread_id",
            "threadId",
            "thread_id",
            "newThreadId",
            "new_thread_id",
        ],
    );
    Ok(singular.into_iter().collect())
}

/// Port of `decodeSubagentReceiverAgents` (`packages/shared/src/subagents.ts:178`).
pub fn decode_subagent_receiver_agents(
    item: serde_json::Value,
    fallback_thread_ids: Vec<String>,
) -> Result<Vec<serde_json::Value>, String> {
    if item.is_null() {
        return Err("Cannot read properties of null (reading 'receiverAgents')".to_string());
    }
    if js::is_undefined(&item) {
        return Err("Cannot read properties of undefined (reading 'receiverAgents')".to_string());
    }
    let object = item.as_object();
    let top_level_model = first_string_value(
        object,
        &[
            "model",
            "modelName",
            "model_name",
            "requestedModel",
            "requested_model",
        ],
    );
    let top_level_effort =
        first_string_value(object, &["effort", "reasoningEffort", "reasoning_effort"]);
    let top_level_prompt = first_string_value(object, &["prompt", "task", "message"]);
    let agents_value = object
        .and_then(|o| o.get("receiverAgents").and_then(Value::as_array))
        .or_else(|| object.and_then(|o| o.get("receiver_agents").and_then(Value::as_array)))
        .or_else(|| object.and_then(|o| o.get("agents").and_then(Value::as_array)));
    let item_background = object.is_some_and(|o| o.get("background") == Some(&Value::Bool(true)));
    let mut decoded: Vec<Value> = Vec::new();
    if let Some(entries) = agents_value {
        for (index, entry) in entries.iter().enumerate() {
            let Some(entry) = entry.as_object() else {
                continue;
            };
            let Some(provider_thread_id) = first_string_value(
                Some(entry),
                &[
                    "threadId",
                    "thread_id",
                    "receiverThreadId",
                    "receiver_thread_id",
                    "newThreadId",
                    "new_thread_id",
                ],
            )
            .or_else(|| fallback_thread_ids.get(index).cloned())
            .filter(|id| !id.is_empty()) else {
                continue;
            };
            let agent_id = first_string_value(
                Some(entry),
                &[
                    "agentId",
                    "agent_id",
                    "receiverAgentId",
                    "receiver_agent_id",
                    "newAgentId",
                    "new_agent_id",
                    "id",
                ],
            );
            let nickname = first_string_value(
                Some(entry),
                &[
                    "agentNickname",
                    "agent_nickname",
                    "receiverAgentNickname",
                    "receiver_agent_nickname",
                    "newAgentNickname",
                    "new_agent_nickname",
                    "nickname",
                    "name",
                ],
            );
            let role = sanitize_role(first_string_value(
                Some(entry),
                &[
                    "agentRole",
                    "agent_role",
                    "receiverAgentRole",
                    "receiver_agent_role",
                    "newAgentRole",
                    "new_agent_role",
                    "agentType",
                    "agent_type",
                ],
            ));
            let direct_model =
                first_string_value(Some(entry), &["model", "modelName", "model_name"]);
            let model = direct_model
                .clone()
                .or_else(|| first_string_value(Some(entry), &["requestedModel", "requested_model"]))
                .or_else(|| top_level_model.clone());
            let effort = first_string_value(
                Some(entry),
                &["effort", "reasoningEffort", "reasoning_effort"],
            )
            .or_else(|| top_level_effort.clone());
            let background = entry.get("background") == Some(&Value::Bool(true)) || item_background;
            let prompt = first_string_value(Some(entry), &["prompt", "task", "message"])
                .or_else(|| top_level_prompt.clone());
            let mut out = Map::new();
            out.insert(
                "providerThreadId".to_string(),
                Value::String(provider_thread_id),
            );
            spread(&mut out, "agentId", agent_id.map(Value::String));
            spread(&mut out, "nickname", nickname.map(Value::String));
            spread(&mut out, "role", role.map(Value::String));
            spread(&mut out, "model", model.clone().map(Value::String));
            spread(&mut out, "effort", effort.map(Value::String));
            if background {
                out.insert("background".to_string(), Value::Bool(true));
            }
            spread(&mut out, "prompt", prompt.map(Value::String));
            if model.is_some() && direct_model.is_none() {
                out.insert("modelIsRequestedHint".to_string(), Value::Bool(true));
            }
            decoded.push(Value::Object(out));
        }
    }
    if !decoded.is_empty() {
        return Ok(decoded);
    }

    let Some(provider_thread_id) = fallback_thread_ids.first().filter(|id| !id.is_empty()) else {
        return Ok(Vec::new());
    };
    let agent_id = first_string_value(
        object,
        &["newAgentId", "new_agent_id", "agentId", "agent_id"],
    );
    let nickname = first_string_value(
        object,
        &[
            "newAgentNickname",
            "new_agent_nickname",
            "agentNickname",
            "agent_nickname",
            "receiverAgentNickname",
            "receiver_agent_nickname",
        ],
    );
    let role = sanitize_role(first_string_value(
        object,
        &[
            "receiverAgentRole",
            "receiver_agent_role",
            "newAgentRole",
            "new_agent_role",
            "agentRole",
            "agent_role",
            "agentType",
            "agent_type",
        ],
    ));
    let mut out = Map::new();
    out.insert(
        "providerThreadId".to_string(),
        Value::String(provider_thread_id.clone()),
    );
    spread(&mut out, "agentId", agent_id.map(Value::String));
    spread(&mut out, "nickname", nickname.map(Value::String));
    spread(&mut out, "role", role.map(Value::String));
    if let Some(model) = &top_level_model {
        // `{model: topLevelModel, modelIsRequestedHint: true}` in a single spread.
        out.insert("model".to_string(), Value::String(model.clone()));
        out.insert("modelIsRequestedHint".to_string(), Value::Bool(true));
    }
    spread(&mut out, "effort", top_level_effort.map(Value::String));
    if item_background {
        out.insert("background".to_string(), Value::Bool(true));
    }
    spread(&mut out, "prompt", top_level_prompt.map(Value::String));
    Ok(vec![Value::Object(out)])
}

fn build_agent_state(thread_id: &str, object: Option<&Map<String, Value>>) -> Value {
    let mut out = Map::new();
    out.insert("threadId".to_string(), Value::String(thread_id.to_string()));
    spread(
        &mut out,
        "agentId",
        first_string_value(object, &["agentId", "agent_id"]).map(Value::String),
    );
    spread(
        &mut out,
        "nickname",
        first_string_value(
            object,
            &[
                "agentNickname",
                "agent_nickname",
                "receiverAgentNickname",
                "receiver_agent_nickname",
            ],
        )
        .map(Value::String),
    );
    spread(
        &mut out,
        "role",
        sanitize_role(first_string_value(
            object,
            &[
                "agentRole",
                "agent_role",
                "receiverAgentRole",
                "receiver_agent_role",
                "agentType",
                "agent_type",
            ],
        ))
        .map(Value::String),
    );
    spread(
        &mut out,
        "model",
        first_string_value(
            object,
            &[
                "model",
                "modelName",
                "model_name",
                "requestedModel",
                "requested_model",
            ],
        )
        .map(Value::String),
    );
    spread(
        &mut out,
        "prompt",
        first_string_value(object, &["prompt", "task", "message"]).map(Value::String),
    );
    spread(
        &mut out,
        "status",
        first_string_value(object, &["status", "state"]).map(Value::String),
    );
    spread(
        &mut out,
        "message",
        first_string_value(
            object,
            &["summary", "message", "latestUpdate", "latest_update"],
        )
        .map(Value::String),
    );
    Value::Object(out)
}

/// Port of `decodeSubagentAgentStates` (`packages/shared/src/subagents.ts:359`).
pub fn decode_subagent_agent_states(
    item: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let object = item.as_ref().and_then(|i| {
        if i.is_null() || js::is_undefined(i) {
            None
        } else {
            i.as_object()
        }
    });
    let candidate = [
        "statuses",
        "agentsStates",
        "agents_states",
        "agentStates",
        "agent_states",
    ]
    .iter()
    .find_map(|k| object.and_then(|o| o.get(*k)).and_then(Value::as_object));
    let mut decoded = Map::new();
    if let Some(candidate) = candidate {
        for (raw_thread_id, raw_value) in candidate {
            let entry = raw_value.as_object();
            let Some(thread_id) = as_trimmed_string(Some(&Value::String(raw_thread_id.clone())))
                .or_else(|| first_string_value(entry, &["threadId", "thread_id"]))
            else {
                continue;
            };
            decoded.insert(thread_id.clone(), build_agent_state(&thread_id, entry));
        }
        return Ok(Value::Object(decoded));
    }
    let values = ["agentStatuses", "agent_statuses", "statuses"]
        .iter()
        .find_map(|k| object.and_then(|o| o.get(*k)).and_then(Value::as_array));
    let Some(values) = values else {
        return Ok(Value::Object(Map::new()));
    };
    for raw_value in values {
        let entry = raw_value.as_object();
        let Some(thread_id) = first_string_value(entry, &["threadId", "thread_id"]) else {
            continue;
        };
        decoded.insert(thread_id.clone(), build_agent_state(&thread_id, entry));
    }
    Ok(Value::Object(decoded))
}

/// Port of `collectSubagentProviderThreadIds` (`packages/shared/src/subagents.ts:400`).
pub fn collect_subagent_provider_thread_ids(
    item: serde_json::Value,
) -> Result<Vec<String>, String> {
    let mut ordered: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let push = |id: Option<String>, ordered: &mut Vec<String>, seen: &mut HashSet<String>| {
        if let Some(id) = id
            && seen.insert(id.clone())
        {
            ordered.push(id);
        }
    };
    for id in decode_subagent_receiver_thread_ids(Some(item.clone()))? {
        push(Some(id), &mut ordered, &mut seen);
    }
    for agent in decode_subagent_receiver_agents(item.clone(), ordered.clone())? {
        push(
            agent
                .get("providerThreadId")
                .and_then(Value::as_str)
                .map(str::to_string),
            &mut ordered,
            &mut seen,
        );
    }
    if let Value::Object(states) = decode_subagent_agent_states(Some(item.clone()))? {
        for key in states.keys() {
            push(Some(key.clone()), &mut ordered, &mut seen);
        }
    }
    let source_identity = item
        .as_object()
        .and_then(|o| extract_identity_from_source(o).map(|(m, _)| m));
    push(
        source_identity
            .and_then(|m| m.get("providerThreadId").cloned())
            .and_then(|v| v.as_str().map(str::to_string)),
        &mut ordered,
        &mut seen,
    );
    push(
        first_string_value(
            item.as_object(),
            &[
                "newThreadId",
                "new_thread_id",
                "receiverThreadId",
                "receiver_thread_id",
            ],
        ),
        &mut ordered,
        &mut seen,
    );
    Ok(ordered)
}

fn dedup_key(hint: &Map<String, Value>) -> String {
    let s = |key: &str| match hint.get(key) {
        Some(v) if !v.is_null() && !js::is_undefined(v) => jscore::js_to_string(v),
        _ => String::new(),
    };
    let flag = |key: &str| {
        if hint.get(key).is_some_and(jscore::js_truthy) {
            "1"
        } else {
            "0"
        }
    };
    [
        s("providerThreadId"),
        s("agentId"),
        s("nickname"),
        s("role"),
        s("model"),
        s("effort"),
        flag("background").to_string(),
        s("prompt"),
        s("status"),
        s("message"),
        flag("modelIsRequestedHint").to_string(),
    ]
    .join("\u{1}")
}

const ALL_HINT_KEYS: [&str; 11] = [
    "providerThreadId",
    "agentId",
    "nickname",
    "role",
    "model",
    "effort",
    "background",
    "prompt",
    "status",
    "message",
    "modelIsRequestedHint",
];

/// Port of `extractSubagentIdentityHints` (`packages/shared/src/subagents.ts:433`).
pub fn extract_subagent_identity_hints(
    item: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    if item.is_null() {
        return Err("Cannot read properties of null (reading 'source')".to_string());
    }
    if js::is_undefined(&item) {
        return Err("Cannot read properties of undefined (reading 'source')".to_string());
    }
    let object = item.as_object();
    let mut hints: Vec<(Value, Vec<String>)> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let push = |value: Option<(Value, Vec<String>)>,
                hints: &mut Vec<(Value, Vec<String>)>,
                seen: &mut HashSet<String>| {
        let Some((v, keys)) = value else { return };
        let Some(m) = v.as_object() else { return };
        let key = dedup_key(m);
        if seen.insert(key) {
            hints.push((v, keys));
        }
    };

    // `extractSubagentIdentityFromSource` spreads only the defined fields.
    push(
        object
            .and_then(extract_identity_from_source)
            .map(|(m, keys)| {
                (
                    Value::Object(m),
                    keys.into_iter().map(str::to_string).collect(),
                )
            }),
        &mut hints,
        &mut seen,
    );

    // `{providerThreadId, agentId, nickname, role}` — all four keys present.
    let mut top = Map::new();
    for (key, value) in [
        (
            "providerThreadId",
            first_string_value(
                object,
                &[
                    "newThreadId",
                    "new_thread_id",
                    "receiverThreadId",
                    "receiver_thread_id",
                    "threadId",
                    "thread_id",
                ],
            ),
        ),
        (
            "agentId",
            first_string_value(
                object,
                &[
                    "newAgentId",
                    "new_agent_id",
                    "receiverAgentId",
                    "receiver_agent_id",
                    "agentId",
                    "agent_id",
                ],
            ),
        ),
        (
            "nickname",
            first_string_value(
                object,
                &[
                    "newAgentNickname",
                    "new_agent_nickname",
                    "receiverAgentNickname",
                    "receiver_agent_nickname",
                    "agentNickname",
                    "agent_nickname",
                    "nickname",
                ],
            ),
        ),
        (
            "role",
            sanitize_role(first_string_value(
                object,
                &[
                    "newAgentRole",
                    "new_agent_role",
                    "receiverAgentRole",
                    "receiver_agent_role",
                    "agentRole",
                    "agent_role",
                    "agentType",
                    "agent_type",
                ],
            )),
        ),
    ] {
        top.insert(
            key.to_string(),
            value.map(Value::String).unwrap_or_else(js::undefined),
        );
    }
    push(
        Some((
            Value::Object(top),
            ["providerThreadId", "agentId", "nickname", "role"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        )),
        &mut hints,
        &mut seen,
    );

    let receiver_ids = decode_subagent_receiver_thread_ids(Some(item.clone()))?;
    for agent in decode_subagent_receiver_agents(item.clone(), receiver_ids)? {
        let Some(m) = agent.as_object() else { continue };
        push(
            Some((agent.clone(), m.keys().cloned().collect::<Vec<String>>())),
            &mut hints,
            &mut seen,
        );
    }

    if let Value::Object(states) = decode_subagent_agent_states(Some(item.clone()))? {
        for state in states.values() {
            let s = state.as_object();
            let get = |k: &str| {
                s.and_then(|o| o.get(k).cloned())
                    .unwrap_or_else(js::undefined)
            };
            let mut hint = Map::new();
            for (key, src) in [
                ("providerThreadId", "threadId"),
                ("agentId", "agentId"),
                ("nickname", "nickname"),
                ("role", "role"),
                ("model", "model"),
                ("prompt", "prompt"),
                ("status", "status"),
                ("message", "message"),
            ] {
                hint.insert(key.to_string(), get(src));
            }
            push(
                Some((
                    Value::Object(hint),
                    [
                        "providerThreadId",
                        "agentId",
                        "nickname",
                        "role",
                        "model",
                        "prompt",
                        "status",
                        "message",
                    ]
                    .iter()
                    .map(|k| k.to_string())
                    .collect(),
                )),
                &mut hints,
                &mut seen,
            );
        }
    }

    Ok(hints
        .into_iter()
        .filter(|(v, _)| {
            let m = v.as_object();
            ["providerThreadId", "agentId", "nickname", "role"]
                .iter()
                .any(|k| {
                    m.and_then(|o| o.get(*k))
                        .is_some_and(|f| !js::is_undefined(f))
                })
        })
        .map(|(v, keys)| {
            let mut out = Map::new();
            let m = v.as_object();
            for key in &keys {
                out.insert(
                    key.clone(),
                    m.and_then(|o| o.get(key).cloned())
                        .unwrap_or_else(js::undefined),
                );
            }
            Value::Object(out)
        })
        .collect())
}

fn merge_hints(existing: Option<&Value>, incoming: &Value) -> Value {
    let ef = |key: &str| existing.and_then(|e| e.get(key)).cloned();
    let ifv = |key: &str| incoming.get(key).cloned();
    let merge_field = |key: &str| -> Value {
        match ifv(key) {
            Some(v) if !v.is_null() && !js::is_undefined(&v) => v,
            _ => ef(key).unwrap_or_else(js::undefined),
        }
    };
    let existing_model = ef("model");
    let incoming_model = ifv("model");
    let existing_mih = ef("modelIsRequestedHint");
    let incoming_mih = ifv("modelIsRequestedHint");
    // `selectMergedModel`: a falsy incoming model, or a requested-hint model
    // arriving over a direct one, keeps the existing pair.
    let keep_existing = incoming_model
        .as_ref()
        .is_none_or(|v| !jscore::js_truthy(v))
        || (incoming_mih == Some(Value::Bool(true))
            && existing_model
                .as_ref()
                .is_some_and(|v| !js::is_undefined(v))
            && existing_mih != Some(Value::Bool(true)));
    let (model, mih) = if keep_existing {
        (
            existing_model.unwrap_or_else(js::undefined),
            existing_mih.unwrap_or_else(js::undefined),
        )
    } else {
        (
            incoming_model.unwrap_or_else(js::undefined),
            incoming_mih.unwrap_or_else(js::undefined),
        )
    };
    let mut out = Map::new();
    for key in ALL_HINT_KEYS {
        let value = match key {
            "model" => model.clone(),
            "modelIsRequestedHint" => mih.clone(),
            _ => merge_field(key),
        };
        out.insert(key.to_string(), value);
    }
    Value::Object(out)
}

fn map_get(map: &[(Value, Value)], key: &Value) -> Option<Value> {
    map.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
}

fn map_set(map: &mut Vec<(Value, Value)>, key: Value, value: Value) {
    if let Some(entry) = map.iter_mut().find(|(k, _)| k == &key) {
        entry.1 = value;
    } else {
        map.push((key, value));
    }
}

/// Port of `buildSubagentIdentityDirectory` (`packages/shared/src/subagents.ts:582`).
pub fn build_subagent_identity_directory(
    hints: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let iter: Vec<Value> = match &hints {
        Value::Array(a) => a.clone(),
        Value::String(s) => s.chars().map(|c| Value::String(c.to_string())).collect(),
        _ => return Err("hints is not iterable".to_string()),
    };
    let mut by_thread: Vec<(Value, Value)> = Vec::new();
    let mut by_agent: Vec<(Value, Value)> = Vec::new();
    for hint in iter {
        if hint.is_null() {
            return Err("Cannot read properties of null (reading 'providerThreadId')".to_string());
        }
        if js::is_undefined(&hint) {
            return Err(
                "Cannot read properties of undefined (reading 'providerThreadId')".to_string(),
            );
        }
        let hint_obj = hint.as_object();
        let provider_thread_id =
            as_trimmed_string(hint_obj.and_then(|o| o.get("providerThreadId")));
        let agent_id = as_trimmed_string(hint_obj.and_then(|o| o.get("agentId")));
        let nickname = hint_obj.and_then(|o| o.get("nickname"));
        let role = hint_obj.and_then(|o| o.get("role"));
        if provider_thread_id.is_none()
            && agent_id.is_none()
            && nickname.is_none_or(js::is_undefined)
            && role.is_none_or(js::is_undefined)
        {
            continue;
        }
        let existing_by_thread = provider_thread_id
            .as_ref()
            .and_then(|t| map_get(&by_thread, &Value::String(t.clone())));
        let existing_by_agent = agent_id
            .as_ref()
            .and_then(|a| map_get(&by_agent, &Value::String(a.clone())));
        let existing = match existing_by_agent {
            Some(agent_entry) => Some(merge_hints(existing_by_thread.as_ref(), &agent_entry)),
            None => existing_by_thread,
        };
        let mut incoming = hint_obj.cloned().unwrap_or_default();
        if let Some(t) = &provider_thread_id {
            incoming.insert("providerThreadId".to_string(), Value::String(t.clone()));
        }
        if let Some(a) = &agent_id {
            incoming.insert("agentId".to_string(), Value::String(a.clone()));
        }
        let merged = merge_hints(existing.as_ref(), &Value::Object(incoming));
        if let Some(t) = provider_thread_id {
            map_set(&mut by_thread, Value::String(t), merged.clone());
        }
        if let Some(a) = agent_id {
            map_set(&mut by_agent, Value::String(a), merged.clone());
        }
        let merged_thread = merged
            .get("providerThreadId")
            .cloned()
            .unwrap_or_else(js::undefined);
        let merged_agent = merged.get("agentId").cloned().unwrap_or_else(js::undefined);
        if jscore::js_truthy(&merged_thread) && jscore::js_truthy(&merged_agent) {
            map_set(&mut by_thread, merged_thread, merged.clone());
            map_set(&mut by_agent, merged_agent, merged.clone());
        }
    }
    let encode_map = |map: Vec<(Value, Value)>| {
        let mut m = Map::new();
        m.insert(
            "$map".to_string(),
            Value::Array(
                map.into_iter()
                    .map(|(k, v)| Value::Array(vec![k, v]))
                    .collect(),
            ),
        );
        Value::Object(m)
    };
    let mut out = Map::new();
    out.insert("byProviderThreadId".to_string(), encode_map(by_thread));
    out.insert("byAgentId".to_string(), encode_map(by_agent));
    Ok(Value::Object(out))
}

/// Port of `resolveSubagentIdentityFromDirectory` (`packages/shared/src/subagents.ts:636`).
pub fn resolve_subagent_identity_from_directory(
    directory: serde_json::Value,
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'providerThreadId')".to_string());
    }
    if js::is_undefined(&input) {
        return Err("Cannot read properties of undefined (reading 'providerThreadId')".to_string());
    }
    let input_obj = input.as_object();
    let normalized_thread = as_trimmed_string(input_obj.and_then(|o| o.get("providerThreadId")));
    let normalized_agent = as_trimmed_string(input_obj.and_then(|o| o.get("agentId")));

    let get_map_entry = |which: &str,
                         normalized: Option<&String>|
     -> Result<Option<Value>, String> {
        if directory.is_null() {
            return Err(format!(
                "Cannot read properties of null (reading '{which}')"
            ));
        }
        match directory.get(which) {
            Some(Value::Object(m)) if m.contains_key("$map") => {
                let entries = m
                    .get("$map")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                Ok(normalized.and_then(|n| {
                    entries.iter().find_map(|pair| {
                        let arr = pair.as_array()?;
                        let key = arr.first()?;
                        if key == &Value::String(n.clone()) {
                            arr.get(1).cloned()
                        } else {
                            None
                        }
                    })
                }))
            }
            Some(Value::Null) => Err("Cannot read properties of null (reading 'get')".to_string()),
            Some(v) if js::is_undefined(v) => {
                Err("Cannot read properties of undefined (reading 'get')".to_string())
            }
            None => Err("Cannot read properties of undefined (reading 'get')".to_string()),
            _ => Err(format!("directory.{which}.get is not a function")),
        }
    };

    let thread_entry = match normalized_thread.as_ref() {
        Some(n) => get_map_entry("byProviderThreadId", Some(n))?,
        None => None,
    };
    let agent_entry = match normalized_agent.as_ref() {
        Some(n) => get_map_entry("byAgentId", Some(n))?,
        None => None,
    };
    if thread_entry.is_none() && agent_entry.is_none() {
        return Ok(js::undefined());
    }

    let mut incoming = match &thread_entry {
        Some(Value::Object(o)) => o.clone(),
        _ => Map::new(),
    };
    let pick = |key: &str, normalized: Option<&String>| -> Value {
        thread_entry
            .as_ref()
            .and_then(|v| v.get(key))
            .filter(|v| !v.is_null() && !js::is_undefined(v))
            .cloned()
            .or_else(|| {
                agent_entry
                    .as_ref()
                    .and_then(|v| v.get(key))
                    .filter(|v| !v.is_null() && !js::is_undefined(v))
                    .cloned()
            })
            .or_else(|| normalized.map(|n| Value::String(n.clone())))
            .unwrap_or_else(js::undefined)
    };
    incoming.insert(
        "providerThreadId".to_string(),
        pick("providerThreadId", normalized_thread.as_ref()),
    );
    incoming.insert(
        "agentId".to_string(),
        pick("agentId", normalized_agent.as_ref()),
    );
    Ok(merge_hints(agent_entry.as_ref(), &Value::Object(incoming)))
}
