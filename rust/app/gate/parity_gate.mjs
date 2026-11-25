// Synara native-backend parity gate: behavioral scenarios against the Rust
// server booted with fake provider fixtures. Each scenario asserts behavior
// that was ported from upstream; a failure means a parity regression.
// Usage: node parity_gate.mjs <ws-url> <project-id> <state.sqlite-path>
import { execSync } from "node:child_process";
import fs from "node:fs";

const [wsUrl, projectId, dbPath] = process.argv.slice(2);
const ws = new WebSocket(wsUrl);
let next = 1;
const pending = new Map();
function call(tag, payload, timeoutMs = 15000) {
  return new Promise((resolve, reject) => {
    const id = String(next++);
    pending.set(id, { resolve, reject, tag });
    ws.send(JSON.stringify({ _tag: "Request", id, tag, payload, headers: [] }));
    setTimeout(() => {
      if (pending.has(id)) {
        pending.delete(id);
        reject(new Error("timeout " + tag));
      }
    }, timeoutMs);
  });
}

// Streaming variant of call(): resolves {chunks, value} collecting Chunk frames.
function callStreaming(tag, payload, timeoutMs = 20000) {
  return new Promise((resolve, reject) => {
    const id = String(next++);
    const chunks = [];
    pending.set(id, { resolve, reject, tag, chunks });
    ws.send(JSON.stringify({ _tag: "Request", id, tag, payload, headers: [] }));
    setTimeout(() => {
      if (pending.has(id)) {
        pending.delete(id);
        reject(new Error("timeout " + tag));
      }
    }, timeoutMs);
  });
}

ws.onmessage = (ev) => {
  const m = JSON.parse(ev.data);
  if (m._tag === "Chunk") {
    const p = pending.get(String(m.requestId));
    if (p?.chunks) for (const v of m.values ?? []) p.chunks.push(v);
    return;
  }
  if (m._tag === "Exit") {
    const p = pending.get(String(m.requestId));
    if (!p) return;
    pending.delete(String(m.requestId));
    if (m.exit?._tag === "Success")
      p.resolve(p.chunks ? { chunks: p.chunks, value: m.exit.value } : m.exit.value);
    else p.reject(new Error(p.tag + ": " + JSON.stringify(m.exit).slice(0, 300)));
  }
};
await new Promise((res, rej) => {
  ws.onopen = res;
  setTimeout(() => rej(new Error("open timeout")), 4000);
});

const results = [];
async function scenario(name, fn) {
  try {
    await fn();
    results.push(["PASS", name]);
  } catch (e) {
    results.push(["FAIL", name + " — " + String(e.message || e).slice(0, 200)]);
  }
}
function assert(cond, msg) {
  if (!cond) throw new Error(msg);
}
function sql(query) {
  return execSync(`sqlite3 ${JSON.stringify(dbPath)} ${JSON.stringify(query)}`)
    .toString()
    .trim();
}
const now = () => new Date().toISOString();
function threadCreateCmd(threadId, provider, model) {
  return {
    type: "thread.create",
    commandId: "c-" + threadId,
    threadId,
    projectId,
    title: "gate " + provider,
    createdAt: now(),
    modelSelection: { provider, model },
    runtimeMode: "full-access",
    interactionMode: "default",
  };
}
function turnCmd(threadId, provider, model, text) {
  return {
    type: "thread.turn.start",
    commandId: "t-" + threadId,
    threadId,
    createdAt: now(),
    dispatchMode: "queue",
    dispatchOrigin: "user",
    runtimeMode: "full-access",
    interactionMode: "default",
    modelSelection: { provider, model },
    message: { messageId: "m-" + threadId, text, attachments: [] },
  };
}
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
async function runTurn(provider, model, text, waitMs = 2500) {
  const threadId = "gate-" + provider + "-" + Date.now();
  await call("orchestration.dispatchCommand", threadCreateCmd(threadId, provider, model));
  await call("orchestration.dispatchCommand", turnCmd(threadId, provider, model, text));
  await sleep(waitMs);
  return threadId;
}

// 1. runtime status responds
await scenario("server.getRuntimeStatus responds", async () => {
  const r = await call("server.getRuntimeStatus", {});
  assert(r && typeof r === "object", "no runtime status object");
});

// 2. codex turn end-to-end
await scenario("codex turn completes with assistant text", async () => {
  const tid = await runTurn("codex", "gpt-5.5", "hello");
  const rows = sql(
    `select count(*) from projection_thread_messages where thread_id='${tid}' and role='assistant' and length(text) > 0`,
  );
  assert(Number(rows) >= 1, "no assistant message persisted");
});

// 3. claude turn end-to-end
await scenario("claude turn completes with assistant text", async () => {
  const tid = await runTurn("claudeAgent", "claude-sonnet-4-5", "hello");
  const rows = sql(
    `select count(*) from projection_thread_messages where thread_id='${tid}' and role='assistant' and length(text) > 0`,
  );
  assert(Number(rows) >= 1, "no assistant message persisted");
});

// 4. ACP omp turn
await scenario("omp ACP turn completes", async () => {
  const tid = await runTurn("omp", "openai/gpt-5", "hello");
  const rows = sql(
    `select count(*) from projection_thread_messages where thread_id='${tid}' and role='assistant' and length(text) > 0`,
  );
  assert(Number(rows) >= 1, "no assistant message persisted");
});

// 5. passive usage snapshot after codex rlimit turn
await scenario("codex rate-limit evidence becomes a usage snapshot", async () => {
  await runTurn("codex", "gpt-5.5", "rlimit", 3000);
  const snap = await call("server.getProviderUsageSnapshot", { provider: "codex" });
  assert(snap.provider === "codex", "wrong provider");
  assert(Array.isArray(snap.limits) && snap.limits.length >= 1, "no limits");
  assert(snap.limits[0].usedPercent === 100, "expected exhausted window");
});

// 6. textgen: thread recap
await scenario("server.generateThreadRecap via Claude CLI", async () => {
  const r = await call("server.generateThreadRecap", {
    cwd: "/tmp",
    newMaterial: "shipped the gate harness",
  });
  assert(typeof r.recap === "string" && r.recap.length > 10, "no recap");
});

// 7. textgen: automation intent
await scenario("server.generateAutomationIntent via Claude CLI", async () => {
  const r = await call("server.generateAutomationIntent", {
    cwd: "/tmp",
    message: "every day update the count",
    nowIso: now(),
  });
  assert(r.isAutomation === true, "not detected as automation");
  assert(r.schedule?.type === "interval", "no interval schedule");
});

// 8. composer prediction outcome semantics
await scenario("composer prediction unavailable for unknown thread", async () => {
  const r = await call("server.generateComposerPrediction", {
    threadId: "nonexistent",
    expectedTurnId: "x",
  });
  assert(r.suggestion === null && r.outcome === "unavailable", "expected unavailable");
});

// 9. stdio MCP connector connect + list
await scenario("stdio MCP connector connect/list roundtrip", async () => {
  const r = await call("server.connectMcpConnector", {
    name: "Gate Server",
    command: process.env.GATE_FIXTURES + "/fake-mcp-stdio",
    args: [],
  });
  assert(r.connector.status === "connected", "probe not connected: " + r.connector.status);
  assert(
    r.connector.serverName === "gate_server_connector",
    "bad serverName " + r.connector.serverName,
  );
  const list = await call("server.listMcpConnectors", {});
  assert(
    list.some((c) => c.connectorId === r.connector.connectorId),
    "not in list",
  );
  assert(!list.find((c) => c.connectorId === r.connector.connectorId).env, "env leaked");
});

// 10. turn.start reviewTarget rejects attachments
await scenario("turn.start reviewTarget rejects attachments", async () => {
  const tid = "gate-review-" + Date.now();
  await call("orchestration.dispatchCommand", threadCreateCmd(tid, "codex", "gpt-5.5"));
  const cmd = turnCmd(tid, "codex", "gpt-5.5", "review this");
  cmd.reviewTarget = { type: "uncommitted" };
  cmd.message.attachments = [{ kind: "file", path: "/tmp/x" }];
  let rejected = false;
  try {
    await call("orchestration.dispatchCommand", cmd);
  } catch (e) {
    rejected = e.message.includes("review") || e.message.includes("attachments");
  }
  assert(rejected, "review+attachments not rejected");
});

// 11. consumeCodexResetCredit outcome
await scenario("server.consumeCodexResetCredit returns an outcome", async () => {
  const r = await call("server.consumeCodexResetCredit", {
    accountId: "acct-test-1",
    idempotencyKey: "gate-key",
  });
  assert(
    ["reset", "nothingToReset", "noCredit", "alreadyRedeemed"].includes(r.outcome),
    "bad outcome " + r.outcome,
  );
});

// 12. claude edge-block retry re-prompts on the same session and completes
await scenario("claude edge-block 403 retried and turn completes", async () => {
  const before = fs.existsSync("/tmp/fake-claude-user-log")
    ? fs.readFileSync("/tmp/fake-claude-user-log", "utf8").trim().split("\n").length
    : 0;
  const tid = await runTurn("claudeAgent", "claude-sonnet-4-5", "edgeblock please retry", 4000);
  const rows = sql(
    `select count(*) from projection_thread_messages where thread_id='${tid}' and role='assistant' and length(text) > 0`,
  );
  assert(Number(rows) >= 1, "no assistant message after edge retry");
  const after = fs.readFileSync("/tmp/fake-claude-user-log", "utf8").trim().split("\n").length;
  assert(
    after - before >= 2,
    "expected at least 2 prompts (initial + retry), got " + (after - before),
  );
});

// 13. claude rate_limit_info evidence persisted as passive usage snapshot
await scenario("claude rate-limit evidence persisted for usage snapshot", async () => {
  await runTurn("claudeAgent", "claude-sonnet-4-5", "rlimit hit now", 3000);
  const snap = await call("server.getProviderUsageSnapshot", { provider: "claudeAgent" });
  assert(snap.provider === "claudeAgent", "wrong provider");
});

// 14. claude task tracking: TodoWrite + TaskCreate surface as turn.tasks.updated activities
await scenario("claude task tools emit turn.tasks.updated activities", async () => {
  const tid = await runTurn("claudeAgent", "claude-sonnet-4-5", "tasklist demo", 3000);
  const rows = sql(
    `select count(*) from projection_thread_activities where thread_id='${tid}' and kind='turn.tasks.updated'`,
  );
  assert(Number(rows) >= 2, "expected >=2 task-list activities, got " + rows);
  const payload = sql(
    `select payload_json from projection_thread_activities where thread_id='${tid}' and kind='turn.tasks.updated' order by sequence desc limit 1`,
  );
  assert(payload.includes("implement parser"), "tracked TaskCreate task missing from payload");
});

// 15. editor.intelligence: TypeScript worker diagnostics + hover
await scenario("editor.intelligence diagnostics and hover via ts worker", async () => {
  const contents = "const n: number = 'oops';\nconsole.log(n);\n";
  const base = {
    cwd: process.env.GATE_REPO_ROOT,
    relativePath: "scratch-gate.ts",
    documents: [{ relativePath: "scratch-gate.ts", contents, version: 1 }],
    length: contents.length,
  };
  const diag = await call(
    "editor.intelligence",
    { ...base, operation: "diagnostics", offset: 0 },
    30000,
  );
  assert(
    Array.isArray(diag.diagnostics) && diag.diagnostics.length >= 1,
    "no diagnostics for type error",
  );
  const hover = await call(
    "editor.intelligence",
    { ...base, operation: "hover", offset: contents.indexOf("console") + 2, length: 0 },
    30000,
  );
  assert(hover && typeof hover === "object", "no hover result");
});

// 16. git.runStackedAction streaming + resume dedup registry
await scenario("git stacked action streams chunks and resume dedups", async () => {
  const repo = "/tmp/gate-git-repo";
  fs.rmSync(repo, { recursive: true, force: true });
  fs.mkdirSync(repo, { recursive: true });
  const gitEnv = {
    env: {
      ...process.env,
      GIT_AUTHOR_NAME: "Gate",
      GIT_AUTHOR_EMAIL: "gate@test",
      GIT_COMMITTER_NAME: "Gate",
      GIT_COMMITTER_EMAIL: "gate@test",
    },
  };
  execSync("git init -q", { cwd: repo, ...gitEnv });
  fs.writeFileSync(repo + "/a.txt", "one\n");
  execSync("git add a.txt && git commit -qm init", { cwd: repo, ...gitEnv });
  fs.writeFileSync(repo + "/b.txt", "two\n");
  const countCommits = () =>
    Number(execSync("git rev-list --count HEAD", { cwd: repo }).toString().trim());
  const before = countCommits();

  const actionId = "gate-action-" + Date.now();
  const payload = {
    cwd: repo,
    actionId,
    action: "commit",
    commitMessage: "gate commit",
    featureBranch: false,
  };
  const first = await callStreaming("git.runStackedAction", payload);
  const kinds = first.chunks.map((c) => c.kind);
  assert(kinds.includes("action_started"), "no action_started chunk: " + kinds.join(","));
  assert(kinds.includes("action_finished"), "no action_finished chunk: " + kinds.join(","));
  assert(countCommits() === before + 1, "commit not created");

  // Resume with the same payload attaches to the completed action and must
  // NOT create a second commit.
  const resumed = await callStreaming("git.runStackedAction", { ...payload, resume: true });
  assert(
    resumed.chunks.some((c) => c.kind === "action_finished"),
    "resume did not replay terminal event",
  );
  assert(countCommits() === before + 1, "resume duplicated the mutation");

  // Same actionId with different inputs is rejected.
  let mismatch = false;
  try {
    await callStreaming("git.runStackedAction", { ...payload, commitMessage: "different" });
  } catch (e) {
    mismatch = e.message.includes("reused with different inputs");
  }
  assert(mismatch, "fingerprint mismatch not rejected");

  // Resume of an unknown action is rejected with the upstream message.
  let missing = false;
  try {
    await callStreaming("git.runStackedAction", {
      ...payload,
      actionId: "never-existed",
      resume: true,
    });
  } catch (e) {
    missing = e.message.includes("result is unavailable");
  }
  assert(missing, "resume-missing not rejected");
});

// 17. claude workflow observability: launch detection + transcript poller
await scenario("claude workflow launch emits task.updated and task.progress", async () => {
  const dir = "/tmp/gate-wf-transcripts";
  fs.rmSync(dir, { recursive: true, force: true });
  fs.mkdirSync(dir, { recursive: true });
  fs.writeFileSync("/tmp/gate-wf.js", 'agent("scan", { label: "scanner", phase: "One" });\n');
  fs.writeFileSync(dir + "/journal.jsonl", '{"type":"started","agentId":"agent-1"}\n');
  fs.writeFileSync(
    dir + "/agent-agent-1.jsonl",
    '{"type":"user","timestamp":"2025-06-01T10:00:00Z","message":{"content":"scan the repo"}}\n',
  );
  const tid = await runTurn("claudeAgent", "claude-sonnet-4-5", "workflowdemo run", 3000);
  const updated = sql(
    `select count(*) from projection_thread_activities where thread_id='${tid}' and kind='task.updated'`,
  );
  assert(Number(updated) >= 1, "no task.updated activity for workflow launch");
  // The poller ticks every 2s; give it time to read the seeded transcripts.
  await sleep(5000);
  const progress = sql(
    `select payload_json from projection_thread_activities where thread_id='${tid}' and kind='task.progress' order by sequence desc limit 1`,
  );
  assert(progress.includes("workflowAgents"), "no workflowAgents progress payload");
  assert(progress.includes("agent-1"), "agent snapshot missing from progress");
  assert(progress.includes("scanner"), "zipped label missing from progress");
});

let pass = 0,
  fail = 0;
for (const [status, name] of results) {
  console.log(`[${status}] ${name}`);
  if (status === "PASS") pass++;
  else fail++;
}
console.log(`\n${pass} passed, ${fail} failed`);
ws.close();
process.exit(fail === 0 ? 0 : 1);
