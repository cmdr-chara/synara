use crate::RuntimeError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

const MAX_SECRET_BYTES: usize = 64 * 1024;
const MAX_REFERENCE_BYTES: usize = 256;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SecretReference {
    pub service: String,
    pub account: String,
}

impl SecretReference {
    pub fn new(service: impl Into<String>, account: impl Into<String>) -> Result<Self, RuntimeError> {
        let reference = Self {
            service: service.into(),
            account: account.into(),
        };
        reference.validate()?;
        Ok(reference)
    }

    pub fn validate(&self) -> Result<(), RuntimeError> {
        for value in [&self.service, &self.account] {
            if value.is_empty()
                || value.len() > MAX_REFERENCE_BYTES
                || value.chars().any(char::is_control)
            {
                return Err(RuntimeError::Invalid(
                    "invalid credential-store reference".into(),
                ));
            }
        }
        Ok(())
    }
}

/// Secret bytes deliberately implement neither Clone nor Serialize. Debug output
/// is always redacted, and owned bytes are cleared before deallocation.
pub struct SecretValue(Vec<u8>);

impl SecretValue {
    pub fn new(value: Vec<u8>) -> Result<Self, RuntimeError> {
        if value.is_empty() || value.len() > MAX_SECRET_BYTES {
            return Err(RuntimeError::Limit);
        }
        Ok(Self(value))
    }

    pub fn expose(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Debug for SecretValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretValue([REDACTED])")
    }
}

impl Drop for SecretValue {
    fn drop(&mut self) {
        self.0.fill(0);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SecretStoreState {
    Available,
    Locked,
    Unavailable,
}

#[async_trait]
pub trait SecretStore: Send + Sync {
    fn state(&self) -> SecretStoreState;

    async fn read(&self, reference: &SecretReference)
    -> Result<Option<SecretValue>, RuntimeError>;

    async fn write(
        &self,
        reference: &SecretReference,
        value: SecretValue,
    ) -> Result<(), RuntimeError>;

    async fn delete(&self, reference: &SecretReference) -> Result<(), RuntimeError>;
}

/// Explicit fail-closed boundary used when the target has no usable OS credential
/// provider. Callers must surface this state instead of persisting a plaintext
/// replacement in SQLite, settings, environment files, or logs.
#[derive(Clone, Copy, Debug)]
pub struct UnavailableSecretStore {
    state: SecretStoreState,
}

impl UnavailableSecretStore {
    pub fn unavailable() -> Self {
        Self {
            state: SecretStoreState::Unavailable,
        }
    }

    pub fn locked() -> Self {
        Self {
            state: SecretStoreState::Locked,
        }
    }

    fn error(self) -> RuntimeError {
        match self.state {
            SecretStoreState::Locked => RuntimeError::Denied(
                "the operating-system credential store is locked".into(),
            ),
            SecretStoreState::Unavailable | SecretStoreState::Available => RuntimeError::Unsupported(
                "an operating-system credential store is unavailable".into(),
            ),
        }
    }
}

#[async_trait]
impl SecretStore for UnavailableSecretStore {
    fn state(&self) -> SecretStoreState {
        self.state
    }

    async fn read(
        &self,
        reference: &SecretReference,
    ) -> Result<Option<SecretValue>, RuntimeError> {
        reference.validate()?;
        Err(self.error())
    }

    async fn write(
        &self,
        reference: &SecretReference,
        _value: SecretValue,
    ) -> Result<(), RuntimeError> {
        reference.validate()?;
        Err(self.error())
    }

    async fn delete(&self, reference: &SecretReference) -> Result<(), RuntimeError> {
        reference.validate()?;
        Err(self.error())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::BTreeMap, sync::Mutex};

    #[derive(Default)]
    struct MemoryStore(Mutex<BTreeMap<SecretReference, Vec<u8>>>);

    #[async_trait]
    impl SecretStore for MemoryStore {
        fn state(&self) -> SecretStoreState {
            SecretStoreState::Available
        }

        async fn read(
            &self,
            reference: &SecretReference,
        ) -> Result<Option<SecretValue>, RuntimeError> {
            reference.validate()?;
            self.0
                .lock()
                .unwrap()
                .get(reference)
                .cloned()
                .map(SecretValue::new)
                .transpose()
        }

        async fn write(
            &self,
            reference: &SecretReference,
            value: SecretValue,
        ) -> Result<(), RuntimeError> {
            reference.validate()?;
            self.0
                .lock()
                .unwrap()
                .insert(reference.clone(), value.expose().to_vec());
            Ok(())
        }

        async fn delete(&self, reference: &SecretReference) -> Result<(), RuntimeError> {
            reference.validate()?;
            self.0.lock().unwrap().remove(reference);
            Ok(())
        }
    }

    #[tokio::test]
    async fn references_are_persistable_but_secret_values_are_redacted() {
        let reference = SecretReference::new("dev.synara", "token:test").unwrap();
        let encoded = serde_json::to_string(&reference).unwrap();
        assert!(encoded.contains("dev.synara"));
        let value = SecretValue::new(b"secret-canary".to_vec()).unwrap();
        assert_eq!(format!("{value:?}"), "SecretValue([REDACTED])");
        assert!(!format!("{value:?}").contains("secret-canary"));

        let store = MemoryStore::default();
        store.write(&reference, value).await.unwrap();
        let restored = store.read(&reference).await.unwrap().unwrap();
        assert_eq!(restored.expose(), b"secret-canary");
        store.delete(&reference).await.unwrap();
        assert!(store.read(&reference).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn unavailable_and_locked_stores_fail_closed() {
        let reference = SecretReference::new("dev.synara", "account").unwrap();
        let secret = SecretValue::new(b"secret".to_vec()).unwrap();
        assert!(matches!(
            UnavailableSecretStore::unavailable()
                .write(&reference, secret)
                .await,
            Err(RuntimeError::Unsupported(_))
        ));
        assert!(matches!(
            UnavailableSecretStore::locked().read(&reference).await,
            Err(RuntimeError::Denied(_))
        ));
    }

    #[test]
    fn hostile_references_and_unbounded_values_are_rejected() {
        assert!(SecretReference::new("", "account").is_err());
        assert!(SecretReference::new("dev.synara", "line\nbreak").is_err());
        assert!(SecretValue::new(Vec::new()).is_err());
        assert!(SecretValue::new(vec![0; MAX_SECRET_BYTES + 1]).is_err());
    }
}
