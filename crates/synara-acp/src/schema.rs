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
        "logout" => validate::<v1::LogoutRequest>(value, method),
        "session/resume" => validate::<v1::ResumeSessionRequest>(value, method),
        "session/close" => validate::<v1::CloseSessionRequest>(value, method),
        "session/list" => validate::<v1::ListSessionsRequest>(value, method),
        "session/delete" => validate::<v1::DeleteSessionRequest>(value, method),
        "session/fork" => validate::<v1::ForkSessionRequest>(value, method),
        "session/set_mode" => validate::<v1::SetSessionModeRequest>(value, method),
        "session/set_config_option" => validate::<v1::SetSessionConfigOptionRequest>(value, method),
        "session/new" => validate::<v1::NewSessionRequest>(value, method),
        "session/load" => validate::<v1::LoadSessionRequest>(value, method),
        "session/prompt" => validate::<v1::PromptRequest>(value, method),
        _ => Ok(()),
    }
}
pub(crate) fn response(method: &str, value: &Value) -> AgentResult<()> {
    match method {
        "initialize" => validate::<v1::InitializeResponse>(value, method),
        "authenticate" => validate::<v1::AuthenticateResponse>(value, method),
        "logout" => validate::<v1::LogoutResponse>(value, method),
        "session/resume" => validate::<v1::ResumeSessionResponse>(value, method),
        "session/close" => validate::<v1::CloseSessionResponse>(value, method),
        "session/list" => validate::<v1::ListSessionsResponse>(value, method),
        "session/delete" => validate::<v1::DeleteSessionResponse>(value, method),
        "session/fork" => validate::<v1::ForkSessionResponse>(value, method),
        "session/set_mode" => validate::<v1::SetSessionModeResponse>(value, method),
        "session/set_config_option" => {
            validate::<v1::SetSessionConfigOptionResponse>(value, method)
        }
        "session/new" => validate::<v1::NewSessionResponse>(value, method),
        "session/load" => validate::<v1::LoadSessionResponse>(value, method),
        "session/prompt" => validate::<v1::PromptResponse>(value, method),
        _ => Ok(()),
    }
}
pub(crate) fn protocol_notification(method: &str, value: &Value) -> AgentResult<()> {
    match method {
        "$/cancel_request" => validate::<v1::CancelRequestNotification>(value, method),
        _ => Ok(()),
    }
}
pub(crate) fn callback(method: &str, value: &Value) -> AgentResult<()> {
    match method {
        "elicitation/create" => validate::<v1::CreateElicitationRequest>(value, method),
        "session/request_permission" => validate::<v1::RequestPermissionRequest>(value, method),
        "fs/read_text_file" => validate::<v1::ReadTextFileRequest>(value, method),
        "fs/write_text_file" => validate::<v1::WriteTextFileRequest>(value, method),
        "terminal/create" => validate::<v1::CreateTerminalRequest>(value, method),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn all_supported_stable_lifecycle_requests_use_pinned_schema_validation() {
        for (method, params) in [
            ("authenticate", json!({"methodId":"login"})),
            ("logout", json!({})),
            ("session/new", json!({"cwd":"/workspace","mcpServers":[]})),
            (
                "session/load",
                json!({"sessionId":"s","cwd":"/workspace","mcpServers":[]}),
            ),
            (
                "session/resume",
                json!({"sessionId":"s","cwd":"/workspace"}),
            ),
            ("session/close", json!({"sessionId":"s"})),
            ("session/delete", json!({"sessionId":"s"})),
            (
                "session/fork",
                json!({"sessionId":"s","cwd":"/workspace","mcpServers":[]}),
            ),
            ("session/list", json!({})),
            ("session/set_mode", json!({"sessionId":"s","modeId":"code"})),
            (
                "session/set_config_option",
                json!({"sessionId":"s","configId":"review","type":"boolean","value":true}),
            ),
        ] {
            assert!(request(method, &params).is_ok(), "{method}");
            assert!(request(method, &json!(false)).is_err(), "{method}");
        }
    }

    #[test]
    fn lifecycle_responses_reject_wrong_shapes_and_missing_mandatory_content() {
        for method in [
            "initialize",
            "authenticate",
            "logout",
            "session/new",
            "session/load",
            "session/resume",
            "session/close",
            "session/list",
            "session/delete",
            "session/fork",
            "session/set_mode",
            "session/set_config_option",
            "session/prompt",
        ] {
            assert!(response(method, &json!(false)).is_err(), "{method}");
        }
        assert!(response("session/list", &json!({"sessions":[]})).is_ok());
        assert!(response("session/set_config_option", &json!({"configOptions":[]})).is_ok());
        assert!(response("session/prompt", &json!({})).is_err());
        assert!(response("session/new", &json!({})).is_err());
    }

    #[test]
    fn protocol_cancellation_uses_the_pinned_stable_schema() {
        assert!(
            protocol_notification("$/cancel_request", &json!({"requestId":"request-1"})).is_ok()
        );
        assert!(protocol_notification("$/cancel_request", &json!({})).is_err());
        assert!(protocol_notification("$/cancel_request", &json!({"requestId":false})).is_err());
    }

    #[test]
    fn elicitation_requires_a_schema_and_valid_scope_at_the_boundary() {
        assert!(
            callback(
                "elicitation/create",
                &json!({
                    "sessionId":"s", "mode":"form", "message":"Choose",
                    "requestedSchema":{"type":"object","properties":{"name":{"type":"string"}}}
                })
            )
            .is_ok()
        );
        assert!(
            callback(
                "elicitation/create",
                &json!({"mode":"form","message":"Choose"})
            )
            .is_err()
        );
    }
}
