# Synara

A native coding-agent workspace written in Rust with a GPUI desktop interface.

The application separates durable workspace and conversation state from external
agent connections. ACP-compatible agents share one protocol adapter. Agent runtimes
are external processes, not application components.

## Development

Use the pinned Rust toolchain. This workspace is under active implementation.

```sh
cargo fmt --check
cargo check --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Project licensing has not yet been selected. Package publication is disabled.
Third-party dependencies retain their own licenses.
