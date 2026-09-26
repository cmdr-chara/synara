//! Explicitly opted-in, paid-account direct-provider acceptance. Never part of ordinary tests.
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
use synara_model::{
    HttpModelProvider, Message, MessageRole, ModelError, ModelEvent, ModelProvider, ModelRequest,
    OutputFormat, ProtocolFamily, ProviderProfile, validate_request,
};
use synara_runtime::{RuntimeError, SecretReference, SecretStore, SecretStoreState, SecretValue};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

const MAX_CONFIG_BYTES: u64 = 64 * 1024;
const MAX_TEXT_BYTES: usize = 64 * 1024;
const MARKER: &str = "SYNARA_OK";
const PROMPT: &str = "Reply with exactly SYNARA_OK. Do not call tools.";
const FOLLOWUP: &str =
    "What exact marker did the previous user message request? Reply with only that marker.";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Matrix {
    candidate_commit: String,
    providers: Vec<Cell>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Cell {
    profile: ProviderProfile,
    model_id: String,
    key_env: String,
    max_output_tokens: u32,
}

impl Cell {
    fn request(&self, messages: Vec<Message>) -> ModelRequest {
        ModelRequest {
            model: self.model_id.clone(),
            messages,
            tools: vec![],
            output: OutputFormat::Text,
            reasoning_effort: None,
            max_output_tokens: self.max_output_tokens,
        }
    }

    fn route_digest(&self) -> String {
        // Bind the reviewed route without exporting endpoint URLs or model metadata.
        let route = json!({"profile": self.profile, "model_id": self.model_id,
            "max_output_tokens": self.max_output_tokens});
        hex::encode(Sha256::digest(serde_json::to_vec(&route).unwrap()))
    }
}

fn commit_id(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn validate_matrix(matrix: &Matrix) -> Result<(), &'static str> {
    if !commit_id(&matrix.candidate_commit) || !(3..=12).contains(&matrix.providers.len()) {
        return Err("a full candidate commit and 3-12 reviewed providers are required");
    }
    let mut ids = HashSet::new();
    let mut protocols = [false; 3];
    for cell in &matrix.providers {
        if !ids.insert(&cell.profile.id)
            || !cell.profile.requires_key
            || cell.profile.allow_loopback_http
            || !cell.profile.endpoint.starts_with("https://")
            || !(32..=2048).contains(&cell.max_output_tokens)
            || !cell.key_env.starts_with("SYNARA_PROVIDER_KEY_")
            || cell.key_env.len() <= "SYNARA_PROVIDER_KEY_".len()
            || cell.key_env.len() > 128
            || !cell
                .key_env
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
        {
            return Err("invalid reviewed provider or credential variable binding");
        }
        validate_request(
            &cell.profile,
            &cell.request(vec![Message::text(MessageRole::User, PROMPT.into())]),
        )
        .map_err(|_| "invalid reviewed provider request")?;
        protocols[match cell.profile.protocol {
            ProtocolFamily::OpenAiChat => 0,
            ProtocolFamily::AnthropicMessages => 1,
            ProtocolFamily::GoogleGenerateContent => 2,
        }] = true;
    }
    if !protocols.into_iter().all(|present| present) {
        return Err("the matrix requires OpenAI-compatible, Anthropic and Google protocols");
    }
    Ok(())
}

struct ScopedSecret {
    reference: SecretReference,
    value: SecretValue,
    reads: AtomicUsize,
}

#[async_trait]
impl SecretStore for ScopedSecret {
    fn state(&self) -> SecretStoreState {
        SecretStoreState::Available
    }

    async fn read(&self, reference: &SecretReference) -> Result<Option<SecretValue>, RuntimeError> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        if reference != &self.reference {
            return Ok(None);
        }
        SecretValue::new(self.value.expose().to_vec()).map(Some)
    }

    async fn write(&self, _: &SecretReference, _: SecretValue) -> Result<(), RuntimeError> {
        Err(RuntimeError::Denied(
            "acceptance credentials are read-only".into(),
        ))
    }

    async fn delete(&self, _: &SecretReference) -> Result<(), RuntimeError> {
        Err(RuntimeError::Denied(
            "acceptance credentials are read-only".into(),
        ))
    }
}

fn error_kind(error: ModelError) -> &'static str {
    match error {
        ModelError::Invalid(_) => "invalid_request",
        ModelError::Unsupported(_) => "unsupported",
        ModelError::Credential => "credential",
        ModelError::SchemaMismatch => "schema_mismatch",
        ModelError::Incomplete => "incomplete",
        ModelError::Http(401 | 403) => "authentication_rejected",
        ModelError::Http(429) => "rate_limited",
        ModelError::Http(_) => "http_error",
        ModelError::Transport => "transport",
        ModelError::Protocol => "protocol",
        ModelError::Limit => "limit",
        ModelError::Cancelled => "cancelled",
    }
}

struct Answer {
    text: String,
    events: usize,
    usage_reported: bool,
}

async fn answer(
    provider: &HttpModelProvider,
    cell: &Cell,
    secrets: &ScopedSecret,
    messages: Vec<Message>,
) -> Result<Answer, &'static str> {
    let cancellation = CancellationToken::new();
    let (tx, mut rx) = mpsc::channel(16);
    let request = cell.request(messages);
    let streamed = provider.stream(&cell.profile, request, secrets, cancellation.clone(), tx);
    let collected = async {
        let mut answer = Answer {
            text: String::new(),
            events: 0,
            usage_reported: false,
        };
        let mut finished = false;
        while let Some(event) = rx.recv().await {
            answer.events += 1;
            if answer.events > 8192 || finished {
                return Err("unexpected_event_sequence");
            }
            match event {
                ModelEvent::Text(text) => {
                    if answer.text.len().saturating_add(text.len()) > MAX_TEXT_BYTES {
                        return Err("text_limit");
                    }
                    answer.text.push_str(&text);
                }
                ModelEvent::Reasoning(_) => {}
                ModelEvent::Usage(usage) => {
                    answer.usage_reported |=
                        usage.input_tokens.is_some() || usage.output_tokens.is_some();
                }
                ModelEvent::ToolCall(_) => return Err("unexpected_tool_call"),
                ModelEvent::Finished { reason } => {
                    if !matches!(reason.as_str(), "stop" | "end_turn") {
                        return Err("incomplete_answer");
                    }
                    finished = true;
                }
            }
        }
        if !finished || answer.text.trim() != MARKER {
            return Err("unexpected_answer");
        }
        Ok(answer)
    };
    let result = tokio::time::timeout(Duration::from_secs(90), async {
        tokio::try_join!(async { streamed.await.map_err(error_kind) }, collected)
    })
    .await;
    cancellation.cancel();
    match result {
        Ok(Ok(((), answer))) => Ok(answer),
        Ok(Err(error)) => Err(error),
        Err(_) => Err("deadline"),
    }
}

async fn probe(provider: &HttpModelProvider, cell: &Cell, secrets: &ScopedSecret) -> Value {
    let mut report = json!({
        "profile_id": cell.profile.id,
        "protocol": cell.profile.protocol,
        "route_sha256": cell.route_digest(),
        "max_output_tokens_per_request": cell.max_output_tokens,
        "pre_cancel": "pending",
        "first_prompt": "not_attempted",
        "history_followup": "not_attempted",
        "passed": false,
    });
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let (tx, mut rx) = mpsc::channel(1);
    let cancelled = provider
        .stream(
            &cell.profile,
            cell.request(vec![Message::text(MessageRole::User, PROMPT.into())]),
            secrets,
            cancellation,
            tx,
        )
        .await;
    if !matches!(cancelled, Err(ModelError::Cancelled))
        || secrets.reads.load(Ordering::Relaxed) != 0
        || rx.try_recv().is_ok()
    {
        report["pre_cancel"] = json!("failed");
        return report;
    }
    report["pre_cancel"] = json!("passed_without_credential_access");
    let first = match answer(
        provider,
        cell,
        secrets,
        vec![Message::text(MessageRole::User, PROMPT.into())],
    )
    .await
    {
        Ok(answer) => answer,
        Err(error) => {
            report["first_prompt"] = json!(error);
            return report;
        }
    };
    report["first_prompt"] = json!("passed");
    report["first_prompt_event_count"] = json!(first.events);
    report["first_prompt_usage_reported"] = json!(first.usage_reported);
    match answer(
        provider,
        cell,
        secrets,
        vec![
            Message::text(MessageRole::User, PROMPT.into()),
            Message::text(MessageRole::Assistant, first.text),
            Message::text(MessageRole::User, FOLLOWUP.into()),
        ],
    )
    .await
    {
        Ok(answer) => {
            report["history_followup"] = json!("passed");
            report["history_followup_event_count"] = json!(answer.events);
            report["history_followup_usage_reported"] = json!(answer.usage_reported);
            report["passed"] = json!(true);
        }
        Err(error) => report["history_followup"] = json!(error),
    }
    report
}

fn same_clean_candidate(expected: &str) -> bool {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let head = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output();
    let clean = Command::new("git")
        .args(["diff", "--quiet", "HEAD", "--", ":/"])
        .current_dir(root)
        .output();
    head.is_ok_and(|output| {
        output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == expected
    }) && clean.is_ok_and(|output| output.status.success())
}

fn write_report(file: &mut File, document: &Value) {
    let data = serde_json::to_vec_pretty(document).expect("encode redacted report");
    file.set_len(0).expect("truncate owned report");
    file.seek(SeekFrom::Start(0)).expect("rewind owned report");
    file.write_all(&data).expect("write redacted report");
    file.sync_all().expect("persist redacted report");
}

#[tokio::test]
#[ignore = "requires explicitly reviewed real accounts and can incur provider charges"]
async fn reviewed_live_direct_provider_matrix() {
    assert!(
        std::env::var("SYNARA_PROVIDER_ACCEPTANCE").as_deref() == Ok("reviewed-paid-accounts"),
        "explicit SYNARA_PROVIDER_ACCEPTANCE opt-in is required"
    );
    let config_path =
        std::env::var_os("SYNARA_PROVIDER_CONFIG").expect("SYNARA_PROVIDER_CONFIG is required");
    let mut bytes = Vec::new();
    File::open(config_path)
        .expect("open reviewed provider configuration")
        .take(MAX_CONFIG_BYTES + 1)
        .read_to_end(&mut bytes)
        .expect("read reviewed provider configuration");
    assert!(
        bytes.len() as u64 <= MAX_CONFIG_BYTES,
        "configuration exceeds limit"
    );
    let matrix: Matrix = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| panic!("invalid provider configuration schema"));
    validate_matrix(&matrix).unwrap_or_else(|reason| panic!("{reason}"));
    assert!(
        same_clean_candidate(&matrix.candidate_commit),
        "candidate must match clean tracked HEAD"
    );

    // Resolve every explicit binding before spending any provider quota. The test
    // never writes to the native credential store or persists environment values.
    let secrets: Vec<_> = matrix
        .providers
        .iter()
        .map(|cell| {
            let mut value = std::env::var(&cell.key_env)
                .unwrap_or_else(|_| panic!("a reviewed credential variable is missing"))
                .into_bytes();
            if value.is_empty()
                || value.len() > 8192
                || !value.iter().all(|b| (33..=126).contains(b))
            {
                value.fill(0);
                panic!("a reviewed credential has an invalid format");
            }
            ScopedSecret {
                reference: cell
                    .profile
                    .secret_reference()
                    .expect("validated route reference"),
                value: SecretValue::new(value).expect("bounded credential"),
                reads: AtomicUsize::new(0),
            }
        })
        .collect();
    let output =
        std::env::var_os("SYNARA_PROVIDER_REPORT").expect("SYNARA_PROVIDER_REPORT is required");
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(output)
        .expect("create a new acceptance report");
    let provider = HttpModelProvider::new().expect("initialize direct provider transport");
    let mut report = json!({
        "format": "synara-live-direct-provider-acceptance-v1",
        "candidate_commit": matrix.candidate_commit,
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "scope": "authenticated direct text streams, explicit history and local pre-cancellation",
        "native_onboarding_accepted": false,
        "acp_interoperability_accepted": false,
        "active_remote_cancellation_accepted": false,
        "status": "running",
        "providers": [],
    });
    write_report(&mut file, &report);
    for (cell, secrets) in matrix.providers.iter().zip(&secrets) {
        let result = probe(&provider, cell, secrets).await;
        report["providers"].as_array_mut().unwrap().push(result);
        write_report(&mut file, &report);
    }
    let passed = report["providers"]
        .as_array()
        .unwrap()
        .iter()
        .all(|cell| cell["passed"] == true)
        && same_clean_candidate(&matrix.candidate_commit);
    report["status"] = json!(if passed { "passed" } else { "failed" });
    write_report(&mut file, &report);
    assert!(
        passed,
        "live provider matrix failed; inspect the redacted report"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use synara_model::{ModelCapabilities, ModelInfo};

    fn matrix() -> Matrix {
        Matrix {
            candidate_commit: "a".repeat(40),
            providers: [
                ProtocolFamily::OpenAiChat,
                ProtocolFamily::AnthropicMessages,
                ProtocolFamily::GoogleGenerateContent,
            ]
            .into_iter()
            .enumerate()
            .map(|(i, protocol)| Cell {
                profile: ProviderProfile {
                    id: format!("provider-{i}"),
                    name: "Reviewed provider".into(),
                    protocol,
                    endpoint: "https://provider.example/v1".into(),
                    allow_loopback_http: false,
                    requires_key: true,
                    models: vec![ModelInfo {
                        id: "reviewed-model".into(),
                        name: "Reviewed model".into(),
                        capabilities: ModelCapabilities::default(),
                    }],
                },
                model_id: "reviewed-model".into(),
                key_env: format!("SYNARA_PROVIDER_KEY_{i}"),
                max_output_tokens: 256,
            })
            .collect(),
        }
    }

    #[test]
    fn matrix_requires_all_protocols_and_bounded_explicit_credential_bindings() {
        validate_matrix(&matrix()).unwrap();
        let mut invalid = matrix();
        invalid.providers[2].profile.protocol = ProtocolFamily::OpenAiChat;
        assert!(validate_matrix(&invalid).is_err());
        for field in ["key_env", "endpoint", "budget", "duplicate"] {
            let mut invalid = matrix();
            match field {
                "key_env" => invalid.providers[0].key_env = "HOME".into(),
                "endpoint" => invalid.providers[0].profile.endpoint = "http://127.0.0.1:80".into(),
                "budget" => invalid.providers[0].max_output_tokens = 2049,
                _ => invalid.providers[1].profile.id = invalid.providers[0].profile.id.clone(),
            }
            assert!(validate_matrix(&invalid).is_err());
        }
    }

    #[tokio::test]
    async fn credential_store_is_read_only_and_bound_to_one_exact_route() {
        let reference = matrix().providers[0].profile.secret_reference().unwrap();
        let secrets = ScopedSecret {
            reference: reference.clone(),
            value: SecretValue::new(b"secret-canary".to_vec()).unwrap(),
            reads: AtomicUsize::new(0),
        };
        assert!(
            secrets
                .read(&SecretReference::new("other", "route").unwrap())
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(
            secrets.read(&reference).await.unwrap().unwrap().expose(),
            b"secret-canary"
        );
        assert!(
            secrets
                .write(
                    &reference,
                    SecretValue::new(b"new-canary".to_vec()).unwrap()
                )
                .await
                .is_err()
        );
        assert!(secrets.delete(&reference).await.is_err());
    }

    #[test]
    fn route_receipt_is_hashed_and_errors_do_not_export_values() {
        let cell = matrix().providers.remove(0);
        assert_eq!(cell.route_digest().len(), 64);
        assert_eq!(
            error_kind(ModelError::Invalid("secret-canary")),
            "invalid_request"
        );
        assert_eq!(error_kind(ModelError::Http(401)), "authentication_rejected");
        assert_eq!(error_kind(ModelError::Http(429)), "rate_limited");
    }
}
