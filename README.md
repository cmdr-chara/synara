# Synara

Synara is a native desktop workspace for coding agents, built with Rust and GPUI.
It brings conversations, project files, Git, a terminal, a browser, and
automations into one app. Agents connect through ACP and run as separate
processes. You can also chat with supported models directly.

The GPUI app lives on `main`. The earlier Electron app remains on
`main-electron`. Synara is in active development: Linux/X11 has the most
testing, while macOS, Windows, and remote workspace support need more validation.

## Get started

Install the pinned Rust toolchain from `rust-toolchain.toml` and your platform's
native development libraries. On Ubuntu 24.04:

```sh
sudo apt-get install clang cmake pkg-config libasound2-dev libxcb1-dev \
  libx11-xcb-dev libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev \
  libvulkan-dev libegl1-mesa-dev libfontconfig1-dev libfreetype-dev \
  libssl-dev libclang-dev libzstd-dev libx11-dev libxrandr-dev \
  libxinerama-dev libxcursor-dev libwebkit2gtk-4.1-dev libgtk-3-dev
cargo run --locked -p synara-app -- --workspace /absolute/path/to/project
```

Configure an ACP agent in Settings or install one from the Agents tab. Authenticate
with its provider, create a task, choose the agent, and send a prompt. Opening a
workspace does not start an agent or send a prompt.

Use `--data-dir /absolute/path` to keep data in a separate directory,
`--agents /path/to/profiles.json` to import agent profiles, or `--help` for the
full command line. For the embedded browser on XWayland, prefix the run command
with `GPUI_PLATFORM=x11`.

## Learn more

- [Roadmap](ROADMAP.md): what works and what remains.
- [Direct models](docs/ui/direct-models.md): provider setup and capabilities.
- [Browser](docs/ui/native-browser.md): Linux/X11 setup and browser permissions.
- [Plugins, skills, and MCP](docs/integrations.md): integration behavior and limits.

## Develop

```sh
cargo fmt --check
cargo test --locked --workspace
```

Synara is available under the [MIT License](LICENSE).
