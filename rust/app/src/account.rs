//! account.* — this server has no account backend, which mirrors an
//! upstream server whose account session cannot complete a call: status
//! reports signed-out, signOut is an idempotent no-op, and every other
//! method answers the verbatim fallback message upstream emits when its
//! account service fails (wsAccountRpc).

use serde_json::{json, Value};

pub fn handles(tag: &str) -> bool {
    tag.starts_with("account.")
}

pub fn handle(tag: &str, _payload: &Value) -> Result<Value, crate::todo::WsErr> {
    let err = |m: &str| crate::todo::WsErr(m.to_string(), None);
    match tag {
        "account.status" => Ok(json!({"state": "signed-out"})),
        "account.signOut" => Ok(json!({})),
        "account.sendOtp" => Err(err("Could not send the code. Try again in a minute.")),
        "account.authenticateOtp" => {
            Err(err("Could not sign in. Check the code and try again."))
        }
        "account.beginSso" => Err(err("Failed to start sign-in")),
        "account.completeSso" => Err(err("Failed to finish signing in")),
        "account.cancelSso" => Err(err("Failed to cancel the sign-in")),
        "account.usageSummary" => Err(err("Failed to load account usage")),
        "account.saveInboxRecap" => Err(err("Failed to save the private recap")),
        "account.listInboxRecaps" => Err(err("Failed to load saved recaps")),
        "account.deleteInboxRecap" => Err(err("Failed to delete the saved recap")),
        "account.updateProfile" => Err(err("Failed to save your profile")),
        "account.uploadAvatar" => Err(err("Failed to upload your avatar")),
        "account.deleteAvatar" => Err(err("Failed to remove your avatar")),
        "account.delete" => Err(err("Could not confirm account deletion")),
        "account.billingStatus" => Err(err("Could not load the subscription")),
        "account.billingCheckout" => Err(err("Could not open the test checkout")),
        "account.billingPortal" => Err(err("Could not open subscription settings")),
        "account.openVerificationUrl" => Err(err(
            "That link is not a sign-in verification URL from this server.",
        )),
        _ => Err(err("unhandled native tag")),
    }
}
