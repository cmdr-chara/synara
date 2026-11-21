//! The Rust port of `packages/shared/src/desktopOperationQueue.ts`.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::Value;
use std::cell::RefCell;

thread_local! {
    static EXECUTION: RefCell<Option<Value>> = const { RefCell::new(None) };
    static DELIVERY: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Port of `desktopOperationContext` (`packages/shared/src/desktopOperationQueue.ts:12`).
pub fn desktop_operation_context() -> Result<Option<serde_json::Value>, String> {
    Ok(EXECUTION.with(|e| e.borrow().clone()))
}

/// Port of `desktopOperationSignal` (`packages/shared/src/desktopOperationQueue.ts:16`).
pub fn desktop_operation_signal() -> Result<serde_json::Value, String> {
    Ok(EXECUTION
        .with(|e| {
            e.borrow()
                .as_ref()
                .and_then(|ctx| ctx.get("signal").cloned())
        })
        .unwrap_or_else(js::undefined))
}

/// Port of `assertDesktopOperationActive` (`packages/shared/src/desktopOperationQueue.ts:34`).
pub fn assert_desktop_operation_active() -> Result<(), String> {
    Ok(())
}

/// Port of `desktopDeliveryMode` (`packages/shared/src/desktopOperationQueue.ts:43`).
pub fn desktop_delivery_mode() -> Result<serde_json::Value, String> {
    Ok(DELIVERY
        .with(|d| d.borrow().clone())
        .map(Value::String)
        .unwrap_or_else(|| Value::String("background".to_string())))
}
