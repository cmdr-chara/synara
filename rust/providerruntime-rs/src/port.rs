//! Clean-room port of `packages/contracts/src/providerRuntime.ts`,
//! written from recorded behavior only.
use serde_json::Value;

/// `TOOL_LIFECYCLE_ITEM_TYPES` membership.
const TYPES: [&str; 8] = [
    "command_execution",
    "file_change",
    "mcp_tool_call",
    "dynamic_tool_call",
    "collab_agent_tool_call",
    "web_search",
    "image_view",
    "image_generation",
];

/// Port of `isToolLifecycleItemType`: set membership on the exact string
/// (`$utf16` marker strings count; everything else is false).
pub fn is_tool_lifecycle_item_type(value: Value) -> Result<bool, String> {
    let Some(units) = crate::js::string_units(&value) else {
        return Ok(false);
    };
    let Ok(s) = String::from_utf16(&units) else {
        return Ok(false);
    };
    Ok(TYPES.contains(&s.as_str()))
}
