//! The Rust port of `apps/web/src/sidebarNavOrdering.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

/// Port of `resolveTasksSurfaceSlot` (`apps/web/src/sidebarNavOrdering.ts:23`).
pub fn resolve_tasks_surface_slot(
    order: Vec<String>,
    tasks_enabled: bool,
) -> Result<Vec<String>, String> {
    let active = if tasks_enabled { "tasks" } else { "kanban" };
    let mut placed = false;
    let mut resolved = Vec::new();
    for id in order {
        if id != "kanban" && id != "tasks" {
            resolved.push(id);
        } else if !placed {
            resolved.push(active.to_string());
            placed = true;
        }
    }
    Ok(resolved)
}
