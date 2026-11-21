//! The Rust port of `packages/shared/src/computerKeyNames.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

const NAMED_KEYS: &[&str] = &[
    "arrowup",
    "arrowdown",
    "arrowleft",
    "arrowright",
    "backspace",
    "delete",
    "end",
    "enter",
    "escape",
    "home",
    "insert",
    "pagedown",
    "pageup",
    "space",
    "tab",
    "f1",
    "f2",
    "f3",
    "f4",
    "f5",
    "f6",
    "f7",
    "f8",
    "f9",
    "f10",
    "f11",
    "f12",
];

/// Port of `isComputerNamedKey` (`packages/shared/src/computerKeyNames.ts`).
pub fn is_computer_named_key(key: Value) -> Result<bool, String> {
    Ok(key.as_str().is_some_and(|k| NAMED_KEYS.contains(&k)))
}
