use crate::{
    elicitation,
    rpc::{RpcId, RpcPeer},
    scope::{SessionState, Sessions},
    wire,
};
use serde_json::{Value, json};
use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Duration};
use synara_agent::{
    AgentError, AgentResult, ConnectionContext, InteractionContext, InteractionScope,
};
use synara_core::{
    ConnectionId, PermissionChoice, PermissionKind, PermissionRequest, ThreadEvent, ThreadId,
    UserInputResponse,
};
use synara_runtime::{LaunchSpec, NativeTerminal, RuntimeError, TerminalSnapshot};
use tokio::sync::Mutex;
use uuid::Uuid;

struct TerminalEntry {
    session_id: String,
    terminal: Arc<NativeTerminal>,
    output_limit: usize,
}
pub(crate) struct CallbackServices {
    context: ConnectionContext,
    sessions: Arc<Sessions>,
    terminals: Mutex<HashMap<String, TerminalEntry>>,
    elicitations: Arc<crate::elicitation_registry::Registry>,
    connection_thread: ThreadId,
    connection_id: ConnectionId,
}
impl CallbackServices {
    pub fn new(
        context: ConnectionContext,
        sessions: Arc<Sessions>,
        connection_id: ConnectionId,
    ) -> Self {
        Self {
            context,
            sessions,
            terminals: Mutex::new(HashMap::new()),
            elicitations: Arc::new(crate::elicitation_registry::Registry::default()),
            connection_thread: ThreadId::new(),
            connection_id,
        }
    }
    pub async fn handle(&self, method: &str, params: Value, peer: &RpcPeer) -> AgentResult<Value> {
        crate::schema::callback(method, &params)?;
        match method {
            "elicitation/create" => self.elicit(params, peer).await,
            "session/request_permission" => self.permission(params).await,
            "fs/read_text_file" => self.read_file(params).await,
            "fs/write_text_file" => self.write_file(params).await,
            "terminal/create" => self.create_terminal(params).await,
            "terminal/output" | "terminal/wait_for_exit" | "terminal/kill" | "terminal/release" => {
                self.terminal_operation(method, params).await
            }
            _ => Err(AgentError::Unsupported("client callback".into())),
        }
    }
    fn scope(&self, params: &Value) -> AgentResult<Arc<SessionState>> {
        self.sessions.get(&wire::id(params, "sessionId")?)
    }
    async fn ask_permission(
        &self,
        session: &SessionState,
        request: PermissionRequest,
    ) -> AgentResult<Option<String>> {
        // Capture ownership once. Looking it up after awaiting the UI can select
        // a later turn or the session lifetime and resurrect stale approval.
        let interaction = session.interaction();
        let cancelled = interaction.cancelled.clone();
        session
            .emit(
                &self.context,
                ThreadEvent::PermissionRequested {
                    request: request.clone(),
                },
            )
            .await?;
        let id = request.id.clone();
        let result = self
            .context
            .interactions
            .permission(interaction, request.clone())
            .await;
        let result = if cancelled.is_cancelled() {
            Ok(None)
        } else {
            result
        };
        let result = if result
            .as_ref()
            .ok()
            .and_then(|value| value.as_ref())
            .is_some_and(|selected| !request.choices.iter().any(|choice| &choice.id == selected))
        {
            Err(wire::invalid("permission response was not offered"))
        } else {
            result
        };
        let selected = result.as_ref().ok().cloned().flatten();
        session
            .emit(
                &self.context,
                ThreadEvent::PermissionResolved { id, selected },
            )
            .await?;
        result
    }
    async fn local_approval(&self, session: &SessionState, title: String) -> AgentResult<()> {
        let request = PermissionRequest {
            id: Uuid::new_v4().to_string(),
            tool_id: None,
            title,
            choices: vec![
                PermissionChoice {
                    id: "allow".into(),
                    label: "Allow once".into(),
                    kind: PermissionKind::AllowOnce,
                },
                PermissionChoice {
                    id: "deny".into(),
                    label: "Deny".into(),
                    kind: PermissionKind::DenyOnce,
                },
            ],
        };
        match self.ask_permission(session, request).await?.as_deref() {
            Some("allow") => Ok(()),
            _ => Err(AgentError::Cancelled),
        }
    }
    async fn permission(&self, params: Value) -> AgentResult<Value> {
        let session = self.scope(&params)?;
        let request = wire::permission(&params, Uuid::new_v4().to_string())?;
        if let Some(tool) = params.get("toolCall") {
            session
                .emit(
                    &self.context,
                    ThreadEvent::ToolChanged {
                        patch: wire::tool_patch(tool)?,
                    },
                )
                .await?;
        }
        let result = self.ask_permission(&session, request).await?;
        Ok(match result {
            Some(id) => json!({"outcome":{"outcome":"selected","optionId":id}}),
            None => json!({"outcome":{"outcome":"cancelled"}}),
        })
    }
    async fn read_file(&self, params: Value) -> AgentResult<Value> {
        let session = self.scope(&params)?;
        let path = PathBuf::from(wire::string(&params, "path")?);
        let root = session.filesystem(&path)?;
        let line = optional_unsigned(&params, "line")?;
        let limit = optional_unsigned(&params, "limit")?;
        let content = tokio::task::spawn_blocking(move || root.read_lines(&path, line, limit))
            .await
            .map_err(|_| AgentError::EventDelivery)??;
        Ok(json!({"content":content}))
    }
    async fn write_file(&self, params: Value) -> AgentResult<Value> {
        let session = self.scope(&params)?;
        let path = PathBuf::from(wire::string(&params, "path")?);
        let root = session.filesystem(&path)?;
        let relative = root.relative(&path)?;
        let content = wire::string(&params, "content")?.to_owned();
        if content.len() > 8 * 1024 * 1024 {
            return Err(AgentError::Limit);
        }
        let read_root = root.clone();
        let read_path = path.clone();
        let previous = tokio::task::spawn_blocking(move || read_root.read(&read_path))
            .await
            .map_err(|_| AgentError::EventDelivery)?;
        let previous = match previous {
            Ok(snapshot) => Some(snapshot),
            Err(RuntimeError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        self.local_approval(
            &session,
            format!("Write {} ({} bytes)", relative.display(), content.len()),
        )
        .await?;
        let bom = previous.as_ref().is_some_and(|snapshot| snapshot.utf8_bom);
        tokio::task::spawn_blocking(move || match previous {
            Some(previous) => root.write(&path, &content, Some(&previous.version), bom),
            None => root.write_new(&path, &content, bom),
        })
        .await
        .map_err(|_| AgentError::EventDelivery)??;
        Ok(Value::Null)
    }
    async fn create_terminal(&self, params: Value) -> AgentResult<Value> {
        if !self.context.host.is_local() {
            return Err(AgentError::Unsupported("remote terminal callbacks".into()));
        }
        let session = self.scope(&params)?;
        let command = wire::string(&params, "command")?.to_owned();
        let args: Vec<String> = match params.get("args") {
            None => vec![],
            Some(_) => wire::array(&params, "args", 256)?
                .iter()
                .map(|value| {
                    value
                        .as_str()
                        .map(str::to_owned)
                        .ok_or_else(|| wire::invalid("invalid terminal argument"))
                })
                .collect::<AgentResult<_>>()?,
        };
        let mut launch = LaunchSpec::new(command);
        launch.args = args;
        if params.get("env").is_some() {
            for env in wire::array(&params, "env", 128)? {
                let name = wire::string(env, "name")?;
                if launch
                    .env
                    .insert(name.into(), wire::string(env, "value")?.into())
                    .is_some()
                {
                    return Err(wire::invalid("duplicate environment variable"));
                }
            }
        }
        launch.validate()?;
        let cwd = params
            .get("cwd")
            .and_then(Value::as_str)
            .map(PathBuf::from)
            .unwrap_or_else(|| session.cwd.clone());
        let roots = session.roots.clone();
        let cwd = tokio::task::spawn_blocking(move || {
            let cwd = cwd.canonicalize().map_err(RuntimeError::from)?;
            if !roots.iter().any(|root| cwd.starts_with(root.root())) {
                return Err(wire::invalid("terminal directory is outside session roots"));
            }
            Ok(cwd)
        })
        .await
        .map_err(|_| AgentError::EventDelivery)??;
        let limit = optional_unsigned(&params, "outputByteLimit")?
            .unwrap_or(1024 * 1024)
            .min(1024 * 1024) as usize;
        let description = format!(
            "Run {} with arguments {} in {}. This command runs with your user permissions, not in a sandbox.",
            launch.command.display(),
            serde_json::to_string(&launch.args).map_err(|_| wire::invalid("invalid arguments"))?,
            cwd.display()
        );
        if description.len() > 64 * 1024 {
            return Err(AgentError::Limit);
        }
        self.local_approval(&session, description).await?;
        // Reserve capacity while creating to avoid concurrent callbacks bypassing the limit.
        let mut terminals = self.terminals.lock().await;
        if terminals.len() >= 64
            || terminals
                .values()
                .filter(|entry| entry.session_id == session.id)
                .count()
                >= 16
        {
            return Err(AgentError::Limit);
        }
        let terminal =
            tokio::task::spawn_blocking(move || NativeTerminal::spawn(&launch, &cwd, 24, 100))
                .await
                .map_err(|_| AgentError::EventDelivery)??;
        if session.interaction().cancelled.is_cancelled() {
            drop(terminal);
            return Err(AgentError::Cancelled);
        }
        let id = Uuid::new_v4().to_string();
        terminals.insert(
            id.clone(),
            TerminalEntry {
                session_id: session.id.clone(),
                terminal: Arc::new(terminal),
                output_limit: limit,
            },
        );
        Ok(json!({"terminalId":id}))
    }
    async fn terminal_operation(&self, method: &str, params: Value) -> AgentResult<Value> {
        let session = self.scope(&params)?;
        let id = wire::id(&params, "terminalId")?;
        let (terminal, limit) = {
            let terminals = self.terminals.lock().await;
            let entry = terminals
                .get(&id)
                .filter(|entry| entry.session_id == session.id)
                .ok_or_else(|| wire::invalid("terminal is not owned by this session"))?;
            (entry.terminal.clone(), entry.output_limit)
        };
        match method {
            "terminal/wait_for_exit" => {
                let cancellation = session.interaction().cancelled;
                tokio::select! {
                    result = terminal.wait() => { result?; },
                    () = cancellation.cancelled() => return Err(AgentError::Cancelled),
                }
            }
            "terminal/kill" | "terminal/release" => {
                let kill = terminal.clone();
                tokio::task::spawn_blocking(move || kill.kill())
                    .await
                    .map_err(|_| AgentError::EventDelivery)??;
                let _ = tokio::time::timeout(Duration::from_secs(5), terminal.wait()).await;
            }
            _ => {}
        }
        let snapshot = terminal.snapshot()?;
        let (output, truncated) = terminal_text(&snapshot, limit);
        session
            .emit(
                &self.context,
                ThreadEvent::TerminalOutput {
                    id: id.clone(),
                    text: output.clone(),
                    truncated,
                    exit_code: snapshot.exit_code,
                },
            )
            .await?;
        let exit_status = snapshot.exit_code.map(|exit| json!({"exitCode":exit}));
        match method {
            "terminal/output" => {
                let mut response = json!({"output":output,"truncated":truncated});
                if let Some(status) = exit_status {
                    response["exitStatus"] = status;
                }
                Ok(response)
            }
            "terminal/wait_for_exit" => Ok(exit_status.unwrap_or_else(|| json!({}))),
            "terminal/release" => {
                self.terminals.lock().await.remove(&id);
                Ok(json!({}))
            }
            _ => Ok(json!({})),
        }
    }
    pub async fn stop_terminals(&self, session_id: Option<&str>) -> AgentResult<()> {
        let terminals = {
            let mut entries = self.terminals.lock().await;
            let ids: Vec<_> = entries
                .iter()
                .filter(|(_, entry)| session_id.is_none_or(|id| id == entry.session_id))
                .map(|(id, _)| id.clone())
                .collect();
            ids.into_iter()
                .filter_map(|id| entries.remove(&id).map(|entry| (id, entry)))
                .collect::<Vec<_>>()
        };
        let mut failure = None;
        // kill only sets the worker's stop flag and unparks it on both platforms.
        // Signal every owner before awaiting any process or diagnostic consumer.
        for (_, entry) in &terminals {
            if let Err(error) = entry.terminal.kill() {
                failure.get_or_insert(AgentError::Runtime(error));
            }
        }
        let exit_deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        for (_, entry) in &terminals {
            match tokio::time::timeout_at(exit_deadline, entry.terminal.wait()).await {
                Ok(Ok(_)) => {}
                Ok(Err(error)) => {
                    failure.get_or_insert(AgentError::Runtime(error));
                }
                Err(_) => {
                    failure.get_or_insert(AgentError::Runtime(RuntimeError::Timeout));
                    break;
                }
            }
        }
        // Final output is best effort, but a delivery failure is returned, not
        // silently treated as a successful cleanup. The budget is shared by all
        // terminals rather than multiplied by the number of owned processes.
        let delivery_deadline = tokio::time::Instant::now() + Duration::from_secs(15);
        for (id, entry) in terminals {
            if let Ok(session) = self.sessions.get(&entry.session_id) {
                let snapshot = match entry.terminal.snapshot() {
                    Ok(snapshot) => snapshot,
                    Err(error) => {
                        failure.get_or_insert(AgentError::Runtime(error));
                        continue;
                    }
                };
                let (text, truncated) = terminal_text(&snapshot, entry.output_limit);
                let result = tokio::time::timeout_at(
                    delivery_deadline,
                    session.emit(
                        &self.context,
                        ThreadEvent::TerminalOutput {
                            id,
                            text,
                            truncated,
                            exit_code: snapshot.exit_code,
                        },
                    ),
                )
                .await;
                if !matches!(result, Ok(Ok(()))) {
                    failure.get_or_insert(AgentError::EventDelivery);
                    break;
                }
            }
        }
        failure.map_or(Ok(()), Err)
    }
    async fn elicit(&self, params: Value, peer: &RpcPeer) -> AgentResult<Value> {
        let mut interaction = if params.get("sessionId").is_some() {
            if params.get("requestId").is_some() {
                return Err(wire::invalid("elicitation has conflicting scopes"));
            }
            self.scope(&params)?.interaction()
        } else {
            let id = params
                .get("requestId")
                .ok_or_else(|| wire::invalid("elicitation scope missing"))?;
            let cancelled = peer
                .request_lifetime(&RpcId::parse(id)?)
                .ok_or_else(|| wire::invalid("elicitation refers to an inactive request"))?;
            InteractionContext {
                scope: InteractionScope::Connection(self.connection_id),
                thread_id: self.connection_thread,
                session_id: "connection".into(),
                cancelled,
            }
        };
        let request = elicitation::parse(&params, Uuid::new_v4().to_string())?;
        let request_id = request.id.clone();
        let owner = interaction.cancelled.clone();
        interaction.cancelled = owner.child_token();
        let mut registration = if request.url.is_some() {
            Some(self.elicitations.register(
                &wire::id(&params, "elicitationId")?,
                &request.id,
                owner,
                interaction.cancelled.clone(),
            )?)
        } else {
            None
        };
        // Request-scoped login prompts use the broker without inventing persistent task ownership.
        if interaction.scope == InteractionScope::Session {
            self.context
                .events
                .emit(
                    interaction.thread_id,
                    ThreadEvent::UserInputRequested {
                        request: if request.url.is_some() {
                            let mut durable = request.clone();
                            durable.url = None;
                            durable.message = "Agent requested a website interaction. Its address is not stored in conversation history.".into();
                            durable
                        } else { request.clone() },
                    },
                )
                .await?;
        }
        let result = self
            .context
            .interactions
            .input(interaction.clone(), request.clone())
            .await;
        if interaction.scope == InteractionScope::Session {
            self.context
                .events
                .emit(
                    interaction.thread_id,
                    ThreadEvent::UserInputResolved { id: request_id },
                )
                .await?;
        }
        let result = if interaction.cancelled.is_cancelled() {
            UserInputResponse::Cancel
        } else {
            result?
        };
        Ok(match result {
            UserInputResponse::Accept { values } => {
                synara_agent::validate_input(&request, &values)?;
                if request.url.is_some() {
                    if let Some(registration) = &mut registration {
                        registration.accepted();
                    }
                    json!({"action":"accept"})
                } else {
                    json!({"action":"accept","content":values})
                }
            }
            UserInputResponse::Decline => json!({"action":"decline"}),
            UserInputResponse::Cancel => json!({"action":"cancel"}),
        })
    }
    pub async fn elicitation_complete(&self, params: &Value) {
        if let Some(id) = params.get("elicitationId").and_then(Value::as_str) {
            self.elicitations.complete(id);
        }
    }
}
fn optional_unsigned(value: &Value, key: &str) -> AgentResult<Option<u64>> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_u64()
            .map(Some)
            .ok_or_else(|| wire::invalid(format!("invalid {key}"))),
    }
}
fn terminal_text(snapshot: &TerminalSnapshot, limit: usize) -> (String, bool) {
    let text = String::from_utf8_lossy(&snapshot.raw_tail);
    let mut start = text.len().saturating_sub(limit);
    while !text.is_char_boundary(start) {
        start += 1;
    }
    (text[start..].into(), snapshot.truncated || start > 0)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn terminal_output_limits_preserve_utf8_boundaries() {
        let snapshot = TerminalSnapshot {
            text: String::new(),
            raw_tail: "a🦀æ".as_bytes().to_vec(),
            truncated: false,
            exit_code: None,
            error: None,
            revision: 0,
        };
        let (text, truncated) = terminal_text(&snapshot, 5);
        assert_eq!(text, "æ");
        assert!(truncated);
        assert_eq!(terminal_text(&snapshot, 0).0, "");
    }
}

#[cfg(test)]
#[path = "callback_lifecycle_tests.rs"]
mod lifecycle_tests;
