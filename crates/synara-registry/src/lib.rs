//! Registry metadata and explicitly approved installations for the generic agent host.
mod download;
mod install;
mod model;
mod paths;

pub use download::{Downloader, HttpsDownloader};
pub use install::{InstalledAgent, RegistryReference, RegistryStore};
pub use model::{AgentEntry, Distribution, InstallPlan, PackageTarget, Platform, Registry};

pub const REGISTRY_URL: &str =
    "https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json";
pub const INDEX_LIMIT: u64 = 8 * 1024 * 1024;
pub const DOWNLOAD_LIMIT: u64 = 512 * 1024 * 1024;
pub const EXPANDED_LIMIT: u64 = 1024 * 1024 * 1024;
pub const FILE_LIMIT: u64 = 256 * 1024 * 1024;
pub const ENTRY_LIMIT: usize = 20_000;

pub type Result<T> = std::result::Result<T, RegistryError>;
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("invalid registry metadata: {0}")]
    Invalid(String),
    #[error("installation unavailable: {0}")]
    Unsupported(String),
    #[error("registry resource limit exceeded")]
    Limit,
    #[error("download checksum does not match the approved manifest")]
    Checksum,
    #[error("installed agent has changed since it was approved")]
    Changed,
    #[error("another installation operation owns this agent directory")]
    Busy,
    #[error("registry HTTPS request failed: {0}")]
    Network(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("registry JSON could not be decoded: {0}")]
    Json(#[from] serde_json::Error),
}
