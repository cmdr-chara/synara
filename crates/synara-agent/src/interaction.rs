use crate::{AgentError, AgentResult};
use async_trait::async_trait;
use std::{collections::BTreeMap, time::Duration};
use synara_core::{
    InputFieldKind, InputValue, PermissionRequest, ThreadId, UserInputRequest, UserInputResponse,
};
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug)]
pub struct InteractionContext {
    pub thread_id: ThreadId,
    pub session_id: String,
    pub cancelled: CancellationToken,
}

#[async_trait]
pub trait InteractionHandler: Send + Sync {
    async fn permission(
        &self,
        context: InteractionContext,
        request: PermissionRequest,
    ) -> AgentResult<Option<String>>;
    async fn input(
        &self,
        context: InteractionContext,
        request: UserInputRequest,
    ) -> AgentResult<UserInputResponse>;
}

#[derive(Default)]
pub struct DenyInteractions;
#[async_trait]
impl InteractionHandler for DenyInteractions {
    async fn permission(
        &self,
        _context: InteractionContext,
        _request: PermissionRequest,
    ) -> AgentResult<Option<String>> {
        Ok(None)
    }
    async fn input(
        &self,
        _context: InteractionContext,
        _request: UserInputRequest,
    ) -> AgentResult<UserInputResponse> {
        Ok(UserInputResponse::Cancel)
    }
}

pub enum UiInteraction {
    Permission {
        context: InteractionContext,
        request: PermissionRequest,
        response: oneshot::Sender<Option<String>>,
    },
    Input {
        context: InteractionContext,
        request: UserInputRequest,
        response: oneshot::Sender<UserInputResponse>,
    },
}

pub struct InteractionBroker {
    sender: mpsc::Sender<UiInteraction>,
    timeout: Duration,
}
impl InteractionBroker {
    pub fn new() -> (Self, mpsc::Receiver<UiInteraction>) {
        let (sender, receiver) = mpsc::channel(32);
        (
            Self {
                sender,
                timeout: Duration::from_secs(300),
            },
            receiver,
        )
    }
}
#[async_trait]
impl InteractionHandler for InteractionBroker {
    async fn permission(
        &self,
        context: InteractionContext,
        request: PermissionRequest,
    ) -> AgentResult<Option<String>> {
        let (response, receiver) = oneshot::channel();
        let cancellation = context.cancelled.clone();
        let allowed = request
            .choices
            .iter()
            .map(|choice| choice.id.clone())
            .collect::<Vec<_>>();
        let interaction = UiInteraction::Permission {
            context,
            request,
            response,
        };
        let future = async {
            self.sender
                .send(interaction)
                .await
                .map_err(|_| AgentError::Cancelled)?;
            receiver.await.map_err(|_| AgentError::Cancelled)
        };
        let selected = tokio::select! {
            biased;
            () = cancellation.cancelled() => None,
            result = tokio::time::timeout(self.timeout, future) => result.ok().and_then(Result::ok).flatten(),
        };
        // A ready UI reply must not revive consent belonging to a cancelled turn.
        if cancellation.is_cancelled() {
            return Ok(None);
        }
        if selected.as_ref().is_some_and(|id| !allowed.contains(id)) {
            return Err(AgentError::Invalid("unknown permission choice".into()));
        }
        Ok(selected)
    }

    async fn input(
        &self,
        context: InteractionContext,
        request: UserInputRequest,
    ) -> AgentResult<UserInputResponse> {
        let (response, receiver) = oneshot::channel();
        let cancellation = context.cancelled.clone();
        let schema = request.clone();
        let interaction = UiInteraction::Input {
            context,
            request,
            response,
        };
        let future = async {
            self.sender
                .send(interaction)
                .await
                .map_err(|_| AgentError::Cancelled)?;
            receiver.await.map_err(|_| AgentError::Cancelled)
        };
        let result = tokio::select! {
            biased;
            () = cancellation.cancelled() => UserInputResponse::Cancel,
            result = tokio::time::timeout(self.timeout, future) => result.ok().and_then(Result::ok).unwrap_or(UserInputResponse::Cancel),
        };
        if cancellation.is_cancelled() {
            return Ok(UserInputResponse::Cancel);
        }
        if let UserInputResponse::Accept { values } = &result {
            validate_input(&schema, values)?;
        }
        Ok(result)
    }
}

pub fn validate_input(
    schema: &UserInputRequest,
    values: &BTreeMap<String, InputValue>,
) -> AgentResult<()> {
    if schema.url.is_some() {
        return if values.is_empty() {
            Ok(())
        } else {
            Err(AgentError::Invalid(
                "URL approval cannot submit form fields".into(),
            ))
        };
    }
    if schema.fields.len() > 32 || values.len() > 32 {
        return Err(AgentError::Limit);
    }
    if values
        .keys()
        .any(|key| !schema.fields.iter().any(|field| &field.id == key))
    {
        return Err(AgentError::Invalid("unknown form field".into()));
    }
    for field in &schema.fields {
        let Some(value) = values.get(&field.id) else {
            if field.required {
                return Err(AgentError::Invalid(format!(
                    "required field: {}",
                    field.label
                )));
            }
            continue;
        };
        let valid = match (&field.kind, value) {
            (
                InputFieldKind::Text {
                    min_length,
                    max_length,
                    ..
                },
                InputValue::Text(text),
            ) => {
                let count = text.chars().count();
                text.len() <= 64 * 1024
                    && min_length.is_none_or(|min| count >= min)
                    && max_length.is_none_or(|max| count <= max)
            }
            (InputFieldKind::Boolean, InputValue::Boolean(_)) => true,
            (
                InputFieldKind::Number {
                    integer,
                    minimum,
                    maximum,
                },
                InputValue::Number(value),
            ) => {
                value.is_finite()
                    && (!integer || value.fract() == 0.0)
                    && minimum.is_none_or(|min| *value >= min)
                    && maximum.is_none_or(|max| *value <= max)
            }
            (InputFieldKind::Choice { options }, InputValue::Text(value)) => {
                options.iter().any(|option| &option.value == value)
            }
            (
                InputFieldKind::MultiChoice {
                    options,
                    minimum,
                    maximum,
                },
                InputValue::Strings(values),
            ) => {
                let unique: std::collections::HashSet<_> = values.iter().collect();
                values.len() <= 512
                    && unique.len() == values.len()
                    && minimum.is_none_or(|min| values.len() >= min)
                    && maximum.is_none_or(|max| values.len() <= max)
                    && values
                        .iter()
                        .all(|value| options.iter().any(|option| &option.value == value))
            }
            _ => false,
        };
        if !valid {
            return Err(AgentError::Invalid(format!(
                "invalid field: {}",
                field.label
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use synara_core::{InputField, PermissionChoice, PermissionKind};

    fn context() -> InteractionContext {
        InteractionContext {
            thread_id: ThreadId::new(),
            session_id: "session".into(),
            cancelled: CancellationToken::new(),
        }
    }

    #[tokio::test]
    async fn closed_ui_never_approves() {
        let (broker, receiver) = InteractionBroker::new();
        drop(receiver);
        let request = PermissionRequest {
            id: "request".into(),
            tool_id: None,
            title: "run command".into(),
            choices: vec![],
        };
        assert_eq!(broker.permission(context(), request).await.unwrap(), None);
    }

    #[tokio::test]
    async fn arbitrary_permission_response_is_rejected() {
        let (broker, mut receiver) = InteractionBroker::new();
        let request = PermissionRequest {
            id: "request".into(),
            tool_id: None,
            title: "write file".into(),
            choices: vec![PermissionChoice {
                id: "once".into(),
                label: "Once".into(),
                kind: PermissionKind::AllowOnce,
            }],
        };
        let responder = tokio::spawn(async move {
            if let Some(UiInteraction::Permission { response, .. }) = receiver.recv().await {
                response.send(Some("unoffered-choice".into())).unwrap();
            }
        });
        assert!(broker.permission(context(), request).await.is_err());
        responder.await.unwrap();
    }

    #[test]
    fn form_validation_rejects_nonfinite_numbers_and_missing_required_values() {
        let request = UserInputRequest {
            id: "input".into(),
            message: "How many?".into(),
            url: None,
            fields: vec![InputField {
                id: "count".into(),
                label: "Count".into(),
                required: true,
                kind: InputFieldKind::Number {
                    integer: true,
                    minimum: Some(1.0),
                    maximum: Some(10.0),
                },
            }],
        };
        assert!(validate_input(&request, &BTreeMap::new()).is_err());
        assert!(
            validate_input(
                &request,
                &BTreeMap::from([("count".into(), InputValue::Number(f64::NAN))])
            )
            .is_err()
        );
        assert!(
            validate_input(
                &request,
                &BTreeMap::from([("count".into(), InputValue::Number(3.0))])
            )
            .is_ok()
        );
    }
}

#[cfg(test)]
#[path = "interaction_lifecycle_tests.rs"]
mod lifecycle_tests;
