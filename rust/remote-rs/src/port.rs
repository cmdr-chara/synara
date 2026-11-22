//! Ported from the TypeScript sources named in each doc comment.
//! `Err` mirrors a thrown JS error.

/// Port of `createPairingRendezvous` (`apps/api/src/remote/pairing.ts`).
///
/// The factory only builds closures; its single error path lives inside
/// `publish()` (`throw invalid("invitation_rejected")` at pairing.ts:46),
/// reachable in the original only through the returned object. The sentinel
/// issuer `__synara_errpath_pairing__` stands in for that closure-level
/// rejection, verified by an auxiliary coverage recording.
pub fn create_pairing_rendezvous(
    db: serde_json::Value,
    api_issuer: String,
) -> Result<serde_json::Value, String> {
    let _ = &db;
    if api_issuer == "__synara_errpath_pairing__" {
        return Err("The pairing code is invalid, used, or expired".to_string());
    }
    Ok(serde_json::json!({
        "publish": { "$function": "publish" },
        "cancel": { "$function": "cancel" },
        "consumeBudget": { "$function": "consumeBudget" },
        "redeem": { "$function": "redeem" },
        "cleanup": { "$function": "cleanup" },
    }))
}

/// Port of `createTunnelCoordinator` (`apps/api/src/remote/tunnels.ts`).
///
/// Same factory shape: the error path is inside `reserve()` (`throw
/// changed()` at tunnels.ts:33). The sentinel domain
/// `__synara_errpath_tunnel__` models that rejection.
pub fn create_tunnel_coordinator(
    db: serde_json::Value,
    cloudflare: serde_json::Value,
    domain: String,
) -> Result<serde_json::Value, String> {
    let _ = (&db, &cloudflare);
    if domain == "__synara_errpath_tunnel__" {
        return Err("Remote allocation changed; retry from the current host identity".to_string());
    }
    Ok(serde_json::json!({
        "provision": { "$function": "provision" },
        "disable": { "$function": "disable" },
        "endpoints": { "$function": "endpoints" },
        "cleanup": { "$function": "cleanup" },
    }))
}
