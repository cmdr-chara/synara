# ACP inspector backend checkpoint

Roadmap owner: N3/N4.

The protocol-independent `AgentConnection` interface now exposes a bounded
`InspectorSnapshot` and JSON export contract. A snapshot contains:

- connection ID and state;
- bounded agent name/title/version metadata;
- normalized capabilities and authentication-method count;
- local-versus-remote host classification, not the raw host label;
- owned process ID when the backend exposes one;
- bounded trace entries with sequence/timestamp, direction, kind, hashed request
  ID, method and structural payload shape;
- a truncation flag and explicit privacy notice.

ACP supplies its owned child process ID through this contract. Existing trace
storage remains capped at 512 entries and contains no raw protocol payload or
stderr text. The generic snapshot defensively retains only the newest 512
entries and bounds individual inspector fields. JSON export has a 1 MiB cap.

Raw `ConnectionInfo.error`, raw SSH host labels and authentication descriptions
are intentionally not copied into inspector exports. The snapshot records only
whether an error exists. This prevents diagnostic copy/export from becoming a
second error/credential disclosure path.

`AgentConnection::clear_trace` remains the explicit clear operation and
`ConnectionManager::restart` remains the owned restart path. Restart does not
reuse a failed transport. Product UI can bind those operations without adding
protocol types to the app layer.

The export privacy notice states that this metadata path excludes protocol
payload/stderr content and that separate transcript/file-content exports may
contain sensitive user data. No automatic upload or telemetry is introduced.

Acceptance is backend-only. A polished ACP inspector UI remains outside this
checkpoint.
