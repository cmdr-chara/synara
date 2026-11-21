// Shim so `mod tables` inside a `#[path]`-included port.rs resolves.
pub use self::imp::*;
#[path = "port/tables.rs"]
mod imp;
