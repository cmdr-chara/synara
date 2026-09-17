//! A deterministic, credential-free external process used only by integration tests.
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    io::{self, BufRead, Write},
};

struct Fixture {
    profile: String,
    authenticated: bool,
    next_session: u64,
    next_callback: u64,
    sessions: HashMap<String, String>,
    pending: HashMap<String, Value>,
    callbacks: HashMap<String, Callback>,
}
struct Callback {
    session: String,
    prompt: Value,
    stage: String,
    terminal: Option<String>,
}
impl Fixture {
    fn send(&self, value: Value) {
        let mut out = io::stdout().lock();
        writeln!(out, "{value}").unwrap();
        out.flush().unwrap();
    }
    fn ok(&self, id: Value, result: Value) {
        self.send(json!({"jsonrpc":"2.0","id":id,"result":result}));
    }
    fn error(&self, id: Value, code: i64) {
        self.send(json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":"fixture error"}}));
    }
    fn update(&self, session: &str, update: Value) {
        self.send(json!({"jsonrpc":"2.0","method":"session/update","params":{"sessionId":session,"update":update}}));
    }
    fn text(&self, session: &str, text: &str) {
        self.update(
            session,
            json!({"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":text}}),
        );
    }
    fn configuration(&self) -> Value {
        json!({"configOptions":[{"id":"model","name":"Model","category":"model","type":"select","currentValue":self.profile,"options":[{"value":self.profile,"name":self.profile},{"value":"alternate","name":"Alternate"}]},{"id":"review","name":"Review first","type":"boolean","currentValue":true}],"modes":{"currentModeId":"code","availableModes":[{"id":"code","name":"Code"},{"id":"plan","name":"Plan"}]}})
    }
    fn request(
        &mut self,
        session: String,
        prompt: Value,
        stage: &str,
        method: &str,
        params: Value,
        terminal: Option<String>,
    ) {
        self.next_callback += 1;
        let id = format!("callback-{}", self.next_callback);
        self.callbacks.insert(
            id.clone(),
            Callback {
                session,
                prompt,
                stage: stage.into(),
                terminal,
            },
        );
        self.send(json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}));
    }
    fn finish(&self, session: &str, prompt: Value, text: &str) {
        self.text(session, text);
        self.ok(prompt, json!({"stopReason":"end_turn"}));
    }
    fn receive(&mut self, value: Value) {
        let id = value.get("id").cloned().unwrap_or(Value::Null);
        let params = &value["params"];
        let session = params["sessionId"].as_str().unwrap_or("").to_owned();
        match value["method"].as_str() {
            Some("initialize") => {
                if params["protocolVersion"] != 1 {
                    self.error(id, -32602);
                    return;
                }
                let caps = if self.profile == "beta" {
                    json!({"promptCapabilities":{}})
                } else {
                    json!({"loadSession":true,"promptCapabilities":{"image":true},"sessionCapabilities":{"list":{},"resume":{},"close":{},"delete":{},"additionalDirectories":{}},"authCapabilities":{"logout":true}})
                };
                self.ok(id,json!({"protocolVersion":1,"agentInfo":{"name":format!("fixture-{}",self.profile),"version":"1.0.0"},"agentCapabilities":caps,"authMethods":[{"id":"test-login","name":"Test login"}]}));
            }
            Some("authenticate") => {
                self.authenticated = true;
                self.ok(id, json!({}));
            }
            Some("logout") => {
                self.authenticated = false;
                self.ok(id, json!({}));
            }
            Some("session/new") => {
                if !self.authenticated {
                    self.error(id, -32000);
                    return;
                }
                self.next_session += 1;
                let session = format!("session-{}", self.next_session);
                self.sessions
                    .insert(session.clone(), params["cwd"].as_str().unwrap().into());
                // Exercise updates delivered before the new-session response.
                self.update(
                    &session,
                    json!({"sessionUpdate":"session_info_update","title":"Fixture task"}),
                );
                let mut result = self.configuration();
                result["sessionId"] = json!(session);
                self.ok(id, result);
            }
            Some("session/load" | "session/resume") => {
                self.sessions
                    .insert(session.clone(), params["cwd"].as_str().unwrap().into());
                if value["method"] == "session/load" {
                    self.update(&session,json!({"sessionUpdate":"user_message_chunk","content":{"type":"text","text":"Earlier question"}}));
                    self.text(&session, "Earlier answer");
                }
                if session == "bad-history" {
                    self.error(id, -32603);
                } else {
                    self.ok(id, self.configuration());
                }
            }
            Some("session/list") => {
                self.ok(id,json!({"sessions":self.sessions.iter().map(|(id,cwd)|json!({"sessionId":id,"cwd":cwd,"title":"Fixture task"})).collect::<Vec<_>>() }));
            }
            Some("session/close" | "session/delete") => {
                self.sessions.remove(&session);
                self.ok(id, json!({}));
            }
            Some("session/set_config_option") => {
                let mut config = self.configuration();
                if let Some(options) = config["configOptions"].as_array_mut() {
                    for option in options {
                        if option["id"] == params["configId"] {
                            option["currentValue"] = params["value"].clone();
                        }
                    }
                }
                self.ok(id, config);
            }
            Some("session/set_mode" | "session/set_model") => self.ok(id, json!({})),
            Some("session/cancel") => {
                if let Some(prompt) = self.pending.remove(&session) {
                    self.text(&session, "Late update before cancellation");
                    self.ok(prompt, json!({"stopReason":"cancelled"}));
                }
            }
            Some("session/prompt") => {
                let text = params["prompt"][0]["text"].as_str().unwrap_or("");
                match text {
                    "hold"|"timeout"=>{ self.text(&session,"Started waiting"); self.pending.insert(session,id); }
                    "crash"=> std::process::exit(23),
                    "malformed"=>{println!("not JSON");io::stdout().flush().unwrap();}
                    "permission"=> self.request(session.clone(),id,"permission","session/request_permission",json!({"sessionId":session,"toolCall":{"toolCallId":"tool-1","title":"Read a file","status":"pending"},"options":[{"optionId":"allow","name":"Allow once","kind":"allow_once"},{"optionId":"deny","name":"Deny","kind":"reject_once"}]}),None),
                    "files"=> {let path=format!("{}/created.txt",self.sessions[&session]); self.request(session.clone(),id,"write","fs/write_text_file",json!({"sessionId":session,"path":path,"content":"native 🦀\n"}),None);}
                    "escape"=>self.request(session.clone(),id,"escape","fs/read_text_file",json!({"sessionId":session,"path":"/etc/passwd"}),None),
                    "unknown-owner"=>self.request(session,id,"escape","fs/read_text_file",json!({"sessionId":"not-owned","path":"/etc/passwd"}),None),
                    "terminal"=>self.request(session.clone(),id,"terminal-create","terminal/create",json!({"sessionId":session,"command":"/bin/sh","args":["-c","printf 'terminal-proof\\n'"],"outputByteLimit":1024}),None),
                    "input"=>self.request(session.clone(),id,"input","elicitation/create",json!({"sessionId":session,"mode":"form","message":"Pick a value","requestedSchema":{"type":"object","properties":{"name":{"type":"string"}},"required":["name"]}}),None),
                    _=>{self.text(&session,"Hello ");self.text(&session,"from ");self.text(&session,&self.profile);self.update(&session,json!({"sessionUpdate":"tool_call","toolCallId":"tool-1","title":"Inspect","status":"in_progress","content":[{"type":"content","content":{"type":"text","text":"tool output"}}]}));self.update(&session,json!({"sessionUpdate":"tool_call_update","toolCallId":"tool-1","status":"completed"}));self.ok(id,json!({"stopReason":"end_turn"}));}
                }
            }
            Some(_) => self.error(id, -32601),
            None => {
                let Some(callback) = id.as_str().and_then(|id| self.callbacks.remove(id)) else {
                    return;
                };
                let s = callback.session;
                let p = callback.prompt;
                if value.get("error").is_some() {
                    self.finish(&s, p, "callback denied");
                    return;
                }
                match callback.stage.as_str() {
                    "permission" => self.finish(
                        &s,
                        p,
                        value["result"]["outcome"]["outcome"]
                            .as_str()
                            .unwrap_or("invalid"),
                    ),
                    "write" => {
                        let path = format!("{}/created.txt", self.sessions[&s]);
                        self.request(
                            s.clone(),
                            p,
                            "read",
                            "fs/read_text_file",
                            json!({"sessionId":s,"path":path}),
                            None,
                        );
                    }
                    "read" => self.finish(
                        &s,
                        p,
                        value["result"]["content"].as_str().unwrap_or("no content"),
                    ),
                    "terminal-create" => {
                        let terminal = value["result"]["terminalId"].as_str().unwrap().to_owned();
                        self.request(
                            s.clone(),
                            p,
                            "terminal-wait",
                            "terminal/wait_for_exit",
                            json!({"sessionId":s,"terminalId":terminal}),
                            Some(terminal),
                        );
                    }
                    "terminal-wait" => {
                        let terminal = callback.terminal.unwrap();
                        self.request(
                            s.clone(),
                            p,
                            "terminal-output",
                            "terminal/output",
                            json!({"sessionId":s,"terminalId":terminal}),
                            Some(terminal),
                        );
                    }
                    "terminal-output" => {
                        self.text(
                            &s,
                            value["result"]["output"].as_str().unwrap_or("no output"),
                        );
                        let terminal = callback.terminal.unwrap();
                        self.request(
                            s.clone(),
                            p,
                            "terminal-release",
                            "terminal/release",
                            json!({"sessionId":s,"terminalId":terminal}),
                            Some(terminal),
                        );
                    }
                    "terminal-release" => self.finish(&s, p, "released"),
                    "input" => self.finish(&s, p, &value["result"].to_string()),
                    _ => self.finish(&s, p, "unexpected access"),
                }
            }
        }
    }
}
fn main() {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() != Some("--integration-fixture") {
        eprintln!("This executable is only an integration-test fixture.");
        std::process::exit(2);
    }
    let profile = args.next().unwrap_or("alpha".into());
    let authenticated = profile != "auth";
    let mut fixture = Fixture {
        profile,
        authenticated,
        next_session: 0,
        next_callback: 0,
        sessions: HashMap::new(),
        pending: HashMap::new(),
        callbacks: HashMap::new(),
    };
    for line in io::stdin().lock().lines() {
        let Ok(line) = line else {
            break;
        };
        if let Ok(value) = serde_json::from_str(&line) {
            fixture.receive(value);
        }
    }
}
