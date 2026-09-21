# Native Plugins, Skills and MCP management

The native Settings pages use the existing workspace service, SQLite preferences,
agent profiles, capability negotiation and credential-store boundary. There is no
provider-specific extension runtime. The implementation and acceptance evidence are
tracked separately in [the session receipt](verification/plugins-skills-mcp.md).

## Ownership and discovery

Settings > Plugins & integrations lists the built-in Synara management surfaces
and the actual locally managed inventory. Search and enabled/disabled filters are
literal, not remote marketplace requests. Built-in functionality is labelled
available, not downloaded or installed on behalf of an external provider.

An external agent may own extensions, skills and MCP servers. The current generic
agent contract does not provide a plugin catalog, installed-extension enumeration
or lifecycle commands. Their state is unknown, not "not installed". Synara does
not fabricate installation, enable, configuration or revoke controls. The page
links to agent profiles and separately shows the selected connection's negotiated
HTTP/SSE MCP configuration flags. Those flags are not plugin installation or
server-connection evidence. No capability is inferred from an agent name.

## Skills: reviewed instruction documents

Settings > Skills is a Synara-owned Markdown document library. Select a local
`.md` file using the native file picker, or enter its absolute path and choose
Review path. The latter uses the same validation and approval path, and remains
usable when a desktop file portal is unavailable. Review shows the complete
literal document, original path and SHA-256. Approval rereads the source and
refuses a changed document. Only explicit approval persists an installed document.

The loader accepts regular UTF-8 files up to 64 KiB, rejects symlinks in the file
or its ancestry, empty content and unsupported control characters. The optional
frontmatter parser accepts simple literal title/name, description and version
lines. It is not a general YAML parser. The document, source hash and declared
version are retained. Source location is provenance, not verified publisher
identity or compatibility evidence. Documents must not contain secrets.

An import or reviewed update is disabled. A separate Enable for drafts action
permits Insert into draft for the selected task. Insertion rereads the library,
checks revision and enabled state, preserves the existing draft, and refuses a
changed conversation selection or ongoing IME composition. Nothing is sent,
launched or injected into future conversations automatically. This is ordinary
visible prompt text, not a claim of native provider skill support.

Review update uses the same file review flow, retains the document identity and
up to eight previous origin receipts, and disables the updated document. Remove
requires confirmation, removes only Synara's stored document, and leaves the
original file and previously inserted drafts unchanged. Search covers title,
description and origin. There is no authoritative remote skill catalog wired
through the present generic contract, so the UI says so rather than inventing one.

No archives, scripts, dependencies, executable permissions or provider directories
are installed. Instructions may contain code as literal text. Enabling a document
is not permission to execute that code. Bundled assets/scripts and marketplace or
provider-native installation are intentionally unsupported.

## MCP configuration and scope

Settings > MCP connections adds a connection for the current task and its exact
agent profile. Existing Project/Hub/Studio task identities remain authoritative.
There is deliberately no all-tasks, all-agents or implicit Hub-wide sharing option.
Repository files are not scanned, imported, trusted or executed.

A record contains a name, HTTP endpoint, task ID, agent profile ID, enabled state
and optional bearer credential reference. Adding or editing always saves disabled.
Enable is a separate action. Edit cannot silently retarget ownership: create a
separately reviewed connection for a different task or agent instead. Orphaned
records can be disabled or removed without recreating the deleted task.

Endpoints require HTTPS, except HTTP on literal loopback addresses for local
servers. URL credentials, query strings, fragments, whitespace and controls are
rejected. Redirects and ambient proxies are disabled for direct probes so a token
cannot be forwarded to a different origin. Localhost DNS is not literal loopback.
Desktop-managed configuration currently supports local workspaces only. Desktop
reachability cannot prove that an SSH-hosted agent can reach the same endpoint.

Before a new or restored agent session is created, the controller selects only
enabled connections matching that task and profile. It requires negotiated HTTP
MCP configuration support and resolves credentials at this boundary. It sends
existing generic `ContextServer::Http` values through the existing ACP adapter.
This does not add a competing MCP tool-execution runtime.

Configuration changes serialize against session creation. Active prompts refuse
changes. Existing sessions must close successfully before configuration is
changed and saved session references are invalidated. If an agent cannot confirm
closure, the UI requires an explicit Disconnect agent confirmation before retry.
That confirmation discloses the shared process boundary, refuses while any task
on that process is active, and never restarts tasks. Removing a local record does
not revoke an external token, delete a possibly shared credential-store entry or
rewrite an agent-owned configuration file.

Task-scoped delivery is a logical consent boundary, not an OS security sandbox.
An external agent process can be shared across tasks and runs with its OS account.
Synara cannot prove how that provider internally isolates or retains received
configuration. Trust and provider-side credential revocation remain necessary.

## Credentials

Only `SecretReference { service, account }` is persisted. The UI never offers a
plaintext token field or puts tokens in general Settings, URLs or source files.
Resolution uses the existing injected `SecretStore`. Locked, unavailable, missing
or malformed credentials fail before a network test or context delivery. Header
values are marked sensitive and raw server/transport errors are not echoed into
the UI. Generic `ContextServer` debug output redacts the entire configuration.

The current desktop bootstrap still uses `UnavailableSecretStore`. A production
OS credential-store adapter is a separate unfinished Settings/platform boundary.
Consequently unauthenticated local/HTTPS probes are usable, while authenticated
connections fail explicitly until the application is wired to a supported store.
Tests inject isolated synthetic stores, not production credentials. There is no
plaintext fallback, implicit environment lookup or simulated successful login.

## Explicit test and discovery

Test and discover performs a real, bounded HTTP exchange from Synara, without
launching an agent, enabling a record or invoking tools. It first uses MCP
2026-07-28 `server/discover`, with the protocol metadata and routing headers. Only
compatible discovery responses count as successful. Eligible older-server HTTP
responses fall back to the 2025-11-25 initialization handshake, accepting supported
2025 protocol versions and emitting `notifications/initialized`. Temporary legacy
session headers are bounded and a DELETE cleanup is attempted after completion.
Unconfirmed cleanup is shown rather than concealed.

Only negotiated tool support leads to `tools/list`. Requests and response bodies
are bounded, JSON-RPC IDs and response envelopes are checked, duplicate tools and
repeated cursors are rejected, and discovery is limited to four pages/256 tools.
The reader accepts bounded JSON responses or SSE response events. Server-initiated
requests are rejected. No sampling, elicitation or `tools/call` is performed.
Server identity and tool descriptions are untrusted self-reported metadata, not
verified authorship or fully validated executable tool contracts.

A successful result displays protocol, duration, advertised capabilities and tool
metadata. It is explicitly an application-side probe, not continuous connection
status or proof of an agent's reachability. Results are ephemeral and invalidated
when configuration changes or the page reloads. HTTP success alone is never MCP
success. Authentication failures and redirects are not retried as legacy probes.

OAuth/pairing, process transports, a separate legacy SSE transport, streaming tool
execution, continuous sessions and provider-side revoke are not implemented by
this management probe. Agent-managed MCP is displayed as outside Synara's control.

## Storage, recovery and limits

The versioned `integrations` preference participates in the existing database
backup key validator. Writes are transactional with a revision check, bounded to
4 MiB, 48 documents and 32 MCP records. Unsupported/corrupt catalogs are preserved
and reported, never reset to empty before a write. Removing a connection also
invalidates the relevant saved session in the same transaction. Backups contain
skill instruction text and credential references, never resolved credential bytes.

The UI uses existing native text inputs, Settings navigation, compact rows,
separators and progressive detail. No competitor code, assets or visual tokens
were used. Keyboard/file-picker/platform and real-provider acceptance must be
reported from execution, not source inspection.

## Protocol references

- https://modelcontextprotocol.io/specification/2026-07-28/server/discover
- https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/streamable-http
- https://modelcontextprotocol.io/specification/2026-07-28/basic/versioning
- https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle
