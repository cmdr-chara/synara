//! Direct inference has a separate wire budget from provider-settings/catalog
//! input. A 2 MiB owned attachment batch can expand to base64 without weakening
//! settings, discovery or stream frame limits.
use crate::*;

pub const MAX_INPUT_BYTES: usize = 4 * 1024 * 1024;
impl ModelSelection {
    pub fn validate_context(&self) -> ModelResult<()> {
        if self.history_turns.is_some_and(|n| n > 256) {
            return Err(ModelError::Invalid(
                "history_turns must be null or between 0 and 256",
            ));
        }
        Ok(())
    }
}
/// Encode without sending to catch wire expansion limits before the workspace
/// records a prompt or acknowledges the selected attachment IDs.
pub fn validate_wire_request(profile: &ProviderProfile, request: &ModelRequest) -> ModelResult<()> {
    crate::protocol::encode(profile, request).map(|_| ())
}
pub(crate) fn canonical_base64(value: &str) -> bool {
    if value.is_empty() || !value.len().is_multiple_of(4) {
        return false;
    }
    let bytes = value.as_bytes();
    let padding = bytes.iter().rev().take_while(|b| **b == b'=').count();
    if padding > 2 {
        return false;
    }
    let data = &bytes[..bytes.len() - padding];
    fn sextet(byte: u8) -> Option<u8> {
        match byte {
            b'A'..=b'Z' => Some(byte - b'A'),
            b'a'..=b'z' => Some(byte - b'a' + 26),
            b'0'..=b'9' => Some(byte - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    if data.iter().any(|b| sextet(*b).is_none()) {
        return false;
    }
    let last = data.last().and_then(|b| sextet(*b)).unwrap_or(0);
    match padding {
        1 => last & 3 == 0,
        2 => last & 15 == 0,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_padding_and_unused_bits_are_checked() {
        for value in ["AAAA", "AA==", "AAA=", "YWJj", "YQ==", "YWI="] {
            assert!(canonical_base64(value), "{value}");
        }
        for value in [
            "", "A", "====", "A===", "AA=A", "A!AA", "AB==", "AAB=", "AA==AAAA", "AA\n=",
        ] {
            assert!(!canonical_base64(value), "{value}");
        }
    }
    #[test]
    fn old_model_selections_keep_all_history_by_default() {
        let selection: ModelSelection = serde_json::from_str(
            r#"{"provider_id":"fixture","model_id":"fixture","max_output_tokens":32}"#,
        )
        .unwrap();
        assert_eq!(selection.history_turns, None);
        selection.validate_context().unwrap();
    }
    #[test]
    fn history_policy_round_trips_and_rejects_out_of_range_values() {
        let mut selection: ModelSelection = serde_json::from_str(r#"{"provider_id":"fixture","model_id":"fixture","max_output_tokens":32,"history_turns":10}"#).unwrap();
        assert_eq!(selection.history_turns, Some(10));
        assert_eq!(
            serde_json::from_str::<ModelSelection>(&serde_json::to_string(&selection).unwrap())
                .unwrap(),
            selection
        );
        selection.history_turns = Some(0);
        selection.validate_context().unwrap();
        selection.history_turns = Some(256);
        selection.validate_context().unwrap();
        selection.history_turns = Some(257);
        assert!(selection.validate_context().is_err());
    }
    #[test]
    fn inference_wire_budget_is_separate_from_configuration_limits() {
        assert_eq!(MAX_REQUEST_BYTES, 1024 * 1024);
        let profile = custom_profile_example();
        let selection: ModelSelection = serde_json::from_value(serde_json::json!({
            "provider_id":profile.id, "model_id":profile.models[0].id, "max_output_tokens":32
        }))
        .unwrap();
        let request = selection.request(vec![Message::text(
            MessageRole::User,
            "a".repeat(MAX_REQUEST_BYTES + 1),
        )]);
        validate_wire_request(&profile, &request).unwrap();
        let request = selection.request(vec![Message::text(
            MessageRole::User,
            "a".repeat(MAX_INPUT_BYTES),
        )]);
        assert!(matches!(
            validate_wire_request(&profile, &request),
            Err(ModelError::Limit)
        ));
    }
    #[test]
    fn malformed_inline_images_fail_before_wire_encoding() {
        let mut profile = custom_profile_example();
        profile.models[0].capabilities.images = Support::Supported;
        let request = ModelRequest {
            model: profile.models[0].id.clone(),
            messages: vec![Message {
                role: MessageRole::User,
                content: vec![Content::Image {
                    media_type: "image/png".into(),
                    base64: "AA=A".into(),
                }],
                tool_calls: vec![],
                tool_call_id: None,
            }],
            tools: vec![],
            output: OutputFormat::Text,
            reasoning_effort: None,
            max_output_tokens: 32,
        };
        assert!(matches!(
            validate_wire_request(&profile, &request),
            Err(ModelError::Invalid("inline image"))
        ));
    }
}
