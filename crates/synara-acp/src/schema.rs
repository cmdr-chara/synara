//! The SDK is contained at this boundary. Domain crates never acquire protocol types.
use agent_client_protocol::schema::v1;
use serde::de::DeserializeOwned;
use serde_json::Value;
use synara_agent::AgentResult;

fn validate<T: DeserializeOwned>(value: &Value, method: &str) -> AgentResult<()> {
    serde_json::from_value::<T>(value.clone())
        .map(|_| ())
        .map_err(|_| {
            crate::wire::invalid(format!(
                "message does not match the pinned schema: {method}"
            ))
        })
}
pub(crate) fn request(method: &str, value: &Value) -> AgentResult<()> {
    match method {
        "initialize" => validate::<v1::InitializeRequest>(value, method),
        "authenticate" => validate::<v1::AuthenticateRequest>(value, method),
        "session/new" => validate::<v1::NewSessionRequest>(value, method),
        "session/load" => validate::<v1::LoadSessionRequest>(value, method),
        "session/prompt" => validate::<v1::PromptRequest>(value, method),
        _ => Ok(()),
    }
}
pub(crate) fn response(method: &str, value: &Value) -> AgentResult<()> {
    match method {
        "initialize" => validate::<v1::InitializeResponse>(value, method),
        "session/new" => validate::<v1::NewSessionResponse>(value, method),
        "session/load" => validate::<v1::LoadSessionResponse>(value, method),
        "session/prompt" => validate::<v1::PromptResponse>(value, method),
        _ => Ok(()),
    }
}
pub(crate) fn callback(method: &str, value: &Value) -> AgentResult<()> {
    match method {
        "session/request_permission" => validate::<v1::RequestPermissionRequest>(value, method),
        "fs/read_text_file" => validate::<v1::ReadTextFileRequest>(value, method),
        "fs/write_text_file" => validate::<v1::WriteTextFileRequest>(value, method),
        "terminal/create" => validate::<v1::CreateTerminalRequest>(value, method),
        _ => Ok(()),
    }
}
