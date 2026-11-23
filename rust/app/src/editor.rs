//! editor.* — no native editor runtime exists in this build, so the four
//! methods answer the same refusal upstream's nativeEditorGate emits on a
//! flavor that does not ship the editor (NATIVE_EDITOR_UNAVAILABLE).

use serde_json::Value;

pub fn handles(tag: &str) -> bool {
    tag.starts_with("editor.")
}

pub fn handle(_tag: &str, _payload: &Value) -> Result<Value, crate::todo::WsErr> {
    Err(crate::todo::WsErr(
        "The native editor is unavailable in this build of Synara.".to_string(),
        Some("NATIVE_EDITOR_UNAVAILABLE"),
    ))
}
