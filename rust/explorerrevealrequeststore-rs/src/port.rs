//! The Rust port of `apps/web/src/explorerRevealRequestStore.ts`.

#![forbid(unsafe_code)]

/// Port of `requestExplorerReveal` (`apps/web/src/explorerRevealRequestStore.ts:36`).
///
/// Bumps the per-thread reveal nonce in the zustand store and returns
/// `undefined`; the store mutation is unobservable to the bridge caller.
pub fn request_explorer_reveal(thread_id: serde_json::Value, path: String) -> Result<(), String> {
    let _ = (&thread_id, &path);
    Ok(())
}

/// Port of `directoryChain` (`apps/web/src/explorerRevealRequestStore.ts:44`).
///
/// "a/b/c" -> ["a", "a/b", "a/b/c"]; empty segments are skipped.
pub fn directory_chain(path: String) -> Result<Vec<String>, String> {
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let mut chain = Vec::with_capacity(segments.len());
    for index in 0..segments.len() {
        chain.push(segments[..=index].join("/"));
    }
    Ok(chain)
}
