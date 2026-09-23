# Native headless workspace preview

`synara-server` starts the native workspace service and recovery without GPUI.
It serves an authenticated local browser for adding existing folders, creating
unsent tasks, editing their drafts, and reading task messages on loopback. This
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
workspace-root or task-directory fields, or pending interactions. Visible
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

Only loopback bind addresses are accepted. Remote access, TLS termination,
deployment/update packaging, and the full web workspace remain outside this
server slice. The responses are bounded and omit workspace filesystem paths.
Focused tests cover authentication, Host/Origin checks, readiness, lock
contention/release, shutdown, private default storage, bounded task reads,
and authenticated task creation and draft updates.
A deployed remote journey was not exercised.
