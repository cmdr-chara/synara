//! Offline-aware catalog loading. Failure must not turn a valid cache into an empty UI.
use crate::{Downloader, Registry, RegistryError, RegistryStore, Result};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CatalogSource {
    Fresh,
    Cache,
    Empty,
}

/// Fixed classifications are safe for UI notices. Do not include raw server bodies.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CatalogWarning {
    Network,
    Rejected,
    Busy,
    Storage,
}
impl CatalogWarning {
    pub fn message(self) -> &'static str {
        match self {
            Self::Network => "Catalog refresh failed. Previously cached agents remain available.",
            Self::Rejected => "The new catalog was rejected. The previous valid cache was kept.",
            Self::Busy => "Another registry operation is running. Showing the cached catalog.",
            Self::Storage => "Catalog storage failed. Showing the last readable cached catalog.",
        }
    }
}

#[derive(Clone, Debug)]
pub struct CatalogSnapshot {
    pub catalog: Option<Registry>,
    pub source: CatalogSource,
    pub warning: Option<CatalogWarning>,
}
impl RegistryStore {
    /// Network is attempted only when the caller explicitly supplies a downloader.
    /// A cache is parsed again before fallback. Corrupt caches are never accepted.
    pub fn load_catalog(&self, refresh: Option<&dyn Downloader>) -> Result<CatalogSnapshot> {
        let warning = if let Some(downloader) = refresh {
            match self.refresh(downloader) {
                Ok(catalog) => {
                    return Ok(CatalogSnapshot {
                        catalog: Some(catalog),
                        source: CatalogSource::Fresh,
                        warning: None,
                    });
                }
                Err(error) => Some(classify(&error)),
            }
        } else {
            None
        };
        let catalog = self.cached()?;
        let source = if catalog.is_some() {
            CatalogSource::Cache
        } else {
            CatalogSource::Empty
        };
        Ok(CatalogSnapshot {
            catalog,
            source,
            warning,
        })
    }
}

fn classify(error: &RegistryError) -> CatalogWarning {
    match error {
        RegistryError::Network(_) => CatalogWarning::Network,
        RegistryError::Busy => CatalogWarning::Busy,
        RegistryError::Io(_) => CatalogWarning::Storage,
        _ => CatalogWarning::Rejected,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, io::Write};

    struct Bytes(&'static [u8]);
    impl Downloader for Bytes {
        fn download(&self, _: &str, target: &mut dyn Write, limit: u64) -> Result<()> {
            if self.0.len() as u64 > limit {
                return Err(RegistryError::Limit);
            }
            target.write_all(self.0)?;
            Ok(())
        }
    }
    struct Offline;
    impl Downloader for Offline {
        fn download(&self, _: &str, _: &mut dyn Write, _: u64) -> Result<()> {
            Err(RegistryError::Network("private error canary".into()))
        }
    }
    const VALID: Bytes = Bytes(br#"{"version":"1.0.0","agents":[]}"#);

    #[test]
    fn empty_local_store_does_not_attempt_a_network_request() {
        let directory = tempfile::tempdir().unwrap();
        let store = RegistryStore::open(directory.path()).unwrap();
        let result = store.load_catalog(None).unwrap();
        assert_eq!(result.source, CatalogSource::Empty);
        assert!(result.catalog.is_none());
        assert!(result.warning.is_none());
    }

    #[test]
    fn successful_refresh_and_local_reload_have_distinct_sources() {
        let directory = tempfile::tempdir().unwrap();
        let store = RegistryStore::open(directory.path()).unwrap();
        let result = store.load_catalog(Some(&VALID)).unwrap();
        assert_eq!(result.source, CatalogSource::Fresh);
        assert!(result.warning.is_none());
        let cached = store.load_catalog(None).unwrap();
        assert_eq!(cached.source, CatalogSource::Cache);
        assert_eq!(cached.catalog.unwrap().version, "1.0.0");
    }

    #[test]
    fn offline_refresh_retains_cache_without_exporting_network_error_text() {
        let directory = tempfile::tempdir().unwrap();
        let store = RegistryStore::open(directory.path()).unwrap();
        store.refresh(&VALID).unwrap();
        let before = fs::read(store.root().join("index.json")).unwrap();
        let result = store.load_catalog(Some(&Offline)).unwrap();
        assert_eq!(result.source, CatalogSource::Cache);
        assert_eq!(result.warning, Some(CatalogWarning::Network));
        assert!(!format!("{result:?}").contains("canary"));
        assert_eq!(fs::read(store.root().join("index.json")).unwrap(), before);
    }

    #[test]
    fn rejected_json_and_unknown_schema_do_not_replace_a_valid_cache() {
        let directory = tempfile::tempdir().unwrap();
        let store = RegistryStore::open(directory.path()).unwrap();
        store.refresh(&VALID).unwrap();
        for bad in [
            Bytes(b"invalid"),
            Bytes(br#"{"version":"future","agents":[]}"#),
        ] {
            let result = store.load_catalog(Some(&bad)).unwrap();
            assert_eq!(result.source, CatalogSource::Cache);
            assert_eq!(result.warning, Some(CatalogWarning::Rejected));
            assert_eq!(result.catalog.unwrap().version, "1.0.0");
        }
    }

    #[test]
    fn failed_first_refresh_is_explicit_and_does_not_fabricate_a_catalog() {
        let directory = tempfile::tempdir().unwrap();
        let store = RegistryStore::open(directory.path()).unwrap();
        let result = store.load_catalog(Some(&Offline)).unwrap();
        assert_eq!(result.source, CatalogSource::Empty);
        assert_eq!(result.warning, Some(CatalogWarning::Network));
        assert!(result.catalog.is_none());
    }

    #[test]
    fn corrupt_cache_is_not_accepted_as_offline_fallback() {
        let directory = tempfile::tempdir().unwrap();
        let store = RegistryStore::open(directory.path()).unwrap();
        fs::write(store.root().join("index.json"), b"bad").unwrap();
        assert!(store.load_catalog(None).is_err());
        assert!(store.load_catalog(Some(&Offline)).is_err());
        assert_eq!(
            store.load_catalog(Some(&VALID)).unwrap().source,
            CatalogSource::Fresh
        );
    }
}
