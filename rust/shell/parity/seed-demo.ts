// Seed a reference home with a small, made-up demo project and two chat threads,
// so the Electron/web reference and the native shell can show the same transcript.
//
//   bun rust/shell/parity/seed-demo.ts <home>/dev/state.sqlite <workspace-dir>
//
// Run it while the server for that home is stopped. It only appends
// orchestration events; the server projects them on its next start.
// Re-running is a no-op once the demo threads exist.
import { Database } from "bun:sqlite";
import { randomUUID } from "node:crypto";

const [dbPath, workspaceRoot] = process.argv.slice(2);
if (!dbPath || !workspaceRoot) {
  console.error("usage: bun seed-demo.ts <state.sqlite> <workspace-dir>");
  process.exit(2);
}

export const DEMO_PROJECT_ID = "22222222-bbbb-4bbb-8bbb-000000000001";
export const DEMO_THREAD_IDS = [
  "11111111-aaaa-4aaa-8aaa-000000000001",
  "11111111-aaaa-4aaa-8aaa-000000000002",
] as const;

const model = { provider: "codex", instanceId: "codex", model: "gpt-6-astra" };
const at = (min: number, sec = 0) =>
  new Date(Date.UTC(2026, 10, 25, 19, min, sec)).toISOString();

const ASSISTANT_REPLY = [
  "Sure. Here is a short outline for a small notes app about gardening. The idea is to keep each note tiny, link it to a plant, and make search fast so you can find what you planted last spring.",
  "",
  "- Plants: name, location and planting date",
  "- Notes: short text with an optional photo",
  "- Reminders: watering and pruning dates",
  "",
  "Store each note as a `Note` value and keep the plant list in a `Vec<Plant>`. A minimal sketch:",
  "",
  "```rust",
  "struct Plant {",
  "    name: String,",
  "    planted: u32,",
  "}",
  "",
  "fn main() {",
  '    let p = Plant { name: "Tomato".to_string(), planted: 2026 };',
  '    println!("{} was planted in {}", p.name, p.planted);',
  "}",
  "```",
  "",
  "Let me know if you want the reminder part sketched next.",
].join("\n");

type Ev = { kind: "project" | "thread"; stream: string; type: string; payload: object };

const message = (threadId: string, role: string, text: string, ts: string): Ev => ({
  kind: "thread",
  stream: threadId,
  type: "thread.message-sent",
  payload: {
    threadId,
    messageId: randomUUID(),
    role,
    text,
    turnId: null,
    streaming: false,
    source: "native",
    createdAt: ts,
    updatedAt: ts,
  },
});

const thread = (threadId: string, title: string, ts: string): Ev => ({
  kind: "thread",
  stream: threadId,
  type: "thread.created",
  payload: {
    threadId,
    projectId: DEMO_PROJECT_ID,
    title,
    modelSelection: model,
    runtimeMode: "full-access",
    interactionMode: "default",
    branch: null,
    worktreePath: null,
    createdAt: ts,
    updatedAt: ts,
  },
});

const [t1, t2] = DEMO_THREAD_IDS;
const events: Ev[] = [
  {
    kind: "project",
    stream: DEMO_PROJECT_ID,
    type: "project.created",
    payload: {
      projectId: DEMO_PROJECT_ID,
      kind: "project",
      title: "Test Project",
      workspaceRoot,
      defaultModelSelection: model,
      scripts: [],
      isPinned: false,
      spaceId: null,
      additionalFolders: [],
      createdAt: at(0),
      updatedAt: at(0),
    },
  },
  thread(t1, "Plan the garden notes", at(0)),
  message(t1, "user", "Can you outline a small plan for my garden notes app?", at(0, 5)),
  message(t1, "assistant", ASSISTANT_REPLY, at(0, 20)),
  thread(
    t2,
    "A very long thread title that keeps going to test how the tab strip and the header handle overflow when there is far too much text to fit",
    at(5),
  ),
  message(t2, "user", "Short question for the long title thread.", at(5, 5)),
  message(t2, "assistant", "This is a brief answer so the thread is not empty.", at(5, 15)),
];

const db = new Database(dbPath);
const exists = db
  .query("select count(*) as n from orchestration_events where stream_id = ?")
  .get(t1) as { n: number };
if (exists.n > 0) {
  console.log("demo threads already present; nothing to do");
  process.exit(0);
}

const insert = db.query(
  `insert into orchestration_events
     (event_id, aggregate_kind, stream_id, stream_version, event_type, occurred_at,
      command_id, causation_event_id, correlation_id, actor_kind, payload_json, metadata_json)
   values (?, ?, ?, ?, ?, ?, ?, null, ?, 'client', ?, '{"persistedEventSchemaVersion":1}')`,
);
const versions = new Map<string, number>();
db.transaction(() => {
  for (const ev of events) {
    const version = versions.get(ev.stream) ?? 0;
    versions.set(ev.stream, version + 1);
    const commandId = randomUUID();
    const occurred = (ev.payload as { createdAt: string }).createdAt;
    insert.run(
      randomUUID(),
      ev.kind,
      ev.stream,
      version,
      ev.type,
      occurred,
      commandId,
      commandId,
      JSON.stringify(ev.payload),
    );
  }
})();
console.log(`seeded ${events.length} events: project ${DEMO_PROJECT_ID}, threads ${DEMO_THREAD_IDS.join(", ")}`);
