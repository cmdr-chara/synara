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
  return execSync(`sqlite3 -cmd ".timeout 8000" ${JSON.stringify(dbPath)} ${JSON.stringify(query)}`)
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

// 18. claude subagent task_progress/task_notification attribution
await scenario("claude subagent task progress attributed, unknown dropped", async () => {
  const tid = await runTurn("claudeAgent", "claude-sonnet-4-5", "subagentdemo run", 3000);
  const progress = sql(
    `select payload_json from projection_thread_activities where thread_id='${tid}' and kind='task.progress' limit 1`,
  );
  assert(progress.includes("task-1"), "no attributed task.progress");
  assert(progress.includes("Read"), "lastToolName missing");
  const dropped = sql(
    `select count(*) from projection_thread_activities where thread_id='${tid}' and payload_json like '%must be dropped%'`,
  );
  assert(Number(dropped) === 0, "unattributable event was not dropped");
  const terminal = sql(
    `select payload_json from projection_thread_activities where thread_id='${tid}' and kind='task.updated' and payload_json like '%found 12 files%' limit 1`,
  );
  assert(terminal.includes("completed"), "no terminal task.updated with status");
});

// 19. automation execution: a due occurrence fires end-to-end
await scenario("due automation fires a turn and reconciles", async () => {
  const aid = "gate-auto-" + Date.now();
  sql(
    `insert into automation_definitions (automation_id, project_id, source_thread_id, name, prompt, schedule_json, enabled, next_run_at, model_selection_json, provider_options_json, runtime_mode, interaction_mode, worktree_mode, mode, target_thread_id, max_iterations, stop_on_error, minimum_interval_seconds, retry_policy_json, misfire_policy, acknowledged_risks_json, iteration_count, created_at, updated_at) values ('${aid}', '${projectId}', NULL, 'Gate Automation', 'hello', '{\"type\":\"interval\",\"everySeconds\":3600}', 1, '2020-01-01T00:00:00.000Z', '{\"provider\":\"omp\",\"model\":\"openai/gpt-5\"}', '{}', 'worktree', 'autonomous', 'reuse', 'standalone', NULL, NULL, 0, 60, '{}', 'fire-if-due', '{}', 0, '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z')`,
  );
  // The scheduler ticks every 60s; the first pass runs ~5s after boot, but
  // this automation is inserted after boot, so wait for the next pass.
  await sleep(16000);
  const run = sql(
    `select status, thread_id, turn_id from automation_runs where automation_id='${aid}' limit 1`,
  );
  assert(run.length > 0, "no automation run created");
  assert(run.includes("completed"), `run not completed: ${run}`);
  const threadId = sql(`select thread_id from automation_runs where automation_id='${aid}' limit 1`)
    .trim()
    .split("\n")[0];
  const messages = sql(
    `select count(*) from projection_thread_messages where thread_id='${threadId}' and role='assistant'`,
  );
  assert(Number(messages) >= 1, "automation turn produced no assistant message");
  const advanced = sql(
    `select next_run_at from automation_definitions where automation_id='${aid}'`,
  );
  assert(!advanced.includes("2020-01-01"), "schedule was not advanced past the fired occurrence");
});

// 20. automation stop policies: completion evaluation + max iterations
await scenario("automation stop policies settle after terminal runs", async () => {
  const suffix = Date.now();
  const aidCompletion = "gate-auto-comp-" + suffix;
  const aidMaxIter = "gate-auto-maxiter-" + suffix;
  const insertDef = (aid, extra) =>
    sql(
      `insert into automation_definitions (automation_id, project_id, source_thread_id, name, prompt, schedule_json, enabled, next_run_at, model_selection_json, provider_options_json, runtime_mode, interaction_mode, worktree_mode, mode, target_thread_id, max_iterations, stop_on_error, minimum_interval_seconds, retry_policy_json, misfire_policy, acknowledged_risks_json, iteration_count, created_at, updated_at, completion_policy_json) values ('${aid}', '${projectId}', NULL, 'Stop Policy Test', 'hello', '{\"type\":\"interval\",\"everySeconds\":3600}', 1, '2020-01-01T00:00:00.000Z', '{\"provider\":\"omp\",\"model\":\"openai/gpt-5\"}', '{}', 'worktree', 'autonomous', 'reuse', 'standalone', NULL, ${extra.maxIter}, 0, 60, '{}', 'fire-if-due', '{}', 0, '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z', '${extra.policy}')`,
    );
  insertDef(aidCompletion, {
    maxIter: "NULL",
    policy:
      '{\"type\":\"ai-evaluated\",\"stopWhen\":\"the goal is met\",\"confidenceThreshold\":0.8}',
  });
  insertDef(aidMaxIter, { maxIter: "1", policy: '{\"type\":\"none\"}' });
  // tick at 3s: fire+dispatch; turn ~2s; next tick: reconcile+settle; AI eval
  // adds a few seconds for the completion path — allow four ticks.
  await sleep(20000);
  const comp = sql(
    `select enabled, disabled_reason from automation_definitions where automation_id='${aidCompletion}'`,
  );
  assert(
    comp.includes("completion"),
    `completion automation not disabled by stop condition: ${comp}`,
  );
  const compRun = sql(
    `select result_json from automation_runs where automation_id='${aidCompletion}' limit 1`,
  );
  assert(compRun.includes("stopMatched"), `no completionEvaluation recorded: ${compRun}`);
  const maxiter = sql(
    `select enabled, disabled_reason, iteration_count from automation_definitions where automation_id='${aidMaxIter}'`,
  );
  assert(maxiter.includes("max-iterations"), `max-iterations automation not disabled: ${maxiter}`);
});

// 21. manual automation run dispatches through the scheduler
await scenario("manual automation.runNow dispatches and completes", async () => {
  const aid = "gate-auto-manual-" + Date.now();
  sql(
    `insert into automation_definitions (automation_id, project_id, source_thread_id, name, prompt, schedule_json, enabled, next_run_at, model_selection_json, provider_options_json, runtime_mode, interaction_mode, worktree_mode, mode, target_thread_id, max_iterations, stop_on_error, minimum_interval_seconds, retry_policy_json, misfire_policy, acknowledged_risks_json, iteration_count, created_at, updated_at) values ('${aid}', '${projectId}', NULL, 'Manual Test', 'hello', '{\"type\":\"manual\"}', 1, NULL, '{\"provider\":\"omp\",\"model\":\"openai/gpt-5\"}', '{}', 'worktree', 'autonomous', 'reuse', 'standalone', NULL, NULL, 0, 60, '{}', 'fire-if-due', '{}', 0, '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z')`,
  );
  const r = await call("automation.runNow", { automationId: aid });
  assert(r && typeof r === "object", "runNow returned nothing");
  // tick at 3s picks up the queued run, turn ~2s, next tick reconciles.
  await sleep(12000);
  const run = sql(
    `select status from automation_runs where automation_id='${aid}' and trigger_type='manual' limit 1`,
  );
  assert(run.includes("completed"), `manual run did not complete: ${run}`);
});

// 22. heartbeat deferral: busy target thread defers instead of competing
await scenario("heartbeat automation defers while target thread is busy", async () => {
  const suffix = Date.now();
  const targetThread = "gate-hb-target-" + suffix;
  const aid = "gate-auto-hb-" + suffix;
  // Seed a target thread with a fake active session so the deferral check
  // sees it busy without needing a real slow turn.
  sql(
    `insert into projection_threads (thread_id, project_id, title, created_at, updated_at, runtime_mode, interaction_mode) values ('${targetThread}', '${projectId}', 'HB Target', '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z', 'worktree', 'autonomous')`,
  );
  sql(
    `insert into projection_thread_sessions (thread_id, status, provider_name, active_turn_id, updated_at) values ('${targetThread}', 'running', 'omp', 'fake-busy-turn', '2026-01-01T00:00:00.000Z')`,
  );
  sql(
    `insert into automation_definitions (automation_id, project_id, source_thread_id, name, prompt, schedule_json, enabled, next_run_at, model_selection_json, provider_options_json, runtime_mode, interaction_mode, worktree_mode, mode, target_thread_id, max_iterations, stop_on_error, minimum_interval_seconds, retry_policy_json, misfire_policy, acknowledged_risks_json, iteration_count, created_at, updated_at) values ('${aid}', '${projectId}', NULL, 'HB Test', 'hello', '{\"type\":\"interval\",\"everySeconds\":3600}', 1, '2020-01-01T00:00:00.000Z', '{\"provider\":\"omp\",\"model\":\"openai/gpt-5\"}', '{}', 'worktree', 'autonomous', 'reuse', 'heartbeat', '${targetThread}', NULL, 0, 60, '{}', 'fire-if-due', '{}', 0, '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z')`,
  );
  // First tick fires the occurrence; the busy target must defer it.
  await sleep(7000);
  const deferred = sql(
    `select status, deferred_until IS NOT NULL from automation_runs where automation_id='${aid}' limit 1`,
  );
  assert(deferred.includes("deferred"), `run was not deferred while target busy: ${deferred}`);
  // Free the target: the next deferred pass must dispatch and complete it.
  sql(
    `update projection_thread_sessions set active_turn_id = NULL, status = 'idle' where thread_id = '${targetThread}'`,
  );
  sql(
    `update automation_runs set deferred_until = '2020-01-01T00:00:00.000Z' where automation_id = '${aid}'`,
  );
  await sleep(12000);
  const after = sql(`select status from automation_runs where automation_id='${aid}' limit 1`);
  assert(after.includes("completed"), `deferred run did not dispatch after target freed: ${after}`);
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
