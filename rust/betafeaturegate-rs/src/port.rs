//! The Rust port of `apps/server/src/betaFeatureGate.ts`.

#![forbid(unsafe_code)]

use std::sync::OnceLock;

const SYNARA_DESKTOP_BUNDLE_ID_ENV: &str = "SYNARA_DESKTOP_BUNDLE_ID";

const SYNARA_PRODUCTION_BUNDLE_ID: &str = "com.emanueledipietro.synara";
const SYNARA_DEVELOPMENT_BUNDLE_ID: &str = "com.emanueledipietro.synara.dev";
const SYNARA_CANARY_BUNDLE_ID: &str = "com.emanueledipietro.synara.canary";
const SYNARA_CUA_BUNDLE_ID: &str = "com.emanueledipietro.synara.cua";
const SYNARA_BETA_BUNDLE_ID: &str = "com.emanueledipietro.synara.beta";
const SYNARA_REMOTE_BUNDLE_ID: &str = "com.emanueledipietro.synara.remote";

const BETA_ONLY_FEATURES: &[&str] = &[];

fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

fn desktop_flavor_from_bundle_id(bundle_id: Option<&str>) -> &'static str {
    match bundle_id.map(|b| b.trim_matches(is_js_space)) {
        Some(SYNARA_PRODUCTION_BUNDLE_ID) => "production",
        Some(SYNARA_DEVELOPMENT_BUNDLE_ID) => "development",
        Some(SYNARA_CANARY_BUNDLE_ID) => "canary",
        Some(SYNARA_CUA_BUNDLE_ID) => "cua",
        Some(SYNARA_REMOTE_BUNDLE_ID) => "remote",
        Some(SYNARA_BETA_BUNDLE_ID) => "beta",
        _ => "unknown",
    }
}

/// The hosting desktop's flavor, resolved once like the source's module-level const.
fn server_desktop_flavor() -> &'static str {
    static FLAVOR: OnceLock<&'static str> = OnceLock::new();
    FLAVOR.get_or_init(|| {
        desktop_flavor_from_bundle_id(std::env::var(SYNARA_DESKTOP_BUNDLE_ID_ENV).ok().as_deref())
    })
}

/// Port of `isServerBetaFeatureEnabled` (`apps/server/src/betaFeatureGate.ts:14`).
pub fn is_server_beta_feature_enabled(feature: String) -> Result<bool, String> {
    Ok(!BETA_ONLY_FEATURES.contains(&feature.as_str()) || server_desktop_flavor() != "production")
}
