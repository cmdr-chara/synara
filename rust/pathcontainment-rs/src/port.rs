//! The Rust port of `packages/shared/src/pathContainment.ts`.
//!
//! Behavior inferred clean-room from recorded cases: true when the parent is
//! empty, the paths are equal, or the child sits under `parent + '/'`.

#![forbid(unsafe_code)]

/// Port of `isPathInside` (`packages/shared/src/pathContainment.ts:12`).
pub fn is_path_inside(parent: String, child: String) -> Result<bool, String> {
    Ok(parent.is_empty() || child == parent || child.starts_with(&format!("{parent}/")))
}
