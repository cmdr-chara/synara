# Synara

A native Rust and GPUI workspace for external coding agents. One generic ACP
adapter connects installed agents to durable tasks, conversations, permissions,
filesystem services and terminals. Agent runtimes remain external processes.

This branch is a development build, not a production release. Linux compilation,
fixture-agent integration and a native X11 window have been exercised. macOS,
Windows and live SSH behavior have not yet been validated.

## Run

Install the pinned Rust toolchain and the native libraries for your platform.
On Ubuntu 24.04, the development packages are:

```sh
sudo apt-get install clang cmake pkg-config libasound2-dev libxcb1-dev \
  libx11-xcb-dev libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev \
  libvulkan-dev libegl1-mesa-dev libfontconfig1-dev libfreetype-dev \
  libssl-dev libclang-dev libzstd-dev libx11-dev libxrandr-dev \
  libxinerama-dev libxcursor-dev
cargo run --locked -p synara-app -- --workspace /absolute/path/to/project
```

Use `--data-dir /absolute/path` for an isolated data directory and
`--agents /path/to/profiles.json` to import agent launch profiles. Opening a
workspace does not launch an agent or execute a prompt automatically.

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

## Desktop workflow

Create or open a workspace, create a task, choose an agent and send a prompt.
Conversation events are persisted before reaching the interface. The composer
supports platform text input, IME composition, selection, clipboard and undo/redo.
Enter sends, Shift+Enter inserts a line, and Stop cancels the active prompt.
Tool permissions and structured questions require explicit user responses.

Files provides a UTF-8 editor with dirty state, guarded saves and external-change
conflict detection. Ctrl+S saves. UTF-8 BOMs and existing line endings are
preserved. The interactive editor currently limits files to 1 MiB. Symlinks,
binary content and oversized files produce visible errors.

Changes shows system Git status and staged/unstaged diffs with literal-path
staging. Local commits use the configured Git identity, with hooks and signing
disabled for this action. Terminal provides a native PTY shell, command input,
interrupt/stop controls and retained output. It is an initial terminal surface,
not yet a full terminal-emulator interface.

Inspector shows bounded, redacted ACP traffic metadata and connection
capabilities. Restart affects tasks sharing the same agent process and directory.
Starting a new agent session preserves the local transcript. Failed restoration
is visible rather than silently discarding saved history.

## Architecture and safety

- `synara-core`: protocol-independent domain, conversation reducer and text model.
- `synara-agent`: backend contracts, shared connections and interaction broker.
- `synara-acp`: ACP transport, negotiation and normalized protocol adapter.
- `synara-runtime`: process ownership, contained filesystem, PTY and execution hosts.
- `synara-workspace`: SQLite, durable event delivery, controller, profiles and tools.
- `synara-app`: native GPUI shell and input surfaces.

Repository content, agent output and tool requests are untrusted. Filesystem
callbacks enforce containment and symlink rules. Permissions are not silently
approved. Agent state and Synara's persistent transcript remain separate.

## Verification

```sh
cargo fmt --check
cargo check --locked --workspace
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace
python3 scripts/audit_workspace.py
```

Tests do not require vendor credentials. The fixture executable accepts only an
explicit `--integration-fixture` invocation and is for testing the generic host.

Agent registry installation, complete terminal emulation, remote workspace UI,
browser/device hosting, updater, native credential integration and broader
platform/performance hardening remain under development. Live vendor-agent
validation is separate from fixture tests.

Project licensing has not yet been selected. Package publication is disabled.
Third-party dependencies retain their own licenses.
