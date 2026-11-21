//! Clean-room port of `packages/contracts/src/providerInstance.ts`,
//! written from recorded behavior only.
use serde_json::Value;

fn is_letter(u: u16) -> bool {
    matches!(u, 97..=122 | 65..=90) // a-z | A-Z
}

/// Port of `isProviderDriverKind`: a string matching
/// `^[A-Za-z][A-Za-z0-9_-]*$` on UTF-16 units.
pub fn is_provider_driver_kind(value: Value) -> Result<bool, String> {
    let Some(units) = crate::js::string_units(&value) else {
        return Ok(false);
    };
    Ok(match units.first() {
        Some(&u) if is_letter(u) => units[1..]
            .iter()
            .all(|&u| is_letter(u) || matches!(u, 48..=57 | 45 | 95)),
        _ => false,
    })
}

/// Port of `defaultInstanceIdForDriver`: the driver name itself is the
/// default instance id — recorded output is the input verbatim.
pub fn default_instance_id_for_driver(driver: Value) -> Result<Value, String> {
    Ok(driver)
}
