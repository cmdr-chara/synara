//! Production OS credential-store adapter. Unsupported platforms never use
//! keyring's mock backend, environment variables, a file, or a plaintext fallback.
use crate::{RuntimeError, SecretReference, SecretStore, SecretStoreState, SecretValue};
use async_trait::async_trait;
use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};

#[derive(Clone)]
pub struct NativeSecretStore {
    observed: Arc<AtomicU8>,
    operation: Arc<tokio::sync::Semaphore>,
}
impl Default for NativeSecretStore {
    fn default() -> Self {
        Self::new()
    }
}
impl NativeSecretStore {
    pub fn new() -> Self {
        Self {
            observed: Arc::new(AtomicU8::new(0)),
            operation: Arc::new(tokio::sync::Semaphore::new(1)),
        }
    }
    /// Read-only access probe. A successful absent-entry lookup proves access to
    /// the native store, not the presence or validity of any provider credential.
    pub async fn probe(&self) -> SecretStoreState {
        if let Ok(reference) = SecretReference::new("synara.credential-store-probe", "availability")
        {
            let _ = self.read(&reference).await;
        }
        self.state()
    }
    async fn run(
        &self,
        reference: SecretReference,
        action: Operation,
    ) -> Result<Option<SecretValue>, RuntimeError> {
        reference.validate()?;
        // Keep one blocking operation owned even if its caller goes away. A second
        // request fails explicitly rather than accumulating unbounded DBus workers.
        let permit = self.operation.clone().try_acquire_owned().map_err(|_| {
            RuntimeError::Invalid("A credential-store operation is already in progress".into())
        })?;
        let observed = self.observed.clone();
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            native_operation(reference, action, &observed)
        })
        .await
        .map_err(|_| RuntimeError::Closed)?
    }
}
enum Operation {
    Read,
    Write(SecretValue),
    Delete,
}
#[async_trait]
impl SecretStore for NativeSecretStore {
    fn state(&self) -> SecretStoreState {
        match self.observed.load(Ordering::Acquire) {
            1 => SecretStoreState::Available,
            2 => SecretStoreState::Locked,
            _ => SecretStoreState::Unavailable,
        }
    }
    async fn read(&self, reference: &SecretReference) -> Result<Option<SecretValue>, RuntimeError> {
        self.run(reference.clone(), Operation::Read).await
    }
    async fn write(
        &self,
        reference: &SecretReference,
        value: SecretValue,
    ) -> Result<(), RuntimeError> {
        self.run(reference.clone(), Operation::Write(value))
            .await
            .map(|_| ())
    }
    async fn delete(&self, reference: &SecretReference) -> Result<(), RuntimeError> {
        self.run(reference.clone(), Operation::Delete)
            .await
            .map(|_| ())
    }
}
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
fn native_operation(
    reference: SecretReference,
    action: Operation,
    observed: &AtomicU8,
) -> Result<Option<SecretValue>, RuntimeError> {
    let entry = keyring::Entry::new(&reference.service, &reference.account)
        .map_err(|e| redact_error(e, observed))?;
    let result = match action {
        Operation::Read => match entry.get_secret() {
            Ok(mut bytes) => {
                observed.store(1, Ordering::Release);
                if bytes.is_empty() || bytes.len() > 64 * 1024 {
                    bytes.fill(0);
                    return Err(RuntimeError::Limit);
                }
                return SecretValue::new(bytes).map(Some);
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(error),
        },
        Operation::Write(value) => entry.set_secret(value.expose()).map(|_| None),
        Operation::Delete => match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(error),
        },
    };
    match result {
        Ok(value) => {
            observed.store(1, Ordering::Release);
            Ok(value)
        }
        Err(error) => Err(redact_error(error, observed)),
    }
}
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
fn redact_error(error: keyring::Error, observed: &AtomicU8) -> RuntimeError {
    // Native errors can embed secret bytes, attribute values or OS paths. Never
    // format them into logs, errors, events, SQLite, clipboard or UI text.
    match error {
        keyring::Error::NoStorageAccess(_) => {
            observed.store(2, Ordering::Release);
            RuntimeError::Denied("The OS credential store is locked or access was denied".into())
        }
        keyring::Error::BadEncoding(mut bytes) => {
            bytes.fill(0);
            observed.store(0, Ordering::Release);
            RuntimeError::Invalid("The OS credential entry is not usable".into())
        }
        _ => {
            observed.store(0, Ordering::Release);
            RuntimeError::Unsupported("The OS credential store could not complete this operation. No fallback or retry was used".into())
        }
    }
}
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn native_operation(
    _: SecretReference,
    _: Operation,
    observed: &AtomicU8,
) -> Result<Option<SecretValue>, RuntimeError> {
    observed.store(0, Ordering::Release);
    Err(RuntimeError::Unsupported(
        "No native credential-store adapter is available on this platform".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_secrets_start_unverified_not_authenticated() {
        assert_eq!(
            NativeSecretStore::new().state(),
            SecretStoreState::Unavailable
        );
    }
    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
    #[test]
    fn native_secrets_errors_never_format_sensitive_payloads() {
        let observed = AtomicU8::new(1);
        for error in [
            keyring::Error::BadEncoding(b"test-private-key".to_vec()),
            keyring::Error::Invalid("token".into(), "test-private-key".into()),
        ] {
            let error = redact_error(error, &observed);
            assert!(!format!("{error:?} {error}").contains("test-private-key"));
            assert_eq!(observed.load(Ordering::Acquire), 0);
        }
    }
    #[tokio::test]
    async fn native_secrets_reject_parallel_workers_without_touching_the_os() {
        let store = NativeSecretStore::new();
        let _permit = store.operation.clone().acquire_owned().await.unwrap();
        let reference = SecretReference::new("synara-test", "never-accessed").unwrap();
        assert!(matches!(
            store.read(&reference).await,
            Err(RuntimeError::Invalid(_))
        ));
        assert_eq!(store.state(), SecretStoreState::Unavailable);
    }
}
