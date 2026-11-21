//! The Rust port of `packages/shared/src/remoteCapabilities.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

const REMOTE_UNAVAILABLE: &[&str] = &[
    "projects.listDevServers",
    "projects.runDevServer",
    "projects.stopDevServer",
    "server.connectMcpConnector",
    "server.getMcpConnectorSetup",
    "server.listMcpConnectorProposals",
    "server.resolveMcpConnectorProposal",
    "shell.openInEditor",
];

/// Port of `remoteMethodUnavailable` (`packages/shared/src/remoteCapabilities.ts:21`).
pub fn remote_method_unavailable(method: String) -> Result<bool, String> {
    Ok(REMOTE_UNAVAILABLE.contains(&method.as_str()))
}
