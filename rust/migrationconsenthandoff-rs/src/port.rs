//! The Rust port of `apps/desktop/src/migrationConsentHandoff.ts`.

#![forbid(unsafe_code)]

/// Port of class `MigrationConsentHandoff` (`apps/desktop/src/migrationConsentHandoff.ts:1`): holds the instance state.
pub struct MigrationConsentHandoff {
    pending: Option<String>,
}

impl MigrationConsentHandoff {
    /// Port of `MigrationConsentHandoff` constructor (`apps/desktop/src/migrationConsentHandoff.ts:1`).
    pub fn new() -> Result<Self, String> {
        Ok(Self { pending: None })
    }

    /// Port of `MigrationConsentHandoff.approve` (`apps/desktop/src/migrationConsentHandoff.ts:4`).
    pub fn approve(&mut self, consent_token: String) -> Result<(), String> {
        self.pending = Some(consent_token);
        Ok(())
    }

    /// Port of `MigrationConsentHandoff.take` (`apps/desktop/src/migrationConsentHandoff.ts:8`).
    pub fn take(&mut self) -> Result<serde_json::Value, String> {
        Ok(self
            .pending
            .take()
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null))
    }
}
