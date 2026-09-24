# Native local headless workspace

`synara-server` starts the native workspace service and recovery without GPUI.
It serves an authenticated local browser for adding existing folders, creating
unsent tasks, editing drafts, explicitly running/stopping local ACP agents,
reviewing one-time task requests and reading messages on loopback. This
is an early headless surface, not the full upstream
web workspace.

```sh
SYNARA_SERVER_TOKEN='replace-with-a-private-32-character-or-longer-token' \
  cargo run -p synara-server -- --bind 127.0.0.1 --port 17341
```

Open `http://127.0.0.1:17341/` and enter the same token to add an existing
local folder as a project, create an unsent task, edit its draft, and browse
recent messages. The token stays in the page input and is sent only in
the Authorization header; the page never stores it in browser storage.
`GET /health` and `GET /ready` return 503 until workspace recovery completes;
`GET /api/catalog`, `GET /api/profiles`, `GET /api/tasks/<task-id>/draft`,
and `GET /api/tasks/<task-id>/thread` require
`Authorization: Bearer <token>`. The catalog returns at most 32 workspaces,
projects and tasks per list. The task endpoint returns up to 24 messages per
page and accepts `?before=<message-index>` to browse earlier pages. Each message
is limited to 1,800 characters; large Unicode pages may contain fewer messages
to keep the response below 64 KiB. A `next_before` index identifies the
previous page when one exists.
`POST /api/workspaces` accepts `{"root":"/existing/absolute/folder"}`.
`POST /api/projects/<project-id>/tasks` accepts
`{"title":"...","agent_id":"...","draft":"..."}` and creates a ready but
unsent task with its draft atomically. `POST /api/tasks/<task-id>/draft` accepts
`{"text":"..."}` to save an unsent draft. These JSON writes require the same
token; any supplied Origin must match the loopback Host. They are limited to
64 KiB bodies and 16 KiB draft
text, and return generic errors without filesystem paths. They do not start
an agent or approve an interaction. The browser warns before discarding unsaved
draft edits when switching tasks.
It excludes reasoning-role messages and does not add tool output, attachments,
workspace-root or task-directory fields to the transcript endpoint. Visible
message text itself may contain paths or
other private content. The server also accepts `--token-file PATH` in place of
the environment variable. On Unix the
file must be inaccessible to group and other users and must not be a symlink.
Neither token source may be configured alongside the other.

The default database is a separate `headless-server.sqlite3` under Synara's
private user-data directory. `--db PATH` or `SYNARA_SERVER_DB` explicitly
selects another database. The desktop and server acquire the same process lock
before opening or recovering a database; starting against one already owned
by another process fails immediately. For an explicit path, the caller is
responsible for its parent-directory permissions.

## Explicit task execution and requests

`GET /api/tasks/<task-id>/run` reports the current web run. An explicit
`POST /api/tasks/<task-id>/run` with `{"text":"...","expected_draft":"..."}`
starts a local ACP prompt after draft, lifecycle and admission checks.
`POST /api/tasks/<task-id>/stop` with `{}` cancels its owned run. The browser
asks for confirmation, retains the submitted draft, preserves later typing,
and polls status and messages. Up to eight runs can be active, each with a
one-hour limit. Reload does not replay a prompt. This does not grant blanket
permission: the agent retains only its existing local authority and explicitly
answered requests. Use an already configured local agent. Direct-model and
remote tasks are not supported by this execution surface.

`GET /api/tasks/<task-id>/interactions` returns a bounded `items` list and `more`.
A maximum of two requests are shown at once, from a queue capped at 32, with
24 KiB per visible item and the existing 64 KiB HTTP response bound. These are
Session-scoped requests from the existing InteractionBroker, not a separate
provider backend. Unsupported connection/URL interactions, invalid request
shapes and overflow cancel rather than grant authority. Broker requests expire
after five minutes and also retire when the turn or response channel closes.

The browser displays agent-provided request text, with one-time allow/deny/cancel
choices, or text, finite number, Boolean, single-choice and multi-choice fields.
No tool output or full command/diff context is shown in this browser slice.
Deny requests that cannot be assessed from the displayed text. URLs are not
made into links. Persistent permission choices are never offered. Form values
are preserved during status polling but are not stored or restored after page
navigation/reload. Accepted answers are sent to the configured agent.

Responses use `POST /api/tasks/<task-id>/interactions` and the same bearer,
Host/Origin and JSON checks as all writes. Examples:

```json
{"action":"permission","id":"<receipt>","choice":"<offered-once-choice>"}
{"action":"permission","id":"<receipt>","choice":null}
{"action":"input","id":"<receipt>","response":{"action":"accept","values":{"count":2}}}
{"action":"input","id":"<receipt>","response":{"action":"decline"}}
{"action":"input","id":"<receipt>","response":{"action":"cancel"}}
```

Receipts are newly generated application identifiers, not provider session/request
IDs. A wrong task, expired request, duplicate answer or previous-server receipt
returns 409 without delivery. Invalid answers return 400 and leave the original
request pending for correction. All fields are checked against the original
schema before consuming its one-shot channel. Successful delivery to the broker
is not proof that the agent executed a tool or accepted the answer. Stop and
shutdown cancel pending requests, and restart does not restore approvals.

## Remaining deployment scope

Only loopback bind addresses are accepted. Remote access, TLS termination,
deployment/update packaging, and the full web workspace remain outside this
server slice. The responses are bounded and omit workspace filesystem paths.
Focused tests cover authentication, Host/Origin checks, readiness, lock
contention/release, shutdown, private default storage, bounded task reads,
authenticated task creation and draft updates, one-shot interaction replies,
queue limits, schema checks and browser draft ownership.
A deployed remote journey was not exercised.
