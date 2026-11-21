//! The Rust port of `apps/analytics/diagnosticsPrivacySql.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

const DIAGNOSTIC_PROCESSES: &[&str] = &[
    "main",
    "renderer",
    "backend",
    "server",
    "Browser",
    "GPU",
    "Utility",
    "Zygote",
    "Sandbox helper",
    "Pepper Plugin",
    "Pepper Plugin Broker",
    "unknown",
];
const DIAGNOSTIC_CRASH_REASONS: &[&str] = &[
    "clean-exit",
    "abnormal-exit",
    "killed",
    "crashed",
    "oom",
    "launch-failed",
    "integrity-failure",
    "memory-eviction",
    "unknown",
];
const DIAGNOSTIC_SIGNALS: &[&str] = &[
    "SIGABRT",
    "SIGALRM",
    "SIGBUS",
    "SIGCHLD",
    "SIGCONT",
    "SIGFPE",
    "SIGHUP",
    "SIGILL",
    "SIGINT",
    "SIGIO",
    "SIGIOT",
    "SIGKILL",
    "SIGPIPE",
    "SIGPOLL",
    "SIGPROF",
    "SIGPWR",
    "SIGQUIT",
    "SIGSEGV",
    "SIGSTKFLT",
    "SIGSTOP",
    "SIGSYS",
    "SIGTERM",
    "SIGTRAP",
    "SIGTSTP",
    "SIGTTIN",
    "SIGTTOU",
    "SIGURG",
    "SIGUSR1",
    "SIGUSR2",
    "SIGVTALRM",
    "SIGWINCH",
    "SIGXCPU",
    "SIGXFSZ",
];
const DESKTOP_UPDATE_FAILURE_REASONS: &[&str] = &[
    "timeout",
    "network",
    "http-client",
    "http-server",
    "manifest",
    "release-feed",
    "verification",
    "cancelled",
    "unknown",
];
const UPDATE_EVENTS: &[&str] = &[
    "update.check",
    "update.available",
    "update.downloaded",
    "update.installed",
    "update.error",
];
const CRASH_EVENTS: &str = "event IN ('app.renderer-crash', 'app.child-process-crash')";

fn literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn list(values: &[&str]) -> String {
    values
        .iter()
        .map(|v| literal(v))
        .collect::<Vec<_>>()
        .join(", ")
}

fn category(column: &str, values: &[&str], fallback: &str) -> String {
    format!(
        "CASE WHEN {column} IN ({}) THEN {column} ELSE {} END",
        list(values),
        literal(fallback)
    )
}

// SQLite has no built-in regexp. Validate the complete release grammar, including the suffix.
fn version(column: &str, fallback: &str) -> String {
    let base = format!(
        "(CASE WHEN instr({column}, '-') > 0 THEN substr({column}, 1, instr({column}, '-') - 1) ELSE {column} END)"
    );
    let suffixes: Vec<String> = ["beta", "remote", "canary"]
        .iter()
        .map(|tag| {
            let prefix = format!("{base} || '-{tag}.'");
            let number = format!("substr({column}, length({base}) + {})", tag.len() + 3);
            format!(
                "(substr({column}, 1, length({base}) + {}) = {prefix}\n      AND length({number}) > 0 AND {number} NOT GLOB '*[^0-9]*')",
                tag.len() + 2
            )
        })
        .collect();
    format!(
        "CASE WHEN length({column}) BETWEEN 1 AND 32\n    AND instr({column}, char(0)) = 0\n    AND {base} NOT GLOB '*[^0-9.]*'\n    AND length({base}) - length(replace({base}, '.', '')) = 2\n    AND {base} NOT LIKE '.%' AND {base} NOT LIKE '%.' AND {base} NOT LIKE '%..%'\n    AND ({column} = {base} OR {})\n    THEN {column} ELSE {} END",
        suffixes.join(" OR "),
        literal(fallback)
    )
}

fn diagnostic_payload_kind(event: &str) -> Option<&'static str> {
    if event == "app.start" || event == "app.exit" {
        return Some("lifecycle");
    }
    if event == "app.error" {
        return Some("error");
    }
    if event == "app.renderer-crash" || event == "app.child-process-crash" {
        return Some("crash");
    }
    if UPDATE_EVENTS.contains(&event) {
        return Some("update");
    }
    if event == "usage.daily" {
        return Some("usage");
    }
    if event == "beta.installed" || event == "beta.left" {
        return Some("beta");
    }
    None
}

/// Port of `diagnosticMetadataSql` (`apps/analytics/diagnosticsPrivacySql.ts:49`).
pub fn diagnostic_metadata_sql(events: serde_json::Value) -> Result<serde_json::Value, String> {
    // `[...values]`: arrays spread directly, strings iterate by code point.
    let owned: Vec<String>;
    let raw: Vec<Value> = match &events {
        Value::Array(a) => a.clone(),
        Value::String(s) => {
            owned = s.chars().map(|c| c.to_string()).collect();
            owned.iter().map(|c| Value::String(c.clone())).collect()
        }
        _ => return Err("values is not iterable".to_string()),
    };
    let mut events: Vec<&str> = Vec::with_capacity(raw.len());
    for v in &raw {
        match v.as_str() {
            Some(s) => events.push(s),
            None => return Err("value.replaceAll is not a function".to_string()),
        }
    }

    let process = category("process_type", DIAGNOSTIC_PROCESSES, "unknown");
    let exit_code = "substr(reason, 6, instr(reason || ' ', ' ') - 6)";
    let exit_digits = format!(
        "(CASE WHEN substr({exit_code}, 1, 1) = '-' THEN substr({exit_code}, 2) ELSE {exit_code} END)"
    );
    let exit_signal = "substr(reason, instr(reason, ' ') + 1)";
    let mut signals: Vec<String> = vec!["signal=null".to_string()];
    signals.extend(DIAGNOSTIC_SIGNALS.iter().map(|s| format!("signal={s}")));
    let numeric_exit = format!(
        "substr(reason, 1, 5) = 'code='\n  AND ({process}) IN ('backend', 'server')\n  AND ({exit_code} = 'null' OR (length({exit_digits}) BETWEEN 1 AND 10 AND {exit_digits} NOT GLOB '*[^0-9]*'))\n  AND (instr(reason, ' ') = 0 OR {exit_signal} IN ({}))",
        list(&signals.iter().map(String::as_str).collect::<Vec<_>>())
    );
    let crash_reason = format!(
        "CASE\n  WHEN reason IN ({}) THEN reason\n  WHEN {numeric_exit} THEN 'code=' || CASE WHEN {exit_code} = 'null' THEN 'null' ELSE CAST(CAST({exit_code} AS INTEGER) AS TEXT) END\n    || CASE WHEN instr(reason, ' ') = 0 THEN '' ELSE ' ' || {exit_signal} END\n  ELSE 'unknown' END",
        list(DIAGNOSTIC_CRASH_REASONS)
    );

    let kind = format!(
        "CASE event {} ELSE '' END",
        events
            .iter()
            .map(|e| format!(
                "WHEN {} THEN {}",
                literal(e),
                literal(diagnostic_payload_kind(e).unwrap_or(""))
            ))
            .collect::<Vec<_>>()
            .join(" ")
    );
    let outcome = format!(
        "CASE WHEN event = 'beta.installed' THEN {}\n      WHEN event = 'beta.left' THEN {}\n      ELSE {} END",
        category(
            "outcome",
            &["imported", "import-failed", "fresh"],
            "unknown"
        ),
        category("outcome", &["trash", "keep"], "unknown"),
        category("outcome", &["ok", "error"], "unknown")
    );
    let reason = format!(
        "CASE WHEN {CRASH_EVENTS} THEN {crash_reason}\n      WHEN event = 'update.error' THEN {}\n      ELSE '' END",
        category("reason", DESKTOP_UPDATE_FAILURE_REASONS, "unknown")
    );

    let mut out = serde_json::Map::new();
    out.insert(
        "ts".to_string(),
        Value::String(
            "COALESCE(strftime('%Y-%m-%dT%H:%M:%fZ', ts), strftime('%Y-%m-%dT%H:%M:%fZ', received_at), '1970-01-01T00:00:00.000Z')"
                .to_string(),
        ),
    );
    out.insert(
        "app_version".to_string(),
        Value::String(version("app_version", "0.0.0")),
    );
    out.insert(
        "target_version".to_string(),
        Value::String(version("target_version", "")),
    );
    out.insert(
        "event".to_string(),
        Value::String(category("event", &events, "")),
    );
    out.insert("kind".to_string(), Value::String(kind));
    out.insert("outcome".to_string(), Value::String(outcome));
    out.insert(
        "platform".to_string(),
        Value::String(category(
            "platform",
            &["darwin", "win32", "linux"],
            "unknown",
        )),
    );
    out.insert(
        "arch".to_string(),
        Value::String(category(
            "arch",
            &["arm64", "x64", "ia32", "arm"],
            "unknown",
        )),
    );
    out.insert(
        "source".to_string(),
        Value::String(category("source", &["main", "renderer"], "")),
    );
    out.insert(
        "flavor".to_string(),
        Value::String(category("flavor", &["beta", "remote"], "unknown")),
    );
    out.insert(
        "process_type".to_string(),
        Value::String(format!(
            "CASE WHEN {CRASH_EVENTS} THEN {process} ELSE '' END"
        )),
    );
    out.insert("reason".to_string(), Value::String(reason));
    out.insert(
        "error_context".to_string(),
        Value::String(category(
            "error_context",
            &["check", "download", "install"],
            "",
        )),
    );
    Ok(Value::Object(out))
}

/// Port of `diagnosticReadCteSql` (`apps/analytics/diagnosticsPrivacySql.ts:72`).
pub fn diagnostic_read_cte_sql(metadata: serde_json::Value) -> Result<String, String> {
    let unchanged = [
        "id",
        "received_at",
        "install_id",
        "duration_ms",
        "client_id",
        "os_version",
        "locale",
        "projects",
        "active_threads",
        "message",
        "stack",
    ];
    let entries = match &metadata {
        Value::Object(m) if m.get("$undefined").is_none() => m
            .iter()
            .map(|(k, v)| format!("{} AS {}", jscore::js_to_string(v), k))
            .collect::<Vec<_>>()
            .join(",\n"),
        Value::Object(_) => {
            return Err("Cannot convert undefined or null to object".to_string());
        }
        Value::Array(a) => a
            .iter()
            .enumerate()
            .map(|(i, v)| format!("{} AS {}", jscore::js_to_string(v), i))
            .collect::<Vec<_>>()
            .join(",\n"),
        // `Object.entries` on a string walks UTF-16 code units.
        Value::String(s) => s
            .encode_utf16()
            .enumerate()
            .map(|(i, u)| {
                format!(
                    "{} AS {}",
                    char::from_u32(u as u32).unwrap_or('\u{fffd}'),
                    i
                )
            })
            .collect::<Vec<_>>()
            .join(",\n"),
        Value::Null => {
            return Err("Cannot convert undefined or null to object".to_string());
        }
        _ => String::new(),
    };
    Ok(format!(
        "WITH diagnostic_metadata AS (\n    SELECT {}, NULL AS log_tail,\n      CASE WHEN length(fingerprint) BETWEEN 8 AND 32 AND instr(fingerprint, char(0)) = 0 AND fingerprint NOT GLOB '*[^0-9a-fA-F]*'\n        THEN fingerprint ELSE '0000000000000000' END AS fingerprint,\n      {}\n    FROM events\n  ), diagnostic_events AS (\n    SELECT diagnostic_metadata.*, CASE\n      WHEN event = 'app.error' THEN 'err:' || fingerprint\n      WHEN {CRASH_EVENTS} THEN 'crash:' || process_type || ':' || reason\n      WHEN event = 'update.error' THEN 'update:' || error_context END AS issue_key\n    FROM diagnostic_metadata\n  )",
        unchanged.join(", "),
        entries
    ))
}
