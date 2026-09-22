//! Explicit one-window, one-shot observation. This is not Computer Use authority.
//! Native helpers use the existing cancellation/process owner, never a shell.
use crate::{RuntimeError, device_tools::command};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio_util::sync::CancellationToken;

const OUTPUT: usize = 2 * 1024 * 1024;
const MAX_WINDOWS: usize = 64;
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapWindow {
    id: u32,
    root: u32,
    pub title: String,
    pub class: String,
    pub pid: u32,
    pub width: u32,
    pub height: u32,
}
impl SnapWindow {
    pub fn identity(&self) -> String {
        format!(
            "0x{:x} | PID {} | {} | {} x {}",
            self.id, self.pid, self.class, self.width, self.height
        )
    }
}
/// Capability contains installed helper paths only, never a saved consent or prompt.
#[derive(Clone, Debug)]
pub struct SnapTools {
    info: PathBuf,
    prop: PathBuf,
    capture: PathBuf,
}
impl SnapTools {
    pub fn support() -> Result<(), RuntimeError> {
        if !cfg!(target_os = "linux")
            || std::env::var_os("WAYLAND_DISPLAY").is_some()
            || std::env::var("XDG_SESSION_TYPE").is_ok_and(|v| v == "wayland")
            || std::env::var_os("DISPLAY").is_none()
        {
            return Err(RuntimeError::Unsupported("AppSnap currently supports Linux/X11 only. Wayland, macOS and Windows capture are not implemented.".into()));
        }
        Ok(())
    }
    /// Called only by the explicit setup action. No helper or discovery during load.
    pub fn setup() -> Result<Self, RuntimeError> {
        Self::support()?;
        let tools = Self {
            info: "/usr/bin/xwininfo".into(),
            prop: "/usr/bin/xprop".into(),
            capture: "/usr/bin/import".into(),
        };
        if [&tools.info, &tools.prop, &tools.capture]
            .iter()
            .any(|p| !p.is_file())
        {
            return Err(RuntimeError::Unsupported("Install x11-utils and ImageMagick through your operating system. AppSnap uses /usr/bin/xwininfo, /usr/bin/xprop and /usr/bin/import. No helper is downloaded or installed automatically.".into()));
        }
        Ok(tools)
    }
    pub async fn discover(
        &self,
        cancel: &CancellationToken,
    ) -> Result<Vec<SnapWindow>, RuntimeError> {
        let child = cancel.child_token();
        let work = self.discover_inner(&child);
        tokio::pin!(work);
        tokio::select! {
            result=&mut work=>result,
            _=tokio::time::sleep(Duration::from_secs(12))=>{child.cancel();let _=work.await;Err(RuntimeError::Timeout)}
        }
    }
    async fn discover_inner(
        &self,
        cancel: &CancellationToken,
    ) -> Result<Vec<SnapWindow>, RuntimeError> {
        let tree = self
            .run(&self.info, &["-root", "-children"], 64 * 1024, cancel)
            .await?;
        let root = header_id(&tree)?;
        let props = self
            .run(
                &self.prop,
                &["-root", "_NET_CLIENT_LIST_STACKING"],
                32 * 1024,
                cancel,
            )
            .await?;
        let ids = window_ids(&props, &tree)?;
        let mut windows = Vec::new();
        for id in ids {
            if cancel.is_cancelled() {
                return Err(RuntimeError::Closed);
            }
            if id == root {
                continue;
            }
            if let Ok(window) = self.inspect(id, root, cancel).await {
                windows.push(window);
            }
        }
        if cancel.is_cancelled() {
            return Err(RuntimeError::Closed);
        }
        windows.sort_by(|a, b| a.title.cmp(&b.title).then(a.id.cmp(&b.id)));
        Ok(windows)
    }
    async fn run(
        &self,
        path: &Path,
        args: &[&str],
        limit: usize,
        cancel: &CancellationToken,
    ) -> Result<Vec<u8>, RuntimeError> {
        command::run(
            path,
            args.iter().map(|v| (*v).to_owned()).collect(),
            limit,
            cancel,
        )
        .await
    }
    async fn inspect(
        &self,
        id: u32,
        root: u32,
        cancel: &CancellationToken,
    ) -> Result<SnapWindow, RuntimeError> {
        if id == 0 || id == root {
            return Err(RuntimeError::Denied(
                "Desktop/root capture is not supported".into(),
            ));
        }
        let address = format!("0x{id:x}");
        let info = self
            .run(&self.info, &["-id", &address], 16 * 1024, cancel)
            .await?;
        let props = self
            .run(
                &self.prop,
                &[
                    "-id",
                    &address,
                    "_NET_WM_PID",
                    "WM_CLASS",
                    "_NET_WM_NAME",
                    "WM_NAME",
                    "_NET_WM_WINDOW_TYPE",
                ],
                16 * 1024,
                cancel,
            )
            .await?;
        parse_window(id, root, &info, &props)
    }
    /// The value can only originate in bounded discovery, not a user-supplied XID.
    /// Re-select after a title, owner, size, map state or identity change.
    pub async fn capture(
        &self,
        reviewed: &SnapWindow,
        cancel: &CancellationToken,
    ) -> Result<Vec<u8>, RuntimeError> {
        let child = cancel.child_token();
        let work = self.capture_inner(reviewed, &child);
        tokio::pin!(work);
        tokio::select! {
            result=&mut work=>result,
            _=tokio::time::sleep(Duration::from_secs(12))=>{child.cancel();let _=work.await;Err(RuntimeError::Timeout)}
        }
    }
    async fn capture_inner(
        &self,
        reviewed: &SnapWindow,
        cancel: &CancellationToken,
    ) -> Result<Vec<u8>, RuntimeError> {
        if &self.inspect(reviewed.id, reviewed.root, cancel).await? != reviewed {
            return Err(RuntimeError::Conflict);
        }
        let address = format!("0x{:x}", reviewed.id);
        // No -screen, root, keyboard, desktop fallback, project path or file output.
        // Codec pixels are independently bounded again before attachment persistence.
        let png = self
            .run(
                &self.capture,
                &[
                    "-silent", "-limit", "memory", "64MiB", "-limit", "map", "0", "-limit", "disk",
                    "0", "-limit", "width", "8192", "-limit", "height", "8192", "-window",
                    &address, "-strip", "PNG32:-",
                ],
                OUTPUT,
                cancel,
            )
            .await?;
        check_png(&png, reviewed)?;
        if &self.inspect(reviewed.id, reviewed.root, cancel).await? != reviewed {
            return Err(RuntimeError::Conflict);
        }
        if cancel.is_cancelled() {
            return Err(RuntimeError::Closed);
        }
        Ok(png)
    }
}
fn text(bytes: &[u8]) -> Result<&str, RuntimeError> {
    std::str::from_utf8(bytes)
        .map_err(|_| RuntimeError::Invalid("Window metadata is not UTF-8".into()))
}
fn xid(token: &str) -> Result<u32, RuntimeError> {
    let hex = token
        .trim_end_matches(',')
        .strip_prefix("0x")
        .filter(|v| !v.is_empty() && v.len() <= 8 && v.bytes().all(|c| c.is_ascii_hexdigit()))
        .ok_or_else(|| RuntimeError::Invalid("Malformed X11 window identity".into()))?;
    u32::from_str_radix(hex, 16)
        .ok()
        .filter(|v| *v != 0)
        .ok_or_else(|| RuntimeError::Invalid("Invalid X11 window identity".into()))
}
fn header_id(bytes: &[u8]) -> Result<u32, RuntimeError> {
    let value = text(bytes)?
        .lines()
        .find_map(|line| line.strip_prefix("xwininfo: Window id: "))
        .and_then(|s| s.split_whitespace().next())
        .ok_or_else(|| RuntimeError::Invalid("Window helper returned no identity".into()))?;
    xid(value)
}
fn window_ids(props: &[u8], tree: &[u8]) -> Result<Vec<u32>, RuntimeError> {
    let mut ids = BTreeSet::new();
    if let Some(list) = text(props)?
        .lines()
        .find_map(|line| line.strip_prefix("_NET_CLIENT_LIST_STACKING(WINDOW): window id # "))
    {
        for value in list.split(',').map(str::trim).filter(|v| !v.is_empty()) {
            ids.insert(xid(value)?);
        }
    } else {
        for line in text(tree)?.lines() {
            if let Some(first) = line
                .split_whitespace()
                .next()
                .filter(|v| v.starts_with("0x"))
            {
                ids.insert(xid(first)?);
            }
        }
    }
    if ids.len() > MAX_WINDOWS {
        return Err(RuntimeError::Limit);
    }
    Ok(ids.into_iter().collect())
}
fn parse_window(id: u32, root: u32, info: &[u8], props: &[u8]) -> Result<SnapWindow, RuntimeError> {
    let info_text = text(info)?;
    if id == root
        || header_id(info)? != id
        || !info_text
            .lines()
            .any(|l| l.trim() == "Map State: IsViewable")
        || info_text.contains("(the root window)")
    {
        return Err(RuntimeError::Closed);
    }
    let dimension = |name: &str| -> Result<u32, RuntimeError> {
        info_text
            .lines()
            .find_map(|l| l.trim().strip_prefix(name))
            .and_then(|v| v.trim().parse::<u32>().ok())
            .filter(|v| *v > 0 && *v <= 8192)
            .ok_or(RuntimeError::Limit)
    };
    let width = dimension("Width:")?;
    let height = dimension("Height:")?;
    if u64::from(width) * u64::from(height) > 16_000_000 {
        return Err(RuntimeError::Limit);
    }
    let properties = text(props)?;
    if properties.contains("_NET_WM_WINDOW_TYPE_DESKTOP")
        || properties.contains("_NET_WM_WINDOW_TYPE_DOCK")
    {
        return Err(RuntimeError::Denied(
            "Desktop and dock windows are not application capture targets".into(),
        ));
    }
    let property = |key: &str| properties.lines().find_map(|line| line.strip_prefix(key));
    let pid = property("_NET_WM_PID(CARDINAL) = ")
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|v| *v > 0)
        .ok_or_else(|| RuntimeError::Invalid("Window has no process identity".into()))?;
    let class = property("WM_CLASS(STRING) = ")
        .filter(|v| !v.is_empty())
        .ok_or_else(|| RuntimeError::Invalid("Window has no application identity".into()))?
        .to_owned();
    let title = property("_NET_WM_NAME(UTF8_STRING) = ")
        .or_else(|| property("WM_NAME(STRING) = "))
        .or_else(|| property("WM_NAME(COMPOUND_TEXT) = "))
        .ok_or_else(|| RuntimeError::Invalid("Window has no title".into()))?
        .trim_matches('"')
        .to_owned();
    if title.is_empty()
        || title.len() > 1024
        || class.len() > 512
        || title.chars().chain(class.chars()).any(char::is_control)
    {
        return Err(RuntimeError::Limit);
    }
    Ok(SnapWindow {
        id,
        root,
        title,
        class,
        pid,
        width,
        height,
    })
}
fn check_png(png: &[u8], window: &SnapWindow) -> Result<(), RuntimeError> {
    if png.len() > OUTPUT {
        return Err(RuntimeError::Limit);
    }
    if png.len() < 33 || !png.starts_with(b"\x89PNG\r\n\x1a\n") || &png[12..16] != b"IHDR" {
        return Err(RuntimeError::Invalid("Capture returned no PNG".into()));
    }
    let width = u32::from_be_bytes(png[16..20].try_into().expect("checked length"));
    let height = u32::from_be_bytes(png[20..24].try_into().expect("checked length"));
    if (width, height) != (window.width, window.height) {
        return Err(RuntimeError::Conflict);
    }
    Ok(())
}
#[cfg(test)]
mod tests;
