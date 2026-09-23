# Synara

A native Rust and GPUI workspace for external coding agents and direct model
conversations. One generic ACP adapter connects installed agents to durable tasks,
permissions, filesystem services and terminals. Agent runtimes remain external
processes. A separate direct-inference runtime supports reviewed HTTP providers
without launching an ACP agent.

This branch is a development build, not a production release. Linux compilation,
fixture-agent integration, native X11 interaction and a controlled loopback SSH
server have been exercised. macOS, Windows and a complete remote-workspace
workflow have not yet been validated.

## Delivery roadmap

[ROADMAP.md](ROADMAP.md) tracks the complete product scope, delivery order,
implementation ownership and acceptance gates. It distinguishes published
functionality from unverified local work and remaining systems. Completion is
recorded with candidate-specific evidence, not inferred from a percentage.

## Run

Install the pinned Rust toolchain and the native libraries for your platform.
On Ubuntu 24.04, the development packages are:

```sh
sudo apt-get install clang cmake pkg-config libasound2-dev libxcb1-dev \
  libx11-xcb-dev libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev \
  libvulkan-dev libegl1-mesa-dev libfontconfig1-dev libfreetype-dev \
  libssl-dev libclang-dev libzstd-dev libx11-dev libxrandr-dev \
  libxinerama-dev libxcursor-dev libwebkit2gtk-4.1-dev libgtk-3-dev
cargo run --locked -p synara-app -- --workspace /absolute/path/to/project
```

Use `--data-dir /absolute/path` for an isolated data directory and
`--agents /path/to/profiles.json` to import agent launch profiles. Opening a
workspace does not launch an agent or execute a prompt automatically.

The composer supports [voice drafts](docs/ui/voice-input.md) when a microphone
and ChatGPT-authenticated Codex session are available. The separate
[headless workspace preview](docs/ui/headless-server.md) can run without GPUI;
its browser view reads recent task messages and is local to the machine.

## Agents

Install and authenticate an ACP-compatible agent using its vendor's instructions.
OpenCode (`opencode acp`) and Gemini CLI (`gemini --acp`) launch presets are
included. These presets are not claims that a vendor service has been tested.
Custom profiles use the same connection implementation, without source changes:

```json
[
  {
    "id": "my-agent",
    "name": "My agent",
    "command": "/absolute/path/to/agent",
    "args": ["acp"],
    "inherit_env": ["MY_AGENT_TOKEN"]
  }
]
```

`inherit_env` contains variable names only. Values are resolved from Synara's
launch environment and are not written to the profiles database. Do not place
secrets in command arguments. Profiles are editable in Settings. Models, modes,
authentication methods and configuration are discovered from the agent.

## Direct models and conversation intake

[Direct models](docs/ui/direct-models.md) provides reviewed provider/model Settings,
endpoint-bound OS credentials, streaming, Stop and durable text conversations.
OpenAI-compatible Chat Completions, Anthropic Messages and Google Generative
Language are shared transport families, not a claim of 75 verified providers.
Structured output is validated locally against the documented bounded subset.
Native multimodal context, approved direct tool execution and broader auth remain open.

[Project Import](docs/ui/project-import.md) reviews local Codex/Claude text history
before importing it into an unsent standalone chat, without changing source files.
[Continue with](docs/ui/provider-handoff.md) creates an explicitly reviewed unsent
related ACP/direct conversation. It preserves the original session and working
folder, and never claims that provider sessions, approvals or secrets transferred.

## Agent Registry

The Agents tab browses the official ACP Registry. Refresh retrieves the index over
HTTPS. Review shows the publisher origin, version, platform, arguments, public
environment defaults and license link before approval. Merely browsing or adding
an agent never starts it. Select the added agent for a task to connect.

Binary installations require a publisher-provided SHA-256 checksum. The installer
verifies the download before extracting into a private staging directory, rejects
traversal, links, device files and case-colliding entries, and limits download and
expanded sizes. ZIP, gzip/bzip2 TAR and raw binaries are supported. Executable and
receipt changes are checked before a new process starts. A missing checksum is a
visible unsupported distribution, not implicit approval.

npm and uv entries create an explicitly approved, version-pinned launcher. Their
package manager downloads code on first connection and owns dependency integrity
and its cache. Synara does not claim to verify those packages as binary archives.
Managed agents start outside the project directory so a repository's local package
configuration cannot silently replace the approved launcher. ACP session paths
still identify the selected workspace. Registry installations are local. Remote
workspaces require a custom profile for an agent installed on the SSH host.

New versions are shown separately and never installed automatically. An update
keeps the previous installation until explicit removal. Removal refuses to strand
a task assigned to that installation, and does not delete conversations, vendor
credentials or npm/uv caches. The catalog is cached for offline browsing. Failed
refreshes preserve the previous valid cache.

An external agent still runs as your operating-system account. Callback path
containment is not an operating-system sandbox. Only approve publishers you trust.

## Desktop workflow

Create or open a workspace, create a task, choose an agent and send a prompt.
Conversation events are persisted before reaching the interface. The composer
supports platform text input, IME composition, selection, clipboard and undo/redo.
Enter sends, Shift+Enter inserts a line, and Stop cancels the active prompt.
Tool permissions and structured questions require explicit user responses.
Ctrl+1 through Ctrl+7 switch between Conversation, Files, Changes, Terminal,
Inspector, Settings and Agents. The platform command modifier is also recognized.

Files provides a UTF-8 editor with dirty state, guarded saves and external-change
conflict detection. Ctrl+S saves. UTF-8 BOMs and existing line endings are
preserved. The interactive editor currently limits files to 1 MiB. Symlinks,
binary content and oversized files produce visible errors. Closing with unsaved
file changes offers Keep working, Discard and close, or Save and close. A failed
or conflicting save leaves the editor open. An outstanding save must finish before
close can proceed. The confirmation does not protect against forced OS termination.

Changes shows system Git status and staged/unstaged diffs with literal-path
staging. Local commits use the configured Git identity, with hooks and signing
disabled for this action. Terminal provides a native PTY shell, command input,
interrupt/stop controls and retained output. It is an initial terminal surface,
not yet a full terminal-emulator interface.

Inspector shows bounded, redacted ACP traffic metadata and connection
capabilities. Restart affects tasks sharing the same agent process and directory.
Starting a new agent session preserves the local transcript. Failed restoration
is visible rather than silently discarding saved history.

## Pull Requests, Automations and Browser

The native command palette opens all three panes. [Pull Requests](docs/ui/pull-requests.md)
uses explicitly selected GitHub repositories and confirms remote writes.
[Automations](docs/ui/automations.md) stores definitions and run history in SQLite,
and scheduling starts disarmed after every restart. Saving is never permission to run.
[Browser](docs/ui/native-browser.md) uses the existing browser owner with Linux/X11
WebKitGTK rendering and separately approved task-isolated agent operations.
For the embedded browser, run `GPUI_PLATFORM=x11 cargo run --locked -p synara-app`
in an X11/XWayland session. Native Wayland, Windows and macOS browser adapters
remain unsupported. [Verification](docs/verification/pr-automations-browser.md)
distinguishes source, fixture and native acceptance from remaining work.

## SSH transport boundary

The Rust runtime provides `SshHost` for user-configured hosts and `PinnedSshHost`
for explicitly selected identity and known-hosts files. Both use the existing
`ExecutionHost` interface, so ACP does not need a separate remote backend.
Background agent transports disable agent/X11/port forwarding, local commands,
connection multiplexing, pseudo-terminal allocation and background detachment.
`SshHost` still reads the user's trusted SSH configuration for host resolution and
authentication. It is not safe to point that configuration at repository content.

`PinnedSshHost` ignores user/system SSH configuration and alternate host-key
sources. It requires existing absolute regular files, rejects configuration
expansion characters and revalidates file metadata before each launch. On Unix,
private identities must exclude group/other access and trust files must not be
group/world writable. Only the selected identity is offered, without ssh-agent.
Unknown or changed hosts must be enrolled through a separately trusted process.
Synara never silently accepts a key to make a connection work.

Connection files and their parent directories remain user-trusted local inputs.
Metadata revalidation is not an atomic filesystem sandbox against another process
with the same account replacing files between validation and OpenSSH opening them.
The pinned API selects a trust store, not immutable key bytes inside the process.

This is a runtime API foundation, not a completed remote workspace UI. Remote
filesystem, Git, PTY, forwarding and remote-process cleanup need their own
implementation and verification. A local SSH process exiting does not establish
that every remote descendant has exited. Encrypted-key interaction and platform
credential selection are tracked in the roadmap.

## Architecture and safety

- `synara-core`: protocol-independent domain, conversation reducer and text model.
- `synara-agent`: backend contracts, shared connections and interaction broker.
- `synara-acp`: ACP transport, negotiation and normalized protocol adapter.
- `synara-model`: normalized direct inference, provider registry, shared HTTP/SSE families and capability/output validation, separate from ACP.
- `synara-runtime`: process ownership, contained filesystem, PTY, execution hosts and OS secret references.
- `synara-workspace`: SQLite, durable event delivery, controller, profiles and tools.
- `synara-app`: native GPUI shell and input surfaces.
- `synara-registry`: validated metadata, approved launchers and bounded installations.

Repository content, agent output and tool requests are untrusted. Filesystem
callbacks enforce containment and symlink rules. Permissions are not silently
approved. Agent state and Synara's persistent transcript remain separate.

Task title, activity and recency are projected in the same SQLite transaction as
each conversation event, before observers receive it. Overlapping permissions keep
a task waiting until the last response. Schema version 3 backfills older catalogs
from ordered events without loading full transcripts into memory. A failed
migration rolls back rather than publishing a partial catalog. Back up the data
directory before testing a development build. Older builds reject newer schemas.

## Verification

```sh
cargo fmt --check
cargo check --locked --workspace
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace
python3 scripts/audit_workspace.py
python3 scripts/test_apply_source.py
python3 scripts/check_roadmap.py --self-test
python3 scripts/check_roadmap.py
```

Tests do not require vendor credentials. The fixture executable accepts only an
explicit `--integration-fixture` invocation and is for testing the generic host.

Linux CI also builds the application and exercises actual mouse and keyboard input
on an isolated Xvfb display. The smoke checks cover streaming, two fixture-agent
profiles, tool output, permission denial, approved filesystem writes, rejected
path escapes, offline registry approval, editor close/cancel/save/conflict handling
and history restoration without agent autostart. Screenshots and results are saved
as the `native-desktop-smoke` workflow artifact. This is fixture proof, not a live
vendor-service test or a claim of complete visual/accessibility coverage.

To run the desktop smoke locally on Linux, install Xvfb, libXtst and Pillow, then:

```sh
cargo build --locked -p synara-app --bin synara-app -p synara-acp --bin synara-acp-fixture
python3 scripts/native_smoke.py --binary target/debug/synara-app \
  --fixture target/debug/synara-acp-fixture --output /tmp/synara-native-smoke
```

The output directory must not already exist. The test never controls an existing
desktop and never connects a vendor agent. The fixture registry launcher is only
approved, never executed.

For controlled SSH testing on Linux, install OpenSSH client/server and run as an
ordinary user, not root:

```sh
python3 scripts/ssh_smoke.py
```

The harness generates temporary keys, binds a disposable server only to loopback,
and explicitly runs the `ssh_live` integration tests. It does not edit the user's
SSH configuration or existing trust/authorization files. The ordinary test suite
lists these tests as ignored because they require this server fixture. The SSH CI
job must run them explicitly. This tests real SSH transport with fixture agents on
the same machine, not vendor credentials or a complete remote desktop workflow.

Complete terminal emulation, remote workspace UI, cross-platform Browser/Device
acceptance, updater, native credential integration and broader platform/performance
hardening remain under development. Linux/X11 Browser hosting and the scoped
ADB/simctl Device implementation are present on the development branch. Live vendor-agent
validation is separate from fixture tests.

Project licensing has not yet been selected. Package publication is disabled.
Third-party dependencies retain their own licenses.

## Native Plugins, Skills and MCP

Settings includes ownership-aware Plugins inventory, a reviewed local Markdown
skill library with explicit unsent draft insertion, and task/agent-scoped HTTP MCP
configuration with explicit protocol discovery tests. External provider extensions
remain outside Synara's control. Credentials use references, and the current
desktop secret-store adapter reports unavailable rather than persisting plaintext.
See [behavior and limitations](docs/integrations.md) and the
[verification receipt](docs/verification/plugins-skills-mcp.md).
